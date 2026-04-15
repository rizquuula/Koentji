import { test, expect } from '@playwright/test';
import { withDb, insertSubscriptionType, deleteSubscriptionTypeByName } from '../../fixtures/db';

const PREFIX = 'e2e_sub_';
const CREATE_NAME = `${PREFIX}create_${Date.now()}`;
const TOGGLE_NAME = `${PREFIX}toggle_${Date.now()}`;
const DELETE_NAME = `${PREFIX}delete_${Date.now()}`;

test.describe('subscription types CRUD', () => {
  test.afterAll(async () => {
    await withDb(async (c) => {
      await c.query("DELETE FROM subscription_types WHERE name LIKE 'e2e_sub_%'");
    });
  });

  test('create subscription type via form', async ({ page }) => {
    await page.goto('/subscriptions');
    await page.getByRole('button', { name: /Add Subscription/i }).click();

    // Use exact match on the modal heading to avoid colliding with the page h1 "Subscription Types".
    await expect(page.getByRole('heading', { name: 'Subscription Type', exact: true })).toBeVisible();

    await page.locator('input[placeholder="e.g. basic"]').fill(CREATE_NAME);
    await page.locator('input[placeholder="e.g. Basic"]').fill('E2E Create');
    await page.locator('form input[type="number"]').fill('1234');
    await page.locator('form select').first().selectOption({ label: 'Daily' });
    await page.locator('form button[type="submit"]').click();

    await expect(page.getByRole('cell', { name: CREATE_NAME })).toBeVisible();
  });

  test('deactivate and reactivate subscription type', async ({ page }) => {
    await withDb((c) =>
      insertSubscriptionType(c, {
        name: TOGGLE_NAME,
        display_name: 'E2E Toggle',
        is_active: true,
      }),
    );

    await page.goto('/subscriptions');
    const row = page.getByRole('row').filter({ hasText: TOGGLE_NAME });
    await expect(row).toBeVisible();

    await row.getByRole('button', { name: /^Deactivate$/ }).click();
    await expect(row).toContainText(/Inactive/);

    await row.getByRole('button', { name: /^Activate$/ }).click();
    await expect(row).toContainText(/Active/);
  });

  test('delete subscription type via confirm modal', async ({ page }) => {
    await withDb((c) =>
      insertSubscriptionType(c, {
        name: DELETE_NAME,
        display_name: 'E2E Delete',
      }),
    );

    await page.goto('/subscriptions');
    const row = page.getByRole('row').filter({ hasText: DELETE_NAME });
    await expect(row).toBeVisible();

    await row.getByRole('button', { name: /^Delete$/ }).click();
    await expect(page.getByRole('heading', { name: 'Delete Subscription Type' })).toBeVisible();
    // The modal confirm button is the only bg-red-600 button on the page.
    await page.locator('button.bg-red-600', { hasText: /^Delete$/ }).click();

    await expect(page.getByRole('cell', { name: DELETE_NAME, exact: true })).toHaveCount(0);
  });
});
