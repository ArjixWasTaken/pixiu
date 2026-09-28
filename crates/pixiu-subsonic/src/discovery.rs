//! `getTopSongs`, `getSimilarSongs` and `getSimilarSongs2`, from the hoard
//! alone: píxiū asks no outside service what is popular or alike.

use pixiu_db::{Album, Track};

use crate::{
    Failure, Params, SubsonicState, catalog,
    catalog::Bind,
    ids::Id,
    lists::Sql,
    response::{Element, Payload},
};

const MAX_COUNT: u32 = 500;

/// `getTopSongs`: an artist's songs, the most played first, then the best
/// rated and starred ones, then in album order.
pub(crate) async fn top_songs(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let name = pixiu_treasury::name_key(params.require("artist")?);
    let count = params.number("count", 50_u32)?.min(MAX_COUNT);
    let mut sql = Sql::new(
        "SELECT tracks.id FROM tracks \
         JOIN artists ON artists.id = tracks.artist_id \
         JOIN albums ON albums.id = tracks.album_id \
         LEFT JOIN annotations ON annotations.item = 'tr-' || tracks.id",
    );
    let name = sql.param(Bind::Text(name));
    sql.push(&format!(
        " WHERE artists.name_key = {name} \
         ORDER BY COALESCE(annotations.play_count, 0) DESC, \
         COALESCE(annotations.rating, 0) DESC, \
         annotations.starred_at IS NULL, \
         albums.year, albums.title_key, tracks.disc_number, tracks.track_number"
    ));
    sql.page(count, 0);

    let mut db = state.db.clone();
    let ids = sql.ids(&mut db).await?;
    let tracks = catalog::tracks_in_order(&mut db, &ids).await?;
    Ok(Element::new("topSongs")
        .list("song", catalog::songs(&mut db, "song", &tracks).await?)
        .into())
}

/// `getSimilarSongs` (an artist, album or song) and `getSimilarSongs2` (an
/// artist, but albums and songs are taken too): songs by the same artists
/// or in the same genres, shuffled.
pub(crate) async fn similar_songs(
    state: &SubsonicState,
    params: &Params,
    id3: bool,
) -> Result<Payload, Failure> {
    let id = Id::parse(params.require("id")?);
    let count = params.number("count", 50_u32)?.min(MAX_COUNT);
    let mut db = state.db.clone();

    // The artists and genres to look for, and a song to leave out.
    let (artists, seed_tracks, skip) = match id {
        Some(Id::Artist(id)) => (vec![id], Vec::new(), None),
        Some(Id::Album(id)) => {
            let album = Album::filter_by_id(id).first().exec(&mut db).await?;
            let tracks = Track::filter_by_album_id(id).exec(&mut db).await?;
            (
                album.map(|album| album.artist_id).into_iter().collect(),
                tracks,
                None,
            )
        }
        Some(Id::Track(id)) => {
            let track = Track::filter_by_id(id).first().exec(&mut db).await?;
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
        let artist_tracks = Track::filter(Track::fields().artist_id().in_list(artists.clone()))
            .exec(&mut db)
            .await?;
        genres = artist_tracks
            .into_iter()
            .filter_map(|track| track.genre)
            .collect();
    }
    genres.sort_unstable();
    genres.dedup();

    let mut sql =
        Sql::new("SELECT tracks.id FROM tracks JOIN albums ON albums.id = tracks.album_id WHERE (");
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
    let tracks = catalog::tracks_in_order(&mut db, &ids).await?;
    Ok(
        Element::new(if id3 { "similarSongs2" } else { "similarSongs" })
            .list("song", catalog::songs(&mut db, "song", &tracks).await?)
            .into(),
    )
}
