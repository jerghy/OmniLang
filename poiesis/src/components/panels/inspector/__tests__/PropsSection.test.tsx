// PropsSection 芯片编辑器（spec D8）：{expr} 段渲染紫色芯片 + label 回归。
// 相对 brief 的修正（如实验证，详见 task-5-report.md）：
// 1) label 回归用例的 tag 用 "button"——getRelevantProps("input") 不含 label
//    （input: ["type","placeholder","value","name"]），label 行不会渲染；
// 2) 「提交 → setProp」用例需要 vi.useFakeTimers + advanceTimersByTime(200)
//    越过 PropsSection 的 200ms debounce（debouncedSetProp），否则断言在
//    setTimeout 回调执行前同步运行而失败；
// 3) getByDisplayValue("Hello ") 的字符串匹配器按原样比较、值被 trim，
//    尾随空格的 "Hello " 永远不命中——加 normalizer: s => s 保留断言意图
//    （实测：默认选项下 "Hello"/regex 命中、"Hello " 不命中）；
// 4) I1 修复用例（/ 选信号）：signalItems 必须传（picker 无 items 不渲染）；
//    rAF 回调（onSlash → setSlash）在 act 外触发 React 延迟刷新，picker 不渲染——
//    advanceTimersByTime(20) 需包 act() 同步冲刷（实测必要）；
// 5) blur 竞态守卫用例：显式 fireEvent.blur 模拟生产 mousedown 移焦时序——
//    picker 交互期 blur 不得提交中间值、不得退出编辑态（slashPendingRef 闩锁）。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within, act } from "@testing-library/react";
import { PropsSection } from "../PropsSection";
import type { ViewElementData } from "../../../../core/types/element";
import type { SignalPickerItem } from "../SignalSlashPicker";

const { mockSetProp } = vi.hoisted(() => ({ mockSetProp: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (sel: (s: any) => unknown) => sel({ setProp: mockSetProp }),
}));

const el = (props: Record<string, unknown>, tag = "input"): ViewElementData => ({
  id: "el-1", nodeType: "view", tag, props, children: [], parentId: null,
});

// / 插入用例的 picker fixture：insertionText 为裸信号名（ContextInspector
// useSignalItems 实际传 s.name / c.name，无花括号——花括号由目标编辑器包裹）。
const signalItem: SignalPickerItem = {
  pid: "isLoading",
  name: "isLoading",
  insertionText: "isLoading",
  type: "bool",
  category: "signals",
};

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

