//! Hunting: finding music on streaming platforms and bringing it into the
//! hoard.

mod download;
pub mod link;
pub mod model;
mod tagging;
pub mod ytmusic;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

use pixiu_db::{Track, toasty};
use pixiu_treasury::{Claim, Cover, IngestError, Provenance, Treasury, tags};

pub use model::{
    AlbumKind, AlbumRef, Discography, RemoteAlbum, RemoteArtist, RemotePlaylist, RemoteTrack,
    SearchResults, SessionCheck, image_url_at,
};
pub use ytmusic::{AudioSource, LIKED_MUSIC, YtMusic};

#[derive(Debug, thiserror::Error)]
pub enum HuntError {
    #[error("the hoard already holds this track")]
    AlreadyHoarded { track_id: u64 },
    #[error("YouTube Music: {0}")]
    YouTube(#[from] rustypipe::error::Error),
    #[error("no audio stream for {0}")]
    NoAudio(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("yt-dlp failed: {0}")]
    YtDlp(String),
    #[error("media: {0}")]
    Media(#[from] pixiu_media::MediaError),
    #[error("tags: {0}")]
    Tags(String),
    #[error(transparent)]
    Ingest(#[from] IngestError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] toasty::Error),
    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// How often yt-dlp is tried again when YouTube refuses it.
const YT_DLP_RETRIES: u32 = 2;

impl HuntError {
    /// Whether the platform wants a login for this, e.g. for liked music or
    /// a private playlist.
    #[must_use]
    pub fn needs_login(&self) -> bool {
        matches!(self, Self::YouTube(rustypipe::error::Error::Auth(_)))
    }
}

impl From<reqwest::Error> for HuntError {
    fn from(error: reqwest::Error) -> Self {
        // Stream URLs are signed and name the server's IP address; keep
        // them out of logs and the WebUI.
        HuntError::Download(error.without_url().to_string())
    }
}

impl From<lofty::error::FileParseError> for HuntError {
    fn from(error: lofty::error::FileParseError) -> Self {
        HuntError::Tags(error.to_string())
    }
}

impl From<lofty::error::FileEncodingError> for HuntError {
    fn from(error: lofty::error::FileEncodingError) -> Self {
        HuntError::Tags(error.to_string())
    }
}

/// How long fetched albums are reused: an album grab downloads its tracks
/// one by one, and each needs the album's metadata.
const ALBUM_CACHE_TTL: Duration = Duration::from_secs(600);

/// One track to bring home.
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub video_id: String,
    pub claim: Claim,
    /// Session cookies, for the `yt-dlp` fallback.
    pub cookies: Option<String>,
}

/// Downloads music into the treasure.
pub struct Hunter {
    ytm: YtMusic,
    treasury: Treasury,
    staging: PathBuf,
    http: reqwest::Client,
    albums: Mutex<HashMap<String, (Instant, RemoteAlbum)>>,
}

impl Hunter {
    /// # Errors
    ///
    /// Fails when the staging directory cannot be created.
    pub fn new(ytm: YtMusic, treasury: Treasury, staging: PathBuf) -> Result<Self, HuntError> {
        std::fs::create_dir_all(&staging)?;
        Ok(Self {
            ytm,
            treasury,
            staging,
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(20))
                .read_timeout(Duration::from_secs(60))
                .build()?,
            albums: Mutex::default(),
        })
    }

    #[must_use]
    pub fn ytmusic(&self) -> &YtMusic {
        &self.ytm
    }

    #[must_use]
    pub fn treasury(&self) -> &Treasury {
        &self.treasury
    }

    /// An album with its tracks, reused for a few minutes.
    ///
    /// # Errors
    ///
    /// Fails when the platform cannot be reached.
    pub async fn album(&self, browse_id: &str) -> Result<RemoteAlbum, HuntError> {
        if let Some((fetched, album)) = self.albums.lock().unwrap().get(browse_id)
            && fetched.elapsed() < ALBUM_CACHE_TTL
        {
            return Ok(album.clone());
        }
        let album = self.ytm.album(browse_id).await?;
        let mut cache = self.albums.lock().unwrap();
        cache.retain(|_, (fetched, _)| fetched.elapsed() < ALBUM_CACHE_TTL);
        cache.insert(browse_id.to_owned(), (Instant::now(), album.clone()));
        Ok(album)
    }

    /// Downloads a track, tags it and files it into the treasure.
    /// `progress` receives percentages.
    ///
    /// # Errors
    ///
    /// Fails when the track is already hoarded, cannot be downloaded (by
    /// the platform client nor `yt-dlp`), or cannot be filed.
    pub async fn download(
        &self,
        request: &DownloadRequest,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<Track, HuntError> {
        let video_id = request.video_id.as_str();
        if let Some(existing) = Track::filter_by_ytm_video_id(video_id)
            .first()
            .exec(&mut self.treasury.db())
            .await?
        {
            return Err(HuntError::AlreadyHoarded {
                track_id: existing.id,
            });
        }

        progress(1);
        let track = self.ytm.track(video_id).await?;
        let album = match &track.album {
            Some(reference) => match self.album(&reference.id).await {
                Ok(album) => Some(album),
                Err(error) => {
                    tracing::warn!(%error, video_id, "album metadata unavailable");
                    None
                }
            },
            None => None,
        };
        progress(5);

        let result = self
            .fetch_and_file(request, &track, album.as_ref(), progress)
            .await;
        self.clean_staging(video_id).await;
        result
    }

    async fn fetch_and_file(
        &self,
        request: &DownloadRequest,
        track: &RemoteTrack,
        album: Option<&RemoteAlbum>,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<Track, HuntError> {
        let video_id = request.video_id.as_str();
        let downloaded = self.fetch_audio(request, progress).await?;
        progress(85);

        // Opus in WebM becomes Ogg Opus; anything else is kept as served.
        let is_webm = downloaded
            .extension()
            .is_some_and(|extension| extension == "webm");
        let audio = if is_webm {
            let output = self.staging.join(format!("{video_id}.opus"));
            let (input, target) = (downloaded.clone(), output.clone());
            tokio::task::spawn_blocking(move || pixiu_media::remux(&input, &target)).await??;
            output
        } else {
            downloaded
        };
        progress(90);

        let cover_url = album
            .and_then(|album| album.cover_url.as_deref())
            .or(track.cover_url.as_deref());
        let cover = match cover_url {
            Some(url) => self.fetch_cover(url).await,
            None => None,
        };

        let album_title = album.map_or_else(
            || {
                track
                    .album
                    .as_ref()
                    .map_or("Singles", |album| album.title.as_str())
            },
            |album| album.title.as_str(),
        );
        let album_artist = album
            .and_then(|album| album.artists.first())
            .or(track.artists.first())
            .map_or("Unknown Artist", String::as_str);
        let track_number = track.track_number.map(u32::from).or_else(|| {
            let position = album?.tracks.iter().position(|item| item.id == track.id)?;
            u32::try_from(position + 1).ok()
        });
        let credit = track.artist_credit();
        let source_url = format!("https://music.youtube.com/watch?v={video_id}");
        let year = album.and_then(|album| album.year);

        {
            let tags = tagging::TrackTags {
                title: &track.title,
                artist_credit: &credit,
                album: album_title,
                album_artist,
                track_number,
                year,
                source_url: &source_url,
                cover: cover.as_ref(),
            };
            let path = audio.clone();
            // Tagging is quick and the tags borrow local data, so it runs
            // inline rather than on the blocking pool.
            tagging::write(&path, &tags)?;
        }
        progress(95);

        let path = audio.clone();
        let mut info = tokio::task::spawn_blocking(move || tags::read(&path))
            .await?
            .map_err(|error| HuntError::Tags(error.to_string()))?;
        info.artists.clone_from(&track.artists);

        // The channel of whichever artist became the album artist above.
        let album_artist_id = match album {
            Some(album) if !album.artists.is_empty() => album.artist_id.clone(),
            _ => track.artist_id.clone(),
        };
        let provenance = Provenance::youtube_music(
            video_id,
            album
                .map(|album| album.id.clone())
                .or_else(|| track.album.as_ref().map(|album| album.id.clone())),
            album_artist_id,
        );
        let stored = self
            .treasury
            .ingest(&audio, &info, None, provenance, request.claim.clone())
            .await
            .map_err(|error| match error {
                IngestError::Duplicate { track_id } => HuntError::AlreadyHoarded { track_id },
                other => HuntError::Ingest(other),
            })?;
        progress(100);
        Ok(stored)
    }

    /// Downloads the audio into staging, falling back to `yt-dlp`.
    /// Runs yt-dlp, again after a pause when YouTube refuses or fails it:
    /// that often passes, e.g. once other downloads are done.
    async fn yt_dlp(&self, request: &DownloadRequest) -> Result<PathBuf, HuntError> {
        let video_id = request.video_id.as_str();
        let mut attempt = 0;
        loop {
            let result =
                download::yt_dlp(video_id, &self.staging, request.cookies.as_deref()).await;
            match result {
                Err(HuntError::YtDlp(reason))
                    if attempt < YT_DLP_RETRIES
                        && (reason.contains("HTTP Error 403")
                            || reason.contains("HTTP Error 5")) =>
                {
                    attempt += 1;
                    tracing::info!(video_id, %reason, attempt, "yt-dlp was refused; trying again");
                    tokio::time::sleep(Duration::from_secs(5 * u64::from(attempt))).await;
                }
                result => return result,
            }
        }
    }

    async fn fetch_audio(
        &self,
        request: &DownloadRequest,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<PathBuf, HuntError> {
        let video_id = request.video_id.as_str();
        let report = |done: u64, total: Option<u64>| {
            if let Some(total) = total.filter(|total| *total > 0) {
                let share = done.saturating_mul(75) / total;
                progress(5 + u8::try_from(share.min(75)).unwrap_or(75));
            }
        };

        let direct = async {
            let source = self.ytm.audio(video_id).await?;
            let path = self
                .staging
                .join(format!("{video_id}.{}", source.extension));
            download::fetch(&self.http, &source, &path, &report).await?;
            Ok::<_, HuntError>(path)
        }
        .await;

        match direct {
            Ok(path) => Ok(path),
            Err(error) => {
                tracing::warn!(%error, video_id, "direct download failed; trying yt-dlp");
                let path = self.yt_dlp(request).await?;
                progress(80);
                Ok(path)
            }
        }
    }

    async fn fetch_cover(&self, url: &str) -> Option<Cover> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map_or("image/jpeg", |mime| {
                if mime.starts_with("image/png") {
                    "image/png"
                } else {
                    "image/jpeg"
                }
            })
            .to_owned();
        let data = response.bytes().await.ok()?.to_vec();
        pixiu_treasury::covers::is_displayable(&data).then_some(Cover { data, mime })
    }

    /// Removes whatever a download left in staging.
    async fn clean_staging(&self, video_id: &str) {
        let Ok(mut entries) = tokio::fs::read_dir(&self.staging).await else {
            return;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let leftover = entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(&format!("{video_id}.")));
            if leftover {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }
    }

    /// The staging directory, for tests and diagnostics.
    #[must_use]
    pub fn staging(&self) -> &Path {
        &self.staging
    }
}
