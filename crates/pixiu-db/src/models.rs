//! Database models.
//!
//! Changing anything here requires a new migration; see `pixiu-migrate`.

use jiff::Timestamp;

/// What a user may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum Role {
    /// Manages users and the server's settings, besides their own library.
    Admin,
    /// Has a library of their own.
    User,
}

/// Where an account stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum UserStatus {
    /// Registered; an admin has yet to approve it.
    Pending,
    /// Approved; the email address has yet to be confirmed.
    Unverified,
    /// Signs in and plays.
    Active,
    /// Turned off by an admin; the library stays.
    Disabled,
}

/// An account: a person with a library of their own. The first one, made
/// by the first-run setup, is an admin.
#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub username: String,

    /// What the player calls the user; the username when unset.
    pub display_name: Option<String>,

    /// Lowercase; for signing in, password resets and alerts. Unique among
    /// those set.
    #[unique]
    pub email: Option<String>,

    /// When the email address was confirmed.
    pub email_verified_at: Option<Timestamp>,

    pub role: Role,

    pub status: UserStatus,

    /// Set by an admin handing out a temporary password: the user picks
    /// their own at their next sign-in.
    pub password_change_required: bool,

    /// PHC-formatted argon2id hash of the password.
    pub password_hash: String,

    /// The password sealed with the instance key. Subsonic token
    /// authentication (`md5(password + salt)`) cannot work from a hash, so
    /// the password is captured whenever the user sets or types it.
    pub subsonic_secret: Option<String>,

    pub created_at: Timestamp,

    #[has_many]
    pub web_sessions: toasty::Deferred<Vec<WebSession>>,

    #[has_many]
    pub api_keys: toasty::Deferred<Vec<ApiKey>>,
}

/// A cookie session of the old server-rendered WebUI. Only the SHA-256 of
/// the session token is stored. Unused since the web player signs in with an
/// API key; the table stays so old databases migrate cleanly.
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

// Two lookups, two indexes: the attributes are not duplicates.
#[allow(clippy::duplicated_attributes)]
#[derive(Debug, toasty::Model)]
#[index(user_id, name_key)]
#[index(user_id, source_key)]
pub struct Artist {
    #[key]
    #[auto]
    pub id: u64,

    /// The user whose library it belongs to.
    pub user_id: u64,

    pub name: String,

    /// `name` normalized for matching (see `pixiu_treasury::name_key`).
    pub name_key: String,

    pub mbid: Option<String>,

    /// The artist on a platform (a YouTube Music channel), when a download
    /// or a watch has named it: a [`crate::SourceKey`]. Lets the user watch
    /// the artist.
    pub source_key: Option<String>,

    /// A short biography, from Wikipedia.
    pub bio: Option<String>,

    /// Where the biography comes from, for attribution.
    pub bio_url: Option<String>,

    /// A picture of the artist, relative to the treasure directory
    /// (`.store/images/…`).
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
#[index(user_id, source_key)]
pub struct Album {
    #[key]
    #[auto]
    pub id: u64,

    /// The user whose library it belongs to.
    pub user_id: u64,

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

    /// The album on the platform it was downloaded from (a YouTube Music
    /// browse id): a [`crate::SourceKey`].
    pub source_key: Option<String>,

    /// The cover image, relative to the treasure directory
    /// (`.store/images/…`). Albums with the same picture share the file.
    pub cover: Option<String>,

    /// A single, or an artist's songs that came with no album: the player
    /// shows its songs on their own, not as an album.
    pub single: bool,

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
#[index(user_id, source_key)]
pub struct Track {
    #[key]
    #[auto]
    pub id: u64,

    /// The user whose library it belongs to.
    pub user_id: u64,

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

    /// The audio file the track plays: its [`AudioFile`], or 0 for a track
    /// filed before files were shared, until it is adopted at startup.
    #[index]
    pub file_id: u64,

    /// The audio file, relative to the treasure directory; a copy of the
    /// [`AudioFile`]'s path. Several tracks may share it.
    pub path: String,

    pub size: u64,

    /// File extension, e.g. `flac`.
    pub suffix: String,

    pub content_type: String,

    /// MusicBrainz recording id.
    pub mbid: Option<String>,

    pub isrc: Option<String>,

    /// The song on the platform it was downloaded from (a YouTube Music
    /// video): a [`crate::SourceKey`].
    pub source_key: Option<String>,

    pub origin: TrackOrigin,

    /// For offerings, the file name it was uploaded under.
    pub source_name: Option<String>,

    /// For offerings, the zip archive it was unpacked from.
    pub source_archive: Option<String>,

    pub added_at: Timestamp,

    #[has_many]
    pub claims: toasty::Deferred<Vec<TrackClaim>>,

    #[has_many]
    pub released_claims: toasty::Deferred<Vec<ReleasedClaim>>,
}

/// Audio bytes, stored once under their SHA-256 and shared by every track
/// made from them. Files never change once stored: edits live in the
/// database. A file goes when no track points at it any more.
#[derive(Debug, toasty::Model)]
pub struct AudioFile {
    #[key]
    #[auto]
    pub id: u64,

