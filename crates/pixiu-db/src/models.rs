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

    pub mbid: Option<String>,

    /// The album's YouTube Music browse id, when it was hunted there.
    #[index]
    pub ytm_browse_id: Option<String>,

    /// The cover image, relative to the treasure directory.
    pub cover: Option<String>,

    pub created_at: Timestamp,

    #[has_many]
    pub tracks: toasty::Deferred<Vec<Track>>,
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
