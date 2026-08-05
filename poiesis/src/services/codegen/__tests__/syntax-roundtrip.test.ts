// poiesis/src/services/codegen/__tests__/syntax-roundtrip.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

function el(id: string, tag: string, props: Record<string, unknown>, source = ""): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, props, parentId: null, source };
}

/** Compile a graph then re-parse it; return the parsed elements. */
function compileThenParse(elements: CodeblockElementData[], wires: WireData[], fnId: string) {
  const compiled = compileCodeFlowGraph(elements, wires, fnId);
  return { compiled, parsed: parseCodeFlowGraph(compiled) };
}

describe("syntax node round-trip", () => {
  it("compile → parse preserves node types and props for all four tags", () => {
    const elements = [
      el("fn1", "fn", { name: "roundtrip", params: "" }),
      el("l1", "let", { name: "items", mutable: true, varType: "Array<String>", value: "[]" }),
      el("a1", "assign", { target: "count", operator: "+=", value: "1" }),
      el("s1", "store-call", { storeName: "TodoStore", action: "add", args: "text" }),
      el("c1", "call", { fnName: "onSelect", args: "item", emit: true }),
      el("r1", "return", {}, ""),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "l1", toPort: "flow-in" },
      { id: "w2", fromId: "l1", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
      { id: "w3", fromId: "a1", fromPort: "flow-out", toId: "s1", toPort: "flow-in" },
      { id: "w4", fromId: "s1", fromPort: "flow-out", toId: "c1", toPort: "flow-in" },
      { id: "w5", fromId: "c1", fromPort: "flow-out", toId: "r1", toPort: "flow-in" },
    ];
    const { parsed } = compileThenParse(elements, wires, "fn1");
    expect(parsed.elements.find((e) => e.id === "l1")?.tag).toBe("let");
    expect(parsed.elements.find((e) => e.id === "l1")?.props).toEqual({ name: "items", mutable: true, varType: "Array<String>", value: "[]" });
    expect(parsed.elements.find((e) => e.id === "a1")?.tag).toBe("assign");
    expect(parsed.elements.find((e) => e.id === "a1")?.props).toEqual({ target: "count", operator: "+=", value: "1" });
    expect(parsed.elements.find((e) => e.id === "s1")?.tag).toBe("store-call");
    expect(parsed.elements.find((e) => e.id === "s1")?.props).toEqual({ storeName: "TodoStore", action: "add", args: "text" });
    expect(parsed.elements.find((e) => e.id === "c1")?.tag).toBe("call");
    expect(parsed.elements.find((e) => e.id === "c1")?.props).toEqual({ emit: true, fnName: "onSelect", args: "item" });
    // Wiring preserved
    expect(parsed.wires.some((w) => w.fromId === "l1" && w.toId === "a1")).toBe(true);
    expect(parsed.wires.some((w) => w.fromId === "s1" && w.toId === "c1")).toBe(true);
  });

  it("compile → parse → compile produces identical source", () => {
    const elements = [
      el("fn1", "fn", { name: "stable", params: "n: i32" }),
      el("l1", "let", { name: "total", mutable: false, value: "0" }),
      el("a1", "assign", { target: "total", operator: "=", value: "total + n" }),
      el("r1", "return", {}, ""),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "l1", toPort: "flow-in" },
      { id: "w2", fromId: "l1", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
      { id: "w3", fromId: "a1", fromPort: "flow-out", toId: "r1", toPort: "flow-in" },
    ];
    const { compiled, parsed } = compileThenParse(elements, wires, "fn1");
    // Recompile the parsed graph — source must be identical (idempotent)
    const parsedFnId = parsed.elements.find((e) => e.tag === "fn")!.id;
    const recompiled = compileCodeFlowGraph(parsed.elements, parsed.wires, parsedFnId);
    expect(recompiled).toBe(compiled);
  });

  it("hand-written source with @pid comments round-trips through the pipeline", () => {
    const fnBody = `// @pid:fn1
fn filter(items: Array<String>) {
  // @pid:l1
  let mut result = [];
  // @pid:a1
  result = items.filter(|i| i != "x");
  // @pid:r1
  return result;
}`;
    const parsed = parseCodeFlowGraph(fnBody);
    const fnId = parsed.elements.find((e) => e.tag === "fn")!.id;
    const recompiled = compileCodeFlowGraph(parsed.elements, parsed.wires, fnId);
    const reparsed = parseCodeFlowGraph(recompiled);
    // Types preserved after the full cycle
    expect(reparsed.elements.find((e) => e.id === "l1")?.tag).toBe("let");
    expect(reparsed.elements.find((e) => e.id === "a1")?.tag).toBe("assign");
    expect(reparsed.elements.find((e) => e.id === "r1")?.tag).toBe("return");
  });
});

