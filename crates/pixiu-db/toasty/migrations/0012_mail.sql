ALTER TABLE "watches" ADD COLUMN "failing_since" TEXT;
-- #[toasty::breakpoint]
-- SQLite needs a default to add a NOT NULL column to a table with rows.
ALTER TABLE "watches" ADD COLUMN "failures" INTEGER NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
CREATE TABLE "account_tokens" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "purpose" TEXT NOT NULL CHECK ("purpose" IN ('verify_email', 'reset_password')),
    "token_hash" TEXT NOT NULL,
    "email" TEXT,
    "created_at" TEXT NOT NULL,
    "expires_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_account_tokens_by_user_id" ON "account_tokens" ("user_id");
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_account_tokens_by_token_hash" ON "account_tokens" ("token_hash");
-- #[toasty::breakpoint]
CREATE TABLE "user_settings" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "key" TEXT NOT NULL,
    "value" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_user_settings_by_user_id_and_key" ON "user_settings" ("user_id", "key");
-- #[toasty::breakpoint]
CREATE TABLE "sent_alerts" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "dedupe_key" TEXT NOT NULL,
    "sent_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_sent_alerts_by_user_id_and_dedupe_key" ON "sent_alerts" ("user_id", "dedupe_key");
