//! Deezer, through its public API, with the audio from Monochrome: Deezer
//! knows the catalogue, and Monochrome serves its songs, found by ISRC.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use futures_util::future::BoxFuture;
use pixiu_db::{Platform, SourceKey};
use reqwest::{Url, header::CONTENT_LENGTH};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{
    HuntError,
    model::{
        AlbumKind, AlbumRef, Discography, RemoteAlbum, RemoteArtist, RemotePlaylist, RemoteTrack,
        SearchResults,
    },
    source::{Link, Page, Source, YtDlpTarget},
    ytmusic::AudioSource,
};

/// Deezer's public API.
pub const DEEZER_API: &str = "https://api.deezer.com";

/// Monochrome's API, which serves the audio.
pub const MONOCHROME_API: &str = "https://tracks.monochrome.st";

const USER_AGENT: &str = concat!("pixiu/", env!("CARGO_PKG_VERSION"));

/// Results per search, as YouTube Music gives.
const SEARCH_LIMIT: u32 = 20;
/// Items per page of a list, the most Deezer gives.
const PAGE_LIMIT: u32 = 100;
/// Pages of a list followed at most: 5000 songs.
const MAX_PAGES: usize = 50;

/// Deezer's error codes: too many requests (50 per 5 seconds), and no such
/// thing.
const QUOTA: i64 = 4;
const NO_DATA: i64 = 800;
/// How often a request refused for the quota is tried again.
const QUOTA_RETRIES: u32 = 3;

/// How long fetched songs are reused: a download asks for its song, then
/// for its audio, which needs the song again.
const TRACK_CACHE_TTL: Duration = Duration::from_secs(600);

/// Hosts of Deezer's short links, as its share buttons give.
const SHORT_LINK_HOSTS: &[&str] = &["link.deezer.com", "deezer.page.link"];

/// Deezer as a [`Source`]. It has no logins: everything is public.
pub struct DeezerSource {
    http: reqwest::Client,
    api: String,
    monochrome: String,
    tracks: Mutex<HashMap<String, (Instant, RemoteTrack)>>,
}

impl DeezerSource {
    /// Talks to Deezer's API and to Monochrome's at `monochrome`.
    ///
    /// # Errors
    ///
    /// Fails when the HTTP client cannot be built.
    pub fn new(monochrome: &str) -> Result<Self, HuntError> {
        Self::with_api(DEEZER_API, monochrome)
    }

