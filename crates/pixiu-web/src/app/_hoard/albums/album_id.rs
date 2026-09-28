//! `/albums/{album_id}`: an album, how MusicBrainz sees it, its lyrics, and
//! editing what the hoard says about it.

use std::collections::HashMap;

use pixiu_db::{
    Album, Annotation, Artist, ClaimKind, Enrichment, Lyrics, LyricsSource, Playlist, Track,
    TrackClaim, TrackOrigin, Watch,
};
use pixiu_enrich::Candidate;
use pixiu_jobs::NewJob;
use pixiu_treasury::{AlbumEdit, ArtistRef, TrackEdit};
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{
        error::{RouterErrorExt, SeeOther, see_other},
        page, path_param, query_params,
        request::Bytes,
        route,
    },
    runtime::signal,
    view::{View, attributes, class, component, view},
};

use crate::{
    auth::{db, jobs, require_user, treasury},
    ui::{
        EYEBROW, H2, ICON_BUTTON, LABEL, Size, TRUNCATE, Tone, artist_picture, btn, cover,
        format_bytes, format_duration, icons, relative, snackbar, text_field,
    },
};

path_param!(album_id: u64, error = bad_request);

#[query_params(error = bad_request)]
struct AlbumQuery {
    queued: Option<String>,
    saved: Option<String>,
    error: Option<String>,
    /// A track to show in the sheet.
    track: Option<u64>,
}

fn album_path(id: u64) -> String {
    format!("/albums/{id}")
}

