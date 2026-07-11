import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

console.log("=== NAVIGATING TO HOME ===");
await page.goto('http://localhost:8789/index.html', { waitUntil: 'networkidle' });

const title = await page.title();
console.log(`Title: ${title}`);

const tabs = await page.$$eval('.browserTab', els => els.map(el => el.textContent.trim()));
console.log(`Tabs found: ${JSON.stringify(tabs)}`);

const activeTab = await page.$('.browserTab.tabActive');
if (activeTab) {
  const activeText = await activeTab.textContent();
  console.log(`Active tab: ${activeText.trim()}`);
}

const contentEl = await page.$('.tab-content');
if (contentEl) {
  const allText = await contentEl.textContent();
  console.log(`Content preview: ${allText.substring(0, 800)}`);
}

const errors = [];
page.on('console', msg => {
  if (msg.type() === 'error') errors.push(msg.text());
});

await new Promise(r => setTimeout(r, 1000));
console.log(`Console errors: ${JSON.stringify(errors)}`);

await page.screenshot({ path: 'D:/space/OmniLang/tests/results/home_page.png', fullPage: true });
console.log("Screenshot saved");

await browser.close();
