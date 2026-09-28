//! Browsing endpoints: music folders, artist indexes, artists, albums, songs,
//! directories and genres.

use std::collections::BTreeMap;

use pixiu_db::{
    Album, Artist, Track,
    toasty::{self, stmt::Value},
};

use crate::{
    Failure, Params, SubsonicState, annotations,
    catalog::{self, as_u64},
    ids::{self, Id},
    response::{ApiError, Element, ErrorCode, Payload},
};

/// Leading words ignored when sorting and indexing artists.
const IGNORED_ARTICLES: &[&str] = &["The", "El", "La", "Los", "Las", "Le", "Les"];

/// píxiū has one music folder: the treasure.
pub(crate) const MUSIC_FOLDER_ID: u32 = 1;

pub(crate) fn not_found(what: &str) -> Failure {
    ApiError::new(ErrorCode::NotFound, format!("{what} not found")).into()
}

pub(crate) fn music_folders() -> Payload {
    Element::new("musicFolders")
        .list(
            "musicFolder",
            [Element::new("musicFolder")
                .attr("id", MUSIC_FOLDER_ID)
                .attr("name", "Treasure")],
        )
        .into()
}

/// The sort key of an artist name: without a leading article, lowercase.
fn sort_key(name: &str) -> String {
    let trimmed = IGNORED_ARTICLES
        .iter()
        .find_map(|article| {
            let (head, rest) = name.split_at_checked(article.len())?;
            (head.eq_ignore_ascii_case(article) && rest.starts_with(' ')).then(|| rest.trim_start())
        })
        .unwrap_or(name);
    trimmed.to_lowercase()
}

/// The index an artist is listed under: its first letter, or `#`.
fn index_name(name: &str) -> String {
    match sort_key(name).chars().next() {
        Some(c) if c.is_alphabetic() => c.to_uppercase().collect(),
        _ => "#".to_owned(),
    }
}

/// `getIndexes` (folder model) and `getArtists` (ID3 model).
pub(crate) async fn artists(state: &SubsonicState, id3: bool) -> Result<Payload, Failure> {
    let mut db = state.db.clone();
    let albums = Album::all().exec(&mut db).await?;
    let summaries = catalog::summarize_artists(&albums);
    let artists = catalog::artists_by_id(&mut db, summaries.keys().copied()).await?;
    let annotations = annotations::for_artists(&mut db, artists.keys().copied()).await?;

    let mut indexes: BTreeMap<String, Vec<&Artist>> = BTreeMap::new();
    for artist in artists.values() {
        indexes
            .entry(index_name(&artist.name))
            .or_default()
            .push(artist);
    }
    let indexes = indexes.into_iter().map(|(name, mut members)| {
        members.sort_by_cached_key(|artist| sort_key(&artist.name));
        Element::new("index").attr("name", name).list(
            "artist",
            members.into_iter().map(|artist| {
                let summary = &summaries[&artist.id];
                let annotation = annotations.get(&artist.id);
                if id3 {
                    catalog::artist_id3(
                        "artist",
                        artist,
                        summary.albums,
                        summary.has_cover,
                        annotation,
                    )
                } else {
                    catalog::artist_folder("artist", artist, annotation)
                }
            }),
        )
    });

    let ignored = IGNORED_ARTICLES.join(" ");
    let root = if id3 {
        Element::new("artists").attr("ignoredArticles", ignored)
    } else {
        let last_modified = albums
            .iter()
            .map(|album| album.created_at.as_millisecond())
            .max()
            .unwrap_or(0);
        Element::new("indexes")
            .attr("lastModified", last_modified)
            .attr("ignoredArticles", ignored)
    };
    Ok(root.list("index", indexes).into())
}

fn parse_id(params: &Params) -> Result<Option<Id>, Failure> {
    Ok(Id::parse(params.require("id")?))
}

async fn load_artist(state: &SubsonicState, id: u64) -> Result<Artist, Failure> {
    Artist::filter_by_id(id)
        .first()
        .exec(&mut state.db.clone())
        .await?
        .ok_or_else(|| not_found("artist"))
}

async fn load_album(state: &SubsonicState, id: u64) -> Result<Album, Failure> {
    Album::filter_by_id(id)
        .first()
        .exec(&mut state.db.clone())
        .await?
        .ok_or_else(|| not_found("album"))
}

async fn artist_albums(state: &SubsonicState, artist_id: u64) -> Result<Vec<Album>, Failure> {
    let mut albums = Album::filter_by_artist_id(artist_id)
        .exec(&mut state.db.clone())
        .await?;
    albums.sort_by(|a, b| (a.year, &a.title_key).cmp(&(b.year, &b.title_key)));
    Ok(albums)
}

async fn album_tracks(state: &SubsonicState, album_id: u64) -> Result<Vec<Track>, Failure> {
    let mut tracks = Track::filter_by_album_id(album_id)
        .exec(&mut state.db.clone())
        .await?;
    catalog::sort_album_tracks(&mut tracks);
    Ok(tracks)
}

/// `getArtist`: an artist with its albums.
pub(crate) async fn artist(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let Some(Id::Artist(id)) = parse_id(params)? else {
        return Err(not_found("artist"));
    };
    let artist = load_artist(state, id).await?;
    let albums = artist_albums(state, id).await?;
    let stats = catalog::album_stats(&mut state.db.clone()).await?;
    let plays =
        annotations::for_albums(&mut state.db.clone(), albums.iter().map(|album| album.id)).await?;

    let has_cover = albums.iter().any(|album| album.cover.is_some());
    let annotation = annotations::for_artists(&mut state.db.clone(), [artist.id])
        .await?
        .remove(&artist.id);
    Ok(catalog::artist_id3(
        "artist",
        &artist,
        albums.len() as u64,
        has_cover,
        annotation.as_ref(),
    )
    .list(
        "album",
        albums.iter().map(|album| {
            catalog::album_id3(
                "album",
                album,
                Some(&artist),
                stats.get(&album.id).copied().unwrap_or_default(),
                plays.get(&album.id),
            )
        }),
    )
    .into())
}

