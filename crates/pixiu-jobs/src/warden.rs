//! The session warden: keeps the YouTube Music login alive, and says so
//! when it cannot.
//!
//! Every [`VALIDATE_EVERY`] it asks the platform whether the session still
//! works. Every [`REFRESH_EVERY`], and at once when a check fails, it
//! revisits the platform in the headless browser with the persistent
//! profile, which rotates short-lived cookies, and stores the fresh ones.
//! When even that fails (a password change, "sign out everywhere"), the
//! session is marked expired and the web player asks the admin to log in again.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use jiff::Timestamp;
use pixiu_core::SecretBox;
use pixiu_db::{Db, SessionEvent, SessionEventKind, SessionState, SourceSession, now, toasty};
use pixiu_hunt::SessionCheck;
use tokio::sync::watch;

/// The platform the warden guards.
pub const SOURCE: &str = "youtube_music";

pub const VALIDATE_EVERY: Duration = Duration::from_secs(30 * 60);
pub const REFRESH_EVERY: Duration = Duration::from_secs(12 * 60 * 60);

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The platform side of a session: YouTube Music in production.
pub trait Platform: Send + Sync {
    /// Logs in with a `Cookie` header.
    fn apply<'a>(&'a self, cookies: &'a str) -> BoxFuture<'a, SessionCheck>;
    /// Asks whether the current login still works.
    fn check(&self) -> BoxFuture<'_, SessionCheck>;
    /// Logs out.
    fn forget(&self) -> BoxFuture<'_, ()>;
}

/// Produces fresh cookies from the persistent browser profile.
pub trait Refresher: Send + Sync {
    fn refresh(&self) -> BoxFuture<'_, Result<String, String>>;
}

/// What the web player shows about the session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Health {
    /// `None` until the admin connects an account.
    pub state: Option<SessionState>,
    pub connected_at: Option<Timestamp>,
    pub last_verified: Option<Timestamp>,
    pub last_refreshed: Option<Timestamp>,
    pub expired_at: Option<Timestamp>,
    pub last_error: Option<String>,
}

impl Health {
    fn of(session: Option<&SourceSession>) -> Self {
        session.map_or_else(Self::default, |session| Self {
            state: Some(session.state),
            connected_at: Some(session.connected_at),
            last_verified: session.last_verified,
            last_refreshed: session.last_refreshed,
            expired_at: session.expired_at,
            last_error: session.last_error.clone(),
        })
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.state == Some(SessionState::Expired)
    }
}

pub struct Warden {
    db: Db,
    secrets: SecretBox,
    platform: Box<dyn Platform>,
    refresher: Box<dyn Refresher>,
    health: watch::Sender<Health>,
    /// Checks and refreshes never overlap.
    lock: tokio::sync::Mutex<()>,
}

impl Warden {
    /// # Errors
    ///
    /// Fails when the stored session cannot be read.
    pub async fn new(
        db: Db,
        secrets: SecretBox,
        platform: Box<dyn Platform>,
        refresher: Box<dyn Refresher>,
    ) -> Result<Arc<Self>, toasty::Error> {
        let session = load(&mut db.clone()).await?;
        let (health, _) = watch::channel(Health::of(session.as_ref()));
        Ok(Arc::new(Self {
            db,
            secrets,
            platform,
            refresher,
            health,
            lock: tokio::sync::Mutex::new(()),
        }))
    }

    #[must_use]
    pub fn health(&self) -> Health {
        self.health.borrow().clone()
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Health> {
        self.health.subscribe()
    }

    /// The session cookies, for tools that need them directly (`yt-dlp`).
    pub async fn cookies(&self) -> Option<String> {
        let session = load(&mut self.db.clone()).await.ok()??;
        (session.state != SessionState::Expired)
            .then(|| self.secrets.open_str(&session.cookies).ok())
            .flatten()
    }

    /// Recent session history, newest first.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn events(&self, limit: usize) -> Result<Vec<SessionEvent>, toasty::Error> {
        let mut events = SessionEvent::filter_by_source(SOURCE)
            .exec(&mut self.db.clone())
            .await?;
        events.sort_by_key(|event| std::cmp::Reverse(event.id));
        events.truncate(limit);
        Ok(events)
    }

