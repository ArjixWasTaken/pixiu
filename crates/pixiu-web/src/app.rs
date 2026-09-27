//! The route tree root (`/`). Routes derive from modules:
//!
//! - `_gate` (group): `/login`, `/setup`, pages for signed-out visitors
//! - `_hoard` (group): `/` and everything behind the login
//! - `logout`: `POST /logout`

mod _gate;
mod _hoard;
mod logout;

use std::time::Duration;

use pixiu_core::CookieSecurity;
use pixiu_db::Db;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    cookie::RouterBuilderCookieExt,
    router::{RouterBuilder, RouterBuilderDiscoverExt, Slot, layout, module_router},
    runtime::RouterBuilderRuntimeExt,
    session::{RouterBuilderSessionExt, SessionConfig},
    tailwind,
    view::{View, view},
};

use crate::{
    session_store::PixiuCookieStore,
    ui::{CINZEL, FAVICON, INTER},
};

/// How long a WebUI login lasts.
const SESSION_LIFETIME: Duration = Duration::from_hours(24 * 30);

/// Everything the WebUI needs from the host application.
pub struct WebDeps {
    pub db: Db,
    pub assets: AssetBundle,
    pub cookie_security: CookieSecurity,
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
                topcoat::font::link(font: CINZEL)
                topcoat::font::link(font: INTER)
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body class="min-h-screen">(slot)</body>
        </html>
    })
}
