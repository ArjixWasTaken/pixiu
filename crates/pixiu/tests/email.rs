//! Email through the web API: admins set up the mail server, people reset
//! forgotten passwords, confirm their addresses, choose their alerts and
//! ask for accounts. Emails are kept in memory instead of sent.

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use pixiu_accounts::{Email, MemoryTransport};
use pixiu_core::{Config, SecretBox};
use serde_json::{Value, json};
use tower::ServiceExt;

struct Server {
    _dir: tempfile::TempDir,
    router: Router,
    mail: MemoryTransport,
}

impl Server {
    async fn new() -> Self {
        Self::with_config(|_| {}).await
    }

    async fn with_config(change: impl FnOnce(&mut Config)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.paths.data_dir = dir.path().join("data");
        config.paths.treasure_dir = dir.path().join("treasure");
        change(&mut config);
        let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let mail = MemoryTransport::default();
        let services = pixiu::Services::with_transport(
            db,
            &config,
            SecretBox::ephemeral(),
            Arc::new(mail.clone()),
        )
        .await
        .unwrap();
        services.mailer.start();
        let web = dir.path().join("web");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(web.join("index.html"), "<!doctype html>").unwrap();
        let router = pixiu::app(&services, &config, &web);
        Self {
            _dir: dir,
            router,
            mail,
        }
    }

    async fn request(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn call(
        &self,
        token: Option<&str>,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        }
        .unwrap();
        self.request(request).await
    }

    async fn post(&self, token: Option<&str>, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(token, Method::POST, path, Some(body)).await
    }

    async fn login(&self, login: &str, password: &str) -> (StatusCode, Value) {
        self.post(
            None,
            "/api/auth/login",
            json!({ "username": login, "password": password }),
        )
        .await
    }

    async fn token(&self, login: &str, password: &str) -> String {
        let (status, body) = self.login(login, password).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }

