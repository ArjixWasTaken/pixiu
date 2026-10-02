//! The JSON API behind píxiū's web player (a fork of koel's frontend).
//!
//! The player reads the library through the Subsonic API; this API covers
//! what Subsonic cannot: signing in, the start-up payload, sorted library
//! lists, and everything about hunting: searches and grabs, watches, jobs,
//! orphans, offerings, the YouTube Music account and settings. Every route
//! but signing in needs an API key as a bearer token; signing in hands one
//! out.

mod admin;
mod albums;
mod auth;
mod bootstrap;
mod events;
mod hunt;
mod jobs;
mod library;
mod me;
mod offerings;
mod orphans;
mod playlists;
mod server_settings;
mod settings;
mod songs;
mod sources;
mod sso;
mod throttle;
mod watches;

pub use throttle::Throttle;

use std::{fmt::Display, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequestParts},
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
};
use pixiu_accounts::{Mailer, Settings, oidc::Sso};
use pixiu_browser::LoginDesks;
use pixiu_core::SecretBox;
use pixiu_db::{ApiKey, Db, Library, Role, User, UserStatus, now, toasty};
use pixiu_hunt::Hunter;
use pixiu_jobs::{Jobs, Wardens};
use pixiu_treasury::{Offerings, Treasury};
use serde::Serialize;

/// Shared state for API handlers.
#[derive(Clone)]
pub struct ApiState {
    pub db: Db,
    /// Seals the password Subsonic token authentication needs.
    pub secrets: SecretBox,
    pub treasury: Treasury,
    pub offerings: Offerings,
    pub hunter: Arc<Hunter>,
    /// Every user's YouTube Music session warden.
    pub wardens: Arc<Wardens>,
    pub jobs: Arc<Jobs>,
    /// Every user's login browser.
    pub desks: Arc<LoginDesks>,
    /// What admins set in the player: the public address, registration,
    /// the mail server.
    pub settings: Arc<Settings>,
    pub mailer: Arc<Mailer>,
    /// The single sign-on provider.
    pub sso: Arc<Sso>,
    /// Limits guessing passwords and flooding inboxes.
    pub throttle: Arc<Throttle>,
    /// Whether `X-Forwarded-For` names the client (behind a reverse proxy).
    pub trust_proxy_headers: bool,
    /// Turns true when the server shuts down: event streams end then, as a
    /// graceful shutdown waits for every open response.
    pub shutdown: tokio::sync::watch::Receiver<bool>,
}

