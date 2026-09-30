//! Songs, albums and artists as Subsonic JSON, for píxiū's web API, which
//! lists them in orders no Subsonic method offers. The web player reads
//! both APIs, so both describe the library the same way.

use pixiu_db::{Library, User, toasty};
use serde_json::Value as Json;

use crate::{annotations, catalog};

/// The library's songs (Subsonic `Child`), in the order of `ids`.
pub async fn songs(lib: &Library, ids: &[u64]) -> Result<Vec<Json>, toasty::Error> {
    let tracks = catalog::tracks_in_order(lib, ids).await?;
    Ok(catalog::songs(lib, "song", &tracks)
        .await?
        .iter()
        .map(|song| song.to_json())
        .collect())
}

/// The library's albums (`AlbumID3`), in the order of `ids`.
pub async fn albums(lib: &Library, ids: &[u64]) -> Result<Vec<Json>, toasty::Error> {
    let albums = catalog::albums_in_order(lib, ids).await?;
    let artists = catalog::artists_by_id(lib, albums.iter().map(|album| album.artist_id)).await?;
    let stats = catalog::album_stats(lib).await?;
    let plays = annotations::for_albums(lib, ids.iter().copied()).await?;
    Ok(albums
        .iter()
        .map(|album| {
            catalog::album_id3(
                "album",
                album,
                artists.get(&album.artist_id),
                stats.get(&album.id).copied().unwrap_or_default(),
                plays.get(&album.id),
            )
            .to_json()
        })
        .collect())
}

/// The library's artists (`ArtistID3`), in the order of `ids`.
pub async fn artists(lib: &Library, ids: &[u64]) -> Result<Vec<Json>, toasty::Error> {
    let artists = catalog::artists_in_order(lib, ids).await?;
    let albums = lib.albums_of_artists(ids).await?;
    let summaries = catalog::summarize_artists(&albums);
    let annotations = annotations::for_artists(lib, ids.iter().copied()).await?;
    Ok(artists
        .iter()
        .map(|artist| {
            let summary = summaries.get(&artist.id);
            catalog::artist_id3(
                "artist",
                artist,
                summary.map_or(0, |summary| summary.albums),
                summary.is_some_and(|summary| summary.has_cover),
                annotations.get(&artist.id),
            )
            .to_json()
        })
        .collect())
}

/// Every playlist of `owner`'s (`Playlist`, without its songs), by name.
pub async fn playlists(lib: &Library, owner: &User) -> Result<Vec<Json>, toasty::Error> {
    let mut playlists = lib.all_playlists().await?;
    playlists.sort_by_key(|playlist| playlist.name.to_lowercase());
    let mut rendered = Vec::with_capacity(playlists.len());
    for playlist in &playlists {
        rendered.push(
            crate::playlists::describe(lib, playlist, owner, false)
                .await?
                .to_json(),
        );
    }
    Ok(rendered)
}
