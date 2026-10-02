//! Every user's session wardens. Each session a user has with a platform
//! has a warden of its own, keeping their login alive with their own client
//! and browser profile; wardens of sessions not connected yet are made when
//! first asked about, e.g. when they connect an account.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use pixiu_core::{SecretBox, alerts::AlertSink};
use pixiu_db::{Db, Platform, SourceSession, toasty};
use tokio::task::JoinHandle;

use crate::{
    Jobs,
    warden::{BoxFuture, Health, Refresher, Session, Warden},
    watch,
};

/// What a user's wardens work with.
pub trait SessionFactory: Send + Sync {
    /// The user's client of a platform.
    fn session(&self, owner: u64, platform: Platform) -> Box<dyn Session>;
    /// The user's browser profile, for fresh cookies from a platform.
    fn refresher(&self, owner: u64, platform: Platform) -> Box<dyn Refresher>;
    /// Lets go of what the user's wardens worked with (their clients and
    /// login browser).
    fn forget(&self, owner: u64) -> BoxFuture<'_, ()>;
}

/// A user's session with a platform.
type SessionOf = (u64, Platform);

struct Running {
    warden: Arc<Warden>,
    tasks: Vec<JoinHandle<()>>,
}

pub struct Wardens {
    db: Db,
    secrets: SecretBox,
    factory: Box<dyn SessionFactory>,
    alerts: Arc<dyn AlertSink>,
    wardens: tokio::sync::Mutex<HashMap<SessionOf, Running>>,
    /// Once started, new wardens run at once and resume their owner's
    /// paused jobs when their login works again.
    jobs: OnceLock<Arc<Jobs>>,
    /// Health of wardens already made, readable without waiting.
    health: Mutex<HashMap<SessionOf, tokio::sync::watch::Receiver<Health>>>,
}

impl Wardens {
    #[must_use]
    pub fn new(
        db: Db,
        secrets: SecretBox,
        factory: Box<dyn SessionFactory>,
        alerts: Arc<dyn AlertSink>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            secrets,
            factory,
            alerts,
            wardens: tokio::sync::Mutex::default(),
            jobs: OnceLock::new(),
            health: Mutex::default(),
        })
    }

    /// `owner`'s warden of their session with `platform`, made (and, once
    /// started, run) on first use.
    ///
    /// # Errors
    ///
    /// Fails when the stored session cannot be read.
    pub async fn get(&self, owner: u64, platform: Platform) -> Result<Arc<Warden>, toasty::Error> {
        let mut wardens = self.wardens.lock().await;
        if let Some(running) = wardens.get(&(owner, platform)) {
            return Ok(Arc::clone(&running.warden));
        }
        let warden = Warden::new(
            self.db.clone(),
            self.secrets.clone(),
            owner,
            platform,
            self.factory.session(owner, platform),
            self.factory.refresher(owner, platform),
            Arc::clone(&self.alerts),
        )
        .await?;
        let tasks = match self.jobs.get() {
            Some(jobs) => run(&warden, jobs),
            None => Vec::new(),
        };
        self.health
            .lock()
            .unwrap()
            .insert((owner, platform), warden.subscribe());
        wardens.insert(
            (owner, platform),
            Running {
                warden: Arc::clone(&warden),
                tasks,
            },
        );
        Ok(warden)
    }

    /// Runs the wardens of everyone with a stored session, and every
    /// warden made from now on.
    ///
    /// # Errors
    ///
    /// Fails when the stored sessions cannot be read.
    pub async fn start(&self, jobs: Arc<Jobs>) -> Result<(), toasty::Error> {
        let sessions: Vec<SessionOf> = SourceSession::all()
            .exec(&mut self.db.clone())
            .await?
            .into_iter()
            .filter_map(|session| Some((session.user_id, Platform::from_name(&session.source)?)))
            .collect();
        {
            let mut wardens = self.wardens.lock().await;
            for running in wardens.values_mut() {
                if running.tasks.is_empty() {
                    running.tasks = run(&running.warden, &jobs);
                }
            }
        }
        let _ = self.jobs.set(jobs);
        for (owner, platform) in sessions {
            self.get(owner, platform).await?;
        }
        Ok(())
    }

    /// The health of `owner`'s session with `platform`; nothing when they
    /// never connected one.
    #[must_use]
    pub fn health(&self, owner: u64, platform: Platform) -> Health {
        self.health
            .lock()
            .unwrap()
            .get(&(owner, platform))
            .map(|health| health.borrow().clone())
            .unwrap_or_default()
    }

    /// The cookies of `owner`'s session with `platform`, for tools that
    /// need them directly (`yt-dlp`).
    pub async fn cookies(&self, owner: u64, platform: Platform) -> Option<String> {
        self.get(owner, platform).await.ok()?.cookies().await
    }

    /// Stops `owner`'s wardens and lets go of their clients and browser.
    /// Their stored sessions stay unless disconnected first.
    pub async fn stop(&self, owner: u64) {
        let stopped: Vec<Running> = {
            let mut wardens = self.wardens.lock().await;
            let theirs: Vec<SessionOf> = wardens
                .keys()
                .filter(|(user, _)| *user == owner)
                .copied()
                .collect();
            theirs
                .iter()
                .filter_map(|session| wardens.remove(session))
                .collect()
        };
        self.health
            .lock()
            .unwrap()
            .retain(|(user, _), _| *user != owner);
        for running in stopped {
            for task in running.tasks {
                task.abort();
            }
        }
        self.factory.forget(owner).await;
    }
}

/// Starts a warden and resumes its owner's paused work whenever their
/// login works again.
fn run(warden: &Arc<Warden>, jobs: &Arc<Jobs>) -> Vec<JoinHandle<()>> {
    vec![
        warden.start(),
        watch::resume_on_login(warden, Arc::clone(jobs)),
    ]
}
