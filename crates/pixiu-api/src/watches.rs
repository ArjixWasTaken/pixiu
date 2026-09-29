//! Watches: YouTube Music playlists, liked music and artists that píxiū
//! keeps in the hoard, and the songs excluded from watched playlists.

use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use jiff::Timestamp;
use pixiu_db::{JobState, Playlist, PlaylistEntry, SessionState, Track, Watch, WatchKind};
use pixiu_hunt::link::{self, Link};
use pixiu_jobs::{
    queue::wanted,
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

fn remote_url(watch: &Watch) -> String {
    match watch.kind {
        WatchKind::Artist => format!("https://music.youtube.com/channel/{}", watch.remote_id),
        WatchKind::Playlist | WatchKind::LikedMusic => format!(
            "https://music.youtube.com/playlist?list={}",
            watch.remote_id
        ),
    }
}

/// The songs of a watched playlist's mirror that the hoard holds, of all.
async fn progress(state: &ApiState, mirror: &Playlist) -> ApiResult<(usize, usize)> {
    let mut db = state.db.clone();
    let videos: Vec<String> = PlaylistEntry::filter_by_playlist_id(mirror.id)
        .exec(&mut db)
        .await?
        .into_iter()
        .filter_map(|entry| entry.ytm_video_id)
        .collect();
    let mut downloaded = HashSet::new();
    for chunk in videos.chunks(500) {
        for track in Track::filter(Track::fields().ytm_video_id().in_list(chunk.to_vec()))
            .exec(&mut db)
            .await?
        {
            downloaded.extend(track.ytm_video_id);
        }
    }
    let have = videos
        .iter()
        .filter(|video| downloaded.contains(*video))
        .count();
    Ok((have, videos.len()))
}

#[derive(Serialize)]
struct Status {
    /// `synced`, `never_synced`, `syncing`, `queued`, `waiting` (for a
    /// login) or `failed`.
    state: &'static str,
    error: Option<String>,
}

/// `GET /api/watches`: every watch with how it is doing.
pub(crate) async fn list(
    State(state): State<ApiState>,
    _: Session,
) -> ApiResult<Json<Vec<JsonValue>>> {
    let mut db = state.db.clone();
    let mut watches = Watch::all().exec(&mut db).await?;
    watches.sort_by_key(|watch| watch.created_at);
    let logged_in = matches!(
        state.warden.health().state,
        Some(SessionState::Valid | SessionState::Degraded)
    );
    let unfinished = state.jobs.unfinished().await?;
    let syncing = watch::syncing(&mut db).await?;
    let mirrors: HashMap<u64, Playlist> = Playlist::all()
        .exec(&mut db)
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
                let (have, total) = progress(&state, mirror).await?;
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
    _: Session,
    Json(form): Json<AddWatch>,
) -> ApiResult<StatusCode> {
    let target = form.target.trim();
    let parsed = if target == "liked" {
        Some(Link::LikedMusic)
    } else {
        link::parse(target)
    };
    let (kind, remote_id) = match parsed {
        Some(Link::Playlist(id)) => (WatchKind::Playlist, id),
        Some(Link::LikedMusic) => (WatchKind::LikedMusic, String::new()),
        Some(Link::Artist(id)) => (WatchKind::Artist, id),
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
        remote_id,
        include_singles: form.singles,
        only_new: form.only_new,
    };
    match watch::add(&state.treasury, &state.jobs, new).await {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(WatchError::Duplicate) => Err(ApiError::new(
            StatusCode::CONFLICT,
            "píxiū already watches that.",
        )),
        Err(WatchError::Db(error)) => Err(error.into()),
    }
}

/// `DELETE /api/watches/{id}`: stops watching; what it kept becomes
/// orphans unless something else keeps it.
pub(crate) async fn remove(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    watch::remove(&state.treasury, &state.jobs, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/watches/{id}/sync`: syncs now.
pub(crate) async fn sync(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let Some(watch) = Watch::filter_by_id(id)
        .first()
        .exec(&mut state.db.clone())
        .await?
    else {
        return Err(ApiError::not_found("watch"));
    };
    watch::queue_sync(&state.jobs, &watch).await?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Serialize)]
struct Excluded {
    video_id: String,
    title: Option<String>,
    artist: Option<String>,
    excluded_at: Timestamp,
}

async fn excluded(state: &ApiState, watch_id: u64) -> ApiResult<Vec<Excluded>> {
    Ok(watch::exclusions(&mut state.db.clone(), watch_id)
        .await?
        .into_iter()
        .map(|exclusion| Excluded {
            video_id: exclusion.ytm_video_id,
            title: exclusion.title,
            artist: exclusion.artist,
            excluded_at: exclusion.excluded_at,
        })
        .collect())
}

/// `GET /api/watches/{id}/exclusions`.
pub(crate) async fn exclusions(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<u64>,
) -> ApiResult<Json<JsonValue>> {
    Ok(Json(json!(excluded(&state, id).await?)))
}

#[derive(Deserialize)]
pub(crate) struct Exclude {
    /// The song, by its Subsonic id (`tr-…`) or its YouTube video id.
    song: String,
}

/// `POST /api/watches/{id}/exclusions`: excludes a song from a watched
/// playlist; it becomes an orphan unless something else keeps it.
pub(crate) async fn exclude(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<u64>,
    Json(form): Json<Exclude>,
) -> ApiResult<StatusCode> {
    let video = match ids::Id::parse(&form.song) {
        Some(ids::Id::Track(track_id)) => Track::filter_by_id(track_id)
            .first()
            .exec(&mut state.db.clone())
            .await?
            .and_then(|track| track.ytm_video_id)
            .ok_or_else(|| ApiError::unprocessable("That song did not come from YouTube Music."))?,
        _ => form.song,
    };
    if watch::exclude(&state.treasury, &state.jobs, id, &video).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("watched playlist"))
    }
}

/// `DELETE /api/watches/{id}/exclusions/{video}`: lets the watch fetch the
/// song again.
pub(crate) async fn include(
    State(state): State<ApiState>,
    _: Session,
    Path((id, video)): Path<(u64, String)>,
) -> ApiResult<StatusCode> {
    watch::include(&state.treasury, &state.jobs, id, &video).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/playlists/{id}/watch`: for a playlist mirroring a watch, the
/// watch, the songs still coming and the songs excluded; `null` for other
/// playlists.
pub(crate) async fn of_playlist(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<String>,
) -> ApiResult<Json<JsonValue>> {
    let Some(ids::Id::Playlist(playlist_id)) = ids::Id::parse(&id) else {
        return Err(ApiError::not_found("playlist"));
    };
    let mut db = state.db.clone();
    let Some(playlist) = Playlist::filter_by_id(playlist_id)
        .first()
        .exec(&mut db)
        .await?
    else {
        return Err(ApiError::not_found("playlist"));
    };
    let Some(watch_id) = playlist.watch_id else {
        return Ok(Json(JsonValue::Null));
    };
    let Some(watch) = Watch::filter_by_id(watch_id).first().exec(&mut db).await? else {
        return Ok(Json(JsonValue::Null));
    };
    let mut entries = PlaylistEntry::filter_by_playlist_id(playlist.id)
        .exec(&mut db)
        .await?;
    entries.sort_by_key(|entry| entry.position);
    let videos: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.ytm_video_id.clone())
        .collect();
    let mut hoarded = HashSet::new();
    for chunk in videos.chunks(500) {
        for track in Track::filter(Track::fields().ytm_video_id().in_list(chunk.to_vec()))
            .exec(&mut db)
            .await?
        {
            hoarded.extend(track.ytm_video_id);
        }
    }
    let coming: Vec<JsonValue> = entries
        .into_iter()
        .filter(|entry| entry.track_id.is_none())
        .filter_map(|entry| {
            let video = entry.ytm_video_id?;
            (!hoarded.contains(&video))
                .then(|| json!({ "video_id": video, "title": entry.title, "artist": entry.artist }))
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
        "excluded": excluded(&state, watch.id).await?,
    })))
}
