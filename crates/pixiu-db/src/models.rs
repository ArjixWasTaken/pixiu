//! Database models.
//!
//! Changing anything here requires a new migration; see `pixiu-migrate`.

use jiff::Timestamp;

/// The admin account. píxiū is single-user, so exactly one row exists once
/// the first-run setup has completed.
#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub username: String,

    /// PHC-formatted argon2id hash of the password.
    pub password_hash: String,

    /// The password sealed with the instance key. Subsonic token
    /// authentication (`md5(password + salt)`) cannot work from a hash, so
    /// the password is captured whenever the admin sets or types it.
    pub subsonic_secret: Option<String>,

    pub created_at: Timestamp,

    #[has_many]
    pub web_sessions: toasty::Deferred<Vec<WebSession>>,

    #[has_many]
    pub api_keys: toasty::Deferred<Vec<ApiKey>>,
}

/// A logged-in WebUI session. Only the SHA-256 of the session token is
/// stored, so a leaked database cannot be replayed as cookies.
#[derive(Debug, toasty::Model)]
pub struct WebSession {
    #[key]
    #[auto]
    pub id: u64,

    /// Hex-encoded SHA-256 of the session token.
    #[unique]
    pub token_hash: String,

    #[index]
    pub user_id: u64,

    #[belongs_to]
    pub user: toasty::Deferred<User>,

    pub created_at: Timestamp,

    pub expires_at: Timestamp,
}

/// An OpenSubsonic API key (`apiKeyAuthentication`). Only its SHA-256 is
/// stored; the key itself is shown once, when created.
#[derive(Debug, toasty::Model)]
pub struct ApiKey {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    #[belongs_to]
    pub user: toasty::Deferred<User>,

    /// A label chosen by the admin, e.g. the client using the key.
    pub name: String,

    /// Hex-encoded SHA-256 of the key.
    #[unique]
    pub key_hash: String,

    pub created_at: Timestamp,

    pub last_used_at: Option<Timestamp>,
}

impl ApiKey {
    /// The stored form of a key: hex-encoded SHA-256.
    #[must_use]
    pub fn hash(key: &str) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(key.as_bytes()))
    }
}

#[derive(Debug, toasty::Model)]
pub struct Artist {
    #[key]
    #[auto]
    pub id: u64,

    pub name: String,

    /// `name` normalized for matching (see `pixiu_treasury::name_key`).
    #[index]
    pub name_key: String,

    pub mbid: Option<String>,

    /// A short biography, from Wikipedia.
    pub bio: Option<String>,

    /// Where the biography comes from, for attribution.
    pub bio_url: Option<String>,

    /// A picture of the artist, relative to the cache directory.
    pub image: Option<String>,

    /// When the biography and picture were last looked for.
    pub info_fetched_at: Option<Timestamp>,

    pub created_at: Timestamp,

    #[has_many]
    pub albums: toasty::Deferred<Vec<Album>>,

    #[has_many]
    pub tracks: toasty::Deferred<Vec<Track>>,
}

#[derive(Debug, toasty::Model)]
pub struct Album {
    #[key]
    #[auto]
    pub id: u64,

    pub title: String,

    /// `title` normalized for matching.
    pub title_key: String,

    /// The album artist.
    #[index]
    pub artist_id: u64,

    #[belongs_to]
    pub artist: toasty::Deferred<Artist>,

    pub year: Option<i32>,

    pub genre: Option<String>,

    /// MusicBrainz release id.
    pub mbid: Option<String>,

    /// MusicBrainz release group id.
    pub rg_mbid: Option<String>,

    /// How looking the album up on MusicBrainz went; `None` until tried.
    pub enrichment: Option<Enrichment>,

    /// Releases that might be the album, as JSON, when no match was
    /// certain enough (see [`Enrichment::Review`]).
    pub candidates: Option<String>,

    pub enriched_at: Option<Timestamp>,

    /// The album's YouTube Music browse id, when it was hunted there.
    #[index]
    pub ytm_browse_id: Option<String>,

    /// The cover image, relative to the treasure directory.
    pub cover: Option<String>,

    pub created_at: Timestamp,

    #[has_many]
    pub tracks: toasty::Deferred<Vec<Track>>,
}

/// How looking an album up on MusicBrainz went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum Enrichment {
    /// Matched to a release; tags follow it.
    Matched,
    /// Some releases might be it; the admin picks one.
    Review,
    /// MusicBrainz knows nothing like it.
    Unmatched,
}

/// How a track entered the hoard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum TrackOrigin {
    /// Uploaded by the admin.
    Offering,
    /// Downloaded from a streaming platform.
    Download,
}

#[derive(Debug, toasty::Model)]
pub struct Track {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub album_id: u64,

