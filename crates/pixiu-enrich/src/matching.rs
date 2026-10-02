//! Deciding whether a MusicBrainz release is a local album, and which of
//! its tracks are which. Pure functions, so the rules are easy to test.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// A track as the hoard knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTrack {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub isrc: Option<String>,
}

/// An album as the hoard knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAlbum {
    pub title: String,
    pub artist: String,
    pub year: Option<i32>,
    /// A MusicBrainz release id the files already name.
    pub mbid: Option<String>,
    pub tracks: Vec<LocalTrack>,
}

/// Who a release or track is credited to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credit {
    /// As printed, e.g. "Artist A feat. Artist B".
    pub name: String,
    /// The credited artists' MusicBrainz ids and names, primary first.
    pub artists: Vec<(String, String)>,
}

/// A release found by a search, before it is looked at closely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub date: Option<String>,
    pub country: Option<String>,
    pub format: Option<String>,
    pub track_count: u32,
    /// How likely it is the album, from 0 to 1.
    pub score: f64,
}

/// A track on a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseTrack {
    pub recording_id: String,
    pub title: String,
    pub artist: Credit,
    pub length_ms: Option<u64>,
    /// Position on its disc, from 1.
    pub position: u32,
    /// Disc number, from 1.
    pub disc: u32,
    pub isrcs: Vec<String>,
}

/// A release with its tracks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub id: String,
    pub title: String,
    pub artist: Credit,
    /// `YYYY`, `YYYY-MM` or `YYYY-MM-DD`.
    pub date: Option<String>,
    pub country: Option<String>,
    pub release_group_id: Option<String>,
    /// The release group's primary type: "Album", "Single", "EP"…
    pub primary_type: Option<String>,
    /// The genre MusicBrainz users voted for most: the release's, else its
    /// release group's.
    pub genre: Option<String>,
    pub has_front_cover: bool,
    pub tracks: Vec<ReleaseTrack>,
}

/// A recording: a song as recorded, whichever releases carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    pub id: String,
    pub title: String,
    pub artist: Credit,
    pub length_ms: Option<u64>,
    pub isrcs: Vec<String>,
    /// When it first came out: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`.
    pub first_released: Option<String>,
    /// The genre MusicBrainz users voted for most, once looked up.
    pub genre: Option<String>,
}

impl Recording {
    /// The year it first came out, when known.
    #[must_use]
    pub fn year(&self) -> Option<i32> {
        self.first_released.as_deref()?.get(..4)?.parse().ok()
    }
}

impl Release {
    /// Whether MusicBrainz files it as a single.
    #[must_use]
    pub fn is_single(&self) -> bool {
        self.primary_type
            .as_deref()
            .is_some_and(|kind| kind.eq_ignore_ascii_case("single"))
    }

    /// The release year, when the date has one.
    #[must_use]
    pub fn year(&self) -> Option<i32> {
        self.date.as_deref()?.get(..4)?.parse().ok()
    }

    /// Condensed, for review.
    #[must_use]
    pub fn candidate(&self, score: f64) -> Candidate {
        Candidate {
            id: self.id.clone(),
            title: self.title.clone(),
            artist: self.artist.name.clone(),
            date: self.date.clone(),
            country: self.country.clone(),
            format: None,
            track_count: u32::try_from(self.tracks.len()).unwrap_or(u32::MAX),
            score,
        }
    }
}

/// How well a release fits an album, and which local track is which.
#[derive(Debug, Clone, PartialEq)]
pub struct Pairing {
    /// From 0 to 1.
    pub confidence: f64,
    /// Local track id, and index into [`Release::tracks`].
    pub tracks: Vec<(u64, usize)>,
    /// Whether every local track found its counterpart.
    pub complete: bool,
}

/// Applied automatically at or above this confidence, when complete.
pub const CERTAIN: f64 = 0.85;

/// Worth showing the admin at or above this confidence.
pub const PLAUSIBLE: f64 = 0.5;

/// Lowercase, `&` as "and", letters, digits and single spaces.
#[must_use]
pub fn normalize(text: &str) -> String {
    let text = text.to_lowercase().replace('&', " and ");
    let kept: String = text
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text without bracketed additions like "(Remastered 2011)" or
/// "[feat. Someone]".
fn without_brackets(text: &str) -> String {
    let mut depth = 0_u32;
    let mut kept = String::new();
    for c in text.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => kept.push(c),
            _ => {}
        }
    }
    kept
}

