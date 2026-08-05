// poiesis/src/services/codegen/__tests__/flow-layout.test.ts
import { describe, it, expect } from "vitest";
import { computeLayeredLayout, estimateNodeSize, LAYOUT_CONSTS } from "../flow-layout";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

function el(id: string, tag: string, props: Record<string, unknown> = {}, position?: { x: number; y: number }): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, parentId: null, props, source: "", ...(position ? { position } : {}) };
}
function wire(id: string, fromId: string, fromPort: string, toId: string, toPort: string): WireData {
  return { id, fromId, fromPort, toId, toPort };
}

/** 主干 + 分支图：fn → let → condition(2 分支 → a1/a2) → 后置 assign；condition flow-out → assign。 */
const ELEMS = [
  el("fn", "fn", { name: "f", params: "", returnType: "void" }),
  el("let", "let", { name: "n", value: "0" }),
  el("cond", "condition", { branches: [{ id: "br0", type: "if", condition: "n > 0" }, { id: "br1", type: "else", condition: "" }] }),
  el("a1", "assign", { target: "n", operator: "-=", value: "1" }),
  el("a2", "assign", { target: "n", operator: "=", value: "0" }),
  el("post", "assign", { target: "done", operator: "=", value: "true" }),
];
const WIRES = [
  wire("w1", "fn", "flow-out", "let", "flow-in"),
  wire("w2", "let", "flow-out", "cond", "flow-in"),
  wire("w3", "cond", "br0", "a1", "flow-in"),
  wire("w4", "cond", "br1", "a2", "flow-in"),
  wire("w5", "cond", "flow-out", "post", "flow-in"),
];

describe("computeLayeredLayout", () => {
  it("前驱恒在上方：父 y < 子 y（严格）", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    expect(pos["fn"].y).toBeLessThan(pos["let"].y);
    expect(pos["let"].y).toBeLessThan(pos["cond"].y);
    expect(pos["cond"].y).toBeLessThan(pos["a1"].y);
    expect(pos["cond"].y).toBeLessThan(pos["post"].y);
  });
  it("同层同 y（let 与 cond 相邻层均布）", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    expect(pos["a1"].y).toBe(pos["a2"].y); // 两分支首句同层
  });
  it("分支右展：分支 x > 主干 x", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    expect(pos["a1"].x).toBeGreaterThan(pos["cond"].x);
    expect(pos["post"].x).toBe(pos["cond"].x); // flow-out 续接随主干列
  });
  it("无重叠：两两矩形不相交（估算尺寸）", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    const ids = ELEMS.map((e) => e.id);
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const a = pos[ids[i]]!, b = pos[ids[j]]!;
        const sa = estimateNodeSize(ELEMS.find((e) => e.id === ids[i])!.tag, ELEMS.find((e) => e.id === ids[i])!.props);
        const sb = estimateNodeSize(ELEMS.find((e) => e.id === ids[j])!.tag, ELEMS.find((e) => e.id === ids[j])!.props);
        const noOverlap = a.x + sa.width <= b.x || b.x + sb.width <= a.x || a.y + sa.height <= b.y || b.y + sb.height <= a.y;
        expect(noOverlap, `${ids[i]} vs ${ids[j]}`).toBe(true);
      }
    }
  });
  it("多 fn 子图垂直堆叠（间距 ≥ SUBGRAPH_GAP）", () => {
    const fn2 = el("fn2", "fn", { name: "g", params: "", returnType: "void" });
    const ret = el("ret2", "return", {});
    const elems = [...ELEMS, fn2, ret];
    const wires = [...WIRES, wire("w6", "fn2", "flow-out", "ret2", "flow-in")];
    const pos = computeLayeredLayout(elems, wires, new Map());
    expect(pos["ret2"].y - pos["post"].y).toBeGreaterThanOrEqual(LAYOUT_CONSTS.SUBGRAPH_GAP);
  });
  it("孤立节点排在最底部", () => {
    const lonely = el("lonely", "let", { name: "x", value: "0" });
    const pos = computeLayeredLayout([...ELEMS, lonely], WIRES, new Map());
    expect(pos["lonely"].y).toBeGreaterThan(pos["post"].y);
  });
  it("循环节点：body 分支右展、flow-out 续接随主干", () => {
    const loop = el("loop", "while", { condition: "n < 10" });
    const body = el("body", "assign", { target: "n", operator: "=", value: "n + 1" });
    const after = el("after", "return", {});
    const elems = [el("fn", "fn"), loop, body, after];
    const wires = [
      wire("a", "fn", "flow-out", "loop", "flow-in"),
      wire("b", "loop", "br0", "body", "flow-in"),
      wire("c", "loop", "flow-out", "after", "flow-in"),
    ];
    const pos = computeLayeredLayout(elems, wires, new Map());
    expect(pos["body"].x).toBeGreaterThan(pos["loop"].x);
    expect(pos["after"].x).toBe(pos["loop"].x);
    expect(pos["body"].y).toBeGreaterThan(pos["loop"].y);
  });
  it("同桶多节点行高预留：2×2 网格合并两两无重叠", () => {
    // let 双 flow-out → c1/c2 同桶 (2,0)；c1/c2 各 flow-out → p1/p2 同桶 (3,0)
    const grid = [
      el("gfn", "fn", { name: "g", params: "", returnType: "void" }),
      el("glet", "let", { name: "n", value: "0" }),
      el("c1", "assign", { target: "a", operator: "=", value: "1" }),
      el("c2", "assign", { target: "b", operator: "=", value: "2" }),
      el("p1", "assign", { target: "a", operator: "=", value: "3" }),
      el("p2", "assign", { target: "b", operator: "=", value: "4" }),
    ];
    const gridWires = [
      wire("g1", "gfn", "flow-out", "glet", "flow-in"),
      wire("g2", "glet", "flow-out", "c1", "flow-in"),
      wire("g3", "glet", "flow-out", "c2", "flow-in"),
      wire("g4", "c1", "flow-out", "p1", "flow-in"),
      wire("g5", "c2", "flow-out", "p2", "flow-in"),
    ];
    const pos = computeLayeredLayout(grid, gridWires, new Map());
    // 两两矩形不相交（估算尺寸）
    const gridIds = grid.map((e) => e.id);
    for (let i = 0; i < gridIds.length; i++) {
      for (let j = i + 1; j < gridIds.length; j++) {
        const a = pos[gridIds[i]]!, b = pos[gridIds[j]]!;
        const sa = estimateNodeSize(grid.find((e) => e.id === gridIds[i])!.tag, grid.find((e) => e.id === gridIds[i])!.props);
        const sb = estimateNodeSize(grid.find((e) => e.id === gridIds[j])!.tag, grid.find((e) => e.id === gridIds[j])!.props);
        const noOverlap = a.x + sa.width <= b.x || b.x + sb.width <= a.x || a.y + sa.height <= b.y || b.y + sb.height <= a.y;
        expect(noOverlap, `${gridIds[i]} vs ${gridIds[j]}`).toBe(true);
      }
    }
    // 关键对：c2（桶 idx=1）底不穿入下一行 p1 顶
    expect(pos["c2"].y + estimateNodeSize("assign", {}).height).toBeLessThanOrEqual(pos["p1"].y);
  });
});
