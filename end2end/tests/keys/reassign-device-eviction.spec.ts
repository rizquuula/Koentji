// G4 — admin device reassignment evicts BOTH cache entries
// (key, prev_dev) and (key, new_dev).
//
// Flow:
//  1. Seed a key bound to `e2e_hardB_prev_dev`.
//  2. Prime the cache by authing successfully through /v1/auth with
//     the prev_dev — now (key, prev_dev) is cached as Allowed.
//  3. Drive the admin UI to reassign the device to `e2e_hardB_new_dev`.
//     The ReassignDevice use case evicts both cache keys and updates
//     the DB row.
//  4. Auth with the OLD device → must be 401 (row is gone AND cache
//     was evicted; if eviction had failed, the still-cached snapshot
//     would serve 200 until AUTH_CACHE_TTL_SECONDS=2 elapsed).
//  5. Auth with the NEW device → must be 200.

import { test, expect } from '@playwright/test';
import { withDb, insertKey } from '../../fixtures/db';
import { BASE_URL } from '../../fixtures/env';

const KEY = 'e2e_hardB_reassign_key';
const PREV_DEV = 'e2e_hardB_prev_dev';
const NEW_DEV = 'e2e_hardB_new_dev';

test.describe.serial('reassign device evicts both cache entries', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_hardB_%' OR device_id LIKE 'e2e_hardB_%'",
      );
      await insertKey(c, {
        key: KEY,
        device_id: PREV_DEV,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
        username: 'reassign-e2e',
      });
    });
  });

  test.afterAll(async () => {
    await withDb((c) =>
      c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_hardB_%' OR device_id LIKE 'e2e_hardB_%'",
      ),
    );
  });

  test('prev_dev 401 after reassign; new_dev 200', async ({ page, playwright }) => {
    // Step 2: prime the cache — anonymous api context (no admin cookie).
    const apiCtx = await playwright.request.newContext({ baseURL: BASE_URL });
    const primed = await apiCtx.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: PREV_DEV, rate_limit_usage: 1 },
    });
    expect(primed.status()).toBe(200);

    // Step 3: drive the admin UI.
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: PREV_DEV });
    await row.getByRole('button', { name: /Edit/i }).click();

    const modal = page.getByRole('heading', { name: 'Edit API Key' }).locator('..').locator('..');
    await expect(modal).toBeVisible();

    const deviceInput = modal.locator('#key-device-id');
    await deviceInput.fill(NEW_DEV);
    await modal.getByRole('button', { name: /Update Key/i }).click();

    // Modal closes on success; row now renders new device id.
    await expect(page.getByRole('heading', { name: 'Edit API Key' })).toHaveCount(0);
    await expect(page.getByRole('row').filter({ hasText: NEW_DEV })).toBeVisible();

    // Step 4: old device must fail — if eviction didn't happen, the
    // cached allow-state would serve 200 for up to AUTH_CACHE_TTL_SECONDS=2.
    // We probe immediately so the test exercises the eviction path, not TTL.
    const oldRes = await apiCtx.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: PREV_DEV, rate_limit_usage: 1 },
    });
    expect(oldRes.status()).toBe(401);

    // Step 5: new device succeeds.
    const newRes = await apiCtx.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: NEW_DEV, rate_limit_usage: 1 },
    });
    expect(newRes.status()).toBe(200);

    await apiCtx.dispose();
  });
});