/// How alike two names are, from 0 to 1, forgiving case, punctuation and
/// bracketed additions.
#[must_use]
pub fn similarity(a: &str, b: &str) -> f64 {
    let full = strsim::jaro_winkler(&normalize(a), &normalize(b));
    let bare = strsim::jaro_winkler(
        &normalize(&without_brackets(a)),
        &normalize(&without_brackets(b)),
    );
    full.max(bare)
}

/// Durations agree within 3 s, or 3% for long tracks.
fn durations_agree(local_ms: u64, remote_ms: Option<u64>) -> bool {
    remote_ms.is_none_or(|remote| local_ms.abs_diff(remote) <= 3_000.max(remote * 3 / 100))
}

fn size_fit(local: usize, release: usize) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let (small, large) = (local.min(release) as f64, local.max(release).max(1) as f64);
    small / large
}

/// How promising a search result is, before looking at its tracks.
#[must_use]
pub fn prescore(
    album: &LocalAlbum,
    title: &str,
    artist: &str,
    track_count: u32,
    search_score: u32,
) -> f64 {
    0.5 * similarity(&album.title, title)
        + 0.3 * similarity(&album.artist, artist)
        + 0.1 * f64::from(search_score.min(100)) / 100.0
        + 0.1 * size_fit(album.tracks.len(), track_count as usize)
}

/// Pairs the album's tracks with the release's, and rates the fit.
#[must_use]
pub fn pair(album: &LocalAlbum, release: &Release) -> Pairing {
    // Every plausible (local, release) pair with its strength.
    let mut options = Vec::new();
    for (local_index, local) in album.tracks.iter().enumerate() {
        for (index, remote) in release.tracks.iter().enumerate() {
            let isrc = local.isrc.as_ref().is_some_and(|isrc| {
                remote
                    .isrcs
                    .iter()
                    .any(|other| other.eq_ignore_ascii_case(isrc))
            });
            let strength = if isrc {
                2.0
            } else if durations_agree(local.duration_ms, remote.length_ms) {
                let title = similarity(&local.title, &remote.title);
                let placed = local.track_number == Some(remote.position)
                    && local.disc_number.unwrap_or(1) == remote.disc;
                if title >= 0.85 || (placed && title >= 0.7) {
                    title + if placed { 0.1 } else { 0.0 }
                } else {
                    continue;
                }
            } else {
                continue;
            };
            options.push((strength, local_index, index));
        }
    }
    // Strongest first; each track takes one partner.
    options.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (mut used_local, mut used_remote) = (HashSet::new(), HashSet::new());
    let mut tracks = Vec::new();
    for (_, local_index, index) in options {
        if used_local.contains(&local_index) || used_remote.contains(&index) {
            continue;
        }
        used_local.insert(local_index);
        used_remote.insert(index);
        tracks.push((album.tracks[local_index].id, index));
    }
    tracks.sort_by_key(|(_, index)| *index);

    let count = album.tracks.len().max(1);
    #[allow(clippy::cast_precision_loss)]
    let matched = tracks.len() as f64 / count as f64;
    let confidence = 0.55 * matched
        + 0.25 * similarity(&album.title, &release.title)
        + 0.15 * similarity(&album.artist, &release.artist.name)
        + 0.05 * size_fit(album.tracks.len(), release.tracks.len());
    Pairing {
        confidence,
        complete: tracks.len() == album.tracks.len(),
        tracks,
    }
}

/// The main artist of a credit like "A feat. B" or "A, B".
fn first_artist(credit: &str) -> &str {
    let lower = credit.to_lowercase();
    let cut = [",", " & ", " feat", " ft.", " x ", " with "]
        .iter()
        .filter_map(|marker| lower.find(marker))
        .min()
        .unwrap_or(credit.len());
    credit.get(..cut).unwrap_or(credit).trim()
}

/// The title without the "Artist - " music videos are often named with,
/// when that artist is the song's.
fn without_artist_prefix<'a>(title: &'a str, artist: &str) -> &'a str {
    match title.split_once(" - ") {
        Some((prefix, rest))
            if !rest.trim().is_empty()
                && similarity(first_artist(prefix), first_artist(artist)) >= 0.85 =>
        {
            rest.trim()
        }
        _ => title,
    }
}

