// PropsSection 芯片编辑器（spec D8）：{expr} 段渲染紫色芯片 + label 回归。
// 相对 brief 的修正（如实验证，详见 task-5-report.md）：
// 1) label 回归用例的 tag 用 "button"——getRelevantProps("input") 不含 label
//    （input: ["type","placeholder","value","name"]），label 行不会渲染；
// 2) 「提交 → setProp」用例需要 vi.useFakeTimers + advanceTimersByTime(200)
//    越过 PropsSection 的 200ms debounce（debouncedSetProp），否则断言在
//    setTimeout 回调执行前同步运行而失败；
// 3) getByDisplayValue("Hello ") 的字符串匹配器按原样比较、值被 trim，
//    尾随空格的 "Hello " 永远不命中——加 normalizer: s => s 保留断言意图
//    （实测：默认选项下 "Hello"/regex 命中、"Hello " 不命中）。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PropsSection } from "../PropsSection";
import type { ViewElementData } from "../../../../core/types/element";

const { mockSetProp } = vi.hoisted(() => ({ mockSetProp: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (sel: (s: any) => unknown) => sel({ setProp: mockSetProp }),
}));

const el = (props: Record<string, unknown>, tag = "input"): ViewElementData => ({
  id: "el-1", nodeType: "view", tag, props, children: [], parentId: null,
});

beforeEach(() => mockSetProp.mockClear());

describe("PropsSection 芯片编辑器（D8）", () => {
  it("value 值为 {count} → 渲染紫色表达式芯片", () => {
    render(<PropsSection element={el({ value: "{count}" })} />);
    expect(screen.getByText("{count}")).toBeTruthy();
  });

  it("静态 value 渲染为普通文本段（无芯片）", () => {
    render(<PropsSection element={el({ value: "Enter name" })} />);
    expect(screen.queryByText("{count}")).toBeNull();
    expect(screen.getByDisplayValue("Enter name")).toBeTruthy();
  });

  it("label 芯片编辑回归：混排 Hello {count} 拆段", () => {
    render(<PropsSection element={el({ label: "Hello {count}" }, "button")} />);
    expect(screen.getByText("{count}")).toBeTruthy();
    expect(screen.getByDisplayValue("Hello ", { normalizer: (s) => s })).toBeTruthy();
  });

  it("编辑芯片后提交 → setProp 收到新值", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({ value: "{count}" })} />);
      fireEvent.click(screen.getByText("{count}"));
      const editInput = screen.getByLabelText("Edit segment");
      fireEvent.change(editInput, { target: { value: "{count + 1}" } });
      fireEvent.keyDown(editInput, { key: "Enter" });
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "value", "{count + 1}");
    } finally {
      vi.useRealTimers();
    }
  });
});
