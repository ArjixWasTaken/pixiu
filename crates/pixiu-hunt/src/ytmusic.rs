//! YouTube Music, through rustypipe.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

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

use futures_util::future::BoxFuture;
use pixiu_db::{Platform, SourceKey};

use crate::{
    HuntError,
    model::{
        AlbumKind, AlbumRef, Discography, RemoteAlbum, RemoteArtist, RemotePlaylist, RemoteTrack,
        SearchResults, SessionCheck, best_image_url,
    },
    source::{Link, LoginSpec, Page, Source, YtDlpTarget},
};

/// The playlist id of an account's liked music.
pub const LIKED_MUSIC: &str = "LM";

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

    /// A playlist with all its tracks; [`LIKED_MUSIC`] is the account's
    /// liked music.
    ///
    /// # Errors
    ///
    /// Fails when the playlist does not exist, needs a login (see
    /// [`HuntError::needs_login`]), or YouTube Music cannot be reached.
    pub async fn playlist(&self, id: &str) -> Result<RemotePlaylist, HuntError> {
        let query = self.rp.query();
        let mut playlist = if id == LIKED_MUSIC {
            query.music_liked_tracks().await?
        } else {
            query.music_playlist(id).await?
        };
        playlist.tracks.extend_all(&query).await?;
        Ok(RemotePlaylist {
            image_url: image(&playlist.thumbnail),
            id: SourceKey::youtube_music(playlist.id),
            name: playlist.name,
            tracks: playlist.tracks.items.into_iter().map(track).collect(),
        })
    }

    /// An artist's releases: albums, EPs and singles.
    ///
    /// # Errors
    ///
    /// Fails when the artist does not exist or YouTube Music cannot be
    /// reached.
    pub async fn discography(&self, channel_id: &str) -> Result<Discography, HuntError> {
        let artist = self.rp.query().music_artist(channel_id, true).await?;
        Ok(Discography {
            image_url: image(&artist.header_image),
            id: SourceKey::youtube_music(artist.id),
            name: artist.name,
            albums: artist.albums.into_iter().map(album_item).collect(),
        })
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

    /// A track's lyrics as YouTube Music shows them (plain text), with the
    /// credit line naming their source.
    ///
    /// # Errors
    ///
    /// Fails when YouTube Music cannot be reached.
    pub async fn lyrics(&self, video_id: &str) -> Result<Option<(String, String)>, HuntError> {
        let query = self.rp.query();
        let Some(lyrics_id) = query.music_details(video_id).await?.lyrics_id else {
            return Ok(None);
        };
        let lyrics = query.music_lyrics(&lyrics_id).await?;
        Ok((!lyrics.body.trim().is_empty()).then_some((lyrics.body, lyrics.footer)))
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

/// YouTube Music clients. rustypipe keeps one login per client, and a
/// client with a login uses it for every request, so each user with a
/// session has a client of their own (their liked music, private playlists,
/// and streams that need a login), and one client that never holds a login
/// serves what everyone shares: searches, albums, artists, lyrics.
pub struct YtMusicPool {
    public: Arc<YtMusic>,
    /// Users' clients keep their caches in `<users>/<id>/youtube-music`.
    users_dir: PathBuf,
    secrets: SecretBox,
    botguard: Option<PathBuf>,
    users: Mutex<HashMap<u64, Arc<YtMusic>>>,
}

impl YtMusicPool {
    /// The public client keeps its cache in `public_dir`.
    ///
    /// # Errors
    ///
    /// Fails when the public client cannot be built.
    pub fn new(
        public_dir: &Path,
        users_dir: PathBuf,
        secrets: SecretBox,
        botguard: Option<PathBuf>,
    ) -> Result<Self, HuntError> {
        Ok(Self {
            public: Arc::new(YtMusic::new(public_dir, secrets.clone(), botguard.clone())?),
            users_dir,
            secrets,
            botguard,
            users: Mutex::default(),
        })
    }

    /// Drops any login the public client's cache still holds (from before
    /// each user had a client).
    pub async fn logout_public(&self) {
        self.public.forget_cookies().await;
    }

    /// The client without a login.
    #[must_use]
    pub fn public(&self) -> Arc<YtMusic> {
        Arc::clone(&self.public)
    }

    /// `owner`'s client, made on first use.
    ///
    /// # Errors
    ///
    /// Fails when the client cannot be built.
    pub fn for_user(&self, owner: u64) -> Result<Arc<YtMusic>, HuntError> {
        let mut users = self.users.lock().unwrap();
        if let Some(client) = users.get(&owner) {
            return Ok(Arc::clone(client));
        }
        let dir = self.users_dir.join(owner.to_string()).join("youtube-music");
        let client = Arc::new(YtMusic::new(
            &dir,
            self.secrets.clone(),
            self.botguard.clone(),
        )?);
        users.insert(owner, Arc::clone(&client));
        Ok(client)
    }

    /// `owner`'s client if they have one (a session was connected), else
    /// the public one.
    #[must_use]
    pub fn client(&self, owner: u64) -> Arc<YtMusic> {
        self.users
            .lock()
            .unwrap()
            .get(&owner)
            .map_or_else(|| self.public(), Arc::clone)
    }

    /// Forgets `owner`'s client.
    pub fn remove(&self, owner: u64) {
        self.users.lock().unwrap().remove(&owner);
    }
}

/// YouTube Music as a [`Source`]: what everyone shares goes through the
/// client without a login; playlists and streams through the user's own
/// client when they have one.
pub struct YouTubeMusicSource {
    pool: Arc<YtMusicPool>,
}

impl YouTubeMusicSource {
    #[must_use]
    pub fn new(pool: Arc<YtMusicPool>) -> Self {
        Self { pool }
    }

    /// Every user's client.
    #[must_use]
    pub fn pool(&self) -> &Arc<YtMusicPool> {
        &self.pool
    }
}

impl Source for YouTubeMusicSource {
    fn platform(&self) -> Platform {
        Platform::YouTubeMusic
    }

    fn parse_link(&self, input: &str) -> Option<Link> {
        crate::link::parse(input)
    }

    fn search<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<SearchResults, HuntError>> {
        Box::pin(async move { self.pool.public().search(query).await })
    }

    fn track<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteTrack, HuntError>> {
        Box::pin(async move { self.pool.public().track(id).await })
    }

    fn album<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteAlbum, HuntError>> {
        Box::pin(async move { self.pool.public().album(id).await })
    }

    fn playlist<'a>(
        &'a self,
        owner: u64,
        id: &'a str,
    ) -> BoxFuture<'a, Result<RemotePlaylist, HuntError>> {
        Box::pin(async move { self.pool.client(owner).playlist(id).await })
    }

    fn discography<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Discography, HuntError>> {
        Box::pin(async move { self.pool.public().discography(id).await })
    }

    fn lyrics<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<(String, String)>, HuntError>> {
        Box::pin(async move { self.pool.public().lyrics(id).await })
    }

    fn audio<'a>(
        &'a self,
        owner: u64,
        id: &'a str,
    ) -> BoxFuture<'a, Result<AudioSource, HuntError>> {
        Box::pin(async move { self.pool.client(owner).audio(id).await })
    }

    fn page_url(&self, page: Page, id: &str) -> String {
        match page {
            Page::Song => format!("https://music.youtube.com/watch?v={id}"),
            Page::Album => format!("https://music.youtube.com/browse/{id}"),
            Page::Artist => format!("https://music.youtube.com/channel/{id}"),
            Page::Playlist => format!("https://music.youtube.com/playlist?list={id}"),
        }
    }

    fn yt_dlp(&self, id: &str) -> Option<YtDlpTarget> {
        Some(YtDlpTarget {
            url: self.page_url(Page::Song, id),
            cookie_domain: Some(".youtube.com"),
        })
    }

    fn liked_music(&self) -> Option<&'static str> {
        Some(LIKED_MUSIC)
    }

    fn login(&self) -> Option<LoginSpec> {
        Some(LoginSpec {
            // Google's sign-in, returning to YouTube Music.
            start_url: "https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fmusic.youtube.com%2F",
            home_url: "https://music.youtube.com/",
            cookie_domain: "youtube.com",
            login_cookies: &["SAPISID", "__Secure-3PAPISID"],
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
        artist_id: item.artist_id.map(SourceKey::youtube_music),
        album: item.album.map(|album| AlbumRef {
            id: SourceKey::youtube_music(album.id),
            title: album.name,
        }),
        duration_secs: item.duration,
        track_number: item.track_nr,
        cover_url: image(&item.cover),
        is_video: item.track_type == TrackType::Video,
        title: item.name,
        id: SourceKey::youtube_music(item.id),
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
        artist_id: item.artist_id.map(SourceKey::youtube_music),
        year: item.year,
        kind: album_kind(item.album_type),
        cover_url: image(&item.cover),
        tracks: Vec::new(),
        title: item.name,
        id: SourceKey::youtube_music(item.id),
    }
}

fn album(album: MusicAlbum) -> RemoteAlbum {
    let cover_url = image(&album.cover);
    let artists = artist_names(&album.artists);
    let artist_id = album.artist_id.clone().map(SourceKey::youtube_music);
    let id = SourceKey::youtube_music(album.id);
    let reference = AlbumRef {
        id: id.clone(),
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
                track.artist_id.clone_from(&artist_id);
            }
            track
        })
        .collect();
    RemoteAlbum {
        id,
        title: album.name,
        artists,
        artist_id,
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
        id: SourceKey::youtube_music(item.id),
    }
}
