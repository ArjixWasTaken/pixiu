//! Watches: playlists and liked music mirrored into Subsonic playlists, and
//! artists whose releases are grabbed as they appear.
//!
//! Syncing is one-way. What a watch brings in is claimed by it. When a
//! track leaves a watched playlist, the mirror follows and the claim goes;
//! the file stays, an orphan unless something else claims it.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use jiff::{SignedDuration, Timestamp};
use pixiu_db::{
    ClaimKind, Db, Job, JobKind, JobState, Playlist, PlaylistEntry, SessionState, Track,
    TrackClaim, Watch, WatchKind, now, toasty,
};
use pixiu_hunt::{AlbumKind, Discography, LIKED_MUSIC, RemotePlaylist};
use pixiu_treasury::Treasury;

use crate::{
    queue::{self, AlbumJob, Jobs, NewJob, SyncJob, TrackJob, Wanted},
    warden::{BoxFuture, Warden},
};

/// What a paused liked-music sync waits for.
pub const WAITING_FOR_LOGIN: &str = "Waiting for a YouTube Music login";

/// How often the scheduler looks for watches that are due.
const TICK: Duration = Duration::from_secs(60);

/// Why the platform could not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// Only a logged-in session may see this.
    NeedsLogin(String),
    Failed(String),
}

/// The platform side of watches.
pub trait Catalog: Send + Sync {
    /// A playlist with all its tracks; [`LIKED_MUSIC`] for liked music.
    fn playlist<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemotePlaylist, CatalogError>>;

    fn discography<'a>(
        &'a self,
        channel_id: &'a str,
    ) -> BoxFuture<'a, Result<Discography, CatalogError>>;

    /// Whether a working login is at hand; liked music needs one.
    fn logged_in(&self) -> bool;
}

/// How often each kind of watch is synced.
#[must_use]
pub fn default_interval(kind: WatchKind) -> Duration {
    Duration::from_secs(match kind {
        WatchKind::LikedMusic => 30 * 60,
        WatchKind::Playlist => 6 * 60 * 60,
        WatchKind::Artist => 24 * 60 * 60,
    })
}

/// What a sync concluded.
#[derive(Debug, PartialEq, Eq)]
pub enum Synced {
    /// Up to date; these downloads and grabs follow.
    Done(Vec<NewJob>),
    /// Waiting for a platform login.
    NeedsLogin(String),
}

fn failed(error: impl ToString) -> String {
    error.to_string()
}

/// Brings a watch up to date: a mirror follows its playlist, claims follow
/// the mirror, and what the hoard lacks is queued. The watch records how
/// it went.
///
/// # Errors
///
/// Fails when the platform cannot answer or on database errors.
pub async fn sync(
    treasury: &Treasury,
    catalog: &dyn Catalog,
    watch_id: u64,
) -> Result<Synced, String> {
    let mut db = treasury.db();
    let Some(watch) = Watch::filter_by_id(watch_id)
        .first()
        .exec(&mut db)
        .await
        .map_err(failed)?
    else {
        // Removed since the sync was queued.
        return Ok(Synced::Done(Vec::new()));
    };
    let result = match watch.kind {
        WatchKind::Playlist | WatchKind::LikedMusic => {
            sync_playlist(treasury, catalog, &watch).await
        }
        WatchKind::Artist => sync_artist(&mut db, catalog, &watch).await,
    };
    record(&mut db, watch_id, &result).await;
    result
}

async fn record(db: &mut Db, watch_id: u64, result: &Result<Synced, String>) {
    let Ok(Some(mut watch)) = Watch::filter_by_id(watch_id).first().exec(db).await else {
        return;
    };
    let (last_synced_at, last_error) = match result {
        Ok(Synced::Done(_)) => (Some(now()), None),
        Ok(Synced::NeedsLogin(reason)) | Err(reason) => {
            (watch.last_synced_at, Some(reason.clone()))
        }
    };
    let updated = toasty::update!(watch {
        last_synced_at,
        last_error,
    })
    .exec(db)
    .await;
    if let Err(error) = updated {
        tracing::error!(%error, watch_id, "cannot record a watch sync");
    }
}

