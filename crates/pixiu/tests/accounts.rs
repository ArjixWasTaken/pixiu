//! Accounts through the web API: admins make users, change roles, turn
//! accounts off, hand out temporary passwords and delete accounts; users
//! manage their own profile, password and keys.

// Tests inspect whole tables; only handlers must go through a `Library`.
#![allow(clippy::disallowed_methods)]

use std::path::Path;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use pixiu_core::{Config, SecretBox};
use pixiu_db::{AudioFile, Db, Track, User};
use pixiu_treasury::{Claim, Provenance, tags};
use serde_json::{Value, json};
use tower::ServiceExt;

struct Server {
    dir: tempfile::TempDir,
    db: Db,
    services: pixiu::Services,
    router: Router,
}

impl Server {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.paths.data_dir = dir.path().join("data");
        config.paths.treasure_dir = dir.path().join("treasure");
        let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let services = pixiu::Services::new(db.clone(), &config, SecretBox::ephemeral())
            .await
            .unwrap();
        let web = dir.path().join("web");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(web.join("index.html"), "<!doctype html>").unwrap();
        let router = pixiu::app(&services, &config, &web);
        Self {
            dir,
            db,
            services,
            router,
        }
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
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn login(&self, login: &str, password: &str) -> (StatusCode, Value) {
        self.call(
            None,
            Method::POST,
            "/api/auth/login",
            Some(json!({ "username": login, "password": password })),
        )
        .await
    }

    async fn token(&self, login: &str, password: &str) -> String {
        let (status, body) = self.login(login, password).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }

