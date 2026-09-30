//! Search YouTube Music and grab what the signed-in user's library lacks.

use std::collections::HashSet;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use pixiu_db::Library;
use pixiu_hunt::{AlbumKind, RemoteAlbum, RemoteTrack, SearchResults, image_url_at};
use pixiu_jobs::NewJob;
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, ApiState, Session};

/// The size covers of results are fetched at.
const COVER_SIZE: u32 = 400;

#[derive(Deserialize)]
pub(crate) struct SearchQuery {
    q: String,
}

/// Where a result stands: in the hoard, on its way, or neither.
#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Standing {
    Hoarded,
    Pending,
    Missing,
}

#[derive(Serialize)]
pub(crate) struct TrackResult {
    id: String,
    title: String,
    artist: String,
    album: Option<String>,
    length: Option<u32>,
    cover: Option<String>,
    is_video: bool,
    standing: Standing,
}

#[derive(Serialize)]
pub(crate) struct AlbumResult {
    id: String,
    title: String,
    artist: String,
    year: Option<u16>,
    /// "Album", "EP", "Single" or "Release".
    kind: &'static str,
    cover: Option<String>,
    standing: Standing,
}

#[derive(Serialize)]
pub(crate) struct Results {
    tracks: Vec<TrackResult>,
    albums: Vec<AlbumResult>,
}

fn kind_label(kind: AlbumKind) -> &'static str {
    match kind {
        AlbumKind::Album => "Album",
        AlbumKind::Ep => "EP",
        AlbumKind::Single => "Single",
        AlbumKind::Other => "Release",
    }
}

/// The video and browse ids of the results that the library holds.
async fn hoarded(
    lib: &Library,
    results: &SearchResults,
) -> ApiResult<(HashSet<String>, HashSet<String>)> {
    let videos: Vec<String> = results
        .tracks
        .iter()
        .map(|track| track.id.clone())
        .collect();
    let browses: Vec<String> = results
        .albums
        .iter()
        .map(|album| album.id.clone())
        .collect();
    let tracks = lib
        .tracks_of_videos(&videos)
        .await?
        .into_iter()
        .filter_map(|track| track.ytm_video_id)
        .collect();
    let albums = lib
        .albums_of_browse_ids(&browses)
        .await?
        .into_iter()
        .filter_map(|album| album.ytm_browse_id)
        .collect();
    Ok((tracks, albums))
}

fn cover(url: Option<&str>) -> Option<String> {
    url.map(|url| image_url_at(url, COVER_SIZE))
}

/// `GET /api/hunt?q=`.
pub(crate) async fn search(
    State(state): State<ApiState>,
    session: Session,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<Results>> {
    let q = query.q.trim();
    if q.is_empty() {
        return Ok(Json(Results {
            tracks: Vec::new(),
            albums: Vec::new(),
        }));
    }
    let results = state.hunter.ytmusic().search(q).await.map_err(|error| {
        tracing::warn!(%error, "YouTube Music search failed");
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            format!("YouTube Music did not answer: {error}"),
        )
    })?;
    let (hoarded_tracks, hoarded_albums) = hoarded(&session.library(&state), &results).await?;
    let pending = pixiu_jobs::pending(&mut state.db.clone(), session.owner()).await?;

    let track = |track: &RemoteTrack| TrackResult {
        standing: if hoarded_tracks.contains(&track.id) {
            Standing::Hoarded
        } else if pending.tracks.contains(&track.id) {
            Standing::Pending
        } else {
            Standing::Missing
        },
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist_credit(),
        album: track.album.as_ref().map(|album| album.title.clone()),
        length: track.duration_secs,
        cover: cover(track.cover_url.as_deref()),
        is_video: track.is_video,
    };
    let album = |album: &RemoteAlbum| AlbumResult {
        standing: if hoarded_albums.contains(&album.id) {
            Standing::Hoarded
        } else if pending.has_album(&album.id) {
            Standing::Pending
        } else {
            Standing::Missing
        },
        id: album.id.clone(),
        title: album.title.clone(),
        artist: album.artists.join(", "),
        year: album.year,
        kind: kind_label(album.kind),
        cover: cover(album.cover_url.as_deref()),
    };
    Ok(Json(Results {
        tracks: results.tracks.iter().map(track).collect(),
        albums: results.albums.iter().map(album).collect(),
    }))
}

/// Platform ids are short and plain; anything else is not one.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(Deserialize)]
pub(crate) struct Grab {
    id: String,
    /// How the job is named on the board, e.g. "Artist — Title".
    title: String,
}

/// `POST /api/hunt/tracks`: queues a download.
pub(crate) async fn grab_track(
    State(state): State<ApiState>,
    session: Session,
    Json(grab): Json<Grab>,
) -> ApiResult<StatusCode> {
    if !valid_id(&grab.id) {
        return Err(ApiError::unprocessable("That is not a YouTube Music song."));
    }
    state
        .jobs
        .enqueue(session.owner(), NewJob::track(&grab.id, &grab.title, None))
        .await?;
    Ok(StatusCode::ACCEPTED)
}

/// `POST /api/hunt/albums`: queues grabbing a whole album.
pub(crate) async fn grab_album(
    State(state): State<ApiState>,
    session: Session,
    Json(grab): Json<Grab>,
) -> ApiResult<StatusCode> {
    if !valid_id(&grab.id) {
        return Err(ApiError::unprocessable(
            "That is not a YouTube Music album.",
        ));
    }
    state
        .jobs
        .enqueue(session.owner(), NewJob::album(&grab.id, &grab.title))
        .await?;
    Ok(StatusCode::ACCEPTED)
}
