//! The MusicBrainz web service (`/ws/2`), as far as píxiū needs it.
//!
//! MusicBrainz asks for at most one request per second and a User-Agent
//! naming the application and a way to reach its operator; see
//! <https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting>.

use std::time::{Duration, Instant};

use reqwest::{StatusCode, Url};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    EnrichError,
    matching::{Candidate, Credit, LocalAlbum, Release, ReleaseTrack, prescore},
};

const API: &str = "https://musicbrainz.org/ws/2/";

/// Spaces requests at least a second apart, across all callers.
#[derive(Debug, Default)]
pub(crate) struct Pace {
    last: Mutex<Option<Instant>>,
}

impl Pace {
    pub(crate) async fn wait(&self, gap: Duration) {
        let mut last = self.last.lock().await;
        if let Some(previous) = *last {
            let ready = previous + gap;
            if let Some(remaining) = ready.checked_duration_since(Instant::now()) {
                tokio::time::sleep(remaining).await;
            }
        }
        *last = Some(Instant::now());
    }
}

#[derive(Debug)]
pub(crate) struct MusicBrainz {
    http: reqwest::Client,
    pace: Pace,
}

/// A title's words, free of Lucene syntax, for an unquoted term.
fn words(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Escapes a phrase for a quoted Lucene term.
fn phrase(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[derive(Deserialize)]
struct ArtistCreditJson {
    name: String,
    #[serde(default)]
    joinphrase: String,
    artist: ArtistRefJson,
}

#[derive(Deserialize)]
struct ArtistRefJson {
    id: String,
    name: String,
}

fn credit(parts: &[ArtistCreditJson]) -> Credit {
    Credit {
        name: parts
            .iter()
            .map(|part| format!("{}{}", part.name, part.joinphrase))
            .collect(),
        artists: parts
            .iter()
            .map(|part| (part.artist.id.clone(), part.artist.name.clone()))
            .collect(),
    }
}

#[derive(Deserialize)]
struct SearchJson {
    #[serde(default)]
    releases: Vec<SearchedReleaseJson>,
}

#[derive(Deserialize)]
struct SearchedReleaseJson {
    id: String,
    #[serde(default)]
    score: u32,
    title: String,
    date: Option<String>,
    country: Option<String>,
    #[serde(rename = "track-count", default)]
    track_count: u32,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<ArtistCreditJson>,
    #[serde(default)]
    media: Vec<SearchedMediumJson>,
}

#[derive(Deserialize)]
struct SearchedMediumJson {
    format: Option<String>,
}

#[derive(Deserialize)]
struct ReleaseJson {
    id: String,
    title: String,
    date: Option<String>,
    country: Option<String>,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<ArtistCreditJson>,
    #[serde(rename = "release-group")]
    release_group: Option<ReleaseGroupJson>,
    #[serde(default)]
    genres: Vec<GenreJson>,
    #[serde(rename = "cover-art-archive")]
    cover_art_archive: Option<CoverArtJson>,
    #[serde(default)]
    media: Vec<MediumJson>,
}

#[derive(Deserialize)]
struct ReleaseGroupJson {
    id: String,
    #[serde(default)]
    genres: Vec<GenreJson>,
}

#[derive(Deserialize)]
struct GenreJson {
    name: String,
    #[serde(default)]
    count: u32,
}

/// The genre with the most votes, named as a tag would name it: "pop
/// rock" is "Pop Rock". Ties go to the first by name.
fn top_genre(genres: &[GenreJson]) -> Option<String> {
    genres
        .iter()
        .filter(|genre| genre.count > 0 && !genre.name.trim().is_empty())
        .min_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)))
        .map(|genre| genre_name(&genre.name))
}

