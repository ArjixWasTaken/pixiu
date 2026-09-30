//! Orphans: tracks of the signed-in user's that nothing keeps any more.
//! píxiū never deletes them on its own; the user keeps them for good or
//! deletes them.

use axum::{Json, extract::State};
use pixiu_db::{ClaimKind, ReleaseReason, ReleasedClaim};
use pixiu_subsonic::{ids, render};
use pixiu_treasury::Claim;
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session};

/// Why nothing keeps a track any more, for people.
fn why(released: Option<&ReleasedClaim>) -> String {
    let Some(released) = released else {
        return "Nothing keeps it".to_owned();
    };
    let named = |name: &Option<String>| {
        name.as_deref()
            .map(|name| format!(" “{name}”"))
            .unwrap_or_default()
    };
    match released.reason {
        ReleaseReason::LeftPlaylist => {
            format!("Left the watched playlist{}", named(&released.source_name))
        }
        ReleaseReason::WatchRemoved => match &released.source_name {
            Some(name) => format!("Its watch “{name}” was removed"),
            None => "Its watch was removed".to_owned(),
        },
        ReleaseReason::PlaylistEdited => {
            format!("Taken out of the playlist{}", named(&released.source_name))
        }
        ReleaseReason::Unstarred => "Unstarred in an app".to_owned(),
        ReleaseReason::Excluded => format!(
            "Excluded from the watched playlist{}",
            named(&released.source_name)
        ),
    }
}

/// `GET /api/orphans`: each orphan as a Subsonic song with why it became
/// one, newest orphan first.
pub(crate) async fn list(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let orphans = state.treasury.orphans(session.owner()).await?;
    let ids: Vec<u64> = orphans.iter().map(|track| track.id).collect();
    let released = state.treasury.released_claims(&ids).await?;
    let songs = render::songs(&session.library(&state), &ids).await?;
    let mut items: Vec<(Option<jiff::Timestamp>, JsonValue)> = orphans
        .iter()
        .zip(songs)
        .map(|(track, song)| {
            let claim = released.get(&track.id);
            let at = claim.map(|claim| claim.released_at);
            (
                at,
                json!({ "song": song, "reason": why(claim), "released_at": at, "size": track.size }),
            )
        })
        .collect();
    items.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    Ok(Json(json!({
        "orphans": items.into_iter().map(|(_, item)| item).collect::<Vec<_>>(),
        "total_size": orphans.iter().map(|track| track.size).sum::<u64>(),
    })))
}

#[derive(Deserialize)]
pub(crate) struct Songs {
    /// Subsonic song ids (`tr-…`).
    #[serde(default)]
    songs: Vec<String>,
    /// Every orphan, whatever `songs` says.
    #[serde(default)]
    all: bool,
}

fn track_ids(songs: &[String]) -> ApiResult<Vec<u64>> {
    songs
        .iter()
        .map(|id| match ids::Id::parse(id) {
            Some(ids::Id::Track(id)) => Ok(id),
            _ => Err(ApiError::unprocessable(format!("`{id}` is not a song"))),
        })
        .collect()
}

/// `POST /api/orphans/keep`: keeps the songs for good.
pub(crate) async fn keep(
    State(state): State<ApiState>,
    session: Session,
    Json(form): Json<Songs>,
) -> ApiResult<Json<JsonValue>> {
    let keep = Claim {
        kind: ClaimKind::ManualGrab,
        reference: None,
    };
    let lib = session.library(&state);
    let mut kept = 0;
    for id in track_ids(&form.songs)? {
        if lib.track(id).await?.is_some() {
            state.treasury.claim(id, &keep).await?;
            kept += 1;
        }
    }
    Ok(Json(json!({ "kept": kept })))
}

/// `POST /api/orphans/delete`: deletes orphans from disk. Songs something
/// keeps are left alone.
pub(crate) async fn delete(
    State(state): State<ApiState>,
    session: Session,
    Json(form): Json<Songs>,
) -> ApiResult<Json<JsonValue>> {
    let owner = session.owner();
    let ids = if form.all {
        state
            .treasury
            .orphans(owner)
            .await?
            .iter()
            .map(|track| track.id)
            .collect()
    } else {
        track_ids(&form.songs)?
    };
    let deleted = state.treasury.delete_orphans(owner, &ids).await?;
    Ok(Json(json!({ "deleted": deleted })))
}
