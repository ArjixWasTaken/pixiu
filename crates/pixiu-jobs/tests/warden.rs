//! The warden's state machine, with a scripted platform and browser.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use pixiu_core::{
    SecretBox,
    alerts::{Alert, AlertSink, NoAlerts},
};
use pixiu_db::Platform;
use pixiu_db::{Db, SessionEventKind, SessionState};
use pixiu_hunt::SessionCheck;
use pixiu_jobs::warden::{BoxFuture, Refresher, Session, Warden};

/// The user every test library and job belongs to.
const OWNER: u64 = 1;

#[derive(Default)]
struct Script {
    applies: Mutex<VecDeque<SessionCheck>>,
    checks: Mutex<VecDeque<SessionCheck>>,
    refreshes: Mutex<VecDeque<Result<String, String>>>,
    applied: Mutex<Vec<String>>,
}

impl Script {
    fn apply_answers(&self, answers: impl IntoIterator<Item = SessionCheck>) {
        self.applies.lock().unwrap().extend(answers);
    }

    fn check_answers(&self, answers: impl IntoIterator<Item = SessionCheck>) {
        self.checks.lock().unwrap().extend(answers);
    }

    fn refresh_answers(&self, answers: impl IntoIterator<Item = Result<String, String>>) {
        self.refreshes.lock().unwrap().extend(answers);
    }
}

struct FakeSession(Arc<Script>);

impl Session for FakeSession {
    fn apply<'a>(&'a self, cookies: &'a str) -> BoxFuture<'a, SessionCheck> {
        self.0.applied.lock().unwrap().push(cookies.to_owned());
        let answer = self.0.applies.lock().unwrap().pop_front();
        Box::pin(async move { answer.expect("a scripted apply answer") })
    }

    fn check(&self) -> BoxFuture<'_, SessionCheck> {
        let answer = self.0.checks.lock().unwrap().pop_front();
        Box::pin(async move { answer.expect("a scripted check answer") })
    }

    fn forget(&self) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

struct FakeRefresher(Arc<Script>);

impl Refresher for FakeRefresher {
    fn refresh(&self) -> BoxFuture<'_, Result<String, String>> {
        let answer = self.0.refreshes.lock().unwrap().pop_front();
        Box::pin(async move { answer.expect("a scripted refresh answer") })
    }
}

/// Keeps the alerts raised.
#[derive(Default)]
struct Alerts(Mutex<Vec<(u64, Alert)>>);

impl Alerts {
    fn raised(&self) -> Vec<(u64, Alert)> {
        self.0.lock().unwrap().clone()
    }
}

impl AlertSink for Alerts {
    fn alert(&self, user: u64, alert: Alert) -> BoxFuture<'_, ()> {
        self.0.lock().unwrap().push((user, alert));
        Box::pin(async {})
    }
}

struct Setup {
    _dir: tempfile::TempDir,
    db: Db,
    secrets: SecretBox,
    script: Arc<Script>,
    alerts: Arc<Alerts>,
}

impl Setup {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        account(&db).await;
        Self {
            _dir: dir,
            db,
            secrets: SecretBox::ephemeral(),
            script: Arc::default(),
            alerts: Arc::default(),
        }
    }

    async fn warden(&self) -> Arc<Warden> {
        Warden::new(
            self.db.clone(),
            self.secrets.clone(),
            OWNER,
            Platform::YouTubeMusic,
            Box::new(FakeSession(Arc::clone(&self.script))),
            Box::new(FakeRefresher(Arc::clone(&self.script))),
            Arc::clone(&self.alerts) as Arc<dyn AlertSink>,
        )
        .await
        .unwrap()
    }
}

async fn event_log(warden: &Warden) -> Vec<String> {
    let mut messages: Vec<String> = warden
        .events(10)
        .await
        .unwrap()
        .into_iter()
        .map(|event| event.message)
        .collect();
    messages.reverse();
    messages
}

