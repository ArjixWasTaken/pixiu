//! A user's library: loading what they own, and SQL scoped to them.
//!
//! Every user has a library of their own. Handlers reach it through a
//! [`Library`], whose loaders return only rows the owner has: another
//! user's id reads as missing. Raw SQL goes through [`Sql::owned`], which
//! binds the owner as `?1` for statements to filter on
//! (`WHERE tracks.user_id = ?1`).

use std::collections::HashMap;

use toasty::stmt::Value;

use crate::{
    Album, Annotation, Artist, Db, Job, Offering, PlayQueue, Playlist, PlaylistFolder,
    SourceSession, Track, Watch,
};

/// A signed-in user's view of the library.
#[derive(Clone)]
pub struct Library {
    db: Db,
    owner: u64,
}

macro_rules! loaders {
    ($($(#[$doc:meta])* $one:ident, $many:ident: $model:ident;)*) => {$(
        $(#[$doc])*
        ///
        /// # Errors
        ///
        /// Fails on database errors.
        pub async fn $one(&self, id: u64) -> toasty::Result<Option<$model>> {
            Ok($model::filter($model::fields().id().eq(id))
                .first()
                .exec(&mut self.db.clone())
                .await?
                .filter(|row| row.user_id == self.owner))
        }

        /// The ones among `ids` the owner has, in no particular order.
        ///
        /// # Errors
        ///
        /// Fails on database errors.
        pub async fn $many(&self, ids: &[u64]) -> toasty::Result<Vec<$model>> {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            Ok($model::filter($model::fields().id().in_list(ids.to_vec()))
                .exec(&mut self.db.clone())
                .await?
                .into_iter()
                .filter(|row| row.user_id == self.owner)
                .collect())
        }
    )*};
}

impl Library {
    #[must_use]
    pub fn new(db: Db, owner: u64) -> Self {
        Self { db, owner }
    }

    /// The user whose library this is.
    #[must_use]
    pub fn owner(&self) -> u64 {
        self.owner
    }

    /// The database, for writes and for tables that belong to an owned row
    /// (claims, lyrics, playlist entries), which callers reach through it.
    #[must_use]
    pub fn db(&self) -> Db {
        self.db.clone()
    }

    /// SQL over this library; see [`Sql::owned`].
    #[must_use]
    pub fn sql(&self, text: impl Into<String>) -> Sql {
        Sql::owned(self.owner, text)
    }

    loaders! {
        /// A track of the owner's.
        track, tracks: Track;
        /// An album of the owner's.
        album, albums: Album;
        /// An artist of the owner's.
        artist, artists: Artist;
        /// A playlist of the owner's.
        playlist, playlists: Playlist;
        /// A playlist folder of the owner's.
        folder, folders: PlaylistFolder;
        /// A watch of the owner's.
        watch, watches: Watch;
        /// A job of the owner's.
        job, jobs: Job;
        /// An uploaded file of the owner's.
        offering, offerings: Offering;
    }

    /// The tracks among `ids` the owner has, by id.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn tracks_by_id(&self, ids: &[u64]) -> toasty::Result<HashMap<u64, Track>> {
        Ok(self
            .tracks(ids)
            .await?
            .into_iter()
            .map(|track| (track.id, track))
            .collect())
    }

    /// The owner's listening data about `item` (a Subsonic id like `tr-12`).
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn annotation(&self, item: &str) -> toasty::Result<Option<Annotation>> {
        Annotation::filter_by_user_id_and_item(self.owner, item)
            .first()
            .exec(&mut self.db.clone())
            .await
    }

    /// The owner's session with a platform, e.g. `youtube_music`.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn source_session(&self, source: &str) -> toasty::Result<Option<SourceSession>> {
        SourceSession::filter_by_user_id_and_source(self.owner, source)
            .first()
            .exec(&mut self.db.clone())
            .await
    }

    /// The play queue the owner saved last.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn play_queue(&self) -> toasty::Result<Option<PlayQueue>> {
        PlayQueue::filter_by_user_id(self.owner)
            .first()
            .exec(&mut self.db.clone())
            .await
    }

    /// The owner's tracks of any of `keys`: downloaded from them, or found
    /// to be them.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn tracks_of_keys(
        &self,
        keys: &[crate::SourceKey],
    ) -> toasty::Result<crate::keyed::Held> {
        crate::keyed::tracks_of_keys(&mut self.db.clone(), self.owner, keys).await
    }

    /// The owner's tracks by any of `artists`.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn tracks_of_artists(&self, artists: &[u64]) -> toasty::Result<Vec<Track>> {
        if artists.is_empty() {
            return Ok(Vec::new());
        }
        Track::filter(
            Track::fields()
                .user_id()
                .eq(self.owner)
                .and(Track::fields().artist_id().in_list(artists.to_vec())),
        )
        .exec(&mut self.db.clone())
        .await
    }

    /// The owner's albums by any of `artists`.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn albums_of_artists(&self, artists: &[u64]) -> toasty::Result<Vec<Album>> {
        if artists.is_empty() {
            return Ok(Vec::new());
        }
        Album::filter(
            Album::fields()
                .user_id()
                .eq(self.owner)
                .and(Album::fields().artist_id().in_list(artists.to_vec())),
        )
        .exec(&mut self.db.clone())
        .await
    }

    /// The owner's albums downloaded as any of `keys`.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn albums_of_keys(&self, keys: &[crate::SourceKey]) -> toasty::Result<Vec<Album>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let stored: Vec<String> = keys.iter().map(crate::SourceKey::as_stored).collect();
        Album::filter(
            Album::fields()
                .user_id()
                .eq(self.owner)
                .and(Album::fields().source_key().in_list(stored)),
        )
        .exec(&mut self.db.clone())
        .await
    }

    /// The owner's listening data about any of `items` (Subsonic ids).
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn annotations(&self, items: &[String]) -> toasty::Result<Vec<Annotation>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        Annotation::filter(
            Annotation::fields()
                .user_id()
                .eq(self.owner)
                .and(Annotation::fields().item().in_list(items.to_vec())),
        )
        .exec(&mut self.db.clone())
        .await
    }

    /// Every annotation of the owner's.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn all_annotations(&self) -> toasty::Result<Vec<Annotation>> {
        Annotation::filter_by_user_id(self.owner)
            .exec(&mut self.db.clone())
            .await
    }

    /// Every album of the owner's.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn all_albums(&self) -> toasty::Result<Vec<Album>> {
        Album::filter_by_user_id(self.owner)
            .exec(&mut self.db.clone())
            .await
    }

    /// Every artist of the owner's.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn all_artists(&self) -> toasty::Result<Vec<Artist>> {
        Artist::filter_by_user_id(self.owner)
            .exec(&mut self.db.clone())
            .await
    }

    /// Every track of the owner's.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn all_tracks(&self) -> toasty::Result<Vec<Track>> {
        Track::filter_by_user_id(self.owner)
            .exec(&mut self.db.clone())
            .await
    }

    /// Every playlist of the owner's.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn all_playlists(&self) -> toasty::Result<Vec<Playlist>> {
        Playlist::filter_by_user_id(self.owner)
            .exec(&mut self.db.clone())
            .await
    }

    /// Every playlist folder of the owner's.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn all_folders(&self) -> toasty::Result<Vec<PlaylistFolder>> {
        PlaylistFolder::filter_by_user_id(self.owner)
            .exec(&mut self.db.clone())
            .await
    }

    /// Every watch of the owner's.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn all_watches(&self) -> toasty::Result<Vec<Watch>> {
        Watch::filter_by_user_id(self.owner)
            .exec(&mut self.db.clone())
            .await
    }
}

