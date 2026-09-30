-- The accounts that exist are the admins who set píxiū up: active, with
-- the passwords they chose. SQLite needs defaults to add NOT NULL columns to
-- a table with rows.
ALTER TABLE "users" ADD COLUMN "password_change_required" BOOLEAN NOT NULL DEFAULT 0;
-- #[toasty::breakpoint]
ALTER TABLE "users" ADD COLUMN "role" TEXT NOT NULL DEFAULT 'admin' CHECK ("role" IN ('admin', 'user'));
-- #[toasty::breakpoint]
ALTER TABLE "users" ADD COLUMN "email" TEXT;
-- #[toasty::breakpoint]
ALTER TABLE "users" ADD COLUMN "status" TEXT NOT NULL DEFAULT 'active' CHECK ("status" IN ('pending', 'unverified', 'active', 'disabled'));
-- #[toasty::breakpoint]
ALTER TABLE "users" ADD COLUMN "email_verified_at" TEXT;
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_users_by_email" ON "users" ("email");
