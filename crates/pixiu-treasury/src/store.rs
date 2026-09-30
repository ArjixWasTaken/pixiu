//! The store: audio files and pictures kept once, named by their content,
//! and shared by everything that holds them.
//!
//! ```text
//! .store/audio/ab/<sha256>.flac
//! .store/images/cd/<sha256>.jpg
//! ```
//!
//! Stored files never change; edits live in the database. A file goes when
//! nothing points at it any more. The store sits inside the treasure, so
//! moving a file in is a rename; `.store` cannot clash with an artist
//! folder of the old layout, whose names never start with a dot.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use pixiu_db::{Album, Artist, AudioFile, Db, Setting, Track, now, toasty};
use sha2::{Digest, Sha256};

use crate::{
    claims::remove_file,
    covers,
    ingest::{IngestError, Treasury, path_string},
    tags::Cover,
};

/// The store's directory, relative to the treasure.
pub const STORE: &str = ".store";

/// Recorded once the files filed before the store existed are adopted.
const ADOPTED: &str = "store.adopted";

/// Where audio with this content lives, relative to the treasure.
pub fn audio_path(sha256: &str, suffix: &str) -> String {
    format!(
        "{STORE}/audio/{}/{sha256}.{}",
        &sha256[..2],
        suffix.to_ascii_lowercase()
    )
}

/// Where a picture with this content lives, relative to the treasure.
fn image_path(sha256: &str, extension: &str) -> String {
    format!("{STORE}/images/{}/{sha256}.{extension}", &sha256[..2])
}

/// Hex-encoded SHA-256 of a file's content. Blocking.
fn hash_file_blocking(path: &Path) -> io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Hex-encoded SHA-256 of a file's content.
pub(crate) async fn hash_file(path: &Path) -> io::Result<String> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || hash_file_blocking(&path))
        .await
        .map_err(io::Error::other)?
}

/// How a file enters the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Entry {
    /// The source goes away (unless the store has the content already).
    Move,
    /// The source stays until the caller removes it: a hard link, or a copy
    /// where links are impossible.
    Link,
}

/// What adopting the files filed before the store existed did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Adoption {
    /// Audio files moved into the store.
    pub files: usize,
    /// Audio files whose content the store had already: the copy went.
    pub duplicates: usize,
    /// Tracks whose file is missing; they are left as they are.
    pub missing: usize,
    /// Album covers and artist pictures moved into the store.
    pub pictures: usize,
    /// Leftover copies of stored content deleted from the old folders.
    pub swept: usize,
    /// Files in the old folders that the store does not have; left alone.
    pub kept: usize,
}

impl Adoption {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl Treasury {
    /// Puts the file at `source` into the store at `relative`. Nothing is
    /// written when the store has it already: the name is the content.
    /// Copies go through a temporary file, so a crash never leaves half a
    /// file under a stored name.
    pub(crate) async fn put(&self, source: &Path, relative: &str, entry: Entry) -> io::Result<()> {
        let target = self.resolve(relative);
        if tokio::fs::try_exists(&target).await? {
            if entry == Entry::Move {
                remove_file(source).await?;
            }
            return Ok(());
        }
        tokio::fs::create_dir_all(target.parent().expect("stored paths have a parent")).await?;
        let placed = match entry {
            Entry::Move => tokio::fs::rename(source, &target).await,
            Entry::Link => tokio::fs::hard_link(source, &target).await,
        };
        match placed {
            Ok(()) => Ok(()),
            // A crash after linking but before recording: the link is there.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            // Different filesystems (e.g. staging on another Docker volume),
            // or no hard links: copy.
            Err(_) => {
                copy_atomically(source, &target).await?;
                if entry == Entry::Move {
                    remove_file(source).await?;
                }
                Ok(())
            }
        }
    }

    /// Stores a picture, returning its path relative to the treasure.
    pub(crate) async fn place_image(&self, cover: &Cover) -> io::Result<String> {
        let sha256 = hex::encode(Sha256::digest(&cover.data));
        let relative = image_path(&sha256, cover.extension());
        let target = self.resolve(&relative);
        if !tokio::fs::try_exists(&target).await? {
            let parent = target.parent().expect("stored paths have a parent");
            tokio::fs::create_dir_all(parent).await?;
            let temp = temp_name(&target);
            tokio::fs::write(&temp, &cover.data).await?;
            tokio::fs::rename(&temp, &target).await?;
        }
        Ok(relative)
    }

    /// Deletes an audio file once no track plays it. Call with the lock
    /// held, after the track rows that used it are gone.
    pub(crate) async fn release_file(&self, db: &mut Db, file_id: u64) -> Result<(), IngestError> {
        if Track::filter_by_file_id(file_id)
            .first()
            .exec(db)
            .await?
            .is_some()
        {
            return Ok(());
        }
        if let Some(file) = AudioFile::filter_by_id(file_id).first().exec(db).await? {
            let path = self.resolve(&file.path);
            file.delete().exec(db).await?;
            remove_file(&path).await?;
            tracing::info!(file = file_id, "stored file deleted: nothing plays it");
        }
        Ok(())
    }

