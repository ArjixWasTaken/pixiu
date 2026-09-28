//! The job queue: downloads and other slow work, persisted so it survives
//! restarts, run a few at a time, with live progress for the WebUI.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use pixiu_db::{Db, Job, JobKind, JobState, now, toasty};
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, Semaphore, broadcast};

use crate::warden::BoxFuture;

/// How many jobs run at once.
pub const CONCURRENCY: usize = 3;

/// A track to download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackJob {
    pub video_id: String,
    /// What the download was requested for, e.g. an album's id.
    pub reference: Option<String>,
}

/// An album whose tracks to queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumJob {
    pub browse_id: String,
}

/// A job to create.
#[derive(Debug, Clone)]
pub struct NewJob {
    pub kind: JobKind,
    pub payload: String,
    pub title: String,
}

impl NewJob {
    #[must_use]
    pub fn track(video_id: &str, title: &str, reference: Option<String>) -> Self {
        Self {
            kind: JobKind::DownloadTrack,
            payload: serde_json::to_string(&TrackJob {
                video_id: video_id.to_owned(),
                reference,
            })
            .expect("job payloads serialize"),
            title: title.to_owned(),
        }
    }

    #[must_use]
    pub fn album(browse_id: &str, title: &str) -> Self {
        Self {
            kind: JobKind::GrabAlbum,
            payload: serde_json::to_string(&AlbumJob {
                browse_id: browse_id.to_owned(),
            })
            .expect("job payloads serialize"),
            title: title.to_owned(),
        }
    }
}

/// How a job ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done {
        track_id: Option<u64>,
    },
    /// Nothing to do: the result already exists.
    AlreadyDone {
        track_id: u64,
    },
    Failed(String),
    /// Done, and these jobs follow from it.
    Expand(Vec<NewJob>),
}

impl PartialEq for NewJob {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.payload == other.payload && self.title == other.title
    }
}

impl Eq for NewJob {}

/// Does the work of a job.
pub trait Executor: Send + Sync {
    fn run<'a>(
        &'a self,
        job: &'a Job,
        progress: &'a (dyn Fn(u8) + Send + Sync),
    ) -> BoxFuture<'a, Outcome>;
}

/// A change to show in the WebUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobUpdate {
    Changed {
        id: u64,
        state: JobState,
        progress: u8,
    },
    /// Finished jobs were forgotten.
    Cleared,
}

pub struct Jobs {
    db: Db,
    executor: Box<dyn Executor>,
    updates: broadcast::Sender<JobUpdate>,
    /// Progress of running jobs; only final states are written to the
    /// database.
    progress: Mutex<HashMap<u64, u8>>,
    wake: Notify,
    slots: Arc<Semaphore>,
}

impl Jobs {
    #[must_use]
    pub fn new(db: Db, executor: Box<dyn Executor>) -> Arc<Self> {
        Arc::new(Self {
            db,
            executor,
            updates: broadcast::channel(256).0,
            progress: Mutex::default(),
            wake: Notify::new(),
            slots: Arc::new(Semaphore::new(CONCURRENCY)),
        })
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<JobUpdate> {
        self.updates.subscribe()
    }

    /// The progress of a running job, in percent.
    #[must_use]
    pub fn progress(&self, id: u64) -> Option<u8> {
        self.progress.lock().unwrap().get(&id).copied()
    }

    /// Queues a job.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn enqueue(&self, job: NewJob) -> Result<Job, toasty::Error> {
        let created = toasty::create!(Job {
            kind: job.kind,
            payload: job.payload,
            title: job.title,
            state: JobState::Queued,
            progress: 0_u8,
            attempts: 0_u32,
            created_at: now(),
        })
        .exec(&mut self.db.clone())
        .await?;
        self.notify(created.id, JobState::Queued, 0);
        self.wake.notify_one();
        Ok(created)
    }

