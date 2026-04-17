-- Phase 3.3 — durable audit trail for domain events.
--
-- The application layer already emits past-tense log lines
-- (`KeyIssued`, `KeyRevoked`, `DeviceReassigned`, `RateLimitReset`,
-- `KeyExpirationExtended`, `AuthenticationSucceeded`,
-- `AuthenticationDenied`). Plain-text log files rotate out; they
-- cannot answer "who revoked this key, and when?" three months later.
--
-- This table is the durable log. 3.4 adds the outbox adapter that
-- writes events here from the use-case layer. Intentionally narrow so
-- the write path stays cheap:
--
--   - event_type : past-tense domain verb, e.g. `KeyRevoked`.
--   - aggregate_id: the issued key's id when the event targets one;
--                   NULL for events that do not carry an aggregate
--                   (e.g. `AuthenticationDenied { UnknownKey }`).
--   - actor       : who triggered the event — `"admin"` for admin
--                   commands, `"system"` for auto-provisioned rows,
--                   a device id for `/v1/auth` originators, etc.
--   - payload     : event-specific detail as JSON. Deliberately
--                   schemaless so 3.4 can evolve the payload shape
--                   without another migration.
--   - occurred_at : single source of truth for the event's time.
--
-- Two indexes cover the dashboards we expect: recent-first scrolling
-- and "show me everything about key 42".

CREATE TABLE IF NOT EXISTS audit_log (
    id            BIGSERIAL PRIMARY KEY,
    event_type    VARCHAR(100) NOT NULL,
    aggregate_id  INTEGER,
    actor         VARCHAR(255) NOT NULL,
    payload       JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_log_occurred_at
  ON audit_log (occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_log_aggregate
  ON audit_log (aggregate_id, occurred_at DESC)
  WHERE aggregate_id IS NOT NULL;
