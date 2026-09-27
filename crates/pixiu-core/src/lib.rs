//! Configuration and shared types for píxiū.

pub mod config;

pub use config::{Config, ConfigError, CookieSecurity};

/// The version reported to Subsonic clients and shown in the WebUI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
