//! The production sides of the warden and the queue: YouTube Music, the
//! browser profile, and the hunter.

use std::sync::Arc;

use pixiu_browser::{Cookie, LoginDesk, cookie_header};
use pixiu_db::{Job, JobKind, Track};
use pixiu_hunt::{DownloadRequest, HuntError, Hunter, SessionCheck};
use pixiu_treasury::Claim;

use crate::{
    queue::{AlbumJob, Executor, NewJob, Outcome, TrackJob},
    warden::{BoxFuture, Platform, Refresher, Warden},
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
        let request = DownloadRequest {
            video_id: payload.video_id,
            claim: Claim {
                kind: pixiu_db::ClaimKind::ManualGrab,
                reference: payload.reference,
            },
            cookies: self.warden.cookies().await,
        };
        match self.hunter.download(&request, progress).await {
            Ok(track) => Outcome::Done {
                track_id: Some(track.id),
            },
            Err(HuntError::AlreadyHoarded { track_id }) => Outcome::AlreadyDone { track_id },
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
        let mut db = self.hunter.treasury().db();
        for track in &album.tracks {
            let hoarded = Track::filter_by_ytm_video_id(&track.id)
                .first()
                .exec(&mut db)
                .await
                .ok()
                .flatten()
                .is_some();
            if !hoarded {
                jobs.push(NewJob::track(
                    &track.id,
                    &format!("{} — {}", track.artist_credit(), track.title),
                    Some(album.id.clone()),
                ));
            }
        }
        Outcome::Expand(jobs)
    }
}
