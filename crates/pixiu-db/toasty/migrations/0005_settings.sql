PRAGMA foreign_keys = OFF;
-- #[toasty::breakpoint]
CREATE TABLE "_toasty_new_jobs" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('download_track', 'grab_album', 'sync_watch', 'enrich', 'refile')),
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
CREATE TABLE "settings" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "key" TEXT NOT NULL,
    "value" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_settings_by_key" ON "settings" ("key");
