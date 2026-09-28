//! Binary endpoints: `stream`, `download` and `getCoverArt`.
//!
//! Files are served by tower-http's `ServeFile`, which handles `Range`
//! requests (seeking) and conditional requests. Transcoded streams are made
//! as they are sent; clients seek in them with `timeOffset`.

use std::{
    os::fd::OwnedFd,
    path::{Path, PathBuf},
    time::Duration,
};

use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, header},
    response::Response,
};
use pixiu_db::{Album, Artist, Track, User};
use pixiu_media::{Codec, Target};
use tokio_util::io::ReaderStream;
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

/// How to serve a stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Plan {
    Original,
    Transcode { target: Target, offset: Duration },
}

/// Picks the original file (with extension `suffix`, of `bitrate` kbps)
/// unless the client asks for another format (`format`), a lower bitrate
/// (`maxBitRate`), or a later start (`timeOffset`). Transcodes keep the
/// file's codec when píxiū can make it, else use `fallback`. `format=raw`
/// always gets the original.
pub(crate) fn plan(suffix: &str, bitrate: Option<u32>, params: &Params, fallback: Codec) -> Plan {
    let format = params.get("format");
    if format.is_some_and(|format| format.eq_ignore_ascii_case("raw")) {
        return Plan::Original;
    }
    let requested = format.and_then(Codec::from_format);
    let source = Codec::from_suffix(suffix);
    let max_bitrate = params
        .get("maxBitRate")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|&kbps| kbps > 0);
    let offset = params
        .get("timeOffset")
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|&seconds| seconds.is_finite() && seconds > 0.0);
    let too_rich = max_bitrate.is_some_and(|max| bitrate.is_none_or(|rate| rate > max));

    let codec = match requested {
        Some(codec) if Some(codec) != source => codec,
        _ if too_rich || offset.is_some() => requested.or(source).unwrap_or(fallback),
        _ => return Plan::Original,
    };
    Plan::Transcode {
        target: Target {
            codec,
            bitrate: codec.clamp_bitrate(max_bitrate.unwrap_or_else(|| codec.default_bitrate())),
        },
        offset: offset.map(Duration::from_secs_f64).unwrap_or_default(),
    }
}

/// A transcode, streamed as FFmpeg makes it. It runs on a thread of its own
/// (at most `state.transcodes` at once) and stops when the client leaves.
async fn transcoded(
    state: &SubsonicState,
    path: PathBuf,
    target: Target,
    offset: Duration,
    method: &Method,
) -> Result<Response, Failure> {
    let response = Response::builder()
        .header(header::CONTENT_TYPE, target.codec.content_type())
        .header(header::ACCEPT_RANGES, "none");
    if method == Method::HEAD {
        return Ok(response.body(Body::empty()).expect("a valid response"));
    }

    let permit = state
        .transcodes
        .clone()
        .acquire_owned()
        .await
        .expect("the transcode semaphore is never closed");
    let (reader, writer) = std::io::pipe()?;
    let reader = tokio::net::unix::pipe::Receiver::from_owned_fd(OwnedFd::from(reader))?;
    std::thread::Builder::new()
        .name("transcode".to_owned())
        .spawn(move || {
            let _permit = permit;
            if let Err(error) = pixiu_media::transcode(&path, target, offset, &writer) {
                // Usually the client moving on (the pipe closed).
                tracing::debug!(%error, path = %path.display(), "transcode ended early");
            }
        })?;
    Ok(response
        .body(Body::from_stream(ReaderStream::new(reader)))
        .expect("a valid response"))
}

/// `stream` and `download`: the original file, or for `stream` a transcode
/// when the client asks for one (see [`plan`]).
pub(crate) async fn stream(
    state: &SubsonicState,
    user: &User,
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
    if download {
        return Ok(original(&track, &path, method, headers, true).await);
    }

    if method != Method::HEAD
        && let Some(player) = params.get("c")
    {
        state
            .now_playing
            .streamed(&user.username, player, (&track).into());
    }
    match plan(&track.suffix, track.bitrate, params, state.transcode_format) {
        Plan::Original => Ok(original(&track, &path, method, headers, false).await),
        Plan::Transcode { target, offset } => transcoded(state, path, target, offset, method).await,
    }
}

