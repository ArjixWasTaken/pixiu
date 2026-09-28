//! `/jobs`: downloads and other background work, updating live.

mod job_id;

use std::time::Duration;

use pixiu_db::{Job, JobKind, JobState};
use pixiu_jobs::Jobs;
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        page, route,
    },
    runtime::{connected, shard},
    view::{View, class, component, emit, live, view},
};

use crate::{
    auth::{jobs, require_user},
    ui::{BUTTON_SECONDARY, card},
};

pub(super) const JOBS_PATH: &str = "/jobs";

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(view! {
        <div class="mx-auto flex max-w-5xl flex-col gap-8">
            <header class="flex items-end justify-between gap-4">
                <div class="flex flex-col gap-1">
                    <h2 class="text-3xl font-bold text-gold">"Jobs"</h2>
                    <p class="text-muted-foreground">"What píxiū is fetching, and what it fetched."</p>
                </div>
                <form method="post" action="/jobs/clear">
                    <button type="submit" class=(BUTTON_SECONDARY)>"Clear finished"</button>
                </form>
            </header>
            card(job_list())
        </div>
    })
}

/// The job table, re-rendered live as jobs progress.
#[shard]
async fn job_list(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(live! {
        let jobs = jobs(cx);
        let mut updates = jobs.subscribe();
        loop {
            let recent = jobs.recent(100).await?;
            let token = emit! { job_table(jobs: jobs, recent: &recent) }?;
            if !connected(cx) {
                break Ok(token);
            }
            if matches!(updates.recv().await, Err(RecvError::Closed)) {
                break Ok(token);
            }
            // Progress arrives in bursts; render at most a few times a
            // second.
            tokio::time::sleep(Duration::from_millis(300)).await;
            while updates.try_recv().is_ok() {}
        }
    })
}

fn state_label(state: JobState) -> (&'static str, &'static str) {
    match state {
        JobState::Queued => ("Queued", "text-muted-foreground"),
        JobState::Running => ("Running", "text-gold"),
        JobState::Done => ("Done", "text-emerald-400"),
        JobState::Failed => ("Failed", "text-destructive"),
        JobState::Paused => ("Waiting", "text-slate-soft"),
    }
}

#[component]
async fn job_table(jobs: &Jobs, recent: &[Job]) -> Result<impl View> {
    Ok(view! {
        if recent.is_empty() {
            <p class="py-6 text-center text-sm text-muted-foreground">
                "No jobs yet. Grab something on the " <a href="/hunt" class="underline">"Hunt"</a> " page."
            </p>
        } else {
            <table class="w-full text-left text-sm">
                <tbody class="divide-y divide-border">
                    for job in recent {
                        let (label, color) = state_label(job.state);
                        let progress = jobs.progress(job.id).unwrap_or(job.progress);
                        <tr class="align-top">
                            <td class="py-3 pr-4">
                                <p class="font-medium">(&job.title)</p>
                                <p class="text-xs text-muted-foreground">
                                    match job.kind {
                                        JobKind::DownloadTrack => "Download",
                                        JobKind::GrabAlbum => "Album",
                                        JobKind::SyncWatch => "Watch",
                                        JobKind::Enrich => "Lookup",
                                    }
                                    " · " (job.created_at.strftime("%Y-%m-%d %H:%M").to_string())
                                </p>
                                if let Some(error) = &job.error {
                                    <p class="mt-1 text-xs text-destructive">(error)</p>
                                }
                            </td>
                            <td class="w-48 py-3">
                                // Only downloads measure their progress.
                                let measured = job.state == JobState::Running
                                    && job.kind == JobKind::DownloadTrack;
                                <p class=(class!("text-xs font-medium", color))>
                                    (label)
                                    if measured {
                                        " · " (progress) "%"
                                    }
                                </p>
                                if measured {
                                    <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                                        <div
                                            class="h-full rounded-full bg-gold transition-all"
                                            style=(format!("width: {progress}%"))
                                        ></div>
                                    </div>
                                }
                            </td>
                            <td class="w-24 py-3 text-right">
                                if job.state == JobState::Failed {
                                    <form method="post" action=(format!("/jobs/{}/retry", job.id))>
                                        <button type="submit" class=(BUTTON_SECONDARY)>"Retry"</button>
                                    </form>
                                }
                            </td>
                        </tr>
                    }
                </tbody>
            </table>
        }
    })
}

#[route(POST "./clear")]
async fn clear(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    jobs(cx).clear_finished().await?;
    Ok(see_other(JOBS_PATH))
}
