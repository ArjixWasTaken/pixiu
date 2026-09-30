//! Assembles the píxiū application from its parts.

pub mod web;

use std::{path::Path, sync::Arc};

use anyhow::Context;
use axum::Router;
use pixiu_accounts::{
    MailTransport, Mailer, Settings, SmtpTransport, alerts::MailAlerts, oidc::Sso,
};
use pixiu_browser::LoginDesks;
use pixiu_core::{Config, SecretBox, TranscodeFormat, config::PathsConfig, playing::NowPlaying};
use pixiu_db::{Db, User};
use pixiu_hunt::{Hunter, YtMusicPool};
use pixiu_jobs::{
    Jobs, Wardens,
    adapters::{HuntExecutor, Sessions},
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
    /// Every user's YouTube Music session warden.
    pub wardens: Arc<Wardens>,
    pub jobs: Arc<Jobs>,
    /// Every user's login browser.
    pub desks: Arc<LoginDesks>,
    /// Fed by the Subsonic API, shown by the web player.
    pub now_playing: NowPlaying,
    /// What admins set in the player: the public address, registration,
    /// the mail server.
    pub settings: Arc<Settings>,
    pub mailer: Arc<Mailer>,
    /// The single sign-on provider.
    pub sso: Arc<Sso>,
    pub throttle: Arc<pixiu_api::Throttle>,
    /// See [`ServerConfig`](pixiu_core::config::ServerConfig).
    pub trust_proxy_headers: bool,
}

impl Services {
    /// Builds the services. Nothing runs in the background until
    /// [`start`](Self::start).
    ///
    /// # Errors
    ///
    /// Fails when a data directory cannot be created, the library's files
    /// cannot be moved into the store, or the stored YouTube Music session
    /// cannot be read.
    pub async fn new(db: Db, config: &Config, secrets: SecretBox) -> anyhow::Result<Self> {
        Self::with_transport(db, config, secrets, Arc::new(SmtpTransport)).await
    }

    /// Builds the services, sending email through `transport` (tests keep
    /// it instead).
    ///
    /// # Errors
    ///
    /// As [`new`](Self::new).
    pub async fn with_transport(
        db: Db,
        config: &Config,
        secrets: SecretBox,
        transport: Arc<dyn MailTransport>,
    ) -> anyhow::Result<Self> {
        let paths = &config.paths;
        let settings = Settings::load(db.clone(), secrets.clone())
            .await
            .context("failed to read the server settings")?;
        let mailer = Mailer::new(Arc::clone(&settings), transport);
        let alerts = Arc::new(MailAlerts {
            db: db.clone(),
            mailer: Arc::clone(&mailer),
        });
        let treasury = Treasury::new(db.clone(), &paths.treasure_dir, paths.cache_dir());
        // Before anything else touches the library: files filed before the
        // store existed move into it, and what a crash left behind goes.
        treasury
            .adopt_legacy()
            .await
            .context("failed to move the library's files into the store")?;
        treasury
            .collect_garbage()
            .await
            .context("failed to clean up the store")?;
        let offerings = Offerings::new(paths.offerings_dir(), treasury.clone());
        offerings
            .relocate_legacy()
            .await
            .context("failed to move uploads to their owners' directories")?;
        move_legacy_profile(&db, paths).await?;
        let pool = Arc::new(
            YtMusicPool::new(
                &paths.youtube_music_dir(),
                paths.users_dir(),
                secrets.clone(),
                config.hunt.botguard.clone(),
            )
            .context("failed to set up the YouTube Music client")?,
        );
        // Its cache may still hold the login of before every user had a
        // client; each user's warden restores their own.
        pool.logout_public().await;
        let hunter = Arc::new(
            Hunter::new(Arc::clone(&pool), treasury.clone(), paths.staging_dir())
                .context("failed to set up the hunter")?,
        );
        let desks = LoginDesks::new(
            config.browser.executable.clone(),
            config.browser.no_sandbox,
            paths.users_dir(),
            config.browser.max_open,
        );
        let wardens = Wardens::new(
            db.clone(),
            secrets.clone(),
            Box::new(Sessions {
                pool,
                desks: Arc::clone(&desks),
            }),
            Arc::clone(&alerts) as _,
        );
        let sources = pixiu_enrich::Online::new(config.enrich.contact.as_deref())
            .context("failed to set up MusicBrainz and friends")?;
        let jobs = Jobs::new(
            db.clone(),
            Box::new(HuntExecutor {
                hunter: Arc::clone(&hunter),
                wardens: Arc::clone(&wardens),
                sources: Arc::new(sources),
                alerts,
            }),
        );
        Ok(Self {
            db,
            secrets,
            treasury,
            offerings,
            hunter,
            wardens,
            jobs,
            desks,
            now_playing: NowPlaying::default(),
            sso: Sso::new(Arc::clone(&settings)),
            settings,
            mailer,
            throttle: Arc::default(),
            trust_proxy_headers: config.server.trust_proxy_headers,
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
            wardens: Arc::clone(&self.wardens),
            jobs: Arc::clone(&self.jobs),
            desks: Arc::clone(&self.desks),
            settings: Arc::clone(&self.settings),
            mailer: Arc::clone(&self.mailer),
            sso: Arc::clone(&self.sso),
            throttle: Arc::clone(&self.throttle),
            trust_proxy_headers: self.trust_proxy_headers,
        }
    }

    /// Starts the background workers: the outbox, every user's session warden (each
    /// resuming its owner's paused jobs whenever their login works again),
    /// the job queue (which resumes unfinished jobs), and the watch
    /// scheduler.
    pub fn start(&self) {
        self.mailer.start();
        self.jobs.start();
        watch::schedule(self.db.clone(), Arc::clone(&self.jobs));
        let (wardens, jobs) = (Arc::clone(&self.wardens), Arc::clone(&self.jobs));
        tokio::spawn(async move {
            if let Err(error) = wardens.start(jobs).await {
                tracing::error!(%error, "cannot start the session wardens");
            }
        });
        let (mut db, jobs) = (self.db.clone(), Arc::clone(&self.jobs));
        tokio::spawn(async move {
            if let Err(error) = pixiu_jobs::enrich::repair_album_artists(&mut db, &jobs).await {
                tracing::error!(%error, "cannot repair album artists");
            }
            if let Err(error) = pixiu_jobs::enrich::backfill_genres(&mut db, &jobs).await {
                tracing::error!(%error, "cannot queue the genres of matched albums");
            }
        });
    }
}

/// Moves the login browser profile of before every user had their own to
/// the first account's, whose session it holds.
async fn move_legacy_profile(db: &Db, paths: &PathsConfig) -> anyhow::Result<()> {
    let legacy = paths.legacy_browser_profile_dir();
    if !tokio::fs::try_exists(&legacy).await? {
        return Ok(());
    }
    let Some(first) = User::all()
        .exec(&mut db.clone())
        .await?
        .iter()
        .map(|user| user.id)
        .min()
    else {
        return Ok(());
    };
    let target = paths
        .users_dir()
        .join(first.to_string())
        .join("browser-profile");
    if tokio::fs::try_exists(&target).await? {
        return Ok(());
    }
    tokio::fs::create_dir_all(target.parent().expect("profiles have a parent")).await?;
    tokio::fs::rename(&legacy, &target)
        .await
        .context("failed to move the login browser profile")?;
    tracing::info!(
        user = first,
        "the login browser profile is now the first account's"
    );
    Ok(())
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