/// The recording `local` is, when one of `candidates` certainly is: the
/// same ISRC and a like title, or a very like title, the same artist and
/// the same length. An ISRC match comes first, then the recording first
/// released earliest.
#[must_use]
pub fn recording_match<'a>(
    local: &LocalTrack,
    candidates: &'a [Recording],
) -> Option<&'a Recording> {
    let same_isrc = |recording: &Recording| {
        local.isrc.as_ref().is_some_and(|isrc| {
            recording
                .isrcs
                .iter()
                .any(|other| other.eq_ignore_ascii_case(isrc))
        })
    };
    let bare_title = without_artist_prefix(&local.title, &local.artist);
    let certain = |recording: &&Recording| {
        let title = similarity(&local.title, &recording.title)
            .max(similarity(bare_title, &recording.title));
        if same_isrc(recording) {
            return title >= 0.6;
        }
        let artist = similarity(&local.artist, &recording.artist.name).max(
            recording.artist.artists.first().map_or(0.0, |(_, name)| {
                similarity(first_artist(&local.artist), name)
            }),
        );
        title >= 0.9
            && artist >= 0.85
            && recording.length_ms.is_some()
            && durations_agree(local.duration_ms, recording.length_ms)
    };
    candidates.iter().filter(certain).min_by_key(|recording| {
        (
            !same_isrc(recording),
            recording.first_released.is_none(),
            recording.first_released.clone(),
        )
    })
}