    #[belongs_to]
    pub album: toasty::Deferred<Album>,

    /// The primary track artist.
    #[index]
    pub artist_id: u64,

    #[belongs_to]
    pub artist: toasty::Deferred<Artist>,

    pub title: String,

    /// The artist credit as tagged, e.g. "Artist A feat. Artist B".
    pub artist_credit: String,

    pub track_number: Option<u32>,

    pub disc_number: Option<u32>,

    pub year: Option<i32>,

    pub genre: Option<String>,

    pub duration_ms: u64,

    /// Kilobits per second.
    pub bitrate: Option<u32>,

    pub sample_rate: Option<u32>,

    pub channels: Option<u8>,

    pub bit_depth: Option<u8>,

    /// The audio file, relative to the treasure directory.
    #[unique]
    pub path: String,

    pub size: u64,

    /// File extension, e.g. `flac`.
    pub suffix: String,

    pub content_type: String,

    /// MusicBrainz recording id.
    pub mbid: Option<String>,

    pub isrc: Option<String>,

    /// The YouTube Music video id the track was downloaded from.
    #[index]
    pub ytm_video_id: Option<String>,

    pub origin: TrackOrigin,

    pub added_at: Timestamp,

    #[has_many]
    pub claims: toasty::Deferred<Vec<TrackClaim>>,
}

/// Why a track is kept. A track without claims is an orphan: kept on disk,
/// listed for the admin to delete or keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum ClaimKind {
    Offering,
    ManualGrab,
    WatchPlaylist,
    WatchArtist,
    LocalPlaylist,
    Starred,
}

#[derive(Debug, toasty::Model)]
pub struct TrackClaim {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub track_id: u64,

    #[belongs_to]
    pub track: toasty::Deferred<Track>,

    pub kind: ClaimKind,

    /// What the claim refers to, e.g. the watched playlist's id.
    pub reference: Option<String>,

    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum OfferingStatus {
    /// Tagged and waiting for the admin's review.
    Pending,
    /// Not a readable audio file; see `error`.
    Unreadable,
}

/// An uploaded file waiting to be absorbed into the treasure. Accepted
/// offerings become tracks and their rows are deleted.
#[derive(Debug, toasty::Model)]
pub struct Offering {
    #[key]
    #[auto]
    pub id: u64,

    /// Files uploaded together share a batch.
    #[index]
    pub batch: String,

    pub file_name: String,

    /// The staged file, relative to the offerings directory.
    pub staged_path: String,

    pub size: u64,

    pub status: OfferingStatus,

    pub error: Option<String>,

    pub title: String,

    pub artist: String,

    pub album: String,

    pub album_artist: Option<String>,

    pub track_number: Option<u32>,

    pub disc_number: Option<u32>,

    pub year: Option<i32>,

    pub genre: Option<String>,

    pub duration_ms: u64,

    pub created_at: Timestamp,
}

/// The play queue a client saved, so another device can resume it
/// (`savePlayQueue` / `getPlayQueue`).
#[derive(Debug, toasty::Model)]
pub struct PlayQueue {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub user_id: u64,

    /// Track ids, in queue order.
    pub entries: Vec<u64>,

    /// The track playing when the queue was saved.
    pub current: Option<u64>,

    /// Playback position within `current`, in milliseconds.
    pub position_ms: u64,

    /// The client that saved the queue.
    pub changed_by: String,

    pub changed_at: Timestamp,
}

/// Listening data about an item: plays now; stars and ratings later.
#[derive(Debug, toasty::Model)]
pub struct Annotation {
    #[key]
    #[auto]
    pub id: u64,

    /// The Subsonic id of the annotated item, e.g. `tr-12` or `al-3`.
    #[unique]
    pub item: String,

    pub play_count: u64,

    pub last_played: Option<Timestamp>,

    pub starred_at: Option<Timestamp>,

    /// 1 to 5 stars.
    pub rating: Option<u8>,
}

/// Health of a platform session, as the session warden last saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum SessionState {
    /// The cookies work.
    Valid,
    /// The last check failed for a transient reason (network, server).
    Degraded,
    /// The platform no longer accepts the cookies; the admin must log in
    /// again.
    Expired,
}

/// The admin's logged-in session with a streaming platform.
#[derive(Debug, toasty::Model)]
pub struct SourceSession {
    #[key]
    #[auto]
    pub id: u64,

    /// The platform, e.g. `youtube_music`.
    #[unique]
    pub source: String,

    /// The session cookies as a `Cookie` header, sealed with the instance
    /// key.
    pub cookies: String,

    pub state: SessionState,

    pub connected_at: Timestamp,

    pub last_verified: Option<Timestamp>,