    /// Connects an account with cookies from the login browser.
    ///
    /// # Errors
    ///
    /// Fails when the platform rejects the cookies or on database errors;
    /// the message is meant for the admin.
    pub async fn connect(&self, cookies: String) -> Result<Health, String> {
        let _guard = self.lock.lock().await;
        match self.platform.apply(&cookies).await {
            SessionCheck::Valid => {}
            SessionCheck::Invalid(reason) | SessionCheck::Unreachable(reason) => {
                return Err(reason);
            }
        }
        let mut db = self.db.clone();
        let sealed = self.secrets.seal_str(&cookies);
        let result = async {
            match load(&mut db).await? {
                Some(mut session) => {
                    toasty::update!(session {
                        cookies: sealed,
                        state: SessionState::Valid,
                        connected_at: now(),
                        last_verified: Some(now()),
                        expired_at: Option::<Timestamp>::None,
                        last_error: Option::<String>::None,
                    })
                    .exec(&mut db)
                    .await?;
                }
                None => {
                    toasty::create!(SourceSession {
                        source: SOURCE,
                        cookies: sealed,
                        state: SessionState::Valid,
                        connected_at: now(),
                        last_verified: Some(now()),
                    })
                    .exec(&mut db)
                    .await?;
                }
            }
            record(&mut db, SessionEventKind::Connected, "Connected").await
        }
        .await;
        result.map_err(|error| error.to_string())?;
        Ok(self.publish().await)
    }

    /// Logs out and forgets the session.
    pub async fn disconnect(&self) {
        let _guard = self.lock.lock().await;
        self.platform.forget().await;
        let mut db = self.db.clone();
        if let Ok(Some(session)) = load(&mut db).await {
            let _ = session.delete().exec(&mut db).await;
            let _ = record(&mut db, SessionEventKind::Disconnected, "Disconnected").await;
        }
        self.publish().await;
    }

    /// Hands the stored cookies to the platform client, e.g. after a
    /// restart, then validates them.
    pub async fn restore(&self) -> Health {
        let Some(cookies) = self.cookies().await else {
            return self.publish().await;
        };
        let _guard = self.lock.lock().await;
        let check = self.platform.apply(&cookies).await;
        self.handle_check(check).await
    }

    /// Asks the platform whether the session still works.
    pub async fn validate(&self) -> Health {
        let _guard = self.lock.lock().await;
        if load(&mut self.db.clone()).await.ok().flatten().is_none() {
            return self.publish().await;
        }
        let check = self.platform.check().await;
        self.handle_check(check).await
    }

    /// Fetches fresh cookies from the browser profile.
    pub async fn refresh(&self) -> Health {
        let _guard = self.lock.lock().await;
        if load(&mut self.db.clone()).await.ok().flatten().is_none() {
            return self.publish().await;
        }
        self.refresh_locked(None).await
    }

    async fn handle_check(&self, check: SessionCheck) -> Health {
        match check {
            SessionCheck::Valid => self.mark_valid(false).await,
            SessionCheck::Unreachable(reason) => self.mark_degraded(&reason).await,
            SessionCheck::Invalid(reason) => self.refresh_locked(Some(reason)).await,
        }
    }

    /// `rejected` holds the reason when the platform just refused the
    /// session, in which case a failed refresh expires it.
    async fn refresh_locked(&self, rejected: Option<String>) -> Health {
        let fresh = match self.refresher.refresh().await {
            Ok(cookies) => cookies,
            Err(reason) => {
                return match rejected {
                    Some(rejected) => {
                        self.expire(&format!("{rejected}; refreshing failed: {reason}"))
                            .await
                    }
                    None => {
                        self.mark_degraded(&format!("refreshing failed: {reason}"))
                            .await
                    }
                };
            }
        };
        match self.platform.apply(&fresh).await {
            SessionCheck::Valid => {
                let mut db = self.db.clone();
                if let Ok(Some(mut session)) = load(&mut db).await {
                    let sealed = self.secrets.seal_str(&fresh);
                    let _ = toasty::update!(session {
                        cookies: sealed,
                        last_refreshed: Some(now()),
                    })
                    .exec(&mut db)
                    .await;
                }
                self.mark_valid(true).await
            }
            SessionCheck::Invalid(reason) => self.expire(&reason).await,
            SessionCheck::Unreachable(reason) => self.mark_degraded(&reason).await,
        }
    }

