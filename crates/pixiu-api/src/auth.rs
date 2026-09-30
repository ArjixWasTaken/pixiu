//! Signing in and out. A successful sign-in (or setting up a fresh server)
//! mints an API key named "Web session", which the player keeps and sends
//! as a bearer token here and as `apiKey` to the Subsonic API. Signing out
//! revokes it.

use axum::{Json, extract::State, http::StatusCode};
use pixiu_accounts::users;
use pixiu_db::{ApiKey, User, UserStatus, now, toasty};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, ApiState, Session};

#[derive(Serialize)]
pub(crate) struct Status {
    /// Whether an account exists; until one does, the player offers to set
    /// píxiū up instead of signing in.
    claimed: bool,
}

/// `GET /api/auth/status`.
pub(crate) async fn status(State(state): State<ApiState>) -> ApiResult<Json<Status>> {
    let claimed = User::all()
        .first()
        .exec(&mut state.db.clone())
        .await?
        .is_some();
    Ok(Json(Status { claimed }))
}

/// Koel's composite token: the same key serves the API and the audio.
#[derive(Serialize)]
pub(crate) struct Tokens {
    token: String,
    #[serde(rename = "audio-token")]
    audio_token: String,
}

impl Tokens {
    pub(crate) fn of(key: String) -> Self {
        Self {
            audio_token: key.clone(),
            token: key,
        }
    }
}

/// A fresh API key, in the clear.
pub(crate) fn new_key() -> String {
    format!("pixiu_{}", hex::encode(rand::random::<[u8; 24]>()))
}

/// Makes a new web session key for `user`; returns it in the clear, once.
pub(crate) async fn mint_key(state: &ApiState, user: &User) -> ApiResult<String> {
    let key = new_key();
    toasty::create!(ApiKey {
        user_id: user.id,
        name: users::WEB_SESSION,
        key_hash: ApiKey::hash(&key),
        created_at: now(),
    })
    .exec(&mut state.db.clone())
    .await?;
    Ok(key)
}

async fn verify(password: String, hash: Option<String>) -> ApiResult<bool> {
    Ok(tokio::task::spawn_blocking(move || {
        pixiu_core::password::verify(&password, hash.as_deref())
    })
    .await?)
}

#[derive(Deserialize)]
pub(crate) struct Credentials {
    /// A username or an email address.
    username: String,
    password: String,
}

/// Why an account whose password is right cannot sign in, for the player.
pub(crate) fn inactive(status: UserStatus) -> Option<ApiError> {
    let (code, message) = match status {
        UserStatus::Active => return None,
        UserStatus::Pending => (
            "pending",
            "An admin has yet to approve your account; you will get an email when they do.",
        ),
        UserStatus::Unverified => (
            "unverified",
            "Confirm your email address first: follow the link píxiū sent you.",
        ),
        UserStatus::Disabled => ("disabled", "Your account is turned off."),
    };
    Some(ApiError::forbidden(message).with_code(code))
}

/// `POST /api/auth/login`.
pub(crate) async fn login(
    State(state): State<ApiState>,
    Json(credentials): Json<Credentials>,
) -> ApiResult<Json<Tokens>> {
    let login = credentials.username.trim();
    let mut db = state.db.clone();
    let user = users::find_by_login(&mut db, login).await?;
    // Always verify, even for unknown users, so timing reveals nothing.
    let hash = user.as_ref().map(|user| user.password_hash.clone());
    let verified = verify(credentials.password.clone(), hash).await?;
    let Some(mut user) = user.filter(|_| verified) else {
        tracing::warn!(login, "failed sign-in");
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "Wrong username or password.",
        ));
    };
    if let Some(refusal) = inactive(user.status) {
        return Err(refusal);
    }
    // Token authentication in Subsonic apps needs the password, sealed.
    if user.subsonic_secret.is_none() {
        let sealed = state.secrets.seal_str(&credentials.password);
        toasty::update!(user {
            subsonic_secret: Some(sealed)
        })
        .exec(&mut db)
        .await?;
    }
    Ok(Json(Tokens::of(mint_key(&state, &user).await?)))
}

#[derive(Deserialize)]
pub(crate) struct Claim {
    username: String,
    password: String,
}

/// `POST /api/auth/setup`: makes the first account, an admin, while there
/// is none.
pub(crate) async fn setup(
    State(state): State<ApiState>,
    Json(claim): Json<Claim>,
) -> ApiResult<Json<Tokens>> {
    let user = users::create_first(
        &mut state.db.clone(),
        &state.secrets,
        &claim.username,
        &claim.password,
    )
    .await?;
    tracing::info!(username = %user.username, "admin account created");
    Ok(Json(Tokens::of(mint_key(&state, &user).await?)))
}

/// `DELETE /api/auth/session`: revokes the key this request came with.
pub(crate) async fn logout(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<StatusCode> {
    session.key.delete().exec(&mut state.db.clone()).await?;
    Ok(StatusCode::NO_CONTENT)
}
