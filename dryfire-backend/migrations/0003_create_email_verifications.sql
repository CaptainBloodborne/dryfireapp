-- 0003_create_email_verifications.sql
-- One-time-use email verification tokens. The token we email out is
-- random opaque; we store only its HMAC so a DB leak cannot replay tokens.

CREATE TABLE email_verifications (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   TEXT NOT NULL UNIQUE,        -- hex of HMAC(token, server_key)
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX email_verifications_user_id_idx ON email_verifications (user_id);
