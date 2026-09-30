//! Watches, with a fake catalog in place of YouTube Music.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use pixiu_db::{
    Album, Artist, ClaimKind, Db, Job, JobKind, JobState, Playlist, PlaylistEntry, ReleaseReason,
    ReleasedClaim, Track, TrackClaim, TrackOrigin, Watch, WatchKind, now, toasty,
};
use pixiu_hunt::{AlbumKind, Discography, RemoteAlbum, RemotePlaylist, RemoteTrack};
use pixiu_jobs::{
    Executor, Jobs, NewJob, Outcome, Wanted,
    queue::{AlbumJob, TrackJob},
    warden::BoxFuture,
    watch::{self, Catalog, CatalogError, NewWatch, Synced, WAITING_FOR_LOGIN},
};
use pixiu_treasury::{Claim, Treasury};

/// The user every test library and job belongs to.
const OWNER: u64 = 1;

#[derive(Default)]
struct FakeCatalog {
    /// Video ids of the playlist, in order.
    playlist: Mutex<Vec<&'static str>>,
    /// Browse ids and kinds of the artist's releases.
    albums: Mutex<Vec<(&'static str, AlbumKind)>>,
    logged_in: Mutex<bool>,
}

impl Catalog for FakeCatalog {
    fn playlist<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemotePlaylist, CatalogError>> {
        Box::pin(async move {
            let tracks = self
                .playlist
                .lock()
                .unwrap()
                .iter()
                .map(|video_id| RemoteTrack {
                    id: (*video_id).to_owned(),
                    title: format!("Song {video_id}"),
                    artists: vec!["Somebody".to_owned()],
                    artist_id: None,
                    album: None,
                    duration_secs: Some(180),
                    track_number: None,
                    cover_url: None,
                    is_video: false,
                })
                .collect();
            Ok(RemotePlaylist {
                id: id.to_owned(),
                name: "Test playlist".to_owned(),
                image_url: Some("https://example.com/playlist.jpg".to_owned()),
                tracks,
            })
        })
    }

    fn discography<'a>(
        &'a self,
        channel_id: &'a str,
    ) -> BoxFuture<'a, Result<Discography, CatalogError>> {
        Box::pin(async move {
            let albums = self
                .albums
                .lock()
                .unwrap()
                .iter()
                .map(|(id, kind)| RemoteAlbum {
                    id: (*id).to_owned(),
                    title: format!("Release {id}"),
                    artists: vec!["The Band".to_owned()],
                    artist_id: Some(channel_id.to_owned()),
                    year: Some(2026),
                    kind: *kind,
                    cover_url: None,
                    tracks: Vec::new(),
                })
                .collect();
            Ok(Discography {
                id: channel_id.to_owned(),
                name: "The Band".to_owned(),
                image_url: Some("https://example.com/band.jpg".to_owned()),
                albums,
            })
        })
    }

    fn logged_in(&self) -> bool {
        *self.logged_in.lock().unwrap()
    }
}

/// Pauses a job the first time it runs, then finishes it.
#[derive(Default)]
struct PausesOnce(Mutex<bool>);

impl Executor for PausesOnce {
    fn run<'a>(
        &'a self,
        _job: &'a Job,
        _progress: &'a (dyn Fn(u8) + Send + Sync),
    ) -> BoxFuture<'a, Outcome> {
        Box::pin(async move {
            let mut paused = self.0.lock().unwrap();
            if *paused {
                Outcome::Done { track_id: None }
            } else {
                *paused = true;
                Outcome::Paused(WAITING_FOR_LOGIN.to_owned())
            }
        })
    }
}

struct Setup {
    _dir: tempfile::TempDir,
    db: Db,
    treasury: Treasury,
    /// Never started: the tests look at what gets queued.
    jobs: Arc<Jobs>,
    catalog: FakeCatalog,
}

async fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    account(&db).await;
    let treasury = Treasury::new(
        db.clone(),
        dir.path().join("treasure"),
        dir.path().join("cache"),
    );
    let jobs = Jobs::new(db.clone(), Box::new(PausesOnce::default()));
    Setup {
        _dir: dir,
        db,
        treasury,
        jobs,
        catalog: FakeCatalog::default(),
    }
}

