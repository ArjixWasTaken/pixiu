//! The library in the orders koel's screens list it: albums, artists and
//! songs sorted by any column and paged with a cursor, plus recently
//! played songs, all of the signed-in user's library. Items are Subsonic
//! JSON (see [`pixiu_subsonic::render`]).

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use pixiu_db::{
    Library,
    owned::{Bind, Sql, as_u64},
    toasty,
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

fn bad_sort(field: &str) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        format!("cannot sort by `{field}`"),
    )
}

/// Runs a page of `sql` (ordered, without `LIMIT`), rendering the ids it
/// selects, as koel's cursor pages: `{data, meta: {next_cursor}}`.
async fn page<F>(
    lib: &Library,
    mut sql: Sql,
    query: &ListQuery,
    render: F,
) -> ApiResult<Json<JsonValue>>
where
    F: AsyncFnOnce(&Library, &[u64]) -> Result<Vec<JsonValue>, toasty::Error>,
{
    let (offset, limit) = (query.offset(), query.limit());
    // One more than asked, to tell whether another page follows.
    let limit_param = sql.param(Bind::Int(i64::from(limit) + 1));
    let offset_param = sql.param(Bind::Int(i64::from(offset)));
    sql.push(&format!(" LIMIT {limit_param} OFFSET {offset_param}"));

    let mut ids = sql.ids(&mut lib.db()).await?;
    let more = ids.len() > limit as usize;
    ids.truncate(limit as usize);
    let data = render(lib, &ids).await?;
    let next_cursor = more.then(|| (offset + limit).to_string());
    Ok(Json(json!({
        "data": data,
        "meta": { "per_page": limit, "next_cursor": next_cursor },
    })))
}

/// `GET /api/albums`: albums, not singles (the player shows those as
/// songs).
pub(crate) async fn albums(
    State(state): State<ApiState>,
    session: Session,
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
    let lib = session.library(&state);
    let mut sql = lib.sql(
        "SELECT albums.id FROM albums \
         JOIN artists ON artists.id = albums.artist_id \
         LEFT JOIN annotations an ON an.item = 'al-' || albums.id \
         AND an.user_id = albums.user_id \
         WHERE albums.user_id = ?1 AND NOT albums.single",
    );
    if query.favorites_only {
        sql.push(" AND an.starred_at IS NOT NULL");
    }
    sql.push(&format!(
        " ORDER BY {key} {order}, albums.title_key, albums.id"
    ));
    page(&lib, sql, &query, render::albums).await
}

/// `GET /api/artists`: album artists, as the Subsonic API lists them.
pub(crate) async fn artists(
    State(state): State<ApiState>,
    session: Session,
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
    let lib = session.library(&state);
    let mut sql = lib.sql(
        "SELECT artists.id FROM artists \
         LEFT JOIN annotations an ON an.item = 'ar-' || artists.id \
         AND an.user_id = artists.user_id \
         WHERE artists.user_id = ?1 \
         AND EXISTS (SELECT 1 FROM albums WHERE albums.artist_id = artists.id)",
    );
    if query.favorites_only {
        sql.push(" AND an.starred_at IS NOT NULL");
    }
    sql.push(&format!(
        " ORDER BY {key} {order}, artists.name_key, artists.id"
    ));
    page(&lib, sql, &query, render::artists).await
}

/// `GET /api/songs`: every song, or a genre's, or an artist's.
pub(crate) async fn songs(
    State(state): State<ApiState>,
    session: Session,
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
    let lib = session.library(&state);
    let mut sql = lib.sql(
        "SELECT tracks.id FROM tracks \
         JOIN albums ON albums.id = tracks.album_id \
         LEFT JOIN annotations an ON an.item = 'tr-' || tracks.id \
         AND an.user_id = tracks.user_id \
         WHERE tracks.user_id = ?1",
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
    page(&lib, sql, &query, render::songs).await
}

#[derive(Deserialize)]
pub(crate) struct RecentQuery {
    limit: Option<u32>,
}

/// `GET /api/songs/recently-played`: the latest plays first.
pub(crate) async fn recently_played(
    State(state): State<ApiState>,
    session: Session,
    Query(query): Query<RecentQuery>,
) -> ApiResult<Json<Vec<JsonValue>>> {
    let lib = session.library(&state);
    let mut sql = lib.sql(
        "SELECT tracks.id FROM tracks \
         JOIN annotations an ON an.item = 'tr-' || tracks.id \
         AND an.user_id = tracks.user_id \
         WHERE tracks.user_id = ?1 AND an.last_played IS NOT NULL \
         ORDER BY an.last_played DESC",
    );
    let limit = sql.param(Bind::Int(i64::from(
        query.limit.unwrap_or(PAGE).clamp(1, MAX_LIMIT),
    )));
    sql.push(&format!(" LIMIT {limit}"));
    let ids = sql.ids(&mut lib.db()).await?;
    Ok(Json(render::songs(&lib, &ids).await?))
}

/// `GET /api/genres`: every genre with its song count and length, which
/// Subsonic's `getGenres` lacks. A genre's name is its id.
pub(crate) async fn genres(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<Vec<JsonValue>>> {
    let lib = session.library(&state);
    let rows = lib
        .sql(
            "SELECT genre, COUNT(*), COALESCE(SUM(duration_ms), 0) FROM tracks \
             WHERE tracks.user_id = ?1 AND genre IS NOT NULL AND genre != '' \
             GROUP BY genre ORDER BY genre",
        )
        .rows(&mut lib.db())
        .await?;
    let number =
        |row: &[toasty::stmt::Value], index: usize| row.get(index).and_then(as_u64).unwrap_or(0);
    Ok(Json(
        rows.iter()
            .filter_map(|row| {
                let name = row.first()?.as_str()?;
                Some(json!({
                    "type": "genres",
                    "id": name,
                    "name": name,
                    "song_count": number(row, 1),
                    "length": number(row, 2) / 1000,
                }))
            })
            .collect(),
    ))
}
