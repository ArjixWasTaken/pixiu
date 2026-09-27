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
