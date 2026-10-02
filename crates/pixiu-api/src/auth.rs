//! Signing in and out. A successful sign-in (or setting up a fresh server)
//! mints an API key named after the browser ("Web session: Firefox on
//! Linux"), which the player keeps and sends
//! as a bearer token here and as `apiKey` to the Subsonic API. Signing out
//! revokes it. Forgotten passwords and email confirmations are here too.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
};
use pixiu_accounts::{links, registration, users};
use pixiu_db::{ApiKey, User, UserStatus, now, toasty};
use serde::{Deserialize, Serialize};

use crate::{
    ApiError, ApiResult, ApiState, Session,
    throttle::{Action, ClientIp, too_many},
};

#[derive(Serialize)]
pub(crate) struct Status {
    /// Whether an account exists; until one does, the player offers to set
    /// píxiū up instead of signing in.
    claimed: bool,
    /// Whether a forgotten password can be reset by email.
    password_reset: bool,
    /// Whether anyone may ask for an account.
    registration: bool,
    /// The single sign-on provider people may sign in with.
    sso: Option<Sso>,
}

#[derive(Serialize)]
pub(crate) struct Sso {
    /// What the button says: "Sign in with <name>".
    name: String,
}

/// `GET /api/auth/status`.
pub(crate) async fn status(State(state): State<ApiState>) -> ApiResult<Json<Status>> {
    let claimed = User::all()
        .first()
        .exec(&mut state.db.clone())
        .await?
        .is_some();
    let settings = state.settings.get();
    Ok(Json(Status {
        claimed,
        password_reset: state.mailer.ready(),
        registration: settings.registration_available(),
        sso: settings.sso().map(|oidc| Sso {
            name: oidc.name.clone(),
        }),
    }))
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

/// What a browser calls itself, as "Firefox on Linux": its own name, then
/// its system's, from the User-Agent. Either may be unknown.
fn browser_name(headers: &HeaderMap) -> Option<String> {
    let agent = headers.get(header::USER_AGENT)?.to_str().ok()?;
    let has = |needle: &str| agent.contains(needle);
    // Order matters: Edge and Opera say Chrome, Chrome says Safari.
    let browser = if has("Edg/") {
        Some("Edge")
    } else if has("OPR/") {
        Some("Opera")
    } else if has("Firefox/") || has("FxiOS/") {
        Some("Firefox")
    } else if has("Chromium/") {
        Some("Chromium")
    } else if has("Chrome/") || has("CriOS/") {
        Some("Chrome")
    } else if has("Safari/") {
        Some("Safari")
    } else {
        None
    };
    let system = if has("Android") {
        Some("Android")
    } else if has("iPhone") || has("iPad") {
        Some("iOS")
    } else if has("Windows") {
        Some("Windows")
    } else if has("Macintosh") || has("Mac OS X") {
        Some("macOS")
    } else if has("CrOS") {
        Some("ChromeOS")
    } else if has("Linux") {
        Some("Linux")
    } else {
        None
    };
    match (browser, system) {
        (Some(browser), Some(system)) => Some(format!("{browser} on {system}")),
        (Some(name), None) | (None, Some(name)) => Some(name.to_owned()),
        (None, None) => None,
    }
}

/// A web session's name: "Web session: Firefox on Linux", so people can
/// tell their browsers apart in the API keys.
fn session_name(headers: &HeaderMap) -> String {
    browser_name(headers).map_or_else(
        || users::WEB_SESSION.to_owned(),
        |browser| format!("{}: {browser}", users::WEB_SESSION),
    )
}

/// Makes a new web session key for `user`, named after the browser asking;
/// returns it in the clear, once.
pub(crate) async fn mint_key(
    state: &ApiState,
    user: &User,
    headers: &HeaderMap,
) -> ApiResult<String> {
    let key = new_key();
    toasty::create!(ApiKey {
        user_id: user.id,
        name: session_name(headers),
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
            "Confirm your email address first: follow the link píxiū sent you. Lost it? \
             “Forgot password?” sends one that confirms it too.",
        ),
        UserStatus::Disabled => ("disabled", "Your account is turned off."),
    };
    Some(ApiError::forbidden(message).with_code(code))
}

/// `POST /api/auth/login`.
pub(crate) async fn login(
    State(state): State<ApiState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Json(credentials): Json<Credentials>,
) -> ApiResult<Json<Tokens>> {
    if state.throttle.blocked(Action::FailedLogin, &ip) {
        return Err(too_many());
    }
    let login = credentials.username.trim();
    let mut db = state.db.clone();
    let user = users::find_by_login(&mut db, login).await?;
    // Always verify, even for unknown users, so timing reveals nothing.
    let hash = user.as_ref().map(|user| user.password_hash.clone());
    let verified = verify(credentials.password.clone(), hash).await?;
    let Some(mut user) = user.filter(|_| verified) else {
        tracing::warn!(login, ip, "failed sign-in");
        state.throttle.hit(Action::FailedLogin, &ip);
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
    Ok(Json(Tokens::of(mint_key(&state, &user, &headers).await?)))
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
    headers: HeaderMap,
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
    Ok(Json(Tokens::of(mint_key(&state, &user, &headers).await?)))
}

/// `DELETE /api/auth/session`: revokes the key this request came with.
pub(crate) async fn logout(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<StatusCode> {
    session.key.delete().exec(&mut state.db.clone()).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct Forgot {
    /// A username or an email address.
    login: String,
}

/// `POST /api/auth/forgot`: emails a reset link to the account, if there
/// is one that may have it. The answer is the same either way, and comes at
/// once, so it tells nobody which accounts exist.
pub(crate) async fn forgot(
    State(state): State<ApiState>,
    ClientIp(ip): ClientIp,
    Json(forgot): Json<Forgot>,
) -> ApiResult<StatusCode> {
    state.throttle.check(Action::Forgot, &ip)?;
    tokio::spawn(async move {
        let mut db = state.db.clone();
        let user = match users::find_by_login(&mut db, forgot.login.trim()).await {
            Ok(Some(user)) if links::may_reset(&user) => user,
            Ok(_) => return,
            Err(error) => {
                tracing::error!(%error, "cannot look up who forgot their password");
                return;
            }
        };
        if !state
            .throttle
            .allow(Action::ResetEmail, &user.id.to_string())
        {
            tracing::warn!(
                user = user.id,
                "too many password reset emails; not sending another"
            );
            return;
        }
        if let Err(error) = links::send_reset(&mut db, &state.mailer, &user).await {
            tracing::warn!(%error, user = user.id, "cannot send a password reset email");
        }
    });
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct Reset {
    token: String,
    password: String,
}

/// `POST /api/auth/reset`: follows a reset link, and signs in with the new
/// password (when the account works).
pub(crate) async fn reset(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(reset): Json<Reset>,
) -> ApiResult<Json<Tokens>> {
    let user = links::reset_password(
        &mut state.db.clone(),
        &state.secrets,
        &reset.token,
        &reset.password,
    )
    .await?;
    if let Some(refusal) = inactive(user.status) {
        return Err(refusal);
    }
    Ok(Json(Tokens::of(mint_key(&state, &user, &headers).await?)))
}

#[derive(Deserialize)]
pub(crate) struct Verification {
    token: String,
}

/// `POST /api/auth/verify-email`: follows a confirmation link. Says
/// whether that opened the account (an approved registration).
pub(crate) async fn verify_email(
    State(state): State<ApiState>,
    Json(verification): Json<Verification>,
) -> ApiResult<Json<serde_json::Value>> {
    let confirmed = links::verify_email(&mut state.db.clone(), &verification.token).await?;
    Ok(Json(serde_json::json!({ "opened": confirmed.opened })))
}

#[derive(Deserialize)]
pub(crate) struct Registration {
    username: String,
    email: String,
    password: String,
}

/// `POST /api/auth/register`: asks for an account, which an admin approves
/// or denies. An address that has an account already gets the same answer
/// (its owner gets a note), so the answer tells nobody who has one.
pub(crate) async fn register(
    State(state): State<ApiState>,
    ClientIp(ip): ClientIp,
    Json(form): Json<Registration>,
) -> ApiResult<StatusCode> {
    if !state.settings.get().registration_available() {
        return Err(pixiu_accounts::AccountError::RegistrationClosed.into());
    }
    state.throttle.check(Action::Register, &ip)?;
    registration::request(
        &mut state.db.clone(),
        &state.secrets,
        &state.mailer,
        registration::Request {
            username: form.username,
            email: form.email,
            password: form.password,
        },
    )
    .await?;
    Ok(StatusCode::ACCEPTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(agent: &str) -> String {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, agent.parse().unwrap());
        session_name(&headers)
    }

    #[test]
    fn web_sessions_are_named_after_the_browser() {
        assert_eq!(
            named("Mozilla/5.0 (X11; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0"),
            "Web session: Firefox on Linux"
        );
        assert_eq!(
            named(
                "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/153.0.0.0 Mobile Safari/537.36"
            ),
            "Web session: Chrome on Android"
        );
        assert_eq!(
            named(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/153.0.0.0 Safari/537.36 Edg/153.0.0.0"
            ),
            "Web session: Edge on Windows"
        );
        assert_eq!(
            named(
                "Mozilla/5.0 (iPhone; CPU iPhone OS 26_0 like Mac OS X) AppleWebKit/605.1.15 \
                 (KHTML, like Gecko) Version/26.0 Mobile/15E148 Safari/604.1"
            ),
            "Web session: Safari on iOS"
        );
        assert_eq!(named("curl/8.0"), "Web session");
        assert_eq!(session_name(&HeaderMap::new()), "Web session");
    }
}
