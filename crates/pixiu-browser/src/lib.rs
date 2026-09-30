//! A real Chromium that the admin drives from the web player to log in to
//! streaming platforms.
//!
//! The browser runs headless on the server. Its screen is streamed as JPEG
//! frames (CDP `Page.startScreencast`), and the admin's mouse and keyboard
//! input is replayed into it, so logins work exactly as in a desktop
//! browser, including two-factor prompts. Its sign-in fields are reported
//! too ([`Field`]), so the web player can mirror them for password managers.
//! Afterwards its cookies are harvested. The profile persists, so the same
//! session can later be refreshed without the admin.

mod fields;

use std::{
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use bytes::Bytes;
use chromiumoxide::{
    Browser, BrowserConfig, Page,
    cdp::browser_protocol::{
        input::{
            DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams,
            DispatchMouseEventType, InsertTextParams, MouseButton,
        },
        network::SetUserAgentOverrideParams,
        page::{
            EventScreencastFrame, ScreencastFrameAckParams, StartScreencastFormat,
            StartScreencastParams,
        },
        target::{CloseTargetParams, GetTargetsParams},
    },
};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore, broadcast, watch},
    task::JoinHandle,
};

/// The browser window's size. The page gets what its toolbars leave, which
/// each [`Frame`] reports.
pub const WIDTH: u32 = 1280;
pub const HEIGHT: u32 = 800;

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("cannot start Chromium: {0}")]
    Launch(String),
    #[error("browser: {0}")]
    Cdp(#[from] chromiumoxide::error::CdpError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("page script: {0}")]
    Script(String),
    /// Every browser píxiū allows at once is in use.
    #[error("another sign-in is running on the server; try again in a few minutes")]
    Busy,
}

pub use fields::{Field, FieldKind};

/// How to run Chromium.
#[derive(Debug, Clone)]
pub struct BrowserOptions {
    /// The Chromium binary; found in `PATH` when `None`.
    pub executable: Option<PathBuf>,
    /// The persistent profile, which keeps the login between sessions.
    pub profile_dir: PathBuf,
    /// Needed when running as root, e.g. in some containers.
    pub no_sandbox: bool,
}

/// A frame of the remote screen.
#[derive(Debug, Clone)]
pub struct Frame {
    pub jpeg: Bytes,
    /// The page's size in CSS pixels, which is also the coordinate space of
    /// [`Input`]. The image may be smaller on high-density screens.
    pub width: u32,
    pub height: u32,
}

/// Input from the admin, as sent by the web player. `Debug` leaves out text,
/// which may be a password.
#[derive(Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    MouseMove {
        x: f64,
        y: f64,
    },
    MouseDown {
        x: f64,
        y: f64,
        button: u8,
    },
    MouseUp {
        x: f64,
        y: f64,
        button: u8,
    },
    Wheel {
        x: f64,
        y: f64,
        dx: f64,
        dy: f64,
    },
    KeyDown {
        key: String,
        code: String,
        /// CDP's bit set: Alt = 1, Ctrl = 2, Meta = 4, Shift = 8.
        #[serde(default)]
        modifiers: u8,
    },
    KeyUp {
        key: String,
        code: String,
        #[serde(default)]
        modifiers: u8,
    },
    /// Pasted text.
    Text {
        text: String,
    },
    /// Replaces what a [`Field`] holds, as typing would: how a password
    /// manager's fill in the web player reaches the page.
    Fill {
        key: String,
        value: String,
    },
    /// Focuses a [`Field`].
    Focus {
        key: String,
    },
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MouseMove { x, y } => write!(f, "MouseMove({x}, {y})"),
            Self::MouseDown { x, y, button } => write!(f, "MouseDown({x}, {y}, {button})"),
            Self::MouseUp { x, y, button } => write!(f, "MouseUp({x}, {y}, {button})"),
            Self::Wheel { x, y, dx, dy } => write!(f, "Wheel({x}, {y}, {dx}, {dy})"),
            Self::KeyDown {
                code, modifiers, ..
            } => write!(f, "KeyDown({code}, {modifiers})"),
            Self::KeyUp {
                code, modifiers, ..
            } => write!(f, "KeyUp({code}, {modifiers})"),
            Self::Text { .. } => f.write_str("Text(..)"),
            Self::Fill { key, .. } => write!(f, "Fill({key}, ..)"),
            Self::Focus { key } => write!(f, "Focus({key})"),
        }
    }
}

