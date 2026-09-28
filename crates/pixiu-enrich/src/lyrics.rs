//! Lyrics: LRCLIB (<https://lrclib.net>), which often has them time-synced,
//! and reading LRC.

use std::time::Duration;

use reqwest::{StatusCode, Url};
use serde::Deserialize;

use crate::{EnrichError, musicbrainz::Pace};

const API: &str = "https://lrclib.net/api/";

/// A track to find lyrics for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricsQuery {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u64,
}

/// Lyrics that were found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoundLyrics {
    /// LRC, time-synced.
    pub synced: Option<String>,
    pub plain: Option<String>,
    /// Known to have no words; then there is no text either.
    pub instrumental: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

impl Entry {
    fn found(self) -> Option<FoundLyrics> {
        if self.instrumental {
            return Some(FoundLyrics {
                instrumental: true,
                ..FoundLyrics::default()
            });
        }
        let keep = |text: Option<String>| text.filter(|text| !text.trim().is_empty());
        let found = FoundLyrics {
            synced: keep(self.synced_lyrics),
            plain: keep(self.plain_lyrics),
            instrumental: false,
        };
        (found.synced.is_some() || found.plain.is_some()).then_some(found)
    }
}

impl FoundLyrics {
    /// Whether these are a lone line at most, which gives way to fuller
    /// lyrics of the same length: LRCLIB has placeholder entries (a lone
    /// `[00:00.00]probe`) for well-known songs. Short lyrics are real too
    /// (a vocal hook over an EDM track), so two lines are never doubted.
    fn is_thin(&self) -> bool {
        let synced = self.synced.as_deref().map_or(0, |lrc| parse_lrc(lrc).len());
        let plain = self.plain.as_deref().map_or(0, |text| {
            text.lines().filter(|line| !line.trim().is_empty()).count()
        });
        synced.max(plain) < 2
    }

    /// Words enough, or known to have none: no need to look further.
    fn settles_it(&self) -> bool {
        self.instrumental || !self.is_thin()
    }
}

/// The pick of LRCLIB's search results: close in length, full rather than
/// thin, time-synced preferred; "instrumental" only if nobody has words.
fn best(entries: Vec<Entry>, duration_secs: u64) -> Option<FoundLyrics> {
    #[allow(clippy::cast_precision_loss)]
    let wanted = duration_secs as f64;
    let mut close: Vec<FoundLyrics> = entries
        .into_iter()
        .filter(|entry| (entry.duration - wanted).abs() <= 3.0)
        .filter_map(Entry::found)
        .collect();
    close.sort_by_key(|found| (found.instrumental, found.is_thin(), found.synced.is_none()));
    close.into_iter().next()
}

/// The exact match, unless it is thin and the search found fuller lyrics.
/// A thin one is kept over nothing (or an "instrumental"): some songs
/// really are one line long.
fn prefer(exact: Option<FoundLyrics>, searched: Option<FoundLyrics>) -> Option<FoundLyrics> {
    match (exact, searched) {
        (Some(exact), _) if exact.settles_it() => Some(exact),
        (_, Some(searched)) if !searched.instrumental && !searched.is_thin() => Some(searched),
        (Some(exact), _) => Some(exact),
        (None, searched) => searched,
    }
}

#[derive(Debug)]
pub(crate) struct Lrclib {
    http: reqwest::Client,
    pace: Pace,
}

