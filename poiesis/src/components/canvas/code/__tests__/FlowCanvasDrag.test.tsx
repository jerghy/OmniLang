// FlowCanvas 拖拽位置回写接线测试。
// 根因（2026-08-04）：受控 ReactFlow（nodes prop，无 defaultNodes）下，xyflow 拖拽中的
// 位置更新被内部 `hasDefaultNodes` 门控丢弃，必须由应用经 onNodesChange 回写 store，
// 否则节点拖拽冻结、仅 onNodeDragStop 提交导致松开后瞬移。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
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
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p, setViewport: vi.fn() }),
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
});
