//! The web player's API: claiming, signing in and out, and start-up.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use pixiu_api::ApiState;
use pixiu_core::SecretBox;
use pixiu_db::{ApiKey, Db, User};
use serde_json::{Value, json};
use tower::ServiceExt;

const PASSWORD: &str = "gold-and-jade";

struct Api {
    _dir: tempfile::TempDir,
    db: Db,
    router: Router,
}

impl Api {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let router = pixiu_api::router(ApiState {
            db: db.clone(),
            secrets: SecretBox::ephemeral(),
        });
        Self {
            _dir: dir,
            db,
            router,
        }
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
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
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    async fn claim(&self) -> String {
        let (status, body) = self
            .request(
                Method::POST,
                "/api/auth/setup",
                None,
                Some(json!({ "username": "keeper", "password": PASSWORD })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }

    async fn keys(&self) -> Vec<ApiKey> {
        ApiKey::all().exec(&mut self.db.clone()).await.unwrap()
    }
}

#[tokio::test]
async fn a_fresh_hoard_is_claimed_once() {
    let api = Api::new().await;
    let (_, status) = api
        .request(Method::GET, "/api/auth/status", None, None)
        .await;
    assert_eq!(status["claimed"], false);

    let (status, body) = api
        .request(
            Method::POST,
            "/api/auth/setup",
            None,
            Some(json!({ "username": "keeper", "password": "short" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["message"].as_str().unwrap().contains("8 characters"));

    let token = api.claim().await;
    assert!(token.starts_with("pixiu_"));
    let (_, status) = api
        .request(Method::GET, "/api/auth/status", None, None)
        .await;
    assert_eq!(status["claimed"], true);

    // Subsonic token authentication can use the password right away.
    let users = User::all().exec(&mut api.db.clone()).await.unwrap();
    assert!(users[0].subsonic_secret.is_some());

    let (status, _) = api
        .request(
            Method::POST,
            "/api/auth/setup",
            None,
            Some(json!({ "username": "intruder", "password": "another-password" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn signing_in_mints_a_key_and_signing_out_revokes_it() {
    let api = Api::new().await;
    let claimed = api.claim().await;

    let (status, body) = api
        .request(
            Method::POST,
            "/api/auth/login",
            None,
            Some(json!({ "username": "keeper", "password": "wrong-password" })),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "Wrong username or password.");

    let (status, body) = api
        .request(
            Method::POST,
            "/api/auth/login",
            None,
            Some(json!({ "username": " keeper ", "password": PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let token = body["token"].as_str().unwrap().to_owned();
    assert_eq!(body["audio-token"], token);
    assert_ne!(token, claimed);

    let keys = api.keys().await;
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|key| key.name == "Web session"));

    let (status, _) = api
        .request(Method::DELETE, "/api/auth/session", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(api.keys().await.len(), 1);

    let (status, _) = api
        .request(Method::GET, "/api/bootstrap", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bootstrap_needs_a_key_and_describes_the_hoard() {
    let api = Api::new().await;
    let (status, _) = api.request(Method::GET, "/api/bootstrap", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = api
        .request(Method::GET, "/api/bootstrap", Some("pixiu_nonsense"), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let token = api.claim().await;
    let (status, body) = api
        .request(Method::GET, "/api/bootstrap", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["current_user"]["name"], "keeper");
    assert_eq!(body["current_user"]["role"], "admin");
    assert_eq!(body["current_version"], pixiu_core::VERSION);
    assert_eq!(body["song_count"], 0);
    assert_eq!(body["song_length"], 0);
    assert_eq!(body["koel_plus"]["active"], false);
    assert_eq!(body["uses_podcasts"], false);
    assert_eq!(body["supports_transcoding"], true);

    // Using a key marks it used.
    let keys = api.keys().await;
    assert!(keys[0].last_used_at.is_some());
}