/// A genre as MusicBrainz names it (lowercase), named as a tag would: each
/// word capitalized, hyphenated parts too ("J-Pop"), small words left alone
/// ("Drum and Bass"), initials in capitals ("EDM", "R&B").
#[must_use]
pub fn genre_name(name: &str) -> String {
    name.split(' ')
        .enumerate()
        .map(|(index, word)| {
            if index > 0 && SMALL_WORDS.contains(&word) {
                return word.to_owned();
            }
            word.split('-')
                .map(|part| {
                    if part.contains('&') || INITIALS.contains(&part) {
                        part.to_uppercase()
                    } else {
                        let mut chars = part.chars();
                        chars.next().map_or_else(String::new, |first| {
                            first.to_uppercase().chain(chars).collect()
                        })
                    }
                })
                .collect::<Vec<_>>()
                .join("-")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Words a genre's name keeps in lowercase, unless they start it.
const SMALL_WORDS: &[&str] = &["and", "n", "of", "the", "in", "to", "a", "de", "y"];

/// Genres' initials, written in capitals.
const INITIALS: &[&str] = &[
    "aor", "bgm", "ebm", "edm", "idm", "mpb", "nwobhm", "nyhc", "rnb", "uk", "us", "usa",
];

#[derive(Deserialize)]
struct CoverArtJson {
    #[serde(default)]
    front: bool,
}

#[derive(Deserialize)]
struct MediumJson {
    #[serde(default = "first")]
    position: u32,
    #[serde(default)]
    tracks: Vec<TrackJson>,
}

fn first() -> u32 {
    1
}

#[derive(Deserialize)]
struct TrackJson {
    position: u32,
    title: String,
    length: Option<u64>,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<ArtistCreditJson>,
    recording: RecordingJson,
}

#[derive(Deserialize)]
struct RecordingJson {
    id: String,
    #[serde(default)]
    isrcs: Vec<String>,
}

#[derive(Deserialize)]
struct ArtistJson {
    #[serde(default)]
    relations: Vec<RelationJson>,
}

#[derive(Deserialize)]
struct RelationJson {
    #[serde(rename = "type")]
    kind: String,
    url: Option<UrlJson>,
}

#[derive(Deserialize)]
struct UrlJson {
    resource: String,
}

impl From<ReleaseJson> for Release {
    fn from(json: ReleaseJson) -> Self {
        let release_credit = credit(&json.artist_credit);
        let tracks = json
            .media
            .into_iter()
            .flat_map(|medium| {
                let disc = medium.position;
                let fallback = release_credit.clone();
                medium.tracks.into_iter().map(move |track| ReleaseTrack {
                    recording_id: track.recording.id,
                    title: track.title,
                    artist: if track.artist_credit.is_empty() {
                        fallback.clone()
                    } else {
                        credit(&track.artist_credit)
                    },
                    length_ms: track.length,
                    position: track.position,
                    disc,
                    isrcs: track.recording.isrcs,
                })
            })
            .collect();
        let genre = top_genre(&json.genres).or_else(|| {
            json.release_group
                .as_ref()
                .and_then(|group| top_genre(&group.genres))
        });
        Self {
            id: json.id,
            title: json.title,
            artist: release_credit,
            date: json.date.filter(|date| !date.is_empty()),
            country: json.country,
            release_group_id: json.release_group.map(|group| group.id),
            genre,
            has_front_cover: json.cover_art_archive.is_some_and(|art| art.front),
            tracks,
        }
    }
}

impl MusicBrainz {
    pub(crate) fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            pace: Pace::default(),
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, url: Url) -> Result<T, EnrichError> {
        for attempt in 0..3_u32 {
            self.pace.wait(Duration::from_secs(1)).await;
            let response = self.http.get(url.clone()).send().await?;
            match response.status() {
                // Throttled: wait a little longer and try again.
                StatusCode::SERVICE_UNAVAILABLE | StatusCode::TOO_MANY_REQUESTS if attempt < 2 => {
                    tokio::time::sleep(Duration::from_secs(2 * u64::from(attempt + 1))).await;
                }
                StatusCode::NOT_FOUND => return Err(EnrichError::NotFound),
                _ => return Ok(response.error_for_status()?.json().await?),
            }
        }
        Err(EnrichError::Busy)
    }

    fn url(path: &str, query: &[(&str, &str)]) -> Url {
        let mut url = Url::parse(API)
            .expect("a valid base")
            .join(path)
            .expect("a valid path");
        url.query_pairs_mut()
            .extend_pairs(query)
            .append_pair("fmt", "json");
        url
    }

    /// Releases that might be `album`, most promising first.
    pub(crate) async fn search(&self, album: &LocalAlbum) -> Result<Vec<Candidate>, EnrichError> {
        let strict = format!(
            "release:{} AND artist:{}",
            phrase(&album.title),
            phrase(&album.artist)
        );
        let mut found: SearchJson = self
            .get(Self::url("release", &[("query", &strict), ("limit", "10")]))
            .await?;
        if found.releases.is_empty() {
            // Looser: any of the title's words, still by the artist.
            let loose = format!(
                "release:({}) AND artist:{}",
                words(&album.title),
                phrase(&album.artist)
            );
            found = self
                .get(Self::url("release", &[("query", &loose), ("limit", "10")]))
                .await?;
        }
        let mut candidates: Vec<Candidate> = found
            .releases
            .into_iter()
            .map(|release| {
                let artist = credit(&release.artist_credit).name;
                let score = prescore(
                    album,
                    &release.title,
                    &artist,
                    release.track_count,
                    release.score,
                );
                Candidate {
                    format: release
                        .media
                        .first()
                        .and_then(|medium| medium.format.clone()),
                    id: release.id,
                    title: release.title,
                    artist,
                    date: release.date,
                    country: release.country,
                    track_count: release.track_count,
                    score,
                }
            })
            .collect();
        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        Ok(candidates)
    }

    /// A release with its tracks.
    pub(crate) async fn release(&self, id: &str) -> Result<Release, EnrichError> {
        let json: ReleaseJson = self
            .get(Self::url(
                &format!("release/{id}"),
                &[(
                    "inc",
                    "recordings+artist-credits+isrcs+release-groups+genres",
                )],
            ))
            .await?;
        Ok(json.into())
    }

    /// An artist's Wikidata item id (`Q…`), when MusicBrainz links one.
    pub(crate) async fn wikidata(&self, artist_id: &str) -> Result<Option<String>, EnrichError> {
        let json: ArtistJson = self
            .get(Self::url(
                &format!("artist/{artist_id}"),
                &[("inc", "url-rels")],
            ))
            .await?;
        Ok(json
            .relations
            .into_iter()
            .filter(|relation| relation.kind == "wikidata")
            .find_map(|relation| {
                let resource = relation.url?.resource;
                resource
                    .rsplit('/')
                    .next()
                    .filter(|id| id.starts_with('Q'))
                    .map(str::to_owned)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_are_escaped() {
        assert_eq!(phrase(r#"Say "Hi" \o/"#), r#""Say \"Hi\" \\o/""#);
        assert_eq!(words("Magic: Escape (Room!) ~ 2"), "Magic Escape Room 2");
    }

    #[test]
    fn releases_are_read() {
        let json: ReleaseJson =
            serde_json::from_str(include_str!("../fixtures/release.json")).unwrap();
        let release = Release::from(json);
        assert_eq!(release.title, "The August Album");
        assert_eq!(release.artist.name, "Kevin MacLeod");
        assert_eq!(release.year(), Some(2023));
        assert!(release.has_front_cover);
        assert_eq!(release.release_group_id.as_deref(), Some("rg-august"));
        assert_eq!(release.tracks.len(), 2);
        let second = &release.tracks[1];
        assert_eq!((second.position, second.disc), (2, 1));
        assert_eq!(second.length_ms, Some(154_000));
        assert_eq!(second.isrcs, ["QZTB82300002"]);
        assert_eq!(second.artist.name, "Kevin MacLeod feat. Somebody");
        assert_eq!(second.artist.artists.len(), 2);
        // The release has none: its release group's, most voted first.
        assert_eq!(release.genre.as_deref(), Some("Electronic"));
    }

    #[test]
    fn a_release_s_own_genres_come_first() {
        let genres = |names: &[(&str, u32)]| -> Vec<GenreJson> {
            names
                .iter()
                .map(|&(name, count)| GenreJson {
                    name: name.to_owned(),
                    count,
                })
                .collect()
        };
        let json = ReleaseJson {
            genres: genres(&[("pop rock", 1)]),
            release_group: Some(ReleaseGroupJson {
                id: "rg".to_owned(),
                genres: genres(&[("rock", 9)]),
            }),
            ..serde_json::from_str(include_str!("../fixtures/release.json")).unwrap()
        };
        assert_eq!(Release::from(json).genre.as_deref(), Some("Pop Rock"));
    }

    #[test]
    fn genres_are_named_like_tags() {
        assert_eq!(genre_name("drum and bass"), "Drum and Bass");
        assert_eq!(genre_name("r&b"), "R&B");
        assert_eq!(genre_name("j-pop"), "J-Pop");
        assert_eq!(genre_name("dance-punk revival"), "Dance-Punk Revival");
        assert_eq!(genre_name("edm"), "EDM");
        assert_eq!(genre_name("uk garage"), "UK Garage");
        assert_eq!(genre_name("the blues"), "The Blues");
        assert_eq!(top_genre(&[]), None);
    }
}
