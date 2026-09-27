//! Where files live inside the treasure:
//!
//! ```text
//! {Album Artist}/{Year} - {Album}/{Disc:02}-{Track:02} {Title}.{ext}
//! ```
//!
//! Path components are sanitized so the tree works on any filesystem,
//! including SMB shares with Windows naming rules.

use std::path::PathBuf;

/// Longest component, in bytes. Well below common 255-byte limits, leaving
/// room for collision suffixes.
const MAX_COMPONENT_BYTES: usize = 150;

/// The pieces of metadata a track's location is derived from.
#[derive(Debug, Clone, Copy)]
pub struct TrackLocation<'a> {
    pub album_artist: &'a str,
    pub album: &'a str,
    pub year: Option<i32>,
    pub disc: Option<u32>,
    pub track: Option<u32>,
    pub title: &'a str,
    pub suffix: &'a str,
}

/// The album directory, relative to the treasure root.
#[must_use]
pub fn album_dir(album_artist: &str, album: &str, year: Option<i32>) -> PathBuf {
    let album = match year {
        Some(year) => format!("{year} - {album}"),
        None => album.to_owned(),
    };
    [
        sanitize(album_artist, "Unknown Artist"),
        sanitize(&album, "Unknown Album"),
    ]
    .iter()
    .collect()
}

/// The audio file, relative to the treasure root.
#[must_use]
pub fn track_path(location: TrackLocation<'_>) -> PathBuf {
    let number = match (location.disc, location.track) {
        (Some(disc), Some(track)) => format!("{disc:02}-{track:02} "),
        (None, Some(track)) => format!("{track:02} "),
        (_, None) => String::new(),
    };
    let stem = sanitize(&format!("{number}{}", location.title), "Untitled");
    let suffix = sanitize(location.suffix, "bin").to_lowercase();

    album_dir(location.album_artist, location.album, location.year).join(format!("{stem}.{suffix}"))
}

/// The `n`th alternative name for `path` when it is taken: `x (2).flac`.
#[must_use]
pub fn numbered(path: &std::path::Path, n: u32) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = match path.extension() {
        Some(extension) => format!("{stem} ({n}).{}", extension.to_string_lossy()),
        None => format!("{stem} ({n})"),
    };
    path.with_file_name(name)
}

/// Makes `name` safe as a single path component.
#[must_use]
pub fn sanitize(name: &str, fallback: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    // Leading dots would hide files; trailing dots and spaces are invalid on
    // Windows shares.
    let trimmed = collapsed
        .trim_start_matches('.')
        .trim_end_matches(['.', ' '])
        .trim();
    let truncated = truncate_bytes(trimmed, MAX_COMPONENT_BYTES).trim_end_matches(['.', ' ']);

    if truncated.is_empty() {
        fallback.to_owned()
    } else {
        truncated.to_owned()
    }
}

fn truncate_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn location<'a>(title: &'a str) -> TrackLocation<'a> {
        TrackLocation {
            album_artist: "Daft Punk",
            album: "Discovery",
            year: Some(2001),
            disc: Some(1),
            track: Some(3),
            title,
            suffix: "FLAC",
        }
    }

    #[test]
    fn builds_the_documented_layout() {
        assert_eq!(
            track_path(location("Digital Love")),
            Path::new("Daft Punk/2001 - Discovery/01-03 Digital Love.flac")
        );
    }

    #[test]
    fn omits_missing_numbers_and_years() {
        let path = track_path(TrackLocation {
            year: None,
            disc: None,
            track: None,
            ..location("Intro")
        });
        assert_eq!(path, Path::new("Daft Punk/Discovery/Intro.flac"));

        let path = track_path(TrackLocation {
            disc: None,
            ..location("Intro")
        });
        assert_eq!(path, Path::new("Daft Punk/2001 - Discovery/03 Intro.flac"));
    }

    #[test]
    fn sanitizes_hostile_names() {
        assert_eq!(sanitize("AC/DC", "x"), "AC_DC");
        assert_eq!(sanitize("..", "Unknown"), "Unknown");
        assert_eq!(sanitize("  What?  ", "x"), "What_");
        assert_eq!(sanitize("tab\there\nnewline", "x"), "tab_here_newline");
        assert_eq!(sanitize("Mr. ", "x"), "Mr");
        assert_eq!(sanitize(".hidden", "x"), "hidden");
        assert_eq!(sanitize("", "Untitled"), "Untitled");
    }

    #[test]
    fn truncates_on_char_boundaries() {
        let long = "ü".repeat(200);
        let sanitized = sanitize(&long, "x");
        assert!(sanitized.len() <= MAX_COMPONENT_BYTES);
        assert!(sanitized.chars().all(|c| c == 'ü'));
    }

    #[test]
    fn numbers_collisions() {
        assert_eq!(
            numbered(Path::new("A/B/01 Song.flac"), 2),
            Path::new("A/B/01 Song (2).flac")
        );
    }
}
