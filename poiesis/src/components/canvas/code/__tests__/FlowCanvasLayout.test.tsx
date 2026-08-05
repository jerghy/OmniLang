// FlowCanvas 自动整理布局按钮接线测试（Task 11）。
// 点击「整理布局」→ computeLayeredLayout 纯函数 → setElementsAndWires 批量写入（一次 undo 条目）。
// mock 模式参照 FlowCanvasDrag.test.tsx：@xyflow/react 与 store 均 mock（按钮读 store 而非 props）。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FlowCanvas } from "../FlowCanvas";

// ── mock @xyflow/react：ReactFlow 渲染 children（布局按钮在 ReactFlow 同级、需真实 DOM）──
vi.mock("@xyflow/react", () => ({
  ReactFlow: ({ children }: { children?: React.ReactNode }) => <div data-testid="reactflow">{children}</div>,
  ReactFlowProvider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
  Background: () => null,
  BackgroundVariant: { Dots: "dots", Lines: "lines", Cross: "cross" },
  Controls: () => null,
  Handle: () => null,
  Position: { Top: "top", Bottom: "bottom", Right: "right", Left: "left" },
}));

// ── mock store：可变 elements/wires + setElementsAndWires 间谍（vi.hoisted 供 mock 工厂引用）──
const { mockStore } = vi.hoisted(() => ({
  mockStore: {
    elements: [
      { id: "fn-1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "f", params: "", returnType: "void" }, source: "", position: { x: 0, y: 0 } },
      { id: "a-2", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "=", value: "1" }, source: "n = 1;", position: { x: 300, y: 200 } },
    ],
    wires: [{ id: "w1", fromId: "fn-1", fromPort: "flow-out", toId: "a-2", toPort: "flow-in" }],
    setElementsAndWires: vi.fn(),
    addElement: () => {}, removeElement: () => {}, setElementPosition: () => {},
    setProp: () => {}, setElementSource: () => {}, setNodeMeta: () => {}, addWire: () => {},
    componentDefs: {}, storeReferences: {}, storeDefs: {}, inlineStoreDefs: {},
    designPage: { codeFlowElementId: "main", previewOutput: null, activeTabId: "t1", openTabs: [{ id: "t1", componentName: "App", filePath: "App.omni" }] },
  },
}));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) => selector(mockStore),
}));

const DEFAULT_ELEMENTS = [...mockStore.elements];
const DEFAULT_WIRES = [...mockStore.wires];

beforeEach(() => {
  mockStore.elements = [...DEFAULT_ELEMENTS];
  mockStore.wires = [...DEFAULT_WIRES];
  mockStore.setElementsAndWires = vi.fn();
});

function setup(overrides: Record<string, unknown> = {}) {
  Object.assign(mockStore, overrides);
  render(<FlowCanvas />);
  return { setElementsAndWires: mockStore.setElementsAndWires };
}

describe("FlowCanvas 自动整理布局", () => {
  it("点击布局按钮 → setElementsAndWires 调用（一次 undo 条目）", () => {
    const { setElementsAndWires } = setup();
    fireEvent.click(screen.getByLabelText("自动整理布局"));
    expect(setElementsAndWires).toHaveBeenCalledTimes(1);
    const [elements, wires] = setElementsAndWires.mock.calls[0] as [unknown[], unknown[]];
    expect(elements).toHaveLength(2);
    expect(wires).toHaveLength(1);
    // 重排后 fn 在前、位置有序（y 递增）
    const fnEl = (elements as Array<{ id: string; position: { y: number } }>).find((e) => e.id === "fn-1")!;
    const aEl = (elements as Array<{ id: string; position: { y: number } }>).find((e) => e.id === "a-2")!;
    expect(fnEl.position.y).toBeLessThan(aEl.position.y);
  });
  it("空画布布局按钮禁用", () => {
    setup({ elements: [], wires: [] });
    expect((screen.getByLabelText("自动整理布局") as HTMLButtonElement).disabled).toBe(true);
  });
});
