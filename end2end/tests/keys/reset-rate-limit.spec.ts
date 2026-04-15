import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey, setRateLimitRemaining } from '../../fixtures/db';

const KEY = 'klab_e2e_reset_0001';
const DEVICE = 'e2e-device-reset';

test.describe('reset rate limit', () => {
  test.beforeEach(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, KEY);
      const seeded = await insertKey(c, {
        key: KEY,
        device_id: DEVICE,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 10,
      });
      await setRateLimitRemaining(c, seeded.id, 10);
    });
  });

  test.afterEach(async () => {
    await withDb((c) => deleteKeyByKey(c, KEY));
  });

  test('confirm reset restores remaining to daily limit', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await expect(row).toContainText('10/100');

    await row.getByRole('button', { name: /Reset Rate Limit/i }).click();
    await expect(page.getByRole('heading', { name: 'Reset Rate Limit' })).toBeVisible();
    await page.getByRole('button', { name: /^Reset$/ }).click();

    await expect(page.getByRole('heading', { name: 'Reset Rate Limit' })).toHaveCount(0);
    await expect(row).toContainText('100/100');
  });
});