#[tokio::test]
async fn connecting_needs_cookies_the_platform_accepts() {
    let setup = Setup::new().await;
    let warden = setup.warden().await;
    assert_eq!(warden.health().state, None);

    setup
        .script
        .apply_answers([SessionCheck::Invalid("not logged in".into())]);
    assert_eq!(
        warden.connect("bad".into()).await.unwrap_err(),
        "not logged in"
    );
    assert_eq!(warden.health().state, None);

    setup.script.apply_answers([SessionCheck::Valid]);
    let health = warden.connect("SID=1".into()).await.unwrap();
    assert_eq!(health.state, Some(SessionState::Valid));
    assert!(health.last_verified.is_some());
    assert_eq!(warden.cookies().await.as_deref(), Some("SID=1"));
    assert_eq!(event_log(&warden).await, ["Connected"]);
}

#[tokio::test]
async fn transient_failures_degrade_then_recover() {
    let setup = Setup::new().await;
    let warden = setup.warden().await;
    let mut watcher = warden.subscribe();
    setup.script.apply_answers([SessionCheck::Valid]);
    warden.connect("SID=1".into()).await.unwrap();

    setup.script.check_answers([
        SessionCheck::Unreachable("timeout".into()),
        SessionCheck::Valid,
    ]);
    let health = warden.validate().await;
    assert_eq!(health.state, Some(SessionState::Degraded));
    assert_eq!(health.last_error.as_deref(), Some("timeout"));
    assert!(
        watcher.has_changed().unwrap(),
        "the web player hears about it"
    );
    watcher.borrow_and_update();

    let health = warden.validate().await;
    assert_eq!(health.state, Some(SessionState::Valid));
    assert_eq!(health.last_error, None);
    assert_eq!(
        event_log(&warden).await,
        [
            "Connected",
            "Check failed: timeout",
            "Session working again"
        ]
    );
    let mut kinds: Vec<SessionEventKind> = warden
        .events(10)
        .await
        .unwrap()
        .into_iter()
        .map(|event| event.kind)
        .collect();
    kinds.reverse();
    assert_eq!(
        kinds,
        [
            SessionEventKind::Connected,
            SessionEventKind::Degraded,
            SessionEventKind::Recovered
        ]
    );

    // Failing again while degraded is not news.
    setup.script.check_answers([
        SessionCheck::Unreachable("timeout".into()),
        SessionCheck::Unreachable("timeout again".into()),
    ]);
    warden.validate().await;
    warden.validate().await;
    assert_eq!(event_log(&warden).await.len(), 4);
}

#[tokio::test]
async fn rejected_sessions_are_refreshed_from_the_browser() {
    let setup = Setup::new().await;
    let warden = setup.warden().await;
    setup.script.apply_answers([SessionCheck::Valid]);
    warden.connect("SID=old".into()).await.unwrap();

    setup
        .script
        .check_answers([SessionCheck::Invalid("rotated".into())]);
    setup.script.refresh_answers([Ok("SID=new".into())]);
    setup.script.apply_answers([SessionCheck::Valid]);

    let health = warden.validate().await;
    assert_eq!(health.state, Some(SessionState::Valid));
    assert!(health.last_refreshed.is_some());
    assert_eq!(warden.cookies().await.as_deref(), Some("SID=new"));
    assert_eq!(
        setup.script.applied.lock().unwrap().as_slice(),
        ["SID=old", "SID=new"]
    );
}

