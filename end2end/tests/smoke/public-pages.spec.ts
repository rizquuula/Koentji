import { test, expect } from '@playwright/test';

const PUBLIC_PAGES: Array<{ path: string; heading: RegExp }> = [
  { path: '/', heading: /Koentji/ },
  { path: '/about', heading: /About/i },
  { path: '/terms', heading: /Terms/i },
  { path: '/privacy', heading: /Privacy/i },
  { path: '/quickstart', heading: /Quickstart/i },
  { path: '/login', heading: /Koentji/ },
];

test.describe('@smoke public pages', () => {
  for (const { path, heading } of PUBLIC_PAGES) {
    test(`GET ${path} renders without errors`, async ({ page }) => {
      const consoleErrors: string[] = [];
      const pageErrors: string[] = [];
      page.on('console', (msg) => {
        if (msg.type() === 'error') consoleErrors.push(msg.text());
      });
      page.on('pageerror', (err) => {
        pageErrors.push(err.message);
      });

      const response = await page.goto(path);
      expect(response, `no response for ${path}`).not.toBeNull();
      expect(response!.status(), `${path} status`).toBeLessThan(400);

      await expect(page.locator('h1').first()).toBeVisible();
      await expect(page.locator('h1').first()).toContainText(heading);

      // Give hydration a moment to complete before asserting clean console.
      await page.waitForLoadState('networkidle');

      expect(pageErrors, `pageerror on ${path}`).toEqual([]);
      expect(consoleErrors, `console errors on ${path}`).toEqual([]);
    });
  }
});

test('@smoke landing page has working CTA links', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: /Sign In/i })).toHaveAttribute('href', '/login');
  await expect(page.getByRole('link', { name: /View Quickstart/i })).toHaveAttribute(
    'href',
    '/quickstart',
  );
});

test('@smoke unknown route returns status 404', async ({ request }) => {
  // Actix returns a bare 404 for unregistered routes (the Leptos NotFound
  // fallback only fires for paths handled by leptos_actix). The useful
  // contract is the status code, not the body.
  const response = await request.get('/this-route-does-not-exist');
  expect(response.status()).toBe(404);
});
