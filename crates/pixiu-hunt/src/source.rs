//! The platforms music is downloaded from, behind one trait. A key names
//! its platform, and the platform's source answers for it: search, songs,
//! albums, playlists, artists, lyrics and audio. Everything after the audio
//! arrives (tags, the store, claims) is the same for every platform.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

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

/// How a platform's login works: users sign in through the login browser,
/// and the session warden keeps the session's cookies fresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginSpec {
    /// Where signing in starts.
    pub start_url: &'static str,
    /// The platform's home page, which the warden visits to refresh the
    /// cookies.
    pub home_url: &'static str,
    /// The domain the session's cookies are for.
    pub cookie_domain: &'static str,
    /// Cookies that are only there while someone is signed in.
    pub login_cookies: &'static [&'static str],
}

impl LoginSpec {
    /// Whether cookies of these names carry a login.
    pub fn is_logged_in<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> bool {
        names
            .into_iter()
            .any(|name| self.login_cookies.contains(&name))
    }
}

/// A platform píxiū downloads from. Ids are the platform's own, the part of
/// a key after its platform.
pub trait Source: Send + Sync {
    fn platform(&self) -> Platform;

    /// What a link, or a bare id, points at on this platform.
    fn parse_link(&self, input: &str) -> Option<Link>;

    /// Hosts of the platform's short links, as its share buttons give:
    /// they redirect to links [`Source::parse_link`] reads.
    fn short_link_hosts(&self) -> &'static [&'static str] {
        &[]
    }

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

    /// How users sign in, when the platform has logins.
    fn login(&self) -> Option<LoginSpec> {
        None
    }
}

/// The platforms píxiū downloads from, each with its source.
#[derive(Clone)]
pub struct Platforms {
    sources: BTreeMap<Platform, Arc<dyn Source>>,
    /// Follows short links.
    http: reqwest::Client,
}

impl Default for Platforms {
    fn default() -> Self {
        Self::new([])
    }
}

impl Platforms {
    /// # Panics
    ///
    /// Panics when the HTTP client cannot be built (no TLS roots).
    #[must_use]
    pub fn new(sources: impl IntoIterator<Item = Arc<dyn Source>>) -> Self {
        Self {
            sources: sources
                .into_iter()
                .map(|source| (source.platform(), source))
                .collect(),
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::limited(5))
                .timeout(Duration::from_secs(15))
                .build()
                .expect("the HTTP client builds"),
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

    /// What a link points at, following it first when it is a platform's
    /// short link.
    pub async fn resolve_link(&self, input: &str) -> Option<Link> {
        if let Some(link) = self.parse_link(input) {
            return Some(link);
        }
        let url = reqwest::Url::parse(input.trim()).ok()?;
        let host = url.host_str()?;
        let short = matches!(url.scheme(), "http" | "https")
            && self
                .all()
                .any(|source| source.short_link_hosts().contains(&host));
        if !short {
            return None;
        }
        let response = self.http.get(url).send().await.ok()?;
        self.parse_link(response.url().as_str())
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

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;
    use crate::{
        AudioSource, Discography, RemoteAlbum, RemotePlaylist, RemoteTrack, SearchResults,
    };

    /// A platform whose links are `http://127.0.0.1:<port>/artist/<id>`, and
    /// whose short links are any others on that host.
    struct Local;

    impl Source for Local {
        fn platform(&self) -> Platform {
            Platform::Deezer
        }

        fn parse_link(&self, input: &str) -> Option<Link> {
            let url = reqwest::Url::parse(input).ok()?;
            let id = url.path().strip_prefix("/artist/")?;
            (url.host_str() == Some("127.0.0.1")).then(|| Link::Artist(SourceKey::deezer(id)))
        }

        fn short_link_hosts(&self) -> &'static [&'static str] {
            &["127.0.0.1"]
        }

        fn search<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<SearchResults, HuntError>> {
            unimplemented!()
        }

        fn track<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<RemoteTrack, HuntError>> {
            unimplemented!()
        }

        fn album<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<RemoteAlbum, HuntError>> {
            unimplemented!()
        }

        fn playlist<'a>(
            &'a self,
            _: u64,
            _: &'a str,
        ) -> BoxFuture<'a, Result<RemotePlaylist, HuntError>> {
            unimplemented!()
        }

        fn discography<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Discography, HuntError>> {
            unimplemented!()
        }

        fn lyrics<'a>(
            &'a self,
            _: &'a str,
        ) -> BoxFuture<'a, Result<Option<(String, String)>, HuntError>> {
            unimplemented!()
        }

        fn audio<'a>(
            &'a self,
            _: u64,
            _: &'a str,
        ) -> BoxFuture<'a, Result<AudioSource, HuntError>> {
            unimplemented!()
        }

        fn page_url(&self, _: Page, _: &str) -> String {
            unimplemented!()
        }

        fn yt_dlp(&self, _: &str) -> Option<YtDlpTarget> {
            None
        }
    }

    #[tokio::test]
    async fn short_links_are_followed() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let target = format!("{base}/artist/42");
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = [0_u8; 2048];
                let read = stream.read(&mut request).await.unwrap_or(0);
                let short = request[..read].starts_with(b"GET /s/abc ");
                let head = if short {
                    format!("HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\n\r\n")
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_owned()
                };
                let _ = stream.write_all(head.as_bytes()).await;
            }
        });
        let platforms = Platforms::new([Arc::new(Local) as Arc<dyn Source>]);

        assert_eq!(
            platforms.resolve_link(&format!("{base}/s/abc")).await,
            Some(Link::Artist(SourceKey::deezer("42")))
        );
        assert_eq!(
            platforms.resolve_link(&format!("{base}/artist/7")).await,
            Some(Link::Artist(SourceKey::deezer("7"))),
            "links that need no following"
        );
        assert_eq!(
            platforms.resolve_link(&format!("{base}/s/gone")).await,
            None
        );
        assert_eq!(
            platforms.resolve_link("https://example.com/s/abc").await,
            None
        );
    }

    #[test]
    fn logins_are_told_by_their_cookies() {
        let login = LoginSpec {
            start_url: "https://example.com/login",
            home_url: "https://example.com/",
            cookie_domain: "example.com",
            login_cookies: &["SID"],
        };
        assert!(login.is_logged_in(["PREF", "SID"]));
        assert!(!login.is_logged_in(["PREF"]));
    }

    #[test]
    fn platforms_without_a_source_say_so() {
        let platforms = Platforms::default();
        assert!(matches!(
            platforms.get(Platform::YouTubeMusic),
            Err(HuntError::NoSource(Platform::YouTubeMusic))
        ));
        assert_eq!(platforms.parse_link("https://example.com"), None);
        assert_eq!(platforms.liked_music(), None);
        assert_eq!(
            platforms.page_url(Page::Song, &SourceKey::youtube_music("x")),
            None
        );
    }
}
