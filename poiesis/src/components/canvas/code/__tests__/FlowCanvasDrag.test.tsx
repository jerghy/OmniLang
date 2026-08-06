// FlowCanvas 拖拽位置回写接线测试。
// 根因（2026-08-04）：受控 ReactFlow（nodes prop，无 defaultNodes）下，xyflow 拖拽中的
// 位置更新被内部 `hasDefaultNodes` 门控丢弃，必须由应用经 onNodesChange 回写 store，
// 否则节点拖拽冻结、仅 onNodeDragStop 提交导致松开后瞬移。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, act } from "@testing-library/react";
import type { NodeChange } from "@xyflow/react";
import { FlowCanvas } from "../FlowCanvas";

// ── mock @xyflow/react：捕获 ReactFlow props，其余组件桩 ──
const { mockRfProps } = vi.hoisted(() => ({ mockRfProps: { current: null as null | Record<string, unknown> } }));

vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    mockRfProps.current = props;
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
  Background: () => null,
  BackgroundVariant: { Dots: "dots", Lines: "lines", Cross: "cross" },
  Controls: () => null,
  Handle: () => null,
  Position: { Top: "top", Bottom: "bottom", Right: "right", Left: "left" },
}));

// ── mock store：elements 含一个可拖拽的 codeblock 节点 ──
const { mockSetElementPosition } = vi.hoisted(() => ({ mockSetElementPosition: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) =>
    selector({
      elements: [
        {
          id: "l1", nodeType: "codeblock", tag: "let", parentId: null,
          props: { name: "x", mutable: false, value: "0" },
          source: "let x = 0;",
          position: { x: 100, y: 100 },
        },
      ],
      wires: [],
      addElement: () => {},
      removeElement: () => {},
      setElementPosition: (id: string, pos: { x: number; y: number }) => {
        mockSetElementPosition(id, pos);
      },
      setProp: () => {},
      setElementSource: () => {},
      setNodeMeta: () => {},
      addWire: () => {},
      componentDefs: {},
      storeReferences: {},
      storeDefs: {},
      inlineStoreDefs: {},
      designPage: {
        codeFlowElementId: "main",
        previewOutput: null,
        activeTabId: "t1",
        openTabs: [{ id: "t1", componentName: "App", filePath: "App.omni" }],
      },
    }),
}));

beforeEach(() => {
  mockSetElementPosition.mockClear();
  mockRfProps.current = null;
});

describe("FlowCanvas 拖拽位置回写（onNodesChange 接线）", () => {
  it("ReactFlow 收到 onNodesChange 处理器", () => {
    render(<FlowCanvas />);
    expect(mockRfProps.current?.onNodesChange).toBeTypeOf("function");
  });

  it("position 变更（拖拽中）回写 store setElementPosition", () => {
    render(<FlowCanvas />);
    const onNodesChange = mockRfProps.current?.onNodesChange as (changes: NodeChange[]) => void;
    onNodesChange([{ type: "position", id: "l1", position: { x: 320, y: 210 }, dragging: true }]);
    expect(mockSetElementPosition).toHaveBeenCalledWith("l1", { x: 320, y: 210 });
  });

  it("非 position 变更（如 select）不回写位置", () => {
    render(<FlowCanvas />);
    const onNodesChange = mockRfProps.current?.onNodesChange as (changes: NodeChange[]) => void;
    onNodesChange([{ type: "select", id: "l1", selected: true }]);
    expect(mockSetElementPosition).not.toHaveBeenCalled();
  });

  // design-preview 是伪节点（不在 store elements）+ draggable:false——拖拽由节点组件自管
  // （ref + transform 直改，松手经 data.onPreviewMove 提交一次，2026-08-06 闪烁修复轮）。
  // className:"nopan" 是硬约束：draggable:false 时 ReactFlow 不再给 wrapper 加 nopan 类，
  // pane 的 pan filter 放行后 d3-zoom stopImmediatePropagation 切断冒泡 → React 合成
  // mousedown 永不触发（实测复现）；nopan 类让 pan filter 拒绝 → 合成事件正常分发。
  it("design-preview 节点 draggable:false + className nopan（不经 ReactFlow 拖拽体系）", () => {
    render(<FlowCanvas />);
    const nodes = mockRfProps.current?.nodes as Array<{ id: string; draggable?: boolean; className?: string }>;
    const preview = nodes.find((n) => n.id === "design-preview");
    expect(preview?.draggable).toBe(false);
    expect(preview?.className).toContain("nopan");
  });

  it("design-preview 松手经 onPreviewMove 提交位置（不落 store）", () => {
    render(<FlowCanvas />);
    const nodesBefore = mockRfProps.current?.nodes as Array<{
      id: string;
      position: { x: number; y: number };
      data: { onPreviewMove: (pos: { x: number; y: number }) => void };
    }>;
    const previewBefore = nodesBefore.find((n) => n.id === "design-preview");
    expect(previewBefore?.position).toEqual({ x: 50, y: 50 });
    expect(previewBefore?.data.onPreviewMove).toBeTypeOf("function");

    act(() => {
      previewBefore!.data.onPreviewMove({ x: 150, y: 180 });
    });

    // 预览节点不在 store → 不得调用 setElementPosition
    expect(mockSetElementPosition).not.toHaveBeenCalled();
    // 位置提交到本地 state → ReactFlow 收到新 nodes prop（落点持久）
    const nodesAfter = mockRfProps.current?.nodes as Array<{ id: string; position: { x: number; y: number } }>;
    expect(nodesAfter.find((n) => n.id === "design-preview")?.position).toEqual({ x: 150, y: 180 });
  });
});
