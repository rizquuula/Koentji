import { test, expect } from '@playwright/test';
import { ADMIN_PASSWORD, ADMIN_USERNAME } from '../../fixtures/env';

const PUBLIC_ROUTES = ['/', '/about', '/terms', '/privacy', '/quickstart', '/login'];
const PROTECTED_ROUTES = ['/dashboard', '/keys', '/subscriptions', '/limits-interval'];

async function assertCleanHydration(page: import('@playwright/test').Page, route: string) {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];
  const hydrationWarnings: string[] = [];

  page.on('console', (msg) => {
    const text = msg.text();
    if (msg.type() === 'error') consoleErrors.push(text);
    if (/hydrat/i.test(text)) hydrationWarnings.push(text);
  });
  page.on('pageerror', (err) => pageErrors.push(err.message));

  await page.goto(route);
  await page.waitForLoadState('networkidle');

  expect(pageErrors, `pageerrors on ${route}`).toEqual([]);
  expect(consoleErrors, `console errors on ${route}`).toEqual([]);
  expect(hydrationWarnings, `hydration warnings on ${route}`).toEqual([]);
}

test.describe('@smoke hydration — public routes', () => {
  for (const route of PUBLIC_ROUTES) {
    test(`no hydration errors on ${route}`, async ({ page }) => {
      await assertCleanHydration(page, route);
    });
  }
});

test.describe('@smoke hydration — protected routes', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.getByPlaceholder('Enter your username').fill(ADMIN_USERNAME);
    await page.getByPlaceholder('Enter your password').fill(ADMIN_PASSWORD);
    await Promise.all([
      page.waitForURL(/\/dashboard/),
      page.getByRole('button', { name: /Sign In/i }).click(),
    ]);
  });

  for (const route of PROTECTED_ROUTES) {
    test(`no hydration errors on ${route}`, async ({ page }) => {
      await assertCleanHydration(page, route);
    });
  }
});
