//! The production sides of the warden and the queue: YouTube Music, the
//! browser profile, and the hunter.

use std::sync::Arc;

use pixiu_browser::{Cookie, LoginDesk, cookie_header};
use pixiu_db::{Job, JobKind, SessionState, Track, Watch};
use pixiu_hunt::{Discography, DownloadRequest, HuntError, Hunter, RemotePlaylist, SessionCheck};

use crate::{
    queue::{AlbumJob, Executor, NewJob, Outcome, SyncJob, TrackJob},
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
        let request = DownloadRequest {
            video_id: payload.video_id,
            claim: claim.clone(),
            cookies: self.warden.cookies().await,
        };
        let treasury = self.hunter.treasury();
        match self.hunter.download(&request, progress).await {
            Ok(track) => {
                // A watch removed while this ran does not keep the track.
                if let (Some(watch_id), Some(reference)) =
                    (payload.wanted.watch_id(), &claim.reference)
                    && !watch_exists(&self.hunter, watch_id).await
                {
                    let released = treasury
                        .release(claim.kind, reference, |other| other.id != track.id)
                        .await;
                    if let Err(error) = released {
                        tracing::warn!(%error, "cannot drop the claim of a removed watch");
                    }
                }
                Outcome::Done {
                    track_id: Some(track.id),
                }
            }
            // Already here: whoever wants it now keeps it too.
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
            let hoarded = match Track::filter_by_ytm_video_id(&track.id)
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
