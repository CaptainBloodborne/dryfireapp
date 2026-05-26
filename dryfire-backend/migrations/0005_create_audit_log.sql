-- 0005_create_audit_log.sql
-- "Система должна вести журнал аудита всех операций доступа к данным
--  и их изменения."
--
-- This is a forensic table — append-only, queryable, never UPDATEd.
-- One row per write operation (create/update/delete on user-owned data).
-- Reads are not logged here (would be too noisy); request access is
-- already captured by the tracing layer's request log.

CREATE TABLE audit_log (
    id            BIGSERIAL PRIMARY KEY,
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id       UUID,                    -- nullable for unauthenticated events (e.g. failed login)
    action        TEXT NOT NULL,           -- 'gun.create', 'license.update', 'auth.login', ...
    resource_type TEXT,                    -- 'gun', 'license', 'session', ...
    resource_id   TEXT,                    -- the affected entity's id (string so it can hold uuid/int/uuid+suffix)
    request_id    TEXT,                    -- correlates with the x-request-id header
    ip_address    INET,
    metadata      JSONB                    -- free-form: { "old_status": "pending", "new_status": "verified" }
);

-- Common query: "what did user X do between Y and Z?"
CREATE INDEX audit_log_user_time_idx ON audit_log (user_id, occurred_at DESC);

-- Common query: "what happened to resource R?"
CREATE INDEX audit_log_resource_idx ON audit_log (resource_type, resource_id);
