// 组注释生成测试：注释插在组内首个 emit 成员前；折叠态不影响输出；不传 groups 行为不变
import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";
import type { CodeFlowGroup } from "../../../core/types/project-json";

const el = (id: string, tag: string, props: Record<string, unknown> = {}): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props });

const ELEMS = [
  el("fn1", "fn", { name: "increment", params: "" }),
  el("cb1", "let", { name: "count", value: "0" }),
  el("cb2", "assign", { target: "count", operator: "+=", value: "1" }),
  el("cb3", "return", { expr: "count" }),
];
const WIRES: WireData[] = [
  { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
  { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
  { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
];

describe("graph-compiler 组注释", () => {
  it("组注释插在组内首个 emit 成员（cb1）前，同缩进", () => {
    const groups: CodeFlowGroup[] = [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }];
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn1", groups);
    expect(out).toContain("  // ── 初始化 ──\n");
    expect(out.indexOf("// ── 初始化 ──")).toBeLessThan(out.indexOf("// @pid:cb1"));
  });

  it("折叠态（collapsed:true）不影响输出——注释与语句完整", () => {
    const groups: CodeFlowGroup[] = [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }];
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn1", groups);
    expect(out).toContain("// ── 初始化 ──");
    expect(out).toContain("// @pid:cb1");
    expect(out).toContain("// @pid:cb2");
  });

  it("组内成员分散在分支时，注释只出现在首个成员处一次", () => {
    const COND = el("cd1", "condition", { branches: [{ id: "br0", type: "if", condition: "count > 0" }] });
    const ELEMS2 = [el("fn1", "fn", { name: "f", params: "" }), COND, el("cb1", "let", { name: "a", value: "1" }), el("cb2", "call", { fnName: "g" })];
    const WIRES2: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cd1", toPort: "flow-in" },
      { id: "w2", fromId: "cd1", fromPort: "br0", toId: "cb1", toPort: "flow-in" },
      { id: "w3", fromId: "cd1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
    ];
    const groups: CodeFlowGroup[] = [{ id: "g1", name: "分支组", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }];
    const out = compileCodeFlowGraph(ELEMS2, WIRES2, "fn1", groups);
    expect(out.match(/\/\/ ── 分支组 ──/g)).toHaveLength(1);
    expect(out.indexOf("// ── 分支组 ──")).toBeLessThan(out.indexOf("// @pid:cb1"));
  });

  it("不传 groups → 输出与既有行为一致（无组注释）", () => {
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn1");
    expect(out).not.toContain("// ──");
    expect(out).toContain("// @pid:cb1");
  });
});
