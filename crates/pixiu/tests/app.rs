//! End-to-end tests against a real server on an ephemeral port.

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use futures_util::{SinkExt, StreamExt};
use md5::Digest;
use pixiu_core::{Config, SecretBox};
use pixiu_db::{Db, SessionState, SourceSession, now, toasty};
use reqwest::{StatusCode, header};
use tokio::{net::TcpListener, sync::oneshot};
use topcoat::{
    Result,
    asset::AssetBundle,
    router::{
        content::websocket::{Message, WebSocketUpgrade},
        response::Response,
        route,
    },
};
use topcoat_asset::{Bundler, BundlerConfig};

/// The WebUI renders assets, so tests need a bundle matching this test
/// binary. Bundle it once, reusing the CLI's download cache for fonts.
static ASSETS: LazyLock<PathBuf> = LazyLock::new(|| {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let out = tmp.join("pixiu-test-assets");
    let cache = tmp.parent().unwrap().join("topcoat/cache/assets");
    let binary = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    Bundler::new(&BundlerConfig::new().cache_dir(cache))
        .bundle(&binary, &out)
        .expect("bundling the WebUI assets");
    out
});

/// A test-only route proving WebSocket upgrades work through the stack.
/// `discover()` registers it because it is linked into this binary.
#[route(GET "/__test/echo")]
async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
    upgrade.on_upgrade(|mut socket| async move {
        while let Some(Ok(message)) = socket.recv().await {
            if matches!(message, Message::Text(_) | Message::Binary(_))
                && socket.send(message).await.is_err()
            {
                break;
            }
        }
    })
}

struct TestServer {
    addr: SocketAddr,
    client: reqwest::Client,
    _data: tempfile::TempDir,
    _shutdown: oneshot::Sender<()>,
}

impl TestServer {
    async fn start() -> Self {
        Self::start_with(async |_| {}).await
    }

    /// Starts a server on a database `prepare` has seeded. The background
    /// workers stay off, so queued jobs stay queued.
    async fn start_with(prepare: impl AsyncFnOnce(&mut Db)) -> Self {
        let data = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.paths.data_dir = data.path().join("data");
        config.paths.treasure_dir = data.path().join("treasure");
        let mut db = pixiu_db::open(&data.path().join("pixiu.db")).await.unwrap();
        prepare(&mut db).await;
        let assets = AssetBundle::load_dir(&*ASSETS).unwrap();
        let services = pixiu::Services::new(db, &config, SecretBox::ephemeral())
            .await
            .unwrap();
        let app = pixiu::app(&services, &config, assets);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown, stopped) = oneshot::channel::<()>();
        tokio::spawn(topcoat::serve_until(listener, app, async {
            stopped.await.ok();
        }));

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        Self {
            addr,
            client,
            _data: data,
            _shutdown: shutdown,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(self.url(path)).send().await.unwrap()
    }

    async fn post_form(&self, path: &str, form: &[(&str, &str)]) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .form(form)
            .send()
            .await
            .unwrap()
    }

    async fn claim(&self) {
        let response = self
            .post_form(
                "/setup",
                &[
                    ("username", "keeper"),
                    ("password", "gold-and-jade"),
                    ("confirm", "gold-and-jade"),
                ],
            )
            .await;
        assert_eq!(location(&response), "/");
    }
}

fn location(response: &reqwest::Response) -> &str {
    assert!(
        response.status().is_redirection(),
        "expected a redirect, got {}",
        response.status()
    );
    response.headers()[header::LOCATION].to_str().unwrap()
}

fn set_cookies(response: &reqwest::Response) -> Vec<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().unwrap().to_owned())
        .collect()
}

async fn subsonic(server: &TestServer, method: &str, query: &str) -> serde_json::Value {
    let response = server
        .get(&format!("/rest/{method}?f=json&v=1.16.1&c=test&{query}"))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    body["subsonic-response"].clone()
}

#[tokio::test]
async fn subsonic_api_is_mounted_under_rest() {
    let server = TestServer::start().await;
    server.claim().await;

    let json = subsonic(&server, "ping.view", "").await;
    assert_eq!(json["status"], "failed");
    assert_eq!(json["error"]["code"], 10);

    let json = subsonic(&server, "ping.view", "u=keeper&p=gold-and-jade").await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["openSubsonic"], true);

    // Setup captured the password, so token authentication works at once.
    let token = hex::encode(md5::Md5::digest("gold-and-jadesalty1"));
    let json = subsonic(&server, "ping", &format!("u=keeper&t={token}&s=salty1")).await;
    assert_eq!(json["status"], "ok");

    // formPost: parameters in a form-encoded body. Extensions are public.
    let response = server
        .post_form("/rest/getOpenSubsonicExtensions", &[("f", "json")])
        .await;
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        body["subsonic-response"]["openSubsonicExtensions"][0]["name"],
        "formPost"
    );
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/audio")
            .join(name),
    )
    .unwrap()
}

