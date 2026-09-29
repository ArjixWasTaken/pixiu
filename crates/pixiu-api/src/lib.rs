//! The JSON API behind píxiū's web player (a fork of koel's frontend).
//!
//! The player reads the library through the Subsonic API; this API covers
//! what Subsonic cannot: signing in, the start-up payload, and (later)
//! hunting, offerings and settings. Every route but signing in needs an
//! API key as a bearer token; signing in hands one out.

mod auth;
mod bootstrap;

use axum::{
    Json, Router,
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use pixiu_core::SecretBox;
use pixiu_db::{ApiKey, Db, User, now, toasty};
use serde::Serialize;

/// Shared state for API handlers.
#[derive(Clone)]
pub struct ApiState {
    pub db: Db,
    /// Seals the password Subsonic token authentication needs.
    pub secrets: SecretBox,
}

/// Builds the API router. Paths are absolute (`/api/...`), so mount it
/// without stripping the prefix.
pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/api/auth/status", get(auth::status))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/setup", post(auth::setup))
        .route("/api/auth/session", delete(auth::logout))
        .route("/api/bootstrap", get(bootstrap::bootstrap))
        .with_state(state)
}

/// A failed request, answered as `{"message": ...}` as koel expects.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "sign in first")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            message: String,
        }
        (
            self.status,
            Json(Body {
                message: self.message,
            }),
        )
            .into_response()
    }
}

impl From<toasty::Error> for ApiError {
    fn from(error: toasty::Error) -> Self {
        tracing::error!(%error, "database error in the web API");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }
}

impl From<tokio::task::JoinError> for ApiError {
    fn from(error: tokio::task::JoinError) -> Self {
        tracing::error!(%error, "background task failed in the web API");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// The signed-in user, and the API key they signed in with.
pub(crate) struct Session {
    pub user: User,
    pub key: ApiKey,
}

impl FromRequestParts<ApiState> for Session {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &ApiState) -> ApiResult<Self> {
        let key = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(ApiError::unauthorized)?;
        let mut db = state.db.clone();
        let Some(mut key) = ApiKey::filter_by_key_hash(ApiKey::hash(key.trim()))
            .first()
            .exec(&mut db)
            .await?
        else {
            return Err(ApiError::unauthorized());
        };
        let user = User::get_by_id(&mut db, &key.user_id).await?;
        toasty::update!(key {
            last_used_at: Some(now()),
        })
        .exec(&mut db)
        .await?;
        Ok(Self { user, key })
    }
}
