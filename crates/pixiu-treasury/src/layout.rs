//! Where files live inside the treasure, following a template. The default:
//!
//! ```text
//! {album_artist}/[{year} - ]{album}/[[{disc:02}-]{track:02} ]{title}
//! ```
//!
//! which files tracks as `Album Artist/2001 - Album/01-03 Title.flac`.
//! `{field}` inserts a value (`{track:02}` pads numbers with zeros); a part
//! in `[brackets]` is left out when a value directly inside it is missing;
//! `/` separates directories; a backslash makes the next character plain
//! text (`\[{year}\]`); the file extension is added at the end.
//!
//! Path components are sanitized so the tree works on any filesystem,
//! including SMB shares with Windows naming rules, and values cannot add
//! directories of their own.

use std::{fmt, path::PathBuf};

/// Longest component, in bytes. Well below common 255-byte limits, leaving
/// room for collision suffixes.
const MAX_COMPONENT_BYTES: usize = 150;

/// The pieces of metadata a track's location is derived from.
#[derive(Debug, Clone, Copy)]
pub struct TrackLocation<'a> {
    pub album_artist: &'a str,
    /// The track's own artist credit.
    pub artist: &'a str,
    pub album: &'a str,
    pub year: Option<i32>,
    pub genre: Option<&'a str>,
    pub disc: Option<u32>,
    pub track: Option<u32>,
    pub title: &'a str,
    pub suffix: &'a str,
}

/// A value a template can insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    AlbumArtist,
    Artist,
    Album,
    Year,
    Genre,
    Disc,
    Track,
    Title,
}

impl Field {
    const ALL: [(&'static str, Field); 8] = [
        ("album_artist", Field::AlbumArtist),
        ("artist", Field::Artist),
        ("album", Field::Album),
        ("year", Field::Year),
        ("genre", Field::Genre),
        ("disc", Field::Disc),
        ("track", Field::Track),
        ("title", Field::Title),
    ];

    fn numeric(self) -> bool {
        matches!(self, Field::Year | Field::Disc | Field::Track)
    }

    /// The value, or `None` when the track has none. Names never go
    /// missing: they fall back to "Unknown ...".
    fn value(self, location: &TrackLocation<'_>, width: usize) -> Option<String> {
        let name = |value: &str, fallback: &str| {
            Some(if value.trim().is_empty() {
                fallback.to_owned()
            } else {
                value.to_owned()
            })
        };
        let number = |value: Option<i64>| value.map(|value| format!("{value:0width$}"));
        match self {
            Field::AlbumArtist => name(location.album_artist, "Unknown Artist"),
            Field::Artist => name(location.artist, "Unknown Artist"),
            Field::Album => name(location.album, "Unknown Album"),
            Field::Title => name(location.title, "Untitled"),
            Field::Genre => location
                .genre
                .filter(|genre| !genre.trim().is_empty())
                .map(str::to_owned),
            Field::Year => number(location.year.map(i64::from)),
            Field::Disc => number(location.disc.map(i64::from)),
            Field::Track => number(location.track.map(i64::from)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Value { field: Field, width: usize },
    Optional(Vec<Piece>),
}

/// Why a template was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error(
        "unknown field `{{{0}}}`; use album_artist, artist, album, year, genre, disc, track or title"
    )]
    UnknownField(String),
    #[error("`{{{0}}}` is not a number, so it cannot be padded")]
    NotANumber(String),
    #[error("a `{0}` is never closed")]
    Unclosed(char),
    #[error("a `{0}` closes nothing")]
    Unopened(char),
    #[error("the template must name the {{title}}, or tracks of an album would collide")]
    NoTitle,
}

/// Where tracks are filed; see the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    text: String,
    pieces: Vec<Piece>,
}

impl Template {
    pub const DEFAULT: &'static str =
        "{album_artist}/[{year} - ]{album}/[[{disc:02}-]{track:02} ]{title}";

