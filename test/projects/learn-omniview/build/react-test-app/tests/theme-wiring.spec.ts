import { test, expect } from '@playwright/test';

// ════════════════════════════════════════════════════════════════
// Task 4.2 (review HIGH-2): clicking a theme card must change
// <html data-theme="..."> and the [data-theme] CSS overrides must apply.
// The @theme-marked ThemeStore signal drives the subscription — no more
// orphan useThemeStore.
// ════════════════════════════════════════════════════════════════

test('theme switching writes data-theme and applies CSS overrides', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));

  await page.goto('http://localhost:5173', { timeout: 8000 });
  await page.waitForSelector('button', { timeout: 8000 });

  // Initial theme from the @theme signal default ("light") — the
  // subscription useEffect applies it on mount.
  await page.waitForFunction(
    () => document.documentElement.dataset.theme !== undefined,
    undefined,
    { timeout: 8000 },
  );
  const initial = await page.evaluate(() => document.documentElement.dataset.theme);
  expect(initial).toBe('light');

  // Open the Design tab (💅 Design) and click the Dark theme card.
  await page.locator('button:has-text("💅 Design")').first().click();
  await page.waitForTimeout(600);
  await page.locator('div:has-text("Theme Selector")').first().waitFor({ timeout: 8000 });
  const darkCard = page
    .getByText('Dark', { exact: true })
    .locator('..');
  await darkCard.click();
  await page.waitForTimeout(300);

  // 1. data-theme attribute changed on <html>.
  const theme = await page.evaluate(() => document.documentElement.dataset.theme);
  expect(theme).toBe('dark');

  // 2. The [data-theme="dark"] CSS override actually applies (--surface
  //    resolves to the dark override #111827, not the :root #ffffff).
  const surface = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue('--surface').trim(),
  );
  expect(surface).toBe('#111827');

  // 3. The store recorded the switch (ordinary store action semantics).
  //    Note: React fragments render adjacent text nodes without spaces.
  await expect(page.getByText(/Toggled\s*1\s*times/).first()).toBeVisible({ timeout: 5000 });

  // 4. Clicking Sepia switches again (multi-theme).
  await page.getByText('Sepia', { exact: true }).locator('..').click();
  await page.waitForTimeout(300);
  const sepia = await page.evaluate(() => document.documentElement.dataset.theme);
  expect(sepia).toBe('sepia');

  expect(errors.filter((e) => !e.includes('favicon'))).toEqual([]);
});
