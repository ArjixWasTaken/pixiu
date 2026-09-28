//! `/watches`: playlists, liked music and artists that píxiū keeps up with.

mod watch_id;

use std::{collections::HashSet, time::Duration};

use pixiu_db::{JobState, Playlist, PlaylistEntry, SessionState, Track, Watch, WatchKind};
use pixiu_hunt::{
    image_url_at,
    link::{self, Link},
};
use pixiu_jobs::{
    queue::wanted,
    watch::{self, NewWatch, WAITING_FOR_LOGIN, WatchError},
};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        page, query_params, route,
    },
    runtime::{Event, connected, shard, signal},
    view::{View, attributes, class, component, emit, live, view},
};

use crate::{
    app::_hoard::count,
    auth::{db, jobs, require_user, treasury, warden},
    ui::{
        LABEL, Size, Tone, btn, confirm_dialog, cover, empty_state, icons, open_dialog,
        page_header, progress, relative, snackbar, switch, text_field,
    },
};

pub(super) const WATCHES_PATH: &str = "/watches";

#[query_params(error = bad_request)]
struct WatchesQuery {
    added: Option<String>,
    removed: Option<String>,
    error: Option<String>,
    /// A link to fill in, e.g. from an artist's page.
    target: Option<String>,
}

/// How far a watch is along.
enum Detail {
    /// Songs of the playlist in the hoard, of all its songs.
    Playlist {
        have: usize,
        total: usize,
    },
    Artist(String),
    NotMirrored,
}

/// A watch as the page shows it.
struct Row {
    watch: Watch,
    link: String,
    detail: Detail,
    status: Status,
    queued: usize,
    failed: usize,
}

enum Status {
    Synced(String),
    Syncing,
    SyncQueued,
    Waiting,
    Failed(String),
}

fn remote_url(watch: &Watch) -> String {
    match watch.kind {
        WatchKind::Artist => format!("https://music.youtube.com/channel/{}", watch.remote_id),
        WatchKind::Playlist | WatchKind::LikedMusic => {
            format!(
                "https://music.youtube.com/playlist?list={}",
                watch.remote_id
            )
        }
    }
}

async fn detail(cx: &Cx, watch: &Watch) -> Result<Detail> {
    let mut db = db(cx);
    Ok(match watch.kind {
        WatchKind::Playlist | WatchKind::LikedMusic => {
            let Some(mirror) = Playlist::filter_by_watch_id(Some(watch.id))
                .first()
                .exec(&mut db)
                .await?
            else {
                return Ok(Detail::NotMirrored);
            };
            let videos: Vec<String> = PlaylistEntry::filter_by_playlist_id(mirror.id)
                .exec(&mut db)
                .await?
                .into_iter()
                .filter_map(|entry| entry.ytm_video_id)
                .collect();
            let mut downloaded = HashSet::new();
            for chunk in videos.chunks(500) {
                for track in Track::filter(Track::fields().ytm_video_id().in_list(chunk.to_vec()))
                    .exec(&mut db)
                    .await?
                {
                    downloaded.extend(track.ytm_video_id);
                }
            }
            Detail::Playlist {
                have: videos
                    .iter()
                    .filter(|video| downloaded.contains(*video))
                    .count(),
                total: videos.len(),
            }
        }
        WatchKind::Artist => {
            let releases = if watch.include_singles {
                "Albums, singles and EPs"
            } else {
                "Albums only"
            };
            let from = if watch.only_new {
                "Only releases from now on"
            } else {
                "Old releases included"
            };
            Detail::Artist(format!(
                "{} · {releases} · {from}",
                count(watch.seen.len(), "release known", "releases known")
            ))
        }
    })
}

