/**
 * E2E tests for OmniLang Visual Editor — 10 critical user flows.
 *
 * Prerequisites:
 *   1. Start server: python editor/server/main.py --dir build/smoketest --port 5199
 *   2. Run tests:  npx playwright test editor/e2e/editor.spec.ts
 *
 * Or use the CDP tool to run interactively:
 *   mcp__flutter-skill__connect_cdp → http://localhost:5199
 */

import { test, expect } from "@playwright/test";

const BASE = "http://localhost:5199";

// ── Flow 1: Project loads with file list ─────────────────
test("project loads with file list", async ({ page }) => {
  await page.goto(BASE);
  // Should show the editor UI
  await expect(page.locator("text=OmniLang").first()).toBeVisible({ timeout: 10000 });
  // File explorer should list Counter.omni
  await expect(page.locator("text=Counter.omni")).toBeVisible({ timeout: 5000 });
});

// ── Flow 2: Open .omni file and see Monaco editor ────────
test("open omni file in code editor", async ({ page }) => {
  await page.goto(BASE);
  await page.click("text=Counter.omni");
  // Monaco editor should show .omni source
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 5000 });
  // Should contain "component Counter"
  await expect(page.locator(".monaco-editor")).toContainText("Counter", { timeout: 5000 });
});

// ── Flow 3: Compile .omni to React ────────────────────────
test("compile omni to react", async ({ page }) => {
  await page.goto(BASE);
  await page.click("text=Counter.omni");
  // Click compile button
  await page.click("text=编译");
  // Should show success status
  await expect(page.locator("text=✅").or(page.locator("text=已编译"))).toBeVisible({ timeout: 10000 });
});

// ── Flow 4: Switch to Design mode and see canvas ──────────
test("switch to design mode shows canvas", async ({ page }) => {
  await page.goto(BASE);
  await page.click("text=Counter.omni");
  await page.click("text=编译");
  // Switch to design mode
  await page.click("text=设计");
  // Canvas should be visible with elements
  await expect(page.locator("[data-omni-id]").first()).toBeVisible({ timeout: 10000 });
});

// ── Flow 5: Select element and see property panel ────────
test("select element shows property panel", async ({ page }) => {
  await page.goto(BASE);
  await page.click("text=Counter.omni");
  await page.click("text=编译");
  await page.click("text=设计");
  // Click on a canvas element
  const el = page.locator("[data-omni-id]").first();
  await el.click();
  // Property panel should show properties
  await expect(page.locator("text=属性")).toBeVisible({ timeout: 5000 });
});

// ── Flow 6: Toggle HUD to see signal bindings ────────────
test("toggle state HUD shows signal values", async ({ page }) => {
  await page.goto(BASE);
  await page.click("text=Counter.omni");
  await page.click("text=编译");
  await page.click("text=设计");
  // Select an element
  await page.locator("[data-omni-id]").first().click();
  // Toggle HUD
  await page.click("text=状态");
  // HUD should appear
  await expect(page.locator("text=signal").or(page.locator("text=绑定"))).toBeVisible({ timeout: 5000 });
});

// ── Flow 7: Open unified search (Ctrl+Shift+F) ────────────
test("unified search finds components", async ({ page }) => {
  await page.goto(BASE);
  await page.keyboard.press("Control+Shift+F");
  // Search modal should appear
  const searchInput = page.locator('input[placeholder*="搜索"]').first();
  await expect(searchInput).toBeVisible({ timeout: 5000 });
  await searchInput.fill("Button");
  // Should find Button component
  await expect(page.locator("text=Button")).toBeVisible({ timeout: 3000 });
});

// ── Flow 8: Save file (Ctrl+S) ───────────────────────────
test("save file persists changes", async ({ page }) => {
  await page.goto(BASE);
  await page.click("text=Counter.omni");
  await page.keyboard.press("Control+S");
  // Should show saved status
  await expect(page.locator("text=已保存")).toBeVisible({ timeout: 5000 });
});

// ── Flow 9: Export to .omni.json ──────────────────────────
test("export to omni json via API", async ({ request }) => {
  const resp = await request.post(`${BASE}/api/omni-json/export`, {
    data: {
      source: "component Test { view { <Text>Hello</Text> } }",
      filepath: "Test.omni",
      target: "react",
    },
  });
  expect(resp.ok()).toBeTruthy();
  const body = await resp.json();
  expect(body.success).toBe(true);
  expect(body.document.components.length).toBeGreaterThan(0);
});

// ── Flow 10: Full round-trip integrity ────────────────────
test("round-trip preserves compilable output", async ({ request }) => {
  const source = "component Test { view { <Text>Hello World</Text> } }";

  // Export
  const exportResp = await request.post(`${BASE}/api/omni-json/export`, {
    data: { source, filepath: "Test.omni", target: "react" },
  });
  const doc = (await exportResp.json()).document;

  // Import
  const importResp = await request.post(`${BASE}/api/omni-json/import`, {
    data: { document: doc },
  });
  const reconstructed = (await importResp.json()).source;

  // Compile reconstructed
  const compileResp = await request.post(`${BASE}/api/compile`, {
    data: { source: reconstructed, target: "react" },
  });
  const compileBody = await compileResp.json();

  expect(compileBody.success).toBe(true);
  expect(Object.keys(compileBody.files).length).toBeGreaterThan(0);
});
