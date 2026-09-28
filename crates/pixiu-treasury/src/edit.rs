//! Changing what the hoard says about an album: the database, the files'
//! tags, and where the files live, which follows the metadata.

use std::path::{Path, PathBuf};

use pixiu_db::{Album, Artist, Db, Track, now, toasty};

use crate::{
    claims::remove_file,
    covers,
    ingest::{IngestError, Treasury, find_or_create_artist, move_file, path_string},
    layout::TrackLocation,
    name_key,
    refile::is_at,
    tags::{self, Cover, TagChanges},
};

/// An artist to credit: by MusicBrainz id when known, else by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistRef {
    pub name: String,
    pub mbid: Option<String>,
}

/// New metadata for a track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackEdit {
    pub track_id: u64,
    pub title: String,
    /// As printed, e.g. "Artist A feat. Artist B".
    pub artist_credit: String,
    /// The primary artist.
    pub artist: ArtistRef,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    /// MusicBrainz recording id; `None` keeps the current one.
    pub mbid: Option<String>,
    /// `None` keeps the current one.
    pub isrc: Option<String>,
}

/// New metadata for an album, and for some or all of its tracks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumEdit {
    pub album_id: u64,
    pub title: String,
    pub artist: ArtistRef,
    pub year: Option<i32>,
    /// MusicBrainz release id; `None` keeps the current one.
    pub mbid: Option<String>,
    /// `None` keeps the current one.
    pub rg_mbid: Option<String>,
    pub tracks: Vec<TrackEdit>,
}

/// The artist `wanted` names: found by MusicBrainz id, then by name, else
/// created. A found artist learns its MusicBrainz id.
async fn resolve_artist(db: &mut Db, wanted: &ArtistRef) -> Result<Artist, toasty::Error> {
    if let Some(mbid) = &wanted.mbid
        && let Some(artist) = Artist::filter(Artist::fields().mbid().eq(Some(mbid.clone())))
            .first()
            .exec(db)
            .await?
    {
        return Ok(artist);
    }
    let mut artist = find_or_create_artist(db, &wanted.name).await?;
    if artist.mbid.is_none() && wanted.mbid.is_some() {
        toasty::update!(artist {
            mbid: wanted.mbid.clone(),
        })
        .exec(db)
        .await?;
    }
    Ok(artist)
}

impl Treasury {
    /// Rewrites an album's metadata: the rows, every file's tags, and the
    /// files' places in the layout, which follow the new metadata.
    ///
    /// # Errors
    ///
    /// Fails on database errors, or when a file cannot be moved. Tags that
    /// cannot be written are logged and skipped: the database is what
    /// clients see.
    pub async fn edit_album(&self, edit: &AlbumEdit) -> Result<(), IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        let Some(mut album) = Album::filter_by_id(edit.album_id)
            .first()
            .exec(&mut db)
            .await?
        else {
            return Ok(());
        };
        let old_artist = album.artist_id;
        let album_artist = resolve_artist(&mut db, &edit.artist).await?;

        let mut track_artists = Vec::new();
        for track in &edit.tracks {
            track_artists.push(resolve_artist(&mut db, &track.artist).await?);
        }

        let mut tx = db.transaction().await?;
        toasty::update!(album {
            title: edit.title.clone(),
            title_key: name_key(&edit.title),
            artist_id: album_artist.id,
            year: edit.year,
            mbid: edit.mbid.clone().or(album.mbid.clone()),
            rg_mbid: edit.rg_mbid.clone().or(album.rg_mbid.clone()),
        })
        .exec(&mut tx)
        .await?;
        let mut old_track_artists = Vec::new();
        for (change, artist) in edit.tracks.iter().zip(&track_artists) {
            let Some(mut track) = Track::filter_by_id(change.track_id)
                .first()
                .exec(&mut tx)
                .await?
                .filter(|track| track.album_id == album.id)
            else {
                continue;
            };
            old_track_artists.push(track.artist_id);
            toasty::update!(track {
                title: change.title.clone(),
                artist_credit: change.artist_credit.clone(),
                artist_id: artist.id,
                track_number: change.track_number,
                disc_number: change.disc_number,
                year: edit.year,
                mbid: change.mbid.clone().or(track.mbid.clone()),
                isrc: change.isrc.clone().or(track.isrc.clone()),
            })
            .exec(&mut tx)
            .await?;
        }
        tx.commit().await?;

