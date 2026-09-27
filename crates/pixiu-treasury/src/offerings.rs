//! Offerings: music the admin uploads for píxiū to absorb.
//!
//! Uploads land in a per-batch staging directory. Archives are unpacked,
//! every audio file is tagged and registered as an [`Offering`] for review,
//! and accepted offerings are [ingested](Treasury::ingest) into the treasure.

use std::{
    io,
    path::{Path, PathBuf},
};

use pixiu_db::{Db, Offering, OfferingStatus, Track, TrackOrigin, now, toasty};

use crate::{
    Claim, IngestError, Treasury, layout,
    tags::{self, AudioInfo, Cover, TagError},
};

/// Extensions of the audio formats píxiū accepts.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "m4a", "aac", "wav", "aif", "aiff", "wv", "ape", "mpc",
];

/// Stems of image files treated as the album cover, like `folder.jpg`.
const COVER_STEMS: &[&str] = &["cover", "folder", "front", "album"];

/// Stop unpacking archives beyond this many bytes.
const MAX_EXTRACTED_BYTES: u64 = 20 * 1024 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum OfferingError {
    #[error("no such offering")]
    NotFound,
    #[error("this offering cannot be accepted: {0}")]
    Unreadable(String),
    #[error("invalid batch id")]
    InvalidBatch,
    #[error("invalid archive: {0}")]
    Archive(String),
    #[error(transparent)]
    Ingest(#[from] IngestError),
    #[error(transparent)]
    Tags(#[from] TagError),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("database error: {0}")]
    Db(#[from] toasty::Error),
    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// The staging area for uploads.
#[derive(Clone)]
pub struct Offerings {
    dir: PathBuf,
    treasury: Treasury,
}

impl Offerings {
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>, treasury: Treasury) -> Self {
        Self {
            dir: dir.into(),
            treasury,
        }
    }

    fn db(&self) -> Db {
        self.treasury.db()
    }

    /// A fresh batch id for files uploaded together.
    #[must_use]
    pub fn new_batch() -> String {
        format!("{}-{:04x}", now().as_millisecond(), rand::random::<u16>())
    }

    fn batch_dir(&self, batch: &str) -> Result<PathBuf, OfferingError> {
        let valid =
            !batch.is_empty() && batch.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if valid {
            Ok(self.dir.join(batch))
        } else {
            Err(OfferingError::InvalidBatch)
        }
    }

    /// Creates the staging file for an uploaded file named `file_name`.
    ///
    /// # Errors
    ///
    /// Fails for invalid batch ids, or when the file cannot be created.
    pub async fn create_upload(
        &self,
        batch: &str,
        file_name: &str,
    ) -> Result<(PathBuf, tokio::fs::File), OfferingError> {
        let dir = self.batch_dir(batch)?;
        tokio::fs::create_dir_all(&dir).await?;
        // Browsers may send paths for folder uploads; keep the last part.
        let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
        let path = free_name(&dir, &layout::sanitize(name, "upload"));
        let file = tokio::fs::File::create_new(&path).await?;
        Ok((path, file))
    }

    /// Unpacks archives in a batch and registers every audio file in it for
    /// review. Files that are neither audio nor a cover image are dropped.
    ///
    /// # Errors
    ///
    /// Fails on invalid archives and I/O or database errors.
    pub async fn process_batch(&self, batch: &str) -> Result<Vec<Offering>, OfferingError> {
        let dir = self.batch_dir(batch)?;

        for path in list_files(&dir).await? {
            if has_extension(&path, &["zip"]) {
                let target = dir.clone();
                let archive = path.clone();
                tokio::task::spawn_blocking(move || extract_zip(&archive, &target)).await??;
                tokio::fs::remove_file(&path).await?;
            }
        }

        let mut offerings = Vec::new();
        for path in list_files(&dir).await? {
            if has_extension(&path, AUDIO_EXTENSIONS) {
                offerings.push(self.register(batch, &path).await?);
            } else if !is_cover_image(&path) {
                tokio::fs::remove_file(&path).await?;
            }
        }
        if offerings.is_empty() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(offerings)
    }

    async fn register(&self, batch: &str, path: &Path) -> Result<Offering, OfferingError> {
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let size = tokio::fs::metadata(path).await?.len();
        let staged_path = format!("{batch}/{file_name}");
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();

        let owned = path.to_owned();
        let offering = match tokio::task::spawn_blocking(move || tags::read(&owned)).await? {
            Ok(info) => toasty::create!(Offering {
                batch,
                file_name: &file_name,
                staged_path,
                size,
                status: OfferingStatus::Pending,
                title: info.title.clone().unwrap_or(stem),
                artist: info
                    .artist
                    .clone()
                    .unwrap_or_else(|| "Unknown Artist".to_owned()),
                album: info
                    .album
                    .clone()
                    .unwrap_or_else(|| "Unknown Album".to_owned()),
                album_artist: info.album_artist.clone(),
                track_number: info.track_number,
                disc_number: info.disc_number,
                year: info.year,
                genre: info.genre.clone(),
                duration_ms: info.duration_ms,
                created_at: now(),
            }),
            Err(error) => toasty::create!(Offering {
                batch,
                file_name: &file_name,
                staged_path,
                size,
                status: OfferingStatus::Unreadable,
                error: Some(error.to_string()),
                title: stem,
                artist: "",
                album: "",
                duration_ms: 0,
                created_at: now(),
            }),
        }
        .exec(&mut self.db())
        .await?;
        Ok(offering)
    }

    /// Every offering awaiting review, in album order.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn pending(&self) -> Result<Vec<Offering>, OfferingError> {
        let mut offerings = Offering::all().exec(&mut self.db()).await?;
        offerings.sort_by(|a, b| {
            (
                &a.batch,
                &a.album,
                a.disc_number,
                a.track_number,
                &a.file_name,
            )
                .cmp(&(
                    &b.batch,
                    &b.album,
                    b.disc_number,
                    b.track_number,
                    &b.file_name,
                ))
        });
        Ok(offerings)
    }

    /// Absorbs an offering into the treasure. The offering's (possibly
    /// corrected) metadata wins over the file's tags.
    ///
    /// # Errors
    ///
    /// Fails for unknown or unreadable offerings, duplicates, and I/O or
    /// database errors.
    pub async fn accept(&self, id: u64) -> Result<Track, OfferingError> {
        let mut db = self.db();
        let offering = Offering::filter_by_id(id)
            .first()
            .exec(&mut db)
            .await?
            .ok_or(OfferingError::NotFound)?;
        if offering.status == OfferingStatus::Unreadable {
            return Err(OfferingError::Unreadable(
                offering.error.clone().unwrap_or_default(),
            ));
        }

        let path = self.dir.join(&offering.staged_path);
        let owned = path.clone();
        let tagged = tokio::task::spawn_blocking(move || tags::read(&owned)).await??;
        let info = AudioInfo {
            title: Some(offering.title.clone()),
            artist: Some(offering.artist.clone()),
            album: Some(offering.album.clone()),
            album_artist: offering.album_artist.clone(),
            track_number: offering.track_number,
            disc_number: offering.disc_number,
            year: offering.year,
            genre: offering.genre.clone(),
            ..tagged
        };
        let cover = self.batch_cover(&offering.batch).await;

        let track = self
            .treasury
            .ingest(
                &path,
                &info,
                cover.as_ref(),
                TrackOrigin::Offering,
                Claim::offering(),
            )
            .await?;

        let batch = offering.batch.clone();
        offering.delete().exec(&mut db).await?;
        self.clean_up_batch(&batch).await?;
        Ok(track)
    }

    /// Accepts every readable offering of a batch, collecting failures.
    ///
    /// # Errors
    ///
    /// Fails only on database errors; per-offering failures are returned.
    pub async fn accept_batch(
        &self,
        batch: &str,
    ) -> Result<Vec<(Offering, OfferingError)>, OfferingError> {
        let offerings = Offering::filter_by_batch(batch)
            .exec(&mut self.db())
            .await?;
        let mut failures = Vec::new();
        for mut offering in offerings {
            if offering.status != OfferingStatus::Pending {
                continue;
            }
            if let Err(error) = self.accept(offering.id).await {
                // Keep the reason for the review page.
                toasty::update!(offering {
                    error: Some(error.to_string())
                })
                .exec(&mut self.db())
                .await?;
                failures.push((offering, error));
            }
        }
        Ok(failures)
    }

    /// Deletes an offering and its staged file.
    ///
    /// # Errors
    ///
    /// Fails for unknown offerings and on I/O or database errors.
    pub async fn discard(&self, id: u64) -> Result<(), OfferingError> {
        let mut db = self.db();
        let offering = Offering::filter_by_id(id)
            .first()
            .exec(&mut db)
            .await?
            .ok_or(OfferingError::NotFound)?;
        remove_if_exists(&self.dir.join(&offering.staged_path)).await?;
        let batch = offering.batch.clone();
        offering.delete().exec(&mut db).await?;
        self.clean_up_batch(&batch).await
    }

    /// Deletes every offering of a batch.
    ///
    /// # Errors
    ///
    /// Fails on I/O or database errors.
    pub async fn discard_batch(&self, batch: &str) -> Result<(), OfferingError> {
        let dir = self.batch_dir(batch)?;
        Offering::filter_by_batch(batch)
            .delete()
            .exec(&mut self.db())
            .await?;
        match tokio::fs::remove_dir_all(&dir).await {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// An image in the batch named like a cover (`folder.jpg`, ...).
    async fn batch_cover(&self, batch: &str) -> Option<Cover> {
        let dir = self.batch_dir(batch).ok()?;
        for path in list_files(&dir).await.ok()? {
            if is_cover_image(&path)
                && let Some(mime) = Cover::mime_for_path(&path)
                && let Ok(data) = tokio::fs::read(&path).await
                && crate::covers::is_displayable(&data)
            {
                return Some(Cover {
                    data,
                    mime: mime.to_owned(),
                });
            }
        }
        None
    }

    /// Removes the batch directory once no offerings refer to it.
    async fn clean_up_batch(&self, batch: &str) -> Result<(), OfferingError> {
        let remaining = Offering::filter_by_batch(batch)
            .first()
            .exec(&mut self.db())
            .await?;
        if remaining.is_none() {
            match tokio::fs::remove_dir_all(self.batch_dir(batch)?).await {
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error.into()),
                _ => {}
            }
        }
        Ok(())
    }
}

async fn list_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

async fn remove_if_exists(path: &Path) -> io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

fn is_cover_image(path: &Path) -> bool {
    Cover::mime_for_path(path).is_some()
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| {
                COVER_STEMS
                    .iter()
                    .any(|candidate| stem.eq_ignore_ascii_case(candidate))
            })
}

/// `dir/name`, or a numbered variant that does not exist yet.
fn free_name(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let mut candidate = path.clone();
    let mut n = 2;
    while candidate.exists() {
        candidate = layout::numbered(&path, n);
        n += 1;
    }
    candidate
}

/// Extracts the audio files and cover images of a zip archive into `dir`,
/// flattening its folders.
fn extract_zip(archive: &Path, dir: &Path) -> Result<(), OfferingError> {
    let file = std::fs::File::open(archive)?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|error| OfferingError::Archive(error.to_string()))?;

    let mut extracted = 0_u64;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| OfferingError::Archive(error.to_string()))?;
        // `enclosed_name` rejects absolute paths and `..` traversal.
        let Some(inner) = entry.enclosed_name() else {
            continue;
        };
        let Some(name) = inner.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let name = Path::new(name);
        if entry.is_dir() || !(has_extension(name, AUDIO_EXTENSIONS) || is_cover_image(name)) {
            continue;
        }
        extracted = extracted.saturating_add(entry.size());
        if extracted > MAX_EXTRACTED_BYTES {
            return Err(OfferingError::Archive(
                "the archive is too large".to_owned(),
            ));
        }

        let target = free_name(dir, &layout::sanitize(&name.to_string_lossy(), "file"));
        let mut out = std::fs::File::create_new(&target)?;
        io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}
