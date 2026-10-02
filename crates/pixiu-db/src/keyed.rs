//! A library's tracks by [`SourceKey`]: the song on a platform each track
//! was downloaded from, and the other keys of the same song it turned out
//! to be ([`TrackAlias`]).

use std::collections::HashMap;

use crate::{SourceKey, Track, TrackAlias, toasty};

/// How many keys go into one `IN (…)` list.
const CHUNK: usize = 500;

/// `owner`'s track of `key`: downloaded from it, or found to be it.
///
/// # Errors
///
/// Fails on database errors.
pub async fn track_of_key(
    db: &mut dyn toasty::Executor,
    owner: u64,
    key: &SourceKey,
) -> toasty::Result<Option<Track>> {
    let stored = key.as_stored();
    if let Some(track) = Track::filter_by_user_id_and_source_key(owner, &stored)
        .first()
        .exec(&mut *db)
        .await?
    {
        return Ok(Some(track));
    }
    let Some(alias) = TrackAlias::filter_by_user_id_and_source_key(owner, &stored)
        .first()
        .exec(&mut *db)
        .await?
    else {
        return Ok(None);
    };
    Track::filter_by_id(alias.track_id)
        .first()
        .exec(&mut *db)
        .await
}

/// The tracks a library holds among some keys.
#[derive(Debug, Default)]
pub struct Held {
    /// Stored keys to track ids.
    by_key: HashMap<String, u64>,
    tracks: HashMap<u64, Track>,
}

impl Held {
    /// The track of `key`.
    #[must_use]
    pub fn track(&self, key: &SourceKey) -> Option<&Track> {
        self.tracks.get(self.by_key.get(&key.as_stored())?)
    }

    /// The keys held, of those asked for.
    pub fn keys(&self) -> impl Iterator<Item = SourceKey> + '_ {
        self.by_key.keys().filter_map(|stored| stored.parse().ok())
    }

    #[must_use]
    pub fn contains(&self, key: &SourceKey) -> bool {
        self.by_key.contains_key(&key.as_stored())
    }

    /// The tracks, by id.
    #[must_use]
    pub fn into_tracks(self) -> HashMap<u64, Track> {
        self.tracks
    }
}

/// `owner`'s tracks of any of `keys`.
///
/// # Errors
///
/// Fails on database errors.
pub async fn tracks_of_keys(
    db: &mut dyn toasty::Executor,
    owner: u64,
    keys: &[SourceKey],
) -> toasty::Result<Held> {
    let stored: Vec<String> = keys.iter().map(SourceKey::as_stored).collect();
    let mut held = Held::default();
    for chunk in stored.chunks(CHUNK) {
        let tracks = Track::filter(
            Track::fields()
                .user_id()
                .eq(owner)
                .and(Track::fields().source_key().in_list(chunk.to_vec())),
        )
        .exec(&mut *db)
        .await?;
        for track in tracks {
            if let Some(key) = track.source_key.clone() {
                held.by_key.insert(key, track.id);
            }
            held.tracks.insert(track.id, track);
        }
    }

    let rest: Vec<String> = stored
        .iter()
        .filter(|key| !held.by_key.contains_key(*key))
        .cloned()
        .collect();
    let mut aliases = Vec::new();
    for chunk in rest.chunks(CHUNK) {
        aliases.extend(
            TrackAlias::filter(
                TrackAlias::fields()
                    .user_id()
                    .eq(owner)
                    .and(TrackAlias::fields().source_key().in_list(chunk.to_vec())),
            )
            .exec(&mut *db)
            .await?,
        );
    }
    let missing: Vec<u64> = aliases
        .iter()
        .map(|alias| alias.track_id)
        .filter(|id| !held.tracks.contains_key(id))
        .collect();
    for chunk in missing.chunks(CHUNK) {
        for track in Track::filter(Track::fields().id().in_list(chunk.to_vec()))
            .exec(&mut *db)
            .await?
        {
            held.tracks.insert(track.id, track);
        }
    }
    // An alias of a track since deleted holds nothing.
    for alias in aliases {
        if held.tracks.contains_key(&alias.track_id) {
            held.by_key.insert(alias.source_key, alias.track_id);
        }
    }
    Ok(held)
}

/// Notes that `key` is `track`: a download of it turned out to be the
/// track. Nothing to note when the track came from that key; a key noted
/// for a track since deleted moves to this one.
///
/// # Errors
///
/// Fails on database errors.
pub async fn alias(
    db: &mut dyn toasty::Executor,
    track: &Track,
    key: &SourceKey,
) -> toasty::Result<()> {
    let stored = key.as_stored();
    if track.source_key.as_deref() == Some(stored.as_str()) {
        return Ok(());
    }
    match TrackAlias::filter_by_user_id_and_source_key(track.user_id, &stored)
        .first()
        .exec(&mut *db)
        .await?
    {
        Some(existing) if existing.track_id == track.id => Ok(()),
        Some(mut existing) => {
            toasty::update!(existing { track_id: track.id })
                .exec(&mut *db)
                .await
        }
        None => {
            toasty::create!(TrackAlias {
                user_id: track.user_id,
                track_id: track.id,
                source_key: &stored,
            })
            .exec(&mut *db)
            .await?;
            tracing::info!(track = track.id, key = %stored, "noted another key of a track");
            Ok(())
        }
    }
}