        self.refile_album(&mut db, album.id, true).await?;
        for artist_id in std::iter::once(old_artist).chain(old_track_artists) {
            self.forget_artist_if_empty(&mut db, artist_id).await?;
        }
        tracing::info!(album = album.id, title = %edit.title, "album metadata changed");
        Ok(())
    }

    /// Moves an album's files (and its cover) where the layout wants them,
    /// after writing every track's tags from the database if `retag`.
    pub(crate) async fn refile_album(
        &self,
        db: &mut Db,
        album_id: u64,
        retag: bool,
    ) -> Result<(), IngestError> {
        let album = Album::get_by_id(&mut *db, &album_id).await?;
        let album_artist = Artist::get_by_id(&mut *db, &album.artist_id).await?;
        let tracks = Track::filter_by_album_id(album_id).exec(&mut *db).await?;
        let mut old_dirs = Vec::new();
        let mut new_dir = None;
        for mut track in tracks {
            let artist = Artist::get_by_id(&mut *db, &track.artist_id).await?;
            let file = self.resolve(&track.path);
            let changes = TagChanges {
                title: &track.title,
                artist: &track.artist_credit,
                album: &album.title,
                album_artist: &album_artist.name,
                track_number: track.track_number,
                disc_number: track.disc_number,
                year: album.year,
                recording_mbid: track.mbid.as_deref(),
                release_mbid: album.mbid.as_deref(),
                release_group_mbid: album.rg_mbid.as_deref(),
                artist_mbid: artist.mbid.as_deref(),
                album_artist_mbid: album_artist.mbid.as_deref(),
                isrc: track.isrc.as_deref(),
            };
            if retag && let Err(error) = write_tags(&file, &changes).await {
                tracing::warn!(%error, path = %track.path, "cannot write tags");
            }

            let wanted = self.layout().track_path(TrackLocation {
                album_artist: &album_artist.name,
                artist: &track.artist_credit,
                album: &album.title,
                year: album.year,
                genre: track.genre.as_deref(),
                disc: track.disc_number,
                track: track.track_number,
                title: &track.title,
                suffix: &track.suffix,
            });
            new_dir = wanted.parent().map(Path::to_path_buf);
            if is_at(Path::new(&track.path), &wanted) {
                continue;
            }
            let destination = self.free_path(wanted).await;
            let target = self.root.join(&destination);
            tokio::fs::create_dir_all(target.parent().expect("track paths have a parent")).await?;
            move_file(&file, &target).await?;
            if let Some(dir) = file.parent() {
                old_dirs.push(dir.to_path_buf());
            }
            toasty::update!(track {
                path: path_string(&destination),
            })
            .exec(&mut *db)
            .await?;
        }

        // The cover follows the tracks.
        if let (Some(cover), Some(dir)) = (album.cover.clone(), new_dir)
            && Path::new(&cover).parent() != Some(dir.as_path())
        {
            let name = Path::new(&cover).file_name().expect("covers have a name");
            let destination = self.free_path(dir.join(name)).await;
            let old = self.resolve(&cover);
            move_file(&old, &self.root.join(&destination)).await?;
            covers::forget(&old, &self.cache_dir).await?;
            if let Some(dir) = old.parent() {
                old_dirs.push(dir.to_path_buf());
            }
            let mut album = album;
            toasty::update!(album {
                cover: Some(path_string(&destination)),
            })
            .exec(&mut *db)
            .await?;
        }
        for dir in old_dirs {
            self.remove_empty_dirs(&dir).await;
        }
        Ok(())
    }

    /// Removes `dir` and its parents while they are empty, up to the root.
    pub(crate) async fn remove_empty_dirs(&self, dir: &Path) {
        let mut current = Some(dir.to_path_buf());
        while let Some(dir) = current.filter(|dir| dir.starts_with(&self.root) && *dir != self.root)
        {
            if tokio::fs::remove_dir(&dir).await.is_err() {
                break;
            }
            current = dir.parent().map(Path::to_path_buf);
        }
    }

    /// Makes `cover` the album's cover, on disk and in every track.
    ///
    /// # Errors
    ///
    /// Fails when the cover cannot be written, or on database errors.
    pub async fn replace_cover(&self, album_id: u64, cover: &Cover) -> Result<(), IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        let mut album = Album::get_by_id(&mut db, &album_id).await?;
        let tracks = Track::filter_by_album_id(album_id).exec(&mut db).await?;
        let Some(dir) = tracks
            .first()
            .and_then(|track| Path::new(&track.path).parent().map(Path::to_path_buf))
        else {
            return Ok(());
        };
        if let Some(old) = &album.cover {
            let old = self.resolve(old);
            remove_file(&old).await?;
            covers::forget(&old, &self.cache_dir).await?;
        }
        let relative = self
            .free_path(dir.join(format!("cover.{}", cover.extension())))
            .await;
        tokio::fs::write(self.root.join(&relative), &cover.data).await?;
        toasty::update!(album {
            cover: Some(path_string(&relative)),
        })
        .exec(&mut db)
        .await?;
        for track in tracks {
            let file = self.resolve(&track.path);
            let cover = cover.clone();
            let embedded =
                tokio::task::spawn_blocking(move || tags::embed_cover(&file, &cover)).await?;
            if let Err(error) = embedded {
                tracing::warn!(%error, path = %track.path, "cannot embed the cover");
            }
        }
        Ok(())
    }

    /// Stores a picture of an artist, and records it.
    ///
    /// # Errors
    ///
    /// Fails when the picture is not an image or cannot be written.
    pub async fn set_artist_image(&self, artist_id: u64, data: &[u8]) -> Result<(), IngestError> {
        let mime = covers::mime_of(data).ok_or_else(|| {
            IngestError::Io(std::io::Error::other("the artist picture is not an image"))
        })?;
        let extension = Cover {
            data: Vec::new(),
            mime: mime.to_owned(),
        }
        .extension();
        let relative = PathBuf::from("artists").join(format!("{artist_id}.{extension}"));
        let path = self.cache_dir.join(&relative);
        tokio::fs::create_dir_all(path.parent().expect("a parent")).await?;
        if let Some(old) = Artist::get_by_id(&mut self.db.clone(), &artist_id)
            .await?
            .image
            .filter(|old| Path::new(old) != relative)
        {
            let old = self.cache_dir.join(old);
            remove_file(&old).await?;
            covers::forget(&old, &self.cache_dir).await?;
        }
        tokio::fs::write(&path, data).await?;
        covers::forget(&path, &self.cache_dir).await?;
        let mut artist = Artist::get_by_id(&mut self.db.clone(), &artist_id).await?;
        toasty::update!(artist {
            image: Some(path_string(&relative)),
        })
        .exec(&mut self.db.clone())
        .await?;
        Ok(())
    }

    /// Records what Wikipedia says about an artist, or that it said
    /// nothing.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn set_artist_bio(
        &self,
        artist_id: u64,
        bio: Option<(String, String)>,
    ) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        let mut artist = Artist::get_by_id(&mut db, &artist_id).await?;
        let (bio, bio_url) = bio.map_or((None, None), |(bio, url)| (Some(bio), Some(url)));
        toasty::update!(artist {
            bio,
            bio_url,
            info_fetched_at: Some(now()),
        })
        .exec(&mut db)
        .await
    }
}

