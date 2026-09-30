//! Claims and orphans: why tracks are kept, and letting go of the ones
//! nothing keeps any more.

use std::{io, path::Path};

use pixiu_db::{
    Album, Annotation, Artist, ClaimKind, Db, Lyrics, PlaylistEntry, ReleaseReason, ReleasedClaim,
    Track, TrackClaim, now, toasty,
};

use crate::ingest::{Claim, IngestError, Treasury};

/// Why claims are being released, remembered for the orphans they leave.
#[derive(Debug, Clone, Copy)]
pub struct Release<'a> {
    pub reason: ReleaseReason,
    /// The watch or playlist, by name: it may be gone by the time anyone
    /// asks.
    pub source_name: Option<&'a str>,
}

impl Treasury {
    /// Gives a track `claim`, unless it has it already.
    ///
    /// A claimed track is wanted again, so the record of claims it lost is
    /// dropped.
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
        ReleasedClaim::filter_by_track_id(track_id)
            .delete()
            .exec(&mut db)
            .await?;
        Ok(())
    }

    /// Takes back every claim of `kind` that refers to `reference`, except
    /// those on tracks `keep` picks, and records `why` for each. Returns the
    /// tracks that lost one.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn release(
        &self,
        kind: ClaimKind,
        reference: &str,
        why: Release<'_>,
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
            toasty::create!(ReleasedClaim {
                track_id: claim.track_id,
                kind,
                reason: why.reason,
                source_name: why.source_name.map(str::to_owned),
                released_at: now(),
            })
            .exec(&mut db)
            .await?;
            claim.delete().exec(&mut db).await?;
        }
        Ok(released)
    }

    /// Tracks nothing claims, newest first. They stay until the admin
    /// deletes them.
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

    /// The latest released claim of each track among `ids`: why it became
    /// an orphan, when known.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn released_claims(
        &self,
        ids: &[u64],
    ) -> Result<std::collections::HashMap<u64, ReleasedClaim>, toasty::Error> {
        let mut latest = std::collections::HashMap::new();
        if ids.is_empty() {
            return Ok(latest);
        }
        let rows = ReleasedClaim::filter(ReleasedClaim::fields().track_id().in_list(ids.to_vec()))
            .exec(&mut self.db.clone())
            .await?;
        for row in rows {
            match latest.get(&row.track_id) {
                Some(seen) if (seen.released_at, seen.id) >= (row.released_at, row.id) => {}
                _ => {
                    latest.insert(row.track_id, row);
                }
            }
        }
        Ok(latest)
    }

    /// How many orphans there are.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn orphan_count(&self) -> Result<usize, toasty::Error> {
        Ok(orphan_ids(&mut self.db.clone()).await?.len())
    }

    /// Deletes the orphans among `ids`: their rows, the albums and artists
    /// they leave empty, and stored files nothing else plays. Claimed
    /// tracks are left alone. Returns how many tracks were deleted.
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
        tracing::info!(track = track.id, "orphan deleted");
        forget_annotation(db, &format!("tr-{}", track.id)).await?;
        PlaylistEntry::filter_by_track_id(Some(track.id))
            .delete()
            .exec(db)
            .await?;
        ReleasedClaim::filter_by_track_id(track.id)
            .delete()
            .exec(db)
            .await?;
        Lyrics::filter_by_track_id(track.id).delete().exec(db).await?;
        let (album_id, artist_id, file_id) = (track.album_id, track.artist_id, track.file_id);
        track.delete().exec(db).await?;
        self.release_file(db, file_id).await?;

        if Track::filter_by_album_id(album_id)
            .first()
            .exec(db)
            .await?
            .is_none()
            && let Some(album) = Album::filter_by_id(album_id).first().exec(db).await?
        {
            let (album_artist, cover) = (album.artist_id, album.cover.clone());
            forget_annotation(db, &format!("al-{}", album.id)).await?;
            album.delete().exec(db).await?;
            if let Some(cover) = cover {
                self.release_image(db, &cover).await?;
            }
            self.forget_artist_if_empty(db, album_artist).await?;
        }
        self.forget_artist_if_empty(db, artist_id).await?;
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
            let image = artist.image.clone();
            forget_annotation(db, &format!("ar-{}", artist.id)).await?;
            artist.delete().exec(db).await?;
            if let Some(image) = image {
                self.release_image(db, &image).await?;
            }
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
    Ok(ids(rows))
}

/// The first column of each row, as ids.
pub(crate) fn ids(rows: impl IntoIterator<Item = toasty::stmt::Value>) -> Vec<u64> {
    rows.into_iter()
        .filter_map(|row| match row {
            toasty::stmt::Value::Record(record) => match record[0] {
                toasty::stmt::Value::I64(id) => u64::try_from(id).ok(),
                _ => None,
            },
            _ => None,
        })
        .collect()
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
