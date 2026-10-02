//! Search YouTube Music and grab what the signed-in user's library lacks.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use pixiu_db::{Library, SourceKey};
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
    id: SourceKey,
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
    id: SourceKey,
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

/// The results the library holds, by key, with the album holding each (as
/// a Subsonic id).
async fn hoarded(
    lib: &Library,
    results: &SearchResults,
) -> ApiResult<(HashMap<SourceKey, String>, HashMap<SourceKey, String>)> {
    let songs: Vec<SourceKey> = results
        .tracks
        .iter()
        .map(|track| track.id.clone())
        .collect();
    let albums: Vec<SourceKey> = results
        .albums
        .iter()
        .map(|album| album.id.clone())
        .collect();
    let held = lib.tracks_of_keys(&songs).await?;
    let tracks = held
        .keys()
        .filter_map(|key| {
            let album = ids::album(held.track(&key)?.album_id);
            Some((key, album))
        })
        .collect();
    let albums = lib
        .albums_of_keys(&albums)
        .await?
        .into_iter()
        .filter_map(|album| {
            let key = SourceKey::from_stored(album.source_key.as_deref())?;
            Some((key, ids::album(album.id)))
        })
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

/// A key from a search result. Platform ids are short and plain; anything
/// else is not one.
fn valid_key(id: &str) -> Option<SourceKey> {
    let key: SourceKey = id.parse().ok()?;
    let plain = key.id().len() <= 64
        && key
            .id()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    plain.then_some(key)
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
    let Some(key) = valid_key(&grab.id) else {
        return Err(ApiError::unprocessable(
            "That is not a song píxiū can download.",
        ));
    };
    state
        .jobs
        .enqueue(session.owner(), NewJob::track(&key, &grab.title, None))
        .await?;
    Ok(StatusCode::ACCEPTED)
}

/// `POST /api/hunt/albums`: queues grabbing a whole album.
pub(crate) async fn grab_album(
    State(state): State<ApiState>,
    session: Session,
    Json(grab): Json<Grab>,
) -> ApiResult<StatusCode> {
    let Some(key) = valid_key(&grab.id) else {
        return Err(ApiError::unprocessable(
            "That is not an album píxiū can download.",
        ));
    };
    state
        .jobs
        .enqueue(session.owner(), NewJob::album(&key, &grab.title))
        .await?;
    Ok(StatusCode::ACCEPTED)
}
