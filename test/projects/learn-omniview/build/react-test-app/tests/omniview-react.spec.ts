import { test, expect, Page } from '@playwright/test';

// ════════════════════════════════════════════════════════════════
// React target — 9-tab browser validation (Task 7.1).
//
// Replaces the T1.6-era spec that targeted the old 10-tab showcase
// structure (首页/待办/基础交互/.../补充 — deleted in Task 7.0's
// refactor; that spec was structurally red against the new app).
// Same component sources as the OmniView suite — the React target
// renders only the ACTIVE tab (no hidden branches), so plain
// selectors suffice. React fragments adjacent text nodes and DROPS
// spaces around interpolations ({'Toggled'}{n}{' times'} renders
// "Toggled1 times"), so multi-token text is matched with \s* regexes.
//
// Tests marked fixme reference RECORDED React codegen bugs
// (task-7.1-report §5; un-fixme when the root cause lands):
//   A/B. `let mut` vars emitted as `const` (TS2588 class) — DataTable
//      getBonusTotal + ErrorHandlingDemo try_catch_finally_divide
//      throw "Assignment to constant variable" DURING RENDER → the
//      whole app unmounts on the UI / Language tabs.
//      FIXED (task 2.1): LetStmtIR now emits `let` unconditionally —
//      the tests below were un-fixme'd and guard this behavior until
//      Phase 5.1 threads the IR mut flag (restoring `const` for
//      immutable bindings).
//   C. Inline-store actions reference an undefined CartStore object
//      (TS2304 class — inline-store actions have no emission path).
//      FIXED (task 2.2): inline stores now hard-diagnose E_CODEGEN on
//      React (BUG-C interim, deferral #37) and InlineStoreDemo was
//      migrated to a top-level CartStore. Test 20 stays fixme for a
//      DIFFERENT recorded bug it now exposes: store GETTERS are frozen
//      after the first set() — Zustand's Object.assign merge flattens
//      the emitted accessor getters into stale data props (affects
//      TodoStore.totalCount too; verified in-browser). Getter emission
//      fix belongs to the IR wave (Phase 5.2 store emission).
//   D. `&` in static text is escaped to `&amp;` INSIDE JSX string
//      literals, rendering literally ("Signal &amp; Store Patterns").
//      FIXED (task 2.4): the string-literal emission path no longer
//      entity-escapes — `<`, `>`, `&` stay raw in JS strings (React
//      renders expression-container strings verbatim); entity escaping
//      remains only on the JSXText/HTML paths. Test 19 un-fixme'd.
//   E. Kebab-case style props ("border-radius") → React console errors
//      (TS2353 class).
//      FIXED (task 2.5): codegen camelCases style keys (font-size →
//      fontSize, -webkit-* → Webkit*) while keeping CSS custom
//      properties (--*) quoted verbatim. Tests 12 + 18 un-fixme'd.
//   F. Lowercase event handlers (onAnimationend / onTransitionend)
//      → React console errors.
//      FIXED (task 2.5): codegen maps the animation/transition family to
//      React's synthetic set (onAnimationEnd / onAnimationIteration /
//      onAnimationStart / onTransitionEnd / onTransitionRun /
//      onTransitionStart / onTransitionCancel). Tests 12 + 18 un-fixme'd.
//   G. @unmount cleanup closed over FIRST-render values — the emitted
//      `useEffect(() => () => {...}, [])` deps were hardcoded `[]`, so
//      Timer's "final count" logged 0 even after ticking (stale cleanup).
//      FIXED (task 2.6): cleanup reads now enter the deps array
//      (`}, [count]);` — React re-registers the cleanup on every count
//      change, so the unmount cleanup closes over the LATEST values).
//      Test 37 guards the final-count behavior.
// ════════════════════════════════════════════════════════════════

const BASE = 'http://localhost:5173';
const TABS = ['📋 Overview', '🎨 UI', '🎬 Motion', '💅 Design', '⚡ State', '🔤 Language', '🔗 Ref', '⚙️ Compiler', '🔮 Tokens'];

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

async function visible(page: Page, text: string | RegExp) {
  await expect(page.getByText(text).first()).toBeVisible({ timeout: 8000 });
}

