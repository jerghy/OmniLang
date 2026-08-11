import { test, expect, Page } from '@playwright/test';

// ════════════════════════════════════════════════════════════════
// Task 7.6 — 行为对等基准（design P5 行为层 / P7 产品假说数据起点）：
// 「同一 .omni 源 → 三目标行为一致」的最小行为对等断言集。
//
// 断言结构：行为级（DOM 节点身份 / 动画播放状态 / 派生状态变化），
// 非字符串快照。5 场景 × React 目标映射（Flutter 对应集见
// omnilang/tests/flutter_widget_tests/smoke/，README 冒烟集状态表）：
//
//   ① 主题切换（点卡片 → data-theme 变化 + CSS 覆盖实际生效）
//      → theme-wiring.spec.ts（已有断言，盘点为覆盖项）
//   ② bind:value 输入回写（键入 → 信号写入 → 派生 UI 状态变化）
//      → omniview-react.spec.ts test 38（已有断言：FilterBar 5/5 → 1/5）
//   ③ @for 排序重排 key 稳定（重排后 DOM 节点身份保持 = 键控复用）
//      → 本文件 test B3（补齐）
//   ④ @if 切换保留状态（3.9 IndexedStack 形态）
//      → React 缺口，test B4（skip + blocker）——Flutter 侧
//        page_switch_test.dart 已覆盖
//   ⑤ motion @enter 播放（挂载即播 → WAAPI 运行态）
//      → 本文件 test B5（补齐）
// ════════════════════════════════════════════════════════════════

const BASE = 'http://localhost:5173';
const TABS = ['📋 Overview', '🎨 UI', '🎬 Motion', '💅 Design', '⚡ State', '🔤 Language', '🔗 Ref', '⚙️ Compiler', '🔮 Tokens'];

async function switchToTab(page: Page, index: number) {
  await page.goto(BASE, { timeout: 8000 });
  await page.waitForSelector('button', { timeout: 8000 });
  await page.locator(`button:has-text("${TABS[index]}")`).first().click();
  await page.waitForTimeout(500);
}

