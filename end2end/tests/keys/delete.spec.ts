import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey } from '../../fixtures/db';

const KEY = 'klab_e2e_delete_0001';
const DEVICE = 'e2e-device-delete';

test.describe('delete (revoke) key', () => {
  test.beforeEach(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, KEY);
      await insertKey(c, {
        key: KEY,
        device_id: DEVICE,
        subscription_type_name: 'free',
      });
    });
  });

  test.afterEach(async () => {
    await withDb((c) => deleteKeyByKey(c, KEY));
  });

  test('confirm modal → row status becomes deleted', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await row.getByRole('button', { name: /Revoke/i }).click();

    await expect(page.getByRole('heading', { name: 'Revoke API Key' })).toBeVisible();
    await page.getByRole('button', { name: /^Revoke$/ }).click();

    await expect(page.getByRole('heading', { name: 'Revoke API Key' })).toHaveCount(0);

    // Filter by deleted status to verify.
    await page.locator('select').nth(1).selectOption('deleted');
    await expect(page.getByRole('row').filter({ hasText: DEVICE })).toBeVisible();
  });

  test('cancel on confirm modal leaves key active', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await row.getByRole('button', { name: /Revoke/i }).click();

    await expect(page.getByRole('heading', { name: 'Revoke API Key' })).toBeVisible();
    await page.getByRole('button', { name: /^Cancel$/ }).click();
    await expect(page.getByRole('heading', { name: 'Revoke API Key' })).toHaveCount(0);
    await expect(page.getByRole('row').filter({ hasText: DEVICE })).toContainText(/active/);
  });
});
