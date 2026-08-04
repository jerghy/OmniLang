import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

function el(id: string, tag: string, props: Record<string, unknown>, source = ""): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, props, parentId: null, source };
}

function wire(fromId: string, toId: string): WireData {
  return { id: `w-${fromId}-${toId}`, fromId, fromPort: "flow-out", toId, toPort: "flow-in" };
}

describe("compileCodeFlowGraph — syntax nodes", () => {
  it("emits let/assign/store-call/call in a linear fn chain", () => {
    const elements = [
      el("fn1", "fn", { name: "handleAdd", params: "text: String" }),
      el("l1", "let", { name: "items", mutable: false, value: "[]" }),
      el("a1", "assign", { target: "count", operator: "=", value: "count + step" }),
      el("s1", "store-call", { storeName: "TodoStore", action: "add", args: "text" }),
      el("c1", "call", { fnName: "close", args: "", emit: false }),
      el("r1", "return", {}, ""),
    ];
    const wires = [
      wire("fn1", "l1"), wire("l1", "a1"), wire("a1", "s1"), wire("s1", "c1"), wire("c1", "r1"),
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("fn handleAdd(text: String) {");
    expect(out).toContain("// @pid:l1\n  let items = [];");
    expect(out).toContain("count = count + step;");
    expect(out).toContain("TodoStore.add(text);");
    expect(out).toContain("close();");
    expect(out).toContain("return;");
  });

  it("emits syntax nodes inside condition branches with branch indent", () => {
    const elements = [
      el("fn1", "fn", { name: "check", params: "" }),
      el("cond1", "condition", { branches: [
        { id: "br0", type: "if", condition: "x > 0" },
        { id: "br1", type: "else", condition: "" },
      ] }),
      el("l1", "let", { name: "y", mutable: true, value: "1" }),
      el("a1", "assign", { target: "y", operator: "+=", value: "1" }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cond1", toPort: "flow-in" },
      { id: "w2", fromId: "cond1", fromPort: "br0", toId: "l1", toPort: "flow-in" },
      { id: "w3", fromId: "cond1", fromPort: "br1", toId: "a1", toPort: "flow-in" },
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("if x > 0 {");
    expect(out).toContain("  let mut y = 1;");
    // Note: 既有 condition emitter 输出 `}\n  else {`（分两行，graph-parser 可解析，round-trip 测试覆盖）
    // 因此不断言单行 "} else {"，而断言 else 分支行本身。
    expect(out).toContain("  else {");
    expect(out).toContain("  y += 1;");
  });

  it("emits emit-call with the emit prefix", () => {
    const elements = [
      el("fn1", "fn", { name: "select", params: "" }),
      el("c1", "call", { fnName: "onSelect", args: "item", emit: true }),
    ];
    const wires = [wire("fn1", "c1")];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("emit onSelect(item);");
  });
});
