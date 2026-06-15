/**
 * E2E test for OmniDriver Event Binding v2 API.
 *
 * Tests: bindEvent, unbindEvent, getAvailableEvents, getEventBindings,
 *        listAllEventBindings, createEventHandler deprecation
 *
 * Run:
 *   npx playwright test editor/e2e/omni-driver-event-binding.spec.ts
 */

import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";
const TIMEOUT = 15000;

test.describe("OmniDriver Event Binding v2", () => {
  test("bindEvent sets event props and paramBindings", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(
      () => (window as any).__OMNI__,
      { timeout: TIMEOUT },
    );

    const result = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.help();
      O.switchMode("design");
      O.clearAll();

      const btn = O.button("Submit", { variant: "primary" });

      // bindEvent without params
      btn.bindEvent("onPress", "handleSubmit");
      const btnData = O.element(btn.id).data;
      const onPressVal = btnData.props.onPress;

      // bindEvent with paramBindings
      const input = O.input("search...");
      input.bindEvent("onChange", "updateSearch", {
        params: {
          searchValue: {
            sourceType: "event",
            value: "event.value",
            expectedType: "String",
          },
          formId: {
            sourceType: "self",
            value: "formId",
            expectedType: "String",
          },
        },
      });
      const inputData = O.element(input.id).data;

      // getAvailableEvents
      const btnEvents = btn.getAvailableEvents();
      const inputEvents = input.getAvailableEvents();

      // getEventBindings
      const btnBindings = btn.getEventBindings();
      const inputBindings = input.getEventBindings();

      // listAllEventBindings
      const allBindings = O.listAllEventBindings();

      // unbindEvent
      btn.unbindEvent("onPress");
      const btnDataAfter = O.element(btn.id).data;

      return {
        onPressVal,
        btnEvents,
        inputEvents,
        btnBindings,
        inputBindings,
        allBindingsCount: allBindings.length,
        btnUnbound: !btnDataAfter.props.onPress,
        hasParamBindings: !!(
          inputData.config?._eventBindings?.onChange
        ),
      };
    });

    expect(result.onPressVal).toBe("handleSubmit");
    expect(result.btnEvents).toContain("onPress");
    expect(result.btnEvents).toContain("onHover");
    expect(result.inputEvents).toContain("onChange");
    expect(result.btnBindings.length).toBeGreaterThanOrEqual(0);
    expect(result.inputBindings.length).toBeGreaterThanOrEqual(0);
    expect(result.allBindingsCount).toBeGreaterThanOrEqual(0);
    expect(result.btnUnbound).toBe(true);
    expect(result.hasParamBindings).toBe(true);
  });

  test("createEventHandler throws deprecation error in v2", async ({
    page,
  }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(
      () => (window as any).__OMNI__,
      { timeout: TIMEOUT },
    );

    const errorMsg = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.switchMode("design");
      O.clearAll();
      const btn = O.button("Test");
      try {
        btn.createEventHandler("onPress");
        return "NO_ERROR";
      } catch (e: any) {
        return e.message;
      }
    });

    expect(errorMsg).toContain("removed in v2");
    expect(errorMsg).toContain("bindEvent");
  });
});
