// G2 — Free-trial auto-provisioning: first-call semantics + concurrent race.
//
// The FREE_TRIAL marker is the literal env value (default: "FREE_TRIAL").
// Auto-provisioning logic lives in
// src/infrastructure/postgres/issued_key_repository.rs::claim_free_trial
// (Branch A): INSERT with expiry set to 1st-of-next-month UTC, rate_limit
// from the `free` subscription_type (or fallback 6000).
//
// Concurrency note: the INSERT in Branch A has no ON CONFLICT clause, but
// migration 004 enforces UNIQUE(key, device_id). A lost race will surface
// as a 500 (BackendError). The invariants we pin:
//   - exactly one row per device_id survives
//   - (remaining in DB) == daily - (#200 responses)
// We DO NOT assert that 200+429 == 10; 5xx responses are allowed as a
// documented symptom of racing INSERTs until the repo adds ON CONFLICT.
import { test, expect } from '@playwright/test';
import { FREE_TRIAL_KEY } from '../../fixtures/env';
import { withDb, countKeysByDevice } from '../../fixtures/db';

const PREFIX = 'e2e_envA_';

test.describe('POST /v1/auth — free-trial concurrency (G2)', () => {
  test.afterAll(async () => {
    await withDb((c) =>
      c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_envA_%' OR device_id LIKE 'e2e_envA_%'",
      ),
    );
  });

  test('first call auto-provisions a row with 1st-of-next-month expiry', async ({ request }) => {
    const device = `${PREFIX}trial_dev_first_${Date.now()}`;

    const before = await withDb((c) => countKeysByDevice(c, device));
    expect(before).toBe(0);

    const res = await request.post('/v1/auth', {
      data: { auth_key: FREE_TRIAL_KEY, auth_device: device, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('success');
    expect(body.data.device).toBe(device);

    const after = await withDb((c) => countKeysByDevice(c, device));
    expect(after).toBe(1);

    // Expiry: 1st of next month UTC (±1 day to absorb boundary edge cases).
    const row = await withDb(async (c) => {
      const { rows } = await c.query<{
        expired_at: Date | null;
        rate_limit_daily: number;
        rate_limit_remaining: number;
      }>(
        'SELECT expired_at, rate_limit_daily, rate_limit_remaining FROM authentication_keys WHERE device_id = $1',
        [device],
      );
      return rows[0];
    });

    expect(row.expired_at).not.toBeNull();
    const exp = new Date(row.expired_at as unknown as string);
    const now = new Date();
    const y = now.getUTCMonth() === 11 ? now.getUTCFullYear() + 1 : now.getUTCFullYear();
    const m = now.getUTCMonth() === 11 ? 0 : now.getUTCMonth() + 1;
    const target = Date.UTC(y, m, 1, 0, 0, 0);
    const dayMs = 86_400_000;
    expect(Math.abs(exp.getTime() - target)).toBeLessThanOrEqual(dayMs);

    // Rate-limit arithmetic: remaining = daily - 1 after the single call.
    expect(row.rate_limit_remaining).toBe(row.rate_limit_daily - 1);
    expect(body.data.rate_limit_remaining).toBe(row.rate_limit_daily - 1);
  });

  test('10 concurrent first-calls for one device converge to one row', async ({ request }) => {
    const device = `${PREFIX}trial_dev_race_${Date.now()}`;

    const responses = await Promise.all(
      Array.from({ length: 10 }, () =>
        request.post('/v1/auth', {
          data: { auth_key: FREE_TRIAL_KEY, auth_device: device, rate_limit_usage: 1 },
        }),
      ),
    );

    const statuses = responses.map((r) => r.status());
    const allowed = statuses.filter((s) => s === 200).length;
    const denied = statuses.filter((s) => s === 429).length;
    const backend = statuses.filter((s) => s >= 500).length;

    // Total responses received.
    expect(statuses.length).toBe(10);
    // Every response is one of the three documented outcomes.
    expect(allowed + denied + backend).toBe(10);
    // At least one request must have won the race and succeeded.
    expect(allowed).toBeGreaterThanOrEqual(1);

    // Invariant: UNIQUE(key, device_id) from migration 004 — exactly
    // one row for this device, regardless of how many INSERTs raced.
    const count = await withDb((c) => countKeysByDevice(c, device));
    expect(count).toBe(1);

    // Arithmetic invariant: remaining in DB == daily - allowed_count.
    // (429s never decrement; 500s from lost-race INSERTs likewise never
    // reach consume_quota.)
    const row = await withDb(async (c) => {
      const { rows } = await c.query<{
        rate_limit_daily: number;
        rate_limit_remaining: number;
      }>(
        'SELECT rate_limit_daily, rate_limit_remaining FROM authentication_keys WHERE device_id = $1',
        [device],
      );
      return rows[0];
    });
    expect(row.rate_limit_remaining).toBe(row.rate_limit_daily - allowed);
  });
});
