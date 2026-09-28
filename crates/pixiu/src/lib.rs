//! Assembles the píxiū application from its parts.

use std::sync::Arc;

use anyhow::Context;
use pixiu_browser::{BrowserOptions, LoginDesk};
use pixiu_core::{Config, SecretBox, TranscodeFormat};
use pixiu_db::Db;
use pixiu_hunt::{Hunter, YtMusic};
use pixiu_jobs::{
    Jobs, Warden,
    adapters::{BrowserRefresher, HuntExecutor, YtMusicPlatform},
    watch,
};
use pixiu_subsonic::{Codec, SubsonicState};
use pixiu_treasury::{Offerings, Treasury};
use pixiu_web::WebDeps;
use tokio::sync::Semaphore;
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

/// píxiū's long-lived parts, shared by the WebUI, the Subsonic API and the
/// background workers.
#[derive(Clone)]
pub struct Services {
    pub db: Db,
    pub secrets: SecretBox,
    pub treasury: Treasury,
    pub offerings: Offerings,
    pub hunter: Arc<Hunter>,
    pub warden: Arc<Warden>,
    pub jobs: Arc<Jobs>,
    pub login_desk: Arc<LoginDesk>,
}

impl Services {
    /// Builds the services. Nothing runs in the background until
    /// [`start`](Self::start).
    ///
    /// # Errors
    ///
    /// Fails when a data directory cannot be created or the stored
    /// YouTube Music session cannot be read.
    pub async fn new(db: Db, config: &Config, secrets: SecretBox) -> anyhow::Result<Self> {
        let paths = &config.paths;
        let treasury = Treasury::new(db.clone(), &paths.treasure_dir, paths.cache_dir());
        treasury
            .load_layout()
            .await
            .context("failed to read the file layout")?;
        let offerings = Offerings::new(paths.offerings_dir(), treasury.clone());
        let ytmusic = YtMusic::new(
            &paths.youtube_music_dir(),
            secrets.clone(),
            config.hunt.botguard.clone(),
        )
        .context("failed to set up the YouTube Music client")?;
        let hunter = Arc::new(
            Hunter::new(ytmusic, treasury.clone(), paths.staging_dir())
                .context("failed to set up the hunter")?,
        );
        let login_desk = LoginDesk::new(BrowserOptions {
            executable: config.browser.executable.clone(),
            profile_dir: paths.browser_profile_dir(),
            no_sandbox: config.browser.no_sandbox,
        });
        let warden = Warden::new(
            db.clone(),
            secrets.clone(),
            Box::new(YtMusicPlatform(Arc::clone(&hunter))),
            Box::new(BrowserRefresher(Arc::clone(&login_desk))),
        )
        .await
        .context("failed to load the YouTube Music session")?;
        let sources = pixiu_enrich::Online::new(config.enrich.contact.as_deref())
            .context("failed to set up MusicBrainz and friends")?;
        let jobs = Jobs::new(
            db.clone(),
            Box::new(HuntExecutor {
                hunter: Arc::clone(&hunter),
                warden: Arc::clone(&warden),
                sources: Arc::new(sources),
            }),
        );
        Ok(Self {
            db,
            secrets,
            treasury,
            offerings,
            hunter,
            warden,
            jobs,
            login_desk,
        })
    }

    /// Starts the background workers: the session warden, the job queue
    /// (which resumes unfinished jobs), the watch scheduler, and resuming
    /// paused jobs whenever the login works again.
    pub fn start(&self) {
        self.warden.start();
        self.jobs.start();
        watch::schedule(self.db.clone(), Arc::clone(&self.jobs));
        watch::resume_on_login(&self.warden, Arc::clone(&self.jobs));
    }
}

/// Builds the HTTP application: the WebUI with the Subsonic API mounted at
/// `/rest`.
pub fn app(services: &Services, config: &Config, assets: AssetBundle) -> Router {
    let subsonic = pixiu_subsonic::router(SubsonicState {
        transcode_format: match config.stream.format {
            TranscodeFormat::Mp3 => Codec::Mp3,
            TranscodeFormat::Opus => Codec::Opus,
            TranscodeFormat::Aac => Codec::Aac,
        },
        transcodes: Arc::new(Semaphore::new(config.stream.max_transcodes.max(1))),
        ..SubsonicState::new(
            services.db.clone(),
            services.treasury.clone(),
            services.secrets.clone(),
        )
    });

    pixiu_web::router_builder(WebDeps {
        db: services.db.clone(),
        assets,
        cookie_security: config.server.cookie_security,
        secrets: services.secrets.clone(),
        treasury: services.treasury.clone(),
        offerings: services.offerings.clone(),
        hunter: Arc::clone(&services.hunter),
        warden: Arc::clone(&services.warden),
        jobs: Arc::clone(&services.jobs),
        login_desk: Arc::clone(&services.login_desk),
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
