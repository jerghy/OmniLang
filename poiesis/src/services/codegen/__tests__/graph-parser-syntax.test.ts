// poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";

describe("parseCodeFlowGraph — syntax nodes", () => {
  it("parses let statement into a let node with extracted props", () => {
    const fnBody = `// @pid:l1
let mut items: Array<String> = [];`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("let");
    expect(el.props).toEqual({ name: "items", mutable: true, varType: "Array<String>", value: "[]" });
    expect(el.source).toBe("let mut items: Array<String> = [];");
  });

  it("parses signal assignment into an assign node", () => {
    const fnBody = `// @pid:a1
count = count + step;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "count", operator: "=", value: "count + step" });
  });

  it("parses compound assignment operator", () => {
    const fnBody = `// @pid:a1
y += 1;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "y", operator: "+=", value: "1" });
  });

  it("parses store call into a store-call node", () => {
    const fnBody = `// @pid:s1
TodoStore.add(text);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("store-call");
    expect(result.elements[0].props).toEqual({ storeName: "TodoStore", action: "add", args: "text" });
  });

  it("parses store signal write as assign (not store-call)", () => {
    const fnBody = `// @pid:a1
ThemeStore.mode = "dark";`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "ThemeStore.mode", operator: "=", value: '"dark"' });
  });

  it("parses emit into a call node with emit=true", () => {
    const fnBody = `// @pid:c1
emit onSelect(item);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: true, fnName: "onSelect", args: "item" });
  });

  it("parses plain function call into a call node", () => {
    const fnBody = `// @pid:c1
close();`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: false, fnName: "close", args: "" });
  });

  it("parses array method call (lowercase) into a call node", () => {
    const fnBody = `// @pid:c1
items.push(x);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: false, fnName: "items.push", args: "x" });
  });

  it("parses let without initial value (name survives round-trip)", () => {
    // LetInspector clearing the initial value emits `let total;` (no `=`).
    // The regex must not fall back to the placeholder name "x".
    const fnBody = `// @pid:l1
let total;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("let");
    expect(el.props).toEqual({ name: "total", mutable: false });
    // formatSyntaxNode round-trip keeps the no-value form
    expect(el.source).toBe("let total;");
  });

  it("parses let mut without initial value", () => {
    const fnBody = `// @pid:l1
let mut x;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("let");
    expect(el.props).toEqual({ name: "x", mutable: true });
    expect(el.source).toBe("let mut x;");
  });

  it("joins multi-line statements instead of truncating to the first line", () => {
    const fnBody = `// @pid:s1
TodoStore.add(
  text
);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("store-call");
    expect(el.props).toEqual({ storeName: "TodoStore", action: "add", args: "text" });
    // Content from the continuation lines survives the round-trip
    expect(el.source).toBe("TodoStore.add(text);");
  });

  it("preserves original source when syntax props cannot be parsed", () => {
    // Unmatched text must not be replaced with placeholder defaults —
    // keep seg.code verbatim so nothing is silently rewritten.
    const fnBody = `// @pid:s1
TodoStore.add(
  text
;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("store-call");
    expect(el.props).toEqual({});
    expect(el.source).toBe(`TodoStore.add(
  text
;`);
  });

  it("keeps for-loop as code-block (starts with 'for', not 'let')", () => {
    const fnBody = `// @pid:cb1
for (let i = 0; i < n; i++) {
  process(i);
}`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("code-block");
  });

  it("wires syntax nodes linearly like code-blocks", () => {
    const fnBody = `// @pid:l1
let x = 0;
// @pid:a1
x = x + 1;
// @pid:c1
close();`;
    const result = parseCodeFlowGraph(fnBody);
    const lw = result.wires.find((w) => w.fromId === "l1" && w.fromPort === "flow-out");
    expect(lw?.toId).toBe("a1");
    const aw = result.wires.find((w) => w.fromId === "a1" && w.fromPort === "flow-out");
    expect(aw?.toId).toBe("c1");
  });
});
