//! Adding tracks to the library.

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use pixiu_db::{
    Album, Artist, AudioFile, ClaimKind, Db, SourceKey, Track, TrackClaim, TrackOrigin, keyed, now,
    toasty,
};
use tokio::sync::Mutex;

use crate::{
    claims::remove_file,
    name_key,
    store::{self, Entry},
    tags::{AudioInfo, Cover},
};

const UNKNOWN_ARTIST: &str = "Unknown Artist";
const UNKNOWN_ALBUM: &str = "Unknown Album";
const UNTITLED: &str = "Untitled";

/// Where a track comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub origin: TrackOrigin,
    /// For downloads, the song on the platform.
    pub source_key: Option<SourceKey>,
    /// For downloads, its album on the platform.
    pub album_key: Option<SourceKey>,
    /// For downloads, its album artist on the platform.
    pub artist_key: Option<SourceKey>,
    /// For uploads, the file name it was offered under.
    pub source_name: Option<String>,
    /// For uploads, the zip archive it was unpacked from.
    pub source_archive: Option<String>,
}

impl Provenance {
    /// A file the admin uploaded as `file_name`, perhaps inside `archive`.
    #[must_use]
    pub fn offering(file_name: impl Into<String>, archive: Option<String>) -> Self {
        Self {
            origin: TrackOrigin::Offering,
            source_key: None,
            album_key: None,
            artist_key: None,
            source_name: Some(file_name.into()),
            source_archive: archive,
        }
    }

    /// A download of `song`, from its album and album artist on the same
    /// platform, when they are known.
    #[must_use]
    pub fn download(
        song: SourceKey,
        album: Option<SourceKey>,
        album_artist: Option<SourceKey>,
    ) -> Self {
        Self {
            origin: TrackOrigin::Download,
            source_key: Some(song),
            album_key: album,
            artist_key: album_artist,
            source_name: None,
            source_archive: None,
        }
    }
}

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
    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// The music library on disk and in the database.
#[derive(Clone)]
pub struct Treasury {
    pub(crate) db: Db,
    pub(crate) root: PathBuf,
    pub(crate) cache_dir: PathBuf,
    /// Changes to the library are serialized, so concurrent ones cannot
    /// create the same artist or album twice, or delete a stored file
    /// another is about to use.
    pub(crate) lock: Arc<Mutex<()>>,
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

    /// Puts the audio file at `source` into the store and records a track
    /// playing it in `owner`'s library. When the store holds the same
    /// content already (in anyone's library), the track shares that file
    /// and `source` is removed.
    ///
    /// `fallback_cover` is used for the album when the file has no embedded
    /// cover. On failure the file is left at `source`.
    ///
    /// # Errors
    ///
    /// Fails when the owner's library already holds the same track, or on
    /// I/O and database errors.
    pub async fn ingest(
        &self,
        owner: u64,
        source: &Path,
        info: &AudioInfo,
        fallback_cover: Option<&Cover>,
        provenance: Provenance,
        claim: Claim,
    ) -> Result<Track, IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();

        let sha256 = store::hash_file(source).await?;
        let stored = AudioFile::filter_by_sha256(&sha256)
            .first()
            .exec(&mut db)
            .await?;
        if let Some(existing) = self
            .duplicate(
                &mut db,
                owner,
                &provenance,
                stored.as_ref().map(|file| file.id),
            )
            .await?
        {
            return Err(note_key(&mut db, existing, &provenance).await?);
        }

        let (album, artist_id) = self.place(&mut db, owner, info, &provenance).await?;

