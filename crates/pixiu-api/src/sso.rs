//! Single sign-on: signing in through the provider, and linking an account
//! there to one here. Both leave for the provider with a cookie binding the
//! sign-in to the browser, and come back to the callback, which sends the
//! browser on to the player: `/sso/<code>` after a sign-in (the player
//! trades the code for its token), Settings after linking, or the sign-in
//! screen with `?sso_error=` saying what went wrong.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use pixiu_accounts::{
    AccountError, identities,
    oidc::{Intent, SsoError, Started},
};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{
    ApiError, ApiResult, ApiState, Session,
    auth::{Tokens, inactive, mint_key},
    throttle::{Action, ClientIp},
};

/// The cookie binding a sign-in to its browser.
const COOKIE: &str = "pixiu_oidc";
/// Where it is sent: the callback only.
const COOKIE_PATH: &str = "/api/auth/oidc";

fn secure(state: &ApiState) -> &'static str {
    let https = state
        .settings
        .get()
        .public_url
        .as_deref()
        .is_some_and(|url| url.starts_with("https://"));
    if https { "; Secure" } else { "" }
}

fn set_binding(state: &ApiState, value: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE}={value}; Path={COOKIE_PATH}; HttpOnly; SameSite=Lax; Max-Age=600{}",
        secure(state)
    ))
    .expect("cookies are ASCII")
}

fn clear_binding(state: &ApiState) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE}=; Path={COOKIE_PATH}; HttpOnly; SameSite=Lax; Max-Age=0{}",
        secure(state)
    ))
    .expect("cookies are ASCII")
}

fn binding(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, value)| value.to_owned())
}

/// Sends the browser to `location` (in the player).
fn redirect(location: &str, cookie: Option<HeaderValue>) -> Response {
    let mut response = (
        StatusCode::SEE_OTHER,
        [(
            header::LOCATION,
            HeaderValue::from_str(location).expect("paths are ASCII"),
        )],
    )
        .into_response();
    if let Some(cookie) = cookie {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

fn why(error: &SsoError) -> &'static str {
    match error {
        SsoError::Unavailable => "unavailable",
        SsoError::Expired => "expired",
        SsoError::Refused(_) => "denied",
        SsoError::Provider(_) => "failed",
    }
}

/// `GET /api/auth/oidc/start`: off to the provider to sign in.
pub(crate) async fn start(State(state): State<ApiState>, ClientIp(ip): ClientIp) -> Response {
    if !state.throttle.allow(Action::SsoStart, &ip) {
        return redirect("/?sso_error=busy", None);
    }
    match state.sso.start(Intent::Login).await {
        Ok(Started { url, binding }) => redirect(&url, Some(set_binding(&state, &binding))),
        Err(error) => {
            tracing::warn!(%error, "cannot start a single sign-on");
            redirect(&format!("/?sso_error={}", why(&error)), None)
        }
    }
}

/// `POST /api/me/identities/oidc`: where to send the browser to link an
/// account at the provider to the user's.
pub(crate) async fn link(State(state): State<ApiState>, session: Session) -> ApiResult<Response> {
    let Started { url, binding } = state
        .sso
        .start(Intent::Link(session.user.id))
        .await
        .map_err(|error| match error {
            SsoError::Unavailable => ApiError::unprocessable("Single sign-on is not set up."),
            error => {
                tracing::warn!(%error, "cannot start linking an account");
                ApiError::new(StatusCode::BAD_GATEWAY, error.to_string())
            }
        })?;
    let mut response = Json(json!({ "url": url })).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, set_binding(&state, &binding));
    Ok(response)
}

