//! What the player adds to Subsonic's playlists: smart playlists, whose
//! songs follow rules, and folders to file playlists in, all in the
//! signed-in user's library. Songs of ordinary playlists are changed
//! through the Subsonic API.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pixiu_db::{Library, Playlist, PlaylistFolder, User, now, toasty};
use pixiu_subsonic::{ids, render, smart};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{ApiError, ApiResult, ApiState, Session};

fn playlist_id(id: &str) -> ApiResult<u64> {
    match ids::Id::parse(id) {
        Some(ids::Id::Playlist(id)) => Ok(id),
        _ => Err(ApiError::not_found("playlist")),
    }
}

fn folder_id(id: &str) -> ApiResult<u64> {
    id.parse().map_err(|_| ApiError::not_found("folder"))
}

async fn load_playlist(lib: &Library, id: u64) -> ApiResult<Playlist> {
    lib.playlist(id)
        .await?
        .ok_or_else(|| ApiError::not_found("playlist"))
}

async fn load_folder(lib: &Library, id: u64) -> ApiResult<PlaylistFolder> {
    lib.folder(id)
        .await?
        .ok_or_else(|| ApiError::not_found("folder"))
}

/// Checks rules the player sent, and stores them as JSON.
fn rules_text(rules: &JsonValue) -> ApiResult<String> {
    let text = rules.to_string();
    smart::parse(&text).map_err(|error| ApiError::unprocessable(error.to_string()))?;
    Ok(text)
}

fn describe_folder(folder: &PlaylistFolder) -> JsonValue {
    json!({
        "type": "playlist-folders",
        "id": folder.id.to_string(),
        "name": folder.name,
        "parent_id": folder.parent_id.map(|id| id.to_string()),
    })
}

/// Every folder of the library, for the start-up payload.
pub(crate) async fn folders(lib: &Library) -> ApiResult<Vec<JsonValue>> {
    Ok(lib
        .all_folders()
        .await?
        .iter()
        .map(describe_folder)
        .collect())
}

/// `GET /api/playlists`: every playlist as Subsonic describes it, with its
/// folder and, for smart playlists, its rules.
pub(crate) async fn list(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<Vec<JsonValue>>> {
    Ok(Json(all(&session.library(&state), &session.user).await?))
}

async fn all(lib: &Library, owner: &User) -> ApiResult<Vec<JsonValue>> {
    let stored: HashMap<String, Playlist> = lib
        .all_playlists()
        .await?
        .into_iter()
        .map(|playlist| (ids::playlist(playlist.id), playlist))
        .collect();
    let mut playlists = render::playlists(lib, owner).await?;
    for playlist in &mut playlists {
        let Some(stored) = playlist["id"].as_str().and_then(|id| stored.get(id)) else {
            continue;
        };
        playlist["folderId"] = json!(stored.folder_id.map(|id| id.to_string()));
        playlist["rules"] = stored
            .rules
            .as_deref()
            .and_then(|rules| serde_json::from_str(rules).ok())
            .unwrap_or(JsonValue::Null);
    }
    Ok(playlists)
}

async fn one(lib: &Library, owner: &User, id: u64) -> ApiResult<JsonValue> {
    let wanted = ids::playlist(id);
    all(lib, owner)
        .await?
        .into_iter()
        .find(|playlist| playlist["id"] == wanted.as_str())
        .ok_or_else(|| ApiError::not_found("playlist"))
}

#[derive(Deserialize)]
pub(crate) struct NewSmartPlaylist {
    name: String,
    #[serde(default)]
    description: String,
    folder_id: Option<String>,
    rules: JsonValue,
}

/// `POST /api/playlists`: makes a smart playlist.
pub(crate) async fn create(
    State(state): State<ApiState>,
    session: Session,
    Json(form): Json<NewSmartPlaylist>,
) -> ApiResult<Json<JsonValue>> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(ApiError::unprocessable("Name the playlist."));
    }
    let rules = rules_text(&form.rules)?;
    let lib = session.library(&state);
    let folder = match form.folder_id.as_deref() {
        Some(id) => Some(load_folder(&lib, folder_id(id)?).await?.id),
        None => None,
    };
    let comment = Some(form.description.trim().to_owned()).filter(|text| !text.is_empty());
    let playlist = toasty::create!(Playlist {
        user_id: lib.owner(),
        name,
        comment,
        public: false,
        folder_id: folder,
        rules: Some(rules),
        created_at: now(),
        changed_at: now(),
    })
    .exec(&mut lib.db())
    .await?;
    Ok(Json(one(&lib, &session.user, playlist.id).await?))
}

#[derive(Deserialize)]
pub(crate) struct PlaylistChanges {
    name: Option<String>,
    description: Option<String>,
    /// A smart playlist's new rules; ignored for other playlists.
    rules: Option<JsonValue>,
}

/// `PUT /api/playlists/{id}`: renames, describes, or changes rules.
pub(crate) async fn update(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<String>,
    Json(changes): Json<PlaylistChanges>,
) -> ApiResult<Json<JsonValue>> {
    let lib = session.library(&state);
    let mut playlist = load_playlist(&lib, playlist_id(&id)?).await?;
    let name = match changes.name.as_deref().map(str::trim) {
        Some("") => return Err(ApiError::unprocessable("Name the playlist.")),
        Some(name) => name.to_owned(),
        None => playlist.name.clone(),
    };
    let comment = match changes.description {
        Some(text) => Some(text.trim().to_owned()).filter(|text| !text.is_empty()),
        None => playlist.comment.clone(),
    };
    let rules = match (&playlist.rules, &changes.rules) {
        (Some(_), Some(rules)) => Some(rules_text(rules)?),
        _ => playlist.rules.clone(),
    };
    let playlist_id = playlist.id;
    toasty::update!(playlist {
        name,
        comment,
        rules,
        changed_at: now(),
    })
    .exec(&mut lib.db())
    .await?;
    Ok(Json(one(&lib, &session.user, playlist_id).await?))
}

