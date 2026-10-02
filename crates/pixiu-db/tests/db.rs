use std::path::Path;

use pixiu_db::{User, WebSession, now, toasty};
use toasty::{
    migration::{History, Snapshot, generate},
    schema::diff::RenameHints,
};

#[tokio::test]
async fn open_applies_migrations_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pixiu.db");

    let db = pixiu_db::open(&path).await.unwrap();
    let mut conn = db.clone();
    let user = toasty::create!(User {
        role: pixiu_db::Role::Admin,
        status: pixiu_db::UserStatus::Active,
        password_change_required: false,
        username: "admin",
        password_hash: "$argon2id$not-a-real-hash",
        created_at: now(),
    })
    .exec(&mut conn)
    .await
    .unwrap();
    drop(db);

    // Reopening must skip the already-applied migrations and keep the data.
    let mut db = pixiu_db::open(&path).await.unwrap();
    let reloaded = User::get_by_username(&mut db, "admin").await.unwrap();
    assert_eq!(reloaded.id, user.id);
}

#[tokio::test]
async fn sessions_belong_to_users() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();

    let user = toasty::create!(User {
        role: pixiu_db::Role::Admin,
        status: pixiu_db::UserStatus::Active,
        password_change_required: false,
        username: "admin",
        password_hash: "hash",
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();
    toasty::create!(WebSession {
        token_hash: "ab".repeat(32),
        user_id: user.id,
        created_at: now(),
        expires_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();

    let session = WebSession::get_by_token_hash(&mut db, "ab".repeat(32))
        .await
        .unwrap();
    assert_eq!(session.user_id, user.id);
}

/// Fails when a model changed without a matching migration.
#[tokio::test]
async fn migrations_match_models() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("toasty");
    let history = History::load(dir.join("history.toml")).unwrap();
    let latest = history
        .entries()
        .last()
        .expect("at least one migration exists");
    let snapshot = Snapshot::load(dir.join("snapshots").join(&latest.snapshot_name)).unwrap();

    let db = pixiu_db::connect("sqlite::memory:").await.unwrap();
    let pending = generate(
        db.driver(),
        &snapshot.schema,
        &db.schema().db,
        &RenameHints::new(),
    );

    assert!(
        pending.is_none(),
        "the models differ from the latest migration snapshot; run \
         `cargo run -p pixiu-db --features cli -- migration generate --name <change>`"
    );
}

/// Rebuilding a table (SQLite's way to change a CHECK constraint) drops its
/// indexes, and Toasty's generator does not always recreate them. Every
/// index the migrations create, and do not drop on purpose, must exist.
#[tokio::test]
async fn migrations_keep_their_indexes() {
    let migrations = Path::new(env!("CARGO_MANIFEST_DIR")).join("toasty/migrations");
    let mut files: Vec<_> = std::fs::read_dir(&migrations)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sql"))
        .collect();
    files.sort();
    let mut expected = std::collections::BTreeSet::new();
    for file in files {
        for line in std::fs::read_to_string(file).unwrap().lines() {
            let name = |statement: &str| {
                line.split('"')
                    .nth(1)
                    .filter(|_| line.starts_with(statement))
            };
            if let Some(index) = name("CREATE INDEX").or_else(|| name("CREATE UNIQUE INDEX")) {
                expected.insert(index.to_owned());
            } else if let Some(index) = name("DROP INDEX") {
                expected.remove(index);
            }
        }
    }
    assert!(expected.contains("index_jobs_by_state"), "{expected:?}");

    let dir = tempfile::tempdir().unwrap();
    let mut db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let rows = toasty::sql::query("SELECT name FROM sqlite_master WHERE type = 'index'")
        .exec(&mut db)
        .await
        .unwrap();
    let present: std::collections::BTreeSet<String> = rows
        .into_iter()
        .filter_map(|row| match row {
            toasty::stmt::Value::Record(record) => match &record[0] {
                toasty::stmt::Value::String(name) => Some(name.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let missing: Vec<_> = expected.difference(&present).collect();
    assert!(
        missing.is_empty(),
        "indexes lost by migrations: {missing:?}"
    );
}

/// Copies a database fixture made with the models of an older schema, so a
/// test can open (and upgrade) it.
fn fixture_db(name: &str, dir: &Path) -> std::path::PathBuf {
    let path = dir.join("pixiu.db");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
        &path,
    )
    .unwrap();
    path
}

/// A library from before files were shared (schema 0008) upgrades with
/// nothing lost: its tracks wait for their files to be adopted into the
/// store, what belonged to the file layout goes, and everything becomes the
/// one user's library.
#[tokio::test]
async fn upgrade_from_0008() {
    use pixiu_db::{Annotation, AudioFile, Job, JobKind, Playlist, Setting, Track, Watch};

    let dir = tempfile::tempdir().unwrap();
    let mut db = pixiu_db::open(&fixture_db("v0008.db", dir.path()))
        .await
        .unwrap();

    let mut tracks = Track::all().exec(&mut db).await.unwrap();
    tracks.sort_by_key(|track| track.id);
    assert_eq!(tracks.len(), 2);
    assert!(tracks.iter().all(|track| track.file_id == 0));
    assert_eq!(
        tracks[0].path,
        "Test Artist/2024 - Test Album/01-01 First Light.flac"
    );
    assert!(AudioFile::all().exec(&mut db).await.unwrap().is_empty());

    let mut kinds: Vec<_> = Job::all()
        .exec(&mut db)
        .await
        .unwrap()
        .into_iter()
        .map(|job| job.kind)
        .collect();
    kinds.sort_by_key(|kind| format!("{kind:?}"));
    assert_eq!(kinds, [JobKind::DownloadTrack, JobKind::Enrich]);
    let keys: Vec<_> = Setting::all()
        .exec(&mut db)
        .await
        .unwrap()
        .into_iter()
        .map(|setting| setting.key)
        .collect();
    assert_eq!(keys, ["repair.album-artists"]);

    assert_eq!(Annotation::all().exec(&mut db).await.unwrap().len(), 2);
    assert_eq!(Playlist::all().exec(&mut db).await.unwrap().len(), 3);
    assert_eq!(Watch::all().exec(&mut db).await.unwrap().len(), 1);

    // Everything was the one user's, and still is: in their library now.
    let owner = User::all().exec(&mut db).await.unwrap()[0].id;
    let owners = |table: &str| format!("SELECT DISTINCT user_id FROM {table}");
    for table in [
        "artists",
        "albums",
        "tracks",
        "annotations",
        "playlists",
        "playlist_folders",
        "watches",
        "jobs",
        "offerings",
        "source_sessions",
        "session_events",
    ] {
        let rows = toasty::sql::query(owners(table))
            .exec(&mut db)
            .await
            .unwrap();
        let ids: Vec<_> = rows
            .into_iter()
            .map(|row| match row {
                toasty::stmt::Value::Record(record) => record[0].clone(),
                other => other,
            })
            .collect();
        assert_eq!(
            ids,
            [toasty::stmt::Value::I64(i64::try_from(owner).unwrap())],
            "{table}"
        );
    }
    // Their liked-music watch and its mirror are found per user.
    assert!(
        Watch::filter_by_user_id_and_source_key(owner, "youtube_music:LM")
            .first()
            .exec(&mut db)
            .await
            .unwrap()
            .is_some()
    );
}

/// A library from before keys (schema 0014) upgrades with every platform
/// id becoming a YouTube Music key: songs, files, aliases, albums, artists,
/// watches and what artist watches saw, mirror entries, exclusions and the
/// albums album grabs name. What came from no platform stays without one.
#[tokio::test]
async fn upgrade_from_0014() {
    use pixiu_db::{
        Album, Artist, AudioFile, PlaylistEntry, SourceKey, Track, TrackAlias, TrackClaim, Watch,
        WatchExclusion, keyed,
    };

    let dir = tempfile::tempdir().unwrap();
    let mut db = pixiu_db::open(&fixture_db("v0014.db", dir.path()))
        .await
        .unwrap();

    let mut tracks = Track::all().exec(&mut db).await.unwrap();
    tracks.sort_by_key(|track| track.id);
    let keys: Vec<_> = tracks
        .iter()
        .map(|track| track.source_key.clone())
        .collect();
    assert_eq!(keys, [Some("youtube_music:video-1".to_owned()), None]);

    let mut files = AudioFile::all().exec(&mut db).await.unwrap();
    files.sort_by_key(|file| file.id);
    let keys: Vec<_> = files.iter().map(|file| file.source_key.clone()).collect();
    assert_eq!(keys, [Some("youtube_music:video-1".to_owned()), None]);

    let aliases = TrackAlias::all().exec(&mut db).await.unwrap();
    assert_eq!(aliases[0].source_key, "youtube_music:video-1b");
    // The alias still finds its track.
    let found = keyed::track_of_key(&mut db, 1, &SourceKey::youtube_music("video-1b"))
        .await
        .unwrap();
    assert_eq!(found.map(|track| track.id), Some(1));

    let mut albums = Album::all().exec(&mut db).await.unwrap();
    albums.sort_by_key(|album| album.id);
    let keys: Vec<_> = albums
        .iter()
        .map(|album| album.source_key.clone())
        .collect();
    assert_eq!(keys, [Some("youtube_music:MPREb_album".to_owned()), None]);

    let mut artists = Artist::all().exec(&mut db).await.unwrap();
    artists.sort_by_key(|artist| artist.id);
    let keys: Vec<_> = artists
        .iter()
        .map(|artist| artist.source_key.clone())
        .collect();
    assert_eq!(keys, [Some("youtube_music:UCartist".to_owned()), None]);

    let mut watches = Watch::all().exec(&mut db).await.unwrap();
    watches.sort_by_key(|watch| watch.id);
    let keys: Vec<_> = watches
        .iter()
        .map(|watch| watch.source_key.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "youtube_music:PLtest",
            "youtube_music:LM",
            "youtube_music:UCartist"
        ]
    );
    assert!(watches[0].seen.is_empty());
    assert_eq!(
        watches[2].seen,
        ["youtube_music:MPREb_album", "youtube_music:MPREb_single"]
    );

    let mut entries = PlaylistEntry::all().exec(&mut db).await.unwrap();
    entries.sort_by_key(|entry| entry.position);
    let keys: Vec<_> = entries
        .iter()
        .map(|entry| entry.source_key.clone())
        .collect();
    assert_eq!(
        keys,
        [
            Some("youtube_music:video-1".to_owned()),
            Some("youtube_music:video-2".to_owned())
        ]
    );

    let exclusions = WatchExclusion::all().exec(&mut db).await.unwrap();
    assert_eq!(exclusions[0].source_key, "youtube_music:video-3");

    let mut claims = TrackClaim::all().exec(&mut db).await.unwrap();
    claims.sort_by_key(|claim| claim.id);
    let references: Vec<_> = claims.iter().map(|claim| claim.reference.clone()).collect();
    assert_eq!(
        references,
        [
            Some("youtube_music:MPREb_album".to_owned()),
            Some("1".to_owned()),
            None,
            None
        ]
    );
}
