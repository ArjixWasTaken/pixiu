//! Assembles the píxiū application from its parts.

pub mod web;

use std::{path::Path, sync::Arc};

use anyhow::Context;
use axum::Router;
use pixiu_browser::{BrowserOptions, LoginDesk};
use pixiu_core::{Config, SecretBox, TranscodeFormat, playing::NowPlaying};
use pixiu_db::Db;
use pixiu_hunt::{Hunter, YtMusic};
use pixiu_jobs::{
    Jobs, Warden,
    adapters::{BrowserRefresher, HuntExecutor, YtMusicPlatform},
    watch,
};
use pixiu_subsonic::{Codec, SubsonicState};
use pixiu_treasury::{Offerings, Treasury};
use tokio::sync::Semaphore;
use tower_http::compression::{
    CompressionLayer,
    predicate::{DefaultPredicate, NotForContentType, Predicate},
};

/// píxiū's long-lived parts, shared by the web player's API, the Subsonic
/// API and the background workers.
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
    /// Fed by the Subsonic API, shown by the web player.
    pub now_playing: NowPlaying,
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
            now_playing: NowPlaying::default(),
        })
    }

    /// What the web player's API needs.
    #[must_use]
    pub fn api_state(&self) -> pixiu_api::ApiState {
        pixiu_api::ApiState {
            db: self.db.clone(),
            secrets: self.secrets.clone(),
            treasury: self.treasury.clone(),
            offerings: self.offerings.clone(),
            hunter: Arc::clone(&self.hunter),
            warden: Arc::clone(&self.warden),
            jobs: Arc::clone(&self.jobs),
            login_desk: Arc::clone(&self.login_desk),
        }
    }

    /// Starts the background workers: the session warden, the job queue
    /// (which resumes unfinished jobs), the watch scheduler, and resuming
    /// paused jobs whenever the login works again.
    pub fn start(&self) {
        self.warden.start();
        self.jobs.start();
        watch::schedule(self.db.clone(), Arc::clone(&self.jobs));
        watch::resume_on_login(&self.warden, Arc::clone(&self.jobs));
        let (mut db, jobs) = (self.db.clone(), Arc::clone(&self.jobs));
        tokio::spawn(async move {
            if let Err(error) = pixiu_jobs::enrich::repair_album_artists(&mut db, &jobs).await {
                tracing::error!(%error, "cannot repair album artists");
            }
        });
    }
}

/// Builds the HTTP application: the Subsonic API under `/rest`, the web
/// player's API under `/api`, and the web player (from `web_dir`) for
/// everything else.
pub fn app(services: &Services, config: &Config, web_dir: &Path) -> Router {
    let subsonic = pixiu_subsonic::router(SubsonicState {
        transcode_format: match config.stream.format {
            TranscodeFormat::Mp3 => Codec::Mp3,
            TranscodeFormat::Opus => Codec::Opus,
            TranscodeFormat::Aac => Codec::Aac,
        },
        transcodes: Arc::new(Semaphore::new(config.stream.max_transcodes.max(1))),
        now_playing: services.now_playing.clone(),
        ..SubsonicState::new(
            services.db.clone(),
            services.treasury.clone(),
            services.secrets.clone(),
        )
    });

    Router::new()
        .merge(subsonic)
        .merge(pixiu_api::router(services.api_state()))
        .merge(web::player(web_dir))
        // Compressing audio gains nothing (it is incompressible) and would
        // drop `Accept-Ranges`, hiding seeking from clients. (The default
        // predicate already leaves images and event streams alone.)
        .layer(
            CompressionLayer::new()
                .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("audio/"))),
        )
}
