//! List endpoints: album lists, random songs and songs by genre, over the
//! caller's library.

use pixiu_db::owned::{Bind, Sql};

use crate::{
    Cx, Failure, Params, annotations, catalog,
    response::{ApiError, Element, ErrorCode, Payload},
};

const MAX_PAGE: u32 = 500;

/// The caller's annotations of albums, joined to `albums`.
const ALBUM_ANNOTATIONS: &str = " JOIN annotations ON annotations.item = 'al-' || albums.id \
                                 AND annotations.user_id = albums.user_id";

/// `getAlbumList` (folder model) and `getAlbumList2` (ID3 model).
pub(crate) async fn album_list(
    cx: &Cx<'_>,
    params: &Params,
    id3: bool,
) -> Result<Payload, Failure> {
    let kind = params.require("type")?;
    let size = params.number("size", 10_u32)?.min(MAX_PAGE);
    let offset = params.number("offset", 0_u32)?;

    let mut sql = cx.lib.sql("SELECT albums.id FROM albums");
    // píxiū's own `singles=false`, which its web player sends: albums only.
    let owned = if params.get("singles") == Some("false") {
        "albums.user_id = ?1 AND NOT albums.single"
    } else {
        "albums.user_id = ?1"
    };
    match kind {
        "random" => sql.push(&format!(" WHERE {owned} ORDER BY RANDOM()")),
        "newest" => sql.push(&format!(
            " WHERE {owned} ORDER BY albums.created_at DESC, albums.id DESC"
        )),
        "alphabeticalByName" => sql.push(&format!(
            " WHERE {owned} ORDER BY albums.title_key, albums.id"
        )),
        "alphabeticalByArtist" => sql.push(&format!(
            " JOIN artists ON artists.id = albums.artist_id WHERE {owned} \
             ORDER BY artists.name_key, albums.year, albums.title_key"
        )),
        "byYear" => {
            let from = params.number::<i64>("fromYear", 0)?;
            let to = params.number::<i64>("toYear", 9999)?;
            // A reversed range asks for newest first.
            let (low, high, order) = if from <= to {
                (from, to, "ASC")
            } else {
                (to, from, "DESC")
            };
            let low = sql.param(Bind::Int(low));
            let high = sql.param(Bind::Int(high));
            sql.push(&format!(
                " WHERE {owned} AND albums.year BETWEEN {low} AND {high} \
                 ORDER BY albums.year {order}, albums.title_key"
            ));
        }
        "byGenre" => {
            let genre = sql.param(Bind::Text(params.require("genre")?.to_owned()));
            sql.push(&format!(
                " WHERE {owned} AND albums.id IN \
                 (SELECT album_id FROM tracks WHERE tracks.user_id = ?1 AND genre = {genre}) \
                 ORDER BY albums.title_key"
            ));
        }
        "frequent" => sql.push(&format!(
            "{ALBUM_ANNOTATIONS} WHERE {owned} AND annotations.play_count > 0 \
             ORDER BY annotations.play_count DESC, annotations.last_played DESC"
        )),
        "recent" => sql.push(&format!(
            "{ALBUM_ANNOTATIONS} WHERE {owned} AND annotations.last_played IS NOT NULL \
             ORDER BY annotations.last_played DESC"
        )),
        "highest" => sql.push(&format!(
            "{ALBUM_ANNOTATIONS} WHERE {owned} AND annotations.rating > 0 \
             ORDER BY annotations.rating DESC, albums.title_key"
        )),
        "starred" => sql.push(&format!(
            "{ALBUM_ANNOTATIONS} WHERE {owned} AND annotations.starred_at IS NOT NULL \
             ORDER BY annotations.starred_at DESC"
        )),
        other => {
            return Err(ApiError::new(
                ErrorCode::Generic,
                format!("unknown album list type `{other}`"),
            )
            .into());
        }
    }
    sql.page(size, offset);

    let album_ids = sql.ids(&mut cx.lib.db()).await?;
    let albums = catalog::albums_in_order(&cx.lib, &album_ids).await?;
    let artists =
        catalog::artists_by_id(&cx.lib, albums.iter().map(|album| album.artist_id)).await?;
    let stats = catalog::album_stats(&cx.lib).await?;
    let plays = annotations::for_albums(&cx.lib, album_ids.iter().copied()).await?;

    let render = |album: &pixiu_db::Album| {
        let artist = artists.get(&album.artist_id);
        let stats = stats.get(&album.id).copied().unwrap_or_default();
        let plays = plays.get(&album.id);
        if id3 {
            catalog::album_id3("album", album, artist, stats, plays)
        } else {
            catalog::album_child("album", album, artist, stats, plays)
        }
    };
    let name = if id3 { "albumList2" } else { "albumList" };
    Ok(Element::new(name)
        .list("album", albums.iter().map(render))
        .into())
}

/// Renders the songs `sql` selects.
async fn songs(cx: &Cx<'_>, sql: Sql) -> Result<Vec<Element>, Failure> {
    let track_ids = sql.ids(&mut cx.lib.db()).await?;
    let tracks = catalog::tracks_in_order(&cx.lib, &track_ids).await?;
    Ok(catalog::songs(&cx.lib, "song", &tracks).await?)
}

/// `getRandomSongs`.
pub(crate) async fn random_songs(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let size = params.number("size", 10_u32)?.min(MAX_PAGE);
    let mut sql = cx
        .lib
        .sql("SELECT id FROM tracks WHERE tracks.user_id = ?1");
    if let Some(genre) = params.get("genre") {
        let genre = sql.param(Bind::Text(genre.to_owned()));
        sql.push(&format!(" AND genre = {genre}"));
    }
    if let Some(from) = params.get("fromYear") {
        let from = sql.param(Bind::Int(parse_year(from)?));
        sql.push(&format!(" AND year >= {from}"));
    }
    if let Some(to) = params.get("toYear") {
        let to = sql.param(Bind::Int(parse_year(to)?));
        sql.push(&format!(" AND year <= {to}"));
    }
    sql.push(" ORDER BY RANDOM()");
    sql.page(size, 0);
    let songs = songs(cx, sql).await?;
    Ok(Element::new("randomSongs").list("song", songs).into())
}

/// `getSongsByGenre`.
pub(crate) async fn songs_by_genre(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let mut sql = cx
        .lib
        .sql("SELECT id FROM tracks WHERE tracks.user_id = ?1 AND genre = ");
    let genre = sql.param(Bind::Text(params.require("genre")?.to_owned()));
    sql.push(&genre);
    sql.push(" ORDER BY album_id, disc_number, track_number, id");
    sql.page(
        params.number("count", 10_u32)?.min(MAX_PAGE),
        params.number("offset", 0_u32)?,
    );
    let songs = songs(cx, sql).await?;
    Ok(Element::new("songsByGenre").list("song", songs).into())
}

fn parse_year(value: &str) -> Result<i64, Failure> {
    value
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::Generic, format!("invalid year `{value}`")).into())
}
