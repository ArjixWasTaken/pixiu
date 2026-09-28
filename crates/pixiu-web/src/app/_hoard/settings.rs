//! `/settings`: how to connect Subsonic clients, and API keys.

mod api_keys;
mod sources;

use pixiu_db::{Album, ApiKey};
use pixiu_jobs::NewJob;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        page, route,
    },
    view::{View, attributes, class, component, view},
};

use crate::{
    app::_hoard::server_url,
    auth::{db, jobs, require_user, warden},
    ui::{BUTTON_DANGER, BUTTON_PRIMARY, BUTTON_SECONDARY, card, field, notice, session_status},
};

pub(super) const SETTINGS_PATH: &str = "/settings";

#[page]
async fn page() -> Result<impl View> {
    Ok(view! { settings() })
}

/// The settings page, optionally showing a just-created API key.
#[component]
pub(super) async fn settings(
    cx: &Cx,
    #[default] created_name: Option<&str>,
    #[default] created_key: Option<&str>,
) -> Result<impl View> {
    let user = require_user(cx).await?;
    let mut keys = ApiKey::filter_by_user_id(user.id).exec(&mut db(cx)).await?;
    keys.sort_by_key(|key| key.created_at);
    let token_ready = user.subsonic_secret.is_some();
    let server = server_url(cx);
    let (session, session_color) = session_status(warden(cx).health().state);
    let unlooked = Album::all()
        .exec(&mut db(cx))
        .await?
        .iter()
        .filter(|album| album.enrichment.is_none())
        .count();

    Ok(view! {
        <div class="mx-auto flex max-w-3xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Settings"</h2>
            </header>

            card(
                <div class="flex flex-col gap-4">
                    <h3 class="text-lg">"Subsonic clients"</h3>
                    <dl class="grid grid-cols-[8rem_1fr] gap-x-4 gap-y-2 text-sm">
                        <dt class="text-muted-foreground">"Server"</dt>
                        <dd><code class="text-gold-soft">(&server)</code></dd>
                        <dt class="text-muted-foreground">"Username"</dt>
                        <dd>(&user.username)</dd>
                        <dt class="text-muted-foreground">"Password"</dt>
                        <dd>
                            "Your píxiū password. "
                            if token_ready {
                                "Clients may send it hashed (token authentication)."
                            } else {
                                "Token authentication starts working after your next "
                                "sign-in."
                            }
                        </dd>
                    </dl>
                    <p class="text-sm text-muted-foreground">
                        "Clients that support OpenSubsonic API keys can use a key "
                        "instead of the password."
                    </p>
                </div>
            )

            card(
                <div class="flex items-center justify-between gap-4">
                    <div class="flex flex-col gap-1">
                        <h3 class="text-lg">"Sources"</h3>
                        <p class="flex items-center gap-2 text-sm text-muted-foreground">
                            "YouTube Music"
                            <span class=(class!("size-2.5 rounded-full", session_color))></span>
                            (session)
                        </p>
                    </div>
                    <a href="/settings/sources" class=(BUTTON_SECONDARY)>"Manage"</a>
                </div>
            )

            card(
                <div class="flex items-center justify-between gap-4">
                    <div class="flex flex-col gap-1">
                        <h3 class="text-lg">"MusicBrainz, covers and lyrics"</h3>
                        <p class="text-sm text-muted-foreground">
                            "New albums are looked up by themselves. "
                            if unlooked == 0 {
                                "Every album has been looked up."
                            } else {
                                (unlooked) if unlooked == 1 { " album has" } else { " albums have" }
                                " not been looked up yet."
                            }
                        </p>
                    </div>
                    if unlooked > 0 {
                        <form method="post" action="/settings/lookup" class="shrink-0">
                            <button type="submit" class=(BUTTON_SECONDARY)>"Look them up"</button>
                        </form>
                    }
                </div>
            )

            card(
                <div class="flex flex-col gap-4">
                    <h3 class="text-lg">"API keys"</h3>
                    if let (Some(name), Some(key)) = (created_name, created_key) {
                        notice(
                            "Key “" (name) "” created. Copy it now; it will not be shown again."
                        )
                        <code
                            class="block select-all break-all rounded-lg border \
                                   border-gold/40 bg-input px-3 py-2 text-sm text-gold-soft"
                        >
                            (key)
                        </code>
                    }
                    if keys.is_empty() {
                        <p class="text-sm text-muted-foreground">"No API keys yet."</p>
                    } else {
                        <ul class="divide-y divide-border">
                            for key in &keys {
                                <li class="flex items-center justify-between gap-4 py-3">
                                    <div>
                                        <p class="font-medium">(&key.name)</p>
                                        <p class="text-xs text-muted-foreground">
                                            "Created " (key.created_at.strftime("%Y-%m-%d").to_string())
                                            " · "
                                            match key.last_used_at {
                                                Some(used) => {
                                                    "last used "
                                                    (used.strftime("%Y-%m-%d %H:%M").to_string())
                                                }
                                                None => "never used",
                                            }
                                        </p>
                                    </div>
                                    <form
                                        method="post"
                                        action=(format!("/settings/api-keys/{}/revoke", key.id))
                                    >
                                        <button type="submit" class=(BUTTON_DANGER)>"Revoke"</button>
                                    </form>
                                </li>
                            }
                        </ul>
                    }
                    <form
                        method="post"
                        action="/settings/api-keys"
                        class="flex flex-col gap-3 sm:flex-row sm:items-end"
                    >
                        <div class="flex-1">
                            field(
                                label: "New key for",
                                attrs: attributes! {
                                    name="name" placeholder="e.g. Symfonium on my phone"
                                    required="" maxlength="100"
                                },
                            )
                        </div>
                        <button type="submit" class=(BUTTON_PRIMARY)>"Create key"</button>
                    </form>
                </div>
            )
        </div>
    })
}

/// Queues a lookup of every album never looked up.
#[route(POST "./lookup")]
async fn lookup(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    let albums = Album::all().exec(&mut db(cx)).await?;
    for album in albums.iter().filter(|album| album.enrichment.is_none()) {
        let title = format!("Look up {}", album.title);
        jobs(cx)
            .enqueue(NewJob::enrich(album.id, &title, None, false))
            .await?;
    }
    Ok(see_other("/jobs"))
}
