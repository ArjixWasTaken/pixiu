//! End-to-end tests against a real server on an ephemeral port.

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use futures_util::{SinkExt, StreamExt};
use pixiu_core::Config;
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
        let data = tempfile::tempdir().unwrap();
        let db = pixiu_db::open(&data.path().join("pixiu.db")).await.unwrap();
        let assets = AssetBundle::load_dir(&*ASSETS).unwrap();
        let app = pixiu::app(db, &Config::default(), assets);

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

#[tokio::test]
async fn subsonic_api_is_mounted_under_rest() {
    let server = TestServer::start().await;

    let response = server.get("/rest/ping.view?f=json").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["subsonic-response"]["status"], "ok");
    assert_eq!(body["subsonic-response"]["openSubsonic"], true);

    let body = server.get("/rest/ping").await.text().await.unwrap();
    assert!(body.contains(r#"status="ok""#), "{body}");

    // formPost: parameters in a form-encoded body.
    let response = server
        .post_form("/rest/getOpenSubsonicExtensions", &[("f", "json")])
        .await;
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        body["subsonic-response"]["openSubsonicExtensions"][0]["name"],
        "formPost"
    );
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
