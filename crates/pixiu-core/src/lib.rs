//! Configuration and shared types for píxiū.

pub mod alerts;
pub mod config;
pub mod password;
pub mod playing;
pub mod secrets;

pub use config::{Config, ConfigError, StreamConfig, TranscodeFormat};
pub use secrets::{SecretBox, SecretError};

/// The version reported to Subsonic clients and shown in the web player.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
