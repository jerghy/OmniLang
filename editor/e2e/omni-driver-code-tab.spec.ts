/**
 * E2E test for OmniDriver Code Tab Management API.
 *
 * Tests: createTab, switchTab, removeTab, listTabs, getActiveTab,
 *        getTab, cloneTab, exportTab, importTab, addVariable,
 *        getVariables, removeVariable, getTabElements, getTabWires
 *
 * Run:
 *   npx playwright test editor/e2e/omni-driver-code-tab.spec.ts
 */

import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";
const TIMEOUT = 15000;

test.describe("OmniDriver Code Tab Management", () => {
  test("creates, lists, switches, clones, and removes tabs", async ({
    page,
  }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(
      () => (window as any).__OMNI__,
      { timeout: TIMEOUT },
    );

    const result = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.help();
      O.switchMode("code");

      // Create tabs
      const logicTab = O.createTab("logic", "Main Logic", "📄");
      const eventTab = O.createTab("event", "Button Events", "🔘");
      O.createTab("event", "Form Events");

      // List tabs
      const allTabs = O.listTabs();

      // Switch tab
      O.switchTab(eventTab.id);
      const activeAfterSwitch = O.getActiveTab();

      // Add variables with scope
      O.addVariable(eventTab.id, "clickCount", "i32", 0, "点击计数");
      O.addVariable(eventTab.id, "lastEvent", "String", "");
      const vars = O.getVariables(eventTab.id);

      // Clone tab
      const cloneResult = O.cloneTab(eventTab.id, "Cloned Events");
      const cloneVars = O.getVariables(cloneResult!.id);

      // Export / Import roundtrip
      const exported = O.exportTab("main");
      const imported = O.importTab(exported!);

      // Remove tab
      O.removeTab(logicTab.id);
      const tabsAfterRemove = O.listTabs();

      // getTab / getTabElements / getTabWires
      O.switchTab("main");
      O.codeBlock("signal", "testSig", { typeAnnotation: "i32" });
      const elements = O.getTabElements("main");
      const mainTab = O.getTab("main");

      return {
        allTabsCount: allTabs.length,
        activeAfterSwitchId: activeAfterSwitch?.id,
        activeAfterSwitchLabel: activeAfterSwitch?.label,
        varsCount: vars.length,
        varsFirstName: vars[0]?.name,
        cloneVarsCount: cloneVars.length,
        exportValid: exported !== null && exported.length > 0,
        importValid: imported !== null,
        tabsAfterRemoveCount: tabsAfterRemove.length,
        elementsNonEmpty: elements.length > 0,
        mainTabElementCount: mainTab?.elementCount,
      };
    });

    expect(result.allTabsCount).toBeGreaterThanOrEqual(4); // main + 3 new
    expect(result.activeAfterSwitchId).toBeTruthy();
    expect(result.activeAfterSwitchLabel).toBe("Button Events");
    expect(result.varsCount).toBe(2);
    expect(result.varsFirstName).toBe("clickCount");
    expect(result.cloneVarsCount).toBe(2);
    expect(result.exportValid).toBe(true);
    expect(result.importValid).toBe(true);
    expect(result.tabsAfterRemoveCount).toBeGreaterThanOrEqual(3);
    expect(result.elementsNonEmpty).toBe(true);
    expect(result.mainTabElementCount).toBeGreaterThanOrEqual(1);
  });
});
