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

/// The user every test library and job belongs to.
const OWNER: u64 = 1;

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

/// The queue runs jobs of active accounts; these are the tests' owners.
async fn accounts(db: &pixiu_db::Db) {
    for name in ["owner", "other"] {
        toasty::create!(pixiu_db::User {
            username: name,
            role: pixiu_db::Role::User,
            status: pixiu_db::UserStatus::Active,
            password_change_required: false,
            password_hash: "x",
            created_at: now(),
        })
        .exec(&mut db.clone())
        .await
        .unwrap();
    }
}

async fn wait_for(jobs: &Jobs, done: impl Fn(&[Job]) -> bool) -> Vec<Job> {
    for _ in 0..200 {
        let recent = jobs.recent(OWNER, 50).await.unwrap();
        if done(&recent) {
            return recent;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!(
        "jobs did not settle: {:#?}",
        jobs.recent(OWNER, 50).await.unwrap()
    );
}

fn settled(jobs: &[Job]) -> bool {
    jobs.iter()
        .all(|job| matches!(job.state, JobState::Done | JobState::Failed))
}

#[tokio::test]
async fn jobs_run_expand_fail_and_retry() {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    accounts(&db).await;
    let executor = Arc::new(FakeExecutor::default());
    *executor.failures_left.lock().unwrap() = 1;
    let jobs = Jobs::new(db, Box::new(Shared(Arc::clone(&executor))));
    let mut updates = jobs.subscribe();
    jobs.start();

    jobs.enqueue(OWNER, NewJob::track("ok", "Good track", None))
        .await
        .unwrap();
    jobs.enqueue(OWNER, NewJob::track("known", "Hoarded track", None))
        .await
        .unwrap();
    jobs.enqueue(OWNER, NewJob::track("flaky", "Flaky track", None))
        .await
        .unwrap();
    jobs.enqueue(OWNER, NewJob::album("album", "An album"))
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
    assert!(seen.iter().any(|update| matches!(
        update,
        JobUpdate::Changed {
            owner: OWNER,
            progress: 50,
            ..
        }
    )));
    assert!(seen.contains(&JobUpdate::Changed {
        owner: OWNER,
        id: flaky.id,
        state: JobState::Failed,
        progress: 0,
    }));

    jobs.retry(OWNER, flaky.id).await.unwrap();
    let all = wait_for(&jobs, |jobs| {
        jobs.iter()
            .any(|job| job.title == "Flaky track" && job.state == JobState::Done)
    })
    .await;
    let flaky = all.iter().find(|job| job.title == "Flaky track").unwrap();
    assert_eq!(flaky.track_id, Some(7));
    assert_eq!(flaky.attempts, 2);

    let mut updates = jobs.subscribe();
    jobs.clear_finished(OWNER).await.unwrap();
    assert_eq!(
        updates.try_recv().unwrap(),
        JobUpdate::Cleared { owner: OWNER }
    );
    assert!(jobs.recent(OWNER, 50).await.unwrap().is_empty());
}

#[tokio::test]
async fn album_grabs_stay_whole_until_their_tracks_finish() {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    accounts(&db).await;
    let jobs = Jobs::new(
        db.clone(),
        Box::new(Shared(Arc::new(FakeExecutor::default()))),
    );
    jobs.start();
    let album = jobs
        .enqueue(OWNER, NewJob::album("stuck", "Stuck album"))
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
    let pending = pixiu_jobs::pending(&mut db.clone(), OWNER).await.unwrap();
    assert!(pending.has_album("stuck"));
    assert!(pending.tracks.contains("hold"));

    // Clearing keeps the family while a track waits.
    jobs.clear_finished(OWNER).await.unwrap();
    assert_eq!(jobs.recent(OWNER, 50).await.unwrap().len(), 3);
}

#[tokio::test]
async fn interrupted_jobs_run_again_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    accounts(&db).await;
    // A job that was running when píxiū stopped, saved before payloads
    // said who wanted them.
    let payload = r#"{"video_id":"ok","reference":null}"#;
    assert_eq!(
        serde_json::from_str::<TrackJob>(payload).unwrap().wanted,
        pixiu_jobs::Wanted::Grab
    );
    toasty::create!(Job {
        user_id: OWNER,
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

/// Users take turns: one user's long queue does not hold another's up.
#[tokio::test]
async fn users_take_turns() {
    const OTHER: u64 = 2;
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    accounts(&db).await;
    let executor = Arc::new(FakeExecutor::default());
    let jobs = Jobs::new(db.clone(), Box::new(Shared(Arc::clone(&executor))));
    for n in 1..=3 {
        jobs.enqueue(
            OWNER,
            NewJob::track(&format!("a{n}"), &format!("A{n}"), None),
        )
        .await
        .unwrap();
    }
    jobs.enqueue(OTHER, NewJob::track("b1", "B1", None))
        .await
        .unwrap();
    let mut updates = jobs.subscribe();
    jobs.start();

    wait_for(&jobs, |jobs| jobs.len() == 3 && settled(jobs)).await;
    let mut started: Vec<Job> = Job::all().exec(&mut db.clone()).await.unwrap();
    started.sort_by_key(|job| job.started_at);
    let order: Vec<&str> = started.iter().map(|job| job.title.as_str()).collect();
    assert_eq!(order, ["A1", "B1", "A2", "A3"]);
    // Each job's news names its owner.
    while let Ok(update) = updates.try_recv() {
        if let JobUpdate::Changed { owner, id, .. } = update {
            let job = started.iter().find(|job| job.id == id).unwrap();
            assert_eq!(owner, job.user_id);
        }
    }
    assert_eq!(jobs.recent(OTHER, 50).await.unwrap().len(), 1);
}
