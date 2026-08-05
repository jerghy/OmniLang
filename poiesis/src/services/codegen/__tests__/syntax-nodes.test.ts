// poiesis/src/services/codegen/__tests__/syntax-nodes.test.ts
import { describe, it, expect } from "vitest";
import { formatSyntaxNode, defaultSyntaxProps, ASSIGN_OPERATORS } from "../syntax-nodes";

describe("formatSyntaxNode", () => {
  it("formats let without mut", () => {
    expect(formatSyntaxNode("let", { name: "x", mutable: false, value: "0" })).toBe("let x = 0;");
  });
  it("formats let mut with type annotation", () => {
    expect(formatSyntaxNode("let", { name: "items", mutable: true, varType: "Array<String>", value: "[]" })).toBe("let mut items: Array<String> = [];");
  });
  it("formats let without initial value", () => {
    expect(formatSyntaxNode("let", { name: "x" })).toBe("let x;");
  });
  it("formats assign with every operator", () => {
    for (const op of ASSIGN_OPERATORS) {
      expect(formatSyntaxNode("assign", { target: "count", operator: op, value: "1" })).toBe(`count ${op} 1;`);
    }
  });
  it("formats store-call", () => {
    expect(formatSyntaxNode("store-call", { storeName: "TodoStore", action: "add", args: "text" })).toBe("TodoStore.add(text);");
  });
  it("formats call without emit", () => {
    expect(formatSyntaxNode("call", { fnName: "close", args: "", emit: false })).toBe("close();");
  });
  it("formats call with emit prefix", () => {
    expect(formatSyntaxNode("call", { fnName: "onSelect", args: "item", emit: true })).toBe("emit onSelect(item);");
  });
});

describe("defaultSyntaxProps", () => {
  it("provides defaults that format to non-empty code for all four tags", () => {
    for (const tag of ["let", "assign", "store-call", "call"]) {
      expect(formatSyntaxNode(tag, defaultSyntaxProps(tag))).not.toBe("");
    }
  });
});

describe("loop 语法节点", () => {
  it("while 生成 `while <condition> {`（无尾分号，块头由编译器补）", () => {
    expect(formatSyntaxNode("while", { condition: "n < 10" })).toBe("while n < 10 {");
  });
  it("for-in 生成 `for <var> in <iterable> {`", () => {
    expect(formatSyntaxNode("for-in", { varName: "item", iterable: "items" })).toBe("for item in items {");
  });
  it("defaultSyntaxProps 提供占位默认值", () => {
    expect(defaultSyntaxProps("while")).toEqual({ condition: "count < 10" });
    expect(defaultSyntaxProps("for-in")).toEqual({ varName: "item", iterable: "items" });
  });
});
