//! Runtime configuration.
//!
//! Values are layered, later sources overriding earlier ones:
//!
//! 1. built-in defaults,
//! 2. a TOML file (`pixiu.toml` in the working directory, or the path in
//!    `PIXIU_CONFIG`),
//! 3. `PIXIU_`-prefixed environment variables, with `__` separating nested
//!    keys (e.g. `PIXIU_SERVER__PORT=8080`).

use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};

use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

/// Environment variable naming an alternative config file.
pub const CONFIG_PATH_ENV: &str = "PIXIU_CONFIG";

const DEFAULT_CONFIG_FILE: &str = "pixiu.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub paths: PathsConfig,
    pub browser: BrowserConfig,
    pub hunt: HuntConfig,
    pub enrich: EnrichConfig,
    pub stream: StreamConfig,
    pub log: LogConfig,
}

/// The Chromium that platform logins run in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    /// The Chromium binary; found in `PATH` when unset.
    pub executable: Option<PathBuf>,
    /// Disables Chromium's sandbox, which containers usually cannot
    /// provide. The Docker image sets this.
    pub no_sandbox: bool,
    /// How many browsers may run at once, for every user's sign-ins and
    /// cookie refreshes together. Each takes a few hundred megabytes.
    pub max_open: usize,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            executable: None,
            no_sandbox: false,
            max_open: 2,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HuntConfig {
    /// `rustypipe-botguard`, which answers YouTube's proof-of-origin
    /// challenges. Not used when unset.
    pub botguard: Option<PathBuf>,
}

/// Looking music up on MusicBrainz, LRCLIB and Wikipedia.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnrichConfig {
    /// How the services can reach whoever runs this píxiū (an email
    /// address or a URL), sent in the User-Agent as MusicBrainz and
    /// Wikimedia ask.
    pub contact: Option<String>,
}

/// Streaming to Subsonic clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StreamConfig {
    /// What to transcode to when a client wants a lower bitrate but names
    /// no format, and the file's own format cannot be made.
    pub format: TranscodeFormat,
    /// Transcodes running at once, at most; more wait their turn.
    pub max_transcodes: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            format: TranscodeFormat::Mp3,
            max_transcodes: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscodeFormat {
    /// Plays everywhere.
    Mp3,
    /// The best quality for its size, but not every player takes it.
    Opus,
    Aac,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Address to bind. Defaults to loopback; the Docker image binds `0.0.0.0`.
    pub host: IpAddr,
    pub port: u16,
    /// Whether a reverse proxy in front of píxiū says who connected, in
    /// `X-Forwarded-For`. Only turn it on behind one: anyone could claim
    /// any address otherwise. Throttles count requests per address.
    pub trust_proxy_headers: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 4533,
            trust_proxy_headers: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    /// Database, secrets, the login browser profile, caches and staged
    /// offerings.
    pub data_dir: PathBuf,
    /// The managed music library. píxiū owns everything in here.
    pub treasure_dir: PathBuf,
    /// The web player's files (`index.html` and its assets). When unset,
    /// `web/` next to the binary, else `web/dist` (a checkout's build).
    pub web_dir: Option<PathBuf>,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            treasure_dir: PathBuf::from("treasure"),
            web_dir: None,
        }
    }
}

impl PathsConfig {
    #[must_use]
    pub fn database_file(&self) -> PathBuf {
        self.data_dir.join("pixiu.db")
    }

    /// The instance key sealing secrets at rest.
    #[must_use]
    pub fn secret_key_file(&self) -> PathBuf {
        self.data_dir.join("secret.key")
    }

    /// Uploaded offerings waiting for review.
    #[must_use]
    pub fn offerings_dir(&self) -> PathBuf {
        self.data_dir.join("offerings")
    }

