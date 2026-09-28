//! `/albums/{album_id}`: an album, how MusicBrainz sees it, its lyrics, and
//! editing what the hoard says about it.

use std::collections::HashMap;

use pixiu_db::{Album, Artist, Enrichment, Lyrics, LyricsSource, Track};
use pixiu_enrich::Candidate;
use pixiu_jobs::NewJob;
use pixiu_treasury::{AlbumEdit, ArtistRef, TrackEdit};
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{RouterErrorExt, SeeOther, see_other},
        page, path_param, query_params,
        request::Bytes,
        route,
    },
    view::{View, attributes, class, view},
};

use crate::{
    auth::{db, jobs, require_user, treasury},
    ui::{BUTTON_PRIMARY, BUTTON_SECONDARY, LOGO, alert, card, field, format_duration, notice},
};

path_param!(album_id: u64, error = bad_request);

#[query_params(error = bad_request)]
struct AlbumQuery {
    queued: Option<String>,
    saved: Option<String>,
    error: Option<String>,
}

fn album_path(id: u64) -> String {
    format!("/albums/{id}")
}

/// The label and colours of a track's lyrics badge.
fn lyrics_badge(lyrics: Option<&Lyrics>) -> Option<(&'static str, &'static str)> {
    const WORDS: &str = "bg-gold/10 text-gold";
    match lyrics.map(|lyrics| (lyrics.source, lyrics.synced.is_some())) {
        None | Some((LyricsSource::Missing, _)) => None,
        Some((LyricsSource::Instrumental, _)) => {
            Some(("Instrumental", "bg-slate/20 text-slate-soft"))
        }
        Some((_, true)) => Some(("Synced lyrics", WORDS)),
        Some((_, false)) => Some(("Lyrics", WORDS)),
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
    let mut lyrics: HashMap<u64, Lyrics> = HashMap::new();
    for track in &tracks {
        if let Some(found) = Lyrics::filter_by_track_id(track.id)
            .first()
            .exec(&mut db)
            .await?
        {
            lyrics.insert(track.id, found);
        }
    }
    let candidates: Vec<Candidate> = album
        .candidates
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    let total_ms: u64 = tracks.iter().map(|track| track.duration_ms).sum();
    let status = match album.enrichment {
        Some(Enrichment::Matched) => "Tagged from MusicBrainz",
        Some(Enrichment::Review) => "MusicBrainz has releases that might be this one; pick one",
        Some(Enrichment::Unmatched) => "MusicBrainz knows nothing like it",
        None => "Not looked up on MusicBrainz yet",
    };
    let artist_image = artist.image.is_some();

    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex items-end gap-6">
                if album.cover.is_some() {
                    <img
                        src=(format!("/covers/{id}?size=400"))
                        alt=""
                        class="size-48 rounded-xl border border-border object-cover shadow-xl"
                    >
                } else {
                    <div class="flex size-48 items-center justify-center rounded-xl border border-border bg-muted">
                        <img src=(LOGO) alt="" class="size-1/2 opacity-30">
                    </div>
                }
                <div class="flex min-w-0 flex-col gap-1">
                    <h2 class="text-3xl font-bold text-gold">(&album.title)</h2>
                    <p class="text-lg">(&artist.name)</p>
                    <p class="text-sm text-muted-foreground">
                        if let Some(year) = album.year {
                            (year) " · "
                        }
                        (tracks.len()) if tracks.len() == 1 { " track" } else { " tracks" }
                        " · " (format_duration(total_ms))
                    </p>
                    <p class="text-sm text-muted-foreground">
                        (status)
                        if let Some(mbid) = &album.mbid {
                            " · "
                            <a
                                href=(format!("https://musicbrainz.org/release/{mbid}"))
                                target="_blank"
                                rel="noreferrer"
                                class="underline"
                            >
                                "MusicBrainz"
                            </a>
                        }
                    </p>
                    <form method="post" action=(format!("/albums/{id}/lookup")) class="mt-2">
                        <button type="submit" class=(BUTTON_SECONDARY)>
                            if album.enrichment.is_some() { "Look it up again" } else { "Look it up now" }
                        </button>
                    </form>
                </div>
            </header>

            if query.queued.is_some() {
                notice("Looking it up. " <a href="/jobs" class="underline">"Jobs"</a> " shows when it is done; reload this page then.")
            }
            if query.saved.is_some() {
                notice("Saved. The files' tags and places changed along.")
            }
            if let Some(error) = &query.error {
                alert((error))
            }

            if album.enrichment == Some(Enrichment::Review) {
                card(
                    <div class="flex flex-col gap-4">
                        <h3 class="text-lg">"Which release is it?"</h3>
                        <p class="text-sm text-muted-foreground">
                            "Picking one tags the album from it: titles, track numbers, "
                            "MusicBrainz ids, and the cover when it has a better one."
                        </p>
                        <table class="w-full text-left text-sm">
                            <tbody class="divide-y divide-border">
                                for candidate in &candidates {
                                    <tr>
                                        <td class="py-2 pr-4">
                                            <p class="font-medium">(&candidate.title)</p>
                                            <p class="text-xs text-muted-foreground">
                                                (&candidate.artist)
                                                if let Some(date) = &candidate.date { " · " (date) }
                                                if let Some(country) = &candidate.country { " · " (country) }
                                                if let Some(format) = &candidate.format { " · " (format) }
                                                " · " (candidate.track_count) " tracks"
                                            </p>
                                        </td>
                                        <td class="w-20 py-2 text-right tabular-nums text-muted-foreground">
                                            (format!("{:.0}%", candidate.score * 100.0))
                                        </td>
                                        <td class="w-56 py-2">
                                            <div class="flex justify-end gap-2">
                                                <a
                                                    href=(format!("https://musicbrainz.org/release/{}", candidate.id))
                                                    target="_blank"
                                                    rel="noreferrer"
                                                    class=(BUTTON_SECONDARY)
                                                >
                                                    "View"
                                                </a>
                                                <form method="post" action=(format!("/albums/{id}/use"))>
                                                    <input type="hidden" name="release" value=(&candidate.id)>
                                                    <button type="submit" class=(BUTTON_PRIMARY)>"Use this"</button>
                                                </form>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            </tbody>
                        </table>
                    </div>
                )
            }

            card(
                <table class="w-full text-left text-sm">
                    <tbody class="divide-y divide-border">
                        for track in &tracks {
                            <tr>
                                <td class="w-12 py-2 tabular-nums text-muted-foreground">
                                    (track.track_number.map(|number| number.to_string()).unwrap_or_default())
                                </td>
                                <td class="py-2 pr-4">
                                    <p class="font-medium">(&track.title)</p>
                                    <p class="text-xs text-muted-foreground">(&track.artist_credit)</p>
                                </td>
                                <td class="w-32 py-2 text-right">
                                    if let Some((badge, colours)) = lyrics_badge(lyrics.get(&track.id)) {
                                        <span class=(class!("rounded-full px-2 text-xs", colours))>(badge)</span>
                                    }
                                </td>
                                <td class="w-16 py-2 text-right tabular-nums text-muted-foreground">
                                    (format_duration(track.duration_ms))
                                </td>
                            </tr>
                        }
                    </tbody>
                </table>
            )

            card(
                <details class="group">
                    <summary class="cursor-pointer text-lg">"Edit"</summary>
                    <form method="post" action=(format!("/albums/{id}/edit")) class="mt-4 flex flex-col gap-4">
                        <p class="text-sm text-muted-foreground">
                            "The files' tags change too, and the files move to match."
                        </p>
                        field(label: "Title", attrs: attributes! { name="title" value=(&album.title) required="" })
                        field(label: "Album artist", attrs: attributes! { name="artist" value=(&artist.name) required="" })
                        field(
                            label: "Year",
                            attrs: attributes! {
                                name="year" type="number" min="1" max="9999"
                                value=(album.year.map(|year| year.to_string()).unwrap_or_default())
                            },
                        )
                        <div class="flex flex-col gap-2">
                            <span class="text-sm font-medium">"Tracks"</span>
                            for track in &tracks {
                                <div class="grid grid-cols-[5rem_1fr] gap-3">
                                    <input
                                        name=(format!("number-{}", track.id))
                                        type="number"
                                        min="1"
                                        value=(track.track_number.map(|number| number.to_string()).unwrap_or_default())
                                        aria-label="Track number"
                                        class="h-10 rounded-lg border border-border bg-input px-3 text-sm"
                                    >
                                    <input
                                        name=(format!("title-{}", track.id))
                                        value=(&track.title)
                                        required=""
                                        aria-label="Title"
                                        class="h-10 rounded-lg border border-border bg-input px-3 text-sm"
                                    >
                                </div>
                            }
                        </div>
                        <div>
                            <button type="submit" class=(BUTTON_PRIMARY)>"Save"</button>
                        </div>
                    </form>
                    <form method="post" action=(format!("/albums/{id}/use")) class="mt-6 flex items-end gap-3 border-t border-border pt-4">
                        <div class="flex-1">
                            field(
                                label: "Or tag it from a MusicBrainz release",
                                attrs: attributes! {
                                    name="release" required=""
                                    placeholder="https://musicbrainz.org/release/…"
                                },
                            )
                        </div>
                        <button type="submit" class=(BUTTON_SECONDARY)>"Use it"</button>
                    </form>
                </details>
            )

            if let Some(bio) = &artist.bio {
                card(
                    <div class="flex gap-5">
                        if artist_image {
                            <img
                                src=(format!("/artists/{}/image", artist.id))
                                alt=""
                                class="size-28 shrink-0 rounded-xl border border-border object-cover"
                            >
                        }
                        <div class="flex flex-col gap-2">
                            <h3 class="text-lg">(&artist.name)</h3>
                            <p class="text-sm leading-relaxed text-muted-foreground">(bio)</p>
                            if let Some(url) = &artist.bio_url {
                                <a href=(url) target="_blank" rel="noreferrer" class=(class!("text-xs underline text-muted-foreground"))>
                                    "From Wikipedia"
                                </a>
                            }
                        </div>
                    </div>
                )
            }
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
    use super::release_id;

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
