//! Loading the library and rendering it as Subsonic elements.
//!
//! Lists and searches select ids with raw SQL (ordering by random, joins,
//! multi-word matching), then load the models in bulk. Everything goes
//! through the caller's [`Library`], so it only ever sees their own.

use std::collections::{HashMap, HashSet};

use pixiu_db::{Album, Annotation, Artist, Library, SourceKey, Track, owned::as_u64, toasty};

use crate::{annotations::annotate, ids, response::Element};

/// Song count and total duration (ms) of every album in the library.
pub(crate) async fn album_stats(lib: &Library) -> Result<HashMap<u64, (u64, u64)>, toasty::Error> {
    let rows = lib
        .sql(
            "SELECT album_id, COUNT(*), COALESCE(SUM(duration_ms), 0) FROM tracks \
             WHERE tracks.user_id = ?1 GROUP BY album_id",
        )
        .rows(&mut lib.db())
        .await?;
    Ok(rows
        .iter()
        .filter_map(|row| {
            Some((
                as_u64(row.first()?)?,
                (
                    row.get(1).and_then(as_u64).unwrap_or(0),
                    row.get(2).and_then(as_u64).unwrap_or(0),
                ),
            ))
        })
        .collect())
}

fn unique(ids: impl IntoIterator<Item = u64>) -> Vec<u64> {
    ids.into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// The library's artists among `ids`, by id.
pub(crate) async fn artists_by_id(
    lib: &Library,
    ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Artist>, toasty::Error> {
    Ok(lib
        .artists(&unique(ids))
        .await?
        .into_iter()
        .map(|artist| (artist.id, artist))
        .collect())
}

/// The library's albums among `ids`, by id.
pub(crate) async fn albums_by_id(
    lib: &Library,
    ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Album>, toasty::Error> {
    Ok(lib
        .albums(&unique(ids))
        .await?
        .into_iter()
        .map(|album| (album.id, album))
        .collect())
}

/// Loads the library's albums by id, keeping the order of `ids`.
pub(crate) async fn albums_in_order(
    lib: &Library,
    ids: &[u64],
) -> Result<Vec<Album>, toasty::Error> {
    let mut by_id = albums_by_id(lib, ids.iter().copied()).await?;
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// Loads the library's tracks by id, keeping the order of `ids`.
pub(crate) async fn tracks_in_order(
    lib: &Library,
    ids: &[u64],
) -> Result<Vec<Track>, toasty::Error> {
    let mut by_id = lib.tracks_by_id(ids).await?;
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// Loads the library's artists by id, keeping the order of `ids`.
pub(crate) async fn artists_in_order(
    lib: &Library,
    ids: &[u64],
) -> Result<Vec<Artist>, toasty::Error> {
    let mut by_id = artists_by_id(lib, ids.iter().copied()).await?;
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// Orders tracks as on the album: disc, then track number, then title.
pub(crate) fn sort_album_tracks(tracks: &mut [Track]) {
    tracks.sort_by(|a, b| {
        (
            a.disc_number.unwrap_or(1),
            a.track_number.unwrap_or(u32::MAX),
            &a.title,
        )
            .cmp(&(
                b.disc_number.unwrap_or(1),
                b.track_number.unwrap_or(u32::MAX),
                &b.title,
            ))
    });
}

fn seconds(duration_ms: u64) -> u64 {
    (duration_ms + 500) / 1000
}

fn artist_ref(name: &'static str, artist: &Artist) -> Element {
    Element::new(name)
        .attr("id", ids::artist(artist.id))
        .attr("name", artist.name.as_str())
}

/// Renders songs of the library, loading the albums and artists they
/// refer to.
pub(crate) async fn songs(
    lib: &Library,
    name: &'static str,
    tracks: &[Track],
) -> Result<Vec<Element>, toasty::Error> {
    let albums = albums_by_id(lib, tracks.iter().map(|track| track.album_id)).await?;
    let artists = artists_by_id(
        lib,
        tracks
            .iter()
            .map(|track| track.artist_id)
            .chain(albums.values().map(|album| album.artist_id)),
    )
    .await?;
    let plays = crate::annotations::for_tracks(lib, tracks.iter().map(|track| track.id)).await?;
    Ok(tracks
        .iter()
        .map(|track| {
            let album = albums.get(&track.album_id);
            let album_artist = album.and_then(|album| artists.get(&album.artist_id));
            annotate(
                song(
                    name,
                    track,
                    album,
                    album_artist,
                    artists.get(&track.artist_id),
                ),
                plays.get(&track.id),
            )
        })
        .collect())
}

/// Where the song would sit in a folder tree, for apps that show paths:
/// files live in the content-addressed store, named by their content.
fn display_path(track: &Track, album: Option<&Album>, album_artist: Option<&Artist>) -> String {
    pixiu_treasury::layout::track_path(pixiu_treasury::layout::TrackLocation {
        album_artist: album_artist
            .map_or(track.artist_credit.as_str(), |artist| artist.name.as_str()),
        artist: &track.artist_credit,
        album: album.map_or("Unknown Album", |album| album.title.as_str()),
        year: track.year.or(album.and_then(|album| album.year)),
        genre: track.genre.as_deref(),
        disc: track.disc_number,
        track: track.track_number,
        title: &track.title,
        suffix: &track.suffix,
    })
    .to_string_lossy()
    .into_owned()
}

/// The platform a song or album was downloaded from (`sourcePlatform`, a
/// píxiū extension that apps ignore); none for uploads.
fn platform_of(stored: Option<&str>) -> Option<&'static str> {
    SourceKey::from_stored(stored).map(|key| key.platform().as_str())
}

/// A song (the Subsonic `Child` type).
pub(crate) fn song(
    name: &'static str,
    track: &Track,
    album: Option<&Album>,
    album_artist: Option<&Artist>,
    artist: Option<&Artist>,
) -> Element {
    Element::new(name)
        .attr("id", ids::track(track.id))
        .attr("parent", ids::album(track.album_id))
        .attr("isDir", false)
        .attr("title", track.title.as_str())
        .attr_opt("album", album.map(|album| album.title.as_str()))
        .attr("artist", track.artist_credit.as_str())
        .attr_opt("track", track.track_number)
        .attr_opt("year", track.year.or(album.and_then(|album| album.year)))
        .attr_opt("genre", track.genre.as_deref())
        .attr_opt(
            "coverArt",
            album
                .filter(|album| album.cover.is_some())
                .map(|album| ids::album(album.id)),
        )
        .attr("size", track.size)
        .attr("contentType", track.content_type.as_str())
        .attr("suffix", track.suffix.as_str())
        .attr("duration", seconds(track.duration_ms))
        .attr_opt("bitRate", track.bitrate)
        .attr_opt("bitDepth", track.bit_depth)
        .attr_opt("samplingRate", track.sample_rate)
        .attr_opt("channelCount", track.channels)
        .attr("path", display_path(track, album, album_artist))
        .attr_opt("discNumber", track.disc_number)
        .attr("created", track.added_at.to_string())
        .attr("albumId", ids::album(track.album_id))
        .attr("artistId", ids::artist(track.artist_id))
        .attr("type", "music")
        .attr("mediaType", "song")
        .attr("isVideo", false)
        .attr_opt("musicBrainzId", track.mbid.as_deref())
        .attr_opt("sourcePlatform", platform_of(track.source_key.as_deref()))
        // píxiū's own: a single's song, which its web player shows on its own.
        .attr_opt("single", album.filter(|album| album.single).map(|_| true))
        .attr("displayArtist", track.artist_credit.as_str())
        .attr_opt(
            "displayAlbumArtist",
            album_artist.map(|artist| artist.name.as_str()),
        )
        .values("isrc", track.isrc.iter().map(String::as_str))
        .list(
            "genres",
            track
                .genre
                .iter()
                .map(|genre| Element::new("genres").attr("name", genre.as_str())),
        )
        .list(
            "artists",
            artist.map(|artist| artist_ref("artists", artist)),
        )
        .list(
            "albumArtists",
            album_artist.map(|artist| artist_ref("albumArtists", artist)),
        )
}

/// An album in the ID3 model (`AlbumID3`).
pub(crate) fn album_id3(
    name: &'static str,
    album: &Album,
    artist: Option<&Artist>,
    (song_count, duration_ms): (u64, u64),
    plays: Option<&Annotation>,
) -> Element {
    let element = Element::new(name)
        .attr("id", ids::album(album.id))
        .attr("name", album.title.as_str())
        .attr_opt("artist", artist.map(|artist| artist.name.as_str()))
        .attr("artistId", ids::artist(album.artist_id))
        .attr_opt(
            "coverArt",
            album.cover.as_ref().map(|_| ids::album(album.id)),
        )
        .attr("songCount", song_count)
        .attr("duration", seconds(duration_ms))
        .attr("created", album.created_at.to_string())
        .attr_opt("year", album.year)
        .attr_opt("genre", album.genre.as_deref())
        .attr_opt("musicBrainzId", album.mbid.as_deref())
        .attr_opt("sourcePlatform", platform_of(album.source_key.as_deref()))
        .attr_opt("displayArtist", artist.map(|artist| artist.name.as_str()))
        .attr("sortName", album.title.as_str())
        .attr("isCompilation", false)
        .attr("mediaType", "album")
        .list(
            "genres",
            album
                .genre
                .iter()
                .map(|genre| Element::new("genres").attr("name", genre.as_str())),
        )
        .list(
            "artists",
            artist.map(|artist| artist_ref("artists", artist)),
        )
        // OpenSubsonic: apps that know release types file singles apart.
        .values("releaseTypes", album.single.then_some("Single"));
    annotate(element, plays)
}

/// An album as a folder (the `Child` type with `isDir`), for the
/// folder-based endpoints.
pub(crate) fn album_child(
    name: &'static str,
    album: &Album,
    artist: Option<&Artist>,
    (song_count, duration_ms): (u64, u64),
    plays: Option<&Annotation>,
) -> Element {
    let element = Element::new(name)
        .attr("id", ids::album(album.id))
        .attr("parent", ids::artist(album.artist_id))
        .attr("isDir", true)
        .attr("title", album.title.as_str())
        .attr("album", album.title.as_str())
        .attr_opt("artist", artist.map(|artist| artist.name.as_str()))
        .attr_opt("year", album.year)
        .attr_opt("genre", album.genre.as_deref())
        .attr_opt(
            "coverArt",
            album.cover.as_ref().map(|_| ids::album(album.id)),
        )
        .attr("songCount", song_count)
        .attr("duration", seconds(duration_ms))
        .attr("created", album.created_at.to_string())
        .attr("albumId", ids::album(album.id))
        .attr("artistId", ids::artist(album.artist_id))
        .attr_opt("sourcePlatform", platform_of(album.source_key.as_deref()))
        .attr_opt("single", album.single.then_some(true))
        .attr("mediaType", "album");
    annotate(element, plays)
}

/// An artist in the ID3 model (`ArtistID3`).
pub(crate) fn artist_id3(
    name: &'static str,
    artist: &Artist,
    album_count: u64,
    has_cover: bool,
    annotation: Option<&Annotation>,
) -> Element {
    let element = Element::new(name)
        .attr("id", ids::artist(artist.id))
        .attr("name", artist.name.as_str())
        .attr_opt(
            "coverArt",
            (has_cover || artist.image.is_some()).then(|| ids::artist(artist.id)),
        )
        .attr("albumCount", album_count)
        .attr_opt("musicBrainzId", artist.mbid.as_deref())
        .attr("sortName", artist.name.as_str())
        .attr("mediaType", "artist");
    annotate(element, annotation)
}

/// An artist in the folder model (`Artist`).
pub(crate) fn artist_folder(
    name: &'static str,
    artist: &Artist,
    annotation: Option<&Annotation>,
) -> Element {
    let element = Element::new(name)
        .attr("id", ids::artist(artist.id))
        .attr("name", artist.name.as_str());
    annotate(element, annotation)
}

/// Album counts and cover availability per album artist.
pub(crate) struct ArtistSummary {
    pub albums: u64,
    pub has_cover: bool,
}

pub(crate) fn summarize_artists(albums: &[Album]) -> HashMap<u64, ArtistSummary> {
    let mut summaries: HashMap<u64, ArtistSummary> = HashMap::new();
    for album in albums {
        let summary = summaries.entry(album.artist_id).or_insert(ArtistSummary {
            albums: 0,
            has_cover: false,
        });
        summary.albums += 1;
        summary.has_cover |= album.cover.is_some();
    }
    summaries
}
