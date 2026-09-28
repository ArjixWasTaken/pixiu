//! The job queue, with a fake executor.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use pixiu_db::{Job, JobKind, JobState, now, toasty};
use pixiu_jobs::{
    Executor, JobUpdate, Jobs, NewJob, Outcome,
    queue::{AlbumJob, TrackJob},
    warden::BoxFuture,
};

/// Succeeds, fails or expands depending on the video id, and records
/// what ran.
#[derive(Default)]
struct FakeExecutor {
    ran: Mutex<Vec<String>>,
    failures_left: Mutex<u32>,
}

/// Shares the fake with the test, which inspects it afterwards.
struct Shared(Arc<FakeExecutor>);

impl Executor for Shared {
    fn run<'a>(
        &'a self,
        job: &'a Job,
        progress: &'a (dyn Fn(u8) + Send + Sync),
    ) -> BoxFuture<'a, Outcome> {
        let fake = &self.0;
        Box::pin(async move {
            fake.ran.lock().unwrap().push(job.title.clone());
            progress(50);
            if job.kind == JobKind::GrabAlbum {
                let payload: AlbumJob = serde_json::from_str(&job.payload).unwrap();
                let album = payload.browse_id;
                let second = if album == "stuck" { "hold" } else { "a2" };
                return Outcome::Expand(vec![
                    NewJob::track("a1", "Album track 1", Some(album.clone())),
                    NewJob::track(second, "Album track 2", Some(album)),
                ]);
            }
            let payload: TrackJob = serde_json::from_str(&job.payload).unwrap();
            match payload.video_id.as_str() {
                "flaky" => {
                    let mut left = fake.failures_left.lock().unwrap();
                    if *left > 0 {
                        *left -= 1;
                        Outcome::Failed("network down".into())
                    } else {
                        Outcome::Done { track_id: Some(7) }
                    }
                }
                "known" => Outcome::AlreadyDone { track_id: 3 },
                "hold" => Outcome::Paused("waiting for a login".into()),
                _ => Outcome::Done { track_id: Some(1) },
            }
        })
    }
}

