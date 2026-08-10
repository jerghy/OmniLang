const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errs = [];
  page.on('pageerror', (e) => errs.push('PAGEERROR: ' + e.message));
  page.on('console', (m) => { if (m.type() === 'error') errs.push('CONSOLE: ' + m.text()); });

  await page.goto('http://localhost:5173/', { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('button', { timeout: 8000 });
  await page.locator('button:has-text("🎨 UI")').first().click();
  await page.waitForTimeout(600);
  const openBtn = page.getByRole('button', { name: /Open Modal/ }).first();
  const state = async (label) => {
    console.log(label, '| open:', await page.locator('dialog[open]').count(), '| :modal:', await page.locator('dialog:modal').count());
  };
  await openBtn.click();
  await page.waitForTimeout(300);
  await state('after open');
  // focus inside dialog (the X button), then Escape
  await page.locator('dialog button').first().focus();
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);
  await state('after Escape');
  // focus return check: click Open Modal again? Instead check body active element after Escape
  const active = await page.evaluate(() => document.activeElement ? document.activeElement.tagName + ':' + (document.activeElement.textContent || '').trim().slice(0, 20) : 'none');
  console.log('activeElement after Escape:', active);
  await openBtn.click();
  await page.waitForTimeout(300);
  await state('after reopen');
  // backdrop click at left edge
  await page.mouse.click(30, 360);
  await page.waitForTimeout(300);
  await state('after backdrop click');
  console.log('ERRORS:', JSON.stringify(errs));
  await browser.close();
})().catch((e) => { console.error('PROBE-FAIL', e.message); process.exit(1); });