    /// Deletes a stored picture once no album or artist shows it. Call with
    /// the lock held, after the rows that used it changed.
    pub(crate) async fn release_image(&self, db: &mut Db, relative: &str) -> Result<(), IngestError> {
        if !relative.starts_with(STORE) || self.image_in_use(db, relative).await? {
            return Ok(());
        }
        let path = self.resolve(relative);
        remove_file(&path).await?;
        covers::forget(&path, &self.cache_dir).await?;
        Ok(())
    }

    async fn image_in_use(&self, db: &mut Db, relative: &str) -> Result<bool, toasty::Error> {
        let path = Some(relative.to_owned());
        Ok(Album::filter(Album::fields().cover().eq(path.clone()))
            .first()
            .exec(db)
            .await?
            .is_some()
            || Artist::filter(Artist::fields().image().eq(path))
                .first()
                .exec(db)
                .await?
                .is_some())
    }

    /// Deletes what nothing points at: audio files no track plays (e.g. a
    /// download that failed after its file was stored), pictures no album
    /// or artist shows, and temporary files of interrupted copies. Returns
    /// how many files went.
    ///
    /// # Errors
    ///
    /// Fails on I/O and database errors.
    pub async fn collect_garbage(&self) -> Result<usize, IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        let mut removed = 0;

        let unplayed = toasty::sql::query(
            "SELECT id FROM audio_files WHERE id NOT IN (SELECT file_id FROM tracks)",
        )
        .exec(&mut db)
        .await?;
        for id in crate::claims::ids(unplayed) {
            self.release_file(&mut db, id).await?;
            removed += 1;
        }

        let stored: std::collections::HashSet<String> = AudioFile::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .map(|file| file.path)
            .collect();
        for path in files_under(&self.resolve(&format!("{STORE}/audio"))).await? {
            let relative = self.relative(&path);
            if !stored.contains(&relative) {
                remove_file(&path).await?;
                removed += 1;
            }
        }
        for path in files_under(&self.resolve(&format!("{STORE}/images"))).await? {
            let relative = self.relative(&path);
            if is_temp(&path) || !self.image_in_use(&mut db, &relative).await? {
                remove_file(&path).await?;
                covers::forget(&path, &self.cache_dir).await?;
                removed += 1;
            }
        }
        if removed > 0 {
            tracing::info!(removed, "deleted stored files nothing uses");
        }
        Ok(removed)
    }

    fn relative(&self, path: &Path) -> String {
        path_string(path.strip_prefix(&self.root).unwrap_or(path))
    }

    /// Moves the files filed before the store existed into it: every
    /// track's audio file, album covers, and artist pictures (then in the
    /// cache directory). Each file is linked into the store, recorded, and
    /// only then removed from its old place, so the database always names a
    /// file that exists; running it again after a crash carries on. Last,
    /// the old folders lose copies of stored content and empty directories.
    /// Files the store does not have are left alone.
    ///
    /// # Errors
    ///
    /// Fails on I/O and database errors; what was adopted stays adopted.
    pub async fn adopt_legacy(&self) -> Result<Adoption, IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        let mut done = Adoption::default();

        let legacy = Track::filter(Track::fields().file_id().eq(0))
            .exec(&mut db)
            .await?;
        if !legacy.is_empty() {
            tracing::info!(tracks = legacy.len(), "moving files into the store");
        }
        for (index, track) in legacy.into_iter().enumerate() {
            self.adopt_track(&mut db, track, &mut done).await?;
            if (index + 1) % 250 == 0 {
                tracing::info!(done = index + 1, "still moving files into the store");
            }
        }

        for mut album in Album::all().exec(&mut db).await? {
            let Some(cover) = album.cover.clone().filter(|cover| !cover.starts_with(STORE)) else {
                continue;
            };
            let cover = self
                .adopt_picture(&self.resolve(&cover), &mut done)
                .await?;
            toasty::update!(album { cover }).exec(&mut db).await?;
        }
        for mut artist in Artist::all().exec(&mut db).await? {
            let Some(image) = artist.image.clone().filter(|image| !image.starts_with(STORE)) else {
                continue;
            };
            let image = self
                .adopt_picture(&self.cache_dir.join(image), &mut done)
                .await?;
            toasty::update!(artist { image }).exec(&mut db).await?;
        }

