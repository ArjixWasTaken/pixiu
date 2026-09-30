//! Listening data: plays (`scrobble`), stars and ratings, shown on songs,
//! albums and artists.

use std::collections::HashMap;

use pixiu_db::{Annotation, Db, Library, now, toasty};

use crate::{
    Cx, Failure, Params,
    ids::{self, Id},
    playing,
    response::{ApiError, Element, ErrorCode, Payload},
};

/// The library owner's annotations of `items`, by item.
async fn by_item(
    lib: &Library,
    items: Vec<String>,
) -> Result<HashMap<String, Annotation>, toasty::Error> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(lib
        .annotations(&items)
        .await?
        .into_iter()
        .map(|annotation| (annotation.item.clone(), annotation))
        .collect())
}

/// Annotations of tracks, by track id.
pub(crate) async fn for_tracks(
    lib: &Library,
    track_ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Annotation>, toasty::Error> {
    let annotations = by_item(lib, track_ids.into_iter().map(ids::track).collect()).await?;
    Ok(annotations
        .into_values()
        .filter_map(|annotation| match Id::parse(&annotation.item) {
            Some(Id::Track(id)) => Some((id, annotation)),
            _ => None,
        })
        .collect())
}

/// Annotations of albums, by album id.
pub(crate) async fn for_albums(
    lib: &Library,
    album_ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Annotation>, toasty::Error> {
    let annotations = by_item(lib, album_ids.into_iter().map(ids::album).collect()).await?;
    Ok(annotations
        .into_values()
        .filter_map(|annotation| match Id::parse(&annotation.item) {
            Some(Id::Album(id)) => Some((id, annotation)),
            _ => None,
        })
        .collect())
}

/// Annotations of artists, by artist id.
pub(crate) async fn for_artists(
    lib: &Library,
    artist_ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Annotation>, toasty::Error> {
    let annotations = by_item(lib, artist_ids.into_iter().map(ids::artist).collect()).await?;
    Ok(annotations
        .into_values()
        .filter_map(|annotation| match Id::parse(&annotation.item) {
            Some(Id::Artist(id)) => Some((id, annotation)),
            _ => None,
        })
        .collect())
}

/// Adds the plays (`playCount`, `played`), star (`starred`) and rating
/// (`userRating`) of a song, album or artist to its element.
pub(crate) fn annotate(element: Element, annotation: Option<&Annotation>) -> Element {
    let Some(annotation) = annotation else {
        return element;
    };
    let element = if annotation.play_count > 0 {
        element.attr("playCount", annotation.play_count).attr_opt(
            "played",
            annotation.last_played.map(|played| played.to_string()),
        )
    } else {
        element
    };
    element
        .attr_opt(
            "starred",
            annotation.starred_at.map(|starred| starred.to_string()),
        )
        .attr_opt("userRating", annotation.rating)
}

async fn record_play(
    db: &mut Db,
    owner: u64,
    item: String,
    at: jiff::Timestamp,
) -> Result<(), toasty::Error> {
    match Annotation::filter_by_user_id_and_item(owner, &item)
        .first()
        .exec(db)
        .await?
    {
        Some(mut annotation) => {
            let last_played = annotation.last_played.map_or(at, |last| last.max(at));
            toasty::update!(annotation {
                play_count: annotation.play_count + 1,
                last_played: Some(last_played),
            })
            .exec(db)
            .await?;
        }
        None => {
            toasty::create!(Annotation {
                user_id: owner,
                item,
                play_count: 1,
                last_played: Some(at),
            })
            .exec(db)
            .await?;
        }
    }
    Ok(())
}

/// `scrobble`: counts plays of songs and their albums. With
/// `submission=false` the client says what it is playing now instead.
pub(crate) async fn scrobble(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let submission = params
        .get("submission")
        .is_none_or(|value| value != "false");
    let mut track_ids = Vec::new();
    for id in params.get_all("id") {
        match Id::parse(id) {
            Some(Id::Track(id)) => track_ids.push(id),
            _ => {
                return Err(
                    ApiError::new(ErrorCode::NotFound, format!("`{id}` is not a song")).into(),
                );
            }
        }
    }
    if track_ids.is_empty() {
        return Err(ApiError::missing_parameter("id").into());
    }
    let mut db = cx.lib.db();
    if !submission {
        let playing = track_ids.last().copied().unwrap_or_default();
        let Some(track) = cx.lib.track(playing).await? else {
            return Err(crate::browse::not_found("song"));
        };
        let player = params.get("c").unwrap_or("unknown");
        cx.state
            .now_playing
            .announced(cx.listener(), player, playing::song(&track));
        return Ok(Payload::default());
    }

    let times: Vec<Option<i64>> = params
        .get_all("time")
        .map(|time| time.parse().ok())
        .collect();
    for (index, id) in track_ids.into_iter().enumerate() {
        let Some(track) = cx.lib.track(id).await? else {
            return Err(crate::browse::not_found("song"));
        };
        let at = times
            .get(index)
            .copied()
            .flatten()
            .and_then(|millis| jiff::Timestamp::from_millisecond(millis).ok())
            .unwrap_or_else(now);
        record_play(&mut db, cx.lib.owner(), ids::track(track.id), at).await?;
        record_play(&mut db, cx.lib.owner(), ids::album(track.album_id), at).await?;
    }
    Ok(Payload::default())
}