    pub last_refreshed: Option<Timestamp>,

    pub expired_at: Option<Timestamp>,

    pub last_error: Option<String>,
}

/// A notable change in a platform session, for the admin's history.
#[derive(Debug, toasty::Model)]
pub struct SessionEvent {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub source: String,

    pub message: String,

    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum JobKind {
    /// Download one track.
    DownloadTrack,
    /// Queue every track of an album.
    GrabAlbum,
    /// Bring a watch up to date with the platform.
    SyncWatch,
    /// Look an album up on MusicBrainz, fetch its cover, lyrics and artist
    /// information.
    Enrich,
    /// Move every file where the file layout wants it.
    Refile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
    /// Waiting for something outside píxiū, like a platform login.
    Paused,
}

/// Background work that survives restarts.
#[derive(Debug, toasty::Model)]
pub struct Job {
    #[key]
    #[auto]
    pub id: u64,

    pub kind: JobKind,

    /// Kind-specific parameters, as JSON.
    pub payload: String,

    /// What the job is about, for people: "Artist — Title".
    pub title: String,

    #[index]
    pub state: JobState,

    /// Percent complete.
    pub progress: u8,

    pub attempts: u32,

    pub error: Option<String>,

    /// The track a finished download produced.
    pub track_id: Option<u64>,

    pub created_at: Timestamp,

    pub started_at: Option<Timestamp>,

    pub finished_at: Option<Timestamp>,
}

/// What a watch follows on YouTube Music.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum WatchKind {
    /// A playlist, mirrored as a Subsonic playlist.
    Playlist,
    /// The account's liked music (the `LM` playlist), mirrored like a
    /// playlist. Needs a login.
    LikedMusic,
    /// An artist, whose releases are grabbed as they appear.
    Artist,
}

/// Something on YouTube Music that píxiū keeps up with. Tracks it brings
/// in are claimed by it; when they leave it, they are kept, and become
/// orphans unless something else claims them.
#[derive(Debug, toasty::Model)]
pub struct Watch {
    #[key]
    #[auto]
    pub id: u64,

    pub kind: WatchKind,

    /// The playlist or channel id; `LM` for liked music.
    #[unique]
    pub remote_id: String,

    /// The name on the platform, filled in by the first sync.
    pub name: String,

    /// Artists: grab singles and EPs too, not only albums.
    pub include_singles: bool,

    /// Artists: skip the releases that were out when the watch was added.
    pub only_new: bool,

    /// Artists: releases already handled, by browse id.
    pub seen: Vec<String>,

    pub interval_secs: u64,

    pub created_at: Timestamp,

    pub last_synced_at: Option<Timestamp>,

    pub next_sync_at: Timestamp,

    pub last_error: Option<String>,
}

/// A playlist: made by the admin in a Subsonic client, or mirroring a
/// watched playlist (read-only to clients).
#[derive(Debug, toasty::Model)]
pub struct Playlist {
    #[key]
    #[auto]
    pub id: u64,

    pub name: String,

    pub comment: Option<String>,

    pub public: bool,

    /// The watch this playlist mirrors.
    #[unique]
    pub watch_id: Option<u64>,

    pub created_at: Timestamp,

    pub changed_at: Timestamp,
}

/// A place in a playlist.
#[derive(Debug, toasty::Model)]
pub struct PlaylistEntry {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub playlist_id: u64,

    /// Zero-based.
    pub position: u32,

    /// The track, in playlists made in píxiū.
    #[index]
    pub track_id: Option<u64>,

    /// The YouTube Music video, in mirrors. It shows once it is downloaded.
    pub ytm_video_id: Option<String>,
}

/// Where lyrics come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum LyricsSource {
    /// Embedded in the audio file.
    File,
    /// LRCLIB, which often has them time-synced.
    Lrclib,
    /// YouTube Music (plain text).
    YouTubeMusic,
    /// LRCLIB knows the track has no words.
    Instrumental,
    /// Looked for everywhere, found nowhere.
    Missing,
}

impl LyricsSource {
    /// Whether there are words to show.
    #[must_use]
    pub fn has_words(self) -> bool {
        !matches!(self, Self::Instrumental | Self::Missing)
    }
}

/// A track's lyrics.
#[derive(Debug, toasty::Model)]
pub struct Lyrics {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub track_id: u64,

    pub source: LyricsSource,

    /// Time-synced lyrics in LRC format (`[mm:ss.xx] line`).
    pub synced: Option<String>,

    pub plain: Option<String>,

    pub fetched_at: Timestamp,
}

/// A setting made in the WebUI, by name.
#[derive(Debug, toasty::Model)]
pub struct Setting {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub key: String,

    pub value: String,
}
