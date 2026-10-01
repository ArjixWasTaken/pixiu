//! `GET /api/bootstrap`: what koel's player loads at start-up (koel's
//! `GET data`): the signed-in user, feature flags, and library totals.
//! Features píxiū does not have are switched off here, which hides them.

use axum::{Json, extract::State};
use pixiu_db::{Library, Role, owned::as_u64};
use serde_json::{Value as JsonValue, json};

use crate::{ApiResult, ApiState, Session};

/// How many songs the library holds, and their total length in seconds.
async fn totals(lib: &Library) -> ApiResult<(u64, u64)> {
    let rows = lib
        .sql(
            "SELECT COUNT(*), COALESCE(SUM(duration_ms), 0) FROM tracks \
             WHERE tracks.user_id = ?1",
        )
        .rows(&mut lib.db())
        .await?;
    let number = |index: usize| {
        rows.first()
            .and_then(|row| row.get(index))
            .and_then(as_u64)
            .unwrap_or(0)
    };
    Ok((number(0), number(1) / 1000))
}

/// `GET /api/bootstrap`.
pub(crate) async fn bootstrap(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let lib = session.library(&state);
    let (song_count, song_length) = totals(&lib).await?;
    let user = &session.user;
    let admin = user.role == Role::Admin;
    let abilities: &[&str] = if admin {
        &["manage settings", "manage songs", "manage users"]
    } else {
        &["manage songs"]
    };
    Ok(Json(json!({
        "current_user": {
            "type": "users",
            "id": user.id.to_string(),
            "name": pixiu_accounts::users::shown_name(user),
            "username": user.username,
            "display_name": user.display_name,
            "email": user.email.as_deref().unwrap_or_default(),
            "email_verified": user.email_verified_at.is_some(),
            "is_prospect": false,
            "avatar": "",
            "role": if admin { "admin" } else { "user" },
            "sso_provider": null,
            "sso_id": null,
            "abilities": abilities,
            "password_change_required": user.password_change_required,
            // Preferences live in the browser.
            "preferences": {},
        },
        "current_version": pixiu_core::VERSION,
        "latest_version": pixiu_core::VERSION,
        "koel_plus": {
            "active": false,
            "short_key": null,
            "customer_name": null,
            "customer_email": null,
            "product_id": null,
        },
        "song_count": song_count,
        "song_length": song_length,
        "allows_download": true,
        "allows_embedding": false,
        "assignable_roles": [],
        "cdn_url": "",
        "media_path_set": true,
        "playlists": [],
        "playlist_folders": crate::playlists::folders(&lib).await?,
        "settings": {},
        "users": [],
        "queue_state": {
            "type": "queue-states",
            "songs": [],
            "current_song": null,
            "playback_position": 0,
        },
        "uses_musicbrainz": true,
        "uses_i_tunes": false,
        "uses_last_fm": false,
        "uses_spotify": false,
        "uses_ticketmaster": false,
        "uses_media_browser": false,
        "uses_podcasts": false,
        "uses_radio": false,
        "uses_ai": false,
        "uses_you_tube": false,
        "storage_driver": "local",
        "supports_presigned_uploads": false,
        "supports_batch_downloading": false,
        "supports_transcoding": true,
        "dir_separator": "/",
        "current_theme": null,
        "hunting": crate::jobs::summarize(&state, &session).await?,
    })))
}
