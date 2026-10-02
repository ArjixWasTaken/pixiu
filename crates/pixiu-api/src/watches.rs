//! Watches: YouTube Music playlists, liked music and artists that píxiū
//! keeps in the signed-in user's library, and the songs excluded from
//! watched playlists.

use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use jiff::Timestamp;
use pixiu_db::{
    Job, JobKind, JobState, Library, Playlist, PlaylistEntry, SessionState, SourceKey, Watch,
    WatchKind,
};
use pixiu_hunt::{
    LIKED_MUSIC,
    link::{self, Link},
};
use pixiu_jobs::{
    queue::{TrackJob, wanted},
    watch::{self, NewWatch, WAITING_FOR_LOGIN, WatchError},
};
use pixiu_subsonic::ids;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session};

fn kind_name(kind: WatchKind) -> &'static str {
    match kind {
        WatchKind::Playlist => "playlist",
        WatchKind::LikedMusic => "liked_music",
        WatchKind::Artist => "artist",
    }
}

fn remote_url(watch: &Watch) -> Option<String> {
    let key: SourceKey = watch.source_key.parse().ok()?;
    Some(match watch.kind {
        WatchKind::Artist => format!("https://music.youtube.com/channel/{}", key.id()),
        WatchKind::Playlist | WatchKind::LikedMusic => {
            format!("https://music.youtube.com/playlist?list={}", key.id())
        }
    })
}

/// The library's tracks among `keys`.
async fn keys_held(lib: &Library, keys: &[SourceKey]) -> ApiResult<HashSet<SourceKey>> {
    Ok(lib.tracks_of_keys(keys).await?.keys().collect())
}

/// The keys of a mirror's entries, in order.
fn entry_keys<'a>(entries: impl IntoIterator<Item = &'a PlaylistEntry>) -> Vec<SourceKey> {
    entries
        .into_iter()
        .filter_map(|entry| SourceKey::from_stored(entry.source_key.as_deref()))
        .collect()
}

/// The songs of a watched playlist's mirror that the library holds, of all.
async fn progress(lib: &Library, mirror: &Playlist) -> ApiResult<(usize, usize)> {
    let mut db = lib.db();
    let entries = PlaylistEntry::filter_by_playlist_id(mirror.id)
        .exec(&mut db)
        .await?;
    let keys = entry_keys(&entries);
    let downloaded = keys_held(lib, &keys).await?;
    let have = keys.iter().filter(|key| downloaded.contains(*key)).count();
    Ok((have, keys.len()))
}

#[derive(Serialize)]
struct Status {
    /// `synced`, `never_synced`, `syncing`, `queued`, `waiting` (for a
    /// login) or `failed`.
    state: &'static str,
    error: Option<String>,
}

