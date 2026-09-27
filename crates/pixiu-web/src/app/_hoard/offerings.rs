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
    router::{
        content::multipart::Multipart,
        error::{SeeOther, see_other},
        page, query_params, route,
    },
    view::{View, class, view},
};

use crate::{
    auth::{offerings, require_user},
    ui::{BUTTON_DANGER, BUTTON_PRIMARY, alert, card, format_bytes, format_duration, notice},
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

/// "Album, by Artist" for a batch, or a count when it mixes albums.
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
        (
            format!("{} files", items.len()),
            "Several albums".to_owned(),
        )
    }
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<OfferingsQuery>(cx)?;
    let batches = by_batch(offerings(cx).pending().await?);

    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Offerings"</h2>
                <p class="text-muted-foreground">
                    "Upload music you already own. píxiū reads its tags, you review, "
                    "and it joins the hoard."
                </p>
            </header>

            card(
                <form
                    method="post"
                    action="/offerings/upload"
                    enctype="multipart/form-data"
                    class="flex flex-col gap-4 sm:flex-row sm:items-end"
                >
                    <label class="flex flex-1 flex-col gap-1.5 text-sm">
                        <span class="font-medium">"Audio files or zip archives"</span>
                        <input
                            type="file"
                            name="files"
                            multiple=""
                            required=""
                            accept=(ACCEPT)
                            class="rounded-lg border border-dashed border-border bg-input \
                                   p-3 text-sm text-muted-foreground file:mr-3 file:rounded-md \
                                   file:border-0 file:bg-gold/15 file:px-3 file:py-1.5 \
                                   file:text-gold"
                        >
                    </label>
                    <button type="submit" class=(BUTTON_PRIMARY)>"Offer"</button>
                </form>
            )

            if let Some(accepted) = query.accepted {
                notice((accepted) " offering(s) joined the hoard.")
            }
            if let Some(failed) = query.failed {
                alert(
                    (failed)
                    " offering(s) could not be absorbed; the reason is shown next to each."
                )
            }

            if batches.is_empty() {
                <p class="text-center text-sm text-muted-foreground">
                    "Nothing awaits review."
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

    Ok(view! {
        card(
            <div class="flex flex-col gap-4">
                <div class="flex flex-wrap items-start justify-between gap-4">
                    <div>
                        <h3 class="text-lg">(title)</h3>
                        <p class="text-sm text-muted-foreground">
                            (subtitle) " · " (format_bytes(size))
                        </p>
                    </div>
                    <div class="flex gap-2">
                        if readable > 0 {
                            <form method="post" action=(format!("/offerings/{batch}/accept"))>
                                <button type="submit" class=(BUTTON_PRIMARY)>
                                    "Accept " (readable)
                                </button>
                            </form>
                        }
                        <form method="post" action=(format!("/offerings/{batch}/discard"))>
                            <button type="submit" class=(BUTTON_DANGER)>"Discard"</button>
                        </form>
                    </div>
                </div>
                <table class="w-full text-left text-sm">
                    <thead class="text-xs uppercase tracking-wider text-muted-foreground">
                        <tr>
                            <th class="w-16 py-2 font-medium">"#"</th>
                            <th class="py-2 font-medium">"Title"</th>
                            <th class="py-2 font-medium">"Artist"</th>
                            <th class="w-16 py-2 text-right font-medium">"Time"</th>
                            <th class="w-10"></th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-border">
                        for item in items {
                            <tr class=(class!(
                                "align-top",
                                "text-muted-foreground" if item.status == OfferingStatus::Unreadable,
                            ))>
                                <td class="py-2 tabular-nums text-muted-foreground">
                                    if let Some(disc) = item.disc_number {
                                        (disc) "-"
                                    }
                                    (item.track_number.map(|n| n.to_string()).unwrap_or_default())
                                </td>
                                <td class="py-2">
                                    <span class="font-medium">(&item.title)</span>
                                    if let Some(error) = &item.error {
                                        <p class="text-xs text-destructive">(error)</p>
                                    }
                                    <p class="text-xs text-muted-foreground">(&item.file_name)</p>
                                </td>
                                <td class="py-2">(&item.artist)</td>
                                <td class="py-2 text-right tabular-nums">
                                    (format_duration(item.duration_ms))
                                </td>
                                <td class="py-2 text-right">
                                    <form
                                        method="post"
                                        action=(format!("/offerings/item/{}/discard", item.id))
                                    >
                                        <button
                                            type="submit"
                                            title="Discard this file"
                                            class="rounded px-2 text-muted-foreground \
                                                   hover:text-destructive"
                                        >
                                            "✕"
                                        </button>
                                    </form>
                                </td>
                            </tr>
                        }
                    </tbody>
                </table>
            </div>
        )
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
