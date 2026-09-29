//! The job queue: downloads and other slow work, persisted so it survives
//! restarts, run a few at a time, with live progress for the web player.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use pixiu_db::{ClaimKind, Db, Job, JobKind, JobState, now, toasty};
use pixiu_treasury::Claim;
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, Semaphore, broadcast};

use crate::warden::BoxFuture;

/// How many jobs run at once.
pub const CONCURRENCY: usize = 3;

/// Who wants a download, which becomes the track's claim.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum Wanted {
    /// The admin grabbed it.
    #[default]
    Grab,
    /// A watched playlist holds it.
    Playlist { watch_id: u64 },
    /// A watched artist released it.
    Artist { watch_id: u64 },
}

impl Wanted {
    /// The claim the track gets; `reference` is kept for grabs.
    #[must_use]
    pub fn claim(self, reference: Option<String>) -> Claim {
        match self {
            Self::Grab => Claim {
                kind: ClaimKind::ManualGrab,
                reference,
            },
            Self::Playlist { watch_id } => Claim {
                kind: ClaimKind::WatchPlaylist,
                reference: Some(watch_id.to_string()),
            },
            Self::Artist { watch_id } => Claim {
                kind: ClaimKind::WatchArtist,
                reference: Some(watch_id.to_string()),
            },
        }
    }

    /// The watch that wants it, if one does.
    #[must_use]
    pub fn watch_id(self) -> Option<u64> {
        match self {
            Self::Grab => None,
            Self::Playlist { watch_id } | Self::Artist { watch_id } => Some(watch_id),
        }
    }
}

/// A track to download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackJob {
    pub video_id: String,
    /// What the download was requested for, e.g. an album's id.
    pub reference: Option<String>,
    #[serde(default)]
    pub wanted: Wanted,
}

/// An album whose tracks to queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumJob {
    pub browse_id: String,
    #[serde(default)]
    pub wanted: Wanted,
}

/// A watch to bring up to date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub watch_id: u64,
}

/// An album to look up on MusicBrainz, with its cover, lyrics and artist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichJob {
    pub album_id: u64,
    /// The release the admin picked; otherwise one is searched for.
    #[serde(default)]
    pub release: Option<String>,
    /// Search again, even if the album is matched already.
    #[serde(default)]
    pub fresh: bool,
}

/// What a job wants, from its payload: `None` for jobs that want nothing
/// (syncs) or cannot be read.
#[must_use]
pub fn wanted(job: &Job) -> Option<Wanted> {
    match job.kind {
        JobKind::DownloadTrack => serde_json::from_str::<TrackJob>(&job.payload)
            .ok()
            .map(|payload| payload.wanted),
        JobKind::GrabAlbum => serde_json::from_str::<AlbumJob>(&job.payload)
            .ok()
            .map(|payload| payload.wanted),
        JobKind::SyncWatch | JobKind::Enrich | JobKind::Refile => None,
    }
}

/// The album an enrich job is about.
#[must_use]
pub fn enriched_album(job: &Job) -> Option<u64> {
    (job.kind == JobKind::Enrich)
        .then(|| serde_json::from_str::<EnrichJob>(&job.payload).ok())
        .flatten()
        .map(|payload| payload.album_id)
}

/// How the jobs that follow from one job are doing: an album grab's
/// downloads.
#[derive(Debug, Clone, Copy, Default)]
pub struct Family {
    pub total: usize,
    pub done: usize,
    pub failed: usize,
    pub running: usize,
    pub waiting: usize,
    /// How far the running ones are, summed, in whole jobs.
    partial: f64,
}

impl Family {
    /// Whether some of the family has yet to finish.
    #[must_use]
    pub fn in_flight(&self) -> bool {
        self.running + self.waiting > 0
    }

    /// Overall completion, from 0 to 1.
    #[must_use]
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let total = self.total as f64;
        #[allow(clippy::cast_precision_loss)]
        let finished = (self.done + self.failed) as f64;
        ((finished + self.partial) / total).min(1.0)
    }
}

/// What unfinished jobs already fetch, by YouTube Music id.
#[derive(Debug, Default)]
pub struct Pending {
    /// Tracks to download, by video id.
    pub tracks: HashSet<String>,
    /// Albums to grab, by browse id.
    pub albums: HashSet<String>,
    /// Albums whose grab has become track downloads that are still under
    /// way, by browse id.
    pub album_tracks: HashSet<String>,
}