/// `GET /api/watches`: every watch of the user's with how it is doing.
pub(crate) async fn list(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<Vec<JsonValue>>> {
    let (owner, lib) = (session.owner(), session.library(&state));
    let mut db = lib.db();
    let mut watches = lib.all_watches().await?;
    watches.sort_by_key(|watch| watch.created_at);
    let logged_in = matches!(
        crate::sources::health_of(&state, owner).state,
        Some(SessionState::Valid | SessionState::Degraded)
    );
    let unfinished = state.jobs.unfinished(owner).await?;
    let syncing = watch::syncing(&mut db, owner).await?;
    let mirrors: HashMap<u64, Playlist> = lib
        .all_playlists()
        .await?
        .into_iter()
        .filter_map(|playlist| playlist.watch_id.map(|id| (id, playlist)))
        .collect();

    let mut rows = Vec::with_capacity(watches.len());
    for watch in watches {
        let waiting = watch.last_error.as_deref() == Some(WAITING_FOR_LOGIN)
            || (watch.kind == WatchKind::LikedMusic && !logged_in);
        let status = match (
            syncing.get(&watch.id),
            &watch.last_error,
            watch.last_synced_at,
        ) {
            (Some(JobState::Running), _, _) => Status {
                state: "syncing",
                error: None,
            },
            _ if waiting => Status {
                state: "waiting",
                error: None,
            },
            (Some(_), _, _) => Status {
                state: "queued",
                error: None,
            },
            (None, Some(error), _) => Status {
                state: "failed",
                error: Some(error.clone()),
            },
            (None, None, Some(_)) => Status {
                state: "synced",
                error: None,
            },
            (None, None, None) => Status {
                state: "never_synced",
                error: None,
            },
        };
        let (failed, queued) = unfinished
            .iter()
            .filter(|job| wanted(job).and_then(|wanted| wanted.watch_id()) == Some(watch.id))
            .fold((0, 0), |(failed, queued), job| {
                if job.state == JobState::Failed {
                    (failed + 1, queued)
                } else {
                    (failed, queued + 1)
                }
            });
        let mirror = mirrors.get(&watch.id);
        let songs = match mirror {
            Some(mirror) => {
                let (have, total) = progress(&lib, mirror).await?;
                Some(json!({ "have": have, "total": total }))
            }
            None => None,
        };
        rows.push(json!({
            "id": watch.id,
            "kind": kind_name(watch.kind),
            "name": watch.name,
            "image": watch.image_url,
            "link": remote_url(&watch),
            "include_singles": watch.include_singles,
            "only_new": watch.only_new,
            "releases_known": watch.seen.len(),
            "created_at": watch.created_at,
            "last_synced_at": watch.last_synced_at,
            "next_sync_at": watch.next_sync_at,
            "status": status,
            "jobs": { "queued": queued, "failed": failed },
            "songs": songs,
            "playlist_id": mirror.map(|mirror| ids::playlist(mirror.id)),
        }));
    }
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub(crate) struct AddWatch {
    /// A playlist or artist link, or `liked` for the account's liked music.
    target: String,
    #[serde(default)]
    only_new: bool,
    #[serde(default)]
    singles: bool,
}

/// `POST /api/watches`.
pub(crate) async fn add(
    State(state): State<ApiState>,
    session: Session,
    Json(form): Json<AddWatch>,
) -> ApiResult<StatusCode> {
    let target = form.target.trim();
    let parsed = if target == "liked" {
        Some(Link::LikedMusic)
    } else {
        link::parse(target)
    };
    let (kind, key) = match parsed {
        Some(Link::Playlist(id)) => (WatchKind::Playlist, SourceKey::youtube_music(id)),
        Some(Link::LikedMusic) => (WatchKind::LikedMusic, SourceKey::youtube_music(LIKED_MUSIC)),
        Some(Link::Artist(id)) => (WatchKind::Artist, SourceKey::youtube_music(id)),
        Some(Link::Album(_) | Link::Track(_)) => {
            return Err(ApiError::unprocessable(
                "That is an album or a song; download it from Discover instead.",
            ));
        }
        None => {
            return Err(ApiError::unprocessable(
                "That does not look like a YouTube Music playlist or artist link.",
            ));
        }
    };
    let new = NewWatch {
        kind,
        key,
        include_singles: form.singles,
        only_new: form.only_new,
    };
    match watch::add(&state.treasury, &state.jobs, session.owner(), new).await {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(WatchError::Duplicate) => Err(ApiError::new(
            StatusCode::CONFLICT,
            "You already watch that.",
        )),
        Err(WatchError::Db(error)) => Err(error.into()),
    }
}

/// `DELETE /api/watches/{id}`: stops watching; what it kept becomes
/// orphans unless something else keeps it.
pub(crate) async fn remove(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let watch = load(&session.library(&state), id).await?;
    watch::remove(&state.treasury, &state.jobs, watch.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/watches/{id}/sync`: syncs now.
pub(crate) async fn sync(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let watch = load(&session.library(&state), id).await?;
    watch::queue_sync(&state.jobs, &watch).await?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Serialize)]
struct Excluded {
    /// The song on its platform.
    key: String,
    title: Option<String>,
    artist: Option<String>,
    excluded_at: Timestamp,
}

/// One of the library's watches.
async fn load(lib: &Library, id: u64) -> ApiResult<Watch> {
    lib.watch(id)
        .await?
        .ok_or_else(|| ApiError::not_found("watch"))
}

/// The songs excluded from one of the library's watches.
async fn excluded(lib: &Library, watch_id: u64) -> ApiResult<Vec<Excluded>> {
    Ok(watch::exclusions(&mut lib.db(), watch_id)
        .await?
        .into_iter()
        .map(|exclusion| Excluded {
            key: exclusion.source_key,
            title: exclusion.title,
            artist: exclusion.artist,
            excluded_at: exclusion.excluded_at,
        })
        .collect())
}

/// `GET /api/watches/{id}/exclusions`.
pub(crate) async fn exclusions(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<u64>,
) -> ApiResult<Json<JsonValue>> {
    let lib = session.library(&state);
    let watch = load(&lib, id).await?;
    Ok(Json(json!(excluded(&lib, watch.id).await?)))
}

#[derive(Deserialize)]
pub(crate) struct Exclude {
    /// The song, by its Subsonic id (`tr-…`) or its key.
    song: String,
}

/// `POST /api/watches/{id}/exclusions`: excludes a song from a watched
/// playlist; it becomes an orphan unless something else keeps it.
pub(crate) async fn exclude(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<u64>,
    Json(form): Json<Exclude>,
) -> ApiResult<StatusCode> {
    let lib = session.library(&state);
    let id = load(&lib, id).await?.id;
    let key = match ids::Id::parse(&form.song) {
        Some(ids::Id::Track(track_id)) => lib
            .track(track_id)
            .await?
            .and_then(|track| SourceKey::from_stored(track.source_key.as_deref())),
        _ => form.song.parse().ok(),
    };
    let Some(key) = key else {
        return Err(ApiError::unprocessable(
            "That song was not downloaded from the watched playlist's platform.",
        ));
    };
    if watch::exclude(&state.treasury, &state.jobs, id, &key).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("watched playlist"))
    }
}

/// `DELETE /api/watches/{id}/exclusions/{key}`: lets the watch fetch the
/// song again.
pub(crate) async fn include(
    State(state): State<ApiState>,
    session: Session,
    Path((id, key)): Path<(u64, String)>,
) -> ApiResult<StatusCode> {
    let watch = load(&session.library(&state), id).await?;
    let key: SourceKey = key
        .parse()
        .map_err(|_| ApiError::not_found("excluded song"))?;
    watch::include(&state.treasury, &state.jobs, watch.id, &key).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The latest download job of each song the user asked for, among the
/// unfinished and the recent.
async fn latest_downloads(state: &ApiState, owner: u64) -> ApiResult<HashMap<SourceKey, Job>> {
    let mut jobs = state.jobs.unfinished(owner).await?;
    jobs.extend(state.jobs.recent(owner, 500).await?);
    jobs.sort_by_key(|job| job.id);
    let mut latest = HashMap::new();
    for job in jobs {
        if job.kind != JobKind::DownloadTrack {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<TrackJob>(&job.payload) {
            latest.insert(payload.key, job);
        }
    }
    Ok(latest)
}

/// `GET /api/playlists/{id}/watch`: for a playlist mirroring a watch, the
/// watch, the songs still coming and the songs excluded; `null` for other
/// playlists.
pub(crate) async fn of_playlist(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<String>,
) -> ApiResult<Json<JsonValue>> {
    let Some(ids::Id::Playlist(playlist_id)) = ids::Id::parse(&id) else {
        return Err(ApiError::not_found("playlist"));
    };
    let lib = session.library(&state);
    let mut db = lib.db();
    let Some(playlist) = lib.playlist(playlist_id).await? else {
        return Err(ApiError::not_found("playlist"));
    };
    let Some(watch_id) = playlist.watch_id else {
        return Ok(Json(JsonValue::Null));
    };
    let Some(watch) = lib.watch(watch_id).await? else {
        return Ok(Json(JsonValue::Null));
    };
    let mut entries = PlaylistEntry::filter_by_playlist_id(playlist.id)
        .exec(&mut db)
        .await?;
    entries.sort_by_key(|entry| entry.position);
    let hoarded = keys_held(&lib, &entry_keys(&entries)).await?;
    let downloads = latest_downloads(&state, session.owner()).await?;
    let coming: Vec<JsonValue> = entries
        .into_iter()
        .filter(|entry| entry.track_id.is_none())
        .filter_map(|entry| {
            let key = SourceKey::from_stored(entry.source_key.as_deref())?;
            (!hoarded.contains(&key)).then(|| {
                // How its download is doing, when there is one.
                let job = downloads.get(&key).map(|job| {
                    json!({
                        "state": crate::jobs::state_name(job.state),
                        "error": job.error,
                    })
                });
                json!({ "key": key, "title": entry.title, "artist": entry.artist, "job": job })
            })
        })
        .collect();
    Ok(Json(json!({
        "watch": {
            "id": watch.id,
            "kind": kind_name(watch.kind),
            "name": watch.name,
            "link": remote_url(&watch),
            "last_synced_at": watch.last_synced_at,
        },
        "coming": coming,
        "excluded": excluded(&lib, watch.id).await?,
    })))
}
