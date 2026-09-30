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
/// store, and what belonged to the file layout goes.
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
}
