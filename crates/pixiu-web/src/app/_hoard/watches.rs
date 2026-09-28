//! `/watches`: playlists, liked music and artists that píxiū keeps up with.

mod watch_id;

use std::collections::HashSet;

use pixiu_db::{JobState, Playlist, PlaylistEntry, SessionState, Track, Watch, WatchKind};
use pixiu_hunt::link::{self, Link};
use pixiu_jobs::{
    queue::wanted,
    watch::{self, NewWatch, WAITING_FOR_LOGIN, WatchError},
};
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        page, query_params, route,
    },
    view::{View, attributes, class, view},
};

use crate::{
    auth::{db, jobs, require_user, treasury, warden},
    ui::{BUTTON_DANGER, BUTTON_PRIMARY, BUTTON_SECONDARY, alert, card, field, notice, relative},
};

pub(super) const WATCHES_PATH: &str = "/watches";

#[query_params(error = bad_request)]
struct WatchesQuery {
    added: Option<String>,
    removed: Option<String>,
    error: Option<String>,
}

/// A watch as the page shows it.
struct Row {
    watch: Watch,
    link: String,
    kind: &'static str,
    detail: String,
    status: String,
    status_class: &'static str,
    queued: usize,
    failed: usize,
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

async fn detail(cx: &Cx, watch: &Watch) -> Result<String> {
    let mut db = db(cx);
    Ok(match watch.kind {
        WatchKind::Playlist | WatchKind::LikedMusic => {
            let Some(mirror) = Playlist::filter_by_watch_id(Some(watch.id))
                .first()
                .exec(&mut db)
                .await?
            else {
                return Ok("Not mirrored yet".to_owned());
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
            let songs = if videos.len() == 1 { "song" } else { "songs" };
            format!(
                "{} of {} {songs} in the hoard",
                videos
                    .iter()
                    .filter(|video| downloaded.contains(*video))
                    .count(),
                videos.len()
            )
        }
        WatchKind::Artist => {
            let releases = if watch.include_singles {
                "albums, EPs and singles"
            } else {
                "albums"
            };
            let from = if watch.only_new {
                "released from now on"
            } else {
                "old and new"
            };
            format!(
                "{} releases known · grabs {releases}, {from}",
                watch.seen.len()
            )
        }
    })
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<WatchesQuery>(cx)?;
    let mut watches = Watch::all().exec(&mut db(cx)).await?;
    watches.sort_by_key(|watch| watch.created_at);
    let logged_in = matches!(
        warden(cx).health().state,
        Some(SessionState::Valid | SessionState::Degraded)
    );
    let unfinished = jobs(cx).unfinished().await?;
    let liked_watched = watches
        .iter()
        .any(|watch| watch.kind == WatchKind::LikedMusic);

    let mut rows = Vec::with_capacity(watches.len());
    for watch in watches {
        let waiting = watch.last_error.as_deref() == Some(WAITING_FOR_LOGIN)
            || (watch.kind == WatchKind::LikedMusic && !logged_in);
        let (status, status_class) = match (&watch.last_error, watch.last_synced_at) {
            _ if waiting => (WAITING_FOR_LOGIN.to_owned(), "text-gold"),
            (Some(error), _) => (error.clone(), "text-destructive"),
            (None, Some(synced)) => (
                format!("Synced {}", relative(synced)),
                "text-muted-foreground",
            ),
            (None, None) => ("Not synced yet".to_owned(), "text-muted-foreground"),
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
            // Liked music says what it is by its name.
            kind: match watch.kind {
                WatchKind::Playlist => "Playlist",
                WatchKind::LikedMusic => "",
                WatchKind::Artist => "Artist",
            },
            detail: detail(cx, &watch).await?,
            status,
            status_class,
            queued,
            failed,
            watch,
        });
    }

    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Watches"</h2>
                <p class="text-muted-foreground">
                    "Playlists and artists píxiū keeps up with. Watched playlists show up "
                    "as playlists in your Subsonic clients; what leaves them is kept, as an "
                    "orphan."
                </p>
            </header>

            if query.added.is_some() {
                notice("Watching. The first sync is on the " <a href="/jobs" class="underline">"Jobs"</a> " page.")
            }
            if query.removed.is_some() {
                notice("No longer watched. Its tracks stay; those nothing else wants are now " <a href="/orphans" class="underline">"orphans"</a> ".")
            }
            if let Some(error) = &query.error {
                alert((error))
            }

            card(
                <form method="post" action="/watches/add" class="flex flex-col gap-4">
                    <h3 class="text-lg">"Watch a playlist or an artist"</h3>
                    <div class="flex items-end gap-3">
                        <div class="flex-1">
                            field(
                                label: "YouTube Music link",
                                attrs: attributes! {
                                    name="target" required=""
                                    placeholder="https://music.youtube.com/playlist?list=…"
                                },
                            )
                        </div>
                        <button type="submit" class=(BUTTON_PRIMARY)>"Watch"</button>
                    </div>
                    <fieldset class="flex flex-wrap gap-x-6 gap-y-2 text-sm text-muted-foreground">
                        <legend class="mb-1 text-xs uppercase tracking-widest">"For artists"</legend>
                        <label class="flex items-center gap-2">
                            <input type="checkbox" name="only_new" checked="" class="accent-gold">
                            "Only releases from now on"
                        </label>
                        <label class="flex items-center gap-2">
                            <input type="checkbox" name="singles" class="accent-gold">
                            "Singles and EPs too"
                        </label>
                    </fieldset>
                </form>
            )

            if !liked_watched {
                card(
                    <div class="flex items-center justify-between gap-4">
                        <div class="flex flex-col gap-1">
                            <h3 class="text-lg">"Your liked music"</h3>
                            <p class="text-sm text-muted-foreground">
                                "Mirror the songs you like on YouTube Music as a playlist; new "
                                "likes are downloaded."
                                if !logged_in {
                                    " Needs a " <a href="/settings/sources" class="underline">"YouTube Music login"</a> "."
                                }
                            </p>
                        </div>
                        <form method="post" action="/watches/liked" class="shrink-0">
                            <button type="submit" class=(BUTTON_SECONDARY)>"Watch liked music"</button>
                        </form>
                    </div>
                )
            }

            if rows.is_empty() {
                <p class="text-center text-sm text-muted-foreground">"Nothing watched yet."</p>
            } else {
                card(
                    <ul class="divide-y divide-border">
                        for row in &rows {
                            <li class="flex items-start justify-between gap-4 py-4 first:pt-0 last:pb-0">
                                <div class="min-w-0 flex-1">
                                    <p class="flex items-baseline gap-2">
                                        <a href=(&row.link) target="_blank" rel="noreferrer" class="truncate font-medium hover:underline">
                                            (&row.watch.name)
                                        </a>
                                        <span class="shrink-0 text-xs uppercase tracking-widest text-muted-foreground">(row.kind)</span>
                                    </p>
                                    <p class="text-sm text-muted-foreground">(&row.detail)</p>
                                    <p class=(class!("text-sm", row.status_class))>
                                        (&row.status)
                                        if row.queued > 0 {
                                            " · " (row.queued) " queued"
                                        }
                                        if row.failed > 0 {
                                            " · " <a href="/jobs" class="text-destructive underline">(row.failed) " failed"</a>
                                        }
                                    </p>
                                </div>
                                <div class="flex shrink-0 items-center gap-2">
                                    <form method="post" action=(format!("/watches/{}/sync", row.watch.id))>
                                        <button type="submit" class=(BUTTON_SECONDARY)>"Sync now"</button>
                                    </form>
                                    <form
                                        method="post"
                                        action=(format!("/watches/{}/remove", row.watch.id))
                                        onsubmit="return confirm('Stop watching? Its tracks stay; those nothing else wants become orphans.')"
                                    >
                                        <button type="submit" class=(BUTTON_DANGER)>"Remove"</button>
                                    </form>
                                </div>
                            </li>
                        }
                    </ul>
                )
            }
        </div>
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