async fn rows(cx: &Cx) -> Result<Vec<Row>> {
    let mut watches = Watch::all().exec(&mut db(cx)).await?;
    watches.sort_by_key(|watch| watch.created_at);
    let logged_in = matches!(
        warden(cx).health().state,
        Some(SessionState::Valid | SessionState::Degraded)
    );
    let unfinished = jobs(cx).unfinished().await?;
    let syncing = watch::syncing(&mut db(cx)).await?;

    let mut rows = Vec::with_capacity(watches.len());
    for watch in watches {
        let waiting = watch.last_error.as_deref() == Some(WAITING_FOR_LOGIN)
            || (watch.kind == WatchKind::LikedMusic && !logged_in);
        let status = match (
            syncing.get(&watch.id),
            &watch.last_error,
            watch.last_synced_at,
        ) {
            (Some(JobState::Running), _, _) => Status::Syncing,
            _ if waiting => Status::Waiting,
            (Some(_), _, _) => Status::SyncQueued,
            (None, Some(error), _) => Status::Failed(error.clone()),
            (None, None, Some(synced)) => Status::Synced(format!("Synced {}", relative(synced))),
            (None, None, None) => Status::Synced("Not synced yet".to_owned()),
        };
        let mine = unfinished
            .iter()
            .filter(|job| wanted(job).and_then(|wanted| wanted.watch_id()) == Some(watch.id));
        let (failed, queued) = mine.fold((0, 0), |(failed, queued), job| {
            if job.state == JobState::Failed {
                (failed + 1, queued)
            } else {
                (failed, queued + 1)
            }
        });
        rows.push(Row {
            link: remote_url(&watch),
            detail: detail(cx, &watch).await?,
            status,
            queued,
            failed,
            watch,
        });
    }
    Ok(rows)
}

fn dialog_id(watch: &Watch) -> String {
    format!("remove-watch-{}", watch.id)
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<WatchesQuery>(cx)?;
    let watches = Watch::all().exec(&mut db(cx)).await?;
    let logged_in = matches!(
        warden(cx).health().state,
        Some(SessionState::Valid | SessionState::Degraded)
    );
    let liked_watched = watches
        .iter()
        .any(|watch| watch.kind == WatchKind::LikedMusic);
    let orphaned = treasury(cx).orphan_count().await?;
    let target = signal(cx, || query.target.clone().unwrap_or_default());

    Ok(view! {
        <div class="flex flex-col gap-7">
            page_header(
                eyebrow: "Watches",
                title: "Keep up with playlists and artists",
                lede: "Watches sync on their own every few hours. Watched playlists show up in \
                       Subsonic apps as read-only playlists.",
            )

            if query.added.is_some() {
                snackbar(message: "Watching. The first sync is queued.", action: ("Jobs", "/jobs"))
            }
            if query.removed.is_some() {
                if orphaned > 0 {
                    snackbar(
                        message: "No longer watched. Songs nothing else keeps are now orphans.",
                        action: ("Orphans", "/orphans"),
                    )
                } else {
                    snackbar(message: "No longer watched. Its music stays in the hoard.")
                }
            }
            if let Some(error) = &query.error {
                snackbar(message: error, error: true)
            }

            <form
                method="post"
                action="/watches/add"
                class="flex flex-col gap-3.5 rounded-3xl bg-card p-5 [--field-bg:var(--card)]"
            >
                <div class="flex flex-wrap items-start gap-3">
                    <div class="flex min-w-0 flex-[1_1_320px] flex-col gap-1">
                        text_field(
                            label: "YouTube Music link",
                            leading: icons::LINK,
                            attrs: attributes! {
                                name="target" required="" autocomplete="off"
                                :value=$(target.get())
                                @input=$(|e: Event| target.set(e.target.value))
                            },
                        )
                        link_hint(target: $(target.get()))
                    </div>
                    <button type="submit" class=(btn(Tone::Filled, Size::M))>"Watch"</button>
                </div>
            </form>

            if !liked_watched {
                <div
                    class="flex flex-wrap items-center gap-x-5 gap-y-3.5 rounded-[20px] border \
                           border-border px-5 py-[18px]"
                >
                    <div class="flex flex-[1_1_280px] flex-col gap-0.5">
                        <span class="text-base font-medium">"Watch your liked music"</span>
                        <span class="text-sm text-muted-foreground">
                            "Mirrors your YouTube Music likes as a playlist. "
                            if logged_in {
                                "New likes are downloaded."
                            } else {
                                "Needs a " <a href="/settings/sources">"YouTube Music login"</a> "."
                            }
                        </span>
                    </div>
                    <form method="post" action="/watches/liked">
                        <button type="submit" class=(btn(Tone::Tonal, Size::S))>"Watch liked music"</button>
                    </form>
                </div>
            }

            watch_list()

            for watch in &watches {
                let id = dialog_id(watch);
                let action = format!("/watches/{}/remove", watch.id);
                confirm_dialog(
                    id: &id,
                    headline: "Remove this watch?",
                    action: &action,
                    confirm: "Remove",
                    "“" (&watch.name) "” stops syncing, and its music stays in the hoard. "
                    "Songs nothing else keeps become orphans."
                )
            }
        </div>
    })
}