        let file = match stored {
            Some(file) => StoredFile::Shared(file),
            None => {
                let relative = store::audio_path(&sha256, &info.suffix);
                self.put(source, &relative, Entry::Move).await?;
                StoredFile::New { sha256, relative }
            }
        };
        let placed = match &file {
            StoredFile::New { relative, .. } => Some(self.resolve(relative)),
            StoredFile::Shared(_) => None,
        };
        let recorded = self
            .record(
                &mut db,
                album,
                artist_id,
                file,
                info,
                fallback_cover,
                provenance,
                claim,
            )
            .await;
        match (&recorded, placed) {
            // The store had the content: the new copy is not needed.
            (Ok(_), None) => {
                if let Err(error) = remove_file(source).await {
                    tracing::warn!(%error, path = %source.display(), "cannot remove a stored copy");
                }
            }
            (Err(_), Some(placed)) => {
                if let Err(error) = move_file(&placed, source).await {
                    tracing::error!(
                        %error,
                        path = %placed.display(),
                        "failed to restore a file after a failed ingest"
                    );
                }
            }
            _ => {}
        }
        recorded
    }

    /// Records a track in `owner`'s library playing `file`, which the store
    /// holds already (e.g. another user downloaded the same video): nothing
    /// is downloaded or copied.
    ///
    /// # Errors
    ///
    /// Fails when the owner's library already holds the same track, or on
    /// database errors.
    pub async fn ingest_stored(
        &self,
        owner: u64,
        file: AudioFile,
        info: &AudioInfo,
        provenance: Provenance,
        claim: Claim,
    ) -> Result<Track, IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        if let Some(existing) = self
            .duplicate(&mut db, owner, &provenance, Some(file.id))
            .await?
        {
            return Err(note_key(&mut db, existing, &provenance).await?);
        }
        let (album, artist_id) = self.place(&mut db, owner, info, &provenance).await?;
        self.record(
            &mut db,
            album,
            artist_id,
            StoredFile::Shared(file),
            info,
            None,
            provenance,
            claim,
        )
        .await
    }

    /// A track of `owner`'s that is the same download (or another video
    /// found to be it), or plays the same file.
    async fn duplicate(
        &self,
        db: &mut Db,
        owner: u64,
        provenance: &Provenance,
        file_id: Option<u64>,
    ) -> Result<Option<Track>, toasty::Error> {
        if let Some(key) = &provenance.source_key
            && let Some(existing) = keyed::track_of_key(db, owner, key).await?
        {
            return Ok(Some(existing));
        }
        match file_id {
            Some(file_id) => {
                Track::filter(
                    Track::fields()
                        .file_id()
                        .eq(file_id)
                        .and(Track::fields().user_id().eq(owner)),
                )
                .first()
                .exec(db)
                .await
            }
            None => Ok(None),
        }
    }

    /// [`Self::find_place`], noting the download's video when the album
    /// holds the same track already.
    async fn place(
        &self,
        db: &mut Db,
        owner: u64,
        info: &AudioInfo,
        provenance: &Provenance,
    ) -> Result<(Album, u64), IngestError> {
        match self.find_place(db, owner, info, provenance).await {
            Err(IngestError::Duplicate { track_id }) => {
                match Track::filter_by_id(track_id).first().exec(&mut *db).await? {
                    Some(existing) => Err(note_key(db, existing, provenance).await?),
                    None => Err(IngestError::Duplicate { track_id }),
                }
            }
            placed => placed,
        }
    }

    /// The album (and track artist) a track described by `info` belongs to
    /// in `owner`'s library, created when missing.
    ///
    /// # Errors
    ///
    /// Fails when the album holds the same track already.
    async fn find_place(
        &self,
        db: &mut Db,
        owner: u64,
        info: &AudioInfo,
        provenance: &Provenance,
    ) -> Result<(Album, u64), IngestError> {
        let credit = info.artist.as_deref().unwrap_or(UNKNOWN_ARTIST);
        let primary = info
            .artists
            .first()
            .map_or_else(|| primary_artist(credit), String::as_str);
        let album_artist_name = info.album_artist.as_deref().unwrap_or(primary);
        let title = info.title.as_deref().unwrap_or(UNTITLED);

        let album_artist =
            find_or_create_artist(db, owner, album_artist_name, provenance.artist_key.as_ref())
                .await?;
        let artist_id = if name_key(primary) == album_artist.name_key {
            album_artist.id
        } else {
            find_or_create_artist(db, owner, primary, None).await?.id
        };
        let album =
            find_or_create_album(db, &album_artist, info, provenance.album_key.as_ref()).await?;

        let title_key = name_key(title);
        if let Some(duplicate) = Track::filter_by_album_id(album.id)
            .exec(db)
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

        Ok((album, artist_id))
    }

    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        db: &mut Db,
        mut album: Album,
        artist_id: u64,
        file: StoredFile,
        info: &AudioInfo,
        fallback_cover: Option<&Cover>,
        provenance: Provenance,
        claim: Claim,
    ) -> Result<Track, IngestError> {
        let cover = match (&album.cover, info.cover.as_ref().or(fallback_cover)) {
            (None, Some(cover)) => Some(self.place_image(cover).await?),
            _ => None,
        };

        let mut tx = db.transaction().await?;
        if let Some(cover) = cover {
            toasty::update!(album { cover: Some(cover) })
                .exec(&mut tx)
                .await?;
        }
        let file = match file {
            StoredFile::Shared(file) => file,
            StoredFile::New { sha256, relative } => {
                let size = tokio::fs::metadata(self.resolve(&relative)).await?.len();
                toasty::create!(AudioFile {
                    sha256,
                    path: relative,
                    size,
                    suffix: info.suffix.to_ascii_lowercase(),
                    content_type: &info.content_type,
                    duration_ms: info.duration_ms,
                    bitrate: info.bitrate,
                    sample_rate: info.sample_rate,
                    channels: info.channels,
                    bit_depth: info.bit_depth,
                    source_key: provenance.source_key.as_ref().map(SourceKey::as_stored),
                    created_at: now(),
                })
                .exec(&mut tx)
                .await?
            }
        };
        let track = toasty::create!(Track {
            user_id: album.user_id,
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
            file_id: file.id,
            path: file.path.clone(),
            size: file.size,
            suffix: file.suffix.clone(),
            content_type: file.content_type.clone(),
            mbid: info.mbid.clone(),
            isrc: info.isrc.clone(),
            source_key: provenance.source_key.as_ref().map(SourceKey::as_stored),
            origin: provenance.origin,
            source_name: provenance.source_name,
            source_archive: provenance.source_archive,
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

        tracing::info!(
            track = track.id,
            file = file.id,
            "track added to the library"
        );
        Ok(track)
    }
}

/// The file a new track plays.
enum StoredFile {
    /// Content the store holds already.
    Shared(AudioFile),
    /// Just put into the store; its row is written with the track's.
    New { sha256: String, relative: String },
}

/// The artist known on a platform as `source`, else the one named `name`,
/// which learns that key if it did not know one; else a new artist.
pub(crate) async fn find_or_create_artist(
    db: &mut Db,
    owner: u64,
    name: &str,
    source: Option<&SourceKey>,
) -> Result<Artist, toasty::Error> {
    let source = source.map(SourceKey::as_stored);
    if let Some(stored) = &source
        && let Some(artist) = Artist::filter_by_user_id_and_source_key(owner, Some(stored.clone()))
            .first()
            .exec(db)
            .await?
    {
        return Ok(artist);
    }
    let key = name_key(name);
    if let Some(mut artist) = Artist::filter_by_user_id_and_name_key(owner, &key)
        .first()
        .exec(db)
        .await?
    {
        if artist.source_key.is_none()
            && let Some(stored) = source
        {
            toasty::update!(artist {
                source_key: Some(stored),
            })
            .exec(db)
            .await?;
        }
        return Ok(artist);
    }
    toasty::create!(Artist {
        user_id: owner,
        name,
        name_key: key,
        source_key: source,
        created_at: now(),
    })
    .exec(db)
    .await
}

impl Treasury {
    /// Tells `owner`'s artist named `name` who it is on a platform (`key`),
    /// if it knows no key yet and no other artist of theirs has that one.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn learn_artist_key(
        &self,
        owner: u64,
        name: &str,
        key: &SourceKey,
    ) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        let stored = key.as_stored();
        if Artist::filter_by_user_id_and_source_key(owner, Some(stored.clone()))
            .first()
            .exec(&mut db)
            .await?
            .is_some()
        {
            return Ok(());
        }
        if let Some(mut artist) = Artist::filter_by_user_id_and_name_key(owner, name_key(name))
            .first()
            .exec(&mut db)
            .await?
            && artist.source_key.is_none()
        {
            toasty::update!(artist {
                source_key: Some(stored),
            })
            .exec(&mut db)
            .await?;
        }
        Ok(())
    }
}