    /// Talks to Deezer's API at `api`, for tests.
    ///
    /// # Errors
    ///
    /// Fails when the HTTP client cannot be built.
    pub fn with_api(api: &str, monochrome: &str) -> Result<Self, HuntError> {
        Ok(Self {
            http: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(30))
                .build()?,
            api: api.trim_end_matches('/').to_owned(),
            monochrome: monochrome.trim_end_matches('/').to_owned(),
            tracks: Mutex::default(),
        })
    }

    /// `path` on Deezer's API with `query`, as `T`. `what` names what was
    /// asked for, for people ("song", "album").
    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
        what: &str,
    ) -> Result<T, HuntError> {
        let mut url = Url::parse(&format!("{}{path}", self.api))
            .map_err(|error| failed(format!("not a Deezer address: {error}")))?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        self.fetch(url.as_str(), what).await
    }

    async fn fetch<T: DeserializeOwned>(&self, url: &str, what: &str) -> Result<T, HuntError> {
        let mut attempt = 0;
        loop {
            let body = async {
                self.http
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await
            }
            .await
            .map_err(|error| failed(format!("cannot reach Deezer: {}", error.without_url())))?;
            let reply: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|error| failed(format!("Deezer answered unexpectedly: {error}")))?;
            if let Some(error) = reply.get("error") {
                let error: ApiError = serde_json::from_value(error.clone()).unwrap_or_default();
                match refusal(&error, what) {
                    None if attempt < QUOTA_RETRIES => {
                        attempt += 1;
                        tracing::info!(attempt, "Deezer's quota was reached; trying again");
                        tokio::time::sleep(Duration::from_secs(2 * u64::from(attempt))).await;
                        continue;
                    }
                    None => return Err(failed("Deezer is refusing requests for now".to_owned())),
                    Some(error) => return Err(error),
                }
            }
            return serde_json::from_value(reply)
                .map_err(|error| failed(format!("Deezer answered unexpectedly: {error}")));
        }
    }

    /// Every item of a list, following its pages.
    async fn all<T: DeserializeOwned>(&self, path: &str, what: &str) -> Result<Vec<T>, HuntError> {
        let limit = PAGE_LIMIT.to_string();
        let mut page: ApiPage<T> = self.get(path, &[("limit", &limit)], what).await?;
        let mut items = Vec::new();
        for _ in 1..MAX_PAGES {
            items.append(&mut page.data);
            // Only pages of the same API: the link is Deezer's to give.
            match page.next.take() {
                Some(next) if next.starts_with(&format!("{}/", self.api)) => {
                    page = self.fetch(&next, what).await?;
                }
                _ => return Ok(items),
            }
        }
        items.append(&mut page.data);
        Ok(items)
    }

    async fn fetch_track(&self, id: &str) -> Result<RemoteTrack, HuntError> {
        if let Some((fetched, track)) = self.tracks.lock().unwrap().get(id)
            && fetched.elapsed() < TRACK_CACHE_TTL
        {
            return Ok(track.clone());
        }
        let track = track(
            self.get::<ApiTrack>(&format!("/track/{id}"), &[], "song")
                .await?,
            None,
        );
        let mut cache = self.tracks.lock().unwrap();
        cache.retain(|_, (fetched, _)| fetched.elapsed() < TRACK_CACHE_TTL);
        cache.insert(id.to_owned(), (Instant::now(), track.clone()));
        Ok(track)
    }

    async fn search_all(&self, query: &str) -> Result<SearchResults, HuntError> {
        let limit = SEARCH_LIMIT.to_string();
        let params = [("q", query), ("limit", limit.as_str())];
        let (tracks, albums, artists) = tokio::try_join!(
            self.get::<ApiPage<ApiTrack>>("/search/track", &params, "song"),
            self.get::<ApiPage<ApiAlbum>>("/search/album", &params, "album"),
            self.get::<ApiPage<ApiArtist>>("/search/artist", &params, "artist"),
        )?;
        Ok(SearchResults {
            tracks: tracks
                .data
                .into_iter()
                .map(|item| track(item, None))
                .collect(),
            albums: albums
                .data
                .into_iter()
                .map(|item| album(item, None, Vec::new()))
                .collect(),
            artists: artists.data.into_iter().map(artist).collect(),
        })
    }

    async fn fetch_album(&self, id: &str) -> Result<RemoteAlbum, HuntError> {
        // The album's own list stops at 25 songs, without their ISRCs or
        // positions; the tracks endpoint has them all.
        let (path, tracks_path) = (format!("/album/{id}"), format!("/album/{id}/tracks"));
        let (info, items) = tokio::try_join!(
            self.get::<ApiAlbum>(&path, &[], "album"),
            self.all::<ApiTrack>(&tracks_path, "album"),
        )?;
        let reference = AlbumRef {
            id: SourceKey::deezer(info.id.to_string()),
            title: info.title.clone(),
        };
        let cover = info.cover_xl.clone();
        let tracks = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let mut track = track(item, Some(&reference));
                track.cover_url.clone_from(&cover);
                if track.track_number.is_none() {
                    track.track_number = u16::try_from(index + 1).ok();
                }
                track
            })
            .collect();
        Ok(album(info, None, tracks))
    }

    async fn fetch_playlist(&self, id: &str) -> Result<RemotePlaylist, HuntError> {
        let (path, tracks_path) = (format!("/playlist/{id}"), format!("/playlist/{id}/tracks"));
        let (info, items) = tokio::try_join!(
            self.get::<ApiPlaylist>(&path, &[], "playlist"),
            self.all::<ApiTrack>(&tracks_path, "playlist"),
        )?;
        Ok(RemotePlaylist {
            id: SourceKey::deezer(info.id.to_string()),
            name: info.title,
            image_url: info.picture_xl,
            tracks: items.into_iter().map(|item| track(item, None)).collect(),
        })
    }

    async fn fetch_discography(&self, id: &str) -> Result<Discography, HuntError> {
        let (path, albums_path) = (format!("/artist/{id}"), format!("/artist/{id}/albums"));
        let (info, items) = tokio::try_join!(
            self.get::<ApiArtist>(&path, &[], "artist"),
            self.all::<ApiAlbum>(&albums_path, "artist"),
        )?;
        let mut albums: Vec<(Option<String>, RemoteAlbum)> = items
            .into_iter()
            .map(|item| {
                (
                    item.release_date.clone(),
                    album(item, Some(&info), Vec::new()),
                )
            })
            .collect();
        // Newest first.
        albums.sort_by(|(a, _), (b, _)| b.cmp(a));
        Ok(Discography {
            id: SourceKey::deezer(info.id.to_string()),
            name: info.name,
            image_url: info.picture_xl,
            albums: albums.into_iter().map(|(_, album)| album).collect(),
        })
    }

    /// The song's stream on Monochrome.
    async fn find_audio(&self, id: &str) -> Result<AudioSource, HuntError> {
        let song = self.fetch_track(id).await?;
        let artist = song.artists.first().map_or("", String::as_str);
        let mut found = None;
        for query in [format!("{artist} {}", song.title), song.title.clone()] {
            let mut url = Url::parse(&format!("{}/search/tracks", self.monochrome))
                .map_err(|error| monochrome_failed(&error.to_string()))?;
            url.query_pairs_mut()
                .append_pair("q", query.trim())
                .append_pair("limit", "10");
            let body = async {
                self.http
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await
            }
            .await
            .map_err(|error| monochrome_failed(&error.without_url().to_string()))?;
            let results: MonochromeResults = serde_json::from_slice(&body)
                .map_err(|error| monochrome_failed(&error.to_string()))?;
            if let Some(hit) = best_match(&song, &results.tracks) {
                found = Some(hit.track_id.clone());
                break;
            }
        }
        let Some(track_id) = found else {
            return Err(HuntError::Download(
                "Monochrome doesn't have this song".to_owned(),
            ));
        };

        let url = format!("{}/track/{track_id}", self.monochrome);
        // Its size lets the download go in ranges, each retried on its own.
        let size = match self.http.head(&url).send().await {
            Ok(response) if response.status().is_success() => response
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok()?.parse().ok()),
            Ok(_) | Err(_) => None,
        };
        Ok(AudioSource {
            url,
            size,
            user_agent: USER_AGENT.to_owned(),
            is_webm_opus: false,
            extension: "flac",
        })
    }
}