/// Puts a track in the hoard, without any claim.
async fn hoard(db: &mut Db, video_id: &str) -> u64 {
    let artist = toasty::create!(Artist {
        user_id: OWNER,
        name: "Somebody",
        name_key: "somebody",
        created_at: now(),
    })
    .exec(db)
    .await
    .unwrap();
    let album = toasty::create!(Album {
        user_id: OWNER,
        title: format!("Album {video_id}"),
        title_key: format!("album {video_id}"),
        artist_id: artist.id,
        created_at: now(),
    })
    .exec(db)
    .await
    .unwrap();
    toasty::create!(Track {
        user_id: OWNER,
        album_id: album.id,
        artist_id: artist.id,
        title: format!("Song {video_id}"),
        artist_credit: "Somebody",
        duration_ms: 180_000_u64,
        file_id: 0_u64,
        path: format!("Somebody/{video_id}.opus"),
        size: 1_u64,
        suffix: "opus",
        content_type: "audio/ogg",
        ytm_video_id: Some(video_id.to_owned()),
        origin: TrackOrigin::Download,
        added_at: now(),
    })
    .exec(db)
    .await
    .unwrap()
    .id
}

async fn claims_of(db: &mut Db, track_id: u64) -> Vec<(ClaimKind, Option<String>)> {
    TrackClaim::filter_by_track_id(track_id)
        .exec(db)
        .await
        .unwrap()
        .into_iter()
        .map(|claim| (claim.kind, claim.reference))
        .collect()
}

async fn mirror_of(db: &mut Db, watch_id: u64) -> (String, Vec<String>) {
    let playlist = Playlist::filter_by_watch_id(Some(watch_id))
        .first()
        .exec(db)
        .await
        .unwrap()
        .expect("a mirror");
    let mut entries = PlaylistEntry::filter_by_playlist_id(playlist.id)
        .exec(db)
        .await
        .unwrap();
    entries.sort_by_key(|entry| entry.position);
    let order = entries
        .into_iter()
        .map(|entry| entry.ytm_video_id.unwrap())
        .collect();
    (playlist.name, order)
}

fn done(synced: Synced) -> Vec<NewJob> {
    match synced {
        Synced::Done(jobs) => jobs,
        Synced::NeedsLogin(reason) => panic!("waiting: {reason}"),
    }
}

fn videos(jobs: &[NewJob]) -> Vec<(String, Wanted)> {
    jobs.iter()
        .map(|job| {
            assert_eq!(job.kind, JobKind::DownloadTrack);
            let payload: TrackJob = serde_json::from_str(&job.payload).unwrap();
            (payload.video_id, payload.wanted)
        })
        .collect()
}

fn albums(jobs: &[NewJob]) -> Vec<(String, Wanted)> {
    jobs.iter()
        .map(|job| {
            assert_eq!(job.kind, JobKind::GrabAlbum);
            let payload: AlbumJob = serde_json::from_str(&job.payload).unwrap();
            (payload.browse_id, payload.wanted)
        })
        .collect()
}

fn playlist_watch(remote_id: &str) -> NewWatch {
    NewWatch {
        kind: WatchKind::Playlist,
        remote_id: remote_id.to_owned(),
        include_singles: false,
        only_new: false,
    }
}

