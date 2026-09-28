//! What Subsonic clients are playing right now, kept in memory.
//!
//! Clients announce a song with `scrobble?submission=false`. Those that
//! never do are followed through their `stream` requests instead; clients
//! that announce are not, since they may fetch the next song early.
//!
//! The Subsonic API feeds it and answers `getNowPlaying` from it; the WebUI
//! shows it, and [`subscribe`](NowPlaying::subscribe)s to follow changes.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use jiff::Timestamp;
use tokio::sync::watch;

/// How long past its end a song still counts as playing, for pauses.
const GRACE: Duration = Duration::from_secs(60);

/// A song, as far as "now playing" cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Song {
    /// The track's id.
    pub id: u64,
    pub length: Duration,
}

/// A song being played.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playing {
    pub username: String,
    /// The client's name (the `c` parameter).
    pub player: String,
    pub player_id: u32,
    pub song: Song,
    /// When the song started, for measuring.
    pub since: Instant,
    /// When the song started, for showing.
    pub started_at: Timestamp,
}

impl Playing {
    /// How far into the song the client should be, assuming it has not
    /// paused: Subsonic clients do not report pauses.
    #[must_use]
    pub fn position(&self) -> Duration {
        self.since.elapsed().min(self.song.length)
    }
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
#[derive(Debug, Clone)]
pub struct NowPlaying {
    players: Arc<Mutex<Players>>,
    /// Bumped whenever a client starts a song.
    changes: Arc<watch::Sender<u64>>,
}

impl Default for NowPlaying {
    fn default() -> Self {
        Self {
            players: Arc::default(),
            changes: Arc::new(watch::channel(0).0),
        }
    }
}

impl NowPlaying {
    fn players(&self) -> MutexGuard<'_, Players> {
        self.players.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn start(&self, players: &mut Players, username: &str, player: &str, song: Song) {
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
                started_at: Timestamp::now(),
            },
        );
        self.changes.send_modify(|version| *version += 1);
    }

    /// A client announced that it plays `song`.
    pub fn announced(&self, username: &str, player: &str, song: Song) {
        let mut players = self.players();
        players.announcing.insert(player.to_owned());
        self.start(&mut players, username, player, song);
    }

    /// A client asked for `song`'s audio.
    pub fn streamed(&self, username: &str, player: &str, song: Song) {
        let mut players = self.players();
        if !players.announcing.contains(player) {
            self.start(&mut players, username, player, song);
        }
    }

    /// What is playing: songs started less than their length (and a
    /// minute's grace for pauses) ago, the latest first.
    #[must_use]
    pub fn current(&self) -> Vec<Playing> {
        let mut players = self.players();
        players
            .playing
            .retain(|_, playing| playing.since.elapsed() < playing.song.length + GRACE);
        let mut current: Vec<Playing> = players.playing.values().cloned().collect();
        current.sort_by_key(|playing| std::cmp::Reverse(playing.since));
        current
    }

    /// When the soonest current song stops counting as playing, if any is:
    /// the list changes then without a client saying so.
    #[must_use]
    pub fn next_expiry(&self) -> Option<Instant> {
        self.current()
            .iter()
            .map(|playing| playing.since + playing.song.length + GRACE)
            .min()
    }

    /// Follows clients starting songs. Songs also end on their own; see
    /// [`next_expiry`](Self::next_expiry).
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }
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
    fn listeners_hear_songs_start() {
        let now_playing = NowPlaying::default();
        let changes = now_playing.subscribe();
        assert!(!changes.has_changed().unwrap());
        assert_eq!(now_playing.next_expiry(), None);
        now_playing.announced("admin", "phone", song(1));
        assert!(changes.has_changed().unwrap());
        assert!(now_playing.next_expiry().is_some());
        let playing = &now_playing.current()[0];
        assert!(playing.position() <= playing.song.length);
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