#[derive(Deserialize)]
pub(crate) struct Callback {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// `GET /api/auth/oidc/callback`: back from the provider.
pub(crate) async fn callback(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(callback): Query<Callback>,
) -> Response {
    let clear = Some(clear_binding(&state));
    let Some(pending) = callback
        .state
        .as_deref()
        .and_then(|key| state.sso.take(key))
    else {
        return redirect("/?sso_error=expired", clear);
    };
    let intent = pending.intent();
    let outcome = match (callback.code, callback.error) {
        (Some(code), None) => {
            state
                .sso
                .complete(pending, &code, binding(&headers).as_deref())
                .await
        }
        (_, error) => Err(SsoError::Refused(
            callback
                .error_description
                .or(error)
                .unwrap_or_else(|| "no code".to_owned()),
        )),
    };
    if let Err(error) = &outcome {
        tracing::warn!(%error, ?intent, "single sign-on failed");
    }
    let mut db = state.db.clone();
    let location = match (intent, outcome) {
        (Intent::Login, Ok(identity)) => match identities::owner(&mut db, &identity).await {
            Ok(None) => "/?sso_error=unlinked".to_owned(),
            Ok(Some((_, user))) if inactive(user.status).is_some() => {
                format!("/?sso_error={}", crate::admin::status_name(user.status))
            }
            Ok(Some((link, user))) => {
                if let Err(error) = identities::signed_in(&mut db, link, &identity).await {
                    tracing::error!(%error, "cannot note a single sign-on");
                }
                tracing::info!(user = user.id, "signed in with single sign-on");
                format!("/sso/{}", state.sso.issue_code(user.id))
            }
            Err(error) => {
                tracing::error!(%error, "cannot look up a single sign-on");
                "/?sso_error=failed".to_owned()
            }
        },
        (Intent::Login, Err(error)) => format!("/?sso_error={}", why(&error)),
        (Intent::Link(user_id), Ok(identity)) => {
            match identities::link(&mut db, user_id, &identity).await {
                Ok(_) => "/settings?linked=1#account".to_owned(),
                Err(AccountError::IdentityTaken) => "/settings?link_error=taken#account".to_owned(),
                Err(error) => {
                    tracing::error!(%error, "cannot link an account");
                    "/settings?link_error=failed#account".to_owned()
                }
            }
        }
        (Intent::Link(_), Err(error)) => {
            format!("/settings?link_error={}#account", why(&error))
        }
    };
    redirect(&location, clear)
}

#[derive(Deserialize)]
pub(crate) struct Exchange {
    code: String,
}

/// `POST /api/auth/oidc/exchange`: trades a sign-in's code for its token.
pub(crate) async fn exchange(
    State(state): State<ApiState>,
    Json(exchange): Json<Exchange>,
) -> ApiResult<Json<Tokens>> {
    let expired = || {
        ApiError::new(StatusCode::GONE, "This sign-in expired; sign in again.").with_code("expired")
    };
    let user_id = state.sso.redeem_code(&exchange.code).ok_or_else(expired)?;
    let user = pixiu_db::User::filter_by_id(user_id)
        .first()
        .exec(&mut state.db.clone())
        .await?
        .ok_or_else(expired)?;
    if let Some(refusal) = inactive(user.status) {
        return Err(refusal);
    }
    Ok(Json(Tokens::of(mint_key(&state, &user).await?)))
}

/// `GET /api/me/identities`: the user's linked accounts.
pub(crate) async fn list(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let settings = state.settings.get();
    let provider = settings.oidc.as_ref();
    let links = identities::of(&mut state.db.clone(), session.user.id).await?;
    Ok(Json(json!(
        links
            .iter()
            .map(|link| json!({
                "id": link.id,
                "provider": provider
                    .filter(|oidc| oidc.issuer == link.issuer)
                    .map_or(link.issuer.as_str(), |oidc| oidc.name.as_str()),
                "email": link.email,
                "linked_at": link.linked_at,
                "last_login_at": link.last_login_at,
            }))
            .collect::<Vec<_>>()
    )))
}

/// `DELETE /api/me/identities/{id}`: unlinks one.
pub(crate) async fn unlink(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    identities::unlink(&mut state.db.clone(), session.user.id, id)
        .await
        .map_err(|error| match error {
            AccountError::NotFound => ApiError::not_found("linked account"),
            error => error.into(),
        })?;
    Ok(StatusCode::NO_CONTENT)
}
