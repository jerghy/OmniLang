// poiesis/src/components/design/__tests__/ContextPanel.test.tsx
// 语境面板（T6）：无选中 → 浮标；单选 → 主面板三标签；[内容] 自动醒；多选 ≤2 面板。
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
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

  it("双选 → 主面板 + 副面板（≤2），主面板仍三标签", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1", "el-2"] });
    render(<ContextPanel />);
    expect(screen.getAllByTestId("context-panel")).toHaveLength(2);
    expect(screen.getAllByRole("tab")).toHaveLength(3);
    expect(screen.queryByTestId("context-panel-stub")).toBeNull();
  });
});