fn failed(message: String) -> HuntError {
    HuntError::Source {
        platform: Platform::Deezer,
        message,
        needs_login: false,
    }
}

fn monochrome_failed(reason: &str) -> HuntError {
    HuntError::Download(format!("Monochrome couldn't be asked: {reason}"))
}

/// What a Deezer error means: `None` when the request may pass if tried
/// again (the quota was reached).
fn refusal(error: &ApiError, what: &str) -> Option<HuntError> {
    match error.code {
        QUOTA => None,
        NO_DATA => Some(failed(format!(
            "Deezer has no such {what} (or not in this server's country)"
        ))),
        // Private playlists need the owner's login, which píxiū doesn't
        // have on Deezer.
        _ if error.kind == "OAuthException" => {
            Some(failed(format!("Deezer keeps that {what} private")))
        }
        _ => Some(failed(format!("Deezer refused: {}", error.message))),
    }
}

/// A title or a name in lower case, without spaces or punctuation.
fn plain(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The stream of `song` among Monochrome's results: the one with its ISRC,
/// else one with the same title and artist and a length within 3 seconds.
fn best_match<'a>(
    song: &RemoteTrack,
    results: &'a [MonochromeTrack],
) -> Option<&'a MonochromeTrack> {
    let playable = results
        .iter()
        .filter(|result| result.playable != Some(false));
    if let Some(isrc) = &song.isrc
        && let Some(hit) = playable.clone().find(|result| {
            result
                .isrc
                .as_deref()
                .is_some_and(|other| other.eq_ignore_ascii_case(isrc))
        })
    {
        return Some(hit);
    }
    let title = plain(&song.title);
    let artists: Vec<String> = song.artists.iter().map(|artist| plain(artist)).collect();
    playable.into_iter().find(|result| {
        let same_artist = result
            .artist_names
            .iter()
            .any(|name| artists.contains(&plain(name)));
        let same_length = match (song.duration_secs, result.duration) {
            (Some(ours), Some(millis)) => {
                (i64::from(ours) - i64::try_from(millis / 1000).unwrap_or(i64::MAX)).abs() <= 3
            }
            _ => false,
        };
        !title.is_empty() && plain(&result.title) == title && same_artist && same_length
    })
}

