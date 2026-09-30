//! The OpenSubsonic REST API.
//!
//! Served by an axum [`Router`] under `/rest/{method}`, which the binary
//! mounts beside the web player and its API. Every method also answers at
//! `/rest/{method}.view`, the form older clients use.

mod annotations;
mod auth;
mod browse;
mod catalog;
mod discovery;
pub mod ids;
mod lists;
mod lyrics;
mod media;
mod params;
mod playing;
mod playlists;
mod queue;
pub mod render;
pub mod response;
mod search;
pub mod smart;
mod stars;
mod system;

use axum::{
    Router,
    body::Bytes,
    extract::{Path, RawQuery, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use std::sync::Arc;

use pixiu_core::SecretBox;
use pixiu_db::{Db, Library, User, toasty};
use pixiu_treasury::Treasury;
use tokio::sync::Semaphore;
use tower_http::cors::{Any, CorsLayer};

pub use params::Params;
pub use pixiu_core::playing::NowPlaying;
pub use pixiu_media::Codec;
pub use response::{ApiError, Element, ErrorCode, Format, Payload, SubsonicResponse};

/// Shared state for API handlers.
#[derive(Clone)]
pub struct SubsonicState {
    pub db: Db,
    pub treasury: Treasury,
    /// Opens the sealed password token authentication needs.
    pub secrets: SecretBox,
    /// What transcodes use when the client names no format píxiū can make
    /// and the file's own cannot be made either.
    pub transcode_format: Codec,
    /// Permits for transcodes, which run one thread each.
    pub transcodes: Arc<Semaphore>,
    pub now_playing: NowPlaying,
}

impl SubsonicState {
    /// State with the defaults: MP3 transcodes, four at a time.
    #[must_use]
    pub fn new(db: Db, treasury: Treasury, secrets: SecretBox) -> Self {
        Self {
            db,
            treasury,
            secrets,
            transcode_format: Codec::Mp3,
            transcodes: Arc::new(Semaphore::new(4)),
            now_playing: NowPlaying::default(),
        }
    }
}

/// Builds the API router. Paths are absolute (`/rest/...`), so mount it
/// without stripping the prefix.
///
/// Browser-based clients call the API from their own origin, so it answers
/// CORS requests from anywhere. That is safe because credentials travel in
/// the parameters, never in cookies.
pub fn router(state: SubsonicState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::HEAD, Method::POST])
        .allow_headers(Any)
        .expose_headers([
            header::ACCEPT_RANGES,
            header::CONTENT_DISPOSITION,
            header::CONTENT_LENGTH,
            header::CONTENT_RANGE,
        ]);
    Router::new()
        .route("/rest/{method}", get(handle).post(handle))
        .layer(cors)
        .with_state(state)
}

/// A request's context: who asks, and their library, which is all they
/// see.
pub(crate) struct Cx<'a> {
    pub state: &'a SubsonicState,
    pub user: &'a User,
    pub lib: Library,
}

impl Cx<'_> {
    /// The caller, for "now playing".
    pub(crate) fn listener(&self) -> pixiu_core::playing::Listener<'_> {
        pixiu_core::playing::Listener {
            id: self.user.id,
            name: &self.user.username,
        }
    }
}

/// The methods píxiū answers, besides those it answers with nothing
/// (see `system::empty`).
pub const METHODS: &[&str] = &[
    "ping",
    "getLicense",
    "getOpenSubsonicExtensions",
    "getUser",
    "getUsers",
    "tokenInfo",
    "getScanStatus",
    "startScan",
    "getMusicFolders",
    "getIndexes",
    "getArtists",
    "getArtist",
    "getAlbum",
    "getSong",
    "getMusicDirectory",
    "getGenres",
    "getAlbumInfo",
    "getAlbumInfo2",
    "getArtistInfo",
    "getArtistInfo2",
    "getAlbumList",
    "getAlbumList2",
    "getRandomSongs",
    "getSongsByGenre",
    "search2",
    "search3",
    "stream",
    "download",
    "getCoverArt",
    "getPlayQueue",
    "savePlayQueue",
    "getLyrics",
    "getLyricsBySongId",
    "getPlaylists",
    "getPlaylist",
    "createPlaylist",
    "updatePlaylist",
    "deletePlaylist",
    "scrobble",
    "getNowPlaying",
    "star",
    "unstar",
    "setRating",
    "getStarred",
    "getStarred2",
    "getTopSongs",
    "getSimilarSongs",
    "getSimilarSongs2",
];

pub(crate) enum Failure {
    Api(ApiError),
    UnknownMethod,
}

impl From<ApiError> for Failure {
    fn from(error: ApiError) -> Self {
        Failure::Api(error)
    }
}

impl From<toasty::Error> for Failure {
    fn from(error: toasty::Error) -> Self {
        tracing::error!(%error, "database error in the Subsonic API");
        Failure::Api(ApiError::new(ErrorCode::Generic, "internal error"))
    }
}

impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        tracing::error!(%error, "I/O error in the Subsonic API");
        Failure::Api(ApiError::new(ErrorCode::Generic, "internal error"))
    }
}

