-- 0001_create_users.sql
-- Core user table. Holds identity + profile attributes.
-- Sensitive auth data is split into `user_credentials` to allow
-- read-only access to user profile without touching the password hash.

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE user_status AS ENUM ('pending', 'verified', 'blocked');

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    login           VARCHAR(64)  NOT NULL,
    firstname       VARCHAR(128) NOT NULL,
    surname         VARCHAR(128) NOT NULL,
    email           VARCHAR(254) NOT NULL,           -- RFC 5321 max length
    date_of_birth   DATE         NOT NULL,
    region          VARCHAR(8)   NOT NULL DEFAULT 'RU',
    language        VARCHAR(8)   NOT NULL DEFAULT 'en',
    status          user_status  NOT NULL DEFAULT 'pending',
    last_visit_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Unique constraints on lowercased copies (citext would also work).
CREATE UNIQUE INDEX users_login_lower_idx ON users (LOWER(login));
CREATE UNIQUE INDEX users_email_lower_idx ON users (LOWER(email));

CREATE TABLE user_credentials (
    user_id        UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash  TEXT NOT NULL,             -- argon2 PHC string
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
