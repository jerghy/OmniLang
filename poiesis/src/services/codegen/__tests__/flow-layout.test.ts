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
    // flow-out 续接随主干列（post 左缘对齐）；分支源 cond 右对齐到列右缘（x > 0）
    expect(pos["post"].x).toBe(0);
    expect(pos["cond"].x).toBeGreaterThan(0);
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
    expect(pos["after"].x).toBe(0); // after 左缘对齐（主干列）
    expect(pos["loop"].x).toBeGreaterThan(0); // loop 分支源右对齐
    expect(pos["body"].y).toBeGreaterThan(pos["loop"].y);
  });
  it("同桶多节点行高预留：2×2 网格合并两两无重叠（含跨子图边界）", () => {
    // let 双 flow-out → c1/c2 同桶 (2,0)；c1/c2 各 flow-out → p1/p2 同桶 (3,0)；第二子图 fn2 → ret2
    const grid = [
      el("gfn", "fn", { name: "g", params: "", returnType: "void" }),
      el("glet", "let", { name: "n", value: "0" }),
      el("c1", "assign", { target: "a", operator: "=", value: "1" }),
      el("c2", "assign", { target: "b", operator: "=", value: "2" }),
      el("p1", "assign", { target: "a", operator: "=", value: "3" }),
      el("p2", "assign", { target: "b", operator: "=", value: "4" }),
      el("gfn2", "fn", { name: "g2", params: "", returnType: "void" }),
      el("gret2", "return", {}),
    ];
    const gridWires = [
      wire("g1", "gfn", "flow-out", "glet", "flow-in"),
      wire("g2", "glet", "flow-out", "c1", "flow-in"),
      wire("g3", "glet", "flow-out", "c2", "flow-in"),
      wire("g4", "c1", "flow-out", "p1", "flow-in"),
      wire("g5", "c2", "flow-out", "p2", "flow-in"),
      wire("g6", "gfn2", "flow-out", "gret2", "flow-in"),
    ];
    const pos = computeLayeredLayout(grid, gridWires, new Map());
    // 两两矩形不相交（估算尺寸，含跨子图对）
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
    // 关键对：p2（最深行桶 idx=1）底不穿入下一子图首节点 gfn2 顶（间距 ≥ SUBGRAPH_GAP）
    expect(pos["gfn2"].y - (pos["p2"].y + estimateNodeSize("assign", {}).height)).toBeGreaterThanOrEqual(LAYOUT_CONSTS.SUBGRAPH_GAP);
  });

  it("分支 lane 按分支 id 排序分配（wire 数组乱序不产生兄弟交叉）", () => {
    // wires 数组乱序：br1 的 wire 在前——lane 分配须按 br0 < br1 排序，使 a1 列 < a2 列
    const elems = [el("fn", "fn"), el("cond", "condition", { branches: [{ id: "br0", type: "if", condition: "a" }, { id: "br1", type: "else", condition: "" }] }), el("a1", "assign", { target: "n", operator: "=", value: "1" }), el("a2", "assign", { target: "n", operator: "=", value: "2" })];
    const wires = [
      wire("x1", "fn", "flow-out", "cond", "flow-in"),
      wire("x2", "cond", "br1", "a2", "flow-in"), // 乱序：br1 在前
      wire("x3", "cond", "br0", "a1", "flow-in"),
    ];
    const pos = computeLayeredLayout(elems, wires, new Map());
    // br0 目标（a1）必须占更小的列（lane0 → x 更小），与分支 id 视觉序一致
    expect(pos["a1"].x).toBeLessThan(pos["a2"].x);
  });

  it("分支源右对齐：有分支出口的节点右缘 = 列右缘（wire 起点不被同列下方更宽节点覆盖）", () => {
    // 主干：fn → while（宽 200）→ for-in（宽 200 估宽一致？用自定义尺寸制造右缘参差）
    // 用 nodeSizes 制造：while 宽 120（窄）、下方主干节点 for-in 宽 160（宽）——
    // 修复前 while 左对齐 → while 右缘 < for-in 右缘 → 分支 wire 起点（while 右缘）被 for-in 覆盖
    const elems = [
      el("fn", "fn"),
      el("loop", "while", { condition: "n < 10" }),
      el("body", "assign", { target: "n", operator: "=", value: "n + 1" }),
      el("next", "for-in", { varName: "item", iterable: "items" }),
    ];
    const wires = [
      wire("a", "fn", "flow-out", "loop", "flow-in"),
      wire("b", "loop", "br0", "body", "flow-in"),
      wire("c", "loop", "flow-out", "next", "flow-in"),
    ];
    const sizes = new Map<string, { width: number; height: number }>([
      ["fn", { width: 200, height: 64 }],
      ["loop", { width: 120, height: 88 }],
      ["body", { width: 200, height: 88 }],
      ["next", { width: 160, height: 88 }],
    ]);
    const pos = computeLayeredLayout(elems, wires, sizes);
    // 分支源 loop 右缘 ≥ 同列（主干列）所有节点右缘——wire 起点不被 next 覆盖
    const loopRight = pos["loop"].x + 120;
    const nextRight = pos["next"].x + 160;
    expect(loopRight).toBeGreaterThanOrEqual(nextRight);
    // body（分支目标）在主干列右缘之后（走廊存在）
    expect(pos["body"].x).toBeGreaterThanOrEqual(loopRight);
  });
});
