ALTER TABLE "playlist_entries" ADD COLUMN "artist" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "playlist_entries" ADD COLUMN "title" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "watches" ADD COLUMN "image_url" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "jobs" ADD COLUMN "parent_id" INTEGER;
-- #[toasty::breakpoint]
CREATE INDEX "index_jobs_by_parent_id" ON "jobs" ("parent_id");
-- #[toasty::breakpoint]
-- SQLite needs a default to add a NOT NULL column to a table with rows;
-- existing events are then classified by their message (see
-- `pixiu_jobs::warden`, which wrote them).
ALTER TABLE "session_events" ADD COLUMN "kind" TEXT NOT NULL DEFAULT 'connected' CHECK ("kind" IN ('connected', 'disconnected', 'recovered', 'refreshed', 'degraded', 'expired'));
-- #[toasty::breakpoint]
UPDATE "session_events" SET "kind" = CASE
    WHEN "message" = 'Disconnected' THEN 'disconnected'
    WHEN "message" = 'Session working again' THEN 'recovered'
    WHEN "message" = 'Cookies refreshed' THEN 'refreshed'
    WHEN "message" LIKE 'Session expired%' THEN 'expired'
    ELSE 'connected'
END;
-- #[toasty::breakpoint]
ALTER TABLE "offerings" ADD COLUMN "archive" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "tracks" ADD COLUMN "source_name" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "tracks" ADD COLUMN "source_archive" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "artists" ADD COLUMN "ytm_channel_id" TEXT;
-- #[toasty::breakpoint]
CREATE INDEX "index_artists_by_ytm_channel_id" ON "artists" ("ytm_channel_id");
-- #[toasty::breakpoint]
CREATE TABLE "released_claims" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "track_id" INTEGER NOT NULL,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('offering', 'manual_grab', 'watch_playlist', 'watch_artist', 'local_playlist', 'starred')),
    "reason" TEXT NOT NULL CHECK ("reason" IN ('left_playlist', 'watch_removed', 'playlist_edited', 'unstarred')),
    "source_name" TEXT,
    "released_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_released_claims_by_track_id" ON "released_claims" ("track_id");