#[tokio::test]
async fn playlists_are_mirrored_and_claims_follow_them() {
    let s = setup().await;
    let mut db = s.db.clone();
    let b = hoard(&mut db, "b").await;
    *s.catalog.playlist.lock().unwrap() = vec!["a", "b", "c"];

    let watch = watch::add(&s.treasury, &s.jobs, OWNER, playlist_watch("PLtest"))
        .await
        .unwrap();
    // Adding queues the first sync, and only one waits at a time.
    let syncs = |jobs: &[Job]| {
        jobs.iter()
            .filter(|job| job.kind == JobKind::SyncWatch)
            .count()
    };
    assert_eq!(syncs(&s.jobs.unfinished(OWNER).await.unwrap()), 1);
    assert!(!watch::queue_sync(&s.jobs, &watch).await.unwrap());
    assert!(matches!(
        watch::add(&s.treasury, &s.jobs, OWNER, playlist_watch("PLtest")).await,
        Err(watch::WatchError::Duplicate)
    ));

    let wanted = Wanted::Playlist { watch_id: watch.id };
    let queued = done(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
    );
    assert_eq!(
        videos(&queued),
        [("a".to_owned(), wanted), ("c".to_owned(), wanted)]
    );
    assert_eq!(
        mirror_of(&mut db, watch.id).await,
        (
            "Test playlist".to_owned(),
            vec!["a".into(), "b".into(), "c".into()]
        )
    );
    // Songs not downloaded yet are named as the platform names them.
    let named: Vec<(Option<String>, Option<String>)> = PlaylistEntry::all()
        .exec(&mut db)
        .await
        .unwrap()
        .into_iter()
        .filter(|entry| entry.ytm_video_id.as_deref() == Some("c"))
        .map(|entry| (entry.title, entry.artist))
        .collect();
    assert_eq!(
        named,
        [(Some("Song c".to_owned()), Some("Somebody".to_owned()))]
    );
    let reference = Some(watch.id.to_string());
    assert_eq!(
        claims_of(&mut db, b).await,
        [(ClaimKind::WatchPlaylist, reference.clone())]
    );
    let synced = Watch::get_by_id(&mut db, &watch.id).await.unwrap();
    assert_eq!(synced.name, "Test playlist");
    assert_eq!(
        synced.image_url.as_deref(),
        Some("https://example.com/playlist.jpg")
    );
    assert!(synced.last_synced_at.is_some() && synced.last_error.is_none());

    // While the downloads are queued, syncing does not queue them again.
    for job in queued {
        s.jobs.enqueue(OWNER, job).await.unwrap();
    }
    assert!(
        done(
            watch::sync(&s.treasury, &s.catalog, watch.id)
                .await
                .unwrap()
        )
        .is_empty()
    );

    // "a" arrives; "b" leaves the playlist, which is reordered.
    let a = hoard(&mut db, "a").await;
    s.treasury.claim(a, &wanted.claim(None)).await.unwrap();
    *s.catalog.playlist.lock().unwrap() = vec!["c", "a"];
    done(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
    );
    assert_eq!(
        mirror_of(&mut db, watch.id).await.1,
        ["c".to_owned(), "a".to_owned()]
    );
    assert_eq!(
        claims_of(&mut db, a).await,
        [(ClaimKind::WatchPlaylist, reference)]
    );
    // The file stays, as an orphan.
    assert!(claims_of(&mut db, b).await.is_empty());
    let orphans: Vec<u64> = s
        .treasury
        .orphans(OWNER)
        .await
        .unwrap()
        .iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(orphans, [b]);
}