impl Source for DeezerSource {
    fn platform(&self) -> Platform {
        Platform::Deezer
    }

    fn parse_link(&self, input: &str) -> Option<Link> {
        parse_link(input)
    }

    fn short_link_hosts(&self) -> &'static [&'static str] {
        SHORT_LINK_HOSTS
    }

    fn search<'a>(&'a self, query: &'a str) -> BoxFuture<'a, Result<SearchResults, HuntError>> {
        Box::pin(self.search_all(query))
    }

    fn track<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteTrack, HuntError>> {
        Box::pin(self.fetch_track(id))
    }

    fn album<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<RemoteAlbum, HuntError>> {
        Box::pin(self.fetch_album(id))
    }

    fn playlist<'a>(
        &'a self,
        _owner: u64,
        id: &'a str,
    ) -> BoxFuture<'a, Result<RemotePlaylist, HuntError>> {
        Box::pin(self.fetch_playlist(id))
    }

    fn discography<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Discography, HuntError>> {
        Box::pin(self.fetch_discography(id))
    }

    /// Deezer's lyrics need a login; LRCLIB is asked before any platform.
    fn lyrics<'a>(
        &'a self,
        _id: &'a str,
    ) -> BoxFuture<'a, Result<Option<(String, String)>, HuntError>> {
        Box::pin(async { Ok(None) })
    }

    fn audio<'a>(
        &'a self,
        _owner: u64,
        id: &'a str,
    ) -> BoxFuture<'a, Result<AudioSource, HuntError>> {
        Box::pin(self.find_audio(id))
    }

    fn page_url(&self, page: Page, id: &str) -> String {
        let kind = match page {
            Page::Song => "track",
            Page::Album => "album",
            Page::Artist => "artist",
            Page::Playlist => "playlist",
        };
        format!("https://www.deezer.com/{kind}/{id}")
    }

    fn yt_dlp(&self, _id: &str) -> Option<YtDlpTarget> {
        None
    }
}

/// Reads a Deezer link: `deezer.com/[language/]track|album|artist|playlist/<id>`.
fn parse_link(input: &str) -> Option<Link> {
    let url = Url::parse(input.trim()).ok()?;
    let host = url.host_str()?.trim_start_matches("www.");
    if host != "deezer.com" || !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let (kind, id) = match segments.as_slice() {
        [kind, id] => (*kind, *id),
        // `/en/…`, `/pt-br/…`
        [language, kind, id]
            if language.len() <= 5
                && language
                    .chars()
                    .all(|c| c.is_ascii_alphabetic() || c == '-') =>
        {
            (*kind, *id)
        }
        _ => return None,
    };
    if id.is_empty() || id.len() > 20 || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let key = SourceKey::deezer(id);
    Some(match kind {
        "track" => Link::Track(key),
        "album" => Link::Album(key),
        "artist" => Link::Artist(key),
        "playlist" => Link::Playlist(key),
        _ => return None,
    })
}

#[derive(Debug, Default, Deserialize)]
struct ApiError {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    code: i64,
}

#[derive(Deserialize)]
struct ApiPage<T> {
    data: Vec<T>,
    #[serde(default)]
    next: Option<String>,
}

#[derive(Deserialize)]
struct ApiArtist {
    id: u64,
    name: String,
    #[serde(default)]
    picture_xl: Option<String>,
    /// "Main" or "Featured", in a song's or album's contributors.
    #[serde(default)]
    role: Option<String>,
}

#[derive(Deserialize)]
struct ApiAlbumRef {
    id: u64,
    title: String,
    #[serde(default)]
    cover_xl: Option<String>,
}

