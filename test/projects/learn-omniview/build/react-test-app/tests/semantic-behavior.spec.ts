import { test, expect, Page } from '@playwright/test';

// v4 review H1/H2: DSL 顺序语义（x = v; use x → 读新值）在 React 生成
// 代码中被陈旧闭包破坏；只写信号发射悬空 setter。断言用户可见结果与
// 页面错误（既有套件断言重渲染显示，结构性看不见顺序语句）。
//
// 本套件为 RED 测试（Task 0.2）：修复在 Phase 2（Task 2.1/2.2），
// 修复落地前 5 个测试必须失败——失败断言即 H1/H2 缺陷的浏览器行为证据。
//
// 组件 → 标签映射（生成 DOM grep 证据，修正 brief 占位值）：
//   App.tsx 九层三元: overview/ui/motion/design/state/language/ref/tokens/else→compiler
//   Tab 1 (🎨 UI)     pages/UiComponents.tsx  L79 DataTable / L108 FormControlsDemo
//   Tab 5 (🔤 Language) pages/LanguagePage.tsx L119 AdvancedPseudoDemo / L148 EmitKeywordDemo
// brief 占位 TAB_UI=1/TAB_STATE=4/TAB_COMPILER=7 与生成 DOM 不符（State/Compiler
// 标签页无上述 demo），已按下述常量修正。
//
// 缺陷宿主（生成代码 + DSL 源证据）：
//   H1 写后读陈旧闭包:
//     FormControlsDemo.omni:22-23 `agreed = !agreed; statusText = if agreed {...}`
//       → handleAgree 读闭包旧值: setStatusText(agreed ? "Agreed ✓" : "Not agreed")
//       勾选后复选框/Status 显示新值而 statusText 仍 "Not agreed"
//     FormControlsDemo.omni:35-36 `linkClicked = linkClicked + 1; statusText = ...`
//       → 首次点击显示 "Link clicked 0 times"（off-by-one）
//     EmitKeywordDemo.omni:23-25 `count = count + 1; emit onEmitClick("emit #" + count)`
//       → 首次 emit 载荷 "emit #0"（跨组件契约 off-by-one）
//     DataTable.omni:39 `sortAsc = !sortAsc; rows = array_sort(rows, |a, b| if sortAsc ...)`
//       → toggleSort 读旧方向排序: 标签翻转 ASC→DESC 而行序不变
//   H2 只写信号悬空 setter:
//     AdvancedPseudoDemo.omni:6-10 `signal variant`（只写不读）→ setVariant 无绑定
//       → Dark Variant 点击 ReferenceError: setVariant is not defined
//
// 选择器适配注记：brief 基线含引号的 "emit #1" 断言目标在生成 DOM 中为
// 无引号文本（{item} 直接渲染字符串值）；链接定位器 brief 用 /link/i 但
// 生成 <a> 文本为 "Click me (N)"（title 属性不被 hasText 匹配）——均按
// 生成 DOM 修正，断言目标不变。

const BASE = 'http://localhost:5173';
const TABS = ['📋 Overview', '🎨 UI', '🎬 Motion', '💅 Design', '⚡ State', '🔤 Language', '🔗 Ref', '⚙️ Compiler', '🔮 Tokens'];

const TAB_UI = 1;        // 🎨 UI —— FormControlsDemo（表单控件）+ DataTable（排序）
const TAB_LANGUAGE = 5;  // 🔤 Language —— AdvancedPseudoDemo（伪类 demo）+ EmitKeywordDemo（emit demo）

async function switchToTab(page: Page, index: number) {
  await page.goto(BASE, { timeout: 8000 });
  await page.waitForSelector('button', { timeout: 8000 });
  await page.locator(`button:has-text("${TABS[index]}")`).first().click();
  await page.waitForTimeout(500);
}

