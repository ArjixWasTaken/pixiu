//! Platform-neutral descriptions of remote music, so the rest of píxiū does
//! not depend on any platform client's types.

use pixiu_db::SourceKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumRef {
    pub id: SourceKey,
    pub title: String,
}

/// A track on a platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteTrack {
    /// The song on the platform (a YouTube Music video).
    pub id: SourceKey,
    pub title: String,
    /// Artist names, primary first.
    pub artists: Vec<String>,
    /// The primary artist on the platform (a YouTube Music channel), when it
    /// names one.
    #[serde(default)]
    pub artist_id: Option<SourceKey>,
    pub album: Option<AlbumRef>,
    pub duration_secs: Option<u32>,
    pub track_number: Option<u16>,
    /// The disc of a release with several, when the platform says.
    #[serde(default)]
    pub disc_number: Option<u16>,
    /// The recording's ISRC, when the platform says.
    #[serde(default)]
    pub isrc: Option<String>,
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
    /// The album on the platform (a YouTube Music browse id).
    pub id: SourceKey,
    pub title: String,
    pub artists: Vec<String>,
    /// The primary artist on the platform (a YouTube Music channel), when it
    /// names one.
    #[serde(default)]
    pub artist_id: Option<SourceKey>,
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
    pub id: SourceKey,
    pub name: String,
    pub image_url: Option<String>,
}

/// A playlist on a platform, with all its tracks in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemotePlaylist {
    pub id: SourceKey,
    pub name: String,
    #[serde(default)]
    pub image_url: Option<String>,
    pub tracks: Vec<RemoteTrack>,
}

/// An artist and their releases, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discography {
    /// The artist on the platform (a YouTube Music channel).
    pub id: SourceKey,
    pub name: String,
    #[serde(default)]
    pub image_url: Option<String>,
    pub albums: Vec<RemoteAlbum>,
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

/// The same image scaled to `size` pixels square, for images Google and
/// Deezer resize on request; other URLs come back unchanged.
#[must_use]
pub fn image_url_at(url: &str, size: u32) -> String {
    if url.contains("googleusercontent.com")
        && let Some((base, options)) = url.rsplit_once('=')
        && options.starts_with('w')
    {
        return format!("{base}=w{size}-h{size}-l90-rj");
    }
    // `…/images/cover/<hash>/1000x1000-000000-80-0-0.jpg`
    if url.contains(".dzcdn.net/images/")
        && let Some((base, name)) = url.rsplit_once('/')
        && let Some((dimensions, rest)) = name.split_once('-')
        && let Some((width, height)) = dimensions.split_once('x')
        && width.parse::<u32>().is_ok()
        && height.parse::<u32>().is_ok()
    {
        return format!("{base}/{size}x{size}-{rest}");
    }
    url.to_owned()
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
    fn resizes_deezer_covers() {
        assert_eq!(
            image_url_at(
                "https://cdn-images.dzcdn.net/images/cover/2fec34/1000x1000-000000-80-0-0.jpg",
                400
            ),
            "https://cdn-images.dzcdn.net/images/cover/2fec34/400x400-000000-80-0-0.jpg"
        );
        let other = "https://cdn-images.dzcdn.net/images/misc/logo.png";
        assert_eq!(image_url_at(other, 400), other);
    }

    #[test]
    fn credits_join_artists() {
        let track = RemoteTrack {
            id: SourceKey::youtube_music("x"),
            title: "t".to_owned(),
            artists: vec!["A".to_owned(), "B".to_owned()],
            artist_id: None,
            album: None,
            duration_secs: None,
            track_number: None,
            disc_number: None,
            isrc: None,
            cover_url: None,
            is_video: false,
        };
        assert_eq!(track.artist_credit(), "A, B");
    }
}