/// What the pasted link is, as the admin types, and the artist options
/// when it is an artist.
#[shard]
async fn link_hint(cx: &Cx, target: String) -> Result<impl View> {
    require_user(cx).await?;
    let target = target.trim();
    let parsed = (!target.is_empty()).then(|| link::parse(target)).flatten();
    let (hint, bad) = match (&parsed, target.is_empty()) {
        (_, true) => (
            "Paste a playlist or artist link from music.youtube.com",
            false,
        ),
        (Some(Link::Artist(_)), _) => ("Artist link: choose what to follow", false),
        (Some(Link::Playlist(_)), _) => ("Playlist link", false),
        (Some(Link::LikedMusic), _) => ("Your liked music", false),
        (Some(Link::Album(_) | Link::Track(_)), _) => {
            ("That’s an album or a song. Grab it on Hunt instead.", true)
        }
        (None, _) => ("That isn’t a YouTube Music playlist or artist link.", true),
    };
    let artist = matches!(parsed, Some(Link::Artist(_)));
    Ok(view! {
        <span class=(class!("px-4 text-xs leading-4", "text-destructive" if bad else "text-muted-foreground"))>
            (hint)
        </span>
        if artist {
            <div class="flex flex-wrap gap-x-7 gap-y-3 px-1 pt-3">
                switch(attrs: attributes! { name="only_new" checked="" }, "Only releases from now on")
                switch(attrs: attributes! { name="singles" }, "Singles and EPs too")
            </div>
        }
    })
}

