//! Subsonic ids: database keys with a type prefix (`ar-1`, `al-2`, `tr-3`,
//! `pl-4`), so endpoints like `getMusicDirectory` and `getCoverArt` can
//! tell what an id refers to.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id {
    Artist(u64),
    Album(u64),
    Track(u64),
    Playlist(u64),
}

impl Id {
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        let (prefix, key) = id.split_once('-')?;
        let key = key.parse().ok()?;
        match prefix {
            "ar" => Some(Self::Artist(key)),
            "al" => Some(Self::Album(key)),
            "tr" => Some(Self::Track(key)),
            "pl" => Some(Self::Playlist(key)),
            _ => None,
        }
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Artist(id) => write!(f, "ar-{id}"),
            Self::Album(id) => write!(f, "al-{id}"),
            Self::Track(id) => write!(f, "tr-{id}"),
            Self::Playlist(id) => write!(f, "pl-{id}"),
        }
    }
}

#[must_use]
pub fn artist(id: u64) -> String {
    format!("ar-{id}")
}

#[must_use]
pub fn album(id: u64) -> String {
    format!("al-{id}")
}

#[must_use]
pub fn track(id: u64) -> String {
    format!("tr-{id}")
}

#[must_use]
pub fn playlist(id: u64) -> String {
    format!("pl-{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        assert_eq!(Id::parse(&artist(7)), Some(Id::Artist(7)));
        assert_eq!(Id::parse(&album(8)), Some(Id::Album(8)));
        assert_eq!(Id::parse(&track(9)), Some(Id::Track(9)));
        assert_eq!(Id::parse(&playlist(10)), Some(Id::Playlist(10)));
        assert_eq!(Id::Album(8).to_string(), album(8));
        assert_eq!(Id::parse("xx-1"), None);
        assert_eq!(Id::parse("tr-"), None);
        assert_eq!(Id::parse("42"), None);
    }
}