#[tokio::test]
async fn liked_music_waits_for_a_login() {
    let s = setup().await;
    let mut db = s.db.clone();
    let watch = watch::add(
        &s.treasury,
        &s.jobs,
        OWNER,
        NewWatch {
            kind: WatchKind::LikedMusic,
            remote_id: String::new(),
            include_singles: false,
            only_new: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        (watch.remote_id.as_str(), watch.name.as_str()),
        ("LM", "Liked music")
    );

    assert_eq!(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
        Synced::NeedsLogin(WAITING_FOR_LOGIN.to_owned())
    );
    let waiting = Watch::get_by_id(&mut db, &watch.id).await.unwrap();
    assert_eq!(waiting.last_error.as_deref(), Some(WAITING_FOR_LOGIN));
    assert!(waiting.last_synced_at.is_none());

    *s.catalog.logged_in.lock().unwrap() = true;
    *s.catalog.playlist.lock().unwrap() = vec!["x"];
    let queued = done(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
    );
    assert_eq!(queued.len(), 1);
    assert_eq!(mirror_of(&mut db, watch.id).await.0, "Liked music");
    assert!(
        Watch::get_by_id(&mut db, &watch.id)
            .await
            .unwrap()
            .last_error
            .is_none()
    );
}

#[tokio::test]
async fn artists_bring_their_releases() {
    let s = setup().await;
    let mut db = s.db.clone();
    *s.catalog.albums.lock().unwrap() =
        vec![("old", AlbumKind::Album), ("single", AlbumKind::Single)];
    // Already in the hoard, but nothing said which channel is theirs.
    let band = toasty::create!(Artist {
        user_id: OWNER,
        name: "The Band",
        name_key: "the band",
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();

    // Only new releases, no singles: the first sync takes note of the album.
    let newcomer = watch::add(
        &s.treasury,
        &s.jobs,
        OWNER,
        NewWatch {
            kind: WatchKind::Artist,
            remote_id: "UCnew".to_owned(),
            include_singles: false,
            only_new: true,
        },
    )
    .await
    .unwrap();
    assert!(
        done(
            watch::sync(&s.treasury, &s.catalog, newcomer.id)
                .await
                .unwrap()
        )
        .is_empty()
    );
    let noted = Watch::get_by_id(&mut db, &newcomer.id).await.unwrap();
    assert_eq!(noted.seen, ["old"]);
    assert_eq!(noted.name, "The Band");
    assert_eq!(
        noted.image_url.as_deref(),
        Some("https://example.com/band.jpg")
    );
    // Now the hoard's artist knows its channel.
    let band = Artist::get_by_id(&mut db, &band.id).await.unwrap();
    assert_eq!(band.ytm_channel_id.as_deref(), Some("UCnew"));

    *s.catalog.albums.lock().unwrap() = vec![
        ("fresh", AlbumKind::Album),
        ("old", AlbumKind::Album),
        ("another single", AlbumKind::Single),
    ];
    let wanted = Wanted::Artist {
        watch_id: newcomer.id,
    };
    let queued = done(
        watch::sync(&s.treasury, &s.catalog, newcomer.id)
            .await
            .unwrap(),
    );
    assert_eq!(albums(&queued), [("fresh".to_owned(), wanted)]);
    // Nothing twice.
    assert!(
        done(
            watch::sync(&s.treasury, &s.catalog, newcomer.id)
                .await
                .unwrap()
        )
        .is_empty()
    );

    // The whole discography, singles included.
    let everything = watch::add(
        &s.treasury,
        &s.jobs,
        OWNER,
        NewWatch {
            kind: WatchKind::Artist,
            remote_id: "UCall".to_owned(),
            include_singles: true,
            only_new: false,
        },
    )
    .await
    .unwrap();
    let queued = done(
        watch::sync(&s.treasury, &s.catalog, everything.id)
            .await
            .unwrap(),
    );
    let grabbed: Vec<String> = albums(&queued).into_iter().map(|(id, _)| id).collect();
    assert_eq!(grabbed, ["fresh", "old", "another single"]);
}

#[tokio::test]
async fn removing_a_watch_lets_go() {
    let s = setup().await;
    let mut db = s.db.clone();
    let kept = hoard(&mut db, "kept").await;
    *s.catalog.playlist.lock().unwrap() = vec!["kept", "coming"];
    let watch = watch::add(&s.treasury, &s.jobs, OWNER, playlist_watch("PLgone"))
        .await
        .unwrap();
    for job in done(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
    ) {
        s.jobs.enqueue(OWNER, job).await.unwrap();
    }
    // An unrelated grab stays queued.
    s.jobs
        .enqueue(OWNER, NewJob::track("mine", "A grab", None))
        .await
        .unwrap();

    watch::remove(&s.treasury, &s.jobs, watch.id).await.unwrap();
    assert!(
        Watch::filter_by_id(watch.id)
            .first()
            .exec(&mut db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        Playlist::filter_by_watch_id(Some(watch.id))
            .first()
            .exec(&mut db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(claims_of(&mut db, kept).await.is_empty());
    // The orphan it leaves knows why.
    let released = ReleasedClaim::filter_by_track_id(kept)
        .exec(&mut db)
        .await
        .unwrap();
    assert_eq!(released.len(), 1);
    assert_eq!(released[0].reason, ReleaseReason::WatchRemoved);
    assert_eq!(released[0].source_name.as_deref(), Some("Test playlist"));
    let left: Vec<String> = s
        .jobs
        .unfinished(OWNER)
        .await
        .unwrap()
        .into_iter()
        .map(|job| job.title)
        .collect();
    assert_eq!(left, ["A grab"]);
}

#[tokio::test]
async fn paused_jobs_resume_later() {
    let s = setup().await;
    s.jobs.start();
    let job = s
        .jobs
        .enqueue(OWNER, NewJob::sync(1, "Sync liked music"))
        .await
        .unwrap();
    let state = async |jobs: &Jobs, wanted: JobState| {
        for _ in 0..200 {
            let found = jobs.recent(OWNER, 10).await.unwrap();
            if let Some(job) = found.iter().find(|found| found.id == job.id)
                && job.state == wanted
            {
                return job.error.clone();
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("never reached {wanted:?}");
    };
    assert_eq!(
        state(&s.jobs, JobState::Paused).await.as_deref(),
        Some(WAITING_FOR_LOGIN)
    );
    assert_eq!(s.jobs.resume_paused(OWNER).await.unwrap(), 1);
    assert_eq!(state(&s.jobs, JobState::Done).await, None);
}

#[tokio::test]
async fn excluded_songs_are_orphaned_and_skipped() {
    let s = setup().await;
    let mut db = s.db.clone();
    let a = hoard(&mut db, "a").await;
    let b = hoard(&mut db, "b").await;
    *s.catalog.playlist.lock().unwrap() = vec!["a", "b", "c"];
    let watch = watch::add(&s.treasury, &s.jobs, OWNER, playlist_watch("PLpick"))
        .await
        .unwrap();
    for job in done(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
    ) {
        s.jobs.enqueue(OWNER, job).await.unwrap();
    }
    let reference = Some(watch.id.to_string());
    assert_eq!(
        claims_of(&mut db, a).await,
        [(ClaimKind::WatchPlaylist, reference.clone())]
    );
    // B is starred too, so something else still keeps it.
    s.treasury
        .claim(
            b,
            &Claim {
                kind: ClaimKind::Starred,
                reference: Some(format!("tr-{b}")),
            },
        )
        .await
        .unwrap();

    // Excluding takes songs out of the mirror and lets go of them; C's
    // queued download is forgotten.
    for video in ["a", "b", "c"] {
        assert!(
            watch::exclude(&s.treasury, &s.jobs, watch.id, video)
                .await
                .unwrap()
        );
    }
    // Twice is the same as once.
    assert!(
        watch::exclude(&s.treasury, &s.jobs, watch.id, "a")
            .await
            .unwrap()
    );
    assert_eq!(mirror_of(&mut db, watch.id).await.1, Vec::<String>::new());
    assert!(claims_of(&mut db, a).await.is_empty());
    let released = ReleasedClaim::filter_by_track_id(a)
        .exec(&mut db)
        .await
        .unwrap();
    assert_eq!(released.len(), 1);
    assert_eq!(released[0].reason, ReleaseReason::Excluded);
    assert_eq!(released[0].source_name.as_deref(), Some("Test playlist"));
    let orphans: Vec<u64> = s
        .treasury
        .orphans(OWNER)
        .await
        .unwrap()
        .iter()
        .map(|track| track.id)
        .collect();
    assert_eq!(orphans, [a]);
    let downloads = s
        .jobs
        .unfinished(OWNER)
        .await
        .unwrap()
        .iter()
        .filter(|job| job.kind == JobKind::DownloadTrack)
        .count();
    assert_eq!(downloads, 0);
    let listed: Vec<(String, Option<String>)> = watch::exclusions(&mut db, watch.id)
        .await
        .unwrap()
        .into_iter()
        .map(|exclusion| (exclusion.ytm_video_id, exclusion.title))
        .collect();
    assert_eq!(listed.len(), 3);
    assert!(listed.contains(&("c".to_owned(), Some("Song c".to_owned()))));

    // Later syncs neither list, claim nor fetch them.
    let jobs = done(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
    );
    assert!(jobs.is_empty(), "{:?}", videos(&jobs));
    assert!(claims_of(&mut db, a).await.is_empty());
    assert_eq!(mirror_of(&mut db, watch.id).await.1, Vec::<String>::new());

    // Taking it back queues a sync, which lists and claims it again.
    watch::include(&s.treasury, &s.jobs, watch.id, "a")
        .await
        .unwrap();
    assert!(
        s.jobs
            .unfinished(OWNER)
            .await
            .unwrap()
            .iter()
            .any(|job| watch::synced_watch(job) == Some(watch.id))
    );
    done(
        watch::sync(&s.treasury, &s.catalog, watch.id)
            .await
            .unwrap(),
    );
    assert_eq!(mirror_of(&mut db, watch.id).await.1, ["a"]);
    assert_eq!(
        claims_of(&mut db, a).await,
        [(ClaimKind::WatchPlaylist, reference)]
    );

    // Removing the watch forgets its exclusions.
    watch::remove(&s.treasury, &s.jobs, watch.id).await.unwrap();
    assert!(
        watch::exclusions(&mut db, watch.id)
            .await
            .unwrap()
            .is_empty()
    );
}

/// The queue runs jobs of active accounts; this is the tests' owner.
async fn account(db: &pixiu_db::Db) {
    toasty::create!(pixiu_db::User {
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
