//! The production sides of the warden and the queue: the platforms'
//! sessions, the browser profile, and the hunter.

use std::sync::Arc;

use pixiu_browser::{Cookie, LoginDesks, cookie_header};
use pixiu_core::alerts::AlertSink;
use pixiu_db::{
    Album, Job, JobKind, JobState, Platform, ReleaseReason, SessionState, SourceKey, Watch,
};
use pixiu_enrich::Sources;
use pixiu_hunt::{
    Discography, DownloadRequest, HuntError, Hunter, LoginSpec, Platforms, RemotePlaylist,
    SessionCheck, YtMusic, YtMusicPool,
};
use pixiu_treasury::Release;

use crate::{
    enrich::{self, PlatformLyrics},
    queue::{
        self, AlbumJob, EnrichJob, Executor, NewJob, Outcome, SyncJob, TrackJob, enriched_album,
    },
    warden::{BoxFuture, Refresher, Session},
    wardens::{SessionFactory, Wardens},
    watch::{self, Catalog, CatalogError, Synced},
};

/// A user's YouTube Music client, as their warden's session.
pub struct YtMusicSession {
    pub pool: Arc<YtMusicPool>,
    pub owner: u64,
}

impl YtMusicSession {
    fn client(&self) -> Result<Arc<YtMusic>, SessionCheck> {
        self.pool
            .for_user(self.owner)
            .map_err(|error| SessionCheck::Unreachable(error.to_string()))
    }
}

impl Session for YtMusicSession {
    fn apply<'a>(&'a self, cookies: &'a str) -> BoxFuture<'a, SessionCheck> {
        Box::pin(async move {
            match self.client() {
                Ok(client) => client.apply_cookies(cookies).await,
                Err(unreachable) => unreachable,
            }
        })
    }

    fn check(&self) -> BoxFuture<'_, SessionCheck> {
        Box::pin(async move {
            match self.client() {
                Ok(client) => client.check_session().await,
                Err(unreachable) => unreachable,
            }
        })
    }

    fn forget(&self) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            if let Ok(client) = self.client() {
                client.forget_cookies().await;
            }
        })
    }
}

/// Fresh cookies for a platform from a user's login browser profile.
pub struct BrowserRefresher {
    pub desks: Arc<LoginDesks>,
    pub owner: u64,
    /// How the platform's login works; none when it has no logins.
    pub login: Option<LoginSpec>,
}

impl Refresher for BrowserRefresher {
    fn refresh(&self) -> BoxFuture<'_, Result<String, String>> {
        Box::pin(async move {
            let Some(login) = self.login else {
                return Err("the platform has no logins".to_owned());
            };
            let cookies = self
                .desks
                .desk(self.owner)
                .harvest(login.home_url, login.cookie_domain)
                .await
                .map_err(|error| error.to_string())?;
            if !is_logged_in(&login, &cookies) {
                return Err("the browser profile is no longer logged in".to_owned());
            }
            Ok(cookie_header(&cookies))
        })
    }
}

/// Whether the browser's cookies carry a login to the platform.
#[must_use]
pub fn is_logged_in(login: &LoginSpec, cookies: &[Cookie]) -> bool {
    login.is_logged_in(cookies.iter().map(|cookie| cookie.name.as_str()))
}

/// The session of a platform without logins, which never holds one.
struct NoLogins(Platform);

impl Session for NoLogins {
    fn apply<'a>(&'a self, _cookies: &'a str) -> BoxFuture<'a, SessionCheck> {
        Box::pin(async move { SessionCheck::Invalid(format!("{} has no logins", self.0.name())) })
    }

    fn check(&self) -> BoxFuture<'_, SessionCheck> {
        Box::pin(async move { SessionCheck::Invalid(format!("{} has no logins", self.0.name())) })
    }

    fn forget(&self) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

/// Users' wardens work with their own clients and login browser: one
/// browser profile holds a user's logins to every platform.
pub struct Sessions {
    pub pool: Arc<YtMusicPool>,
    pub desks: Arc<LoginDesks>,
    /// How each platform's login works.
    pub platforms: Platforms,
}

