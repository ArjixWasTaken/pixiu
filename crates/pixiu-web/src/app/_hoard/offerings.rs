//! `/offerings`: upload music, review what píxiū read from it, and let it be
//! absorbed into the treasure.

mod batch;
mod item;

use pixiu_db::{Offering, OfferingStatus};
use pixiu_treasury::Offerings;
use tokio::io::AsyncWriteExt;
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{
        content::multipart::Multipart,
        error::{SeeOther, see_other},
        page, query_params, route,
    },
    runtime::signal,
    view::{View, class, view},
};

use crate::{
    app::_hoard::count,
    auth::{offerings, require_user},
    ui::{
        ICON_BUTTON, Size, TRUNCATE, Tone, btn, format_bytes, format_duration, icons, page_header,
        relative, snackbar,
    },
};

pub(super) const OFFERINGS_PATH: &str = "/offerings";

/// What the file picker offers; the server re-checks every file.
const ACCEPT: &str = ".zip,.mp3,.flac,.ogg,.oga,.opus,.m4a,.aac,.wav,.aif,.aiff,.wv,.ape,.mpc,\
                      .jpg,.jpeg,.png,.webp";

#[query_params(error = bad_request)]
struct OfferingsQuery {
    /// How many offerings the last "accept" could not absorb.
    failed: Option<u32>,
    accepted: Option<u32>,
}

/// Offerings grouped by upload batch, in display order.
fn by_batch(offerings: Vec<Offering>) -> Vec<(String, Vec<Offering>)> {
    let mut batches: Vec<(String, Vec<Offering>)> = Vec::new();
    for offering in offerings {
        match batches.last_mut() {
            Some((batch, items)) if *batch == offering.batch => items.push(offering),
            _ => batches.push((offering.batch.clone(), vec![offering])),
        }
    }
    batches
}

/// "Album" and "Artist · year" for a batch, or a count when it mixes
/// albums.
fn describe(items: &[Offering]) -> (String, String) {
    let first = &items[0];
    let same_album = items
        .iter()
        .all(|item| item.album == first.album || item.status == OfferingStatus::Unreadable);
    if same_album && first.status == OfferingStatus::Pending {
        let artist = first.album_artist.as_deref().unwrap_or(&first.artist);
        let year = first
            .year
            .map(|year| format!(" · {year}"))
            .unwrap_or_default();
        (first.album.clone(), format!("{artist}{year}"))
    } else {
        ("Several albums".to_owned(), String::new())
    }
}

/// Where a batch came from: its archives, or loose files.
fn origin(items: &[Offering]) -> String {
    let mut archives: Vec<&str> = items
        .iter()
        .filter_map(|item| item.archive.as_deref())
        .collect();
    archives.sort_unstable();
    archives.dedup();
    let loose = items.iter().filter(|item| item.archive.is_none()).count();
    match (archives.as_slice(), loose) {
        ([], _) => "uploaded as files".to_owned(),
        ([archive], 0) => format!("from {archive}"),
        (archives, 0) => format!("from {} archives", archives.len()),
        (archives, _) => format!(
            "from {} and loose files",
            count(archives.len(), "archive", "archives")
        ),
    }
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<OfferingsQuery>(cx)?;
    let batches = by_batch(offerings(cx).pending().await?);
    let uploading = signal(cx, || false);
    let accepted_message = query.accepted.map(|accepted| {
        format!(
            "{} joined the treasure.",
            count(accepted as usize, "track", "tracks")
        )
    });
    let failed_message = query.failed.map(|failed| {
        format!(
            "{} could not be absorbed; the reason is shown next to each.",
            count(failed as usize, "file", "files"),
        )
    });

    Ok(view! {
        <div class="flex flex-col gap-7">
            page_header(eyebrow: "Offerings", title: "Feed it music you already own")

            if let Some(message) = &accepted_message {
                snackbar(message: message, action: ("Library", "/library"))
            }
            if let Some(message) = &failed_message {
                snackbar(message: message, error: true)
            }

            <form method="post" action="/offerings/upload" enctype="multipart/form-data">
                <label
                    class="relative flex min-h-[190px] cursor-pointer flex-col items-center justify-center \
                           gap-2.5 rounded-[28px] border-[1.5px] border-dashed border-gold/50 bg-dim p-7 \
                           text-center transition hover:border-gold \
                           bg-[radial-gradient(60%_80%_at_50%_0%,rgb(230_182_92/.08),transparent_70%)] \
                           focus-within:border-gold"
                >
                    <span class="grid size-14 place-items-center rounded-full bg-gold-container text-gold-soft">
                        icon(data: icons::UPLOAD, size: 24)
                    </span>
                    <span class="text-lg" :hidden=$(uploading.get())>
                        "Drop audio files or zip archives here"
                    </span>
                    <span class="text-lg" :hidden=$(!uploading.get())>"Offering…"</span>
                    <span class="text-sm text-muted-foreground">
                        "Several at once. MP3, FLAC, M4A, Opus, Ogg, WAV or ZIP. "
                        <span class="font-medium text-gold">"Choose files"</span>
                    </span>
                    <input
                        type="file"
                        name="files"
                        multiple=""
                        required=""
                        accept=(ACCEPT)
                        aria-label="Audio files or zip archives"
                        onchange="if (this.files.length) this.form.requestSubmit()"
                        @change=$(|_e| uploading.set(true))
                        class="absolute inset-0 cursor-pointer opacity-0"
                    >
                </label>
                <noscript>
                    <div class="mt-3 flex justify-end">
                        <button type="submit" class=(btn(Tone::Filled, Size::S))>"Offer"</button>
                    </div>
                </noscript>
            </form>

            if batches.is_empty() {
                <p class="m-0 text-center text-sm text-muted-foreground">
                    "No offerings wait for review. Whatever you upload lands here first."
                </p>
            }
            for (batch, items) in &batches {
                batch_card(batch: batch, items: items)
            }
        </div>
    })
}