/// The watches, live: syncs start and finish while the page is open.
#[shard]
async fn watch_list(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(live! {
        let mut updates = jobs(cx).subscribe();
        loop {
            let rows = rows(cx).await?;
            let token = emit! { watch_rows(rows: rows) }?;
            if !connected(cx) {
                break Ok(token);
            }
            if matches!(updates.recv().await, Err(RecvError::Closed)) {
                break Ok(token);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            while updates.try_recv().is_ok() {}
        }
    })
}

#[component]
async fn watch_rows(rows: Vec<Row>) -> Result<impl View> {
    Ok(view! {
        if rows.is_empty() {
            empty_state(<p class="m-0">"Nothing watched yet. Paste a playlist or artist link above."</p>)
        } else {
            <section class="flex flex-col gap-2.5">
                for row in &rows {
                    let watch = &row.watch;
                    let artist = watch.kind == WatchKind::Artist;
                    <div class="flex flex-wrap items-center gap-x-5 gap-y-4 rounded-[20px] bg-card px-[18px] py-4">
                        <div class="w-16 shrink-0">
                            if let Some(url) = &watch.image_url {
                                <img
                                    src=(image_url_at(url, 160))
                                    alt=""
                                    loading="lazy"
                                    referrerpolicy="no-referrer"
                                    class=(class!(
                                        "aspect-square w-full object-cover",
                                        "rounded-full" if artist else "rounded-xl",
                                    ))
                                >
                            } else {
                                cover(album: None, size: 120)
                            }
                        </div>
                        <div class="flex min-w-0 flex-[1_1_260px] flex-col gap-1.5">
                            <span class=(class!(LABEL, "tracking-[.1em]"))>
                                match watch.kind {
                                    WatchKind::Playlist => "Playlist",
                                    WatchKind::LikedMusic => "Liked music",
                                    WatchKind::Artist => "Artist",
                                }
                            </span>
                            <a
                                href=(&row.link)
                                target="_blank"
                                rel="noopener"
                                class="text-[17px] font-medium text-foreground hover:text-foreground hover:underline"
                            >
                                (&watch.name)
                            </a>
                            match &row.detail {
                                Detail::Playlist { have, total } => {
                                    <div class="flex max-w-[360px] items-center gap-2.5">
                                        progress(
                                            value: Some(if *total == 0 { 0.0 } else {
                                                #[allow(clippy::cast_precision_loss)]
                                                let fraction = *have as f64 / *total as f64;
                                                fraction
                                            }),
                                            attrs: attributes! { class="flex-1" },
                                        )
                                        <span class="text-[13px] whitespace-nowrap text-muted-foreground">
                                            (have) " of " (count(*total, "song", "songs")) " in the hoard"
                                        </span>
                                    </div>
                                },
                                Detail::Artist(text) => {
                                    <span class="text-[13px] text-muted-foreground">(text)</span>
                                },
                                Detail::NotMirrored => {
                                    <span class="text-[13px] text-muted-foreground">"Not mirrored yet"</span>
                                },
                            }
                            <span class="text-[13px] leading-[18px]">
                                match &row.status {
                                    Status::Synced(text) => <span class="text-muted-foreground">(text)</span>,
                                    Status::Syncing => <span class="text-gold">"Syncing…"</span>,
                                    Status::SyncQueued => <span class="text-muted-foreground">"Sync queued"</span>,
                                    Status::Waiting => {
                                        <a href="/settings/sources" class="text-slate-pale hover:text-foreground">
                                            "Waiting for a YouTube Music login"
                                        </a>
                                    },
                                    Status::Failed(error) => <span class="text-destructive">(error)</span>,
                                }
                                if row.queued > 0 {
                                    <span class="text-muted-foreground">" · " (row.queued) " queued"</span>
                                }
                                if row.failed > 0 {
                                    " · " <a href="/jobs" class="text-destructive">(row.failed) " failed"</a>
                                }
                            </span>
                        </div>
                        <div class="flex gap-2">
                            <form method="post" action=(format!("/watches/{}/sync", watch.id))>
                                <button type="submit" class=(btn(Tone::Tonal, Size::S))>"Sync now"</button>
                            </form>
                            <form method="post" action=(format!("/watches/{}/remove", watch.id))>
                                <button
                                    type="submit"
                                    onclick=(format!("{}; return false", open_dialog(&dialog_id(watch))))
                                    class=(btn(Tone::Text, Size::S))
                                >
                                    "Remove"
                                </button>
                            </form>
                        </div>
                    </div>
                }
            </section>
        }
    })
}

fn back_with_error(message: &str) -> SeeOther {
    see_other(format!(
        "{WATCHES_PATH}?error={}",
        form_urlencoded::byte_serialize(message.as_bytes()).collect::<String>()
    ))
}

#[derive(Deserialize)]
struct AddWatch {
    target: String,
    only_new: Option<String>,
    singles: Option<String>,
}

async fn start_watching(cx: &Cx, new: NewWatch) -> Result<SeeOther> {
    match watch::add(treasury(cx), jobs(cx), new).await {
        Ok(_) => Ok(see_other(format!("{WATCHES_PATH}?added=1"))),
        Err(WatchError::Duplicate) => Ok(back_with_error("píxiū already watches that.")),
        Err(WatchError::Db(error)) => Err(error.into()),
    }
}

#[route(POST "./add")]
async fn add(cx: &Cx, Form(form): Form<AddWatch>) -> Result<SeeOther> {
    require_user(cx).await?;
    let (kind, remote_id) = match link::parse(&form.target) {
        Some(Link::Playlist(id)) => (WatchKind::Playlist, id),
        Some(Link::LikedMusic) => (WatchKind::LikedMusic, String::new()),
        Some(Link::Artist(id)) => (WatchKind::Artist, id),
        Some(Link::Album(_) | Link::Track(_)) => {
            return Ok(back_with_error(
                "That is an album or a song; grab it on the Hunt page instead.",
            ));
        }
        None => {
            return Ok(back_with_error(
                "That does not look like a YouTube Music playlist or artist link.",
            ));
        }
    };
    start_watching(
        cx,
        NewWatch {
            kind,
            remote_id,
            include_singles: form.singles.is_some(),
            only_new: form.only_new.is_some(),
        },
    )
    .await
}

#[route(POST "./liked")]
async fn liked(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    start_watching(
        cx,
        NewWatch {
            kind: WatchKind::LikedMusic,
            remote_id: String::new(),
            include_singles: false,
            only_new: false,
        },
    )
    .await
}
