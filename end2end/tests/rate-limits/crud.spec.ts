import { test, expect } from '@playwright/test';
import { withDb } from '../../fixtures/db';

const TEST_INTERVAL = {
  name: `e2e_interval_${Date.now()}`,
  display: 'E2E Interval',
  duration: '42',
};

async function cleanup() {
  await withDb(async (c) => {
    await c.query('DELETE FROM rate_limit_intervals WHERE name LIKE $1', ['e2e_interval_%']);
  });
}

test.describe('rate limit intervals CRUD', () => {
  test.afterAll(cleanup);

  test('create interval', async ({ page }) => {
    await page.goto('/limits-interval');
    await page.getByRole('button', { name: /Add Interval/i }).click();

    const modal = page.locator('form').locator('..').filter({ has: page.getByText(/Name/i) }).first();
    await modal.locator('input').nth(0).fill(TEST_INTERVAL.name);
    await modal.locator('input').nth(1).fill(TEST_INTERVAL.display);
    await modal.locator('input[type="number"]').fill(TEST_INTERVAL.duration);

    await modal.locator('button[type="submit"]').click();

    await expect(page.getByRole('cell', { name: TEST_INTERVAL.name })).toBeVisible();
    await expect(page.getByText(/Rate limit interval saved successfully/i)).toBeVisible();
  });

  test('deactivate and reactivate interval', async ({ page }) => {
    await page.goto('/limits-interval');
    const row = page.getByRole('row').filter({ hasText: TEST_INTERVAL.name });
    await row.getByRole('button', { name: /Deactivate/i }).click();
    await expect(row).toContainText(/Inactive/);
    await row.getByRole('button', { name: /Activate/i }).click();
    await expect(row).toContainText(/Active/);
  });

  test('delete interval via confirm modal', async ({ page }) => {
    await page.goto('/limits-interval');
    const row = page.getByRole('row').filter({ hasText: TEST_INTERVAL.name });
    await row.getByRole('button', { name: /^Delete$/ }).click();
    await expect(page.getByRole('heading', { name: /Delete.*Interval/i })).toBeVisible();
    await page.getByRole('button', { name: /^Delete$/ }).last().click();
    await expect(page.getByRole('cell', { name: TEST_INTERVAL.name })).toHaveCount(0);
  });
});
