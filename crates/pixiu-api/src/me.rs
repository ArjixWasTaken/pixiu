//! The signed-in user's account: their name and email, their password, the
//! API keys that sign apps in as them, and which alerts they get by email.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::collections::HashMap;

use pixiu_accounts::{alerts, links, users};
use pixiu_core::alerts::AlertKind;
use pixiu_db::{ApiKey, Role, User, now, toasty};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{
    ApiError, ApiResult, ApiState, Session, auth,
    throttle::{Action, too_many},
};

/// How an account is shown to its owner and to admins.
pub(crate) fn describe(user: &User) -> JsonValue {
    json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
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

/// The account as its owner sees it: with whether email works here.
fn describe_own(state: &ApiState, user: &User) -> JsonValue {
    let mut account = describe(user);
    account["mail_ready"] = json!(state.mailer.ready());
    account
}

/// `GET /api/me`.
pub(crate) async fn show(State(state): State<ApiState>, session: Session) -> Json<JsonValue> {
    Json(describe_own(&state, &session.user))
}

#[derive(Deserialize)]
pub(crate) struct Profile {
    username: String,
    #[serde(default)]
    email: String,
    /// Left as it is when absent; blank clears it.
    display_name: Option<String>,
}

/// `PUT /api/me`: a new username, display name or email. A new email waits
/// to be confirmed; píxiū sends the link when it can.
pub(crate) async fn update(
    State(state): State<ApiState>,
    session: Session,
    Json(profile): Json<Profile>,
) -> ApiResult<Json<JsonValue>> {
    let mut db = state.db.clone();
    let before = session.user.email.clone();
    // Checked first, so a bad name changes nothing.
    if let Some(name) = &profile.display_name {
        users::check_display_name(name)?;
    }
    let user = users::set_profile(
        &mut db,
        session.user,
        &profile.username,
        &profile.email,
        false,
    )
    .await?;
    let user = match &profile.display_name {
        Some(name) => users::set_display_name(&mut db, user, name).await?,
        None => user,
    };
    if user.email.is_some() && user.email != before && state.mailer.ready() {
        state
            .throttle
            .hit(Action::VerifyEmail, &user.id.to_string());
        links::send_verification(&mut db, &state.mailer, &user).await?;
    }
    Ok(Json(describe_own(&state, &user)))
}

/// `POST /api/me/email/resend`: sends the link confirming the user's email
/// again.
pub(crate) async fn resend_verification(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<StatusCode> {
    let user = session.user;
    if user.email.is_some() && user.email_verified_at.is_some() {
        return Err(ApiError::unprocessable(
            "Your email address is confirmed already.",
        ));
    }
    if !state
        .throttle
        .allow(Action::VerifyEmail, &user.id.to_string())
    {
        return Err(too_many());
    }
    links::send_verification(&mut state.db.clone(), &state.mailer, &user).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/me/alerts`: which alerts the user gets by email, and whether
/// email can reach them at all.
pub(crate) async fn alerts(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let wanted = alerts::wanted(&mut state.db.clone(), session.user.id).await?;
    Ok(Json(describe_alerts(&state, &session.user, &wanted)))
}

fn describe_alerts(state: &ApiState, user: &User, wanted: &HashMap<AlertKind, bool>) -> JsonValue {
    json!({
        "deliverable": state.mailer.ready() && user.email.is_some() && user.email_verified_at.is_some(),
        "alerts": AlertKind::ALL
            .iter()
            .map(|kind| (kind.name(), wanted[kind]))
            .collect::<HashMap<_, _>>(),
    })
}

/// `PUT /api/me/alerts`: `{kind: on}` for the kinds to switch.
pub(crate) async fn set_alerts(
    State(state): State<ApiState>,
    session: Session,
    Json(changes): Json<HashMap<String, bool>>,
) -> ApiResult<Json<JsonValue>> {
    let changes = changes
        .into_iter()
        .map(|(name, on)| {
            AlertKind::ALL
                .into_iter()
                .find(|kind| kind.name() == name)
                .map(|kind| (kind, on))
                .ok_or_else(|| ApiError::unprocessable(format!("no such alert: {name}")))
        })
        .collect::<ApiResult<HashMap<_, _>>>()?;
    let mut db = state.db.clone();
    alerts::set_wanted(&mut db, session.user.id, &changes).await?;
    let wanted = alerts::wanted(&mut db, session.user.id).await?;
    Ok(Json(describe_alerts(&state, &session.user, &wanted)))
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
