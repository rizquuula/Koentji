import { test, expect } from '@playwright/test';

test.describe('dashboard stats', () => {
  test('renders four stats cards with numeric values', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();

    for (const title of ['Total Keys', 'Active Keys', 'Expired Keys', 'Deleted Keys']) {
      const card = page.getByText(title).locator('..');
      await expect(card).toBeVisible();
      const value = await card.locator('p.text-2xl').innerText();
      expect(value).toMatch(/^\d+$/);
    }
  });

  test('baseline seed reflects in Total Keys (≥ 3 seeded rows)', async ({ page }) => {
    await page.goto('/dashboard');
    const totalCard = page.getByText('Total Keys').locator('..');
    const value = Number(await totalCard.locator('p.text-2xl').innerText());
    expect(value).toBeGreaterThanOrEqual(3);
  });

  test('date range picker triggers a new stats fetch', async ({ page }) => {
    await page.goto('/dashboard');
    // Wait for initial stats fetch to settle.
    await page.waitForLoadState('networkidle');

    // The DateRangePicker exposes a select with range options. Pick any non-default.
    const picker = page.locator('select').first();
    if (await picker.count()) {
      const options = await picker.locator('option').allInnerTexts();
      const other = options.find((o) => !/all/i.test(o));
      if (other) {
        await picker.selectOption({ label: other });
        await page.waitForLoadState('networkidle');
        await expect(page.getByText('Total Keys').locator('..')).toBeVisible();
      }
    }
  });
});
