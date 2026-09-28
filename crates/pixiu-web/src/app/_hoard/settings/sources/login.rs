//! `/settings/sources/login`: log in to YouTube Music in a browser that runs
//! on the server, shown here as a live picture.

use std::sync::Arc;

use pixiu_browser::{Field, Frame, Input, LoginDesk, Viewer, cookie_header};
use pixiu_jobs::adapters::{COOKIE_DOMAIN, is_logged_in};
use serde::Serialize;
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    asset::{Asset, asset},
    context::Cx,
    router::{
        content::{
            Json,
            websocket::{Message, WebSocket, WebSocketUpgrade},
        },
        error::{SeeOther, redirect, see_other},
        page,
        response::Response,
        route,
    },
    view::{View, view},
};

use super::{SOURCES_PATH, with_error};
use crate::{
    auth::{login_desk, require_user, warden},
    ui::{BUTTON_PRIMARY, BUTTON_SECONDARY, card},
};

const SCREEN_SCRIPT: Asset = asset!("assets/login-screen.js");

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    if !login_desk(cx).is_open().await {
        return Err(redirect(SOURCES_PATH).into());
    }
    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-6">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Log in to YouTube Music"</h2>
                <p class="text-muted-foreground">
                    "This is a browser running on your píxiū server. Log in as you would "
                    "anywhere else; you can paste into it. When you are signed in, press "
                    "Done."
                </p>
            </header>
            card(
                <canvas
                    id="screen"
                    width=(pixiu_browser::WIDTH)
                    height=(pixiu_browser::HEIGHT)
                    tabindex="0"
                    class="block h-auto w-full cursor-default rounded-lg border border-border bg-black outline-none focus:border-gold/60"
                ></canvas>
                // Owns the sign-in fields mirrored into the canvas (see the
                // script), so password managers see a login form.
                <form id="mirror" action="#"></form>
                <p id="autofill-hint" class="mt-3 text-xs text-muted-foreground"></p>
                <div class="mt-4 flex items-center justify-between gap-4">
                    <p id="login-status" class="text-sm text-muted-foreground">"Not signed in yet."</p>
                    <div class="flex items-center gap-2">
                        <form method="post" action="/settings/sources/login/cancel">
                            <button type="submit" class=(BUTTON_SECONDARY)>"Cancel"</button>
                        </form>
                        <form method="post" action="/settings/sources/login/finish">
                            <button id="done" type="submit" disabled="" class=(BUTTON_PRIMARY)>"Done"</button>
                        </form>
                    </div>
                </div>
            )
            <script type="module" src=(SCREEN_SCRIPT)></script>
        </div>
    })
}

/// Streams the screen as binary JPEG frames and replays JSON input. Text
/// messages describe the screen: `{"type":"size",...}` precedes the first
/// frame and any frame of a new size (input coordinates are in that space),
/// and `{"type":"fields",...}` lists the page's sign-in fields whenever they
/// change, for the page to mirror.
#[route(GET "./ws")]
async fn screen(cx: &Cx, upgrade: WebSocketUpgrade) -> Result<Response> {
    require_user(cx).await?;
    let desk = Arc::clone(login_desk(cx));
    upgrade.on_upgrade(move |socket| relay(desk, socket))
}

/// Sends a frame, announcing its size first when it changed.
async fn send_frame(
    socket: &mut WebSocket,
    size: &mut Option<(u32, u32)>,
    frame: Frame,
) -> Result<()> {
    if *size != Some((frame.width, frame.height)) {
        *size = Some((frame.width, frame.height));
        let announcement = serde_json::json!({
            "type": "size",
            "width": frame.width,
            "height": frame.height,
        });
        socket.send(Message::text(announcement.to_string())).await?;
    }
    socket.send(Message::binary(frame.jpeg)).await
}

async fn send_fields(socket: &mut WebSocket, fields: &[Field]) -> Result<()> {
    let message = serde_json::json!({ "type": "fields", "fields": fields });
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
        let _ = socket.close().await;
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

#[derive(Serialize)]
struct Status {
    open: bool,
    logged_in: bool,
}

/// Lets the page enable "Done" once the browser holds a login.
#[route(GET "./status")]
async fn status(cx: &Cx) -> Result<Json<Status>> {
    require_user(cx).await?;
    let desk = login_desk(cx);
    let open = desk.is_open().await;
    let logged_in = open && is_logged_in(&desk.cookies(COOKIE_DOMAIN).await.unwrap_or_default());
    Ok(Json(Status { open, logged_in }))
}

#[route(POST "./finish")]
async fn finish(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    let desk = login_desk(cx);
    let cookies = desk.cookies(COOKIE_DOMAIN).await.unwrap_or_default();
    if !is_logged_in(&cookies) {
        return Ok(see_other("/settings/sources/login"));
    }
    match warden(cx).connect(cookie_header(&cookies)).await {
        Ok(_) => {
            desk.close().await;
            Ok(see_other(format!("{SOURCES_PATH}?connected=1")))
        }
        Err(reason) => Ok(see_other(with_error(&format!(
            "YouTube Music did not accept the login: {reason}"
        )))),
    }
}

#[route(POST "./cancel")]
async fn cancel(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    login_desk(cx).close().await;
    Ok(see_other(SOURCES_PATH))
}
