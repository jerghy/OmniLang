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

  // design-preview 是伪节点（不在 store elements）：拖拽 position 变更必须回写本地
  // previewPos state（写 store 是无操作 → 节点瞬移回原位，用户实测拖不动）。
  it("design-preview 位置变更写回本地 state（不落 store）", () => {
    render(<FlowCanvas />);
    const nodesBefore = mockRfProps.current?.nodes as Array<{ id: string; position: { x: number; y: number } }>;
    expect(nodesBefore.find((n) => n.id === "design-preview")?.position).toEqual({ x: 50, y: 50 });

    const onNodesChange = mockRfProps.current?.onNodesChange as (changes: NodeChange[]) => void;
    act(() => {
      onNodesChange([{ type: "position", id: "design-preview", position: { x: 150, y: 180 }, dragging: true }]);
    });

    // 预览节点不在 store → 不得调用 setElementPosition
    expect(mockSetElementPosition).not.toHaveBeenCalled();
    // 位置已回写本地 state → ReactFlow 收到新 nodes prop（拖拽后不瞬移）
    const nodesAfter = mockRfProps.current?.nodes as Array<{ id: string; position: { x: number; y: number } }>;
    expect(nodesAfter.find((n) => n.id === "design-preview")?.position).toEqual({ x: 150, y: 180 });
  });
});
