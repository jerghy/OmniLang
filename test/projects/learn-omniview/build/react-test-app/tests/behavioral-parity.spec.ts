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
    //  3. PageSwitcherDemo is not mounted in the React app at all
    //     (no import in App.tsx / any page), so the divergence cannot even
    //     be exercised in-browser today.
    //
    // Un-skip contract (when the React codegen gains a state-preserving
    // conditional AND the demo is mounted): type into the Home draft →
    // Go to About → Back to Home → the draft text must still be present
    // (mirror of smoke/page_switch_test.dart).
  });
});