/// The text after `prefix` in `html`, up to the next `"` or `<`.
fn between<'a>(html: &'a str, prefix: &str, suffix: &str) -> &'a str {
    let start = html
        .find(prefix)
        .unwrap_or_else(|| panic!("`{prefix}` not in page"))
        + prefix.len();
    let end = start + html[start..].find(suffix).unwrap();
    &html[start..end]
}

#[tokio::test]
async fn offerings_become_subsonic_music() {
    let server = TestServer::start().await;
    server.claim().await;

    // Upload an album through the WebUI.
    let form = reqwest::multipart::Form::new()
        .part(
            "files",
            reqwest::multipart::Part::bytes(fixture("01-first-light.flac"))
                .file_name("01-first-light.flac"),
        )
        .part(
            "files",
            reqwest::multipart::Part::bytes(fixture("02-second-wind.mp3"))
                .file_name("02-second-wind.mp3"),
        );
    let response = server
        .client
        .post(server.url("/offerings/upload"))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(location(&response), "/offerings");

    let review = server.get("/offerings").await.text().await.unwrap();
    assert!(review.contains("Test Album"), "{review}");
    assert!(review.contains("Accept 2"), "{review}");
    let accept_end = review.find("/accept\"").expect("an accept form");
    let batch = &review
        [review[..accept_end].rfind("/offerings/").unwrap() + "/offerings/".len()..accept_end];

    let accepted = server
        .post_form(&format!("/offerings/{batch}/accept"), &[])
        .await;
    assert_eq!(location(&accepted), "/offerings?accepted=2");
    let review = server.get("/offerings").await.text().await.unwrap();
    assert!(review.contains("Nothing awaits review."));

    // The dashboard shows the album, with its cover.
    let home = server.get("/").await.text().await.unwrap();
    assert!(home.contains("Recently hoarded"), "{home}");
    let cover = between(&home, "src=\"/covers/", "\"");
    let cover = server.get(&format!("/covers/{cover}")).await;
    assert_eq!(cover.status(), StatusCode::OK);
    assert_eq!(cover.headers()[header::CONTENT_TYPE], "image/png");

    // An API key from the settings page opens the Subsonic API.
    let settings = server
        .post_form("/settings/api-keys", &[("name", "tests")])
        .await
        .text()
        .await
        .unwrap();
    let key = format!("pixiu_{}", between(&settings, "pixiu_", "<"));
    assert_eq!(key.len(), "pixiu_".len() + 48);

    let auth = format!("apiKey={key}");
    let json = subsonic(&server, "getArtists", &auth).await;
    assert_eq!(
        json["artists"]["index"][0]["artist"][0]["name"],
        "Test Artist"
    );

    let json = subsonic(&server, "getAlbumList2", &format!("{auth}&type=newest")).await;
    let album_id = json["albumList2"]["album"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let json = subsonic(&server, "getAlbum", &format!("{auth}&id={album_id}")).await;
    let song_id = json["album"]["song"][0]["id"].as_str().unwrap().to_owned();

    // Seeking works through the whole stack.
    let partial = server
        .client
        .get(server.url(&format!("/rest/stream?{auth}&id={song_id}")))
        .header(header::RANGE, "bytes=0-9")
        .send()
        .await
        .unwrap();
    assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(partial.headers()[header::CONTENT_TYPE], "audio/flac");
    assert_eq!(
        &partial.bytes().await.unwrap()[..],
        &fixture("01-first-light.flac")[..10]
    );

    // Revoking the key closes the door.
    let settings = server.get("/settings").await.text().await.unwrap();
    let revoke = between(&settings, "action=\"/settings/api-keys/", "/revoke");
    server
        .post_form(&format!("/settings/api-keys/{revoke}/revoke"), &[])
        .await;
    let json = subsonic(&server, "ping", &auth).await;
    assert_eq!(json["error"]["code"], 44);
}

#[tokio::test]
async fn first_run_setup_then_login_and_logout() {
    let server = TestServer::start().await;

    // A fresh instance funnels every visitor to the setup page.
    assert_eq!(location(&server.get("/").await), "/login");
    assert_eq!(location(&server.get("/login").await), "/setup");
    let setup = server.get("/setup").await.text().await.unwrap();
    assert!(setup.contains("Claim this hoard"));

    let mismatch = server
        .post_form(
            "/setup",
            &[
                ("username", "keeper"),
                ("password", "gold-and-jade"),
                ("confirm", "gold-and-silver"),
            ],
        )
        .await;
    assert_eq!(location(&mismatch), "/setup?error=mismatch");

    server.claim().await;
    let home = server.get("/").await;
    assert_eq!(home.status(), StatusCode::OK);
    let home = home.text().await.unwrap();
    assert!(home.contains("The Hoard"));
    assert!(home.contains("keeper"));
    // Subsonic clients want the bare server address; they append `/rest`.
    assert!(home.contains(&format!(">{}</code>", server.url(""))));

    // Setup is closed once claimed.
    assert_eq!(location(&server.get("/setup").await), "/login");

    assert_eq!(location(&server.post_form("/logout", &[]).await), "/login");
    assert_eq!(location(&server.get("/").await), "/login");

    let wrong = server
        .post_form(
            "/login",
            &[("username", "keeper"), ("password", "nope-nope")],
        )
        .await;
    assert_eq!(location(&wrong), "/login?error=credentials");
    let unknown = server
        .post_form(
            "/login",
            &[("username", "thief"), ("password", "gold-and-jade")],
        )
        .await;
    assert_eq!(location(&unknown), "/login?error=credentials");

    let right = server
        .post_form(
            "/login",
            &[("username", "keeper"), ("password", "gold-and-jade")],
        )
        .await;
    assert_eq!(location(&right), "/");
    assert_eq!(server.get("/").await.status(), StatusCode::OK);
}

#[tokio::test]
async fn hunting_queues_jobs() {
    let server = TestServer::start().await;
    for path in ["/hunt", "/jobs"] {
        assert_eq!(location(&server.get(path).await), "/login", "{path}");
    }
    server.claim().await;

    assert!(
        server
            .get("/hunt")
            .await
            .text()
            .await
            .unwrap()
            .contains("Search YouTube Music")
    );
    assert!(
        server
            .get("/jobs")
            .await
            .text()
            .await
            .unwrap()
            .contains("No jobs yet")
    );

    let hostile = server
        .post_form(
            "/hunt/grab-track",
            &[("video_id", "../../etc/passwd"), ("title", "nope")],
        )
        .await;
    assert_eq!(hostile.status(), StatusCode::BAD_REQUEST);

    let grab = server
        .post_form(
            "/hunt/grab-track",
            &[
                ("video_id", "NPdgPZ0u3zQ"),
                ("title", "Kevin MacLeod — Monkeys Spinning Monkeys"),
                ("q", "monkeys spinning"),
            ],
        )
        .await;
    assert_eq!(location(&grab), "/hunt?q=monkeys+spinning&queued=1");
    let album = server
        .post_form(
            "/hunt/grab-album",
            &[("browse_id", "MPREb_jwN9EIjDfPS"), ("title", "Some Album")],
        )
        .await;
    assert_eq!(location(&album), "/hunt?queued=1");

    let jobs = server.get("/jobs").await.text().await.unwrap();
    assert!(
        jobs.contains("Kevin MacLeod — Monkeys Spinning Monkeys"),
        "{jobs}"
    );
    assert!(jobs.contains("Some Album"));
    assert_eq!(jobs.matches(">Queued").count(), 2, "{jobs}");

    // Clearing keeps unfinished jobs.
    assert_eq!(
        location(&server.post_form("/jobs/clear", &[]).await),
        "/jobs"
    );
    let jobs = server.get("/jobs").await.text().await.unwrap();
    assert!(jobs.contains("Some Album"));
}

#[tokio::test]
async fn sources_report_the_youtube_music_session() {
    let server = TestServer::start().await;
    assert_eq!(location(&server.get("/settings/sources").await), "/login");
    server.claim().await;

    let sources = server.get("/settings/sources").await.text().await.unwrap();
    assert!(sources.contains("Not connected"), "{sources}");
    assert!(sources.contains(r#"action="/settings/sources/connect""#));
    assert!(!sources.contains("session expired"));

    // The login screen only exists while a login browser is open.
    assert_eq!(
        location(&server.get("/settings/sources/login").await),
        "/settings/sources"
    );
    let status: serde_json::Value = server
        .get("/settings/sources/login/status")
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(
        status,
        serde_json::json!({ "open": false, "logged_in": false })
    );
}

#[tokio::test]
async fn an_expired_session_is_announced_on_every_page() {
    let server = TestServer::start_with(async |db| {
        toasty::create!(SourceSession {
            source: pixiu_jobs::warden::SOURCE,
            cookies: "sealed",
            state: SessionState::Expired,
            connected_at: now(),
            expired_at: Some(now()),
            last_error: Some("signed out everywhere".to_owned()),
        })
        .exec(db)
        .await
        .unwrap();
    })
    .await;
    server.claim().await;

    for path in ["/", "/hunt", "/jobs", "/offerings", "/settings"] {
        let page = server.get(path).await.text().await.unwrap();
        assert!(
            page.contains("YouTube Music session expired"),
            "no banner on {path}"
        );
    }
    let sources = server.get("/settings/sources").await.text().await.unwrap();
    assert!(sources.contains("Expired: log in again"));
    assert!(sources.contains("signed out everywhere"));
    assert!(sources.contains("Log in again"));
}

#[tokio::test]
async fn session_cookie_hardening_follows_transport() {
    let server = TestServer::start().await;
    server.claim().await;

    let plain = server
        .post_form(
            "/login",
            &[("username", "keeper"), ("password", "gold-and-jade")],
        )
        .await;
    let cookie = set_cookies(&plain).join("\n");
    assert!(cookie.starts_with("pixiu_session="), "{cookie}");
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    assert!(!cookie.contains("Secure"), "{cookie}");

    let proxied = server
        .client
        .post(server.url("/login"))
        .header("x-forwarded-proto", "https")
        .form(&[("username", "keeper"), ("password", "gold-and-jade")])
        .send()
        .await
        .unwrap();
    let cookie = set_cookies(&proxied).join("\n");
    assert!(cookie.starts_with("__Host-pixiu_session="), "{cookie}");
    assert!(cookie.contains("Secure"), "{cookie}");
}

#[tokio::test]
async fn bundled_assets_are_served() {
    let server = TestServer::start().await;
    let page = server.get("/setup").await.text().await.unwrap();

    let start = page
        .find("/_topcoat/assets/tailwind-")
        .expect("stylesheet link");
    let end = start + page[start..].find('"').unwrap();
    let response = server.get(&page[start..end]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/css")
    );
    let css = response.text().await.unwrap();
    assert!(
        css.contains("--gold"),
        "theme tokens compiled into the stylesheet"
    );
}

#[tokio::test]
async fn websocket_upgrades_reach_topcoat_routes() {
    let server = TestServer::start().await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{}/__test/echo", server.addr))
            .await
            .unwrap();

    socket
        .send(tokio_tungstenite::tungstenite::Message::text("hoard"))
        .await
        .unwrap();
    let echoed = socket.next().await.unwrap().unwrap();
    assert_eq!(echoed.to_text().unwrap(), "hoard");
}

/// The login screen drives a Google login, so other sites must not be able
/// to open it with the admin's cookies.
#[tokio::test]
async fn the_login_screen_socket_refuses_other_origins() {
    use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest};

    let server = TestServer::start().await;
    server.claim().await;
    let login = server
        .post_form(
            "/login",
            &[("username", "keeper"), ("password", "gold-and-jade")],
        )
        .await;
    let session = set_cookies(&login)[0].split(';').next().unwrap().to_owned();
    let request = |origin: &str| {
        let mut request = format!("ws://{}/settings/sources/login/ws", server.addr)
            .into_client_request()
            .unwrap();
        let headers = request.headers_mut();
        headers.insert(header::COOKIE, session.parse().unwrap());
        headers.insert(header::ORIGIN, origin.parse().unwrap());
        request
    };

    match tokio_tungstenite::connect_async(request("https://elsewhere.example")).await {
        Err(tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
        other => panic!("expected a refusal, got {other:?}"),
    }

    // The WebUI itself gets through; with no login browser open, the
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
async fn only_the_api_accepts_cross_origin_posts() {
    let server = TestServer::start().await;
    server.claim().await;

    let cross_site = |path: &str| {
        server
            .client
            .post(server.url(path))
            .header(header::ORIGIN, "https://elsewhere.example")
            .header("sec-fetch-site", "cross-site")
            .form(&[("f", "json"), ("u", "keeper"), ("p", "gold-and-jade")])
    };

    let api = cross_site("/rest/ping.view").send().await.unwrap();
    assert_eq!(api.status(), StatusCode::OK);
    assert_eq!(api.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    let body: serde_json::Value = api.json().await.unwrap();
    assert_eq!(body["subsonic-response"]["status"], "ok");

    let webui = cross_site("/logout").send().await.unwrap();
    assert_eq!(webui.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn audio_is_never_compressed_but_pages_are() {
    let server = TestServer::start().await;
    server.claim().await;

    let form = reqwest::multipart::Form::new().part(
        "files",
        reqwest::multipart::Part::bytes(fixture("01-first-light.flac"))
            .file_name("01-first-light.flac"),
    );
    server
        .client
        .post(server.url("/offerings/upload"))
        .multipart(form)
        .send()
        .await
        .unwrap();
    let review = server.get("/offerings").await.text().await.unwrap();
    let accept_end = review.find("/accept\"").unwrap();
    let batch = &review
        [review[..accept_end].rfind("/offerings/").unwrap() + "/offerings/".len()..accept_end];
    server
        .post_form(&format!("/offerings/{batch}/accept"), &[])
        .await;

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
        .get(server.url("/offerings"))
        .header(header::ACCEPT_ENCODING, "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(page.headers()[header::CONTENT_ENCODING], "gzip");
}
