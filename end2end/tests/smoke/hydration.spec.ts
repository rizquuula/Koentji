import { test, expect } from '@playwright/test';
import { STORAGE_STATE_PATH } from '../../fixtures/env';

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
  page.on('pageerror', (err) => {
    // WebKit reports in-flight fetches that were aborted by navigation as
    // "TypeError: Load failed". This is a browser quirk, not a real error
    // from our code — the actual hydration completed cleanly.
    if (/TypeError:\s*Load failed/.test(err.message)) return;
    pageErrors.push(err.message);
  });

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
  // Reuse the admin session cookies saved in global-setup instead of doing a
  // fresh form login per test — the form-login dance flakes on WebKit.
  test.use({ storageState: STORAGE_STATE_PATH });

  for (const route of PROTECTED_ROUTES) {
    test(`no hydration errors on ${route}`, async ({ page }) => {
      await assertCleanHydration(page, route);
    });
  }
});
