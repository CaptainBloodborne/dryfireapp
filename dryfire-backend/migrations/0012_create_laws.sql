-- 0012_create_laws.sql
--
-- The laws domain has three tables:
--
-- `law_categories` — flat (for now) taxonomy of law topics:
--    оборот, транспортировка, хранение, охота, самооборона, ...
-- `laws`           — the current version of each document. One row per
--   legal act. UNIQUE on `law_key` so consumers can refer to acts by a
--   stable slug (e.g. `fz-150-1996`).
-- `law_versions`   — immutable history. Every time a law is updated,
--   we (a) snapshot the *previous* state into law_versions, and
--   (b) bump `laws.current_version`. A SELECT on law_versions WHERE
--   law_key = X gives the full revision history.
--
-- Tags are a fixed enumeration stored as a TEXT[] for cheap GIN
-- intersection (`tags && ARRAY['hunting']`). Categories are FK because
-- they're admin-managed.
--
-- Full-text search uses a single tsvector column populated by a
-- BEFORE INSERT/UPDATE trigger. It combines a `russian` and an
-- `english` config so the same column matches stems from either
-- language (the most common case for RU legal text quoting English
-- treaty names, manufacturer names, etc.). Title is weighted A,
-- summary B, body C — affects ts_rank ordering.

CREATE TABLE law_categories (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code        TEXT NOT NULL UNIQUE,         -- slug: 'storage', 'transport', 'hunting', ...
    name        TEXT NOT NULL,
    parent_id   UUID REFERENCES law_categories(id) ON DELETE SET NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX law_categories_parent_idx ON law_categories (parent_id);

-- Fixed vocabulary for the `tags` column. A CHECK constraint validates
-- each element; using an array (rather than a junction table) keeps
-- single-document reads as one row and lets GIN handle filtering.
CREATE TABLE laws (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    law_key          TEXT NOT NULL UNIQUE,        -- stable external slug
    title            TEXT NOT NULL,
    summary          TEXT,
    body             TEXT NOT NULL,
    region           TEXT NOT NULL DEFAULT 'RU',
    category_id      UUID REFERENCES law_categories(id) ON DELETE SET NULL,
    tags             TEXT[] NOT NULL DEFAULT '{}',
    -- See CHECK below.
    current_version  INTEGER NOT NULL DEFAULT 1,
    -- The actual issuance / amendment date (NOT NULL — we always know).
    effective_at     DATE NOT NULL,
    -- App-side timestamps for "what changed since" queries.
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    search_vector    tsvector,

    CONSTRAINT laws_tags_valid CHECK (
        tags <@ ARRAY[
            'transport', 'carry', 'storage', 'hunting',
            'self_defense', 'sport', 'license', 'inheritance',
            'inspection', 'penalty', 'other'
        ]::text[]
    )
);

CREATE INDEX laws_region_idx       ON laws (region);
CREATE INDEX laws_category_idx     ON laws (category_id);
CREATE INDEX laws_updated_idx      ON laws (updated_at DESC);
CREATE INDEX laws_tags_gin         ON laws USING GIN (tags);
CREATE INDEX laws_search_gin       ON laws USING GIN (search_vector);

-- Trigger maintains search_vector. Why a trigger and not a generated
-- column: `tsvector_update_trigger()` doesn't take weights, and
-- `GENERATED ... STORED` can't use immutable subexpressions involving
-- `to_tsvector('russian', col)` because the text-search config is not
-- considered IMMUTABLE in Postgres < 15. A trigger sidesteps both.

CREATE OR REPLACE FUNCTION laws_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('russian', coalesce(NEW.title,   '')), 'A') ||
        setweight(to_tsvector('english', coalesce(NEW.title,   '')), 'A') ||
        setweight(to_tsvector('russian', coalesce(NEW.summary, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(NEW.summary, '')), 'B') ||
        setweight(to_tsvector('russian', coalesce(NEW.body,    '')), 'C') ||
        setweight(to_tsvector('english', coalesce(NEW.body,    '')), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER laws_search_vector_trigger
    BEFORE INSERT OR UPDATE OF title, summary, body
    ON laws
    FOR EACH ROW
    EXECUTE FUNCTION laws_search_vector_update();

-- Immutable history. Snapshots are taken at the *moment of an update*,
-- recording what the row looked like *before* the change. So the first
-- snapshot of a law lands when the second version is published.
-- Inserts and reads only — never updated, never deleted (cascade aside).
CREATE TABLE law_versions (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    law_id          UUID NOT NULL REFERENCES laws(id) ON DELETE CASCADE,
    law_key         TEXT NOT NULL,                -- denormalized for history queries when the parent is gone
    version         INTEGER NOT NULL,
    title           TEXT NOT NULL,
    summary         TEXT,
    body            TEXT NOT NULL,
    tags            TEXT[] NOT NULL DEFAULT '{}',
    category_id     UUID,
    effective_at    DATE NOT NULL,
    snapshot_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (law_id, version)
);

CREATE INDEX law_versions_law_idx  ON law_versions (law_id);
CREATE INDEX law_versions_key_idx  ON law_versions (law_key);

-- Snapshot trigger: copies the OLD row into law_versions whenever the
-- substantive body/title/summary/tags/category changes. Updates that
-- only touch metadata (e.g. updated_at, region rename) don't bump
-- versions — but they still trigger a search_vector refresh.

CREATE OR REPLACE FUNCTION laws_snapshot_on_update() RETURNS trigger AS $$
BEGIN
    IF NEW.title <> OLD.title
       OR NEW.body <> OLD.body
       OR NEW.summary IS DISTINCT FROM OLD.summary
       OR NEW.tags <> OLD.tags
       OR NEW.category_id IS DISTINCT FROM OLD.category_id
       OR NEW.effective_at <> OLD.effective_at
    THEN
        INSERT INTO law_versions
            (law_id, law_key, version, title, summary, body,
             tags, category_id, effective_at, snapshot_at)
        VALUES
            (OLD.id, OLD.law_key, OLD.current_version,
             OLD.title, OLD.summary, OLD.body,
             OLD.tags, OLD.category_id, OLD.effective_at, NOW());

        NEW.current_version := OLD.current_version + 1;
        NEW.updated_at := NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER laws_snapshot_trigger
    BEFORE UPDATE
    ON laws
    FOR EACH ROW
    EXECUTE FUNCTION laws_snapshot_on_update();
