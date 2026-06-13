/**
 * E2E test for OmniDriver Task Manager Pro — 完整演示应用
 *
 * 验证内容:
 *   1. 所有便捷方法 (button, text, heading, card, flex, container, input, image, list, alert)
 *   2. O.component() 方法 (Link, Form, Checkbox, Select, Spinner, Progress, ListItem, TextArea)
 *   3. 事件处理 (onPress, onChange)
 *   4. 状态管理 (signal, computed, action, store — 全部4种 codeBlock)
 *   5. 内联代码块 (codeBlock with source)
 *   6. 外部代码块 (writeFile → .omni source file)
 *   7. Progressive Build-Test Loop (按 Phase 验证)
 *   8. O.validate() 画布健康检查
 *   9. 语义化命名 (Naming Convention)
 *
 * 运行前提:
 *   Terminal 1: python -m editor.server.main --dir build/test --port 8000
 *   Terminal 2: cd editor/client && npm run dev  (→ http://localhost:5174)
 *
 * 运行:
 *   npx playwright test editor/e2e/omni-driver-task-manager.spec.ts
 *   npx playwright test editor/e2e/omni-driver-task-manager.spec.ts --headed  # 可视化
 */

import { test, expect } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const EDITOR_URL = "http://localhost:5174";
const POLL_INTERVAL_MS = 200;
const WAIT_TIMEOUT_MS = 15000;

// ESM-compatible __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// 读取 OmniDriver 应用脚本
const APP_SCRIPT_PATH = path.resolve(
  __dirname,
  "..",
  "..",
  "build",
  "test",
  "omni-driver",
  "task-manager-app.js",
);

const TASK_MANAGER_APP_SCRIPT = fs.readFileSync(APP_SCRIPT_PATH, "utf-8");

