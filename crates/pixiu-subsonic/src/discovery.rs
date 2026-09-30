//! `getTopSongs`, `getSimilarSongs` and `getSimilarSongs2`, from the
//! caller's library alone: píxiū asks no outside service what is popular or
//! alike.

use pixiu_db::{Track, owned::Bind};

use crate::{
    Cx, Failure, Params, catalog,
    ids::Id,
    response::{Element, Payload},
};

const MAX_COUNT: u32 = 500;

/// `getTopSongs`: an artist's songs, the most played first, then the best
/// rated and starred ones, then in album order.
pub(crate) async fn top_songs(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let name = pixiu_treasury::name_key(params.require("artist")?);
    let count = params.number("count", 50_u32)?.min(MAX_COUNT);
    let mut sql = cx.lib.sql(
        "SELECT tracks.id FROM tracks \
         JOIN artists ON artists.id = tracks.artist_id \
         JOIN albums ON albums.id = tracks.album_id \
         LEFT JOIN annotations ON annotations.item = 'tr-' || tracks.id \
         AND annotations.user_id = tracks.user_id",
    );
    let name = sql.param(Bind::Text(name));
    sql.push(&format!(
        " WHERE tracks.user_id = ?1 AND artists.name_key = {name} \
         ORDER BY COALESCE(annotations.play_count, 0) DESC, \
         COALESCE(annotations.rating, 0) DESC, \
         annotations.starred_at IS NULL, \
         albums.year, albums.title_key, tracks.disc_number, tracks.track_number"
    ));
    sql.page(count, 0);

    let ids = sql.ids(&mut cx.lib.db()).await?;
    let tracks = catalog::tracks_in_order(&cx.lib, &ids).await?;
    Ok(Element::new("topSongs")
        .list("song", catalog::songs(&cx.lib, "song", &tracks).await?)
        .into())
}

/// `getSimilarSongs` (an artist, album or song) and `getSimilarSongs2` (an
/// artist, but albums and songs are taken too): songs by the same artists
/// or in the same genres, shuffled.
pub(crate) async fn similar_songs(
    cx: &Cx<'_>,
    params: &Params,
    id3: bool,
) -> Result<Payload, Failure> {
    let id = Id::parse(params.require("id")?);
    let count = params.number("count", 50_u32)?.min(MAX_COUNT);
    let lib = &cx.lib;
    let mut db = lib.db();

    // The artists and genres to look for, and a song to leave out.
    let (artists, seed_tracks, skip) = match id {
        Some(Id::Artist(id)) => (
            lib.artist(id)
                .await?
                .map(|artist| artist.id)
                .into_iter()
                .collect(),
            Vec::new(),
            None,
        ),
        Some(Id::Album(id)) => match lib.album(id).await? {
            Some(album) => {
                let tracks = Track::filter_by_album_id(album.id).exec(&mut db).await?;
                (vec![album.artist_id], tracks, None)
            }
            None => (Vec::new(), Vec::new(), None),
        },
        Some(Id::Track(id)) => {
            let track = lib.track(id).await?;
            let artists = track.iter().map(|track| track.artist_id).collect();
            (artists, track.into_iter().collect(), Some(id))
        }
        _ => return Err(crate::browse::not_found("artist, album or song")),
    };
    let mut genres: Vec<String> = seed_tracks
        .iter()
        .filter_map(|track| track.genre.clone())
        .collect();
    if genres.is_empty() {
        let artist_tracks = lib.tracks_of_artists(&artists).await?;
        genres = artist_tracks
            .into_iter()
            .filter_map(|track| track.genre)
            .collect();
    }
    genres.sort_unstable();
    genres.dedup();

    let mut sql = lib.sql(
        "SELECT tracks.id FROM tracks JOIN albums ON albums.id = tracks.album_id \
         WHERE tracks.user_id = ?1 AND (",
    );
    let mut alternatives = Vec::new();
    for artist in artists {
        let artist = sql.param(Bind::Int(i64::try_from(artist).unwrap_or(i64::MAX)));
        alternatives.push(format!(
            "tracks.artist_id = {artist} OR albums.artist_id = {artist}"
        ));
    }
    for genre in genres {
        let genre = sql.param(Bind::Text(genre));
        alternatives.push(format!("tracks.genre = {genre}"));
    }
    if alternatives.is_empty() {
        alternatives.push("0".to_owned());
    }
    sql.push(&alternatives.join(" OR "));
    sql.push(")");
    if let Some(skip) = skip {
        let skip = sql.param(Bind::Int(i64::try_from(skip).unwrap_or(i64::MAX)));
        sql.push(&format!(" AND tracks.id != {skip}"));
    }
    sql.push(" ORDER BY RANDOM()");
    sql.page(count, 0);

    let ids = sql.ids(&mut db).await?;
    let tracks = catalog::tracks_in_order(lib, &ids).await?;
    Ok(
        Element::new(if id3 { "similarSongs2" } else { "similarSongs" })
            .list("song", catalog::songs(lib, "song", &tracks).await?)
            .into(),
    )
}
