import { describe, it, expect } from "vitest";
import { GROUP_COLORS, validateGroupMembers, fnNameOf, computeGroupBoxes, collectCollapsedMemberIds, buildFlowEdges } from "../groups";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";
import type { CodeFlowGroup } from "../../../core/types/project-json";

const el = (id: string, tag: string, name?: string, position?: { x: number; y: number }): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {}, position });

const ELEMS = [
  el("fn1", "fn", "increment"),
  el("cb1", "let", "count", { x: 0, y: 0 }),
  el("cb2", "assign", undefined, { x: 0, y: 60 }),
  el("cb3", "call", "close", { x: 300, y: 0 }),
];
const WIRES: WireData[] = [
  { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
  { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
  { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
];

describe("groups 纯函数", () => {
  it("GROUP_COLORS 恰 8 色", () => {
    expect(GROUP_COLORS).toHaveLength(8);
    expect(GROUP_COLORS[0]).toBe("#6366f1");
  });

  it("validateGroupMembers：同 fn 语句节点通过，返回 ownerFnId", () => {
    const r = validateGroupMembers(["cb1", "cb2"], ELEMS, WIRES);
    expect(r).toEqual({ valid: ["cb1", "cb2"], ownerFnId: "fn1", error: undefined });
  });

  it("validateGroupMembers：含 fn 节点 → 拒绝并报错", () => {
    const r = validateGroupMembers(["fn1", "cb1"], ELEMS, WIRES);
    expect(r.error).toBeTruthy();
    expect(r.valid).toEqual([]);
  });

  it("validateGroupMembers：跨 fn 成员 → 拒绝", () => {
    const ELEMS2 = [...ELEMS, el("fn2", "fn", "decrement"), el("cb9", "let", "x")];
    const WIRES2 = [...WIRES, { id: "w9", fromId: "fn2", fromPort: "flow-out", toId: "cb9", toPort: "flow-in" }];
    const r = validateGroupMembers(["cb1", "cb9"], ELEMS2, WIRES2);
    expect(r.error).toBeTruthy();
    expect(r.valid).toEqual([]);
  });

  it("validateGroupMembers：孤岛节点（无 fn 可达）→ 拒绝", () => {
    const r = validateGroupMembers(["cb1", "cb3"], ELEMS.slice(0, 3), WIRES.slice(0, 2));
    // cb3 不在 WIRES 里 → findOwnerFnId 为 null → 拒绝
    expect(r.error).toBeTruthy();
  });

  it("fnNameOf：取 fn 元素 props.name，缺失回退 unnamed", () => {
    expect(fnNameOf("fn1", ELEMS)).toBe("increment");
    expect(fnNameOf("nonexist", ELEMS)).toBe("unnamed");
  });

  it("computeGroupBoxes：成员包围盒含 padding，缺尺寸成员跳过", () => {
    const g: CodeFlowGroup = { id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false };
    const sizes = new Map([["cb1", { width: 200, height: 40 }], ["cb2", { width: 200, height: 40 }]]);
    const boxes = computeGroupBoxes([g], ELEMS, sizes);
    expect(boxes.get("g1")).toEqual({ x: -24, y: -24, width: 248, height: 148 });
  });

  it("computeGroupBoxes：空组/无尺寸 → 该组不产出包围盒", () => {
    const g: CodeFlowGroup = { id: "g1", name: "空", memberIds: [], color: "#6366f1", collapsed: false };
    expect(computeGroupBoxes([g], ELEMS, new Map()).size).toBe(0);
  });

  it("computeGroupBoxes：成员存在但尺寸缺失 → 该组跳过", () => {
    const g: CodeFlowGroup = { id: "g1", name: "缺尺寸", memberIds: ["cb1"], color: "#6366f1", collapsed: false };
    // cb1 有 position 但 sizes 无记录 → 无法可靠摆放，跳过
    expect(computeGroupBoxes([g], ELEMS, new Map()).size).toBe(0);
  });
});

// 折叠态连线重路由
it("collectCollapsedMemberIds：收集所有折叠组的成员", () => {
  const groups = [
    { id: "g1", name: "a", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true },
    { id: "g2", name: "b", memberIds: ["cb3"], color: "#6366f1", collapsed: false },
  ] as CodeFlowGroup[];
  expect(collectCollapsedMemberIds(groups)).toEqual(new Set(["cb1", "cb2"]));
});

it("buildFlowEdges：进线/出线重路由到组边界，组内连线隐藏，wire id 保留", () => {
  const groups = [{ id: "g1", name: "a", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }] as CodeFlowGroup[];
  const edges = buildFlowEdges([
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
    { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
    { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
  ], groups);
  expect(edges).toEqual([
    { id: "w1", type: "default", source: "fn1", target: "g1", sourceHandle: "flow-out", targetHandle: "flow-in" },
    { id: "w3", type: "default", source: "g1", target: "cb3", sourceHandle: "flow-out", targetHandle: "flow-in" },
  ]);
});

it("buildFlowEdges：无折叠组时原样透传（含 type: default）", () => {
  const edges = buildFlowEdges([
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
  ], []);
  expect(edges).toEqual([{ id: "w1", type: "default", source: "fn1", target: "cb1", sourceHandle: "flow-out", targetHandle: "flow-in" }]);
});