/// How a track's lyrics read in the track list: a label and its color.
fn lyrics_label(lyrics: Option<&Lyrics>) -> (&'static str, &'static str) {
    match lyrics.map(|lyrics| (lyrics.source, lyrics.synced.is_some())) {
        None | Some((LyricsSource::Missing, _)) => ("None", "text-outline"),
        Some((LyricsSource::Instrumental, _)) => ("Instrumental", "text-muted-foreground"),
        Some((_, true)) => ("Synced", "text-gold"),
        Some((_, false)) => ("Lyrics", "text-slate-soft"),
    }
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let id = *path_param::<AlbumId>(cx)?;
    let query = query_params::<AlbumQuery>(cx)?;
    let mut db = db(cx);
    let album = Album::filter_by_id(id)
        .first()
        .exec(&mut db)
        .await?
        .ok_or_not_found()?;
    let artist = Artist::get_by_id(&mut db, &album.artist_id).await?;
    let mut tracks = Track::filter_by_album_id(id).exec(&mut db).await?;
    tracks.sort_by_key(|track| (track.disc_number, track.track_number, track.id));
    let track_ids: Vec<u64> = tracks.iter().map(|track| track.id).collect();
    let lyrics: HashMap<u64, Lyrics> = if track_ids.is_empty() {
        HashMap::new()
    } else {
        Lyrics::filter(Lyrics::fields().track_id().in_list(track_ids))
            .exec(&mut db)
            .await?
            .into_iter()
            .map(|lyrics| (lyrics.track_id, lyrics))
            .collect()
    };
    let candidates: Vec<Candidate> = album
        .candidates
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    let total_ms: u64 = tracks.iter().map(|track| track.duration_ms).sum();
    let sheet = match query.track {
        Some(track_id) => match tracks.iter().find(|track| track.id == track_id) {
            Some(track) => Some(TrackSheet::load(cx, track, lyrics.get(&track_id)).await?),
            None => None,
        },
        None => None,
    };
    let multi_disc = tracks
        .iter()
        .any(|track| track.disc_number.is_some_and(|disc| disc > 1));
    let editing = signal(cx, || false);
    let close = album_path(id);
    let edit_action = format!("/albums/{id}/edit");
    let artist_href = format!("/artists/{}", artist.id);
    let source = if album.ytm_browse_id.is_some() {
        "YouTube Music"
    } else {
        "Offering"
    };

    Ok(view! {
        <div class="flex flex-col gap-7">
            if query.queued.is_some() {
                snackbar(
                    message: "Looking it up. Jobs shows when it is done.",
                    action: ("Jobs", "/jobs"),
                )
            }
            if query.saved.is_some() {
                snackbar(message: "Saved. The files’ tags and places changed along.")
            }
            if let Some(error) = &query.error {
                snackbar(message: error, error: true)
            }

            <form id="album-edit" method="post" action=(&edit_action) hidden=""></form>

            <section class="relative flex flex-wrap items-end gap-7 overflow-hidden rounded-[28px] bg-card p-6">
                if album.cover.is_some() {
                    <img
                        src=(format!("/covers/{id}?size=300"))
                        alt=""
                        aria-hidden="true"
                        class="pointer-events-none absolute inset-0 size-full scale-125 object-cover \
                               opacity-20 blur-3xl"
                    >
                }
                <div class="relative w-[min(240px,100%)] shrink-0">
                    cover(album: album.cover.as_ref().map(|_| id), size: 600)
                </div>
                <div class="relative flex min-w-0 flex-[1_1_280px] flex-col gap-2.5">
                    <span class=(EYEBROW)>"Album · " (source)</span>
                    <div :hidden=$(editing.get()) class="flex flex-col gap-2.5">
                        <h1 class="m-0 text-[clamp(28px,4vw,44px)] leading-[1.1] font-normal text-balance">
                            (&album.title)
                        </h1>
                        <div class="flex flex-wrap items-center gap-x-2.5 gap-y-1.5 text-[15px] text-muted-foreground">
                            <a href=(&artist_href) class="font-medium text-foreground hover:text-foreground hover:underline">
                                (&artist.name)
                            </a>
                            if let Some(year) = album.year {
                                <span>"·"</span><span>(year)</span>
                            }
                            <span>"·"</span>
                            <span>(tracks.len()) if tracks.len() == 1 { " track" } else { " tracks" }</span>
                            <span>"·"</span>
                            <span>(format_duration(total_ms))</span>
                        </div>
                    </div>
                    <div :hidden=$(!editing.get()) class="flex max-w-[520px] flex-col gap-3 [--field-bg:var(--card)]">
                        text_field(
                            label: "Album title",
                            filled: true,
                            attrs: attributes! { form="album-edit" name="title" value=(&album.title) required="" },
                        )
                        <div class="flex flex-wrap gap-3">
                            text_field(
                                label: "Album artist",
                                filled: true,
                                attrs: attributes! {
                                    wrapper-class="flex-[1_1_220px]"
                                    form="album-edit" name="artist" value=(&artist.name) required=""
                                },
                            )
                            text_field(
                                label: "Year",
                                filled: true,
                                attrs: attributes! {
                                    wrapper-class="flex-[0_1_120px] min-w-[100px]"
                                    form="album-edit" name="year" inputmode="numeric" pattern="[0-9]{1,4}"
                                    value=(album.year.map(|year| year.to_string()).unwrap_or_default())
                                },
                            )
                        </div>
                    </div>
                    <div class="mt-2 flex flex-wrap gap-2">
                        <button
                            type="button"
                            :hidden=$(editing.get())
                            @click=$(|_e| editing.set(true))
                            class=(btn(Tone::Tonal, Size::S))
                        >
                            icon(data: icons::EDIT, size: 20)
                            "Edit"
                        </button>
                        <button
                            type="submit"
                            form="album-edit"
                            :hidden=$(!editing.get())
                            class=(btn(Tone::Filled, Size::S))
                        >
                            "Save and move files"
                        </button>
                        <button
                            type="reset"
                            form="album-edit"
                            :hidden=$(!editing.get())
                            @click=$(|_e| editing.set(false))
                            class=(btn(Tone::Text, Size::S))
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            </section>

            <section class="flex flex-wrap items-center gap-x-5 gap-y-3.5 rounded-[20px] border border-border px-5 py-[18px]">
                <div
                    class="grid size-11 shrink-0 place-items-center rounded-full border border-gold/40 \
                           bg-gold-container text-[13px] font-medium text-gold-soft"
                >
                    "MB"
                </div>
                <div class="flex flex-[1_1_260px] flex-col gap-0.5">
                    match album.enrichment {
                        Some(Enrichment::Matched) => {
                            <span class="text-base font-medium">"Tagged from MusicBrainz"</span>
                            if let Some(mbid) = &album.mbid {
                                <a
                                    href=(format!("https://musicbrainz.org/release/{mbid}"))
                                    target="_blank"
                                    rel="noopener"
                                    class="text-sm"
                                >
                                    "View the release on MusicBrainz"
                                </a>
                            }
                        },
                        Some(Enrichment::Review) => {
                            <span class="text-base font-medium text-gold-soft">
                                "Several releases might be this one; pick one."
                            </span>
                            <span class="text-sm text-muted-foreground">"Compare them below."</span>
                        },
                        Some(Enrichment::Unmatched) => {
                            <span class="text-base font-medium">"MusicBrainz knows nothing like it."</span>
                            <span class="text-sm text-muted-foreground">
                                "Its tags stay as they came. You can edit them, or paste a release below."
                            </span>
                        },
                        None => {
                            <span class="text-base font-medium">"Not looked up yet."</span>
                            <span class="text-sm text-muted-foreground">
                                "A lookup runs in the background and takes a few seconds."
                            </span>
                        },
                    }
                </div>
                <form method="post" action=(format!("/albums/{id}/lookup"))>
                    <button type="submit" class=(btn(Tone::Outlined, Size::S))>
                        if album.enrichment.is_some() { "Look it up again" } else { "Look it up now" }
                    </button>
                </form>
            </section>

            if album.enrichment == Some(Enrichment::Review) && !candidates.is_empty() {
                <section class="flex flex-col gap-3.5">
                    <h2 class=(H2)>"Which release is it?"</h2>
                    <div class="grid grid-cols-[repeat(auto-fill,minmax(min(100%,280px),1fr))] gap-3">
                        for candidate in &candidates {
                            let percent = (candidate.score * 100.0).round();
                            <div class="flex flex-col gap-3.5 rounded-[20px] bg-card p-[18px]">
                                <div class="flex items-center gap-3.5">
                                    <div
                                        class="grid size-[52px] shrink-0 place-items-center rounded-full"
                                        style=(format!(
                                            "background:conic-gradient(var(--gold) {percent}%, var(--border) 0)"
                                        ))
                                    >
                                        <div
                                            class="grid size-[42px] place-items-center rounded-full bg-card \
                                                   text-[13px] font-medium text-gold-soft"
                                        >
                                            (format!("{percent:.0}%"))
                                        </div>
                                    </div>
                                    <div class="flex min-w-0 flex-col gap-0.5">
                                        <span class=(class!(TRUNCATE, "text-base font-medium"))>(&candidate.title)</span>
                                        <span class="text-[13px] text-muted-foreground">(&candidate.artist)</span>
                                    </div>
                                </div>
                                <dl class="m-0 grid grid-cols-[auto_1fr] gap-x-3.5 gap-y-1.5 text-sm">
                                    <dt class="text-muted-foreground">"Date"</dt>
                                    <dd class="m-0">(candidate.date.as_deref().unwrap_or("—"))</dd>
                                    <dt class="text-muted-foreground">"Country"</dt>
                                    <dd class="m-0">(candidate.country.as_deref().unwrap_or("—"))</dd>
                                    <dt class="text-muted-foreground">"Format"</dt>
                                    <dd class="m-0">(candidate.format.as_deref().unwrap_or("—"))</dd>
                                    <dt class="text-muted-foreground">"Tracks"</dt>
                                    <dd class="m-0">(candidate.track_count) " tracks"</dd>
                                </dl>
                                <div class="mt-auto flex items-center justify-between gap-2">
                                    <a
                                        href=(format!("https://musicbrainz.org/release/{}", candidate.id))
                                        target="_blank"
                                        rel="noopener"
                                        class="text-sm"
                                    >
                                        "On MusicBrainz"
                                    </a>
                                    <form method="post" action=(format!("/albums/{id}/use"))>
                                        <input type="hidden" name="release" value=(&candidate.id)>
                                        <button type="submit" class=(btn(Tone::Filled, Size::S))>"Use this"</button>
                                    </form>
                                </div>
                            </div>
                        }
                    </div>
                </section>
            }

            <div class="flex flex-wrap items-start gap-7">
                <section class="flex min-w-0 flex-[2_1_480px] flex-col gap-1.5">
                    <h2 class=(class!(H2, "mb-2"))>"Tracks"</h2>
                    <div :hidden=$(!editing.get()) class="flex flex-col gap-2.5">
                        for track in &tracks {
                            <div class="flex gap-2.5">
                                text_field(
                                    label: "No.",
                                    attrs: attributes! {
                                        wrapper-class="w-[84px] shrink-0"
                                        form="album-edit" name=(format!("number-{}", track.id))
                                        inputmode="numeric" pattern="[0-9]{1,3}"
                                        value=(track.track_number.map(|number| number.to_string()).unwrap_or_default())
                                    },
                                )
                                text_field(
                                    label: "Title",
                                    attrs: attributes! {
                                        wrapper-class="flex-1"
                                        form="album-edit" name=(format!("title-{}", track.id))
                                        value=(&track.title) required=""
                                    },
                                )
                            </div>
                        }
                    </div>
                    <div :hidden=$(editing.get()) class="flex flex-col">
                        for track in &tracks {
                            let (label, color) = lyrics_label(lyrics.get(&track.id));
                            let number = match (multi_disc, track.disc_number, track.track_number) {
                                (_, _, None) => String::new(),
                                (true, Some(disc), Some(number)) => format!("{disc}·{number:02}"),
                                (_, _, Some(number)) => format!("{number:02}"),
                            };
                            <a
                                href=(format!("/albums/{id}?track={}", track.id))
                                class="grid min-h-14 grid-cols-[44px_minmax(0,1fr)_auto_48px] items-center \
                                       gap-3 rounded-xl px-3 py-1.5 text-foreground hover:bg-hover \
                                       hover:text-foreground"
                            >
                                <span class="font-mono text-[13px] text-slate-soft">(number)</span>
                                <span class="flex min-w-0 flex-col gap-0.5">
                                    <span class=(class!(TRUNCATE, "text-[15px]"))>(&track.title)</span>
                                    <span class=(class!(TRUNCATE, "text-[13px] text-muted-foreground"))>
                                        (&track.artist_credit)
                                    </span>
                                </span>
                                <span class=(class!("text-xs font-medium whitespace-nowrap", color))>(label)</span>
                                <span class="text-right text-[13px] text-muted-foreground tabular-nums">
                                    (format_duration(track.duration_ms))
                                </span>
                            </a>
                        }
                    </div>
                </section>
                <aside class="flex min-w-0 flex-[1_1_300px] flex-col gap-4">
                    <div class="flex flex-col gap-3.5 rounded-3xl bg-card p-5">
                        <div class="flex items-center gap-3.5">
                            artist_picture(artist: &artist, size: "size-14 text-2xl")
                            <div class="flex min-w-0 flex-col gap-0.5">
                                <span class=(LABEL)>"Artist"</span>
                                <a href=(&artist_href) class="text-lg text-foreground hover:text-foreground hover:underline">
                                    (&artist.name)
                                </a>
                            </div>
                        </div>
                        if let Some(bio) = &artist.bio {
                            <p class="m-0 text-sm leading-[21px] text-pretty text-foreground-soft">(bio)</p>
                            if let Some(url) = &artist.bio_url {
                                <a href=(url) target="_blank" rel="noopener" class="text-sm">"Read on Wikipedia"</a>
                            }
                        } else if artist.info_fetched_at.is_some() {
                            <p class="m-0 text-sm text-muted-foreground">
                                "Wikipedia has no article about this artist yet."
                            </p>
                        }
                    </div>
                    <form
                        method="post"
                        action=(format!("/albums/{id}/use"))
                        class="flex flex-col gap-3 rounded-3xl border border-border p-5"
                    >
                        <span class="text-base font-medium">"Tag from a MusicBrainz release"</span>
                        <span class="text-[13px] leading-[18px] text-muted-foreground">
                            "Paste a release link or its id. Titles, numbers and the cover come from it."
                        </span>
                        text_field(label: "Release link or id", attrs: attributes! { name="release" required="" })
                        <div>
                            <button type="submit" class=(btn(Tone::Tonal, Size::S))>"Tag from release"</button>
                        </div>
                    </form>
                </aside>
            </div>

            if let Some(sheet) = &sheet {
                track_sheet(sheet: sheet, album: &album, close: &close)
            }
        </div>
    })
}

