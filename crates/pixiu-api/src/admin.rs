//! Managing users, for admins: who has an account, what their libraries
//! take up, making accounts, roles, turning accounts off, temporary
//! passwords, and deleting an account with its library.

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use jiff::Timestamp;
use pixiu_accounts::{NewUser, links, registration, users};
use pixiu_db::{ApiKey, JobState, Role, User, UserIdentity, UserStatus};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{AdminSession, ApiError, ApiResult, ApiState, me, sources::state_name};

/// How long deleting a user waits for their running jobs to end.
const JOB_WAIT: Duration = Duration::from_secs(60);

pub(crate) fn status_name(status: UserStatus) -> &'static str {
    match status {
        UserStatus::Pending => "pending",
        UserStatus::Unverified => "unverified",
        UserStatus::Active => "active",
        UserStatus::Disabled => "disabled",
    }
}

fn role_of(name: &str) -> ApiResult<Role> {
    match name {
        "admin" => Ok(Role::Admin),
        "user" => Ok(Role::User),
        other => Err(ApiError::unprocessable(format!("`{other}` is not a role"))),
    }
}

/// `GET /api/admin/users`: every account, with what its library takes up.
pub(crate) async fn list(
    State(state): State<ApiState>,
    _: AdminSession,
) -> ApiResult<Json<JsonValue>> {
    let mut db = state.db.clone();
    let mut accounts = User::all().exec(&mut db).await?;
    accounts.sort_by_key(|user| user.id);
    let usage = users::usage(&mut db).await?;
    let linked: HashSet<u64> = UserIdentity::all()
        .exec(&mut db)
        .await?
        .iter()
        .map(|identity| identity.user_id)
        .collect();
    let mut last_seen: HashMap<u64, Timestamp> = HashMap::new();
    for key in ApiKey::all().exec(&mut db).await? {
        if let Some(used) = key.last_used_at {
            let seen = last_seen.entry(key.user_id).or_insert(used);
            *seen = (*seen).max(used);
        }
    }
    Ok(Json(json!(
        accounts
            .iter()
            .map(|user| {
                let mut row = me::describe(user);
                let usage = usage.get(&user.id).copied().unwrap_or_default();
                row["last_seen"] = json!(last_seen.get(&user.id));
                row["songs"] = json!(usage.songs);
                row["bytes"] = json!(usage.bytes);
                row["exclusive_bytes"] = json!(usage.exclusive_bytes);
                row["youtube_music"] = json!(state_name(
                    state
                        .wardens
                        .health(user.id, crate::sources::PLATFORM)
                        .state
                ));
                row["sso"] = json!(linked.contains(&user.id));
                row
            })
            .collect::<Vec<_>>()
    )))
}

#[derive(Deserialize)]
pub(crate) struct NewAccount {
    username: String,
    #[serde(default)]
    email: String,
    /// Temporary: the user picks their own at their first sign-in.
    password: String,
    #[serde(default = "user_role")]
    role: String,
}

fn user_role() -> String {
    "user".to_owned()
}

/// `POST /api/admin/users`: makes an account with a temporary password.
pub(crate) async fn create(
    State(state): State<ApiState>,
    AdminSession(admin): AdminSession,
    Json(form): Json<NewAccount>,
) -> ApiResult<Json<JsonValue>> {
    let user = users::create(
        &mut state.db.clone(),
        &state.secrets,
        NewUser {
            username: form.username,
            email: Some(form.email).filter(|email| !email.trim().is_empty()),
            // The admin typed it: it counts as confirmed.
            email_verified: true,
            password: form.password,
            role: role_of(&form.role)?,
            status: UserStatus::Active,
            password_change_required: true,
        },
    )
    .await?;
    tracing::info!(
        admin = admin.user.id,
        user = user.id,
        "admin made an account"
    );
    Ok(Json(me::describe(&user)))
}

#[derive(Deserialize)]
pub(crate) struct Changes {
    role: Option<String>,
    /// `active` or `disabled`.
    status: Option<String>,
    username: Option<String>,
    email: Option<String>,
    /// Blank clears it.
    display_name: Option<String>,
}

async fn load(state: &ApiState, id: u64) -> ApiResult<User> {
    User::filter_by_id(id)
        .first()
        .exec(&mut state.db.clone())
        .await?
        .ok_or_else(|| ApiError::not_found("user"))
}

/// `PATCH /api/admin/users/{id}`: role, on or off, username and email (an
/// email the admin types counts as confirmed).
pub(crate) async fn update(
    State(state): State<ApiState>,
    _: AdminSession,
    Path(id): Path<u64>,
    Json(changes): Json<Changes>,
) -> ApiResult<Json<JsonValue>> {
    let mut db = state.db.clone();
    if let Some(name) = &changes.display_name {
        users::check_display_name(name)?;
    }
    let user = load(&state, id).await?;
    let user = match &changes.display_name {
        Some(name) => users::set_display_name(&mut db, user, name).await?,
        None => user,
    };
    if changes.username.is_some() || changes.email.is_some() {
        let username = changes
            .username
            .as_deref()
            .unwrap_or(&user.username)
            .to_owned();
        let email = changes
            .email
            .clone()
            .or_else(|| user.email.clone())
            .unwrap_or_default();
        users::set_profile(&mut db, user, &username, &email, true).await?;
    }
    if let Some(role) = &changes.role {
        users::set_role(&mut db, id, role_of(role)?).await?;
    }
    match changes.status.as_deref() {
        None => {}
        Some("active") => {
            users::set_status(&mut db, id, UserStatus::Active).await?;
        }
        Some("disabled") => {
            users::set_status(&mut db, id, UserStatus::Disabled).await?;
            // Their session warden stops; their library stays.
            state.wardens.stop(id).await;
        }
        Some(other) => {
            return Err(ApiError::unprocessable(format!(
                "`{other}` is not a status an admin sets"
            )));
        }
    }
    Ok(Json(me::describe(&load(&state, id).await?)))
}

