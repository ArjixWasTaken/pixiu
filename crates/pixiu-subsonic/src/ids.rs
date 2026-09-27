//! Subsonic ids: database keys with a type prefix (`ar-1`, `al-2`, `tr-3`),
//! so endpoints like `getMusicDirectory` and `getCoverArt` can tell what an
//! id refers to.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id {
    Artist(u64),
    Album(u64),
    Track(u64),
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
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        assert_eq!(Id::parse(&artist(7)), Some(Id::Artist(7)));
        assert_eq!(Id::parse(&album(8)), Some(Id::Album(8)));
        assert_eq!(Id::parse(&track(9)), Some(Id::Track(9)));
        assert_eq!(Id::parse("xx-1"), None);
        assert_eq!(Id::parse("tr-"), None);
        assert_eq!(Id::parse("42"), None);
    }
}
