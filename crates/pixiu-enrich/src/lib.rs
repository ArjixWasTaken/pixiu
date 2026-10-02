//! Knowing more about the hoard: MusicBrainz tags, Cover Art Archive
//! covers, LRCLIB lyrics and Wikipedia biographies.
//!
//! [`Sources`] is what the rest of píxiū uses, so tests can stand in for
//! the internet; [`Online`] is the real thing.

pub mod lyrics;
pub mod matching;
mod musicbrainz;
mod wiki;

use std::{future::Future, pin::Pin, time::Duration};

use reqwest::StatusCode;

pub use lyrics::{FoundLyrics, Line, LyricsQuery, looks_synced, parse_lrc, plain_from_lrc};
pub use matching::{Candidate, Credit, LocalAlbum, LocalTrack, Pairing, Release, ReleaseTrack};
pub use musicbrainz::genre_name;
pub use wiki::ArtistInfo;

use crate::{lyrics::Lrclib, musicbrainz::MusicBrainz};

#[derive(Debug, thiserror::Error)]
pub enum EnrichError {
    #[error("{}", http_problem(.0))]
    Http(#[from] reqwest::Error),
    #[error("not found")]
    NotFound,
    #[error("the service is busy; try again later")]
    Busy,
    #[error("unexpected answer: {0}")]
    Unexpected(String),
}

/// A failed request, for people: which service, and what went wrong, without
/// the request's URL.
fn http_problem(error: &reqwest::Error) -> String {
    let host = error.url().and_then(|url| url.host_str()).unwrap_or("");
    let name = if host.ends_with("musicbrainz.org") {
        "MusicBrainz"
    } else if host.ends_with("coverartarchive.org") || host.ends_with("archive.org") {
        "the Cover Art Archive"
    } else if host.ends_with("lrclib.net") {
        "LRCLIB"
    } else if host.ends_with("wikipedia.org") || host.ends_with("wikidata.org") {
        "Wikipedia"
    } else {
        "the service"
    };
    // The name starting a sentence.
    let mut chars = name.chars();
    let subject: String = chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    if let Some(status) = error.status() {
        let answer = format!(
            "{} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or_default()
        );
        let answer = answer.trim();
        if status.is_server_error() || status.as_u16() == 429 {
            format!("{subject} is busy or down ({answer}); try again later.")
        } else {
            format!("{subject} answered {answer}.")
        }
    } else if error.is_timeout() {
        format!("{subject} took too long to answer; try again later.")
    } else if error.is_connect() {
        format!("píxiū couldn't reach {name}; check the server's connection.")
    } else if error.is_decode() {
        format!("{subject} sent an answer píxiū can't read.")
    } else {
        format!("Asking {name} failed.")
    }
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Where knowledge about music comes from.
pub trait Sources: Send + Sync {
    /// MusicBrainz releases that might be `album`, most promising first.
    fn search<'a>(
        &'a self,
        album: &'a LocalAlbum,
    ) -> BoxFuture<'a, Result<Vec<Candidate>, EnrichError>>;

    /// A MusicBrainz release with its tracks.
    fn release<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Release, EnrichError>>;

    /// A release's front cover from the Cover Art Archive.
    fn front_cover<'a>(
        &'a self,
        release_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<Vec<u8>>, EnrichError>>;

    fn lyrics<'a>(
        &'a self,
        query: &'a LyricsQuery,
    ) -> BoxFuture<'a, Result<Option<FoundLyrics>, EnrichError>>;

    /// What Wikipedia says about a MusicBrainz artist.
    fn artist_info<'a>(
        &'a self,
        artist_mbid: &'a str,
    ) -> BoxFuture<'a, Result<Option<ArtistInfo>, EnrichError>>;

    /// Any image, e.g. an artist's picture.
    fn image<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<Vec<u8>, EnrichError>>;
}

/// The services on the internet.
#[derive(Debug)]
pub struct Online {
    http: reqwest::Client,
    musicbrainz: MusicBrainz,
    lrclib: Lrclib,
}

impl Online {
    /// `contact` (an email address or a URL) goes into the User-Agent, as
    /// MusicBrainz and Wikimedia ask of applications.
    ///
    /// # Errors
    ///
    /// Fails when the HTTP client cannot be built.
    pub fn new(contact: Option<&str>) -> Result<Self, EnrichError> {
        let contact = contact.filter(|contact| !contact.trim().is_empty());
        let user_agent = format!(
            "pixiu/{} ( {} )",
            env!("CARGO_PKG_VERSION"),
            contact.unwrap_or("self-hosted OpenSubsonic server")
        );
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            musicbrainz: MusicBrainz::new(http.clone()),
            lrclib: Lrclib::new(http.clone()),
            http,
        })
    }
}

impl Sources for Online {
    fn search<'a>(
        &'a self,
        album: &'a LocalAlbum,
    ) -> BoxFuture<'a, Result<Vec<Candidate>, EnrichError>> {
        Box::pin(self.musicbrainz.search(album))
    }

    fn release<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Release, EnrichError>> {
        Box::pin(self.musicbrainz.release(id))
    }

    fn front_cover<'a>(
        &'a self,
        release_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<Vec<u8>>, EnrichError>> {
        Box::pin(async move {
            // A 1200 px rendition when there is one, else the original.
            for size in ["front-1200", "front"] {
                let url = format!("https://coverartarchive.org/release/{release_id}/{size}");
                let response = self.http.get(url).send().await?;
                if response.status() == StatusCode::NOT_FOUND {
                    continue;
                }
                let bytes = response.error_for_status()?.bytes().await?;
                return Ok(Some(bytes.to_vec()));
            }
            Ok(None)
        })
    }

    fn lyrics<'a>(
        &'a self,
        query: &'a LyricsQuery,
    ) -> BoxFuture<'a, Result<Option<FoundLyrics>, EnrichError>> {
        Box::pin(self.lrclib.find(query))
    }

    fn artist_info<'a>(
        &'a self,
        artist_mbid: &'a str,
    ) -> BoxFuture<'a, Result<Option<ArtistInfo>, EnrichError>> {
        Box::pin(async move {
            match self.musicbrainz.wikidata(artist_mbid).await? {
                Some(item) => wiki::artist_info(&self.http, &item).await,
                None => Ok(None),
            }
        })
    }

    fn image<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<Vec<u8>, EnrichError>> {
        Box::pin(async move {
            let response = self.http.get(url).send().await?.error_for_status()?;
            Ok(response.bytes().await?.to_vec())
        })
    }
}

#[cfg(test)]
mod http_problem_tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::EnrichError;

    /// The error of a request to a server that answers `status_line`.
    async fn answered(status_line: &str) -> EnrichError {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let reply =
            format!("HTTP/1.1 {status_line}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 1024];
            let _ = socket.read(&mut request).await;
            socket.write_all(reply.as_bytes()).await.unwrap();
        });
        let response = reqwest::get(format!("http://{address}/ws/2/release?query=secret"))
            .await
            .unwrap();
        response.error_for_status().unwrap_err().into()
    }

    #[tokio::test]
    async fn failed_requests_say_what_happened_without_their_url() {
        let busy = answered("503 Service Unavailable").await.to_string();
        assert_eq!(
            busy,
            "The service is busy or down (503 Service Unavailable); try again later."
        );
        assert!(!busy.contains("secret"));
        assert_eq!(
            answered("404 Not Found").await.to_string(),
            "The service answered 404 Not Found."
        );
    }
}
