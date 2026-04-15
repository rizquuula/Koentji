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
    const first = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(first.status()).toBe(200);

    // Second call brings remaining to 0; endpoint returns 429 when new_remaining <= 0.
    const second = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    // Per main.rs:365, the 429 triggers when new_remaining <= 0. Second call: 1 - 1 = 0 → 429.
    expect(second.status()).toBe(429);
    const body = await second.json();
    expect(body.message).toMatch(/Rate limit exceeded/i);

    // Wait past the interval + a margin so the next call resets.
    // Also need to wait past the AUTH_CACHE_TTL_SECONDS=2 set in fixtures/env.ts.
    await new Promise((r) => setTimeout(r, (INTERVAL_SECONDS + 3) * 1000));

    const third = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(third.status()).toBe(200);
  });
});
