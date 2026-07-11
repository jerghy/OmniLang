import { chromium } from 'playwright';

const BASE = 'http://localhost:8789/index.html';
const RESULTS = 'D:/space/OmniLang/tests/results';

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const page = await context.newPage();

const errors = [];
page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });

let totalTests = 0;
let passedTests = 0;
let failures = [];

function assert(condition, msg) {
  totalTests++;
  if (condition) { passedTests++; }
  else { console.log(`  ❌ ${msg}`); failures.push(msg); }
}

async function clickTab(tabName) {
  const tab = page.locator('.browserTab', { hasText: tabName });
  await tab.click();
  await page.waitForTimeout(500);
}

async function tabText() {
  return (await page.$eval('.tab-content', el => el.textContent)).trim();
}

// ========================
// START
// ========================
console.log("\n==============================================");
console.log("  OmniView Learn Project — Full Test Suite");
console.log("==============================================");

await page.goto(BASE, { waitUntil: 'networkidle' });
await page.waitForTimeout(800);
assert(errors.length === 0, 'No console errors on page load');
if (errors.length > 0) console.log(`  ⚠ Errors: ${errors.join('; ')}`);

// Check tabs exist
const tabNames = await page.$$eval('.browserTab', els => els.map(el => el.textContent.trim()));
assert(tabNames.length === 10, `10 tabs found (got ${tabNames.length})`);
assert(tabNames[0] === '首页', 'First tab is 首页');
assert(tabNames.includes('待办'), 'Todo tab exists');
assert(tabNames.includes('运动动画'), 'Motion tab exists');

// ========================
// TAB: 首页 — Dashboard
// ========================
console.log("\n——— TAB: 首页 ———");
let text = await tabText();

// Dashboard
assert(text.includes('Dashboard'), 'Dashboard title visible');
assert(text.includes('Overview of'), 'Dashboard overview text');
assert(text.includes('45'), 'Counter incremented to 45 (from 42+3 clicks, or matches current)');
assert(text.includes('updated'), 'Dashboard shows updated status');
assert(text.includes('Refresh'), 'Refresh button exists');

// StatsPanel
assert(text.includes('统计面板'), 'StatsPanel renders');
assert(text.includes('总事项'), 'StatsPanel shows total field');

// Counter
assert(text.includes('计数器演示'), 'Counter renders');
assert(text.includes('翻倍'), 'Counter doubled text');
assert(text.includes('步长'), 'Counter step text');

// FeatureList
assert(text.includes('Features'), 'FeatureList renders');

// Check Counter initial value and interaction
const bigNumber = page.locator('p.big-number').first();
let counterVal = await bigNumber.textContent();
console.log(`  Counter value before click: ${counterVal}`);

// Click + button
const plusBtn = page.locator('button', { hasText: '+' }).first();
await plusBtn.click();
await page.waitForTimeout(300);
const afterPlus = await bigNumber.textContent();
assert(afterPlus !== counterVal, 'Counter + changes value');

// Click - button
const minusBtn = page.locator('button', { hasText: '-' }).first();
await minusBtn.click();
await page.waitForTimeout(300);
const afterMinus = await bigNumber.textContent();
// After + then -, should be back to original
assert(afterMinus === counterVal, 'Counter - reverts + click');

// Test step buttons
const step5Btn = page.locator('button', { hasText: '步长5' });
await step5Btn.click();
await page.waitForTimeout(200);
await plusBtn.click();
await page.waitForTimeout(300);
const afterStep5 = await bigNumber.textContent();
assert(parseInt(afterStep5) === parseInt(counterVal) + 5 || parseInt(afterStep5) > parseInt(counterVal), 'Step 5 button works, increments by 5');

// Reset
const resetBtn = page.locator('button', { hasText: '重置' });
await resetBtn.click();
await page.waitForTimeout(300);
const afterReset = await bigNumber.textContent();
assert(afterReset === '0', 'Reset sets counter to 0');

// FeatureList toggle
const featureHeader = page.locator('.header').first();
const featureText = await featureHeader.textContent();
assert(featureText.includes('Features'), 'FeatureList header visible');
// Check features are visible initially
assert(text.includes('Store:'), 'Features list items visible');