#[tokio::test]
async fn sessions_expire_when_refreshing_fails_and_stay_expired() {
    let setup = Setup::new().await;
    let warden = setup.warden().await;
    setup.script.apply_answers([SessionCheck::Valid]);
    warden.connect("SID=1".into()).await.unwrap();

    setup
        .script
        .check_answers([SessionCheck::Invalid("password changed".into())]);
    setup
        .script
        .refresh_answers([Err("the browser profile is no longer logged in".into())]);
    let health = warden.validate().await;
    assert!(health.is_expired());
    assert!(health.expired_at.is_some());
    let error = health.last_error.unwrap();
    assert!(error.contains("password changed"), "{error}");
    assert!(error.contains("no longer logged in"), "{error}");
    assert_eq!(warden.cookies().await, None, "expired cookies are not used");
    // The owner hears about it.
    let raised = setup.alerts.raised();
    assert!(
        matches!(
            &raised[..],
            [(OWNER, Alert::YouTubeMusicExpired { expired_at, reason })]
                if Some(*expired_at) == health.expired_at && reason.contains("password changed")
        ),
        "{raised:?}"
    );

    // Transient trouble does not hide an expiry.
    setup
        .script
        .check_answers([SessionCheck::Unreachable("offline".into())]);
    assert!(warden.validate().await.is_expired());

    // The state survives a restart, and restoring leaves it alone.
    let restarted = setup.warden().await;
    assert!(restarted.health().is_expired());
    assert!(restarted.restore().await.is_expired());
    assert_eq!(setup.alerts.raised().len(), 1, "one alert per expiry");

    // Logging in again recovers.
    setup.script.apply_answers([SessionCheck::Valid]);
    let health = restarted.connect("SID=2".into()).await.unwrap();
    assert_eq!(health.state, Some(SessionState::Valid));
    assert_eq!(health.expired_at, None);
    let log = event_log(&restarted).await;
    assert_eq!(log[0], "Connected");
    assert!(log[1].starts_with("Session expired: password changed"));
    assert_eq!(log[2], "Connected");
}

#[tokio::test]
async fn scheduled_refreshes_that_fail_only_degrade() {
    let setup = Setup::new().await;
    let warden = setup.warden().await;
    setup.script.apply_answers([SessionCheck::Valid]);
    warden.connect("SID=1".into()).await.unwrap();

    setup
        .script
        .refresh_answers([Err("Chromium is not installed".into())]);
    let health = warden.refresh().await;
    assert_eq!(health.state, Some(SessionState::Degraded));
    assert_eq!(warden.cookies().await.as_deref(), Some("SID=1"));
}

/// Pauses a job the first time it runs, then finishes it.
#[derive(Default)]
struct PausesOnce(Mutex<bool>);

impl pixiu_jobs::Executor for PausesOnce {
    fn run<'a>(
        &'a self,
        _job: &'a pixiu_db::Job,
        _progress: &'a (dyn Fn(u8) + Send + Sync),
    ) -> BoxFuture<'a, pixiu_jobs::Outcome> {
        Box::pin(async move {
            let mut paused = self.0.lock().unwrap();
            if *paused {
                pixiu_jobs::Outcome::Done { track_id: None }
            } else {
                *paused = true;
                pixiu_jobs::Outcome::Paused("waiting for a login".to_owned())
            }
        })
    }
}

#[tokio::test]
async fn logging_in_resumes_paused_jobs() {
    use pixiu_db::JobState;
    use pixiu_jobs::{Jobs, NewJob, watch};

    let setup = Setup::new().await;
    let warden = setup.warden().await;
    let jobs = Jobs::new(setup.db.clone(), Box::new(PausesOnce::default()));
    jobs.start();
    watch::resume_on_login(&warden, Arc::clone(&jobs));

    let job = jobs
        .enqueue(OWNER, NewJob::sync(1, "Sync liked music"))
        .await
        .unwrap();
    let reaches = async |state: JobState| {
        for _ in 0..200 {
            let recent = jobs.recent(OWNER, 10).await.unwrap();
            if recent
                .iter()
                .any(|found| found.id == job.id && found.state == state)
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        panic!("the job never reached {state:?}");
    };
    reaches(JobState::Paused).await;

    setup.script.apply_answers([SessionCheck::Valid]);
    warden.connect("SID=abc".to_owned()).await.unwrap();
    reaches(JobState::Done).await;
}

/// Each user's warden works with their own scripted platform.
struct FakeSessions {
    scripts: std::collections::HashMap<u64, Arc<Script>>,
    forgotten: Arc<Mutex<Vec<u64>>>,
}

impl pixiu_jobs::SessionFactory for FakeSessions {
    fn session(&self, owner: u64, _platform: Platform) -> Box<dyn Session> {
        Box::new(FakeSession(Arc::clone(&self.scripts[&owner])))
    }

    fn refresher(&self, owner: u64, _platform: Platform) -> Box<dyn Refresher> {
        Box::new(FakeRefresher(Arc::clone(&self.scripts[&owner])))
    }

    fn forget(&self, owner: u64) -> BoxFuture<'_, ()> {
        self.forgotten.lock().unwrap().push(owner);
        Box::pin(async {})
    }
}