test.describe("OmniDriver Task Manager Pro", () => {
  test(
    "builds complete Task Manager Pro app via window.__OMNI__ API",
    async ({ page }) => {
      // ── Navigate and wait for OmniDriver ────────────────────
      await page.goto(EDITOR_URL);

      await page.waitForFunction(
        () =>
          typeof (window as any).__OMNI__ !== "undefined" &&
          (window as any).__OMNI__ !== null,
        { timeout: WAIT_TIMEOUT_MS, polling: POLL_INTERVAL_MS },
      );

      // ── Execute the full app script ─────────────────────────
      const result = await page.evaluate(async (script) => {
        // eval() the IIFE script — it returns the build report
        const buildReport = await eval(script);
        return buildReport;
      }, TASK_MANAGER_APP_SCRIPT);

      // ═══════════════════════════════════════════════════════
      // ASSERTIONS
      // ═══════════════════════════════════════════════════════

      // ── Canvas validity ────────────
      expect(result.isValid, "Canvas should be valid").toBe(true);
      expect(result.report).toBeDefined();
      expect(
        result.report.orphanElements.length,
        "Should have no orphan elements",
      ).toBe(0);

      // ── Element counts ─────────────
      // Expected: root Card + header (Flex, Image, Heading, Flex with 3 Links)
      //   + body Flex + sidebar (Flex, Heading, 3 Buttons, Alert)
      //   + main Flex + 4 stat Cards (each with 2 Text = 8) + Input
      //   + task List (8 ListItems, each with Flex+Checkbox+Text = 24)
      //   + Heading + Form + Flex (Input, Select, Button) + Alert
      //   + Progress + Text + Spinner + TextArea
      //   + codeBlocks (5 signals, 5 computed, 5 handlers, 1 store = 16)
      // Total should be 60+ elements
      expect(
        result.elementCount,
        "Should have 65+ elements total",
      ).toBeGreaterThanOrEqual(65);
      expect(
        result.wireCount,
        "Should have 15+ wires",
      ).toBeGreaterThanOrEqual(15);

      // ── View/CodeBlock split ───────
      expect(
        result.viewCount,
        "Should have 50+ view elements",
      ).toBeGreaterThanOrEqual(50);
      expect(
        result.codeBlockCount,
        "Should have 16 code blocks (5 signals + 5 computed + 5 handlers + 1 store)",
      ).toBe(16);

      // ── Validation report ──────────
      expect(
        result.report.unwiredEvents.length,
        "Should have no unwired events",
      ).toBe(0);
      expect(
        result.report.typeMismatches.length,
        "Should have no type mismatches",
      ).toBe(0);
      expect(
        result.report.unreachableBlocks.length,
        "Should have no unreachable blocks",
      ).toBe(0);

      // ── Root element exists ────────
      expect(result.rootId).toBeDefined();
      expect(result.rootId).toMatch(/^el-/);
    },
  );

  test("OmniDriver help() and discovery APIs work", async ({ page }) => {
    await page.goto(EDITOR_URL);

    await page.waitForFunction(
      () => typeof (window as any).__OMNI__ !== "undefined",
      { timeout: WAIT_TIMEOUT_MS, polling: POLL_INTERVAL_MS },
    );

    const info = await page.evaluate(() => {
      const O = (window as any).__OMNI__;

      const help = O.help();
      const snippets = O.listSnippets();
      const components = O.listComponents();
      const buttonInfo = O.getComponentInfo("Button");
      const formInfo = O.getComponentInfo("Form");

      return {
        helpKeys: Object.keys(help),
        hasPortRef: !!help.portReference,
        hasEventTable: !!help.eventUuidTable,
        hasCriticalRules: Array.isArray(help.criticalRules),
        snippets,
        componentCount: components.length,
        hasButton: components.some((c: any) => c.tag === "Button"),
        hasForm: components.some((c: any) => c.tag === "Form"),
        buttonEvents: Object.keys(buttonInfo?.events || {}),
        formEvents: Object.keys(formInfo?.events || {}),
      };
    });

    // API discovery
    expect(info.helpKeys).toContain("version");
    expect(info.helpKeys).toContain("portReference");
    expect(info.helpKeys).toContain("eventUuidTable");
    expect(info.hasPortRef).toBe(true);
    expect(info.hasEventTable).toBe(true);
    expect(info.hasCriticalRules).toBe(true);

    // Snippets
    expect(info.snippets.builtin).toContain("login-form");
    expect(info.snippets.builtin).toContain("data-card");
    expect(info.snippets.builtin).toContain("crud-table");

    // Component discovery
    expect(info.componentCount).toBeGreaterThanOrEqual(19);
    expect(info.hasButton).toBe(true);
    expect(info.hasForm).toBe(true);
    expect(info.buttonEvents).toContain("onPress");
    expect(info.formEvents).toContain("onSubmit");
  });

  test("O.validate() returns healthy report on empty canvas", async ({
    page,
  }) => {
    await page.goto(EDITOR_URL);

    await page.waitForFunction(
      () => typeof (window as any).__OMNI__ !== "undefined",
      { timeout: WAIT_TIMEOUT_MS, polling: POLL_INTERVAL_MS },
    );

    const report = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.help();
      O.clearAll();
      O.switchMode("design");
      return O.validate();
    });

    // Empty canvas should be valid (no orphans, no mismatches)
    expect(report.isValid).toBe(true);
    expect(report.orphanElements).toEqual([]);
    expect(report.unwiredEvents).toEqual([]);
    expect(report.typeMismatches).toEqual([]);
    expect(report.unreachableBlocks).toEqual([]);
  });

  test("external .omni source file is writable", async ({ page }) => {
    await page.goto(EDITOR_URL);

    await page.waitForFunction(
      () => typeof (window as any).__OMNI__ !== "undefined",
      { timeout: WAIT_TIMEOUT_MS, polling: POLL_INTERVAL_MS },
    );

    const fileResult = await page.evaluate(async () => {
      const O = (window as any).__OMNI__;
      O.help();
      O.switchMode("design");
      O.clearAll();

      // Write a minimal .omni source as an external code block demo
      const testSource = `component TestStore {
  store ItemStore {
    items: Item[]
  }
  signal counter: i32 = 0
}`;

      const saved = await O.writeFile(
        "src/components/TestStore.omni",
        testSource,
      );

      // Now read it back
      const content = await O.readFile("src/components/TestStore.omni");

      return { saved, content, hasStore: content.includes("store ItemStore") };
    });

    expect(fileResult.saved).toBe(true);
    expect(fileResult.content).toContain("component TestStore");
    expect(fileResult.hasStore).toBe(true);
  });
});
