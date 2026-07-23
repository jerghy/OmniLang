// poiesis/src/components/canvas/code/lexical/__tests__/integration-v2.test.ts
import { describe, it, expect } from "vitest";
import { convertToNodes } from "../transforms/convertToNodes";
import { convertToOps } from "../transforms/convertToOps";
import { emitOps } from "../../../../../services/codegen/code-op-emitter";
import { parseOps } from "../../../../../services/codegen/code-op-parser";
import type { CodeOp } from "../../../../../core/types/code-op";

describe("CodeCard v2: full pipeline roundtrip", () => {
  const testRoundtrip = (ops: CodeOp[]) => {
    // CodeOp → EditorNode → CodeOp
    const nodes = convertToNodes(ops);
    const back = convertToOps(nodes);

    // Compare semantic fields (pids will differ for auto-generated)
    expect(back.length).toBe(ops.length);
    for (let i = 0; i < ops.length; i++) {
      expect(back[i].kind).toBe(ops[i].kind);
    }

    // Emit should still produce valid source
    const source = emitOps(back);
    const reparsed = parseOps(source);
    expect(reparsed.length).toBeGreaterThanOrEqual(ops.length);
  };

  it("declare_var roundtrips through EditorNode", () => {
    testRoundtrip([
      { kind: "declare_var", pid: "d1", mutable: true, name: "count", varType: "i32", initValue: "0", scope: "fn" },
    ]);
  });

  it("set_var roundtrips through EditorNode", () => {
    testRoundtrip([
      { kind: "set_var", pid: "s1", name: "total", operator: "+=", value: "1" },
    ]);
  });

  it("if_block with elif/else roundtrips", () => {
    testRoundtrip([{
      kind: "if_block", pid: "if1",
      branches: [
        { condition: "x > 0", body: [{ kind: "return", pid: "r1", value: "1" }] },
        { condition: "x < 0", body: [{ kind: "return", pid: "r2", value: "-1" }] },
        { condition: "", body: [{ kind: "return", pid: "r3", value: "0" }] },
      ],
    }]);
  });

  it("while loop roundtrips", () => {
    testRoundtrip([{
      kind: "while", pid: "w1", condition: "i < n",
      body: [
        { kind: "set_var", pid: "s1", name: "sum", operator: "+=", value: "i" },
        { kind: "set_var", pid: "s2", name: "i", operator: "+=", value: "1" },
      ],
    }]);
  });

  it("for_in roundtrips", () => {
    testRoundtrip([{
      kind: "for_in", pid: "f1", itemName: "item", collection: "items",
      body: [{ kind: "break", pid: "b1" }],
    }]);
  });

  it("complex real-world code roundtrips", () => {
    const source = `let sum = 0.0;
let mut i = 0;
while i < values.len() {
    sum = sum + values.get(i);
    i = i + 1;
}
if count != 0 {
    return sum / count;
}
return 0.0;`;
    const ops = parseOps(source);
    const nodes = convertToNodes(ops);
    const back = convertToOps(nodes);
    const reEmitted = emitOps(back);

    // Verify key elements are present
    expect(reEmitted).toContain("let sum = 0.0");
    expect(reEmitted).toContain("while i < values.len()");
    expect(reEmitted).toContain("return");
  });
});