#[topcoat::view::component]
async fn batch_card(batch: &str, items: &[Offering]) -> Result<impl View> {
    let (title, subtitle) = describe(items);
    let size: u64 = items.iter().map(|item| item.size).sum();
    let readable = items
        .iter()
        .filter(|item| item.status == OfferingStatus::Pending)
        .count();
    let offered = items
        .iter()
        .map(|item| item.created_at)
        .min()
        .map(relative)
        .unwrap_or_default();

    Ok(view! {
        <section class="overflow-hidden rounded-3xl bg-card">
            <div class="flex flex-wrap items-center gap-x-4 gap-y-3 border-b border-border px-5 py-[18px]">
                <div class="flex min-w-0 flex-[1_1_240px] flex-col gap-0.5">
                    <span class="text-lg">(title)</span>
                    <span class="text-[13px] text-muted-foreground">
                        if !subtitle.is_empty() {
                            (subtitle) " · "
                        }
                        (count(items.len(), "file", "files")) " · " (origin(items))
                        " · " (format_bytes(size)) " · offered " (offered)
                    </span>
                </div>
                <div class="flex gap-2">
                    <form method="post" action=(format!("/offerings/{batch}/discard"))>
                        <button type="submit" class=(btn(Tone::Text, Size::S))>"Discard"</button>
                    </form>
                    if readable > 0 {
                        <form method="post" action=(format!("/offerings/{batch}/accept"))>
                            <button type="submit" class=(btn(Tone::Filled, Size::S))>
                                if readable == items.len() { "Accept" } else { "Accept " (readable) }
                            </button>
                        </form>
                    }
                </div>
            </div>
            for item in items {
                let unreadable = item.status == OfferingStatus::Unreadable;
                <div
                    class="grid min-h-14 grid-cols-[36px_minmax(0,1fr)_auto_auto] items-center gap-3 \
                           border-b border-muted py-1.5 pr-2 pl-5"
                >
                    <span class="font-mono text-[13px] text-slate-soft">
                        match (item.disc_number, item.track_number) {
                            (Some(disc), Some(number)) if disc > 1 => (format!("{disc}·{number:02}")),
                            (_, Some(number)) => (format!("{number:02}")),
                            _ => "",
                        }
                    </span>
                    <span class="flex min-w-0 flex-col gap-0.5">
                        <span class=(class!(TRUNCATE, "text-[15px]", "text-muted-foreground" if unreadable))>
                            (&item.title)
                        </span>
                        if let Some(error) = &item.error {
                            <span class="text-[13px] text-destructive">(error)</span>
                        } else {
                            <span class=(class!(TRUNCATE, "text-[13px] text-muted-foreground"))>
                                (&item.artist) " · " (&item.file_name)
                            </span>
                        }
                    </span>
                    <span class="text-[13px] text-muted-foreground tabular-nums">
                        if !unreadable {
                            (format_duration(item.duration_ms))
                        }
                    </span>
                    <form method="post" action=(format!("/offerings/item/{}/discard", item.id))>
                        <button
                            type="submit"
                            aria-label="Discard this file"
                            title="Discard this file"
                            class=(ICON_BUTTON)
                        >
                            icon(data: icons::CLOSE, size: 20)
                        </button>
                    </form>
                </div>
            }
            <div class="px-5 py-3 text-[13px] text-muted-foreground">
                "Accepted files join the treasure and are looked up on MusicBrainz."
            </div>
        </section>
    })
}

/// Streams uploaded files to the staging area, then registers them.
#[route(POST "./upload")]
async fn upload(cx: &Cx, mut multipart: Multipart) -> Result<SeeOther> {
    require_user(cx).await?;
    let offerings = offerings(cx);
    let batch = Offerings::new_batch();

    let mut files = 0_u32;
    while let Some(mut field) = multipart.next_field().await? {
        let Some(name) = field
            .file_name()
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
        else {
            continue;
        };
        let (_, mut file) = offerings.create_upload(&batch, &name).await?;
        while let Some(chunk) = field.chunk().await? {
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        files += 1;
    }
    if files > 0 {
        let registered = offerings.process_batch(&batch).await?;
        tracing::info!(
            batch,
            files,
            offerings = registered.len(),
            "offering received"
        );
    }
    Ok(see_other(OFFERINGS_PATH))
}
