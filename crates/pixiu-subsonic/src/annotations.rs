//! Listening data: plays (`scrobble`), stars and ratings, shown on songs,
//! albums and artists.

use std::collections::HashMap;

use pixiu_db::{Annotation, Db, Track, User, now, toasty};

use crate::{
    Failure, Params, SubsonicState,
    ids::{self, Id},
    response::{ApiError, Element, ErrorCode, Payload},
};

async fn by_item(
    db: &mut Db,
    items: Vec<String>,
) -> Result<HashMap<String, Annotation>, toasty::Error> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(
        Annotation::filter(Annotation::fields().item().in_list(items))
            .exec(db)
            .await?
            .into_iter()
            .map(|annotation| (annotation.item.clone(), annotation))
            .collect(),
    )
}

/// Annotations of tracks, by track id.
pub(crate) async fn for_tracks(
    db: &mut Db,
    track_ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Annotation>, toasty::Error> {
    let annotations = by_item(db, track_ids.into_iter().map(ids::track).collect()).await?;
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
    db: &mut Db,
    album_ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Annotation>, toasty::Error> {
    let annotations = by_item(db, album_ids.into_iter().map(ids::album).collect()).await?;
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
    db: &mut Db,
    artist_ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Annotation>, toasty::Error> {
    let annotations = by_item(db, artist_ids.into_iter().map(ids::artist).collect()).await?;
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

async fn record_play(db: &mut Db, item: String, at: jiff::Timestamp) -> Result<(), toasty::Error> {
    match Annotation::filter_by_item(&item).first().exec(db).await? {
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
pub(crate) async fn scrobble(
    state: &SubsonicState,
    user: &User,
    params: &Params,
) -> Result<Payload, Failure> {
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
    let mut db = state.db.clone();
    if !submission {
        let playing = track_ids.last().copied().unwrap_or_default();
        let Some(track) = Track::filter_by_id(playing).first().exec(&mut db).await? else {
            return Err(crate::browse::not_found("song"));
        };
        let player = params.get("c").unwrap_or("unknown");
        state
            .now_playing
            .announced(&user.username, player, (&track).into());
        return Ok(Payload::default());
    }

    let times: Vec<Option<i64>> = params
        .get_all("time")
        .map(|time| time.parse().ok())
        .collect();
    for (index, id) in track_ids.into_iter().enumerate() {
        let Some(track) = Track::filter_by_id(id).first().exec(&mut db).await? else {
            return Err(crate::browse::not_found("song"));
        };
        let at = times
            .get(index)
            .copied()
            .flatten()
            .and_then(|millis| jiff::Timestamp::from_millisecond(millis).ok())
            .unwrap_or_else(now);
        record_play(&mut db, ids::track(track.id), at).await?;
        record_play(&mut db, ids::album(track.album_id), at).await?;
    }
    Ok(Payload::default())
}
