/**
 * Critical Path E2E — safety net for editor architecture refactoring.
 *
 * These tests use OmniDriver API (`window.__OMNI__`) and the backward-compatible
 * Zustand combined store (`window.__ZUSTAND_EDITOR_STORE__`) to verify that
 * core editor workflows survive refactoring changes.
 *
 * Prerequisites:
 *   1. Build frontend: cd editor/client && npm run build
 *   2. Start server: python editor/server/main.py --dir build/smoketest --port 5199
 *   3. Run: npx playwright test --config=editor/e2e/playwright.config.ts editor/e2e/critical-path.spec.ts
 */

import { test, expect } from "@playwright/test";

const BASE = "http://localhost:5199";

test.describe("Critical Path E2E", () => {
  // ── Test 1: Add component via OmniDriver, verify in store ──────────────
  test("add component via OmniDriver and verify store state", async ({
    page,
  }) => {
    await page.goto(BASE);
    await page.waitForFunction(() => !!(window as any).__OMNI__);

    // Create a Button via OmniDriver
    await page.evaluate(() => {
      (window as any).__OMNI__.button("TestBtn");
    });

    // Store should have at least one element
    const count = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().elements.length,
    );
    expect(count).toBeGreaterThanOrEqual(1);

    // The element should have the correct tag and label
    const tag = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const btn = store
        .getState()
        .elements.find((e: any) => e.tag === "Button");
      return btn?.tag;
    });
    expect(tag).toBe("Button");

    const label = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const btn = store
        .getState()
        .elements.find((e: any) => e.tag === "Button");
      return btn?.props?.label;
    });
    expect(label).toBe("TestBtn");
  });

  // ── Test 2: Modify element property via store, verify change ──────────
  test("modify property updates element in store", async ({ page }) => {
    await page.goto(BASE);
    await page.waitForFunction(() => !!(window as any).__OMNI__);

    // Create a Text element via OmniDriver
    await page.evaluate(() => {
      (window as any).__OMNI__.text("Hello");
    });

    // Mutate the label via store.setProp()
    await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const el = store
        .getState()
        .elements.find((e: any) => e.tag === "Text");
      if (el) store.getState().setProp(el.id, "label", "已更新");
    });

    // Verify the label changed
    const label = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const el = store
        .getState()
        .elements.find((e: any) => e.tag === "Text");
      return el?.props?.label;
    });
    expect(label).toBe("已更新");
  });

  // ── Test 3: Add wire between two elements, verify in store ────────────
  test("add wire and verify store", async ({ page }) => {
    await page.goto(BASE);
    await page.waitForFunction(() => !!(window as any).__OMNI__);

    // Add two elements and a wire connecting them
    await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      store.getState().addElement({
        id: "test-btn-1",
        nodeType: "view",
        tag: "Button",
        props: { label: "Btn" },
        children: [],
        parentId: null,
      });
      store.getState().addElement({
        id: "test-txt-1",
        nodeType: "view",
        tag: "Text",
        props: { label: "Txt" },
        children: [],
        parentId: null,
      });
      store.getState().addWire({
        id: "test-wire-1",
        fromId: "test-btn-1",
        fromPort: "out",
        toId: "test-txt-1",
        toPort: "in",
      });
    });

    // Store should have at least one wire
    const wireCount = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().wires.length,
    );
    expect(wireCount).toBeGreaterThanOrEqual(1);

    // Verify wire details
    const wire = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      return store
        .getState()
        .wires.find((w: any) => w.id === "test-wire-1");
    });
    expect(wire).toBeTruthy();
    expect(wire.fromId).toBe("test-btn-1");
    expect(wire.toId).toBe("test-txt-1");
  });

  // ── Test 4: Switch between code and design modes, elements preserved ──
  test("switch mode via store preserves element count", async ({ page }) => {
    await page.goto(BASE);
    await page.waitForFunction(() => !!(window as any).__OMNI__);

    // Add an element via OmniDriver
    await page.evaluate(() => {
      (window as any).__OMNI__.button("ModeTest");
    });

    const beforeCount = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().elements.length,
    );
    expect(beforeCount).toBeGreaterThanOrEqual(1);

    // Switch to code mode
    await page.evaluate(() => {
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().setActiveMode(
        "code",
      );
    });
    await page.waitForTimeout(300);

    // Switch back to design mode
    await page.evaluate(() => {
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().setActiveMode(
        "design",
      );
    });

    const afterCount = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().elements.length,
    );
    expect(afterCount).toBe(beforeCount);
  });

  // ── Test 5: Save marks project clean ──────────────────────────────────
  test("save marks project clean", async ({ page }) => {
    await page.goto(BASE);
    await page.waitForFunction(() => !!(window as any).__OMNI__);

    // Add an element to make the project dirty
    await page.evaluate(() => {
      (window as any).__OMNI__.text("SaveTest");
    });

    const isDirtyBefore = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().isDirty,
    );
    expect(isDirtyBefore).toBe(true);

    // Press Ctrl+S to trigger save
    await page.keyboard.press("Control+s");
    await page.waitForTimeout(500);

    // After save, the project should be clean (markClean called by saveProject)
    const isDirtyAfter = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().isDirty,
    );
    expect(isDirtyAfter).toBe(false);
  });

  // ── Test 6: Reorder elements updates rootOrder ────────────────────────
  test("reorder elements updates rootOrder", async ({ page }) => {
    await page.goto(BASE);
    await page.waitForFunction(() => !!(window as any).__OMNI__);

    // Add two elements via store with known IDs
    const [id1, id2] = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const e1 = {
        id: "reorder-a",
        nodeType: "view" as const,
        tag: "Button",
        props: { label: "A" },
        children: [] as string[],
        parentId: null as string | null,
      };
      const e2 = {
        id: "reorder-b",
        nodeType: "view" as const,
        tag: "Text",
        props: { label: "B" },
        children: [] as string[],
        parentId: null as string | null,
      };
      store.getState().addElement(e1);
      store.getState().addElement(e2);
      return [e1.id, e2.id];
    });

    // Capture order before reorder
    const orderBefore = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().rootOrder,
    );
    expect(orderBefore).toEqual([id1, id2]);

    // Reorder: move A after B
    await page.evaluate(
      ([a, b]) => {
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().reorderElement(
          a,
          b,
          "after",
        );
      },
      [id1, id2],
    );

    // Order should now be [id2, id1]
    const orderAfter = await page.evaluate(
      () =>
        (window as any).__ZUSTAND_EDITOR_STORE__.getState().rootOrder,
    );
    expect(orderAfter).not.toEqual(orderBefore);
    expect(orderAfter).toEqual([id2, id1]);
  });
});
