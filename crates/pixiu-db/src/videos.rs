//! A library's tracks by YouTube Music video: the video each track was
//! downloaded from, and the other videos of the same song it turned out to
//! be ([`TrackAlias`]).

use std::collections::HashMap;

use crate::{Track, TrackAlias, toasty};

/// How many ids go into one `IN (…)` list.
const CHUNK: usize = 500;

/// `owner`'s track of `video_id`: downloaded from it, or found to be it.
///
/// # Errors
///
/// Fails on database errors.
pub async fn track_of_video(
    db: &mut dyn toasty::Executor,
    owner: u64,
    video_id: &str,
) -> toasty::Result<Option<Track>> {
    if let Some(track) = Track::filter_by_user_id_and_ytm_video_id(owner, video_id)
        .first()
        .exec(&mut *db)
        .await?
    {
        return Ok(Some(track));
    }
    let Some(alias) = TrackAlias::filter_by_user_id_and_ytm_video_id(owner, video_id)
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

/// The tracks a library holds among some videos.
#[derive(Debug, Default)]
pub struct Held {
    by_video: HashMap<String, u64>,
    tracks: HashMap<u64, Track>,
}

impl Held {
    /// The track of `video_id`.
    #[must_use]
    pub fn track(&self, video_id: &str) -> Option<&Track> {
        self.tracks.get(self.by_video.get(video_id)?)
    }

    /// The videos held, of those asked for.
    pub fn videos(&self) -> impl Iterator<Item = &str> {
        self.by_video.keys().map(String::as_str)
    }

    #[must_use]
    pub fn contains(&self, video_id: &str) -> bool {
        self.by_video.contains_key(video_id)
    }

    /// The tracks, by id.
    #[must_use]
    pub fn into_tracks(self) -> HashMap<u64, Track> {
        self.tracks
    }
}

/// `owner`'s tracks of any of `videos`.
///
/// # Errors
///
/// Fails on database errors.
pub async fn tracks_of_videos(
    db: &mut dyn toasty::Executor,
    owner: u64,
    videos: &[String],
) -> toasty::Result<Held> {
    let mut held = Held::default();
    for chunk in videos.chunks(CHUNK) {
        let tracks = Track::filter(
            Track::fields()
                .user_id()
                .eq(owner)
                .and(Track::fields().ytm_video_id().in_list(chunk.to_vec())),
        )
        .exec(&mut *db)
        .await?;
        for track in tracks {
            if let Some(video_id) = track.ytm_video_id.clone() {
                held.by_video.insert(video_id, track.id);
            }
            held.tracks.insert(track.id, track);
        }
    }

    let rest: Vec<String> = videos
        .iter()
        .filter(|video| !held.by_video.contains_key(*video))
        .cloned()
        .collect();
    let mut aliases = Vec::new();
    for chunk in rest.chunks(CHUNK) {
        aliases.extend(
            TrackAlias::filter(
                TrackAlias::fields()
                    .user_id()
                    .eq(owner)
                    .and(TrackAlias::fields().ytm_video_id().in_list(chunk.to_vec())),
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
            held.by_video.insert(alias.ytm_video_id, alias.track_id);
        }
    }
    Ok(held)
}

/// Notes that `video_id` is `track`: a download of it turned out to be the
/// track. Nothing to note when the track came from that video; a video
/// noted for a track since deleted moves to this one.
///
/// # Errors
///
/// Fails on database errors.
pub async fn alias(
    db: &mut dyn toasty::Executor,
    track: &Track,
    video_id: &str,
) -> toasty::Result<()> {
    if track.ytm_video_id.as_deref() == Some(video_id) {
        return Ok(());
    }
    match TrackAlias::filter_by_user_id_and_ytm_video_id(track.user_id, video_id)
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
                ytm_video_id: video_id,
            })
            .exec(&mut *db)
            .await?;
            tracing::info!(track = track.id, video_id, "noted another video of a track");
            Ok(())
        }
    }
}
