-- 0010_create_licenses.sql
--
-- Four tables for the licenses domain:
--
-- `license_types`  — admin-managed reference list. Each type carries
--   a "default validity" (in days) so we can auto-compute expiry, and
--   freeform `instructions` text for obtaining/renewing.
-- `licenses`       — user-owned records.
-- `license_guns`   — many-to-many between licenses and guns.
-- `license_notifications` — bookkeeping for the scheduler: which
--   reminders have been *sent* for each license, so we never double-send.
--
-- "Чувствительные поля (... сканы лицензий ...) должны быть зашифрованы
--  при хранении." — the `scan_blob_ciphertext` column holds an
-- AES-GCM-encrypted scan (image bytes). The `scan_url` is a plaintext
-- reference for clients that already uploaded to object storage.

CREATE TABLE license_types (
    id                   UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code                 TEXT NOT NULL UNIQUE,         -- 'storage', 'hunting', 'self_defense', ...
    name                 TEXT NOT NULL,
    region               TEXT NOT NULL DEFAULT 'RU',
    -- Default validity period in days (e.g. 5 years = 1826).
    -- NULL means manual expiry only — no auto-compute.
    validity_days        INTEGER,
    instructions         TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX license_types_region_idx ON license_types (region);

CREATE TABLE licenses (
    id                       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id                  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    license_type_id          UUID REFERENCES license_types(id) ON DELETE SET NULL,
    license_number           TEXT NOT NULL,
    issuer                   TEXT NOT NULL,
    issued_at                DATE NOT NULL,
    expires_at               DATE NOT NULL,
    -- Free-form, plaintext. Use the *_encrypted columns for sensitive bits.
    notes                    TEXT,
    notes_encrypted          BYTEA,
    -- URL of an uploaded scan in object storage. The blob itself lives
    -- outside Postgres; encryption is the storage tier's responsibility.
    scan_url                 TEXT,
    -- Inline-stored encrypted scan (for small / dev cases without S3).
    scan_blob_ciphertext     BYTEA,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX licenses_user_idx          ON licenses (user_id);
CREATE INDEX licenses_user_expires_idx  ON licenses (user_id, expires_at);
-- For the scheduler: find every license that expires soon, regardless of user.
CREATE INDEX licenses_expires_idx       ON licenses (expires_at);

-- Many-to-many: a license can cover multiple guns; a gun can be
-- referenced by multiple licenses (historical or overlapping).
CREATE TABLE license_guns (
    license_id  UUID NOT NULL REFERENCES licenses(id) ON DELETE CASCADE,
    gun_id      UUID NOT NULL REFERENCES guns(id)     ON DELETE CASCADE,
    PRIMARY KEY (license_id, gun_id)
);

CREATE INDEX license_guns_gun_idx ON license_guns (gun_id);

-- Records which reminder (90, 60, 45, 30, 14 days) was sent for which
-- license. The unique constraint makes the scheduler idempotent — if
-- it retries, the second INSERT is rejected and no duplicate email
-- goes out.
CREATE TABLE license_notifications (
    id              BIGSERIAL PRIMARY KEY,
    license_id      UUID NOT NULL REFERENCES licenses(id) ON DELETE CASCADE,
    days_before     INTEGER NOT NULL,
    sent_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (license_id, days_before)
);

CREATE INDEX license_notifications_license_idx ON license_notifications (license_id);