/// A value bound to a numbered parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bind {
    Int(i64),
    Text(String),
}

/// SQL with numbered parameters (`?1`, `?2`, …).
#[derive(Debug, Clone)]
pub struct Sql {
    text: String,
    binds: Vec<Bind>,
}

impl Sql {
    /// SQL over `owner`'s library: `?1` is the owner, for every statement
    /// to filter its root table on (`WHERE albums.user_id = ?1`), and joins
    /// on (`annotations.user_id = albums.user_id`).
    #[must_use]
    pub fn owned(owner: u64, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            binds: vec![Bind::Int(i64::try_from(owner).unwrap_or(i64::MAX))],
        }
    }

    /// The owner's placeholder.
    pub const OWNER: &'static str = "?1";

    /// Registers a parameter, returning its placeholder.
    pub fn param(&mut self, bind: Bind) -> String {
        self.binds.push(bind);
        format!("?{}", self.binds.len())
    }

    pub fn push(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Appends `LIMIT .. OFFSET ..`.
    pub fn page(&mut self, limit: u32, offset: u32) {
        let limit = self.param(Bind::Int(i64::from(limit)));
        let offset = self.param(Bind::Int(i64::from(offset)));
        self.text
            .push_str(&format!(" LIMIT {limit} OFFSET {offset}"));
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The bound values, `?1` (the owner) first.
    #[must_use]
    pub fn binds(&self) -> &[Bind] {
        &self.binds
    }

    /// Runs the statement, returning its rows.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn rows(self, db: &mut Db) -> toasty::Result<Vec<Vec<Value>>> {
        let mut query = toasty::sql::query(&self.text);
        for bind in self.binds {
            query = match bind {
                Bind::Int(value) => query.bind(value),
                Bind::Text(value) => query.bind(value),
            };
        }
        Ok(query
            .exec(db)
            .await?
            .into_iter()
            .filter_map(|row| match row {
                Value::Record(record) => Some(record.fields.into_iter().collect()),
                _ => None,
            })
            .collect())
    }

    /// Runs the statement, whose first column is an id.
    ///
    /// # Errors
    ///
    /// Fails on database errors.
    pub async fn ids(self, db: &mut Db) -> toasty::Result<Vec<u64>> {
        Ok(self
            .rows(db)
            .await?
            .iter()
            .filter_map(|row| row.first().and_then(as_u64))
            .collect())
    }
}

/// A number from a row.
#[must_use]
pub fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::I64(n) => u64::try_from(*n).ok(),
        Value::I32(n) => u64::try_from(*n).ok(),
        Value::U64(n) => Some(*n),
        Value::U32(n) => Some(u64::from(*n)),
        _ => None,
    }
}