impl Lrclib {
    pub(crate) fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            pace: Pace::default(),
        }
    }

    async fn get(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Option<reqwest::Response>, EnrichError> {
        // LRCLIB asks nothing in particular; stay polite anyway.
        self.pace.wait(Duration::from_millis(250)).await;
        let mut url = Url::parse(API)
            .expect("a valid base")
            .join(path)
            .expect("a valid path");
        url.query_pairs_mut().extend_pairs(query);
        let response = self.http.get(url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(response.error_for_status()?))
    }

    /// The lyrics of a track: an exact match first, then a search.
    pub(crate) async fn find(
        &self,
        query: &LyricsQuery,
    ) -> Result<Option<FoundLyrics>, EnrichError> {
        let duration = query.duration_secs.to_string();
        let exact = self
            .get(
                "get",
                &[
                    ("artist_name", &query.artist),
                    ("track_name", &query.title),
                    ("album_name", &query.album),
                    ("duration", &duration),
                ],
            )
            .await?;
        let exact = match exact {
            Some(response) => response.json::<Entry>().await?.found(),
            None => None,
        };
        if exact.as_ref().is_some_and(FoundLyrics::settles_it) {
            return Ok(exact);
        }

        let Some(response) = self
            .get(
                "search",
                &[("artist_name", &query.artist), ("track_name", &query.title)],
            )
            .await?
        else {
            return Ok(exact);
        };
        let entries: Vec<Entry> = response.json().await?;
        Ok(prefer(exact, best(entries, query.duration_secs)))
    }
}

/// A line of time-synced lyrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub start_ms: u64,
    pub text: String,
}

/// `mm:ss`, `mm:ss.xx` or `mm:ss.xxx`, in milliseconds.
fn timestamp(stamp: &str) -> Option<u64> {
    let (minutes, rest) = stamp.split_once(':')?;
    let (seconds, fraction) = rest.split_once('.').unwrap_or((rest, ""));
    let minutes: u64 = minutes.trim().parse().ok()?;
    let seconds: u64 = seconds.trim().parse().ok()?;
    let millis = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u64>().ok()? * 100,
        2 => fraction.parse::<u64>().ok()? * 10,
        _ => fraction.get(..3)?.parse().ok()?,
    };
    Some(minutes * 60_000 + seconds * 1000 + millis)
}

/// Reads LRC: `[mm:ss.xx] text`, several stamps per line allowed. Header
/// tags (`[ar:…]`) are skipped, except `[offset:±ms]`, which shifts every
/// line. The result is in time order.
#[must_use]
pub fn parse_lrc(lrc: &str) -> Vec<Line> {
    let mut offset: i64 = 0;
    let mut lines = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(tag) = rest.strip_prefix('[') {
            let Some((inside, after)) = tag.split_once(']') else {
                break;
            };
            if let Some(value) = inside.strip_prefix("offset:") {
                offset = value.trim().parse().unwrap_or(0);
            } else if let Some(stamp) = timestamp(inside) {
                stamps.push(stamp);
            }
            rest = after;
        }
        let text = rest.trim();
        for stamp in stamps {
            // A positive offset shows lyrics earlier.
            let start = i64::try_from(stamp)
                .unwrap_or(i64::MAX)
                .saturating_sub(offset)
                .max(0);
            lines.push(Line {
                start_ms: u64::try_from(start).unwrap_or(0),
                text: text.to_owned(),
            });
        }
    }
    lines.sort_by_key(|line| line.start_ms);
    lines
}

/// Whether text is LRC rather than plain lyrics.
#[must_use]
pub fn looks_synced(text: &str) -> bool {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .take(5)
        .any(|line| line.trim_start().starts_with('[') && !parse_lrc(line).is_empty())
}