async fn sync_playlist(
    treasury: &Treasury,
    catalog: &dyn Catalog,
    watch: &Watch,
) -> Result<Synced, String> {
    let liked = watch.kind == WatchKind::LikedMusic;
    if liked && !catalog.logged_in() {
        return Ok(Synced::NeedsLogin(WAITING_FOR_LOGIN.to_owned()));
    }
    let remote_id = if liked {
        LIKED_MUSIC
    } else {
        watch.remote_id.as_str()
    };
    let playlist = match catalog.playlist(remote_id).await {
        Ok(playlist) => playlist,
        Err(CatalogError::NeedsLogin(reason)) => return Ok(Synced::NeedsLogin(reason)),
        Err(CatalogError::Failed(reason)) => return Err(reason),
    };
    let name = if liked {
        "Liked music".to_owned()
    } else {
        playlist.name.clone()
    };
    let mut db = treasury.db();
    rename(&mut db, watch.id, &name).await.map_err(failed)?;

    // The mirror follows the playlist, order included.
    let order: Vec<String> = playlist
        .tracks
        .iter()
        .map(|track| track.id.clone())
        .collect();
    let mirror = mirror(&mut db, watch.id, &name).await.map_err(failed)?;
    set_entries(&mut db, mirror, &order).await.map_err(failed)?;

    // Tracks that left lose this watch's claim ...
    let listed: HashSet<&str> = order.iter().map(String::as_str).collect();
    let reference = watch.id.to_string();
    treasury
        .release(ClaimKind::WatchPlaylist, &reference, |track| {
            track
                .ytm_video_id
                .as_deref()
                .is_some_and(|id| listed.contains(id))
        })
        .await
        .map_err(failed)?;

    // ... those in the hoard get it ...
    let wanted = Wanted::Playlist { watch_id: watch.id };
    let hoarded = hoarded(&mut db, &order).await.map_err(failed)?;
    let claimed = claimed(&mut db, ClaimKind::WatchPlaylist, &reference)
        .await
        .map_err(failed)?;
    for &track_id in hoarded.values() {
        if !claimed.contains(&track_id) {
            treasury
                .claim(track_id, &wanted.claim(None))
                .await
                .map_err(failed)?;
        }
    }

    // ... and the rest is downloaded, unless it already is being.
    let pending = pending(&mut db).await.map_err(failed)?;
    let mut queued = HashSet::new();
    let jobs = playlist
        .tracks
        .iter()
        .filter(|track| {
            !hoarded.contains_key(&track.id)
                && !pending.tracks.contains(&track.id)
                && queued.insert(track.id.clone())
        })
        .map(|track| {
            let title = format!("{} — {}", track.artist_credit(), track.title);
            NewJob::wanted_track(&track.id, &title, None, wanted)
        })
        .collect();
    Ok(Synced::Done(jobs))
}

async fn sync_artist(db: &mut Db, catalog: &dyn Catalog, watch: &Watch) -> Result<Synced, String> {
    let discography = match catalog.discography(&watch.remote_id).await {
        Ok(discography) => discography,
        Err(CatalogError::NeedsLogin(reason)) => return Ok(Synced::NeedsLogin(reason)),
        Err(CatalogError::Failed(reason)) => return Err(reason),
    };
    let wanted = Wanted::Artist { watch_id: watch.id };
    // A watch of new releases only takes note of the old ones at first.
    let skip_old = watch.only_new && watch.last_synced_at.is_none();
    let known: HashSet<&str> = watch.seen.iter().map(String::as_str).collect();
    let pending = pending(db).await.map_err(failed)?;
    let mut seen = watch.seen.clone();
    let mut jobs = Vec::new();
    for album in &discography.albums {
        let wanted_kind = match album.kind {
            AlbumKind::Album | AlbumKind::Other => true,
            AlbumKind::Ep | AlbumKind::Single => watch.include_singles,
        };
        if !wanted_kind || known.contains(album.id.as_str()) || seen.contains(&album.id) {
            continue;
        }
        seen.push(album.id.clone());
        if skip_old || pending.albums.contains(&album.id) {
            continue;
        }
        let title = format!("{} — {}", album.artists.join(", "), album.title);
        jobs.push(NewJob::wanted_album(&album.id, &title, wanted));
    }

    if let Some(mut watch) = Watch::filter_by_id(watch.id)
        .first()
        .exec(db)
        .await
        .map_err(failed)?
    {
        toasty::update!(watch {
            name: discography.name,
            seen,
        })
        .exec(db)
        .await
        .map_err(failed)?;
    }
    Ok(Synced::Done(jobs))
}

async fn rename(db: &mut Db, watch_id: u64, name: &str) -> Result<(), toasty::Error> {
    if let Some(mut watch) = Watch::filter_by_id(watch_id).first().exec(db).await?
        && watch.name != name
    {
        toasty::update!(watch { name }).exec(db).await?;
    }
    Ok(())
}

/// The playlist mirroring a watch, made when missing.
async fn mirror(db: &mut Db, watch_id: u64, name: &str) -> Result<u64, toasty::Error> {
    match Playlist::filter_by_watch_id(Some(watch_id))
        .first()
        .exec(db)
        .await?
    {
        Some(mut playlist) => {
            if playlist.name != name {
                toasty::update!(playlist {
                    name,
                    changed_at: now(),
                })
                .exec(db)
                .await?;
            }
            Ok(playlist.id)
        }
        None => Ok(toasty::create!(Playlist {
            name,
            public: false,
            watch_id: Some(watch_id),
            created_at: now(),
            changed_at: now(),
        })
        .exec(db)
        .await?
        .id),
    }
}

