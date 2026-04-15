import { test, expect } from '@playwright/test';
import { withDb } from '../../fixtures/db';

const TEST_SUB = {
  name: `e2e_sub_${Date.now()}`,
  display: 'E2E Sub',
  limit: '1234',
};

async function cleanup() {
  await withDb(async (c) => {
    await c.query('DELETE FROM subscription_types WHERE name LIKE $1', ['e2e_sub_%']);
  });
}

test.describe('subscription types CRUD', () => {
  test.afterAll(cleanup);

  test('create subscription type', async ({ page }) => {
    await page.goto('/subscriptions');
    await page.getByRole('button', { name: /Add Subscription/i }).click();

    const modal = page.getByRole('heading', { name: 'Subscription Type' }).locator('..').locator('..');
    await expect(modal).toBeVisible();

    await modal.getByPlaceholder('e.g. basic').fill(TEST_SUB.name);
    await modal.getByPlaceholder('e.g. Basic').fill(TEST_SUB.display);
    await modal.locator('input[type="number"]').fill(TEST_SUB.limit);
    await modal.locator('select').selectOption({ label: 'Daily' });
    await modal.getByRole('button', { name: /^Save$|^Create$|^Submit$/i }).first().click().catch(async () => {
      // Fallback: click the submit button inside the form.
      await modal.locator('button[type="submit"]').click();
    });

    await expect(page.getByRole('cell', { name: TEST_SUB.name })).toBeVisible();
    await expect(page.getByText(/Subscription type saved successfully/i)).toBeVisible();
  });

  test('deactivate and reactivate subscription type', async ({ page }) => {
    await page.goto('/subscriptions');
    const row = page.getByRole('row').filter({ hasText: TEST_SUB.name });
    await row.getByRole('button', { name: /Deactivate/i }).click();
    await expect(row).toContainText(/Inactive/);
    await row.getByRole('button', { name: /Activate/i }).click();
    await expect(row).toContainText(/Active/);
  });

  test('delete subscription type via confirm modal', async ({ page }) => {
    await page.goto('/subscriptions');
    const row = page.getByRole('row').filter({ hasText: TEST_SUB.name });
    await row.getByRole('button', { name: /^Delete$/ }).click();
    await expect(page.getByRole('heading', { name: 'Delete Subscription Type' })).toBeVisible();
    await page.getByRole('button', { name: /^Delete$/ }).last().click();
    await expect(page.getByRole('cell', { name: TEST_SUB.name })).toHaveCount(0);
  });
});
