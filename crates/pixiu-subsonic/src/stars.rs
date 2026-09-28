//! Stars and ratings: `star`, `unstar`, `setRating`, `getStarred` and
//! `getStarred2`.
//!
//! A starred song or album is something to keep: its tracks get a `Starred`
//! claim (referring to the song or album), so they never become orphans
//! while starred, even after leaving a watched playlist.

use jiff::Timestamp;
use pixiu_db::{Album, Annotation, Artist, ClaimKind, Db, Track, now, toasty};
use pixiu_treasury::Claim;

use crate::{
    Failure, Params, SubsonicState, annotations,
    browse::not_found,
    catalog,
    ids::Id,
    response::{ApiError, Element, ErrorCode, Payload},
};

/// The songs, albums and artists a `star` or `unstar` call names.
fn items(params: &Params) -> Result<Vec<Id>, Failure> {
    let mut items = Vec::new();
    for name in ["id", "albumId", "artistId"] {
        for value in params.get_all(name) {
            match Id::parse(value) {
                Some(id @ (Id::Track(_) | Id::Album(_) | Id::Artist(_))) => items.push(id),
                _ => {
                    return Err(ApiError::new(
                        ErrorCode::NotFound,
                        format!("`{value}` is not a song, album or artist"),
                    )
                    .into());
                }
            }
        }
    }
    if items.is_empty() {
        return Err(ApiError::missing_parameter("id").into());
    }
    Ok(items)
}

async fn exists(db: &mut Db, id: Id) -> Result<bool, toasty::Error> {
    Ok(match id {
        Id::Track(id) => Track::filter_by_id(id).first().exec(db).await?.is_some(),
        Id::Album(id) => Album::filter_by_id(id).first().exec(db).await?.is_some(),
        Id::Artist(id) => Artist::filter_by_id(id).first().exec(db).await?.is_some(),
        Id::Playlist(_) => false,
    })
}

/// Sets an item's star and rating, creating its annotation if needed.
async fn annotate(
    db: &mut Db,
    id: Id,
    change: impl FnOnce(Option<Timestamp>, Option<u8>) -> (Option<Timestamp>, Option<u8>),
) -> Result<(), toasty::Error> {
    let item = id.to_string();
    match Annotation::filter_by_item(&item).first().exec(db).await? {
        Some(mut annotation) => {
            let (starred_at, rating) = change(annotation.starred_at, annotation.rating);
            toasty::update!(annotation { starred_at, rating })
                .exec(db)
                .await?;
        }
        None => {
            let (starred_at, rating) = change(None, None);
            toasty::create!(Annotation {
                item,
                play_count: 0,
                starred_at,
                rating,
            })
            .exec(db)
            .await?;
        }
    }
    Ok(())
}

/// The tracks a star on `id` keeps.
async fn kept_tracks(db: &mut Db, id: Id) -> Result<Vec<u64>, toasty::Error> {
    Ok(match id {
        Id::Track(id) => vec![id],
        Id::Album(id) => Track::filter_by_album_id(id)
            .exec(db)
            .await?
            .into_iter()
            .map(|track| track.id)
            .collect(),
        Id::Artist(_) | Id::Playlist(_) => Vec::new(),
    })
}

/// `star`: stars songs, albums and artists (again: the first star stays).
pub(crate) async fn star(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let items = items(params)?;
    let mut db = state.db.clone();
    for &id in &items {
        if !exists(&mut db, id).await? {
            return Err(not_found("item"));
        }
    }
    for id in items {
        annotate(&mut db, id, |starred, rating| {
            (starred.or_else(|| Some(now())), rating)
        })
        .await?;
        let claim = Claim {
            kind: ClaimKind::Starred,
            reference: Some(id.to_string()),
        };
        for track_id in kept_tracks(&mut db, id).await? {
            state.treasury.claim(track_id, &claim).await?;
        }
    }
    Ok(Payload::default())
}

