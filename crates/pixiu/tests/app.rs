//! End-to-end tests against a real server on an ephemeral port: the APIs
//! mounted together, the web player served for its routes, WebSockets,
//! cross-origin requests and compression.

use std::{net::SocketAddr, path::Path};

use futures_util::StreamExt;
use md5::Digest;
use pixiu_core::{Config, SecretBox};
use pixiu_db::{
    Album, Artist, ClaimKind, Db, Playlist, PlaylistEntry, Track, TrackClaim, TrackOrigin, Watch,
    WatchKind, now, toasty,
};
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::oneshot};

/// The player's `index.html` in tests; long enough to be worth compressing.
const INDEX: &str = "<!doctype html><html><head><title>píxiū</title></head>\
                     <body><div id=\"app\"></div><script src=\"/assets/app-abc123.js\"></script></body></html>";

struct TestServer {
    addr: SocketAddr,
    client: reqwest::Client,
    /// The key the tests' requests sign in with, once claimed.
    token: String,
    _data: tempfile::TempDir,
    _shutdown: oneshot::Sender<()>,
}

impl TestServer {
    async fn start() -> Self {
        Self::start_with(async |_| {}).await
    }

    /// Starts a server on a database `prepare` has seeded, and claims it.
    /// The background workers stay off, so queued jobs stay queued.
    async fn start_with(prepare: impl AsyncFnOnce(&mut Db)) -> Self {
        let data = tempfile::tempdir().unwrap();
        let web = data.path().join("web");
        std::fs::create_dir_all(web.join("assets")).unwrap();
        std::fs::create_dir_all(web.join("img")).unwrap();
        std::fs::write(web.join("index.html"), INDEX).unwrap();
        std::fs::write(web.join("assets/app-abc123.js"), "console.log('player')").unwrap();
        std::fs::write(web.join("img/logo.svg"), "<svg/>").unwrap();

        let mut config = Config::default();
        config.paths.data_dir = data.path().join("data");
        config.paths.treasure_dir = data.path().join("treasure");
        let mut db = pixiu_db::open(&data.path().join("pixiu.db")).await.unwrap();
        prepare(&mut db).await;
        let services = pixiu::Services::new(db, &config, SecretBox::ephemeral())
            .await
            .unwrap();
        let app = pixiu::app(&services, &config, &web);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    stopped.await.ok();
                })
                .await
                .unwrap();
        });
        let mut server = Self {
            addr,
            client: reqwest::Client::new(),
            token: String::new(),
            _data: data,
            _shutdown: shutdown,
        };
        server.claim().await;
        server
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(self.url(path)).send().await.unwrap()
    }

    /// Calls the web player's API with the test key.
    async fn api(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> reqwest::Response {
        let mut request = self
            .client
            .request(method, self.url(path))
            .bearer_auth(&self.token);
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().await.unwrap()
    }

    async fn api_json(&self, path: &str) -> Value {
        let response = self.api(reqwest::Method::GET, path, None).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        response.json().await.unwrap()
    }

    async fn claim(&mut self) {
        let response = self
            .client
            .post(self.url("/api/auth/setup"))
            .json(&json!({ "username": "keeper", "password": "gold-and-jade" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value = response.json().await.unwrap();
        self.token = body["token"].as_str().unwrap().to_owned();
    }
}

async fn subsonic(server: &TestServer, method: &str, query: &str) -> Value {
    let response = server
        .get(&format!("/rest/{method}?f=json&v=1.16.1&c=test&{query}"))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    body["subsonic-response"].clone()
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
async fn subsonic_api_is_mounted_under_rest() {
    let server = TestServer::start().await;

    let json = subsonic(&server, "ping.view", "").await;
    assert_eq!(json["status"], "failed");
    assert_eq!(json["error"]["code"], 10);

    let json = subsonic(&server, "ping.view", "u=keeper&p=gold-and-jade").await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["openSubsonic"], true);

    // Claiming captured the password, so token authentication works at once.
    let token = hex::encode(md5::Md5::digest("gold-and-jadesalty1"));
    let json = subsonic(&server, "ping", &format!("u=keeper&t={token}&s=salty1")).await;
    assert_eq!(json["status"], "ok");

    // The player's key works for Subsonic too.
    let json = subsonic(&server, "ping", &format!("apiKey={}", server.token)).await;
    assert_eq!(json["status"], "ok");

    // formPost: parameters in a form-encoded body. Extensions are public.
    let response = server
        .client
        .post(server.url("/rest/getOpenSubsonicExtensions"))
        .form(&[("f", "json")])
        .send()
        .await
        .unwrap();
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["subsonic-response"]["openSubsonicExtensions"][0]["name"],
        "formPost"
    );
}

#[tokio::test]
async fn the_player_is_served_for_its_routes() {
    let server = TestServer::start().await;

    for path in ["/", "/home", "/albums/al-1", "/genres/Funk%20%2F%20Jazz"] {
        let response = server.get(path).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-cache",
            "{path}"
        );
        assert_eq!(response.text().await.unwrap(), INDEX, "{path}");
    }

    let asset = server.get("/assets/app-abc123.js").await;
    assert_eq!(asset.status(), StatusCode::OK);
    assert_eq!(
        asset.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    let image = server.get("/img/logo.svg").await;
    assert_eq!(image.text().await.unwrap(), "<svg/>");

    // A missing asset is a 404, which tells an open player it was updated;
    // so is an unknown API path.
    assert_eq!(
        server.get("/assets/app-old456.js").await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        server.get("/api/nope").await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(server.get("/api").await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_login_screen_socket_refuses_other_origins() {
    use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest};

    let server = TestServer::start().await;
    let request = |origin: &str| {
        let mut request = format!(
            "ws://{}/api/sources/login/ws?api_key={}",
            server.addr, server.token
        )
        .into_client_request()
        .unwrap();
        request
            .headers_mut()
            .insert(header::ORIGIN, origin.parse().unwrap());
        request
    };

    match tokio_tungstenite::connect_async(request("https://elsewhere.example")).await {
        Err(tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
        other => panic!("expected a refusal, got {other:?}"),
    }

    // The player itself gets through; with no login browser open, the
    // server just closes the socket.
    let (mut socket, _) = tokio_tungstenite::connect_async(request(&server.url("")))
        .await
        .unwrap();
    assert!(matches!(
        socket.next().await,
        None | Some(Ok(tungstenite::Message::Close(_)))
    ));
}

#[tokio::test]
async fn subsonic_apps_may_call_from_other_origins() {
    let server = TestServer::start().await;
    let response = server
        .client
        .post(server.url("/rest/ping.view"))
        .header(header::ORIGIN, "https://elsewhere.example")
        .form(&[("f", "json"), ("u", "keeper"), ("p", "gold-and-jade")])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["subsonic-response"]["status"], "ok");
}

#[tokio::test]
async fn audio_is_never_compressed_but_pages_are() {
    let server = TestServer::start().await;

    let form = reqwest::multipart::Form::new()
        .text("batch", "1700000000000-cafe")
        .part(
            "file",
            reqwest::multipart::Part::bytes(fixture("01-first-light.flac"))
                .file_name("01-first-light.flac"),
        );
    let upload = server
        .client
        .post(server.url("/api/offerings/upload"))
        .bearer_auth(&server.token)
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(upload.status(), StatusCode::OK);
    let accepted = server
        .api(
            reqwest::Method::POST,
            "/api/offerings/batches/1700000000000-cafe/accept",
            None,
        )
        .await;
    assert_eq!(accepted.status(), StatusCode::OK);

    let stream = server
        .client
        .get(server.url("/rest/stream?u=keeper&p=gold-and-jade&id=tr-1"))
        .header(header::ACCEPT_ENCODING, "gzip, br")
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), StatusCode::OK);
    assert!(stream.headers().get(header::CONTENT_ENCODING).is_none());
    assert_eq!(stream.headers()[header::ACCEPT_RANGES], "bytes");
    let length = fixture("01-first-light.flac").len().to_string();
    assert_eq!(stream.headers()[header::CONTENT_LENGTH], length.as_str());

    let page = server
        .client
        .get(server.url("/"))
        .header(header::ACCEPT_ENCODING, "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(page.headers()[header::CONTENT_ENCODING], "gzip");
}

#[tokio::test]
async fn songs_are_excluded_from_watched_playlists() {
    // (watch, playlist, track) ids.
    let ids = std::sync::Arc::new(std::sync::Mutex::new((0, 0, 0)));
    let seeded = std::sync::Arc::clone(&ids);
    let server = TestServer::start_with(async move |db| {
        let artist = toasty::create!(Artist {
            name: "Somebody",
            name_key: "somebody",
            created_at: now(),
        })
        .exec(db)
        .await
        .unwrap();
        let album = toasty::create!(Album {
            title: "Road Songs",
            title_key: "road songs",
            artist_id: artist.id,
            created_at: now(),
        })
        .exec(db)
        .await
        .unwrap();
        let track = toasty::create!(Track {
            album_id: album.id,
            artist_id: artist.id,
            title: "Unwanted Song",
            artist_credit: "Somebody",
            duration_ms: 1_000_u64,
            file_id: 0_u64,
            path: "Somebody/Road Songs/Unwanted Song.opus",
            size: 1_u64,
            suffix: "opus",
            content_type: "audio/ogg",
            ytm_video_id: Some("vidA".to_owned()),
            origin: TrackOrigin::Download,
            added_at: now(),
        })
        .exec(db)
        .await
        .unwrap();
        let watch = toasty::create!(Watch {
            kind: WatchKind::Playlist,
            remote_id: "PLroad",
            name: "Road trip",
            include_singles: false,
            only_new: false,
            seen: Vec::<String>::new(),
            interval_secs: 3600_u64,
            created_at: now(),
            next_sync_at: now(),
        })
        .exec(db)
        .await
        .unwrap();
        toasty::create!(TrackClaim {
            track_id: track.id,
            kind: ClaimKind::WatchPlaylist,
            reference: Some(watch.id.to_string()),
            created_at: now(),
        })
        .exec(db)
        .await
        .unwrap();
        let playlist = toasty::create!(Playlist {
            name: "Road trip",
            public: false,
            watch_id: Some(watch.id),
            created_at: now(),
            changed_at: now(),
        })
        .exec(db)
        .await
        .unwrap();
        for (position, (video, title)) in [("vidA", "Unwanted Song"), ("vidB", "Coming Song")]
            .into_iter()
            .enumerate()
        {
            toasty::create!(PlaylistEntry {
                playlist_id: playlist.id,
                position: u32::try_from(position).unwrap(),
                ytm_video_id: Some(video.to_owned()),
                title: Some(title.to_owned()),
                artist: Some("Somebody".to_owned()),
            })
            .exec(db)
            .await
            .unwrap();
        }
        *seeded.lock().unwrap() = (watch.id, playlist.id, track.id);
    })
    .await;
    let (watch, playlist, track) = *ids.lock().unwrap();

    let mirror = server
        .api_json(&format!("/api/playlists/pl-{playlist}/watch"))
        .await;
    assert_eq!(mirror["watch"]["name"], "Road trip");
    assert_eq!(mirror["coming"][0]["title"], "Coming Song");
    assert_eq!(mirror["excluded"], json!([]));

    // The song sheet offers to exclude it from the watch that keeps it.
    let info = server
        .api_json(&format!("/api/songs/tr-{track}/info"))
        .await;
    assert_eq!(info["kept"][0]["why"], "Watched playlist “Road trip”");
    assert_eq!(info["kept"][0]["excludable_from"], watch);

    let excluded = server
        .api(
            reqwest::Method::POST,
            &format!("/api/watches/{watch}/exclusions"),
            Some(json!({ "song": format!("tr-{track}") })),
        )
        .await;
    assert_eq!(excluded.status(), StatusCode::NO_CONTENT);

    // Gone from the mirror, now an orphan that says why.
    let mirror = server
        .api_json(&format!("/api/playlists/pl-{playlist}/watch"))
        .await;
    assert_eq!(mirror["excluded"][0]["title"], "Unwanted Song");
    let listed = subsonic(
        &server,
        "getPlaylist",
        &format!("apiKey={}&id=pl-{playlist}", server.token),
    )
    .await;
    assert!(listed["playlist"]["entry"].as_array().unwrap().is_empty());
    let orphans = server.api_json("/api/orphans").await;
    assert_eq!(
        orphans["orphans"][0]["reason"],
        "Excluded from the watched playlist “Road trip”"
    );

    // Including it again queues a sync that fetches it.
    let included = server
        .api(
            reqwest::Method::DELETE,
            &format!("/api/watches/{watch}/exclusions/vidA"),
            None,
        )
        .await;
    assert_eq!(included.status(), StatusCode::NO_CONTENT);
    let mirror = server
        .api_json(&format!("/api/playlists/pl-{playlist}/watch"))
        .await;
    assert_eq!(mirror["excluded"], json!([]));
    let jobs = server.api_json("/api/jobs").await;
    assert!(
        jobs.as_array()
            .unwrap()
            .iter()
            .any(|job| job["kind"] == "watch_sync"),
        "{jobs}"
    );
}