/// What a method answers with: a Subsonic document, or raw bytes (audio,
/// images).
enum Reply {
    Payload(Payload),
    Raw(Response),
}

impl From<Payload> for Reply {
    fn from(payload: Payload) -> Self {
        Reply::Payload(payload)
    }
}

async fn handle(
    State(state): State<SubsonicState>,
    Path(name): Path<String>,
    RawQuery(query): RawQuery,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let form_body = is_form(&headers).then_some(&body[..]);
    let params = Params::parse(query.as_deref(), form_body);
    let format = params.format();
    let name = name.strip_suffix(".view").unwrap_or(&name);

    match dispatch(&state, name, &params, &method, &headers).await {
        Ok(Reply::Payload(payload)) => SubsonicResponse::ok(format, payload).into_response(),
        Ok(Reply::Raw(response)) => response,
        Err(Failure::Api(error)) => SubsonicResponse::error(format, error).into_response(),
        Err(Failure::UnknownMethod) => {
            tracing::debug!(name, "unknown Subsonic method");
            SubsonicResponse::error(
                format,
                ApiError::new(ErrorCode::Generic, format!("unknown method `{name}`")),
            )
            .with_status(StatusCode::NOT_FOUND)
            .into_response()
        }
    }
}

async fn dispatch(
    state: &SubsonicState,
    name: &str,
    params: &Params,
    method: &Method,
    headers: &HeaderMap,
) -> Result<Reply, Failure> {
    // Clients probe extensions before they have credentials.
    if name == "getOpenSubsonicExtensions" {
        return Ok(system::extensions().into());
    }
    let user = auth::authenticate(state, params).await?;
    let cx = Cx {
        state,
        user: &user,
        lib: Library::new(state.db.clone(), user.id),
    };
    let cx = &cx;

    Ok(match name {
        "ping" => Payload::default().into(),
        "getLicense" => system::license().into(),
        "getUser" => system::user(cx, params).await?.into(),
        "getUsers" => system::users(cx).await?.into(),
        "tokenInfo" => system::token_info(&user).into(),
        "getScanStatus" | "startScan" => system::scan_status(cx).await?.into(),
        "getMusicFolders" => browse::music_folders().into(),
        "getIndexes" => browse::artists(cx, false).await?.into(),
        "getArtists" => browse::artists(cx, true).await?.into(),
        "getArtist" => browse::artist(cx, params).await?.into(),
        "getAlbum" => browse::album(cx, params).await?.into(),
        "getSong" => browse::song(cx, params).await?.into(),
        "getMusicDirectory" => browse::music_directory(cx, params).await?.into(),
        "getGenres" => browse::genres(cx).await?.into(),
        "getAlbumInfo" | "getAlbumInfo2" => browse::album_info(cx, params).await?.into(),
        "getArtistInfo" => browse::artist_info(cx, params, false).await?.into(),
        "getArtistInfo2" => browse::artist_info(cx, params, true).await?.into(),
        "getAlbumList" => lists::album_list(cx, params, false).await?.into(),
        "getAlbumList2" => lists::album_list(cx, params, true).await?.into(),
        "getRandomSongs" => lists::random_songs(cx, params).await?.into(),
        "getSongsByGenre" => lists::songs_by_genre(cx, params).await?.into(),
        "search2" => search::search(cx, params, false).await?.into(),
        "search3" => search::search(cx, params, true).await?.into(),
        "stream" => Reply::Raw(media::stream(cx, params, method, headers, false).await?),
        "download" => Reply::Raw(media::stream(cx, params, method, headers, true).await?),
        "getCoverArt" => Reply::Raw(media::cover_art(cx, params, method, headers).await?),
        "getPlayQueue" => queue::get(cx).await?.into(),
        "getLyrics" => lyrics::by_name(cx, params).await?.into(),
        "getLyricsBySongId" => lyrics::by_song(cx, params).await?.into(),
        "getPlaylists" => playlists::list(cx).await?.into(),
        "getPlaylist" => playlists::get(cx, params).await?.into(),
        "createPlaylist" => playlists::create(cx, params).await?.into(),
        "updatePlaylist" => playlists::update(cx, params).await?.into(),
        "deletePlaylist" => playlists::delete(cx, params).await?.into(),
        "scrobble" => annotations::scrobble(cx, params).await?.into(),
        "getNowPlaying" => playing::now_playing(cx).await?.into(),
        "star" => stars::star(cx, params).await?.into(),
        "unstar" => stars::unstar(cx, params).await?.into(),
        "setRating" => stars::set_rating(cx, params).await?.into(),
        "getStarred" => stars::starred(cx, false).await?.into(),
        "getStarred2" => stars::starred(cx, true).await?.into(),
        "getTopSongs" => discovery::top_songs(cx, params).await?.into(),
        "getSimilarSongs" => discovery::similar_songs(cx, params, false).await?.into(),
        "getSimilarSongs2" => discovery::similar_songs(cx, params, true).await?.into(),
        "savePlayQueue" => queue::save(cx, params).await?.into(),
        other => system::empty(other).ok_or(Failure::UnknownMethod)?.into(),
    })
}

fn is_form(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/x-www-form-urlencoded"))
}
