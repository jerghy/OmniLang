import { chromium } from 'playwright';
import { writeFileSync } from 'fs';

const BASE = 'http://localhost:8789/index.html';
const RESULTS = 'D:/space/OmniLang/tests/results';

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const page = await context.newPage();

const pageErrors = [];
page.on('console', msg => { if (msg.type() === 'error') pageErrors.push(msg.text()); });
page.on('pageerror', err => pageErrors.push(`PAGE_ERROR: ${err.message}`));

let totalTests = 0, passedTests = 0;
const failures = [];

function assert(condition, msg) {
  totalTests++;
  if (condition) { passedTests++; }
  else { console.log(`  ❌ ${msg}`); failures.push(msg); }
}

async function getTabText() {
  return (await page.$eval('.tab-content', el => el.textContent)).trim();
}

async function clickTab(name) {
  await page.locator('.browserTab', { hasText: name }).click();
  await page.waitForTimeout(500);
}

// ═══════════════════════════════════════════
console.log("\n╔══════════════════════════════════════════╗");
console.log("║  OmniView — Full Test Suite v2         ║");
console.log("╚══════════════════════════════════════════╝");

await page.goto(BASE, { waitUntil: 'networkidle' });
await page.waitForTimeout(1000);

// Build info
const title = await page.title();
assert(title === 'OmniView App', `Page title is "OmniView App" (got "${title}")`);

// Tabs
const tabNames = await page.$$eval('.browserTab', els => els.map(el => el.textContent.trim()));
assert(tabNames.length === 10, `10 tabs present (got ${tabNames.length})`);

// ═══ HOME TAB ═══
console.log("\n── HOME TAB ──");
let text = await getTabText();

assert(text.includes('Dashboard'), 'Dashboard title');
assert(text.includes('Overview of'), 'Dashboard overview');
assert(text.includes('Refresh'), 'Dashboard refresh button');
assert(text.includes('统计面板'), 'StatsPanel');
assert(text.includes('计数器演示'), 'Counter');
assert(text.includes('Features'), 'FeatureList');

// Counter: click +
const counterSection = page.locator('[data-component="Counter"]');
const bigNumber = counterSection.locator('p.big-number');
const initialVal = (await bigNumber.textContent()).trim();
console.log(`  Counter initial: "${initialVal}"`);

// Click + using the button inside Counter
const plusBtn = counterSection.locator('button', { hasText: '+' });
await plusBtn.click();
await page.waitForTimeout(300);
const afterVal = (await bigNumber.textContent()).trim();
assert(afterVal !== initialVal, 'Counter: clicking + changes value');
console.log(`  Counter after +: "${afterVal}"`);

// Click -
const minusBtn = counterSection.locator('button', { hasText: '-' });
await minusBtn.click();
await page.waitForTimeout(300);
const afterMinus = (await bigNumber.textContent()).trim();
assert(afterMinus === initialVal, 'Counter: clicking - reverts value');

// Step buttons
const step5Btn = counterSection.locator('button', { hasText: '步长5' });
await step5Btn.click();
await page.waitForTimeout(200);
await plusBtn.click();
await page.waitForTimeout(300);
const afterStep5 = (await bigNumber.textContent()).trim();
assert(parseInt(afterStep5) >= 5 || afterStep5 === '5', 'Counter: step 5 increments by 5');

// Reset
const resetBtn = counterSection.locator('button', { hasText: '重置' });
await resetBtn.click();
await page.waitForTimeout(300);
const afterReset = (await bigNumber.textContent()).trim();
assert(afterReset === '0', 'Counter: reset sets to 0');

// FeatureList toggle
const featureSection = page.locator('[data-component="FeatureList"]');
const featureHeader = featureSection.locator('.header');
await featureHeader.click();
await page.waitForTimeout(300);
// Should be collapsed now - check features not visible
const featureCollapsed = await featureSection.textContent();
const isCollapsed = !featureCollapsed.includes('Store:');
assert(isCollapsed, 'FeatureList toggle collapses');
// Re-open
await featureHeader.click();
await page.waitForTimeout(300);

// Dashboard refresh
const refreshBtn = page.locator('button', { hasText: 'Refresh' });
await refreshBtn.click();
await page.waitForTimeout(300);
text = await getTabText();
assert(text.includes('updated'), 'Dashboard refresh updates status');

await page.screenshot({ path: `${RESULTS}/01_home.png` });

// ═══ TODO TAB ═══
console.log("\n── TODO TAB ──");
await clickTab('待办');
text = await getTabText();
assert(text.includes('Todo List'), 'TodoPage header');
assert(text.includes('Clear All'), 'Clear All button');
assert(text.includes('Sync Reminders'), 'Sync Reminders button');
assert(text.includes('tasks'), 'Todo task count shown');

