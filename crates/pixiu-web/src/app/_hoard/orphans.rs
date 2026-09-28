//! `/orphans`: tracks nothing keeps any more, until the admin deletes or
//! keeps them.

use std::collections::HashMap;

use pixiu_db::{Album, ClaimKind, ReleaseReason, ReleasedClaim};
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
    runtime::{Event, signal},
    view::{View, view},
};

use crate::{
    app::_hoard::count,
    auth::{db, require_user, treasury},
    ui::{
        CHECKBOX, Size, Tone, btn, confirm_dialog, empty_state, format_bytes, format_duration,
        open_dialog, page_header, relative, snackbar,
    },
};

pub(super) const ORPHANS_PATH: &str = "/orphans";

#[query_params(error = bad_request)]
struct OrphansQuery {
    deleted: Option<u64>,
    kept: Option<String>,
}

/// Why nothing keeps a track any more, for people.
fn why(released: Option<&ReleasedClaim>) -> String {
    let Some(released) = released else {
        return "nothing claims it".to_owned();
    };
    let named = |name: &Option<String>| {
        name.as_deref()
            .map(|name| format!(" “{name}”"))
            .unwrap_or_default()
    };
    let what = match released.reason {
        ReleaseReason::LeftPlaylist => {
            format!("left the watched playlist{}", named(&released.source_name))
        }
        ReleaseReason::WatchRemoved => match &released.source_name {
            Some(name) => format!("its watch “{name}” was removed"),
            None => "its watch was removed".to_owned(),
        },
        ReleaseReason::PlaylistEdited => {
            format!("taken out of the playlist{}", named(&released.source_name))
        }
        ReleaseReason::Unstarred => "unstarred in an app".to_owned(),
    };
    format!("{what} {}", relative(released.released_at))
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<OrphansQuery>(cx)?;
    let treasury = treasury(cx);
    let orphans = treasury.orphans().await?;
    let ids: Vec<u64> = orphans.iter().map(|track| track.id).collect();
    let released = treasury.released_claims(&ids).await?;
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
    let total_size: u64 = orphans.iter().map(|track| track.size).sum();
    let total = orphans.len();
    let selected = signal(cx, || 0usize);
    let deleted_message = query.deleted.map(|deleted| {
        format!(
            "Deleted {} from disk.",
            count(
                usize::try_from(deleted).unwrap_or(usize::MAX),
                "track",
                "tracks"
            ),
        )
    });
    let summary = format!(
        "{} · {}",
        count(total, "orphan", "orphans"),
        format_bytes(total_size)
    );

    Ok(view! {
        <div class="flex flex-col gap-6">
            page_header(
                eyebrow: "Orphans",
                title: "Nothing keeps these any more",
                lede: "They left a watched playlist, or their watch was removed. píxiū never gives \
                       treasure back on its own: keep them for good, or delete them yourself.",
            )

            if let Some(message) = &deleted_message {
                snackbar(message: message)
            }
            if query.kept.is_some() {
                snackbar(message: "Kept for good. It stays in the hoard.")
            }

            if orphans.is_empty() {
                empty_state(
                    title: "Everything in the hoard is wanted.",
                    <p class="m-0">"Tracks show up here when nothing keeps them any more."</p>
                )
            } else {
                <form id="orphans" method="post" action="/orphans/delete"></form>
                <section class="overflow-hidden rounded-3xl bg-card">
                    <div
                        :class=$(if selected.get() > 0 {
                            "flex items-center gap-3 px-4 py-2.5 bg-slate-container"
                        } else {
                            "flex items-center gap-3 px-4 py-2.5"
                        })
                    >
                        <label class="grid size-10 place-items-center">
                            <input
                                type="checkbox"
                                id="orphans-all"
                                aria-label="Select every orphan"
                                class=(CHECKBOX)
                                :checked=$(selected.get() == total)
                                // Checks or clears every row, then counts them.
                                @change=$(|_e: Event| selected.set(raw!(
                                    "(document.querySelectorAll('[data-orphan]').forEach(c => { c.checked = ${_e}.target.checked; }), document.querySelectorAll('[data-orphan]:checked').length)",
                                    0usize
                                )))
                            >
                        </label>
                        <span class="flex-1 text-sm font-medium">
                            $(if selected.get() == 0 {
                                summary.clone()
                            } else {
                                let n = selected.get();
                                raw!("${n} + ' selected'", format!("{n} selected"))
                            })
                        </span>
                        <button
                            type="button"
                            :hidden=$(selected.get() == 0)
                            onclick=(open_dialog("delete-selected"))
                            class=(btn(Tone::Filled, Size::S))
                        >
                            "Delete selected"
                        </button>
                        <button
                            type="button"
                            :hidden=$(selected.get() > 0)
                            onclick=(open_dialog("delete-all"))
                            class=(btn(Tone::Text, Size::S))
                        >
                            "Delete all"
                        </button>
                    </div>
                    for track in &orphans {
                        let album = albums.get(&track.album_id).map_or("", String::as_str);
                        <div
                            class="flex min-h-16 flex-wrap items-center gap-x-3 gap-y-1 border-t border-muted \
                                   px-3 py-2"
                        >
                            <label class="grid size-10 place-items-center">
                                <input
                                    type="checkbox"
                                    aria-label=(format!("Select {}", track.title))
                                    form="orphans"
                                    name="track"
                                    value=(track.id)
                                    data-orphan=""
                                    class=(CHECKBOX)
                                    @change=$(|_e| selected.set(raw!("document.querySelectorAll('[data-orphan]:checked').length", 0usize)))
                                >
                            </label>
                            <div class="flex min-w-0 flex-[1_1_200px] flex-col gap-0.5">
                                <a
                                    href=(format!("/albums/{}?track={}", track.album_id, track.id))
                                    class="text-[15px] text-foreground hover:text-foreground hover:underline"
                                >
                                    (&track.title)
                                </a>
                                <span class="text-[13px] text-muted-foreground">
                                    (&track.artist_credit) " · " (album) " · "
                                    (format_duration(track.duration_ms)) " · " (format_bytes(track.size))
                                </span>
                                <span class="text-[13px] text-muted-foreground">
                                    "Added " (relative(track.added_at)) "; " (why(released.get(&track.id)))
                                </span>
                            </div>
                            <div class="ml-auto flex gap-1">
                                <button
                                    type="submit"
                                    form="keep-one"
                                    name="track"
                                    value=(track.id)
                                    class=(btn(Tone::Text, Size::S))
                                >
                                    "Keep"
                                </button>
                                <button
                                    type="submit"
                                    form="delete-one-direct"
                                    name="track"
                                    value=(track.id)
                                    onclick=(format!(
                                        "document.getElementById('delete-one-track').value = '{}'; {}; return false",
                                        track.id,
                                        open_dialog("delete-one"),
                                    ))
                                    class=(btn(Tone::Text, Size::S))
                                >
                                    "Delete"
                                </button>
                            </div>
                        </div>
                    }
                </section>
                <form id="keep-one" method="post" action="/orphans/keep"></form>
                // Without JavaScript, "Delete" posts here directly.
                <form id="delete-one-direct" method="post" action="/orphans/delete"></form>

                confirm_dialog(
                    id: "delete-one",
                    headline: "Delete from disk?",
                    action: "/orphans/delete",
                    confirm: "Delete",
                    <input type="hidden" id="delete-one-track" name="track" value="">
                    "This deletes the track for good. Files that are gone can’t be brought back."
                )
                confirm_dialog(
                    id: "delete-selected",
                    headline: "Delete from disk?",
                    action: "/orphans/delete",
                    confirm: "Delete",
                    submits: "orphans",
                    "This deletes the selected tracks for good. Files that are gone can’t be brought back."
                )
                confirm_dialog(
                    id: "delete-all",
                    headline: "Delete from disk?",
                    action: "/orphans/delete",
                    confirm: "Delete all",
                    <input type="hidden" name="all" value="1">
                    "This deletes " (count(total, "orphan", "every orphan")) " ("
                    (format_bytes(total_size)) ") for good. Files that are gone can’t be brought back."
                )
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
