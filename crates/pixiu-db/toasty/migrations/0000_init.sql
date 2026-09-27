CREATE TABLE "users" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "username" TEXT NOT NULL,
    "password_hash" TEXT NOT NULL,
    "created_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_users_by_username" ON "users" ("username");
-- #[toasty::breakpoint]
CREATE TABLE "web_sessions" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "token_hash" TEXT NOT NULL,
    "user_id" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL,
    "expires_at" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_web_sessions_by_token_hash" ON "web_sessions" ("token_hash");
-- #[toasty::breakpoint]
CREATE INDEX "index_web_sessions_by_user_id" ON "web_sessions" ("user_id");
