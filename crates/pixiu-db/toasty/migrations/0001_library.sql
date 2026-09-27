ALTER TABLE "users" ADD COLUMN "subsonic_secret" TEXT;
-- #[toasty::breakpoint]
CREATE TABLE "annotations" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "item" TEXT NOT NULL,
    "play_count" INTEGER NOT NULL,
    "last_played" TEXT,
    "starred_at" TEXT,
    "rating" INTEGER
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_annotations_by_item" ON "annotations" ("item");
-- #[toasty::breakpoint]
CREATE TABLE "api_keys" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "key_hash" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "last_used_at" TEXT
);
-- #[toasty::breakpoint]
CREATE INDEX "index_api_keys_by_user_id" ON "api_keys" ("user_id");
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_api_keys_by_key_hash" ON "api_keys" ("key_hash");
-- #[toasty::breakpoint]
CREATE TABLE "play_queues" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "entries" TEXT NOT NULL,
    "current" INTEGER,
    "position_ms" INTEGER NOT NULL,
    "changed_by" TEXT NOT NULL,
    "changed_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_play_queues_by_user_id" ON "play_queues" ("user_id");
-- #[toasty::breakpoint]
CREATE TABLE "offerings" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "batch" TEXT NOT NULL,
    "file_name" TEXT NOT NULL,
    "staged_path" TEXT NOT NULL,
    "size" INTEGER NOT NULL,
    "status" TEXT NOT NULL CHECK ("status" IN ('pending', 'unreadable')),
    "error" TEXT,
    "title" TEXT NOT NULL,
    "artist" TEXT NOT NULL,
    "album" TEXT NOT NULL,
    "album_artist" TEXT,
    "track_number" INTEGER,
    "disc_number" INTEGER,
    "year" INTEGER,
    "genre" TEXT,
    "duration_ms" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_offerings_by_batch" ON "offerings" ("batch");
-- #[toasty::breakpoint]
CREATE TABLE "track_claims" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "track_id" INTEGER NOT NULL,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('offering', 'manual_grab', 'watch_playlist', 'watch_artist', 'local_playlist', 'starred')),
    "reference" TEXT,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_track_claims_by_track_id" ON "track_claims" ("track_id");
-- #[toasty::breakpoint]
CREATE TABLE "tracks" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "album_id" INTEGER NOT NULL,
    "artist_id" INTEGER NOT NULL,
    "title" TEXT NOT NULL,
    "artist_credit" TEXT NOT NULL,
    "track_number" INTEGER,
    "disc_number" INTEGER,
    "year" INTEGER,
    "genre" TEXT,
    "duration_ms" INTEGER NOT NULL,
    "bitrate" INTEGER,
    "sample_rate" INTEGER,
    "channels" INTEGER,
    "bit_depth" INTEGER,
    "path" TEXT NOT NULL,
    "size" INTEGER NOT NULL,
    "suffix" TEXT NOT NULL,
    "content_type" TEXT NOT NULL,
    "mbid" TEXT,
    "isrc" TEXT,
    "origin" TEXT NOT NULL CHECK ("origin" IN ('offering', 'download')),
    "added_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_tracks_by_album_id" ON "tracks" ("album_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_tracks_by_artist_id" ON "tracks" ("artist_id");
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_tracks_by_path" ON "tracks" ("path");
-- #[toasty::breakpoint]
CREATE TABLE "albums" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "title" TEXT NOT NULL,
    "title_key" TEXT NOT NULL,
    "artist_id" INTEGER NOT NULL,
    "year" INTEGER,
    "genre" TEXT,
    "mbid" TEXT,
    "cover" TEXT,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_albums_by_artist_id" ON "albums" ("artist_id");
-- #[toasty::breakpoint]
CREATE TABLE "artists" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "name_key" TEXT NOT NULL,
    "mbid" TEXT,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_artists_by_name_key" ON "artists" ("name_key");
