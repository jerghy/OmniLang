import { test, expect } from '@playwright/test';
test('debug tab 7', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await page.waitForSelector('button');
  console.log('Button texts:', (await page.locator('button').allTextContents()).map(t => t.trim()));
  await page.locator('button').nth(7).click();
  await page.waitForTimeout(1000);
  const text = await page.evaluate(() => document.body.innerText);
  console.log('Tab 7 body content:', text.substring(0, 1000));
  // Check if InlineStoreDemo text exists
  const has = text.includes('Inline Store Demo');
  console.log('Has Inline Store Demo:', has);
});
