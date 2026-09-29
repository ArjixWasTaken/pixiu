//! Signing in and out. A successful sign-in (or claiming a fresh hoard)
//! mints an API key named "Web session", which the player keeps and sends
//! as a bearer token here and as `apiKey` to the Subsonic API. Signing out
//! revokes it.

use axum::{Json, extract::State, http::StatusCode};
use pixiu_db::{ApiKey, User, now, toasty};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, ApiState, Session};

/// What the key minted at sign-in is called in the API key list.
const SESSION_KEY_NAME: &str = "Web session";

/// The shortest password a hoard may be claimed with.
const MIN_PASSWORD_LEN: usize = 8;

#[derive(Serialize)]
pub(crate) struct Status {
    /// Whether the admin account exists; until it does, the player offers
    /// to claim the hoard instead of signing in.
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
    fn of(key: String) -> Self {
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

/// Makes a new API key for `user`; returns it in the clear, once.
async fn mint_key(state: &ApiState, user: &User) -> ApiResult<String> {
    let key = new_key();
    toasty::create!(ApiKey {
        user_id: user.id,
        name: SESSION_KEY_NAME,
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
    username: String,
    password: String,
}

/// `POST /api/auth/login`.
pub(crate) async fn login(
    State(state): State<ApiState>,
    Json(credentials): Json<Credentials>,
) -> ApiResult<Json<Tokens>> {
    let username = credentials.username.trim();
    let mut db = state.db.clone();
    let user = User::filter_by_username(username)
        .first()
        .exec(&mut db)
        .await?;
    // Always verify, even for unknown users, so timing reveals nothing.
    let hash = user.as_ref().map(|user| user.password_hash.clone());
    let verified = verify(credentials.password.clone(), hash).await?;
    let Some(mut user) = user.filter(|_| verified) else {
        tracing::warn!(username, "failed sign-in");
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "Wrong username or password.",
        ));
    };
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

/// `POST /api/auth/setup`: creates the admin account while there is none.
pub(crate) async fn setup(
    State(state): State<ApiState>,
    Json(claim): Json<Claim>,
) -> ApiResult<Json<Tokens>> {
    let username = claim.username.trim().to_owned();
    if username.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Choose a username.",
        ));
    }
    if claim.password.chars().count() < MIN_PASSWORD_LEN {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Use at least 8 characters for the password.",
        ));
    }
    let subsonic_secret = state.secrets.seal_str(&claim.password);
    let password = claim.password;
    let password_hash =
        tokio::task::spawn_blocking(move || pixiu_core::password::hash(&password)).await?;

    // The check and the insert share a transaction, so two racing claims
    // cannot both create an admin.
    let mut db = state.db.clone();
    let mut tx = db.transaction().await?;
    if User::all().first().exec(&mut tx).await?.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "This hoard is already claimed.",
        ));
    }
    let user = toasty::create!(User {
        username,
        password_hash,
        subsonic_secret: Some(subsonic_secret),
        created_at: now(),
    })
    .exec(&mut tx)
    .await?;
    tx.commit().await?;
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
