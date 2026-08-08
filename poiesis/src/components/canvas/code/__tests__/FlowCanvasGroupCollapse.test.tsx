// 折叠态：成员节点从 nodes 消失、edges 重路由到组节点（mock @xyflow/react 捕获 props）
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";

const captured = { nodes: [] as unknown[], edges: [] as unknown[] };
vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    captured.nodes = props.nodes as unknown[];
    captured.edges = props.edges as unknown[];
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  // 真实 xyflow Handle 渲染 data-handleid={id}；mock 转发 props 以支持 data-handleid 断言
  Handle: ({ id, ...rest }: { id?: string } & Record<string, unknown>) => <div data-handleid={id} {...rest} />,
  Controls: () => <div />,
  Background: () => <div />,
  BackgroundVariant: { Dots: "dots" },
  // GroupBox 折叠卡 Handle 用到 Position（Left/Right 钉缘）；mock 缺失该导出会在渲染期抛错
  Position: { Left: "left", Right: "right" },
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
}));

import { FlowCanvas } from "../FlowCanvas";
import { GroupBox } from "../nodes/GroupBox";
import { usePoiesisStore } from "../../../../store";

const codeEl = (id: string, tag: string, name?: string) =>
  ({ id, nodeType: "codeblock" as const, tag, parentId: null, source: "", props: name ? { name } : {}, position: { x: 0, y: 0 } });

const reset = () => usePoiesisStore.setState({
  elements: [
    codeEl("fn1", "fn", "increment"),
    codeEl("cb1", "let", "count"),
    codeEl("cb2", "assign"),
    codeEl("cb3", "call", "close"),
  ],
  wires: [
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
    { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
    { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
  ],
  codeFlowGraphs: {}, componentDefs: {}, persistedPositions: {}, tabData: {},
  codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }] },
});

describe("FlowCanvas 折叠组", () => {
  beforeEach(() => { reset(); vi.clearAllMocks(); });

  it("折叠组：成员节点 cb1/cb2 不在渲染 nodes，组节点 g1 存在", () => {
    render(<FlowCanvas />);
    const ids = (captured.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(ids).toContain("g1");
    expect(ids).not.toContain("cb1");
    expect(ids).not.toContain("cb2");
    expect(ids).toContain("cb3"); // 组外节点保留
  });

  it("折叠组：edges 重路由（fn1→g1、g1→cb3），w2 组内连线隐藏", () => {
    render(<FlowCanvas />);
    const edges = captured.edges as Array<{ id: string; source: string; target: string }>;
    expect(edges.find((e) => e.id === "w1")).toMatchObject({ source: "fn1", target: "g1" });
    expect(edges.find((e) => e.id === "w3")).toMatchObject({ source: "g1", target: "cb3" });
    expect(edges.find((e) => e.id === "w2")).toBeUndefined();
  });

  it("展开（collapsed:false）→ 成员与原始 edges 恢复", () => {
    usePoiesisStore.setState({
      codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }] },
    });
    render(<FlowCanvas />);
    const ids = (captured.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(ids).toContain("cb1");
    const edges = captured.edges as Array<{ id: string }>;
    expect(edges.map((e) => e.id).sort()).toEqual(["w1", "w2", "w3"]);
  });

  it("折叠组卡渲染 flow-in/flow-out Handle（error008：重路由边不被 ReactFlow 丢弃）", () => {
    // 折叠态连线重路由为 source/target=g1 + flow-in/flow-out handle；
    // ReactFlow 要求组节点渲染对应 id 的 Handle 才保留边（缺失 → error008 静默丢弃）。
    const { container } = render(
      <div className="codeflow-group">
        <GroupBox data={{
          group: { id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true },
          editing: false, onRename: () => {}, onColorChange: () => {}, onToggleCollapsed: () => {}, onDelete: () => {},
        }} />
      </div>,
    );
    expect(container.querySelector('.codeflow-group [data-handleid="flow-in"]')).toBeTruthy();
    expect(container.querySelector('.codeflow-group [data-handleid="flow-out"]')).toBeTruthy();
  });

  it("展开态组框不渲染 Handle（保持 Task 4 现状）", () => {
    const { container } = render(
      <GroupBox data={{
        group: { id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false },
        editing: false, onRename: () => {}, onColorChange: () => {}, onToggleCollapsed: () => {}, onDelete: () => {},
      }} />,
    );
    expect(container.querySelector("[data-handleid]")).toBeNull();
  });
});
