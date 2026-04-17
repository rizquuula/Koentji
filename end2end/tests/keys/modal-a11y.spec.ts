import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey, countKeysByKey } from '../../fixtures/db';

// G10 — Modal a11y: ESC close, Tab/Shift+Tab focus trap, focus return to opener.
// Both Modal (role="dialog") and ConfirmModal (role="alertdialog") share the
// same escape/tab/focus-snapshot contract in src/ui/design/modal.rs.

const PREFIX = 'e2e_feD_';
const CONFIRM_KEY = `${PREFIX}a11y_confirm`;
const CONFIRM_DEVICE = `${PREFIX}a11y_confirm_dev`;

test.describe('modal accessibility', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, CONFIRM_KEY);
      await insertKey(c, {
        key: CONFIRM_KEY,
        device_id: CONFIRM_DEVICE,
        subscription_type_name: 'free',
      });
    });
  });

  test.afterAll(async () => {
    await withDb(async (c) => {
      await c.query(`DELETE FROM authentication_keys WHERE key LIKE '${PREFIX}%'`);
    });
  });

  test('opening Add Key modal puts focus inside the dialog', async ({ page }) => {
    await page.goto('/keys');
    await page.getByRole('button', { name: /Create Key/i }).click();

    const dialog = page.getByRole('dialog', { name: 'Create New API Key' });
    await expect(dialog).toBeVisible();

    // Focus must land somewhere inside the dialog (first focusable descendant).
    const activeInsideDialog = await page.evaluate(() => {
      const dlg = document.querySelector('[role="dialog"]');
      const active = document.activeElement;
      return !!(dlg && active && dlg.contains(active));
    });
    expect(activeInsideDialog).toBe(true);

    const activeTag = await page.evaluate(() =>
      document.activeElement?.tagName.toLowerCase() ?? '',
    );
    expect(['input', 'button', 'select', 'textarea', 'a']).toContain(activeTag);
  });

  test('Escape closes the Add Key modal', async ({ page }) => {
    await page.goto('/keys');
    await page.getByRole('button', { name: /Create Key/i }).click();
    await expect(page.getByRole('dialog', { name: 'Create New API Key' })).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog', { name: 'Create New API Key' })).toHaveCount(0);
  });

  test('focus returns to the opener button after closing via Escape', async ({ page }) => {
    await page.goto('/keys');
    const opener = page.getByRole('button', { name: /Create Key/i });
    await opener.click();

    await expect(page.getByRole('dialog', { name: 'Create New API Key' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog', { name: 'Create New API Key' })).toHaveCount(0);

    // Focus snapshot/restore in modal.rs returns focus to whatever had it at open time.
    const activeText = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      return (el?.innerText ?? '').trim();
    });
    expect(activeText).toMatch(/Create Key/i);
  });

  test('Tab cycles focus inside the dialog (never escapes)', async ({ page }) => {
    await page.goto('/keys');
    await page.getByRole('button', { name: /Create Key/i }).click();
    await expect(page.getByRole('dialog', { name: 'Create New API Key' })).toBeVisible();

    // Press Tab more times than any reasonable focusable count — focus must
    // remain inside the dialog for every hop thanks to trap_tab.
    for (let i = 0; i < 20; i += 1) {
      await page.keyboard.press('Tab');
      const inside = await page.evaluate(() => {
        const dlg = document.querySelector('[role="dialog"]');
        const active = document.activeElement;
        return !!(dlg && active && dlg.contains(active));
      });
      expect(inside, `focus escaped dialog at Tab #${i + 1}`).toBe(true);
    }
  });

  test('Shift+Tab cycles focus inside the dialog (never escapes)', async ({ page }) => {
    await page.goto('/keys');
    await page.getByRole('button', { name: /Create Key/i }).click();
    await expect(page.getByRole('dialog', { name: 'Create New API Key' })).toBeVisible();

    for (let i = 0; i < 20; i += 1) {
      await page.keyboard.press('Shift+Tab');
      const inside = await page.evaluate(() => {
        const dlg = document.querySelector('[role="dialog"]');
        const active = document.activeElement;
        return !!(dlg && active && dlg.contains(active));
      });
      expect(inside, `focus escaped dialog at Shift+Tab #${i + 1}`).toBe(true);
    }
  });

  test('Escape on Revoke confirm alertdialog cancels without deleting', async ({ page }) => {
    await page.goto('/keys');

    const row = page.getByRole('row').filter({ hasText: CONFIRM_DEVICE });
    await row.getByRole('button', { name: /Revoke/i }).click();

    const confirmDialog = page.getByRole('alertdialog', { name: 'Revoke API Key' });
    await expect(confirmDialog).toBeVisible();

    // ConfirmModal wires "Escape" → on_cancel in modal.rs (same as Modal).
    await page.keyboard.press('Escape');
    await expect(page.getByRole('alertdialog', { name: 'Revoke API Key' })).toHaveCount(0);

    // Row must still exist — Escape cancels, never confirms.
    const count = await withDb((c) => countKeysByKey(c, CONFIRM_KEY));
    expect(count).toBe(1);
  });
});