/// A cookie from the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
}

/// Joins cookies into a `Cookie` header.
#[must_use]
pub fn cookie_header(cookies: &[Cookie]) -> String {
    cookies
        .iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

fn config(options: &BrowserOptions) -> Result<BrowserConfig, BrowserError> {
    let mut builder = BrowserConfig::builder()
        .user_data_dir(&options.profile_dir)
        .new_headless_mode()
        .window_size(WIDTH, HEIGHT)
        .viewport(None)
        // The defaults include `--enable-automation`, which makes sign-in
        // pages refuse to work; start from a clean slate instead.
        .disable_default_args()
        // Also turns off `navigator.webdriver`.
        .hide()
        // Switch names without their `--`, which chromiumoxide adds.
        .args([
            "no-first-run",
            "no-default-browser-check",
            "password-store=basic",
            // Containers often have a tiny `/dev/shm`.
            "disable-dev-shm-usage",
            "force-device-scale-factor=1",
            "lang=en-US",
            // Headless screens default to 800x600, smaller than the window:
            // a giveaway. Present an ordinary desktop screen.
            "screen-info={1920x1080}",
        ])
        .launch_timeout(Duration::from_secs(30));
    if let Some(executable) = &options.executable {
        builder = builder.chrome_executable(executable);
    }
    if options.no_sandbox {
        builder = builder.no_sandbox();
    }
    builder.build().map_err(BrowserError::Launch)
}

/// A running browser and the task driving its connection.
struct Session {
    browser: Browser,
    handler: JoinHandle<()>,
    page: Page,
}

impl Session {
    async fn launch(options: &BrowserOptions, url: &str) -> Result<Self, BrowserError> {
        std::fs::create_dir_all(&options.profile_dir)?;
        let (browser, mut handler) = Browser::launch(config(options)?)
            .await
            .map_err(|error| BrowserError::Launch(error.to_string()))?;
        let handler = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if event.is_err() {
                    break;
                }
            }
        });

        let page = browser.new_page("about:blank").await?;
        close_other_tabs(&browser, &page).await;
        // Headless Chromium says so in its user agent, and sign-in pages
        // turn it away; present it as the regular browser it is.
        let version = browser.version().await?;
        let user_agent = version.user_agent.replace("HeadlessChrome", "Chrome");
        page.execute(SetUserAgentOverrideParams::new(user_agent))
            .await?;
        page.goto(url).await?;

        Ok(Self {
            browser,
            handler,
            page,
        })
    }

    async fn cookies(&self, domain_suffix: &str) -> Result<Vec<Cookie>, BrowserError> {
        let bare = domain_suffix.trim_start_matches('.');
        Ok(self
            .browser
            .get_cookies()
            .await?
            .into_iter()
            .filter(|cookie| {
                let domain = cookie.domain.trim_start_matches('.');
                domain == bare || domain.ends_with(&format!(".{bare}"))
            })
            .map(|cookie| Cookie {
                name: cookie.name,
                value: cookie.value,
                domain: cookie.domain,
            })
            .collect())
    }

    async fn close(mut self) {
        let _ = self.browser.close().await;
        let _ = self.browser.wait().await;
        self.handler.abort();
    }
}

/// A browser the admin drives remotely.
pub struct LoginBrowser {
    session: Session,
    frames: broadcast::Sender<Frame>,
    latest: Arc<Mutex<Option<Frame>>>,
    screencast: JoinHandle<()>,
    fields: watch::Sender<Vec<Field>>,
    tracker: JoinHandle<()>,
    /// Where fills and focus changes run.
    world: tokio::sync::Mutex<fields::World>,
}

