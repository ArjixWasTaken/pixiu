//! Platform-neutral descriptions of remote music, so the rest of píxiū does
//! not depend on any platform client's types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumRef {
    pub id: String,
    pub title: String,
}

/// A track on a platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteTrack {
    /// The platform's id (a YouTube video id).
    pub id: String,
    pub title: String,
    /// Artist names, primary first.
    pub artists: Vec<String>,
    pub album: Option<AlbumRef>,
    pub duration_secs: Option<u32>,
    pub track_number: Option<u16>,
    pub cover_url: Option<String>,
    /// A music video rather than a studio track.
    pub is_video: bool,
}

impl RemoteTrack {
    /// "Artist A, Artist B", as shown to people and written to tags.
    #[must_use]
    pub fn artist_credit(&self) -> String {
        self.artists.join(", ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlbumKind {
    Album,
    Ep,
    Single,
    Other,
}

/// An album on a platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteAlbum {
    /// The platform's id (a YouTube Music browse id).
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub year: Option<u16>,
    pub kind: AlbumKind,
    pub cover_url: Option<String>,
    /// Filled when the album itself was fetched, not when it was only
    /// listed.
    pub tracks: Vec<RemoteTrack>,
}

/// An artist on a platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteArtist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResults {
    pub tracks: Vec<RemoteTrack>,
    pub albums: Vec<RemoteAlbum>,
    pub artists: Vec<RemoteArtist>,
}

/// What a session check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCheck {
    /// The platform accepts the session.
    Valid,
    /// The platform rejected the session (logged out, password changed).
    Invalid(String),
    /// The platform could not be asked (network or server trouble).
    Unreachable(String),
}

/// Picks the largest image and asks the image CDN for a large rendition.
///
/// Google's image URLs end in size options (`=w120-h120-...`); replacing
/// them yields a bigger picture.
#[must_use]
pub(crate) fn best_image_url(candidates: &[(String, u32)]) -> Option<String> {
    let (url, _) = candidates.iter().max_by_key(|(_, width)| *width)?;
    Some(image_url_at(url, 1200))
}

/// The same image scaled to `size` pixels square, for images Google resizes
/// on request; other URLs come back unchanged.
#[must_use]
pub fn image_url_at(url: &str, size: u32) -> String {
    match url.rsplit_once('=') {
        Some((base, options))
            if url.contains("googleusercontent.com") && options.starts_with('w') =>
        {
            format!("{base}=w{size}-h{size}-l90-rj")
        }
        _ => url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_google_image_urls() {
        let url = best_image_url(&[
            (
                "https://lh3.googleusercontent.com/abc=w60-h60-l90-rj".to_owned(),
                60,
            ),
            (
                "https://lh3.googleusercontent.com/abc=w544-h544-l90-rj".to_owned(),
                544,
            ),
        ]);
        assert_eq!(
            url.as_deref(),
            Some("https://lh3.googleusercontent.com/abc=w1200-h1200-l90-rj")
        );
        let url = best_image_url(&[("https://i.ytimg.com/vi/x/hqdefault.jpg".to_owned(), 480)]);
        assert_eq!(
            url.as_deref(),
            Some("https://i.ytimg.com/vi/x/hqdefault.jpg")
        );
        assert_eq!(best_image_url(&[]), None);
        assert_eq!(
            image_url_at(
                "https://yt3.googleusercontent.com/abc=w1200-h1200-l90-rj",
                400
            ),
            "https://yt3.googleusercontent.com/abc=w400-h400-l90-rj"
        );
    }

    #[test]
    fn credits_join_artists() {
        let track = RemoteTrack {
            id: "x".to_owned(),
            title: "t".to_owned(),
            artists: vec!["A".to_owned(), "B".to_owned()],
            album: None,
            duration_secs: None,
            track_number: None,
            cover_url: None,
            is_video: false,
        };
        assert_eq!(track.artist_credit(), "A, B");
    }
}