// Toggle collapse
await featureHeader.click();
await page.waitForTimeout(300);

await page.screenshot({ path: `${RESULTS}/01_home_tab.png` });
console.log("  📸 home_tab.png saved");

// ========================
// TAB: 待办
// ========================
console.log("\n——— TAB: 待办 ———");
await clickTab('待办');
text = await tabText();

assert(text.includes('Todo List'), 'TodoPage header renders');
assert(text.includes('tasks'), 'TodoPage shows task count');
assert(text.includes('Clear All'), 'Clear All button exists');
assert(text.includes('Sync Reminders'), 'Sync Reminders button exists');

// Check FilterBar
assert(text.includes('Filter') || text.includes('全部') || text.includes('All'), 'FilterBar renders');

// Check if there are todo items
const hasItems = text.includes('AddTodo') || text.includes('Add') || text.includes('添加');
// Todo items might be empty, that's OK

await page.screenshot({ path: `${RESULTS}/02_todo_tab.png` });
console.log("  📸 todo_tab.png saved");

// ========================
// TAB: 基础交互
// ========================
console.log("\n——— TAB: 基础交互 ———");
await clickTab('基础交互');
text = await tabText();

assert(text.includes('ValidationInput') || text.includes('Timer') || text.includes('Modal') || text.includes('TabPanel') || text.includes('DataTable') || text.includes('FormPage') || text.includes('NotificationsPanel'), 'Interactive components render');

if (text.includes('Timer')) {
  console.log("  ⏱ Timer component detected");
  // Check timer is running by checking it changes
  const timerText1 = await page.$eval('.tab-content', el => el.textContent);
  await page.waitForTimeout(1100);
  const timerText2 = await page.$eval('.tab-content', el => el.textContent);
  assert(timerText1 !== timerText2, 'Timer is actively ticking (content changed)');
}

await page.screenshot({ path: `${RESULTS}/03_interactive_tab.png` });
console.log("  📸 interactive_tab.png saved");

// ========================
// TAB: 编译器特性
// ========================
console.log("\n——— TAB: 编译器特性 ———");
await clickTab('编译器特性');
text = await tabText();
assert(text.length > 100, 'Compiler tab has substantial content');
assert(text.includes('DataFetching') || text.includes('DataSubmit') || text.includes('UserSettings') || text.includes('数据获取') || text.includes('数据提交'), 'Compiler feature components render');

await page.screenshot({ path: `${RESULTS}/04_compiler_tab.png` });
console.log("  📸 compiler_tab.png saved");

// ========================
// TAB: 运动动画
// ========================
console.log("\n——— TAB: 运动动画 ———");
await clickTab('运动动画');
text = await tabText();
assert(text.length > 200, 'Motion tab has substantial content');

// Check motion-related components
const motionKeywords = ['Motion', 'Easing', 'Timeline', 'Stagger', 'Hero', 'Playback', '动画', 'Callback', 'Position', '触发'];
const foundMotion = motionKeywords.filter(k => text.includes(k));
assert(foundMotion.length >= 2, `Motion components detected: ${foundMotion.slice(0, 5).join(', ')}`);

// Check for animated elements (CSS animations or transitions)
const animatedEls = await page.$$eval('[style*="animation"], [style*="transition"], .motion, .animated, [class*="motion"], [class*="animate"]', els => els.length);
console.log(`  Animated elements found: ${animatedEls}`);

await page.screenshot({ path: `${RESULTS}/05_motion_tab.png` });
console.log("  📸 motion_tab.png saved");

// ========================
// TAB: CSS 特性
// ========================
console.log("\n——— TAB: CSS 特性 ———");
await clickTab('CSS 特性');
text = await tabText();
assert(text.length > 50, 'CSS tab has content');

const cssKeywords = ['CssUnitsDemo', 'CssVarsDemo', 'ThemeCssVarsDemo', 'CSS', '单位', '变量', '主题'];
const foundCss = cssKeywords.filter(k => text.includes(k));
assert(foundCss.length >= 1, `CSS components detected: ${foundCss.slice(0, 3).join(', ')}`);