// 布尔属性行（D9）：开关 ↔ 表达式互斥视图。
// 相对 brief 的修正（如实验证，详见 task-6-report.md）：
// 1) 断言 mockSetProp 的用例加 vi.useFakeTimers + advanceTimersByTime(200)
//    越过 PropsSection 的 200ms debounce（Task 5 先例）；
// 2) 「点表达式」用例的按钮用 within(disabled 行) 定位——el({ disabled: true })
//    时 checked/disabled/required/readonly 四行各有一个「表达式」按钮，
//    裸 getByRole("button", { name: /表达式/ }) 会因多元素命中而抛错。
describe("布尔属性行（D9）", () => {
  it("input 白名单包含 checked/disabled/required/readonly（D9 前置）", () => {
    // 通过渲染断言：<input checked={agreed}> 解析后面板显示 checked 行
    render(<PropsSection element={el({ type: "checkbox", checked: "{agreed}" })} />);
    expect(screen.getByText("checked")).toBeTruthy();
  });

  it("boolean true → 显示开关且勾选", () => {
    render(<PropsSection element={el({ disabled: true })} />);
    const toggle = screen.getByRole("checkbox", { name: /disabled/ }) as HTMLInputElement;
    expect(toggle.checked).toBe(true);
  });

  it("勾选开关 → setProp 收到 true", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({})} />);
      const toggle = screen.getByRole("checkbox", { name: /disabled/ });
      fireEvent.click(toggle);
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", true);
    } finally {
      vi.useRealTimers();
    }
  });

  it("点「表达式」→ 输入 {expr} → setProp 收到字符串", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({ disabled: true })} />);
      const row = screen.getByRole("checkbox", { name: /disabled/ }).closest("div")!;
      fireEvent.click(within(row).getByRole("button", { name: /表达式/ }));
      const input = screen.getByLabelText("disabled-expression");
      fireEvent.change(input, { target: { value: "{isLoading}" } });
      fireEvent.keyDown(input, { key: "Enter" });
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", "{isLoading}");
    } finally {
      vi.useRealTimers();
    }
  });

  it("表达式形态显示紫色芯片、开关隐藏", () => {
    render(<PropsSection element={el({ disabled: "{isLoading}" })} />);
    expect(screen.getByText("{isLoading}")).toBeTruthy();
    expect(screen.queryByRole("checkbox", { name: /disabled/ })).toBeNull();
  });

  it("删除表达式芯片 → 回开关态", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({ disabled: "{isLoading}" })} />);
      fireEvent.click(screen.getByTitle("删除"));
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", true);
    } finally {
      vi.useRealTimers();
    }
  });

  it("空前缀：/ 选信号 → Enter 提交 → setProp 收到 {isLoading}（I1 主流程）", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({ disabled: true })} signalItems={[signalItem]} />);
      const row = screen.getByRole("checkbox", { name: /disabled/ }).closest("div")!;
      fireEvent.click(within(row).getByRole("button", { name: /表达式/ }));
      const input = screen.getByLabelText("disabled-expression");
      fireEvent.keyDown(input, { key: "/" });
      // rAF 回调（onSlash → setSlash）在 act 外触发 React 延迟刷新，picker 不渲染——
      // advanceTimersByTime 需包在 act 里同步冲刷
      act(() => { vi.advanceTimersByTime(20); });
      fireEvent.click(screen.getByRole("button", { name: /isLoading/ }));
      const after = screen.getByLabelText("disabled-expression") as HTMLInputElement;
      expect(after.value).toBe("isLoading");
      fireEvent.keyDown(after, { key: "Enter" });
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", "{isLoading}");
    } finally {
      vi.useRealTimers();
    }
  });

  it("前缀 isLo + / 选信号 → 光标插入 isLoisLoading → 提交收到 {isLoisLoading}（I1）", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({ disabled: true })} signalItems={[signalItem]} />);
      const row = screen.getByRole("checkbox", { name: /disabled/ }).closest("div")!;
      fireEvent.click(within(row).getByRole("button", { name: /表达式/ }));
      const input = screen.getByLabelText("disabled-expression");
      fireEvent.change(input, { target: { value: "isLo" } });
      fireEvent.keyDown(input, { key: "/" });
      act(() => { vi.advanceTimersByTime(20); });
      fireEvent.click(screen.getByRole("button", { name: /isLoading/ }));
      const after = screen.getByLabelText("disabled-expression") as HTMLInputElement;
      // 光标插入语义：前缀保留 + 裸信号名插入（不整体包花括号，避免 {isLo{isLoading}}）
      expect(after.value).toBe("isLoisLoading");
      fireEvent.keyDown(after, { key: "Enter" });
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", "{isLoisLoading}");
    } finally {
      vi.useRealTimers();
    }
  });

  it("blur 竞态守卫：picker 打开期间 blur 不提交中间值，选中后插入生效", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({ disabled: true })} signalItems={[signalItem]} />);
      const row = screen.getByRole("checkbox", { name: /disabled/ }).closest("div")!;
      fireEvent.click(within(row).getByRole("button", { name: /表达式/ }));
      const input = screen.getByLabelText("disabled-expression");
      fireEvent.change(input, { target: { value: "isLo" } });
      fireEvent.keyDown(input, { key: "/" });
      act(() => { vi.advanceTimersByTime(20); }); // picker 渲染
      // 生产时序：mousedown picker 项 → 输入框 blur（尚未选中）→
      // 不得提交中间值（"{isLo}"）、不得退出编辑态
      fireEvent.blur(input);
      vi.advanceTimersByTime(200);
      expect(mockSetProp).not.toHaveBeenCalled();
      const still = screen.getByLabelText("disabled-expression") as HTMLInputElement;
      expect(still.value).toBe("isLo");
      // 随后完成选中 → 插入不被丢弃
      fireEvent.click(screen.getByRole("button", { name: /isLoading/ }));
      const after = screen.getByLabelText("disabled-expression") as HTMLInputElement;
      expect(after.value).toBe("isLoisLoading");
      fireEvent.keyDown(after, { key: "Enter" });
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", "{isLoisLoading}");
    } finally {
      vi.useRealTimers();
    }
  });

  it("芯片点击进入编辑态（D9 表达式态可编辑——真机 bug 回归）", () => {
    vi.useFakeTimers();
    try {
      render(<PropsSection element={el({ disabled: "{busy}" })} signalItems={[signalItem]} />);
      // 表达式态显示芯片、无开关
      const chip = screen.getByText("{busy}");
      expect(screen.queryByRole("checkbox", { name: /disabled/ })).toBeNull();
      // 点击芯片 → 编辑态输入框出现，预填剥花括号
      fireEvent.click(chip);
      const input = screen.getByLabelText("disabled-expression") as HTMLInputElement;
      expect(input.value).toBe("busy");
      // 修改表达式 → Enter → 提交新值
      fireEvent.change(input, { target: { value: "busy2" } });
      fireEvent.keyDown(input, { key: "Enter" });
      vi.advanceTimersByTime(200);
      expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", "{busy2}");
    } finally {
      vi.useRealTimers();
    }
  });
});
