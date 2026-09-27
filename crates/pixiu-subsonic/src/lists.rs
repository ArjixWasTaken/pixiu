//! List endpoints: album lists, random songs and songs by genre.

use crate::{
    Failure, Params, SubsonicState, annotations,
    catalog::{self, Bind},
    response::{ApiError, Element, ErrorCode, Payload},
};

/// Builds SQL with numbered parameters.
#[derive(Default)]
pub(crate) struct Sql {
    pub text: String,
    pub binds: Vec<Bind>,
}

impl Sql {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            binds: Vec::new(),
        }
    }

    /// Registers a parameter, returning its placeholder.
    pub(crate) fn param(&mut self, bind: Bind) -> String {
        self.binds.push(bind);
        format!("?{}", self.binds.len())
    }

    pub(crate) fn push(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Appends `LIMIT .. OFFSET ..`.
    pub(crate) fn page(&mut self, limit: u32, offset: u32) {
        let limit = self.param(Bind::Int(i64::from(limit)));
        let offset = self.param(Bind::Int(i64::from(offset)));
        self.text
            .push_str(&format!(" LIMIT {limit} OFFSET {offset}"));
    }

    pub(crate) async fn ids(self, db: &mut pixiu_db::Db) -> Result<Vec<u64>, Failure> {
        Ok(catalog::select_ids(db, &self.text, self.binds).await?)
    }
}

const MAX_PAGE: u32 = 500;

/// `getAlbumList` (folder model) and `getAlbumList2` (ID3 model).
pub(crate) async fn album_list(
    state: &SubsonicState,
    params: &Params,
    id3: bool,
) -> Result<Payload, Failure> {
    let kind = params.require("type")?;
    let size = params.number("size", 10_u32)?.min(MAX_PAGE);
    let offset = params.number("offset", 0_u32)?;

    let mut sql = Sql::new("SELECT albums.id FROM albums");
    match kind {
        "random" => sql.push(" ORDER BY RANDOM()"),
        "newest" => sql.push(" ORDER BY albums.created_at DESC, albums.id DESC"),
        "alphabeticalByName" => sql.push(" ORDER BY albums.title_key, albums.id"),
        "alphabeticalByArtist" => sql.push(
            " JOIN artists ON artists.id = albums.artist_id \
             ORDER BY artists.name_key, albums.year, albums.title_key",
        ),
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
                " WHERE albums.year BETWEEN {low} AND {high} \
                 ORDER BY albums.year {order}, albums.title_key"
            ));
        }
        "byGenre" => {
            let genre = sql.param(Bind::Text(params.require("genre")?.to_owned()));
            sql.push(&format!(
                " WHERE albums.id IN (SELECT album_id FROM tracks WHERE genre = {genre}) \
                 ORDER BY albums.title_key"
            ));
        }
        "frequent" => sql.push(
            " JOIN annotations ON annotations.item = 'al-' || albums.id \
             WHERE annotations.play_count > 0 \
             ORDER BY annotations.play_count DESC, annotations.last_played DESC",
        ),
        "recent" => sql.push(
            " JOIN annotations ON annotations.item = 'al-' || albums.id \
             WHERE annotations.last_played IS NOT NULL \
             ORDER BY annotations.last_played DESC",
        ),
        // Ratings and favourites arrive later.
        "highest" | "starred" => sql.push(" WHERE 0"),
        other => {
            return Err(ApiError::new(
                ErrorCode::Generic,
                format!("unknown album list type `{other}`"),
            )
            .into());
        }
    }
    sql.page(size, offset);

    let mut db = state.db.clone();
    let album_ids = sql.ids(&mut db).await?;
    let albums = catalog::albums_in_order(&mut db, &album_ids).await?;
    let artists =
        catalog::artists_by_id(&mut db, albums.iter().map(|album| album.artist_id)).await?;
    let stats = catalog::album_stats(&mut db).await?;
    let plays = annotations::for_albums(&mut db, album_ids.iter().copied()).await?;

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

/// `getRandomSongs`.
pub(crate) async fn random_songs(
    state: &SubsonicState,
    params: &Params,
) -> Result<Payload, Failure> {
    let size = params.number("size", 10_u32)?.min(MAX_PAGE);
    let mut sql = Sql::new("SELECT id FROM tracks WHERE 1");
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

    let mut db = state.db.clone();
    let track_ids = sql.ids(&mut db).await?;
    let tracks = catalog::tracks_in_order(&mut db, &track_ids).await?;
    let songs = catalog::songs(&mut db, "song", &tracks).await?;
    Ok(Element::new("randomSongs").list("song", songs).into())
}

/// `getSongsByGenre`.
pub(crate) async fn songs_by_genre(
    state: &SubsonicState,
    params: &Params,
) -> Result<Payload, Failure> {
    let mut sql = Sql::new("SELECT id FROM tracks WHERE genre = ");
    let genre = sql.param(Bind::Text(params.require("genre")?.to_owned()));
    sql.push(&genre);
    sql.push(" ORDER BY album_id, disc_number, track_number, id");
    sql.page(
        params.number("count", 10_u32)?.min(MAX_PAGE),
        params.number("offset", 0_u32)?,
    );

    let mut db = state.db.clone();
    let track_ids = sql.ids(&mut db).await?;
    let tracks = catalog::tracks_in_order(&mut db, &track_ids).await?;
    let songs = catalog::songs(&mut db, "song", &tracks).await?;
    Ok(Element::new("songsByGenre").list("song", songs).into())
}

fn parse_year(value: &str) -> Result<i64, Failure> {
    value
        .parse()
        .map_err(|_| ApiError::new(ErrorCode::Generic, format!("invalid year `{value}`")).into())
}
