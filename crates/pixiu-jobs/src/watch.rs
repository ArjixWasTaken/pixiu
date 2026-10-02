//! Watches: playlists and liked music mirrored into Subsonic playlists, and
//! artists whose releases are grabbed as they appear.
//!
//! Syncing is one-way. What a watch brings in is claimed by it. When a
//! track leaves a watched playlist, the mirror follows and the claim goes;
//! the file stays, an orphan unless something else claims it. The user may
//! also exclude a song from a watched playlist: the watch then neither
//! keeps, lists nor downloads it, as if it had left. Every watch belongs to
//! a user, and everything it brings in goes to their library.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use jiff::{SignedDuration, Timestamp};
use pixiu_core::alerts::{Alert, AlertSink};
use pixiu_db::{
    ClaimKind, Db, Job, JobKind, JobState, Playlist, PlaylistEntry, ReleaseReason, SessionState,
    SourceKey, TrackClaim, User, UserStatus, Watch, WatchExclusion, WatchKind, keyed, now, toasty,
};
use pixiu_hunt::{AlbumKind, Discography, LIKED_MUSIC, RemotePlaylist, RemoteTrack};
use pixiu_treasury::{Release, Treasury};

use crate::{
    queue::{self, Jobs, NewJob, SyncJob, TrackJob, Wanted, pending},
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
    /// A playlist with all its tracks; the key of [`LIKED_MUSIC`] for liked
    /// music.
    fn playlist<'a>(
        &'a self,
        key: &'a SourceKey,
    ) -> BoxFuture<'a, Result<RemotePlaylist, CatalogError>>;

    /// An artist's releases.
    fn discography<'a>(
        &'a self,
        key: &'a SourceKey,
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

/// Failed syncs in a row before the watch's owner hears about it.
pub const FAILING_AFTER: u32 = 3;

fn failed(error: impl ToString) -> String {
    error.to_string()
}

/// Brings a watch up to date: a mirror follows its playlist, claims follow
/// the mirror, and what its owner's library lacks is queued. The watch records how
/// it went, and its owner hears about it when it keeps failing.
///
/// # Errors
///
/// Fails when the platform cannot answer or on database errors.
pub async fn sync(
    treasury: &Treasury,
    catalog: &dyn Catalog,
    alerts: &dyn AlertSink,
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
        WatchKind::Artist => sync_artist(treasury, &mut db, catalog, &watch).await,
    };
    if let Some(alert) = record(&mut db, watch_id, &result).await {
        alerts.alert(watch.user_id, alert).await;
    }
    result
}

/// Records how a sync went; the alert to raise when the watch has failed
/// [`FAILING_AFTER`] times in a row.
async fn record(db: &mut Db, watch_id: u64, result: &Result<Synced, String>) -> Option<Alert> {
    let Ok(Some(mut watch)) = Watch::filter_by_id(watch_id).first().exec(db).await else {
        return None;
    };
    // Waiting for a login is not the watch's failure: its owner hears
    // about their session instead.
    let (last_synced_at, last_error, failures, failing_since) = match result {
        Ok(Synced::Done(_)) => (Some(now()), None, 0, None),
        Ok(Synced::NeedsLogin(reason)) => (
            watch.last_synced_at,
            Some(reason.clone()),
            watch.failures,
            watch.failing_since,
        ),
        Err(reason) => (
            watch.last_synced_at,
            Some(reason.clone()),
            watch.failures + 1,
            watch.failing_since.or_else(|| Some(now())),
        ),
    };
    let alert = match (result, failing_since) {
        (Err(error), Some(since)) if failures >= FAILING_AFTER => Some(Alert::WatchFailing {
            watch_id,
            name: watch.name.clone(),
            error: error.clone(),
            since,
        }),
        _ => None,
    };
    let updated = toasty::update!(watch {
        last_synced_at,
        last_error,
        failures,
        failing_since,
    })
    .exec(db)
    .await;
    if let Err(error) = updated {
        tracing::error!(%error, watch_id, "cannot record a watch sync");
    }
    alert
}

