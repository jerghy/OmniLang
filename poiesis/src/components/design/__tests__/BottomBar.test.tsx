// components/design/__tests__/BottomBar.test.tsx
// 底栏两件套（M2 T1）：[组件库]（弹出面板开关，DesignPage 受控）+ [数据节点]（源节点显隐开关，store 瞬态）。
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { BottomBar } from "../BottomBar";
import { usePoiesisStore } from "../../../store";

describe("BottomBar 底栏两件套", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, dataNodesVisible: false, awake: false },
    });
  });

  it("渲染 [组件库] 与 [数据节点] 两个按钮", () => {
    render(<BottomBar onPaletteToggle={() => {}} />);
    expect(screen.getByTestId("palette-toggle")).toBeTruthy();
    expect(screen.getByTestId("data-nodes-toggle")).toBeTruthy();
  });

  it("点 [数据节点] → dataNodesVisible 翻转", () => {
    render(<BottomBar onPaletteToggle={() => {}} />);
    fireEvent.click(screen.getByTestId("data-nodes-toggle"));
    expect(usePoiesisStore.getState().designPage.dataNodesVisible).toBe(true);
    fireEvent.click(screen.getByTestId("data-nodes-toggle"));
    expect(usePoiesisStore.getState().designPage.dataNodesVisible).toBe(false);
  });

  it("点 [组件库] → 触发 onPaletteToggle 回调（DesignPage 受控）", () => {
    const onPaletteToggle = vi.fn();
    render(<BottomBar onPaletteToggle={onPaletteToggle} />);
    fireEvent.click(screen.getByTestId("palette-toggle"));
    expect(onPaletteToggle).toHaveBeenCalledTimes(1);
  });

  it("active 类 = dataNodesVisible && awake（醒态门控）", () => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: true, dataNodesVisible: true },
    });
    render(<BottomBar onPaletteToggle={() => {}} />);
    expect(screen.getByTestId("data-nodes-toggle").className).toContain("btnActive");
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false },
    });
    fireEvent.click(screen.getByTestId("data-nodes-toggle")); // 醒态关闭 → 即使开关开也不激活
    expect(screen.getByTestId("data-nodes-toggle").className).not.toContain("btnActive");
  });
});
