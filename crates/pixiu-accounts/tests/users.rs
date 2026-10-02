//! Accounts: making them, their rules, and the last-admin guard.

use pixiu_accounts::{AccountError, NewUser, users};
use pixiu_core::SecretBox;
use pixiu_db::{ApiKey, Db, Role, Track, User, UserStatus, now, toasty};

async fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    (dir, db)
}

fn new(username: &str, role: Role) -> NewUser {
    NewUser {
        username: username.to_owned(),
        email: None,
        email_verified: false,
        password: "long enough".to_owned(),
        role,
        status: UserStatus::Active,
        password_change_required: false,
    }
}

#[test]
fn values_follow_the_rules() {
    assert_eq!(users::check_username("  alice ").unwrap(), "alice");
    assert!(users::check_username("").is_err());
    assert!(users::check_username("a@b").is_err());
    assert!(users::check_username(&"x".repeat(65)).is_err());
    assert!(users::check_password("short").is_err());
    assert!(users::check_password("long enough").is_ok());
    assert_eq!(
        users::check_email(" Alice@Example.COM ")
            .unwrap()
            .as_deref(),
        Some("alice@example.com")
    );
    assert_eq!(users::check_email("").unwrap(), None);
    for bad in [
        "alice",
        "alice@",
        "@example.com",
        "alice@example",
        "a b@c.d",
    ] {
        assert!(users::check_email(bad).is_err(), "{bad}");
    }
}