impl SessionFactory for Sessions {
    fn session(&self, owner: u64, platform: Platform) -> Box<dyn Session> {
        match platform {
            Platform::YouTubeMusic => Box::new(YtMusicSession {
                pool: Arc::clone(&self.pool),
                owner,
            }),
            platform @ Platform::Deezer => Box::new(NoLogins(platform)),
        }
    }

    fn refresher(&self, owner: u64, platform: Platform) -> Box<dyn Refresher> {
        Box::new(BrowserRefresher {
            desks: Arc::clone(&self.desks),
            owner,
            login: self
                .platforms
                .get(platform)
                .ok()
                .and_then(|source| source.login()),
        })
    }

    fn forget(&self, owner: u64) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.desks.remove(owner).await;
            self.pool.remove(owner);
        })
    }
}

/// Runs jobs with the hunter.
pub struct HuntExecutor {
    pub hunter: Arc<Hunter>,
    pub wardens: Arc<Wardens>,
    /// MusicBrainz and friends, for enriching albums.
    pub sources: Arc<dyn Sources>,
    /// Tells owners about watches that keep failing.
    pub alerts: Arc<dyn AlertSink>,
}

/// The platforms' lyrics, through the hunter.
pub struct HunterLyrics(pub Arc<Hunter>);

impl PlatformLyrics for HunterLyrics {
    fn lyrics<'a>(&'a self, key: &'a SourceKey) -> BoxFuture<'a, Option<(String, String)>> {
        Box::pin(async move {
            let lyrics = match self.0.platforms().of(key) {
                Ok(source) => source.lyrics(key.id()).await,
                Err(error) => Err(error),
            };
            match lyrics {
                Ok(lyrics) => lyrics,
                Err(error) => {
                    tracing::debug!(%error, %key, "no lyrics from the platform");
                    None
                }
            }
        })
    }
}

impl Executor for HuntExecutor {
    fn run<'a>(
        &'a self,
        job: &'a Job,
        progress: &'a (dyn Fn(u8) + Send + Sync),
    ) -> BoxFuture<'a, Outcome> {
        Box::pin(async move {
            match job.kind {
                JobKind::DownloadTrack => self.download(job, progress).await,
                JobKind::GrabAlbum => self.expand_album(job).await,
                JobKind::SyncWatch => self.sync_watch(job).await,
                JobKind::Enrich => self.enrich(job).await,
                // Retired: files no longer move.
                JobKind::Refile => Outcome::Done { track_id: None },
            }
        })
    }
}

impl HuntExecutor {
    async fn download(&self, job: &Job, progress: &(dyn Fn(u8) + Send + Sync)) -> Outcome {
        let payload: TrackJob = match serde_json::from_str(&job.payload) {
            Ok(payload) => payload,
            Err(error) => return Outcome::Failed(format!("invalid job: {error}")),
        };
        let claim = payload.wanted.claim(payload.reference);
        let owner = job.user_id;
        let platform = payload.key.platform();
        // Only platforms with logins have sessions to lend cookies.
        let has_logins = self
            .hunter
            .platforms()
            .get(platform)
            .is_ok_and(|source| source.login().is_some());
        let cookies = if has_logins {
            self.wardens.cookies(owner, platform).await
        } else {
            None
        };
        let request = DownloadRequest {
            owner,
            job_id: job.id,
            key: payload.key.clone(),
            claim: claim.clone(),
            cookies,
        };
        let treasury = self.hunter.treasury();
        // Excluded from its playlist while this was queued or running.
        let excluded = async || match payload.wanted.watch_id() {
            Some(watch_id) => watch::is_excluded(&mut treasury.db(), watch_id, &payload.key)
                .await
                .unwrap_or(false),
            None => false,
        };
        match self.hunter.download(&request, progress).await {
            Ok(track) => {
                if let Some(reference) = &claim.reference
                    && excluded().await
                {
                    let released = treasury
                        .release(
                            owner,
                            claim.kind,
                            reference,
                            Release {
                                reason: ReleaseReason::Excluded,
                                source_name: None,
                            },
                            |other| other.id != track.id,
                        )
                        .await;
                    if let Err(error) = released {
                        tracing::warn!(%error, "cannot drop the claim of an excluded song");
                    }
                }
                // A watch removed while this ran does not keep the track.
                if let (Some(watch_id), Some(reference)) =
                    (payload.wanted.watch_id(), &claim.reference)
                    && !watch_exists(&self.hunter, watch_id).await
                {
                    let released = treasury
                        .release(
                            owner,
                            claim.kind,
                            reference,
                            Release {
                                reason: ReleaseReason::WatchRemoved,
                                source_name: None,
                            },
                            |other| other.id != track.id,
                        )
                        .await;
                    if let Err(error) = released {
                        tracing::warn!(%error, "cannot drop the claim of a removed watch");
                    }
                }
                Outcome::DoneWith {
                    track_id: Some(track.id),
                    then: self.enrich_later(owner, track.album_id).await,
                }
            }
            // Already here: whoever wants it now keeps it too.
            Err(HuntError::AlreadyHoarded { track_id }) if excluded().await => {
                Outcome::AlreadyDone { track_id }
            }
            Err(HuntError::AlreadyHoarded { track_id }) => {
                match treasury.claim(track_id, &claim).await {
                    Ok(()) => Outcome::AlreadyDone { track_id },
                    Err(error) => Outcome::Failed(error.to_string()),
                }
            }
            Err(error) => Outcome::Failed(error.to_string()),
        }
    }

