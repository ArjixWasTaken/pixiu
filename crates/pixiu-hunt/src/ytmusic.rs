//! YouTube Music, through rustypipe.

use std::path::{Path, PathBuf};

use pixiu_core::SecretBox;
use rustypipe::{
    cache::CacheStorage,
    client::RustyPipe,
    error::{AuthError, Error as RpError},
    model::{
        AlbumItem, AlbumType, ArtistId, ArtistItem, AudioCodec, AudioFormat, MusicAlbum, Thumbnail,
        TrackItem, TrackType,
    },
    param::StreamFilter,
};

use crate::{
    HuntError,
    model::{
        AlbumKind, AlbumRef, RemoteAlbum, RemoteArtist, RemoteTrack, SearchResults, SessionCheck,
        best_image_url,
    },
};

/// Where a track's audio can be downloaded from.
#[derive(Debug, Clone)]
pub struct AudioSource {
    pub url: String,
    /// Bytes, when the platform says.
    pub size: Option<u64>,
    /// Downloads must present the same client as the request that found
    /// the stream.
    pub user_agent: String,
    /// Opus in WebM, which píxiū remuxes into Ogg; otherwise kept as is.
    pub is_webm_opus: bool,
    /// The file extension of the downloaded stream.
    pub extension: &'static str,
}

/// rustypipe's cache (client versions, deobfuscation code, and the session
/// cookies), sealed at rest.
struct SealedStorage {
    path: PathBuf,
    secrets: SecretBox,
}

impl CacheStorage for SealedStorage {
    fn write(&self, data: &str) {
        let sealed = self.secrets.seal_str(data);
        let temp = self.path.with_extension("tmp");
        if let Err(error) =
            std::fs::write(&temp, sealed).and_then(|()| std::fs::rename(&temp, &self.path))
        {
            tracing::error!(%error, path = %self.path.display(), "failed to write the YouTube Music cache");
        }
    }

    fn read(&self) -> Option<String> {
        let sealed = std::fs::read_to_string(&self.path).ok()?;
        match self.secrets.open_str(&sealed) {
            Ok(data) => Some(data),
            Err(error) => {
                tracing::warn!(%error, "discarding an unreadable YouTube Music cache");
                None
            }
        }
    }
}

pub struct YtMusic {
    rp: RustyPipe,
}

impl YtMusic {
    /// Creates the client. `dir` holds its sealed cache; `botguard` is the
    /// `rustypipe-botguard` binary that answers YouTube's proof-of-origin
    /// challenges. Without one, streams come from clients that need no
    /// proof, and `yt-dlp` covers what they cannot fetch.
    ///
    /// # Errors
    ///
    /// Fails when the directory cannot be created or the client cannot be
    /// built.
    pub fn new(
        dir: &Path,
        secrets: SecretBox,
        botguard: Option<PathBuf>,
    ) -> Result<Self, HuntError> {
        std::fs::create_dir_all(dir)?;
        let builder = RustyPipe::builder()
            .storage_dir(dir)
            .storage(Box::new(SealedStorage {
                path: dir.join("cache.sealed"),
                secrets,
            }))
            // Failures are logged; report files (megabytes of player code
            // each) would pile up in the data directory.
            .no_reporter();
        // Only the configured binary: rustypipe would otherwise pick one up
        // from `PATH` or the working directory.
        let builder = match botguard {
            Some(botguard) => builder.botguard_bin(botguard),
            None => builder.no_botguard(),
        };
        Ok(Self {
            rp: builder.build()?,
        })
    }

    /// Logs in with browser cookies (a `Cookie` header).
    ///
    /// # Errors
    ///
    /// Fails when YouTube does not accept the cookies.
    pub async fn set_cookies(&self, cookie_header: &str) -> Result<(), HuntError> {
        Ok(self.rp.user_auth_set_cookie(cookie_header).await?)
    }

    /// Logs in with browser cookies, reporting whether YouTube accepted
    /// them.
    pub async fn apply_cookies(&self, cookie_header: &str) -> SessionCheck {
        match self.rp.user_auth_set_cookie(cookie_header).await {
            Ok(()) => SessionCheck::Valid,
            Err(error @ RpError::Auth(_)) => SessionCheck::Invalid(error.to_string()),
            Err(error) => SessionCheck::Unreachable(error.to_string()),
        }
    }

    /// Logs out.
    pub async fn forget_cookies(&self) {
        let _ = self.rp.user_auth_remove_cookie().await;
    }

    /// Asks YouTube whether the stored session still works.
    pub async fn check_session(&self) -> SessionCheck {
        match self.rp.user_auth_check_cookie().await {
            Ok(()) => SessionCheck::Valid,
            Err(RpError::Auth(AuthError::NoLogin)) => {
                SessionCheck::Invalid("YouTube no longer accepts the session".to_owned())
            }
            Err(error @ RpError::Auth(_)) => SessionCheck::Invalid(error.to_string()),
            Err(error) => SessionCheck::Unreachable(error.to_string()),
        }
    }

