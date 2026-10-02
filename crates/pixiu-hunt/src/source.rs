//! The platforms music is downloaded from, behind one trait. A key names
//! its platform, and the platform's source answers for it: search, songs,
//! albums, playlists, artists, lyrics and audio. Everything after the audio
//! arrives (tags, the store, claims) is the same for every platform.

use std::{collections::BTreeMap, sync::Arc};

pub use futures_util::future::BoxFuture;
use pixiu_db::{Platform, SourceKey};

use crate::{
    AudioSource, Discography, HuntError, RemoteAlbum, RemotePlaylist, RemoteTrack, SearchResults,
};

/// What a pasted link (or a bare id) points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Playlist(SourceKey),
    /// The account's liked songs.
    LikedMusic(SourceKey),
    Artist(SourceKey),
    Album(SourceKey),
    Track(SourceKey),
}

/// A page on a platform, for links to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Song,
    Album,
    Artist,
    Playlist,
}

/// Where yt-dlp fetches a song when the platform's own stream fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YtDlpTarget {
    pub url: String,
    /// The domain the user's session cookies are for; without one, yt-dlp
    /// goes without them.
    pub cookie_domain: Option<&'static str>,
}

/// A platform píxiū downloads from. Ids are the platform's own, the part of
/// a key after its platform.
pub trait Source: Send + Sync {
    fn platform(&self) -> Platform;

    /// What a link, or a bare id, points at on this platform.
    fn parse_link(&self, input: &str) -> Option<Link>;

    /// Songs, albums and artists matching `query`.
    fn search<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<SearchResults, HuntError>>;

    /// A song's details.
    fn track<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteTrack, HuntError>>;

    /// An album with its songs.
    fn album<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteAlbum, HuntError>>;

    /// A playlist with all its songs, as `owner` sees it: their login lets
    /// private ones through, and their liked songs (see
    /// [`Source::liked_music`]).
    fn playlist<'a>(
        &'a self,
        owner: u64,
        id: &'a str,
    ) -> BoxFuture<'a, Result<RemotePlaylist, HuntError>>;

    /// An artist's releases.
    fn discography<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Discography, HuntError>>;

    /// A song's lyrics as the platform shows them, with their credit line.
    fn lyrics<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<(String, String)>, HuntError>>;

    /// The best audio stream of a song, with `owner`'s login when the
    /// stream needs one.
    fn audio<'a>(
        &'a self,
        owner: u64,
        id: &'a str,
    ) -> BoxFuture<'a, Result<AudioSource, HuntError>>;

    /// The address of a page on the platform.
    fn page_url(&self, page: Page, id: &str) -> String;

    /// How yt-dlp fetches a song, if it can.
    fn yt_dlp(&self, id: &str) -> Option<YtDlpTarget>;

    /// The playlist id of an account's liked songs, when the platform keeps
    /// them as one.
    fn liked_music(&self) -> Option<&'static str> {
        None
    }
}

/// The platforms píxiū downloads from, each with its source.
#[derive(Clone, Default)]
pub struct Platforms {
    sources: BTreeMap<Platform, Arc<dyn Source>>,
}

impl Platforms {
    #[must_use]
    pub fn new(sources: impl IntoIterator<Item = Arc<dyn Source>>) -> Self {
        Self {
            sources: sources
                .into_iter()
                .map(|source| (source.platform(), source))
                .collect(),
        }
    }

    /// The source of `platform`.
    ///
    /// # Errors
    ///
    /// Fails when píxiū has no source for it.
    pub fn get(&self, platform: Platform) -> Result<&dyn Source, HuntError> {
        self.sources
            .get(&platform)
            .map(AsRef::as_ref)
            .ok_or(HuntError::NoSource(platform))
    }

    /// The source of the platform `key` is on.
    ///
    /// # Errors
    ///
    /// Fails when píxiū has no source for it.
    pub fn of(&self, key: &SourceKey) -> Result<&dyn Source, HuntError> {
        self.get(key.platform())
    }

    /// Every source, in a stable order.
    pub fn all(&self) -> impl Iterator<Item = &dyn Source> {
        self.sources.values().map(AsRef::as_ref)
    }

    /// What a link points at, on whichever platform knows it.
    #[must_use]
    pub fn parse_link(&self, input: &str) -> Option<Link> {
        self.all().find_map(|source| source.parse_link(input))
    }

    /// The account's liked songs, on the first platform that keeps them as
    /// a playlist.
    #[must_use]
    pub fn liked_music(&self) -> Option<SourceKey> {
        self.all().find_map(|source| {
            source
                .liked_music()
                .map(|id| SourceKey::new(source.platform(), id))
        })
    }

    /// The address of `key`'s page on its platform.
    #[must_use]
    pub fn page_url(&self, page: Page, key: &SourceKey) -> Option<String> {
        Some(self.of(key).ok()?.page_url(page, key.id()))
    }
}