impl LoginBrowser {
    /// Starts Chromium at `url` and streams its screen.
    ///
    /// # Errors
    ///
    /// Fails when Chromium cannot be started.
    pub async fn launch(options: &BrowserOptions, url: &str) -> Result<Self, BrowserError> {
        let session = Session::launch(options, url).await?;
        let (frames, _) = broadcast::channel(4);
        let latest = Arc::new(Mutex::new(None));

        let mut events = session
            .page
            .event_listener::<EventScreencastFrame>()
            .await?;
        session
            .page
            .execute(
                StartScreencastParams::builder()
                    .format(StartScreencastFormat::Jpeg)
                    .quality(70)
                    .max_width(i64::from(WIDTH))
                    .max_height(i64::from(HEIGHT))
                    .build(),
            )
            .await?;

        let page = session.page.clone();
        let sender = frames.clone();
        let store = Arc::clone(&latest);
        let screencast = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                // Chromium sends the next frame only after an ack.
                let _ = page
                    .execute(ScreencastFrameAckParams::new(event.session_id))
                    .await;
                let data: &str = event.data.as_ref();
                let Ok(jpeg) = STANDARD.decode(data) else {
                    continue;
                };
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let frame = Frame {
                    jpeg: Bytes::from(jpeg),
                    width: event.metadata.device_width.round() as u32,
                    height: event.metadata.device_height.round() as u32,
                };
                *store.lock().unwrap() = Some(frame.clone());
                let _ = sender.send(frame);
            }
        });

        let (fields, _) = watch::channel(Vec::new());
        let tracker = tokio::spawn(fields::track(session.page.clone(), fields.clone()));

        Ok(Self {
            session,
            frames,
            latest,
            screencast,
            fields,
            tracker,
            world: tokio::sync::Mutex::default(),
        })
    }

    /// New frames as they arrive.
    #[must_use]
    pub fn frames(&self) -> broadcast::Receiver<Frame> {
        self.frames.subscribe()
    }

    /// The page's sign-in fields, kept up to date.
    #[must_use]
    pub fn fields(&self) -> watch::Receiver<Vec<Field>> {
        self.fields.subscribe()
    }

    /// The most recent frame, to show a newly connected viewer at once.
    #[must_use]
    pub fn latest_frame(&self) -> Option<Frame> {
        self.latest.lock().unwrap().clone()
    }

    /// Replays the admin's input into the page.
    ///
    /// # Errors
    ///
    /// Fails when the browser connection is broken.
    pub async fn input(&self, input: Input) -> Result<(), BrowserError> {
        let page = &self.session.page;
        match input {
            Input::MouseMove { x, y } => {
                page.execute(mouse(DispatchMouseEventType::MouseMoved, x, y, None))
                    .await?;
            }
            Input::MouseDown { x, y, button } => {
                page.execute(mouse(
                    DispatchMouseEventType::MousePressed,
                    x,
                    y,
                    Some(button),
                ))
                .await?;
            }
            Input::MouseUp { x, y, button } => {
                page.execute(mouse(
                    DispatchMouseEventType::MouseReleased,
                    x,
                    y,
                    Some(button),
                ))
                .await?;
            }
            Input::Wheel { x, y, dx, dy } => {
                let mut event = mouse(DispatchMouseEventType::MouseWheel, x, y, None);
                event.delta_x = Some(dx);
                event.delta_y = Some(dy);
                page.execute(event).await?;
            }
            Input::KeyDown {
                key,
                code,
                modifiers,
            } => {
                page.execute(key_event(true, &key, &code, modifiers))
                    .await?;
            }
            Input::KeyUp {
                key,
                code,
                modifiers,
            } => {
                page.execute(key_event(false, &key, &code, modifiers))
                    .await?;
            }
            Input::Text { text } => {
                page.execute(InsertTextParams::new(text)).await?;
            }
            Input::Fill { key, value } => {
                if self.world.lock().await.focus(page, &key).await? {
                    // Replace what the field holds, as a person would.
                    page.execute(key_event(true, "a", "KeyA", CTRL)).await?;
                    page.execute(key_event(false, "a", "KeyA", CTRL)).await?;
                    if value.is_empty() {
                        page.execute(key_event(true, "Backspace", "Backspace", 0))
                            .await?;
                        page.execute(key_event(false, "Backspace", "Backspace", 0))
                            .await?;
                    } else {
                        page.execute(InsertTextParams::new(value)).await?;
                    }
                }
            }
            Input::Focus { key } => {
                self.world.lock().await.focus(page, &key).await?;
            }
        }
        Ok(())
    }

    /// The page's current address.
    pub async fn url(&self) -> Option<String> {
        self.session.page.url().await.ok().flatten()
    }

    /// Cookies whose domain is `domain_suffix` or below it.
    ///
    /// # Errors
    ///
    /// Fails when the browser connection is broken.
    pub async fn cookies(&self, domain_suffix: &str) -> Result<Vec<Cookie>, BrowserError> {
        self.session.cookies(domain_suffix).await
    }

    /// Closes the browser, keeping the profile.
    pub async fn close(self) {
        self.screencast.abort();
        self.tracker.abort();
        self.session.close().await;
    }
}

