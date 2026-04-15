import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey } from '../../fixtures/db';

const KEY = 'klab_e2e_reveal_00000000';
const DEVICE = 'e2e-device-reveal';

test.describe('reveal key', () => {
  test.beforeEach(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, KEY);
      await insertKey(c, { key: KEY, device_id: DEVICE, subscription_type_name: 'free' });
    });
  });

  test.afterEach(async () => {
    await withDb((c) => deleteKeyByKey(c, KEY));
  });

  test('reveal swaps masked key for full value', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await expect(row).toContainText(/klab_\*+/);
    await row.getByRole('button', { name: /Reveal/i }).click();
    await expect(row).toContainText(KEY);
  });
});
