import { test, expect } from "@playwright/test";

test.describe("CodeCard Editor", () => {
  test.beforeEach(async ({ page }) => {
    // Assumes app is running with a project that has a code-block node
    // Double-click the code-block node to open editor
    await page.goto("http://localhost:1420");
    // Wait for the app to load
    await page.waitForSelector(".react-flow", { timeout: 10000 });
  });

  test("typing 'l' shows autocomplete with let options", async ({ page }) => {
    // This test requires Poiesis running with a project open
    // and a code-block node to double-click
    test.skip(true, "Requires running app with project — run manually");
  });

  test("save button emits valid CodeOp[]", async ({ page }) => {
    test.skip(true, "Requires running app with project — run manually");
  });
});
