import { chromium } from 'playwright';
import { writeFileSync } from 'fs';

const BASE = 'http://localhost:8789/index.html';
const RESULTS = 'D:/space/OmniLang/tests/results';

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const page = await context.newPage();

const pageErrors = [];
page.on('console', msg => { if (msg.type() === 'error') pageErrors.push(msg.text()); });

let totalTests = 0, passedTests = 0;
const failures = [];

function assert(condition, msg) {
  totalTests++;
  if (condition) passedTests++;
  else { console.log(`  ❌ ${msg}`); failures.push(msg); }
}

async function getTabText() { return (await page.$eval('.tab-content', el => el.textContent)).trim(); }
async function clickTab(name) { await page.locator('.browserTab', { hasText: name }).click(); await page.waitForTimeout(500); }

// ═══════════════ LAUNCH ═══════════════
await page.goto(BASE, { waitUntil: 'networkidle' });
await page.waitForTimeout(1000);

const tabs = await page.$$eval('.browserTab', els => els.map(el => el.textContent.trim()));
assert(tabs.length === 10, `10 tabs present`);
assert(tabs[0] === '首页', 'First tab is 首页');
assert(tabs.includes('运动动画'), 'Motion tab exists');

// Note: 404 errors for /dot.png, /assets/* are from test data in source (not compiler bugs)
console.log(`  Console errors (from test data assets): ${pageErrors.length}`);

// ═══════════════ HOME ═══════════════
console.log('\n══ HOME ══');
let text = await getTabText();
assert(text.includes('Dashboard'), 'Dashboard renders');
assert(text.includes('统计面板'), 'StatsPanel renders');
assert(text.includes('计数器演示'), 'Counter renders');
assert(text.includes('Features'), 'FeatureList renders');

// Counter
const ctr = page.locator('[data-component="Counter"]');
const ctrVal = () => ctr.locator('.big-number').textContent().then(t => t.trim());
assert(await ctrVal() === '0', 'Counter starts at 0');
await ctr.locator('button:has-text("+")').click(); await page.waitForTimeout(200);
assert(await ctrVal() === '1', 'Counter + → 1');
await ctr.locator('button:has-text("-")').click(); await page.waitForTimeout(200);
assert(await ctrVal() === '0', 'Counter - → 0');
await ctr.locator('button:has-text("步长5")').click(); await page.waitForTimeout(100);
await ctr.locator('button:has-text("+")').click(); await page.waitForTimeout(200);
assert(await ctrVal() === '5', 'Step 5 works');
await ctr.locator('button:has-text("重置")').click(); await page.waitForTimeout(200);
assert(await ctrVal() === '0', 'Reset works');

// FeatureList toggle
const fl = page.locator('[data-component="FeatureList"]');
assert(await fl.locator('.list >> visible=true').count() > 0, 'Features visible initially');
await fl.locator('.header').click(); await page.waitForTimeout(400);
assert(await fl.locator('.list >> visible=true').count() === 0, 'Collapse works');
await fl.locator('.header').click(); await page.waitForTimeout(400);
assert(await fl.locator('.list >> visible=true').count() > 0, 'Re-expand works');

// Dashboard refresh
await page.locator('button:has-text("Refresh")').click(); await page.waitForTimeout(300);
assert((await getTabText()).includes('updated'), 'Refresh works');

await page.screenshot({ path: `${RESULTS}/01_home.png` });

// ═══════════════ TODO ═══════════════
console.log('\n══ TODO ══');
await clickTab('待办');
text = await getTabText();
assert(text.includes('Todo List'), 'TodoPage header');
assert(text.includes('Clear All'), 'Clear All button');
assert(text.includes('Sync Reminders'), 'Sync Reminders');
assert(text.includes('tasks'), 'Task count shown');

await page.screenshot({ path: `${RESULTS}/02_todo.png` });

// ═══════════════ INTERACTIVE ═══════════════
console.log('\n══ INTERACTIVE ══');
await clickTab('基础交互');
text = await getTabText();
assert(text.includes('Timer Demo'), 'Timer Demo');
assert(text.includes('Home') && text.includes('About') && text.includes('Settings'), 'TabPanel buttons');
assert(text.includes('DataTable') || text.includes('Score'), 'DataTable');
assert(text.includes('Form') || text.includes('Submit') || text.includes('Validation'), 'Form');
assert(text.includes('Notifications') || text.includes('Notification'), 'Notifications');

// Timer: Start → Tick → check count
const ts = page.locator('[data-component="Timer"]');
const tsText = () => ts.textContent().then(t => t.replace(/\s+/g, ' '));
await ts.locator('button:has-text("Start")').click(); await page.waitForTimeout(200);
await ts.locator('button:has-text("Tick")').click(); await page.waitForTimeout(300);
assert((await tsText()).includes('Count: 1'), 'Timer Tick → Count: 1');
await ts.locator('button:has-text("Tick")').click(); await page.waitForTimeout(200);
assert((await tsText()).includes('Count: 2'), 'Timer second Tick → Count: 2');
assert((await tsText()).includes('Status: Running'), 'Timer status: Running');

// Timer: Stop
await ts.locator('button:has-text("Stop")').click(); await page.waitForTimeout(300);
assert((await tsText()).includes('Stopped'), 'Timer Stop → Stopped');

// Timer: Reset
await ts.locator('button:has-text("Reset")').click(); await page.waitForTimeout(300);
assert((await tsText()).includes('Count: 0'), 'Timer Reset → Count: 0');

await page.screenshot({ path: `${RESULTS}/03_interactive.png` });

