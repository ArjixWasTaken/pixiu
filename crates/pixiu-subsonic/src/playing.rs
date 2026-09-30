//! `getNowPlaying`, answered from [`NowPlaying`](pixiu_core::playing::NowPlaying).

use std::{collections::HashMap, time::Duration};

use pixiu_core::playing::Song;
use pixiu_db::Track;

use crate::{
    Cx, Failure, catalog,
    response::{Element, Payload},
};

/// A track, as far as "now playing" cares.
pub(crate) fn song(track: &Track) -> Song {
    Song {
        id: track.id,
        length: Duration::from_millis(track.duration_ms),
    }
}

/// `getNowPlaying`: what the caller's clients play.
pub(crate) async fn now_playing(cx: &Cx<'_>) -> Result<Payload, Failure> {
    let current = cx.state.now_playing.current(cx.user.id);
    let ids: Vec<u64> = current.iter().map(|playing| playing.song.id).collect();
    let tracks = catalog::tracks_in_order(&cx.lib, &ids).await?;
    let songs: HashMap<u64, Element> = tracks
        .iter()
        .map(|track| track.id)
        .zip(catalog::songs(&cx.lib, "entry", &tracks).await?)
        .collect();
    let entries: Vec<Element> = current
        .iter()
        .filter_map(|playing| {
            let song = songs.get(&playing.song.id)?.clone();
            Some(
                song.attr("username", playing.username.as_str())
                    .attr("minutesAgo", playing.since.elapsed().as_secs() / 60)
                    .attr("playerId", playing.player_id)
                    .attr("playerName", playing.player.as_str()),
            )
        })
        .collect();
    Ok(Element::new("nowPlaying").list("entry", entries).into())
}