/// Everything the track sheet shows about a track.
struct TrackSheet {
    track: Track,
    lyrics: Option<Lyrics>,
    annotation: Option<Annotation>,
    /// Why it is kept, for people.
    kept: Vec<String>,
}

impl TrackSheet {
    async fn load(cx: &Cx, track: &Track, lyrics: Option<&Lyrics>) -> Result<Self> {
        let mut db = db(cx);
        let track = Track::get_by_id(&mut db, &track.id).await?;
        let annotation = Annotation::filter_by_item(format!("tr-{}", track.id))
            .first()
            .exec(&mut db)
            .await?;
        let lyrics = match lyrics {
            Some(lyrics) => {
                Lyrics::filter_by_id(lyrics.id)
                    .first()
                    .exec(&mut db)
                    .await?
            }
            None => None,
        };
        let mut kept = Vec::new();
        for claim in TrackClaim::filter_by_track_id(track.id)
            .exec(&mut db)
            .await?
        {
            let reference: Option<u64> = claim.reference.as_deref().and_then(|id| id.parse().ok());
            let watch_name = async |db: &mut pixiu_db::Db| -> Result<Option<String>> {
                Ok(match reference {
                    Some(id) => Watch::filter_by_id(id)
                        .first()
                        .exec(db)
                        .await?
                        .map(|watch| watch.name),
                    None => None,
                })
            };
            kept.push(match claim.kind {
                ClaimKind::Offering => "You offered it".to_owned(),
                ClaimKind::ManualGrab => "You grabbed or kept it".to_owned(),
                ClaimKind::Starred => "Starred in an app".to_owned(),
                ClaimKind::WatchPlaylist => match watch_name(&mut db).await? {
                    Some(name) => format!("Watched playlist “{name}”"),
                    None => "A watched playlist".to_owned(),
                },
                ClaimKind::WatchArtist => match watch_name(&mut db).await? {
                    Some(name) => format!("Watched artist “{name}”"),
                    None => "A watched artist".to_owned(),
                },
                ClaimKind::LocalPlaylist => {
                    let name = match reference {
                        Some(id) => Playlist::filter_by_id(id)
                            .first()
                            .exec(&mut db)
                            .await?
                            .map(|playlist| playlist.name),
                        None => None,
                    };
                    match name {
                        Some(name) => format!("Playlist “{name}”"),
                        None => "A playlist".to_owned(),
                    }
                }
            });
        }
        kept.dedup();
        Ok(Self {
            track,
            lyrics,
            annotation,
            kept,
        })
    }

