PRAGMA foreign_keys = OFF;
-- #[toasty::breakpoint]
CREATE TABLE "_toasty_new_released_claims" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "track_id" INTEGER NOT NULL,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('offering', 'manual_grab', 'watch_playlist', 'watch_artist', 'local_playlist', 'starred')),
    "reason" TEXT NOT NULL CHECK ("reason" IN ('left_playlist', 'watch_removed', 'playlist_edited', 'unstarred', 'excluded')),
    "source_name" TEXT,
    "released_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
INSERT INTO "_toasty_new_released_claims" ("id", "track_id", "kind", "reason", "source_name", "released_at") SELECT "id", "track_id", "kind", "reason", "source_name", "released_at" FROM "released_claims";
-- #[toasty::breakpoint]
DROP TABLE "released_claims";
-- #[toasty::breakpoint]
ALTER TABLE "_toasty_new_released_claims" RENAME TO "released_claims";
-- #[toasty::breakpoint]
-- Dropping the old table dropped its index too; Toasty's generator forgets it.
CREATE INDEX "index_released_claims_by_track_id" ON "released_claims" ("track_id");
-- #[toasty::breakpoint]
PRAGMA foreign_keys = ON;
-- #[toasty::breakpoint]
CREATE TABLE "watch_exclusions" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "watch_id" INTEGER NOT NULL,
    "ytm_video_id" TEXT NOT NULL,
    "title" TEXT,
    "artist" TEXT,
    "excluded_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_watch_exclusions_by_watch_id" ON "watch_exclusions" ("watch_id");
