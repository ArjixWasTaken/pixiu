//! Songs, albums and artists as Subsonic JSON, for píxiū's web API, which
//! lists them in orders no Subsonic method offers. The web player reads
//! both APIs, so both describe the library the same way.

use pixiu_db::{Album, Db, Playlist, User, toasty};
use serde_json::Value as Json;

use crate::{annotations, catalog};

/// Songs (Subsonic `Child`), in the order of `ids`.
pub async fn songs(db: &mut Db, ids: &[u64]) -> Result<Vec<Json>, toasty::Error> {
    let tracks = catalog::tracks_in_order(db, ids).await?;
    Ok(catalog::songs(db, "song", &tracks)
        .await?
        .iter()
        .map(|song| song.to_json())
        .collect())
}

/// Albums (`AlbumID3`), in the order of `ids`.
pub async fn albums(db: &mut Db, ids: &[u64]) -> Result<Vec<Json>, toasty::Error> {
    let albums = catalog::albums_in_order(db, ids).await?;
    let artists = catalog::artists_by_id(db, albums.iter().map(|album| album.artist_id)).await?;
    let stats = catalog::album_stats(db).await?;
    let plays = annotations::for_albums(db, ids.iter().copied()).await?;
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

/// Artists (`ArtistID3`), in the order of `ids`.
pub async fn artists(db: &mut Db, ids: &[u64]) -> Result<Vec<Json>, toasty::Error> {
    let artists = catalog::artists_in_order(db, ids).await?;
    let albums = if ids.is_empty() {
        Vec::new()
    } else {
        Album::filter(Album::fields().artist_id().in_list(ids.to_vec()))
            .exec(db)
            .await?
    };
    let summaries = catalog::summarize_artists(&albums);
    let annotations = annotations::for_artists(db, ids.iter().copied()).await?;
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

/// Every playlist (`Playlist`, without its songs), by name, owned by
/// `owner`: the only user.
pub async fn playlists(db: &mut Db, owner: &User) -> Result<Vec<Json>, toasty::Error> {
    let mut playlists = Playlist::all().exec(db).await?;
    playlists.sort_by_key(|playlist| playlist.name.to_lowercase());
    let mut rendered = Vec::with_capacity(playlists.len());
    for playlist in &playlists {
        rendered.push(
            crate::playlists::describe(db, playlist, owner, false)
                .await?
                .to_json(),
        );
    }
    Ok(rendered)
}
