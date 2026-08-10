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
  await openBtn.click();
  await page.waitForTimeout(400);

  const info = await page.evaluate(() => {
    const d = document.querySelector('dialog');
    if (!d) return { found: false };
    const r = d.getBoundingClientRect();
    return {
      found: true,
      rect: { x: r.x, y: r.y, w: r.width, h: r.height },
      vw: window.innerWidth, vh: window.innerHeight,
      hasBackdrop: !!window.getComputedStyle(d, '::backdrop') && getComputedStyle(d).display !== 'none',
      open: d.open,
      modal: !!d.matches(':modal'),
    };
  });
  console.log('DIALOG INFO:', JSON.stringify(info));

  // click in the middle of the left-side backdrop area (x=30, y=vh/2)
  const y = Math.min(info.rect.y + info.rect.h + 40, info.vh - 30);
  await page.mouse.click(30, y);
  await page.waitForTimeout(400);
  console.log('after left-backdrop click: dialogs =', await page.locator('dialog').count(), '| open attr =', await page.locator('dialog[open]').count());

  if (await page.locator('dialog').count() === 1) {
    // try clicking at very edge
    await page.mouse.click(5, 5);
    await page.waitForTimeout(400);
    console.log('after edge click: dialogs =', await page.locator('dialog').count());
  }
  if (await page.locator('dialog').count() === 1) {
    await page.keyboard.press('Escape');
    await page.waitForTimeout(400);
    console.log('after Escape: dialogs =', await page.locator('dialog').count());
  }
  console.log('ERRORS:', JSON.stringify(errs));
  await browser.close();
})().catch((e) => { console.error('PROBE-FAIL', e.message); process.exit(1); });
