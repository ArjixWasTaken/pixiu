PRAGMA foreign_keys = OFF;
-- #[toasty::breakpoint]
CREATE TABLE "_toasty_new_jobs" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('download_track', 'grab_album', 'sync_watch')),
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
INSERT INTO "_toasty_new_jobs" ("id", "kind", "payload", "title", "state", "progress", "attempts", "error", "track_id", "created_at", "started_at", "finished_at") SELECT "id", "kind", "payload", "title", "state", "progress", "attempts", "error", "track_id", "created_at", "started_at", "finished_at" FROM "jobs";
-- #[toasty::breakpoint]
DROP TABLE "jobs";
-- #[toasty::breakpoint]
ALTER TABLE "_toasty_new_jobs" RENAME TO "jobs";
-- #[toasty::breakpoint]
-- Dropping the old table dropped its index too; Toasty's generator forgets it.
CREATE INDEX "index_jobs_by_state" ON "jobs" ("state");
-- #[toasty::breakpoint]
PRAGMA foreign_keys = ON;
-- #[toasty::breakpoint]
CREATE TABLE "playlists" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "comment" TEXT,
    "public" BOOLEAN NOT NULL,
    "watch_id" INTEGER,
    "created_at" TEXT NOT NULL,
    "changed_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_playlists_by_watch_id" ON "playlists" ("watch_id");
-- #[toasty::breakpoint]
CREATE TABLE "watches" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('playlist', 'liked_music', 'artist')),
    "remote_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "include_singles" BOOLEAN NOT NULL,
    "only_new" BOOLEAN NOT NULL,
    "seen" TEXT NOT NULL,
    "interval_secs" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL,
    "last_synced_at" TEXT,
    "next_sync_at" TEXT NOT NULL,
    "last_error" TEXT
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_watches_by_remote_id" ON "watches" ("remote_id");
-- #[toasty::breakpoint]
CREATE TABLE "playlist_entries" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "playlist_id" INTEGER NOT NULL,
    "position" INTEGER NOT NULL,
    "track_id" INTEGER,
    "ytm_video_id" TEXT
);
-- #[toasty::breakpoint]
CREATE INDEX "index_playlist_entries_by_playlist_id" ON "playlist_entries" ("playlist_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_playlist_entries_by_track_id" ON "playlist_entries" ("track_id");
