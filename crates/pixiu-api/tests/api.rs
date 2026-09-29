//! The web player's API: claiming, signing in and out, and start-up.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use pixiu_api::ApiState;
use pixiu_core::SecretBox;
use std::path::Path;

use pixiu_db::{Annotation, ApiKey, Db, Track, User, now, toasty};
use pixiu_treasury::{Claim, Provenance, Treasury, tags};
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

    /// Adds the test albums: two tagged songs (genre Ambient) and one
    /// untagged.
    async fn stock(&self) {
        let dir = self._dir.path();
        let treasury = Treasury::new(self.db.clone(), dir.join("treasure"), dir.join("cache"));
        let staging = dir.join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/audio");
        for name in ["01-first-light.flac", "02-second-wind.mp3", "untagged.opus"] {
            let staged = staging.join(name);
            std::fs::copy(fixtures.join(name), &staged).unwrap();
            let info = tags::read(&staged).unwrap();
            treasury
                .ingest(
                    &staged,
                    &info,
                    None,
                    Provenance::offering("upload.flac", None),
                    Claim::offering(),
                )
                .await
                .unwrap();
        }
    }

    async fn track_id(&self, title: &str) -> u64 {
        Track::all()
            .exec(&mut self.db.clone())
            .await
            .unwrap()
            .into_iter()
            .find(|track| track.title == title)
            .unwrap()
            .id
    }

    async fn get(&self, token: &str, path: &str) -> Value {
        let (status, body) = self.request(Method::GET, path, Some(token), None).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        body
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

fn titles(page: &Value) -> Vec<&str> {
    page.as_array()
        .or_else(|| page["data"].as_array())
        .unwrap()
        .iter()
        .map(|item| item["title"].as_str().or(item["name"].as_str()).unwrap())
        .collect()
}

#[tokio::test]
async fn songs_page_with_a_cursor_in_any_order() {
    let api = Api::new().await;
    let token = api.claim().await;
    api.stock().await;

    let first = api.get(&token, "/api/songs?sort=title&limit=2").await;
    assert_eq!(titles(&first).len(), 2);
    let cursor = first["meta"]["next_cursor"].as_str().unwrap();
    let rest = api
        .get(
            &token,
            &format!("/api/songs?sort=title&limit=2&cursor={cursor}"),
        )
        .await;
    assert_eq!(titles(&rest).len(), 1);
    assert!(rest["meta"]["next_cursor"].is_null());

    let by_length = api.get(&token, "/api/songs?sort=length&order=desc").await;
    let lengths: Vec<i64> = by_length["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|song| song["duration"].as_i64().unwrap())
        .collect();
    assert!(
        lengths.windows(2).all(|pair| pair[0] >= pair[1]),
        "{lengths:?}"
    );

    let ambient = api.get(&token, "/api/songs?genre=Ambient&sort=track").await;
    assert_eq!(titles(&ambient), ["First Light", "Second Wind"]);

    let (status, _) = api
        .request(Method::GET, "/api/songs?sort=owner", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn favorites_plays_and_genres_shape_the_lists() {
    let api = Api::new().await;
    let token = api.claim().await;
    api.stock().await;
    let second_wind = api.track_id("Second Wind").await;
    toasty::create!(Annotation {
        item: format!("tr-{second_wind}"),
        play_count: 3,
        last_played: Some(now()),
        starred_at: Some(now()),
    })
    .exec(&mut api.db.clone())
    .await
    .unwrap();

    let starred = api.get(&token, "/api/songs?favorites_only=true").await;
    assert_eq!(titles(&starred), ["Second Wind"]);
    let played = api
        .get(&token, "/api/songs?sort=play_count&order=desc")
        .await;
    assert_eq!(titles(&played)[0], "Second Wind");
    let recent = api.get(&token, "/api/songs/recently-played").await;
    assert_eq!(titles(&recent), ["Second Wind"]);

    let albums = api.get(&token, "/api/albums?sort=name").await;
    assert!(titles(&albums).contains(&"Test Album"), "{albums}");
    let artists = api.get(&token, "/api/artists").await;
    assert!(!titles(&artists).is_empty());

    let genres = api.get(&token, "/api/genres").await;
    let ambient = genres
        .as_array()
        .unwrap()
        .iter()
        .find(|genre| genre["id"] == "Ambient")
        .unwrap();
    assert_eq!(ambient["song_count"], 2);
    assert!(ambient["length"].as_u64().unwrap() > 0);
}
