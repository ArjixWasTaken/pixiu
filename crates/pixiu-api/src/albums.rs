//! What the player's album screen shows beyond Subsonic: the MusicBrainz
//! match, releases to pick from when no match was certain, editing tags,
//! and looking the album up again.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pixiu_db::{Album, Artist, Db, Enrichment, Track};
use pixiu_jobs::NewJob;
use pixiu_subsonic::ids;
use pixiu_treasury::{AlbumEdit, ArtistRef, TrackEdit};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session};

fn album_id(id: &str) -> ApiResult<u64> {
    match ids::Id::parse(id) {
        Some(ids::Id::Album(id)) => Ok(id),
        _ => Err(ApiError::not_found("album")),
    }
}

async fn load(db: &mut Db, id: u64) -> ApiResult<Album> {
    Album::filter_by_id(id)
        .first()
        .exec(db)
        .await?
        .ok_or_else(|| ApiError::not_found("album"))
}

/// `GET /api/albums/{id}/details`.
pub(crate) async fn details(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<String>,
) -> ApiResult<Json<JsonValue>> {
    let mut db = state.db.clone();
    let album = load(&mut db, album_id(&id)?).await?;
    let artist = Artist::get_by_id(&mut db, &album.artist_id).await?;
    let candidates: JsonValue = album
        .candidates
        .as_deref()
        .and_then(|candidates| serde_json::from_str(candidates).ok())
        .unwrap_or_else(|| json!([]));
    let mut tracks = Track::filter_by_album_id(album.id).exec(&mut db).await?;
    tracks.sort_by_key(|track| (track.disc_number, track.track_number, track.id));
    Ok(Json(json!({
        "title": album.title,
        "artist": artist.name,
        "year": album.year,
        "enrichment": album.enrichment.map(|enrichment| match enrichment {
            Enrichment::Matched => "matched",
            Enrichment::Review => "review",
            Enrichment::Unmatched => "unmatched",
        }),
        "enriched_at": album.enriched_at,
        "mbid": album.mbid,
        "candidates": candidates,
        "source": if album.ytm_browse_id.is_some() { "youtube_music" } else { "offering" },
        "youtube_url": album
            .ytm_browse_id
            .as_ref()
            .map(|browse| format!("https://music.youtube.com/browse/{browse}")),
        "tracks": tracks.iter().map(|track| json!({
            "id": ids::track(track.id),
            "title": track.title,
            "track": track.track_number,
            "disc": track.disc_number,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub(crate) struct TrackForm {
    id: String,
    title: String,
    track: Option<u32>,
}

#[derive(Deserialize)]
pub(crate) struct AlbumForm {
    title: String,
    artist: String,
    year: Option<i32>,
    #[serde(default)]
    tracks: Vec<TrackForm>,
}

fn nonempty(text: &str) -> Option<String> {
    Some(text.trim().to_owned()).filter(|text| !text.is_empty())
}

/// `PUT /api/albums/{id}`: changes the album's tags; files move to match.
pub(crate) async fn edit(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<String>,
    Json(form): Json<AlbumForm>,
) -> ApiResult<StatusCode> {
    let mut db = state.db.clone();
    let album = load(&mut db, album_id(&id)?).await?;
    let album_artist = Artist::get_by_id(&mut db, &album.artist_id).await?;
    let artist_name = nonempty(&form.artist).unwrap_or_else(|| album_artist.name.clone());
    let mut tracks = Vec::new();
    for track in Track::filter_by_album_id(album.id).exec(&mut db).await? {
        let changed = form
            .tracks
            .iter()
            .find(|edit| ids::Id::parse(&edit.id) == Some(ids::Id::Track(track.id)));
        let artist = Artist::get_by_id(&mut db, &track.artist_id).await?;
        tracks.push(TrackEdit {
            track_id: track.id,
            title: changed
                .and_then(|edit| nonempty(&edit.title))
                .unwrap_or_else(|| track.title.clone()),
            artist_credit: track.artist_credit.clone(),
            artist: ArtistRef {
                name: artist.name,
                mbid: artist.mbid,
            },
            track_number: match changed {
                Some(edit) => edit.track,
                None => track.track_number,
            },
            disc_number: track.disc_number,
            mbid: None,
            isrc: None,
        });
    }
    state
        .treasury
        .edit_album(&AlbumEdit {
            album_id: album.id,
            title: nonempty(&form.title).unwrap_or_else(|| album.title.clone()),
            // The same name keeps the same artist, MusicBrainz id and all.
            artist: ArtistRef {
                mbid: (artist_name == album_artist.name)
                    .then(|| album_artist.mbid.clone())
                    .flatten(),
                name: artist_name,
            },
            year: form.year,
            mbid: None,
            rg_mbid: None,
            tracks,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A MusicBrainz release id, from an id or a link.
fn release_id(input: &str) -> Option<String> {
    let input = input.trim().trim_end_matches('/');
    let id = input.rsplit('/').next()?;
    let valid = id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    valid.then(|| id.to_ascii_lowercase())
}

#[derive(Deserialize, Default)]
pub(crate) struct Lookup {
    /// A MusicBrainz release (id or link) to use; else píxiū searches.
    release: Option<String>,
}

/// `POST /api/albums/{id}/lookup`: looks the album up on MusicBrainz
/// again, or follows the given release.
pub(crate) async fn lookup(
    State(state): State<ApiState>,
    _: Session,
    Path(id): Path<String>,
    body: Option<Json<Lookup>>,
) -> ApiResult<StatusCode> {
    let album = load(&mut state.db.clone(), album_id(&id)?).await?;
    let release =
        match body.and_then(|Json(lookup)| lookup.release) {
            Some(given) => Some(release_id(&given).ok_or_else(|| {
                ApiError::unprocessable("That is not a MusicBrainz release link.")
            })?),
            None => None,
        };
    let fresh = release.is_none();
    let title = format!("Look up {}", album.title);
    state
        .jobs
        .enqueue(NewJob::enrich(album.id, &title, release, fresh))
        .await?;
    Ok(StatusCode::ACCEPTED)
}

#[cfg(test)]
mod tests {
    use super::release_id;

    #[test]
    fn release_links_are_read() {
        let id = "0a1b2c3d-4e5f-6789-abcd-ef0123456789";
        assert_eq!(release_id(id).as_deref(), Some(id));
        assert_eq!(
            release_id(&format!("https://musicbrainz.org/release/{id}/")).as_deref(),
            Some(id)
        );
        assert_eq!(release_id("https://example.com/nope"), None);
    }
}
