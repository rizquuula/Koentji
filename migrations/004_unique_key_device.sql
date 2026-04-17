-- Phase 3.1 — close the concurrent-insert race (B10).
--
-- Since migration 003 removed `UNIQUE(key)` to allow FREE_TRIAL to be
-- shared across devices, nothing prevents two simultaneous inserts
-- from creating duplicate `(key, device_id)` rows. The
-- `IssuedKey` aggregate implicitly assumes this tuple is unique
-- (`find` / `consume_quota` / `revoke_key` all address a row by it).
-- Enforce it at the database.
--
-- Defensive dedup runs first so the unique index creation does not
-- fail on ambient data. The `a.id < b.id` predicate keeps the
-- newest row per `(key, device_id)` — admin-issued rows tend to be
-- re-issued, so "last write wins" matches how operators already
-- think about the system.

DELETE FROM authentication_keys a
USING authentication_keys b
WHERE a.key = b.key
  AND a.device_id = b.device_id
  AND a.id < b.id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_keys_key_device_unique
  ON authentication_keys (key, device_id);
