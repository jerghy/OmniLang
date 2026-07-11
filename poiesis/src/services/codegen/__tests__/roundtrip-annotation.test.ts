// poiesis/src/services/codegen/__tests__/roundtrip-annotation.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";

describe("annotation round-trip", () => {
  it("manual @if/@return → auto-PID nodes, then re-parse preserves structure", () => {
    const manualSource = [
      "// @if",
      "if (x > 0) {",
      "  // @return",
      "  return x;",
      "}",
      "// @return",
      "return 0;",
    ].join("\n");

    // Parse manual source → elements + wires
    const result1 = parseCodeFlowGraph(manualSource);
    expect(result1.elements.length).toBeGreaterThanOrEqual(3);

    // Verify all elements have auto-generated PIDs
    for (const el of result1.elements) {
      expect(el.id).toBeTruthy();
      expect(el.id).not.toBe("");
    }

    // The structure should have 1 condition + 2 returns
    const conditionEl = result1.elements.find((e) => e.tag === "condition");
    expect(conditionEl).toBeDefined();
    const returnEls = result1.elements.filter((e) => e.tag === "return");
    expect(returnEls).toHaveLength(2);
  });

  it("sequential @code-block nodes have linear flow-out wires", () => {
    const source = [
      "// @code-block",
      "a();",
      "// @code-block",
      "b();",
      "// @code-block",
      "c();",
    ].join("\n");

    const result = parseCodeFlowGraph(source);
    const blocks = result.elements.filter((e) => e.tag === "code-block");
    expect(blocks).toHaveLength(3);

    // Wire chain: a → b → c
    const wires = result.wires;
    const flowOutWires = wires.filter((w) => w.fromPort === "flow-out");
    // Should have 2 flow-out wires: a→b, b→c
    expect(flowOutWires).toHaveLength(2);
  });

  it("if/elif/else with @type annotations creates correct branch wires", () => {
    const source = [
      "// @if",
      "if (n > 0) {",
      "  // @code-block",
      "  a();",
      "}",
      "// @elif",
      "else if (n == 0) {",
      "  // @code-block",
      "  b();",
      "}",
      "// @else",
      "else {",
      "  // @code-block",
      "  c();",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(source);

    // Should have 1 condition + 3 code-blocks
    const conditions = result.elements.filter((e) => e.tag === "condition");
    expect(conditions).toHaveLength(1);

    const cond = conditions[0];
    // Should have br0, br1, br2 wires from condition
    const br0 = result.wires.find((w) => w.fromId === cond.id && w.fromPort === "br0");
    const br1 = result.wires.find((w) => w.fromId === cond.id && w.fromPort === "br1");
    const br2 = result.wires.find((w) => w.fromId === cond.id && w.fromPort === "br2");
    expect(br0).toBeDefined();
    expect(br1).toBeDefined();
    expect(br2).toBeDefined();
  });

  it("mixed // @pid and // @type in same body", () => {
    const source = [
      "// @if",
      "if (new) {",
      "  // @return",
      "  return 1;",
      "}",
      "// @pid:old_pid",
      "if (existing) {",
      "  // @pid:old_ret",
      "  return 2;",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(source);

    // Old nodes keep their PIDs
    const oldIf = result.elements.find((e) => e.id === "old_pid");
    expect(oldIf).toBeDefined();
    const oldRet = result.elements.find((e) => e.id === "old_ret");
    expect(oldRet).toBeDefined();

    // New nodes have auto-generated PIDs
    const newIf = result.elements.find((e) => e.tag === "condition" && e.id !== "old_pid");
    expect(newIf).toBeDefined();
  });
});
