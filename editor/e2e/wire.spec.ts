/**
 * Wire system E2E tests — React Flow canvas.
 *
 * Prerequisites:
 *   1. Start server: python editor/server/main.py --dir build/smoketest --port 5199
 *   2. Run: npx playwright test --config=editor/e2e/playwright.config.ts editor/e2e/wire.spec.ts
 */

import { test, expect } from "@playwright/test";

const BASE = "http://localhost:5199";

// ── Helpers ──────────────────────────────────────────

async function navigateToDesignWithCounter(page: any) {
  await page.goto(BASE);
  await page.waitForTimeout(1500);
  // Convert project from read-only if needed
  const convertBtn = page.getByRole("button", { name: "Convert to Visual Project" });
  if (await convertBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await convertBtn.click();
    await page.waitForTimeout(500);
  }
  // Navigate to src/Counter.omni
  await page.locator("text=src").first().click();
  await page.waitForTimeout(300);
  await page.locator("text=Counter.omni").first().click();
  await page.waitForTimeout(500);
  // Compile
  await page.locator("button").filter({ hasText: "编译" }).first().click();
  await page.waitForTimeout(1500);
  // Switch to design mode
  await page.locator("button").filter({ hasText: "设计" }).first().click();
  // Wait for React Flow to render
  await page.waitForSelector(".react-flow__node", { timeout: 10000 });
}

function buildWire(id: string, fromId: string, toId: string) {
  return { id, fromId, fromPort: "out", toId, toPort: "in", isCollapsed: false, collapseGroupId: "" };
}

// ═══════════════════════════════════════════════════
// RF1: React Flow renders in design mode
// ═══════════════════════════════════════════════════

test("design mode shows React Flow canvas with nodes", async ({ page }) => {
  await navigateToDesignWithCounter(page);

  // React Flow container
  const rf = page.locator(".react-flow");
  await expect(rf).toBeVisible({ timeout: 5000 });

  // Nodes
  const nodes = page.locator(".react-flow__node");
  const nodeCount = await nodes.count();
  expect(nodeCount).toBeGreaterThan(0);

  // Each node should have handles (ports)
  const handles = page.locator(".react-flow__handle");
  const handleCount = await handles.count();
  expect(handleCount).toBeGreaterThan(0);
});

// ═══════════════════════════════════════════════════
// RF2: Edges render when wires exist
// ═══════════════════════════════════════════════════

test("adding wires creates visible edges", async ({ page }) => {
  await navigateToDesignWithCounter(page);

  // Get real element IDs from the store
  const ids: string[] = await page.evaluate(() => {
    const store = (window as any).__ZUSTAND_EDITOR_STORE__;
    if (!store) return [];
    return store.getState().elements.map((e: any) => e.id);
  });

  expect(ids.length).toBeGreaterThan(2);

  // Add 2 wires connecting real elements
  await page.evaluate(([a, b, c]: string[]) => {
    const store = (window as any).__ZUSTAND_EDITOR_STORE__;
    store.getState().addWire({ id: "rft-1", fromId: a, fromPort: "out", toId: b, toPort: "in", isCollapsed: false, collapseGroupId: "" });
    store.getState().addWire({ id: "rft-2", fromId: b, fromPort: "out", toId: c, toPort: "in", isCollapsed: false, collapseGroupId: "" });
    // Force React Flow to sync from store (bypasses React batching)
    if ((window as any).__FLOW_SYNC__) (window as any).__FLOW_SYNC__();
  }, ids);

  await page.waitForTimeout(300);

  // Edges should appear
  const edges = page.locator(".react-flow__edge");
  await expect(edges.first()).toBeVisible({ timeout: 5000 });
  const edgeCount = await edges.count();
  expect(edgeCount).toBe(2);

  // Each edge should have an SVG path
  const paths = page.locator(".react-flow__edge path");
  expect(await paths.count()).toBeGreaterThan(0);
});

// ═══════════════════════════════════════════════════
// RF3: Store→Flow sync (wires persisted across remount)
// ═══════════════════════════════════════════════════

test("wires survive mode switch reloading edges", async ({ page }) => {
  await navigateToDesignWithCounter(page);

  // Add wires
  await page.evaluate(() => {
    const store = (window as any).__ZUSTAND_EDITOR_STORE__;
    const els = store.getState().elements;
    if (els.length < 2) return;
    store.getState().addWire({ id: "rft-p1", fromId: els[0].id, fromPort: "out", toId: els[1].id, toPort: "in", isCollapsed: false, collapseGroupId: "" });
    if ((window as any).__FLOW_SYNC__) (window as any).__FLOW_SYNC__();
  });
  await page.waitForTimeout(300);

  // Switch to code and back
  await page.locator("button").filter({ hasText: "代码" }).first().click();
  await page.waitForTimeout(500);
  await page.locator("button").filter({ hasText: "设计" }).first().click();
  await page.waitForSelector(".react-flow__node", { timeout: 10000 });
  await page.waitForTimeout(500);

  // Edge should still be there
  const edgeCount = await page.locator(".react-flow__edge").count();
  expect(edgeCount).toBe(1);
});

// ═══════════════════════════════════════════════════
// RF4: Wire removal removes edge
// ═══════════════════════════════════════════════════

