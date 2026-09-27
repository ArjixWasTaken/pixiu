//! Everything behind the login: the app shell and the Hoard dashboard (`/`).

mod covers;
mod offerings;
mod settings;

use std::collections::HashMap;

use pixiu_db::{
    Album, Artist, Offering,
    toasty::{self, stmt::Value},
};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Slot, layout, page,
        request::{headers, uri},
    },
    view::{View, class, view},
};

use crate::{
    auth::{db, require_user},
    ui::{BUTTON_GHOST, BUTTON_PRIMARY, LOGO, LOGO_SMALL, card, format_bytes},
};

const NAV: &[(&str, &str)] = &[
    ("/", "Hoard"),
    ("/offerings", "Offerings"),
    ("/settings", "Settings"),
];

#[layout]
async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let user = require_user(cx).await?;
    let current = uri(cx).path();
    let pending = Offering::all().exec(&mut db(cx)).await?.len();

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
                    for (path, label) in NAV {
                        <a
                            let active = *path == current;
                            href=(*path)
                            aria-current=(active.then_some("page"))
                            class=(class!(
                                "flex items-center justify-between rounded-lg px-3 py-2 transition",
                                "bg-gold/10 font-medium text-gold" if active,
                                "text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
                                    if !active,
                            ))
                        >
                            (*label)
                            if *path == "/offerings" && pending > 0 {
                                <span
                                    class="rounded-full bg-gold px-2 text-xs font-semibold \
                                           text-gold-foreground"
                                >
                                    (pending)
                                </span>
                            }
                        </a>
                    }
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
            <main class="min-w-0 flex-1 px-10 py-10">(slot)</main>
        </div>
    })
}

struct Stats {
    artists: u64,
    albums: u64,
    tracks: u64,
    bytes: u64,
}

async fn stats(cx: &Cx) -> Result<Stats> {
    let rows = toasty::sql::query(
        "SELECT \
            (SELECT COUNT(DISTINCT artist_id) FROM albums), \
            (SELECT COUNT(*) FROM albums), \
            (SELECT COUNT(*) FROM tracks), \
            (SELECT COALESCE(SUM(size), 0) FROM tracks)",
    )
    .exec(&mut db(cx))
    .await?;
    let number = |value: &Value| match value {
        Value::I64(n) => u64::try_from(*n).unwrap_or(0),
        _ => 0,
    };
    let Some(Value::Record(row)) = rows.first() else {
        return Ok(Stats {
            artists: 0,
            albums: 0,
            tracks: 0,
            bytes: 0,
        });
    };
    Ok(Stats {
        artists: number(&row[0]),
        albums: number(&row[1]),
        tracks: number(&row[2]),
        bytes: number(&row[3]),
    })
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let stats = stats(cx).await?;
    let mut db = db(cx);
    let recent = Album::all()
        .order_by(Album::fields().created_at().desc())
        .limit(12)
        .exec(&mut db)
        .await?;
    let artist_ids: Vec<u64> = recent.iter().map(|album| album.artist_id).collect();
    let artists: HashMap<u64, String> = if artist_ids.is_empty() {
        HashMap::new()
    } else {
        Artist::filter(Artist::fields().id().in_list(artist_ids))
            .exec(&mut db)
            .await?
            .into_iter()
            .map(|artist| (artist.id, artist.name))
            .collect()
    };
    // Clients append `/rest/...` themselves, so they want the bare origin.
    let server = server_url(cx);

    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"The Hoard"</h2>
                <p class="text-muted-foreground">"Everything píxiū has gathered."</p>
            </header>

            <dl class="grid grid-cols-2 gap-4 md:grid-cols-4">
                for (label, value) in [
                    ("Artists", stats.artists.to_string()),
                    ("Albums", stats.albums.to_string()),
                    ("Tracks", stats.tracks.to_string()),
                    ("Size", format_bytes(stats.bytes)),
                ] {
                    <div class="rounded-2xl border border-border bg-card/80 px-5 py-4">
                        <dt class="text-xs uppercase tracking-widest text-muted-foreground">
                            (label)
                        </dt>
                        <dd class="mt-1 font-display text-2xl text-foreground">(value)</dd>
                    </div>
                }
            </dl>

            if recent.is_empty() {
                card(
                    <div class="flex flex-col items-center gap-3 py-10 text-center">
                        <h3 class="text-xl">"Nothing hoarded yet"</h3>
                        <p class="max-w-md text-sm text-muted-foreground">
                            "Offer music to píxiū and it will be kept here, ready for "
                            "your Subsonic clients."
                        </p>
                        <a href="/offerings" class=(BUTTON_PRIMARY)>"Make an offering"</a>
                    </div>
                )
            } else {
                <section class="flex flex-col gap-4">
                    <h3 class="text-lg">"Recently hoarded"</h3>
                    <ul class="grid grid-cols-2 gap-5 sm:grid-cols-3 lg:grid-cols-4">
                        for album in &recent {
                            <li class="flex flex-col gap-2">
                                if album.cover.is_some() {
                                    <img
                                        src=(format!("/covers/{}?size=300", album.id))
                                        alt=""
                                        loading="lazy"
                                        class="aspect-square w-full rounded-xl border \
                                               border-border object-cover shadow-lg"
                                    >
                                } else {
                                    <div
                                        class="flex aspect-square w-full items-center \
                                               justify-center rounded-xl border border-border \
                                               bg-muted"
                                    >
                                        <img src=(LOGO) alt="" class="size-1/2 opacity-30">
                                    </div>
                                }
                                <div class="min-w-0">
                                    <p class="truncate font-medium">(&album.title)</p>
                                    <p class="truncate text-sm text-muted-foreground">
                                        (artists.get(&album.artist_id).map_or("", String::as_str))
                                        if let Some(year) = album.year {
                                            " · " (year)
                                        }
                                    </p>
                                </div>
                            </li>
                        }
                    </ul>
                </section>
            }

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
pub(crate) fn server_url(cx: &Cx) -> String {
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
