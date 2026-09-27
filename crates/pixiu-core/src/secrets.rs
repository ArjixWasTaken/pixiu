//! Encryption of secrets at rest: the password Subsonic token authentication
//! needs, and later the streaming platforms' session cookies.
//!
//! Secrets are sealed with XChaCha20-Poly1305 under a per-instance key kept
//! in the data directory. Anyone holding both the database and the key file
//! can read them; the point is that a copied database alone reveals nothing.

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, Generate, Key, KeyInit},
};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("failed to access the secret key at {path}: {source}")]
    Io { path: String, source: io::Error },
    #[error("the secret key at {0} is corrupt")]
    CorruptKey(String),
    #[error("a sealed secret could not be decrypted (wrong key or tampered data)")]
    Undecryptable,
}

/// Seals and opens secrets with the instance key.
#[derive(Clone)]
pub struct SecretBox {
    cipher: XChaCha20Poly1305,
}

impl std::fmt::Debug for SecretBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretBox(..)")
    }
}

impl SecretBox {
    /// Loads the key at `path`, generating it on first use.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read or created, or holds a key of the
    /// wrong length.
    pub fn load_or_create(path: &Path) -> Result<Self, SecretError> {
        let io_error = |source| SecretError::Io {
            path: path.display().to_string(),
            source,
        };

        let key = match fs::read(path) {
            Ok(bytes) => Key::<XChaCha20Poly1305>::try_from(bytes.as_slice())
                .map_err(|_| SecretError::CorruptKey(path.display().to_string()))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let key = Key::<XChaCha20Poly1305>::generate();
                write_private(path, key.as_slice()).map_err(io_error)?;
                key
            }
            Err(error) => return Err(io_error(error)),
        };
        debug_assert_eq!(key.len(), KEY_LEN);
        Ok(Self {
            cipher: XChaCha20Poly1305::new(&key),
        })
    }

    /// A box with a random, unpersisted key (for tests).
    #[must_use]
    pub fn ephemeral() -> Self {
        Self {
            cipher: XChaCha20Poly1305::new(&Key::<XChaCha20Poly1305>::generate()),
        }
    }

    /// Encrypts `plaintext` into a printable string.
    #[must_use]
    pub fn seal(&self, plaintext: &[u8]) -> String {
        let nonce = XNonce::generate();
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .expect("encryption with a valid key cannot fail");
        let mut sealed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        sealed.extend_from_slice(nonce.as_slice());
        sealed.extend_from_slice(&ciphertext);
        STANDARD.encode(sealed)
    }

    /// Decrypts a string produced by [`seal`](Self::seal).
    ///
    /// # Errors
    ///
    /// Fails when the input was sealed with another key or was modified.
    pub fn open(&self, sealed: &str) -> Result<Vec<u8>, SecretError> {
        let bytes = STANDARD
            .decode(sealed)
            .map_err(|_| SecretError::Undecryptable)?;
        if bytes.len() < NONCE_LEN {
            return Err(SecretError::Undecryptable);
        }
        let (nonce, ciphertext) = bytes.split_at(NONCE_LEN);
        let nonce = XNonce::try_from(nonce).map_err(|_| SecretError::Undecryptable)?;
        self.cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| SecretError::Undecryptable)
    }

    /// Convenience for sealing UTF-8 text.
    #[must_use]
    pub fn seal_str(&self, plaintext: &str) -> String {
        self.seal(plaintext.as_bytes())
    }

    /// Convenience for opening UTF-8 text.
    ///
    /// # Errors
    ///
    /// Fails like [`open`](Self::open), or when the plaintext is not UTF-8.
    pub fn open_str(&self, sealed: &str) -> Result<String, SecretError> {
        String::from_utf8(self.open(sealed)?).map_err(|_| SecretError::Undecryptable)
    }
}

/// Creates `path` readable only by the current user.
fn write_private(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_persists_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.key");

        let first = SecretBox::load_or_create(&path).unwrap();
        let sealed = first.seal_str("hunter2");
        assert_ne!(sealed, first.seal_str("hunter2"), "nonces are random");

        let reloaded = SecretBox::load_or_create(&path).unwrap();
        assert_eq!(reloaded.open_str(&sealed).unwrap(), "hunter2");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn rejects_foreign_or_tampered_secrets() {
        let sealed = SecretBox::ephemeral().seal_str("hunter2");
        assert!(SecretBox::ephemeral().open(&sealed).is_err());

        let secrets = SecretBox::ephemeral();
        let mut bytes = STANDARD.decode(secrets.seal_str("hunter2")).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        assert!(secrets.open(&STANDARD.encode(bytes)).is_err());
        assert!(secrets.open("not base64!").is_err());
    }

    #[test]
    fn rejects_a_corrupt_key_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.key");
        fs::write(&path, b"short").unwrap();
        assert!(matches!(
            SecretBox::load_or_create(&path),
            Err(SecretError::CorruptKey(_))
        ));
    }
}