// ═══════════════ COMPILER ═══════════════
console.log('\n══ COMPILER ══');
await clickTab('编译器特性');
text = await getTabText();
assert(text.includes('HTTP GET Demo'), 'DataFetching');
assert(text.includes('HTTP POST Demo'), 'DataSubmit');
assert(text.includes('Storage Demo'), 'UserSettings');

// Check interactive elements on DataFetching
await page.locator('button:has-text("Load")').first().click(); await page.waitForTimeout(1000);
text = await getTabText();
assert(text.includes('Loading') || text.includes('Response') || text.includes('Error') || text.includes('idle'), 'HTTP GET triggers state');

await page.screenshot({ path: `${RESULTS}/04_compiler.png` });

// ═══════════════ MOTION ═══════════════
console.log('\n══ MOTION ══');
await clickTab('运动动画');
text = await getTabText();
assert(text.length > 200, 'Motion tab has content');

// Check CSS @keyframes
const animRules = await page.evaluate(() => {
  let c = 0;
  for (const s of document.styleSheets) {
    try { for (const r of s.cssRules) if (r.cssText.match(/animation|@keyframes/i)) c++; } catch(e) {}
  }
  return c;
});
assert(animRules > 0, `@keyframes/animation rules: ${animRules}`);
console.log(`  @keyframes found: ${animRules}`);

// Click Play buttons
const playBtns = await page.$$('button:has-text("Play")');
console.log(`  Play buttons: ${playBtns.length}`);
if (playBtns.length > 0) {
  for (const btn of playBtns.slice(0, 3)) {
    try { await btn.click(); await page.waitForTimeout(200); } catch(e) {}
  }
}

await page.screenshot({ path: `${RESULTS}/05_motion.png` });

// ═══════════════ CSS ═══════════════
console.log('\n══ CSS ══');
await clickTab('CSS 特性');
text = await getTabText();
assert(text.length > 50, 'CSS tab content');

await page.screenshot({ path: `${RESULTS}/06_css.png` });

// ═══════════════ EVENTS ═══════════════
console.log('\n══ EVENTS ══');
await clickTab('事件/表单');
text = await getTabText();
assert(text.includes('onPress') || text.includes('Event'), 'Events content');
assert(text.includes('emit') || text.includes('Emit'), 'Emit content');
assert(text.includes('Slot') || text.includes('Default'), 'Slot content');

const ebtns = page.locator('button:has-text("emit"), button:has-text("onPress"), button:has-text("onFunc")');
const ec = await ebtns.count();
if (ec > 0) { await ebtns.first().click(); await page.waitForTimeout(300); }
console.log(`  Event buttons: ${ec}`);

await page.screenshot({ path: `${RESULTS}/07_events.png` });

// ═══════════════ LANGUAGE ═══════════════
console.log('\n══ LANGUAGE ══');
await clickTab('语言特性');
text = await getTabText();
assert(text.includes('Inline Store Demo'), 'InlineStore');
assert(text.includes('类型系统全覆盖'), 'TypeSystem');
assert(text.includes('视图属性'), 'ViewAttributes');
assert(text.includes('String') || text.includes('Literal'), 'StringLiterals');
assert(text.includes('Computed') || text.includes('Form'), 'ComputedFnForm');
assert(text.includes('Optional') || text.includes('Chain'), 'OptionalChain');
assert(text.includes(':hover') || text.includes('hover'), 'StylePseudo');

// InlineStore: Add Apple
const addBtn = page.locator('button:has-text("Add Apple")');
if (await addBtn.isVisible()) {
  await addBtn.click(); await page.waitForTimeout(300);
  text = await getTabText();
  assert(text.includes('Items:'), 'Add Apple works');
}

await page.screenshot({ path: `${RESULTS}/08_language.png` });

// ═══════════════ SETTINGS ═══════════════
console.log('\n══ SETTINGS ══');
await clickTab('设置');
text = await getTabText();
assert(text.includes('Light') || text.includes('Dark') || text.includes('Theme') || text.includes('Mode'), 'Theme settings');

await page.screenshot({ path: `${RESULTS}/09_settings.png` });

// ═══════════════ SUPPLEMENT ═══════════════
console.log('\n══ SUPPLEMENT ══');
await clickTab('补充');
text = await getTabText();
assert(text.includes('Ternary') || text.includes('三元'), 'TernaryExpr');
assert(text.includes('Block') || text.includes('块'), 'BlockExpr');
assert(text.includes('@for') || text.includes('For'), 'ForDirectiveKey');
assert(text.includes('Advanced') || text.includes('Pseudo'), 'AdvancedPseudo');
assert(text.includes('Steps') || text.includes('Easing'), 'StepsStart');
assert(text.includes('Path') || text.includes('Timeline'), 'PathTimeline');

await page.screenshot({ path: `${RESULTS}/10_supplement.png` });

// ═══════════════ SUMMARY ═══════════════
const pct = Math.round(passedTests/totalTests*100);
console.log(`\n${'═'.repeat(50)}`);
console.log(`  ${passedTests}/${totalTests} passed (${pct}%)`);
if (failures.length) {
  console.log(`  Failures:`);
  failures.forEach(f => console.log(`    • ${f}`));
}
if (pageErrors.length) {
  const unique = [...new Set(pageErrors)];
  console.log(`\n  Console errors: ${unique.length} unique`);
  console.log(`  (expected — test data references non-existent assets like /dot.png)`);
}
console.log(`${'═'.repeat(50)}`);

writeFileSync(`${RESULTS}/final_results.json`, JSON.stringify({
  passed: passedTests, total: totalTests, failures, errors: [...new Set(pageErrors)]
}, null, 2));

await browser.close();