    async fn mark_valid(&self, refreshed: bool) -> Health {
        let mut db = self.db.clone();
        if let Ok(Some(mut session)) = load(&mut db).await {
            let recovered = session.state != SessionState::Valid;
            let _ = toasty::update!(session {
                state: SessionState::Valid,
                last_verified: Some(now()),
                expired_at: Option::<Timestamp>::None,
                last_error: Option::<String>::None,
            })
            .exec(&mut db)
            .await;
            if recovered {
                let _ = record(
                    &mut db,
                    SessionEventKind::Recovered,
                    "Session working again",
                )
                .await;
            } else if refreshed {
                let _ = record(&mut db, SessionEventKind::Refreshed, "Cookies refreshed").await;
            }
        }
        self.publish().await
    }

    async fn mark_degraded(&self, reason: &str) -> Health {
        let mut db = self.db.clone();
        if let Ok(Some(mut session)) = load(&mut db).await {
            // An expired session stays expired until the admin logs in.
            if session.state != SessionState::Expired {
                let worsened = session.state == SessionState::Valid;
                let _ = toasty::update!(session {
                    state: SessionState::Degraded,
                    last_error: Some(reason.to_owned()),
                })
                .exec(&mut db)
                .await;
                // Only the change is news; repeated failures are not.
                if worsened {
                    let _ = record(
                        &mut db,
                        SessionEventKind::Degraded,
                        &format!("Check failed: {reason}"),
                    )
                    .await;
                }
            }
        }
        tracing::warn!(reason, "YouTube Music session check inconclusive");
        self.publish().await
    }

    async fn expire(&self, reason: &str) -> Health {
        let mut db = self.db.clone();
        if let Ok(Some(mut session)) = load(&mut db).await
            && session.state != SessionState::Expired
        {
            let _ = toasty::update!(session {
                state: SessionState::Expired,
                expired_at: Some(now()),
                last_error: Some(reason.to_owned()),
            })
            .exec(&mut db)
            .await;
            let _ = record(
                &mut db,
                SessionEventKind::Expired,
                &format!("Session expired: {reason}"),
            )
            .await;
        }
        tracing::warn!(reason, "YouTube Music session expired");
        self.publish().await
    }

    async fn publish(&self) -> Health {
        let session = load(&mut self.db.clone()).await.ok().flatten();
        let health = Health::of(session.as_ref());
        self.health.send_replace(health.clone());
        health
    }

    /// Runs the warden: restores the session, then validates and refreshes
    /// it on schedule.
    pub fn start(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let warden = Arc::clone(self);
        tokio::spawn(async move {
            warden.restore().await;
            let mut since_refresh = Duration::ZERO;
            loop {
                // Spread checks out a little.
                let jitter = Duration::from_secs(rand::random_range(0..120));
                tokio::time::sleep(VALIDATE_EVERY + jitter).await;
                since_refresh += VALIDATE_EVERY + jitter;
                if since_refresh >= REFRESH_EVERY {
                    since_refresh = Duration::ZERO;
                    warden.refresh().await;
                } else {
                    warden.validate().await;
                }
            }
        })
    }
}

async fn load(db: &mut Db) -> Result<Option<SourceSession>, toasty::Error> {
    SourceSession::filter_by_source(SOURCE)
        .first()
        .exec(db)
        .await
}

async fn record(db: &mut Db, kind: SessionEventKind, message: &str) -> Result<(), toasty::Error> {
    toasty::create!(SessionEvent {
        source: SOURCE,
        kind,
        message,
        created_at: now(),
    })
    .exec(db)
    .await?;
    Ok(())
}
