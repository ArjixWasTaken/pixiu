//! `/playing`: what Subsonic apps are playing, live.

use std::{collections::HashMap, time::Duration};

use pixiu_db::{Album, Track};
use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{connected, shard},
    view::{View, class, emit, live, view},
};

use crate::{
    auth::{db, now_playing, require_user},
    ui::{TRUNCATE, cover, empty_state, format_duration, page_header},
};

/// A song an app plays, ready to show.
pub(super) struct Entry {
    /// The album, when it has a cover.
    pub cover: Option<u64>,
    pub album_id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// The app (the Subsonic `c` parameter).
    pub player: String,
    pub username: String,
    pub position: Duration,
    pub length: Duration,
}

/// What plays now, the latest first.
pub(super) async fn now(cx: &Cx) -> Result<Vec<Entry>> {
    let current = now_playing(cx).current();
    if current.is_empty() {
        return Ok(Vec::new());
    }
    let mut db = db(cx);
    let ids: Vec<u64> = current.iter().map(|playing| playing.song.id).collect();
    let tracks: HashMap<u64, Track> = Track::filter(Track::fields().id().in_list(ids))
        .exec(&mut db)
        .await?
        .into_iter()
        .map(|track| (track.id, track))
        .collect();
    let album_ids: Vec<u64> = tracks.values().map(|track| track.album_id).collect();
    let albums: HashMap<u64, Album> = Album::filter(Album::fields().id().in_list(album_ids))
        .exec(&mut db)
        .await?
        .into_iter()
        .map(|album| (album.id, album))
        .collect();
    Ok(current
        .into_iter()
        .filter_map(|playing| {
            let track = tracks.get(&playing.song.id)?;
            let album = albums.get(&track.album_id)?;
            Some(Entry {
                cover: album.cover.as_ref().map(|_| album.id),
                album_id: album.id,
                title: track.title.clone(),
                artist: track.artist_credit.clone(),
                album: album.title.clone(),
                position: playing.position(),
                length: playing.song.length,
                player: playing.player,
                username: playing.username,
            })
        })
        .collect())
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(view! {
        <div class="flex flex-col gap-6">
            page_header(eyebrow: "Now playing", title: "What your apps are playing")
            playing_list()
        </div>
    })
}

/// The cards, live: re-rendered when an app starts a song, when one ends,
/// and now and then so the times move.
#[shard]
async fn playing_list(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(live! {
        let mut changes = now_playing(cx).subscribe();
        loop {
            let entries = now(cx).await?;
            let token = emit! { playing_cards(entries: entries) }?;
            if !connected(cx) {
                break Ok(token);
            }
            tokio::select! {
                changed = changes.changed() => {
                    if changed.is_err() {
                        break Ok(token);
                    }
                }
                () = tokio::time::sleep(Duration::from_secs(10)) => {}
            }
        }
    })
}

#[topcoat::view::component]
async fn playing_cards(entries: Vec<Entry>) -> Result<impl View> {
    Ok(view! {
        if entries.is_empty() {
            empty_state("Nothing is playing. Open a Subsonic app and press play.")
        } else {
            <div class="grid grid-cols-[repeat(auto-fill,minmax(min(100%,420px),1fr))] gap-4">
                for entry in &entries {
                    let fraction = if entry.length.is_zero() {
                        0.0
                    } else {
                        entry.position.as_secs_f64() / entry.length.as_secs_f64()
                    };
                    let remaining = entry.length.saturating_sub(entry.position);
                    <div class="flex items-center gap-[18px] rounded-3xl bg-card p-[18px]">
                        <a href=(format!("/albums/{}", entry.album_id)) class="w-28 shrink-0">
                            cover(album: entry.cover, size: 300)
                        </a>
                        <div class="flex min-w-0 flex-1 flex-col gap-1.5">
                            <span class="text-xs font-medium tracking-[.04em] text-gold">
                                "Playing · " (&entry.player) " · " (&entry.username)
                            </span>
                            <span class=(class!(TRUNCATE, "text-[19px]"))>(&entry.title)</span>
                            <span class=(class!(TRUNCATE, "text-[13px] text-muted-foreground"))>
                                (&entry.artist) " · " (&entry.album)
                            </span>
                            // The bar runs on by itself until the song should end;
                            // apps do not report pauses.
                            <div class="relative mt-1.5 h-1 w-full overflow-hidden rounded-full bg-border">
                                <div
                                    class="h-full w-full origin-left rounded-full bg-gold animate-pxgrow"
                                    style=(format!(
                                        "--from:{fraction:.4};--run:{:.1}s",
                                        remaining.as_secs_f64(),
                                    ))
                                ></div>
                            </div>
                            <div
                                class="flex justify-between text-xs text-muted-foreground tabular-nums"
                            >
                                <span>(format_duration(millis(entry.position)))</span>
                                <span>(format_duration(millis(entry.length)))</span>
                            </div>
                        </div>
                    </div>
                }
            </div>
        }
    })
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