        if Setting::filter_by_key(ADOPTED)
            .first()
            .exec(&mut db)
            .await?
            .is_none()
        {
            self.sweep(&mut db, &mut done).await?;
            toasty::create!(Setting {
                key: ADOPTED,
                value: now().to_string(),
            })
            .exec(&mut db)
            .await?;
        }
        if !done.is_empty() {
            tracing::info!(?done, "files moved into the store");
        }
        Ok(done)
    }

    async fn adopt_track(
        &self,
        db: &mut Db,
        mut track: Track,
        done: &mut Adoption,
    ) -> Result<(), IngestError> {
        let old = self.resolve(&track.path);
        let sha256 = match hash_file(&old).await {
            Ok(sha256) => sha256,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                tracing::error!(track = track.id, path = %track.path, "the track's file is missing");
                done.missing += 1;
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        let size = tokio::fs::metadata(&old).await?.len();

        let mut tx = db.transaction().await?;
        let file = match AudioFile::filter_by_sha256(&sha256)
            .first()
            .exec(&mut tx)
            .await?
        {
            Some(file) => {
                done.duplicates += 1;
                file
            }
            None => {
                let relative = audio_path(&sha256, &track.suffix);
                self.put(&old, &relative, Entry::Link).await?;
                done.files += 1;
                toasty::create!(AudioFile {
                    sha256,
                    path: relative,
                    size,
                    suffix: track.suffix.to_ascii_lowercase(),
                    content_type: track.content_type.clone(),
                    duration_ms: track.duration_ms,
                    bitrate: track.bitrate,
                    sample_rate: track.sample_rate,
                    channels: track.channels,
                    bit_depth: track.bit_depth,
                    ytm_video_id: track.ytm_video_id.clone(),
                    created_at: now(),
                })
                .exec(&mut tx)
                .await?
            }
        };
        toasty::update!(track {
            file_id: file.id,
            path: file.path.clone(),
            size: file.size,
        })
        .exec(&mut tx)
        .await?;
        tx.commit().await?;
        remove_file(&old).await?;
        Ok(())
    }

    /// Stores the picture at `old` and removes it there; `None` when it is
    /// missing or unreadable, so the album or artist can get a new one.
    async fn adopt_picture(
        &self,
        old: &Path,
        done: &mut Adoption,
    ) -> Result<Option<String>, IngestError> {
        let data = match tokio::fs::read(old).await {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                tracing::warn!(path = %old.display(), "a picture is missing; it will be fetched again");
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let Some(mime) = covers::mime_of(&data).or_else(|| Cover::mime_for_path(old)) else {
            tracing::warn!(path = %old.display(), "not a picture; it will be fetched again");
            return Ok(None);
        };
        let relative = self
            .place_image(&Cover {
                data,
                mime: mime.to_owned(),
            })
            .await?;
        remove_file(old).await?;
        covers::forget(old, &self.cache_dir).await?;
        done.pictures += 1;
        Ok(Some(relative))
    }

    /// Clears the old folders: files whose content is stored go (earlier
    /// adoptions or crashes can leave copies), others stay and are logged,
    /// and empty directories go.
    async fn sweep(&self, db: &mut Db, done: &mut Adoption) -> Result<(), IngestError> {
        let store = self.resolve(STORE);
        for path in files_under(&self.root).await? {
            if path.starts_with(&store) {
                continue;
            }
            let sha256 = hash_file(&path).await?;
            let stored = AudioFile::filter_by_sha256(&sha256)
                .first()
                .exec(db)
                .await?
                .is_some()
                || self.image_stored(&sha256).await;
            if stored {
                remove_file(&path).await?;
                done.swept += 1;
            } else {
                tracing::info!(path = %path.display(), "left in place: not part of the library");
                done.kept += 1;
            }
        }
        remove_empty_dirs(&self.root, &store).await;
        Ok(())
    }

    async fn image_stored(&self, sha256: &str) -> bool {
        let Ok(mut entries) = tokio::fs::read_dir(
            self.resolve(&format!("{STORE}/images/{}", &sha256[..2])),
        )
        .await
        else {
            return false;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.file_name().to_string_lossy().starts_with(sha256) {
                return true;
            }
        }
        false
    }
}

/// Copies `from` to `to` through a temporary file beside `to`, synced
/// before it takes the name.
async fn copy_atomically(from: &Path, to: &Path) -> io::Result<()> {
    let temp = temp_name(to);
    tokio::fs::copy(from, &temp).await?;
    tokio::fs::File::open(&temp).await?.sync_all().await?;
    tokio::fs::rename(&temp, to).await
}

fn temp_name(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .expect("stored paths have a name")
        .to_string_lossy();
    path.with_file_name(format!(".{name}.tmp"))
}

fn is_temp(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().ends_with(".tmp"))
}

/// Every file below `dir`, depth first; nothing when it does not exist.
async fn files_under(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        while let Some(entry) = entries.next_entry().await? {
            let kind = entry.file_type().await?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    Ok(files)
}

/// Removes the empty directories below `root`, deepest first, except
/// `keep`.
async fn remove_empty_dirs(root: &Path, keep: &Path) {
    let mut dirs = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path != keep && entry.file_type().await.is_ok_and(|kind| kind.is_dir()) {
                dirs.push(path.clone());
                pending.push(path);
            }
        }
    }
    dirs.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));
    for dir in dirs {
        let _ = tokio::fs::remove_dir(&dir).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_names_follow_the_content() {
        let sha = "ab".repeat(32);
        assert_eq!(
            audio_path(&sha, "FLAC"),
            format!(".store/audio/ab/{sha}.flac")
        );
        assert_eq!(image_path(&sha, "jpg"), format!(".store/images/ab/{sha}.jpg"));
    }
}
