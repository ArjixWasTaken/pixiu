//! `/hunt`: search YouTube Music and grab what the hoard lacks.

use std::collections::HashSet;

use pixiu_db::{Album, Track};
use pixiu_hunt::{AlbumKind, SearchResults, image_url_at};
use pixiu_jobs::NewJob;
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, bad_request, see_other},
        page, query_params, route,
    },
    view::{View, attributes, component, view},
};

use crate::{
    auth::{db, hunter, jobs, require_user},
    ui::{BUTTON_PRIMARY, BUTTON_SECONDARY, alert, card, field, format_duration, notice},
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
    let q_value = q.clone().unwrap_or_default();

    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Hunt"</h2>
                <p class="text-muted-foreground">
                    "Search YouTube Music and bring music into the hoard."
                </p>
            </header>

            <form method="get" action="/hunt" class="flex items-end gap-3">
                <div class="flex-1">
                    field(
                        label: "Search",
                        attrs: attributes! {
                            name="q" value=(&q_value) placeholder="Artist, album or song"
                            autofocus="" required=""
                        },
                    )
                </div>
                <button type="submit" class=(BUTTON_PRIMARY)>"Search"</button>
            </form>

            if query.queued.is_some() {
                notice("Queued. Follow the download on the " <a href="/jobs" class="underline">"Jobs"</a> " page.")
            }

            match &search {
                Some(Err(error)) => {
                    alert("YouTube Music could not be searched: " (error.to_string()))
                }
                Some(Ok(results)) => {
                    if results.albums.is_empty() && results.tracks.is_empty() {
                        <p class="text-sm text-muted-foreground">"Nothing found."</p>
                    }
                    if !results.albums.is_empty() {
                        <section class="flex flex-col gap-4">
                            <h3 class="text-lg">"Albums"</h3>
                            <ul class="grid grid-cols-2 gap-5 sm:grid-cols-3 lg:grid-cols-4">
                                for album in &results.albums {
                                    <li class="flex flex-col gap-2">
                                        remote_cover(url: album.cover_url.as_deref())
                                        <div class="min-w-0">
                                            <p class="truncate font-medium" title=(&album.title)>
                                                (&album.title)
                                            </p>
                                            <p class="truncate text-sm text-muted-foreground">
                                                (album.artists.join(", "))
                                            </p>
                                            <p class="text-xs text-muted-foreground">
                                                (kind_label(album.kind))
                                                if let Some(year) = album.year {
                                                    " · " (year)
                                                }
                                            </p>
                                        </div>
                                        if hoarded.albums.contains(&album.id) {
                                            <span class="text-xs text-gold">"In the hoard"</span>
                                        } else {
                                            <form method="post" action="/hunt/grab-album">
                                                <input type="hidden" name="browse_id" value=(&album.id)>
                                                <input
                                                    type="hidden"
                                                    name="title"
                                                    value=(format!("{} — {}", album.artists.join(", "), album.title))
                                                >
                                                <input type="hidden" name="q" value=(&q_value)>
                                                <button type="submit" class=(BUTTON_SECONDARY)>"Grab album"</button>
                                            </form>
                                        }
                                    </li>
                                }
                            </ul>
                        </section>
                    }
                    if !results.tracks.is_empty() {
                        card(
                            <h3 class="mb-3 text-lg">"Songs"</h3>
                            <table class="w-full text-left text-sm">
                                <tbody class="divide-y divide-border">
                                    for track in &results.tracks {
                                        <tr>
                                            <td class="py-2 pr-3">
                                                <p class="font-medium">(&track.title)</p>
                                                <p class="text-xs text-muted-foreground">
                                                    (track.artist_credit())
                                                    if let Some(album) = &track.album {
                                                        " · " (&album.title)
                                                    }
                                                </p>
                                            </td>
                                            <td class="w-16 py-2 text-right tabular-nums text-muted-foreground">
                                                (track.duration_secs.map(|secs| format_duration(u64::from(secs) * 1000)).unwrap_or_default())
                                            </td>
                                            <td class="w-32 py-2 text-right">
                                                if hoarded.tracks.contains(&track.id) {
                                                    <span class="text-xs text-gold">"In the hoard"</span>
                                                } else {
                                                    <form method="post" action="/hunt/grab-track">
                                                        <input type="hidden" name="video_id" value=(&track.id)>
                                                        <input
                                                            type="hidden"
                                                            name="title"
                                                            value=(format!("{} — {}", track.artist_credit(), track.title))
                                                        >
                                                        <input type="hidden" name="q" value=(&q_value)>
                                                        <button type="submit" class=(BUTTON_SECONDARY)>"Grab"</button>
                                                    </form>
                                                }
                                            </td>
                                        </tr>
                                    }
                                </tbody>
                            </table>
                        )
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
                    class="aspect-square w-full rounded-xl border border-border object-cover"
                >
            }
            None => {
                <div class="aspect-square w-full rounded-xl border border-border bg-muted"></div>
            }
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