test("removing wire removes edge from canvas", async ({ page }) => {
  await navigateToDesignWithCounter(page);

  // Add then remove
  await page.evaluate(() => {
    const store = (window as any).__ZUSTAND_EDITOR_STORE__;
    const els = store.getState().elements;
    store.getState().addWire({ id: "rft-rm", fromId: els[0].id, fromPort: "out", toId: els[1].id, toPort: "in", isCollapsed: false, collapseGroupId: "" });
    if ((window as any).__FLOW_SYNC__) (window as any).__FLOW_SYNC__();
  });
  await page.waitForTimeout(300);
  expect(await page.locator(".react-flow__edge").count()).toBe(1);

  // Remove it
  await page.evaluate(() => {
    (window as any).__ZUSTAND_EDITOR_STORE__.getState().removeWire("rft-rm");
    if ((window as any).__FLOW_SYNC__) (window as any).__FLOW_SYNC__();
  });
  await page.waitForTimeout(300);

  expect(await page.locator(".react-flow__edge").count()).toBe(0);
});

// ═══════════════════════════════════════════════════
// RF5: API: wire survives export→import→compile
// ═══════════════════════════════════════════════════

test("wire round-trip: export→import→compile", async ({ request }) => {
  const source = "component Test { signal count: i32 = 0; view { <Text>Hi</Text> } }";
  const wires = [buildWire("w-rt-1", "cb-count", "el-text")];

  const exportResp = await request.post(`${BASE}/api/omni-json/export`, {
    data: { source, filepath: "Test.omni", target: "react", existingWires: wires },
  });
  const doc = (await exportResp.json()).document;
  expect(doc.wires).toHaveLength(1);
  expect(doc.wires[0].id).toBe("w-rt-1");

  const importResp = await request.post(`${BASE}/api/omni-json/import`, {
    data: { document: doc },
  });
  expect(importResp.ok()).toBeTruthy();
  const reconstructed = (await importResp.json()).source;
  expect(reconstructed.length).toBeGreaterThan(10);

  const compileResp = await request.post(`${BASE}/api/compile`, {
    data: { source: reconstructed, target: "react" },
  });
  expect((await compileResp.json()).success).toBe(true);
});

// ═══════════════════════════════════════════════════
// RF6: API: multiple wires preserved
// ═══════════════════════════════════════════════════

test("five wires survive export with correct IDs", async ({ request }) => {
  const wires = [
    buildWire("wm1", "btn-1", "sig-a"),
    buildWire("wm2", "btn-2", "sig-b"),
    buildWire("wm3", "sig-a", "lbl-1"),
    buildWire("wm4", "sig-b", "lbl-2"),
    buildWire("wm5", "inp-1", "sig-c"),
  ];

  const resp = await request.post(`${BASE}/api/omni-json/export`, {
    data: { source: "component Test { view { <Text>Hi</Text> } }", filepath: "T.omni", target: "react", existingWires: wires },
  });
  const body = await resp.json();
  expect(body.success).toBe(true);
  expect(body.document.wires).toHaveLength(5);

  const ids = body.document.wires.map((w: any) => w.id).sort();
  expect(ids).toEqual(["wm1", "wm2", "wm3", "wm4", "wm5"]);
});

// ═══════════════════════════════════════════════════
// RF7: API: wire merge conflict
// ═══════════════════════════════════════════════════

test("3-way merge detects wire port conflicts", async ({ request }) => {
  const resp = await request.post(`${BASE}/api/merge`, {
    data: {
      base: { wires: [{ id: "w1", fromId: "a", toId: "b", fromPort: "old", toPort: "old" }] },
      local: { wires: [{ id: "w1", fromId: "a", toId: "b", fromPort: "local", toPort: "old" }] },
      remote: { wires: [{ id: "w1", fromId: "a", toId: "b", fromPort: "remote", toPort: "old" }] },
    },
  });
  const body = await resp.json();
  expect(body.success).toBe(false);
  expect(body.conflicts.length).toBeGreaterThan(0);
  expect(body.conflicts[0].path).toContain("fromPort");
});

// ═══════════════════════════════════════════════════
// RF8: API: wire save→load cycle
// ═══════════════════════════════════════════════════

test("wires survive save and load via omni-json API", async ({ request }) => {
  const wires = [{
    id: "w-save-1", fromId: "el-x", fromPort: "out1", toId: "el-y", toPort: "in1",
    isCollapsed: true, collapseGroupId: "g1",
  }];

  const exportResp = await request.post(`${BASE}/api/omni-json/export`, {
    data: { source: "component Test { view { <Text>Hi</Text> } }", filepath: "T.omni", target: "react", existingWires: wires },
  });
  const doc = (await exportResp.json()).document;

  const saveResp = await request.post(`${BASE}/api/omni-json/save`, {
    data: { filePath: "test-wire-save.omni.json", document: doc },
  });
  expect(saveResp.ok()).toBeTruthy();

  const loadResp = await request.post(`${BASE}/api/omni-json/load`, {
    data: { filePath: "test-wire-save.omni.json" },
  });
  const loaded = (await loadResp.json()).document;
  expect(loaded.wires).toHaveLength(1);
  expect(loaded.wires[0].id).toBe("w-save-1");
  expect(loaded.wires[0].isCollapsed).toBe(true);
});
