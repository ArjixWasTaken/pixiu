//! YouTube Music's links and ids, as people paste them: what each points
//! at, as keys.

use pixiu_db::{Platform, SourceKey};
use reqwest::Url;

use crate::source::Link;

fn key(id: String) -> SourceKey {
    SourceKey::youtube_music(id)
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
        Link::LikedMusic(key(candidate))
    } else if let Some(playlist) = candidate.strip_prefix("VL") {
        playlist_link(playlist)?
    } else if candidate.starts_with("UC") && candidate.len() == 24 {
        Link::Artist(key(candidate))
    } else if candidate.starts_with("MPREb_") {
        Link::Album(key(candidate))
    } else if ["PL", "OLAK5uy_", "RDCLAK", "RD", "FL", "UU", "LL"]
        .iter()
        .any(|prefix| candidate.starts_with(prefix))
    {
        Link::Playlist(key(candidate))
    } else if candidate.len() == 11 {
        Link::Track(key(candidate))
    } else {
        return None;
    })
}

fn playlist_link(list: &str) -> Option<Link> {
    let list = id(list)?;
    Some(if list == crate::LIKED_MUSIC {
        Link::LikedMusic(key(list))
    } else {
        Link::Playlist(key(list))
    })
}

/// Reads a link, an id or a key; `None` when it is not something YouTube
/// Music knows.
#[must_use]
pub(crate) fn parse(input: &str) -> Option<Link> {
    let input = input.trim();
    if let Some(id) = input
        .strip_prefix(Platform::YouTubeMusic.as_str())
        .and_then(|rest| rest.strip_prefix(':'))
    {
        return bare(id);
    }
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
        ("youtu.be", [video]) => id(video).map(|id| Link::Track(key(id))),
        ("music.youtube.com" | "youtube.com", ["playlist"]) => playlist_link(&query("list")?),
        ("music.youtube.com" | "youtube.com", ["watch"]) => {
            id(&query("v")?).map(|id| Link::Track(key(id)))
        }
        ("music.youtube.com" | "youtube.com", ["channel", channel, ..]) => {
            id(channel).map(|id| Link::Artist(key(id)))
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
        let playlist = Link::Playlist(key("PLx1y2".to_owned()));
        for input in [
            "https://music.youtube.com/playlist?list=PLx1y2",
            "https://www.youtube.com/playlist?list=PLx1y2&si=abc",
            "  PLx1y2 ",
            "https://music.youtube.com/browse/VLPLx1y2",
            "youtube_music:PLx1y2",
        ] {
            assert_eq!(parse(input), Some(playlist.clone()), "{input}");
        }
        let liked = Link::LikedMusic(key("LM".to_owned()));
        assert_eq!(
            parse("https://music.youtube.com/playlist?list=LM"),
            Some(liked.clone())
        );
        assert_eq!(parse("LM"), Some(liked));

        let artist = Link::Artist(key("UCabcdefghijklmnopqrstuv".to_owned()));
        for input in [
            "https://music.youtube.com/channel/UCabcdefghijklmnopqrstuv",
            "https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv/videos",
            "UCabcdefghijklmnopqrstuv",
        ] {
            assert_eq!(parse(input), Some(artist.clone()), "{input}");
        }

        assert_eq!(
            parse("https://music.youtube.com/browse/MPREb_jwN9EIjDfPS"),
            Some(Link::Album(key("MPREb_jwN9EIjDfPS".to_owned())))
        );
        let track = Link::Track(key("NPdgPZ0u3zQ".to_owned()));
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
            "youtube_music:",
        ] {
            assert_eq!(parse(input), None, "{input}");
        }
    }
}
