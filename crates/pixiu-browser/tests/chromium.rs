//! Drives a real Chromium: `cargo test -p pixiu-browser -- --ignored`.

use std::time::Duration;

use axum::{Router, http::header, response::IntoResponse, routing::get};
use pixiu_browser::{BrowserOptions, FieldKind, Input, LoginBrowser, harvest};
use tokio::net::TcpListener;

/// A page with a text box whose value is mirrored into a cookie, served
/// with a persistent session cookie. The box hugs the bottom of the
/// viewport, so clicking it proves frames report the page's real size.
async fn page() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/html"),
            (header::SET_COOKIE, "session=abc123; Max-Age=3600; Path=/"),
        ],
        r#"<!doctype html>
        <input id="box" style="position:fixed;left:0;bottom:0;width:400px;height:40px"
               oninput="document.cookie = 'typed=' + encodeURIComponent(this.value) + '; Max-Age=3600; Path=/'">"#,
    )
}

/// A sign-in form like Google's, among inputs that are not sign-in fields.
/// Typed values are mirrored into cookies.
async fn login() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html")],
        r#"<!doctype html>
        <form style="margin-top:300px">
          <label for="identifierId">Email or phone</label>
          <input id="identifierId" type="email" autocomplete="username webauthn" value="old@example.com"
                 oninput="document.cookie = 'user=' + encodeURIComponent(this.value) + '; Path=/'">
          <input name="Passwd" type="password" autocomplete="current-password" aria-label="Enter your password"
                 oninput="document.cookie = 'pass=' + encodeURIComponent(this.value) + '; Path=/'">
          <input name="identifier" type="email" hidden>
          <input name="q" type="search" placeholder="Search">
          <input type="checkbox" name="remember">
        </form>"#,
    )
}

async fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/", get(page))
        .route("/login", get(login));
    tokio::spawn(axum::serve(listener, app).into_future());
    format!("http://{addr}/")
}

fn options(profile: &std::path::Path) -> BrowserOptions {
    BrowserOptions {
        executable: std::env::var_os("PIXIU_BROWSER").map(Into::into),
        profile_dir: profile.to_owned(),
        no_sandbox: false,
    }
}

#[tokio::test]
#[ignore = "needs Chromium"]
async fn screen_input_and_cookies_round_trip() {
    let url = serve().await;
    let profile = tempfile::tempdir().unwrap();
    let browser = LoginBrowser::launch(&options(profile.path()), &url)
        .await
        .unwrap();

    // The screen streams as JPEG frames.
    let mut frames = browser.frames();
    let frame = tokio::time::timeout(Duration::from_secs(15), frames.recv())
        .await
        .expect("a frame within 15 s")
        .unwrap();
    assert!(frame.jpeg.starts_with(&[0xFF, 0xD8]), "JPEG data");
    // The window's toolbars take some of its height from the page.
    assert_eq!(frame.width, pixiu_browser::WIDTH);
    assert!(frame.height > 0 && frame.height <= pixiu_browser::HEIGHT);

    // Click the box, type, select all and retype, correct a typo, paste.
    let (x, y) = (20.0, f64::from(frame.height) - 20.0);
    let mut inputs = vec![
        Input::MouseDown { x, y, button: 0 },
        Input::MouseUp { x, y, button: 0 },
    ];
    let mut press = |key: &str, code: &str, modifiers: u8| {
        for down in [true, false] {
            let (key, code) = (key.to_owned(), code.to_owned());
            inputs.push(if down {
                Input::KeyDown {
                    key,
                    code,
                    modifiers,
                }
            } else {
                Input::KeyUp {
                    key,
                    code,
                    modifiers,
                }
            });
        }
    };
    press("q", "KeyQ", 0);
    press("a", "KeyA", 2);
    press("h", "KeyH", 0);
    press("i", "KeyI", 0);
    press("x", "KeyX", 0);
    press("Backspace", "Backspace", 0);
    inputs.push(Input::Text {
        text: " there".into(),
    });
    for input in inputs {
        browser.input(input).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let cookies = browser.cookies("127.0.0.1").await.unwrap();
    let value = |name: &str| {
        cookies
            .iter()
            .find(|cookie| cookie.name == name)
            .map(|cookie| cookie.value.clone())
    };
    assert_eq!(value("session").as_deref(), Some("abc123"));
    assert_eq!(value("typed").as_deref(), Some("hi%20there"));
    browser.close().await;

    // The profile keeps the cookies for a later headless visit.
    let harvested = harvest(&options(profile.path()), &url, "127.0.0.1")
        .await
        .unwrap();
    assert!(
        harvested
            .iter()
            .any(|cookie| cookie.name == "typed" && cookie.value == "hi%20there"),
        "{harvested:?}"
    );
}

#[tokio::test]
#[ignore = "needs Chromium"]
async fn sign_in_fields_are_found_and_filled() {
    let url = serve().await;
    let profile = tempfile::tempdir().unwrap();
    let browser = LoginBrowser::launch(&options(profile.path()), &format!("{url}login"))
        .await
        .unwrap();

    // The username and password fields, and nothing else.
    let mut fields = browser.fields();
    let found = tokio::time::timeout(
        Duration::from_secs(15),
        fields.wait_for(|fields| fields.len() == 2),
    )
    .await
    .expect("fields within 15 s")
    .unwrap()
    .clone();
    assert_eq!(found[0].key, "id:identifierId");
    assert_eq!(found[0].kind, FieldKind::Username);
    assert_eq!(found[0].label, "Email or phone");
    assert_eq!(found[0].value.as_deref(), Some("old@example.com"));
    assert!(found[0].y >= 300.0 && found[0].width > 0.0, "{found:?}");
    assert_eq!(found[1].key, "name:Passwd");
    assert_eq!(found[1].kind, FieldKind::Password);
    assert_eq!(found[1].label, "Enter your password");

    // Fills replace what the fields hold.
    for (key, value) in [
        ("id:identifierId", "keeper@example.com"),
        ("name:Passwd", "gold and jade"),
    ] {
        browser
            .input(Input::Fill {
                key: key.into(),
                value: value.into(),
            })
            .await
            .unwrap();
    }
    let found = tokio::time::timeout(
        Duration::from_secs(5),
        fields.wait_for(|fields| {
            fields.first().and_then(|field| field.value.as_deref()) == Some("keeper@example.com")
        }),
    )
    .await
    .expect("the new username within 5 s")
    .unwrap()
    .clone();
    // Passwords never come back.
    assert_eq!(found[1].value, None);

    let cookies = browser.cookies("127.0.0.1").await.unwrap();
    let value = |name: &str| {
        cookies
            .iter()
            .find(|cookie| cookie.name == name)
            .map(|cookie| cookie.value.clone())
    };
    assert_eq!(value("user").as_deref(), Some("keeper%40example.com"));
    assert_eq!(value("pass").as_deref(), Some("gold%20and%20jade"));

    // Unknown fields are ignored.
    browser
        .input(Input::Fill {
            key: "id:nope".into(),
            value: "x".into(),
        })
        .await
        .unwrap();
    browser.close().await;
}
