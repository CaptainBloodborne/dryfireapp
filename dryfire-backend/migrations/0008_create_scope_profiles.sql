-- 0008_create_scope_profiles.sql

CREATE TABLE scope_profiles (
    id                    UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id               UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    gun_id                UUID,
    name                  TEXT NOT NULL,
    unit                  TEXT NOT NULL,         -- 'moa' | 'iphy' | 'mil'
    click_fraction        DOUBLE PRECISION NOT NULL,
    max_elevation_units   DOUBLE PRECISION,
    max_windage_units     DOUBLE PRECISION,
    mount_height_mm       DOUBLE PRECISION NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, name)
);

CREATE INDEX scope_profiles_user_idx ON scope_profiles (user_id);
