-- 0011_create_ammo.sql
--
-- Three tables for the ammo domain.
--
-- `ammo_types` — admin-managed catalog of ammo SKUs.
-- `ammo_stocks` — per-user, per-type current on-hand count. A
--   *materialized aggregate* derived from the transaction log so
--   reads are O(1). The transaction log is the source of truth; the
--   stocks row is the cached current value, updated atomically by
--   the same transaction that inserts each ledger entry.
-- `ammo_transactions` — immutable append-only ledger. One row per
--   acquire / consume / adjustment event. Deletes are forbidden at
--   the app layer (the spec says "вести учет всех транзакций" — a
--   record that gets deleted isn't really a record).
--
-- Why a stocks table at all (instead of computing totals from the log
-- on every read): the most common UI screen is "current quantity of
-- every ammo type I own", which would otherwise need a GROUP BY scan
-- of every row in the ledger. With this table the same query is a
-- single index hit.

CREATE TYPE bullet_type AS ENUM (
    'fmj',      -- full metal jacket
    'jhp',      -- jacketed hollow point
    'sp',       -- soft point
    'lrn',      -- lead round nose
    'wad',      -- wadcutter
    'match',    -- match-grade
    'slug',     -- shotgun slug
    'buckshot', -- shotgun buckshot
    'birdshot', -- shotgun birdshot
    'other'
);

CREATE TYPE projectile_type AS ENUM (
    'centerfire',
    'rimfire',
    'shotshell',
    'blank',
    'other'
);

CREATE TABLE ammo_types (
    id                   UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    manufacturer         TEXT NOT NULL,
    name                 TEXT NOT NULL,        -- e.g. "Match King 175gr"
    caliber              TEXT NOT NULL,
    bullet_type          bullet_type NOT NULL,
    projectile_type      projectile_type NOT NULL,
    -- Powder charge in grains. Optional — many factory cartridges
    -- don't publish this, and handloaders specify it themselves.
    powder_charge_grain  DOUBLE PRECISION,
    bullet_weight_grain  DOUBLE PRECISION,
    notes                TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (manufacturer, name, caliber)
);

CREATE INDEX ammo_types_caliber_idx       ON ammo_types (caliber);
CREATE INDEX ammo_types_bullet_idx        ON ammo_types (bullet_type);
CREATE INDEX ammo_types_projectile_idx    ON ammo_types (projectile_type);
CREATE INDEX ammo_types_manufacturer_idx  ON ammo_types (manufacturer);

CREATE TABLE ammo_stocks (
    user_id        UUID NOT NULL REFERENCES users(id)       ON DELETE CASCADE,
    ammo_type_id   UUID NOT NULL REFERENCES ammo_types(id)  ON DELETE CASCADE,
    -- Current on-hand quantity. CHECK keeps the cache honest: if the
    -- app ever tries to write a negative cached value, the constraint
    -- trips and the transaction rolls back (better than silently
    -- drifting from the ledger).
    quantity       INTEGER NOT NULL DEFAULT 0 CHECK (quantity >= 0),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, ammo_type_id)
);

CREATE INDEX ammo_stocks_user_idx ON ammo_stocks (user_id);

CREATE TABLE ammo_transactions (
    id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id        UUID NOT NULL REFERENCES users(id)      ON DELETE CASCADE,
    ammo_type_id   UUID NOT NULL REFERENCES ammo_types(id) ON DELETE RESTRICT,
    -- Optional: this transaction relates to a specific firearm
    -- (typical for consumption — "I shot 50 rounds with my Sako TRG").
    gun_id         UUID REFERENCES guns(id) ON DELETE SET NULL,
    -- Signed delta. Positive = acquired, negative = consumed.
    -- CHECK <> 0 prevents accidental no-op rows.
    delta          INTEGER NOT NULL CHECK (delta <> 0),
    -- Time the user *attributes* the event to (their range session,
    -- their purchase date). Distinct from `created_at` which is
    -- when the row was written.
    occurred_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    note           TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ammo_tx_user_occurred_idx ON ammo_transactions (user_id, occurred_at DESC);
CREATE INDEX ammo_tx_user_type_idx     ON ammo_transactions (user_id, ammo_type_id);
CREATE INDEX ammo_tx_user_gun_idx      ON ammo_transactions (user_id, gun_id) WHERE gun_id IS NOT NULL;
