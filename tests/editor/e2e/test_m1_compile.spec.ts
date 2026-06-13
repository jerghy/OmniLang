/**
 * E2E: Compile flow — edit .omni source → compile → verify output.
 *
 * Tests the core "write → compile → see result" loop that is the
 * primary user interaction of the M1a editor.
 */
import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5173";

test.describe("Compile flow", () => {
  test("loads editor and renders compile UI", async ({ page }) => {
    await page.goto(EDITOR_URL);

    // Editor should load with source textarea or Monaco editor
    const sourceArea = page.locator("textarea#source");
    const monacoEditor = page.locator(".monaco-editor");

    const hasEditor = (await sourceArea.count()) > 0 || (await monacoEditor.count()) > 0;
    expect(hasEditor).toBeTruthy();
  });

  test("compiles pure logic .omni → shows output", async ({ page }) => {
    await page.goto(EDITOR_URL);

    // Find the source input area
    const sourceArea = page.locator("textarea#source");
    const monacoEditor = page.locator(".monaco-editor .view-lines");

    if ((await sourceArea.count()) > 0) {
      // M1a minimal HTML editor
      await sourceArea.fill('fn main() { println("hello test"); }');
      await page.locator("text=Compile").click();

      // Wait for output
      const output = page.locator("#output");
      await expect(output).not.toBeEmpty({ timeout: 5000 });
    } else if ((await monacoEditor.count()) > 0) {
      // React editor with Monaco
      await monacoEditor.click();
      await page.keyboard.type('fn main() { println("hello test"); }');
      await page.keyboard.press("Control+Enter");

      // Status should show compiled
      const status = page.locator("text=已编译").or(page.locator("text=Compiled"));
      await expect(status).toBeVisible({ timeout: 10000 });
    }
  });

  test("compile error shows in status bar", async ({ page }) => {
    await page.goto(EDITOR_URL);

    const sourceArea = page.locator("textarea#source");
    if ((await sourceArea.count()) > 0) {
      // Enter invalid source
      await sourceArea.fill("this is not valid omni code {{{");
      await page.locator("text=Compile").click();

      // Should show error status (red)
      const errorStatus = page.locator("#status");
      await expect(errorStatus).toContainText(/fail|error|Error/i, { timeout: 5000 });
    }
  });

  test("Ctrl+Enter keyboard shortcut compiles", async ({ page }) => {
    await page.goto(EDITOR_URL);

    const sourceArea = page.locator("textarea#source");
    if ((await sourceArea.count()) > 0) {
      await sourceArea.fill('fn main() { println("kb test"); }');
      await page.keyboard.press("Control+Enter");

      const output = page.locator("#output");
      await expect(output).not.toBeEmpty({ timeout: 5000 });
    }
  });
});
