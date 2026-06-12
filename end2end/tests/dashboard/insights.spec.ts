import { test, expect, Page, Locator } from '@playwright/test';

// The four dashboard insight panels — Expiring Soon, Recent Admin Activity,
// Tier Health, Key Hygiene — are backed by one server fn,
// `get_dashboard_insights`. Like the analytics snapshot, the initial fetch is
// resolved server-side during SSR (the Suspense Resource has no reactive
// deps), so there's no client POST on first load — wait for the page heading.
//
// Every assertion tolerates the documented empty state: a fresh database may
// have no expiring keys, no audit history, or no live keys per tier, so each
// panel must be allowed to render either its table/feed or its empty-state
// copy. The seeded tier catalogue (Free/Basic/Pro/Enterprise) means Tier
// Health is rarely empty, but the test does not depend on that.

async function gotoDashboard(page: Page): Promise<void> {
  await page.goto('/dashboard');
  await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
}

// The Surface card that wraps a panel — located from its heading. Surface
// renders `shadow-raised`, which `contains(@class,"shadow")` matches, mirroring
// the analytics spec's scoping idiom.
function panelFor(page: Page, heading: string): Locator {
  return page
    .getByRole('heading', { name: heading })
    .locator('xpath=ancestor::div[contains(@class,"shadow")][1]');
}

// A panel shows EITHER the named column headers OR its documented empty state.
async function expectTableOrEmpty(
  panel: Locator,
  columns: string[],
  emptyText: string | RegExp,
): Promise<void> {
  const firstCol = panel.getByRole('columnheader', { name: columns[0], exact: true });
  if (await firstCol.count()) {
    for (const col of columns) {
      await expect(panel.getByRole('columnheader', { name: col, exact: true })).toBeVisible();
    }
    return;
  }
  await expect(panel.getByText(emptyText)).toBeVisible();
}

test.describe('dashboard insights', () => {
  test('renders all four insight panel headings with no load error', async ({ page }) => {
    await gotoDashboard(page);

    for (const heading of ['Expiring Soon', 'Recent Admin Activity', 'Tier Health', 'Key Hygiene']) {
      await expect(page.getByRole('heading', { name: heading })).toBeVisible();
    }

    await expect(page.getByText(/Failed to load/i)).toHaveCount(0);
  });

  test('Expiring Soon shows the expiry table or its empty state', async ({ page }) => {
    await gotoDashboard(page);

    const panel = panelFor(page, 'Expiring Soon');
    await expectTableOrEmpty(
      panel,
      ['Key', 'Owner', 'Expires', 'Days left'],
      'No keys expiring in the next 90 days',
    );
  });

  test('Tier Health shows one row per tier or its empty state', async ({ page }) => {
    await gotoDashboard(page);

    const panel = panelFor(page, 'Tier Health');
    await expectTableOrEmpty(
      panel,
      ['Tier', 'Live Keys', 'Quota', 'Interval', 'Status'],
      'No subscription tiers configured',
    );
  });

  test('Recent Admin Activity shows the feed or its empty state', async ({ page }) => {
    await gotoDashboard(page);

    const panel = panelFor(page, 'Recent Admin Activity');
    // The feed is a semantic list; an empty history renders the documented copy.
    const items = panel.getByRole('listitem');
    if (await items.count()) {
      await expect(items.first()).toBeVisible();
    } else {
      await expect(panel.getByText('No admin activity yet')).toBeVisible();
    }
  });

  test('Key Hygiene shows both Unclaimed and Dormant sub-sections', async ({ page }) => {
    await gotoDashboard(page);

    await expect(page.getByRole('heading', { name: 'Key Hygiene' })).toBeVisible();

    // Both sub-sections are real <h3> headings inside the panel.
    await expect(page.getByRole('heading', { name: 'Unclaimed' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Dormant' })).toBeVisible();

    const panel = panelFor(page, 'Key Hygiene');
    // Each sub-section renders its DataTable or its own empty state; the panel
    // as a whole must show no error and at least the two sub-headings above.
    await expect(panel.getByText(/Failed to load/i)).toHaveCount(0);
  });
});
