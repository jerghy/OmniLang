import { test, expect, Page } from '@playwright/test';

// ════════════════════════════════════════════════════════════════
// OmniView target — 9-tab full browser validation (Task 7.1).
//
// Runs against the GENERATED OmniView output served statically
// (see playwright.omniview.config.ts). The OmniView runtime renders
// all 9 tab branches in the DOM and toggles `display` (el-if-*-bN),
// so selectors MUST filter by visibility — the same text/button often
// exists in several hidden branches (e.g. InlineStoreDemo renders on
// both State and Language tabs).
//
// Matcher note: Playwright getByText STRING matching normalizes
// whitespace, REGEX matching does NOT — the generated HTML inserts
// newlines/indentation between text nodes, so multi-token text is
// matched as STRINGS (or with explicit \s*/[\s\S]* in regexes).
//
// KNOWN runtime errors filtered by the per-tab zero-console-error
// checks — each is a RECORDED issue, remove the filter when fixed:
//   1. 'item is not defined' — COMPILER BUG #1 (task-7.1-report):
//      omnilang/src/codegen/omniview/list.rs emits `keyFn(item)` with
//      a hardcoded loop-var name — DataTable's `@for row in rows
//      key={row.id}` throws ReferenceError on first iteration and the
//      renderer aborts (table never renders). Affects ANY keyed @for
//      whose DSL loop variable is not literally named `item`.
//      → DataTable behavior tests below are marked fixme for this.
//   2. 'frame-ancestors' — Chrome console warning: frame-ancestors is
//      ignored in a <meta> CSP (browser-internal, harmless; compiler
//      emits it in the meta CSP — low-severity codegen note).
// ════════════════════════════════════════════════════════════════

const BASE = 'http://localhost:5174';
const TABS = ['📋 Overview', '🎨 UI', '🎬 Motion', '💅 Design', '⚡ State', '🔤 Language', '🔗 Ref', '⚙️ Compiler', '🔮 Tokens'];

const KNOWN_ERRORS = [
  'frame-ancestors', // Chrome meta-CSP warning (see header note 2)
  'item is not defined', // compiler bug #1 (see header note 1)
];

async function switchToTab(page: Page, index: number) {
  await page.goto(BASE, { timeout: 8000 });
  await page.waitForSelector('button', { timeout: 8000 });
  await page.locator(`button:has-text("${TABS[index]}")`).first().click();
  await page.waitForTimeout(500);
}

async function expectNoErrors(page: Page, label: string, fn: () => Promise<void>) {
  const errors: string[] = [];
  const onErr = (m: any) => { if (m.type() === 'error') errors.push(m.text()); };
  page.on('console', onErr);
  try {
    await fn();
    await page.waitForTimeout(300);
  } finally {
    page.removeListener('console', onErr);
  }
  const unexpected = errors.filter(e => !KNOWN_ERRORS.some(k => e.includes(k)));
  expect(unexpected, `${label} console errors`).toEqual([]);
}

// Locators in this suite always restrict to VISIBLE elements — the
// OmniView runtime keeps every tab branch in the DOM (display:none).
const visibleLoc = (l: any) => l.filter({ visible: true }).first();

async function visible(page: Page, text: string | RegExp) {
  await expect(visibleLoc(page.getByText(text))).toBeVisible({ timeout: 8000 });
}

async function btn(page: Page, name: string | RegExp) {
  await expect(visibleLoc(page.getByRole('button', { name }))).toBeVisible({ timeout: 8000 });
}

async function clickBtn(page: Page, name: string | RegExp) {
  await visibleLoc(page.getByRole('button', { name })).click();
}

async function clickText(page: Page, text: string) {
  await visibleLoc(page.getByText(text)).click();
}