    /// Sets píxiū up with Alice as admin; her token.
    async fn setup(&self) -> String {
        let (status, body) = self
            .post(
                None,
                "/api/auth/setup",
                json!({ "username": "alice", "password": "alice's secret" }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }

    /// Sets the public address and a mail server up.
    async fn mail_ready(&self, admin: &str) {
        let (status, _) = self
            .call(
                Some(admin),
                Method::PUT,
                "/api/admin/settings/server",
                Some(json!({ "public_url": "https://music.example.com" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = self
            .call(
                Some(admin),
                Method::PUT,
                "/api/admin/settings/smtp",
                Some(json!({
                    "host": "mail.example.com",
                    "port": 587,
                    "security": "starttls",
                    "username": "pixiu",
                    "password": "the smtp password",
                    "from": "píxiū <no-reply@example.com>",
                })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    /// Makes Bob, with an address the admin typed (so confirmed); his id.
    async fn bob(&self, admin: &str) -> u64 {
        let (status, body) = self
            .post(
                Some(admin),
                "/api/admin/users",
                json!({
                    "username": "bob",
                    "email": "bob@example.com",
                    "password": "bob's own secret",
                    "role": "user",
                }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["id"].as_u64().unwrap()
    }

    /// Waits for `count` emails to have gone out.
    async fn sent(&self, count: usize) -> Vec<Email> {
        for _ in 0..200 {
            if self.mail.sent().len() >= count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        self.mail.sent()
    }
}

/// The token at the end of an email's link.
fn token_in(email: &Email) -> String {
    let link = email
        .text
        .split_whitespace()
        .find(|word| word.starts_with("https://music.example.com/"))
        .expect("a link");
    link.rsplit('/').next().unwrap().to_owned()
}

#[tokio::test]
async fn admins_set_up_email_and_never_see_its_password_again() {
    let server = Server::new().await;
    let admin = server.setup().await;
    let (_, status) = server
        .call(None, Method::GET, "/api/auth/status", None)
        .await;
    assert_eq!(status["password_reset"], false);

    let (_, settings) = server
        .call(Some(&admin), Method::GET, "/api/admin/settings", None)
        .await;
    assert_eq!(settings["smtp"], Value::Null);
    assert_eq!(settings["mail_ready"], false);
    let (status, body) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/admin/settings/server",
            Some(json!({ "public_url": "music.example.com" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    server.mail_ready(&admin).await;
    let (_, settings) = server
        .call(Some(&admin), Method::GET, "/api/admin/settings", None)
        .await;
    assert_eq!(settings["mail_ready"], true);
    assert_eq!(settings["smtp"]["password_set"], true);
    assert_eq!(settings["smtp"]["security"], "starttls");
    assert!(!settings.to_string().contains("the smtp password"));
    let (_, status) = server
        .call(None, Method::GET, "/api/auth/status", None)
        .await;
    assert_eq!(status["password_reset"], true);

    // Saving again without a password keeps it; an empty one removes it.
    let form = json!({
        "host": "mail.example.com",
        "port": 25,
        "security": "none",
        "username": "pixiu",
        "from": "píxiū <no-reply@example.com>",
    });
    let (status, settings) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/admin/settings/smtp",
            Some(form.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (&settings["smtp"]["port"], &settings["smtp"]["password_set"]),
        (&json!(25), &json!(true))
    );
    let mut cleared = form;
    cleared["password"] = json!("");
    let (_, settings) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/admin/settings/smtp",
            Some(cleared),
        )
        .await;
    assert_eq!(settings["smtp"]["password_set"], false);

    // The test email goes to the address named, or the admin's own.
    let (status, body) = server
        .post(Some(&admin), "/api/admin/settings/smtp/test", json!({}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let (status, _) = server
        .post(
            Some(&admin),
            "/api/admin/settings/smtp/test",
            json!({ "to": "alice@example.com" }),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(server.mail.sent()[0].to, "alice@example.com");

    // Only admins.
    server.bob(&admin).await;
    let bob = server.token("bob", "bob's own secret").await;
    for (method, path) in [
        (Method::GET, "/api/admin/settings"),
        (Method::PUT, "/api/admin/settings/server"),
        (Method::DELETE, "/api/admin/settings/smtp"),
        (Method::POST, "/api/admin/settings/smtp/test"),
    ] {
        let (status, _) = server
            .call(Some(&bob), method.clone(), path, Some(json!({})))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }

    let (status, settings) = server
        .call(
            Some(&admin),
            Method::DELETE,
            "/api/admin/settings/smtp",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(settings["mail_ready"], false);
}

#[tokio::test]
async fn forgotten_passwords_are_reset_by_email() {
    let server = Server::new().await;
    let admin = server.setup().await;
    server.mail_ready(&admin).await;
    server.bob(&admin).await;
    let before = server.token("bob", "bob's own secret").await;

    // Nobody learns which accounts exist.
    for login in ["nobody", "nobody@example.com"] {
        let (status, _) = server
            .post(None, "/api/auth/forgot", json!({ "login": login }))
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    let (status, _) = server
        .post(
            None,
            "/api/auth/forgot",
            json!({ "login": "BOB@example.com" }),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let sent = server.sent(1).await;
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].to, "bob@example.com");
    let token = token_in(&sent[0]);

    // A password that breaks the rules leaves the link working.
    let (status, _) = server
        .post(
            None,
            "/api/auth/reset",
            json!({ "token": token, "password": "short" }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, body) = server
        .post(
            None,
            "/api/auth/reset",
            json!({ "token": token, "password": "a fresh password" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let after = body["token"].as_str().unwrap();
    let (status, _) = server.call(Some(after), Method::GET, "/api/me", None).await;
    assert_eq!(status, StatusCode::OK);

    // Web sessions from before sign out; the old password is gone.
    let (status, _) = server
        .call(Some(&before), Method::GET, "/api/me", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        server.login("bob", "bob's own secret").await.0,
        StatusCode::UNAUTHORIZED
    );
    server.token("bob", "a fresh password").await;

    // The link works once.
    let (status, body) = server
        .post(
            None,
            "/api/auth/reset",
            json!({ "token": token, "password": "yet another one" }),
        )
        .await;
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(body["code"], "expired");

    // An admin can send the link too.
    let bob_id = server.bob_id(&admin).await;
    let (status, _) = server
        .post(
            Some(&admin),
            &format!("/api/admin/users/{bob_id}/password-reset"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(server.sent(2).await.len(), 2);
}

impl Server {
    async fn bob_id(&self, admin: &str) -> u64 {
        let (_, users) = self
            .call(Some(admin), Method::GET, "/api/admin/users", None)
            .await;
        users
            .as_array()
            .unwrap()
            .iter()
            .find(|user| user["username"] == "bob")
            .unwrap()["id"]
            .as_u64()
            .unwrap()
    }
}

#[tokio::test]
async fn new_addresses_are_confirmed_by_link() {
    let server = Server::new().await;
    let admin = server.setup().await;
    server.mail_ready(&admin).await;
    server.bob(&admin).await;
    let bob = server.token("bob", "bob's own secret").await;

    let (status, me) = server
        .call(
            Some(&bob),
            Method::PUT,
            "/api/me",
            Some(json!({ "username": "bob", "email": "bob@example.org" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["email_verified"], false);
    let sent = server.sent(1).await;
    assert_eq!(sent[0].to, "bob@example.org");
    assert!(sent[0].text.contains("/verify-email/"));

    // Until confirmed, a forgotten password gets no link there.
    server
        .post(None, "/api/auth/forgot", json!({ "login": "bob" }))
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(server.mail.sent().len(), 1);

    // Sending it again replaces the first link.
    let (status, _) = server
        .post(Some(&bob), "/api/me/email/resend", json!({}))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let sent = server.sent(2).await;
    let (first, second) = (token_in(&sent[0]), token_in(&sent[1]));
    let (status, _) = server
        .post(None, "/api/auth/verify-email", json!({ "token": first }))
        .await;
    assert_eq!(status, StatusCode::GONE);
    let (status, body) = server
        .post(None, "/api/auth/verify-email", json!({ "token": second }))
        .await;
    assert_eq!(
        (status, &body),
        (StatusCode::OK, &json!({ "opened": false }))
    );
    let (_, me) = server.call(Some(&bob), Method::GET, "/api/me", None).await;
    assert_eq!(me["email_verified"], true);
    let (status, _) = server
        .post(Some(&bob), "/api/me/email/resend", json!({}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn people_choose_their_alerts() {
    let server = Server::new().await;
    let admin = server.setup().await;
    let (_, alerts) = server
        .call(Some(&admin), Method::GET, "/api/me/alerts", None)
        .await;
    assert_eq!(
        alerts,
        json!({
            "deliverable": false,
            "alerts": { "youtube_music_expired": true, "watch_failing": true },
        })
    );
    let (status, alerts) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/me/alerts",
            Some(json!({ "watch_failing": false })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        alerts["alerts"],
        json!({ "youtube_music_expired": true, "watch_failing": false })
    );
    let (status, _) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/me/alerts",
            Some(json!({ "sunrise": true })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// Several threads, as in the server: reset emails go out from tasks that
// overlap, and SQLite blocks the thread of whichever waits for the lock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn guessing_and_flooding_are_throttled() {
    let server = Server::with_config(|config| config.server.trust_proxy_headers = true).await;
    let admin = server.setup().await;
    server.mail_ready(&admin).await;
    server.bob(&admin).await;
    let from = |ip: &str, body: Value| {
        Request::builder()
            .method(Method::POST)
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", format!("10.0.0.1, {ip}"))
            .body(Body::from(body.to_string()))
            .unwrap()
    };

    // Twenty wrong passwords from one address, and it has to wait, even
    // with the right one; other addresses may still sign in.
    for _ in 0..20 {
        let (status, _) = server
            .request(from(
                "203.0.113.7",
                json!({ "username": "bob", "password": "a guess" }),
            ))
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let right = json!({ "username": "bob", "password": "bob's own secret" });
    let (status, _) = server.request(from("203.0.113.7", right.clone())).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let (status, _) = server.request(from("203.0.113.8", right)).await;
    assert_eq!(status, StatusCode::OK);

    // At most three reset emails an hour per account.
    for _ in 0..5 {
        let (status, _) = server
            .post(None, "/api/auth/forgot", json!({ "login": "bob" }))
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.sent(3).await.len(), 3);
}

impl Server {
    /// Opens registration, with Alice (the admin) at a confirmed address.
    async fn open_registration(&self, admin: &str) {
        self.mail_ready(admin).await;
        let (status, _) = self
            .call(
                Some(admin),
                Method::PATCH,
                "/api/admin/users/1",
                Some(json!({ "email": "alice@example.com" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = self
            .call(
                Some(admin),
                Method::PUT,
                "/api/admin/settings/registration",
                Some(json!({ "open": true })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    async fn register(&self, username: &str, email: &str) -> (StatusCode, Value) {
        self.post(
            None,
            "/api/auth/register",
            json!({ "username": username, "email": email, "password": "carol's secret" }),
        )
        .await
    }

    /// A Subsonic call with a password; the error code, if any.
    async fn subsonic_error(&self, user: &str, password: &str) -> Option<i64> {
        let uri = format!("/rest/ping?u={user}&p={password}&v=1.16.1&c=test&f=json");
        let (_, body) = self
            .call(
                None,
                Method::GET,
                &uri.replace(' ', "%20").replace('\'', "%27"),
                None,
            )
            .await;
        body["subsonic-response"]["error"]["code"].as_i64()
    }
}

#[tokio::test]
async fn registrations_wait_for_an_admin() {
    let server = Server::new().await;
    let admin = server.setup().await;

    // Closed until an admin opens it, which needs email.
    assert_eq!(
        server.register("carol", "carol@example.com").await.0,
        StatusCode::NOT_FOUND
    );
    let (status, _) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/admin/settings/registration",
            Some(json!({ "open": true })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    server.open_registration(&admin).await;
    let (_, status) = server
        .call(None, Method::GET, "/api/auth/status", None)
        .await;
    assert_eq!(status["registration"], true);

    // Carol asks; the admin hears of it.
    let (status, body) = server.register("carol", "Carol@Example.com").await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    let sent = server.sent(1).await;
    assert_eq!(sent[0].to, "alice@example.com");
    assert!(sent[0].subject.contains("carol"), "{}", sent[0].subject);
    assert!(sent[0].text.contains("/settings?tab=users"));
    let (_, summary) = server
        .call(Some(&admin), Method::GET, "/api/hunting", None)
        .await;
    assert_eq!(summary["registrations"], 1);

    // She cannot sign in yet, anywhere.
    let (status, body) = server.login("carol", "carol's secret").await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!("pending"))
    );
    assert_eq!(
        server.subsonic_error("carol", "carol's secret").await,
        Some(50)
    );

    // Approved, she confirms her address and is in.
    let (_, users) = server
        .call(Some(&admin), Method::GET, "/api/admin/users", None)
        .await;
    let carol = users
        .as_array()
        .unwrap()
        .iter()
        .find(|user| user["username"] == "carol")
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    let (status, body) = server
        .post(
            Some(&admin),
            &format!("/api/admin/registrations/{carol}/approve"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "unverified");
    let (status, _) = server
        .post(
            Some(&admin),
            &format!("/api/admin/registrations/{carol}/approve"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "handled already");
    let approval = server.sent(2).await.pop().unwrap();
    assert_eq!(approval.to, "carol@example.com");
    assert!(
        approval.subject.contains("approved"),
        "{}",
        approval.subject
    );
    let (status, body) = server.login("carol", "carol's secret").await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::FORBIDDEN, &json!("unverified"))
    );

    let (status, body) = server
        .post(
            None,
            "/api/auth/verify-email",
            json!({ "token": token_in(&approval) }),
        )
        .await;
    assert_eq!(
        (status, &body),
        (StatusCode::OK, &json!({ "opened": true }))
    );
    server.token("carol", "carol's secret").await;
    assert_eq!(server.subsonic_error("carol", "carol's secret").await, None);
    let (_, summary) = server
        .call(Some(&admin), Method::GET, "/api/hunting", None)
        .await;
    assert_eq!(summary["registrations"], 0);
}

#[tokio::test]
async fn denied_registrations_are_gone() {
    let server = Server::new().await;
    let admin = server.setup().await;
    server.open_registration(&admin).await;
    server.register("dave", "dave@example.com").await;
    let (_, users) = server
        .call(Some(&admin), Method::GET, "/api/admin/users", None)
        .await;
    let dave = users.as_array().unwrap().last().unwrap()["id"]
        .as_u64()
        .unwrap();

    let (status, _) = server
        .post(
            Some(&admin),
            &format!("/api/admin/registrations/{dave}/deny"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let note = server.sent(2).await.pop().unwrap();
    assert_eq!(note.to, "dave@example.com");
    assert!(note.text.contains("declined"), "{}", note.text);
    assert_eq!(
        server.login("dave", "carol's secret").await.0,
        StatusCode::UNAUTHORIZED
    );
    // Only requests are approved or denied.
    let (status, _) = server
        .post(Some(&admin), "/api/admin/registrations/1/deny", json!({}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn nobody_learns_whose_address_has_an_account() {
    let server = Server::new().await;
    let admin = server.setup().await;
    server.open_registration(&admin).await;
    server.bob(&admin).await;

    // Bob's address: the same answer, no account, and a note to Bob.
    let (status, _) = server.register("mallory", "BOB@example.com").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let note = server.sent(1).await.pop().unwrap();
    assert_eq!(note.to, "bob@example.com");
    assert!(note.subject.contains("your email"), "{}", note.subject);
    let (_, users) = server
        .call(Some(&admin), Method::GET, "/api/admin/users", None)
        .await;
    assert_eq!(users.as_array().unwrap().len(), 2);

    // A taken username says so: people need to pick another.
    let (status, _) = server.register("bob", "someone@example.com").await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Five requests an hour from one address.
    for n in 0..3 {
        let (status, body) = server
            .register(&format!("user{n}"), &format!("user{n}@example.com"))
            .await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    }
    assert_eq!(
        server.register("user9", "user9@example.com").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );

    // Without email, registration closes by itself.
    server
        .call(
            Some(&admin),
            Method::DELETE,
            "/api/admin/settings/smtp",
            None,
        )
        .await;
    let (_, status) = server
        .call(None, Method::GET, "/api/auth/status", None)
        .await;
    assert_eq!(status["registration"], false);
}
