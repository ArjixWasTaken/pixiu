//! `/jobs`: downloads and other background work, updating live.

mod job_id;

use std::{collections::HashSet, time::Duration};

use pixiu_db::{Job, JobKind, JobState};
use pixiu_jobs::{Family, Jobs};
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::Cx,
    icon::icon,
    router::{
        error::{SeeOther, see_other},
        page, route,
    },
    runtime::{connected, shard},
    view::{View, class, component, emit, live, view},
};

use crate::{
    auth::{jobs, require_user},
    ui::{
        EYEBROW, GROUP_TITLE, Size, TRUNCATE, Tone, btn, empty_state, icons, pill, progress,
        relative,
    },
};

pub(super) const JOBS_PATH: &str = "/jobs";

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(view! { job_board() })
}

/// The whole board, re-rendered live as jobs progress.
#[shard]
async fn job_board(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    Ok(live! {
        let jobs = jobs(cx);
        let mut updates = jobs.subscribe();
        loop {
            let recent = jobs.recent(150).await?;
            let parents: Vec<u64> = recent
                .iter()
                .filter(|job| job.kind == JobKind::GrabAlbum)
                .map(|job| job.id)
                .collect();
            let families = jobs.families(&parents).await?;
            let token = emit! { board(jobs: jobs, recent: &recent, families: &families) }?;
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

/// A job's kind, for people.
pub(super) fn kind_label(kind: JobKind) -> &'static str {
    match kind {
        JobKind::DownloadTrack => "Download",
        JobKind::GrabAlbum => "Album",
        JobKind::SyncWatch => "Watch sync",
        JobKind::Enrich => "MusicBrainz lookup",
        JobKind::Refile => "Moving files",
    }
}

/// A job's state, for people.
pub(super) fn state_label(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "Queued",
        JobState::Running => "Running",
        JobState::Done => "Done",
        JobState::Failed => "Failed",
        JobState::Paused => "Waiting for login",
    }
}

/// A job's state as a colored pill's classes.
fn state_pill(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "bg-border text-foreground",
        JobState::Running => "bg-gold-container text-gold",
        JobState::Done => "border border-border text-muted-foreground",
        JobState::Failed => "bg-destructive-container text-destructive-soft",
        JobState::Paused => "bg-slate-container text-slate-pale",
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

#[component]
async fn board(
    jobs: &Jobs,
    recent: &[Job],
    families: &std::collections::HashMap<u64, Family>,
) -> Result<impl View> {
    // Tracks of an album on the board show through their album, unless
    // they failed and need the admin.
    let shown_parents: HashSet<u64> = recent
        .iter()
        .filter(|job| families.contains_key(&job.id))
        .map(|job| job.id)
        .collect();
    let visible: Vec<&Job> = recent
        .iter()
        .filter(|job| {
            job.state == JobState::Failed
                || !job
                    .parent_id
                    .is_some_and(|parent| shown_parents.contains(&parent))
        })
        .collect();
    let in_state = |state: JobState| -> Vec<&Job> {
        visible
            .iter()
            .copied()
            .filter(|job| standing(job, families.get(&job.id)) == state)
            .collect()
    };
    let failed = in_state(JobState::Failed);
    let running = in_state(JobState::Running);
    let mut next = in_state(JobState::Queued);
    next.extend(in_state(JobState::Paused));
    next.sort_by_key(|job| job.id);
    let done = in_state(JobState::Done);

    Ok(view! {
        <div class="flex flex-col gap-7">
            <header class="flex flex-wrap items-end justify-between gap-3">
                <div class="flex flex-col gap-1">
                    <span class=(class!(EYEBROW, "flex items-center gap-2"))>
                        <span class="size-2 rounded-full bg-gold animate-pxpulse"></span>
                        "Live"
                    </span>
                    <h1 class="m-0 text-4xl leading-11 font-normal">"Jobs"</h1>
                    <span class="text-sm text-muted-foreground">
                        (running.len()) " running · " (next.len()) " waiting"
                    </span>
                </div>
                if !done.is_empty() {
                    <form method="post" action="/jobs/clear">
                        <button type="submit" class=(btn(Tone::Outlined, Size::S))>
                            "Clear finished"
                        </button>
                    </form>
                }
            </header>

            if visible.is_empty() {
                empty_state(
                    title: "Nothing going on",
                    <p class="m-0">"Grab something on Hunt and its download shows up here."</p>
                    <a href="/hunt" class=(btn(Tone::Filled, Size::S))>"Go hunt"</a>
                )
            }

            if !failed.is_empty() {
                <section class="flex flex-col gap-2.5">
                    <h2 class=(class!(GROUP_TITLE, "text-destructive"))>"Needs you"</h2>
                    for job in &failed {
                        <div
                            class="flex flex-wrap items-center gap-x-4 gap-y-3 rounded-[20px] \
                                   bg-destructive-container px-[18px] py-4"
                        >
                            <div class="flex min-w-0 flex-[1_1_280px] flex-col gap-1">
                                <span class="text-[15px] font-medium text-foreground">(&job.title)</span>
                                <span class="text-xs text-destructive-soft">
                                    (kind_label(job.kind)) " · queued " (relative(job.created_at))
                                </span>
                                if let Some(error) = &job.error {
                                    <span class="text-sm leading-5 break-words text-destructive-soft">
                                        (error)
                                    </span>
                                }
                            </div>
                            <form method="post" action=(format!("/jobs/{}/retry", job.id))>
                                <button type="submit" class=(btn(Tone::Filled, Size::S))>"Retry"</button>
                            </form>
                        </div>
                    }
                </section>
            }

            if !running.is_empty() {
                <section class="flex flex-col gap-2.5">
                    <h2 class=(GROUP_TITLE)>"Running"</h2>
                    for job in &running {
                        let family = families.get(&job.id);
                        let fraction = match family {
                            Some(family) => Some(family.fraction()),
                            // Only downloads measure their progress.
                            None if job.kind == JobKind::DownloadTrack => Some(
                                f64::from(jobs.progress(job.id).unwrap_or(job.progress)) / 100.0,
                            ),
                            None => None,
                        };
                        <div class="flex flex-col gap-3 rounded-[20px] bg-card px-[18px] py-4">
                            <div class="flex items-start justify-between gap-3">
                                <div class="flex min-w-0 flex-col gap-1">
                                    <span class="text-[15px] font-medium">(&job.title)</span>
                                    <span class="text-xs text-muted-foreground">
                                        (kind_label(job.kind)) " · queued " (relative(job.created_at))
                                    </span>
                                </div>
                                if let Some(fraction) = fraction {
                                    <span class="text-xl font-medium text-gold-soft tabular-nums">
                                        (format!("{:.0}%", fraction * 100.0))
                                    </span>
                                }
                            </div>
                            progress(value: fraction)
                            if let Some(family) = family {
                                <span class="text-[13px] text-muted-foreground">
                                    (family.done) " of " (family.total) " tracks"
                                    if family.failed > 0 {
                                        " · " (family.failed) " failed"
                                    }
                                    if family.waiting > 0 && family.running == 0 {
                                        " · the rest wait their turn"
                                    }
                                </span>
                            }
                        </div>
                    }
                </section>
            }

            if !next.is_empty() {
                <section class="flex flex-col gap-1">
                    <h2 class=(class!(GROUP_TITLE, "mb-1.5"))>"Up next"</h2>
                    for job in &next {
                        <div
                            class="flex min-h-[60px] flex-wrap items-center gap-x-4 gap-y-2 rounded-2xl \
                                   border border-muted px-[18px] py-2"
                        >
                            <div class="flex min-w-0 flex-[1_1_240px] flex-col gap-0.5">
                                <span class="text-[15px]">(&job.title)</span>
                                <span class="text-xs text-muted-foreground">
                                    (kind_label(job.kind)) " · queued " (relative(job.created_at))
                                </span>
                            </div>
                            if job.state == JobState::Paused {
                                <a href="/settings/sources" class="text-[13px]">
                                    (job.error.as_deref().unwrap_or("Paused until you log in"))
                                </a>
                            }
                            pill(
                                attrs: topcoat::view::attributes! { class=(state_pill(job.state)) },
                                (state_label(job.state))
                            )
                        </div>
                    }
                </section>
            }

            if !done.is_empty() {
                <section class="flex flex-col">
                    <h2 class=(class!(GROUP_TITLE, "mb-1.5"))>"Finished"</h2>
                    for job in &done {
                        <div
                            class="flex min-h-[52px] items-center gap-4 border-b border-muted px-[18px] \
                                   py-1.5 text-muted-foreground"
                        >
                            <span class="inline-flex text-slate-soft">icon(data: icons::CHECK, size: 18)</span>
                            <span class=(class!(TRUNCATE, "flex-1 text-sm text-foreground-soft"))>
                                (&job.title)
                            </span>
                            <span class="text-xs whitespace-nowrap">
                                (kind_label(job.kind))
                                if let Some(family) = families.get(&job.id) {
                                    " · " (family.total) " tracks"
                                }
                                " · " (relative(job.finished_at.unwrap_or(job.created_at)))
                            </span>
                        </div>
                    }
                </section>
            }
        </div>
    })
}

#[route(POST "./clear")]
async fn clear(cx: &Cx) -> Result<SeeOther> {
    require_user(cx).await?;
    jobs(cx).clear_finished().await?;
    Ok(see_other(JOBS_PATH))
}
