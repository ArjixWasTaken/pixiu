//! `/library`: every album in the hoard, and those that need a look.

use std::collections::HashMap;

use pixiu_db::{Album, Artist, Enrichment};
use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    view::{View, view},
};

use crate::{
    auth::{db, require_user},
    ui::{ALBUM_GRID, Size, Tone, album_tile, btn, chip, empty_state, page_header},
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
    Unmatched,
    New,
}

impl Show {
    const ALL: [Self; 4] = [Self::All, Self::Review, Self::Unmatched, Self::New];

    fn parse(value: Option<&str>) -> Self {
        match value {
            Some("review") => Self::Review,
            Some("unmatched") => Self::Unmatched,
            Some("new") => Self::New,
            _ => Self::All,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Review => "review",
            Self::Unmatched => "unmatched",
            Self::New => "new",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Review => "Needs your pick",
            Self::Unmatched => "Not on MusicBrainz",
            Self::New => "Not looked up yet",
        }
    }

    fn includes(self, album: &Album) -> bool {
        match self {
            Self::All => true,
            Self::Review => album.enrichment == Some(Enrichment::Review),
            Self::Unmatched => album.enrichment == Some(Enrichment::Unmatched),
            Self::New => album.enrichment.is_none(),
        }
    }

    /// What an empty filter says.
    fn empty_text(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Review => "No album is waiting for your pick.",
            Self::Unmatched => "Every album is known to MusicBrainz.",
            Self::New => "Every album has been looked up.",
        }
    }

    fn href(self, number: usize) -> String {
        match (self, number) {
            (Self::All, 1) => "/library".to_owned(),
            (Self::All, number) => format!("/library?page={number}"),
            (show, 1) => format!("/library?show={}", show.key()),
            (show, number) => format!("/library?show={}&page={number}", show.key()),
        }
    }
}

/// The label on an album's cover, for albums that want a look.
fn flag(album: &Album) -> Option<&'static str> {
    match album.enrichment {
        Some(Enrichment::Review) => Some("Needs your pick"),
        Some(Enrichment::Unmatched) => Some("Not on MusicBrainz"),
        Some(Enrichment::Matched) => None,
        None => Some("Not looked up"),
    }
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<LibraryQuery>(cx)?;
    let show = Show::parse(query.show.as_deref());
    let mut db = db(cx);
    let albums = Album::all().exec(&mut db).await?;
    let artists: HashMap<u64, String> = Artist::all()
        .exec(&mut db)
        .await?
        .into_iter()
        .map(|artist| (artist.id, artist.name))
        .collect();
    let counts: Vec<(Show, usize)> = Show::ALL
        .iter()
        .map(|filter| {
            (
                *filter,
                albums.iter().filter(|album| filter.includes(album)).count(),
            )
        })
        .collect();
    let total = albums.len();

    let mut shown: Vec<Album> = albums
        .into_iter()
        .filter(|album| show.includes(album))
        .collect();
    let artist_name = move |id: u64| artists.get(&id).cloned().unwrap_or_default();
    shown.sort_by_key(|album| {
        (
            artist_name(album.artist_id).to_lowercase(),
            album.year,
            album.title.to_lowercase(),
        )
    });
    let matching = shown.len();
    let pages = matching.div_ceil(PAGE_SIZE).max(1);
    let current = query.page.unwrap_or(1).clamp(1, pages);
    let first = (current - 1) * PAGE_SIZE;
    let shown: Vec<Album> = shown.into_iter().skip(first).take(PAGE_SIZE).collect();
    let range = if matching == 0 {
        String::new()
    } else if pages > 1 {
        format!("{}–{} of {matching}", first + 1, first + shown.len())
    } else if matching == 1 {
        "1 album".to_owned()
    } else {
        format!("{matching} albums")
    };

    Ok(view! {
        <div class="flex flex-col gap-6">
            page_header(
                eyebrow: "The treasure",
                title: "Library",
                <span class="text-sm text-muted-foreground">(range)</span>
            )

            <nav aria-label="Filter" class="flex gap-2 overflow-x-auto pb-0.5">
                for (filter, count) in &counts {
                    let href = filter.href(1);
                    let label = format!("{} · {count}", filter.label());
                    chip(href: &href, selected: *filter == show, label: &label)
                }
            </nav>

            if shown.is_empty() {
                if total == 0 {
                    empty_state(
                        title: "The treasure is empty",
                        <p class="m-0">"Hunt an album or make an offering, and it lands here."</p>
                        <div class="flex gap-2">
                            <a href="/hunt" class=(btn(Tone::Filled, Size::S))>"Hunt"</a>
                            <a href="/offerings" class=(btn(Tone::Outlined, Size::S))>
                                "Make an offering"
                            </a>
                        </div>
                    )
                } else {
                    empty_state(<p class="m-0 text-[15px]">(show.empty_text())</p>)
                }
            } else {
                <div class=(ALBUM_GRID)>
                    for album in &shown {
                        let href = format!("/albums/{}", album.id);
                        let subtitle = match album.year {
                            Some(year) => format!("{} · {year}", artist_name(album.artist_id)),
                            None => artist_name(album.artist_id),
                        };
                        album_tile(
                            href: &href,
                            cover_of: album.cover.as_ref().map(|_| album.id),
                            title: &album.title,
                            subtitle: subtitle,
                            flag: flag(album),
                        )
                    }
                </div>
            }

            if pages > 1 {
                <nav aria-label="Pages" class="flex items-center justify-center gap-4 pt-2">
                    <a
                        href=(show.href(current.saturating_sub(1).max(1)))
                        aria-disabled=((current <= 1).then_some("true"))
                        class=(btn(Tone::Outlined, Size::S))
                    >
                        "Previous"
                    </a>
                    <span class="text-sm text-muted-foreground tabular-nums">
                        "Page " (current) " of " (pages)
                    </span>
                    <a
                        href=(show.href((current + 1).min(pages)))
                        aria-disabled=((current >= pages).then_some("true"))
                        class=(btn(Tone::Outlined, Size::S))
                    >
                        "Next"
                    </a>
                </nav>
            }
        </div>
    })
}
