// FlowCanvas 拖拽防闪烁（measured 传递）测试。
// 根因（2026-08-09 真机实测）：受控 ReactFlow 下，每次 onNodesChange 位置写回都会重建
// nodes useMemo → 全量新节点对象 → adoptUserNodes(checkEquality) 重建内部节点 → measured
// 丢失 → NodeWrapper 渲染 visibility:hidden → ResizeObserver 重新测量恢复 → 拖拽期间每帧
// 隐藏↔显示循环（屏闪）+ error015 每帧警告 + ResizeObserver loop 错误（实测组卡 52 次、
// fn 节点 36 次 visibility 切换）。
// 修复：handleNodesChange 接收 React Flow 的 dimensions change（RO 测量结果）写入缓存，
// 节点对象构建时携带 measured → adoptUserNodes 重建内部节点时 measured 保留 → 零闪烁。
// 注意：节点对象必须每帧重建（对象身份复用会让 adoptUserNodes checkEquality 命中且
// 内部节点位置不更新 → 拖不动，2026-08-09 回归教训）；本测试同时断言 position 写回生效。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, act } from "@testing-library/react";
import type { NodeChange, Node as RFNode } from "@xyflow/react";
import { FlowCanvas } from "../FlowCanvas";
import { usePoiesisStore } from "../../../../store";
import type { ElementData } from "../../../../core/types/element";

const captured = {
  nodes: [] as RFNode[],
  onNodesChange: null as null | ((changes: NodeChange[]) => void),
  onNodeClick: null as null | ((_e: unknown, node: RFNode) => void),
};
vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    captured.nodes = props.nodes as RFNode[];
    captured.onNodesChange = props.onNodesChange as never;
    captured.onNodeClick = props.onNodeClick as never;
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p, setViewport: vi.fn() }),
  Background: () => null,
  BackgroundVariant: { Dots: "dots" },
  Controls: () => null,
  Handle: () => null,
  Position: { Top: "top", Bottom: "bottom", Right: "right", Left: "left" },
}));

const codeEl = (id: string, tag: string, name?: string) =>
  ({ id, nodeType: "codeblock" as const, tag, parentId: null, source: "", props: name ? { name } : {}, position: { x: 0, y: 0 } });

const reset = () => usePoiesisStore.setState({
  elements: [
    codeEl("fn1", "fn", "increment"),
    codeEl("cb1", "let", "count"),
    codeEl("cb2", "assign"),
  ],
  wires: [
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
    { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
  ],
  codeFlowGraphs: {}, componentDefs: {}, persistedPositions: {}, tabData: {},
  codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }] },
});

beforeEach(() => {
  reset();
  vi.clearAllMocks();
  captured.nodes = [];
  captured.onNodesChange = null;
  captured.onNodeClick = null;
});

describe("FlowCanvas 拖拽防闪烁（measured 传递）", () => {
  it("dimensions 变更后节点对象携带 measured（防 adoptUserNodes 重建丢 measured）", () => {
    render(<FlowCanvas />);
    // 先触发 RO 测量结果（dimensions change）→ 缓存
    act(() => {
      captured.onNodesChange!([{ type: "dimensions", id: "fn1", dimensions: { width: 120, height: 40 } }]);
    });
    // 再触发一次 position 写回 → 节点重建 → measured 必须保留
    act(() => {
      captured.onNodesChange!([{ type: "position", id: "fn1", position: { x: 320, y: 210 }, dragging: true }]);
    });
    const fn = captured.nodes.find((n) => n.id === "fn1");
    expect(fn?.measured).toEqual({ width: 120, height: 40 }); // measured 传递 → 不闪烁
    expect(fn?.position).toEqual({ x: 320, y: 210 });         // position 写回生效 → 拖拽正常
  });

  it("折叠组卡 dimensions 变更后组节点携带 measured", () => {
    render(<FlowCanvas />);
    act(() => {
      captured.onNodesChange!([{ type: "dimensions", id: "g1", dimensions: { width: 160, height: 36 } }]);
    });
    act(() => {
      captured.onNodesChange!([{ type: "position", id: "g1", position: { x: 500, y: 400 }, dragging: true }]);
    });
    const g = captured.nodes.find((n) => n.id === "g1");
    expect(g?.measured).toEqual({ width: 160, height: 36 });
    expect(g?.position).toEqual({ x: 500, y: 400 });
  });

  it("拖拽位置写回后节点对象 position 更新（每帧重建但位置正确）", () => {
    render(<FlowCanvas />);
    act(() => {
      captured.onNodesChange!([{ type: "position", id: "fn1", position: { x: 320, y: 210 }, dragging: true }]);
    });
    const fn = captured.nodes.find((n) => n.id === "fn1");
    expect(fn?.position).toEqual({ x: 320, y: 210 });
  });

  it("键盘方向键移动（同写回路径）：位置更新", () => {
    render(<FlowCanvas />);
    const fn = captured.nodes.find((n) => n.id === "fn1")!;
    act(() => { captured.onNodeClick!({}, fn); });
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true }));
    });
    const after = captured.nodes.find((n) => n.id === "fn1");
    expect(after?.position).toEqual({ x: 10, y: 0 }); // MOVE_STEP = 10
  });

  it("节点数据变化（props 编辑）时节点对象重建且 data 刷新", () => {
    render(<FlowCanvas />);
    const before = captured.nodes.find((n) => n.id === "fn1");
    act(() => {
      usePoiesisStore.setState({
        elements: usePoiesisStore.getState().elements.map((e) => {
          if (e.id !== "fn1") return e;
          const code = e as Extract<ElementData, { nodeType: "codeblock" }>;
          return { ...code, props: { ...code.props, name: "other" } };
        }),
      });
    });
    const after = captured.nodes.find((n) => n.id === "fn1");
    expect(after).not.toBe(before); // props 变了 → 重建（data 需刷新）
    expect((after?.data as { props: Record<string, unknown> }).props).toEqual({ name: "other" });
  });
});