/// Makes the mirror list exactly `order`, when it does not already.
async fn set_entries(db: &mut Db, playlist_id: u64, order: &[String]) -> Result<(), toasty::Error> {
    let mut entries = PlaylistEntry::filter_by_playlist_id(playlist_id)
        .exec(db)
        .await?;
    entries.sort_by_key(|entry| entry.position);
    let current: Vec<&str> = entries
        .iter()
        .filter_map(|entry| entry.ytm_video_id.as_deref())
        .collect();
    if current.len() == entries.len() && current.iter().eq(order.iter()) {
        return Ok(());
    }

    let mut tx = db.transaction().await?;
    PlaylistEntry::filter_by_playlist_id(playlist_id)
        .delete()
        .exec(&mut tx)
        .await?;
    for (position, video_id) in order.iter().enumerate() {
        toasty::create!(PlaylistEntry {
            playlist_id,
            position: u32::try_from(position).unwrap_or(u32::MAX),
            ytm_video_id: Some(video_id.clone()),
        })
        .exec(&mut tx)
        .await?;
    }
    if let Some(mut playlist) = Playlist::filter_by_id(playlist_id)
        .first()
        .exec(&mut tx)
        .await?
    {
        toasty::update!(playlist { changed_at: now() })
            .exec(&mut tx)
            .await?;
    }
    tx.commit().await
}

/// The tracks the hoard holds among these videos, by video id.
async fn hoarded(db: &mut Db, video_ids: &[String]) -> Result<HashMap<String, u64>, toasty::Error> {
    let mut hoarded = HashMap::new();
    for chunk in video_ids.chunks(500) {
        let tracks = Track::filter(Track::fields().ytm_video_id().in_list(chunk.to_vec()))
            .exec(db)
            .await?;
        for track in tracks {
            if let Some(video_id) = track.ytm_video_id {
                hoarded.insert(video_id, track.id);
            }
        }
    }
    Ok(hoarded)
}

/// The tracks that hold a claim of `kind` on `reference`.
async fn claimed(
    db: &mut Db,
    kind: ClaimKind,
    reference: &str,
) -> Result<HashSet<u64>, toasty::Error> {
    Ok(TrackClaim::filter(TrackClaim::fields().kind().eq(kind))
        .exec(db)
        .await?
        .into_iter()
        .filter(|claim| claim.reference.as_deref() == Some(reference))
        .map(|claim| claim.track_id)
        .collect())
}

/// What unfinished jobs already fetch.
#[derive(Default)]
struct Pending {
    tracks: HashSet<String>,
    albums: HashSet<String>,
}

async fn pending(db: &mut Db) -> Result<Pending, toasty::Error> {
    let mut pending = Pending::default();
    for job in queue::unfinished(db).await? {
        match job.kind {
            JobKind::DownloadTrack => {
                if let Ok(payload) = serde_json::from_str::<TrackJob>(&job.payload) {
                    pending.tracks.insert(payload.video_id);
                }
            }
            JobKind::GrabAlbum => {
                if let Ok(payload) = serde_json::from_str::<AlbumJob>(&job.payload) {
                    pending.albums.insert(payload.browse_id);
                }
            }
            JobKind::SyncWatch | JobKind::Enrich => {}
        }
    }
    Ok(pending)
}

/// The watch a sync job syncs.
fn synced_watch(job: &Job) -> Option<u64> {
    (job.kind == JobKind::SyncWatch)
        .then(|| serde_json::from_str::<SyncJob>(&job.payload).ok())
        .flatten()
        .map(|payload| payload.watch_id)
}

/// A watch to add.
#[derive(Debug, Clone)]
pub struct NewWatch {
    pub kind: WatchKind,
    /// The playlist or channel id; ignored for liked music.
    pub remote_id: String,
    pub include_singles: bool,
    pub only_new: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("píxiū already watches this")]
    Duplicate,
    #[error("database error: {0}")]
    Db(#[from] toasty::Error),
}

fn after(interval: Duration) -> Timestamp {
    let seconds = i64::try_from(interval.as_secs()).unwrap_or(i64::MAX);
    now()
        .checked_add(SignedDuration::from_secs(seconds))
        .unwrap_or(Timestamp::MAX)
}

