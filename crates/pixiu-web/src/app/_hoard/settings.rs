//! `/settings`: how to connect Subsonic clients, API keys, and where files
//! are filed.

mod api_keys;
mod sources;

use pixiu_db::{Album, ApiKey, JobKind};
use pixiu_jobs::NewJob;
use pixiu_treasury::{Template, layout::TrackLocation};
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        page, route,
    },
    runtime::{Event, shard, signal},
    view::{View, attributes, class, component, view},
};

use crate::{
    app::_hoard::{count, server_url},
    auth::{db, jobs, require_user, treasury, warden},
    ui::{
        CARD_TITLE, LABEL, LINK_CLASS, Size, Tone, btn, copy_button, dot, page_header, progress,
        relative, session_look, snackbar, surface, text_field,
    },
};

pub(super) const SETTINGS_PATH: &str = "/settings";

#[page]
async fn page() -> Result<impl View> {
    Ok(view! { settings() })
}

/// The settings page, optionally showing a just-created API key, or the
/// outcome of changing the file layout.
#[component]
pub(super) async fn settings(
    cx: &Cx,
    #[default] created_name: Option<&str>,
    #[default] created_key: Option<&str>,
    #[default] layout_draft: Option<&str>,
    #[default] layout_error: Option<&str>,
    #[default] layout_saved: bool,
) -> Result<impl View> {
    let user = require_user(cx).await?;
    let mut keys = ApiKey::filter_by_user_id(user.id).exec(&mut db(cx)).await?;
    keys.sort_by_key(|key| key.created_at);
    let token_ready = user.subsonic_secret.is_some();
    let server = server_url(cx);
    let look = session_look(warden(cx).health().state);
    let unlooked = Album::all()
        .exec(&mut db(cx))
        .await?
        .iter()
        .filter(|album| album.enrichment.is_none())
        .count();
    let layout = treasury(cx).layout();
    let current = layout.as_str().to_owned();
    let is_default = current == Template::DEFAULT;
    let misplaced = treasury(cx).misplaced().await?;
    let refiling = jobs(cx)
        .unfinished()
        .await?
        .iter()
        .any(|job| job.kind == JobKind::Refile);
    let draft = signal(cx, || layout_draft.unwrap_or(&current).to_owned());
    let saved_template = current.clone();

    Ok(view! {
        <div class="flex flex-col gap-5">
            page_header(eyebrow: "Settings", title: "How the hoard is kept")

            if layout_saved {
                snackbar(message: "Saved. New tracks follow the new layout.")
            }

            surface(
                <h2 class=(CARD_TITLE)>"Subsonic apps"</h2>
                <dl
                    class="m-0 grid grid-cols-[minmax(120px,auto)_minmax(0,1fr)] items-baseline gap-x-5 \
                           gap-y-3 text-sm"
                >
                    <dt class="text-muted-foreground">"Server address"</dt>
                    <dd class="m-0 flex flex-wrap items-center gap-2">
                        <code class="font-mono text-sm font-medium text-gold-soft">(&server)</code>
                        copy_button(text: server.clone(), class: LINK_CLASS)
                    </dd>
                    <dt class="text-muted-foreground">"Username"</dt>
                    <dd class="m-0">(&user.username)</dd>
                    <dt class="text-muted-foreground">"Password"</dt>
                    <dd class="m-0 leading-5 text-foreground-soft">
                        "Your píxiū password. Apps that support it can use an API key instead (below)."
                    </dd>
                    <dt class="text-muted-foreground">"Token auth"</dt>
                    <dd class="m-0 flex items-center gap-2">
                        if token_ready {
                            <span class="size-2 rounded-full bg-success"></span>
                            "Works: apps may send the password hashed"
                        } else {
                            <span class="size-2 rounded-full bg-gold"></span>
                            "Starts after your next sign-in"
                        }
                    </dd>
                </dl>
            )

            <div class="grid grid-cols-[repeat(auto-fit,minmax(min(100%,300px),1fr))] gap-5">
                surface(
                    attrs: attributes! { class="gap-3" },
                    <h2 class=(CARD_TITLE)>"Sources"</h2>
                    <div class="flex items-center gap-2.5 text-[15px]">
                        dot(look: look)
                        "YouTube Music · " (look.long)
                    </div>
                    <div class="mt-auto">
                        <a href="/settings/sources" class=(btn(Tone::Tonal, Size::S))>"Manage"</a>
                    </div>
                )
                surface(
                    attrs: attributes! { class="gap-3" },
                    <h2 class=(CARD_TITLE)>"MusicBrainz"</h2>
                    if unlooked == 0 {
                        <span class="text-[15px] text-foreground-soft">"Every album has been looked up."</span>
                    } else {
                        <span class="text-[15px] text-foreground-soft">
                            (count(unlooked, "album has", "albums have")) " not been looked up yet."
                        </span>
                        <form method="post" action="/settings/lookup" class="mt-auto">
                            <button type="submit" class=(btn(Tone::Tonal, Size::S))>"Look them up"</button>
                        </form>
                    }
                )
            </div>

            surface(
                attrs: attributes! { id="layout" },
                <h2 class=(CARD_TITLE)>"File layout"</h2>
                <p class="m-0 text-sm leading-5 text-muted-foreground">
                    "Where each track is filed inside the treasure. The file extension is added."
                </p>
                <form id="layout-form" method="post" action="/settings/layout" class="flex flex-col gap-4">
                    text_field(
                        label: "Layout template",
                        mono: true,
                        invalid: layout_error.is_some(),
                        supporting: layout_error,
                        attrs: attributes! {
                            name="template" required="" spellcheck="false" autocomplete="off"
                            :value=$(draft.get())
                            @input=$(|e: Event| draft.set(e.target.value))
                        },
                    )
                    layout_preview(draft: $(draft.get()))
                    <details class="text-sm text-foreground-soft">
                        <summary class="font-medium text-gold">"Template syntax"</summary>
                        <div class="flex flex-col gap-2 pt-3 leading-5">
                            <div class="flex flex-wrap gap-1.5">
                                for field in Template::FIELDS {
                                    <code class="rounded-md bg-highest px-2 py-0.5 font-mono text-[13px] text-foreground">
                                        "{" (field) "}"
                                    </code>
                                }
                            </div>
                            <span><code class="font-mono text-gold-soft">"{track:02}"</code>" pads numbers to two digits."</span>
                            <span><code class="font-mono text-gold-soft">"[parts]"</code>" are left out when a value inside is missing."</span>
                            <span>
                                <code class="font-mono text-gold-soft">"\\["</code>" is a plain bracket. "
                                <code class="font-mono text-gold-soft">"/"</code>" makes a folder."
                            </span>
                        </div>
                    </details>
                    <div class="flex flex-wrap gap-2">
                        <button
                            type="submit"
                            :disabled=$(draft.get().trim() == saved_template)
                            class=(btn(Tone::Filled, Size::S))
                        >
                            "Save"
                        </button>
                        <button
                            type="submit"
                            name="reset"
                            value="1"
                            formnovalidate=""
                            disabled=(is_default)
                            class=(btn(Tone::Text, Size::S))
                        >
                            "Back to the default"
                        </button>
                    </div>
                </form>
                if refiling {
                    <div class="flex flex-col gap-2 rounded-[14px] bg-dim px-4 py-3.5">
                        <span class="text-sm text-foreground-soft">
                            "Moving files… " <a href="/jobs">"Jobs"</a> " follows it."
                        </span>
                        progress(value: None)
                    </div>
                } else if misplaced > 0 {
                    <form
                        method="post"
                        action="/settings/refile"
                        class="flex flex-wrap items-center gap-x-4 gap-y-3 rounded-[14px] bg-gold-container px-4 py-3.5"
                    >
                        <span class="flex-[1_1_240px] text-sm text-gold-soft">
                            (count(misplaced, "track is", "tracks are"))
                            if misplaced == 1 { " not where the layout wants it." } else { " not where the layout wants them." }
                        </span>
                        <button type="submit" class=(btn(Tone::Filled, Size::S))>"Move them"</button>
                    </form>
                }
            )

            surface(
                <h2 class=(CARD_TITLE)>"API keys"</h2>
                <p class="m-0 text-sm leading-5 text-muted-foreground">
                    "Give each Subsonic app its own key, so you can revoke one without touching the others."
                </p>
                if let (Some(name), Some(key)) = (created_name, created_key) {
                    <div class="flex flex-col gap-2.5 rounded-2xl border border-gold bg-gold-container p-4">
                        <span class="text-sm text-gold-soft">
                            "Here is the key for “" (name) "”. It is shown only this once."
                        </span>
                        <div class="flex flex-wrap items-center gap-2">
                            <code
                                class="min-w-0 flex-[1_1_260px] rounded-[10px] bg-dim px-3 py-2.5 font-mono \
                                       text-sm font-medium break-all text-foreground select-all"
                            >
                                (key)
                            </code>
                            copy_button(text: key, class: btn(Tone::Filled, Size::S))
                            <a href=(SETTINGS_PATH) class=(btn(Tone::Text, Size::S))>"Done"</a>
                        </div>
                    </div>
                }
                <div class="flex flex-col">
                    for key in &keys {
                        <div class="flex min-h-[60px] flex-wrap items-center gap-x-4 gap-y-1 border-b border-muted py-1.5">
                            <div class="flex min-w-0 flex-[1_1_220px] flex-col gap-0.5">
                                <span class="text-[15px]">(&key.name)</span>
                                <span class="text-[13px] text-muted-foreground">
                                    "Created " (relative(key.created_at)) " · "
                                    match key.last_used_at {
                                        Some(used) => { "last used " (relative(used)) },
                                        None => "never used",
                                    }
                                </span>
                            </div>
                            <form method="post" action=(format!("/settings/api-keys/{}/revoke", key.id))>
                                <button type="submit" class=(btn(Tone::Text, Size::S))>"Revoke"</button>
                            </form>
                        </div>
                    }
                    if keys.is_empty() {
                        <p class="m-0 text-sm text-muted-foreground">"No keys yet. Name one after the app it’s for."</p>
                    }
                </div>
                <form method="post" action="/settings/api-keys" class="flex flex-wrap items-center gap-3">
                    text_field(
                        label: "Key name, e.g. Symfonium on phone",
                        attrs: attributes! {
                            wrapper-class="flex-[1_1_260px]" name="name" required="" maxlength="100"
                        },
                    )
                    <button type="submit" class=(btn(Tone::Tonal, Size::M))>"Create key"</button>
                </form>
            )
        </div>
    })
}