#[derive(Deserialize)]
struct ApiTrack {
    id: u64,
    title: String,
    #[serde(default)]
    duration: Option<u32>,
    #[serde(default)]
    isrc: Option<String>,
    #[serde(default)]
    track_position: Option<u16>,
    #[serde(default)]
    disk_number: Option<u16>,
    artist: ApiArtist,
    #[serde(default)]
    contributors: Vec<ApiArtist>,
    #[serde(default)]
    album: Option<ApiAlbumRef>,
}

#[derive(Deserialize)]
struct ApiAlbum {
    id: u64,
    title: String,
    #[serde(default)]
    cover_xl: Option<String>,
    #[serde(default)]
    release_date: Option<String>,
    #[serde(default)]
    record_type: Option<String>,
    #[serde(default)]
    artist: Option<ApiArtist>,
    #[serde(default)]
    contributors: Vec<ApiArtist>,
}

#[derive(Deserialize)]
struct ApiPlaylist {
    id: u64,
    title: String,
    #[serde(default)]
    picture_xl: Option<String>,
}

#[derive(Deserialize)]
struct MonochromeResults {
    #[serde(default)]
    tracks: Vec<MonochromeTrack>,
}

#[derive(Deserialize)]
struct MonochromeTrack {
    #[serde(rename = "trackId")]
    track_id: String,
    title: String,
    #[serde(default, rename = "artistNames")]
    artist_names: Vec<String>,
    /// Milliseconds.
    #[serde(default)]
    duration: Option<u64>,
    #[serde(default)]
    isrc: Option<String>,
    #[serde(default)]
    playable: Option<bool>,
}

/// The names of `artists`, each once, in order.
fn names<'a>(artists: impl IntoIterator<Item = &'a ApiArtist>) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for artist in artists {
        if !names.contains(&artist.name) {
            names.push(artist.name.clone());
        }
    }
    names
}

/// A song's artists: its main artist, the other main ones, then guests.
fn credited(main: &ApiArtist, contributors: &[ApiArtist]) -> Vec<String> {
    let mut others: Vec<&ApiArtist> = contributors.iter().collect();
    others.sort_by_key(|artist| artist.role.as_deref() != Some("Main"));
    names(std::iter::once(main).chain(others))
}

/// A song; `album` stands in for the album songs listed by their album
/// leave out.
fn track(item: ApiTrack, album: Option<&AlbumRef>) -> RemoteTrack {
    RemoteTrack {
        id: SourceKey::deezer(item.id.to_string()),
        title: item.title,
        artists: credited(&item.artist, &item.contributors),
        artist_id: Some(SourceKey::deezer(item.artist.id.to_string())),
        cover_url: item.album.as_ref().and_then(|album| album.cover_xl.clone()),
        album: item
            .album
            .map(|album| AlbumRef {
                id: SourceKey::deezer(album.id.to_string()),
                title: album.title,
            })
            .or_else(|| album.cloned()),
        duration_secs: item.duration,
        track_number: item.track_position,
        disc_number: item.disk_number,
        isrc: item.isrc.filter(|isrc| !isrc.is_empty()),
        is_video: false,
    }
}

fn album_kind(record_type: Option<&str>) -> AlbumKind {
    match record_type {
        Some("album") => AlbumKind::Album,
        Some("ep") => AlbumKind::Ep,
        Some("single") => AlbumKind::Single,
        _ => AlbumKind::Other,
    }
}

/// An album; `artist` stands in for the artist albums listed by their
/// artist leave out.
fn album(item: ApiAlbum, artist: Option<&ApiArtist>, tracks: Vec<RemoteTrack>) -> RemoteAlbum {
    let main = item.artist.as_ref().or(artist);
    // Guests are credited on their songs, not on the album.
    let artists = names(
        main.into_iter().chain(
            item.contributors
                .iter()
                .filter(|artist| artist.role.as_deref() != Some("Featured")),
        ),
    );
    RemoteAlbum {
        id: SourceKey::deezer(item.id.to_string()),
        title: item.title,
        artists,
        artist_id: main.map(|artist| SourceKey::deezer(artist.id.to_string())),
        year: item
            .release_date
            .as_deref()
            .and_then(|date| date.get(..4)?.parse().ok())
            .filter(|year| *year > 0),
        kind: album_kind(item.record_type.as_deref()),
        cover_url: item.cover_xl,
        tracks,
    }
}

