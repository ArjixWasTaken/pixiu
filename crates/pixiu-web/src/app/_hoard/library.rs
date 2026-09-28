//! `/library`: every album in the hoard, and those that need a look.

use std::collections::HashMap;

use pixiu_db::{Album, Artist, Enrichment};
use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    view::{View, class, view},
};

use crate::{
    auth::{db, require_user},
    ui::{BUTTON_SECONDARY, LOGO, card},
};

const PAGE_SIZE: usize = 60;

#[query_params(error = bad_request)]
struct LibraryQuery {
    show: Option<String>,
    page: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Show {
    All,
    Review,
    New,
}

fn badge(album: &Album) -> Option<(&'static str, &'static str)> {
    match album.enrichment {
        Some(Enrichment::Review) => Some(("Needs your pick", "bg-gold text-gold-foreground")),
        Some(Enrichment::Unmatched) => {
            Some(("Not on MusicBrainz", "bg-muted text-muted-foreground"))
        }
        Some(Enrichment::Matched) => None,
        None => Some(("Not looked up", "bg-muted text-muted-foreground")),
    }
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<LibraryQuery>(cx)?;
    let show = match query.show.as_deref() {
        Some("review") => Show::Review,
        Some("new") => Show::New,
        _ => Show::All,
    };
    let mut db = db(cx);
    let albums = Album::all().exec(&mut db).await?;
    let artists: HashMap<u64, String> = Artist::all()
        .exec(&mut db)
        .await?
        .into_iter()
        .map(|artist| (artist.id, artist.name))
        .collect();
    let review = albums
        .iter()
        .filter(|album| album.enrichment == Some(Enrichment::Review))
        .count();
    let fresh = albums
        .iter()
        .filter(|album| album.enrichment.is_none())
        .count();
    let total = albums.len();

    let mut shown: Vec<Album> = albums
        .into_iter()
        .filter(|album| match show {
            Show::All => true,
            Show::Review => album.enrichment == Some(Enrichment::Review),
            Show::New => album.enrichment.is_none(),
        })
        .collect();
    let artist_name = |id: u64| artists.get(&id).map_or("", String::as_str).to_lowercase();
    shown.sort_by_key(|album| {
        (
            artist_name(album.artist_id),
            album.year,
            album.title.to_lowercase(),
        )
    });
    let pages = shown.len().div_ceil(PAGE_SIZE).max(1);
    let current = query.page.unwrap_or(1).clamp(1, pages);
    let shown: Vec<Album> = shown
        .into_iter()
        .skip((current - 1) * PAGE_SIZE)
        .take(PAGE_SIZE)
        .collect();
    let filter = match show {
        Show::All => "",
        Show::Review => "review",
        Show::New => "new",
    };
    let page_link = move |page: usize| format!("/library?show={filter}&page={page}");

    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Library"</h2>
                <p class="text-muted-foreground">
                    "Every album in the hoard. píxiū looks new albums up on MusicBrainz for "
                    "their tags, covers and lyrics; the doubtful ones wait for you."
                </p>
            </header>

            <nav class="flex gap-2 text-sm">
                for (key, label, count) in [
                    (Show::All, "All", total),
                    (Show::Review, "Needs your pick", review),
                    (Show::New, "Not looked up", fresh),
                ] {
                    <a
                        href=(match key {
                            Show::All => "/library",
                            Show::Review => "/library?show=review",
                            Show::New => "/library?show=new",
                        })
                        class=(class!(
                            "rounded-full border px-3 py-1 transition",
                            "border-gold/60 bg-gold/10 text-gold" if key == show,
                            "border-border text-muted-foreground hover:text-foreground" if key != show,
                        ))
                    >
                        (label) " · " (count)
                    </a>
                }
            </nav>

            if shown.is_empty() {
                card(
                    <p class="py-6 text-center text-sm text-muted-foreground">"Nothing here."</p>
                )
            } else {
                <ul class="grid grid-cols-2 gap-5 sm:grid-cols-3 lg:grid-cols-5">
                    for album in &shown {
                        <li>
                            <a href=(format!("/albums/{}", album.id)) class="group flex flex-col gap-2">
                                if album.cover.is_some() {
                                    <img
                                        src=(format!("/covers/{}?size=300", album.id))
                                        alt=""
                                        loading="lazy"
                                        class="aspect-square w-full rounded-xl border border-border object-cover shadow-lg transition group-hover:border-gold/60"
                                    >
                                } else {
                                    <div class="flex aspect-square w-full items-center justify-center rounded-xl border border-border bg-muted transition group-hover:border-gold/60">
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
                                    if let Some((label, colors)) = badge(album) {
                                        <span class=(class!("mt-1 inline-block rounded-full px-2 text-xs", colors))>(label)</span>
                                    }
                                </div>
                            </a>
                        </li>
                    }
                </ul>
            }

            if pages > 1 {
                <div class="flex items-center justify-center gap-3 text-sm">
                    if current > 1 {
                        <a href=(page_link(current - 1)) class=(BUTTON_SECONDARY)>"Previous"</a>
                    }
                    <span class="text-muted-foreground">"Page " (current) " of " (pages)</span>
                    if current < pages {
                        <a href=(page_link(current + 1)) class=(BUTTON_SECONDARY)>"Next"</a>
                    }
                </div>
            }
        </div>
    })
}