async function clickBtn(page: Page, name: string | RegExp) {
  await page.getByRole('button', { name }).first().click();
}

test.describe('OmniView React — 9 标签页验证', () => {

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

    test('02 - CapabilityCard navigates to tab (React bug A: UI tab crashes the app)', async ({ page }) => {
      await switchToTab(page, 0);
      await page.getByText('UI Components').first().click();
      await page.waitForTimeout(400);
      await visible(page, 'ValidationInput Demo');
    });

    test('03 - no runtime errors', async ({ page }) => {
      await expectNoErrors(page, 'tab0', () => switchToTab(page, 0));
    });
  });

  // ═══ TAB 1: UI ═══
  test.describe('Tab 1: 🎨 UI', () => {
    test('04 - ValidationInput validate flow (React bug A: UI tab crashes the app)', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'ValidationInput Demo');
      await visible(page, 'Waiting for input');
      await page.locator('input[placeholder*="valid"]').first().fill('valid@test.com');
      await clickBtn(page, 'Validate');
      await page.waitForTimeout(200);
      await visible(page, 'Result ready');
    });

    test('05 - Modal open/close (React bug A)', async ({ page }) => {
      await switchToTab(page, 1);
      await clickBtn(page, 'Open Modal');
      await page.waitForTimeout(300);
      await visible(page, 'Modal Title');
      await clickBtn(page, 'Cancel');
      await page.waitForTimeout(300);
    });

    test('06 - TabPanel switching (React bug A)', async ({ page }) => {
      await switchToTab(page, 1);
      await clickBtn(page, 'About');
      await page.waitForTimeout(200);
      await visible(page, 'About Tab');
      await clickBtn(page, 'Settings');
      await page.waitForTimeout(200);
      await visible(page, 'Settings Tab');
    });

    test('07 - FormPage validation state (React bug A)', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'FormPage Demo');
      await visible(page, 'Form incomplete');
      await page.locator('input[placeholder="Your name"]').fill('Test');
      await page.locator('input[placeholder*="email"]').first().fill('test@example.com');
      await page.waitForTimeout(200);
      await visible(page, 'Form is valid');
    });

    test('08 - FormControlsDemo select/checkbox (React bug A)', async ({ page }) => {
      await switchToTab(page, 1);
      // EventsDemo renders an earlier <select> whose options carry no
      // value attributes — target the demo-select by its option value.
      await page.locator('select').filter({ has: page.locator('option[value="option3"]') }).selectOption('option3');
      await page.waitForTimeout(200);
      await visible(page, /Value:\s*option3/);
      await page.locator('input[type="checkbox"]').first().check({ force: true });
      await page.waitForTimeout(200);
      await visible(page, /Status:\s*Checked/);
    });

    test('09 - NotificationsPanel add (React bug A)', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'No notifications');
      await clickBtn(page, 'Add');
      await page.waitForTimeout(300);
      await visible(page, 'Total:');
    });

    test('10 - A11yDemo img alt (React bug A)', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, '可访问性');
      const img = page.locator('img[src="/dot.png"][alt="演示用小圆点"]').first();
      await expect(img).toBeVisible();
    });

    test('11 - DataTable rows + sort (React bug A: getBonusTotal const reassignment)', async ({ page }) => {
      await switchToTab(page, 1);
      await visible(page, 'DataTable Demo');
      await visible(page, 'Alpha');
      await clickBtn(page, /Toggle Sort/);
      await page.waitForTimeout(300);
      await visible(page, 'Sort:DESC');
      await visible(page, 'Alpha');
    });

    test('12 - no runtime errors (React bugs E+F: kebab style props + lowercase event handlers — fixed by task 2.5)', async ({ page }) => {
      await expectNoErrors(page, 'tab1', () => switchToTab(page, 1));
    });

    test('37 - Timer unmount logs the FINAL count (task 2.6: stale cleanup)', async ({ page }) => {
      // The @unmount cleanup used to close over the FIRST-render count
      // (hardcoded `}, []);`) — after Start + 3 Ticks the unmount log
      // still said "final count: 0". Cleanup reads now enter the deps
      // (`}, [count]);`), so the cleanup re-registers with the latest
      // count and the unmount log must show the real value.
      await switchToTab(page, 1);
      await visible(page, 'Timer Demo');
      const card = page.locator('div.Card', { has: page.getByText('Timer Demo') });
      await card.getByRole('button', { name: 'Start' }).click();
      await card.getByRole('button', { name: 'Tick' }).click();
      await card.getByRole('button', { name: 'Tick' }).click();
      await card.getByRole('button', { name: 'Tick' }).click();
      await page.waitForTimeout(200);
      const logs: string[] = [];
      const onLog = (m: any) => {
        const t = m.text();
        if (t.includes('Timer unmounted')) logs.push(t);
      };
      page.on('console', onLog);
      // Leave the UI tab in-app (no reload) — Timer unmounts
      await page.locator('button:has-text("📋 Overview")').first().click();
      await page.waitForTimeout(400);
      page.removeListener('console', onLog);
      // EXACT count: ticked exactly 3x → the unmount cleanup must log the
      // latest count (3). A stale closure logs 0; a cleanup that killed the
      // running gate (state write inside the cleanup, review Important-1)
      // would leave the count at 1 — both must fail this assertion.
      expect(
        logs.some((l) => /final count: 3$/.test(l)),
        `cleanup must log the LATEST count after exactly 3 ticks. Logs: ${JSON.stringify(logs)}`,
      ).toBe(true);
    });

    test('38 - FilterBar input filtering survives literal-signal hoisting (task 2.9: perf M3)', async ({ page }) => {
      // demoFrameworks is a literal-initialized never-mutated signal —
      // task 2.9 hoists it to a module-scope const and filters it OUT of
      // the filteredFrameworks useMemo deps (a constant is not reactive;
      // the deps become [searchText]). searchText stays in the deps, so
      // typing must still re-filter the computed (5/5 → 1/5 for "React")
      // — the hoist must NOT freeze the filter. React drops the space
      // after the colon, so the count text is "Filtered frameworks:5/5".
      await switchToTab(page, 1);
      await visible(page, /Filtered frameworks:\s*5\/5/);
      const filterCard = page.locator('div.Card', { has: page.getByText(/Filtered frameworks:/) });
      const filterInput = filterCard.locator('input[placeholder="Filter todos..."]');
      await filterInput.fill('React');
      await page.waitForTimeout(300);
      await visible(page, /Filtered frameworks:\s*1\/5/);
      // Clearing the filter (empty the input — same searchText change path
      // as the X button, which an unrelated demo element overlays)
      // restores the full list — the memo must recompute back to 5/5.
      await filterInput.fill('');
      await page.waitForTimeout(300);
      await visible(page, /Filtered frameworks:\s*5\/5/);
    });
  });

  // ═══ TAB 2: Motion ═══
  test.describe('Tab 2: 🎬 Motion', () => {
    test('13 - motion sections render', async ({ page }) => {
      await switchToTab(page, 2);
      await visible(page, 'Motion Demo');
      await visible(page, 'Keyframe Animation Demo');
      await visible(page, '缓动函数全家桶');
      await visible(page, '全动画属性覆盖');
      await visible(page, 'Motion Triggers Demo');
    });

    test('14 - Replay All buttons run without errors', async ({ page }) => {
      await switchToTab(page, 2);
      await clickBtn(page, 'Replay All @enter Animations');
      await page.waitForTimeout(300);
      await clickBtn(page, 'Replay All Animations');
      await page.waitForTimeout(300);
      await visible(page, 'Power 曲线');
    });

    test('15 - no runtime errors', async ({ page }) => {
      await expectNoErrors(page, 'tab2', () => switchToTab(page, 2));
    });
  });

  // ═══ TAB 3: Design ═══
  test.describe('Tab 3: 💅 Design', () => {
    test('16 - theme switching writes data-theme + CSS overrides', async ({ page }) => {
      await switchToTab(page, 3);
      await visible(page, 'Theme Selector');
      await page.waitForFunction(() => document.documentElement.dataset.theme !== undefined);
      expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('light');

      await page.getByText('Dark', { exact: true }).locator('..').click();
      await page.waitForTimeout(300);
      expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('dark');
      const surface = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--surface').trim());
      expect(surface).toBe('#111827');
      await visible(page, /Toggled\s*1\s*times/);

      await page.getByText('Sepia', { exact: true }).locator('..').click();
      await page.waitForTimeout(300);
      expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('sepia');
    });

    test('17 - design sections render', async ({ page }) => {
      await switchToTab(page, 3);
      await visible(page, '@themes');
      await visible(page, 'Design Tokens Visualizer');
      await visible(page, 'Component Variants');
      await visible(page, 'CSS 变量与单位');
    });

    test('18 - no runtime errors (React bugs E+F: kebab style props + lowercase event handlers — fixed by task 2.5)', async ({ page }) => {
      await expectNoErrors(page, 'tab3', () => switchToTab(page, 3));
    });
  });

  // ═══ TAB 4: State ═══
  test.describe('Tab 4: ⚡ State', () => {
    test('19 - SignalPatternsDemo counter (React bug D: & renders as literal &amp; — fixed by task 2.4 entity-escape separation)', async ({ page }) => {
      await switchToTab(page, 4);
      await visible(page, 'Signal & Store Patterns');
      await clickBtn(page, '+1');
      await page.waitForTimeout(200);
    });

    test.fixme('20 - InlineStoreDemo add/remove (BUG-C fixed: demo migrated to top-level CartStore; REMAINING: store getters frozen after first set() — Zustand Object.assign flattens accessor getters, see header note C)', async ({ page }) => {
      await switchToTab(page, 4);
      await visible(page, 'Inline Store Demo');
      await visible(page, 'cart empty');
      await clickBtn(page, 'Add Apple');
      await page.waitForTimeout(300);
      // React drops spaces around interpolations — match with \s* (suite convention)
      await visible(page, /Items:\s*1/);
      await visible(page, /Total:\s*5/);
      await clickBtn(page, 'Remove');
      await page.waitForTimeout(300);
      await visible(page, /Items:\s*0/);
      await visible(page, /Total:\s*0/);
    });

    test('21 - ComputedFnFormDemo calls counter', async ({ page }) => {
      await switchToTab(page, 4);
      await visible(page, 'Computed Function Form Demo');
      await clickBtn(page, /Set "San"/);
      await page.waitForTimeout(300);
      await visible(page, /Calls:\s*1/);
      await visible(page, 'Zhang San');
    });

    test('22 - no runtime errors', async ({ page }) => {
      await expectNoErrors(page, 'tab4', () => switchToTab(page, 4));
    });
  });

  // ═══ TAB 5: Language ═══
  test.describe('Tab 5: 🔤 Language', () => {
    test('23 - OptionalChainDemo interactions (React bug B: Language tab crashes the app)', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, 'Optional Chain ( ?. ) Demo');
      await clickBtn(page, /Set "Bob"/);
      await page.waitForTimeout(200);
      await visible(page, 'Bob');
      await clickBtn(page, /Toggle Details/);
      await page.waitForTimeout(200);
    });

    test('24 - TernaryExprDemo grade buttons (React bug B)', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, 'Ternary ( ? : )');
      await clickBtn(page, '95 → A');
      await page.waitForTimeout(200);
      await visible(page, /等级:\s*A/);
    });

    test('25 - TypeSystemDemo values (React bug B)', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, '类型系统全覆盖');
      await visible(page, /i8[\s\S]*127/);
    });

    test('26 - list add/remove/reorder (React bug B: tab crashes)', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, /@for\s*key=/);
      // add two items
      await clickBtn(page, 'Add Item');
      await page.waitForTimeout(150);
      await clickBtn(page, 'Add Item');
      await page.waitForTimeout(200);
      await visible(page, /Total:\s*5/);
      // move last to front — React keyed reconciliation must reorder
      await clickBtn(page, 'Move Last to Front');
      await page.waitForTimeout(200);
      // React emits CSS-module-hashed classes — match the hashed
      // list-item class instead of the literal one (OmniView keeps
      // literal class names; React hashes them via *.module.css).
      const order = await page.locator('li[class*="list-item"]').allTextContents();
      // React drops spaces around interpolations — match with \s* like
      // the other multi-token assertions in this suite.
      expect(order[0].replace(/\s+/g, ' ').trim()).toMatch(/#\s*5\s*—\s*Fruit-5/);
      expect(order[1].replace(/\s+/g, ' ').trim()).toMatch(/#\s*1\s*—\s*Apple/);
      // remove last
      await clickBtn(page, 'Remove Last');
      await page.waitForTimeout(200);
      await visible(page, /Total:\s*4/);
    });

    test('27 - BlockExprDemo + StringLiterals (React bug B)', async ({ page }) => {
      await switchToTab(page, 5);
      await visible(page, 'Block Expression Demo');
      await clickBtn(page, 'a + 5');
      await page.waitForTimeout(200);
      await visible(page, '字符串字面量演示');
      await visible(page, '异常处理');
    });

    test('28 - no runtime errors (React bug B crash)', async ({ page }) => {
      await expectNoErrors(page, 'tab5', () => switchToTab(page, 5));
    });
  });

  // ═══ TAB 6: Ref ═══
  test.describe('Tab 6: 🔗 Ref', () => {
    test('29 - imperative .focus()/.blur()', async ({ page }) => {
      await switchToTab(page, 6);
      await visible(page, 'Element Reference System');
      await clickBtn(page, 'Focus');
      await page.waitForTimeout(200);
      await visible(page, /Status:\s*focused/);
      await clickBtn(page, 'Blur');
      await page.waitForTimeout(200);
      await visible(page, /Status:\s*blurred/);
    });

    test('30 - pointer capture down/up', async ({ page }) => {
      await switchToTab(page, 6);
      // note: 'Press & hold me' renders as 'Press &amp; hold me' (React bug D) — match loosely
      const box = page.getByText(/hold me/).first().locator('..');
      await box.scrollIntoViewIfNeeded();
      await box.hover();
      await page.mouse.down();
      await page.waitForTimeout(150);
      await visible(page, /Pointer:\s*captured/);
      await page.mouse.up();
      await page.waitForTimeout(150);
      await visible(page, /Pointer:\s*released/);
    });

    test('31 - no runtime errors', async ({ page }) => {
      await expectNoErrors(page, 'tab6', () => switchToTab(page, 6));
    });
  });

  // ═══ TAB 7: Compiler ═══
  test.describe('Tab 7: ⚙️ Compiler', () => {
    test('32 - sections render + storage save', async ({ page }) => {
      await switchToTab(page, 7);
      await visible(page, 'HTTP GET Demo');
      await visible(page, 'HTTP POST Demo');
      await visible(page, 'Storage Demo');
      await visible(page, '标准库（Std Lib）演示');
      await page.locator('input[placeholder*="name"]').first().fill('测试用户');
      await clickBtn(page, 'Save');
      await page.waitForTimeout(300);
    });

    test('33 - no runtime errors', async ({ page }) => {
      await expectNoErrors(page, 'tab7', () => switchToTab(page, 7));
    });

    // H1 (task 2.3): error_message's MATCH tail must RETURN its value.
    // Before the fix the match IIFE ran as a bare statement, error_message
    // returned undefined and `msg === ""` always fell into the error
    // branch — the success path was unreachable. The upstream fetch is
    // route-intercepted so the test is deterministic and offline-safe.
    test('36 - DataFetching API success path (tail match return)', async ({ page }) => {
      await switchToTab(page, 7);
      await visible(page, 'HTTP GET Demo');
      await page.route('https://jsonplaceholder.typicode.com/todos/1', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          headers: { 'access-control-allow-origin': '*' },
          body: '[{"id":1,"title":"delectus aut autem"}]',
        });
      });
      await clickBtn(page, 'API');
      await visible(page, 'api.omni fetch_demo OK');
    });
  });

  // ═══ TAB 8: Tokens ═══
  test.describe('Tab 8: 🔮 Tokens', () => {
    test('34 - preset buttons flip live token CSS vars', async ({ page }) => {
      await switchToTab(page, 8);
      await visible(page, 'Dynamic Design Tokens');
      const colorBefore = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--dynColor').trim());
      await clickBtn(page, 'Emerald');
      await page.waitForTimeout(300);
      const colorAfter = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--dynColor').trim());
      expect(colorAfter).not.toBe(colorBefore);
      expect(colorAfter).toBe('#10b981');
    });

    test('35 - no runtime errors', async ({ page }) => {
      await expectNoErrors(page, 'tab8', () => switchToTab(page, 8));
    });
  });
});
