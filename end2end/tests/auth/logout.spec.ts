import { test, expect } from '@playwright/test';
import { ADMIN_PASSWORD, ADMIN_USERNAME } from '../../fixtures/env';

test.describe('logout', () => {
  test('logout purges session and redirects to /login', async ({ page }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your username').fill(ADMIN_USERNAME);
    await page.getByPlaceholder('Enter your password').fill(ADMIN_PASSWORD);
    await page.getByRole('button', { name: /Sign In/i }).click();
    await page.waitForURL(/\/dashboard/);

    await page.getByRole('button', { name: /Logout/i }).click();
    await page.waitForURL(/\/login/);
    await expect(page).toHaveURL(/\/login$/);

    // Revisiting a protected route should send us back to /login.
    await page.goto('/dashboard');
    await page.waitForURL(/\/login/);
  });

  test('session cookie cleared after logout', async ({ page, context }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your username').fill(ADMIN_USERNAME);
    await page.getByPlaceholder('Enter your password').fill(ADMIN_PASSWORD);
    await page.getByRole('button', { name: /Sign In/i }).click();
    await page.waitForURL(/\/dashboard/);

    const cookiesBefore = await context.cookies();
    expect(cookiesBefore.find((c) => c.name === 'koentjilab_session')).toBeTruthy();

    await page.getByRole('button', { name: /Logout/i }).click();
    await page.waitForURL(/\/login/);

    const cookiesAfter = await context.cookies();
    const session = cookiesAfter.find((c) => c.name === 'koentjilab_session');
    expect(!session || session.value === '' || session.value === 'null').toBeTruthy();
  });
});
