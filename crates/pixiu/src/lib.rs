//! Assembles the píxiū application from its parts.

use pixiu_core::{Config, SecretBox};
use pixiu_db::Db;
use pixiu_subsonic::SubsonicState;
use pixiu_treasury::{Offerings, Treasury};
use pixiu_web::WebDeps;
use topcoat::{
    asset::AssetBundle,
    router::{
        Compression, OriginPolicy, Router,
        tower::{TowerLayer, TowerRoute},
    },
};
use tower_http::compression::{
    CompressionLayer,
    predicate::{DefaultPredicate, NotForContentType, Predicate},
};

/// Builds the HTTP application: the WebUI with the Subsonic API mounted at
/// `/rest`.
pub fn app(db: Db, config: &Config, secrets: SecretBox, assets: AssetBundle) -> Router {
    let treasury = Treasury::new(
        db.clone(),
        &config.paths.treasure_dir,
        config.paths.cache_dir(),
    );
    let offerings = Offerings::new(config.paths.offerings_dir(), treasury.clone());
    let subsonic = pixiu_subsonic::router(SubsonicState {
        db: db.clone(),
        treasury: treasury.clone(),
        secrets: secrets.clone(),
    });

    pixiu_web::router_builder(WebDeps {
        db,
        assets,
        cookie_security: config.server.cookie_security,
        secrets,
        treasury,
        offerings,
    })
    .route(TowerRoute::any("/rest/{*rest}", subsonic))
    // Web-based Subsonic clients post to the API from other origins. The
    // API authenticates every request by its parameters, so the WebUI's
    // cross-site request forgery defense does not apply to it.
    .origin_policy(OriginPolicy::new().exempt_paths(["/rest/{*rest}"]))
    // Topcoat's built-in compression would gzip whole audio files (audio is
    // incompressible) and drop `Accept-Ranges`, hiding seeking from clients.
    // Compress everything else as usual.
    .compression(Compression::off())
    .layer(TowerLayer::new(CompressionLayer::new().compress_when(
        DefaultPredicate::new().and(NotForContentType::const_new("audio/")),
    )))
    .build()
}
