import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey, upsertInterval } from '../../fixtures/db';

const KEY = 'klab_e2e_api_ratelimit_0001';
const DEVICE = 'api-device-ratelimit';
const INTERVAL_NAME = 'e2e_short_interval';
const INTERVAL_SECONDS = 3;

test.describe('POST /v1/auth — rate limit exhaustion and reset', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, KEY);
      await upsertInterval(c, INTERVAL_NAME, 'E2E Short', INTERVAL_SECONDS);
      await insertKey(c, {
        key: KEY,
        device_id: DEVICE,
        subscription_type_name: 'free',
        rate_limit_interval_name: INTERVAL_NAME,
        rate_limit_daily: 2,
        rate_limit_remaining: 2,
      });
    });
  });

  test.afterAll(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, KEY);
      await c.query('DELETE FROM rate_limit_intervals WHERE name = $1', [INTERVAL_NAME]);
    });
  });

  test('exhaust the limit → 429 → wait interval → 200', async ({ request }) => {
    // daily=2, remaining=2. Predicate is `>=`, so both slots are
    // consumable: first call → remaining=1, second call → remaining=0,
    // third call is refused (remaining < usage).
    const first = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(first.status()).toBe(200);

    const second = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(second.status()).toBe(200);

    const third = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(third.status()).toBe(429);
    const body = await third.json();
    expect(body.message).toMatch(/Rate limit exceeded/i);

    // Wait past the interval + a margin so the next call resets.
    // Also need to wait past the AUTH_CACHE_TTL_SECONDS=2 set in fixtures/env.ts.
    await new Promise((r) => setTimeout(r, (INTERVAL_SECONDS + 3) * 1000));

    const fourth = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(fourth.status()).toBe(200);
  });
});
