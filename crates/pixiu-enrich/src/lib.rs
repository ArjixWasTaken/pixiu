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
pub use wiki::ArtistInfo;

use crate::{lyrics::Lrclib, musicbrainz::MusicBrainz};

#[derive(Debug, thiserror::Error)]
pub enum EnrichError {
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("not found")]
    NotFound,
    #[error("the service is busy; try again later")]
    Busy,
    #[error("unexpected answer: {0}")]
    Unexpected(String),
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
