import { test, expect } from '@playwright/test';
import { withDb } from '../../fixtures/db';

// Unclaimed keys all share the `-` device id, so the username is the only
// value that identifies this run's row for assertion and cleanup.
const USERNAME = `e2e-unclaimed-${Date.now()}`;
const UNCLAIMED_SENTINEL = '-';

test.describe('create key modal — unclaimed device', () => {
  test.afterEach(async () => {
    await withDb((c) =>
      c.query('DELETE FROM authentication_keys WHERE username = $1', [USERNAME]),
    );
  });

  test('unclaimed toggle fills the sentinel device id and creates the key', async ({ page }) => {
    await page.goto('/keys');
    await page.getByRole('button', { name: /Create Key/i }).click();

    const modal = page.getByRole('heading', { name: 'Create New API Key' }).locator('..').locator('..');
    await expect(modal).toBeVisible();

    const deviceInput = modal.locator('input[type="text"]').first();
    await modal.getByRole('switch', { name: /Leave unclaimed/i }).click();

    await expect(deviceInput).toHaveValue(UNCLAIMED_SENTINEL);
    // `Input` drives read-only through the DOM property, not the attribute.
    await expect(deviceInput).toHaveJSProperty('readOnly', true);

    await modal.locator('input[type="text"]').nth(1).fill(USERNAME);
    await modal.locator('select').selectOption({ label: 'Free' });

    await modal.getByRole('button', { name: /^Create Key$/ }).click();

    // Modal closes, row appears — matched on the unique username, because the
    // device cell is both shared with other unclaimed keys and hidden behind a
    // reveal toggle.
    await expect(page.getByRole('heading', { name: 'Create New API Key' })).toHaveCount(0);
    await expect(page.getByRole('cell', { name: USERNAME })).toBeVisible();
  });
});
