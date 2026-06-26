// editor/client/src/omni-gen/__tests__/expression-lowering.test.ts
import { describe, it, expect } from "vitest";
import { lowerCode } from "../expression-lowering";
import type { ElementData, WireData } from "../../../core/types";

describe("lowerCode", () => {
  it("builds setVariable expression from literal add", () => {
    const els: ElementData[] = [
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 42 }, children: [], parentId: null },
      { id: "lit2", nodeType: "codeblock", tag: "literal", props: { value: 1 }, children: [], parentId: null },
      { id: "add1", nodeType: "codeblock", tag: "add", props: {}, children: [], parentId: null },
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "x" }, children: [], parentId: null },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "lit1", fromPort: "value", toId: "add1", toPort: "a" },
      { id: "w2", fromId: "lit2", fromPort: "value", toId: "add1", toPort: "b" },
      { id: "w3", fromId: "add1", fromPort: "result", toId: "set1", toPort: "value" },
    ];
    const r = lowerCode(els, wires);
    expect(r.expressions.get("set1")).toBe("x = 42 + 1;");
  });

  it("builds setVariable expression from signal add", () => {
    const els: ElementData[] = [
      { id: "sig1", nodeType: "codeblock", tag: "signal", props: { name: "counter" }, children: [], parentId: null },
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 1 }, children: [], parentId: null },
      { id: "add1", nodeType: "codeblock", tag: "add", props: {}, children: [], parentId: null },
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "counter" }, children: [], parentId: null },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "sig1", fromPort: "value", toId: "add1", toPort: "a" },
      { id: "w2", fromId: "lit1", fromPort: "value", toId: "add1", toPort: "b" },
      { id: "w3", fromId: "add1", fromPort: "result", toId: "set1", toPort: "value" },
    ];
    const r = lowerCode(els, wires);
    expect(r.expressions.get("set1")).toBe("counter = counter + 1;");
  });

  it("builds function body from customFn", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "increment", params: [], returnType: "void" }, children: [], parentId: null },
      { id: "gv1", nodeType: "codeblock", tag: "getVariable", props: { name: "counter" }, children: [], parentId: null },
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 1 }, children: [], parentId: null },
      { id: "add1", nodeType: "codeblock", tag: "add", props: {}, children: [], parentId: null },
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "counter" }, children: [], parentId: null },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "gv1", fromPort: "value", toId: "add1", toPort: "a" },
      { id: "w2", fromId: "lit1", fromPort: "value", toId: "add1", toPort: "b" },
      { id: "w3", fromId: "add1", fromPort: "result", toId: "set1", toPort: "value" },
      { id: "w4", fromId: "fn1", fromPort: "body", toId: "set1", toPort: "body" },
    ];
    const r = lowerCode(els, wires);
    expect(r.functions).toHaveLength(1);
    expect(r.functions[0].name).toBe("increment");
    expect(r.functions[0].body).toBe("counter = counter + 1;");
  });

  it("uses props.source for fn tag", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "fn", props: { source: "console.log('hello')" }, children: [], parentId: null },
    ];
    const r = lowerCode(els, []);
    expect(r.expressions.get("fn1")).toBe("console.log('hello')");
  });

  it("detects wire cycles", () => {
    const els: ElementData[] = [
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "x" }, children: [], parentId: null },
      { id: "add1", nodeType: "codeblock", tag: "add", props: {}, children: [], parentId: null },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "add1", fromPort: "result", toId: "set1", toPort: "value" },
      { id: "w2", fromId: "add1", fromPort: "result", toId: "add1", toPort: "a" },
    ];
    const r = lowerCode(els, wires);
    expect(r.expressions.get("set1")).toContain("/* cycle */");
  });

  it("handles empty wires gracefully", () => {
    const els: ElementData[] = [
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "x" }, children: [], parentId: null },
    ];
    const r = lowerCode(els, []);
    expect(r.expressions.get("set1")).toBe("x = ;");
    expect(r.functions).toHaveLength(0);
  });

  it("converts string literals with quotes", () => {
    const els: ElementData[] = [
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: "hello" }, children: [], parentId: null },
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "msg" }, children: [], parentId: null },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "lit1", fromPort: "value", toId: "set1", toPort: "value" },
    ];
    const r = lowerCode(els, wires);
    expect(r.expressions.get("set1")).toBe('msg = "hello";');
  });

  it("handles all binary ops (sub, mul, div, compare, and, or, concat)", () => {
    const ops = [
      { tag: "sub", expected: "10 - 3;" },
      { tag: "mul", expected: "10 * 3;" },
      { tag: "div", expected: "10 / 3;" },
      { tag: "compare", props: { op: ">" }, expected: "10 > 3;" },
      { tag: "and", expected: "10 && 3;" },
      { tag: "or", expected: "10 || 3;" },
      { tag: "concat", expected: "10 + 3;" },
    ];
    for (const op of ops) {
      const els: ElementData[] = [
        { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 10 }, children: [], parentId: null },
        { id: "lit2", nodeType: "codeblock", tag: "literal", props: { value: 3 }, children: [], parentId: null },
        { id: "op1", nodeType: "codeblock", tag: op.tag, props: op.props || {}, children: [], parentId: null },
        { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "x" }, children: [], parentId: null },
      ];
      const wires: WireData[] = [
        { id: "w1", fromId: "lit1", fromPort: "value", toId: "op1", toPort: "a" },
        { id: "w2", fromId: "lit2", fromPort: "value", toId: "op1", toPort: "b" },
        { id: "w3", fromId: "op1", fromPort: "result", toId: "set1", toPort: "value" },
      ];
      const r = lowerCode(els, wires);
      expect(r.expressions.get("set1")).toBe(`x = ${op.expected}`);
    }
  });

  it("handles not unary op", () => {
    const els: ElementData[] = [
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: false }, children: [], parentId: null },
      { id: "not1", nodeType: "codeblock", tag: "not", props: {}, children: [], parentId: null },
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "x" }, children: [], parentId: null },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "lit1", fromPort: "value", toId: "not1", toPort: "a" },
      { id: "w2", fromId: "not1", fromPort: "result", toId: "set1", toPort: "value" },
    ];
    const r = lowerCode(els, wires);
    expect(r.expressions.get("set1")).toBe("x = !false;");
  });

  it("handles return statement", () => {
    const els: ElementData[] = [
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 42 }, children: [], parentId: null },
      { id: "ret1", nodeType: "codeblock", tag: "return", props: {}, children: [], parentId: null },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "lit1", fromPort: "value", toId: "ret1", toPort: "value" },
    ];
    const r = lowerCode(els, wires);
    expect(r.expressions.get("ret1")).toBe("return 42;");
  });

  it("returns all LoweredCode fields with correct shape", () => {
    const r = lowerCode([], []);
    expect(r).toHaveProperty("expressions");
    expect(r).toHaveProperty("functions");
    expect(r).toHaveProperty("actions");
    expect(r).toHaveProperty("getters");
    expect(r.expressions instanceof Map).toBe(true);
    expect(Array.isArray(r.functions)).toBe(true);
    expect(Array.isArray(r.actions)).toBe(true);
    expect(Array.isArray(r.getters)).toBe(true);
  });
});
