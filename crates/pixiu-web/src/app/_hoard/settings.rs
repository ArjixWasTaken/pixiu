//! `/settings`: how to connect Subsonic clients, and API keys.

mod api_keys;

use pixiu_db::ApiKey;
use topcoat::{
    Result,
    context::Cx,
    router::page,
    view::{View, attributes, component, view},
};

use crate::{
    app::_hoard::server_url,
    auth::{db, require_user},
    ui::{BUTTON_DANGER, BUTTON_PRIMARY, card, field, notice},
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