/// Adds a watch and queues its first sync.
///
/// # Errors
///
/// Fails when the thing is already watched, or on database errors.
pub async fn add(treasury: &Treasury, jobs: &Jobs, new: NewWatch) -> Result<Watch, WatchError> {
    let mut db = treasury.db();
    let (remote_id, name) = match new.kind {
        WatchKind::LikedMusic => (LIKED_MUSIC.to_owned(), "Liked music".to_owned()),
        _ => (new.remote_id.clone(), new.remote_id),
    };
    if Watch::filter_by_remote_id(&remote_id)
        .first()
        .exec(&mut db)
        .await?
        .is_some()
    {
        return Err(WatchError::Duplicate);
    }
    let interval = default_interval(new.kind);
    let watch = toasty::create!(Watch {
        kind: new.kind,
        remote_id,
        name,
        include_singles: new.include_singles,
        only_new: new.only_new,
        seen: Vec::<String>::new(),
        interval_secs: interval.as_secs(),
        created_at: now(),
        next_sync_at: after(interval),
    })
    .exec(&mut db)
    .await?;
    tracing::info!(watch = watch.id, remote_id = %watch.remote_id, "watch added");
    queue_sync(jobs, &watch).await?;
    Ok(watch)
}

/// Queues a sync of `watch` now, unless one is waiting or running.
/// Earlier failed syncs of it are forgotten.
///
/// # Errors
///
/// Fails on database errors.
pub async fn queue_sync(jobs: &Jobs, watch: &Watch) -> Result<bool, toasty::Error> {
    let mut busy = false;
    let mut failed_before = false;
    for job in jobs.unfinished().await? {
        if synced_watch(&job) == Some(watch.id) {
            if job.state == JobState::Failed {
                failed_before = true;
            } else {
                busy = true;
            }
        }
    }
    if failed_before {
        jobs.forget(|job| synced_watch(job) == Some(watch.id) && job.state == JobState::Failed)
            .await?;
    }
    if busy {
        return Ok(false);
    }
    jobs.enqueue(NewJob::sync(watch.id, &format!("Sync {}", watch.name)))
        .await?;
    Ok(true)
}

/// Stops watching: forgets the mirror, the watch's claims and its pending
/// work. The tracks stay; those nothing else claims become orphans.
///
/// # Errors
///
/// Fails on database errors.
pub async fn remove(treasury: &Treasury, jobs: &Jobs, watch_id: u64) -> Result<(), toasty::Error> {
    let mut db = treasury.db();
    jobs.forget(|job| {
        synced_watch(job) == Some(watch_id)
            || queue::wanted(job).and_then(Wanted::watch_id) == Some(watch_id)
    })
    .await?;
    if let Some(playlist) = Playlist::filter_by_watch_id(Some(watch_id))
        .first()
        .exec(&mut db)
        .await?
    {
        PlaylistEntry::filter_by_playlist_id(playlist.id)
            .delete()
            .exec(&mut db)
            .await?;
        playlist.delete().exec(&mut db).await?;
    }
    let reference = watch_id.to_string();
    for kind in [ClaimKind::WatchPlaylist, ClaimKind::WatchArtist] {
        treasury.release(kind, &reference, |_| false).await?;
    }
    if let Some(watch) = Watch::filter_by_id(watch_id).first().exec(&mut db).await? {
        tracing::info!(watch = watch.id, remote_id = %watch.remote_id, "watch removed");
        watch.delete().exec(&mut db).await?;
    }
    Ok(())
}

/// Queues syncs of watches as they fall due, until aborted.
pub fn schedule(db: Db, jobs: Arc<Jobs>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(error) = queue_due(&db, &jobs).await {
                tracing::warn!(%error, "cannot schedule watch syncs");
            }
            tokio::time::sleep(TICK).await;
        }
    })
}

async fn queue_due(db: &Db, jobs: &Jobs) -> Result<(), toasty::Error> {
    let mut db = db.clone();
    let now = now();
    for mut watch in Watch::all().exec(&mut db).await? {
        if watch.next_sync_at > now {
            continue;
        }
        queue_sync(jobs, &watch).await?;
        // Spread syncs out a little, so watches added together drift apart.
        let interval = Duration::from_secs(watch.interval_secs);
        let jitter = Duration::from_secs(rand::random_range(0..=watch.interval_secs / 10));
        toasty::update!(watch {
            next_sync_at: after(interval + jitter),
        })
        .exec(&mut db)
        .await?;
    }
    Ok(())
}

/// Resumes paused work whenever the session starts working, e.g. after
/// the admin logs in again.
pub fn resume_on_login(warden: &Warden, jobs: Arc<Jobs>) -> tokio::task::JoinHandle<()> {
    let mut health = warden.subscribe();
    tokio::spawn(async move {
        let mut working = false;
        loop {
            let now_working = health.borrow_and_update().state == Some(SessionState::Valid);
            if now_working && !working {
                match jobs.resume_paused().await {
                    Ok(0) => {}
                    Ok(resumed) => tracing::info!(resumed, "login works again; resuming jobs"),
                    Err(error) => tracing::error!(%error, "cannot resume paused jobs"),
                }
            }
            working = now_working;
            if health.changed().await.is_err() {
                break;
            }
        }
    })
}
