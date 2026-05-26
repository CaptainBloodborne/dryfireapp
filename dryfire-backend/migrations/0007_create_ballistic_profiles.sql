-- 0007_create_ballistic_profiles.sql
-- We persist the nested Bullet/Sight/Atmosphere structs as JSONB.
-- This keeps the schema simple, avoids a wide table with 12 numeric
-- columns that change shape if we ever add a parameter (e.g. spin-drift
-- rate), and lets us query e.g. by caliber with
-- `WHERE bullet->>'caliber_mm' = '...'` if we ever need to.
--
-- The trade-off: no type-checking at the DB level. We rely on the
-- application's `serde_json` to enforce shape, which is fine since the
-- application is the only writer.

CREATE TABLE ballistic_profiles (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    gun_id       UUID,
    ammo_id      UUID,
    bullet       JSONB NOT NULL,
    sight        JSONB NOT NULL,
    atmosphere   JSONB NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, name)
);

CREATE INDEX ballistic_profiles_user_idx ON ballistic_profiles (user_id);
