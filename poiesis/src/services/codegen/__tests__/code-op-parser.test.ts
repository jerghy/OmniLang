import { describe, it, expect } from "vitest";
import { parseOps, detectOpKind } from "../code-op-parser";

describe("detectOpKind", () => {
  it("detects let declaration", () => {
    expect(detectOpKind("let x = 1;")).toBe("declare_var");
  });
  it("detects let mut", () => {
    expect(detectOpKind("let mut i = 0;")).toBe("declare_var");
  });
  it("detects if", () => {
    expect(detectOpKind("if x > 0 {")).toBe("if_block");
  });
  it("detects while", () => {
    expect(detectOpKind("while i < n {")).toBe("while");
  });
  it("detects for in", () => {
    expect(detectOpKind("for item in items {")).toBe("for_in");
  });
  it("detects return", () => {
    expect(detectOpKind("return x;")).toBe("return");
  });
  it("detects function call", () => {
    expect(detectOpKind('console_log("hi");')).toBe("call_fn");
  });
  it("detects assignment", () => {
    expect(detectOpKind("count += 1;")).toBe("set_var");
  });
  it("falls back to raw", () => {
    expect(detectOpKind("}")).toBe("raw");
  });
});

describe("parseOps", () => {
  it("parses simple sequence", () => {
    const source = "let x = 1;\nreturn x;\n";
    const ops = parseOps(source);
    expect(ops).toHaveLength(2);
    expect(ops[0].kind).toBe("declare_var");
    if (ops[0].kind === "declare_var") {
      expect(ops[0].name).toBe("x");
      expect(ops[0].initValue).toBe("1");
    }
    expect(ops[1].kind).toBe("return");
  });

  it("parses while loop with nested body", () => {
    const source = `let mut i = 0;
while i < 10 {
    i += 1;
}
return i;`;
    const ops = parseOps(source);
    expect(ops).toHaveLength(3);
    expect(ops[1].kind).toBe("while");
    if (ops[1].kind === "while") {
      expect(ops[1].condition).toBe("i < 10");
      expect(ops[1].body).toHaveLength(1);
      expect(ops[1].body[0].kind).toBe("set_var");
    }
  });

  it("parses if/else with nested return", () => {
    const source = `if x > 0 {
    return 1;
} else {
    return 0;
}`;
    const ops = parseOps(source);
    expect(ops).toHaveLength(1);
    expect(ops[0].kind).toBe("if_block");
    if (ops[0].kind === "if_block") {
      expect(ops[0].branches).toHaveLength(2);
      expect(ops[0].branches[0].condition).toBe("x > 0");
      expect(ops[0].branches[0].body).toHaveLength(1);
      expect(ops[0].branches[1].condition).toBe("");
    }
  });

  it("round-trips with emitOps", async () => {
    const { emitOps } = await import("../code-op-emitter");
    const source = `let sum = 0.0;
let mut i = 0;
while i < 10 {
    sum += i;
    i += 1;
}
return sum;`;
    const ops = parseOps(source);
    const emitted = emitOps(ops);
    // after round-trip, parsing the emitted source should produce equivalent ops
    const ops2 = parseOps(emitted);
    expect(ops2.length).toBe(ops.length);
    expect(ops2.map(o => o.kind)).toEqual(ops.map(o => o.kind));
  });
});