    /// "Opus · 160 kbps · 48 kHz".
    fn format(&self) -> String {
        let track = &self.track;
        let mut parts = vec![track.suffix.to_uppercase()];
        if let Some(bitrate) = track.bitrate {
            parts.push(format!("{bitrate} kbps"));
        }
        if let Some(depth) = track.bit_depth {
            parts.push(format!("{depth}-bit"));
        }
        if let Some(rate) = track.sample_rate {
            #[allow(clippy::cast_precision_loss)]
            let khz = f64::from(rate) / 1000.0;
            parts.push(format!(
                "{} kHz",
                format!("{khz:.1}").trim_end_matches(".0")
            ));
        }
        parts.join(" · ")
    }
}

/// Synced lyrics as `(timestamp, words)` lines.
fn synced_lines(lrc: &str) -> Vec<(String, String)> {
    lrc.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix('[')?;
            let (stamp, words) = rest.split_once(']')?;
            // Tags like `[ar: …]` are not times.
            stamp
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
                .then(|| (stamp.to_owned(), words.trim().to_owned()))
        })
        .collect()
}

/// A track's details, in a side sheet (a bottom sheet on phones). Closing
/// it returns to the album.
#[component]
async fn track_sheet(sheet: &TrackSheet, album: &Album, close: &str) -> Result<impl View> {
    let track = &sheet.track;
    let annotation = sheet.annotation.as_ref();
    let rating = annotation
        .and_then(|annotation| annotation.rating)
        .unwrap_or(0);
    let plays = annotation.map_or(0, |annotation| annotation.play_count);
    let lyrics = sheet.lyrics.as_ref();
    let lyrics_state = match lyrics {
        None => "not looked up yet",
        Some(lyrics) => match lyrics.source {
            LyricsSource::File => "from the file",
            LyricsSource::Lrclib => "from LRCLIB",
            LyricsSource::YouTubeMusic => "from YouTube Music",
            LyricsSource::Instrumental => "instrumental",
            LyricsSource::Missing => "none found",
        },
    };
    let origin = match (track.origin, &track.source_name, &track.source_archive) {
        (TrackOrigin::Download, _, _) => "YouTube Music".to_owned(),
        (TrackOrigin::Offering, Some(name), Some(archive)) => {
            format!("An offering: {name}, from {archive}")
        }
        (TrackOrigin::Offering, Some(name), None) => format!("An offering: {name}"),
        (TrackOrigin::Offering, None, _) => "An offering".to_owned(),
    };
    Ok(view! {
        <a
            href=(close)
            aria-label="Close"
            class="fixed inset-0 z-[80] bg-black/45"
        ></a>
        <div
            role="dialog"
            aria-modal="true"
            aria-label=(&track.title)
            class="fixed inset-x-0 bottom-0 z-[81] h-[82%] overflow-hidden rounded-t-[28px] \
                   shadow-2xl md:inset-y-0 md:right-0 md:left-auto md:h-auto md:w-[420px] \
                   md:max-w-full md:rounded-l-[28px] md:rounded-tr-none"
        >
            <div class="flex h-full flex-col gap-5 overflow-y-auto bg-muted px-[22px] pt-5 pb-7">
                <div class="flex items-start justify-between gap-3">
                    <div class="flex min-w-0 flex-col gap-1">
                        <span class=(LABEL)>
                            "Track " (track.track_number.map(|number| number.to_string()).unwrap_or_default())
                            " · " (&album.title)
                        </span>
                        <span class="text-[22px] leading-7">(&track.title)</span>
                        <span class="text-sm text-muted-foreground">
                            (&track.artist_credit) " · " (format_duration(track.duration_ms))
                        </span>
                    </div>
                    <a href=(close) aria-label="Close" class=(ICON_BUTTON)>
                        icon(data: icons::CLOSE, size: 24)
                    </a>
                </div>
                <div class="flex items-center gap-4 rounded-[14px] bg-card px-3.5 py-3">
                    <div class="flex gap-0.5" aria-label=(format!("Rated {rating} of 5"))>
                        for star in 1..=5_u8 {
                            if star <= rating {
                                <span class="text-gold">icon(data: icons::STAR, size: 18)</span>
                            } else {
                                <span class="text-outline">icon(data: icons::STAR_OUTLINE, size: 18)</span>
                            }
                        }
                    </div>
                    <span class="flex-1 text-[13px] text-muted-foreground">"Rating set in Subsonic apps"</span>
                    if annotation.is_some_and(|annotation| annotation.starred_at.is_some()) {
                        <span class="text-xs font-medium text-gold-soft">"Starred"</span>
                    }
                </div>
                <dl class="m-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-[18px] gap-y-2.5 text-sm">
                    <dt class="text-muted-foreground">"Format"</dt>
                    <dd class="m-0">(sheet.format())</dd>
                    <dt class="text-muted-foreground">"Size"</dt>
                    <dd class="m-0">(format_bytes(track.size))</dd>
                    <dt class="text-muted-foreground">"Came from"</dt>
                    <dd class="m-0">
                        if let Some(video) = &track.ytm_video_id {
                            <a href=(format!("https://music.youtube.com/watch?v={video}")) target="_blank" rel="noopener">
                                (origin)
                            </a>
                        } else {
                            (origin)
                        }
                    </dd>
                    <dt class="text-muted-foreground">"Plays"</dt>
                    <dd class="m-0">
                        (plays)
                        if let Some(last) = annotation.and_then(|annotation| annotation.last_played) {
                            " · last " (relative(last))
                        }
                    </dd>
                    <dt class="text-muted-foreground">"Added"</dt>
                    <dd class="m-0">(relative(track.added_at))</dd>
                    <dt class="text-muted-foreground">"File"</dt>
                    <dd class="m-0 font-mono text-xs leading-[18px] break-all text-foreground-soft">(&track.path)</dd>
                </dl>
                <div class="flex flex-col gap-2">
                    <span class="text-sm font-medium">"Why it’s kept"</span>
                    if sheet.kept.is_empty() {
                        <a href="/orphans" class="text-[13px]">"Nothing keeps it: it is an orphan."</a>
                    } else {
                        <div class="flex flex-wrap gap-1.5">
                            for reason in &sheet.kept {
                                <span class="rounded-lg border border-outline px-3 py-1.5 text-[13px]">(reason)</span>
                            }
                        </div>
                    }
                </div>
                <div class="flex flex-col gap-2.5">
                    <span class="text-sm font-medium">"Lyrics · " (lyrics_state)</span>
                    match lyrics {
                        Some(Lyrics { synced: Some(synced), .. }) => {
                            <div class="flex flex-col gap-2">
                                for (stamp, words) in synced_lines(synced) {
                                    <div class="flex items-baseline gap-3">
                                        <span class="shrink-0 font-mono text-xs text-gold">(stamp)</span>
                                        <span class="text-sm leading-5 text-foreground-soft">
                                            if words.is_empty() { "♪" } else { (words) }
                                        </span>
                                    </div>
                                }
                            </div>
                        },
                        Some(Lyrics { plain: Some(plain), .. }) => {
                            <p class="m-0 text-sm leading-6 whitespace-pre-line text-foreground-soft">(plain)</p>
                        },
                        Some(Lyrics { source: LyricsSource::Instrumental, .. }) => {
                            <span class="text-sm text-muted-foreground">"Marked instrumental: no words to show."</span>
                        },
                        _ => {
                            <span class="text-sm text-muted-foreground">"No lyrics found yet."</span>
                        },
                    }
                </div>
            </div>
        </div>
    })
}

