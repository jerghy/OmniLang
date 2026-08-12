// poiesis/src/components/design/__tests__/ContextPanel.test.tsx
// 语境面板（T6）：无选中 → 浮标；单选 → 主面板三标签；[内容] 自动醒；多选 ≤2 面板。
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { ContextPanel } from "../ContextPanel";
import { usePoiesisStore } from "../../../store";
import type { ViewElementData } from "../../../core/types/element";

const el1: ViewElementData = {
  id: "el-1",
  nodeType: "view",
  tag: "button",
  props: { label: "按钮" },
  children: [],
  parentId: null,
};
const el2: ViewElementData = {
  id: "el-2",
  nodeType: "view",
  tag: "input",
  props: {},
  children: [],
  parentId: null,
};

describe("ContextPanel", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false, mainPanelTab: "structure" },
      selectedElementIds: [],
      elements: [el1, el2],
    });
  });

  it("无选中 → 只显示浮标，不显示面板", () => {
    render(<ContextPanel />);
    expect(screen.queryByTestId("context-panel")).toBeNull();
    expect(screen.getByTestId("context-panel-stub")).toBeTruthy();
  });

  it("单选 → 主面板 + [结构][内容][行为] 三标签", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1"] });
    render(<ContextPanel />);
    expect(screen.getAllByTestId("context-panel")).toHaveLength(1);
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.textContent)).toEqual(["结构", "内容", "行为"]);
    expect(screen.queryByTestId("context-panel-stub")).toBeNull();
  });

  it("点击 [内容] → setMainPanelTab('content') 且自动醒", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1"] });
    render(<ContextPanel />);
    fireEvent.click(screen.getByRole("tab", { name: "内容" }));
    expect(usePoiesisStore.getState().designPage.mainPanelTab).toBe("content");
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
  });

  it("tabs 区域真实点击（pointerdown 序列）不被 header 拖拽捕获吞掉——setPointerCapture 不得触发（A2 实机发现）", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1"] });
    render(<ContextPanel />);
    const tablist = screen.getByRole("tablist");
    const header = tablist.parentElement as HTMLElement;
    // jsdom 未实现 setPointerCapture（生产代码 try/catch 吞掉 → 测试环境恒 no-op，
    // 这正是本 bug 在合成测试下存活的原因）——补 mock 模拟真实浏览器的 capture 方法存在
    const capture = vi.fn();
    Object.defineProperty(header, "setPointerCapture", { value: capture, configurable: true, writable: true });
    // 真实指针点击序列的第一步：pointerdown 冒泡到 header 拖拽处理
    fireEvent.pointerDown(screen.getByRole("tab", { name: "内容" }), { button: 0, pointerId: 7 });
    // 若在此调用 capture，后续 click 将被重定向到 header——标签切换在真实指针下失效
    expect(capture).not.toHaveBeenCalled();
  });

  it("双选 → 主面板 + 副面板（≤2），主面板仍三标签", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1", "el-2"] });
    render(<ContextPanel />);
    expect(screen.getAllByTestId("context-panel")).toHaveLength(2);
    expect(screen.getAllByRole("tab")).toHaveLength(3);
    expect(screen.queryByTestId("context-panel-stub")).toBeNull();
  });
});
