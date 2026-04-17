// G6 — admin extend-expiration verb.
//
// An already-expired key returns 401 (DenialReason::Expired). After
// an admin extends `expired_at` into the future, the same (key,
// device) pair must authenticate successfully.
//
// Cache-priming caveat (confirmed):
//   In `application/authenticate_api_key.rs::resolve_snapshot`, a
//   DB hit is `cache.put(snapshot)`-ed BEFORE the decision is made.
//   So our BEFORE-POST (which returns 401 Expired) still warms the
//   cache with an expired snapshot. Without eviction the AFTER-POST
//   would also return 401 until AUTH_CACHE_TTL_SECONDS=2 elapses.
//
//   `application/extend_expiration.rs` evicts the `(key, device)`
//   cache entry on success — so the AFTER-POST is observably 200
//   immediately after the modal closes, exercising G6.

import { test, expect } from '@playwright/test';
import { withDb, insertKey } from '../../fixtures/db';
import { BASE_URL } from '../../fixtures/env';

const KEY = 'e2e_hardB_expired_key';
const DEVICE = 'e2e_hardB_expired_dev';

test.describe.serial('extend expiration revives an expired key', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_hardB_%' OR device_id LIKE 'e2e_hardB_%'",
      );
      // Expired 1 day ago; remaining quota intact so the ONLY reason
      // for denial is expiry.
      await insertKey(c, {
        key: KEY,
        device_id: DEVICE,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
        expired_at: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
        username: 'extend-e2e',
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

  test('expired → extend → authenticates', async ({ page, playwright }) => {
    const apiCtx = await playwright.request.newContext({ baseURL: BASE_URL });

    // BEFORE: denied for Expired (401). Also primes the cache with
    // the expired snapshot — eviction on extend is what makes the
    // AFTER-POST observable without waiting for TTL.
    const before = await apiCtx.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(before.status()).toBe(401);

    // Admin extends expired_at via the Edit modal.
    // The filter by device matches the "show expired" filter since the
    // list includes expired rows; if filtering is required, the row is
    // found by device string in either active or expired view.
    await page.goto('/keys');
    // Some admin tables default to "active" only. Try expired filter
    // if the row isn't initially present.
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    if ((await row.count()) === 0) {
      // Status filter is the 2nd <select> on the keys page (see
      // tests/keys/delete.spec.ts). "expired" reveals expired rows.
      await page.locator('select').nth(1).selectOption('expired');
    }
    await expect(row).toBeVisible();
    await row.getByRole('button', { name: /Edit/i }).click();

    const modal = page.getByRole('heading', { name: 'Edit API Key' }).locator('..').locator('..');
    await expect(modal).toBeVisible();

    // `datetime-local` input — format `YYYY-MM-DDTHH:mm`, 1 year ahead.
    const future = new Date(Date.now() + 365 * 24 * 60 * 60 * 1000);
    const pad = (n: number) => String(n).padStart(2, '0');
    const futureStr = `${future.getFullYear()}-${pad(future.getMonth() + 1)}-${pad(
      future.getDate(),
    )}T${pad(future.getHours())}:${pad(future.getMinutes())}`;

    const expiredInput = modal.locator('#key-expired-at');
    await expiredInput.fill(futureStr);
    await modal.getByRole('button', { name: /Update Key/i }).click();

    await expect(page.getByRole('heading', { name: 'Edit API Key' })).toHaveCount(0);

    // AFTER: the expired-snapshot cache entry was evicted by
    // ExtendExpiration, so the next /v1/auth call hits the DB
    // and sees the new expiry. Must be 200.
    const after = await apiCtx.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(after.status()).toBe(200);
    const body = await after.json();
    expect(body.status).toBe('success');

    await apiCtx.dispose();
  });
});