fn artist(item: ApiArtist) -> RemoteArtist {
    RemoteArtist {
        id: SourceKey::deezer(item.id.to_string()),
        name: item.name,
        image_url: item.picture_xl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_understood() {
        let track = Link::Track(SourceKey::deezer("3135556"));
        for input in [
            "https://www.deezer.com/track/3135556",
            "https://www.deezer.com/en/track/3135556?host=1&utm_campaign=x",
            "https://deezer.com/pt-br/track/3135556/",
            "  https://www.deezer.com/track/3135556 ",
        ] {
            assert_eq!(parse_link(input), Some(track.clone()), "{input}");
        }
        assert_eq!(
            parse_link("https://www.deezer.com/fr/album/302127"),
            Some(Link::Album(SourceKey::deezer("302127")))
        );
        assert_eq!(
            parse_link("https://www.deezer.com/artist/27"),
            Some(Link::Artist(SourceKey::deezer("27")))
        );
        assert_eq!(
            parse_link("https://www.deezer.com/us/playlist/908622995"),
            Some(Link::Playlist(SourceKey::deezer("908622995")))
        );
        for input in [
            "",
            "3135556",
            "https://www.deezer.com/track/abc",
            "https://www.deezer.com/show/123",
            "https://www.deezer.com/en/profile/123/loved",
            "https://example.com/track/3135556",
            "https://link.deezer.com/s/32Og8UXbA28PAXtXrw2BM",
            "ftp://www.deezer.com/track/3135556",
        ] {
            assert_eq!(parse_link(input), None, "{input}");
        }
    }

    fn error(code: i64, kind: &str) -> ApiError {
        ApiError {
            kind: kind.to_owned(),
            message: "nope".to_owned(),
            code,
        }
    }

    #[test]
    fn errors_say_what_happened() {
        assert!(
            refusal(&error(QUOTA, "Exception"), "song").is_none(),
            "tried again"
        );
        let missing = refusal(&error(NO_DATA, "DataException"), "album").unwrap();
        assert!(missing.to_string().contains("no such album"), "{missing}");
        assert!(!missing.needs_login());
        let private = refusal(&error(200, "OAuthException"), "playlist").unwrap();
        assert!(private.to_string().contains("private"), "{private}");
        let other = refusal(&error(100, "Exception"), "song").unwrap();
        assert!(other.to_string().contains("nope"), "{other}");
    }

    fn song(isrc: Option<&str>) -> RemoteTrack {
        RemoteTrack {
            id: SourceKey::deezer("1"),
            title: "Dawn Chorus (Live)".to_owned(),
            artists: vec!["Main Artist".to_owned(), "Guest".to_owned()],
            artist_id: None,
            album: None,
            duration_secs: Some(200),
            track_number: None,
            disc_number: None,
            isrc: isrc.map(str::to_owned),
            cover_url: None,
            is_video: false,
        }
    }

    fn result(id: &str, title: &str, artist: &str, millis: u64, isrc: &str) -> MonochromeTrack {
        MonochromeTrack {
            track_id: id.to_owned(),
            title: title.to_owned(),
            artist_names: vec![artist.to_owned()],
            duration: Some(millis),
            isrc: Some(isrc.to_owned()),
            playable: Some(true),
        }
    }

    #[test]
    fn monochrome_streams_are_matched_by_isrc_first() {
        let results = [
            result("a", "Dawn Chorus (Live)", "Main Artist", 200_000, "ZZXX1"),
            result("b", "Something Else", "Somebody", 90_000, "ZZXX2"),
        ];
        let hit = best_match(&song(Some("zzxx2")), &results).unwrap();
        assert_eq!(hit.track_id, "b", "the ISRC settles it");
        let hit = best_match(&song(Some("ZZXX9")), &results).unwrap();
        assert_eq!(hit.track_id, "a", "same title, artist and length");
        assert_eq!(best_match(&song(None), &results).unwrap().track_id, "a");
    }

    #[test]
    fn near_misses_are_not_streams() {
        for near in [
            result("x", "Dawn Chorus", "Main Artist", 200_000, "Z"),
            result("x", "Dawn Chorus (Live)", "Another Artist", 200_000, "Z"),
            result("x", "Dawn Chorus (Live)", "Main Artist", 205_000, "Z"),
        ] {
            assert!(best_match(&song(None), &[near]).is_none());
        }
        let mut unplayable = result("x", "Dawn Chorus (Live)", "Main Artist", 200_000, "ZZXX1");
        unplayable.playable = Some(false);
        assert!(best_match(&song(Some("ZZXX1")), &[unplayable]).is_none());
        let guest = result("g", "dawn chorus live", "guest", 201_400, "Z");
        assert_eq!(
            best_match(&song(None), &[guest]).unwrap().track_id,
            "g",
            "case, punctuation and any credited artist"
        );
    }

    #[test]
    fn songs_read_from_deezer() {
        let item: ApiTrack = serde_json::from_str(
            r#"{"id": 42, "title": "Dawn Chorus", "duration": 211, "isrc": "ZZXX12100003",
                "track_position": 3, "disk_number": 2,
                "artist": {"id": 7, "name": "Main Artist"},
                "contributors": [
                    {"id": 8, "name": "Guest", "role": "Featured"},
                    {"id": 7, "name": "Main Artist", "role": "Main"},
                    {"id": 9, "name": "Partner", "role": "Main"}
                ],
                "album": {"id": 5, "title": "Morning",
                          "cover_xl": "https://cdn-images.dzcdn.net/images/cover/ab/1000x1000-000000-80-0-0.jpg"}}"#,
        )
        .unwrap();
        let song = track(item, None);
        assert_eq!(song.id, SourceKey::deezer("42"));
        assert_eq!(song.artists, ["Main Artist", "Partner", "Guest"]);
        assert_eq!(song.artist_id, Some(SourceKey::deezer("7")));
        assert_eq!(
            song.album,
            Some(AlbumRef {
                id: SourceKey::deezer("5"),
                title: "Morning".to_owned()
            })
        );
        assert_eq!(
            (song.duration_secs, song.track_number, song.disc_number),
            (Some(211), Some(3), Some(2))
        );
        assert_eq!(song.isrc.as_deref(), Some("ZZXX12100003"));
        assert!(song.cover_url.unwrap().contains("1000x1000"));
    }

    #[test]
    fn albums_read_from_deezer() {
        let item: ApiAlbum = serde_json::from_str(
            r#"{"id": 5, "title": "Morning", "release_date": "2021-04-02", "record_type": "ep",
                "cover_xl": "https://cdn-images.dzcdn.net/images/cover/ab/1000x1000-000000-80-0-0.jpg",
                "artist": {"id": 7, "name": "Main Artist"},
                "contributors": [
                    {"id": 7, "name": "Main Artist", "role": "Main"},
                    {"id": 8, "name": "Guest", "role": "Featured"}
                ]}"#,
        )
        .unwrap();
        let morning = album(item, None, Vec::new());
        assert_eq!(morning.id, SourceKey::deezer("5"));
        assert_eq!(morning.artists, ["Main Artist"]);
        assert_eq!(morning.artist_id, Some(SourceKey::deezer("7")));
        assert_eq!(morning.year, Some(2021));
        assert_eq!(morning.kind, AlbumKind::Ep);

        // An artist's albums leave the artist out.
        let listed: ApiAlbum = serde_json::from_str(
            r#"{"id": 6, "title": "Collected", "release_date": "0000-00-00", "record_type": "compile"}"#,
        )
        .unwrap();
        let main: ApiArtist = serde_json::from_str(r#"{"id": 7, "name": "Main Artist"}"#).unwrap();
        let collected = album(listed, Some(&main), Vec::new());
        assert_eq!(collected.artists, ["Main Artist"]);
        assert_eq!(collected.year, None);
        assert_eq!(collected.kind, AlbumKind::Other);
    }

    #[test]
    fn pages_are_deezer_s() {
        let source = DeezerSource::new(MONOCHROME_API).unwrap();
        assert_eq!(
            source.page_url(Page::Song, "42"),
            "https://www.deezer.com/track/42"
        );
        assert_eq!(
            parse_link(&source.page_url(Page::Album, "5")),
            Some(Link::Album(SourceKey::deezer("5")))
        );
        assert!(source.yt_dlp("42").is_none());
        assert!(source.login().is_none());
        assert_eq!(source.short_link_hosts(), SHORT_LINK_HOSTS);
    }
}
