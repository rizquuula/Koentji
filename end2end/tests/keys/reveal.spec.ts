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
    await row.getByRole('button', { name: 'Reveal full key' }).click();
    await expect(row).toContainText(KEY);
  });

  test('copy works without revealing first, key stays masked', async ({ page, context, browserName }) => {
    // Reading the clipboard needs permissions only chromium grants here.
    test.skip(browserName !== 'chromium', 'clipboard read needs chromium permissions');
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);

    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await expect(row).toContainText(/klab_\*+/);

    // Copy without pressing reveal first — the fix makes this a synchronous
    // clipboard write that keeps the click's user-activation alive.
    await row.getByRole('button', { name: 'Copy key to clipboard' }).click();

    await expect
      .poll(() => page.evaluate(() => navigator.clipboard.readText()))
      .toBe(KEY);

    // Copy must not unmask the on-screen key.
    await expect(row).toContainText(/klab_\*+/);
    await expect(row).not.toContainText(KEY);
  });
});
