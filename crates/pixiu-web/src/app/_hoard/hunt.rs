//! `/hunt`: search YouTube Music and grab what the hoard lacks.

use std::collections::HashSet;

use pixiu_db::{Album, Track};
use pixiu_hunt::{AlbumKind, SearchResults, image_url_at};
use pixiu_jobs::NewJob;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{
        content::Form,
        error::{SeeOther, bad_request, see_other},
        page, query_params, route,
    },
    view::{View, attributes, class, component, view},
};

use crate::{
    auth::{db, hunter, jobs, require_user},
    ui::{
        H2, Size, TRUNCATE, Tone, alert, btn, cover, empty_state, format_duration, icons, in_hoard,
        page_header, snackbar, text_field,
    },
};

#[query_params(error = bad_request)]
struct HuntQuery {
    q: Option<String>,
    queued: Option<String>,
}

/// What of the results the hoard already holds.
#[derive(Default)]
struct Hoarded {
    tracks: HashSet<String>,
    albums: HashSet<String>,
}

async fn hoarded(cx: &Cx, results: &SearchResults) -> Result<Hoarded> {
    let mut db = db(cx);
    let video_ids: Vec<String> = results
        .tracks
        .iter()
        .map(|track| track.id.clone())
        .collect();
    let browse_ids: Vec<String> = results
        .albums
        .iter()
        .map(|album| album.id.clone())
        .collect();
    let mut hoarded = Hoarded::default();
    if !video_ids.is_empty() {
        hoarded.tracks = Track::filter(Track::fields().ytm_video_id().in_list(video_ids))
            .exec(&mut db)
            .await?
            .into_iter()
            .filter_map(|track| track.ytm_video_id)
            .collect();
    }
    if !browse_ids.is_empty() {
        hoarded.albums = Album::filter(Album::fields().ytm_browse_id().in_list(browse_ids))
            .exec(&mut db)
            .await?
            .into_iter()
            .filter_map(|album| album.ytm_browse_id)
            .collect();
    }
    Ok(hoarded)
}

fn kind_label(kind: AlbumKind) -> &'static str {
    match kind {
        AlbumKind::Album => "Album",
        AlbumKind::Ep => "EP",
        AlbumKind::Single => "Single",
        AlbumKind::Other => "Release",
    }
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<HuntQuery>(cx)?;
    let q = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(str::to_owned);
    let search = match &q {
        Some(q) => Some(hunter(cx).ytmusic().search(q).await),
        None => None,
    };
    let hoarded = match &search {
        Some(Ok(results)) => hoarded(cx, results).await?,
        _ => Hoarded::default(),
    };
    let pending = pixiu_jobs::pending(&mut db(cx)).await?;
    let q_value = q.clone().unwrap_or_default();
    let nothing = format!("Nothing found for “{q_value}”");

    Ok(view! {
        <div class="flex flex-col gap-7">
            <section class="flex flex-col gap-4">
                page_header(eyebrow: "Hunt", title: "What should the beast bring back?")
                <form method="get" action="/hunt" class="max-w-[720px]">
                    text_field(
                        label: "Artist, album or song",
                        filled: true,
                        leading: icons::SEARCH,
                        placeholder: "Search YouTube Music",
                        attrs: attributes! {
                            type="search" name="q" value=(&q_value) required=""
                            autofocus=(q.is_none())
                        },
                    )
                </form>
            </section>

            if query.queued.is_some() {
                snackbar(message: "Queued. Follow it on Jobs.", action: ("Jobs", "/jobs"))
            }

            match &search {
                Some(Err(error)) => {
                    alert(
                        title: "YouTube Music couldn’t be searched.",
                        <code class="font-mono text-[13px] break-words">(error.to_string())</code>
                        <span>"Try again in a moment."</span>
                    )
                }
                Some(Ok(results)) => {
                    if results.albums.is_empty() && results.tracks.is_empty() {
                        empty_state(
                            title: nothing.as_str(),
                            <p class="m-0">"Try the artist’s name alone, or check the spelling."</p>
                        )
                    }
                    if !results.albums.is_empty() {
                        <section class="flex flex-col gap-3.5">
                            <h2 class=(H2)>"Albums"</h2>
                            <div
                                class="grid grid-cols-[repeat(auto-fill,minmax(min(100%,300px),1fr))] gap-3"
                            >
                                for album in &results.albums {
                                    <div class="flex items-center gap-3.5 rounded-[20px] bg-card p-3">
                                        <div class="w-[84px] shrink-0">
                                            remote_cover(url: album.cover_url.as_deref())
                                        </div>
                                        <div class="flex min-w-0 flex-1 flex-col gap-1">
                                            <span class=(class!(TRUNCATE, "text-[15px] font-medium")) title=(&album.title)>
                                                (&album.title)
                                            </span>
                                            <span class=(class!(TRUNCATE, "text-[13px] text-muted-foreground"))>
                                                (album.artists.join(", "))
                                            </span>
                                            <span class="text-xs text-slate-soft">
                                                (kind_label(album.kind))
                                                if let Some(year) = album.year {
                                                    " · " (year)
                                                }
                                            </span>
                                            <div class="mt-1">
                                                if hoarded.albums.contains(&album.id) {
                                                    in_hoard()
                                                } else if pending.has_album(&album.id) {
                                                    <a href="/jobs" class="text-[13px] text-muted-foreground">
                                                        "Queued on Jobs"
                                                    </a>
                                                } else {
                                                    <form method="post" action="/hunt/grab-album">
                                                        <input type="hidden" name="browse_id" value=(&album.id)>
                                                        <input
                                                            type="hidden"
                                                            name="title"
                                                            value=(format!("{} — {}", album.artists.join(", "), album.title))
                                                        >
                                                        <input type="hidden" name="q" value=(&q_value)>
                                                        <button type="submit" class=(btn(Tone::Tonal, Size::Xs))>
                                                            icon(data: icons::DOWNLOAD, size: 18)
                                                            "Grab album"
                                                        </button>
                                                    </form>
                                                }
                                            </div>
                                        </div>
                                    </div>
                                }
                            </div>
                        </section>
                    }
                    if !results.tracks.is_empty() {
                        <section class="flex flex-col gap-1.5">
                            <h2 class=(class!(H2, "mb-2"))>"Songs"</h2>
                            for track in &results.tracks {
                                <div
                                    class="grid min-h-16 grid-cols-[minmax(0,1fr)_auto_auto] items-center \
                                           gap-4 border-b border-muted px-3 py-2"
                                >
                                    <span class="flex min-w-0 flex-col gap-0.5">
                                        <span class=(class!(TRUNCATE, "text-[15px]"))>(&track.title)</span>
                                        <span class=(class!(TRUNCATE, "text-[13px] text-muted-foreground"))>
                                            (track.artist_credit())
                                            if let Some(album) = &track.album {
                                                " · " (&album.title)
                                            }
                                        </span>
                                    </span>
                                    <span class="text-[13px] text-muted-foreground tabular-nums">
                                        (track.duration_secs.map(|secs| format_duration(u64::from(secs) * 1000)).unwrap_or_default())
                                    </span>
                                    <span>
                                        if hoarded.tracks.contains(&track.id) {
                                            in_hoard()
                                        } else if pending.tracks.contains(&track.id) {
                                            <a href="/jobs" class="text-[13px] text-muted-foreground">"Queued"</a>
                                        } else {
                                            <form method="post" action="/hunt/grab-track">
                                                <input type="hidden" name="video_id" value=(&track.id)>
                                                <input
                                                    type="hidden"
                                                    name="title"
                                                    value=(format!("{} — {}", track.artist_credit(), track.title))
                                                >
                                                <input type="hidden" name="q" value=(&q_value)>
                                                <button type="submit" class=(btn(Tone::Outlined, Size::Xs))>"Grab"</button>
                                            </form>
                                        }
                                    </span>
                                </div>
                            }
                        </section>
                    }
                }
                None => "",
            }
        </div>
    })
}

