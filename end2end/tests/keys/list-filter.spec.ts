import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey } from '../../fixtures/db';

const SEED_PREFIX = 'klab_e2e_list_';

test.describe('keys list + filter', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await insertKey(c, {
        key: `${SEED_PREFIX}alpha`,
        device_id: 'list-device-alpha',
        subscription_type_name: 'free',
        username: 'alpha-user',
        email: 'alpha@e2e.test',
      });
      await insertKey(c, {
        key: `${SEED_PREFIX}beta`,
        device_id: 'list-device-beta',
        subscription_type_name: 'pro',
        username: 'beta-user',
        email: 'beta@e2e.test',
      });
      await insertKey(c, {
        key: `${SEED_PREFIX}expired`,
        device_id: 'list-device-expired',
        subscription_type_name: 'basic',
        expired_at: new Date(Date.now() - 3600 * 1000).toISOString(),
      });
    });
  });

  test.afterAll(async () => {
    await withDb(async (c) => {
      for (const suffix of ['alpha', 'beta', 'expired']) {
        await deleteKeyByKey(c, `${SEED_PREFIX}${suffix}`);
      }
    });
  });

  test('table renders seeded rows', async ({ page }) => {
    await page.goto('/keys');
    await expect(page.getByRole('cell', { name: 'list-device-alpha' })).toBeVisible();
    await expect(page.getByRole('cell', { name: 'list-device-beta' })).toBeVisible();
  });

  test('search filter narrows to a single device (debounced 300ms)', async ({ page }) => {
    await page.goto('/keys');
    await page.getByPlaceholder(/Search by device ID/i).fill('list-device-alpha');
    await expect(page.getByRole('cell', { name: 'list-device-alpha' })).toBeVisible();
    await expect(page.getByRole('cell', { name: 'list-device-beta' })).toHaveCount(0);
  });

  test('subscription dropdown filters by pro', async ({ page }) => {
    await page.goto('/keys');
    await page.locator('select').first().selectOption('pro');
    await expect(page.getByRole('cell', { name: 'list-device-beta' })).toBeVisible();
    await expect(page.getByRole('cell', { name: 'list-device-alpha' })).toHaveCount(0);
  });

  test('status filter shows only expired keys', async ({ page }) => {
    await page.goto('/keys');
    await page.locator('select').nth(1).selectOption('expired');
    await expect(page.getByRole('cell', { name: 'list-device-expired' })).toBeVisible();
    await expect(page.getByRole('cell', { name: 'list-device-alpha' })).toHaveCount(0);
  });
});
