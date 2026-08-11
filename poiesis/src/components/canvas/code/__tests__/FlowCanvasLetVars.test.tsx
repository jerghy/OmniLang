// FlowCanvasLetVars.test.tsx（新建——独立 mock，不动 FlowCanvasDrag 既有 mock 结构）
// FlowCanvas 向语法节点 data 注入所属 fn 的 letVars（wire 拓扑推断）。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { FlowCanvas } from "../FlowCanvas";

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

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) =>
    selector({
      elements: [
        { id: "fn1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "increment", params: "" }, source: "" },
        { id: "let1", nodeType: "codeblock", tag: "let", parentId: null, props: { name: "x", mutable: false, value: "0" }, source: "", position: { x: 0, y: 0 } },
        { id: "a1", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "count", operator: "=", value: "" }, source: "", position: { x: 0, y: 120 } },
      ],
      wires: [
        { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "let1", toPort: "flow-in" },
        { id: "w2", fromId: "let1", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
      ],
      addElement: () => {}, removeElement: () => {}, setElementPosition: () => {},
      setProp: () => {}, setElementSource: () => {}, setNodeMeta: () => {}, addWire: () => {},
      componentDefs: {}, storeReferences: {}, storeDefs: {}, inlineStoreDefs: {},
      designPage: { activeTabId: "t1", openTabs: [{ id: "t1", componentName: "App", filePath: "App.omni" }] },
    }),
}));

beforeEach(() => { mockRfProps.current = null; });

describe("FlowCanvas letVars 注入（wire 拓扑）", () => {
  it("assign 节点 data.letVars 含所属 fn 的 let 变量", () => {
    render(<FlowCanvas />);
    const nodes = mockRfProps.current?.nodes as Array<{ id: string; data: Record<string, unknown> }>;
    const assignNode = nodes.find((n) => n.id === "a1");
    expect(assignNode?.data.letVars).toEqual([{ name: "x" }]);
  });
  it("fn 节点自身携带其 fn 的 letVars（不渲染公式框，无害）", () => {
    render(<FlowCanvas />);
    const nodes = mockRfProps.current?.nodes as Array<{ id: string; data: Record<string, unknown> }>;
    const fnNode = nodes.find((n) => n.id === "fn1");
    expect(fnNode?.data.letVars).toEqual([{ name: "x" }]);
  });
});
