//! `search2` and `search3`.
//!
//! Every word of the query must match (case-insensitively, anywhere) in one
//! of the searched fields. An empty query matches everything, which clients
//! use to sync the whole library page by page.

use crate::{
    Failure, Params, SubsonicState, annotations,
    catalog::{self, Bind},
    ids,
    lists::Sql,
    response::{Element, Payload},
};

/// Clients page through large results; cap each page.
const MAX_PAGE: u32 = 5000;

/// Cleans up what clients send: quotes, and trailing wildcards some add.
fn words(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|word| word.trim_matches(['"', '*']).to_owned())
        .filter(|word| !word.is_empty())
        .collect()
}

/// A LIKE pattern matching `word` literally anywhere.
fn contains(word: &str) -> String {
    let escaped = word
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

/// Adds `AND (a LIKE w OR b LIKE w ...)` for every word.
fn match_all(sql: &mut Sql, words: &[String], columns: &[&str]) {
    for word in words {
        let pattern = sql.param(Bind::Text(contains(word)));
        let alternatives: Vec<String> = columns
            .iter()
            .map(|column| format!("{column} LIKE {pattern} ESCAPE '\\'"))
            .collect();
        sql.push(&format!(" AND ({})", alternatives.join(" OR ")));
    }
}

struct Page {
    count: u32,
    offset: u32,
}

fn page(params: &Params, kind: &str) -> Result<Page, Failure> {
    Ok(Page {
        count: params
            .number(&format!("{kind}Count"), 20_u32)?
            .min(MAX_PAGE),
        offset: params.number(&format!("{kind}Offset"), 0_u32)?,
    })
}

pub(crate) async fn search(
    state: &SubsonicState,
    params: &Params,
    id3: bool,
) -> Result<Payload, Failure> {
    let words = words(params.get("query").unwrap_or_default());
    let mut db = state.db.clone();

    // Artists: album artists whose name matches.
    let artists_page = page(params, "artist")?;
    let mut sql = Sql::new("SELECT id FROM artists WHERE id IN (SELECT artist_id FROM albums)");
    match_all(&mut sql, &words, &["name"]);
    sql.push(" ORDER BY name_key, id");
    sql.page(artists_page.count, artists_page.offset);
    let artist_ids = sql.ids(&mut db).await?;

    // Albums: by title or album artist.
    let albums_page = page(params, "album")?;
    let mut sql = Sql::new(
        "SELECT albums.id FROM albums JOIN artists ON artists.id = albums.artist_id WHERE 1",
    );
    match_all(&mut sql, &words, &["albums.title", "artists.name"]);
    sql.push(" ORDER BY albums.title_key, albums.id");
    sql.page(albums_page.count, albums_page.offset);
    let album_ids = sql.ids(&mut db).await?;

    // Songs: by title, artist credit or album title.
    let songs_page = page(params, "song")?;
    let mut sql =
        Sql::new("SELECT tracks.id FROM tracks JOIN albums ON albums.id = tracks.album_id WHERE 1");
    match_all(
        &mut sql,
        &words,
        &["tracks.title", "tracks.artist_credit", "albums.title"],
    );
    sql.push(if words.is_empty() {
        " ORDER BY tracks.id"
    } else {
        " ORDER BY tracks.title, tracks.id"
    });
    sql.page(songs_page.count, songs_page.offset);
    let track_ids = sql.ids(&mut db).await?;

    let artists = catalog::artists_in_order(&mut db, &artist_ids).await?;
    let albums = catalog::albums_in_order(&mut db, &album_ids).await?;
    let tracks = catalog::tracks_in_order(&mut db, &track_ids).await?;

    let all_albums = pixiu_db::Album::all().exec(&mut db).await?;
    let summaries = catalog::summarize_artists(&all_albums);
    let stats = catalog::album_stats(&mut db).await?;
    let album_artists =
        catalog::artists_by_id(&mut db, albums.iter().map(|album| album.artist_id)).await?;
    let songs = catalog::songs(&mut db, "song", &tracks).await?;

    let artist_elements = artists.iter().map(|artist| {
        if id3 {
            let summary = summaries.get(&artist.id);
            catalog::artist_id3(
                "artist",
                artist,
                summary.map_or(0, |summary| summary.albums),
                summary.is_some_and(|summary| summary.has_cover),
            )
        } else {
            Element::new("artist")
                .attr("id", ids::artist(artist.id))
                .attr("name", artist.name.as_str())
        }
    });
    let plays = annotations::for_albums(&mut db, album_ids.iter().copied()).await?;
    let album_elements = albums.iter().map(|album| {
        let artist = album_artists.get(&album.artist_id);
        let stats = stats.get(&album.id).copied().unwrap_or_default();
        let plays = plays.get(&album.id);
        if id3 {
            catalog::album_id3("album", album, artist, stats, plays)
        } else {
            catalog::album_child("album", album, artist, stats, plays)
        }
    });

    let name = if id3 {
        "searchResult3"
    } else {
        "searchResult2"
    };
    Ok(Element::new(name)
        .list("artist", artist_elements)
        .list("album", album_elements)
        .list("song", songs)
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_are_split_and_cleaned() {
        assert_eq!(words(r#" "daft  punk*" "#), ["daft", "punk"]);
        assert!(words(r#""""#).is_empty());
        assert!(words("").is_empty());
    }

    #[test]
    fn like_patterns_are_literal() {
        assert_eq!(contains("100%"), "%100\\%%");
        assert_eq!(contains("a_b"), "%a\\_b%");
    }
}
