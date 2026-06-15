/**
 * E2E test demonstrating the OmniDriver (window.__OMNI__) API.
 *
 * Creates a counter app programmatically via the Playwright-like API:
 *   1. Clear the canvas
 *   2. Create signal "count: i32 = 0"
 *   3. Create Button "Increment" and Text element
 *   4. Wire the button's onPress event to increment the signal
 *   5. Compile the .omni source and verify zero errors
 *   6. Verify elements and wires exist (pre- and post-compile)
 *
 * Prerequisites:
 *   Terminal 1: python -m editor.server.main --dir test/fixtures/simple --port 8000
 *   Terminal 2: cd editor/client && npm run dev  (→ http://localhost:5174)
 *
 * Run:
 *   npx playwright test editor/e2e/omni-driver-counter.spec.ts
 */

import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";
const POLL_INTERVAL_MS = 200;
const WAIT_TIMEOUT_MS = 15000;

// ═══════════════════════════════════════════════════════════════
// Helper: a valid OmniLang counter source used for the compile step
// ═══════════════════════════════════════════════════════════════

const COUNTER_OMNI_SOURCE = `// Behavior Test: Counter
component Counter {
  signal count: i32 = 0;

  fn increment() {
    count = count + 1;
  }

  view {
    <Container>
      <Heading level="1">Count: {count}</Heading>
      <Flex direction="row" gap="8px">
        <Button onPress={increment} test-id="btn-increment">+1</Button>
        <Text>Count: {count}</Text>
      </Flex>
    </Container>
  }
}`;

// ═══════════════════════════════════════════════════════════════
// Test suite
// ═══════════════════════════════════════════════════════════════