/// Whether a pairing is good enough to apply without asking.
#[must_use]
pub fn is_certain(pairing: &Pairing) -> bool {
    pairing.complete && pairing.confidence >= CERTAIN
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(id: u64, title: &str, seconds: u64, number: u32) -> LocalTrack {
        LocalTrack {
            id,
            title: title.to_owned(),
            artist: "Kevin MacLeod".to_owned(),
            duration_ms: seconds * 1000,
            track_number: Some(number),
            disc_number: None,
            isrc: None,
        }
    }

    fn remote(title: &str, seconds: u64, position: u32) -> ReleaseTrack {
        ReleaseTrack {
            recording_id: format!("rec-{position}"),
            title: title.to_owned(),
            artist: Credit {
                name: "Kevin MacLeod".to_owned(),
                artists: vec![("art-1".to_owned(), "Kevin MacLeod".to_owned())],
            },
            length_ms: Some(seconds * 1000),
            position,
            disc: 1,
            isrcs: Vec::new(),
        }
    }

    fn august() -> Release {
        Release {
            id: "rel-1".to_owned(),
            title: "The August Album".to_owned(),
            artist: Credit {
                name: "Kevin MacLeod".to_owned(),
                artists: vec![("art-1".to_owned(), "Kevin MacLeod".to_owned())],
            },
            date: Some("2023-08-01".to_owned()),
            country: Some("XW".to_owned()),
            release_group_id: Some("rg-1".to_owned()),
            primary_type: Some("Album".to_owned()),
            genre: None,
            has_front_cover: true,
            tracks: vec![
                remote("Evening", 187, 1),
                remote("Morning", 154, 2),
                remote("Southern Gothic", 148, 3),
                remote("Vibing Over Venus", 412, 4),
            ],
        }
    }

    fn album(tracks: Vec<LocalTrack>) -> LocalAlbum {
        LocalAlbum {
            title: "The August Album".to_owned(),
            artist: "Kevin MacLeod".to_owned(),
            year: Some(2023),
            mbid: None,
            tracks,
        }
    }

    #[test]
    fn names_are_compared_forgivingly() {
        assert!(similarity("Vibing Over Venus", "vibing over venus!") > 0.99);
        assert!(similarity("Song (Remastered 2011)", "Song") > 0.99);
        assert!(similarity("Simon & Garfunkel", "Simon and Garfunkel") > 0.99);
        assert!(similarity("Evening", "Morning") < 0.85);
        assert_eq!(normalize("  Déjà   Vu? "), "déjà vu");
    }

    #[test]
    fn a_whole_album_matches_with_certainty() {
        let pairing = pair(
            &album(vec![
                local(10, "Morning", 155, 2),
                local(11, "Evening", 186, 1),
                local(12, "Southern Gothic", 148, 3),
                local(13, "Vibing over Venus", 411, 4),
            ]),
            &august(),
        );
        assert!(is_certain(&pairing), "{pairing:?}");
        assert_eq!(pairing.tracks, [(11, 0), (10, 1), (12, 2), (13, 3)]);
    }

    #[test]
    fn part_of_an_album_matches_too() {
        let pairing = pair(
            &album(vec![local(20, "Southern Gothic", 149, 3)]),
            &august(),
        );
        assert!(pairing.complete);
        assert!(pairing.confidence >= CERTAIN, "{pairing:?}");
        assert_eq!(pairing.tracks, [(20, 2)]);
    }

    #[test]
    fn different_music_does_not() {
        // Right titles, wrong lengths: another recording.
        let pairing = pair(
            &album(vec![
                local(30, "Evening", 240, 1),
                local(31, "Morning", 60, 2),
            ]),
            &august(),
        );
        assert!(!pairing.complete);
        assert!(!is_certain(&pairing));

        // Another album altogether.
        let mut other = album(vec![local(40, "Evening", 187, 1)]);
        other.title = "Nocturnes".to_owned();
        other.artist = "Chopin".to_owned();
        let pairing = pair(&other, &august());
        assert!(pairing.complete);
        assert!(!is_certain(&pairing), "{pairing:?}");
    }

    #[test]
    fn isrcs_settle_it() {
        let mut track = local(50, "Totally Different Title", 999, 9);
        track.isrc = Some("usrc17607839".to_owned());
        let mut release = august();
        release.tracks[3].isrcs = vec!["USRC17607839".to_owned()];
        let pairing = pair(&album(vec![track]), &release);
        assert_eq!(pairing.tracks, [(50, 3)]);
    }

    #[test]
    fn search_results_are_ranked() {
        let wanted = album(vec![local(1, "Evening", 187, 1)]);
        let right = prescore(&wanted, "The August Album", "Kevin MacLeod", 4, 100);
        let wrong = prescore(&wanted, "Greatest Hits", "Kevin MacLeod", 40, 90);
        assert!(right > wrong);
        assert_eq!(august().year(), Some(2023));
    }

    fn song(title: &str, artist: &str, seconds: u64, isrc: Option<&str>) -> LocalTrack {
        LocalTrack {
            id: 1,
            title: title.to_owned(),
            artist: artist.to_owned(),
            duration_ms: seconds * 1000,
            track_number: None,
            disc_number: None,
            isrc: isrc.map(str::to_owned),
        }
    }

    fn recording(id: &str, title: &str, artist: &str, seconds: u64, isrc: &str) -> Recording {
        Recording {
            id: id.to_owned(),
            title: title.to_owned(),
            artist: Credit {
                name: artist.to_owned(),
                artists: vec![(format!("art-{id}"), artist.to_owned())],
            },
            length_ms: Some(seconds * 1000),
            isrcs: vec![isrc.to_owned()],
            first_released: None,
            genre: None,
        }
    }

    #[test]
    fn recordings_are_matched_by_isrc_first() {
        let candidates = [
            recording("a", "Dawn Chorus", "Main Artist", 211, "ZZ1"),
            recording("b", "Dawn Chorus (Radio Edit)", "Main Artist", 180, "ZZ2"),
        ];
        let local = song("Dawn Chorus", "Main Artist", 211, Some("zz2"));
        assert_eq!(recording_match(&local, &candidates).unwrap().id, "b");
        // A wrong ISRC on a different song is not enough.
        let other = [recording(
            "c",
            "Something Else Entirely",
            "Main Artist",
            211,
            "ZZ2",
        )];
        assert_eq!(recording_match(&local, &other), None);
    }

    #[test]
    fn recordings_are_matched_by_name_artist_and_length() {
        let candidates = [recording("a", "Dawn Chorus", "Main Artist", 212, "ZZ1")];
        for local in [
            song("Dawn Chorus", "Main Artist", 211, None),
            song("Dawn chorus", "Main Artist, Guest", 210, None),
            song("Dawn Chorus", "Main Artist feat. Guest", 213, Some("XX9")),
            // As music videos are named.
            song("Main Artist - Dawn Chorus", "Main Artist", 211, None),
            song(
                "Main Artist & Guest - Dawn Chorus",
                "Main Artist",
                211,
                None,
            ),
        ] {
            assert_eq!(
                recording_match(&local, &candidates).map(|r| r.id.as_str()),
                Some("a"),
                "{local:?}"
            );
        }
        for local in [
            song("Dawn Chorus", "Main Artist", 240, None),
            song("Dusk Chorus", "Main Artist", 211, None),
            song("Dawn Chorus", "Somebody Else", 211, None),
            song("Somebody Else - Dawn Chorus", "Main Artist", 211, None),
        ] {
            assert_eq!(recording_match(&local, &candidates), None, "{local:?}");
        }
    }

    #[test]
    fn the_first_release_wins_among_equals() {
        let mut early = recording("early", "Dawn Chorus", "Main Artist", 211, "ZZ1");
        early.first_released = Some("2019-01-01".to_owned());
        let mut late = early.clone();
        late.id = "late".to_owned();
        late.first_released = Some("2021-01-01".to_owned());
        let mut undated = early.clone();
        undated.id = "undated".to_owned();
        undated.first_released = None;
        let local = song("Dawn Chorus", "Main Artist", 211, Some("ZZ1"));
        assert_eq!(
            recording_match(&local, &[undated, late, early]).unwrap().id,
            "early"
        );
    }
}
