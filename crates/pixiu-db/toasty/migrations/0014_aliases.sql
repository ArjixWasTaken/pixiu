ALTER TABLE "users" ADD COLUMN "display_name" TEXT;
-- #[toasty::breakpoint]
CREATE TABLE "track_aliases" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "track_id" INTEGER NOT NULL,
    "ytm_video_id" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_track_aliases_by_user_id_and_ytm_video_id" ON "track_aliases" ("user_id", "ytm_video_id");
-- #[toasty::breakpoint]
CREATE INDEX "index_track_aliases_by_track_id" ON "track_aliases" ("track_id");