    /// The most recent jobs, newest first.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn recent(&self, limit: usize) -> Result<Vec<Job>, toasty::Error> {
        let mut jobs = Job::all()
            .order_by(Job::fields().id().desc())
            .limit(limit)
            .exec(&mut self.db.clone())
            .await?;
        jobs.truncate(limit);
        Ok(jobs)
    }

    /// Queues a failed job again.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn retry(&self, id: u64) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        if let Some(mut job) = Job::filter_by_id(id).first().exec(&mut db).await?
            && job.state == JobState::Failed
        {
            toasty::update!(job {
                state: JobState::Queued,
                progress: 0_u8,
                error: Option::<String>::None,
            })
            .exec(&mut db)
            .await?;
            self.notify(id, JobState::Queued, 0);
            self.wake.notify_one();
        }
        Ok(())
    }

    /// Forgets finished jobs.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn clear_finished(&self) -> Result<(), toasty::Error> {
        Job::filter_by_state(JobState::Done)
            .delete()
            .exec(&mut self.db.clone())
            .await?;
        let _ = self.updates.send(JobUpdate::Cleared);
        Ok(())
    }

    fn notify(&self, id: u64, state: JobState, progress: u8) {
        let _ = self.updates.send(JobUpdate::Changed {
            id,
            state,
            progress,
        });
    }

    fn set_progress(&self, id: u64, percent: u8) {
        let changed = {
            let mut all = self.progress.lock().unwrap();
            let previous = all.insert(id, percent);
            previous != Some(percent)
        };
        if changed {
            self.notify(id, JobState::Running, percent);
        }
    }

    /// Starts running queued jobs. Jobs a shutdown interrupted run again.
    pub fn start(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let jobs = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(error) = jobs.requeue_interrupted().await {
                tracing::error!(%error, "failed to requeue interrupted jobs");
            }
            loop {
                let Ok(permit) = Arc::clone(&jobs.slots).acquire_owned().await else {
                    break;
                };
                match jobs.claim_next().await {
                    Ok(Some(job)) => {
                        let jobs = Arc::clone(&jobs);
                        tokio::spawn(async move {
                            jobs.run(job).await;
                            drop(permit);
                            jobs.wake.notify_one();
                        });
                    }
                    Ok(None) => {
                        drop(permit);
                        // Wake up for new jobs, or look again now and then.
                        let _ = tokio::time::timeout(Duration::from_secs(60), jobs.wake.notified())
                            .await;
                    }
                    Err(error) => {
                        drop(permit);
                        tracing::error!(%error, "cannot read the job queue");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        })
    }

    async fn requeue_interrupted(&self) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        for mut job in Job::filter_by_state(JobState::Running)
            .exec(&mut db)
            .await?
        {
            toasty::update!(job {
                state: JobState::Queued,
                progress: 0_u8,
            })
            .exec(&mut db)
            .await?;
        }
        Ok(())
    }

    /// Takes the oldest queued job and marks it running.
    async fn claim_next(&self) -> Result<Option<Job>, toasty::Error> {
        let mut db = self.db.clone();
        let Some(mut job) = Job::filter_by_state(JobState::Queued)
            .order_by(Job::fields().id().asc())
            .first()
            .exec(&mut db)
            .await?
        else {
            return Ok(None);
        };
        toasty::update!(job {
            state: JobState::Running,
            attempts: job.attempts + 1,
            started_at: Some(now()),
        })
        .exec(&mut db)
        .await?;
        self.progress.lock().unwrap().insert(job.id, 0);
        self.notify(job.id, JobState::Running, 0);
        Ok(Some(job))
    }

    async fn run(&self, mut job: Job) {
        let id = job.id;
        let outcome = self
            .executor
            .run(&job, &|percent| self.set_progress(id, percent))
            .await;
        let (state, track_id, error) = match &outcome {
            Outcome::Done { track_id } => (JobState::Done, *track_id, None),
            Outcome::AlreadyDone { track_id } => (JobState::Done, Some(*track_id), None),
            Outcome::Expand(_) => (JobState::Done, None, None),
            Outcome::Failed(error) => {
                tracing::warn!(id, title = %job.title, %error, "job failed");
                (JobState::Failed, None, Some(error.clone()))
            }
        };
        let progress = if state == JobState::Done { 100 } else { 0 };
        let mut db = self.db.clone();
        let result = toasty::update!(job {
            state,
            progress,
            track_id,
            error,
            finished_at: Some(now()),
        })
        .exec(&mut db)
        .await;
        if let Err(error) = result {
            tracing::error!(%error, id, "cannot record a job's outcome");
        }
        self.progress.lock().unwrap().remove(&id);
        self.notify(id, state, progress);

        if let Outcome::Expand(follow_ups) = outcome {
            for follow_up in follow_ups {
                if let Err(error) = self.enqueue(follow_up).await {
                    tracing::error!(%error, "cannot queue a follow-up job");
                }
            }
        }
    }
}
