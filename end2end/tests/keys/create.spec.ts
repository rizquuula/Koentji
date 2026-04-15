import { test, expect } from '@playwright/test';
import { withDb, deleteKeyByDevice, countKeysByDevice } from '../../fixtures/db';

const DEVICE_ID = 'e2e-device-create-flow';

test.describe('create key modal', () => {
  test.afterEach(async () => {
    await withDb((c) => deleteKeyByDevice(c, DEVICE_ID));
  });

  test('create new key via modal persists and appears in table', async ({ page }) => {
    await page.goto('/keys');
    await page.getByRole('button', { name: /Create Key/i }).click();

    const modal = page.getByRole('heading', { name: 'Create New API Key' }).locator('..').locator('..');
    await expect(modal).toBeVisible();

    await modal.locator('input[type="text"]').first().fill(DEVICE_ID);
    await modal.locator('input[type="text"]').nth(1).fill('new-user');
    await modal.locator('input[type="email"]').fill('new-user@e2e.test');
    await modal.locator('select').selectOption({ label: 'Free' });

    await modal.getByRole('button', { name: /^Create Key$/ }).click();

    // Modal closes, row appears.
    await expect(page.getByRole('heading', { name: 'Create New API Key' })).toHaveCount(0);
    await expect(page.getByRole('cell', { name: DEVICE_ID })).toBeVisible();

    // DB assertion.
    const count = await withDb((c) => countKeysByDevice(c, DEVICE_ID));
    expect(count).toBe(1);
  });
});
