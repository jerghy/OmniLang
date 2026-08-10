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
  console.log('openBtn count:', await openBtn.count());
  if (await openBtn.count()) {
    await openBtn.click();
    await page.waitForTimeout(400);
    const dialogs = await page.locator('dialog').count();
    const modalMatch = await page.locator('dialog:modal').count();
    const openAttr = await page.locator('dialog[open]').count();
    const titleVisible = await page.locator('dialog').getByText('Modal Title').first().isVisible().catch(() => false);
    const bodyText = await page.locator('dialog').textContent().catch(() => '');
    console.log('dialogs:', dialogs, '| :modal:', modalMatch, '| [open]:', openAttr, '| titleVisible:', titleVisible);
    console.log('dialog text head:', JSON.stringify((bodyText || '').slice(0, 60)));
    // backdrop click should close
    await page.mouse.click(20, 20);
    await page.waitForTimeout(300);
    console.log('after backdrop click, dialog count:', await page.locator('dialog').count());
    // reopen + Escape
    if (await openBtn.count()) {
      await openBtn.click();
      await page.waitForTimeout(300);
      console.log('reopened, :modal count:', await page.locator('dialog:modal').count());
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);
      console.log('after Escape, dialog count:', await page.locator('dialog').count());
    }
  }
  console.log('ERRORS:', JSON.stringify(errs));
  await browser.close();
})().catch((e) => { console.error('PROBE-FAIL', e.message); process.exit(1); });
