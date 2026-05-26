-- 0009_create_armory.sql
--
-- Two tables:
--
-- `gun_catalog` — admin-curated reference of common firearm models.
-- All users can read; only admins can write. Used by the client to
-- autofill a Gun record when the user picks "Sako TRG-22" from a
-- dropdown.
--
-- `guns` — the user's own arsenal records. One row per physical
-- firearm. `serial_ciphertext` is AES-256-GCM ciphertext of the serial
-- number — the application encrypts before INSERT and decrypts after
-- SELECT. We also store `serial_last4_plain` so the user can spot-check
-- their list of guns ("which one is …1234?") without us having to
-- decrypt every row.
--
-- Spec: "Чувствительные поля (серийные номера ...) должны быть
-- зашифрованы при хранении."

CREATE TABLE gun_catalog (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    manufacturer    TEXT NOT NULL,
    model           TEXT NOT NULL,
    class           TEXT NOT NULL,            -- 'rifle' | 'shotgun' | 'pistol' | 'revolver' | 'smg' | 'other'
    caliber         TEXT NOT NULL,            -- '.308 Win', '12/76', '9x19', ...
    barrel_length_mm INTEGER,
    weight_g        INTEGER,
    capacity        INTEGER,                  -- magazine/cylinder capacity
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (manufacturer, model)
);

CREATE INDEX gun_catalog_caliber_idx ON gun_catalog (caliber);
CREATE INDEX gun_catalog_class_idx   ON gun_catalog (class);

CREATE TABLE guns (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    catalog_id          UUID REFERENCES gun_catalog(id) ON DELETE SET NULL,
    manufacturer        TEXT NOT NULL,
    model               TEXT NOT NULL,
    class               TEXT NOT NULL,
    caliber             TEXT NOT NULL,
    serial_ciphertext   BYTEA NOT NULL,        -- AES-256-GCM(serial)
    serial_last4_plain  TEXT,                  -- e.g. "1234" — last 4 chars of serial, plaintext, for UI hints
    date_of_purchase    DATE NOT NULL,
    photo_url           TEXT,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX guns_user_idx          ON guns (user_id);
CREATE INDEX guns_user_class_idx    ON guns (user_id, class);
CREATE INDEX guns_user_caliber_idx  ON guns (user_id, caliber);
