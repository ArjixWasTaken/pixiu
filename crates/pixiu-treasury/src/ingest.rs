//! Filing tracks into the treasure.

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use pixiu_db::{Album, Artist, ClaimKind, Db, Track, TrackClaim, TrackOrigin, now, toasty};
use tokio::sync::Mutex;

use crate::{
    layout::{self, TrackLocation},
    name_key,
    tags::{AudioInfo, Cover},
};

const UNKNOWN_ARTIST: &str = "Unknown Artist";
const UNKNOWN_ALBUM: &str = "Unknown Album";
const UNTITLED: &str = "Untitled";

/// Why a track is kept; recorded as a [`TrackClaim`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub kind: ClaimKind,
    pub reference: Option<String>,
}

impl Claim {
    /// The claim of a track the admin uploaded.
    #[must_use]
    pub fn offering() -> Self {
        Self {
            kind: ClaimKind::Offering,
            reference: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("the hoard already holds this track")]
    Duplicate { track_id: u64 },
    #[error("failed to file the track: {0}")]
    Io(#[from] io::Error),
    #[error("database error: {0}")]
    Db(#[from] toasty::Error),
}

/// The music library on disk and in the database.
#[derive(Clone)]
pub struct Treasury {
    db: Db,
    root: PathBuf,
    cache_dir: PathBuf,
    /// Ingests are serialized so concurrent ones cannot create the same
    /// artist or album twice, or race for a file name.
    lock: Arc<Mutex<()>>,
}

impl Treasury {
    #[must_use]
    pub fn new(db: Db, root: impl Into<PathBuf>, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            db,
            root: root.into(),
            cache_dir: cache_dir.into(),
            lock: Arc::default(),
        }
    }

    #[must_use]
    pub fn db(&self) -> Db {
        self.db.clone()
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// The absolute path of a treasure-relative path stored in the database.
    #[must_use]
    pub fn resolve(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Moves the audio file at `source` into the treasure and records it.
    ///
    /// `fallback_cover` is used for the album when the file has no embedded
    /// cover. On failure the file is left at `source`.
    ///
    /// # Errors
    ///
    /// Fails when the album already holds the same track, or on I/O and
    /// database errors.
    pub async fn ingest(
        &self,
        source: &Path,
        info: &AudioInfo,
        fallback_cover: Option<&Cover>,
        origin: TrackOrigin,
        claim: Claim,
    ) -> Result<Track, IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();

        let credit = info.artist.as_deref().unwrap_or(UNKNOWN_ARTIST);
        let primary = primary_artist(credit);
        let album_artist_name = info.album_artist.as_deref().unwrap_or(primary);
        let title = info.title.as_deref().unwrap_or(UNTITLED);

        let album_artist = find_or_create_artist(&mut db, album_artist_name).await?;
        let artist_id = if name_key(primary) == album_artist.name_key {
            album_artist.id
        } else {
            find_or_create_artist(&mut db, primary).await?.id
        };
        let album = find_or_create_album(&mut db, &album_artist, info).await?;

        let title_key = name_key(title);
        if let Some(duplicate) = Track::filter_by_album_id(album.id)
            .exec(&mut db)
            .await?
            .into_iter()
            .find(|track| {
                name_key(&track.title) == title_key
                    && track.track_number == info.track_number
                    && track.disc_number == info.disc_number
            })
        {
            return Err(IngestError::Duplicate {
                track_id: duplicate.id,
            });
        }

        let relative = self
            .free_path(layout::track_path(TrackLocation {
                album_artist: &album_artist.name,
                album: &album.title,
                year: album.year,
                disc: info.disc_number,
                track: info.track_number,
                title,
                suffix: &info.suffix,
            }))
            .await;
        let destination = self.root.join(&relative);
        tokio::fs::create_dir_all(destination.parent().expect("track paths have a parent")).await?;
        move_file(source, &destination).await?;

        let recorded = self
            .record(
                &mut db,
                album,
                artist_id,
                &relative,
                &destination,
                info,
                fallback_cover,
                origin,
                claim,
            )
            .await;
        if recorded.is_err()
            && let Err(error) = move_file(&destination, source).await
        {
            tracing::error!(
                %error,
                path = %destination.display(),
                "failed to restore a file after a failed ingest"
            );
        }
        recorded
    }

    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        db: &mut Db,
        mut album: Album,
        artist_id: u64,
        relative: &Path,
        destination: &Path,
        info: &AudioInfo,
        fallback_cover: Option<&Cover>,
        origin: TrackOrigin,
        claim: Claim,
    ) -> Result<Track, IngestError> {
        let size = tokio::fs::metadata(destination).await?.len();
        let album_dir = relative.parent().expect("track paths have a parent");
        let cover = match (&album.cover, info.cover.as_ref().or(fallback_cover)) {
            (None, Some(cover)) => Some(self.write_cover(album_dir, cover).await?),
            _ => None,
        };

        let mut tx = db.transaction().await?;
        if let Some(cover) = cover {
            toasty::update!(album { cover: Some(cover) })
                .exec(&mut tx)
                .await?;
        }
        let track = toasty::create!(Track {
            album_id: album.id,
            artist_id,
            title: info.title.as_deref().unwrap_or(UNTITLED),
            artist_credit: info.artist.as_deref().unwrap_or(UNKNOWN_ARTIST),
            track_number: info.track_number,
            disc_number: info.disc_number,
            year: info.year,
            genre: info.genre.clone(),
            duration_ms: info.duration_ms,
            bitrate: info.bitrate,
            sample_rate: info.sample_rate,
            channels: info.channels,
            bit_depth: info.bit_depth,
            path: path_string(relative),
            size,
            suffix: &info.suffix,
            content_type: &info.content_type,
            mbid: info.mbid.clone(),
            isrc: info.isrc.clone(),
            origin,
            added_at: now(),
        })
        .exec(&mut tx)
        .await?;
        toasty::create!(TrackClaim {
            track_id: track.id,
            kind: claim.kind,
            reference: claim.reference,
            created_at: now(),
        })
        .exec(&mut tx)
        .await?;
        tx.commit().await?;

        tracing::info!(track = track.id, path = %track.path, "track added to the hoard");
        Ok(track)
    }

    /// Writes `cover` into the album directory, returning its relative path.
    async fn write_cover(&self, album_dir: &Path, cover: &Cover) -> io::Result<String> {
        let relative = self
            .free_path(album_dir.join(format!("cover.{}", cover.extension())))
            .await;
        tokio::fs::write(self.root.join(&relative), &cover.data).await?;
        Ok(path_string(&relative))
    }

    /// `relative`, or a numbered variant of it that is not taken on disk.
    async fn free_path(&self, relative: PathBuf) -> PathBuf {
        let mut candidate = relative.clone();
        for n in 2.. {
            if !tokio::fs::try_exists(self.root.join(&candidate))
                .await
                .unwrap_or(true)
            {
                break;
            }
            candidate = layout::numbered(&relative, n);
        }
        candidate
    }
}

async fn find_or_create_artist(db: &mut Db, name: &str) -> Result<Artist, toasty::Error> {
    let key = name_key(name);
    if let Some(artist) = Artist::filter_by_name_key(&key).first().exec(db).await? {
        return Ok(artist);
    }
    toasty::create!(Artist {
        name,
        name_key: key,
        created_at: now(),
    })
    .exec(db)
    .await
}

async fn find_or_create_album(
    db: &mut Db,
    album_artist: &Artist,
    info: &AudioInfo,
) -> Result<Album, toasty::Error> {
    let title = info.album.as_deref().unwrap_or(UNKNOWN_ALBUM);
    let key = name_key(title);
    let existing = Album::filter_by_artist_id(album_artist.id)
        .exec(db)
        .await?
        .into_iter()
        .find(|album| album.title_key == key);
    if let Some(album) = existing {
        return Ok(album);
    }
    toasty::create!(Album {
        title,
        title_key: key,
        artist_id: album_artist.id,
        year: info.year,
        genre: info.genre.clone(),
        mbid: info.album_mbid.clone(),
        created_at: now(),
    })
    .exec(db)
    .await
}

/// The main artist of a credit like "Artist A feat. Artist B".
fn primary_artist(credit: &str) -> &str {
    const MARKERS: &[&str] = &[" feat. ", " feat ", " ft. ", " ft ", " featuring "];
    let cut = credit
        .char_indices()
        .map(|(index, _)| index)
        .find(|&index| {
            MARKERS.iter().any(|marker| {
                credit
                    .get(index..index + marker.len())
                    .is_some_and(|window| window.eq_ignore_ascii_case(marker))
            })
        });
    match cut {
        Some(index) if index > 0 => credit[..index].trim(),
        _ => credit.trim(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_str()
        .expect("treasure paths are built from UTF-8 strings")
        .to_owned()
}

/// Renames `from` to `to`, copying when they are on different filesystems
/// (e.g. separate Docker volumes for staging and the treasure).
pub(crate) async fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match tokio::fs::rename(from, to).await {
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            tokio::fs::copy(from, to).await?;
            tokio::fs::remove_file(from).await
        }
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::primary_artist;

    #[test]
    fn primary_artists_drop_featured_guests() {
        assert_eq!(primary_artist("Test Artist feat. Guest"), "Test Artist");
        assert_eq!(primary_artist("A FT. B"), "A");
        assert_eq!(primary_artist("A featuring B & C"), "A");
        assert_eq!(primary_artist("Simon & Garfunkel"), "Simon & Garfunkel");
        assert_eq!(primary_artist("Feat. Somebody"), "Feat. Somebody");
        assert_eq!(primary_artist("Ünï feat. Çø"), "Ünï");
    }
}
