//! Settings: MusicBrainz lookups, and API keys.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pixiu_db::{ApiKey, now, toasty};
use pixiu_jobs::NewJob;
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session, auth};

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
    let unlooked = session
        .library(&state)
        .all_albums()
        .await?
        .iter()
        .filter(|album| album.enrichment.is_none())
        .count();
    Ok(Json(json!({
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

/// `POST /api/settings/lookup-all`: looks up every album of the user's
/// never looked up.
pub(crate) async fn lookup_all(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let albums = session.library(&state).all_albums().await?;
    let mut queued = 0;
    for album in albums.iter().filter(|album| album.enrichment.is_none()) {
        let title = format!("Look up {}", album.title);
        state
            .jobs
            .enqueue(
                session.owner(),
                NewJob::enrich(album.id, &title, None, false),
            )
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
