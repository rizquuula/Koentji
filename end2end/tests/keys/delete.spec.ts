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

  test('confirm modal → row status becomes revoked in place', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await expect(row).toContainText(/active/);
    await row.getByRole('button', { name: 'Revoke key' }).click();

    const dialog = page.getByRole('alertdialog', { name: 'Revoke API Key' });
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: /^Revoke$/ }).click();

    await expect(page.getByRole('heading', { name: 'Revoke API Key' })).toHaveCount(0);

    // The row updates in place — no reload, no filter switch. This pins
    // the content-version remount fix: the badge flips to "revoked".
    await expect(page.getByRole('row').filter({ hasText: DEVICE })).toContainText(/revoked/);
  });

  test('revoke then unrevoke returns the row to active in place', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });

    // Revoke.
    await row.getByRole('button', { name: 'Revoke key' }).click();
    const revokeDialog = page.getByRole('alertdialog', { name: 'Revoke API Key' });
    await expect(revokeDialog).toBeVisible();
    await revokeDialog.getByRole('button', { name: /^Revoke$/ }).click();
    await expect(page.getByRole('row').filter({ hasText: DEVICE })).toContainText(/revoked/);

    // Unrevoke — the toggle button is now the green "Unrevoke key".
    await page
      .getByRole('row')
      .filter({ hasText: DEVICE })
      .getByRole('button', { name: 'Unrevoke key' })
      .click();
    const unrevokeDialog = page.getByRole('alertdialog', { name: 'Unrevoke API Key' });
    await expect(unrevokeDialog).toBeVisible();
    await unrevokeDialog.getByRole('button', { name: /^Unrevoke$/ }).click();
    await expect(page.getByRole('heading', { name: 'Unrevoke API Key' })).toHaveCount(0);

    // Back to active, in place.
    await expect(page.getByRole('row').filter({ hasText: DEVICE })).toContainText(/active/);
  });

  test('cancel on confirm modal leaves key active', async ({ page }) => {
    await page.goto('/keys');
    const row = page.getByRole('row').filter({ hasText: DEVICE });
    await row.getByRole('button', { name: 'Revoke key' }).click();

    await expect(page.getByRole('heading', { name: 'Revoke API Key' })).toBeVisible();
    await page.getByRole('button', { name: /^Cancel$/ }).click();
    await expect(page.getByRole('heading', { name: 'Revoke API Key' })).toHaveCount(0);
    await expect(page.getByRole('row').filter({ hasText: DEVICE })).toContainText(/active/);
  });
});
