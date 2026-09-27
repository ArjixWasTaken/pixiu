//! Loading the library and rendering it as Subsonic elements.
//!
//! Lists and searches select ids with raw SQL (ordering by random, joins,
//! multi-word matching), then load the models in bulk.

use std::collections::{HashMap, HashSet};

use pixiu_db::{
    Album, Annotation, Artist, Db, Track,
    toasty::{self, stmt::Value},
};

use crate::{annotations::with_plays, ids, response::Element};

/// A bound value for raw SQL.
pub(crate) enum Bind {
    Int(i64),
    Text(String),
}

/// Runs `sql`, whose first column is an id, returning the ids in order.
pub(crate) async fn select_ids(
    db: &mut Db,
    sql: &str,
    binds: Vec<Bind>,
) -> Result<Vec<u64>, toasty::Error> {
    let mut query = toasty::sql::query(sql);
    for bind in binds {
        query = match bind {
            Bind::Int(value) => query.bind(value),
            Bind::Text(value) => query.bind(value),
        };
    }
    let rows = query.exec(db).await?;
    Ok(rows
        .iter()
        .filter_map(|row| match row {
            Value::Record(record) => as_u64(&record[0]),
            _ => None,
        })
        .collect())
}

/// Runs `sql` returning `(u64, u64)` pairs, e.g. grouped counts.
pub(crate) async fn select_pairs(
    db: &mut Db,
    sql: &str,
) -> Result<HashMap<u64, (u64, u64)>, toasty::Error> {
    let rows = toasty::sql::query(sql).exec(db).await?;
    Ok(rows
        .iter()
        .filter_map(|row| match row {
            Value::Record(record) => Some((
                as_u64(&record[0])?,
                (
                    as_u64(&record[1]).unwrap_or(0),
                    as_u64(&record[2]).unwrap_or(0),
                ),
            )),
            _ => None,
        })
        .collect())
}

pub(crate) fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::I64(n) => u64::try_from(*n).ok(),
        Value::I32(n) => u64::try_from(*n).ok(),
        Value::U64(n) => Some(*n),
        Value::U32(n) => Some(u64::from(*n)),
        _ => None,
    }
}

/// Song count and total duration (ms) of every album.
pub(crate) async fn album_stats(db: &mut Db) -> Result<HashMap<u64, (u64, u64)>, toasty::Error> {
    select_pairs(
        db,
        "SELECT album_id, COUNT(*), COALESCE(SUM(duration_ms), 0) FROM tracks GROUP BY album_id",
    )
    .await
}

pub(crate) async fn artists_by_id(
    db: &mut Db,
    ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Artist>, toasty::Error> {
    let ids: Vec<u64> = ids
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(Artist::filter(Artist::fields().id().in_list(ids))
        .exec(db)
        .await?
        .into_iter()
        .map(|artist| (artist.id, artist))
        .collect())
}

pub(crate) async fn albums_by_id(
    db: &mut Db,
    ids: impl IntoIterator<Item = u64>,
) -> Result<HashMap<u64, Album>, toasty::Error> {
    let ids: Vec<u64> = ids
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(Album::filter(Album::fields().id().in_list(ids))
        .exec(db)
        .await?
        .into_iter()
        .map(|album| (album.id, album))
        .collect())
}

/// Loads albums by id, keeping the order of `ids`.
pub(crate) async fn albums_in_order(db: &mut Db, ids: &[u64]) -> Result<Vec<Album>, toasty::Error> {
    let mut by_id = albums_by_id(db, ids.iter().copied()).await?;
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// Loads tracks by id, keeping the order of `ids`.
pub(crate) async fn tracks_in_order(db: &mut Db, ids: &[u64]) -> Result<Vec<Track>, toasty::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_id: HashMap<u64, Track> = Track::filter(Track::fields().id().in_list(ids.to_vec()))
        .exec(db)
        .await?
        .into_iter()
        .map(|track| (track.id, track))
        .collect();
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// Loads artists by id, keeping the order of `ids`.
pub(crate) async fn artists_in_order(
    db: &mut Db,
    ids: &[u64],
) -> Result<Vec<Artist>, toasty::Error> {
    let mut by_id = artists_by_id(db, ids.iter().copied()).await?;
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

/// Renders songs, loading the albums and artists they refer to.
pub(crate) async fn songs(
    db: &mut Db,
    name: &'static str,
    tracks: &[Track],
) -> Result<Vec<Element>, toasty::Error> {
    let albums = albums_by_id(db, tracks.iter().map(|track| track.album_id)).await?;
    let artists = artists_by_id(
        db,
        tracks
            .iter()
            .map(|track| track.artist_id)
            .chain(albums.values().map(|album| album.artist_id)),
    )
    .await?;
    let plays = crate::annotations::for_tracks(db, tracks.iter().map(|track| track.id)).await?;
    Ok(tracks
        .iter()
        .map(|track| {
            let album = albums.get(&track.album_id);
            let album_artist = album.and_then(|album| artists.get(&album.artist_id));
            with_plays(
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
        .attr("path", track.path.as_str())
        .attr_opt("discNumber", track.disc_number)
        .attr("created", track.added_at.to_string())
        .attr("albumId", ids::album(track.album_id))
        .attr("artistId", ids::artist(track.artist_id))
        .attr("type", "music")
        .attr("mediaType", "song")
        .attr("isVideo", false)
        .attr_opt("musicBrainzId", track.mbid.as_deref())
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
        );
    with_plays(element, plays)
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
        .attr("mediaType", "album");
    with_plays(element, plays)
}

/// An artist in the ID3 model (`ArtistID3`).
pub(crate) fn artist_id3(
    name: &'static str,
    artist: &Artist,
    album_count: u64,
    has_cover: bool,
) -> Element {
    Element::new(name)
        .attr("id", ids::artist(artist.id))
        .attr("name", artist.name.as_str())
        .attr_opt("coverArt", has_cover.then(|| ids::artist(artist.id)))
        .attr("albumCount", album_count)
        .attr_opt("musicBrainzId", artist.mbid.as_deref())
        .attr("sortName", artist.name.as_str())
        .attr("mediaType", "artist")
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
