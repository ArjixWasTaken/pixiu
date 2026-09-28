//! `/orphans`: tracks nothing keeps any more, until the admin deletes or
//! keeps them.

use std::collections::HashMap;

use pixiu_db::{Album, ClaimKind};
use pixiu_treasury::Claim;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        page, query_params,
        request::Bytes,
        route,
    },
    view::{View, view},
};

use crate::{
    auth::{db, require_user, treasury},
    ui::{BUTTON_DANGER, BUTTON_SECONDARY, card, format_bytes, format_duration, notice, relative},
};

pub(super) const ORPHANS_PATH: &str = "/orphans";

#[query_params(error = bad_request)]
struct OrphansQuery {
    deleted: Option<u64>,
    kept: Option<String>,
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<OrphansQuery>(cx)?;
    let orphans = treasury(cx).orphans().await?;
    let album_ids: Vec<u64> = orphans.iter().map(|track| track.album_id).collect();
    let albums: HashMap<u64, String> = if album_ids.is_empty() {
        HashMap::new()
    } else {
        Album::filter(Album::fields().id().in_list(album_ids))
            .exec(&mut db(cx))
            .await?
            .into_iter()
            .map(|album| (album.id, album.title))
            .collect()
    };
    let total: u64 = orphans.iter().map(|track| track.size).sum();

    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex flex-col gap-1">
                <h2 class="text-3xl font-bold text-gold">"Orphans"</h2>
                <p class="text-muted-foreground">
                    "Tracks nothing keeps any more: they left a watched playlist, or "
                    "the playlist or watch that wanted them is gone. píxiū never deletes "
                    "them by itself."
                </p>
            </header>

            if let Some(deleted) = query.deleted {
                notice(
                    "Deleted " (deleted) if deleted == 1 { " track." } else { " tracks." }
                )
            }
            if query.kept.is_some() {
                notice("Kept. It stays in the hoard for good.")
            }

            if orphans.is_empty() {
                card(
                    <p class="py-6 text-center text-sm text-muted-foreground">
                        "No orphans. Everything in the hoard is wanted."
                    </p>
                )
            } else {
                card(
                    <form id="orphans" method="post" action="/orphans/delete">
                        <table class="w-full text-left text-sm">
                            <tbody class="divide-y divide-border">
                                for track in &orphans {
                                    <tr class="align-middle">
                                        <td class="w-8 py-3">
                                            <input
                                                type="checkbox"
                                                name="track"
                                                value=(track.id)
                                                aria-label=(format!("Select {}", track.title))
                                                class="accent-gold"
                                            >
                                        </td>
                                        <td class="py-3 pr-4">
                                            <p class="font-medium">(&track.title)</p>
                                            <p class="text-xs text-muted-foreground">
                                                (&track.artist_credit)
                                                if let Some(album) = albums.get(&track.album_id) {
                                                    " · " (album)
                                                }
                                            </p>
                                        </td>
                                        <td class="w-40 py-3 text-xs text-muted-foreground">
                                            (format_duration(track.duration_ms)) " · " (format_bytes(track.size))
                                            <br>
                                            "Added " (relative(track.added_at))
                                        </td>
                                        <td class="w-44 py-3">
                                            <div class="flex justify-end gap-2">
                                                <button
                                                    form="keep-one"
                                                    name="track"
                                                    value=(track.id)
                                                    class=(BUTTON_SECONDARY)
                                                >
                                                    "Keep"
                                                </button>
                                                <button
                                                    form="delete-one"
                                                    name="track"
                                                    value=(track.id)
                                                    onclick="return confirm('Delete this track and its file?')"
                                                    class=(BUTTON_DANGER)
                                                >
                                                    "Delete"
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            </tbody>
                        </table>
                    </form>
                    <div class="mt-4 flex items-center justify-between gap-4 border-t border-border pt-4">
                        <p class="text-sm text-muted-foreground">
                            (orphans.len()) if orphans.len() == 1 { " orphan, " } else { " orphans, " }
                            (format_bytes(total))
                        </p>
                        <div class="flex items-center gap-2">
                            <button
                                form="orphans"
                                type="submit"
                                onclick="return confirm('Delete the selected tracks and their files?')"
                                class=(BUTTON_SECONDARY)
                            >
                                "Delete selected"
                            </button>
                            <form
                                method="post"
                                action="/orphans/delete"
                                onsubmit="return confirm('Delete every orphan and its file?')"
                            >
                                <input type="hidden" name="all" value="1">
                                <button type="submit" class=(BUTTON_DANGER)>"Delete all"</button>
                            </form>
                        </div>
                    </div>
                )
                <form id="delete-one" method="post" action="/orphans/delete"></form>
                <form id="keep-one" method="post" action="/orphans/keep"></form>
            }
        </div>
    })
}

/// The `track` ids of a form body, which may repeat.
fn tracks(body: &[u8]) -> Vec<u64> {
    form_urlencoded::parse(body)
        .filter(|(key, _)| key == "track")
        .filter_map(|(_, value)| value.parse().ok())
        .collect()
}

#[route(POST "./delete")]
async fn delete(cx: &Cx, body: Bytes) -> Result<SeeOther> {
    require_user(cx).await?;
    let treasury = treasury(cx);
    let everything = form_urlencoded::parse(&body).any(|(key, value)| key == "all" && value == "1");
    let ids = if everything {
        treasury
            .orphans()
            .await?
            .iter()
            .map(|track| track.id)
            .collect()
    } else {
        tracks(&body)
    };
    let deleted = treasury.delete_orphans(&ids).await?;
    Ok(see_other(format!("{ORPHANS_PATH}?deleted={deleted}")))
}

#[route(POST "./keep")]
async fn keep(cx: &Cx, body: Bytes) -> Result<SeeOther> {
    require_user(cx).await?;
    let keep = Claim {
        kind: ClaimKind::ManualGrab,
        reference: None,
    };
    for id in tracks(&body) {
        if pixiu_db::Track::filter_by_id(id)
            .first()
            .exec(&mut db(cx))
            .await?
            .is_some()
        {
            treasury(cx).claim(id, &keep).await?;
        }
    }
    Ok(see_other(format!("{ORPHANS_PATH}?kept=1")))
}

#[cfg(test)]
mod tests {
    use super::tracks;

    #[test]
    fn repeated_ids_are_read() {
        assert_eq!(tracks(b"track=3&track=12&all=0&track=x"), [3, 12]);
    }
}
