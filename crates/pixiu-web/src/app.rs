//! The route tree root (`/`). Routes derive from modules:
//!
//! - `_gate` (group): `/login`, `/setup`, pages for signed-out visitors
//! - `_hoard` (group): `/`, `/offerings`, `/settings`, `/covers/..`: everything
//!   behind the login
//! - `logout`: `POST /logout`

mod _gate;
mod _hoard;
mod logout;

use std::time::Duration;

use std::sync::Arc;

use pixiu_browser::LoginDesk;
use pixiu_core::{CookieSecurity, SecretBox, playing::NowPlaying};
use pixiu_db::Db;
use pixiu_hunt::Hunter;
use pixiu_jobs::{Jobs, Warden};
use pixiu_treasury::{Offerings, Treasury};
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    cookie::RouterBuilderCookieExt,
    router::{BodyLimit, RouterBuilder, RouterBuilderDiscoverExt, Slot, layout, module_router},
    runtime::RouterBuilderRuntimeExt,
    session::{RouterBuilderSessionExt, SessionConfig},
    tailwind,
    view::{View, view},
};

use crate::{
    session_store::PixiuCookieStore,
    ui::{FAVICON, ROBOTO, ROBOTO_MONO},
};

/// How long a WebUI login lasts.
const SESSION_LIFETIME: Duration = Duration::from_hours(24 * 30);

/// Everything the WebUI needs from the host application.
pub struct WebDeps {
    pub db: Db,
    pub assets: AssetBundle,
    pub cookie_security: CookieSecurity,
    pub secrets: SecretBox,
    pub treasury: Treasury,
    pub offerings: Offerings,
    pub hunter: Arc<Hunter>,
    pub warden: Arc<Warden>,
    pub jobs: Arc<Jobs>,
    pub login_desk: Arc<LoginDesk>,
    pub now_playing: NowPlaying,
}

/// Builds the WebUI router. Callers may register more routes (the Subsonic
/// API) before calling `build()`.
pub fn router_builder(deps: WebDeps) -> RouterBuilder {
    let sessions = SessionConfig::builder()
        .token_store(PixiuCookieStore::new(deps.cookie_security))
        .lifetime(SESSION_LIFETIME)
        .build();

    module_router!()
        .discover()
        .assets(deps.assets)
        .app_context(deps.db)
        .app_context(deps.secrets)
        .app_context(deps.treasury)
        .app_context(deps.offerings)
        .app_context(deps.hunter)
        .app_context(deps.warden)
        .app_context(deps.jobs)
        .app_context(deps.login_desk)
        .app_context(deps.now_playing)
        // Uploads stream to disk, and whole albums are large. The handler
        // checks the session before reading any of the body. (Layers match
        // route groups, hence `(_hoard)`.)
        .layer(BodyLimit::disable().at("/(_hoard)/offerings/upload"))
        .cookies()
        .sessions(sessions)
        // Must come after the layers above so reruns pass through them.
        .runtime()
}

/// The HTML document around every page.
#[layout]
async fn shell(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="theme-color" content="#0f1622">
                <title>"píxiū"</title>
                <link rel="icon" type="image/png" href=(FAVICON)>
                topcoat::dev::script()
                topcoat::runtime::script()
                topcoat::font::link(font: ROBOTO)
                topcoat::font::link(font: ROBOTO_MONO)
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body>(slot)</body>
        </html>
    })
}