#[derive(Deserialize)]
pub(crate) struct TemporaryPassword {
    password: String,
}

/// `POST /api/admin/users/{id}/password`: a temporary password, which
/// signs the user out; they pick their own at their next sign-in.
pub(crate) async fn set_password(
    State(state): State<ApiState>,
    _: AdminSession,
    Path(id): Path<u64>,
    Json(form): Json<TemporaryPassword>,
) -> ApiResult<StatusCode> {
    let user = load(&state, id).await?;
    users::set_password(
        &mut state.db.clone(),
        &state.secrets,
        user,
        &form.password,
        true,
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/admin/users/{id}/password-reset`: emails the user a link to
/// choose a new password. Their current one keeps working until they do.
pub(crate) async fn send_reset(
    State(state): State<ApiState>,
    AdminSession(admin): AdminSession,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let user = load(&state, id).await?;
    links::send_reset(&mut state.db.clone(), &state.mailer, &user).await?;
    tracing::info!(
        admin = admin.user.id,
        user = id,
        "password reset email sent by an admin"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/admin/registrations/{id}/approve`: the applicant gets the
/// link confirming their address, which opens their account.
pub(crate) async fn approve(
    State(state): State<ApiState>,
    AdminSession(admin): AdminSession,
    Path(id): Path<u64>,
) -> ApiResult<Json<JsonValue>> {
    let user = registration::approve(&mut state.db.clone(), &state.mailer, id).await?;
    tracing::info!(
        admin = admin.user.id,
        user = id,
        "registration approved by an admin"
    );
    Ok(Json(me::describe(&user)))
}

/// `POST /api/admin/registrations/{id}/deny`: the applicant gets a short
/// note, and the request goes.
pub(crate) async fn deny(
    State(state): State<ApiState>,
    AdminSession(admin): AdminSession,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    registration::deny(&mut state.db.clone(), &state.mailer, id).await?;
    tracing::info!(
        admin = admin.user.id,
        user = id,
        "registration denied by an admin"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/admin/users/{id}/verification`: sends the link confirming the
/// user's address again.
pub(crate) async fn resend_verification(
    State(state): State<ApiState>,
    _: AdminSession,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let user = load(&state, id).await?;
    if user.email_verified_at.is_some() {
        return Err(ApiError::unprocessable(
            "Their email address is confirmed already.",
        ));
    }
    links::send_verification(&mut state.db.clone(), &state.mailer, &user).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/admin/users/{id}`: deletes an account and its library.
/// Stored files other libraries play stay. Answers how much disk it freed.
pub(crate) async fn delete(
    State(state): State<ApiState>,
    AdminSession(admin): AdminSession,
    Path(id): Path<u64>,
) -> ApiResult<Json<JsonValue>> {
    if id == admin.user.id {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "You cannot delete your own account.",
        ));
    }
    let freed = remove(&state, id).await?;
    tracing::info!(admin = admin.user.id, user = id, freed, "account deleted");
    Ok(Json(json!({ "freed_bytes": freed })))
}

/// Deletes an account and everything of it; returns the bytes freed.
pub(crate) async fn remove(state: &ApiState, id: u64) -> ApiResult<u64> {
    let mut db = state.db.clone();
    users::check_deletable(&mut db, id).await?;
    let freed = users::usage(&mut db)
        .await?
        .get(&id)
        .map_or(0, |usage| usage.exclusive_bytes);
    // Off first: their keys stop working and their jobs stop starting.
    users::set_status(&mut db, id, UserStatus::Disabled).await?;
    state.wardens.stop(id).await;
    state.jobs.forget(id, |_| true).await?;
    let started = std::time::Instant::now();
    while started.elapsed() < JOB_WAIT
        && state
            .jobs
            .unfinished(id)
            .await?
            .iter()
            .any(|job| job.state == JobState::Running)
    {
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    users::delete(&mut db, id).await?;
    state.treasury.collect_garbage().await?;
    state.offerings.remove_owner(id).await;
    let dir = state.desks.user_dir(id);
    if let Err(error) = tokio::fs::remove_dir_all(&dir).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%error, path = %dir.display(), "cannot remove a user's files");
    }
    Ok(freed)
}

/// `GET /api/admin/storage`: what the store holds.
pub(crate) async fn storage(
    State(state): State<ApiState>,
    _: AdminSession,
) -> ApiResult<Json<JsonValue>> {
    let store = users::store(&mut state.db.clone()).await?;
    Ok(Json(json!({
        "files": store.files,
        "bytes": store.bytes,
        "shared_files": store.shared_files,
    })))
}
