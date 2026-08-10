const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errs = [];
  page.on('pageerror', (e) => errs.push('PAGEERROR: ' + e.message));
  await page.goto('http://localhost:5173/', { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('button', { timeout: 8000 });
  await page.locator('button:has-text("🎨 UI")').first().click();
  await page.waitForTimeout(600);
  await page.getByRole('button', { name: 'Open Modal' }).first().click();
  await page.waitForTimeout(400);
  for (let i = 0; i < 8; i++) {
    const info = await page.evaluate(() => {
      const d = document.querySelector('dialog[open]');
      const a = document.activeElement;
      return JSON.stringify({
        inside: d ? d.contains(a) : null,
        tag: a ? a.tagName : 'null',
        cls: a && a.className ? String(a.className).slice(0, 30) : '',
        text: a && a.textContent ? a.textContent.trim().slice(0, 15) : '',
        dlgOpen: !!d,
      });
    });
    console.log('Tab', i, ':', info);
    await page.keyboard.press('Tab');
    await page.waitForTimeout(80);
  }
  console.log('ERRORS:', JSON.stringify(errs));
  await browser.close();
})().catch((e) => { console.error('PROBE-FAIL', e.message); process.exit(1); });