    /// Reads a template.
    ///
    /// # Errors
    ///
    /// Fails on unknown fields, unbalanced brackets, or without `{title}`.
    pub fn parse(text: &str) -> Result<Self, TemplateError> {
        let mut chars = text.trim().chars().peekable();
        let pieces = parse_pieces(&mut chars, false)?;
        if !mentions_title(&pieces) {
            return Err(TemplateError::NoTitle);
        }
        Ok(Self {
            text: text.trim().to_owned(),
            pieces,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The audio file, relative to the treasure root.
    #[must_use]
    pub fn track_path(&self, location: TrackLocation<'_>) -> PathBuf {
        let rendered = render(&self.pieces, &location, true).unwrap_or_default();
        let mut components: Vec<String> = rendered
            .split('/')
            .filter(|component| !component.trim().is_empty())
            .map(|component| sanitize(component, "Unknown"))
            .collect();
        if components.is_empty() {
            components.push("Untitled".to_owned());
        }
        let suffix = sanitize(location.suffix, "bin").to_lowercase();
        let name = components.pop().expect("at least one component");
        components.push(format!("{name}.{suffix}"));
        components.iter().collect()
    }
}

impl Default for Template {
    fn default() -> Self {
        Self::parse(Self::DEFAULT).expect("the default template is valid")
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

fn parse_pieces(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    nested: bool,
) -> Result<Vec<Piece>, TemplateError> {
    let mut pieces = Vec::new();
    let mut text = String::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => text.push(chars.next().unwrap_or('\\')),
            '{' => {
                let mut inside = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(c) => inside.push(c),
                        None => return Err(TemplateError::Unclosed('{')),
                    }
                }
                let (name, width) = match inside.split_once(':') {
                    Some((name, width)) => (name.trim(), Some(width.trim())),
                    None => (inside.trim(), None),
                };
                let field = Field::ALL
                    .iter()
                    .find(|(known, _)| *known == name)
                    .map(|(_, field)| *field)
                    .ok_or_else(|| TemplateError::UnknownField(name.to_owned()))?;
                let width = match width {
                    None => 0,
                    Some(_) if !field.numeric() => {
                        return Err(TemplateError::NotANumber(name.to_owned()));
                    }
                    Some(width) => width.parse::<usize>().unwrap_or(0).min(9),
                };
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                pieces.push(Piece::Value { field, width });
            }
            '}' => return Err(TemplateError::Unopened('}')),
            '[' => {
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                pieces.push(Piece::Optional(parse_pieces(chars, true)?));
            }
            ']' if nested => {
                if !text.is_empty() {
                    pieces.push(Piece::Text(text));
                }
                return Ok(pieces);
            }
            ']' => return Err(TemplateError::Unopened(']')),
            c => text.push(c),
        }
    }
    if nested {
        return Err(TemplateError::Unclosed('['));
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    Ok(pieces)
}

fn mentions_title(pieces: &[Piece]) -> bool {
    pieces.iter().any(|piece| match piece {
        Piece::Value { field, .. } => *field == Field::Title,
        Piece::Optional(inner) => mentions_title(inner),
        Piece::Text(_) => false,
    })
}

/// Renders `pieces`; `None` when an optional part misses a value.
fn render(pieces: &[Piece], location: &TrackLocation<'_>, top: bool) -> Option<String> {
    let mut out = String::new();
    for piece in pieces {
        match piece {
            Piece::Text(text) => out.push_str(text),
            Piece::Value { field, width } => match field.value(location, *width) {
                // Values cannot add directories.
                Some(value) => out.push_str(&value.replace(['/', '\\'], "_")),
                None if top => {}
                None => return None,
            },
            Piece::Optional(inner) => {
                if let Some(text) = render(inner, location, false) {
                    out.push_str(&text);
                }
            }
        }
    }
    Some(out)
}

/// The audio file under the default template.
#[must_use]
pub fn track_path(location: TrackLocation<'_>) -> PathBuf {
    Template::default().track_path(location)
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
            artist: "Daft Punk feat. Romanthony",
            album: "Discovery",
            year: Some(2001),
            genre: Some("House"),
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
    fn a_disc_without_a_track_number_is_left_out() {
        let path = track_path(TrackLocation {
            track: None,
            ..location("Intro")
        });
        assert_eq!(path, Path::new("Daft Punk/2001 - Discovery/Intro.flac"));
    }

    #[test]
    fn templates_file_tracks_their_way() {
        let template = Template::parse("{genre}/{artist}/[{year}/]{title} [({disc})]").unwrap();
        assert_eq!(
            template.track_path(location("Digital Love")),
            Path::new("House/Daft Punk feat. Romanthony/2001/Digital Love (1).flac")
        );
        // A missing genre leaves no empty directory; names fall back.
        let path = template.track_path(TrackLocation {
            genre: None,
            year: None,
            disc: None,
            artist: " ",
            ..location("Digital Love")
        });
        assert_eq!(path, Path::new("Unknown Artist/Digital Love.flac"));
        // Values cannot climb out of the treasure or add directories.
        let path = template.track_path(TrackLocation {
            genre: Some("../../etc"),
            title: "a/b",
            ..location("")
        });
        assert_eq!(
            path,
            Path::new("_.._etc/Daft Punk feat. Romanthony/2001/a_b (1).flac")
        );
        assert_eq!(
            template.to_string(),
            "{genre}/{artist}/[{year}/]{title} [({disc})]"
        );

        // Escaped brackets are plain text.
        let template = Template::parse(r"{album} [\[{year}\]]/{title}").unwrap();
        assert_eq!(
            template.track_path(location("Digital Love")),
            Path::new("Discovery [2001]/Digital Love.flac")
        );
        let path = template.track_path(TrackLocation {
            year: None,
            ..location("Digital Love")
        });
        assert_eq!(path, Path::new("Discovery/Digital Love.flac"));
    }

    #[test]
    fn bad_templates_are_refused() {
        for (text, error) in [
            (
                "{album}/{name}",
                TemplateError::UnknownField("name".to_owned()),
            ),
            (
                "{album}/{title:02}",
                TemplateError::NotANumber("title".to_owned()),
            ),
            ("{album}/[{year} {title}", TemplateError::Unclosed('[')),
            ("{album}/{title", TemplateError::Unclosed('{')),
            ("{album}]/{title}", TemplateError::Unopened(']')),
            ("{album}/{track}", TemplateError::NoTitle),
        ] {
            assert_eq!(Template::parse(text), Err(error), "{text}");
        }
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
