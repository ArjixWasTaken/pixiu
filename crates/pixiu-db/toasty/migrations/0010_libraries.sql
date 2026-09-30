-- Every library belongs to a user. Until now píxiū had one, so what exists
-- is theirs: SQLite needs a default to add a NOT NULL column to a table
-- with rows, and the backfill names the first user.
ALTER TABLE "playlist_folders" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "playlist_folders" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_playlist_folders_by_user_id" ON "playlist_folders" ("user_id");
-- #[toasty::breakpoint]
ALTER TABLE "playlists" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "playlists" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_playlists_by_user_id" ON "playlists" ("user_id");
-- #[toasty::breakpoint]
DROP INDEX "index_watches_by_remote_id";
-- #[toasty::breakpoint]
ALTER TABLE "watches" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "watches" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_watches_by_user_id_and_remote_id" ON "watches" ("user_id", "remote_id");
-- #[toasty::breakpoint]
ALTER TABLE "jobs" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "jobs" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_jobs_by_user_id_and_state" ON "jobs" ("user_id", "state");
-- #[toasty::breakpoint]
DROP INDEX "index_session_events_by_source";
-- #[toasty::breakpoint]
ALTER TABLE "session_events" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "session_events" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_session_events_by_user_id_and_source" ON "session_events" ("user_id", "source");
-- #[toasty::breakpoint]
DROP INDEX "index_source_sessions_by_source";
-- #[toasty::breakpoint]
ALTER TABLE "source_sessions" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "source_sessions" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_source_sessions_by_user_id_and_source" ON "source_sessions" ("user_id", "source");
-- #[toasty::breakpoint]
DROP INDEX "index_annotations_by_item";
-- #[toasty::breakpoint]
ALTER TABLE "annotations" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "annotations" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_annotations_by_user_id_and_item" ON "annotations" ("user_id", "item");
-- #[toasty::breakpoint]
DROP INDEX "index_offerings_by_batch";
-- #[toasty::breakpoint]
ALTER TABLE "offerings" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "offerings" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_offerings_by_user_id_and_batch" ON "offerings" ("user_id", "batch");
-- #[toasty::breakpoint]
DROP INDEX "index_tracks_by_ytm_video_id";
-- #[toasty::breakpoint]
ALTER TABLE "tracks" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "tracks" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_tracks_by_user_id_and_ytm_video_id" ON "tracks" ("user_id", "ytm_video_id");
-- #[toasty::breakpoint]
DROP INDEX "index_albums_by_ytm_browse_id";
-- #[toasty::breakpoint]
ALTER TABLE "albums" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "albums" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_albums_by_user_id_and_ytm_browse_id" ON "albums" ("user_id", "ytm_browse_id");
-- #[toasty::breakpoint]
DROP INDEX "index_artists_by_name_key";
-- #[toasty::breakpoint]
DROP INDEX "index_artists_by_ytm_channel_id";
-- #[toasty::breakpoint]
ALTER TABLE "artists" ADD COLUMN "user_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
UPDATE "artists" SET "user_id" = COALESCE((SELECT MIN("id") FROM "users"), 0);
-- #[toasty::breakpoint]
CREATE INDEX "index_artists_by_user_id_and_ytm_channel_id" ON "artists" ("user_id", "ytm_channel_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_artists_by_user_id_and_name_key" ON "artists" ("user_id", "name_key");
