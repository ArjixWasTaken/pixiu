//! The library in the orders koel's screens list it: albums, artists and
//! songs sorted by any column and paged with a cursor, plus recently
//! played songs. Items are Subsonic JSON (see [`pixiu_subsonic::render`]).

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use pixiu_db::{
    Db,
    toasty::{self, stmt::Value},
};
use pixiu_subsonic::{ids, render};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session};

/// Items per page.
const PAGE: u32 = 60;

/// The most items one request may ask for (e.g. a queue of all songs).
const MAX_LIMIT: u32 = 500;

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    sort: Option<String>,
    order: Option<String>,
    /// Where the previous page ended: an offset, opaque to the player.
    cursor: Option<String>,
    limit: Option<u32>,
    #[serde(default)]
    favorites_only: bool,
    genre: Option<String>,
    /// A Subsonic artist id: songs by the artist or on their albums.
    artist: Option<String>,
}

impl ListQuery {
    fn descending(&self) -> bool {
        self.order.as_deref() == Some("desc")
    }

    fn offset(&self) -> u32 {
        self.cursor
            .as_deref()
            .and_then(|cursor| cursor.parse().ok())
            .unwrap_or(0)
    }

    fn limit(&self) -> u32 {
        self.limit.unwrap_or(PAGE).clamp(1, MAX_LIMIT)
    }
}

/// SQL with numbered parameters.
#[derive(Default)]
struct Sql {
    text: String,
    binds: Vec<Bind>,
}

enum Bind {
    Int(i64),
    Text(String),
}

impl Sql {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            binds: Vec::new(),
        }
    }

    fn param(&mut self, bind: Bind) -> String {
        self.binds.push(bind);
        format!("?{}", self.binds.len())
    }

    fn push(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Runs the query, whose first column is an id.
    async fn ids(self, db: &mut Db) -> Result<Vec<u64>, toasty::Error> {
        let mut query = toasty::sql::query(&self.text);
        for bind in self.binds {
            query = match bind {
                Bind::Int(value) => query.bind(value),
                Bind::Text(value) => query.bind(value),
            };
        }
        Ok(query
            .exec(db)
            .await?
            .iter()
            .filter_map(|row| match row {
                Value::Record(record) => match &record[0] {
                    Value::I64(id) => u64::try_from(*id).ok(),
                    Value::U64(id) => Some(*id),
                    _ => None,
                },
                _ => None,
            })
            .collect())
    }
}

fn bad_sort(field: &str) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        format!("cannot sort by `{field}`"),
    )
}

/// Runs a page of `sql` (ordered, without `LIMIT`), rendering the ids it
/// selects, as koel's cursor pages: `{data, meta: {next_cursor}}`.
async fn page<F>(
    state: &ApiState,
    mut sql: Sql,
    query: &ListQuery,
    render: F,
) -> ApiResult<Json<JsonValue>>
where
    F: AsyncFnOnce(&mut Db, &[u64]) -> Result<Vec<JsonValue>, toasty::Error>,
{
    let (offset, limit) = (query.offset(), query.limit());
    // One more than asked, to tell whether another page follows.
    let limit_param = sql.param(Bind::Int(i64::from(limit) + 1));
    let offset_param = sql.param(Bind::Int(i64::from(offset)));
    sql.push(&format!(" LIMIT {limit_param} OFFSET {offset_param}"));

    let mut db = state.db.clone();
    let mut ids = sql.ids(&mut db).await?;
    let more = ids.len() > limit as usize;
    ids.truncate(limit as usize);
    let data = render(&mut db, &ids).await?;
    let next_cursor = more.then(|| (offset + limit).to_string());
    Ok(Json(json!({
        "data": data,
        "meta": { "per_page": limit, "next_cursor": next_cursor },
    })))
}

