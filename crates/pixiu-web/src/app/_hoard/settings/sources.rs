//! `/settings/sources`: the YouTube Music connection and its health.

mod login;

use jiff::Timestamp;
use pixiu_db::SessionState;
use pixiu_jobs::adapters::LOGIN_URL;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        page, query_params, route,
    },
    view::{View, class, view},
};

use crate::{
    auth::{login_desk, require_user, warden},
    ui::{BUTTON_DANGER, BUTTON_PRIMARY, BUTTON_SECONDARY, alert, card, notice, session_status},
};

pub(super) const SOURCES_PATH: &str = "/settings/sources";

#[query_params(error = bad_request)]
struct SourcesQuery {
    connected: Option<String>,
    error: Option<String>,
}

fn when(timestamp: Option<Timestamp>) -> String {
    timestamp.map_or_else(
        || "never".to_owned(),
        |timestamp| timestamp.strftime("%Y-%m-%d %H:%M UTC").to_string(),
    )
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<SourcesQuery>(cx)?;
    let warden = warden(cx);
    let health = warden.health();
    let events = warden.events(8).await?;
    let (status, color) = session_status(health.state);
    let needs_login = matches!(health.state, None | Some(SessionState::Expired));

    Ok(view! {
        <div class="mx-auto flex max-w-3xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Sources"</h2>
                <p class="text-muted-foreground">
                    "Accounts píxiū hunts with. Logging in unlocks your liked music "
                    "and private playlists; public music needs no login."
                </p>
            </header>

            if query.connected.is_some() {
                notice("YouTube Music is connected.")
            }
            if let Some(error) = &query.error {
                alert((error))
            }

            card(
                <div class="flex flex-col gap-5">
                    <div class="flex items-center justify-between gap-4">
                        <h3 class="text-lg">"YouTube Music"</h3>
                        <span class="flex items-center gap-2 text-sm">
                            <span class=(class!("size-2.5 rounded-full", color))></span>
                            (status)
                        </span>
                    </div>
                    if health.state.is_some() {
                        <dl class="grid grid-cols-[10rem_1fr] gap-x-4 gap-y-1.5 text-sm">
                            <dt class="text-muted-foreground">"Connected"</dt>
                            <dd>(when(health.connected_at))</dd>
                            <dt class="text-muted-foreground">"Last checked"</dt>
                            <dd>(when(health.last_verified))</dd>
                            <dt class="text-muted-foreground">"Cookies refreshed"</dt>
                            <dd>(when(health.last_refreshed))</dd>
                            if let Some(expired) = health.expired_at {
                                <dt class="text-muted-foreground">"Expired"</dt>
                                <dd>(when(Some(expired)))</dd>
                            }
                            if let Some(error) = &health.last_error {
                                <dt class="text-muted-foreground">"Last problem"</dt>
                                <dd class="text-destructive">(error)</dd>
                            }
                        </dl>
                    }
                    <div class="flex flex-wrap gap-2">
                        if needs_login {
                            <form method="post" action="/settings/sources/connect">
                                <button type="submit" class=(BUTTON_PRIMARY)>
                                    if health.state.is_some() { "Log in again" } else { "Log in" }
                                </button>
                            </form>
                        }
                        if health.state.is_some() {
                            <form method="post" action="/settings/sources/validate">
                                <button type="submit" class=(BUTTON_SECONDARY)>"Check now"</button>
                            </form>
                            <form method="post" action="/settings/sources/refresh">
                                <button type="submit" class=(BUTTON_SECONDARY)>"Refresh cookies"</button>
                            </form>
                            <form method="post" action="/settings/sources/disconnect">
                                <button type="submit" class=(BUTTON_DANGER)>"Disconnect"</button>
                            </form>
                        }
                    </div>
                    if !events.is_empty() {
                        <div class="border-t border-border pt-4">
                            <h4 class="mb-2 text-sm font-medium">"History"</h4>
                            <ul class="flex flex-col gap-1 text-sm text-muted-foreground">
                                for event in &events {
                                    <li>
                                        <span class="tabular-nums">
                                            (event.created_at.strftime("%Y-%m-%d %H:%M").to_string())
                                        </span>
                                        " · " (&event.message)
                                    </li>
                                }
                            </ul>
                        </div>
                    }
                </div>
            )
        </div>
    })
}

/// The sources page showing `message` as an error.
fn with_error(message: &str) -> String {
    format!(
        "{SOURCES_PATH}?error={}",
        form_urlencoded::byte_serialize(message.as_bytes()).collect::<String>()
    )
}

/// Opens the login browser.
#[route(POST "./connect")]
async fn connect(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    match login_desk(cx).open(LOGIN_URL).await {
        Ok(()) => Ok(see_other("/settings/sources/login")),
        Err(error) => {
            tracing::error!(%error, "cannot open the login browser");
            Ok(see_other(with_error(&format!(
                "The login browser could not start: {error}"
            ))))
        }
    }
}

#[route(POST "./validate")]
async fn validate(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    warden(cx).validate().await;
    Ok(see_other(SOURCES_PATH))
}

#[route(POST "./refresh")]
async fn refresh(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    warden(cx).refresh().await;
    Ok(see_other(SOURCES_PATH))
}

#[route(POST "./disconnect")]
async fn disconnect(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    warden(cx).disconnect().await;
    Ok(see_other(SOURCES_PATH))
}
