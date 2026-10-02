//! The account with a platform: the session's health and history, and the
//! login browser, a real browser on the server whose screen streams to the
//! player over a WebSocket while the user signs in. YouTube Music is the
//! only platform with logins so far; another would name its own here.

use std::sync::Arc;

use axum::{
    Json,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use pixiu_browser::{BrowserError, Field, Frame, Input, LoginDesk, Viewer, cookie_header};
use pixiu_db::{Platform, SessionState};
use pixiu_hunt::LoginSpec;
use pixiu_jobs::{Health, Warden, adapters::is_logged_in};
use serde_json::{Value as JsonValue, json};
use tokio::sync::broadcast::error::RecvError;

use crate::{ApiError, ApiResult, ApiState, Session};

/// The session's state as the player names it.
pub(crate) fn state_name(state: Option<SessionState>) -> &'static str {
    match state {
        None => "none",
        Some(SessionState::Valid) => "valid",
        Some(SessionState::Degraded) => "degraded",
        Some(SessionState::Expired) => "expired",
    }
}

/// The platform whose account these endpoints manage.
pub(crate) const PLATFORM: Platform = Platform::YouTubeMusic;

/// `owner`'s session health; nothing when they never connected one.
pub(crate) fn health_of(state: &ApiState, owner: u64) -> Health {
    state.wardens.health(owner, PLATFORM)
}

/// The signed-in user's warden.
async fn warden(state: &ApiState, session: &Session) -> ApiResult<Arc<Warden>> {
    Ok(state.wardens.get(session.owner(), PLATFORM).await?)
}

/// How the platform's login works.
fn login(state: &ApiState) -> ApiResult<LoginSpec> {
    state
        .hunter
        .platforms()
        .get(PLATFORM)?
        .login()
        .ok_or_else(|| ApiError::not_found("login"))
}

/// The signed-in user's login browser.
fn desk(state: &ApiState, session: &Session) -> Arc<LoginDesk> {
    state.desks.desk(session.owner())
}

fn describe(health: &Health) -> JsonValue {
    json!({
        "state": state_name(health.state),
        "connected_at": health.connected_at,
        "last_verified": health.last_verified,
        "last_refreshed": health.last_refreshed,
        "expired_at": health.expired_at,
        "last_error": health.last_error,
    })
}

/// `GET /api/sources`: the session's health and latest events.
pub(crate) async fn status(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let warden = warden(&state, &session).await?;
    let events: Vec<JsonValue> = warden
        .events(8)
        .await?
        .iter()
        .map(|event| {
            json!({
                "message": event.message,
                "problem": event.kind.is_problem(),
                "created_at": event.created_at,
            })
        })
        .collect();
    Ok(Json(json!({
        "health": describe(&warden.health()),
        "events": events,
        "login_open": desk(&state, &session).is_open().await,
    })))
}

/// `POST /api/sources/validate`: checks the session now.
pub(crate) async fn validate(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    Ok(Json(describe(
        &warden(&state, &session).await?.validate().await,
    )))
}

/// `POST /api/sources/refresh`: renews the session's cookies now.
pub(crate) async fn refresh(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    Ok(Json(describe(
        &warden(&state, &session).await?.refresh().await,
    )))
}

