//! Database models.
//!
//! Changing anything here requires a new migration; see `pixiu-migrate`.

use jiff::Timestamp;

/// The admin account. píxiū is single-user, so exactly one row exists once
/// the first-run setup has completed.
#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub username: String,

    /// PHC-formatted argon2id hash of the WebUI password.
    pub password_hash: String,

    pub created_at: Timestamp,

    #[has_many]
    pub web_sessions: toasty::Deferred<Vec<WebSession>>,
}

/// A logged-in WebUI session. Only the SHA-256 of the session token is
/// stored, so a leaked database cannot be replayed as cookies.
#[derive(Debug, toasty::Model)]
pub struct WebSession {
    #[key]
    #[auto]
    pub id: u64,

    /// Hex-encoded SHA-256 of the session token.
    #[unique]
    pub token_hash: String,

    #[index]
    pub user_id: u64,

    #[belongs_to]
    pub user: toasty::Deferred<User>,

    pub created_at: Timestamp,

    pub expires_at: Timestamp,
}