/// Builds the API router. Paths are absolute (`/api/...`), so mount it
/// without stripping the prefix.
pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/api/auth/status", get(auth::status))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/setup", post(auth::setup))
        .route("/api/auth/session", delete(auth::logout))
        .route("/api/auth/forgot", post(auth::forgot))
        .route("/api/auth/reset", post(auth::reset))
        .route("/api/auth/verify-email", post(auth::verify_email))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/oidc/start", get(sso::start))
        .route("/api/auth/oidc/callback", get(sso::callback))
        .route("/api/auth/oidc/exchange", post(sso::exchange))
        .route("/api/bootstrap", get(bootstrap::bootstrap))
        .route("/api/albums", get(library::albums))
        .route("/api/artists", get(library::artists))
        .route("/api/genres", get(library::genres))
        .route("/api/songs", get(library::songs))
        .route("/api/songs/recently-played", get(library::recently_played))
        .route("/api/songs/{id}/info", get(songs::info))
        .route("/api/albums/{id}/details", get(albums::details))
        .route("/api/albums/{id}", put(albums::edit))
        .route("/api/albums/{id}/lookup", post(albums::lookup))
        .route("/api/hunt", get(hunt::search))
        .route("/api/hunt/tracks", post(hunt::grab_track))
        .route("/api/hunt/albums", post(hunt::grab_album))
        .route("/api/watches", get(watches::list).post(watches::add))
        .route("/api/watches/{id}", delete(watches::remove))
        .route("/api/watches/{id}/sync", post(watches::sync))
        .route(
            "/api/watches/{id}/exclusions",
            get(watches::exclusions).post(watches::exclude),
        )
        .route(
            "/api/watches/{id}/exclusions/{video}",
            delete(watches::include),
        )
        .route(
            "/api/playlists",
            get(playlists::list).post(playlists::create),
        )
        .route("/api/playlists/{id}", put(playlists::update))
        .route("/api/playlists/{id}/watch", get(watches::of_playlist))
        .route("/api/playlist-folders", post(playlists::create_folder))
        .route(
            "/api/playlist-folders/{id}",
            put(playlists::update_folder)
                .patch(playlists::update_folder)
                .delete(playlists::delete_folder),
        )
        .route(
            "/api/playlist-folders/{id}/playlists",
            post(playlists::add_playlists).delete(playlists::remove_playlists),
        )
        .route("/api/hunting", get(jobs::summary))
        .route("/api/jobs", get(jobs::board))
        .route("/api/jobs/finished", delete(jobs::clear_finished))
        .route("/api/jobs/{id}/retry", post(jobs::retry))
        .route("/api/events", get(events::stream))
        .route("/api/orphans", get(orphans::list))
        .route("/api/orphans/keep", post(orphans::keep))
        .route("/api/orphans/delete", post(orphans::delete))
        .route("/api/offerings", get(offerings::list))
        .route(
            "/api/offerings/upload",
            // Uploads stream to disk; whole albums are large.
            post(offerings::upload).layer(DefaultBodyLimit::disable()),
        )
        .route(
            "/api/offerings/batches/{batch}/accept",
            post(offerings::accept_batch),
        )
        .route(
            "/api/offerings/batches/{batch}",
            delete(offerings::discard_batch),
        )
        .route("/api/offerings/{id}", delete(offerings::discard))
        .route("/api/settings", get(settings::show))
        .route("/api/settings/lookup-all", post(settings::lookup_all))
        .route("/api/me", get(me::show).put(me::update))
        .route("/api/me/password", put(me::change_password))
        .route("/api/me/keys", get(me::keys).post(me::create_key))
        .route("/api/me/keys/{id}", delete(me::revoke_key))
        .route("/api/me/email/resend", post(me::resend_verification))
        .route("/api/me/alerts", get(me::alerts).put(me::set_alerts))
        .route("/api/me/identities", get(sso::list))
        .route("/api/me/identities/oidc", post(sso::link))
        .route("/api/me/identities/{id}", delete(sso::unlink))
        .route("/api/admin/users", get(admin::list).post(admin::create))
        .route(
            "/api/admin/users/{id}",
            patch(admin::update).delete(admin::delete),
        )
        .route("/api/admin/users/{id}/password", post(admin::set_password))
        .route(
            "/api/admin/users/{id}/password-reset",
            post(admin::send_reset),
        )
        .route(
            "/api/admin/users/{id}/verification",
            post(admin::resend_verification),
        )
        .route(
            "/api/admin/registrations/{id}/approve",
            post(admin::approve),
        )
        .route("/api/admin/registrations/{id}/deny", post(admin::deny))
        .route("/api/admin/storage", get(admin::storage))
        .route("/api/admin/settings", get(server_settings::show))
        .route(
            "/api/admin/settings/server",
            put(server_settings::set_server),
        )
        .route(
            "/api/admin/settings/smtp",
            put(server_settings::set_smtp).delete(server_settings::remove_smtp),
        )
        .route(
            "/api/admin/settings/smtp/test",
            post(server_settings::test_smtp),
        )
        .route(
            "/api/admin/settings/registration",
            put(server_settings::set_registration),
        )
        .route(
            "/api/admin/settings/oidc",
            put(server_settings::set_oidc).delete(server_settings::remove_oidc),
        )
        .route(
            "/api/admin/settings/oidc/test",
            post(server_settings::test_oidc),
        )
        .route("/api/sources", get(sources::status))
        .route("/api/sources/validate", post(sources::validate))
        .route("/api/sources/refresh", post(sources::refresh))
        .route("/api/sources/disconnect", post(sources::disconnect))
        .route(
            "/api/sources/login",
            post(sources::open_login).delete(sources::cancel_login),
        )
        .route("/api/sources/login/status", get(sources::login_status))
        .route("/api/sources/login/finish", post(sources::finish_login))
        .route("/api/sources/login/ws", get(sources::screen))
        .with_state(state)
}

