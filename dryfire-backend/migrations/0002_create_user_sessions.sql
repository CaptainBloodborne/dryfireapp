-- 0002_create_user_sessions.sql
-- Server-side session table for refresh tokens.
-- The token returned to the client is `id.exp.hmac(id.exp)` — see TokenGenerator.
-- Storing sessions in DB lets us revoke individual sessions, list active devices, etc.

CREATE TABLE user_sessions (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL,
    last_seen_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at    TIMESTAMPTZ,
    user_agent    TEXT,
    ip_address    INET
);

CREATE INDEX user_sessions_user_id_idx ON user_sessions (user_id);
CREATE INDEX user_sessions_expires_at_idx ON user_sessions (expires_at);
