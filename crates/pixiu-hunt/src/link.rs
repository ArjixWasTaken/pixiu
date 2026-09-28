//! What the admin pastes: YouTube Music and YouTube links, or bare ids.

use reqwest::Url;

/// What a link points at on YouTube Music.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Playlist(String),
    /// The account's liked music.
    LikedMusic,
    /// A channel id.
    Artist(String),
    /// An album browse id.
    Album(String),
    /// A video id.
    Track(String),
}

/// Platform ids are short and use a URL-safe alphabet.
fn id(candidate: &str) -> Option<String> {
    let valid = !candidate.is_empty()
        && candidate.len() <= 64
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    valid.then(|| candidate.to_owned())
}

/// Tells what a bare id is by its shape.
fn bare(candidate: &str) -> Option<Link> {
    let candidate = id(candidate)?;
    Some(if candidate == crate::LIKED_MUSIC {
        Link::LikedMusic
    } else if let Some(playlist) = candidate.strip_prefix("VL") {
        playlist_link(playlist)?
    } else if candidate.starts_with("UC") && candidate.len() == 24 {
        Link::Artist(candidate)
    } else if candidate.starts_with("MPREb_") {
        Link::Album(candidate)
    } else if ["PL", "OLAK5uy_", "RDCLAK", "RD", "FL", "UU", "LL"]
        .iter()
        .any(|prefix| candidate.starts_with(prefix))
    {
        Link::Playlist(candidate)
    } else if candidate.len() == 11 {
        Link::Track(candidate)
    } else {
        return None;
    })
}

fn playlist_link(list: &str) -> Option<Link> {
    let list = id(list)?;
    Some(if list == crate::LIKED_MUSIC {
        Link::LikedMusic
    } else {
        Link::Playlist(list)
    })
}

/// Reads a link or an id; `None` when it is not something YouTube Music
/// knows.
#[must_use]
pub fn parse(input: &str) -> Option<Link> {
    let input = input.trim();
    let Ok(url) = Url::parse(input) else {
        return bare(input);
    };
    let host = url
        .host_str()?
        .trim_start_matches("www.")
        .trim_start_matches("m.");
    let query = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    match (host, segments.as_slice()) {
        ("youtu.be", [video]) => id(video).map(Link::Track),
        ("music.youtube.com" | "youtube.com", ["playlist"]) => playlist_link(&query("list")?),
        ("music.youtube.com" | "youtube.com", ["watch"]) => id(&query("v")?).map(Link::Track),
        ("music.youtube.com" | "youtube.com", ["channel", channel, ..]) => {
            id(channel).map(Link::Artist)
        }
        ("music.youtube.com", ["browse", browse]) => bare(browse),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_and_ids_are_understood() {
        let playlist = Link::Playlist("PLx1y2".to_owned());
        for input in [
            "https://music.youtube.com/playlist?list=PLx1y2",
            "https://www.youtube.com/playlist?list=PLx1y2&si=abc",
            "  PLx1y2 ",
            "https://music.youtube.com/browse/VLPLx1y2",
        ] {
            assert_eq!(parse(input), Some(playlist.clone()), "{input}");
        }
        assert_eq!(
            parse("https://music.youtube.com/playlist?list=LM"),
            Some(Link::LikedMusic)
        );
        assert_eq!(parse("LM"), Some(Link::LikedMusic));

        let artist = Link::Artist("UCabcdefghijklmnopqrstuv".to_owned());
        for input in [
            "https://music.youtube.com/channel/UCabcdefghijklmnopqrstuv",
            "https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv/videos",
            "UCabcdefghijklmnopqrstuv",
        ] {
            assert_eq!(parse(input), Some(artist.clone()), "{input}");
        }

        assert_eq!(
            parse("https://music.youtube.com/browse/MPREb_jwN9EIjDfPS"),
            Some(Link::Album("MPREb_jwN9EIjDfPS".to_owned()))
        );
        let track = Link::Track("NPdgPZ0u3zQ".to_owned());
        for input in [
            "https://music.youtube.com/watch?v=NPdgPZ0u3zQ&list=RDAMVM",
            "https://youtu.be/NPdgPZ0u3zQ",
            "NPdgPZ0u3zQ",
        ] {
            assert_eq!(parse(input), Some(track.clone()), "{input}");
        }

        for input in [
            "",
            "https://example.com/playlist?list=PLx",
            "https://music.youtube.com/playlist?list=../etc",
            "not an id at all",
            "https://music.youtube.com/@someone",
        ] {
            assert_eq!(parse(input), None, "{input}");
        }
    }
}
