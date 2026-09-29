//! Settings: the file layout, MusicBrainz lookups, and API keys.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use pixiu_db::{Album, ApiKey, JobKind, now, toasty};
use pixiu_jobs::NewJob;
use pixiu_treasury::{Template, layout::TrackLocation};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session, auth};

async fn refiling(state: &ApiState) -> ApiResult<bool> {
    Ok(state
        .jobs
        .unfinished()
        .await?
        .iter()
        .any(|job| job.kind == JobKind::Refile))
}

/// `GET /api/settings`.
pub(crate) async fn show(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let mut db = state.db.clone();
    let mut keys = ApiKey::filter_by_user_id(session.user.id)
        .exec(&mut db)
        .await?;
    keys.sort_by_key(|key| key.created_at);
    let unlooked = Album::all()
        .exec(&mut db)
        .await?
        .iter()
        .filter(|album| album.enrichment.is_none())
        .count();
    let example = TrackLocation::EXAMPLE;
    Ok(Json(json!({
        "layout": {
            "template": state.treasury.layout().as_str(),
            "default": Template::DEFAULT,
            "misplaced": state.treasury.misplaced().await?,
            "refiling": refiling(&state).await?,
            "example": {
                "title": example.title,
                "album": example.album,
                "track": example.track,
            },
        },
        "albums_not_looked_up": unlooked,
        "keys": keys.iter().map(|key| json!({
            "id": key.id,
            "name": key.name,
            "created_at": key.created_at,
            "last_used_at": key.last_used_at,
            "current": key.id == session.key.id,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub(crate) struct Layout {
    template: String,
}

/// `GET /api/settings/layout/preview?template=`: where the template would
/// file the example track.
pub(crate) async fn preview_layout(
    _: Session,
    Query(layout): Query<Layout>,
) -> ApiResult<Json<JsonValue>> {
    match Template::preview(&layout.template) {
        Ok(path) => Ok(Json(json!({ "path": path.display().to_string() }))),
        Err(error) => Err(ApiError::unprocessable(error.to_string())),
    }
}

/// `PUT /api/settings/layout`: new tracks follow it at once; the others
/// move when the admin asks (`POST /api/settings/refile`).
pub(crate) async fn save_layout(
    State(state): State<ApiState>,
    _: Session,
    Json(layout): Json<Layout>,
) -> ApiResult<Json<JsonValue>> {
    let template = Template::parse(&layout.template)
        .map_err(|error| ApiError::unprocessable(error.to_string()))?;
    tracing::info!(%template, "file layout changed");
    state.treasury.set_layout(template).await?;
    Ok(Json(
        json!({ "misplaced": state.treasury.misplaced().await? }),
    ))
}

/// `POST /api/settings/refile`: moves every file to the layout, unless
/// that is under way.
pub(crate) async fn refile(State(state): State<ApiState>, _: Session) -> ApiResult<StatusCode> {
    if !refiling(&state).await? {
        state.jobs.enqueue(NewJob::refile()).await?;
    }
    Ok(StatusCode::ACCEPTED)
}

/// `POST /api/settings/lookup-all`: looks up every album never looked up.
pub(crate) async fn lookup_all(
    State(state): State<ApiState>,
    _: Session,
) -> ApiResult<Json<JsonValue>> {
    let albums = Album::all().exec(&mut state.db.clone()).await?;
    let mut queued = 0;
    for album in albums.iter().filter(|album| album.enrichment.is_none()) {
        let title = format!("Look up {}", album.title);
        state
            .jobs
            .enqueue(NewJob::enrich(album.id, &title, None, false))
            .await?;
        queued += 1;
    }
    Ok(Json(json!({ "queued": queued })))
}

#[derive(Deserialize)]
pub(crate) struct NewKey {
    name: String,
}

/// `POST /api/keys`: makes a key for a Subsonic app, shown this once.
pub(crate) async fn create_key(
    State(state): State<ApiState>,
    session: Session,
    Json(form): Json<NewKey>,
) -> ApiResult<Json<JsonValue>> {
    let name = match form.name.trim() {
        "" => "Unnamed",
        name => name,
    };
    let key = auth::new_key();
    let created = toasty::create!(ApiKey {
        user_id: session.user.id,
        name,
        key_hash: ApiKey::hash(&key),
        created_at: now(),
    })
    .exec(&mut state.db.clone())
    .await?;
    tracing::info!(name, "API key created");
    Ok(Json(json!({ "id": created.id, "name": name, "key": key })))
}

/// `DELETE /api/keys/{id}`.
pub(crate) async fn revoke_key(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let mut db = state.db.clone();
    match ApiKey::filter_by_id(id).first().exec(&mut db).await? {
        Some(key) if key.user_id == session.user.id => {
            tracing::info!(name = key.name, "API key revoked");
            key.delete().exec(&mut db).await?;
            Ok(StatusCode::NO_CONTENT)
        }
        _ => Err(ApiError::not_found("key")),
    }
}
