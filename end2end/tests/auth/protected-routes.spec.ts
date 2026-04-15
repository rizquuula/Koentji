import { test, expect } from '@playwright/test';

const PROTECTED = ['/dashboard', '/keys', '/subscriptions', '/limits-interval'];

test.describe('protected routes redirect guests to /login', () => {
  for (const route of PROTECTED) {
    test(`guest at ${route} → /login`, async ({ page }) => {
      await page.goto(route);
      await page.waitForURL(/\/login/, { timeout: 10_000 });
      await expect(page).toHaveURL(/\/login$/);
    });
  }
});