    /// Searches tracks, albums and artists.
    ///
    /// # Errors
    ///
    /// Fails when YouTube Music cannot be reached or answers unexpectedly.
    pub async fn search(&self, query: &str) -> Result<SearchResults, HuntError> {
        let rp = self.rp.query();
        let (tracks, albums, artists) = tokio::try_join!(
            rp.music_search_tracks(query),
            rp.music_search_albums(query),
            rp.music_search_artists(query),
        )?;
        Ok(SearchResults {
            tracks: tracks.items.items.into_iter().map(track).collect(),
            albums: albums.items.items.into_iter().map(album_item).collect(),
            artists: artists.items.items.into_iter().map(artist_item).collect(),
        })
    }

    /// An album with its tracks.
    ///
    /// # Errors
    ///
    /// Fails when the album does not exist or YouTube Music cannot be
    /// reached.
    pub async fn album(&self, browse_id: &str) -> Result<RemoteAlbum, HuntError> {
        Ok(album(self.rp.query().music_album(browse_id).await?))
    }

    /// A track's metadata.
    ///
    /// # Errors
    ///
    /// Fails when the track does not exist or YouTube Music cannot be
    /// reached.
    pub async fn track(&self, video_id: &str) -> Result<RemoteTrack, HuntError> {
        Ok(track(self.rp.query().music_details(video_id).await?.track))
    }

    /// Finds the best audio stream of a track: Opus in WebM when available.
    ///
    /// # Errors
    ///
    /// Fails when YouTube offers no audio stream for the track.
    pub async fn audio(&self, video_id: &str) -> Result<AudioSource, HuntError> {
        let query = self.rp.query();
        let player = query.player(video_id).await?;
        let opus = StreamFilter::new()
            .no_video()
            .audio_formats([AudioFormat::Webm])
            .audio_codecs([AudioCodec::Opus]);
        let stream = player
            .select_audio_stream(&opus)
            .or_else(|| player.select_audio_stream(&StreamFilter::new().no_video()))
            .ok_or_else(|| HuntError::NoAudio(video_id.to_owned()))?;

        let is_webm_opus = stream.format == AudioFormat::Webm && stream.codec == AudioCodec::Opus;
        Ok(AudioSource {
            url: stream.url.clone(),
            size: Some(stream.size).filter(|size| *size > 0),
            user_agent: query.user_agent(player.client_type).into_owned(),
            is_webm_opus,
            extension: match stream.format {
                AudioFormat::Webm => "webm",
                _ => "m4a",
            },
        })
    }
}

fn artist_names(artists: &[ArtistId]) -> Vec<String> {
    artists.iter().map(|artist| artist.name.clone()).collect()
}

fn image(thumbnails: &[Thumbnail]) -> Option<String> {
    let candidates: Vec<(String, u32)> = thumbnails
        .iter()
        .map(|thumbnail| (thumbnail.url.clone(), thumbnail.width))
        .collect();
    best_image_url(&candidates)
}

fn track(item: TrackItem) -> RemoteTrack {
    RemoteTrack {
        artists: artist_names(&item.artists),
        album: item.album.map(|album| AlbumRef {
            id: album.id,
            title: album.name,
        }),
        duration_secs: item.duration,
        track_number: item.track_nr,
        cover_url: image(&item.cover),
        is_video: item.track_type == TrackType::Video,
        title: item.name,
        id: item.id,
    }
}

fn album_kind(kind: AlbumType) -> AlbumKind {
    match kind {
        AlbumType::Album => AlbumKind::Album,
        AlbumType::Ep => AlbumKind::Ep,
        AlbumType::Single => AlbumKind::Single,
        _ => AlbumKind::Other,
    }
}

fn album_item(item: AlbumItem) -> RemoteAlbum {
    RemoteAlbum {
        artists: artist_names(&item.artists),
        year: item.year,
        kind: album_kind(item.album_type),
        cover_url: image(&item.cover),
        tracks: Vec::new(),
        title: item.name,
        id: item.id,
    }
}

fn album(album: MusicAlbum) -> RemoteAlbum {
    let cover_url = image(&album.cover);
    let artists = artist_names(&album.artists);
    let reference = AlbumRef {
        id: album.id.clone(),
        title: album.name.clone(),
    };
    let tracks = album
        .tracks
        .into_iter()
        .map(|item| {
            let mut track = track(item);
            // Album listings omit what the album page already shows.
            track.album.get_or_insert_with(|| reference.clone());
            if track.cover_url.is_none() {
                track.cover_url.clone_from(&cover_url);
            }
            if track.artists.is_empty() {
                track.artists.clone_from(&artists);
            }
            track
        })
        .collect();
    RemoteAlbum {
        id: album.id,
        title: album.name,
        artists,
        year: album.year,
        kind: album_kind(album.album_type),
        cover_url,
        tracks,
    }
}

fn artist_item(item: ArtistItem) -> RemoteArtist {
    RemoteArtist {
        image_url: image(&item.avatar),
        name: item.name,
        id: item.id,
    }
}
