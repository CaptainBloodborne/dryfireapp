-- 0006_add_user_role.sql
-- Spec calls for admin endpoints; a single boolean is enough for now.
-- If the project grows roles (moderator, support, etc.), promote this
-- to an enum without changing semantics.

ALTER TABLE users
    ADD COLUMN is_admin BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX users_admin_idx ON users (is_admin) WHERE is_admin = TRUE;