/// `GET /api/albums`.
pub(crate) async fn albums(
    State(state): State<ApiState>,
    _: Session,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<JsonValue>> {
    let field = query.sort.as_deref().unwrap_or("name");
    let key = match field {
        "name" => "albums.title_key",
        "year" => "albums.year",
        "artist_name" => "artists.name_key",
        "created_at" => "albums.created_at",
        "length" => "(SELECT SUM(duration_ms) FROM tracks WHERE tracks.album_id = albums.id)",
        "rating" => "COALESCE(an.rating, 0)",
        "favorite" => "an.starred_at IS NOT NULL",
        other => return Err(bad_sort(other)),
    };
    let order = if query.descending() { "DESC" } else { "ASC" };
    let mut sql = Sql::new(
        "SELECT albums.id FROM albums \
         JOIN artists ON artists.id = albums.artist_id \
         LEFT JOIN annotations an ON an.item = 'al-' || albums.id",
    );
    if query.favorites_only {
        sql.push(" WHERE an.starred_at IS NOT NULL");
    }
    sql.push(&format!(
        " ORDER BY {key} {order}, albums.title_key, albums.id"
    ));
    page(&state, sql, &query, render::albums).await
}

/// `GET /api/artists`: album artists, as the Subsonic API lists them.
pub(crate) async fn artists(
    State(state): State<ApiState>,
    _: Session,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<JsonValue>> {
    let field = query.sort.as_deref().unwrap_or("name");
    let key = match field {
        "name" => "artists.name_key",
        "created_at" => "artists.created_at",
        "rating" => "COALESCE(an.rating, 0)",
        "favorite" => "an.starred_at IS NOT NULL",
        other => return Err(bad_sort(other)),
    };
    let order = if query.descending() { "DESC" } else { "ASC" };
    let mut sql = Sql::new(
        "SELECT artists.id FROM artists \
         LEFT JOIN annotations an ON an.item = 'ar-' || artists.id \
         WHERE EXISTS (SELECT 1 FROM albums WHERE albums.artist_id = artists.id)",
    );
    if query.favorites_only {
        sql.push(" AND an.starred_at IS NOT NULL");
    }
    sql.push(&format!(
        " ORDER BY {key} {order}, artists.name_key, artists.id"
    ));
    page(&state, sql, &query, render::artists).await
}

/// `GET /api/songs`: every song, or a genre's, or an artist's.
pub(crate) async fn songs(
    State(state): State<ApiState>,
    _: Session,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<JsonValue>> {
    let field = query.sort.as_deref().unwrap_or("title");
    let key = match field {
        "title" => "tracks.title COLLATE NOCASE",
        "album_name" => "albums.title_key",
        "artist_name" => "tracks.artist_credit COLLATE NOCASE",
        "length" => "tracks.duration_ms",
        "track" => "tracks.track_number",
        "disc" => "tracks.disc_number",
        "genre" => "tracks.genre COLLATE NOCASE",
        "year" => "COALESCE(tracks.year, albums.year)",
        "created_at" => "tracks.added_at",
        "rating" => "COALESCE(an.rating, 0)",
        "favorite" => "an.starred_at IS NOT NULL",
        "play_count" => "COALESCE(an.play_count, 0)",
        other => return Err(bad_sort(other)),
    };
    let order = if query.descending() { "DESC" } else { "ASC" };
    let mut sql = Sql::new(
        "SELECT tracks.id FROM tracks \
         JOIN albums ON albums.id = tracks.album_id \
         LEFT JOIN annotations an ON an.item = 'tr-' || tracks.id \
         WHERE 1",
    );
    if query.favorites_only {
        sql.push(" AND an.starred_at IS NOT NULL");
    }
    if let Some(genre) = &query.genre {
        let genre = sql.param(Bind::Text(genre.clone()));
        sql.push(&format!(" AND tracks.genre = {genre}"));
    }
    if let Some(artist) = &query.artist {
        let Some(ids::Id::Artist(artist)) = ids::Id::parse(artist) else {
            return Err(ApiError::new(StatusCode::NOT_FOUND, "no such artist"));
        };
        let artist = sql.param(Bind::Int(i64::try_from(artist).unwrap_or(i64::MAX)));
        sql.push(&format!(
            " AND (tracks.artist_id = {artist} OR albums.artist_id = {artist})"
        ));
    }
    sql.push(&format!(
        " ORDER BY {key} {order}, albums.title_key, tracks.disc_number, tracks.track_number, tracks.id"
    ));
    page(&state, sql, &query, render::songs).await
}

#[derive(Deserialize)]
pub(crate) struct RecentQuery {
    limit: Option<u32>,
}

/// `GET /api/songs/recently-played`: the latest plays first.
pub(crate) async fn recently_played(
    State(state): State<ApiState>,
    _: Session,
    Query(query): Query<RecentQuery>,
) -> ApiResult<Json<Vec<JsonValue>>> {
    let mut sql = Sql::new(
        "SELECT tracks.id FROM tracks \
         JOIN annotations an ON an.item = 'tr-' || tracks.id \
         WHERE an.last_played IS NOT NULL \
         ORDER BY an.last_played DESC",
    );
    let limit = sql.param(Bind::Int(i64::from(
        query.limit.unwrap_or(PAGE).clamp(1, MAX_LIMIT),
    )));
    sql.push(&format!(" LIMIT {limit}"));
    let mut db = state.db.clone();
    let ids = sql.ids(&mut db).await?;
    Ok(Json(render::songs(&mut db, &ids).await?))
}

/// `GET /api/genres`: every genre with its song count and length, which
/// Subsonic's `getGenres` lacks. A genre's name is its id.
pub(crate) async fn genres(
    State(state): State<ApiState>,
    _: Session,
) -> ApiResult<Json<Vec<JsonValue>>> {
    let rows = toasty::sql::query(
        "SELECT genre, COUNT(*), COALESCE(SUM(duration_ms), 0) FROM tracks \
         WHERE genre IS NOT NULL AND genre != '' GROUP BY genre ORDER BY genre",
    )
    .exec(&mut state.db.clone())
    .await?;
    let number = |value: &Value| match value {
        Value::I64(value) => u64::try_from(*value).unwrap_or(0),
        Value::U64(value) => *value,
        _ => 0,
    };
    Ok(Json(
        rows.iter()
            .filter_map(|row| match row {
                Value::Record(record) => match &record[0] {
                    Value::String(name) => Some(json!({
                        "type": "genres",
                        "id": name,
                        "name": name,
                        "song_count": number(&record[1]),
                        "length": number(&record[2]) / 1000,
                    })),
                    _ => None,
                },
                _ => None,
            })
            .collect(),
    ))
}
