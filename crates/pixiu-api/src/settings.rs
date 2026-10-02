//! A user's library settings: MusicBrainz lookups.

use axum::{Json, extract::State};
use pixiu_jobs::NewJob;
use serde_json::{Value as JsonValue, json};

use crate::{ApiResult, ApiState, Session};

/// `GET /api/settings`: the signed-in user's library settings.
pub(crate) async fn show(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let unlooked = session
        .library(&state)
        .all_albums()
        .await?
        .iter()
        .filter(|album| album.enrichment.is_none())
        .count();
    Ok(Json(json!({ "albums_not_looked_up": unlooked })))
}

/// `POST /api/settings/lookup-all`: looks up every album of the user's
/// never looked up.
pub(crate) async fn lookup_all(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let lib = session.library(&state);
    let albums = lib.all_albums().await?;
    let mut queued = 0;
    for album in albums.iter().filter(|album| album.enrichment.is_none()) {
        let title = pixiu_jobs::enrich::lookup_title(&mut lib.db(), album).await;
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