async fn sync_playlist(
    treasury: &Treasury,
    catalog: &dyn Catalog,
    watch: &Watch,
) -> Result<Synced, String> {
    let owner = watch.user_id;
    let liked = watch.kind == WatchKind::LikedMusic;
    if liked && !catalog.logged_in() {
        return Ok(Synced::NeedsLogin(WAITING_FOR_LOGIN.to_owned()));
    }
    let key = watch_key(watch)?;
    let playlist = match catalog.playlist(&key).await {
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
    describe(&mut db, watch.id, &name, playlist.image_url.as_deref())
        .await
        .map_err(failed)?;

    // Excluded songs count as gone from the playlist.
    let excluded = excluded_keys(&mut db, watch.id).await.map_err(failed)?;
    let tracks: Vec<RemoteTrack> = playlist
        .tracks
        .into_iter()
        .filter(|track| !excluded.contains(&track.id.as_stored()))
        .collect();

    // The mirror follows the playlist, order included.
    let order: Vec<SourceKey> = tracks.iter().map(|track| track.id.clone()).collect();
    let mirror = mirror(&mut db, owner, watch.id, &name)
        .await
        .map_err(failed)?;
    set_entries(&mut db, mirror, &tracks)
        .await
        .map_err(failed)?;

    // Tracks that were excluded or left lose this watch's claim ...
    let reference = watch.id.to_string();
    treasury
        .release(
            owner,
            ClaimKind::WatchPlaylist,
            &reference,
            Release {
                reason: ReleaseReason::Excluded,
                source_name: Some(&name),
            },
            |track| {
                !track
                    .source_key
                    .as_ref()
                    .is_some_and(|key| excluded.contains(key))
            },
        )
        .await
        .map_err(failed)?;
    let listed: HashSet<String> = order.iter().map(SourceKey::as_stored).collect();
    treasury
        .release(
            owner,
            ClaimKind::WatchPlaylist,
            &reference,
            Release {
                reason: ReleaseReason::LeftPlaylist,
                source_name: Some(&name),
            },
            |track| {
                track
                    .source_key
                    .as_ref()
                    .is_some_and(|key| listed.contains(key))
            },
        )
        .await
        .map_err(failed)?;

    // ... those in the hoard get it ...
    let wanted = Wanted::Playlist { watch_id: watch.id };
    let hoarded = hoarded(&mut db, owner, &order).await.map_err(failed)?;
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
    let pending = pending(&mut db, owner).await.map_err(failed)?;
    let mut queued = HashSet::new();
    let jobs = tracks
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

async fn sync_artist(
    treasury: &Treasury,
    db: &mut Db,
    catalog: &dyn Catalog,
    watch: &Watch,
) -> Result<Synced, String> {
    let discography = match catalog.discography(&watch_key(watch)?).await {
        Ok(discography) => discography,
        Err(CatalogError::NeedsLogin(reason)) => return Ok(Synced::NeedsLogin(reason)),
        Err(CatalogError::Failed(reason)) => return Err(reason),
    };
    let wanted = Wanted::Artist { watch_id: watch.id };
    // A watch of new releases only takes note of the old ones at first.
    let skip_old = watch.only_new && watch.last_synced_at.is_none();
    // Releases seen before keys were bare YouTube Music ids; they read as
    // keys all the same.
    let known: HashSet<SourceKey> = watch
        .seen
        .iter()
        .filter_map(|seen| seen.parse().ok())
        .collect();
    let pending = pending(db, watch.user_id).await.map_err(failed)?;
    let mut seen = watch.seen.clone();
    let mut jobs = Vec::new();
    for album in &discography.albums {
        let wanted_kind = match album.kind {
            AlbumKind::Album | AlbumKind::Other => true,
            AlbumKind::Ep | AlbumKind::Single => watch.include_singles,
        };
        let stored = album.id.as_stored();
        if !wanted_kind || known.contains(&album.id) || seen.contains(&stored) {
            continue;
        }
        seen.push(stored);
        if skip_old || pending.has_album(&album.id) {
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
        // The library's artist of that name is this one.
        treasury
            .learn_artist_key(watch.user_id, &discography.name, &discography.id)
            .await
            .map_err(failed)?;
        toasty::update!(watch {
            name: discography.name,
            image_url: discography.image_url,
            seen,
        })
        .exec(db)
        .await
        .map_err(failed)?;
    }
    Ok(Synced::Done(jobs))
}

/// Keeps a watch's name and picture as the platform shows them.
async fn describe(
    db: &mut Db,
    watch_id: u64,
    name: &str,
    image_url: Option<&str>,
) -> Result<(), toasty::Error> {
    if let Some(mut watch) = Watch::filter_by_id(watch_id).first().exec(db).await?
        && (watch.name != name || watch.image_url.as_deref() != image_url)
    {
        toasty::update!(watch {
            name,
            image_url: image_url.map(str::to_owned),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

/// The playlist mirroring a watch, made in `owner`'s library when missing.
async fn mirror(db: &mut Db, owner: u64, watch_id: u64, name: &str) -> Result<u64, toasty::Error> {
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
            user_id: owner,
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

/// Makes a mirror list `tracks`, in order, each named as the platform
/// names it.
async fn set_entries(
    db: &mut Db,
    playlist_id: u64,
    tracks: &[RemoteTrack],
) -> Result<(), toasty::Error> {
    let mut entries = PlaylistEntry::filter_by_playlist_id(playlist_id)
        .exec(db)
        .await?;
    entries.sort_by_key(|entry| entry.position);
    let unchanged = entries.len() == tracks.len()
        && entries.iter().zip(tracks).all(|(entry, track)| {
            entry.source_key.as_deref() == Some(track.id.as_stored().as_str())
                && entry.title.as_deref() == Some(track.title.as_str())
                && entry.artist.as_deref() == Some(track.artist_credit().as_str())
        });
    if unchanged {
        return Ok(());
    }

    let mut tx = db.transaction().await?;
    PlaylistEntry::filter_by_playlist_id(playlist_id)
        .delete()
        .exec(&mut tx)
        .await?;
    for (position, track) in tracks.iter().enumerate() {
        toasty::create!(PlaylistEntry {
            playlist_id,
            position: u32::try_from(position).unwrap_or(u32::MAX),
            source_key: Some(track.id.as_stored()),
            title: Some(track.title.clone()),
            artist: Some(track.artist_credit()),
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

/// The tracks `owner`'s library holds among these songs, by key.
async fn hoarded(
    db: &mut Db,
    owner: u64,
    keys: &[SourceKey],
) -> Result<HashMap<SourceKey, u64>, toasty::Error> {
    let held = keyed::tracks_of_keys(db, owner, keys).await?;
    Ok(held
        .keys()
        .filter_map(|key| {
            let track = held.track(&key)?.id;
            Some((key, track))
        })
        .collect())
}

/// What a watch follows, by key.
fn watch_key(watch: &Watch) -> Result<SourceKey, String> {
    watch
        .source_key
        .parse()
        .map_err(|error| format!("cannot read what the watch follows: {error}"))
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

/// The watch a sync job brings up to date.
pub fn synced_watch(job: &Job) -> Option<u64> {
    (job.kind == JobKind::SyncWatch)
        .then(|| serde_json::from_str::<SyncJob>(&job.payload).ok())
        .flatten()
        .map(|payload| payload.watch_id)
}

/// A watch to add.
#[derive(Debug, Clone)]
pub struct NewWatch {
    pub kind: WatchKind,
    /// The playlist or artist; ignored for liked music.
    pub key: SourceKey,
    pub include_singles: bool,
    pub only_new: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("You already watch this")]
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

/// Adds a watch for `owner` and queues its first sync.
///
/// # Errors
///
/// Fails when the owner already watches the thing, or on database errors.
pub async fn add(
    treasury: &Treasury,
    jobs: &Jobs,
    owner: u64,
    new: NewWatch,
) -> Result<Watch, WatchError> {
    let mut db = treasury.db();
    let (key, name) = match new.kind {
        WatchKind::LikedMusic => (
            SourceKey::youtube_music(LIKED_MUSIC),
            "Liked music".to_owned(),
        ),
        _ => (new.key.clone(), new.key.id().to_owned()),
    };
    if Watch::filter_by_user_id_and_source_key(owner, key.as_stored())
        .first()
        .exec(&mut db)
        .await?
        .is_some()
    {
        return Err(WatchError::Duplicate);
    }
    let interval = default_interval(new.kind);
    let watch = toasty::create!(Watch {
        failures: 0_u32,
        user_id: owner,
        kind: new.kind,
        source_key: key.as_stored(),
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
    tracing::info!(watch = watch.id, key = %watch.source_key, "watch added");
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
    let owner = watch.user_id;
    let mut busy = false;
    let mut failed_before = false;
    for job in jobs.unfinished(owner).await? {
        if synced_watch(&job) == Some(watch.id) {
            if job.state == JobState::Failed {
                failed_before = true;
            } else {
                busy = true;
            }
        }
    }
    if failed_before {
        jobs.forget(owner, |job| {
            synced_watch(job) == Some(watch.id) && job.state == JobState::Failed
        })
        .await?;
    }
    if busy {
        return Ok(false);
    }
    jobs.enqueue(
        owner,
        NewJob::sync(watch.id, &format!("Sync {}", watch.name)),
    )
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
    let Some(watch) = Watch::filter_by_id(watch_id).first().exec(&mut db).await? else {
        return Ok(());
    };
    let owner = watch.user_id;
    jobs.forget(owner, |job| {
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
    let why = Release {
        reason: ReleaseReason::WatchRemoved,
        source_name: Some(watch.name.as_str()),
    };
    for kind in [ClaimKind::WatchPlaylist, ClaimKind::WatchArtist] {
        treasury
            .release(owner, kind, &reference, why, |_| false)
            .await?;
    }
    WatchExclusion::filter_by_watch_id(watch_id)
        .delete()
        .exec(&mut db)
        .await?;
    tracing::info!(watch = watch.id, key = %watch.source_key, "watch removed");
    watch.delete().exec(&mut db).await?;
    Ok(())
}

/// The songs excluded from a watch, by stored key.
async fn excluded_keys(db: &mut Db, watch_id: u64) -> Result<HashSet<String>, toasty::Error> {
    Ok(WatchExclusion::filter_by_watch_id(watch_id)
        .exec(db)
        .await?
        .into_iter()
        .map(|exclusion| exclusion.source_key)
        .collect())
}

/// Whether the user excluded a song from a watch.
///
/// # Errors
///
/// Fails on database errors.
pub async fn is_excluded(
    db: &mut Db,
    watch_id: u64,
    key: &SourceKey,
) -> Result<bool, toasty::Error> {
    Ok(excluded_keys(db, watch_id)
        .await?
        .contains(&key.as_stored()))
}

/// The songs excluded from a watch, the latest first.
///
/// # Errors
///
/// Fails on database errors.
pub async fn exclusions(db: &mut Db, watch_id: u64) -> Result<Vec<WatchExclusion>, toasty::Error> {
    let mut exclusions = WatchExclusion::filter_by_watch_id(watch_id)
        .exec(db)
        .await?;
    exclusions.sort_by_key(|exclusion| std::cmp::Reverse((exclusion.excluded_at, exclusion.id)));
    Ok(exclusions)
}

/// Excludes a song from a watched playlist: it leaves the mirror, the watch
/// lets go of it (an orphan, unless something else keeps it), a queued
/// download of it is forgotten, and later syncs skip it. Returns `false`
/// when there is no such playlist watch.
///
/// # Errors
///
/// Fails on database errors.
pub async fn exclude(
    treasury: &Treasury,
    jobs: &Jobs,
    watch_id: u64,
    key: &SourceKey,
) -> Result<bool, toasty::Error> {
    let mut db = treasury.db();
    let stored = key.as_stored();
    let Some(watch) = Watch::filter_by_id(watch_id).first().exec(&mut db).await? else {
        return Ok(false);
    };
    if watch.kind == WatchKind::Artist {
        return Ok(false);
    }

    // The mirror entry names the song; else the downloaded track does.
    let mirror = Playlist::filter_by_watch_id(Some(watch_id))
        .first()
        .exec(&mut db)
        .await?;
    let mut entries = match &mirror {
        Some(mirror) => {
            PlaylistEntry::filter_by_playlist_id(mirror.id)
                .exec(&mut db)
                .await?
        }
        None => Vec::new(),
    };
    entries.sort_by_key(|entry| entry.position);
    let entry = entries
        .iter()
        .position(|entry| entry.source_key.as_deref() == Some(stored.as_str()))
        .map(|index| entries.remove(index));
    let track = keyed::track_of_key(&mut db, watch.user_id, key).await?;

    if !is_excluded(&mut db, watch_id, key).await? {
        let (title, artist) = match (&entry, &track) {
            (Some(entry), _) if entry.title.is_some() => {
                (entry.title.clone(), entry.artist.clone())
            }
            (_, Some(track)) => (Some(track.title.clone()), Some(track.artist_credit.clone())),
            _ => (None, None),
        };
        toasty::create!(WatchExclusion {
            watch_id,
            source_key: &stored,
            title,
            artist,
            excluded_at: now(),
        })
        .exec(&mut db)
        .await?;
    }

    if let (Some(entry), Some(mirror)) = (entry, mirror) {
        let mut tx = db.transaction().await?;
        entry.delete().exec(&mut tx).await?;
        for (position, mut entry) in entries.into_iter().enumerate() {
            let position = u32::try_from(position).unwrap_or(u32::MAX);
            if entry.position != position {
                toasty::update!(entry { position }).exec(&mut tx).await?;
            }
        }
        let mut mirror = mirror;
        toasty::update!(mirror { changed_at: now() })
            .exec(&mut tx)
            .await?;
        tx.commit().await?;
    }

    treasury
        .release(
            watch.user_id,
            ClaimKind::WatchPlaylist,
            &watch_id.to_string(),
            Release {
                reason: ReleaseReason::Excluded,
                source_name: Some(&watch.name),
            },
            |track| track.source_key.as_deref() != Some(stored.as_str()),
        )
        .await?;
    jobs.forget(watch.user_id, |job| {
        queue::wanted(job).and_then(Wanted::watch_id) == Some(watch_id)
            && job.kind == JobKind::DownloadTrack
            && serde_json::from_str::<TrackJob>(&job.payload)
                .is_ok_and(|payload| payload.key == *key)
    })
    .await?;
    tracing::info!(watch = watch_id, %key, "song excluded from a watch");
    Ok(true)
}

/// Takes an exclusion back; the next sync, queued now, lists and claims the
/// song again (or downloads it).
///
/// # Errors
///
/// Fails on database errors.
pub async fn include(
    treasury: &Treasury,
    jobs: &Jobs,
    watch_id: u64,
    key: &SourceKey,
) -> Result<(), toasty::Error> {
    let mut db = treasury.db();
    let stored = key.as_stored();
    for exclusion in WatchExclusion::filter_by_watch_id(watch_id)
        .exec(&mut db)
        .await?
    {
        if exclusion.source_key == stored {
            exclusion.delete().exec(&mut db).await?;
        }
    }
    if let Some(watch) = Watch::filter_by_id(watch_id).first().exec(&mut db).await? {
        queue_sync(jobs, &watch).await?;
    }
    Ok(())
}

/// `owner`'s watches with a sync queued or under way, and that job's
/// state: `Queued`, `Running`, or `Paused` while it waits for a login.
///
/// # Errors
///
/// Fails on database errors.
pub async fn syncing(db: &mut Db, owner: u64) -> Result<HashMap<u64, JobState>, toasty::Error> {
    Ok(queue::unfinished(db, owner)
        .await?
        .iter()
        .filter(|job| job.state != JobState::Failed)
        .filter_map(|job| synced_watch(job).map(|watch| (watch, job.state)))
        .collect())
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
    let active: HashSet<u64> = User::all()
        .exec(&mut db)
        .await?
        .into_iter()
        .filter(|user| user.status == UserStatus::Active)
        .map(|user| user.id)
        .collect();
    for mut watch in Watch::all().exec(&mut db).await? {
        // Watches of accounts turned off wait for them to be back on.
        if watch.next_sync_at > now || !active.contains(&watch.user_id) {
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

/// Resumes the owner's paused work whenever their session starts working,
/// e.g. after they log in again.
pub fn resume_on_login(warden: &Warden, jobs: Arc<Jobs>) -> tokio::task::JoinHandle<()> {
    let mut health = warden.subscribe();
    let owner = warden.owner();
    tokio::spawn(async move {
        let mut working = false;
        loop {
            let now_working = health.borrow_and_update().state == Some(SessionState::Valid);
            if now_working && !working {
                match jobs.resume_paused(owner).await {
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