impl Pending {
    /// Whether the album is being grabbed, in either stage.
    #[must_use]
    pub fn has_album(&self, browse_id: &str) -> bool {
        self.albums.contains(browse_id) || self.album_tracks.contains(browse_id)
    }
}

/// What unfinished jobs, failed ones included, already fetch.
///
/// # Errors
///
/// Fails on database errors.
pub async fn pending(db: &mut Db) -> Result<Pending, toasty::Error> {
    let mut pending = Pending::default();
    for job in unfinished(db).await? {
        match job.kind {
            JobKind::DownloadTrack => {
                if let Ok(payload) = serde_json::from_str::<TrackJob>(&job.payload) {
                    pending.tracks.insert(payload.video_id);
                    if job.parent_id.is_some()
                        && let Some(album) = payload.reference
                    {
                        pending.album_tracks.insert(album);
                    }
                }
            }
            JobKind::GrabAlbum => {
                if let Ok(payload) = serde_json::from_str::<AlbumJob>(&job.payload) {
                    pending.albums.insert(payload.browse_id);
                }
            }
            JobKind::SyncWatch | JobKind::Enrich | JobKind::Refile => {}
        }
    }
    Ok(pending)
}

/// Jobs that have not finished: queued, running, paused or failed.
///
/// # Errors
///
/// Fails on database errors.
pub async fn unfinished(db: &mut Db) -> Result<Vec<Job>, toasty::Error> {
    let mut jobs = Vec::new();
    for state in [
        JobState::Queued,
        JobState::Running,
        JobState::Paused,
        JobState::Failed,
    ] {
        jobs.extend(Job::filter_by_state(state).exec(db).await?);
    }
    jobs.sort_by_key(|job| job.id);
    Ok(jobs)
}

/// A job to create.
#[derive(Debug, Clone)]
pub struct NewJob {
    pub kind: JobKind,
    pub payload: String,
    pub title: String,
    /// The job this one follows from; see [`Job::parent_id`].
    pub parent: Option<u64>,
}

impl NewJob {
    fn new(kind: JobKind, payload: &impl Serialize, title: &str) -> Self {
        Self {
            kind,
            payload: serde_json::to_string(payload).expect("job payloads serialize"),
            title: title.to_owned(),
            parent: None,
        }
    }

    /// The admin grabs a track.
    #[must_use]
    pub fn track(video_id: &str, title: &str, reference: Option<String>) -> Self {
        Self::wanted_track(video_id, title, reference, Wanted::Grab)
    }

    #[must_use]
    pub fn wanted_track(
        video_id: &str,
        title: &str,
        reference: Option<String>,
        wanted: Wanted,
    ) -> Self {
        let payload = TrackJob {
            video_id: video_id.to_owned(),
            reference,
            wanted,
        };
        Self::new(JobKind::DownloadTrack, &payload, title)
    }

    /// The admin grabs an album.
    #[must_use]
    pub fn album(browse_id: &str, title: &str) -> Self {
        Self::wanted_album(browse_id, title, Wanted::Grab)
    }

    #[must_use]
    pub fn wanted_album(browse_id: &str, title: &str, wanted: Wanted) -> Self {
        let payload = AlbumJob {
            browse_id: browse_id.to_owned(),
            wanted,
        };
        Self::new(JobKind::GrabAlbum, &payload, title)
    }

    #[must_use]
    pub fn sync(watch_id: u64, title: &str) -> Self {
        Self::new(JobKind::SyncWatch, &SyncJob { watch_id }, title)
    }

    #[must_use]
    pub fn enrich(album_id: u64, title: &str, release: Option<String>, fresh: bool) -> Self {
        let payload = EnrichJob {
            album_id,
            release,
            fresh,
        };
        Self::new(JobKind::Enrich, &payload, title)
    }