/// A download that turned out to be `existing`: its key is the track's too,
/// so it is not fetched again.
async fn note_key(
    db: &mut Db,
    existing: Track,
    provenance: &Provenance,
) -> Result<IngestError, toasty::Error> {
    if let Some(key) = &provenance.source_key {
        keyed::alias(db, &existing, key).await?;
    }
    Ok(IngestError::Duplicate {
        track_id: existing.id,
    })
}

/// The album a track belongs to: by its key on the platform when known
/// (names can differ between releases), then by artist and title.
async fn find_or_create_album(
    db: &mut Db,
    album_artist: &Artist,
    info: &AudioInfo,
    source: Option<&SourceKey>,
) -> Result<Album, toasty::Error> {
    let source = source.map(SourceKey::as_stored);
    if let Some(stored) = &source
        && let Some(album) = Album::filter_by_user_id_and_source_key(album_artist.user_id, stored)
            .first()
            .exec(db)
            .await?
    {
        return Ok(album);
    }

    let title = info.album.as_deref().unwrap_or(UNKNOWN_ALBUM);
    let key = name_key(title);
    let existing = Album::filter_by_artist_id(album_artist.id)
        .exec(db)
        .await?
        .into_iter()
        .find(|album| album.title_key == key);
    if let Some(mut album) = existing {
        if album.source_key.is_none()
            && let Some(stored) = source
        {
            toasty::update!(album {
                source_key: Some(stored)
            })
            .exec(db)
            .await?;
        }
        return Ok(album);
    }
    toasty::create!(Album {
        user_id: album_artist.user_id,
        title,
        title_key: key,
        artist_id: album_artist.id,
        year: info.year,
        genre: info.genre.clone(),
        mbid: info.album_mbid.clone(),
        source_key: source,
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

pub(crate) fn path_string(path: &Path) -> String {
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