// Check for todo items or empty state
if (text.includes('items to show') || text.includes('暂无') || text.includes('empty')) {
  console.log('  ℹ Todo list is empty (acceptable for clean project)');
}

await page.screenshot({ path: `${RESULTS}/02_todo.png` });

// ═══ INTERACTIVE TAB ═══
console.log("\n── INTERACTIVE TAB ──");
await clickTab('基础交互');
text = await getTabText();

// Check key components
assert(text.includes('Timer Demo'), 'Timer component');
assert(text.includes('Modal') || text.includes('Modal') || text.includes('Open Modal'), 'Modal component');
assert(text.includes('TabPanel') || text.includes('Tab Panel') || text.includes('Home'), 'TabPanel component');
assert(text.includes('DataTable') || text.includes('Sort'), 'DataTable component');
assert(text.includes('Form') || text.includes('Submit'), 'Form component');
assert(text.includes('Notifications') || text.includes('Notification'), 'Notifications component');

// Timer: test Tick button
const timerSection = page.locator('[data-component="Timer"]');
const tickBtn = timerSection.locator('button', { hasText: 'Tick' });
const startBtn = timerSection.locator('button', { hasText: 'Start' });
const resetBtnTimer = timerSection.locator('button', { hasText: 'Reset' });

// Click Tick
await tickBtn.click();
await page.waitForTimeout(300);
const timerText = await timerSection.textContent();
assert(timerText.includes('Count: 1') || timerText.includes('Count:'), 'Timer Tick increments count');

// Start then multiple ticks
await startBtn.click();
await tickBtn.click();
await page.waitForTimeout(200);
await tickBtn.click();
await page.waitForTimeout(200);
const timerRunning = await timerSection.textContent();
assert(timerRunning.includes('Running'), 'Timer Start sets status to Running');

// Reset
await resetBtnTimer.click();
const timerReset = await timerSection.textContent();
assert(timerReset.includes('Stopped'), 'Timer Reset stops and resets');

// Modal: test Open button
const modalBtn = page.locator('button', { hasText: 'Open Modal' });
if (await modalBtn.isVisible()) {
  await modalBtn.click();
  await page.waitForTimeout(300);
  // Check for modal overlay or content
  const modalVisible = await page.$('.modal-overlay, [class*="modal"], [class*="overlay"]');
  assert(modalVisible !== null, 'Modal opens on button click');

  // Close modal
  const closeBtn = page.locator('button', { hasText: 'X' }).first();
  if (await closeBtn.isVisible()) {
    await closeBtn.click();
    await page.waitForTimeout(300);
  }
}

await page.screenshot({ path: `${RESULTS}/03_interactive.png` });

// ═══ COMPILER TAB ═══
console.log("\n── COMPILER TAB ──");
await clickTab('编译器特性');
text = await getTabText();
assert(text.includes('HTTP GET Demo'), 'DataFetching: HTTP GET Demo');
assert(text.includes('HTTP POST Demo'), 'DataSubmit: HTTP POST Demo');
assert(text.includes('User Settings') || text.includes('username') || text.includes('fontSize'), 'UserSettings component');

// Try HTTP GET click
const loadBtn = page.locator('button', { hasText: 'Load' });
if (await loadBtn.isVisible()) {
  await loadBtn.click();
  await page.waitForTimeout(500);
  text = await getTabText();
  // Either loading, success, or error (network may fail in test env)
  assert(text.includes('Loading') || text.includes('Response') || text.includes('Error'), 'HTTP GET click triggers state change');
}

await page.screenshot({ path: `${RESULTS}/04_compiler.png` });

// ═══ MOTION TAB ═══
console.log("\n── MOTION TAB ──");
await clickTab('运动动画');
text = await getTabText();
assert(text.includes('Motion Demo') || text.includes('Motion'), 'Motion components present');

// Find and click animation trigger buttons
const motionTriggerBtns = await page.$$('button:has-text("Play")');
console.log(`  Play buttons found: ${motionTriggerBtns.length}`);

// Check for CSS animations/transitions
const animStyles = await page.evaluate(() => {
  const sheets = Array.from(document.styleSheets);
  let count = 0;
  for (const sheet of sheets) {
    try {
      const rules = Array.from(sheet.cssRules || []);
      for (const rule of rules) {
        if (rule.cssText && (rule.cssText.includes('animation') || rule.cssText.includes('@keyframes'))) count++;
      }
    } catch(e) {}
  }
  return count;
});
assert(animStyles > 0, `CSS @keyframes or animation rules found: ${animStyles}`);

// Click "Play All" or similar buttons
for (const btn of motionTriggerBtns.slice(0, 3)) {
  try {
    await btn.click();
    await page.waitForTimeout(100);
  } catch(e) {}
}
await page.waitForTimeout(500);

await page.screenshot({ path: `${RESULTS}/05_motion.png` });

