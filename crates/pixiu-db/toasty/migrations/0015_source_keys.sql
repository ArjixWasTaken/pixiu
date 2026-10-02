ALTER TABLE "watch_exclusions" RENAME COLUMN "ytm_video_id" TO "source_key";
-- #[toasty::breakpoint]
DROP INDEX "index_track_aliases_by_user_id_and_ytm_video_id";
-- #[toasty::breakpoint]
ALTER TABLE "track_aliases" RENAME COLUMN "ytm_video_id" TO "source_key";
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_track_aliases_by_user_id_and_source_key" ON "track_aliases" ("user_id", "source_key");
-- #[toasty::breakpoint]
ALTER TABLE "playlist_entries" RENAME COLUMN "ytm_video_id" TO "source_key";
-- #[toasty::breakpoint]
DROP INDEX "index_watches_by_user_id_and_remote_id";
-- #[toasty::breakpoint]
ALTER TABLE "watches" RENAME COLUMN "remote_id" TO "source_key";
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_watches_by_user_id_and_source_key" ON "watches" ("user_id", "source_key");
-- #[toasty::breakpoint]
DROP INDEX "index_tracks_by_user_id_and_ytm_video_id";
-- #[toasty::breakpoint]
ALTER TABLE "tracks" RENAME COLUMN "ytm_video_id" TO "source_key";
-- #[toasty::breakpoint]
CREATE INDEX "index_tracks_by_user_id_and_source_key" ON "tracks" ("user_id", "source_key");
-- #[toasty::breakpoint]
DROP INDEX "index_albums_by_user_id_and_ytm_browse_id";
-- #[toasty::breakpoint]
ALTER TABLE "albums" RENAME COLUMN "ytm_browse_id" TO "source_key";
-- #[toasty::breakpoint]
CREATE INDEX "index_albums_by_user_id_and_source_key" ON "albums" ("user_id", "source_key");
-- #[toasty::breakpoint]
DROP INDEX "index_artists_by_user_id_and_ytm_channel_id";
-- #[toasty::breakpoint]
ALTER TABLE "artists" RENAME COLUMN "ytm_channel_id" TO "source_key";
-- #[toasty::breakpoint]
CREATE INDEX "index_artists_by_user_id_and_source_key" ON "artists" ("user_id", "source_key");
-- #[toasty::breakpoint]
DROP INDEX "index_audio_files_by_ytm_video_id";
-- #[toasty::breakpoint]
ALTER TABLE "audio_files" RENAME COLUMN "ytm_video_id" TO "source_key";
-- #[toasty::breakpoint]
CREATE INDEX "index_audio_files_by_source_key" ON "audio_files" ("source_key");
-- #[toasty::breakpoint]
-- Ids become keys naming their platform: every one so far is YouTube Music's.
UPDATE "tracks" SET "source_key" = 'youtube_music:' || "source_key" WHERE "source_key" IS NOT NULL AND instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
UPDATE "audio_files" SET "source_key" = 'youtube_music:' || "source_key" WHERE "source_key" IS NOT NULL AND instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
UPDATE "track_aliases" SET "source_key" = 'youtube_music:' || "source_key" WHERE instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
UPDATE "watch_exclusions" SET "source_key" = 'youtube_music:' || "source_key" WHERE instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
UPDATE "playlist_entries" SET "source_key" = 'youtube_music:' || "source_key" WHERE "source_key" IS NOT NULL AND instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
UPDATE "albums" SET "source_key" = 'youtube_music:' || "source_key" WHERE "source_key" IS NOT NULL AND instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
UPDATE "artists" SET "source_key" = 'youtube_music:' || "source_key" WHERE "source_key" IS NOT NULL AND instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
UPDATE "watches" SET "source_key" = 'youtube_music:' || "source_key" WHERE instr("source_key", ':') = 0;
-- #[toasty::breakpoint]
-- Album grabs name their album.
UPDATE "track_claims" SET "reference" = 'youtube_music:' || "reference" WHERE "kind" = 'manual_grab' AND "reference" IS NOT NULL AND instr("reference", ':') = 0;
-- #[toasty::breakpoint]
-- Artist watches remember the releases they handled, by key too.
UPDATE "watches" SET "seen" = (SELECT json_group_array(CASE WHEN instr("value", ':') = 0 THEN 'youtube_music:' || "value" ELSE "value" END) FROM json_each("watches"."seen")) WHERE "seen" <> '[]';