/// `unstar`: takes stars (and the claims that came with them) back.
pub(crate) async fn unstar(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let mut db = state.db.clone();
    for id in items(params)? {
        annotate(&mut db, id, |_, rating| (None, rating)).await?;
        state
            .treasury
            .release(ClaimKind::Starred, &id.to_string(), |_| false)
            .await?;
    }
    Ok(Payload::default())
}

/// `setRating`: 1 to 5 stars, or 0 to clear the rating.
pub(crate) async fn set_rating(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let value = params.require("id")?;
    let Some(id @ (Id::Track(_) | Id::Album(_) | Id::Artist(_))) = Id::parse(value) else {
        return Err(not_found("item"));
    };
    let rating = params
        .require("rating")?
        .parse::<u8>()
        .ok()
        .filter(|rating| *rating <= 5)
        .ok_or_else(|| ApiError::new(ErrorCode::Generic, "a rating is 0 to 5"))?;
    let mut db = state.db.clone();
    if !exists(&mut db, id).await? {
        return Err(not_found("item"));
    }
    annotate(&mut db, id, |starred, _| {
        (starred, (rating > 0).then_some(rating))
    })
    .await?;
    Ok(Payload::default())
}

/// `getStarred` (folder model) and `getStarred2` (ID3 model): what is
/// starred, the latest first.
pub(crate) async fn starred(state: &SubsonicState, id3: bool) -> Result<Payload, Failure> {
    let mut db = state.db.clone();
    let mut starred: Vec<Annotation> = Annotation::all()
        .exec(&mut db)
        .await?
        .into_iter()
        .filter(|annotation| annotation.starred_at.is_some())
        .collect();
    starred.sort_by_key(|annotation| std::cmp::Reverse(annotation.starred_at));
    let (mut artist_ids, mut album_ids, mut track_ids) = (Vec::new(), Vec::new(), Vec::new());
    for annotation in &starred {
        match Id::parse(&annotation.item) {
            Some(Id::Artist(id)) => artist_ids.push(id),
            Some(Id::Album(id)) => album_ids.push(id),
            Some(Id::Track(id)) => track_ids.push(id),
            _ => {}
        }
    }

    let artists = catalog::artists_in_order(&mut db, &artist_ids).await?;
    let artist_annotations = annotations::for_artists(&mut db, artist_ids).await?;
    let all_albums = Album::all().exec(&mut db).await?;
    let summaries = catalog::summarize_artists(&all_albums);
    let artist_elements: Vec<Element> = artists
        .iter()
        .map(|artist| {
            let annotation = artist_annotations.get(&artist.id);
            if id3 {
                let summary = summaries.get(&artist.id);
                catalog::artist_id3(
                    "artist",
                    artist,
                    summary.map_or(0, |summary| summary.albums),
                    summary.is_some_and(|summary| summary.has_cover),
                    annotation,
                )
            } else {
                catalog::artist_folder("artist", artist, annotation)
            }
        })
        .collect();

    let albums = catalog::albums_in_order(&mut db, &album_ids).await?;
    let album_artists =
        catalog::artists_by_id(&mut db, albums.iter().map(|album| album.artist_id)).await?;
    let stats = catalog::album_stats(&mut db).await?;
    let album_annotations = annotations::for_albums(&mut db, album_ids).await?;
    let album_elements: Vec<Element> = albums
        .iter()
        .map(|album| {
            let artist = album_artists.get(&album.artist_id);
            let stats = stats.get(&album.id).copied().unwrap_or_default();
            let annotation = album_annotations.get(&album.id);
            if id3 {
                catalog::album_id3("album", album, artist, stats, annotation)
            } else {
                catalog::album_child("album", album, artist, stats, annotation)
            }
        })
        .collect();

    let tracks = catalog::tracks_in_order(&mut db, &track_ids).await?;
    let songs = catalog::songs(&mut db, "song", &tracks).await?;

    Ok(Element::new(if id3 { "starred2" } else { "starred" })
        .list("artist", artist_elements)
        .list("album", album_elements)
        .list("song", songs)
        .into())
}
