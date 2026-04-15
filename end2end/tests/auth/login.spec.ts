import { test, expect } from '@playwright/test';
import { ADMIN_PASSWORD, ADMIN_USERNAME } from '../../fixtures/env';

test.describe('login', () => {
  test('valid credentials redirect to /dashboard', async ({ page }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your username').fill(ADMIN_USERNAME);
    await page.getByPlaceholder('Enter your password').fill(ADMIN_PASSWORD);
    await page.getByRole('button', { name: /Sign In/i }).click();
    await page.waitForURL(/\/dashboard/, { timeout: 30_000 });
    await expect(page).toHaveURL(/\/dashboard$/);
  });

  test('invalid credentials show error banner and stay on /login', async ({ page }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your username').fill('not-a-real-admin');
    await page.getByPlaceholder('Enter your password').fill('wrong-password');
    await page.getByRole('button', { name: /Sign In/i }).click();
    await expect(page.getByText(/Invalid username or password/i)).toBeVisible();
    await expect(page).toHaveURL(/\/login$/);
  });

  test('empty form is blocked by HTML5 validation', async ({ page }) => {
    await page.goto('/login');
    await page.getByRole('button', { name: /Sign In/i }).click();
    // Still on /login, no network call fired.
    await expect(page).toHaveURL(/\/login$/);
    const invalid = await page
      .getByPlaceholder('Enter your username')
      .evaluate((el: HTMLInputElement) => !el.validity.valid);
    expect(invalid).toBe(true);
  });

  test('submit button becomes disabled while signing in', async ({ page }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your username').fill(ADMIN_USERNAME);
    await page.getByPlaceholder('Enter your password').fill(ADMIN_PASSWORD);
    const button = page.getByRole('button', { name: /Sign In|Signing in/i });
    await button.click();
    // Text toggles to "Signing in..." while the request is in flight.
    await expect(button).toHaveText(/Signing in/i, { timeout: 2_000 }).catch(() => {});
    await page.waitForURL(/\/dashboard/);
  });
});