/// Queues a lookup of the album.
async fn look_up(cx: &Cx, release: Option<String>) -> Result<SeeOther> {
    let id = *path_param::<AlbumId>(cx)?;
    let album = Album::filter_by_id(id)
        .first()
        .exec(&mut db(cx))
        .await?
        .ok_or_not_found()?;
    let title = format!("Look up {}", album.title);
    let fresh = release.is_none();
    jobs(cx)
        .enqueue(NewJob::enrich(id, &title, release, fresh))
        .await?;
    Ok(see_other(format!("{}?queued=1", album_path(id))))
}

#[route(POST "./lookup")]
async fn lookup(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    look_up(cx, None).await
}

/// A MusicBrainz release id, from an id or a link.
fn release_id(input: &str) -> Option<String> {
    let input = input.trim().trim_end_matches('/');
    let id = input.rsplit('/').next()?;
    let valid = id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    valid.then(|| id.to_ascii_lowercase())
}

#[route(POST "./use")]
async fn use_release(cx: &Cx, body: Bytes) -> Result<SeeOther> {
    require_user(cx).await?;
    let given = form_urlencoded::parse(&body)
        .find(|(key, _)| key == "release")
        .map(|(_, value)| value.into_owned())
        .unwrap_or_default();
    match release_id(&given) {
        Some(release) => look_up(cx, Some(release)).await,
        None => {
            let id = *path_param::<AlbumId>(cx)?;
            Ok(see_other(format!(
                "{}?error={}",
                album_path(id),
                form_urlencoded::byte_serialize(b"That is not a MusicBrainz release link.")
                    .collect::<String>()
            )))
        }
    }
}