    /// Derived files that can be regenerated (resized covers, ...).
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }

    /// The login browser's profile from before every user had their own
    /// (see [`users_dir`](Self::users_dir)); moved to the first account's.
    #[must_use]
    pub fn legacy_browser_profile_dir(&self) -> PathBuf {
        self.data_dir.join("browser-profile")
    }

    /// What each user keeps on the server: `<users>/<id>/browser-profile`
    /// (their login browser) and `<users>/<id>/youtube-music` (their
    /// client's sealed cache).
    #[must_use]
    pub fn users_dir(&self) -> PathBuf {
        self.data_dir.join("users")
    }

    /// The cache of the YouTube Music client without a login, which serves
    /// what every user shares (searches, albums, artists).
    #[must_use]
    pub fn youtube_music_dir(&self) -> PathBuf {
        self.data_dir.join("youtube-music")
    }

    /// Downloads in progress.
    #[must_use]
    pub fn staging_dir(&self) -> PathBuf {
        self.data_dir.join("staging")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    /// A `tracing` filter directive; `RUST_LOG` takes precedence when set.
    pub filter: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            filter: "info".to_owned(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file {0} does not exist")]
    MissingFile(PathBuf),
    #[error(transparent)]
    Invalid(#[from] Box<figment::Error>),
}

impl Config {
    /// Loads the configuration from the default sources.
    ///
    /// # Errors
    ///
    /// Fails when `PIXIU_CONFIG` names a missing file, or when a source holds
    /// a value of the wrong type.
    pub fn load() -> Result<Self, ConfigError> {
        let explicit = std::env::var_os(CONFIG_PATH_ENV).map(PathBuf::from);
        if let Some(path) = &explicit
            && !path.is_file()
        {
            return Err(ConfigError::MissingFile(path.clone()));
        }
        let file = explicit.unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
        Self::load_from(&file)
    }

    /// Loads the configuration using `file` (if it exists) and the
    /// environment.
    ///
    /// # Errors
    ///
    /// Fails when a source holds a value of the wrong type.
    pub fn load_from(file: &Path) -> Result<Self, ConfigError> {
        Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(file))
            .merge(Env::prefixed("PIXIU_").split("__"))
            .extract()
            .map_err(|error| ConfigError::Invalid(Box::new(error)))
    }
}

#[cfg(test)]
// `figment::Jail` closures must return `figment::Error`, which is large.
#[allow(clippy::result_large_err)]
mod tests {
    use figment::Jail;

    use super::*;

    #[test]
    fn defaults_apply_without_sources() {
        Jail::expect_with(|jail| {
            let config = Config::load_from(&jail.directory().join("missing.toml")).unwrap();
            assert_eq!(config, Config::default());
            assert_eq!(config.server.port, 4533);
            assert_eq!(config.paths.database_file(), PathBuf::from("data/pixiu.db"));
            Ok(())
        });
    }

    #[test]
    fn the_example_file_is_valid() {
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pixiu.example.toml");
        Jail::expect_with(|_| {
            let config = Config::load_from(&example).unwrap();
            assert_eq!(config.stream, StreamConfig::default());
            assert_eq!(config.log.filter, "info");
            Ok(())
        });
    }

    #[test]
    fn env_overrides_file() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "pixiu.toml",
                r#"
                [server]
                port = 9000

                [paths]
                treasure_dir = "/music"

                [stream]
                format = "opus"
                "#,
            )?;
            jail.set_env("PIXIU_SERVER__PORT", "9100");
            jail.set_env("PIXIU_SERVER__HOST", "0.0.0.0");

            let config = Config::load_from(&jail.directory().join("pixiu.toml")).unwrap();
            assert_eq!(config.server.port, 9100);
            assert_eq!(config.server.host, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            assert_eq!(config.paths.treasure_dir, PathBuf::from("/music"));
            assert_eq!(config.paths.data_dir, PathBuf::from("data"));
            assert_eq!(config.stream.format, TranscodeFormat::Opus);
            assert_eq!(config.stream.max_transcodes, 4);
            Ok(())
        });
    }

    #[test]
    fn wrong_types_are_rejected() {
        Jail::expect_with(|jail| {
            jail.set_env("PIXIU_SERVER__PORT", "not-a-port");
            assert!(Config::load_from(&jail.directory().join("pixiu.toml")).is_err());
            Ok(())
        });
    }
}
