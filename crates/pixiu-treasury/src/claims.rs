//! Claims and orphans: why tracks are kept, and letting go of the ones
//! nothing keeps any more.

use std::{io, path::Path};

use pixiu_db::{
    Album, Annotation, Artist, ClaimKind, Db, PlaylistEntry, Track, TrackClaim, now, toasty,
};

use crate::{
    covers,
    ingest::{Claim, IngestError, Treasury},
};

impl Treasury {
    /// Gives a track `claim`, unless it has it already.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn claim(&self, track_id: u64, claim: &Claim) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        let held = TrackClaim::filter_by_track_id(track_id)
            .exec(&mut db)
            .await?
            .iter()
            .any(|existing| existing.kind == claim.kind && existing.reference == claim.reference);
        if !held {
            toasty::create!(TrackClaim {
                track_id,
                kind: claim.kind,
                reference: claim.reference.clone(),
                created_at: now(),
            })
            .exec(&mut db)
            .await?;
        }
        Ok(())
    }

    /// Takes back every claim of `kind` that refers to `reference`, except
    /// those on tracks `keep` picks. Returns the tracks that lost one.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn release(
        &self,
        kind: ClaimKind,
        reference: &str,
        keep: impl Fn(&Track) -> bool,
    ) -> Result<Vec<u64>, toasty::Error> {
        let mut db = self.db.clone();
        let claims: Vec<TrackClaim> = TrackClaim::filter(TrackClaim::fields().kind().eq(kind))
            .exec(&mut db)
            .await?
            .into_iter()
            .filter(|claim| claim.reference.as_deref() == Some(reference))
            .collect();
        let mut released = Vec::new();
        for claim in claims {
            let track = Track::filter_by_id(claim.track_id)
                .first()
                .exec(&mut db)
                .await?;
            if track.as_ref().is_some_and(&keep) {
                continue;
            }
            released.push(claim.track_id);
            claim.delete().exec(&mut db).await?;
        }
        Ok(released)
    }

    /// Tracks nothing claims, newest first. They stay on disk until the
    /// admin deletes them.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn orphans(&self) -> Result<Vec<Track>, toasty::Error> {
        let mut db = self.db.clone();
        let ids = orphan_ids(&mut db).await?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut tracks = Track::filter(Track::fields().id().in_list(ids))
            .exec(&mut db)
            .await?;
        tracks.sort_by_key(|track| std::cmp::Reverse((track.added_at, track.id)));
        Ok(tracks)
    }

    /// How many orphans there are.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn orphan_count(&self) -> Result<usize, toasty::Error> {
        Ok(orphan_ids(&mut self.db.clone()).await?.len())
    }

    /// Deletes the orphans among `ids`: their files, their rows, and the
    /// albums and artists they leave empty. Claimed tracks are left alone.
    /// Returns how many tracks were deleted.
    ///
    /// # Errors
    ///
    /// Fails on I/O and database errors; tracks deleted before the failure
    /// stay deleted.
    pub async fn delete_orphans(&self, ids: &[u64]) -> Result<usize, IngestError> {
        let _guard = self.lock.lock().await;
        let mut db = self.db.clone();
        let orphans = orphan_ids(&mut db).await?;
        let mut deleted = 0;
        for &id in ids.iter().filter(|id| orphans.contains(id)) {
            let Some(track) = Track::filter_by_id(id).first().exec(&mut db).await? else {
                continue;
            };
            self.delete_track(&mut db, track).await?;
            deleted += 1;
        }
        Ok(deleted)
    }

    async fn delete_track(&self, db: &mut Db, track: Track) -> Result<(), IngestError> {
        let file = self.resolve(&track.path);
        remove_file(&file).await?;
        tracing::info!(track = track.id, path = %track.path, "orphan deleted");

        forget_annotation(db, &format!("tr-{}", track.id)).await?;
        PlaylistEntry::filter_by_track_id(Some(track.id))
            .delete()
            .exec(db)
            .await?;
        let (album_id, artist_id) = (track.album_id, track.artist_id);
        track.delete().exec(db).await?;

        if Track::filter_by_album_id(album_id)
            .first()
            .exec(db)
            .await?
            .is_none()
            && let Some(album) = Album::filter_by_id(album_id).first().exec(db).await?
        {
            let album_artist = album.artist_id;
            if let Some(cover) = &album.cover {
                let cover = self.resolve(cover);
                remove_file(&cover).await?;
                covers::forget(&cover, &self.cache_dir).await?;
            }
            forget_annotation(db, &format!("al-{}", album.id)).await?;
            album.delete().exec(db).await?;
            self.forget_artist_if_empty(db, album_artist).await?;
        }
        self.forget_artist_if_empty(db, artist_id).await?;

        // Directories the deletion left empty go too.
        let mut dir = file.parent();
        while let Some(current) = dir.filter(|dir| dir.starts_with(&self.root) && *dir != self.root)
        {
            if tokio::fs::remove_dir(current).await.is_err() {
                break;
            }
            dir = current.parent();
        }
        Ok(())
    }

    pub(crate) async fn forget_artist_if_empty(
        &self,
        db: &mut Db,
        artist_id: u64,
    ) -> Result<(), IngestError> {
        let used = Album::filter_by_artist_id(artist_id)
            .first()
            .exec(db)
            .await?
            .is_some()
            || Track::filter_by_artist_id(artist_id)
                .first()
                .exec(db)
                .await?
                .is_some();
        if !used && let Some(artist) = Artist::filter_by_id(artist_id).first().exec(db).await? {
            forget_annotation(db, &format!("ar-{}", artist.id)).await?;
            artist.delete().exec(db).await?;
        }
        Ok(())
    }
}

async fn orphan_ids(db: &mut Db) -> Result<Vec<u64>, toasty::Error> {
    let rows = toasty::sql::query(
        "SELECT id FROM tracks WHERE id NOT IN (SELECT track_id FROM track_claims)",
    )
    .exec(db)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| match row {
            toasty::stmt::Value::Record(record) => match record[0] {
                toasty::stmt::Value::I64(id) => u64::try_from(id).ok(),
                _ => None,
            },
            _ => None,
        })
        .collect())
}

async fn forget_annotation(db: &mut Db, item: &str) -> Result<(), toasty::Error> {
    if let Some(annotation) = Annotation::filter_by_item(item).first().exec(db).await? {
        annotation.delete().exec(db).await?;
    }
    Ok(())
}

pub(crate) async fn remove_file(path: &Path) -> io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
