//! `GET /api/events`: server-sent events telling the player what changed,
//! so its hunting screens stay live: `jobs` when the signed-in user's job
//! board changed (the player fetches it again), `session` with their
//! YouTube Music session's state when that changed.

use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::{Stream, stream};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

use crate::{ApiState, Session, sources::state_name};

/// Job progress arrives in bursts; tell the player at most this often.
const JOB_EVENT_GAP: Duration = Duration::from_millis(300);

/// Resolves once the server shuts down; never, if nothing can tell it to.
async fn shutting_down(shutdown: &mut tokio::sync::watch::Receiver<bool>) {
    if shutdown.wait_for(|stopping| *stopping).await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// The next change to one of `owner`'s jobs; a missed burst counts as one.
async fn next_job_change(
    jobs: &mut tokio::sync::broadcast::Receiver<pixiu_jobs::JobUpdate>,
    owner: u64,
) -> Option<()> {
    loop {
        match jobs.recv().await {
            Ok(update) if update.owner() != owner => {}
            Ok(_) | Err(RecvError::Lagged(_)) => return Some(()),
            Err(RecvError::Closed) => return None,
        }
    }
}

pub(crate) async fn stream(
    State(state): State<ApiState>,
    session: Session,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let owner = session.owner();
    let jobs = state.jobs.subscribe();
    let health = state
        .wardens
        .get(owner)
        .await
        .ok()
        .map(|warden| warden.subscribe());
    let shutdown = state.shutdown.clone();
    let events = stream::unfold(
        (jobs, health, shutdown),
        move |(mut jobs, mut health, mut shutdown)| async move {
            let session_changed = async {
                match health.as_mut() {
                    Some(health) => {
                        health.changed().await.ok()?;
                        Some(health.borrow_and_update().state)
                    }
                    None => std::future::pending().await,
                }
            };
            let event = tokio::select! {
                changed = next_job_change(&mut jobs, owner) => {
                    changed?;
                    tokio::time::sleep(JOB_EVENT_GAP).await;
                    while !matches!(jobs.try_recv(), Err(TryRecvError::Empty | TryRecvError::Closed)) {}
                    Event::default().event("jobs").data("changed")
                },
                current = session_changed => {
                    Event::default().event("session").data(state_name(current?))
                },
                () = shutting_down(&mut shutdown) => return None,
            };
            Some((Ok(event), (jobs, health, shutdown)))
        },
    );
    Sse::new(events).keep_alive(KeepAlive::default())
}
