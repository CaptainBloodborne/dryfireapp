-- 0004_create_password_resets.sql
-- Same shape as email_verifications but separate so the lifecycles
-- can diverge (different TTLs, different rate limits).

CREATE TABLE password_resets (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   TEXT NOT NULL UNIQUE,
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX password_resets_user_id_idx ON password_resets (user_id);
