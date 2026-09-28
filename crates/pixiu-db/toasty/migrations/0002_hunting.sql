ALTER TABLE "tracks" ADD COLUMN "ytm_video_id" TEXT;
-- #[toasty::breakpoint]
CREATE INDEX "index_tracks_by_ytm_video_id" ON "tracks" ("ytm_video_id");
-- #[toasty::breakpoint]
ALTER TABLE "albums" ADD COLUMN "ytm_browse_id" TEXT;
-- #[toasty::breakpoint]
CREATE INDEX "index_albums_by_ytm_browse_id" ON "albums" ("ytm_browse_id");
-- #[toasty::breakpoint]
CREATE TABLE "session_events" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "source" TEXT NOT NULL,
    "message" TEXT NOT NULL,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_session_events_by_source" ON "session_events" ("source");
-- #[toasty::breakpoint]
CREATE TABLE "source_sessions" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "source" TEXT NOT NULL,
    "cookies" TEXT NOT NULL,
    "state" TEXT NOT NULL CHECK ("state" IN ('valid', 'degraded', 'expired')),
    "connected_at" TEXT NOT NULL,
    "last_verified" TEXT,
    "last_refreshed" TEXT,
    "expired_at" TEXT,
    "last_error" TEXT
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_source_sessions_by_source" ON "source_sessions" ("source");
-- #[toasty::breakpoint]
CREATE TABLE "jobs" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('download_track', 'grab_album')),
    "payload" TEXT NOT NULL,
    "title" TEXT NOT NULL,
    "state" TEXT NOT NULL CHECK ("state" IN ('queued', 'running', 'done', 'failed', 'paused')),
    "progress" INTEGER NOT NULL,
    "attempts" INTEGER NOT NULL,
    "error" TEXT,
    "track_id" INTEGER,
    "created_at" TEXT NOT NULL,
    "started_at" TEXT,
    "finished_at" TEXT
);
-- #[toasty::breakpoint]
CREATE INDEX "index_jobs_by_state" ON "jobs" ("state");
