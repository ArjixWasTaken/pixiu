//! What píxiū knows about a song beyond its tags: the file, where it came
//! from, and why the hoard keeps it.

use axum::{
    Json,
    extract::{Path, State},
};
use pixiu_db::{
    ClaimKind, Db, Lyrics, LyricsSource, Playlist, Track, TrackClaim, TrackOrigin, Watch,
};
use pixiu_subsonic::ids;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session};

/// "Opus · 160 kbps · 48 kHz".
fn format(track: &Track) -> String {
    let mut parts = vec![track.suffix.to_uppercase()];
    if let Some(bitrate) = track.bitrate {
        parts.push(format!("{bitrate} kbps"));
    }
    if let Some(depth) = track.bit_depth {
        parts.push(format!("{depth}-bit"));
    }
    if let Some(rate) = track.sample_rate {
        #[allow(clippy::cast_precision_loss)]
        let khz = f64::from(rate) / 1000.0;
        parts.push(format!(
            "{} kHz",
            format!("{khz:.1}").trim_end_matches(".0")
        ));
    }
    parts.join(" · ")
}

fn lyrics_name(lyrics: Option<&Lyrics>) -> &'static str {
    match lyrics.map(|lyrics| lyrics.source) {
        None => "not_looked_up",
        Some(LyricsSource::File) => "file",
        Some(LyricsSource::Lrclib) => "lrclib",
        Some(LyricsSource::YouTubeMusic) => "youtube_music",
        Some(LyricsSource::Instrumental) => "instrumental",
        Some(LyricsSource::Missing) => "missing",
    }
}

async fn watch_name(db: &mut Db, id: Option<u64>) -> ApiResult<Option<String>> {
    Ok(match id {
        Some(id) => Watch::filter_by_id(id)
            .first()
            .exec(db)
            .await?
            .map(|watch| watch.name),
        None => None,
    })
}

/// Why the hoard keeps the track, for people, each with the watched
/// playlist the song could be excluded from.
async fn kept(db: &mut Db, track: &Track) -> ApiResult<Vec<JsonValue>> {
    let mut reasons: Vec<(String, Option<u64>)> = Vec::new();
    for claim in TrackClaim::filter_by_track_id(track.id).exec(db).await? {
        let reference: Option<u64> = claim.reference.as_deref().and_then(|id| id.parse().ok());
        let why = match claim.kind {
            ClaimKind::Offering => "You uploaded it".to_owned(),
            ClaimKind::ManualGrab => "You downloaded or kept it".to_owned(),
            ClaimKind::Starred => "Starred in an app".to_owned(),
            ClaimKind::WatchPlaylist => match watch_name(db, reference).await? {
                Some(name) => format!("Watched playlist “{name}”"),
                None => "A watched playlist".to_owned(),
            },
            ClaimKind::WatchArtist => match watch_name(db, reference).await? {
                Some(name) => format!("Watched artist “{name}”"),
                None => "A watched artist".to_owned(),
            },
            ClaimKind::LocalPlaylist => {
                let name = match reference {
                    Some(id) => Playlist::filter_by_id(id)
                        .first()
                        .exec(db)
                        .await?
                        .map(|playlist| playlist.name),
                    None => None,
                };
                match name {
                    Some(name) => format!("Playlist “{name}”"),
                    None => "A playlist".to_owned(),
                }
            }
        };
        let excludable = (claim.kind == ClaimKind::WatchPlaylist && track.ytm_video_id.is_some())
            .then_some(reference)
            .flatten();
        reasons.push((why, excludable));
    }
    reasons.dedup();
    Ok(reasons
        .into_iter()
        .map(|(why, watch)| json!({ "why": why, "excludable_from": watch }))
        .collect())
}

/// `GET /api/songs/{id}/info`.
pub(crate) async fn info(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<String>,
) -> ApiResult<Json<JsonValue>> {
    let Some(ids::Id::Track(track_id)) = ids::Id::parse(&id) else {
        return Err(ApiError::not_found("song"));
    };
    let mut db = state.db.clone();
    let Some(track) = Track::filter_by_id(track_id).first().exec(&mut db).await? else {
        return Err(ApiError::not_found("song"));
    };
    let lyrics = Lyrics::filter_by_track_id(track.id)
        .first()
        .exec(&mut db)
        .await?;
    Ok(Json(json!({
        "format": format(&track),
        "size": track.size,
        "origin": match track.origin {
            TrackOrigin::Offering => "offering",
            TrackOrigin::Download => "download",
        },
        "source_name": track.source_name,
        "source_archive": track.source_archive,
        "youtube_url": track
            .ytm_video_id
            .as_ref()
            .map(|video| format!("https://music.youtube.com/watch?v={video}")),
        "mbid": track.mbid,
        "isrc": track.isrc,
        "added_at": track.added_at,
        "lyrics": lyrics_name(lyrics.as_ref()),
        "kept": kept(&mut db, &track).await?,
    })))
}