    async fn setup(&self) -> String {
        let (status, body) = self
            .call(
                None,
                Method::POST,
                "/api/auth/setup",
                Some(json!({ "username": "alice", "password": "alice's secret" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }

    /// A Subsonic call as `user` with `password`; the answer's status and
    /// error code.
    async fn subsonic(&self, user: &str, password: &str) -> (String, Option<i64>) {
        let uri = format!("/rest/ping?u={user}&p={password}&v=1.16.1&c=test&f=json");
        let (_, body) = self
            .call(None, Method::GET, &uri.replace(' ', "%20"), None)
            .await;
        let response = &body["subsonic-response"];
        (
            response["status"].as_str().unwrap_or_default().to_owned(),
            response["error"]["code"].as_i64(),
        )
    }

    async fn ingest(&self, owner: u64, name: &str) -> Track {
        let staged = self.dir.path().join(format!("{owner}-{name}"));
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/audio")
                .join(name),
            &staged,
        )
        .unwrap();
        let info = tags::read(&staged).unwrap();
        self.services
            .treasury
            .ingest(
                owner,
                &staged,
                &info,
                None,
                Provenance::offering(name, None),
                Claim::offering(),
            )
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn admins_make_and_manage_accounts() {
    let server = Server::new().await;
    let admin = server.setup().await;
    let (_, me) = server
        .call(Some(&admin), Method::GET, "/api/me", None)
        .await;
    assert_eq!(
        (me["role"].as_str(), me["status"].as_str()),
        (Some("admin"), Some("active"))
    );

    // A new account, with a temporary password and an email the admin typed.
    let (status, bob) = server
        .call(
            Some(&admin),
            Method::POST,
            "/api/admin/users",
            Some(
                json!({ "username": "bob", "email": "Bob@Example.com", "password": "temporary!" }),
            ),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{bob}");
    assert_eq!(bob["email"], "bob@example.com");
    assert_eq!(bob["email_verified"], true);
    assert_eq!(bob["password_change_required"], true);
    let bob_id = bob["id"].as_u64().unwrap();
    let (status, _) = server
        .call(
            Some(&admin),
            Method::POST,
            "/api/admin/users",
            Some(json!({ "username": "BOB", "password": "temporary!" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Bob signs in by email, and must choose a password first.
    let bob_token = server.token("bob@example.com", "temporary!").await;
    let (_, bootstrap) = server
        .call(Some(&bob_token), Method::GET, "/api/bootstrap", None)
        .await;
    assert_eq!(bootstrap["current_user"]["password_change_required"], true);
    assert_eq!(bootstrap["current_user"]["role"], "user");
    assert_eq!(
        bootstrap["current_user"]["abilities"],
        json!(["manage songs"])
    );
    let (status, _) = server
        .call(
            Some(&bob_token),
            Method::PUT,
            "/api/me/password",
            Some(json!({ "password": "bob's own!" })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, me) = server
        .call(Some(&bob_token), Method::GET, "/api/me", None)
        .await;
    assert_eq!(me["password_change_required"], false);
    // Now changing it again needs the current one.
    let (status, _) = server
        .call(
            Some(&bob_token),
            Method::PUT,
            "/api/me/password",
            Some(json!({ "current_password": "wrong", "password": "another one" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Bob is not an admin.
    for (method, path) in [
        (Method::GET, "/api/admin/users".to_owned()),
        (Method::GET, "/api/admin/storage".to_owned()),
        (Method::PATCH, format!("/api/admin/users/{bob_id}")),
        (Method::DELETE, "/api/admin/users/1".to_owned()),
    ] {
        let (status, body) = server
            .call(Some(&bob_token), method.clone(), &path, Some(json!({})))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {body}");
    }

    // The last admin cannot step down, be turned off or delete themselves.
    let (status, body) = server
        .call(
            Some(&admin),
            Method::PATCH,
            "/api/admin/users/1",
            Some(json!({ "role": "user" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, _) = server
        .call(
            Some(&admin),
            Method::PATCH,
            "/api/admin/users/1",
            Some(json!({ "status": "disabled" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = server
        .call(Some(&admin), Method::DELETE, "/api/admin/users/1", None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // With Bob an admin too, she may.
    let (status, bob) = server
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/admin/users/{bob_id}"),
            Some(json!({ "role": "admin" })),
        )
        .await;
    assert_eq!(
        (status, bob["role"].as_str()),
        (StatusCode::OK, Some("admin"))
    );
    let (status, _) = server
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/admin/users/{bob_id}"),
            Some(json!({ "role": "user" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (_, list) = server
        .call(Some(&admin), Method::GET, "/api/admin/users", None)
        .await;
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|user| user["username"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["alice", "bob"]);
}

#[tokio::test]
async fn display_names_stand_in_for_usernames() {
    let server = Server::new().await;
    let admin = server.setup().await;
    let name = |bootstrap: &Value| {
        bootstrap["current_user"]["name"]
            .as_str()
            .map(str::to_owned)
    };
    let (_, bootstrap) = server
        .call(Some(&admin), Method::GET, "/api/bootstrap", None)
        .await;
    let username = bootstrap["current_user"]["username"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(name(&bootstrap), Some(username.clone()));

    let (status, me) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/me",
            Some(json!({ "username": username, "display_name": "  Arjix  " })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["display_name"], "Arjix");
    let (_, bootstrap) = server
        .call(Some(&admin), Method::GET, "/api/bootstrap", None)
        .await;
    assert_eq!(name(&bootstrap).as_deref(), Some("Arjix"));
    assert_eq!(bootstrap["current_user"]["username"], username.as_str());

    // Leaving it out keeps it; a bad one changes nothing; blank clears it.
    let (_, me) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/me",
            Some(json!({ "username": username })),
        )
        .await;
    assert_eq!(me["display_name"], "Arjix");
    let (status, _) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/me",
            Some(json!({ "username": "renamed", "display_name": "x".repeat(65) })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, me) = server
        .call(Some(&admin), Method::GET, "/api/me", None)
        .await;
    assert_eq!(me["username"], username.as_str());
    let (_, me) = server
        .call(
            Some(&admin),
            Method::PUT,
            "/api/me",
            Some(json!({ "username": username, "display_name": " " })),
        )
        .await;
    assert_eq!(me["display_name"], Value::Null);

    // Admins name others too.
    let (_, bob) = server
        .call(
            Some(&admin),
            Method::POST,
            "/api/admin/users",
            Some(json!({ "username": "bob", "password": "temporary!" })),
        )
        .await;
    let (status, bob) = server
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/admin/users/{}", bob["id"]),
            Some(json!({ "display_name": "Bob B." })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{bob}");
    assert_eq!(bob["display_name"], "Bob B.");
    assert_eq!(bob["username"], "bob");
}

#[tokio::test]
async fn turned_off_accounts_are_signed_out_everywhere() {
    let server = Server::new().await;
    let admin = server.setup().await;
    let (_, bob) = server
        .call(
            Some(&admin),
            Method::POST,
            "/api/admin/users",
            Some(json!({ "username": "bob", "password": "temporary!" })),
        )
        .await;
    let bob_id = bob["id"].as_u64().unwrap();
    let bob_token = server.token("bob", "temporary!").await;
    assert_eq!(server.subsonic("bob", "temporary!").await.0, "ok");

    server
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/admin/users/{bob_id}"),
            Some(json!({ "status": "disabled" })),
        )
        .await;
    let (status, _) = server
        .call(Some(&bob_token), Method::GET, "/api/bootstrap", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = server.login("bob", "temporary!").await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::FORBIDDEN, Some("disabled"))
    );
    // A wrong password says nothing about the account.
    let (status, body) = server.login("bob", "wrong").await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::UNAUTHORIZED, None)
    );
    assert_eq!(
        server.subsonic("bob", "temporary!").await,
        ("failed".to_owned(), Some(50))
    );
    assert_eq!(
        server.subsonic("bob", "wrong").await,
        ("failed".to_owned(), Some(40))
    );

    // Back on, and a temporary password signs him out and back in.
    server
        .call(
            Some(&admin),
            Method::PATCH,
            &format!("/api/admin/users/{bob_id}"),
            Some(json!({ "status": "active" })),
        )
        .await;
    let bob_token = server.token("bob", "temporary!").await;
    let (status, _) = server
        .call(
            Some(&admin),
            Method::POST,
            &format!("/api/admin/users/{bob_id}/password"),
            Some(json!({ "password": "another temporary" })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = server
        .call(Some(&bob_token), Method::GET, "/api/bootstrap", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = server.login("bob", "temporary!").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    server.token("bob", "another temporary").await;
}

#[tokio::test]
async fn deleting_an_account_frees_only_its_own_files() {
    let server = Server::new().await;
    let admin = server.setup().await;
    let (_, bob) = server
        .call(
            Some(&admin),
            Method::POST,
            "/api/admin/users",
            Some(json!({ "username": "bob", "password": "temporary!" })),
        )
        .await;
    let bob_id = bob["id"].as_u64().unwrap();
    // One file both play, one only Bob plays.
    let shared = server.ingest(1, "01-first-light.flac").await;
    server.ingest(bob_id, "01-first-light.flac").await;
    let own = server.ingest(bob_id, "02-second-wind.mp3").await;
    let own_file = server.services.treasury.resolve(&own.path);
    let shared_file = server.services.treasury.resolve(&shared.path);

    let (_, list) = server
        .call(Some(&admin), Method::GET, "/api/admin/users", None)
        .await;
    let bob_row = &list[1];
    assert_eq!(bob_row["songs"], 2);
    assert_eq!(
        bob_row["exclusive_bytes"].as_u64().unwrap(),
        std::fs::metadata(&own_file).unwrap().len()
    );
    let (_, storage) = server
        .call(Some(&admin), Method::GET, "/api/admin/storage", None)
        .await;
    assert_eq!(
        (storage["files"].as_u64(), storage["shared_files"].as_u64()),
        (Some(2), Some(1))
    );

    let (status, _) = server
        .call(Some(&admin), Method::DELETE, "/api/admin/users/1", None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "not oneself");
    let (status, deleted) = server
        .call(
            Some(&admin),
            Method::DELETE,
            &format!("/api/admin/users/{bob_id}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{deleted}");
    assert_eq!(
        deleted["freed_bytes"].as_u64().unwrap(),
        bob_row["exclusive_bytes"].as_u64().unwrap()
    );
    assert!(!own_file.exists());
    assert!(shared_file.is_file(), "Alice still plays it");
    let mut db = server.db.clone();
    assert!(
        User::filter_by_id(bob_id)
            .first()
            .exec(&mut db)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(Track::all().exec(&mut db).await.unwrap().len(), 1);
    assert_eq!(AudioFile::all().exec(&mut db).await.unwrap().len(), 1);
    let (status, _) = server.login("bob", "temporary!").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