#[tokio::test]
async fn names_and_emails_are_taken_once() {
    let (_dir, mut db) = db().await;
    let secrets = SecretBox::ephemeral();
    let alice = users::create(
        &mut db,
        &secrets,
        NewUser {
            email: Some("Alice@Example.com".to_owned()),
            ..new("Alice", Role::Admin)
        },
    )
    .await
    .unwrap();
    assert_eq!(alice.email.as_deref(), Some("alice@example.com"));
    assert!(alice.subsonic_secret.is_some());
    assert!(matches!(
        users::create(&mut db, &secrets, new("alice", Role::User)).await,
        Err(AccountError::UsernameTaken)
    ));
    assert!(matches!(
        users::create(
            &mut db,
            &secrets,
            NewUser {
                email: Some("ALICE@example.com".to_owned()),
                ..new("bob", Role::User)
            }
        )
        .await,
        Err(AccountError::EmailTaken)
    ));

    // Signing in takes the username in any case, or the email.
    for login in ["Alice", "ALICE", "alice@EXAMPLE.com"] {
        let found = users::find_by_login(&mut db, login).await.unwrap();
        assert_eq!(found.map(|user| user.id), Some(alice.id), "{login}");
    }
    assert!(
        users::find_by_login(&mut db, "nobody")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn an_active_admin_always_remains() {
    let (_dir, mut db) = db().await;
    let secrets = SecretBox::ephemeral();
    let alice = users::create(&mut db, &secrets, new("alice", Role::Admin))
        .await
        .unwrap();
    let bob = users::create(&mut db, &secrets, new("bob", Role::User))
        .await
        .unwrap();

    for refused in [
        users::set_role(&mut db, alice.id, Role::User).await.err(),
        users::set_status(&mut db, alice.id, UserStatus::Disabled)
            .await
            .err(),
        users::check_deletable(&mut db, alice.id).await.err(),
        users::delete(&mut db, alice.id).await.err(),
    ] {
        assert!(
            matches!(refused, Some(AccountError::LastAdmin)),
            "{refused:?}"
        );
    }

    // With a second admin, the first may step down.
    users::set_role(&mut db, bob.id, Role::Admin).await.unwrap();
    users::set_role(&mut db, alice.id, Role::User)
        .await
        .unwrap();
    // A disabled admin does not count.
    users::set_status(&mut db, alice.id, UserStatus::Active)
        .await
        .unwrap();
    users::set_role(&mut db, alice.id, Role::Admin)
        .await
        .unwrap();
    users::set_status(&mut db, alice.id, UserStatus::Disabled)
        .await
        .unwrap();
    assert!(matches!(
        users::set_role(&mut db, bob.id, Role::User).await,
        Err(AccountError::LastAdmin)
    ));
}

#[tokio::test]
async fn new_passwords_sign_web_sessions_out() {
    let (_dir, mut db) = db().await;
    let secrets = SecretBox::ephemeral();
    let alice = users::create(&mut db, &secrets, new("alice", Role::Admin))
        .await
        .unwrap();
    let mut keys = Vec::new();
    for name in [
        users::WEB_SESSION,
        "Web session: Firefox on Linux",
        "Phone",
        "Web sessions of mine",
    ] {
        keys.push(
            toasty::create!(ApiKey {
                user_id: alice.id,
                name,
                key_hash: ApiKey::hash(&format!("key-{}", keys.len())),
                created_at: now(),
            })
            .exec(&mut db)
            .await
            .unwrap()
            .id,
        );
    }
    let alice = users::set_password(&mut db, &secrets, alice, "a new one!", true, Some(keys[0]))
        .await
        .unwrap();
    assert!(alice.password_change_required);
    let left: Vec<u64> = ApiKey::filter_by_user_id(alice.id)
        .exec(&mut db)
        .await
        .unwrap()
        .iter()
        .map(|key| key.id)
        .collect();
    // The session that made the change stays; so do app keys, whatever
    // their name.
    assert_eq!(left, [keys[0], keys[2], keys[3]]);
    let user = User::get_by_id(&mut db, &alice.id).await.unwrap();
    assert!(pixiu_core::password::verify(
        "a new one!",
        Some(&user.password_hash)
    ));
    assert!(
        users::set_password(&mut db, &secrets, user, "short", false, None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn deleting_takes_the_library_along() {
    let (_dir, mut db) = db().await;
    let secrets = SecretBox::ephemeral();
    users::create(&mut db, &secrets, new("alice", Role::Admin))
        .await
        .unwrap();
    let bob = users::create(&mut db, &secrets, new("bob", Role::User))
        .await
        .unwrap();
    let artist = toasty::create!(pixiu_db::Artist {
        user_id: bob.id,
        name: "Bob's",
        name_key: "bob's",
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();
    let album = toasty::create!(pixiu_db::Album {
        user_id: bob.id,
        title: "Album",
        title_key: "album",
        artist_id: artist.id,
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();
    toasty::create!(Track {
        user_id: bob.id,
        album_id: album.id,
        artist_id: artist.id,
        title: "Song",
        artist_credit: "Bob's",
        duration_ms: 1_u64,
        file_id: 0_u64,
        path: "x",
        size: 1_u64,
        suffix: "mp3",
        content_type: "audio/mpeg",
        origin: pixiu_db::TrackOrigin::Offering,
        added_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();
    assert_eq!(users::usage(&mut db).await.unwrap()[&bob.id].songs, 1);

    users::delete(&mut db, bob.id).await.unwrap();
    assert!(
        User::filter_by_id(bob.id)
            .first()
            .exec(&mut db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(Track::all().exec(&mut db).await.unwrap().is_empty());
    assert!(
        pixiu_db::Album::all()
            .exec(&mut db)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        pixiu_db::Artist::all()
            .exec(&mut db)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn names_from_before_the_rules_stay() {
    let (_dir, mut db) = db().await;
    // The first account of an older píxiū was named by its email.
    let old = toasty::create!(User {
        username: "me@example.com",
        password_hash: "$argon2id$old",
        role: Role::Admin,
        status: UserStatus::Active,
        password_change_required: false,
        created_at: now(),
    })
    .exec(&mut db)
    .await
    .unwrap();

    let old = users::set_profile(&mut db, old, "me@example.com", "me@example.com", true)
        .await
        .unwrap();
    assert_eq!(old.username, "me@example.com");
    assert_eq!(old.email.as_deref(), Some("me@example.com"));
    // A new name follows the rules.
    let id = old.id;
    assert!(matches!(
        users::set_profile(&mut db, old, "you@example.com", "", false).await,
        Err(AccountError::Invalid(_))
    ));
    let old = User::get_by_id(&mut db, &id).await.unwrap();
    let renamed = users::set_profile(&mut db, old, "me", "me@example.com", false)
        .await
        .unwrap();
    assert_eq!(renamed.username, "me");
}