/// An album cover from the platform, or a placeholder.
#[component]
async fn remote_cover(url: Option<&str>) -> Result<impl View> {
    Ok(view! {
        match url {
            Some(url) => {
                <img
                    src=(image_url_at(url, 400))
                    alt=""
                    loading="lazy"
                    referrerpolicy="no-referrer"
                    class="aspect-square w-full rounded-xl object-cover"
                >
            }
            None => cover(album: None),
        }
    })
}

/// Platform ids are short and use a URL-safe alphabet.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn back_to_results(q: Option<&str>) -> String {
    match q.filter(|q| !q.is_empty()) {
        Some(q) => format!(
            "/hunt?q={}&queued=1",
            form_urlencoded::byte_serialize(q.as_bytes()).collect::<String>()
        ),
        None => "/hunt?queued=1".to_owned(),
    }
}

#[derive(Deserialize)]
struct GrabTrack {
    video_id: String,
    title: String,
    q: Option<String>,
}

#[route(POST "./grab-track")]
async fn grab_track(cx: &Cx, Form(form): Form<GrabTrack>) -> Result<SeeOther> {
    require_user(cx).await?;
    if !valid_id(&form.video_id) {
        return Err(bad_request("invalid track").into());
    }
    jobs(cx)
        .enqueue(NewJob::track(&form.video_id, &form.title, None))
        .await?;
    Ok(see_other(back_to_results(form.q.as_deref())))
}

#[derive(Deserialize)]
struct GrabAlbum {
    browse_id: String,
    title: String,
    q: Option<String>,
}

#[route(POST "./grab-album")]
async fn grab_album(cx: &Cx, Form(form): Form<GrabAlbum>) -> Result<SeeOther> {
    require_user(cx).await?;
    if !valid_id(&form.browse_id) {
        return Err(bad_request("invalid album").into());
    }
    jobs(cx)
        .enqueue(NewJob::album(&form.browse_id, &form.title))
        .await?;
    Ok(see_other(back_to_results(form.q.as_deref())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_checked_and_queries_encoded() {
        assert!(valid_id("NPdgPZ0u3zQ"));
        assert!(valid_id("MPREb_jwN9EIjDfPS"));
        assert!(!valid_id("../etc"));
        assert!(!valid_id(""));
        assert_eq!(back_to_results(Some("a b&c")), "/hunt?q=a+b%26c&queued=1");
        assert_eq!(back_to_results(None), "/hunt?queued=1");
    }
}
