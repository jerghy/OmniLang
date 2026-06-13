/**
 * OmniLang Canvas Diagnostic & Fix Script — Counter App
 *
 * Diagnoses and fixes a broken "Counter" visual editor canvas where:
 * 1. Event port uses event name "onPress" instead of UUID "evt-btn-press-001"
 * 2. Signal connection targets "value" (output) instead of "set" (input port)
 * 3. Handler has empty source code (no increment logic)
 * 4. No validate() was called to detect issues
 *
 * Usage: Paste this entire script into the browser DevTools console
 *        after the OmniLang editor app has loaded.
 */

(function fixCounterCanvas() {
  "use strict";

  const O = window.__OMNI__;
  if (!O) {
    console.error("window.__OMNI__ not found — is the editor loaded?");
    return;
  }

  // ═══════════════════════════════════════════════════════════
  // Phase 1: Diagnose
  // ═══════════════════════════════════════════════════════════
  console.log("%c🧪 Phase 1: Canvas Diagnosis", "font-size:16px;font-weight:bold");

  // 1a. Learn the API (prints port reference and event UUID table)
  const apiHelp = O.help();
  console.log("API Reference — event UUID table:", apiHelp.eventUuidTable);
  console.log("API Reference — signal ports:", apiHelp.portReference.signal);

  // 1b. Inspect current state
  const state = O.getState();
  console.log("=== Elements on Canvas ===");
  state.elements.forEach(function (el) {
    var label = el.props.label || el.props.name || "(unnamed)";
    var srcPreview = el.source
      ? (el.source.length > 40 ? el.source.slice(0, 40) + "..." : el.source)
      : "EMPTY";
    console.log(
      "  [" + el.id + "] <" + el.tag + "> (" + el.nodeType + ') label="' + label + '" source="' + srcPreview + '"'
    );
  });

  console.log("=== Wires on Canvas ===");
  state.wires.forEach(function (w) {
    var fromTag = (state.elements.find(function (e) { return e.id === w.fromId; }) || {}).tag || "?";
    var toTag = (state.elements.find(function (e) { return e.id === w.toId; }) || {}).tag || "?";
    console.log("  [" + w.id + "] " + fromTag + "." + w.fromPort + " → " + toTag + "." + w.toPort);
  });

  // 1c. Run validation
  var initialReport = O.validate();
  console.log("=== Initial Validation Report ===", JSON.stringify(initialReport, null, 2));

  if (initialReport.isValid && initialReport.unwiredEvents.length === 0 && initialReport.unreachableBlocks.length === 0) {
    console.log("%c✓ Canvas appears healthy — nothing to fix.", "font-size:14px;color:green");
    return;
  }

  // ═══════════════════════════════════════════════════════════
  // Phase 2: Fix all bugs
  // ═══════════════════════════════════════════════════════════
  console.log("\n%c🔧 Phase 2: Fixing Canvas Bugs", "font-size:16px;font-weight:bold");

  var els = O.getElements();
  var btn = els.find(function (e) { return e.tag === "Button"; });
  var handlerNode = els.find(function (e) { return e.tag === "action"; });
  var signalNode = els.find(function (e) { return e.tag === "signal"; });
  var displayNode = els.find(function (e) { return e.tag === "Text"; });

  // --- BUG 1: Wrong event port (using event name "onPress" instead of UUID "evt-btn-press-001") ---
  var wireToHandler = O.getWires().find(function (w) {
    return w.fromPort === "onPress" && w.toPort === "call";
  });
  if (wireToHandler) {
    console.log("BUG 1: Event port uses \"onPress\" instead of UUID \"evt-btn-press-001\"");
    O.disconnect(wireToHandler.id);
    console.log("  ✓ Disconnected faulty wire " + wireToHandler.id);
  } else {
    console.log("BUG 1: No wire found with onPress — may already be fixed or missing.");
  }

  // --- BUG 2: Wrong target port (connecting to "value" (output) instead of "set" (input)) ---
  var wireToSignal = O.getWires().find(function (w) {
    return w.toPort === "value";
  });
  if (wireToSignal) {
    console.log("BUG 2: Signal connection targets \"value\" (output) instead of \"set\" (input)");
    O.disconnect(wireToSignal.id);
    console.log("  ✓ Disconnected faulty wire " + wireToSignal.id);
  } else {
    console.log("BUG 2: No wire found targeting \"value\" — may already be fixed or missing.");
  }

  // --- Reconnect wire 1: Button event → Handler call (idempotent) ---
  if (btn && handlerNode) {
    var existingWire1 = O.getWires().find(function (w) {
      return w.fromId === btn.id && w.fromPort === "evt-btn-press-001" && w.toId === handlerNode.id;
    });
    if (!existingWire1) {
      O.connect(btn.id, "evt-btn-press-001", handlerNode.id, "call");
      console.log("✓ Wire: Button(\"" + (btn.props.label || "") + "\").evt-btn-press-001 → action(\"" + (handlerNode.props.name || "") + "\").call");
    } else {
      console.log("✓ Wire already exists: Button → action (evt-btn-press-001 → call)");
    }
  } else {
    console.warn("  ✗ Cannot create wire 1 — Button or handler element not found.");
  }

  // --- Reconnect wire 2: Handler result → Signal set (idempotent) ---
  if (handlerNode && signalNode) {
    var existingWire2 = O.getWires().find(function (w) {
      return w.fromId === handlerNode.id && w.fromPort === "result" && w.toId === signalNode.id;
    });
    if (!existingWire2) {
      O.connect(handlerNode.id, "result", signalNode.id, "set");
      console.log("✓ Wire: action(\"" + (handlerNode.props.name || "") + "\").result → signal(\"" + (signalNode.props.name || "") + "\").set");
    } else {
      console.log("✓ Wire already exists: handler → signal (result → set)");
    }
  } else {
    console.warn("  ✗ Cannot create wire 2 — handler or signal element not found.");
  }

  // --- BUG 3: Empty handler source code ---
  if (handlerNode && (!handlerNode.source || handlerNode.source.trim() === "")) {
    console.log("BUG 3: Handler has empty source code");
    var INCREMENT_SOURCE = [
      "action increment(event) {",
      "  // Read current count, increment by 1, send new value to signal",
      "  return count + 1;",
      "}"
    ].join("\n");
    O.setSource(handlerNode.id, INCREMENT_SOURCE);
    console.log("  ✓ Source code set to:\n" + INCREMENT_SOURCE);
  } else if (handlerNode) {
    console.log("BUG 3: Handler already has source — skipping.");
  } else {
    console.warn("  ✗ Cannot set source — handler element not found.");
  }

  // --- Additional fix: Bind display Text to signal via interpolation ---
  if (displayNode) {
    var currentLabel = displayNode.props.label;
    if (currentLabel === "{count}") {
      console.log("Display already bound to {count} — skipping.");
    } else {
      console.log("Additional: Binding display label from \"" + currentLabel + "\" to \"{count}\" for signal interpolation");
      O.setProp(displayNode.id, "label", "{count}");
      console.log("  ✓ Text label updated to \"{count}\"");
    }
  } else {
    console.warn("  ✗ Cannot update display — Text element not found.");
  }

  // ═══════════════════════════════════════════════════════════
  // Phase 3: Verify
  // ═══════════════════════════════════════════════════════════
  console.log("\n%c✅ Phase 3: Verification", "font-size:16px;font-weight:bold");

  var finalState = O.getState();

  console.log("=== Final Elements ===");
  finalState.elements.forEach(function (el) {
    if (el.nodeType === "codeblock") {
      console.log(
        "  [" + el.id + "] <" + el.tag + "> name=\"" + (el.props.name || "") + "\" source=" + (el.source ? "SET ✓" : "EMPTY ✗")
      );
    } else if (el.tag === "Text") {
      console.log("  [" + el.id + "] <Text> label=\"" + el.props.label + "\"");
    }
  });

  console.log("=== Final Wires ===");
  finalState.wires.forEach(function (w) {
    var fromTag = (finalState.elements.find(function (e) { return e.id === w.fromId; }) || {}).tag || "?";
    var toTag = (finalState.elements.find(function (e) { return e.id === w.toId; }) || {}).tag || "?";
    console.log("  [" + w.id + "] " + fromTag + "." + w.fromPort + " → " + toTag + "." + w.toPort);
  });

  // BUG 4: Call validate() to verify the fix
  var finalReport = O.validate();
  console.log("=== Final Validation Report ===", JSON.stringify(finalReport, null, 2));

  if (finalReport.isValid && finalReport.unwiredEvents.length === 0) {
    console.log("%c🎉 SUCCESS: All bugs fixed! Canvas is healthy.", "font-size:16px;font-weight:bold;color:green");
  } else {
    console.log("%c⚠️  Canvas still has issues:", "font-size:14px;font-weight:bold;color:orange");
    if (finalReport.orphanElements.length > 0) {
      console.log("  - Orphan elements:", finalReport.orphanElements);
    }
    if (finalReport.unwiredEvents.length > 0) {
      console.log("  - Unwired events:", finalReport.unwiredEvents);
    }
    if (finalReport.typeMismatches.length > 0) {
      console.log("  - Type mismatches:", finalReport.typeMismatches);
    }
    if (finalReport.unreachableBlocks.length > 0) {
      console.log("  - Unreachable blocks:", finalReport.unreachableBlocks);
    }
  }

  console.log("\n%c📋 Summary of Changes:", "font-size:14px;font-weight:bold");
  console.log("  Bug 1: Event port 'onPress' → 'evt-btn-press-001' (UUID)");
  console.log("  Bug 2: Signal target 'value' → 'set' (input port)");
  console.log("  Bug 3: Handler source set with increment logic");
  console.log("  Bug 4: O.validate() called to verify canvas health");
  console.log("  Extra: Text label '0' → '{count}' for signal interpolation");
})();
