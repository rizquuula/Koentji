import { test, expect } from '@playwright/test';
import { ADMIN_PASSWORD, ADMIN_USERNAME } from '../../fixtures/env';

test.describe('navbar — guest', () => {
  test('shows Login link and hides admin tabs', async ({ page }) => {
    await page.goto('/about');
    const nav = page.locator('nav').first();

    await expect(nav.getByRole('link', { name: /^Login$/ })).toBeVisible();
    await expect(nav.getByRole('button', { name: /Logout/i })).toHaveCount(0);

    // Admin-only tabs should not appear.
    await expect(nav.getByRole('link', { name: /^Dashboard$/ })).toHaveCount(0);
    await expect(nav.getByRole('link', { name: /^Keys$/ })).toHaveCount(0);
    await expect(nav.getByRole('link', { name: /^Subscriptions$/ })).toHaveCount(0);
    await expect(nav.getByRole('link', { name: /^Limits Interval$/ })).toHaveCount(0);

    // Public tabs should appear.
    await expect(nav.getByRole('link', { name: /^Quickstart$/ })).toBeVisible();
    await expect(nav.getByRole('link', { name: /^About$/ })).toBeVisible();
  });
});

test.describe('navbar — authenticated', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your username').fill(ADMIN_USERNAME);
    await page.getByPlaceholder('Enter your password').fill(ADMIN_PASSWORD);
    await Promise.all([
      page.waitForURL(/\/dashboard/),
      page.getByRole('button', { name: /Sign In/i }).click(),
    ]);
  });

  test('shows admin tabs, username, and Logout button', async ({ page }) => {
    const nav = page.locator('nav').first();
    await expect(nav.getByRole('link', { name: /^Dashboard$/ })).toBeVisible();
    await expect(nav.getByRole('link', { name: /^Keys$/ })).toBeVisible();
    await expect(nav.getByRole('link', { name: /^Subscriptions$/ })).toBeVisible();
    await expect(nav.getByRole('link', { name: /^Limits Interval$/ })).toBeVisible();
    await expect(nav.getByText(ADMIN_USERNAME)).toBeVisible();
    await expect(nav.getByRole('button', { name: /Logout/i })).toBeVisible();
    await expect(nav.getByRole('link', { name: /^Login$/ })).toHaveCount(0);
  });
});
