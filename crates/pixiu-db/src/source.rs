//! Where downloaded music comes from: the platforms píxiū fetches from, and
//! the keys naming what is on them. A key is the platform and its own id,
//! as in `youtube_music:dQw4w9WgXcQ`, so ids of different platforms never
//! collide, and one column or lookup serves them all.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A platform music is fetched from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Platform {
    YouTubeMusic,
    Deezer,
}

impl Platform {
    /// Every platform, in a stable order.
    pub const ALL: &[Platform] = &[Platform::YouTubeMusic, Platform::Deezer];

    /// The platform as stored: in keys, and in `source_sessions.source`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::YouTubeMusic => "youtube_music",
            Self::Deezer => "deezer",
        }
    }

    /// The platform's name, for people.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::YouTubeMusic => "YouTube Music",
            Self::Deezer => "Deezer",
        }
    }

    /// The platform stored as `name`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|platform| platform.as_str() == name)
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Something on a platform (a song, an album, an artist, a playlist), by
/// the platform's own id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceKey {
    platform: Platform,
    id: String,
}

/// Why a key could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    #[error("unknown platform “{0}”")]
    Platform(String),
    #[error("a key needs an id")]
    Empty,
}

impl SourceKey {
    /// # Panics
    ///
    /// Panics when `id` is empty: keys always name something.
    #[must_use]
    pub fn new(platform: Platform, id: impl Into<String>) -> Self {
        let id = id.into();
        assert!(!id.is_empty(), "a key needs an id");
        Self { platform, id }
    }

    /// A YouTube Music video, album browse id, channel or playlist.
    #[must_use]
    pub fn youtube_music(id: impl Into<String>) -> Self {
        Self::new(Platform::YouTubeMusic, id)
    }

    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// A Deezer song, album, artist or playlist.
    #[must_use]
    pub fn deezer(id: impl Into<String>) -> Self {
        Self::new(Platform::Deezer, id)
    }

    /// The platform's own id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The key as stored.
    #[must_use]
    pub fn as_stored(&self) -> String {
        self.to_string()
    }

    /// Reads a stored key, if there is one.
    #[must_use]
    pub fn from_stored(stored: Option<&str>) -> Option<Self> {
        stored?.parse().ok()
    }
}

impl fmt::Display for SourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.platform, self.id)
    }
}

impl FromStr for SourceKey {
    type Err = KeyError;

    /// Reads `platform:id`. A bare id, without a platform, is a YouTube
    /// Music one, as every id was before there were keys (their ids never
    /// hold a colon): old job payloads and links still read.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (platform, id) = match value.split_once(':') {
            Some((name, id)) => (
                Platform::from_name(name).ok_or_else(|| KeyError::Platform(name.to_owned()))?,
                id,
            ),
            None => (Platform::YouTubeMusic, value),
        };
        if id.is_empty() {
            return Err(KeyError::Empty);
        }
        Ok(Self {
            platform,
            id: id.to_owned(),
        })
    }
}

impl Serialize for SourceKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for SourceKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_name_their_platform() {
        let key = SourceKey::youtube_music("dQw4w9WgXcQ");
        assert_eq!(key.to_string(), "youtube_music:dQw4w9WgXcQ");
        assert_eq!(key.platform(), Platform::YouTubeMusic);
        assert_eq!(key.id(), "dQw4w9WgXcQ");
        assert_eq!("youtube_music:dQw4w9WgXcQ".parse(), Ok(key));
    }

    #[test]
    fn deezer_keys_read_back() {
        let key = SourceKey::deezer("3135556");
        assert_eq!(key.to_string(), "deezer:3135556");
        assert_eq!("deezer:3135556".parse(), Ok(key));
        assert_eq!(Platform::from_name("deezer"), Some(Platform::Deezer));
    }

    #[test]
    fn bare_ids_are_youtube_music_ones() {
        assert_eq!("LM".parse(), Ok(SourceKey::youtube_music("LM")));
        assert_eq!(
            "MPREb_jwN9EIjDfPS".parse(),
            Ok(SourceKey::youtube_music("MPREb_jwN9EIjDfPS"))
        );
    }

    #[test]
    fn unknown_platforms_and_empty_ids_are_refused() {
        assert_eq!(
            "myspace:123".parse::<SourceKey>(),
            Err(KeyError::Platform("myspace".to_owned()))
        );
        assert_eq!("youtube_music:".parse::<SourceKey>(), Err(KeyError::Empty));
        assert_eq!("".parse::<SourceKey>(), Err(KeyError::Empty));
    }

    #[test]
    fn keys_go_through_json_as_strings() {
        let key = SourceKey::youtube_music("abc");
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(json, r#""youtube_music:abc""#);
        assert_eq!(serde_json::from_str::<SourceKey>(&json).unwrap(), key);
        assert_eq!(
            serde_json::from_str::<SourceKey>(r#""abc""#).unwrap(),
            key,
            "a bare id from before keys"
        );
    }

    #[test]
    fn stored_keys_read_back() {
        assert_eq!(
            SourceKey::from_stored(Some("youtube_music:abc")),
            Some(SourceKey::youtube_music("abc"))
        );
        assert_eq!(SourceKey::from_stored(None), None);
    }
}
