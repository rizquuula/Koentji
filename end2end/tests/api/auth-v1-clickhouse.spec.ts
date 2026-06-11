// /v1/auth → ClickHouse roundtrip.
//
// After the v2→v1 merge, /v1/auth is the sole auth endpoint and emits an
// AuthEvent for every request that lands in the `auth_events` table via
// the bounded mpsc + 1 s batch flush. The hot path is fire-and-forget, so
// we poll on a deadline rather than assuming a single SELECT sees the row
// immediately.
import { test, expect } from '@playwright/test';
import { withDb, insertKey, setRateLimitRemaining } from '../../fixtures/db';
import { chExec, waitForAuthEvents } from '../../fixtures/clickhouse';

const PREFIX = 'e2e_v1ch_';
const KEYS = {
  allowed: `${PREFIX}allowed`,
  denied: `${PREFIX}denied`,
  unknown: `${PREFIX}unknown`,
};
const DEVICES = {
  allowed: `${PREFIX}allowed_dev`,
  denied: `${PREFIX}denied_dev`,
  unknown: `${PREFIX}unknown_dev`,
};

test.describe('POST /v1/auth — ClickHouse auth_events emit', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v1ch_%' OR device_id LIKE 'e2e_v1ch_%'",
      );
      await insertKey(c, {
        key: KEYS.allowed,
        device_id: DEVICES.allowed,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
      const denied = await insertKey(c, {
        key: KEYS.denied,
        device_id: DEVICES.denied,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
      // Stamp updated_at so the interval-reset branch in consume_quota
      // doesn't refill the bucket on first hit.
      await setRateLimitRemaining(c, denied.id, 0.25);
    });
    // Clear any stale rows from prior runs. Mutation, not a SELECT.
    await chExec(`ALTER TABLE auth_events DELETE WHERE auth_key LIKE 'e2e_v1ch_%'`);
  });

  test.afterAll(async () => {
    await withDb((c) =>
      c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v1ch_%' OR device_id LIKE 'e2e_v1ch_%'",
      ),
    );
  });

  test('Allowed v1 request emits an Allowed AuthEvent', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.allowed, auth_device: DEVICES.allowed, rate_limit_usage: 0.5 },
    });
    expect(res.status()).toBe(200);

    const rows = await waitForAuthEvents(`auth_key = '${KEYS.allowed}'`, 1);
    expect(rows).toHaveLength(1);
    const row = rows[0];
    expect(row.decision).toBe('allowed');
    expect(row.device_id).toBe(DEVICES.allowed);
    // The event carries the true fractional usage/remaining even though
    // the HTTP envelope ceils the remaining away.
    expect(row.usage).toBeCloseTo(0.5, 9);
    expect(row.remaining_after).toBeCloseTo(99.5, 9);
    expect(row.denial_reason).toBe('');
    expect(Number(row.auth_key_id)).toBeGreaterThan(0);
  });

  test('429 v1 request emits a Denied AuthEvent with RateLimitExceeded', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.denied, auth_device: DEVICES.denied, rate_limit_usage: 1.0 },
    });
    expect(res.status()).toBe(429);

    const rows = await waitForAuthEvents(`auth_key = '${KEYS.denied}'`, 1);
    expect(rows).toHaveLength(1);
    expect(rows[0].decision).toBe('denied');
    expect(rows[0].denial_reason).toBe('RateLimitExceeded');
    expect(rows[0].remaining_after).toBe(0);
  });

  test('401 unknown v1 request emits a Denied AuthEvent with UnknownKey', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.unknown, auth_device: DEVICES.unknown, rate_limit_usage: 1.0 },
    });
    expect(res.status()).toBe(401);

    const rows = await waitForAuthEvents(`auth_key = '${KEYS.unknown}'`, 1);
    expect(rows).toHaveLength(1);
    expect(rows[0].decision).toBe('denied');
    expect(rows[0].denial_reason).toBe('UnknownKey');
  });
});
