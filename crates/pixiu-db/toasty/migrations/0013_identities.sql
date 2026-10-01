CREATE TABLE "user_identities" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "user_id" INTEGER NOT NULL,
    "issuer" TEXT NOT NULL,
    "subject" TEXT NOT NULL,
    "email" TEXT,
    "linked_at" TEXT NOT NULL,
    "last_login_at" TEXT
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_user_identities_by_issuer_and_subject" ON "user_identities" ("issuer", "subject");
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_user_identities_by_user_id_and_issuer" ON "user_identities" ("user_id", "issuer");
-- #[toasty::breakpoint]
CREATE INDEX "index_user_identities_by_user_id" ON "user_identities" ("user_id");
