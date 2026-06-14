/**
 * E2E: Visual Scripting — @xyflow/react FlowCanvas
 *
 * Tests: canvas renders in code mode, right-click context menu,
 * adding blocks (code block style), and canvas controls.
 */
import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5173";

test.describe("M4: Visual Scripting — @xyflow/react FlowCanvas", () => {

  test("canvas renders in code mode with @xyflow/react container", async ({ page }) => {
    await page.goto(EDITOR_URL);

    // Switch to code mode to make the FlowCanvas visible
    const codeBtn = page.locator("button", { hasText: "代码" });
    await expect(codeBtn).toBeVisible({ timeout: 10000 });
    await codeBtn.click();
    await page.waitForTimeout(500);

    // The React Flow wrapper element should now be visible (class = "react-flow")
    const reactFlow = page.locator(".react-flow");
    await expect(reactFlow).toBeVisible({ timeout: 15000 });
  });

  test("right-click opens block menu on canvas", async ({ page }) => {
    await page.goto(EDITOR_URL);

    // Switch to code mode
    const codeBtn = page.locator("button", { hasText: "代码" });
    await expect(codeBtn).toBeVisible({ timeout: 10000 });
    await codeBtn.click();
    await page.waitForTimeout(500);

    // Wait for React Flow to render
    const reactFlow = page.locator(".react-flow");
    await expect(reactFlow).toBeVisible({ timeout: 15000 });

    // Right-click on the canvas area
    const box = await reactFlow.boundingBox();
    if (box) {
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2, { button: "right" });
    }

    // Context menu should appear with the "添加节点" header
    const menuHeader = page.locator("text=添加节点");
    await expect(menuHeader).toBeVisible({ timeout: 5000 });
  });

  test("adding a code block creates a ReactFlow node", async ({ page }) => {
    await page.goto(EDITOR_URL);

    // Switch to code mode
    const codeBtn = page.locator("button", { hasText: "代码" });
    await expect(codeBtn).toBeVisible({ timeout: 10000 });
    await codeBtn.click();
    await page.waitForTimeout(500);

    // Wait for React Flow to render
    const reactFlow = page.locator(".react-flow");
    await expect(reactFlow).toBeVisible({ timeout: 15000 });

    // Right-click to open context menu
    const box = await reactFlow.boundingBox();
    if (box) {
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2, { button: "right" });
    }

    // Click the "代码块" menu item — use getByRole('main') to scope to the
    // context menu inside the FlowCanvas, avoiding BlockPalette outside <main>
    const blockMenuItem = page.getByRole("main").getByText("代码块");
    await expect(blockMenuItem).toBeVisible({ timeout: 3000 });
    await blockMenuItem.click();

    // A ReactFlow node should appear
    const node = page.locator(".react-flow__node");
    await expect(node).toBeVisible({ timeout: 5000 });
  });

  test("node appears with correct label text", async ({ page }) => {
    await page.goto(EDITOR_URL);

    // Switch to code mode
    const codeBtn = page.locator("button", { hasText: "代码" });
    await expect(codeBtn).toBeVisible({ timeout: 10000 });
    await codeBtn.click();
    await page.waitForTimeout(500);

    // Wait for React Flow to render
    const reactFlow = page.locator(".react-flow");
    await expect(reactFlow).toBeVisible({ timeout: 15000 });

    // Right-click and add a code block
    const box = await reactFlow.boundingBox();
    if (box) {
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2, { button: "right" });
    }
    await page.getByRole("main").getByText("代码块").click();
    await page.waitForTimeout(500);

    // The node should contain "Action" text (the default label for action blocks)
    const nodeLabel = page.locator(".react-flow__node").first();
    await expect(nodeLabel).toContainText("Action", { timeout: 3000 });
  });

  test("canvas controls and minimap are visible", async ({ page }) => {
    await page.goto(EDITOR_URL);

    // Switch to code mode
    const codeBtn = page.locator("button", { hasText: "代码" });
    await expect(codeBtn).toBeVisible({ timeout: 10000 });
    await codeBtn.click();
    await page.waitForTimeout(500);

    // Wait for React Flow to render
    const reactFlow = page.locator(".react-flow");
    await expect(reactFlow).toBeVisible({ timeout: 15000 });

    // React Flow controls (zoom in/out/fit) should be present
    const controls = page.locator(".react-flow__controls");
    await expect(controls).toBeVisible({ timeout: 5000 });

    // Minimap should be present
    const minimap = page.locator(".react-flow__minimap");
    await expect(minimap).toBeVisible({ timeout: 5000 });
  });

});