#[route(POST "./edit")]
async fn edit(cx: &Cx, body: Bytes) -> Result<SeeOther> {
    require_user(cx).await?;
    let id = *path_param::<AlbumId>(cx)?;
    let form: HashMap<String, String> = form_urlencoded::parse(&body).into_owned().collect();
    let text = |key: &str| {
        form.get(key)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let mut db = db(cx);
    let album = Album::filter_by_id(id)
        .first()
        .exec(&mut db)
        .await?
        .ok_or_not_found()?;
    let album_artist = Artist::get_by_id(&mut db, &album.artist_id).await?;
    let artist_name = text("artist").unwrap_or_else(|| album_artist.name.clone());
    let mut tracks = Vec::new();
    for track in Track::filter_by_album_id(id).exec(&mut db).await? {
        let artist = Artist::get_by_id(&mut db, &track.artist_id).await?;
        tracks.push(TrackEdit {
            track_id: track.id,
            title: text(&format!("title-{}", track.id)).unwrap_or_else(|| track.title.clone()),
            artist_credit: track.artist_credit.clone(),
            artist: ArtistRef {
                name: artist.name,
                mbid: artist.mbid,
            },
            track_number: text(&format!("number-{}", track.id))
                .and_then(|number| number.parse().ok()),
            disc_number: track.disc_number,
            mbid: None,
            isrc: None,
        });
    }
    treasury(cx)
        .edit_album(&AlbumEdit {
            album_id: id,
            title: text("title").unwrap_or_else(|| album.title.clone()),
            // The same name keeps the same artist, MusicBrainz id and all.
            artist: ArtistRef {
                mbid: (artist_name == album_artist.name)
                    .then(|| album_artist.mbid.clone())
                    .flatten(),
                name: artist_name,
            },
            year: text("year").and_then(|year| year.parse().ok()),
            mbid: None,
            rg_mbid: None,
            tracks,
        })
        .await?;
    Ok(see_other(format!("{}?saved=1", album_path(id))))
}

#[cfg(test)]
mod tests {
    use super::{release_id, synced_lines};

    #[test]
    fn synced_lyrics_are_read_by_line() {
        let lines = synced_lines("[ar: Someone]\n[00:12.30] First line\n[00:15.00]\nnot a line");
        assert_eq!(
            lines,
            [
                ("00:12.30".to_owned(), "First line".to_owned()),
                ("00:15.00".to_owned(), String::new()),
            ]
        );
    }

    #[test]
    fn release_links_are_read() {
        let id = "8c0b6e0e-6c9d-4ac5-8a31-3f2f3f5c0d1e";
        assert_eq!(release_id(id).as_deref(), Some(id));
        assert_eq!(
            release_id(&format!("https://musicbrainz.org/release/{id}/")).as_deref(),
            Some(id)
        );
        assert_eq!(release_id("https://musicbrainz.org/release/nope"), None);
    }
}
