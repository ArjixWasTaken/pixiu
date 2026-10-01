DROP INDEX "index_tracks_by_path";
-- #[toasty::breakpoint]
-- SQLite needs a default to add a NOT NULL column to a table with rows. 0
-- names no file: the server adopts those tracks' files into the store at
-- startup (see `pixiu_treasury::Treasury::adopt_legacy`).
ALTER TABLE "tracks" ADD COLUMN "file_id" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
CREATE INDEX "index_tracks_by_file_id" ON "tracks" ("file_id");
-- #[toasty::breakpoint]
CREATE TABLE "audio_files" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "sha256" TEXT NOT NULL,
    "path" TEXT NOT NULL,
    "size" INTEGER NOT NULL,
    "suffix" TEXT NOT NULL,
    "content_type" TEXT NOT NULL,
    "duration_ms" INTEGER NOT NULL,
    "bitrate" INTEGER,
    "sample_rate" INTEGER,
    "channels" INTEGER,
    "bit_depth" INTEGER,
    "ytm_video_id" TEXT,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_audio_files_by_sha256" ON "audio_files" ("sha256");
-- #[toasty::breakpoint]
CREATE INDEX "index_audio_files_by_ytm_video_id" ON "audio_files" ("ytm_video_id");
-- #[toasty::breakpoint]
-- Files no longer move: the file layout and its "move files" jobs are gone.
DELETE FROM "jobs" WHERE "kind" = 'refile';
-- #[toasty::breakpoint]
DELETE FROM "settings" WHERE "key" = 'layout';
