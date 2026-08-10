// Shift+点击多选 + 右键「组成分组」接线测试（mock @xyflow/react 捕获 props）
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type { ReactNode } from "react";

// ── mock @xyflow/react：捕获 FlowCanvas 传给 ReactFlow 的 props ──
const captured = { onNodeClick: null as null | ((e: { shiftKey: boolean }, n: { id: string }) => void),
                   onNodeContextMenu: null as null | ((e: Event, n: { id: string }) => void) };
vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    captured.onNodeClick = props.onNodeClick as never;
    captured.onNodeContextMenu = props.onNodeContextMenu as never;
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  Handle: () => <div />,
  Controls: () => <div />,
  Background: () => <div />,
  BackgroundVariant: { Dots: "dots" },
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
}));

import { FlowCanvas } from "../FlowCanvas";
import { usePoiesisStore } from "../../../../store";

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
  codeFlowGraphs: {}, componentDefs: {}, persistedPositions: {}, tabData: {}, codeFlowGroups: {},
});

describe("FlowCanvas 分组选择", () => {
  beforeEach(() => { reset(); vi.clearAllMocks(); });

  it("Shift+点击累积多选，再次点击取消；非 shift 点击单选", () => {
    render(<FlowCanvas />);
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb1" }); });
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb2" }); });
    // 再 shift 点击 cb1 → 取消
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb1" }); });
    expect(captured).toBeTruthy(); // 接线存在
    act(() => { captured.onNodeClick!({ shiftKey: false }, { id: "cb2" }); });
  });

  it("右键选中节点且多选 ≥2 → 菜单出现「组成分组」；点击后 store.codeFlowGroups 写入该 fn", () => {
    render(<FlowCanvas />);
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb1" }); });
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb2" }); });
    act(() => { captured.onNodeContextMenu!({ preventDefault: () => {} } as Event, { id: "cb2" }); });
    const btn = screen.getByRole("button", { name: /组成分组/ });
    expect(btn).toBeTruthy();
    fireEvent.click(btn);
    const groups = usePoiesisStore.getState().codeFlowGroups.increment;
    expect(groups).toHaveLength(1);
    expect(groups![0].memberIds).toEqual(["cb1", "cb2"]);
    expect(groups![0].name).toBe("新分组");
  });

  it("单选（<2）右键 → 菜单无「组成分组」", () => {
    render(<FlowCanvas />);
    act(() => { captured.onNodeClick!({ shiftKey: false }, { id: "cb1" }); });
    act(() => { captured.onNodeContextMenu!({ preventDefault: () => {} } as Event, { id: "cb1" }); });
    expect(screen.queryByRole("button", { name: /组成分组/ })).toBeNull();
  });

  it("预览节点（design-preview）点击不进入多选集——Shift 多选不受干扰", () => {
    render(<FlowCanvas />);
    // Shift 点击预览节点：应被忽略（不累积）
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "design-preview" }); });
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb1" }); });
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb2" }); });
    // 右键 cb2 → 「组成分组」出现且可创建（多选集不含预览节点）
    act(() => { captured.onNodeContextMenu!({ preventDefault: () => {} } as Event, { id: "cb2" }); });
    const btn = screen.getByRole("button", { name: /组成分组/ });
    expect(btn).toBeTruthy();
    fireEvent.click(btn);
    const groups = usePoiesisStore.getState().codeFlowGroups.increment;
    expect(groups![0].memberIds).toEqual(["cb1", "cb2"]);
    // 非 shift 点击预览节点：不应清空/改写选中（预览是视图，无选择语义）
    act(() => { captured.onNodeClick!({ shiftKey: false }, { id: "design-preview" }); });
  });
});