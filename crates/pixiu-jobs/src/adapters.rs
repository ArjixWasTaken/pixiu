//! The production sides of the warden and the queue: YouTube Music, the
//! browser profile, and the hunter.

use std::sync::Arc;

use pixiu_browser::{Cookie, LoginDesk, cookie_header};
use pixiu_db::{Album, Job, JobKind, JobState, ReleaseReason, SessionState, Track, Watch};
use pixiu_enrich::Sources;
use pixiu_hunt::{Discography, DownloadRequest, HuntError, Hunter, RemotePlaylist, SessionCheck};
use pixiu_treasury::Release;

use crate::{
    enrich::{self, PlatformLyrics},
    queue::{
        self, AlbumJob, EnrichJob, Executor, NewJob, Outcome, SyncJob, TrackJob, enriched_album,
    },
    warden::{BoxFuture, Platform, Refresher, Warden},
    watch::{self, Catalog, CatalogError, Synced},
};

/// Where the login browser starts and the warden refreshes.
pub const YOUTUBE_MUSIC_URL: &str = "https://music.youtube.com/";
/// Google's sign-in page, returning to YouTube Music.
pub const LOGIN_URL: &str = "https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fmusic.youtube.com%2F";
/// The cookies that matter live on this domain.
pub const COOKIE_DOMAIN: &str = "youtube.com";

/// Whether the cookies carry a Google login.
#[must_use]
pub fn is_logged_in(cookies: &[Cookie]) -> bool {
    cookies
        .iter()
        .any(|cookie| matches!(cookie.name.as_str(), "SAPISID" | "__Secure-3PAPISID"))
}

/// YouTube Music, as the warden's platform.
pub struct YtMusicPlatform(pub Arc<Hunter>);

impl Platform for YtMusicPlatform {
    fn apply<'a>(&'a self, cookies: &'a str) -> BoxFuture<'a, SessionCheck> {
        Box::pin(self.0.ytmusic().apply_cookies(cookies))
    }

    fn check(&self) -> BoxFuture<'_, SessionCheck> {
        Box::pin(self.0.ytmusic().check_session())
    }

    fn forget(&self) -> BoxFuture<'_, ()> {
        Box::pin(self.0.ytmusic().forget_cookies())
    }
}

/// Fresh cookies from the login browser's persistent profile.
pub struct BrowserRefresher(pub Arc<LoginDesk>);

impl Refresher for BrowserRefresher {
    fn refresh(&self) -> BoxFuture<'_, Result<String, String>> {
        Box::pin(async move {
            let cookies = self
                .0
                .harvest(YOUTUBE_MUSIC_URL, COOKIE_DOMAIN)
                .await
                .map_err(|error| error.to_string())?;
            if !is_logged_in(&cookies) {
                return Err("the browser profile is no longer logged in".to_owned());
            }
            Ok(cookie_header(&cookies))
        })
    }
}

/// Runs jobs with the hunter.
pub struct HuntExecutor {
    pub hunter: Arc<Hunter>,
    pub warden: Arc<Warden>,
    /// MusicBrainz and friends, for enriching albums.
    pub sources: Arc<dyn Sources>,
}

/// YouTube Music lyrics, through the hunter.
pub struct HunterLyrics(pub Arc<Hunter>);

impl PlatformLyrics for HunterLyrics {
    fn lyrics<'a>(&'a self, video_id: &'a str) -> BoxFuture<'a, Option<(String, String)>> {
        Box::pin(async move {
            match self.0.ytmusic().lyrics(video_id).await {
                Ok(lyrics) => lyrics,
                Err(error) => {
                    tracing::debug!(%error, video_id, "no lyrics from YouTube Music");
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
        let request = DownloadRequest {
            owner,
            job_id: job.id,
            video_id: payload.video_id.clone(),
            claim: claim.clone(),
            cookies: self.warden.cookies().await,
        };
        let treasury = self.hunter.treasury();
        // Excluded from its playlist while this was queued or running.
        let excluded = async || match payload.wanted.watch_id() {
            Some(watch_id) => watch::is_excluded(&mut treasury.db(), watch_id, &payload.video_id)
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
        let album = match self.hunter.album(&payload.browse_id).await {
            Ok(album) => album,
            Err(error) => return Outcome::Failed(error.to_string()),
        };
        let mut jobs = Vec::new();
        let treasury = self.hunter.treasury();
        let mut db = treasury.db();
        let claim = payload.wanted.claim(Some(album.id.clone()));
        for track in &album.tracks {
            let hoarded = match Track::filter_by_user_id_and_ytm_video_id(job.user_id, &track.id)
                .first()
                .exec(&mut db)
                .await
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
                    Some(album.id.clone()),
                    payload.wanted,
                )),
            }
        }
        Outcome::Expand(jobs)
    }

    /// An enrich job for `owner`'s album, unless one is already waiting.
    async fn enrich_later(&self, owner: u64, album_id: u64) -> Vec<NewJob> {
        let mut db = self.hunter.treasury().db();
        let waiting = queue::unfinished(&mut db, owner).await.is_ok_and(|jobs| {
            jobs.iter()
                .any(|job| job.state == JobState::Queued && enriched_album(job) == Some(album_id))
        });
        let Ok(Some(album)) = Album::filter_by_id(album_id).first().exec(&mut db).await else {
            return Vec::new();
        };
        if waiting {
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
        let Some(browse_id) = album.ytm_browse_id else {
            return false;
        };
        queue::unfinished(&mut db, owner).await.is_ok_and(|jobs| {
            jobs.iter().any(|job| {
                matches!(job.state, JobState::Queued | JobState::Running)
                    && job.kind == JobKind::DownloadTrack
                    && serde_json::from_str::<TrackJob>(&job.payload).is_ok_and(|payload| {
                        payload.reference.as_deref() == Some(browse_id.as_str())
                    })
            })
        })
    }

    async fn sync_watch(&self, job: &Job) -> Outcome {
        let payload: SyncJob = match serde_json::from_str(&job.payload) {
            Ok(payload) => payload,
            Err(error) => return Outcome::Failed(format!("invalid job: {error}")),
        };
        let catalog = YtMusicCatalog {
            hunter: Arc::clone(&self.hunter),
            warden: Arc::clone(&self.warden),
        };
        match watch::sync(self.hunter.treasury(), &catalog, payload.watch_id).await {
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

/// YouTube Music, as the catalog of watches.
pub struct YtMusicCatalog {
    pub hunter: Arc<Hunter>,
    pub warden: Arc<Warden>,
}

fn catalog_error(error: &HuntError) -> CatalogError {
    if error.needs_login() {
        CatalogError::NeedsLogin(format!("YouTube Music wants a login: {error}"))
    } else {
        CatalogError::Failed(error.to_string())
    }
}

impl Catalog for YtMusicCatalog {
    fn playlist<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemotePlaylist, CatalogError>> {
        Box::pin(async move {
            self.hunter
                .ytmusic()
                .playlist(id)
                .await
                .map_err(|error| catalog_error(&error))
        })
    }

    fn discography<'a>(
        &'a self,
        channel_id: &'a str,
    ) -> BoxFuture<'a, Result<Discography, CatalogError>> {
        Box::pin(async move {
            self.hunter
                .ytmusic()
                .discography(channel_id)
                .await
                .map_err(|error| catalog_error(&error))
        })
    }

    fn logged_in(&self) -> bool {
        matches!(
            self.warden.health().state,
            Some(SessionState::Valid | SessionState::Degraded)
        )
    }
}