    async fn expand_album(&self, job: &Job) -> Outcome {
        let payload: AlbumJob = match serde_json::from_str(&job.payload) {
            Ok(payload) => payload,
            Err(error) => return Outcome::Failed(format!("invalid job: {error}")),
        };
        let album = match self.hunter.album(&payload.key).await {
            Ok(album) => album,
            Err(error) => return Outcome::Failed(error.to_string()),
        };
        let mut jobs = Vec::new();
        let treasury = self.hunter.treasury();
        let mut db = treasury.db();
        let claim = payload.wanted.claim(Some(album.id.as_stored()));
        for track in &album.tracks {
            let hoarded = match pixiu_db::keyed::track_of_key(&mut db, job.user_id, &track.id).await
            {
                Ok(hoarded) => hoarded,
                Err(error) => return Outcome::Failed(error.to_string()),
            };
            match hoarded {
                // Already here: whoever wants the album keeps it too.
                Some(hoarded) => {
                    if let Err(error) = treasury.claim(hoarded.id, &claim).await {
                        return Outcome::Failed(error.to_string());
                    }
                }
                None => jobs.push(NewJob::wanted_track(
                    &track.id,
                    &format!("{} — {}", track.artist_credit(), track.title),
                    Some(album.id.as_stored()),
                    payload.wanted,
                )),
            }
        }
        Outcome::Expand(jobs)
    }

    /// An enrich job for `owner`'s album, unless one is already waiting or
    /// the album is the artist's singles.
    async fn enrich_later(&self, owner: u64, album_id: u64) -> Vec<NewJob> {
        let mut db = self.hunter.treasury().db();
        let waiting = queue::unfinished(&mut db, owner).await.is_ok_and(|jobs| {
            jobs.iter()
                .any(|job| job.state == JobState::Queued && enriched_album(job) == Some(album_id))
        });
        let Ok(Some(album)) = Album::filter_by_id(album_id).first().exec(&mut db).await else {
            return Vec::new();
        };
        // Songs without an album are no release MusicBrainz could know.
        if waiting || album.title == pixiu_hunt::SINGLES {
            return Vec::new();
        }
        vec![NewJob::enrich(
            album_id,
            &format!("Look up {}", album.title),
            None,
            false,
        )]
    }

    async fn enrich(&self, job: &Job) -> Outcome {
        let payload: EnrichJob = match serde_json::from_str(&job.payload) {
            Ok(payload) => payload,
            Err(error) => return Outcome::Failed(format!("invalid job: {error}")),
        };
        let treasury = self.hunter.treasury();
        // Downloads of the album still to come would each ask again; the
        // last one's request does the work.
        let automatic = payload.release.is_none() && !payload.fresh;
        if automatic
            && self
                .album_downloads_pending(job.user_id, payload.album_id)
                .await
        {
            return Outcome::Done { track_id: None };
        }
        let request = enrich::Request {
            album_id: payload.album_id,
            release: payload.release,
            fresh: payload.fresh,
            genres_only: payload.genres_only,
        };
        let lyrics = HunterLyrics(Arc::clone(&self.hunter));
        match enrich::enrich(treasury, self.sources.as_ref(), &lyrics, &request).await {
            Ok(summary) => {
                tracing::info!(album = request.album_id, %summary, "album enriched");
                Outcome::Done { track_id: None }
            }
            Err(error) => Outcome::Failed(error),
        }
    }

