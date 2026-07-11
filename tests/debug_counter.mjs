import { chromium } from 'playwright';
const page = await (await chromium.launch({ headless: true })).newPage({ viewport: { width: 1440, height: 900 } });

page.on('console', msg => console.log(`[${msg.type()}] ${msg.text()}`));
page.on('pageerror', err => console.log(`PAGE ERROR: ${err.message}`));

await page.goto('http://localhost:8789/index.html', { waitUntil: 'networkidle' });
await page.waitForTimeout(1000);

// Find all Counter component instances
const counters = await page.$$('[data-component="Counter"]');
console.log(`Found ${counters.length} Counter instances`);

for (let i = 0; i < counters.length; i++) {
  const text = await counters[i].textContent();
  console.log(`Counter ${i}: "${text.substring(0, 100).replace(/\s+/g, ' ').trim()}"`);
}

// Find big numbers
const bigNumbers = await page.$$('.big-number');
console.log(`Found ${bigNumbers.length} .big-number elements`);

for (let i = 0; i < bigNumbers.length; i++) {
  const text = await bigNumbers[i].textContent();
  console.log(`big-number ${i}: "${JSON.stringify(text)}"`);

  // Check the interpolation span inside
  const span = await bigNumbers[i].$('span');
  if (span) {
    const spanText = await span.textContent();
    console.log(`  interp span: "${JSON.stringify(spanText)}"`);
  }
}

// Find - buttons
const minusBtns = await page.$$('button');
console.log(`\nAll buttons:`);
for (const btn of minusBtns) {
  const text = await btn.textContent();
  console.log(`  Button: "${text.replace(/\s+/g, ' ').trim()}"`);
}

// Try clicking + on first Counter
const counter = counters[0];
const plusBtn = await counter.$('button');
if (plusBtn) {
  const plusText = await plusBtn.textContent();
  console.log(`\nFirst button in first Counter: "${plusText.replace(/\s+/g, ' ').trim()}"`);
}

// Click the specific + button (button with text "+")
const allBtns = await page.$$('button');
let plusClicked = false;
for (const btn of allBtns) {
  const text = (await btn.textContent()).replace(/\s+/g, '').trim();
  if (text === '+') {
    console.log(`\nFound + button: ${text}`);
    await btn.click();
    await page.waitForTimeout(500);

    // Check if any big number changed
    for (let i = 0; i < bigNumbers.length; i++) {
      const newText = await bigNumbers[i].textContent();
      console.log(`big-number ${i} after click: "${JSON.stringify(newText)}"`);
    }
    plusClicked = true;
    break;
  }
}
if (!plusClicked) console.log('Could not find + button to click');

// Also try the - button
for (const btn of allBtns) {
  const text = (await btn.textContent()).replace(/\s+/g, '').trim();
  if (text === '-') {
    console.log(`Found - button, clicking...`);
    await btn.click();
    await page.waitForTimeout(500);

    for (let i = 0; i < bigNumbers.length; i++) {
      const newText = await bigNumbers[i].textContent();
      console.log(`big-number ${i} after - click: "${JSON.stringify(newText)}"`);
    }
    break;
  }
}

// Check full text of first Counter
console.log(`\nFull first counter text:`);
const firstCounter = counters[0];
const fullText = await firstCounter.evaluate(el => el.innerHTML);
console.log(fullText.substring(0, 1000));

await page.close();
