// poiesis/src/components/canvas/code/lexical/__tests__/integration.test.ts
import { describe, it, expect } from "vitest";
import { emitOps } from "../../../../../services/codegen/code-op-emitter";
import { parseOps } from "../../../../../services/codegen/code-op-parser";
import type { CodeOp } from "../../../../../core/types/code-op";

describe("CodeCard Lexical Integration: emit ↔ parse roundtrip", () => {
  it("declare_var roundtrips", () => {
    const ops: CodeOp[] = [
      { kind: "declare_var", pid: "op1", mutable: false, name: "sum", varType: "f64", initValue: "0.0", scope: "fn" },
    ];
    const source = emitOps(ops);
    expect(source).toContain("let sum: f64 = 0.0");

    const reparsed = parseOps(source);
    expect(reparsed.length).toBe(1);
    expect(reparsed[0].kind).toBe("declare_var");
    if (reparsed[0].kind === "declare_var") {
      expect(reparsed[0].name).toBe("sum");
      expect(reparsed[0].varType).toBe("f64");
      expect(reparsed[0].initValue).toBe("0.0");
    }
  });

  it("set_var roundtrips", () => {
    const ops: CodeOp[] = [
      { kind: "set_var", pid: "op2", name: "count", operator: "=", value: "0" },
    ];
    const source = emitOps(ops);
    expect(source).toContain("count = 0");

    const reparsed = parseOps(source);
    expect(reparsed.length).toBe(1);
    expect(reparsed[0].kind).toBe("set_var");
  });

  it("while loop roundtrips", () => {
    const ops: CodeOp[] = [{
      kind: "while",
      pid: "op3",
      condition: "i < n",
      body: [
        { kind: "set_var", pid: "op4", name: "sum", operator: "+=", value: "i" },
      ],
    }];
    const source = emitOps(ops);
    expect(source).toContain("while i < n");
    expect(source).toContain("sum += i");

    const reparsed = parseOps(source);
    expect(reparsed.length).toBe(1);
    expect(reparsed[0].kind).toBe("while");
  });

  it("multiple ops in sequence", () => {
    const ops: CodeOp[] = [
      { kind: "declare_var", pid: "a1", mutable: false, name: "x", varType: "i32", initValue: "0", scope: "fn" },
      { kind: "declare_var", pid: "a2", mutable: true, name: "y", initValue: "1", scope: "fn" },
      { kind: "set_var", pid: "a3", name: "x", operator: "=", value: "y + 1" },
      { kind: "return", pid: "a4", value: "x" },
    ];
    const source = emitOps(ops);
    const reparsed = parseOps(source);
    expect(reparsed.length).toBe(4);
  });
});
