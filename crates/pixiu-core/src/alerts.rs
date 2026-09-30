//! Alerts: news a user should hear about while away from the player, such
//! as their YouTube Music session expiring. Whoever notices raises one; an
//! [`AlertSink`] delivers it (by email) to the user it concerns, once.

use std::{future::Future, pin::Pin};

use jiff::Timestamp;

/// Something a user should hear about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alert {
    /// YouTube Music no longer accepts the user's session, and refreshing
    /// it failed: they must sign in again.
    YouTubeMusicExpired {
        /// When it expired; one alert per expiry.
        expired_at: Timestamp,
        reason: String,
    },
    /// A watch failed to sync several times in a row.
    WatchFailing {
        watch_id: u64,
        name: String,
        error: String,
        /// When the first of the failures was; one alert per streak.
        since: Timestamp,
    },
}

impl Alert {
    /// The kind of alert, which users switch on or off.
    #[must_use]
    pub fn kind(&self) -> AlertKind {
        match self {
            Self::YouTubeMusicExpired { .. } => AlertKind::YouTubeMusicExpired,
            Self::WatchFailing { .. } => AlertKind::WatchFailing,
        }
    }

    /// Tells repeats of the same alert apart from new ones.
    #[must_use]
    pub fn dedupe_key(&self) -> String {
        match self {
            Self::YouTubeMusicExpired { expired_at, .. } => {
                format!("youtube-music/expired/{expired_at}")
            }
            Self::WatchFailing {
                watch_id, since, ..
            } => format!("watch/{watch_id}/failing/{since}"),
        }
    }
}

/// The kinds of alert, which users switch on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertKind {
    YouTubeMusicExpired,
    WatchFailing,
}

impl AlertKind {
    pub const ALL: [Self; 2] = [Self::YouTubeMusicExpired, Self::WatchFailing];

    /// The name the player and the stored settings use.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::YouTubeMusicExpired => "youtube_music_expired",
            Self::WatchFailing => "watch_failing",
        }
    }
}

/// Delivers alerts.
pub trait AlertSink: Send + Sync {
    /// Tells `user` about `alert`, unless they heard about it already or do
    /// not want to. Never fails: problems are logged.
    fn alert(&self, user: u64, alert: Alert) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// Drops every alert: for tests, and wherever nobody listens.
pub struct NoAlerts;

impl AlertSink for NoAlerts {
    fn alert(&self, _user: u64, _alert: Alert) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}
