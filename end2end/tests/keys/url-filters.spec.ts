import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey } from '../../fixtures/db';

// G9 — URL query params are the source of truth for keys-page filters.
// Page 1 and empty search/status/subscription are encoded as NO param.

const PREFIX = 'e2e_feD_';
const KEY_ACTIVE_ALPHA = `${PREFIX}active_alpha`;
const KEY_ACTIVE_BETA = `${PREFIX}active_beta`;
const KEY_EXPIRED_GAMMA = `${PREFIX}expired_gamma`;

const DEV_ACTIVE_ALPHA = `${PREFIX}dev_alpha`;
const DEV_ACTIVE_BETA = `${PREFIX}dev_beta`;
const DEV_EXPIRED_GAMMA = `${PREFIX}dev_gamma`;

test.describe('keys page URL-roundtrip filters', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      // Clean any stragglers from a previous failed run so inserts don't clash.
      for (const k of [KEY_ACTIVE_ALPHA, KEY_ACTIVE_BETA, KEY_EXPIRED_GAMMA]) {
        await deleteKeyByKey(c, k);
      }
      await insertKey(c, {
        key: KEY_ACTIVE_ALPHA,
        device_id: DEV_ACTIVE_ALPHA,
        subscription_type_name: 'free',
        username: 'alpha-user',
        email: 'alpha@e2e.test',
      });
      await insertKey(c, {
        key: KEY_ACTIVE_BETA,
        device_id: DEV_ACTIVE_BETA,
        subscription_type_name: 'pro',
        username: 'beta-user',
        email: 'beta@e2e.test',
      });
      await insertKey(c, {
        key: KEY_EXPIRED_GAMMA,
        device_id: DEV_EXPIRED_GAMMA,
        subscription_type_name: 'free',
        expired_at: new Date(Date.now() - 24 * 3600 * 1000).toISOString(),
      });
    });
  });

  test.afterAll(async () => {
    await withDb(async (c) => {
      await c.query(`DELETE FROM authentication_keys WHERE key LIKE '${PREFIX}%'`);
    });
  });

  test('search filter round-trips via URL and survives reload', async ({ page }) => {
    // `list_keys` searches device_id/username/email — not the key itself.
    await page.goto(`/keys?search=${DEV_ACTIVE_ALPHA}`);

    await expect(page.getByRole('cell', { name: DEV_ACTIVE_ALPHA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_BETA })).toHaveCount(0);
    await expect(page.getByRole('cell', { name: DEV_EXPIRED_GAMMA })).toHaveCount(0);
    await expect(page).toHaveURL(new RegExp(`search=${DEV_ACTIVE_ALPHA}`));

    await page.reload();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_ALPHA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_BETA })).toHaveCount(0);
    await expect(page).toHaveURL(new RegExp(`search=${DEV_ACTIVE_ALPHA}`));
  });

  test('status=expired round-trips via URL and survives reload', async ({ page }) => {
    await page.goto('/keys?status=expired');

    await expect(page.getByRole('cell', { name: DEV_EXPIRED_GAMMA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_ALPHA })).toHaveCount(0);
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_BETA })).toHaveCount(0);
    await expect(page).toHaveURL(/status=expired/);

    await page.reload();
    await expect(page.getByRole('cell', { name: DEV_EXPIRED_GAMMA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_ALPHA })).toHaveCount(0);
    await expect(page).toHaveURL(/status=expired/);
  });

  test('combined search + status filters round-trip', async ({ page }) => {
    await page.goto(`/keys?search=${PREFIX}&status=active`);

    await expect(page.getByRole('cell', { name: DEV_ACTIVE_ALPHA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_BETA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_EXPIRED_GAMMA })).toHaveCount(0);
    await expect(page).toHaveURL(new RegExp(`search=${PREFIX}`));
    await expect(page).toHaveURL(/status=active/);

    await page.reload();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_ALPHA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_ACTIVE_BETA })).toBeVisible();
    await expect(page.getByRole('cell', { name: DEV_EXPIRED_GAMMA })).toHaveCount(0);
  });

  test('page=1 is the default and does not appear in the URL', async ({ page }) => {
    await page.goto('/keys');
    await expect(page).toHaveURL(/\/keys\/?$/);
    const url = new URL(page.url());
    expect(url.searchParams.get('page')).toBeNull();
  });

  test('changing the search filter resets page to 1 (drops page= from URL)', async ({ page }) => {
    await page.goto('/keys?page=3');
    await expect(page).toHaveURL(/page=3/);

    await page.getByPlaceholder(/Search by device ID/i).fill('alpha');
    // Debounce is ~300ms; wait for URL to reflect the new search param.
    await expect(page).toHaveURL(/search=alpha/);
    await expect(page).not.toHaveURL(/page=3/);

    const url = new URL(page.url());
    expect(url.searchParams.get('page')).toBeNull();
  });

  test('browser back walks filter history', async ({ page }) => {
    await page.goto('/keys');
    await expect(page).toHaveURL(/\/keys\/?$/);

    await page.getByPlaceholder(/Search by device ID/i).fill('alpha');
    await expect(page).toHaveURL(/search=alpha/);

    await page.goBack();
    await expect(page).not.toHaveURL(/search=/);
    await expect(page).toHaveURL(/\/keys\/?$/);
  });
});
