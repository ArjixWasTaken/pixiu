//! `GET /api/events`: server-sent events telling the player what changed,
//! so its hunting screens stay live: `jobs` when the job board changed (the
//! player fetches it again), `session` with the YouTube Music session's
//! state when that changed.

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

pub(crate) async fn stream(
    State(state): State<ApiState>,
    _: Session,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let jobs = state.jobs.subscribe();
    let health = state.warden.subscribe();
    let events = stream::unfold((jobs, health), |(mut jobs, mut health)| async move {
        let event = tokio::select! {
            update = jobs.recv() => match update {
                Ok(_) | Err(RecvError::Lagged(_)) => {
                    tokio::time::sleep(JOB_EVENT_GAP).await;
                    while !matches!(jobs.try_recv(), Err(TryRecvError::Empty | TryRecvError::Closed)) {}
                    Event::default().event("jobs").data("changed")
                }
                Err(RecvError::Closed) => return None,
            },
            changed = health.changed() => {
                changed.ok()?;
                let current = health.borrow_and_update().state;
                Event::default().event("session").data(state_name(current))
            },
        };
        Some((Ok(event), (jobs, health)))
    });
    Sse::new(events).keep_alive(KeepAlive::default())
}