    /// Hex-encoded SHA-256 of the content.
    #[unique]
    pub sha256: String,

    /// Relative to the treasure directory:
    /// `.store/audio/<first two hex digits>/<sha256>.<suffix>`.
    pub path: String,

    pub size: u64,

    /// File extension, e.g. `flac`.
    pub suffix: String,

    pub content_type: String,

    pub duration_ms: u64,

    /// Kilobits per second.
    pub bitrate: Option<u32>,

    pub sample_rate: Option<u32>,

    pub channels: Option<u8>,

    pub bit_depth: Option<u8>,

    /// The song on the platform the file was downloaded from (a
    /// [`crate::SourceKey`]), so it is not downloaded again.
    #[index]
    pub source_key: Option<String>,

    pub created_at: Timestamp,
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

/// Why a claim on a track was let go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum ReleaseReason {
    /// The track left a watched playlist.
    LeftPlaylist,
    /// The watch that claimed it was removed.
    WatchRemoved,
    /// A Subsonic app removed it from a playlist.
    PlaylistEdited,
    /// A Subsonic app unstarred it.
    Unstarred,
    /// The admin excluded it from a watched playlist.
    Excluded,
}

/// A claim that was released, kept so an orphan can say why nothing keeps it
/// any more. Cleared when the track is claimed again or deleted.
#[derive(Debug, toasty::Model)]
pub struct ReleasedClaim {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub track_id: u64,

    #[belongs_to]
    pub track: toasty::Deferred<Track>,

    pub kind: ClaimKind,

    pub reason: ReleaseReason,

    /// The watch or playlist the claim came from, named as it was then: the
    /// watch itself may be gone.
    pub source_name: Option<String>,

    pub released_at: Timestamp,
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
#[index(user_id, batch)]
pub struct Offering {
    #[key]
    #[auto]
    pub id: u64,

    /// The user who uploaded it.
    pub user_id: u64,

    /// Files uploaded together share a batch.
    pub batch: String,

    pub file_name: String,

    /// The zip archive the file was unpacked from, if any.
    pub archive: Option<String>,

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

/// Listening data about an item: plays, stars and ratings.
#[derive(Debug, toasty::Model)]
#[unique(user_id, item)]
pub struct Annotation {
    #[key]
    #[auto]
    pub id: u64,

    /// The user who listened; items are in their library.
    pub user_id: u64,

    /// The Subsonic id of the annotated item, e.g. `tr-12` or `al-3`.
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
    /// The platform no longer accepts the cookies; the user must log in
    /// again.
    Expired,
}

/// A user's logged-in session with a streaming platform.
#[derive(Debug, toasty::Model)]
#[unique(user_id, source)]
pub struct SourceSession {
    #[key]
    #[auto]
    pub id: u64,

    pub user_id: u64,

    /// The platform, e.g. `youtube_music`.
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

/// What happened to a platform session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum SessionEventKind {
    Connected,
    Disconnected,
    /// Working again after trouble or expiry.
    Recovered,
    Refreshed,
    /// A check failed for a transient reason.
    Degraded,
    Expired,
}

impl SessionEventKind {
    /// Whether the event is bad news.
    #[must_use]
    pub fn is_problem(self) -> bool {
        matches!(self, Self::Degraded | Self::Expired)
    }
}

/// A notable change in a platform session, for the user's history.
#[derive(Debug, toasty::Model)]
#[index(user_id, source)]
pub struct SessionEvent {
    #[key]
    #[auto]
    pub id: u64,

    pub user_id: u64,

    pub source: String,

    pub kind: SessionEventKind,

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
    /// Retired: moved files to follow the file layout, before files were
    /// shared and stopped moving. None are queued any more; the variant
    /// stays because the schema cannot drop it.
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
#[index(user_id, state)]
pub struct Job {
    #[key]
    #[auto]
    pub id: u64,

    /// The user the work is for.
    pub user_id: u64,

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

    /// The job that queued this one: an album grab for its tracks.
    #[index]
    pub parent_id: Option<u64>,

    pub created_at: Timestamp,

    pub started_at: Option<Timestamp>,

    pub finished_at: Option<Timestamp>,
}

/// What a watch follows on a platform.
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

/// Something on a platform that píxiū keeps up with. Tracks it brings
/// in are claimed by it; when they leave it, they are kept, and become
/// orphans unless something else claims them.
#[derive(Debug, toasty::Model)]
#[unique(user_id, source_key)]
pub struct Watch {
    #[key]
    #[auto]
    pub id: u64,

    pub user_id: u64,

    pub kind: WatchKind,

    /// What it follows on the platform (a [`crate::SourceKey`]): a playlist
    /// or a channel; `youtube_music:LM` for liked music.
    pub source_key: String,

    /// The name on the platform, filled in by the first sync.
    pub name: String,

    /// The playlist's thumbnail or the artist's picture on the platform,
    /// filled in by syncs.
    pub image_url: Option<String>,

    /// Artists: grab singles and EPs too, not only albums.
    pub include_singles: bool,

    /// Artists: skip the releases that were out when the watch was added.
    pub only_new: bool,

