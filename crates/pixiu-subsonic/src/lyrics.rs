//! Lyrics: `getLyrics` (by artist and title, plain text) and OpenSubsonic's
//! `getLyricsBySongId` (structured, time-synced when known).

use pixiu_db::{Db, Lyrics, toasty};
use pixiu_enrich::{parse_lrc, plain_from_lrc};

use crate::{
    Cx, Failure, Params,
    ids::Id,
    response::{ApiError, Element, ErrorCode, Payload},
};

async fn lyrics_of(db: &mut Db, track_id: u64) -> Result<Option<Lyrics>, toasty::Error> {
    Ok(Lyrics::filter_by_track_id(track_id)
        .first()
        .exec(db)
        .await?
        .filter(|lyrics| lyrics.source.has_words()))
}

/// The lyrics as plain text.
fn plain(lyrics: &Lyrics) -> Option<String> {
    lyrics
        .plain
        .clone()
        .or_else(|| lyrics.synced.as_deref().map(plain_from_lrc))
}

/// `getLyrics`: the first track of the caller's by that artist with that
/// title.
pub(crate) async fn by_name(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let mut db = cx.lib.db();
    let artist = params.get("artist").unwrap_or_default();
    let title = params.get("title").unwrap_or_default();
    let wanted_title = pixiu_treasury::name_key(title);
    let wanted_artist = pixiu_treasury::name_key(artist);

    let mut found = None;
    if !wanted_title.is_empty() {
        let tracks = cx.lib.all_tracks().await?;
        for track in tracks.into_iter().filter(|track| {
            pixiu_treasury::name_key(&track.title) == wanted_title
                && pixiu_treasury::name_key(&track.artist_credit).contains(&wanted_artist)
        }) {
            if let Some(lyrics) = lyrics_of(&mut db, track.id).await? {
                found = Some((track, lyrics));
                break;
            }
        }
    }
    let mut element = Element::new("lyrics");
    if let Some((track, lyrics)) = found {
        element = element
            .attr("artist", track.artist_credit.as_str())
            .attr("title", track.title.as_str());
        if let Some(text) = plain(&lyrics) {
            element = element.text(text);
        }
    }
    Ok(element.into())
}

/// `getLyricsBySongId`: time-synced lines when known, else plain ones.
pub(crate) async fn by_song(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let mut db = cx.lib.db();
    let id = params.require("id")?;
    let Some(Id::Track(track_id)) = Id::parse(id) else {
        return Err(ApiError::new(ErrorCode::NotFound, format!("`{id}` is not a song")).into());
    };
    let Some(track) = cx.lib.track(track_id).await? else {
        return Err(ApiError::new(ErrorCode::NotFound, "song not found").into());
    };

    let mut structured = Vec::new();
    if let Some(lyrics) = lyrics_of(&mut db, track.id).await? {
        let synced = lyrics
            .synced
            .as_deref()
            .map(parse_lrc)
            .filter(|lines| !lines.is_empty());
        let entry = Element::new("structuredLyrics")
            .attr("displayArtist", track.artist_credit.as_str())
            .attr("displayTitle", track.title.as_str())
            .attr("lang", "und")
            .attr("synced", synced.is_some());
        let entry = match synced {
            Some(lines) => entry.list(
                "line",
                lines.into_iter().map(|line| {
                    Element::new("line")
                        .attr("start", line.start_ms)
                        .text(line.text)
                }),
            ),
            None => entry.list(
                "line",
                plain(&lyrics)
                    .unwrap_or_default()
                    .lines()
                    .map(|line| Element::new("line").text(line))
                    .collect::<Vec<_>>(),
            ),
        };
        structured.push(entry);
    }
    Ok(Element::new("lyricsList")
        .list("structuredLyrics", structured)
        .into())
}
