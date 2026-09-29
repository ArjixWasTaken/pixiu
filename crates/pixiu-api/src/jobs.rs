//! The job board: downloads, album grabs, watch syncs, lookups and file
//! moves, as they queue, run, finish or fail. `GET /api/events` says when
//! the board changed.

use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pixiu_db::{Job, JobKind, JobState};
use pixiu_jobs::Family;
use serde_json::{Value as JsonValue, json};

use crate::{ApiResult, ApiState, Session};

/// How many of the latest jobs the board shows.
const BOARD_SIZE: usize = 150;

pub(crate) fn kind_name(kind: JobKind) -> &'static str {
    match kind {
        JobKind::DownloadTrack => "download",
        JobKind::GrabAlbum => "album",
        JobKind::SyncWatch => "watch_sync",
        JobKind::Enrich => "lookup",
        JobKind::Refile => "refile",
    }
}

fn state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Done => "done",
        JobState::Failed => "failed",
        JobState::Paused => "paused",
    }
}

/// Where a job stands on the board. An album grab stands with its tracks:
/// running while any of them has yet to finish.
fn standing(job: &Job, family: Option<&Family>) -> JobState {
    match family {
        Some(family) if job.state == JobState::Done && family.in_flight() => JobState::Running,
        _ => job.state,
    }
}

/// `GET /api/jobs`: the latest jobs. Tracks of an album on the board show
/// through their album, unless they failed and need the admin.
pub(crate) async fn board(State(state): State<ApiState>, _: Session) -> ApiResult<Json<JsonValue>> {
    let recent = state.jobs.recent(BOARD_SIZE).await?;
    let parents: Vec<u64> = recent
        .iter()
        .filter(|job| job.kind == JobKind::GrabAlbum)
        .map(|job| job.id)
        .collect();
    let families: HashMap<u64, Family> = state.jobs.families(&parents).await?;
    let shown_parents: HashSet<u64> = recent
        .iter()
        .filter(|job| families.contains_key(&job.id))
        .map(|job| job.id)
        .collect();

    let jobs: Vec<JsonValue> = recent
        .iter()
        .filter(|job| {
            job.state == JobState::Failed
                || !job
                    .parent_id
                    .is_some_and(|parent| shown_parents.contains(&parent))
        })
        .map(|job| {
            let family = families.get(&job.id);
            let standing = standing(job, family);
            // Downloads report their own progress; an album, its tracks'.
            let progress = match family {
                Some(family) => Some(family.fraction()),
                None if job.kind == JobKind::DownloadTrack && standing == JobState::Running => {
                    Some(f64::from(state.jobs.progress(job.id).unwrap_or(job.progress)) / 100.0)
                }
                None => None,
            };
            json!({
                "id": job.id,
                "kind": kind_name(job.kind),
                "state": state_name(standing),
                "title": job.title,
                "error": job.error,
                "progress": progress,
                "family": family.map(|family| json!({
                    "total": family.total,
                    "done": family.done,
                    "failed": family.failed,
                    "running": family.running,
                    "waiting": family.waiting,
                })),
                "created_at": job.created_at,
                "finished_at": job.finished_at,
            })
        })
        .collect();
    Ok(Json(json!(jobs)))
}

/// `POST /api/jobs/{id}/retry`.
pub(crate) async fn retry(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    state.jobs.retry(id).await?;
    Ok(StatusCode::ACCEPTED)
}

/// `DELETE /api/jobs/finished`: forgets the finished jobs.
pub(crate) async fn clear_finished(
    State(state): State<ApiState>,
    _: Session,
) -> ApiResult<StatusCode> {
    state.jobs.clear_finished().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/hunting`: what the sidebar shows about hunting.
pub(crate) async fn summary(
    State(state): State<ApiState>,
    _: Session,
) -> ApiResult<Json<JsonValue>> {
    Ok(Json(summarize(&state).await?))
}

/// The session's state, how many orphans and offering batches await the
/// admin, and how the jobs are doing.
pub(crate) async fn summarize(state: &ApiState) -> ApiResult<JsonValue> {
    let (mut running, mut waiting, mut failed) = (0, 0, 0);
    for job in state.jobs.unfinished().await? {
        match job.state {
            JobState::Running => running += 1,
            JobState::Failed => failed += 1,
            JobState::Queued | JobState::Paused => waiting += 1,
            JobState::Done => {}
        }
    }
    let batches: HashSet<String> = state
        .offerings
        .pending()
        .await?
        .into_iter()
        .map(|offering| offering.batch)
        .collect();
    Ok(json!({
        "session": crate::sources::state_name(state.warden.health().state),
        "orphans": state.treasury.orphan_count().await?,
        "offerings": batches.len(),
        "jobs": { "running": running, "waiting": waiting, "failed": failed },
    }))
}
