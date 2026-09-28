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
    ui::{CARD_TITLE, EYEBROW, Size, Tone, btn, dot, outlined, relative, session_look, snackbar},
};

pub(super) const SOURCES_PATH: &str = "/settings/sources";

#[query_params(error = bad_request)]
struct SourcesQuery {
    connected: Option<String>,
    checked: Option<String>,
    refreshed: Option<String>,
    disconnected: Option<String>,
    error: Option<String>,
}

fn when(timestamp: Option<Timestamp>) -> String {
    timestamp.map_or_else(|| "—".to_owned(), relative)
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<SourcesQuery>(cx)?;
    let warden = warden(cx);
    let health = warden.health();
    let events = warden.events(8).await?;
    let look = session_look(health.state);
    let working = matches!(
        health.state,
        Some(SessionState::Valid | SessionState::Degraded)
    );

    Ok(view! {
        <div class="flex flex-col gap-5">
            if query.connected.is_some() {
                snackbar(message: "Connected to YouTube Music.")
            }
            if query.checked.is_some() {
                snackbar(message: if working {
                    "Checked: the session works."
                } else {
                    "Checked: the session does not work."
                })
            }
            if query.refreshed.is_some() {
                snackbar(message: if working {
                    "Cookies refreshed."
                } else {
                    "The cookies could not be refreshed. Log in again."
                })
            }
            if query.disconnected.is_some() {
                snackbar(message: "Disconnected from YouTube Music.")
            }
            if let Some(error) = &query.error {
                snackbar(message: error, error: true)
            }

            <a href="/settings" class="self-start text-sm">"Settings"</a>
            <section class="flex flex-wrap items-center gap-6 rounded-[28px] bg-card p-7">
                <div class="flex flex-[1_1_300px] flex-col gap-2.5">
                    <span class=(EYEBROW)>"Source · YouTube Music"</span>
                    <div class="flex items-center gap-3.5">
                        dot(look: look, big: true)
                        <h1 class="m-0 text-4xl leading-11 font-normal">(look.long)</h1>
                    </div>
                    match health.state {
                        Some(SessionState::Expired) => {
                            <p class="m-0 max-w-[56ch] text-sm leading-5 text-foreground-soft">
                                "Public music can still be hunted; your account’s music cannot. Watches on "
                                "your playlists and likes wait until you log in again."
                            </p>
                        },
                        Some(SessionState::Degraded) => {
                            <p class="m-0 max-w-[56ch] text-sm leading-5 text-foreground-soft">
                                "The last check could not reach YouTube Music. It is tried again soon."
                            </p>
                        },
                        Some(SessionState::Valid) => {
                            <p class="m-0 max-w-[56ch] text-sm leading-5 text-foreground-soft">
                                "Your playlists and liked music can be hunted and watched."
                            </p>
                        },
                        None => {
                            <p class="m-0 text-sm leading-5 text-foreground-soft">
                                "Log in to hunt your playlists and liked music. Public music needs no login."
                            </p>
                        },
                    }
                </div>
                <div class="flex flex-wrap gap-2">
                    <form method="post" action="/settings/sources/connect">
                        <button type="submit" class=(btn(if working { Tone::Outlined } else { Tone::Filled }, Size::S))>
                            match health.state {
                                None => "Log in",
                                Some(SessionState::Expired) => "Log in again",
                                Some(_) => "Log in with another account",
                            }
                        </button>
                    </form>
                    if health.state.is_some() {
                        <form method="post" action="/settings/sources/validate">
                            <button type="submit" class=(btn(Tone::Tonal, Size::S))>"Check now"</button>
                        </form>
                        <form method="post" action="/settings/sources/refresh">
                            <button type="submit" class=(btn(Tone::Outlined, Size::S))>"Refresh cookies"</button>
                        </form>
                        <form method="post" action="/settings/sources/disconnect">
                            <button type="submit" class=(btn(Tone::Text, Size::S))>"Disconnect"</button>
                        </form>
                    }
                </div>
            </section>
            <div class="grid grid-cols-[repeat(auto-fit,minmax(min(100%,320px),1fr))] items-start gap-5">
                outlined(
                    <h2 class=(CARD_TITLE)>"Details"</h2>
                    <dl class="m-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-5 gap-y-3 text-sm">
                        <dt class="text-muted-foreground">"Connected"</dt>
                        <dd class="m-0">(when(health.connected_at))</dd>
                        <dt class="text-muted-foreground">"Last checked"</dt>
                        <dd class="m-0">(when(health.last_verified))</dd>
                        <dt class="text-muted-foreground">"Cookies refreshed"</dt>
                        <dd class="m-0">(when(health.last_refreshed))</dd>
                        <dt class="text-muted-foreground">"Expired"</dt>
                        <dd class="m-0">(when(health.expired_at))</dd>
                        <dt class="text-muted-foreground">"Last problem"</dt>
                        <dd class="m-0 leading-5 break-words">(health.last_error.as_deref().unwrap_or("None"))</dd>
                    </dl>
                )
                outlined(
                    <h2 class=(CARD_TITLE)>"History"</h2>
                    if events.is_empty() {
                        <p class="m-0 text-sm text-muted-foreground">"Nothing has happened yet."</p>
                    }
                    for event in &events {
                        <div class="flex gap-3.5">
                            <span
                                class=(class!(
                                    "mt-[5px] size-2.5 shrink-0 rounded-full",
                                    "bg-destructive" if event.kind.is_problem() else "bg-slate-soft",
                                ))
                            ></span>
                            <div class="flex flex-col gap-0.5">
                                <span class="text-[15px]">(&event.message)</span>
                                <span
                                    class="text-xs text-slate-soft"
                                    title=(event.created_at.strftime("%Y-%m-%d %H:%M UTC").to_string())
                                >
                                    (relative(event.created_at))
                                </span>
                            </div>
                        </div>
                    }
                )
            </div>
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
    Ok(see_other(format!("{SOURCES_PATH}?checked=1")))
}

#[route(POST "./refresh")]
async fn refresh(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    warden(cx).refresh().await;
    Ok(see_other(format!("{SOURCES_PATH}?refreshed=1")))
}

#[route(POST "./disconnect")]
async fn disconnect(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    warden(cx).disconnect().await;
    Ok(see_other(format!("{SOURCES_PATH}?disconnected=1")))
}
