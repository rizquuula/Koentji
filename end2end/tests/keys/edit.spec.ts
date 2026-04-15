import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey } from '../../fixtures/db';

const KEY = 'klab_e2e_edit_0001';
const DEVICE = 'e2e-device-edit';

test.describe('edit key', () => {
  test.beforeEach(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, KEY);
      await insertKey(c, {
        key: KEY,
        device_id: DEVICE,
        subscription_type_name: 'free',
        username: 'orig-user',
      });
    });
  });

  test.afterEach(async () => {
    await withDb((c) => deleteKeyByKey(c, KEY));
  });

  test('edit username via modal updates the row', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await row.getByRole('button', { name: /Edit/i }).click();

    const modal = page.getByRole('heading', { name: 'Edit API Key' }).locator('..').locator('..');
    await expect(modal).toBeVisible();

    const usernameInput = modal.locator('input[type="text"]').nth(1);
    await usernameInput.fill('updated-user');
    await modal.getByRole('button', { name: /Update Key/i }).click();

    await expect(page.getByRole('heading', { name: 'Edit API Key' })).toHaveCount(0);
    await expect(page.getByRole('row').filter({ hasText: DEVICE })).toContainText('updated-user');
  });
});
