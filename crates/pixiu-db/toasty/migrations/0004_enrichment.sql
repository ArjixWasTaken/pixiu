PRAGMA foreign_keys = OFF;
-- #[toasty::breakpoint]
CREATE TABLE "_toasty_new_jobs" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('download_track', 'grab_album', 'sync_watch', 'enrich')),
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
ALTER TABLE "albums" ADD COLUMN "rg_mbid" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "albums" ADD COLUMN "candidates" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "albums" ADD COLUMN "enriched_at" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "albums" ADD COLUMN "enrichment" TEXT CHECK ("enrichment" IN ('matched', 'review', 'unmatched'));
-- #[toasty::breakpoint]
ALTER TABLE "artists" ADD COLUMN "bio_url" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "artists" ADD COLUMN "info_fetched_at" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "artists" ADD COLUMN "bio" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "artists" ADD COLUMN "image" TEXT;
-- #[toasty::breakpoint]
CREATE TABLE "lyrics" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "track_id" INTEGER NOT NULL,
    "source" TEXT NOT NULL CHECK ("source" IN ('file', 'lrclib', 'you_tube_music', 'instrumental', 'missing')),
    "synced" TEXT,
    "plain" TEXT,
    "fetched_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_lyrics_by_track_id" ON "lyrics" ("track_id");
