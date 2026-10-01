//! Offerings: files the admin uploads (audio, or zip archives of albums),
//! reviewed in batches before they join the treasure.

use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use pixiu_db::{Offering, OfferingStatus};
use pixiu_jobs::NewJob;
use serde_json::{Value as JsonValue, json};
use tokio::io::AsyncWriteExt;

use crate::{ApiError, ApiResult, ApiState, Session};

fn describe(offering: &Offering) -> JsonValue {
    json!({
        "id": offering.id,
        "file_name": offering.file_name,
        "archive": offering.archive,
        "size": offering.size,
        "readable": offering.status == OfferingStatus::Pending,
        "error": offering.error,
        "title": offering.title,
        "artist": offering.artist,
        "album": offering.album,
        "album_artist": offering.album_artist,
        "track": offering.track_number,
        "disc": offering.disc_number,
        "year": offering.year,
        "genre": offering.genre,
        "length": offering.duration_ms / 1000,
    })
}

/// `GET /api/offerings`: what awaits review, by batch.
pub(crate) async fn list(
    State(state): State<ApiState>,
    session: Session,
) -> ApiResult<Json<JsonValue>> {
    let mut batches: Vec<(String, Vec<JsonValue>)> = Vec::new();
    for offering in state.offerings.pending(session.owner()).await? {
        match batches.last_mut() {
            Some((batch, files)) if *batch == offering.batch => files.push(describe(&offering)),
            _ => batches.push((offering.batch.clone(), vec![describe(&offering)])),
        }
    }
    Ok(Json(json!(
        batches
            .into_iter()
            .map(|(batch, files)| json!({ "batch": batch, "files": files }))
            .collect::<Vec<_>>()
    )))
}

/// `POST /api/offerings/upload`: a multipart form with the `batch` it
/// belongs to (files dropped together share one), then one `file`, which
/// streams to disk. Answers with the offerings it made: an archive makes
/// one per audio file inside.
pub(crate) async fn upload(
    State(state): State<ApiState>,
    session: Session,
    mut multipart: Multipart,
) -> ApiResult<Json<JsonValue>> {
    let bad = |error: axum::extract::multipart::MultipartError| {
        ApiError::new(error.status(), error.body_text())
    };
    let mut batch = None;
    let mut registered = Vec::new();
    while let Some(mut field) = multipart.next_field().await.map_err(bad)? {
        match field.name() {
            Some("batch") => batch = Some(field.text().await.map_err(bad)?),
            Some("file") => {
                let batch = batch
                    .as_deref()
                    .ok_or_else(|| ApiError::unprocessable("Name the batch before the file."))?;
                let name = field
                    .file_name()
                    .filter(|name| !name.is_empty())
                    .unwrap_or("upload")
                    .to_owned();
                let (path, mut file) = state
                    .offerings
                    .create_upload(session.owner(), batch, &name)
                    .await?;
                let written = async {
                    while let Some(chunk) = field.chunk().await.map_err(bad)? {
                        file.write_all(&chunk)
                            .await
                            .map_err(|error| ApiError::internal(error, "saving an upload"))?;
                    }
                    file.flush()
                        .await
                        .map_err(|error| ApiError::internal(error, "saving an upload"))
                }
                .await;
                if let Err(error) = written {
                    // A cut-off upload must not be reviewed as a whole file.
                    let _ = tokio::fs::remove_file(&path).await;
                    return Err(error);
                }
                let offerings = state
                    .offerings
                    .process_upload(session.owner(), batch, &path)
                    .await?;
                tracing::info!(
                    batch,
                    file = name,
                    offerings = offerings.len(),
                    "offering received"
                );
                registered.extend(offerings.iter().map(describe));
            }
            _ => {}
        }
    }
    Ok(Json(json!(registered)))
}

/// `POST /api/offerings/batches/{batch}/accept`: absorbs every readable
/// offering of the batch, then looks the albums up on MusicBrainz.
pub(crate) async fn accept_batch(
    State(state): State<ApiState>,
    session: Session,
    Path(batch): Path<String>,
) -> ApiResult<Json<JsonValue>> {
    let (owner, lib) = (session.owner(), session.library(&state));
    let outcome = state.offerings.accept_batch(owner, &batch).await?;
    for album_id in &outcome.albums {
        if let Some(album) = lib.album(*album_id).await? {
            let title = format!("Look up {}", album.title);
            state
                .jobs
                .enqueue(owner, NewJob::enrich(*album_id, &title, None, false))
                .await?;
        }
    }
    let failures: Vec<JsonValue> = outcome
        .failures
        .iter()
        .map(|(offering, error)| json!({ "file_name": offering.file_name, "error": error.to_string() }))
        .collect();
    Ok(Json(
        json!({ "albums": outcome.albums.len(), "failures": failures }),
    ))
}

/// `DELETE /api/offerings/batches/{batch}`.
pub(crate) async fn discard_batch(
    State(state): State<ApiState>,
    session: Session,
    Path(batch): Path<String>,
) -> ApiResult<StatusCode> {
    state
        .offerings
        .discard_batch(session.owner(), &batch)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/offerings/{id}`.
pub(crate) async fn discard(
    State(state): State<ApiState>,
    session: Session,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    state.offerings.discard(session.owner(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
