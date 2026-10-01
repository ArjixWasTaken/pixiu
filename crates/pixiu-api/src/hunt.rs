//! Search YouTube Music and grab what the signed-in user's library lacks.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use pixiu_db::Library;
use pixiu_hunt::{AlbumKind, RemoteAlbum, RemoteTrack, SearchResults, image_url_at};
use pixiu_jobs::NewJob;
use pixiu_subsonic::ids;
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
    /// The album of the library's copy, when the library holds it.
    library_album: Option<String>,
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
    /// The library's copy, when the library holds it.
    library_album: Option<String>,
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

/// The results the library holds: video ids and browse ids, with the album
/// holding each (as a Subsonic id).
async fn hoarded(
    lib: &Library,
    results: &SearchResults,
) -> ApiResult<(HashMap<String, String>, HashMap<String, String>)> {
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
    let held = lib.tracks_of_videos(&videos).await?;
    let tracks = held
        .videos()
        .filter_map(|video| Some((video.to_owned(), ids::album(held.track(video)?.album_id))))
        .collect();
    let albums = lib
        .albums_of_browse_ids(&browses)
        .await?
        .into_iter()
        .filter_map(|album| Some((album.ytm_browse_id?, ids::album(album.id))))
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
        standing: if hoarded_tracks.contains_key(&track.id) {
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
        library_album: hoarded_tracks.get(&track.id).cloned(),
    };
    let album = |album: &RemoteAlbum| AlbumResult {
        standing: if hoarded_albums.contains_key(&album.id) {
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
        library_album: hoarded_albums.get(&album.id).cloned(),
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