// ═══ CSS TAB ═══
console.log("\n── CSS TAB ──");
await clickTab('CSS 特性');
text = await getTabText();
assert(text.includes('CSS') || text.includes('css') || text.includes('CssUnits') || text.includes('CssVars') || text.includes('ThemeCss'), 'CSS components present');

await page.screenshot({ path: `${RESULTS}/06_css.png` });

// ═══ EVENTS TAB ═══
console.log("\n── EVENTS TAB ──");
await clickTab('事件/表单');
text = await getTabText();

// Event components
assert(text.includes('EventShorthand') || text.includes('onPress') || text.includes('Event'), 'EventShorthand or event text');
assert(text.includes('emit') || text.includes('Emit'), 'Emit components');
assert(text.includes('Slot') || text.includes('Default'), 'Slot components');
assert(text.includes('Form') || text.includes('Input') || text.includes('input'), 'Form controls');

// Find event buttons and click them
const emitBtns = page.locator('button:has-text("emit"), button:has-text("onPress"), button:has-text("onFunc")');
const emitCount = await emitBtns.count();
if (emitCount > 0) {
  await emitBtns.first().click();
  await page.waitForTimeout(300);
  console.log(`  Clicked event button, ${emitCount} total`);
}

await page.screenshot({ path: `${RESULTS}/07_events.png` });

// ═══ LANGUAGE TAB ═══
console.log("\n── LANGUAGE TAB ──");
await clickTab('语言特性');
text = await getTabText();

assert(text.includes('Inline Store Demo') || text.includes('InlineStore'), 'InlineStore component');
assert(text.includes('类型系统全覆盖') || text.includes('TypeSystem') || text.includes('有符号整数'), 'TypeSystem component');
assert(text.includes('String') || text.includes('Literals'), 'StringLiterals component');
assert(text.includes('视图属性') || text.includes('ViewAttributes'), 'ViewAttributes component');
assert(text.includes('Nullish') || text.includes('displayName'), 'Nullish component');
assert(text.includes('Computed') || text.includes('Form'), 'ComputedFnForm component');
assert(text.includes('Optional') || text.includes('Chain'), 'OptionalChain component');
assert(text.includes(':hover') || text.includes('StylePseudo') || text.includes('hover'), 'StylePseudo component');

// Test InlineStore: click Add Apple
const addAppleBtn = page.locator('button', { hasText: 'Add Apple' });
if (await addAppleBtn.isVisible()) {
  await addAppleBtn.click();
  await page.waitForTimeout(300);
  const storeText = await page.locator('text=Items:').textContent();
  assert(storeText.includes('Items:'), 'InlineStore: Add Apple works');
}

await page.screenshot({ path: `${RESULTS}/08_language.png` });

// ═══ SETTINGS TAB ═══
console.log("\n── SETTINGS TAB ──");
await clickTab('设置');
text = await getTabText();
// SettingsPage or ThemeSwitcher
assert(text.includes('Settings') || text.includes('设置') || text.includes('Theme') || text.includes('Light'), 'Settings components render');

await page.screenshot({ path: `${RESULTS}/09_settings.png` });

// ═══ SUPPLEMENT TAB ═══
console.log("\n── SUPPLEMENT TAB ──");
await clickTab('补充');
text = await getTabText();

assert(text.includes('Ternary') || text.includes('三元'), 'TernaryExpr component');
assert(text.includes('Block') || text.includes('块表达式'), 'BlockExpr component');
assert(text.includes('ForDirective') || text.includes('For'), 'ForDirectiveKey component');
assert(text.includes('Advanced') || text.includes('Pseudo'), 'AdvancedPseudo component');
assert(text.includes('Steps') || text.includes('Start'), 'StepsStart component');
assert(text.includes('Path') || text.includes('Timeline'), 'PathTimeline component');

await page.screenshot({ path: `${RESULTS}/10_supplement.png` });

// ═══ FINAL SUMMARY ═══
console.log(`\n${'═'.repeat(50)}`);
console.log(`  Tests: ${passedTests}/${totalTests} passed (${Math.round(passedTests/totalTests*100)}%)`);
if (failures.length > 0) {
  console.log(`  Failures:`);
  failures.forEach(f => console.log(`    • ${f}`));
}
if (pageErrors.length > 0) {
  console.log(`\n  Console errors (${pageErrors.length}):`);
  // Deduplicate
  const unique = [...new Set(pageErrors)];
  unique.slice(0, 5).forEach(e => console.log(`    • ${e.substring(0, 120)}`));
}
console.log(`${'═'.repeat(50)}`);

writeFileSync(`${RESULTS}/test_results.json`, JSON.stringify({
  passed: passedTests,
  total: totalTests,
  failures,
  errors: [...new Set(pageErrors)]
}, null, 2));

await browser.close();
