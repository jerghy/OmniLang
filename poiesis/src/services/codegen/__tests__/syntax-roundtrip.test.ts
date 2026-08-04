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
