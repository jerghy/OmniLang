// poiesis/src/services/codegen/__tests__/conditional-roundtrip.test.ts
// Round-trip integration test: parse -> emit -> parse, verifying conditional
// blocks survive without data loss.

import { describe, it, expect } from "vitest";
import { tokenize } from "../omni-tokenizer";
import { TokenStream } from "../token-stream";
import { Parser } from "../parser";
import { generateOmniSource } from "../pipeline";
import type { ConditionalBlockData } from "../../../core/types/element";

describe("conditional rendering round-trip", () => {
  it("parse -> emit -> parse produces identical conditional blocks", () => {
    const source = `
import omniview;
component Test {
  signal status: String = "loading";

  view {
    @if status == "loading" {
      <div id="a">Loading</div>
    } @else if status == "error" {
      <div id="b">Error</div>
    } @else {
      <div id="c">OK</div>
    }
  }
}`;

    // First parse
    const result1 = new Parser(
      new TokenStream(tokenize(source))
    ).parse();

    expect(result1.errors.filter(e => e.severity === "error")).toHaveLength(0);
    const cond1 = result1.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(cond1).toBeDefined();
    expect(cond1!.branches).toHaveLength(3);

    // Emit
    const emitResult = generateOmniSource({
      designElements: result1.designElements ?? [],
      designWires: [],
      codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "", graph: { elements: [], wires: [] }, variables: [] }],
      componentName: "Test",
      rootOrder: undefined,
      inlineStores: [],
    });

    expect(emitResult.errors.filter(e => e.severity === "fatal")).toHaveLength(0);
    expect(emitResult.value).toBeDefined();

    // Second parse
    const result2 = new Parser(
      new TokenStream(tokenize(emitResult.value!))
    ).parse();

    expect(result2.errors.filter(e => e.severity === "error")).toHaveLength(0);
    const cond2 = result2.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(cond2).toBeDefined();
    expect(cond2!.branches).toHaveLength(3);

    // Verify branch conditions match
    expect(cond2!.branches[0].condition).toBe(cond1!.branches[0].condition);
    expect(cond2!.branches[1].condition).toBe(cond1!.branches[1].condition);
    expect(cond2!.branches[2].condition).toBe(cond1!.branches[2].condition);

    // Verify branch types match
    expect(cond2!.branches[0].type).toBe(cond1!.branches[0].type);
    expect(cond2!.branches[1].type).toBe(cond1!.branches[1].type);
    expect(cond2!.branches[2].type).toBe(cond1!.branches[2].type);
  });
});