/// The lyrics without their timing.
#[must_use]
pub fn plain_from_lrc(lrc: &str) -> String {
    parse_lrc(lrc)
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_is_read() {
        let lines = parse_lrc(
            "[ar:Somebody]\n[ti:Song]\n\n[00:01.50]First\n[00:12.345][01:02]Chorus\n[00:05]  Second  \nnot a line",
        );
        let pairs: Vec<(u64, &str)> = lines
            .iter()
            .map(|line| (line.start_ms, line.text.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                (1_500, "First"),
                (5_000, "Second"),
                (12_345, "Chorus"),
                (62_000, "Chorus")
            ]
        );
        assert_eq!(
            parse_lrc("[offset:+500]\n[00:02.00]Early")[0].start_ms,
            1_500
        );
    }

    #[test]
    fn synced_and_plain_are_told_apart() {
        assert!(looks_synced("[00:01.00]Hello"));
        assert!(looks_synced("\n[ar:Me]\n[00:01.00]Hello"));
        assert!(!looks_synced("Hello\n[Chorus]\nWorld"));
        assert_eq!(plain_from_lrc("[00:01.00]A\n[00:02.00]B"), "A\nB");
    }

    #[test]
    fn instrumentals_have_no_lyrics() {
        let entry: Entry = serde_json::from_str(
            r#"{"duration": 180, "instrumental": true, "plainLyrics": null, "syncedLyrics": null}"#,
        )
        .unwrap();
        assert_eq!(
            entry.found(),
            Some(FoundLyrics {
                instrumental: true,
                ..FoundLyrics::default()
            })
        );
        let entry: Entry = serde_json::from_str(
            r#"{"duration": 180, "instrumental": false, "plainLyrics": "la la", "syncedLyrics": "  "}"#,
        )
        .unwrap();
        assert_eq!(
            entry.found(),
            Some(FoundLyrics {
                plain: Some("la la".to_owned()),
                ..FoundLyrics::default()
            })
        );
    }

    fn entry(duration: u32, synced: Option<&str>, plain: Option<&str>) -> Entry {
        Entry {
            duration: duration.into(),
            instrumental: false,
            plain_lyrics: plain.map(str::to_owned),
            synced_lyrics: synced.map(str::to_owned),
        }
    }

    fn instrumental(duration: u32) -> Entry {
        Entry {
            instrumental: true,
            ..entry(duration, None, None)
        }
    }

    #[test]
    fn placeholders_give_way_to_full_lyrics() {
        // What LRCLIB's exact match for a well-known song once returned.
        let probe = entry(213, Some("[00:00.00]probe"), Some("probe"))
            .found()
            .unwrap();
        assert!(probe.is_thin());
        let verse = "[00:18.00]First line\n[00:22.00]Second line\n[00:26.00]Third line";
        let full = entry(213, Some(verse), None).found().unwrap();
        assert!(!full.is_thin());
        assert_eq!(
            prefer(Some(probe.clone()), Some(full.clone())),
            Some(full.clone())
        );
        assert_eq!(prefer(Some(probe.clone()), None), Some(probe.clone()));

        // Short is not thin: an EDM track may have a hook of a few lines.
        let hook = entry(213, Some("[01:02.00]Hold on\n[01:10.00]Let go"), None)
            .found()
            .unwrap();
        assert!(!hook.is_thin());
        assert_eq!(prefer(Some(hook.clone()), Some(full)), Some(hook));

        // Between two lone lines, or a lone line and "instrumental", the
        // exact match stays.
        let shout = entry(213, None, Some("Tequila!")).found().unwrap();
        assert_eq!(
            prefer(Some(probe.clone()), Some(shout.clone())),
            Some(probe.clone())
        );
        assert_eq!(prefer(None, Some(shout.clone())), Some(shout));
        let no_words = instrumental(213).found().unwrap();
        assert_eq!(
            prefer(Some(probe.clone()), Some(no_words.clone())),
            Some(probe)
        );
        assert_eq!(prefer(None, Some(no_words.clone())), Some(no_words));
    }

    #[test]
    fn search_results_are_picked_by_length_then_fullness() {
        let verse = "[00:01.00]a\n[00:02.00]b\n[00:03.00]c";
        let entries = vec![
            entry(300, Some(verse), None),
            instrumental(213),
            entry(212, Some("[00:00.00]probe"), None),
            entry(211, None, Some("a\nb\nc")),
            entry(215, Some(verse), None),
        ];
        assert_eq!(
            best(entries, 213),
            Some(FoundLyrics {
                synced: Some(verse.to_owned()),
                ..FoundLyrics::default()
            })
        );
        assert_eq!(best(vec![entry(300, Some(verse), None)], 213), None);
        // Words beat "instrumental", even a lone line of them.
        assert_eq!(
            best(vec![instrumental(213), entry(213, None, Some("Hey!"))], 213)
                .map(|found| found.instrumental),
            Some(false)
        );
        assert!(best(vec![instrumental(214)], 213).is_some_and(|found| found.instrumental));
    }
}