/// A web player viewer's view of the login browser.
pub struct Viewer {
    /// The most recent frame, to show at once.
    pub latest: Option<Frame>,
    pub frames: broadcast::Receiver<Frame>,
    pub fields: watch::Receiver<Vec<Field>>,
}

/// A login browser and the permit it runs under.
struct Open {
    browser: LoginBrowser,
    _permit: OwnedSemaphorePermit,
}

/// One user's login browser: at most one open, shared by that user's web
/// player tabs, and closed once nobody has used it for a while. It runs
/// under a permit from the server-wide cap on browsers.
pub struct LoginDesk {
    options: BrowserOptions,
    permits: Arc<Semaphore>,
    current: tokio::sync::Mutex<Option<Open>>,
    last_used: Mutex<std::time::Instant>,
}

/// Idle login browsers close after this long.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(600);

impl LoginDesk {
    /// A desk of its own, under its own cap of one browser.
    #[must_use]
    pub fn new(options: BrowserOptions) -> Arc<Self> {
        Self::sharing(options, Arc::new(Semaphore::new(1)))
    }

    /// A desk whose browsers count against `permits`, shared with other
    /// desks.
    #[must_use]
    pub fn sharing(options: BrowserOptions, permits: Arc<Semaphore>) -> Arc<Self> {
        let desk = Arc::new(Self {
            options,
            permits,
            current: tokio::sync::Mutex::new(None),
            last_used: Mutex::new(std::time::Instant::now()),
        });
        let weak = Arc::downgrade(&desk);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let Some(desk) = weak.upgrade() else { break };
                let idle = desk.last_used.lock().unwrap().elapsed() > IDLE_TIMEOUT;
                if idle && desk.is_open().await {
                    tracing::info!("closing an idle login browser");
                    desk.close().await;
                }
            }
        });
        desk
    }

    #[must_use]
    pub fn options(&self) -> &BrowserOptions {
        &self.options
    }

    fn touch(&self) {
        *self.last_used.lock().unwrap() = std::time::Instant::now();
    }

    /// Opens a browser at `url`, replacing this desk's open one.
    ///
    /// # Errors
    ///
    /// Fails when every browser the server allows is in use
    /// ([`BrowserError::Busy`]), or Chromium cannot be started.
    pub async fn open(&self, url: &str) -> Result<(), BrowserError> {
        let mut current = self.current.lock().await;
        if let Some(previous) = current.take() {
            previous.browser.close().await;
        }
        let permit = Arc::clone(&self.permits)
            .try_acquire_owned()
            .map_err(|_| BrowserError::Busy)?;
        *current = Some(Open {
            browser: LoginBrowser::launch(&self.options, url).await?,
            _permit: permit,
        });
        self.touch();
        Ok(())
    }

    pub async fn is_open(&self) -> bool {
        self.current.lock().await.is_some()
    }

    /// What a viewer of the open browser needs.
    pub async fn watch(&self) -> Option<Viewer> {
        let current = self.current.lock().await;
        current.as_ref().map(|Open { browser, .. }| Viewer {
            latest: browser.latest_frame(),
            frames: browser.frames(),
            fields: browser.fields(),
        })
    }

    /// Replays input into the open browser.
    ///
    /// # Errors
    ///
    /// Fails when the browser connection is broken.
    pub async fn input(&self, input: Input) -> Result<(), BrowserError> {
        self.touch();
        match self.current.lock().await.as_ref() {
            Some(open) => open.browser.input(input).await,
            None => Ok(()),
        }
    }

    /// The host of the page the open browser shows, e.g.
    /// `accounts.google.com`.
    pub async fn host(&self) -> Option<String> {
        let url = match self.current.lock().await.as_ref() {
            Some(open) => open.browser.url().await?,
            None => return None,
        };
        host_of(&url)
    }

    /// Cookies of the open browser for `domain_suffix`.
    ///
    /// # Errors
    ///
    /// Fails when the browser connection is broken.
    pub async fn cookies(&self, domain_suffix: &str) -> Result<Vec<Cookie>, BrowserError> {
        match self.current.lock().await.as_ref() {
            Some(open) => open.browser.cookies(domain_suffix).await,
            None => Ok(Vec::new()),
        }
    }

    /// Closes the open browser, keeping its profile.
    pub async fn close(&self) {
        if let Some(open) = self.current.lock().await.take() {
            open.browser.close().await;
        }
    }

    /// See [`harvest`]. Waits for this desk's open login browser to close
    /// first (two browsers cannot share a profile), and for a free permit.
    ///
    /// # Errors
    ///
    /// Fails when Chromium cannot be started.
    pub async fn harvest(
        &self,
        url: &str,
        domain_suffix: &str,
    ) -> Result<Vec<Cookie>, BrowserError> {
        let _exclusive = self.current.lock().await;
        let _permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(|_| BrowserError::Busy)?;
        harvest(&self.options, url, domain_suffix).await
    }
}