/// The file itself, with its content type (and a file name to save it
/// under, for downloads).
async fn original(
    track: &Track,
    path: &Path,
    method: &Method,
    headers: &HeaderMap,
    download: bool,
) -> Response {
    let mut response = serve_file(path, method, headers).await;
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
    response
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
    let size = params.get("size").and_then(|size| size.parse().ok());
    let id = Id::parse(params.require("id")?);
    // An artist's own picture, when píxiū has one.
    if let Some(Id::Artist(artist_id)) = id
        && let Some(image) = Artist::filter_by_id(artist_id)
            .first()
            .exec(&mut db)
            .await?
            .and_then(|artist| artist.image)
    {
        let path = pixiu_treasury::covers::sized(
            &state.treasury.cache_dir().join(image),
            state.treasury.cache_dir(),
            size,
        )
        .await?;
        return Ok(serve_file(&path, method, headers).await);
    }
    let album = match id {
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
    use super::*;

    #[test]
    fn attachments_escape_names() {
        assert_eq!(
            attachment("01 Ünï \"x\".flac"),
            "attachment; filename=\"01 _n_ _x_.flac\"; \
             filename*=UTF-8''01%20%C3%9Cn%C3%AF%20%22x%22.flac"
        );
    }

    fn plan_for(suffix: &str, bitrate: u32, query: &str) -> Plan {
        plan(
            suffix,
            Some(bitrate),
            &Params::parse(Some(query), None),
            Codec::Mp3,
        )
    }

    fn transcode(codec: Codec, bitrate: u32, offset: f64) -> Plan {
        Plan::Transcode {
            target: Target { codec, bitrate },
            offset: Duration::from_secs_f64(offset),
        }
    }

    #[test]
    fn originals_are_kept_unless_asked_otherwise() {
        assert_eq!(plan_for("opus", 140, ""), Plan::Original);
        assert_eq!(plan_for("opus", 140, "format=opus"), Plan::Original);
        assert_eq!(plan_for("opus", 140, "maxBitRate=320"), Plan::Original);
        assert_eq!(plan_for("opus", 140, "maxBitRate=0"), Plan::Original);
        assert_eq!(plan_for("flac", 900, "format=flac"), Plan::Original);
        assert_eq!(
            plan_for("opus", 140, "format=raw&maxBitRate=64&timeOffset=30"),
            Plan::Original
        );
    }

    #[test]
    fn transcodes_follow_the_request() {
        // Another format, at its usual bitrate or the one asked for.
        assert_eq!(
            plan_for("opus", 140, "format=mp3"),
            transcode(Codec::Mp3, 192, 0.0)
        );
        assert_eq!(
            plan_for("flac", 900, "format=aac&maxBitRate=256"),
            transcode(Codec::Aac, 256, 0.0)
        );
        // A lower bitrate keeps the codec when píxiū can make it...
        assert_eq!(
            plan_for("opus", 140, "maxBitRate=96"),
            transcode(Codec::Opus, 96, 0.0)
        );
        // ...else takes the fallback.
        assert_eq!(
            plan_for("flac", 900, "maxBitRate=320"),
            transcode(Codec::Mp3, 320, 0.0)
        );
        // A later start needs a transcode too.
        assert_eq!(
            plan_for("mp3", 320, "timeOffset=61.5"),
            transcode(Codec::Mp3, 192, 61.5)
        );
        // Bitrates stay within what the encoder does well.
        assert_eq!(
            plan_for("flac", 900, "format=mp3&maxBitRate=1000"),
            transcode(Codec::Mp3, 320, 0.0)
        );
    }
}
