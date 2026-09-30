//! Every user's session warden. Each user with a YouTube Music session has
//! a warden of their own, keeping their login alive with their own client
//! and browser profile; wardens of users without one are made when first
//! asked about, e.g. when they connect an account.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use pixiu_core::SecretBox;
use pixiu_db::{Db, SourceSession, toasty};
use tokio::task::JoinHandle;

use crate::{
    Jobs,
    warden::{BoxFuture, Health, Platform, Refresher, SOURCE, Warden},
    watch,
};

/// What a user's warden works with.
pub trait SessionFactory: Send + Sync {
    /// The user's platform client.
    fn platform(&self, owner: u64) -> Box<dyn Platform>;
    /// The user's browser profile, for fresh cookies.
    fn refresher(&self, owner: u64) -> Box<dyn Refresher>;
    /// Lets go of what the user's warden worked with (their client and
    /// login browser).
    fn forget(&self, owner: u64) -> BoxFuture<'_, ()>;
}

struct Running {
    warden: Arc<Warden>,
    tasks: Vec<JoinHandle<()>>,
}

pub struct Wardens {
    db: Db,
    secrets: SecretBox,
    factory: Box<dyn SessionFactory>,
    wardens: tokio::sync::Mutex<HashMap<u64, Running>>,
    /// Once started, new wardens run at once and resume their owner's
    /// paused jobs when their login works again.
    jobs: OnceLock<Arc<Jobs>>,
    /// Health of wardens already made, readable without waiting.
    health: Mutex<HashMap<u64, tokio::sync::watch::Receiver<Health>>>,
}

impl Wardens {
    #[must_use]
    pub fn new(db: Db, secrets: SecretBox, factory: Box<dyn SessionFactory>) -> Arc<Self> {
        Arc::new(Self {
            db,
            secrets,
            factory,
            wardens: tokio::sync::Mutex::default(),
            jobs: OnceLock::new(),
            health: Mutex::default(),
        })
    }

    /// `owner`'s warden, made (and, once started, run) on first use.
    ///
    /// # Errors
    ///
    /// Fails when the stored session cannot be read.
    pub async fn get(&self, owner: u64) -> Result<Arc<Warden>, toasty::Error> {
        let mut wardens = self.wardens.lock().await;
        if let Some(running) = wardens.get(&owner) {
            return Ok(Arc::clone(&running.warden));
        }
        let warden = Warden::new(
            self.db.clone(),
            self.secrets.clone(),
            owner,
            self.factory.platform(owner),
            self.factory.refresher(owner),
        )
        .await?;
        let tasks = match self.jobs.get() {
            Some(jobs) => run(&warden, jobs),
            None => Vec::new(),
        };
        self.health
            .lock()
            .unwrap()
            .insert(owner, warden.subscribe());
        wardens.insert(
            owner,
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
        let owners: Vec<u64> = SourceSession::all()
            .exec(&mut self.db.clone())
            .await?
            .into_iter()
            .filter(|session| session.source == SOURCE)
            .map(|session| session.user_id)
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
        for owner in owners {
            self.get(owner).await?;
        }
        Ok(())
    }

    /// `owner`'s session health; nothing when they never connected one.
    #[must_use]
    pub fn health(&self, owner: u64) -> Health {
        self.health
            .lock()
            .unwrap()
            .get(&owner)
            .map(|health| health.borrow().clone())
            .unwrap_or_default()
    }

    /// `owner`'s session cookies, for tools that need them directly
    /// (`yt-dlp`).
    pub async fn cookies(&self, owner: u64) -> Option<String> {
        self.get(owner).await.ok()?.cookies().await
    }

    /// Stops `owner`'s warden and lets go of their client and browser.
    /// Their stored session stays unless disconnected first.
    pub async fn stop(&self, owner: u64) {
        let running = self.wardens.lock().await.remove(&owner);
        self.health.lock().unwrap().remove(&owner);
        if let Some(running) = running {
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
