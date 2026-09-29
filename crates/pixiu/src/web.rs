//! Serving the web player: its hashed assets, cached for good, its other
//! files, and `index.html` for every other path, where the player's own
//! router takes over.

use std::path::{Path, PathBuf};

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use tower::{Layer, ServiceExt};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

/// Where the player's files are: the configured directory, else `web/`
/// next to the binary (as the Docker image installs it), else `web/dist`
/// in the working directory (a checkout after `pnpm build`).
#[must_use]
pub fn locate(configured: Option<&Path>) -> PathBuf {
    if let Some(dir) = configured {
        return dir.to_owned();
    }
    let beside_binary = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("web")))
        .filter(|dir| dir.join("index.html").is_file());
    beside_binary.unwrap_or_else(|| PathBuf::from("web/dist"))
}

/// Paths the APIs own: an unknown one is a 404, never the player.
fn is_api(path: &str) -> bool {
    ["/api", "/rest"]
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
}

/// The player, served from `dir`.
pub fn player(dir: &Path) -> Router {
    let index = dir.join("index.html");
    if !index.is_file() {
        tracing::warn!(
            dir = %dir.display(),
            "the web player is not built; run `pnpm build` in web/ or set paths.web_dir"
        );
    }

    // Hashed names change with their content; a missing one is a 404, which
    // tells an open player that a newer version was deployed.
    let assets = SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    )
    .layer(ServeDir::new(dir.join("assets")));

    // Everything else is checked for changes on every load.
    let files = SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    )
    .layer(ServeDir::new(dir).fallback(ServeFile::new(index)));

    Router::new()
        .nest_service("/assets", assets)
        .fallback(move |request: Request| {
            let files = files.clone();
            async move {
                if is_api(request.uri().path()) {
                    return (StatusCode::NOT_FOUND, "not found").into_response();
                }
                match files.oneshot(request).await {
                    Ok(response) => response.into_response(),
                    Err(infallible) => match infallible {},
                }
            }
        })
}
