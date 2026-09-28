//! The file layout setting, and moving the hoard when it changes.

use std::{collections::HashMap, path::Path, sync::PoisonError};

use pixiu_db::{Album, Artist, Setting, Track, toasty};

use crate::{
    ingest::{IngestError, Treasury},
    layout::{Template, TrackLocation},
};

/// Whether a file at `path` is where `wanted` says, or a numbered
/// alternative of it (`x (2).flac`).
pub(crate) fn is_at(path: &Path, wanted: &Path) -> bool {
    if path == wanted {
        return true;
    }
    let (Some(stem), Some(wanted_stem)) = (path.file_stem(), wanted.file_stem()) else {
        return false;
    };
    let (stem, wanted_stem) = (stem.to_string_lossy(), wanted_stem.to_string_lossy());
    path.parent() == wanted.parent()
        && path.extension() == wanted.extension()
        && stem
            .strip_prefix(wanted_stem.as_ref())
            .and_then(|rest| rest.strip_prefix(" ("))
            .and_then(|rest| rest.strip_suffix(')'))
            .is_some_and(|n| n.parse::<u32>().is_ok())
}

/// The setting that holds the layout template.
const LAYOUT: &str = "layout";

impl Treasury {
    /// Where new tracks are filed.
    #[must_use]
    pub fn layout(&self) -> Template {
        self.layout
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Reads the layout setting, keeping the default when there is none
    /// (or it no longer parses).
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn load_layout(&self) -> Result<(), toasty::Error> {
        let stored = Setting::filter_by_key(LAYOUT)
            .first()
            .exec(&mut self.db.clone())
            .await?;
        if let Some(setting) = stored {
            match Template::parse(&setting.value) {
                Ok(template) => {
                    *self.layout.write().unwrap_or_else(PoisonError::into_inner) = template;
                }
                Err(error) => {
                    tracing::warn!(%error, template = %setting.value, "ignoring the stored file layout");
                }
            }
        }
        Ok(())
    }

    /// Files new tracks by `template` from now on. Files already in the
    /// treasure stay where they are until [`Treasury::refile_all`].
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn set_layout(&self, template: Template) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        match Setting::filter_by_key(LAYOUT).first().exec(&mut db).await? {
            Some(mut setting) => {
                toasty::update!(setting {
                    value: template.as_str().to_owned(),
                })
                .exec(&mut db)
                .await?;
            }
            None => {
                toasty::create!(Setting {
                    key: LAYOUT,
                    value: template.as_str().to_owned(),
                })
                .exec(&mut db)
                .await?;
            }
        }
        *self.layout.write().unwrap_or_else(PoisonError::into_inner) = template;
        Ok(())
    }

    /// How many tracks are not where the layout wants them (the layout
    /// changed since they were filed). Tracks renamed to dodge a name
    /// collision, `x (2).flac`, count as in place.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn misplaced(&self) -> Result<usize, toasty::Error> {
        let mut db = self.db.clone();
        let layout = self.layout();
        let albums: HashMap<u64, Album> = Album::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .map(|album| (album.id, album))
            .collect();
        let artists: HashMap<u64, Artist> = Artist::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .map(|artist| (artist.id, artist))
            .collect();
        let tracks = Track::all().exec(&mut db).await?;
        Ok(tracks
            .iter()
            .filter(|track| {
                let Some(album) = albums.get(&track.album_id) else {
                    return false;
                };
                let wanted = layout.track_path(TrackLocation {
                    album_artist: artists
                        .get(&album.artist_id)
                        .map_or("", |artist| artist.name.as_str()),
                    artist: &track.artist_credit,
                    album: &album.title,
                    year: album.year,
                    genre: track.genre.as_deref(),
                    disc: track.disc_number,
                    track: track.track_number,
                    title: &track.title,
                    suffix: &track.suffix,
                });
                !is_at(Path::new(&track.path), &wanted)
            })
            .count())
    }

    /// Moves every file where the layout wants it, album by album (other
    /// changes to the library may happen in between). `progress` hears how
    /// many albums are done out of how many. Returns the number of albums.
    ///
    /// # Errors
    ///
    /// Stops at the first album that cannot be moved; the albums before it
    /// stay moved.
    pub async fn refile_all(&self, progress: impl Fn(usize, usize)) -> Result<usize, IngestError> {
        let mut db = self.db.clone();
        let albums: Vec<u64> = Album::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .map(|album| album.id)
            .collect();
        for (done, album_id) in albums.iter().enumerate() {
            {
                let _guard = self.lock.lock().await;
                // Deleted while this ran: nothing to move.
                if Album::filter_by_id(*album_id)
                    .first()
                    .exec(&mut db)
                    .await?
                    .is_some()
                {
                    self.refile_album(&mut db, *album_id, false).await?;
                }
            }
            progress(done + 1, albums.len());
        }
        tracing::info!(albums = albums.len(), layout = %self.layout(), "treasure refiled");
        Ok(albums.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_names_are_in_place() {
        let wanted = Path::new("A/B/01 Song.flac");
        assert!(is_at(Path::new("A/B/01 Song.flac"), wanted));
        assert!(is_at(Path::new("A/B/01 Song (2).flac"), wanted));
        assert!(!is_at(Path::new("A/B/01 Song (two).flac"), wanted));
        assert!(!is_at(Path::new("A/C/01 Song (2).flac"), wanted));
        assert!(!is_at(Path::new("A/B/01 Song.mp3"), wanted));
    }
}