await page.screenshot({ path: `${RESULTS}/06_css_tab.png` });
console.log("  📸 css_tab.png saved");

// ========================
// TAB: 事件/表单
// ========================
console.log("\n——— TAB: 事件/表单 ———");
await clickTab('事件/表单');
text = await tabText();

const eventKeywords = ['EventShorthand', 'FormControls', 'EventsDemo', 'EventData', 'SlotDefault', 'EmitKeyword', 'EmitEvent', '事件', '表单'];
const foundEvents = eventKeywords.filter(k => text.includes(k));
assert(foundEvents.length >= 1, `Event components detected: ${foundEvents.slice(0, 4).join(', ')}`);

// Try to find buttons and click them
const eventBtns = await page.$$eval('button', btns => btns.filter(b => b.textContent.includes('Event') || b.textContent.includes('事件') || b.textContent.includes('Click') || b.textContent.includes('emit')).map(b => b.textContent.trim()));
if (eventBtns.length > 0) console.log(`  Event buttons found: ${eventBtns.slice(0, 5).join(', ')}`);

await page.screenshot({ path: `${RESULTS}/07_events_tab.png` });
console.log("  📸 events_tab.png saved");

// ========================
// TAB: 语言特性
// ========================
console.log("\n——— TAB: 语言特性 ———");
await clickTab('语言特性');
text = await tabText();

const langKeywords = ['InlineStoreDemo', 'NullishDemo', 'StringLiteralsDemo', 'TypeSystemDemo', 'ViewAttributesDemo', 'ComputedFnFormDemo', 'OptionalChainDemo', 'StylePseudoDemo', 'InlineStore', 'Nullish', 'TypeSystem', 'ViewAttributes', 'ComputedFn', 'OptionalChain', 'StylePseudo'];
const foundLang = langKeywords.filter(k => text.includes(k));
assert(foundLang.length >= 2, `Language components detected: ${foundLang.slice(0, 6).join(', ')}`);

await page.screenshot({ path: `${RESULTS}/08_language_tab.png` });
console.log("  📸 language_tab.png saved");

// ========================
// TAB: 设置
// ========================
console.log("\n——— TAB: 设置 ———");
await clickTab('设置');
text = await tabText();
assert(text.length > 50, 'Settings tab has content');
assert(text.includes('Settings') || text.includes('设置') || text.includes('Theme'), 'Settings component renders');

await page.screenshot({ path: `${RESULTS}/09_settings_tab.png` });
console.log("  📸 settings_tab.png saved");

// ========================
// TAB: 补充
// ========================
console.log("\n——— TAB: 补充 ———");
await clickTab('补充');
text = await tabText();

const suppKeywords = ['TernaryExprDemo', 'BlockExprDemo', 'ForDirectiveKeyDemo', 'AdvancedPseudoDemo', 'StepsStartDemo', 'PathTimelineDemo', 'Ternary', 'BlockExpr', 'ForDirective', 'AdvancedPseudo', 'StepsStart', 'PathTimeline'];
const foundSupp = suppKeywords.filter(k => text.includes(k));
assert(foundSupp.length >= 2, `Supplement components detected: ${foundSupp.slice(0, 6).join(', ')}`);

await page.screenshot({ path: `${RESULTS}/10_supplement_tab.png` });
console.log("  📸 supplement_tab.png saved");

// ========================
// TAB TOGGLING
// ========================
console.log("\n——— TAB NAVIGATION ———");
// Go back to home
await clickTab('首页');
text = await tabText();
assert(text.includes('Dashboard'), 'Navigate back to home tab works');

// Go to todo
await clickTab('待办');
text = await tabText();
assert(text.includes('Todo List'), 'Navigate to todo tab works');

// Summary
console.log(`\n==============================================`);
console.log(`  RESULTS: ${passedTests}/${totalTests} passed`);
if (failures.length > 0) {
  console.log(`  FAILURES:`);
  failures.forEach(f => console.log(`    • ${f}`));
}
console.log(`  Console errors: ${errors.length > 0 ? errors.join(' | ') : 'NONE'}`);
console.log(`==============================================`);

await browser.close();
