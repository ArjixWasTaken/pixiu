//! Singles: releases the player shows as songs rather than albums, and the
//! catch-alls of songs that came with no album.

use pixiu_db::{Album, Setting, Track, now, toasty};

use crate::{Treasury, ingest::UNKNOWN_ALBUM, name_key};

/// The album downloaded songs without one are filed under, per artist. It
/// is no release: MusicBrainz looks its songs up one by one.
pub const SINGLES: &str = "Singles";

/// The setting that records [`Treasury::mark_singles`] ran.
const MARKED_SINGLES: &str = "repair.singles";

impl Treasury {
    /// Once per server: marks as singles the albums filed before píxiū told
    /// singles apart. Those are the catch-alls of songs without an album
    /// (downloads' "Singles", uploads' "Unknown Album"), and albums of one
    /// song named like it. Returns how many it marked.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn mark_singles(&self) -> Result<usize, toasty::Error> {
        let mut db = self.db.clone();
        if Setting::filter_by_key(MARKED_SINGLES)
            .first()
            .exec(&mut db)
            .await?
            .is_some()
        {
            return Ok(0);
        }
        let mut marked = 0;
        for mut album in Album::all().exec(&mut db).await? {
            if album.single {
                continue;
            }
            let catch_all =
                album.mbid.is_none() && [SINGLES, UNKNOWN_ALBUM].contains(&album.title.as_str());
            let named_like_its_song = !catch_all && {
                let tracks = Track::filter_by_album_id(album.id).exec(&mut db).await?;
                matches!(tracks.as_slice(), [track] if name_key(&track.title) == album.title_key)
            };
            if catch_all || named_like_its_song {
                toasty::update!(album { single: true })
                    .exec(&mut db)
                    .await?;
                marked += 1;
            }
        }
        toasty::create!(Setting {
            key: MARKED_SINGLES,
            value: now().to_string(),
        })
        .exec(&mut db)
        .await?;
        if marked > 0 {
            tracing::info!(
                albums = marked,
                "marked singles, shown as songs from now on"
            );
        }
        Ok(marked)
    }
}