async fn wait_for(jobs: &Jobs, done: impl Fn(&[Job]) -> bool) -> Vec<Job> {
    for _ in 0..200 {
        let recent = jobs.recent(50).await.unwrap();
        if done(&recent) {
            return recent;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("jobs did not settle: {:#?}", jobs.recent(50).await.unwrap());
}

fn settled(jobs: &[Job]) -> bool {
    jobs.iter()
        .all(|job| matches!(job.state, JobState::Done | JobState::Failed))
}

#[tokio::test]
async fn jobs_run_expand_fail_and_retry() {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let executor = Arc::new(FakeExecutor::default());
    *executor.failures_left.lock().unwrap() = 1;
    let jobs = Jobs::new(db, Box::new(Shared(Arc::clone(&executor))));
    let mut updates = jobs.subscribe();
    jobs.start();

    jobs.enqueue(NewJob::track("ok", "Good track", None))
        .await
        .unwrap();
    jobs.enqueue(NewJob::track("known", "Hoarded track", None))
        .await
        .unwrap();
    jobs.enqueue(NewJob::track("flaky", "Flaky track", None))
        .await
        .unwrap();
    jobs.enqueue(NewJob::album("album", "An album"))
        .await
        .unwrap();

    let all = wait_for(&jobs, |jobs| jobs.len() == 6 && settled(jobs)).await;
    let by_title = |title: &str| all.iter().find(|job| job.title == title).unwrap();
    assert_eq!(by_title("Good track").state, JobState::Done);
    assert_eq!(by_title("Good track").track_id, Some(1));
    assert_eq!(by_title("Good track").progress, 100);
    assert_eq!(by_title("Hoarded track").track_id, Some(3));
    assert_eq!(by_title("An album").state, JobState::Done);
    assert_eq!(by_title("Album track 1").state, JobState::Done);
    // The album's tracks are its family.
    let album = by_title("An album").id;
    assert_eq!(by_title("Album track 1").parent_id, Some(album));
    assert_eq!(by_title("Album track 2").parent_id, Some(album));
    assert_eq!(by_title("Good track").parent_id, None);
    let family = jobs.families(&[album]).await.unwrap()[&album];
    assert_eq!((family.total, family.done), (2, 2));
    assert!(!family.in_flight());
    assert!((family.fraction() - 1.0).abs() < f64::EPSILON);
    let flaky = by_title("Flaky track");
    assert_eq!(flaky.state, JobState::Failed);
    assert_eq!(flaky.error.as_deref(), Some("network down"));
    assert_eq!(flaky.attempts, 1);

    // Progress and final states were broadcast.
    let mut seen = Vec::new();
    while let Ok(update) = updates.try_recv() {
        seen.push(update);
    }
    assert!(
        seen.iter()
            .any(|update| matches!(update, JobUpdate::Changed { progress: 50, .. }))
    );
    assert!(seen.contains(&JobUpdate::Changed {
        id: flaky.id,
        state: JobState::Failed,
        progress: 0,
    }));

    jobs.retry(flaky.id).await.unwrap();
    let all = wait_for(&jobs, |jobs| {
        jobs.iter()
            .any(|job| job.title == "Flaky track" && job.state == JobState::Done)
    })
    .await;
    let flaky = all.iter().find(|job| job.title == "Flaky track").unwrap();
    assert_eq!(flaky.track_id, Some(7));
    assert_eq!(flaky.attempts, 2);

    let mut updates = jobs.subscribe();
    jobs.clear_finished().await.unwrap();
    assert_eq!(updates.try_recv().unwrap(), JobUpdate::Cleared);
    assert!(jobs.recent(50).await.unwrap().is_empty());
}

#[tokio::test]
async fn album_grabs_stay_whole_until_their_tracks_finish() {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let jobs = Jobs::new(
        db.clone(),
        Box::new(Shared(Arc::new(FakeExecutor::default()))),
    );
    jobs.start();
    let album = jobs
        .enqueue(NewJob::album("stuck", "Stuck album"))
        .await
        .unwrap();

    let all = wait_for(&jobs, |jobs| {
        jobs.len() == 3
            && jobs
                .iter()
                .any(|job| job.title == "Album track 2" && job.state == JobState::Paused)
            && jobs
                .iter()
                .any(|job| job.title == "Album track 1" && job.state == JobState::Done)
    })
    .await;
    assert!(
        all.iter()
            .all(|job| job.id == album.id || job.parent_id == Some(album.id))
    );
    let family = jobs.families(&[album.id]).await.unwrap()[&album.id];
    assert_eq!((family.total, family.done, family.waiting), (2, 1, 1));
    assert!(family.in_flight());
    assert!((family.fraction() - 0.5).abs() < f64::EPSILON);

    // The album is still being grabbed, through its tracks.
    let pending = pixiu_jobs::pending(&mut db.clone()).await.unwrap();
    assert!(pending.has_album("stuck"));
    assert!(pending.tracks.contains("hold"));

    // Clearing keeps the family while a track waits.
    jobs.clear_finished().await.unwrap();
    assert_eq!(jobs.recent(50).await.unwrap().len(), 3);
}

#[tokio::test]
async fn interrupted_jobs_run_again_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    // A job that was running when píxiū stopped, saved before payloads
    // said who wanted them.
    let payload = r#"{"video_id":"ok","reference":null}"#;
    assert_eq!(
        serde_json::from_str::<TrackJob>(payload).unwrap().wanted,
        pixiu_jobs::Wanted::Grab
    );
    toasty::create!(Job {
        kind: JobKind::DownloadTrack,
        payload,
        title: "Interrupted",
        state: JobState::Running,
        progress: 40_u8,
        attempts: 1_u32,
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();

    let executor = Arc::new(FakeExecutor::default());
    let jobs = Jobs::new(db, Box::new(Shared(Arc::clone(&executor))));
    jobs.start();
    let all = wait_for(&jobs, settled).await;
    assert_eq!(all[0].state, JobState::Done);
    assert_eq!(executor.ran.lock().unwrap().as_slice(), ["Interrupted"]);
}
