const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errs = [];
  page.on('pageerror', (e) => errs.push('PAGEERROR: ' + e.message));
  page.on('console', (m) => { if (m.type() === 'error') errs.push('CONSOLE: ' + m.text()); });
  const resp = await page.goto('http://localhost:5173/', { waitUntil: 'domcontentloaded' });
  console.log('STATUS:', resp && resp.status());
  await page.waitForTimeout(2500);
  console.log('TITLE:', await page.title());
  const body = await page.evaluate(() => document.body ? document.body.innerText.slice(0, 400) : 'NO BODY');
  console.log('BODY:', JSON.stringify(body));
  console.log('BUTTONS:', await page.locator('button').count());
  console.log('ERRORS:', JSON.stringify(errs));
  await browser.close();
})().catch((e) => { console.error('PROBE-FAIL', e.message); process.exit(1); });
