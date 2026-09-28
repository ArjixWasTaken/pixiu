//! `/playlists`: playlists made in Subsonic apps, and the mirrors of watched
//! ones. A song can be excluded from a mirror: its watch then lets go of it
//! and never fetches it again.

use std::collections::HashMap;

use pixiu_db::{Album, Playlist, PlaylistEntry, Track, Watch, WatchExclusion};
use pixiu_jobs::watch;
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{page, query_params},
    view::{View, class, view},
};

use crate::{
    app::_hoard::count,
    auth::{db, require_user},
    ui::{
        ICON_BUTTON, TRUNCATE, cover, empty_state, format_duration, icons, page_header, pill,
        snackbar,
    },
};

#[query_params(error = bad_request)]
struct PlaylistsQuery {
    /// The watch a song was just excluded from.
    excluded: Option<u64>,
    /// The watch a song was just taken back into.
    included: Option<u64>,
}

/// A song in a playlist: in the hoard, or (in a mirror) not yet.
enum Song {
    Hoarded {
        track: Box<Track>,
        art: Option<u64>,
    },
    Coming {
        video_id: String,
        title: Option<String>,
        artist: Option<String>,
    },
}

struct Shown {
    playlist: Playlist,
    /// The watch it mirrors, by name.
    mirrors: Option<String>,
    songs: Vec<Song>,
    /// Songs excluded from the watch it mirrors.
    excluded: Vec<WatchExclusion>,
}

impl Shown {
    /// The first cover among its songs.
    fn cover(&self) -> Option<u64> {
        self.songs.iter().find_map(|song| match song {
            Song::Hoarded { art, .. } => *art,
            Song::Coming { .. } => None,
        })
    }

    fn duration_ms(&self) -> u64 {
        self.songs
            .iter()
            .map(|song| match song {
                Song::Hoarded { track, .. } => track.duration_ms,
                Song::Coming { .. } => 0,
            })
            .sum()
    }
}