    /// Whether downloads for `owner`'s album (grabbed as an album) are
    /// queued.
    async fn album_downloads_pending(&self, owner: u64, album_id: u64) -> bool {
        let mut db = self.hunter.treasury().db();
        let Ok(Some(album)) = Album::filter_by_id(album_id).first().exec(&mut db).await else {
            return false;
        };
        let Some(key) = SourceKey::from_stored(album.source_key.as_deref()) else {
            return false;
        };
        queue::unfinished(&mut db, owner).await.is_ok_and(|jobs| {
            jobs.iter().any(|job| {
                matches!(job.state, JobState::Queued | JobState::Running)
                    && job.kind == JobKind::DownloadTrack
                    && serde_json::from_str::<TrackJob>(&job.payload).is_ok_and(|payload| {
                        SourceKey::from_stored(payload.reference.as_deref()).as_ref() == Some(&key)
                    })
            })
        })
    }

    async fn sync_watch(&self, job: &Job) -> Outcome {
        let payload: SyncJob = match serde_json::from_str(&job.payload) {
            Ok(payload) => payload,
            Err(error) => return Outcome::Failed(format!("invalid job: {error}")),
        };
        let catalog = PlatformCatalog {
            hunter: Arc::clone(&self.hunter),
            wardens: Arc::clone(&self.wardens),
            owner: job.user_id,
        };
        match watch::sync(
            self.hunter.treasury(),
            &catalog,
            self.alerts.as_ref(),
            payload.watch_id,
        )
        .await
        {
            Ok(Synced::Done(jobs)) => Outcome::Expand(jobs),
            Ok(Synced::NeedsLogin(reason)) => Outcome::Paused(reason),
            Err(reason) => Outcome::Failed(reason),
        }
    }
}

async fn watch_exists(hunter: &Hunter, watch_id: u64) -> bool {
    !matches!(
        Watch::filter_by_id(watch_id)
            .first()
            .exec(&mut hunter.treasury().db())
            .await,
        Ok(None)
    )
}

/// The platforms, as the catalog of a user's watches.
pub struct PlatformCatalog {
    pub hunter: Arc<Hunter>,
    pub wardens: Arc<Wardens>,
    /// Whose watches: their login sees their liked music and private
    /// playlists.
    pub owner: u64,
}

fn catalog_error(key: &SourceKey, error: &HuntError) -> CatalogError {
    if error.needs_login() {
        CatalogError::NeedsLogin(format!("{} wants a login: {error}", key.platform().name()))
    } else {
        CatalogError::Failed(error.to_string())
    }
}

impl Catalog for PlatformCatalog {
    fn playlist<'a>(
        &'a self,
        key: &'a SourceKey,
    ) -> BoxFuture<'a, Result<RemotePlaylist, CatalogError>> {
        Box::pin(async move {
            let source = self
                .hunter
                .platforms()
                .of(key)
                .map_err(|error| catalog_error(key, &error))?;
            source
                .playlist(self.owner, key.id())
                .await
                .map_err(|error| catalog_error(key, &error))
        })
    }

    fn discography<'a>(
        &'a self,
        key: &'a SourceKey,
    ) -> BoxFuture<'a, Result<Discography, CatalogError>> {
        Box::pin(async move {
            let source = self
                .hunter
                .platforms()
                .of(key)
                .map_err(|error| catalog_error(key, &error))?;
            source
                .discography(key.id())
                .await
                .map_err(|error| catalog_error(key, &error))
        })
    }

    fn logged_in(&self, platform: Platform) -> bool {
        matches!(
            self.wardens.health(self.owner, platform).state,
            Some(SessionState::Valid | SessionState::Degraded)
        )
    }
}
