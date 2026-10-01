//! `savePlayQueue` and `getPlayQueue`: resume listening on another device.

use pixiu_db::{PlayQueue, now, toasty};

use crate::{
    Cx, Failure, Params, catalog,
    ids::{self, Id},
    response::{ApiError, Element, ErrorCode, Payload},
};

fn track_id(id: &str) -> Result<u64, Failure> {
    match Id::parse(id) {
        Some(Id::Track(id)) => Ok(id),
        _ => Err(ApiError::new(ErrorCode::NotFound, format!("`{id}` is not a song")).into()),
    }
}

/// Saves the queue. Without `id` parameters the saved queue is cleared.
/// Songs not in the caller's library are left out.
pub(crate) async fn save(cx: &Cx<'_>, params: &Params) -> Result<Payload, Failure> {
    let mut db = cx.lib.db();
    let asked = params
        .get_all("id")
        .map(track_id)
        .collect::<Result<Vec<_>, _>>()?;
    let owned = cx.lib.tracks_by_id(&asked).await?;
    let entries: Vec<u64> = asked
        .into_iter()
        .filter(|id| owned.contains_key(id))
        .collect();
    let current = params
        .get("current")
        .map(track_id)
        .transpose()?
        .filter(|id| owned.contains_key(id));
    let position_ms = params.number("position", 0_u64)?;
    let changed_by = params.get("c").unwrap_or("unknown client");

    let existing = cx.lib.play_queue().await?;
    match (existing, entries.is_empty()) {
        (Some(queue), true) => queue.delete().exec(&mut db).await?,
        (None, true) => {}
        (Some(mut queue), false) => {
            toasty::update!(queue {
                entries,
                current,
                position_ms,
                changed_by,
                changed_at: now(),
            })
            .exec(&mut db)
            .await?;
        }
        (None, false) => {
            toasty::create!(PlayQueue {
                user_id: cx.user.id,
                entries,
                current,
                position_ms,
                changed_by,
                changed_at: now(),
            })
            .exec(&mut db)
            .await?;
        }
    }
    Ok(Payload::default())
}

/// Returns the saved queue, or nothing when none is saved.
pub(crate) async fn get(cx: &Cx<'_>) -> Result<Payload, Failure> {
    let Some(queue) = cx.lib.play_queue().await? else {
        return Ok(Payload::default());
    };

    // Tracks deleted since the queue was saved are skipped.
    let tracks = catalog::tracks_in_order(&cx.lib, &queue.entries).await?;
    let entries = catalog::songs(&cx.lib, "entry", &tracks).await?;
    Ok(Element::new("playQueue")
        .attr_opt("current", queue.current.map(ids::track))
        .attr("position", queue.position_ms)
        .attr("username", cx.user.username.as_str())
        .attr("changed", queue.changed_at.to_string())
        .attr("changedBy", queue.changed_by.as_str())
        .list("entry", entries)
        .into())
}
