//! Password hashing (argon2id). These functions are deliberately slow; call
//! them from a blocking context.

use std::sync::LazyLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier},
};

/// Hashes `password` into a PHC string.
///
/// # Panics
///
/// Never in practice: hashing with default parameters cannot fail.
#[must_use]
pub fn hash(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .expect("hashing with default parameters succeeds")
        .to_string()
}

/// Checks `password` against a stored PHC hash. Pass `None` for unknown
/// users: the password is then checked against a dummy hash, so a miss costs
/// as much time as a wrong password and timing does not reveal usernames.
#[must_use]
pub fn verify(password: &str, hash: Option<&str>) -> bool {
    static DUMMY: LazyLock<String> = LazyLock::new(|| self::hash("not the password"));

    let known = hash.is_some();
    let matches = Argon2::default()
        .verify_password(password.as_bytes(), hash.unwrap_or(&DUMMY))
        .is_ok();
    known && matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_only_the_right_password() {
        let stored = hash("gold and jade");
        assert!(stored.starts_with("$argon2id$"));
        assert!(verify("gold and jade", Some(&stored)));
        assert!(!verify("gold and brass", Some(&stored)));
        assert!(!verify("gold and jade", None));
        assert!(!verify("gold and jade", Some("not a hash")));
    }
}