/// Every user's login desk, and the server-wide cap on browsers open at
/// once (sign-ins and cookie refreshes alike).
pub struct LoginDesks {
    executable: Option<PathBuf>,
    no_sandbox: bool,
    /// Profiles live in `<users>/<id>/browser-profile`.
    users_dir: PathBuf,
    permits: Arc<Semaphore>,
    desks: Mutex<std::collections::HashMap<u64, Arc<LoginDesk>>>,
}

impl LoginDesks {
    /// At most `max_open` browsers run at once (at least one).
    #[must_use]
    pub fn new(
        executable: Option<PathBuf>,
        no_sandbox: bool,
        users_dir: PathBuf,
        max_open: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            executable,
            no_sandbox,
            users_dir,
            permits: Arc::new(Semaphore::new(max_open.max(1))),
            desks: Mutex::default(),
        })
    }

    /// What the server keeps of `owner`'s: their browser profile, and their
    /// YouTube Music client's cache.
    #[must_use]
    pub fn user_dir(&self, owner: u64) -> PathBuf {
        self.users_dir.join(owner.to_string())
    }

    /// Where `owner`'s browser keeps its profile.
    #[must_use]
    pub fn profile_dir(&self, owner: u64) -> PathBuf {
        self.user_dir(owner).join("browser-profile")
    }

    /// `owner`'s desk, made on first use.
    #[must_use]
    pub fn desk(&self, owner: u64) -> Arc<LoginDesk> {
        let mut desks = self.desks.lock().unwrap();
        Arc::clone(desks.entry(owner).or_insert_with(|| {
            LoginDesk::sharing(
                BrowserOptions {
                    executable: self.executable.clone(),
                    profile_dir: self.profile_dir(owner),
                    no_sandbox: self.no_sandbox,
                },
                Arc::clone(&self.permits),
            )
        }))
    }

    /// Closes `owner`'s browser and forgets their desk.
    pub async fn remove(&self, owner: u64) {
        let desk = self.desks.lock().unwrap().remove(&owner);
        if let Some(desk) = desk {
            desk.close().await;
        }
    }
}

