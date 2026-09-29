ALTER TABLE "playlists" ADD COLUMN "folder_id" INTEGER;
-- #[toasty::breakpoint]
ALTER TABLE "playlists" ADD COLUMN "rules" TEXT;
-- #[toasty::breakpoint]
CREATE INDEX "index_playlists_by_folder_id" ON "playlists" ("folder_id");
-- #[toasty::breakpoint]
CREATE TABLE "playlist_folders" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL,
    "parent_id" INTEGER,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_playlist_folders_by_parent_id" ON "playlist_folders" ("parent_id");