// ---------------------------------------------------------------------------
// loop round-trip 幂等契约（Task 5）
// ---------------------------------------------------------------------------

/** 嵌套图形：while(body: condition) → flow-out 续接 assign。 */
const ELEMS: CodeblockElementData[] = [
  { id: "fn-1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "f", params: "", returnType: "void" }, source: "" },
  { id: "w-2", nodeType: "codeblock", tag: "while", parentId: null, props: { condition: "running" }, source: "while running {" },
  { id: "cond-3", nodeType: "codeblock", tag: "condition", parentId: null, props: { branches: [{ id: "br0", type: "if", condition: "n > 0" }, { id: "br1", type: "else", condition: "" }] }, source: "" },
  { id: "a-4", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "-=", value: "1" }, source: "n -= 1;" },
  { id: "a-5", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "=", value: "10" }, source: "n = 10;" },
  { id: "a-6", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "done", operator: "=", value: "true" }, source: "done = true;" },
];
const WIRES: WireData[] = [
  { id: "w1", fromId: "fn-1", fromPort: "flow-out", toId: "w-2", toPort: "flow-in" },
  { id: "w2", fromId: "w-2", fromPort: "br0", toId: "cond-3", toPort: "flow-in" },
  { id: "w3", fromId: "cond-3", fromPort: "br0", toId: "a-4", toPort: "flow-in" },
  { id: "w4", fromId: "cond-3", fromPort: "br1", toId: "a-5", toPort: "flow-in" },
  { id: "w5", fromId: "cond-3", fromPort: "flow-out", toId: "a-6", toPort: "flow-in" },
  { id: "w6", fromId: "w-2", fromPort: "flow-out", toId: "a-6", toPort: "flow-in" },
];

describe("loop round-trip", () => {
  it("compile → parse → compile 幂等（嵌套 condition-in-while，wire 双入口去重）", () => {
    const out1 = compileCodeFlowGraph(ELEMS, WIRES, "fn-1");
    const g = parseCodeFlowGraph(out1);
    const out2 = compileCodeFlowGraph(g.elements, g.wires, "fn-1");
    // 结构幂等：二次发射与首次发射逐字一致（循环/分支/语句全部保持）
    expect(out2).toBe(out1);
    // wire 集幂等：out1 再解析与首次解析的 wire 集一致（无累积 wire）
    const g2 = parseCodeFlowGraph(out2);
    const wires1 = g.wires.map((x) => `${x.fromId}(${x.fromPort})->${x.toId}(${x.toPort})`).sort();
    const wires2 = g2.wires.map((x) => `${x.fromId}(${x.fromPort})->${x.toId}(${x.toPort})`).sort();
    expect(wires2).toEqual(wires1);
  });
  it("解析后 while 节点 props 完整（condition 不丢）", () => {
    const out1 = compileCodeFlowGraph(ELEMS, WIRES, "fn-1");
    const g = parseCodeFlowGraph(out1);
    const w = g.elements.find((e) => e.id === "w-2");
    expect(w?.props.condition).toBe("running");
    const wires = g.wires.map((x) => `${x.fromId}(${x.fromPort})->${x.toId}(${x.toPort})`).sort();
    // 双入口（cond-3(flow-out) 与 w-2(flow-out) 同指 a-6）是合法图形，但线性文本中
    // 同一节点只能落位一处：a-6 发射为 cond-3 的 flow-out 续接（循环体内），w-2 的
    // flow-out 经 visited 集去重为注释残留——解析重建以文本为准（ADR-061 negative
    // 已知形态），幂等由 out2 === out1 契约保证。
    expect(wires).toContain("w-2(br0)->cond-3(flow-in)");
    expect(wires).toContain("cond-3(br0)->a-4(flow-in)");
    expect(wires).toContain("cond-3(br1)->a-5(flow-in)");
    expect(wires).toContain("cond-3(flow-out)->a-6(flow-in)");
    expect(wires).not.toContain("w-2(flow-out)->a-6(flow-in)");
  });
});
