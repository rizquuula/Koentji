import { test, expect, Page } from '@playwright/test';

// The /analytics page is backed by one server fn, `get_analytics_snapshot`,
// reachable under `/api/...` (Leptos appends a content hash, so match by
// prefix). The window may legitimately be empty against a fresh ClickHouse,
// so every assertion tolerates empty-state text instead of demanding data.
const SNAPSHOT_URL = /\/api\/get_analytics_snapshot/;

async function gotoAnalytics(page: Page): Promise<void> {
  // The initial snapshot is resolved server-side during SSR (the Suspense
  // Resource), so there's no client POST on first load — wait for the heading
  // instead. Client-side `/api/...` POSTs only fire on range switches and the
  // auto-refresh timer, which the relevant tests assert separately.
  await page.goto('/analytics');
  await expect(page.getByRole('heading', { name: 'Analytics' })).toBeVisible();
}

test.describe('analytics page', () => {
  test('loads with heading, range buttons, and no load error', async ({ page }) => {
    await gotoAnalytics(page);

    await expect(page.getByRole('heading', { name: 'Analytics' })).toBeVisible();
    await expect(page.getByRole('button', { name: '24 Hours' })).toBeVisible();
    await expect(page.getByRole('button', { name: '7 Days' })).toBeVisible();
    await expect(page.getByRole('button', { name: '30 Days' })).toBeVisible();

    await expect(page.getByText(/Failed to load/i)).toHaveCount(0);
  });

  test('renders the five summary cards with values', async ({ page }) => {
    await gotoAnalytics(page);

    for (const title of [
      'Total Requests',
      'Deny Rate',
      'p95 Latency',
      'Unique Keys',
      'Unique Devices',
    ]) {
      const card = page.getByText(title, { exact: true }).locator('..');
      await expect(card).toBeVisible();
      const value = (await card.locator('p.text-2xl').innerText()).trim();
      // Empty window renders "—" for p95; counts render digits or "0".
      expect(value.length).toBeGreaterThan(0);
    }
  });

  test('renders the three chart canvases', async ({ page }) => {
    await gotoAnalytics(page);

    await expect(page.locator('canvas#traffic-chart')).toBeVisible();
    await expect(page.locator('canvas#latency-chart')).toBeVisible();
    // The denials panel swaps the canvas for an empty-state message when there
    // are no denials in the window — accept either.
    const denialsCanvas = page.locator('canvas#denials-chart');
    if (await denialsCanvas.count()) {
      await expect(denialsCanvas).toBeVisible();
    } else {
      await expect(page.getByText('No denials in this window')).toBeVisible();
    }
  });

  test('renders the busiest-keys and quota-pressure tables', async ({ page }) => {
    await gotoAnalytics(page);

    // Busiest keys: heading + its column headers (or documented empty state).
    await expect(page.getByRole('heading', { name: 'Busiest keys' })).toBeVisible();
    const busiest = page
      .getByRole('heading', { name: 'Busiest keys' })
      .locator('xpath=ancestor::div[contains(@class,"shadow")][1]');
    for (const col of ['Key', 'Requests', 'Deny rate', 'Last seen']) {
      await expect(busiest.getByRole('columnheader', { name: col, exact: true })).toBeVisible();
    }

    // Quota pressure: heading + its column headers.
    await expect(page.getByRole('heading', { name: 'Quota pressure' })).toBeVisible();
    const quota = page
      .getByRole('heading', { name: 'Quota pressure' })
      .locator('xpath=ancestor::div[contains(@class,"shadow")][1]');
    for (const col of ['Key', 'Remaining', 'Limit', '% remaining']) {
      await expect(quota.getByRole('columnheader', { name: col, exact: true })).toBeVisible();
    }
  });

  test('switching range to 7 Days and 30 Days refetches without error', async ({ page }) => {
    await gotoAnalytics(page);

    await Promise.all([
      page.waitForResponse((r) => SNAPSHOT_URL.test(r.url())),
      page.getByRole('button', { name: '7 Days' }).click(),
    ]);
    await expect(page.getByText(/Failed to load/i)).toHaveCount(0);

    await Promise.all([
      page.waitForResponse((r) => SNAPSHOT_URL.test(r.url())),
      page.getByRole('button', { name: '30 Days' }).click(),
    ]);
    await expect(page.getByText(/Failed to load/i)).toHaveCount(0);

    await expect(page.getByRole('heading', { name: 'Analytics' })).toBeVisible();
  });

  test('auto-refreshes the snapshot on a timer without interaction', async ({ page }) => {
    // The page re-fetches every 30s; give it ~35s of slack on top of the
    // initial load. No user interaction between the two POSTs.
    test.slow();

    await gotoAnalytics(page);

    // A second snapshot POST must arrive on its own within the refresh window.
    await page.waitForResponse((r) => SNAPSHOT_URL.test(r.url()), { timeout: 40_000 });

    await expect(page.getByText(/Failed to load/i)).toHaveCount(0);
  });
});
