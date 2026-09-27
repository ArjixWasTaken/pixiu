//! Everything behind the login: the app shell and the Hoard dashboard (`/`).

use topcoat::{
    Result,
    context::Cx,
    router::{Slot, layout, page, request::headers},
    view::{View, view},
};

use crate::{
    auth::require_user,
    ui::{BUTTON_GHOST, LOGO_SMALL, card},
};

#[layout]
async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let user = require_user(cx).await?;

    Ok(view! {
        <div class="flex min-h-screen">
            <aside
                class="sticky top-0 flex h-screen w-60 shrink-0 flex-col gap-6 border-r \
                       border-border bg-card/60 p-4 backdrop-blur"
            >
                <a href="/" class="flex items-center gap-3 px-2 pt-2">
                    <img src=(LOGO_SMALL) alt="" width="40" height="40" class="size-10">
                    <span class="font-display text-xl font-bold tracking-widest text-gold">
                        "PÍXIŪ"
                    </span>
                </a>
                <nav class="flex flex-1 flex-col gap-1 text-sm">
                    <a
                        href="/"
                        aria-current="page"
                        class="rounded-lg bg-gold/10 px-3 py-2 font-medium text-gold"
                    >
                        "Hoard"
                    </a>
                </nav>
                <div class="flex flex-col gap-1 border-t border-border pt-4">
                    <p class="truncate px-3 text-xs text-muted-foreground">
                        "Signed in as "
                        <span class="text-foreground">(&user.username)</span>
                    </p>
                    <form method="post" action="/logout">
                        <button type="submit" class=(BUTTON_GHOST)>"Sign out"</button>
                    </form>
                </div>
            </aside>
            <main class="flex-1 px-10 py-10">(slot)</main>
        </div>
    })
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    // Clients append `/rest/...` themselves, so they want the bare origin.
    let server = server_url(cx);

    Ok(view! {
        <div class="mx-auto flex max-w-4xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"The Hoard"</h2>
                <p class="text-muted-foreground">"Everything píxiū has gathered."</p>
            </header>

            card(
                <div class="flex flex-col items-center gap-2 py-10 text-center">
                    <h3 class="text-xl">"Nothing hoarded yet"</h3>
                    <p class="max-w-md text-sm text-muted-foreground">
                        "Music you offer to píxiū, and music it hunts down, will be "
                        "kept here."
                    </p>
                </div>
            )

            card(
                <div class="flex flex-col gap-3">
                    <h3 class="text-lg">"Subsonic clients"</h3>
                    <p class="text-sm text-muted-foreground">
                        "Use this server address in OpenSubsonic clients."
                    </p>
                    <code
                        class="w-fit rounded-lg border border-border bg-input px-3 py-2 \
                               text-sm text-gold-soft"
                    >
                        (server)
                    </code>
                </div>
            )
        </div>
    })
}

/// The origin clients reached us at, honoring a reverse proxy's headers.
fn server_url(cx: &Cx) -> String {
    let headers = headers(cx);
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    let scheme = header("x-forwarded-proto").unwrap_or_else(|| "http".to_owned());
    let host = header("x-forwarded-host")
        .or_else(|| header("host"))
        .unwrap_or_else(|| "localhost".to_owned());
    format!("{scheme}://{host}")
}