    /// Moving every file to the current layout.
    #[must_use]
    pub fn refile() -> Self {
        Self::new(
            JobKind::Refile,
            &serde_json::Map::new(),
            "Move files to the new layout",
        )
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
    /// Done with a result, and these jobs follow from it.
    DoneWith {
        track_id: Option<u64>,
        then: Vec<NewJob>,
    },
    /// Cannot run until something outside píxiū changes, like a platform
    /// login; see [`Jobs::resume_paused`].
    Paused(String),
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

/// A change to show in the web player.
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
            parent_id: job.parent,
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

    /// How the jobs that follow from each of `parents` are doing.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn families(&self, parents: &[u64]) -> Result<HashMap<u64, Family>, toasty::Error> {
        let mut families: HashMap<u64, Family> = HashMap::new();
        if parents.is_empty() {
            return Ok(families);
        }
        let children = Job::filter(
            Job::fields()
                .parent_id()
                .in_list(parents.iter().map(|id| Some(*id)).collect::<Vec<_>>()),
        )
        .exec(&mut self.db.clone())
        .await?;
        for child in children {
            let Some(parent) = child.parent_id else {
                continue;
            };
            let family = families.entry(parent).or_default();
            family.total += 1;
            match child.state {
                JobState::Done => family.done += 1,
                JobState::Failed => family.failed += 1,
                JobState::Running => {
                    family.running += 1;
                    family.partial += f64::from(self.progress(child.id).unwrap_or(0)) / 100.0;
                }
                JobState::Queued | JobState::Paused => family.waiting += 1,
            }
        }
        Ok(families)
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

    /// Queues paused jobs again, e.g. once a platform login works again.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn resume_paused(&self) -> Result<usize, toasty::Error> {
        let mut db = self.db.clone();
        let paused = Job::filter_by_state(JobState::Paused).exec(&mut db).await?;
        let count = paused.len();
        for mut job in paused {
            let id = job.id;
            toasty::update!(job {
                state: JobState::Queued,
                error: Option::<String>::None,
            })
            .exec(&mut db)
            .await?;
            self.notify(id, JobState::Queued, 0);
        }
        if count > 0 {
            self.wake.notify_one();
        }
        Ok(count)
    }

    /// See [`unfinished`].
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn unfinished(&self) -> Result<Vec<Job>, toasty::Error> {
        unfinished(&mut self.db.clone()).await
    }

    /// Forgets the jobs `unwanted` picks among those not running, e.g. the
    /// work of a watch being removed.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn forget(&self, unwanted: impl Fn(&Job) -> bool) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        for job in self.unfinished().await? {
            if job.state != JobState::Running && unwanted(&job) {
                job.delete().exec(&mut db).await?;
            }
        }
        let _ = self.updates.send(JobUpdate::Cleared);
        Ok(())
    }

    /// Forgets finished jobs.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    ///
    /// A family (an album grab and its downloads) is cleared only once all of
    /// it has finished, so its progress stays whole while it runs.
    pub async fn clear_finished(&self) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        let in_flight: HashSet<u64> = unfinished(&mut db)
            .await?
            .into_iter()
            .filter_map(|job| job.parent_id)
            .collect();
        for job in Job::filter_by_state(JobState::Done).exec(&mut db).await? {
            let busy = in_flight.contains(&job.id)
                || job
                    .parent_id
                    .is_some_and(|parent| in_flight.contains(&parent));
            if !busy {
                job.delete().exec(&mut db).await?;
            }
        }
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
            Outcome::DoneWith { track_id, .. } => (JobState::Done, *track_id, None),
            Outcome::Failed(error) => {
                tracing::warn!(id, title = %job.title, %error, "job failed");
                (JobState::Failed, None, Some(error.clone()))
            }
            Outcome::Paused(reason) => {
                tracing::info!(id, title = %job.title, %reason, "job paused");
                (JobState::Paused, None, Some(reason.clone()))
            }
        };
        let progress = if state == JobState::Done { 100 } else { 0 };
        let finished_at = (state != JobState::Paused).then(now);
        let mut db = self.db.clone();
        let result = toasty::update!(job {
            state,
            progress,
            track_id,
            error,
            finished_at,
        })
        .exec(&mut db)
        .await;
        if let Err(error) = result {
            tracing::error!(%error, id, "cannot record a job's outcome");
        }
        self.progress.lock().unwrap().remove(&id);
        self.notify(id, state, progress);

        // An expansion's jobs are its children; see `Jobs::families`.
        let parent = matches!(outcome, Outcome::Expand(_)).then_some(id);
        if let Outcome::Expand(follow_ups)
        | Outcome::DoneWith {
            then: follow_ups, ..
        } = outcome
        {
            for mut follow_up in follow_ups {
                follow_up.parent = follow_up.parent.or(parent);
                if let Err(error) = self.enqueue(follow_up).await {
                    tracing::error!(%error, "cannot queue a follow-up job");
                }
            }
        }
    }
}