/// Where the draft template would file an example track, as the admin
/// types.
#[shard]
async fn layout_preview(cx: &Cx, draft: String) -> Result<impl View> {
    require_user(cx).await?;
    let preview = Template::preview(&draft);
    let example = TrackLocation::EXAMPLE;
    Ok(view! {
        <div class="flex flex-col gap-1.5 rounded-[14px] border border-border bg-dim px-4 py-3.5">
            <span class=(class!(LABEL, "tracking-[.1em]"))>
                "Example · " (example.title) ", track " (example.track.unwrap_or(1)) " of " (example.album)
            </span>
            match preview {
                Ok(path) => {
                    <code class="font-mono text-sm leading-5 break-all text-gold-soft">
                        (path.display().to_string())
                    </code>
                },
                Err(error) => {
                    <span class="text-sm text-destructive">(error.to_string())</span>
                },
            }
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

#[derive(Deserialize)]
struct LayoutForm {
    template: String,
    reset: Option<String>,
}

/// Changes the file layout. New tracks follow it at once; the others move
/// when the admin asks (`./refile`).
#[page(POST "./layout")]
async fn save_layout(cx: &Cx, Form(form): Form<LayoutForm>) -> Result<impl View> {
    require_user(cx).await?;
    let parsed = if form.reset.is_some() {
        Ok(Template::default())
    } else {
        Template::parse(&form.template)
    };
    let error = match parsed {
        Ok(template) => {
            tracing::info!(%template, "file layout changed");
            treasury(cx).set_layout(template).await?;
            None
        }
        Err(error) => Some(error.to_string()),
    };
    Ok(view! {
        settings(
            layout_draft: error.is_some().then_some(form.template.as_str()),
            layout_error: error.as_deref(),
            layout_saved: error.is_none(),
        )
    })
}

/// Queues moving every file to the layout, unless that is under way.
#[route(POST "./refile")]
async fn refile(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    let running = jobs(cx)
        .unfinished()
        .await?
        .iter()
        .any(|job| job.kind == JobKind::Refile);
    if !running {
        jobs(cx).enqueue(NewJob::refile()).await?;
    }
    Ok(see_other("/jobs"))
}