/// `POST /api/sources/disconnect`: forgets the session.
pub(crate) async fn disconnect(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<StatusCode> {
    warden(&state, &session).await?.disconnect().await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/sources/login`: starts the login browser on the platform's
/// sign-in.
pub(crate) async fn open_login(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<StatusCode> {
    desk(&state, &session)
        .open(login(&state)?.start_url)
        .await
        .map_err(|error| match error {
            BrowserError::Busy => ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "Another sign-in is running on the server; try again in a few minutes.",
            ),
            error => {
                tracing::error!(%error, "cannot open the login browser");
                ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("The login browser could not start: {error}"),
                )
            }
        })?;
    Ok(StatusCode::CREATED)
}

/// `DELETE /api/sources/login`: closes the login browser.
pub(crate) async fn cancel_login(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<StatusCode> {
    desk(&state, &session).close().await;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/sources/login/status`: lets the player enable "Done" once the
/// browser holds a login.
pub(crate) async fn login_status(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let login = login(&state)?;
    let desk = desk(&state, &session);
    let open = desk.is_open().await;
    let logged_in = open
        && is_logged_in(
            &login,
            &desk.cookies(login.cookie_domain).await.unwrap_or_default(),
        );
    Ok(Json(
        json!({ "open": open, "logged_in": logged_in, "host": desk.host().await }),
    ))
}

/// `POST /api/sources/login/finish`: hands the browser's login to píxiū.
pub(crate) async fn finish_login(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let login = login(&state)?;
    let desk = desk(&state, &session);
    let cookies = desk.cookies(login.cookie_domain).await.unwrap_or_default();
    if !is_logged_in(&login, &cookies) {
        return Err(ApiError::unprocessable(
            "The login browser is not signed in yet.",
        ));
    }
    match warden(&state, &session)
        .await?
        .connect(cookie_header(&cookies))
        .await
    {
        Ok(health) => {
            desk.close().await;
            Ok(Json(describe(&health)))
        }
        Err(reason) => Err(ApiError::unprocessable(format!(
            "{} did not accept the login: {reason}",
            PLATFORM.name()
        ))),
    }
}

/// Whether the request comes from a page of this server. The key already
/// authenticates the socket; this keeps other sites' pages from opening
/// one with a key they somehow hold.
fn same_origin(headers: &HeaderMap) -> bool {
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        // Not a browser.
        return true;
    };
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    let origin_host = origin.split_once("://").map(|(_, rest)| rest);
    host.is_some() && origin_host == host
}

/// `GET /api/sources/login/ws?api_key=`: streams the screen as binary
/// JPEG frames and replays JSON input. Text messages describe the screen:
/// `{"type":"size",...}` precedes the first frame and any frame of a new
/// size (input coordinates are in that space), and `{"type":"fields",...}`
/// lists the page's sign-in fields whenever they change, for the player to
/// mirror.
pub(crate) async fn screen(
    State(state): State<ApiState>,
    session: Session,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> ApiResult<Response> {
    if !same_origin(&headers) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "wrong origin"));
    }
    let desk = desk(&state, &session);
    Ok(upgrade.on_upgrade(move |socket| relay(desk, socket)))
}

/// Sends a frame, announcing its size first when it changed.
async fn send_frame(
    socket: &mut WebSocket,
    size: &mut Option<(u32, u32)>,
    frame: Frame,
) -> Result<(), axum::Error> {
    if *size != Some((frame.width, frame.height)) {
        *size = Some((frame.width, frame.height));
        let announcement = json!({ "type": "size", "width": frame.width, "height": frame.height });
        socket.send(Message::text(announcement.to_string())).await?;
    }
    socket.send(Message::binary(frame.jpeg)).await
}

async fn send_fields(socket: &mut WebSocket, fields: &[Field]) -> Result<(), axum::Error> {
    let message = json!({ "type": "fields", "fields": fields });
    socket.send(Message::text(message.to_string())).await
}

async fn relay(desk: Arc<LoginDesk>, mut socket: WebSocket) {
    let Some(Viewer {
        latest,
        mut frames,
        mut fields,
    }) = desk.watch().await
    else {
        tracing::debug!("no login browser to show");
        let _ = socket.send(Message::Close(None)).await;
        return;
    };
    let mut size = None;
    if let Some(frame) = latest
        && send_frame(&mut socket, &mut size, frame).await.is_err()
    {
        return;
    }
    let current = fields.borrow_and_update().clone();
    if send_fields(&mut socket, &current).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            frame = frames.recv() => match frame {
                Ok(frame) => {
                    if send_frame(&mut socket, &mut size, frame).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => break,
            },
            changed = fields.changed() => {
                if changed.is_err() {
                    break;
                }
                let current = fields.borrow_and_update().clone();
                if send_fields(&mut socket, &current).await.is_err() {
                    break;
                }
            },
            message = socket.recv() => match message {
                Some(Ok(Message::Text(text))) => {
                    match serde_json::from_str::<Input>(text.as_str()) {
                        Ok(input) => {
                            if let Err(error) = desk.input(input).await {
                                tracing::warn!(%error, "login browser input failed");
                            }
                        }
                        // The error could quote a typed password.
                        Err(error) => {
                            tracing::debug!(category = ?error.classify(), "ignoring malformed input");
                        }
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
        }
    }
}
