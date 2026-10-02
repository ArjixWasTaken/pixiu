//! Hunting: finding music on streaming platforms and bringing it into the
//! hoard.

pub mod deezer;
mod download;
mod link;
pub mod model;
pub mod source;
mod tagging;
pub mod ytmusic;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use pixiu_db::{AudioFile, Platform, SourceKey, Track, toasty};
use pixiu_treasury::{Claim, Cover, IngestError, Provenance, Treasury, tags};

pub use deezer::DeezerSource;
pub use model::{
    AlbumKind, AlbumRef, Discography, RemoteAlbum, RemoteArtist, RemotePlaylist, RemoteTrack,
    SearchResults, SessionCheck, image_url_at,
};
pub use source::{Link, LoginSpec, Page, Platforms, Source, YtDlpTarget};
pub use ytmusic::{AudioSource, LIKED_MUSIC, YouTubeMusicSource, YtMusic, YtMusicPool};

#[derive(Debug, thiserror::Error)]
pub enum HuntError {
    #[error("the hoard already holds this track")]
    AlreadyHoarded { track_id: u64 },
    #[error("{}: {message}", platform.name())]
    Source {
        platform: Platform,
        message: String,
        /// The platform wants a login for this, e.g. for liked music or a
        /// private playlist.
        needs_login: bool,
    },
    #[error("píxiū cannot download from {} yet", .0.name())]
    NoSource(Platform),
    #[error("no audio stream for {0}")]
    NoAudio(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("yt-dlp couldn't download it: {0}")]
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

/// The album songs without one are filed under, per artist. It is no
/// release, so MusicBrainz is not asked about it.
pub const SINGLES: &str = "Singles";

/// How often yt-dlp is tried again when YouTube refuses it.
const YT_DLP_RETRIES: u32 = 2;

/// The bitrate lossless downloads are stored at, as Opus: transparent for
/// nearly everyone, at a seventh of the size.
const LOSSLESS_AS_OPUS_KBPS: u32 = 160;

impl HuntError {
    /// Whether the platform wants a login for this, e.g. for liked music or
    /// a private playlist.
    #[must_use]
    pub fn needs_login(&self) -> bool {
        matches!(
            self,
            Self::Source {
                needs_login: true,
                ..
            }
        )
    }
}

impl From<rustypipe::error::Error> for HuntError {
    fn from(error: rustypipe::error::Error) -> Self {
        HuntError::Source {
            platform: Platform::YouTubeMusic,
            needs_login: matches!(error, rustypipe::error::Error::Auth(_)),
            message: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for HuntError {
    fn from(error: reqwest::Error) -> Self {
        // Stream URLs are signed and name the server's IP address; keep
        // them out of logs and the web player.
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
    /// The user whose library the track joins.
    pub owner: u64,
    /// The job downloading it, which names its staging directory.
    pub job_id: u64,
    /// The song on its platform.
    pub key: SourceKey,
    pub claim: Claim,
    /// Session cookies, for the `yt-dlp` fallback.
    pub cookies: Option<String>,
}

/// Downloads music into the treasure.
pub struct Hunter {
    platforms: Platforms,
    treasury: Treasury,
    staging: PathBuf,
    http: reqwest::Client,
    albums: Mutex<HashMap<SourceKey, (Instant, RemoteAlbum)>>,
    /// Songs being downloaded: a second download of the same song waits,
    /// then finds its file stored.
    songs: Mutex<HashMap<SourceKey, Arc<tokio::sync::Mutex<()>>>>,
}

impl Hunter {
    /// # Errors
    ///
    /// Fails when the staging directory cannot be created.
    pub fn new(
        platforms: Platforms,
        treasury: Treasury,
        staging: PathBuf,
    ) -> Result<Self, HuntError> {
        std::fs::create_dir_all(&staging)?;
        Ok(Self {
            platforms,
            treasury,
            staging,
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(20))
                .read_timeout(Duration::from_secs(60))
                .build()?,
            albums: Mutex::default(),
            songs: Mutex::default(),
        })
    }

    /// The platforms it downloads from.
    #[must_use]
    pub fn platforms(&self) -> &Platforms {
        &self.platforms
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
    pub async fn album(&self, key: &SourceKey) -> Result<RemoteAlbum, HuntError> {
        if let Some((fetched, album)) = self.albums.lock().unwrap().get(key)
            && fetched.elapsed() < ALBUM_CACHE_TTL
        {
            return Ok(album.clone());
        }
        let album = self.platforms.of(key)?.album(key.id()).await?;
        let mut cache = self.albums.lock().unwrap();
        cache.retain(|_, (fetched, _)| fetched.elapsed() < ALBUM_CACHE_TTL);
        cache.insert(key.clone(), (Instant::now(), album.clone()));
        Ok(album)
    }

    /// Downloads a track, tags it and adds it to the owner's library. A
    /// video the store holds already (someone else downloaded it) is not
    /// downloaded again: the new track shares the file. `progress`
    /// receives percentages.
    ///
    /// # Errors
    ///
    /// Fails when the owner has the track already, or it cannot be
    /// downloaded (by the platform client nor `yt-dlp`) or filed.
    pub async fn download(
        &self,
        request: &DownloadRequest,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<Track, HuntError> {
        let song = Arc::clone(
            self.songs
                .lock()
                .unwrap()
                .entry(request.key.clone())
                .or_default(),
        );
        let result = {
            let _one_at_a_time = song.lock().await;
            self.download_song(request, progress).await
        };
        // Forget the lock once nobody else waits on it.
        let mut songs = self.songs.lock().unwrap();
        if Arc::strong_count(&song) == 2 {
            songs.remove(&request.key);
        }
        result
    }

    async fn download_song(
        &self,
        request: &DownloadRequest,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<Track, HuntError> {
        let key = &request.key;
        let mut db = self.treasury.db();
        if let Some(existing) = pixiu_db::keyed::track_of_key(&mut db, request.owner, key).await? {
            return Err(HuntError::AlreadyHoarded {
                track_id: existing.id,
            });
        }

        progress(1);
        let source = self.platforms.of(key)?;
        let track = source.track(key.id()).await?;
        let album = match &track.album {
            Some(reference) => match self.album(&reference.id).await {
                Ok(album) => Some(album),
                Err(error) => {
                    tracing::warn!(%error, %key, "album metadata unavailable");
                    None
                }
            },
            None => None,
        };
        progress(5);

        let stored = AudioFile::filter_by_source_key(Some(key.as_stored()))
            .first()
            .exec(&mut db)
            .await?;
        if let Some(file) = stored
            && tokio::fs::try_exists(self.treasury.resolve(&file.path)).await?
        {
            let filed = self
                .file_stored(request, &track, album.as_ref(), file)
                .await?;
            progress(100);
            return Ok(filed);
        }

        let staging = self.staging.join(request.job_id.to_string());
        tokio::fs::create_dir_all(&staging).await?;
        let result = self
            .fetch_and_file(request, &staging, &track, album.as_ref(), progress)
            .await;
        if let Err(error) = tokio::fs::remove_dir_all(&staging).await {
            tracing::warn!(%error, path = %staging.display(), "cannot clean up staging");
        }
        result
    }

    /// Adds a track playing a file the store holds, described as a fresh
    /// download would be.
    async fn file_stored(
        &self,
        request: &DownloadRequest,
        track: &RemoteTrack,
        album: Option<&RemoteAlbum>,
        file: AudioFile,
    ) -> Result<Track, HuntError> {
        let described = describe(self.platforms.of(&request.key)?, &request.key, track, album);
        let path = self.treasury.resolve(&file.path);
        let mut info = tokio::task::spawn_blocking(move || tags::read(&path))
            .await?
            .map_err(|error| HuntError::Tags(error.to_string()))?;
        info.title = Some(track.title.clone());
        info.artist = Some(described.credit.clone());
        info.album = Some(described.album_title.to_owned());
        info.album_artist = Some(described.album_artist.to_owned());
        info.track_number = described.track_number;
        if described.disc_number.is_some() {
            info.disc_number = described.disc_number;
        }
        info.year = described.year.map(i32::from);
        if let Some(isrc) = described.isrc {
            info.isrc = Some(isrc.to_owned());
        }
        info.artists.clone_from(&track.artists);
        let file_id = file.id;
        let stored = self
            .treasury
            .ingest_stored(
                request.owner,
                file,
                &info,
                described.provenance,
                request.claim.clone(),
            )
            .await
            .map_err(|error| match error {
                IngestError::Duplicate { track_id } => HuntError::AlreadyHoarded { track_id },
                other => HuntError::Ingest(other),
            })?;
        tracing::info!(key = %request.key, file = file_id, "reused a stored file");
        Ok(stored)
    }

    async fn fetch_and_file(
        &self,
        request: &DownloadRequest,
        staging: &Path,
        track: &RemoteTrack,
        album: Option<&RemoteAlbum>,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<Track, HuntError> {
        let downloaded = self.fetch_audio(request, staging, progress).await?;
        progress(85);

        // Opus in WebM becomes Ogg Opus, and lossless audio is encoded as
        // Opus; anything else is kept as served.
        let extension = downloaded
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        // Each job has its own staging directory.
        let output = staging.join("audio.opus");
        let (input, target) = (downloaded.clone(), output.clone());
        let audio = match extension.as_deref() {
            Some("webm") => {
                tokio::task::spawn_blocking(move || pixiu_media::remux(&input, &target)).await??;
                output
            }
            Some("flac") => {
                let opus = pixiu_media::Target {
                    codec: pixiu_media::Codec::Opus,
                    bitrate: LOSSLESS_AS_OPUS_KBPS,
                };
                tokio::task::spawn_blocking(move || pixiu_media::encode(&input, &target, opus))
                    .await??;
                output
            }
            _ => downloaded,
        };
        progress(90);

        let described = describe(self.platforms.of(&request.key)?, &request.key, track, album);
        let cover = match described.cover_url {
            Some(url) => self.fetch_cover(url).await,
            None => None,
        };
        {
            let tags = tagging::TrackTags {
                title: &track.title,
                artist_credit: &described.credit,
                album: described.album_title,
                album_artist: described.album_artist,
                track_number: described.track_number,
                disc_number: described.disc_number,
                year: described.year,
                isrc: described.isrc,
                source_url: &described.source_url,
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

        let stored = self
            .treasury
            .ingest(
                request.owner,
                &audio,
                &info,
                None,
                described.provenance,
                request.claim.clone(),
            )
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
    async fn yt_dlp(
        &self,
        request: &DownloadRequest,
        staging: &Path,
    ) -> Result<PathBuf, HuntError> {
        let key = &request.key;
        let Some(target) = self.platforms.of(key)?.yt_dlp(key.id()) else {
            return Err(HuntError::YtDlp(format!(
                "it cannot download from {}",
                key.platform().name()
            )));
        };
        let mut attempt = 0;
        loop {
            let result = download::yt_dlp(&target, staging, request.cookies.as_deref()).await;
            match result {
                Err(HuntError::YtDlp(reason))
                    if attempt < YT_DLP_RETRIES
                        && (reason.contains("HTTP Error 403")
                            || reason.contains("HTTP Error 5")) =>
                {
                    attempt += 1;
                    tracing::info!(%key, %reason, attempt, "yt-dlp was refused; trying again");
                    tokio::time::sleep(Duration::from_secs(5 * u64::from(attempt))).await;
                }
                result => return result,
            }
        }
    }

    async fn fetch_audio(
        &self,
        request: &DownloadRequest,
        staging: &Path,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<PathBuf, HuntError> {
        let key = &request.key;
        let report = |done: u64, total: Option<u64>| {
            if let Some(total) = total.filter(|total| *total > 0) {
                let share = done.saturating_mul(75) / total;
                progress(5 + u8::try_from(share.min(75)).unwrap_or(75));
            }
        };

        let direct = async {
            // The owner's login, when they have one, lets streams that need
            // one through.
            let source = self
                .platforms
                .of(key)?
                .audio(request.owner, key.id())
                .await?;
            let path = staging.join(format!("audio.{}", source.extension));
            download::fetch(&self.http, &source, &path, &report).await?;
            Ok::<_, HuntError>(path)
        }
        .await;

        match direct {
            Ok(path) => Ok(path),
            // Without yt-dlp to fall back on, the platform's own reason is
            // the one to give.
            Err(error) if self.platforms.of(key)?.yt_dlp(key.id()).is_none() => Err(error),
            Err(error) => {
                tracing::warn!(%error, %key, "direct download failed; trying yt-dlp");
                let path = self.yt_dlp(request, staging).await?;
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

    /// The staging directory, for tests and diagnostics.
    #[must_use]
    pub fn staging(&self) -> &Path {
        &self.staging
    }
}

/// A track as a download files it: tags and provenance, from the
/// platform's metadata.
struct Described<'a> {
    credit: String,
    album_title: &'a str,
    album_artist: &'a str,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    year: Option<u16>,
    isrc: Option<&'a str>,
    source_url: String,
    cover_url: Option<&'a str>,
    provenance: Provenance,
}

fn describe<'a>(
    source: &dyn Source,
    key: &SourceKey,
    track: &'a RemoteTrack,
    album: Option<&'a RemoteAlbum>,
) -> Described<'a> {
    let album_title = album.map_or_else(
        || {
            track
                .album
                .as_ref()
                .map_or(SINGLES, |album| album.title.as_str())
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
    // The channel of whichever artist became the album artist above.
    let album_artist_id = match album {
        Some(album) if !album.artists.is_empty() => album.artist_id.clone(),
        _ => track.artist_id.clone(),
    };
    Described {
        credit: track.artist_credit(),
        album_title,
        album_artist,
        track_number,
        disc_number: track.disc_number.map(u32::from),
        year: album.and_then(|album| album.year),
        isrc: track.isrc.as_deref(),
        source_url: source.page_url(Page::Song, key.id()),
        cover_url: album
            .and_then(|album| album.cover_url.as_deref())
            .or(track.cover_url.as_deref()),
        provenance: Provenance::download(
            key.clone(),
            album
                .map(|album| album.id.clone())
                .or_else(|| track.album.as_ref().map(|album| album.id.clone())),
            album_artist_id,
        ),
    }
}
