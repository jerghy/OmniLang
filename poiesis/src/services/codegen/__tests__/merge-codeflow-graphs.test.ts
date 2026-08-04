import { describe, it, expect } from "vitest";
import { mergeCodeFlowGraphs } from "../graph-parser";

describe("mergeCodeFlowGraphs", () => {
  it("多文件合并，fn 名去重", () => {
    const a = { increment: { elements: [{ id: "fn1", nodeType: "codeblock", tag: "fn", parentId: null, source: "", props: {} } as never], wires: [] as never[] } };
    const b = { close: { elements: [], wires: [] } };
    const merged = mergeCodeFlowGraphs([a, b]);
    expect(Object.keys(merged)).toEqual(["increment", "close"]);
  });
  it("undefined 条目跳过", () => {
    const merged = mergeCodeFlowGraphs([undefined, { fa: { elements: [], wires: [] } }]);
    expect(Object.keys(merged)).toEqual(["fa"]);
  });
  it("同名 fn 后者覆盖", () => {
    const a = { fa: { elements: [{ id: "x", nodeType: "codeblock", tag: "let", parentId: null, source: "", props: { name: "x" } } as never], wires: [] as never[] } };
    const b = { fa: { elements: [], wires: [] } };
    expect(mergeCodeFlowGraphs([a, b]).fa.elements).toEqual([]);
  });
});
