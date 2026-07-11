import { chromium } from 'playwright';
const page = await (await chromium.launch({ headless: true })).newPage();
page.on('console', msg => {
  if (msg.type() === 'error') console.log(`CONSOLE ERROR: ${msg.text()}`);
});
page.on('requestfailed', req => console.log(`REQUEST FAILED: ${req.url()} (${req.failure().errorText})`));
page.on('response', resp => {
  if (resp.status() >= 400) console.log(`HTTP ${resp.status()}: ${resp.url().substring(0, 200)}`);
});
await page.goto('http://localhost:8789/index.html', { waitUntil: 'networkidle' });
await new Promise(r => setTimeout(r, 2000));
await page.close();