#[tokio::test]
async fn every_user_has_a_warden_of_their_own() {
    const OTHER: u64 = 2;
    let setup = Setup::new().await;
    let (mine, theirs) = (Arc::new(Script::default()), Arc::new(Script::default()));
    let forgotten = Arc::default();
    let wardens = pixiu_jobs::Wardens::new(
        setup.db.clone(),
        setup.secrets.clone(),
        Box::new(FakeSessions {
            scripts: [(OWNER, Arc::clone(&mine)), (OTHER, Arc::clone(&theirs))].into(),
            forgotten: Arc::clone(&forgotten),
        }),
        Arc::new(NoAlerts),
    );

    mine.apply_answers([SessionCheck::Valid]);
    wardens
        .get(OWNER, Platform::YouTubeMusic)
        .await
        .unwrap()
        .connect("SAPISID=mine".to_owned())
        .await
        .unwrap();
    assert_eq!(
        wardens.health(OWNER, Platform::YouTubeMusic).state,
        Some(SessionState::Valid)
    );
    // The other user never connected: nothing to show, and no cookies.
    assert_eq!(wardens.health(OTHER, Platform::YouTubeMusic).state, None);
    assert_eq!(wardens.cookies(OTHER, Platform::YouTubeMusic).await, None);
    assert_eq!(
        wardens
            .cookies(OWNER, Platform::YouTubeMusic)
            .await
            .as_deref(),
        Some("SAPISID=mine")
    );

    // Their session expiring leaves mine alone.
    theirs.apply_answers([SessionCheck::Valid]);
    let other = wardens.get(OTHER, Platform::YouTubeMusic).await.unwrap();
    other.connect("SAPISID=theirs".to_owned()).await.unwrap();
    theirs.check_answers([SessionCheck::Invalid("signed out".to_owned())]);
    theirs.refresh_answers([Err("the profile is logged out".to_owned())]);
    other.validate().await;
    assert_eq!(
        wardens.health(OTHER, Platform::YouTubeMusic).state,
        Some(SessionState::Expired)
    );
    assert_eq!(
        wardens.health(OWNER, Platform::YouTubeMusic).state,
        Some(SessionState::Valid)
    );
    assert_eq!(*mine.applied.lock().unwrap(), ["SAPISID=mine"]);

    // Stopping one lets go of their client and browser only.
    wardens.stop(OTHER).await;
    assert_eq!(*forgotten.lock().unwrap(), [OTHER]);
    assert_eq!(
        wardens.health(OWNER, Platform::YouTubeMusic).state,
        Some(SessionState::Valid)
    );
}

/// The queue runs jobs of active accounts; this is the tests' owner.
async fn account(db: &pixiu_db::Db) {
    pixiu_db::toasty::create!(pixiu_db::User {
        username: "owner",
        role: pixiu_db::Role::Admin,
        status: pixiu_db::UserStatus::Active,
        password_change_required: false,
        password_hash: "x",
        created_at: pixiu_db::now(),
    })
    .exec(&mut db.clone())
    .await
    .unwrap();
}