/// Visits `url` with the persistent profile, without showing it to anyone,
/// and returns the cookies for `domain_suffix`. Visiting lets the platform
/// rotate short-lived cookies, which keeps the session alive.
///
/// # Errors
///
/// Fails when Chromium cannot be started.
pub async fn harvest(
    options: &BrowserOptions,
    url: &str,
    domain_suffix: &str,
) -> Result<Vec<Cookie>, BrowserError> {
    let session = Session::launch(options, url).await?;
    // Give the page's scripts a moment to refresh their cookies.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let cookies = session.cookies(domain_suffix).await;
    session.close().await;
    cookies
}

/// Closes the tab Chromium opens at startup (its New Tab page), leaving
/// `keep`.
async fn close_other_tabs(browser: &Browser, keep: &Page) {
    let Ok(targets) = browser.execute(GetTargetsParams::default()).await else {
        return;
    };
    for target in &targets.result.target_infos {
        if target.r#type == "page" && target.target_id != *keep.target_id() {
            let _ = browser
                .execute(CloseTargetParams::new(target.target_id.clone()))
                .await;
        }
    }
}

fn mouse(
    kind: DispatchMouseEventType,
    x: f64,
    y: f64,
    button: Option<u8>,
) -> DispatchMouseEventParams {
    let mut event = DispatchMouseEventParams::new(kind, x, y);
    if let Some(button) = button {
        event.button = Some(match button {
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            _ => MouseButton::Left,
        });
        event.click_count = Some(1);
    }
    event
}

const CTRL: u8 = 2;
const META: u8 = 4;

/// A key press or release, as a desktop browser would report it: keys that
/// type something carry their text, the rest (and shortcuts) go raw.
fn key_event(down: bool, key: &str, code: &str, modifiers: u8) -> DispatchKeyEventParams {
    let shortcut = modifiers & (CTRL | META) != 0;
    let text = match key {
        // Enter types a carriage return, which submits forms.
        "Enter" => Some("\r"),
        _ if key.chars().count() == 1 && !shortcut => Some(key),
        _ => None,
    };
    let kind = match (down, text) {
        (false, _) => DispatchKeyEventType::KeyUp,
        (true, Some(_)) => DispatchKeyEventType::KeyDown,
        (true, None) => DispatchKeyEventType::RawKeyDown,
    };
    let mut event = DispatchKeyEventParams::new(kind);
    event.key = Some(key.to_owned());
    event.code = Some(code.to_owned());
    event.modifiers = Some(i64::from(modifiers));
    event.windows_virtual_key_code = virtual_key_code(key, code);
    if down {
        event.text = text.map(str::to_owned);
    }
    event
}

