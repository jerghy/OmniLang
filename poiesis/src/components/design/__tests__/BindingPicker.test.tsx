// BindingPicker（M3 T3 五类槽点名绑定）：来源清单浮层。
// 纯 props 组件（sources/onPick/onClose）——无 store 依赖，直接渲染断言。
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";
import { BindingPicker, type BindingSource } from "../BindingPicker";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

// 三类来源 fixture：组件信号 / fn 出口 / 醒着的元素
const sources: BindingSource[] = [
  { id: "sig:1", kind: "signal", name: "count", type: "i32" },
  { id: "fn:1", kind: "fn", name: "newFn", type: "() => void" },
  { id: "el:1", kind: "element", name: "input_2", type: "值" },
];

describe("BindingPicker", () => {
  test("渲染来源清单：信号 / fn 出口 / 画布元素", () => {
    render(<BindingPicker sources={sources} onPick={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByText("count")).toBeTruthy();
    expect(screen.getByText("newFn")).toBeTruthy();
    expect(screen.getByText("input_2")).toBeTruthy();
    // 类型标注也可见
    expect(screen.getByText("i32")).toBeTruthy();
    expect(screen.getByText("值")).toBeTruthy();
  });

  test("点击来源 → onPick(该来源)", () => {
    const onPick = vi.fn();
    render(<BindingPicker sources={sources} onPick={onPick} onClose={vi.fn()} />);
    fireEvent.click(screen.getByText("count"));
    expect(onPick).toHaveBeenCalledTimes(1);
    expect(onPick).toHaveBeenCalledWith(sources[0]);
  });

  test("关闭按钮 → onClose", () => {
    const onClose = vi.fn();
    render(<BindingPicker sources={sources} onPick={vi.fn()} onClose={onClose} />);
    fireEvent.click(screen.getByTestId("binding-picker-close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
