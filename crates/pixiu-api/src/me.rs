//! The signed-in user's account: their name and email, their password, and
//! the API keys that sign apps in as them.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pixiu_accounts::users;
use pixiu_db::{ApiKey, Role, User, now, toasty};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session, auth};

/// How an account is shown to its owner and to admins.
pub(crate) fn describe(user: &User) -> JsonValue {
    json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "email_verified": user.email.is_some() && user.email_verified_at.is_some(),
        "role": match user.role {
            Role::Admin => "admin",
            Role::User => "user",
        },
        "status": crate::admin::status_name(user.status),
        "password_change_required": user.password_change_required,
        "created_at": user.created_at,
    })
}

/// `GET /api/me`.
pub(crate) async fn show(session: Session) -> Json<JsonValue> {
    Json(describe(&session.user))
}

#[derive(Deserialize)]
pub(crate) struct Profile {
    username: String,
    #[serde(default)]
    email: String,
}

/// `PUT /api/me`: a new username or email. A new email waits to be
/// confirmed.
pub(crate) async fn update(
    State(state): State<ApiState>,
    session: Session,
    Json(profile): Json<Profile>,
) -> ApiResult<Json<JsonValue>> {
    let user = users::set_profile(
        &mut state.db.clone(),
        session.user,
        &profile.username,
        &profile.email,
        false,
    )
    .await?;
    Ok(Json(describe(&user)))
}

#[derive(Deserialize)]
pub(crate) struct NewPassword {
    /// Not asked for when an admin handed out a temporary password.
    #[serde(default)]
    current_password: String,
    password: String,
}

/// `PUT /api/me/password`: needs the current password, unless the user is
/// replacing a temporary one. Signs the user's other web sessions out.
pub(crate) async fn change_password(
    State(state): State<ApiState>,
    session: Session,
    Json(change): Json<NewPassword>,
) -> ApiResult<StatusCode> {
    let user = session.user;
    if !user.password_change_required {
        let (password, hash) = (change.current_password.clone(), user.password_hash.clone());
        let right = tokio::task::spawn_blocking(move || {
            pixiu_core::password::verify(&password, Some(&hash))
        })
        .await?;
        if !right {
            return Err(ApiError::unprocessable("The current password is wrong."));
        }
    }
    users::set_password(
        &mut state.db.clone(),
        &state.secrets,
        user,
        &change.password,
        false,
        Some(session.key.id),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/me/keys`: the user's API keys, oldest first.
pub(crate) async fn keys(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let mut keys = ApiKey::filter_by_user_id(session.user.id)
        .exec(&mut state.db.clone())
        .await?;
    keys.sort_by_key(|key| key.created_at);
    Ok(Json(json!(
        keys.iter()
            .map(|key| json!({
                "id": key.id,
                "name": key.name,
                "created_at": key.created_at,
                "last_used_at": key.last_used_at,
                "current": key.id == session.key.id,
            }))
            .collect::<Vec<_>>()
    )))
}

#[derive(Deserialize)]
pub(crate) struct NewKey {
    name: String,
}

/// `POST /api/me/keys`: makes a key for a Subsonic app, shown this once.
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
    tracing::info!(name, user = session.user.id, "API key created");
    Ok(Json(json!({ "id": created.id, "name": name, "key": key })))
}

/// `DELETE /api/me/keys/{id}`.
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
