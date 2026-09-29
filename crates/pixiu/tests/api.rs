//! The web player's API: claiming, signing in and out, start-up, the
//! library lists, and hunting.

use std::path::Path;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use pixiu_core::{Config, SecretBox};
use pixiu_db::{Annotation, ApiKey, Db, Track, User, now, toasty};
use pixiu_treasury::{Claim, Provenance, tags};
use serde_json::{Value, json};
use tower::ServiceExt;

const PASSWORD: &str = "gold-and-jade";

struct Api {
    _dir: tempfile::TempDir,
    db: Db,
    services: pixiu::Services,
    router: Router,
}

impl Api {
    /// The API on a fresh hoard. The background workers stay off, so
    /// queued jobs stay queued.
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.paths.data_dir = dir.path().join("data");
        config.paths.treasure_dir = dir.path().join("treasure");
        let db = pixiu_db::open(&dir.path().join("pixiu.db")).await.unwrap();
        let services = pixiu::Services::new(db.clone(), &config, SecretBox::ephemeral())
            .await
            .unwrap();
        let router = pixiu_api::router(services.api_state());
        Self {
            _dir: dir,
            db,
            services,
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
        let treasury = &self.services.treasury;
        let staging = self._dir.path().join("staging");
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

impl Api {
    /// Sends JSON with a key, asserting the status; returns the body.
    async fn send(
        &self,
        token: &str,
        method: Method,
        path: &str,
        body: Value,
        expect: StatusCode,
    ) -> Value {
        let (status, reply) = self.request(method, path, Some(token), Some(body)).await;
        assert_eq!(status, expect, "{path}: {reply}");
        reply
    }

    /// Uploads a file as an offering into `batch`.
    async fn upload(&self, token: &str, batch: &str, name: &str, data: &[u8]) -> Value {
        let boundary = "pixiu-test-boundary";
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"batch\"\r\n\r\n{batch}\r\n\
             --{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let request = Request::post("/api/offerings/upload")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(status, StatusCode::OK, "{json}");
        json
    }

    /// Makes a track an orphan by dropping what claims it.
    async fn orphan(&self, track_id: u64) {
        let mut db = self.db.clone();
        for claim in pixiu_db::TrackClaim::filter_by_track_id(track_id)
            .exec(&mut db)
            .await
            .unwrap()
        {
            claim.delete().exec(&mut db).await.unwrap();
        }
    }
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/audio")
            .join(name),
    )
    .unwrap()
}

#[tokio::test]
async fn watches_are_added_listed_and_removed() {
    let api = Api::new().await;
    let token = api.claim().await;
    let link = "https://music.youtube.com/playlist?list=PLtest1234567890";

    api.send(
        &token,
        Method::POST,
        "/api/watches",
        json!({ "target": link }),
        StatusCode::CREATED,
    )
    .await;
    let duplicate = api
        .send(
            &token,
            Method::POST,
            "/api/watches",
            json!({ "target": link }),
            StatusCode::CONFLICT,
        )
        .await;
    assert_eq!(duplicate["message"], "píxiū already watches that.");
    api.send(
        &token,
        Method::POST,
        "/api/watches",
        json!({ "target": "https://music.youtube.com/watch?v=abcdefghijk" }),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;

    let watches = api.get(&token, "/api/watches").await;
    let watch = &watches[0];
    assert_eq!(watch["kind"], "playlist");
    assert_eq!(watch["link"], link);
    // Adding a watch queues its first sync.
    assert_eq!(watch["status"]["state"], "queued");

    let id = watch["id"].as_u64().unwrap();
    let (status, _) = api
        .request(
            Method::DELETE,
            &format!("/api/watches/{id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(api.get(&token, "/api/watches").await, json!([]));
}

#[tokio::test]
async fn grabs_show_on_the_job_board() {
    let api = Api::new().await;
    let token = api.claim().await;

    api.send(
        &token,
        Method::POST,
        "/api/hunt/tracks",
        json!({ "id": "abcdefghijk", "title": "Kevin MacLeod — Local Forecast" }),
        StatusCode::ACCEPTED,
    )
    .await;
    api.send(
        &token,
        Method::POST,
        "/api/hunt/albums",
        json!({ "id": "../../etc", "title": "nope" }),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;

    let board = api.get(&token, "/api/jobs").await;
    assert_eq!(board[0]["kind"], "download");
    assert_eq!(board[0]["state"], "queued");
    assert_eq!(board[0]["title"], "Kevin MacLeod — Local Forecast");

    let (status, _) = api
        .request(Method::DELETE, "/api/jobs/finished", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn offerings_are_uploaded_reviewed_and_accepted() {
    let api = Api::new().await;
    let token = api.claim().await;
    let batch = "1700000000000-beef";

    let first = api
        .upload(
            &token,
            batch,
            "01-first-light.flac",
            &fixture("01-first-light.flac"),
        )
        .await;
    assert_eq!(first[0]["title"], "First Light");
    api.upload(
        &token,
        batch,
        "02-second-wind.mp3",
        &fixture("02-second-wind.mp3"),
    )
    .await;
    let junk = api.upload(&token, batch, "notes.txt", b"liner notes").await;
    assert_eq!(junk, json!([]));

    let pending = api.get(&token, "/api/offerings").await;
    assert_eq!(pending.as_array().unwrap().len(), 1, "one batch: {pending}");
    assert_eq!(pending[0]["batch"], batch);
    assert_eq!(pending[0]["files"].as_array().unwrap().len(), 2);

    let accepted = api
        .send(
            &token,
            Method::POST,
            &format!("/api/offerings/batches/{batch}/accept"),
            json!(null),
            StatusCode::OK,
        )
        .await;
    assert_eq!(accepted["albums"], 1);
    assert_eq!(accepted["failures"], json!([]));
    assert_eq!(api.get(&token, "/api/offerings").await, json!([]));
    assert!(titles(&api.get(&token, "/api/songs").await).contains(&"Second Wind"));

    // The new album is looked up on MusicBrainz.
    assert_eq!(api.get(&token, "/api/jobs").await[0]["kind"], "lookup");
}

#[tokio::test]
async fn orphans_are_kept_or_deleted() {
    let api = Api::new().await;
    let token = api.claim().await;
    api.stock().await;
    let first_light = api.track_id("First Light").await;
    let second_wind = api.track_id("Second Wind").await;
    api.orphan(first_light).await;
    api.orphan(second_wind).await;

    let orphans = api.get(&token, "/api/orphans").await;
    assert_eq!(orphans["orphans"].as_array().unwrap().len(), 2);
    assert_eq!(orphans["orphans"][0]["reason"], "Nothing claims it");
    assert!(orphans["total_size"].as_u64().unwrap() > 0);

    let kept = api
        .send(
            &token,
            Method::POST,
            "/api/orphans/keep",
            json!({ "songs": [format!("tr-{first_light}")] }),
            StatusCode::OK,
        )
        .await;
    assert_eq!(kept["kept"], 1);
    let deleted = api
        .send(
            &token,
            Method::POST,
            "/api/orphans/delete",
            json!({ "all": true }),
            StatusCode::OK,
        )
        .await;
    assert_eq!(deleted["deleted"], 1);
    assert_eq!(api.get(&token, "/api/orphans").await["orphans"], json!([]));
    assert_eq!(
        titles(&api.get(&token, "/api/songs?sort=title").await).len(),
        2
    );
}

#[tokio::test]
async fn songs_and_albums_tell_more_than_subsonic() {
    let api = Api::new().await;
    let token = api.claim().await;
    api.stock().await;
    let first_light = api.track_id("First Light").await;

    let info = api
        .get(&token, &format!("/api/songs/tr-{first_light}/info"))
        .await;
    assert!(
        info["format"].as_str().unwrap().starts_with("FLAC"),
        "{info}"
    );
    assert_eq!(info["origin"], "offering");
    assert_eq!(info["kept"][0]["why"], "You offered it");
    assert!(info["kept"][0]["excludable_from"].is_null());

    let album_id = api.get(&token, "/api/albums?sort=name").await["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|album| album["name"] == "Test Album")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let details = api
        .get(&token, &format!("/api/albums/{album_id}/details"))
        .await;
    assert_eq!(details["source"], "offering");
    assert_eq!(details["tracks"].as_array().unwrap().len(), 2);

    api.send(
        &token,
        Method::PUT,
        &format!("/api/albums/{album_id}"),
        json!({
            "title": "Renamed Album",
            "artist": details["artist"],
            "year": 2020,
            "tracks": [{ "id": format!("tr-{first_light}"), "title": "First Light (Remaster)", "track": 1 }],
        }),
        StatusCode::NO_CONTENT,
    )
    .await;
    let details = api
        .get(&token, &format!("/api/albums/{album_id}/details"))
        .await;
    assert_eq!(details["title"], "Renamed Album");
    assert_eq!(details["year"], 2020);
    assert_eq!(details["tracks"][0]["title"], "First Light (Remaster)");

    api.send(
        &token,
        Method::POST,
        &format!("/api/albums/{album_id}/lookup"),
        json!({ "release": "not a release" }),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    api.send(
        &token,
        Method::POST,
        &format!("/api/albums/{album_id}/lookup"),
        json!({}),
        StatusCode::ACCEPTED,
    )
    .await;
}

#[tokio::test]
async fn settings_manage_the_layout_and_keys() {
    let api = Api::new().await;
    let token = api.claim().await;

    let settings = api.get(&token, "/api/settings").await;
    assert_eq!(
        settings["layout"]["template"],
        settings["layout"]["default"]
    );
    assert_eq!(settings["keys"][0]["current"], true);

    let preview = api
        .get(&token, "/api/settings/layout/preview?template=%7Btitle%7D")
        .await;
    assert_eq!(preview["path"], "Vibing Over Venus.opus");
    let (status, _) = api
        .request(
            Method::GET,
            "/api/settings/layout/preview?template=%7Bnope%7D",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let created = api
        .send(
            &token,
            Method::POST,
            "/api/keys",
            json!({ "name": "Phone" }),
            StatusCode::OK,
        )
        .await;
    let key = created["key"].as_str().unwrap();
    assert!(key.starts_with("pixiu_"));
    // The new key signs Subsonic apps and the player in alike.
    api.get(key, "/api/bootstrap").await;
    let (status, _) = api
        .request(
            Method::DELETE,
            &format!("/api/keys/{}", created["id"]),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = api
        .request(Method::GET, "/api/bootstrap", Some(key), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn events_say_when_the_job_board_changes() {
    use futures_util::StreamExt;

    let api = Api::new().await;
    let token = api.claim().await;
    // Event streams authenticate by query, as browsers cannot add headers.
    let request = Request::get(format!("/api/events?api_key={token}"))
        .body(Body::empty())
        .unwrap();
    let response = api.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut events = response.into_body().into_data_stream();

    api.send(
        &token,
        Method::POST,
        "/api/hunt/tracks",
        json!({ "id": "abcdefghijk", "title": "A song" }),
        StatusCode::ACCEPTED,
    )
    .await;
    let mut seen = String::new();
    while !seen.contains("event: jobs") {
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), events.next())
            .await
            .expect("an event within 5 s")
            .unwrap()
            .unwrap();
        seen.push_str(&String::from_utf8_lossy(&chunk));
    }
}

fn rules_json(model: &str, operator: &str, value: &[&str]) -> Value {
    json!([{ "id": "g", "rules": [{ "id": "r", "model": model, "operator": operator, "value": value }] }])
}

#[tokio::test]
async fn smart_playlists_follow_their_rules() {
    let api = Api::new().await;
    let token = api.claim().await;
    api.stock().await;

    let created = api
        .send(
            &token,
            Method::POST,
            "/api/playlists",
            json!({ "name": "Lights", "rules": rules_json("title", "contains", &["light"]) }),
            StatusCode::OK,
        )
        .await;
    let id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["songCount"], 1, "{created}");
    assert_eq!(created["readonly"], true);
    assert_eq!(created["rules"][0]["rules"][0]["model"], "title");

    let updated = api
        .send(
            &token,
            Method::PUT,
            &format!("/api/playlists/{id}"),
            json!({ "name": "Ambient", "rules": rules_json("genre", "is", &["ambient"]) }),
            StatusCode::OK,
        )
        .await;
    assert_eq!(updated["name"], "Ambient");
    assert_eq!(updated["songCount"], 2);

    api.send(
        &token,
        Method::POST,
        "/api/playlists",
        json!({ "name": "Broken", "rules": rules_json("year", "contains", &["19"]) }),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
}

#[tokio::test]
async fn folders_hold_playlists_and_folders() {
    let api = Api::new().await;
    let token = api.claim().await;
    api.stock().await;

    let outer = api
        .send(
            &token,
            Method::POST,
            "/api/playlist-folders",
            json!({ "name": "Moods" }),
            StatusCode::OK,
        )
        .await;
    let outer_id = outer["id"].as_str().unwrap().to_owned();
    let inner = api
        .send(
            &token,
            Method::POST,
            "/api/playlist-folders",
            json!({ "name": "Calm", "parent_id": outer_id }),
            StatusCode::OK,
        )
        .await;
    let inner_id = inner["id"].as_str().unwrap().to_owned();
    assert_eq!(inner["parent_id"], outer_id);

    // A folder cannot go inside its own subfolder.
    api.send(
        &token,
        Method::PATCH,
        &format!("/api/playlist-folders/{outer_id}"),
        json!({ "parent_id": inner_id }),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;

    let playlist = api
        .send(
            &token,
            Method::POST,
            "/api/playlists",
            json!({ "name": "Soft", "folder_id": inner_id, "rules": rules_json("title", "contains", &["wind"]) }),
            StatusCode::OK,
        )
        .await;
    assert_eq!(playlist["folderId"], inner_id);
    let playlist_id = playlist["id"].as_str().unwrap().to_owned();

    api.send(
        &token,
        Method::DELETE,
        &format!("/api/playlist-folders/{inner_id}/playlists"),
        json!({ "playlists": [playlist_id] }),
        StatusCode::NO_CONTENT,
    )
    .await;
    api.send(
        &token,
        Method::POST,
        &format!("/api/playlist-folders/{outer_id}/playlists"),
        json!({ "playlists": [playlist_id] }),
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        api.get(&token, "/api/playlists").await[0]["folderId"],
        outer_id
    );

    // Deleting a folder moves what it held to the top.
    let (status, _) = api
        .request(
            Method::DELETE,
            &format!("/api/playlist-folders/{outer_id}"),
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(api.get(&token, "/api/playlists").await[0]["folderId"].is_null());
    let bootstrap = api.get(&token, "/api/bootstrap").await;
    assert_eq!(bootstrap["playlist_folders"][0]["name"], "Calm");
    assert!(bootstrap["playlist_folders"][0]["parent_id"].is_null());
}