/// A failed request, answered as `{"message": ...}` as koel expects, with
/// a `code` the player can act on when there is one.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
    code: Option<&'static str>,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            code: None,
        }
    }

    fn with_code(mut self, code: &'static str) -> Self {
        self.code = Some(code);
        self
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "sign in first")
    }

    fn not_found(what: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, format!("no such {what}"))
    }

    fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }

    /// An unexpected failure, logged here and kept vague for the client.
    fn internal(error: impl Display, doing: &str) -> Self {
        tracing::error!(%error, "{doing} failed in the web API");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            message: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            code: Option<&'static str>,
        }
        (
            self.status,
            Json(Body {
                message: self.message,
                code: self.code,
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

impl From<pixiu_accounts::AccountError> for ApiError {
    fn from(error: pixiu_accounts::AccountError) -> Self {
        use pixiu_accounts::AccountError;
        match error {
            AccountError::Invalid(message) => Self::unprocessable(message),
            AccountError::NotFound => Self::not_found("user"),
            error @ AccountError::LinkExpired => {
                Self::new(StatusCode::GONE, error.to_string()).with_code("expired")
            }
            error @ AccountError::RegistrationClosed => {
                Self::new(StatusCode::NOT_FOUND, error.to_string())
            }
            error @ (AccountError::UsernameTaken
            | AccountError::EmailTaken
            | AccountError::LastAdmin
            | AccountError::AlreadySetUp
            | AccountError::NotPending
            | AccountError::IdentityTaken) => Self::new(StatusCode::CONFLICT, error.to_string()),
            error @ (AccountError::Db(_) | AccountError::Join(_)) => {
                Self::internal(error, "changing an account")
            }
        }
    }
}

impl From<tokio::task::JoinError> for ApiError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::internal(error, "a background task")
    }
}

impl From<pixiu_treasury::IngestError> for ApiError {
    fn from(error: pixiu_treasury::IngestError) -> Self {
        Self::internal(error, "changing the treasure")
    }
}

impl From<pixiu_treasury::OfferingError> for ApiError {
    fn from(error: pixiu_treasury::OfferingError) -> Self {
        use pixiu_treasury::OfferingError;
        match error {
            OfferingError::NotFound => Self::not_found("offering"),
            OfferingError::InvalidBatch => Self::not_found("batch"),
            OfferingError::Unreadable(reason) => {
                Self::unprocessable(format!("píxiū cannot read that file: {reason}"))
            }
            OfferingError::Archive(reason) => {
                Self::unprocessable(format!("That archive cannot be opened: {reason}"))
            }
            error => Self::internal(error, "handling offerings"),
        }
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// The signed-in user, and the API key they signed in with.
pub(crate) struct Session {
    pub user: User,
    pub key: ApiKey,
}

impl Session {
    /// The signed-in user's library: all they see and change.
    pub(crate) fn library(&self, state: &ApiState) -> Library {
        Library::new(state.db.clone(), self.user.id)
    }

    /// The signed-in user's id, whose library everything is in.
    pub(crate) fn owner(&self) -> u64 {
        self.user.id
    }
}

impl FromRequestParts<ApiState> for Session {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &ApiState) -> ApiResult<Self> {
        // A bearer token, or `api_key` in the query where browsers cannot
        // set headers (event streams, WebSockets).
        let from_query = || {
            form_urlencoded::parse(parts.uri.query().unwrap_or_default().as_bytes())
                .find(|(name, _)| name == "api_key")
                .map(|(_, value)| value.into_owned())
        };
        let key = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::to_owned)
            .or_else(from_query)
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
        // Disabled accounts (and ones not yet approved or confirmed) are
        // signed out wherever they were signed in.
        if user.status != UserStatus::Active {
            return Err(ApiError::unauthorized());
        }
        toasty::update!(key {
            last_used_at: Some(now()),
        })
        .exec(&mut db)
        .await?;
        Ok(Self { user, key })
    }
}

/// A signed-in admin.
pub(crate) struct AdminSession(pub Session);

impl FromRequestParts<ApiState> for AdminSession {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &ApiState) -> ApiResult<Self> {
        let session = Session::from_request_parts(parts, state).await?;
        if session.user.role == Role::Admin {
            Ok(Self(session))
        } else {
            Err(ApiError::forbidden("Only admins can do that."))
        }
    }
}
