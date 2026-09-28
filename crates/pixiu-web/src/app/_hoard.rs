//! Everything behind the login: the app shell (nav rail or bottom bar, top
//! bar, Pulse column) and the Hoard dashboard (`/`).

mod albums;
mod artists;
mod covers;
mod hunt;
mod jobs;
mod library;
mod offerings;
mod orphans;
mod playing;
mod playlists;
mod settings;
mod watches;

use std::{collections::HashMap, time::Duration};

use pixiu_db::{
    Album, Artist, JobState, Offering,
    toasty::{self, stmt::Value},
};
use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon},
    router::{
        Slot, layout, page,
        request::{headers, uri},
    },
    runtime::{connected, shard, signal},
    view::{View, class, emit, live, view},
};

use crate::{
    auth::{db, jobs, now_playing, require_user, treasury, warden},
    ui::{
        ALBUM_GRID, EYEBROW, H2, LABEL, LOGO_SMALL, Size, TRUNCATE, Tone, album_tile, btn,
        copy_button, cover, dot, format_bytes, icons, initial, progress, session_look,
    },
};

/// An entry in the navigation.
struct NavItem {
    path: &'static str,
    label: &'static str,
    icon: IconData,
}

static NAV: [NavItem; 10] = [
    NavItem {
        path: "/",
        label: "Hoard",
        icon: icons::ARCHIVE,
    },
    NavItem {
        path: "/library",
        label: "Library",
        icon: icons::LIBRARY,
    },
    NavItem {
        path: "/hunt",
        label: "Hunt",
        icon: icons::SEARCH,
    },
    NavItem {
        path: "/watches",
        label: "Watches",
        icon: icons::WATCHES,
    },
    NavItem {
        path: "/jobs",
        label: "Jobs",
        icon: icons::JOBS,
    },
    NavItem {
        path: "/offerings",
        label: "Offerings",
        icon: icons::UPLOAD,
    },
    NavItem {
        path: "/orphans",
        label: "Orphans",
        icon: icons::ORPHANS,
    },
    NavItem {
        path: "/playing",
        label: "Playing",
        icon: icons::PLAYING,
    },
    NavItem {
        path: "/playlists",
        label: "Playlists",
        icon: icons::PLAYLISTS,
    },
    NavItem {
        path: "/settings",
        label: "Settings",
        icon: icons::SETTINGS,
    },
];

/// The entries the bottom bar shows; the rest live under "More".
const BAR: [&str; 4] = ["/", "/library", "/hunt", "/jobs"];

/// Whether the nav entry for `path` covers the page at `current`. Albums
/// and artists belong to the library.
fn is_active(path: &str, current: &str) -> bool {
    let under = |prefix: &str| {
        current
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    };
    match path {
        "/" => current == "/",
        "/library" => under("/library") || under("/albums") || under("/artists"),
        _ => under(path),
    }
}

