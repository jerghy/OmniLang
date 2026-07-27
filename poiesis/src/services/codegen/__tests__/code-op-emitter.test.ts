import { describe, it, expect } from "vitest";
import { emitOp, emitOps } from "../code-op-emitter";
import type { CodeOp } from "../../../core/types/code-op";
import type { SlotValue } from "../../../core/types/slot-value";
import { uid } from "../../../core/utils/id";

// Helper: create a text SlotValue from a plain string
const tv = (s: string): SlotValue => ({ kind: "text", value: s });

describe("emitOp", () => {
  it("emits let declaration with type and init", () => {
    const op: CodeOp = {
      kind: "declare_var", pid: uid("test"),
      mutable: false, name: tv("sum"), varType: tv("f64"), initValue: tv("0.0"),
      scope: "fn",
    };
    expect(emitOp(op, 0)).toBe("let sum: f64 = 0.0;\n");
  });

  it("emits let mut without type (inferred)", () => {
    const op: CodeOp = {
      kind: "declare_var", pid: uid("test"),
      mutable: true, name: tv("i"), initValue: tv("0"),
      scope: "fn",
    };
    expect(emitOp(op, 0)).toBe("let mut i = 0;\n");
  });

  it("emits set_var with += operator", () => {
    const op: CodeOp = {
      kind: "set_var", pid: uid("test"),
      name: tv("count"), operator: "+=", value: tv("1"),
    };
    expect(emitOp(op, 0)).toBe("count += 1;\n");
  });

  it("emits while loop with nested body", () => {
    const op: CodeOp = {
      kind: "while", pid: uid("test"),
      condition: tv("i < n"),
      body: [{
        kind: "set_var", pid: uid("test"),
        name: tv("sum"), operator: "+=", value: tv("i"),
      }],
      bodyId: "test-body",
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
        { condition: tv("x > 0"), body: [{ kind: "return", pid: uid("test"), value: tv("1") }] },
        { condition: tv(""), body: [{ kind: "return", pid: uid("test"), value: tv("0") }] },
      ],
      bodyId: "test-body",
    };
    const result = emitOp(op, 0);
    expect(result).toContain("if x > 0 {");
    expect(result).toContain("  return 1;");
    expect(result).toContain("} else {");
    expect(result).toContain("  return 0;");
  });

  it("emits return with and without value", () => {
    expect(emitOp({ kind: "return", pid: uid("test"), value: tv("x") }, 0))
      .toBe("return x;\n");
    expect(emitOp({ kind: "return", pid: uid("test") }, 0))
      .toBe("return;\n");
  });
});

describe("emitOps", () => {
  it("joins multiple ops", () => {
    const ops: CodeOp[] = [
      { kind: "declare_var", pid: uid("test"), mutable: false, name: tv("x"), initValue: tv("1"), scope: "fn" },
      { kind: "return", pid: uid("test"), value: tv("x") },
    ];
    expect(emitOps(ops)).toBe("let x = 1;\nreturn x;\n");
  });
});
