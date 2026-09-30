//! Email: the settings it needs, the outbox, single-use links and alerts.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use jiff::SignedDuration;
use pixiu_accounts::{
    Email, MailTransport, Mailer, MemoryTransport, NewUser, Security, Settings, Smtp,
    alerts::{self, MailAlerts},
    links,
    mail::BoxFuture,
    tokens, users,
};
use pixiu_core::{
    SecretBox,
    alerts::{Alert, AlertKind, AlertSink},
};
use pixiu_db::{AccountToken, Db, Role, Setting, TokenPurpose, User, UserStatus, now, toasty};

struct Setup {
    _dir: tempfile::TempDir,
    db: Db,
    secrets: SecretBox,
    settings: Arc<Settings>,
}

async fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
    let secrets = SecretBox::ephemeral();
    let settings = Settings::load(db.clone(), secrets.clone()).await.unwrap();
    Setup {
        _dir: dir,
        db,
        secrets,
        settings,
    }
}

fn smtp() -> Smtp {
    Smtp {
        host: "mail.example.com".to_owned(),
        port: 587,
        security: Security::StartTls,
        username: Some("pixiu".to_owned()),
        password: Some("hunter2 but longer".to_owned()),
        from: "píxiū <no-reply@example.com>".to_owned(),
    }
}

impl Setup {
    async fn mail_ready(&self) {
        self.settings
            .set_public_url(Some("https://music.example.com/"))
            .await
            .unwrap();
        self.settings.set_smtp(Some(smtp())).await.unwrap();
    }

    async fn user(&self, username: &str, email: Option<&str>, verified: bool) -> User {
        users::create(
            &mut self.db.clone(),
            &self.secrets,
            NewUser {
                username: username.to_owned(),
                email: email.map(str::to_owned),
                email_verified: verified,
                password: "long enough".to_owned(),
                role: Role::User,
                status: UserStatus::Active,
                password_change_required: false,
            },
        )
        .await
        .unwrap()
    }
}