async function expectNoErrors(page: Page, label: string, fn: () => Promise<void>) {
  const errors: string[] = [];
  const onErr = (m: any) => { if (m.type() === 'error') errors.push(m.text()); };
  page.on('pageerror', (e) => errors.push('PAGEERROR: ' + e.message));
  page.on('console', onErr);
  try {
    await fn();
    await page.waitForTimeout(300);
  } finally {
    page.removeListener('pageerror', () => {});
    page.removeListener('console', onErr);
  }
  const unexpected = errors.filter(e => !e.includes('favicon') && !e.includes('frame-ancestors'));
  expect(unexpected, `${label} console/page errors`).toEqual([]);
}

test('checkbox 勾选后状态文字与显示一致（H1 陈旧闭包）', async ({ page }) => {
  await switchToTab(page, TAB_UI);
  // 勾选第一个 checkbox 后，DSL 源 `agreed = !agreed; statusText = if agreed {...}`
  // 期望状态文字显示新值（与相邻显示一致）——断言文本含 "Agreed" 而非 "Not agreed"
  await page.locator('input[type="checkbox"]').first().check({ force: true });
  await expect(page.getByText(/Agreed|Not agreed/).first()).toContainText('Agreed');
});

test('链接计数器首次点击报 1（H1 off-by-one）', async ({ page }) => {
  await switchToTab(page, TAB_UI);
  // 生成 <a> 文本为 "Click me (N)"（brief 的 /link/i 只匹配 title 属性，hasText 不覆盖）——按 DOM 修正
  await page.locator('a', { hasText: /click me/i }).first().click();
  await expect(page.getByText(/clicked/i).first()).toContainText('1');
});

test('emit 载荷无 off-by-one（H1 跨组件契约）', async ({ page }) => {
  await switchToTab(page, TAB_LANGUAGE);
  // 精确按钮名：Language 标签页上 AdvancedTypesDemo 的 FnFormatter 还有
  // 'emit onSelect("item #N")' 按钮且 DOM 序在前——/emit/i 的 first() 会点错
  await page.getByRole('button', { name: 'emit onEmitClick' }).click();
  // 生成 DOM 中载荷为无引号字符串值（{item} 直接渲染）——brief 的 "emit #1" 引号按 DOM 去除
  await expect(page.getByText(/emit #/).first()).toContainText('emit #1');
});

test('排序方向与标签一致（H1 toggleSort）', async ({ page }) => {
  await switchToTab(page, TAB_UI);
  // 数据无关断言：点击前捕获全部行值 → 点击 → 标签翻转时顺序必须翻转
  // 定位器限定 DataTable 卡片：UI 标签页仅 DataTable 含 <table>，但卡片内
  // 定位保证未来页面增表时不误捕；Sort 标签读 Controls 内 info-text <p>
  // （FilterBar 另有 SortASC/DESC 按钮，位于 DataTable 之后，不误伤 first()）
  const tableCard = page.locator('div.Card', { has: page.getByText('DataTable Demo') });
  await expect(tableCard.getByText('Alpha').first()).toBeVisible({ timeout: 8000 }); // 等 useEffect 灌入行
  const before = await tableCard.locator('tbody tr td:first-child').allTextContents();
  const labelBefore = (await tableCard.locator('p', { hasText: /Sort:/ }).first().textContent()) ?? '';
  await page.getByRole('button', { name: /sort/i }).first().click();
  const after = await tableCard.locator('tbody tr td:first-child').allTextContents();
  const labelAfter = (await tableCard.locator('p', { hasText: /Sort:/ }).first().textContent()) ?? '';
  if (labelAfter !== labelBefore) {
    expect(after).toEqual([...before].reverse());
  }
});

test('只写信号点击不崩溃（H2 悬空 setter）', async ({ page }) => {
  await switchToTab(page, TAB_LANGUAGE);
  await expectNoErrors(page, 'write-only signal click', async () => {
    await page.getByRole('button', { name: /dark variant/i }).click();
    await page.waitForTimeout(300);
  });
});
