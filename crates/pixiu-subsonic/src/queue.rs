//! `savePlayQueue` and `getPlayQueue`: resume listening on another device.

use pixiu_db::{PlayQueue, User, now, toasty};

use crate::{
    Failure, Params, SubsonicState, catalog,
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
pub(crate) async fn save(
    state: &SubsonicState,
    user: &User,
    params: &Params,
) -> Result<Payload, Failure> {
    let mut db = state.db.clone();
    let entries = params
        .get_all("id")
        .map(track_id)
        .collect::<Result<Vec<_>, _>>()?;
    let current = params.get("current").map(track_id).transpose()?;
    let position_ms = params.number("position", 0_u64)?;
    let changed_by = params.get("c").unwrap_or("unknown client");

    let existing = PlayQueue::filter_by_user_id(user.id)
        .first()
        .exec(&mut db)
        .await?;
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
                user_id: user.id,
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
pub(crate) async fn get(state: &SubsonicState, user: &User) -> Result<Payload, Failure> {
    let mut db = state.db.clone();
    let Some(queue) = PlayQueue::filter_by_user_id(user.id)
        .first()
        .exec(&mut db)
        .await?
    else {
        return Ok(Payload::default());
    };

    // Tracks deleted since the queue was saved are skipped.
    let tracks = catalog::tracks_in_order(&mut db, &queue.entries).await?;
    let entries = catalog::songs(&mut db, "entry", &tracks).await?;
    Ok(Element::new("playQueue")
        .attr_opt("current", queue.current.map(ids::track))
        .attr("position", queue.position_ms)
        .attr("username", user.username.as_str())
        .attr("changed", queue.changed_at.to_string())
        .attr("changedBy", queue.changed_by.as_str())
        .list("entry", entries)
        .into())
}