async fn write_tags(file: &Path, changes: &TagChanges<'_>) -> Result<(), IngestError> {
    let file = file.to_path_buf();
    let changes = OwnedChanges::from(changes);
    tokio::task::spawn_blocking(move || tags::update(&file, &changes.borrowed()))
        .await?
        .map_err(|error| IngestError::Io(std::io::Error::other(error.to_string())))
}

/// [`TagChanges`] that can move to a blocking thread.
struct OwnedChanges {
    strings: [String; 4],
    numbers: (Option<u32>, Option<u32>, Option<i32>),
    ids: [Option<String>; 6],
}

impl From<&TagChanges<'_>> for OwnedChanges {
    fn from(changes: &TagChanges<'_>) -> Self {
        let owned = |value: Option<&str>| value.map(str::to_owned);
        Self {
            strings: [
                changes.title.to_owned(),
                changes.artist.to_owned(),
                changes.album.to_owned(),
                changes.album_artist.to_owned(),
            ],
            numbers: (changes.track_number, changes.disc_number, changes.year),
            ids: [
                owned(changes.recording_mbid),
                owned(changes.release_mbid),
                owned(changes.release_group_mbid),
                owned(changes.artist_mbid),
                owned(changes.album_artist_mbid),
                owned(changes.isrc),
            ],
        }
    }
}

impl OwnedChanges {
    fn borrowed(&self) -> TagChanges<'_> {
        let [title, artist, album, album_artist] = &self.strings;
        let [recording, release, group, artist_id, album_artist_id, isrc] = &self.ids;
        TagChanges {
            title,
            artist,
            album,
            album_artist,
            track_number: self.numbers.0,
            disc_number: self.numbers.1,
            year: self.numbers.2,
            recording_mbid: recording.as_deref(),
            release_mbid: release.as_deref(),
            release_group_mbid: group.as_deref(),
            artist_mbid: artist_id.as_deref(),
            album_artist_mbid: album_artist_id.as_deref(),
            isrc: isrc.as_deref(),
        }
    }
}