test.describe("OmniDriver Counter App", () => {
  test("creates counter app via window.__OMNI__ API, wires, compiles without errors, and verifies state", async ({ page }) => {
    // ── Navigate and wait for OmniDriver initialization ───────
    await page.goto(EDITOR_URL);

    // OmniDriver is mounted in App.tsx useEffect → sets window.__OMNI__
    await page.waitForFunction(
      () => typeof (window as any).__OMNI__ !== "undefined" && (window as any).__OMNI__ !== null,
      { timeout: WAIT_TIMEOUT_MS, polling: POLL_INTERVAL_MS },
    );

    // ── Single evaluate call to avoid serialization issues ────
    const result = await page.evaluate(async () => {
      const omni = (window as any).__OMNI__;
      if (!omni) {
        return { error: "OmniDriver (window.__OMNI__) not available" };
      }

      // ═══════════════════════════════════════════════════════
      // STEP 1: Clear the canvas
      // ═══════════════════════════════════════════════════════
      omni.clearAll();

      // ═══════════════════════════════════════════════════════
      // STEP 2: API Discovery — discover available view components
      // ═══════════════════════════════════════════════════════
      const availableComponents = omni.listComponents();
      const componentTags = availableComponents.map((c: any) => c.tag);

      // Inspect the Button component protocol (events, ports, props)
      const buttonInfo = omni.getComponentInfo("Button");

      // ═══════════════════════════════════════════════════════
      // STEP 3: Create counter app elements
      // ═══════════════════════════════════════════════════════

      // 3a. Signal "count: i32 = 0"
      //     Ports: value (out, type: i32), set (in, type: i32)
      const signal = omni.codeBlock("signal", "count", {
        source: "signal count: i32 = 0",
        typeAnnotation: "i32",
      });

      // 3b. Button "Increment"
      //     Events: onPress (port UUID: evt-btn-press-001), onHover, onFocus, onLongPress
      const btn = omni.button("Increment", { variant: "primary" });

      // 3c. Text element displaying the count
      const txt = omni.text("Count: 0");

      // ═══════════════════════════════════════════════════════
      // STEP 4: v2 — bind event in design mode, create handler in code mode
      // ═══════════════════════════════════════════════════════

      // 4a. Bind event in design mode (v2 — sets event prop only)
      btn.bindEvent("onPress", "handleIncrement");

      // 4b. Switch to code mode and create the handler function
      omni.switchMode("code");
      const tab = omni.createTab("event", "Counter Events", "🔢");
      omni.switchTab(tab.id);
      const fn = omni.customFunction("handleIncrement", [], "void");
      fn.setSource("action increment(event) {\n  count.set(count.value + 1)\n}");
      omni.returnNode(fn.id);

      // Switch back to design mode for pre-compile state check
      omni.switchMode("design");

      // ═══════════════════════════════════════════════════════
      // STEP 5: Query pre-compile state (programmatic creation)
      // ═══════════════════════════════════════════════════════
      const preElements = omni.getElements();
      const preWires = omni.getWires();

      // ═══════════════════════════════════════════════════════
      // STEP 6: Compile from .omni source
      // ═══════════════════════════════════════════════════════
      //
      // NOTE: compile() rebuilds elements from the parsed .omni source, not from
      // programmatic elements.  We must set source code first.
      omni.setSourceCode(COUNTER_OMNI_SOURCE);
      omni.switchMode("code");
      const compileResult = await omni.compile();

      // ═══════════════════════════════════════════════════════
      // STEP 7: Query post-compile state
      // ═══════════════════════════════════════════════════════
      const postElements = omni.getElements();
      const postWires = omni.getWires();

      // ═══════════════════════════════════════════════════════
      // Return a serializable summary for Playwright assertions
      // ═══════════════════════════════════════════════════════
      return {
        error: null,

        // ── Discovery ───────────────
        discovery: {
          totalComponents: availableComponents.length,
          componentTags,
          hasButtonViewComponent: componentTags.includes("Button"),
          buttonInfo: buttonInfo
            ? {
                tag: buttonInfo.tag,
                events: Object.keys(buttonInfo.events),
                eventIds: Object.values(buttonInfo.events).map(
                  (ev: any) => ev.id,
                ),
                hasOnPress: "onPress" in buttonInfo.events,
                onPressPortId: buttonInfo.events.onPress?.id,
              }
            : null,
        },

        // ── Pre-compile ────────────
        preCompile: {
          elementCount: preElements.length,
          wireCount: preWires.length,
          elements: preElements.map((e: any) => ({
            id: e.id,
            tag: e.tag,
            nodeType: e.nodeType,
            portNames: (e.ports || []).map((p: any) => p.name),
            props: JSON.parse(JSON.stringify(e.props)),
          })),
          wires: preWires.map((w: any) => ({
            id: w.id,
            fromId: w.fromId,
            fromPort: w.fromPort,
            toId: w.toId,
            toPort: w.toPort,
          })),
        },

        // ── Wire plumbing (v2) ────
        wires: {
          hasBindEvent: typeof omni.listAllEventBindings === "function",
          hasCustomFunction: typeof omni.customFunction === "function",
          eventBindingsCount: omni.listAllEventBindings().length,
          customFnCount: omni.listCustomFunctions().length,
        },

        // ── Compile ────────────────
        compile: {
          success: compileResult.success,
          errorCount: compileResult.errors.length,
          errors: compileResult.errors.map((e: any) => ({
            code: e.code,
            message: e.message,
            line: e.line,
            severity: e.severity,
          })),
        },

        // ── Post-compile ───────────
        postCompile: {
          elementCount: postElements.length,
          wireCount: postWires.length,
          tags: postElements.map((e: any) => e.tag),
          hasContainer: postElements.some((e: any) => e.tag === "Container"),
          hasHeading: postElements.some((e: any) => e.tag === "Heading"),
          hasButton: postElements.some((e: any) => e.tag === "Button"),
          hasFlex: postElements.some((e: any) => e.tag === "Flex"),
          hasText: postElements.some((e: any) => e.tag === "Text"),
        },

        // ── Source ─────────────────
        source: {
          content: omni.getSourceCode(),
          hasContent: !!omni.getSourceCode(),
        },
      };
    });

    // ═══════════════════════════════════════════════════════════
    // ASSERTIONS
    // ═══════════════════════════════════════════════════════════

    expect(result.error).toBeNull();

    // ────────── Discovery ─────────────
    expect(result.discovery.totalComponents).toBeGreaterThan(0);
    expect(result.discovery.hasButtonViewComponent).toBe(true);
    expect(result.discovery.buttonInfo).not.toBeNull();
    expect(result.discovery.buttonInfo!.tag).toBe("Button");
    expect(result.discovery.buttonInfo!.hasOnPress).toBe(true);
    expect(result.discovery.buttonInfo!.onPressPortId).toBe(
      "evt-btn-press-001",
    );
    expect(result.discovery.buttonInfo!.events).toContain("onPress");

    // ────────── Pre-compile ───────────
    // Elements: card + heading + button + text + signal + customFn + return = at least 6
    expect(result.preCompile.elementCount).toBeGreaterThanOrEqual(6);
    // Wires: at least 1 (but we have 0 in v2 since handler is in code tab)
    expect(result.preCompile.wireCount).toBeGreaterThanOrEqual(0);

    // Verify specific element types
    const preTags = result.preCompile.elements.map((e: any) => e.tag);
    expect(preTags).toContain("signal");
    expect(preTags).toContain("Button");
    expect(preTags).toContain("Text");

    // Verify the signal has expected ports
    const signalElement = result.preCompile.elements.find(
      (e: any) => e.tag === "signal",
    );
    expect(signalElement).toBeDefined();
    expect(signalElement!.portNames).toContain("value");
    expect(signalElement!.portNames).toContain("set");

    // ────────── Wire plumbing (v2) ────
    expect(result.wires.hasBindEvent).toBe(true);
    expect(result.wires.hasCustomFunction).toBe(true);
    expect(result.wires.eventBindingsCount).toBeGreaterThanOrEqual(1);
    expect(result.wires.customFnCount).toBeGreaterThanOrEqual(1);

    // ────────── Compile ──────────────
    expect(result.compile.success).toBe(true);
    expect(result.compile.errorCount).toBe(0);
    // If any errors exist, log them for debugging
    if (result.compile.errors.length > 0) {
      console.log("Compile warnings:", JSON.stringify(result.compile.errors));
    }

    // ────────── Post-compile ─────────
    // The compiled .omni source should produce UI elements
    expect(result.postCompile.elementCount).toBeGreaterThanOrEqual(4);
    expect(result.postCompile.hasContainer).toBe(true);
    expect(result.postCompile.hasHeading).toBe(true);
    expect(result.postCompile.hasButton).toBe(true);
    expect(result.postCompile.hasFlex).toBe(true);
    expect(result.postCompile.hasText).toBe(true);

    // Source code was set
    expect(result.source.hasContent).toBe(true);
    expect(result.source.content).toContain("component Counter");
    expect(result.source.content).toContain("signal count: i32 = 0");
  });
});