#[derive(Deserialize)]
pub(crate) struct NewFolder {
    name: String,
    parent_id: Option<String>,
}

/// `POST /api/playlist-folders`.
pub(crate) async fn create_folder(
    State(state): State<ApiState>,
    session: Session,
    Json(form): Json<NewFolder>,
) -> ApiResult<Json<JsonValue>> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(ApiError::unprocessable("Name the folder."));
    }
    let lib = session.library(&state);
    let mut db = lib.db();
    let parent = match form.parent_id.as_deref() {
        Some(id) => Some(load_folder(&lib, folder_id(id)?).await?.id),
        None => None,
    };
    let folder = toasty::create!(PlaylistFolder {
        user_id: lib.owner(),
        name,
        parent_id: parent,
        created_at: now(),
    })
    .exec(&mut db)
    .await?;
    Ok(Json(describe_folder(&folder)))
}

#[derive(Deserialize)]
pub(crate) struct FolderChanges {
    name: Option<String>,
    /// Absent keeps the parent; `null` moves the folder to the top.
    #[serde(default, deserialize_with = "present")]
    parent_id: Option<Option<String>>,
}

/// Tells a field set to `null` (`Some(None)`) from one left out (`None`).
fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

/// Whether `candidate` is `folder` or inside it, which would make a loop.
async fn is_within(lib: &Library, candidate: u64, folder: u64) -> ApiResult<bool> {
    let parents: HashMap<u64, Option<u64>> = lib
        .all_folders()
        .await?
        .into_iter()
        .map(|folder| (folder.id, folder.parent_id))
        .collect();
    let mut current = Some(candidate);
    let mut steps = 0;
    while let Some(id) = current {
        if id == folder {
            return Ok(true);
        }
        steps += 1;
        if steps > parents.len() {
            break;
        }
        current = parents.get(&id).copied().flatten();
    }
    Ok(false)
}

/// `PUT` and `PATCH /api/playlist-folders/{id}`: renames or moves a folder.
pub(crate) async fn update_folder(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<String>,
    Json(changes): Json<FolderChanges>,
) -> ApiResult<StatusCode> {
    let lib = session.library(&state);
    let mut db = lib.db();
    let mut folder = load_folder(&lib, folder_id(&id)?).await?;
    let name = match changes.name.as_deref().map(str::trim) {
        Some("") => return Err(ApiError::unprocessable("Name the folder.")),
        Some(name) => name.to_owned(),
        None => folder.name.clone(),
    };
    let parent_id = match changes.parent_id {
        None => folder.parent_id,
        Some(None) => None,
        Some(Some(parent)) => {
            let parent = load_folder(&lib, folder_id(&parent)?).await?.id;
            if is_within(&lib, parent, folder.id).await? {
                return Err(ApiError::unprocessable("A folder cannot go inside itself."));
            }
            Some(parent)
        }
    };
    toasty::update!(folder { name, parent_id })
        .exec(&mut db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/playlist-folders/{id}`: its folders and playlists move to
/// the top.
pub(crate) async fn delete_folder(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let lib = session.library(&state);
    let mut db = lib.db();
    let folder = load_folder(&lib, folder_id(&id)?).await?;
    for mut child in PlaylistFolder::filter_by_parent_id(Some(folder.id))
        .exec(&mut db)
        .await?
    {
        toasty::update!(child { parent_id: None })
            .exec(&mut db)
            .await?;
    }
    for mut playlist in Playlist::filter_by_folder_id(Some(folder.id))
        .exec(&mut db)
        .await?
    {
        toasty::update!(playlist { folder_id: None })
            .exec(&mut db)
            .await?;
    }
    folder.delete().exec(&mut db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct Playlists {
    playlists: Vec<String>,
}

async fn file_playlists(lib: &Library, playlists: &[String], folder: Option<u64>) -> ApiResult<()> {
    for id in playlists {
        let mut playlist = load_playlist(lib, playlist_id(id)?).await?;
        toasty::update!(playlist { folder_id: folder })
            .exec(&mut lib.db())
            .await?;
    }
    Ok(())
}

/// `POST /api/playlist-folders/{id}/playlists`: files playlists in it.
pub(crate) async fn add_playlists(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<String>,
    Json(form): Json<Playlists>,
) -> ApiResult<StatusCode> {
    let lib = session.library(&state);
    let folder = load_folder(&lib, folder_id(&id)?).await?;
    file_playlists(&lib, &form.playlists, Some(folder.id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/playlist-folders/{id}/playlists`: moves playlists out of
/// it, to the top.
pub(crate) async fn remove_playlists(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<String>,
    Json(form): Json<Playlists>,
) -> ApiResult<StatusCode> {
    let lib = session.library(&state);
    load_folder(&lib, folder_id(&id)?).await?;
    file_playlists(&lib, &form.playlists, None).await?;
    Ok(StatusCode::NO_CONTENT)
}
