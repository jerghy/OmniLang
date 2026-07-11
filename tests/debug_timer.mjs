import { chromium } from 'playwright';
const page = await (await chromium.launch({ headless: true })).newPage({ viewport: { width: 1440, height: 900 } });

page.on('console', msg => console.log(`[${msg.type()}] ${msg.text()}`));
page.on('pageerror', err => console.log(`PAGE ERROR: ${err.message}`));

await page.goto('http://localhost:8789/index.html', { waitUntil: 'networkidle' });
await page.waitForTimeout(1000);

// Go to interactive tab
const tab = page.locator('.browserTab', { hasText: '基础交互' });
await tab.click();
await page.waitForTimeout(800);

// Check if Timer component exists
const timerCount = await page.locator('[data-component="Timer"]').count();
console.log(`Timer components found: ${timerCount}`);

if (timerCount > 0) {
  const timer = page.locator('[data-component="Timer"]').first();
  const timerHtml = await timer.innerHTML();
  console.log(`Timer HTML:\n${timerHtml.substring(0, 800)}`);

  // Find all buttons inside Timer
  const btns = await timer.locator('button').all();
  console.log(`\nTimer buttons:`);
  for (const btn of btns) {
    const text = (await btn.textContent()).trim();
    console.log(`  - "${text}"`);
  }

  // Take initial snapshot
  let t0 = await timer.textContent();
  console.log(`\nInitial timer content:\n${t0.replace(/\s+/g, ' ').trim()}`);

  // Click "Start" button
  const startBtn = timer.locator('button', { hasText: 'Start' });
  console.log(`\nStart button exists: ${await startBtn.count()}`);
  await startBtn.click();
  await page.waitForTimeout(300);

  let t1 = await timer.textContent();
  console.log(`After Start:\n${t1.replace(/\s+/g, ' ').trim()}`);

  // Click "Tick" button
  const tickBtn = timer.locator('button', { hasText: 'Tick' });
  console.log(`Tick button exists: ${await tickBtn.count()}`);
  await tickBtn.click();
  await page.waitForTimeout(300);

  let t2 = await timer.textContent();
  console.log(`After Tick:\n${t2.replace(/\s+/g, ' ').trim()}`);

  // Click Tick again
  await tickBtn.click();
  await page.waitForTimeout(300);
  let t3 = await timer.textContent();
  console.log(`After second Tick:\n${t3.replace(/\s+/g, ' ').trim()}`);

  // Check if the count actually changed by looking at the text
  console.log(`\nCount in text contains "Count:" ? ${t2.includes('Count:')}`);
  console.log(`Contains "Count: 1": ${t2.replace(/\s+/g, ' ').includes('Count: 1')}`);
}

await page.close();