/// `getAlbum`: an album with its songs.
pub(crate) async fn album(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let Some(Id::Album(id)) = parse_id(params)? else {
        return Err(not_found("album"));
    };
    let album = load_album(state, id).await?;
    let artist = load_artist(state, album.artist_id).await.ok();
    let tracks = album_tracks(state, id).await?;
    let duration = tracks.iter().map(|track| track.duration_ms).sum();
    let songs = catalog::songs(&mut state.db.clone(), "song", &tracks).await?;
    let plays = annotations::for_albums(&mut state.db.clone(), [album.id]).await?;

    Ok(catalog::album_id3(
        "album",
        &album,
        artist.as_ref(),
        (tracks.len() as u64, duration),
        plays.get(&album.id),
    )
    .list("song", songs)
    .into())
}

/// `getSong`.
pub(crate) async fn song(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let Some(Id::Track(id)) = parse_id(params)? else {
        return Err(not_found("song"));
    };
    let mut db = state.db.clone();
    let track = Track::filter_by_id(id)
        .first()
        .exec(&mut db)
        .await?
        .ok_or_else(|| not_found("song"))?;
    let mut songs = catalog::songs(&mut db, "song", &[track]).await?;
    Ok(songs.remove(0).into())
}

/// `getMusicDirectory`: an artist's albums, or an album's songs.
pub(crate) async fn music_directory(
    state: &SubsonicState,
    params: &Params,
) -> Result<Payload, Failure> {
    match parse_id(params)? {
        Some(Id::Artist(id)) => {
            let artist = load_artist(state, id).await?;
            let albums = artist_albums(state, id).await?;
            let stats = catalog::album_stats(&mut state.db.clone()).await?;
            let plays =
                annotations::for_albums(&mut state.db.clone(), albums.iter().map(|album| album.id))
                    .await?;
            Ok(Element::new("directory")
                .attr("id", ids::artist(artist.id))
                .attr("name", artist.name.as_str())
                .list(
                    "child",
                    albums.iter().map(|album| {
                        catalog::album_child(
                            "child",
                            album,
                            Some(&artist),
                            stats.get(&album.id).copied().unwrap_or_default(),
                            plays.get(&album.id),
                        )
                    }),
                )
                .into())
        }
        Some(Id::Album(id)) => {
            let album = load_album(state, id).await?;
            let tracks = album_tracks(state, id).await?;
            let songs = catalog::songs(&mut state.db.clone(), "child", &tracks).await?;
            Ok(Element::new("directory")
                .attr("id", ids::album(album.id))
                .attr("parent", ids::artist(album.artist_id))
                .attr("name", album.title.as_str())
                .list("child", songs)
                .into())
        }
        _ => Err(not_found("directory")),
    }
}

/// `getAlbumInfo` and `getAlbumInfo2`: what píxiū knows about an album.
/// Notes and images from external sources arrive with enrichment.
pub(crate) async fn album_info(state: &SubsonicState, params: &Params) -> Result<Payload, Failure> {
    let Some(Id::Album(id)) = parse_id(params)? else {
        return Err(not_found("album"));
    };
    let album = load_album(state, id).await?;
    Ok(Element::new("albumInfo")
        .field_opt("musicBrainzId", album.mbid.as_deref())
        .into())
}

/// `getArtistInfo` and `getArtistInfo2`: what píxiū knows about an artist.
pub(crate) async fn artist_info(
    state: &SubsonicState,
    params: &Params,
    id3: bool,
) -> Result<Payload, Failure> {
    let Some(Id::Artist(id)) = parse_id(params)? else {
        return Err(not_found("artist"));
    };
    let artist = load_artist(state, id).await?;
    let name = if id3 { "artistInfo2" } else { "artistInfo" };
    Ok(Element::new(name)
        .field_opt("biography", artist.bio.as_deref())
        .field_opt("musicBrainzId", artist.mbid.as_deref())
        .list("similarArtist", Vec::new())
        .into())
}

/// `getGenres`.
pub(crate) async fn genres(state: &SubsonicState) -> Result<Payload, Failure> {
    let rows = toasty::sql::query(
        "SELECT genre, COUNT(*), COUNT(DISTINCT album_id) FROM tracks \
         WHERE genre IS NOT NULL GROUP BY genre ORDER BY genre",
    )
    .exec(&mut state.db.clone())
    .await?;
    let genres = rows.iter().filter_map(|row| {
        let Value::Record(record) = row else {
            return None;
        };
        let name = record[0].as_str()?;
        Some(
            Element::new("genre")
                .attr("songCount", as_u64(&record[1]).unwrap_or(0))
                .attr("albumCount", as_u64(&record[2]).unwrap_or(0))
                .text(name),
        )
    });
    Ok(Element::new("genres").list("genre", genres).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn articles_are_ignored_for_sorting_and_indexing() {
        assert_eq!(sort_key("The Beatles"), "beatles");
        assert_eq!(sort_key("Theatre of Tragedy"), "theatre of tragedy");
        assert_eq!(sort_key("Los Lobos"), "lobos");
        assert_eq!(index_name("The Beatles"), "B");
        assert_eq!(index_name("ångström"), "Å");
        assert_eq!(index_name("2Pac"), "#");
        assert_eq!(index_name(""), "#");
    }
}
