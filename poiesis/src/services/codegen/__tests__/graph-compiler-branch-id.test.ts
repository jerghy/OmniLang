// poiesis/src/services/codegen/__tests__/graph-compiler-branch-id.test.ts
// 分支 id 与数组索引不一致时（历史源/手写编辑产物），编译必须用分支真实 id 作为端口 key，
// 否则 branchSuccessors 查空 → 分支后继整段静默丢失。
import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

// source：code-block 节点 legacy 路径发射的是 source 文本，分支体文本经此断言。
const el = (id: string, tag: string, props: Record<string, unknown> = {}, source = ""): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source, props });

describe("compileCodeFlowGraph 分支端口（用真实分支 id）", () => {
  it("id 与索引不一致的数组（[br0, br2, br1]）编译不丢分支", () => {
    const elements = [
      el("fn1", "fn", { name: "f", params: "" }),
      el("c1", "condition", {
        branches: [
          { id: "br0", type: "if", condition: "a > 1" },
          { id: "br2", type: "elif", condition: "a > 0" }, // 注意：id≠索引（br2 在索引 1）
          { id: "br1", type: "else", condition: "" },
        ],
      }),
      el("cb1", "code-block", {}, "then-body"),
      el("cb2", "code-block", {}, "elif-body"),
      el("cb3", "code-block", {}, "else-body"),
    ];
    const wires: WireData[] = [
      { id: "w0", fromId: "fn1", fromPort: "flow-out", toId: "c1", toPort: "flow-in" },
      { id: "w1", fromId: "c1", fromPort: "br0", toId: "cb1", toPort: "flow-in" },
      { id: "w2", fromId: "c1", fromPort: "br2", toId: "cb2", toPort: "flow-in" },
      { id: "w3", fromId: "c1", fromPort: "br1", toId: "cb3", toPort: "flow-in" },
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("then-body");
    expect(out).toContain("elif-body");
    expect(out).toContain("else-body");
    // 顺序断言：文本齐全不足以捕获错位（br\${i} 索引失配时三体仍全发射，只是位置交叉）——
    // elif 体必须落在 "else if" 关键字之后、"else {" 之前。
    expect(out.indexOf("else if a > 0 {")).toBeGreaterThan(out.indexOf("then-body"));
    expect(out.indexOf("elif-body")).toBeGreaterThan(out.indexOf("else if a > 0 {"));
    expect(out.indexOf("elif-body")).toBeLessThan(out.indexOf("else {"));
    expect(out.indexOf("else-body")).toBeGreaterThan(out.indexOf("else {"));
  });

  it("正常索引序数组行为不变（if/elif/else 全发射）", () => {
    const elements = [
      el("fn1", "fn", { name: "f", params: "" }),
      el("c1", "condition", {
        branches: [
          { id: "br0", type: "if", condition: "a > 1" },
          { id: "br1", type: "else", condition: "" },
        ],
      }),
      el("cb1", "code-block", {}, "if-body"),
      el("cb2", "code-block", {}, "else-body"),
    ];
    const wires: WireData[] = [
      { id: "w0", fromId: "fn1", fromPort: "flow-out", toId: "c1", toPort: "flow-in" },
      { id: "w1", fromId: "c1", fromPort: "br0", toId: "cb1", toPort: "flow-in" },
      { id: "w2", fromId: "c1", fromPort: "br1", toId: "cb2", toPort: "flow-in" },
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("if-body");
    expect(out).toContain("else-body");
  });
});
