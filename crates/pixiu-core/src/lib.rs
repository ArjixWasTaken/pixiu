//! Configuration and shared types for píxiū.

pub mod config;
pub mod password;
pub mod secrets;

pub use config::{Config, ConfigError, CookieSecurity};
pub use secrets::{SecretBox, SecretError};

/// The version reported to Subsonic clients and shown in the WebUI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
