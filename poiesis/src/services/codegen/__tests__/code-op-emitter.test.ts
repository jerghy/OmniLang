import { describe, it, expect } from "vitest";
import { emitOp, emitOps } from "../code-op-emitter";
import type { CodeOp } from "../../../core/types/code-op";
import { uid } from "../../../core/utils/id";

describe("emitOp", () => {
  it("emits let declaration with type and init", () => {
    const op: CodeOp = {
      kind: "declare_var", pid: uid("test"),
      mutable: false, name: "sum", varType: "f64", initValue: "0.0",
      scope: "fn",
    };
    expect(emitOp(op, 0)).toBe("let sum: f64 = 0.0;\n");
  });

  it("emits let mut without type (inferred)", () => {
    const op: CodeOp = {
      kind: "declare_var", pid: uid("test"),
      mutable: true, name: "i", initValue: "0",
      scope: "fn",
    };
    expect(emitOp(op, 0)).toBe("let mut i = 0;\n");
  });

  it("emits set_var with += operator", () => {
    const op: CodeOp = {
      kind: "set_var", pid: uid("test"),
      name: "count", operator: "+=", value: "1",
    };
    expect(emitOp(op, 0)).toBe("count += 1;\n");
  });

  it("emits while loop with nested body", () => {
    const op: CodeOp = {
      kind: "while", pid: uid("test"),
      condition: "i < n",
      body: [{
        kind: "set_var", pid: uid("test"),
        name: "sum", operator: "+=", value: "i",
      }],
    };
    const result = emitOp(op, 0);
    expect(result).toContain("while i < n {");
    expect(result).toContain("  sum += i;");
    expect(result).toContain("}");
  });

  it("emits if/else block", () => {
    const op: CodeOp = {
      kind: "if_block", pid: uid("test"),
      branches: [
        { condition: "x > 0", body: [{ kind: "return", pid: uid("test"), value: "1" }] },
        { condition: "", body: [{ kind: "return", pid: uid("test"), value: "0" }] },
      ],
    };
    const result = emitOp(op, 0);
    expect(result).toContain("if x > 0 {");
    expect(result).toContain("  return 1;");
    expect(result).toContain("} else {");
    expect(result).toContain("  return 0;");
  });

  it("emits return with and without value", () => {
    expect(emitOp({ kind: "return", pid: uid("test"), value: "x" }, 0))
      .toBe("return x;\n");
    expect(emitOp({ kind: "return", pid: uid("test") }, 0))
      .toBe("return;\n");
  });
});

describe("emitOps", () => {
  it("joins multiple ops", () => {
    const ops: CodeOp[] = [
      { kind: "declare_var", pid: uid("test"), mutable: false, name: "x", initValue: "1", scope: "fn" },
      { kind: "return", pid: uid("test"), value: "x" },
    ];
    expect(emitOps(ops)).toBe("let x = 1;\nreturn x;\n");
  });
});