async fn playlists(cx: &Cx) -> Result<Vec<Shown>> {
    let mut db = db(cx);
    let mut playlists = Playlist::all().exec(&mut db).await?;
    playlists.sort_by_key(|playlist| std::cmp::Reverse(playlist.changed_at));
    let watches: HashMap<u64, String> = Watch::all()
        .exec(&mut db)
        .await?
        .into_iter()
        .map(|watch| (watch.id, watch.name))
        .collect();
    let mut shown = Vec::with_capacity(playlists.len());
    for playlist in playlists {
        let mut entries = PlaylistEntry::filter_by_playlist_id(playlist.id)
            .exec(&mut db)
            .await?;
        entries.sort_by_key(|entry| entry.position);

        // Own playlists list tracks; mirrors list videos, hoarded or not.
        let track_ids: Vec<u64> = entries.iter().filter_map(|entry| entry.track_id).collect();
        let video_ids: Vec<String> = entries
            .iter()
            .filter_map(|entry| entry.ytm_video_id.clone())
            .collect();
        let mut by_id: HashMap<u64, Track> = HashMap::new();
        let mut by_video: HashMap<String, u64> = HashMap::new();
        for chunk in track_ids.chunks(500) {
            for track in Track::filter(Track::fields().id().in_list(chunk.to_vec()))
                .exec(&mut db)
                .await?
            {
                by_id.insert(track.id, track);
            }
        }
        for chunk in video_ids.chunks(500) {
            for track in Track::filter(Track::fields().ytm_video_id().in_list(chunk.to_vec()))
                .exec(&mut db)
                .await?
            {
                if let Some(video) = &track.ytm_video_id {
                    by_video.insert(video.clone(), track.id);
                }
                by_id.insert(track.id, track);
            }
        }
        let album_ids: Vec<u64> = by_id.values().map(|track| track.album_id).collect();
        let covers: HashMap<u64, bool> = if album_ids.is_empty() {
            HashMap::new()
        } else {
            Album::filter(Album::fields().id().in_list(album_ids))
                .exec(&mut db)
                .await?
                .into_iter()
                .map(|album| (album.id, album.cover.is_some()))
                .collect()
        };

        let songs = entries
            .into_iter()
            .filter_map(|entry| {
                let hoarded = entry
                    .track_id
                    .or_else(|| {
                        entry
                            .ytm_video_id
                            .as_ref()
                            .and_then(|video| by_video.get(video).copied())
                    })
                    .and_then(|id| by_id.remove(&id));
                match (hoarded, entry.ytm_video_id) {
                    (Some(track), _) => {
                        let art = covers
                            .get(&track.album_id)
                            .copied()
                            .unwrap_or(false)
                            .then_some(track.album_id);
                        Some(Song::Hoarded {
                            track: Box::new(track),
                            art,
                        })
                    }
                    (None, Some(video_id)) => Some(Song::Coming {
                        video_id,
                        title: entry.title,
                        artist: entry.artist,
                    }),
                    (None, None) => None,
                }
            })
            .collect();
        let excluded = match playlist.watch_id {
            Some(watch_id) => watch::exclusions(&mut db, watch_id).await?,
            None => Vec::new(),
        };
        shown.push(Shown {
            mirrors: playlist
                .watch_id
                .map(|id| watches.get(&id).cloned().unwrap_or_default()),
            playlist,
            songs,
            excluded,
        });
    }
    Ok(shown)
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<PlaylistsQuery>(cx)?;
    let playlists = playlists(cx).await?;
    // The playlist just changed stays open.
    let touched = query.excluded.or(query.included);

    Ok(view! {
        <div class="flex flex-col gap-6">
            if query.excluded.is_some() {
                snackbar(message: "Excluded. The watch lets go of it and won’t fetch it again.")
            }
            if query.included.is_some() {
                snackbar(message: "Included again. A sync is on its way.", action: ("Jobs", "/jobs"))
            }
            page_header(
                eyebrow: "Playlists",
                title: "Yours and your watches’",
                lede: "Make playlists in any Subsonic app. Watched playlists are mirrored here, read-only.",
            )
            if playlists.is_empty() {
                empty_state(
                    <p class="m-0">
                        "No playlists yet. Make one in a Subsonic app, or "
                        <a href="/watches">"watch a playlist"</a> " on YouTube Music."
                    </p>
                )
            }
            <div class="flex flex-col gap-2.5">
                for shown in &playlists {
                    let playlist = &shown.playlist;
                    let source = match &shown.mirrors {
                        Some(name) if !name.is_empty() => format!("mirrors the watch “{name}”"),
                        Some(_) => "mirrors a watch".to_owned(),
                        None => "made in a Subsonic app".to_owned(),
                    };
                    let watch_id = playlist.watch_id;
                    let back = |flag: &str| {
                        format!("/playlists?{flag}={}#playlist-{}", watch_id.unwrap_or_default(), playlist.id)
                    };
                    <details
                        id=(format!("playlist-{}", playlist.id))
                        open=(watch_id.is_some() && watch_id == touched)
                        class="group overflow-hidden rounded-[20px] bg-card"
                    >
                        <summary
                            class="flex cursor-pointer list-none items-center gap-4 px-[18px] py-3.5 \
                                   hover:bg-hover"
                        >
                            <div class="w-14 shrink-0">cover(album: shown.cover(), size: 120)</div>
                            <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                                <span class=(class!(TRUNCATE, "text-base"))>(&playlist.name)</span>
                                <span class="text-[13px] text-muted-foreground">
                                    (count(shown.songs.len(), "song", "songs"))
                                    if shown.duration_ms() > 0 {
                                        " · " (format_duration(shown.duration_ms()))
                                    }
                                    " · " (source)
                                </span>
                            </div>
                            if shown.mirrors.is_some() {
                                pill(
                                    attrs: topcoat::view::attributes! { class="bg-slate-container text-foreground" },
                                    "Read-only"
                                )
                            }
                        </summary>
                        <div class="border-t border-border py-1">
                            if shown.songs.is_empty() {
                                <p class="m-0 px-[18px] py-3 text-sm text-muted-foreground">"No songs yet."</p>
                            }
                            for (index, song) in shown.songs.iter().enumerate() {
                                <div
                                    class="grid min-h-[52px] grid-cols-[28px_36px_minmax(0,1fr)_auto] items-center \
                                           gap-3 px-[18px] py-1"
                                >
                                    <span class="text-right text-[13px] text-slate-soft">(index + 1)</span>
                                    match song {
                                        Song::Hoarded { track, art } => {
                                            <a href=(format!("/albums/{}?track={}", track.album_id, track.id)) class="block">
                                                cover(album: *art, size: 120, attrs: topcoat::view::attributes! { class="rounded-md" })
                                            </a>
                                            <span class="flex min-w-0 flex-col">
                                                <span class=(class!(TRUNCATE, "text-sm"))>(&track.title)</span>
                                                <span class=(class!(TRUNCATE, "text-xs text-muted-foreground"))>
                                                    (&track.artist_credit)
                                                </span>
                                            </span>
                                            <span class="flex items-center gap-1">
                                                <span class="text-[13px] text-muted-foreground tabular-nums">
                                                    (format_duration(track.duration_ms))
                                                </span>
                                                if let (Some(watch_id), Some(video)) = (watch_id, &track.ytm_video_id) {
                                                    exclude_button(watch_id: watch_id, video: video.as_str(), back: back("excluded"))
                                                }
                                            </span>
                                        },
                                        Song::Coming { video_id, title, artist } => {
                                            <span class="size-9 rounded-md border border-dashed border-outline"></span>
                                            <span class="flex min-w-0 flex-col">
                                                <a
                                                    href=(format!("https://music.youtube.com/watch?v={video_id}"))
                                                    target="_blank"
                                                    rel="noopener"
                                                    class=(class!(TRUNCATE, "text-sm text-foreground-soft hover:text-foreground"))
                                                >
                                                    (title.as_deref().unwrap_or(video_id))
                                                </a>
                                                <span class=(class!(TRUNCATE, "text-xs text-muted-foreground"))>
                                                    if let Some(artist) = artist {
                                                        (artist) " · "
                                                    }
                                                    "not in the hoard yet"
                                                </span>
                                            </span>
                                            if let Some(watch_id) = watch_id {
                                                exclude_button(watch_id: watch_id, video: video_id.as_str(), back: back("excluded"))
                                            } else {
                                                <span></span>
                                            }
                                        },
                                    }
                                </div>
                            }
                            if let (Some(watch_id), false) = (watch_id, shown.excluded.is_empty()) {
                                <div class="mt-1 border-t border-border px-[18px] pt-3 pb-2">
                                    <p class="m-0 mb-1 text-[13px] font-medium text-muted-foreground">
                                        "Excluded (" (shown.excluded.len()) ")"
                                    </p>
                                    for exclusion in &shown.excluded {
                                        <div class="flex min-h-[44px] items-center gap-3">
                                            <span class="flex min-w-0 flex-1 flex-col">
                                                <span class=(class!(TRUNCATE, "text-sm text-foreground-soft"))>
                                                    (exclusion.title.as_deref().unwrap_or(&exclusion.ytm_video_id))
                                                </span>
                                                if let Some(artist) = &exclusion.artist {
                                                    <span class=(class!(TRUNCATE, "text-xs text-muted-foreground"))>(artist)</span>
                                                }
                                            </span>
                                            <form method="post" action=(format!("/watches/{watch_id}/include"))>
                                                <input type="hidden" name="video" value=(&exclusion.ytm_video_id)>
                                                <input type="hidden" name="back" value=(back("included"))>
                                                <button
                                                    type="submit"
                                                    aria-label="Include again"
                                                    title="Include again"
                                                    class=(ICON_BUTTON)
                                                >
                                                    icon(data: icons::PLAYLIST_ADD, size: 20)
                                                </button>
                                            </form>
                                        </div>
                                    }
                                </div>
                            }
                        </div>
                    </details>
                }
            </div>
        </div>
    })
}

/// Excludes a song from its watched playlist.
#[topcoat::view::component]
async fn exclude_button(watch_id: u64, video: &str, back: String) -> Result<impl View> {
    Ok(view! {
        <form method="post" action=(format!("/watches/{watch_id}/exclude"))>
            <input type="hidden" name="video" value=(video)>
            <input type="hidden" name="back" value=(back)>
            <button
                type="submit"
                aria-label="Exclude from this playlist"
                title="Exclude from this playlist"
                class=(ICON_BUTTON)
            >
                icon(data: icons::PLAYLIST_REMOVE, size: 20)
            </button>
        </form>
    })
}