/// Windows virtual key codes, which Chromium needs to act on editing keys
/// and shortcuts.
fn virtual_key_code(key: &str, code: &str) -> Option<i64> {
    // Letters and digits by their position, whatever the layout.
    if let Some(letter) = code.strip_prefix("Key")
        && let [byte] = letter.as_bytes()
        && byte.is_ascii_uppercase()
    {
        return Some(i64::from(*byte));
    }
    if let Some(digit) = code.strip_prefix("Digit")
        && let [byte] = digit.as_bytes()
        && byte.is_ascii_digit()
    {
        return Some(i64::from(*byte));
    }
    Some(match key {
        "Backspace" => 8,
        "Tab" => 9,
        "Enter" => 13,
        "Shift" => 16,
        "Control" => 17,
        "Alt" => 18,
        "Escape" => 27,
        " " => 32,
        "PageUp" => 33,
        "PageDown" => 34,
        "End" => 35,
        "Home" => 36,
        "ArrowLeft" => 37,
        "ArrowUp" => 38,
        "ArrowRight" => 39,
        "ArrowDown" => 40,
        "Delete" => 46,
        "Meta" => 91,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_parses_from_the_webui_protocol() {
        let input: Input =
            serde_json::from_str(r#"{"type":"mouse_down","x":10.5,"y":20,"button":0}"#).unwrap();
        assert_eq!(
            input,
            Input::MouseDown {
                x: 10.5,
                y: 20.0,
                button: 0
            }
        );
        let input: Input = serde_json::from_str(r#"{"type":"text","text":"hunter2"}"#).unwrap();
        assert_eq!(
            input,
            Input::Text {
                text: "hunter2".to_owned()
            }
        );
        let input: Input =
            serde_json::from_str(r#"{"type":"fill","key":"name:Passwd","value":"hunter2"}"#)
                .unwrap();
        assert_eq!(
            input,
            Input::Fill {
                key: "name:Passwd".to_owned(),
                value: "hunter2".to_owned()
            }
        );
        // Typed text may be a password; it stays out of logs.
        assert_eq!(format!("{input:?}"), "Fill(name:Passwd, ..)");
    }

    #[test]
    fn keys_type_text_unless_they_edit_or_are_shortcuts() {
        let a = key_event(true, "a", "KeyA", 0);
        assert_eq!(a.r#type, DispatchKeyEventType::KeyDown);
        assert_eq!(a.text.as_deref(), Some("a"));
        assert_eq!(a.windows_virtual_key_code, Some(65));

        let select_all = key_event(true, "a", "KeyA", CTRL);
        assert_eq!(select_all.r#type, DispatchKeyEventType::RawKeyDown);
        assert_eq!(select_all.text, None);
        assert_eq!(select_all.modifiers, Some(2));

        let enter = key_event(true, "Enter", "Enter", 0);
        assert_eq!(enter.text.as_deref(), Some("\r"));
        assert_eq!(enter.windows_virtual_key_code, Some(13));

        let backspace = key_event(true, "Backspace", "Backspace", 0);
        assert_eq!(backspace.r#type, DispatchKeyEventType::RawKeyDown);
        assert_eq!(backspace.windows_virtual_key_code, Some(8));

        let up = key_event(false, "a", "KeyA", 0);
        assert_eq!(up.r#type, DispatchKeyEventType::KeyUp);
        assert_eq!(up.text, None);
    }

    #[test]
    fn cookie_headers_join_pairs() {
        let cookie = |name: &str, value: &str| Cookie {
            name: name.to_owned(),
            value: value.to_owned(),
            domain: ".youtube.com".to_owned(),
        };
        assert_eq!(
            cookie_header(&[cookie("SID", "a"), cookie("HSID", "b")]),
            "SID=a; HSID=b"
        );
    }
}

/// The host part of an absolute URL.
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = host.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

#[cfg(test)]
mod host_tests {
    #[tokio::test]
    async fn desks_share_the_browser_cap() {
        let dir = tempfile::tempdir().unwrap();
        let desks = super::LoginDesks::new(None, true, dir.path().to_owned(), 1);
        let (mine, theirs) = (desks.desk(1), desks.desk(2));
        assert!(std::sync::Arc::ptr_eq(&mine, &desks.desk(1)));
        assert_ne!(mine.options().profile_dir, theirs.options().profile_dir);
        assert!(mine.options().profile_dir.ends_with("1/browser-profile"));

        // With the one permit taken, nobody else starts a browser.
        let _held = std::sync::Arc::clone(&desks.permits)
            .try_acquire_owned()
            .unwrap();
        assert!(matches!(
            theirs.open("about:blank").await,
            Err(super::BrowserError::Busy)
        ));
        assert!(!theirs.is_open().await);
    }

    use super::host_of;

    #[test]
    fn hosts_are_read_from_urls() {
        assert_eq!(
            host_of("https://accounts.google.com/v3/signin?x=1").as_deref(),
            Some("accounts.google.com")
        );
        assert_eq!(
            host_of("http://user@Example.com:8080/").as_deref(),
            Some("example.com")
        );
        assert_eq!(host_of("about:blank"), None);
    }
}