test.describe('Behavioral parity — 5 scenarios × React', () => {
  test('B3 - @for reorder keeps DOM node identity (keyed reconciliation)', async ({ page }) => {
    await switchToTab(page, 5); // 🔤 Language — ForDirectiveKeyDemo
    await expect(page.getByText(/@for\s*key=/).first()).toBeVisible({ timeout: 8000 });

    // Add two items → 5 seeded + added items (#1..#5).
    await page.getByRole('button', { name: 'Add Item' }).first().click();
    await page.waitForTimeout(150);
    await page.getByRole('button', { name: 'Add Item' }).first().click();
    await page.waitForTimeout(200);
    await expect(page.getByText(/Total:\s*5/).first()).toBeVisible({ timeout: 8000 });

    // Key-stability evidence: tag every current <li> DOM node with an
    // identity marker. If React recreated nodes on reorder (unkeyed), the
    // markers vanish; if it matched by index, nodes stay in place. Keyed
    // reconciliation MOVES the nodes — the previously-last node must be
    // first after "Move Last to Front", marker intact.
    await page.locator('li[class*="list-item"]').evaluateAll((els) => {
      els.forEach((el, i) => {
        (el as HTMLElement).dataset.omniKey = String(i);
      });
    });

    await page.getByRole('button', { name: 'Move Last to Front' }).first().click();
    await page.waitForTimeout(200);

    // DOM order of identity markers: [4,0,1,2,3] — the #5 node moved to the
    // front WITH its identity (its dataset survived the reorder).
    const keys = await page
      .locator('li[class*="list-item"]')
      .evaluateAll((els) => els.map((el) => (el as HTMLElement).dataset.omniKey ?? 'none'));
    expect(keys, 'reorder must reuse the keyed DOM nodes, not recreate them').toEqual([
      '4',
      '0',
      '1',
      '2',
      '3',
    ]);

    // The moved node still renders its original content (#5 — Fruit-5).
    const first = (await page.locator('li[class*="list-item"]').first().textContent()) ?? '';
    expect(first.replace(/\s+/g, ' ').trim()).toMatch(/#\s*5\s*—\s*Fruit-5/);
  });

  test('B5 - motion @enter plays on mount (WAAPI running)', async ({ page }) => {
    // Entering the Motion tab mounts KeyframeAnimationDemo (8 triggers) +
    // AnimPropertiesDemo (17 @enter properties) — the @enter elements'
    // mount effects call el.animate() immediately, so right after the
    // switch the document must contain RUNNING WAAPI animations (the
    // longest @enter is 2000ms — well past the switchToTab settle wait).
    await switchToTab(page, 2); // 🎬 Motion
    await expect(page.getByText('Keyframe Animation Demo').first()).toBeVisible({ timeout: 8000 });

    await expect
      .poll(
        async () =>
          page.evaluate(
            () => document.getAnimations().filter((a) => a.playState === 'running').length,
          ),
        { timeout: 3000, intervals: [100] },
      )
      .toBeGreaterThan(0);
  });

  test.skip('B4 - @if switch preserves component-local state — KNOWN PARITY GAP (React)', async () => {
    // KNOWN GAP — registered as a blocker (task-70 report). test.skip: there
    // is no React surface to assert yet, and asserting would FAIL.
    //
    // Evidence:
    //  1. React codegen renders component-root @if as a ternary
    //     (components/PageSwitcherDemo.tsx):
    //       page === "home" ? <MemoizedHomePage/> : <MemoizedAboutPage/>
    //     — the outgoing page UNMOUNTS and its local state
    //     (HomePage `const [draft, setDraft] = useState("")`) is LOST.
    //  2. Flutter emits IndexedStack for the same source (3.9, task 3.9) —
    //     pages stay mounted and page_switch_test.dart asserts the typed
    //     draft survives the switch: the targets DIVERGE here.
    //
    // 批次 2 迁移注记（task-10）：PageSwitcherDemo 已迁移为 @route 路由形态
    // （HomePage @route "/home" / AboutPage @route "/about" + navigate()），
    // demo 层面不再有 @if 互斥——本 gap 作为一般性编译器能力差距登记
    // （spec §6 边界 7：页面保挂载仅对 @layout ShellRoute 生效，React 端
    // 切页卸载为预期行为，与 Flutter GoRouter push/pop 对等）。
    // Un-skip contract 不变：React codegen 获得状态保持条件渲染后恢复断言。
  });
});

// ════════════════════════════════════════════════════════════════
// Task 7.10 — 路由断言集（批次 2，spec §7 断言 1-10 × React 目标）
// React 端：createBrowserRouter → URL 为路径形式（/todos/42，无 hash）。
// 直接访问由 vite dev history fallback 接管 → React Router 渲染。
// 守卫/懒加载/嵌套布局/空路由兜底行为逐条记录（断言 10 为行为记录，
// 不设强断言——W2 已警告，行为由目标端决定）。
// ════════════════════════════════════════════════════════════════

test.describe('Routing — 10 assertions × React (BrowserRouter)', () => {
  test('R1 - default route "/" renders Main (Overview landing)', async ({ page }) => {
    await page.goto(BASE, { timeout: 8000 });
    await expect(page.getByText(/OmniLang Capability Showcase/).first()).toBeVisible({ timeout: 8000 });
    // @route "/" 注册 → Main 直接渲染；URL 保持根路径
    await expect(page).toHaveURL(`${BASE}/`);
  });

  test('R2 - navigate() click switches route and syncs URL', async ({ page }) => {
    await page.goto(`${BASE}/nav`, { timeout: 8000 });
    await expect(page.getByText('导航中心').first()).toBeVisible({ timeout: 8000 });

    await page.getByRole('button', { name: '待办列表（query 过滤）' }).click();
    await expect(page).toHaveURL(`${BASE}/todos`);
    await expect(page.getByText('当前过滤: all').first()).toBeVisible({ timeout: 8000 });
  });

  test('R3 - direct access /todos/:id injects path param', async ({ page }) => {
    await page.goto(`${BASE}/todos/42`, { timeout: 8000 });
    await expect(page.getByText('待办详情').first()).toBeVisible({ timeout: 8000 });
    await expect(page.getByText('待办 #42').first()).toBeVisible({ timeout: 8000 });
  });

  test('R4 - direct access ?filter=all injects query param + click updates URL query', async ({ page }) => {
    await page.goto(`${BASE}/todos?filter=all`, { timeout: 8000 });
    await expect(page.getByText('当前过滤: all').first()).toBeVisible({ timeout: 8000 });

    await page.getByRole('button', { name: '已完成' }).click();
    await expect(page).toHaveURL(/\/todos\?filter=done/);
    await expect(page.getByText('当前过滤: done').first()).toBeVisible({ timeout: 8000 });
  });

  test('R5 - back() button returns to previous route', async ({ page }) => {
    await page.goto(`${BASE}/nav`, { timeout: 8000 });
    await expect(page.getByText('导航中心').first()).toBeVisible({ timeout: 8000 });

    await page.getByRole('button', { name: '待办列表（query 过滤）' }).click();
    await expect(page.getByText('当前过滤: all').first()).toBeVisible({ timeout: 8000 });

    await page.getByRole('button', { name: '返回' }).first().click();
    await expect(page).toHaveURL(`${BASE}/nav`);
    await expect(page.getByText('导航中心').first()).toBeVisible({ timeout: 8000 });
  });

  test('R6 - browser back returns to previous route', async ({ page }) => {
    await page.goto(`${BASE}/nav`, { timeout: 8000 });
    await expect(page.getByText('导航中心').first()).toBeVisible({ timeout: 8000 });

    await page.getByRole('button', { name: '待办列表（query 过滤）' }).click();
    await expect(page.getByText('当前过滤: all').first()).toBeVisible({ timeout: 8000 });

    await page.goBack();
    await expect(page).toHaveURL(`${BASE}/nav`);
    await expect(page.getByText('导航中心').first()).toBeVisible({ timeout: 8000 });
  });

  test('R7 - guard redirects unauthorized access to /login', async ({ page }) => {
    // V14 注记：@route 路径与 @layout 前缀精确相等冲突（硬错误）——守卫演示
    // 落在 /admin/console（/admin 布局前缀下独立路径），而非 /admin 本身。
    await page.goto(`${BASE}/admin/console`, { timeout: 8000 });
    await expect(page).toHaveURL(`${BASE}/login`);
    await expect(page.getByText(/未授权访问管理页/).first()).toBeVisible({ timeout: 8000 });
    // 被守卫的内容不得渲染
    await expect(page.getByText(/控制台（需授权）/)).toHaveCount(0);
  });

  test('R8 - nested @layout ShellRoute renders outlet inside layout', async ({ page }) => {
    await page.goto(`${BASE}/admin/users`, { timeout: 8000 });
    await expect(page.getByText('Admin 控制台').first()).toBeVisible({ timeout: 8000 });
    await expect(page.getByText('用户管理').first()).toBeVisible({ timeout: 8000 });
    await expect(page.getByText(/AdminLayout 外壳内嵌页面/).first()).toBeVisible({ timeout: 8000 });
  });

  test('R9 - lazy: true route loads via Suspense', async ({ page }) => {
    // React 端：React.lazy + <Suspense fallback>。本地 vite 模块加载极快，
    // fallback 可能一闪而过——轮询最终内容渲染（Suspense 完成后挂载）。
    await page.goto(`${BASE}/heavy`, { timeout: 8000 });
    await expect(page.getByText('懒加载页面').first()).toBeVisible({ timeout: 8000 });
    await expect(page.getByText(/React 路由级 lazy \+ Suspense fallback/).first()).toBeVisible({ timeout: 8000 });
  });

  test('R10 - unregistered path renders React Router default 404 (behavior recorded)', async ({ page }) => {
    // 行为记录（非强断言）：createBrowserRouter 无匹配路径 → React Router
    // 抛出 404 并渲染默认错误页（dev 模式 "Unexpected Application Error! /
    // 404 Not Found"），无崩溃、无未捕获错误。与 OmniView 端（兜底回退
    // __omni_routes[0] = "/"）存在目标端差异（W2 已警告，行为由目标端决定）。
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(String(err)));
    await page.goto(`${BASE}/does-not-exist`, { timeout: 8000 });
    await expect(page.getByText(/404 Not Found/).first()).toBeVisible({ timeout: 8000 });
    expect(errors, 'unregistered path must not throw').toEqual([]);
  });
});