    /// Artists: releases already handled, by key.
    pub seen: Vec<String>,

    pub interval_secs: u64,

    pub created_at: Timestamp,

    pub last_synced_at: Option<Timestamp>,

    pub next_sync_at: Timestamp,

    pub last_error: Option<String>,

    /// Syncs failed in a row; the user hears about a long streak.
    pub failures: u32,

    /// When the current streak of failures began.
    pub failing_since: Option<Timestamp>,
}

/// A playlist: made by its user in a Subsonic client or the player, or
/// mirroring a watched playlist (read-only to clients).
#[derive(Debug, toasty::Model)]
pub struct Playlist {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    pub name: String,

    pub comment: Option<String>,

    pub public: bool,

    /// The watch this playlist mirrors.
    #[unique]
    pub watch_id: Option<u64>,

    /// The folder the player files it under; `None` at the top.
    #[index]
    pub folder_id: Option<u64>,

    /// A smart playlist's rules, as JSON: groups of rules, any group
    /// matching and all rules within it. Its songs are whatever matches
    /// when it is read; it has no entries of its own.
    pub rules: Option<String>,

    pub created_at: Timestamp,

    pub changed_at: Timestamp,
}

/// A folder of playlists (and folders) in the player's sidebar.
#[derive(Debug, toasty::Model)]
pub struct PlaylistFolder {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    pub name: String,

    /// The folder it sits in; `None` at the top.
    #[index]
    pub parent_id: Option<u64>,

    pub created_at: Timestamp,
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

    /// The song on the platform, in mirrors (a [`crate::SourceKey`]). It
    /// shows once it is downloaded.
    pub source_key: Option<String>,

    /// Mirrors: the song's title on the platform, to name it before it is
    /// downloaded.
    pub title: Option<String>,

    /// Mirrors: the song's artists on the platform.
    pub artist: Option<String>,
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

/// A setting made in the web player, by name.
#[derive(Debug, toasty::Model)]
pub struct Setting {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub key: String,

    pub value: String,
}

/// Another key of a track the library holds: YouTube Music lists some
/// songs under several videos, and a download of one of them turned out to
/// be a track the library had. The library holds that video too, so watches
/// and searches do not fetch it again.
#[derive(Debug, toasty::Model)]
#[unique(user_id, source_key)]
pub struct TrackAlias {
    #[key]
    #[auto]
    pub id: u64,

    pub user_id: u64,

    #[index]
    pub track_id: u64,

    /// A [`crate::SourceKey`].
    pub source_key: String,
}

/// A song the admin excluded from a watched playlist: the watch neither
/// keeps, lists nor downloads it.
#[derive(Debug, toasty::Model)]
pub struct WatchExclusion {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub watch_id: u64,

    /// The song on the platform: a [`crate::SourceKey`].
    pub source_key: String,

    /// The song as the playlist named it, to list it after it is excluded.
    pub title: Option<String>,

    pub artist: Option<String>,

    pub excluded_at: Timestamp,
}

/// What an [`AccountToken`] is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, toasty::Embed)]
pub enum TokenPurpose {
    /// Confirms the email address it was sent to.
    VerifyEmail,
    /// Lets its holder choose a new password.
    ResetPassword,
}

/// A single-use link sent by email. Only the SHA-256 of the token is
/// stored; the link holds the token itself.
#[derive(Debug, toasty::Model)]
pub struct AccountToken {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    pub purpose: TokenPurpose,

    /// Hex-encoded SHA-256 of the token.
    #[unique]
    pub token_hash: String,

    /// For email confirmations, the address confirmed.
    pub email: Option<String>,

    pub created_at: Timestamp,

    pub expires_at: Timestamp,
}

/// A user's setting, by name, as JSON.
#[derive(Debug, toasty::Model)]
#[unique(user_id, key)]
pub struct UserSetting {
    #[key]
    #[auto]
    pub id: u64,

    pub user_id: u64,

    pub key: String,

    pub value: String,
}

/// An alert emailed to a user, so each is sent once (e.g. once per expiry
/// of their YouTube Music session), even across restarts.
#[derive(Debug, toasty::Model)]
#[unique(user_id, dedupe_key)]
pub struct SentAlert {
    #[key]
    #[auto]
    pub id: u64,

    pub user_id: u64,

    /// What the alert was about, e.g. `youtube-music/expired/<when>`.
    pub dedupe_key: String,

    pub sent_at: Timestamp,
}

/// An account at a single sign-on provider, linked to a píxiū account by
/// its owner. Signing in there signs them in here.
// Two rules, two indexes: the attributes are not duplicates.
#[allow(clippy::duplicated_attributes)]
#[derive(Debug, toasty::Model)]
#[unique(issuer, subject)]
#[unique(user_id, issuer)]
pub struct UserIdentity {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    /// The provider, by its issuer URL.
    pub issuer: String,

    /// Who they are there: the ID token's `sub`, stable for the account.
    pub subject: String,

    /// Their address there, when the provider says; shown, never trusted.
    pub email: Option<String>,

    pub linked_at: Timestamp,

    pub last_login_at: Option<Timestamp>,
}