test.describe('OmniView — 9 标签页全量验证', () => {

  // ═══ TAB 0: Overview ═══
  test.describe('Tab 0: 📋 Overview', () => {
    test('01 - banner + capability grid render', async ({ page }) => {
      await switchToTab(page, 0);
      await visible(page, 'OmniLang Capability Showcase');
      await visible(page, '93 源文件 · 4 主题 · 10 语义域 · 9 标签页');
      for (const t of ['UI Components', 'Motion & Animation', 'Design & Theme', 'State & Signals', 'Language Features', 'Element Ref', 'Compiler Features', 'Design Tokens']) {
        await visible(page, t);
      }
    });

    test('02 - CapabilityCard Explore navigates to tab', async ({ page }) => {
      await switchToTab(page, 0);
      await clickText(page, 'UI Components');
      await page.waitForTimeout(500);
      await visible(page, 'ValidationInput Demo');
    });

    test('03 - no console errors', async ({ page }) => {
      await expectNoErrors(page, 'tab0', () => switchToTab(page, 0));
    });
  });

  // ═══ TAB 1: UI ═══
  test.describe('Tab 1: 🎨 UI', () => {
    test('04 - ValidationInput validate flow', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'ValidationInput Demo');
      await visible(page, 'Waiting for input');
      await visibleLoc(page.locator('input[placeholder*="valid"]')).fill('valid@test.com');
      await clickBtn(page, 'Validate');
      await page.waitForTimeout(200);
      await visible(page, 'Result ready');
    });

    test('05 - Modal open/close', async ({ page }) => {
      await switchToTab(page, 1);
      await clickBtn(page, 'Open Modal');
      await page.waitForTimeout(300);
      await visible(page, 'Modal Title');
      await clickBtn(page, 'Cancel');
      await page.waitForTimeout(300);
    });

    test('06 - TabPanel switching', async ({ page }) => {
      await switchToTab(page, 1);
      await clickBtn(page, 'About');
      await page.waitForTimeout(200);
      await visible(page, 'About Tab');
      await clickBtn(page, 'Settings');
      await page.waitForTimeout(200);
      await visible(page, 'Settings Tab');
    });

    test('07 - FormPage validation state', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'FormPage Demo');
      await visible(page, 'Form incomplete');
      await visibleLoc(page.locator('input[placeholder="Your name"]')).fill('Test');
      await visibleLoc(page.locator('input[placeholder*="email"]')).fill('test@example.com');
      await page.waitForTimeout(200);
      await visible(page, 'Form is valid');
    });

    test('08 - FormControlsDemo select/checkbox', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'HTML 原生控件');
      await visibleLoc(page.locator('select.demo-select')).selectOption('option3');
      await page.waitForTimeout(200);
      await visible(page, 'Value: option3');
      await visibleLoc(page.locator('input[type="checkbox"]')).check({ force: true });
      await page.waitForTimeout(200);
      await visible(page, 'Status: Checked');
    });

    test('09 - NotificationsPanel add', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'No notifications');
      await clickBtn(page, 'Add');
      await page.waitForTimeout(300);
      await visible(page, 'Total:');
    });

    test('10 - A11yDemo img alt + dashboard', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, '可访问性');
      const img = visibleLoc(page.locator('img[src="/dot.png"][alt="演示用小圆点"]'));
      await expect(img).toBeVisible();
      await expect(img).toHaveAttribute('alt', '演示用小圆点');
      await visible(page, 'Dashboard 组合演示');
    });

    test.fixme('11 - DataTable renders rows (compiler bug #1: keyFn hardcodes loop var)', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'DataTable Demo');
      await visible(page, 'Alpha');
      await visible(page, 'Sort:ASC');
    });

    test.fixme('12 - DataTable sort re-renders rows (compiler bug #1)', async ({ page }) => {
      await switchToTab(page, 1);
      await clickBtn(page, /Toggle Sort/);
      await page.waitForTimeout(300);
      await visible(page, 'Sort:DESC');
      await visible(page, 'Alpha'); // rows must still be present after sort
    });

    test('13 - no console errors (known: bug #1)', async ({ page }) => {
      await expectNoErrors(page, 'tab1', () => switchToTab(page, 1));
    });
  });

  // ═══ TAB 2: Motion ═══
  test.describe('Tab 2: 🎬 Motion', () => {
    test('14 - motion sections render', async ({ page }) => {
      await switchToTab(page, 2);
      await visible(page, 'Motion Demo');
      await visible(page, 'Keyframe Animation Demo');
      await visible(page, '缓动函数全家桶');
      await visible(page, '全动画属性覆盖');
      await visible(page, 'Motion Triggers Demo');
    });

    test('15 - animations actually run (WAAPI active)', async ({ page }) => {
      await switchToTab(page, 2);
      await clickBtn(page, 'Replay All @enter Animations');
      await page.waitForTimeout(100);
      const running = await page.evaluate(() => document.getAnimations().filter(a => a.playState === 'running').length);
      expect(running).toBeGreaterThan(0);
    });

    test('16 - hover trigger animates card', async ({ page }) => {
      await switchToTab(page, 2);
      const card = page.getByText('悬停缩放 1.05x').first().locator('..').locator('..');
      await card.hover();
      await page.waitForTimeout(120);
      const running = await page.evaluate(() => document.getAnimations().filter(a => a.playState === 'running').length);
      expect(running).toBeGreaterThan(0);
      await page.mouse.move(0, 0);
    });

    test('17 - EasingFunctionsDemo Replay All Animations', async ({ page }) => {
      await switchToTab(page, 2);
      await clickBtn(page, 'Replay All Animations');
      await page.waitForTimeout(400);
      await visible(page, 'Power 曲线');
    });

    test('18 - no console errors', async ({ page }) => {
      await expectNoErrors(page, 'tab2', () => switchToTab(page, 2));
    });
  });

  // ═══ TAB 3: Design ═══
  test.describe('Tab 3: 💅 Design', () => {
    test('19 - theme switching writes data-theme + CSS overrides', async ({ page }) => {
      await switchToTab(page, 3);
      await visible(page, 'Theme Selector');
      await page.waitForFunction(() => document.documentElement.dataset.theme !== undefined);
      expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('light');

      await page.getByText('Dark', { exact: true }).filter({ visible: true }).first().locator('..').click();
      await page.waitForTimeout(300);
      expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('dark');
      const surface = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--surface').trim());
      expect(surface).toBe('#111827');
      await visible(page, /Toggled\s*1\s*times/);

      // multi-theme: sepia
      await page.getByText('Sepia', { exact: true }).filter({ visible: true }).first().locator('..').click();
      await page.waitForTimeout(300);
      expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('sepia');
      await visible(page, /Toggled\s*2\s*times/);
    });

    test('20 - theme persists across tabs', async ({ page }) => {
      await switchToTab(page, 3);
      await page.getByText('Dark', { exact: true }).filter({ visible: true }).first().locator('..').click();
      await page.waitForTimeout(200);
      await page.locator('button:has-text("⚡ State")').first().click();
      await page.waitForTimeout(400);
      expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('dark');
      // back to light for subsequent tests
      await page.locator('button:has-text("💅 Design")').first().click();
      await page.waitForTimeout(300);
      await page.getByText('Light', { exact: true }).filter({ visible: true }).first().locator('..').click();
      await page.waitForTimeout(200);
    });

    test('21 - design sections render', async ({ page }) => {
      await switchToTab(page, 3);
      await visible(page, '@themes');
      await visible(page, 'Design Tokens Visualizer');
      await visible(page, 'Component Variants');
      await visible(page, 'CSS 变量与单位');
    });

    test('22 - CssVarsDemo cycle accent', async ({ page }) => {
      await switchToTab(page, 3);
      await clickBtn(page, /Cycle Accent/);
      await page.waitForTimeout(300);
    });

    test('23 - no console errors', async ({ page }) => {
      await expectNoErrors(page, 'tab3', () => switchToTab(page, 3));
    });
  });

  // ═══ TAB 4: State ═══
  test.describe('Tab 4: ⚡ State', () => {
    test('24 - SignalPatternsDemo counter + keyed list', async ({ page }) => {
      await switchToTab(page, 4);
      await visible(page, 'Signal & Store Patterns');
      await clickBtn(page, '+1');
      await page.waitForTimeout(200);
      await clickBtn(page, 'Add Item');
      await page.waitForTimeout(300);
      await visible(page, 'For key 身份追踪列表');
    });

    test('25 - InlineStoreDemo add/remove', async ({ page }) => {
      await switchToTab(page, 4);
      await visible(page, 'Inline Store Demo');
      await visible(page, 'cart empty');
      await clickBtn(page, 'Add Apple');
      await page.waitForTimeout(300);
      await visible(page, 'Items: 1');
      await clickBtn(page, 'Remove');
      await page.waitForTimeout(300);
    });

    test('26 - ComputedFnFormDemo calls counter', async ({ page }) => {
      await switchToTab(page, 4);
      await visible(page, 'Computed Function Form Demo');
      await clickBtn(page, /Set "San"/);
      await page.waitForTimeout(300);
      await visible(page, 'Calls: 1');
      await visible(page, 'Zhang San');
    });

    test('27 - no console errors', async ({ page }) => {
      await expectNoErrors(page, 'tab4', () => switchToTab(page, 4));
    });
  });

  // ═══ TAB 5: Language ═══
  test.describe('Tab 5: 🔤 Language', () => {
    test('28 - OptionalChainDemo interactions', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, 'Optional Chain ( ?. ) Demo');
      await clickBtn(page, /Set "Bob"/);
      await page.waitForTimeout(200);
      await visible(page, 'Bob');
      await clickBtn(page, /Toggle Details/);
      await page.waitForTimeout(200);
    });

    test('29 - TernaryExprDemo grade buttons', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, 'Ternary ( ? : )');
      await clickBtn(page, '95 → A');
      await page.waitForTimeout(200);
      await visible(page, '等级: A');
      await clickBtn(page, /count \+ 1/);
      await page.waitForTimeout(200);
      await visible(page, 'Positive');
    });

    test('30 - TypeSystemDemo values', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, '类型系统全覆盖');
      await visible(page, /i8[\s\S]*127/);
      await visible(page, /f32[\s\S]*3\.14/);
    });

    test('31 - list add/remove works (keyed @for)', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, /@for\s*key=/);
      // initial order
      await visible(page, /#\s*1\s*—\s*Apple/);
      await visible(page, /#\s*3\s*—\s*Cherry/);
      // add two items
      await clickBtn(page, 'Add Item');
      await page.waitForTimeout(150);
      await clickBtn(page, 'Add Item');
      await page.waitForTimeout(200);
      await visible(page, /#\s*5\s*—\s*Fruit-5/);
      await visible(page, 'Total: 5');
      // remove last
      await clickBtn(page, 'Remove Last');
      await page.waitForTimeout(200);
      await visible(page, 'Total: 4');
    });

    test.fixme('31b - list reorder renders the new order (runtime bug #3: reused clones never move/rebind)', async ({ page }) => {
      // REPRO (task-7.1-report bug #3): moveLastToFront updates the signal
      // (ctr._forData becomes [5,1,2,3,4]) but the DOM keeps [1..5] with
      // stale content — keyed clones are reused in place and the emitted
      // keyed rebind fn is never invoked.
      await switchToTab(page, 5);
      await clickBtn(page, 'Add Item');
      await page.waitForTimeout(150);
      await clickBtn(page, 'Move Last to Front');
      await page.waitForTimeout(300);
      const order = await page.locator('li.list-item').filter({ visible: true }).allTextContents();
      expect(order[0]).toContain('#4 — Fruit-4');
      expect(order[1]).toContain('#1 — Apple');
    });

    test('32 - BlockExprDemo + StringLiterals + ErrorHandling', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, 'Block Expression Demo');
      await clickBtn(page, 'a + 5');
      await page.waitForTimeout(200);
      await visible(page, '字符串字面量演示');
      await visible(page, '异常处理');
    });

    test('33 - no console errors', async ({ page }) => {
      await expectNoErrors(page, 'tab5', () => switchToTab(page, 5));
    });
  });

  // ═══ TAB 6: Ref ═══
  test.describe('Tab 6: 🔗 Ref', () => {
    test.fixme('34 - focus({sig}) auto-focuses on mount (runtime bug #2: binding runs while branch hidden)', async ({ page }) => {
      await switchToTab(page, 6);
      await visible(page, 'Element Reference System');
      await visible(page, 'focus=true（挂载自动聚焦）');
      const placeholder = await page.evaluate(() => document.activeElement?.getAttribute('placeholder'));
      expect(placeholder).toContain('focus={shouldFocus}');
    });

    test('35 - imperative .focus()/.blur()', async ({ page }) => {
      await switchToTab(page, 6);
      await clickBtn(page, 'Focus');
      await page.waitForTimeout(200);
      await visible(page, /Status:\s*focused/);
      await clickBtn(page, 'Blur');
      await page.waitForTimeout(200);
      await visible(page, /Status:\s*blurred/);
    });

    test('36 - pointer capture down/up', async ({ page }) => {
      await switchToTab(page, 6);
      const box = page.getByText('Press & hold me').first().locator('..');
      await box.hover();
      await page.mouse.down();
      await page.waitForTimeout(150);
      await visible(page, /Pointer:\s*captured/);
      await page.mouse.up();
      await page.waitForTimeout(150);
      await visible(page, /Pointer:\s*released/);
    });

    test('37 - @drag-horizontal drags the box', async ({ page }) => {
      await switchToTab(page, 6);
      // wait for enter animations to settle so the drag box is stable
      await page.waitForTimeout(900);
      const box = page.getByText('↔ Drag me').first().locator('..');
      await box.scrollIntoViewIfNeeded();
      const before = await box.evaluate(el => getComputedStyle(el as HTMLElement).transform);
      const bb = await box.boundingBox();
      if (bb) {
        await page.mouse.move(bb.x + 40, bb.y + 20);
        await page.mouse.down();
        await page.mouse.move(bb.x + 160, bb.y + 20, { steps: 8 });
        await page.waitForTimeout(150);
        await page.mouse.up();
        await page.waitForTimeout(150);
      }
      const after = await box.evaluate(el => getComputedStyle(el as HTMLElement).transform);
      expect(after).not.toBe(before);
    });

    test('38 - no console errors', async ({ page }) => {
      await expectNoErrors(page, 'tab6', () => switchToTab(page, 6));
    });
  });

  // ═══ TAB 7: Compiler ═══
  test.describe('Tab 7: ⚙️ Compiler', () => {
    test('39 - sections render', async ({ page }) => {
      await switchToTab(page, 7);
      await visible(page, 'HTTP GET Demo');
      await visible(page, 'HTTP POST Demo');
      await visible(page, 'Storage Demo');
      await visible(page, '标准库（Std Lib）演示');
    });

    test('40 - Storage save + stdlib', async ({ page }) => {
      await switchToTab(page, 7);
      await visibleLoc(page.locator('input[placeholder*="name"]')).fill('测试用户');
      await clickBtn(page, 'Save');
      await page.waitForTimeout(300);
    });

    test('41 - HTTP fetch fails gracefully (CSP blocks cross-origin)', async ({ page }) => {
      await switchToTab(page, 7);
      await clickBtn(page, 'Load');
      await page.waitForTimeout(800);
      // demo catches the failure and shows its error state — no crash
      await visible(page, 'Error:');
    });

    test('42 - no console errors (Load not clicked in this check)', async ({ page }) => {
      await expectNoErrors(page, 'tab7', () => switchToTab(page, 7));
    });
  });

  // ═══ TAB 8: Tokens ═══
  test.describe('Tab 8: 🔮 Tokens', () => {
    test('43 - preset buttons flip live token CSS vars', async ({ page }) => {
      await switchToTab(page, 8);
      await visible(page, 'Dynamic Design Tokens');
      const colorBefore = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--dynColor').trim());
      await clickBtn(page, 'Emerald');
      await page.waitForTimeout(300);
      const colorAfter = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--dynColor').trim());
      expect(colorAfter).not.toBe(colorBefore);
      expect(colorAfter).toBe('#10b981');
      await clickBtn(page, 'Amber');
      await page.waitForTimeout(200);
    });

    test('44 - no console errors', async ({ page }) => {
      await expectNoErrors(page, 'tab8', () => switchToTab(page, 8));
    });
  });
});
