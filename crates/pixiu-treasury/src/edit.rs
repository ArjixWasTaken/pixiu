//! Changing what the library says about an album. Stored files are shared
//! and never change, so edits live in the database only.

use pixiu_db::{Album, Artist, Db, Track, now, toasty};

use crate::{
    covers,
    ingest::{IngestError, Treasury, find_or_create_artist},
    name_key,
    tags::Cover,
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
    let mut artist = find_or_create_artist(db, &wanted.name, None).await?;
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
    /// Rewrites an album's metadata, and its tracks'. Only the database
    /// changes: stored files keep the tags they arrived with.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
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

        for artist_id in std::iter::once(old_artist).chain(old_track_artists) {
            self.forget_artist_if_empty(&mut db, artist_id).await?;
        }
        tracing::info!(album = album.id, title = %edit.title, "album metadata changed");
        Ok(())
    }

    /// Makes `cover` the album's cover.
    ///
    /// # Errors
    ///
    /// Fails when the cover cannot be written, or on database errors.
    pub async fn replace_cover(&self, album_id: u64, cover: &Cover) -> Result<(), IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        let mut album = Album::get_by_id(&mut db, &album_id).await?;
        let old = album.cover.clone();
        let relative = self.place_image(cover).await?;
        toasty::update!(album {
            cover: Some(relative),
        })
        .exec(&mut db)
        .await?;
        if let Some(old) = old {
            self.release_image(&mut db, &old).await?;
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
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        let relative = self
            .place_image(&Cover {
                data: data.to_vec(),
                mime: mime.to_owned(),
            })
            .await?;
        let mut artist = Artist::get_by_id(&mut db, &artist_id).await?;
        let old = artist.image.clone();
        toasty::update!(artist {
            image: Some(relative),
        })
        .exec(&mut db)
        .await?;
        if let Some(old) = old {
            self.release_image(&mut db, &old).await?;
        }
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