/// Waits for the outbox to have sent `count` emails.
async fn sent(transport: &MemoryTransport, count: usize) -> Vec<Email> {
    for _ in 0..200 {
        let sent = transport.sent();
        if sent.len() >= count {
            return sent;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    transport.sent()
}

/// The link's last path segment, from an email's text.
fn token_in(email: &Email) -> String {
    let link = email
        .text
        .split_whitespace()
        .find(|word| word.starts_with("https://"))
        .expect("a link");
    link.rsplit('/').next().unwrap().to_owned()
}

#[tokio::test]
async fn settings_persist_with_the_password_sealed() {
    let s = setup().await;
    assert!(!s.settings.get().mail_ready());
    assert!(
        s.settings
            .set_public_url(Some("music.example.com"))
            .await
            .is_err()
    );
    s.mail_ready().await;
    let settings = s.settings.get();
    assert_eq!(
        settings.public_url.as_deref(),
        Some("https://music.example.com")
    );
    assert!(settings.mail_ready());

    let stored: Vec<String> = Setting::all()
        .exec(&mut s.db.clone())
        .await
        .unwrap()
        .into_iter()
        .map(|setting| setting.value)
        .collect();
    assert!(
        stored
            .iter()
            .any(|value| value.contains("mail.example.com"))
    );
    assert!(
        stored.iter().all(|value| !value.contains("hunter2")),
        "{stored:?}"
    );

    // A restart reads them back, the password opened again.
    let again = Settings::load(s.db.clone(), s.secrets.clone())
        .await
        .unwrap();
    assert_eq!(*again.get(), *settings);

    s.settings.set_smtp(None).await.unwrap();
    assert!(!s.settings.get().mail_ready());
    let again = Settings::load(s.db.clone(), s.secrets.clone())
        .await
        .unwrap();
    assert_eq!(again.get().smtp, None);
}

/// Fails the first `failures` sends.
struct Flaky {
    failures: Mutex<u32>,
    inner: MemoryTransport,
}

impl MailTransport for Flaky {
    fn send<'a>(
        &'a self,
        smtp: &'a Smtp,
        hello: Option<&'a str>,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<(), String>> {
        let mut failures = self.failures.lock().unwrap();
        if *failures > 0 {
            *failures -= 1;
            return Box::pin(async { Err("421 try again later".to_owned()) });
        }
        drop(failures);
        self.inner.send(smtp, hello, email)
    }
}

#[tokio::test(start_paused = true)]
async fn the_outbox_tries_again() {
    let s = setup().await;
    s.mail_ready().await;
    let inner = MemoryTransport::default();
    let mailer = Mailer::new(
        Arc::clone(&s.settings),
        Arc::new(Flaky {
            failures: Mutex::new(2),
            inner: inner.clone(),
        }),
    );
    mailer.start();
    mailer.queue(pixiu_accounts::mail::templates::test("alice@example.com"));
    // Time is paused, so waiting out the backoff takes no real time.
    tokio::time::sleep(Duration::from_secs(120)).await;
    assert_eq!(inner.sent().len(), 1);

    // Three failures in a row, and it gives up.
    let inner = MemoryTransport::default();
    let mailer = Mailer::new(
        Arc::clone(&s.settings),
        Arc::new(Flaky {
            failures: Mutex::new(3),
            inner: inner.clone(),
        }),
    );
    mailer.start();
    mailer.queue(pixiu_accounts::mail::templates::test("alice@example.com"));
    tokio::time::sleep(Duration::from_secs(120)).await;
    assert!(inner.sent().is_empty());
}

#[tokio::test]
async fn links_work_once_and_expire() {
    let s = setup().await;
    let mut db = s.db.clone();
    let bob = s.user("bob", Some("bob@example.com"), true).await;

    let token = tokens::issue(&mut db, bob.id, TokenPurpose::ResetPassword, None)
        .await
        .unwrap();
    // Not for another purpose.
    assert_eq!(
        tokens::redeem(&mut db, &token, TokenPurpose::VerifyEmail)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        tokens::redeem(&mut db, &token, TokenPurpose::ResetPassword)
            .await
            .unwrap()
            .map(|redeemed| redeemed.user_id),
        Some(bob.id)
    );
    assert_eq!(
        tokens::redeem(&mut db, &token, TokenPurpose::ResetPassword)
            .await
            .unwrap(),
        None,
        "used up"
    );

    // A new link replaces the older ones.
    let older = tokens::issue(&mut db, bob.id, TokenPurpose::ResetPassword, None)
        .await
        .unwrap();
    let newer = tokens::issue(&mut db, bob.id, TokenPurpose::ResetPassword, None)
        .await
        .unwrap();
    assert!(
        tokens::redeem(&mut db, &older, TokenPurpose::ResetPassword)
            .await
            .unwrap()
            .is_none()
    );

    // Past its hour, it no longer works.
    let mut stored = AccountToken::filter_by_user_id(bob.id)
        .first()
        .exec(&mut db)
        .await
        .unwrap()
        .unwrap();
    toasty::update!(stored {
        expires_at: now() - SignedDuration::from_secs(1),
    })
    .exec(&mut db)
    .await
    .unwrap();
    assert!(
        tokens::redeem(&mut db, &newer, TokenPurpose::ResetPassword)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        AccountToken::all().exec(&mut db).await.unwrap().is_empty(),
        "and it is gone"
    );
}

#[tokio::test]
async fn following_links_confirms_addresses_and_resets_passwords() {
    let s = setup().await;
    s.mail_ready().await;
    let mut db = s.db.clone();
    let transport = MemoryTransport::default();
    let mailer = Mailer::new(Arc::clone(&s.settings), Arc::new(transport.clone()));
    mailer.start();

    // Confirming an address.
    let bob = s.user("bob", Some("bob@example.com"), false).await;
    assert!(
        !links::may_reset(&bob),
        "an unconfirmed address gets no reset link"
    );
    links::send_verification(&mut db, &mailer, &bob)
        .await
        .unwrap();
    let email = sent(&transport, 1).await.pop().unwrap();
    assert_eq!(email.to, "bob@example.com");
    assert!(
        email
            .text
            .contains("https://music.example.com/verify-email/"),
        "{}",
        email.text
    );
    let bob = links::verify_email(&mut db, &token_in(&email))
        .await
        .unwrap();
    assert!(bob.email_verified_at.is_some());
    assert!(links::may_reset(&bob));
    assert!(matches!(
        links::verify_email(&mut db, &token_in(&email)).await,
        Err(pixiu_accounts::AccountError::LinkExpired)
    ));

    // A link for an address the account no longer has does nothing.
    let carol = s.user("carol", Some("carol@example.com"), false).await;
    links::send_verification(&mut db, &mailer, &carol)
        .await
        .unwrap();
    let email = sent(&transport, 2).await.pop().unwrap();
    users::set_profile(&mut db, carol, "carol", "carol@example.org", false)
        .await
        .unwrap();
    assert!(
        links::verify_email(&mut db, &token_in(&email))
            .await
            .is_err()
    );

    // Resetting a password: a bad one leaves the link working.
    links::send_reset(&mut db, &mailer, &bob).await.unwrap();
    let email = sent(&transport, 3).await.pop().unwrap();
    assert!(
        email
            .text
            .contains("https://music.example.com/reset-password/")
    );
    let token = token_in(&email);
    assert!(
        links::reset_password(&mut db, &s.secrets, &token, "short")
            .await
            .is_err()
    );
    let bob = links::reset_password(&mut db, &s.secrets, &token, "a brand new one")
        .await
        .unwrap();
    assert!(pixiu_core::password::verify(
        "a brand new one",
        Some(&bob.password_hash)
    ));
    assert!(
        links::reset_password(&mut db, &s.secrets, &token, "and another one")
            .await
            .is_err()
    );

    // No mail server, no links.
    s.settings.set_smtp(None).await.unwrap();
    assert!(links::send_reset(&mut db, &mailer, &bob).await.is_err());
}

#[tokio::test]
async fn alerts_go_out_once_to_those_who_want_them() {
    let s = setup().await;
    s.mail_ready().await;
    let mut db = s.db.clone();
    let transport = MemoryTransport::default();
    let mailer = Mailer::new(Arc::clone(&s.settings), Arc::new(transport.clone()));
    mailer.start();
    let alerts = MailAlerts {
        db: s.db.clone(),
        mailer: Arc::clone(&mailer),
    };
    let bob = s.user("bob", Some("bob@example.com"), true).await;
    let expired = |at| Alert::YouTubeMusicExpired {
        expired_at: at,
        reason: "the browser profile is no longer logged in".to_owned(),
    };
    let first = now();

    alerts.alert(bob.id, expired(first)).await;
    let email = sent(&transport, 1).await.pop().unwrap();
    assert_eq!(email.to, "bob@example.com");
    assert!(
        email
            .text
            .contains("https://music.example.com/settings?tab=youtube-music"),
        "{}",
        email.text
    );

    // The same expiry again (a restart, say): nothing new.
    alerts.alert(bob.id, expired(first)).await;
    // A later expiry is news.
    let second = first + SignedDuration::from_hours(1);
    alerts.alert(bob.id, expired(second)).await;
    assert_eq!(sent(&transport, 2).await.len(), 2);

    // Switched off, it stays quiet; other kinds still come.
    alerts::set_wanted(
        &mut db,
        bob.id,
        &[(AlertKind::YouTubeMusicExpired, false)].into(),
    )
    .await
    .unwrap();
    let wanted = alerts::wanted(&mut db, bob.id).await.unwrap();
    assert!(!wanted[&AlertKind::YouTubeMusicExpired]);
    assert!(wanted[&AlertKind::WatchFailing]);
    alerts
        .alert(bob.id, expired(second + SignedDuration::from_hours(1)))
        .await;
    alerts
        .alert(
            bob.id,
            Alert::WatchFailing {
                watch_id: 1,
                name: "Liked music".to_owned(),
                error: "gone".to_owned(),
                since: first,
            },
        )
        .await;
    let emails = sent(&transport, 3).await;
    assert_eq!(emails.len(), 3);
    assert!(
        emails[2].subject.contains("Liked music"),
        "{}",
        emails[2].subject
    );

    // Nobody hears without a confirmed address, or with the account off.
    let carol = s.user("carol", Some("carol@example.com"), false).await;
    alerts.alert(carol.id, expired(first)).await;
    let dave = s.user("dave", Some("dave@example.com"), true).await;
    users::set_status(&mut db, dave.id, UserStatus::Disabled)
        .await
        .unwrap();
    alerts.alert(dave.id, expired(first)).await;
    // Nor without a mail server.
    let erin = s.user("erin", Some("erin@example.com"), true).await;
    s.settings.set_smtp(None).await.unwrap();
    alerts.alert(erin.id, expired(first)).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(transport.sent().len(), 3);
}