#[layout]
async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let user = require_user(cx).await?;
    let current = uri(cx).path();
    let pending = offering_batches(cx).await?;
    let orphans = treasury(cx).orphan_count().await?;
    // Counts worth a badge, by nav entry.
    let badge = move |path: &str| match path {
        "/offerings" => pending,
        "/orphans" => orphans,
        _ => 0,
    };
    let in_more = !BAR.iter().any(|path| is_active(path, current));
    let detail = current.starts_with("/albums/") || current.starts_with("/artists/");
    let query = if current == "/hunt" {
        uri(cx)
            .query()
            .and_then(|query| {
                form_urlencoded::parse(query.as_bytes())
                    .find(|(key, _)| key == "q")
                    .map(|(_, value)| value.into_owned())
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    let more = signal(cx, || false);

    Ok(view! {
        <div class="flex h-dvh overflow-hidden bg-background">
            // The rail, on wide screens.
            <nav
                aria-label="Main"
                class="hidden w-[88px] shrink-0 flex-col items-center overflow-y-auto border-r \
                       border-muted bg-dim pb-4 md:flex"
            >
                <a
                    href="/"
                    title="Hoard"
                    class="mt-4 mb-2 grid size-14 shrink-0 place-items-center rounded-full border \
                           border-gold/35 bg-background transition hover:border-gold/70"
                >
                    <img src=(LOGO_SMALL) alt="píxiū" width="46" height="46" class="size-[46px]">
                </a>
                <div class="flex flex-col gap-2.5 py-2">
                    for item in &NAV {
                        let active = is_active(item.path, current);
                        <a
                            href=(item.path)
                            aria-current=(active.then_some("page"))
                            class=(class!(
                                "group flex w-[88px] flex-col items-center gap-1 text-xs font-medium",
                                "text-foreground hover:text-foreground" if active
                                    else "text-muted-foreground hover:text-foreground",
                            ))
                        >
                            <span
                                class=(class!(
                                    "relative grid h-8 w-14 place-items-center rounded-full transition",
                                    "bg-slate-container" if active else "group-hover:bg-foreground/8",
                                ))
                            >
                                icon(data: item.icon.clone(), size: 24)
                                if badge(item.path) > 0 {
                                    <span
                                        class="absolute -top-1 right-1.5 min-w-4 rounded-full bg-gold \
                                               px-1 text-center text-[11px] leading-4 font-medium \
                                               text-gold-foreground"
                                    >
                                        (badge(item.path))
                                    </span>
                                }
                            </span>
                            (item.label)
                        </a>
                    }
                </div>
            </nav>

            <div class="flex min-w-0 flex-1 flex-col">
                // The top bar.
                <header
                    class="flex h-16 shrink-0 items-center gap-3 border-b border-muted bg-background px-4"
                >
                    if detail {
                        <button
                            type="button"
                            aria-label="Back"
                            onclick="history.back()"
                            class="inline-grid size-10 shrink-0 place-items-center rounded-full \
                                   text-foreground hover:bg-foreground/10 md:hidden"
                        >
                            icon(data: icons::BACK, size: 24)
                        </button>
                    }
                    <a href="/" class="shrink-0 md:hidden" aria-label="Hoard">
                        <img src=(LOGO_SMALL) alt="" width="36" height="36" class="size-9">
                    </a>
                    <form
                        method="get"
                        action="/hunt"
                        role="search"
                        class="mr-auto flex h-11 max-w-[560px] min-w-0 flex-[1_1_560px] items-center \
                               gap-2.5 rounded-full bg-muted pr-1.5 pl-4 text-muted-foreground \
                               focus-within:ring-2 focus-within:ring-gold/40"
                    >
                        icon(data: icons::SEARCH, size: 22)
                        <input
                            type="search"
                            name="q"
                            value=(query)
                            placeholder="Hunt YouTube Music: artist, album or song"
                            aria-label="Hunt"
                            class="min-w-0 flex-1 border-none bg-transparent text-[15px] text-foreground \
                                   outline-none placeholder:text-muted-foreground"
                        >
                    </form>
                    session_pill()
                    <div class="hidden items-center gap-2.5 pl-1 md:flex">
                        <span
                            class="grid size-9 place-items-center rounded-full bg-slate text-sm \
                                   font-medium"
                        >
                            (initial(&user.username))
                        </span>
                        <div class="flex flex-col leading-4">
                            <span class="text-sm font-medium">(&user.username)</span>
                            <form method="post" action="/logout">
                                <button
                                    type="submit"
                                    class="text-xs text-muted-foreground hover:text-foreground"
                                >
                                    "Sign out"
                                </button>
                            </form>
                        </div>
                    </div>
                </header>

                <div class="flex min-h-0 flex-1">
                    <main
                        class="min-w-0 flex-1 overflow-y-auto \
                               bg-[radial-gradient(900px_360px_at_30%_-120px,rgb(230_182_92/.06),transparent_70%)]"
                    >
                        session_banner()
                        <div class="mx-auto max-w-[1120px] px-4 pt-5 pb-8 md:px-10 md:pt-8 md:pb-14">
                            (slot)
                        </div>
                    </main>
                    <aside
                        aria-label="Pulse"
                        class="hidden w-[300px] shrink-0 overflow-y-auto border-l border-muted bg-dim \
                               px-5 py-6 xl:block"
                    >
                        pulse()
                    </aside>
                </div>

                // The bottom bar, on narrow screens.
                <nav
                    aria-label="Main"
                    class="grid h-20 shrink-0 grid-cols-5 border-t border-muted bg-dim md:hidden"
                >
                    for item in NAV.iter().filter(|item| BAR.contains(&item.path)) {
                        let active = is_active(item.path, current);
                        bar_item(label: item.label, glyph: item.icon.clone(), active: active, href: item.path)
                    }
                    <button
                        type="button"
                        @click=$(|_e| more.set(true))
                        class=(class!(
                            "group flex flex-col items-center justify-center gap-1 text-xs font-medium",
                            "text-foreground" if in_more else "text-muted-foreground",
                        ))
                    >
                        <span
                            class=(class!(
                                "grid h-8 w-16 place-items-center rounded-full",
                                "bg-slate-container" if in_more,
                            ))
                        >
                            icon(data: icons::MORE, size: 24)
                        </span>
                        "More"
                    </button>
                </nav>
            </div>
        </div>

        // "More": the rest of the navigation, as a bottom sheet.
        <div :hidden=$(!more.get()) class="md:hidden">
            <div class="fixed inset-0 z-[90] bg-black/50" @click=$(|_e| more.set(false))></div>
            <div
                class="fixed inset-x-0 bottom-0 z-[91] rounded-t-[28px] bg-muted px-3 pt-2.5 pb-6"
            >
                <div class="mx-auto mt-1 mb-3 h-1 w-8 rounded-sm bg-outline"></div>
                <nav aria-label="More" class="flex flex-col">
                    for item in NAV.iter().filter(|item| !BAR.contains(&item.path)) {
                        let active = is_active(item.path, current);
                        <a
                            href=(item.path)
                            aria-current=(active.then_some("page"))
                            class=(class!(
                                "flex h-14 items-center gap-3 rounded-full px-4 text-sm font-medium",
                                "bg-slate-container text-foreground hover:text-foreground" if active
                                    else "text-foreground-soft hover:bg-foreground/8 hover:text-foreground",
                            ))
                        >
                            icon(data: item.icon.clone(), size: 24)
                            <span class="flex-1">
                                if item.path == "/playing" { "Now playing" } else { (item.label) }
                            </span>
                            if badge(item.path) > 0 {
                                <span class="text-xs text-gold-soft">(badge(item.path))</span>
                            }
                        </a>
                    }
                </nav>
                <div class="mt-2 flex items-center gap-3 border-t border-border px-4 pt-3">
                    <span class="grid size-8 place-items-center rounded-full bg-slate text-[13px]">
                        (initial(&user.username))
                    </span>
                    <span class="flex-1 text-sm">(&user.username)</span>
                    <form method="post" action="/logout">
                        <button type="submit" class=(btn(Tone::Text, Size::S))>"Sign out"</button>
                    </form>
                </div>
            </div>
        </div>
    })
}

/// A bottom-bar entry.
#[topcoat::view::component]
async fn bar_item(label: &str, glyph: IconData, active: bool, href: &str) -> Result<impl View> {
    Ok(view! {
        <a
            href=(href)
            aria-current=(active.then_some("page"))
            class=(class!(
                "group flex flex-col items-center justify-center gap-1 text-xs font-medium",
                "text-foreground hover:text-foreground" if active
                    else "text-muted-foreground hover:text-foreground",
            ))
        >
            <span
                class=(class!(
                    "grid h-8 w-16 place-items-center rounded-full",
                    "bg-slate-container" if active,
                ))
            >
                icon(data: glyph, size: 24)
            </span>
            (label)
        </a>
    })
}

/// The YouTube Music session in the top bar, following the warden live.
#[shard]
async fn session_pill(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(live! {
        let mut health = warden(cx).subscribe();
        loop {
            let look = session_look(health.borrow_and_update().state);
            let token = emit! {
                <a
                    href="/settings/sources"
                    title="YouTube Music session"
                    class="flex h-9 shrink-0 items-center gap-2 rounded-full border border-border px-3 \
                           text-[13px] font-medium whitespace-nowrap text-foreground \
                           hover:bg-foreground/5 hover:text-foreground md:pr-3.5"
                >
                    dot(look: look)
                    <span class="hidden md:inline">(look.short)</span>
                </a>
            }?;
            if !connected(cx) || health.changed().await.is_err() {
                break Ok(token);
            }
        }
    })
}

/// A strip on every page while the YouTube Music login is expired. It
/// appears and disappears live as the session warden changes its mind.
#[shard]
async fn session_banner(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(live! {
        let mut health = warden(cx).subscribe();
        loop {
            let expired = health.borrow_and_update().is_expired();
            let token = emit! {
                if expired {
                    <div
                        role="alert"
                        class="flex flex-wrap items-center gap-x-4 gap-y-3 border-b \
                               border-destructive/35 bg-destructive-container px-5 py-3 \
                               text-destructive-soft"
                    >
                        <span class="inline-flex text-destructive">
                            icon(data: icons::ERROR, size: 22)
                        </span>
                        <p class="m-0 flex-[1_1_260px] text-sm leading-5">
                            <strong class="font-medium text-foreground">
                                "YouTube Music session expired. "
                            </strong>
                            "Public music can still be hunted; your account's music cannot."
                        </p>
                        <form method="post" action="/settings/sources/connect">
                            <button type="submit" class=(btn(Tone::Filled, Size::S))>
                                "Log in again"
                            </button>
                        </form>
                    </div>
                }
            }?;
            if !connected(cx) || health.changed().await.is_err() {
                break Ok(token);
            }
        }
    })
}

/// The Pulse column: what plays, what runs, and what needs the admin. Live
/// on jobs, the session and now playing; counts refresh with them.
#[shard]
async fn pulse(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(live! {
        let mut job_updates = jobs(cx).subscribe();
        let mut playing_updates = now_playing(cx).subscribe();
        loop {
            let token = emit! { pulse_body() }?;
            if !connected(cx) {
                break Ok(token);
            }
            let expiry = now_playing(cx)
                .next_expiry()
                .map_or(Duration::from_secs(60), |at| {
                    at.saturating_duration_since(std::time::Instant::now())
                })
                .min(Duration::from_secs(60));
            tokio::select! {
                update = job_updates.recv() => {
                    if matches!(update, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                        break Ok(token);
                    }
                    // Downloads report often; a moment's pause batches them.
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    while job_updates.try_recv().is_ok() {}
                }
                changed = playing_updates.changed() => {
                    if changed.is_err() {
                        break Ok(token);
                    }
                }
                () = tokio::time::sleep(expiry) => {}
            }
        }
    })
}

#[topcoat::view::component]
async fn pulse_body(cx: &Cx) -> Result<impl View> {
    let queue = jobs(cx);
    let unfinished = queue.unfinished().await?;
    let failed = unfinished
        .iter()
        .filter(|job| job.state == JobState::Failed)
        .count();
    let mut active: Vec<_> = unfinished
        .into_iter()
        .filter(|job| job.state != JobState::Failed)
        .collect();
    active.sort_by_key(|job| (job.state != JobState::Running, job.id));
    active.truncate(4);
    let offered = offering_batches(cx).await?;
    let orphaned = treasury(cx).orphan_count().await?;
    let playing = playing::now(cx).await?;

    Ok(view! {
        <div class="flex flex-col gap-6">
            <div class="flex items-center gap-2.5 text-gold">
                <span class="size-2 rounded-full bg-gold animate-pxpulse"></span>
                <span class=(EYEBROW)>"Pulse"</span>
            </div>
            if let Some(entry) = playing.first() {
                <a href="/playing" class="flex items-center gap-3 text-foreground hover:text-foreground">
                    <span class="w-[52px] shrink-0">cover(album: entry.cover, size: 120)</span>
                    <span class="flex min-w-0 flex-col gap-0.5">
                        <span class="text-xs font-medium tracking-[.04em] text-gold">
                            "Playing · " (&entry.player)
                        </span>
                        <span class=(class!(TRUNCATE, "text-sm"))>(&entry.title)</span>
                        <span class=(class!(TRUNCATE, "text-xs text-muted-foreground"))>
                            (&entry.artist)
                        </span>
                    </span>
                </a>
            }
            <div class="flex flex-col gap-3">
                <div class="flex items-baseline justify-between">
                    <span class="text-sm font-medium">"Working on"</span>
                    <a href="/jobs" class="text-[13px]">"Jobs"</a>
                </div>
                if active.is_empty() {
                    <span class="text-[13px] text-muted-foreground">"Nothing right now."</span>
                }
                for job in &active {
                    let running = job.state == JobState::Running;
                    let percent = queue.progress(job.id).unwrap_or(job.progress);
                    <div class="flex flex-col gap-1.5">
                        <div class="flex justify-between gap-2 text-[13px]">
                            <span class=(TRUNCATE)>(&job.title)</span>
                            if running {
                                <span class="text-gold-soft tabular-nums">(percent) "%"</span>
                            }
                        </div>
                        if running {
                            progress(value: Some(f64::from(percent) / 100.0))
                        } else {
                            <span class="text-xs text-muted-foreground">
                                (jobs::state_label(job.state))
                            </span>
                        }
                    </div>
                }
            </div>
            <div class="flex flex-col gap-2 text-[13px]">
                if failed > 0 {
                    <a
                        href="/jobs"
                        class="rounded-[14px] bg-destructive-container px-3.5 py-2.5 \
                               text-destructive-soft hover:text-destructive-soft"
                    >
                        (count(failed, "job failed", "jobs failed"))
                    </a>
                }
                if offered > 0 {
                    <a
                        href="/offerings"
                        class="rounded-[14px] bg-hover px-3.5 py-2.5 text-foreground \
                               hover:text-foreground"
                    >
                        (count(offered, "offering waits for review", "offerings wait for review"))
                    </a>
                }
                if orphaned > 0 {
                    <a
                        href="/orphans"
                        class="rounded-[14px] bg-hover px-3.5 py-2.5 text-foreground \
                               hover:text-foreground"
                    >
                        (count(orphaned, "orphan waits", "orphans wait"))
                    </a>
                }
            </div>
        </div>
    })
}

/// How many uploads wait for review: offerings are reviewed by batch.
async fn offering_batches(cx: &Cx) -> Result<usize> {
    let batches: std::collections::HashSet<String> = Offering::all()
        .exec(&mut db(cx))
        .await?
        .into_iter()
        .map(|offering| offering.batch)
        .collect();
    Ok(batches.len())
}

/// "1 thing", "3 things".
pub(crate) fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
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

/// A count with thousands separators: `4,961`.
pub(crate) fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let stats = stats(cx).await?;
    let mut db = db(cx);
    let recent = Album::all()
        .order_by(Album::fields().created_at().desc())
        .limit(8)
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
    let server = server_url(cx);
    let health = warden(cx).health();
    let look = session_look(health.state);

    Ok(view! {
        <div class="flex flex-col gap-9">
            <section class="flex flex-col gap-[18px]">
                <div class="flex items-center gap-3 text-gold">
                    <span class=(EYEBROW)>"The hoard"</span>
                    <span class="h-px flex-1 bg-linear-to-r from-gold/50 to-transparent"></span>
                </div>
                <dl
                    class="m-0 grid grid-cols-[repeat(auto-fit,minmax(140px,1fr))] border-y \
                           border-border"
                >
                    for (value, label) in [
                        (grouped(stats.artists), "artists"),
                        (grouped(stats.albums), "albums"),
                        (grouped(stats.tracks), "tracks"),
                        (format_bytes(stats.bytes), "on disk"),
                    ] {
                        <div class="flex flex-col-reverse gap-0.5 border-r border-muted py-[18px] pr-5">
                            <dt class="text-[13px] tracking-[.02em] text-muted-foreground">(label)</dt>
                            <dd
                                class="m-0 text-[clamp(32px,4.4vw,52px)] leading-[1.05] font-light \
                                       tracking-[-.02em] text-gold-soft tabular-nums"
                            >
                                (value)
                            </dd>
                        </div>
                    }
                </dl>
            </section>

            if recent.is_empty() {
                <section
                    class="flex flex-wrap items-center gap-7 rounded-[28px] border border-border \
                           bg-card p-8"
                >
                    <div
                        class="grid size-[140px] shrink-0 place-items-center rounded-full border \
                               border-gold/40 bg-dim"
                    >
                        <img src=(LOGO_SMALL) alt="" width="104" height="104" class="size-[104px] opacity-90">
                    </div>
                    <div class="flex flex-[1_1_280px] flex-col gap-3">
                        <h2 class="m-0 text-[28px] font-normal">"Nothing hoarded yet"</h2>
                        <p class="m-0 max-w-[52ch] text-[15px] leading-[22px] text-pretty text-muted-foreground">
                            "The beast is hungry. Hunt music on YouTube Music, or make an offering "
                            "of files you already own. Whatever it takes in, it keeps, ready for "
                            "your Subsonic apps."
                        </p>
                        <div class="mt-1 flex flex-wrap gap-2">
                            <a href="/hunt" class=(btn(Tone::Filled, Size::S))>"Hunt"</a>
                            <a href="/offerings" class=(btn(Tone::Outlined, Size::S))>
                                "Make an offering"
                            </a>
                        </div>
                    </div>
                </section>
            } else {
                <section class="flex flex-col gap-4">
                    <div class="flex items-baseline justify-between gap-3">
                        <h2 class=(H2)>"Recently hoarded"</h2>
                        <a href="/library" class="text-sm font-medium">"All albums"</a>
                    </div>
                    <div class=(ALBUM_GRID)>
                        for album in &recent {
                            let artist = artists.get(&album.artist_id).map_or("", String::as_str);
                            let href = format!("/albums/{}", album.id);
                            album_tile(
                                href: &href,
                                cover_of: album.cover.as_ref().map(|_| album.id),
                                title: &album.title,
                                subtitle: format!("{artist} · {}", crate::ui::relative(album.created_at)),
                            )
                        }
                    </div>
                </section>
            }

            <section class="grid grid-cols-[repeat(auto-fit,minmax(min(100%,320px),1fr))] gap-4">
                <div class="flex flex-col gap-3.5 rounded-3xl bg-card p-6">
                    <div class="flex items-center justify-between gap-3">
                        <span class=(LABEL)>"YouTube Music"</span>
                        dot(look: look)
                    </div>
                    <div class="text-2xl">(look.long)</div>
                    <p class="m-0 text-sm leading-5 text-muted-foreground">
                        match health.state {
                            None => "Log in to hunt your own playlists and liked music.",
                            Some(pixiu_db::SessionState::Expired) => {
                                "Public music can still be hunted. Watches on your own playlists \
                                 and likes are waiting."
                            },
                            Some(_) => "",
                        }
                        if let (Some(pixiu_db::SessionState::Valid | pixiu_db::SessionState::Degraded), Some(checked)) =
                            (health.state, health.last_verified)
                        {
                            "Checked " (crate::ui::relative(checked)) ". Your playlists and likes can be hunted."
                        }
                    </p>
                    <div class="mt-auto flex flex-wrap gap-2">
                        <a href="/settings/sources" class=(btn(Tone::Tonal, Size::S))>"Manage"</a>
                    </div>
                </div>
                <div class="flex flex-col gap-3.5 rounded-3xl bg-card p-6">
                    <span class=(LABEL)>"Subsonic apps"</span>
                    <p class="m-0 text-sm leading-5 text-muted-foreground">
                        "Paste this server address into Feishin, Symfonium or any OpenSubsonic app. "
                        "Sign in with your píxiū username and password."
                    </p>
                    <div
                        class="flex items-center gap-2 rounded-[14px] border border-border bg-dim py-1.5 \
                               pr-1.5 pl-4"
                    >
                        <code
                            class=(class!(TRUNCATE, "flex-1 font-mono text-[15px] font-medium text-gold-soft"))
                        >
                            (&server)
                        </code>
                        copy_button(text: server.clone())
                    </div>
                </div>
            </section>
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_entries_cover_their_pages() {
        assert!(is_active("/", "/"));
        assert!(!is_active("/", "/library"));
        assert!(is_active("/library", "/albums/3"));
        assert!(is_active("/library", "/artists/3"));
        assert!(is_active("/settings", "/settings/sources/login"));
        assert!(!is_active("/playing", "/playlists"));
    }

    #[test]
    fn counts_are_grouped() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(4961), "4,961");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }
}
