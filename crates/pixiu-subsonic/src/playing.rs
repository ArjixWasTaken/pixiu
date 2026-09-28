//! What clients are playing right now (`getNowPlaying`), kept in memory.
//!
//! Clients announce a song with `scrobble?submission=false`. Those that
//! never do are followed through their `stream` requests instead; clients
//! that announce are not, since they may fetch the next song early.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use pixiu_db::Track;

use crate::{
    Failure, SubsonicState, catalog,
    response::{Element, Payload},
};

/// A song, as far as "now playing" cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Song {
    pub id: u64,
    pub length: Duration,
}

impl From<&Track> for Song {
    fn from(track: &Track) -> Self {
        Self {
            id: track.id,
            length: Duration::from_millis(track.duration_ms),
        }
    }
}

/// A song being played.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Playing {
    pub username: String,
    /// The client's name (the `c` parameter).
    pub player: String,
    pub player_id: u32,
    pub song: Song,
    pub since: Instant,
}

#[derive(Debug, Default)]
struct Players {
    /// By client name.
    playing: HashMap<String, Playing>,
    ids: HashMap<String, u32>,
    /// Clients that announce what they play.
    announcing: HashSet<String>,
}

/// The songs clients are playing.
#[derive(Debug, Clone, Default)]
pub struct NowPlaying(Arc<Mutex<Players>>);

impl NowPlaying {
    fn players(&self) -> MutexGuard<'_, Players> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn start(players: &mut Players, username: &str, player: &str, song: Song) {
        let next = u32::try_from(players.ids.len() + 1).unwrap_or(u32::MAX);
        let player_id = *players.ids.entry(player.to_owned()).or_insert(next);
        players.playing.insert(
            player.to_owned(),
            Playing {
                username: username.to_owned(),
                player: player.to_owned(),
                player_id,
                song,
                since: Instant::now(),
            },
        );
    }

    /// A client announced that it plays `song`.
    pub(crate) fn announced(&self, username: &str, player: &str, song: Song) {
        let mut players = self.players();
        players.announcing.insert(player.to_owned());
        Self::start(&mut players, username, player, song);
    }

    /// A client asked for `song`'s audio.
    pub(crate) fn streamed(&self, username: &str, player: &str, song: Song) {
        let mut players = self.players();
        if !players.announcing.contains(player) {
            Self::start(&mut players, username, player, song);
        }
    }

    /// What is playing: songs started less than their length (and a
    /// minute's grace for pauses) ago, the latest first.
    pub(crate) fn current(&self) -> Vec<Playing> {
        let mut players = self.players();
        players.playing.retain(|_, playing| {
            playing.since.elapsed() < playing.song.length + Duration::from_secs(60)
        });
        let mut current: Vec<Playing> = players.playing.values().cloned().collect();
        current.sort_by_key(|playing| std::cmp::Reverse(playing.since));
        current
    }
}

/// `getNowPlaying`.
pub(crate) async fn now_playing(state: &SubsonicState) -> Result<Payload, Failure> {
    let mut db = state.db.clone();
    let current = state.now_playing.current();
    let ids: Vec<u64> = current.iter().map(|playing| playing.song.id).collect();
    let tracks = catalog::tracks_in_order(&mut db, &ids).await?;
    let songs: HashMap<u64, Element> = tracks
        .iter()
        .map(|track| track.id)
        .zip(catalog::songs(&mut db, "entry", &tracks).await?)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn song(id: u64) -> Song {
        Song {
            id,
            length: Duration::from_secs(180),
        }
    }

    #[test]
    fn announcing_clients_are_not_followed_by_their_streams() {
        let now_playing = NowPlaying::default();
        now_playing.streamed("admin", "web", song(1));
        now_playing.streamed("admin", "phone", song(2));
        now_playing.announced("admin", "phone", song(3));
        // The phone fetches its next song early; it said what it plays.
        now_playing.streamed("admin", "phone", song(4));

        let mut current: Vec<(String, u64, u32)> = now_playing
            .current()
            .into_iter()
            .map(|playing| (playing.player, playing.song.id, playing.player_id))
            .collect();
        current.sort_unstable();
        assert_eq!(
            current,
            [("phone".to_owned(), 3, 2), ("web".to_owned(), 1, 1)]
        );
    }

    #[test]
    fn finished_songs_are_not_playing() {
        let now_playing = NowPlaying::default();
        now_playing.streamed(
            "admin",
            "web",
            Song {
                id: 1,
                length: Duration::ZERO,
            },
        );
        {
            let mut players = now_playing.players();
            let playing = players.playing.get_mut("web").unwrap();
            playing.since -= Duration::from_secs(61);
        }
        assert!(now_playing.current().is_empty());
    }
}
