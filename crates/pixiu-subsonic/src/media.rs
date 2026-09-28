//! Binary endpoints: `stream`, `download` and `getCoverArt`.
//!
//! Files are served by tower-http's `ServeFile`, which handles `Range`
//! requests (seeking) and conditional requests.

use std::path::Path;

use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, header},
    response::Response,
};
use pixiu_db::{Album, Track};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{Failure, Params, SubsonicState, browse::not_found, ids::Id};

/// Request headers that matter for serving a file.
const FORWARDED: [HeaderName; 6] = [
    header::RANGE,
    header::IF_RANGE,
    header::IF_MATCH,
    header::IF_NONE_MATCH,
    header::IF_MODIFIED_SINCE,
    header::IF_UNMODIFIED_SINCE,
];

async fn serve_file(path: &Path, method: &Method, headers: &HeaderMap) -> Response {
    let mut request = Request::new(Body::empty());
    *request.method_mut() = method.clone();
    for name in FORWARDED {
        if let Some(value) = headers.get(&name) {
            request.headers_mut().insert(name, value.clone());
        }
    }
    match ServeFile::new(path).oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(never) => match never {},
    }
}

async fn load_track(state: &SubsonicState, params: &Params) -> Result<Track, Failure> {
    let Some(Id::Track(id)) = Id::parse(params.require("id")?) else {
        return Err(not_found("song"));
    };
    Track::filter_by_id(id)
        .first()
        .exec(&mut state.db.clone())
        .await?
        .ok_or_else(|| not_found("song"))
}

/// `stream` and `download`: the original file. Transcoding (`format`,
/// `maxBitRate`) is not supported yet, so it is ignored.
pub(crate) async fn stream(
    state: &SubsonicState,
    params: &Params,
    method: &Method,
    headers: &HeaderMap,
    download: bool,
) -> Result<Response, Failure> {
    let track = load_track(state, params).await?;
    let path = state.treasury.resolve(&track.path);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        tracing::warn!(path = %path.display(), "audio file missing from the treasure");
        return Err(not_found("audio file"));
    }

    let mut response = serve_file(&path, method, headers).await;
    if response.status().is_success() {
        if let Ok(content_type) = HeaderValue::from_str(&track.content_type) {
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, content_type);
        }
        if download {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Ok(disposition) = HeaderValue::from_str(&attachment(&name)) {
                response
                    .headers_mut()
                    .insert(header::CONTENT_DISPOSITION, disposition);
            }
        }
    }
    Ok(response)
}

/// A `Content-Disposition` naming the file, with an ASCII fallback
/// (RFC 6266 / RFC 5987).
fn attachment(name: &str) -> String {
    let fallback: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

/// `getCoverArt`: an album's cover (also for its songs and its artist),
/// resized when `size` is given.
pub(crate) async fn cover_art(
    state: &SubsonicState,
    params: &Params,
    method: &Method,
    headers: &HeaderMap,
) -> Result<Response, Failure> {
    let mut db = state.db.clone();
    let album = match Id::parse(params.require("id")?) {
        Some(Id::Album(id)) => Album::filter_by_id(id).first().exec(&mut db).await?,
        Some(Id::Track(id)) => match Track::filter_by_id(id).first().exec(&mut db).await? {
            Some(track) => {
                Album::filter_by_id(track.album_id)
                    .first()
                    .exec(&mut db)
                    .await?
            }
            None => None,
        },
        Some(Id::Artist(id)) => {
            let mut albums = Album::filter_by_artist_id(id).exec(&mut db).await?;
            albums.sort_by_key(|album| std::cmp::Reverse(album.year));
            albums.into_iter().find(|album| album.cover.is_some())
        }
        Some(Id::Playlist(id)) => crate::playlists::cover_album(&mut db, id).await?,
        None => None,
    };
    let Some(cover) = album.and_then(|album| album.cover) else {
        return Err(not_found("cover art"));
    };

    let size = params.get("size").and_then(|size| size.parse().ok());
    let path = pixiu_treasury::covers::sized(
        &state.treasury.resolve(&cover),
        state.treasury.cache_dir(),
        size,
    )
    .await?;
    Ok(serve_file(&path, method, headers).await)
}

#[cfg(test)]
mod tests {
    use super::attachment;

    #[test]
    fn attachments_escape_names() {
        assert_eq!(
            attachment("01 Ünï \"x\".flac"),
            "attachment; filename=\"01 _n_ _x_.flac\"; \
             filename*=UTF-8''01%20%C3%9Cn%C3%AF%20%22x%22.flac"
        );
    }
}
