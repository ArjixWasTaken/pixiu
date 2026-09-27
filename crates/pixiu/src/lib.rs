//! Assembles the píxiū application from its parts.

use pixiu_core::Config;
use pixiu_db::Db;
use pixiu_subsonic::SubsonicState;
use pixiu_web::WebDeps;
use topcoat::{
    asset::AssetBundle,
    router::{Router, tower::TowerRoute},
};

/// Builds the HTTP application: the WebUI with the Subsonic API mounted at
/// `/rest`.
pub fn app(db: Db, config: &Config, assets: AssetBundle) -> Router {
    let subsonic = pixiu_subsonic::router(SubsonicState { db: db.clone() });

    pixiu_web::router_builder(WebDeps {
        db,
        assets,
        cookie_security: config.server.cookie_security,
    })
    .route(TowerRoute::any("/rest/{*rest}", subsonic))
    .build()
}
