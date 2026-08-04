import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeFormulaField, type FormulaCandidateSource } from "../NodeFormulaField";

const cands: FormulaCandidateSource = {
  filter: (q) => (q === "" ? [{ value: "count", label: "count" }] : [{ value: "count", label: "count" }].filter((c) => c.value.startsWith(q))),
  storeSignals: (name) => (name === "ThemeStore" ? [{ value: "ThemeStore.mode", label: "ThemeStore.mode" }] : []),
};

describe("NodeFormulaField", () => {
  it("blur 提交 onCommit（草稿语义不变）", () => {
    const onCommit = vi.fn();
    render(<NodeFormulaField value="0" onCommit={onCommit} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值");
    fireEvent.change(input, { target: { value: "count + 1" } });
    expect(onCommit).not.toHaveBeenCalled();
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledWith("count + 1");
  });

  it("输入字母 → 弹候选（operand 上下文）", () => {
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    expect(screen.getByText("count")).toBeTruthy();
  });

  it("Tab 选中候选 → 替换当前 token 并关闭弹层（不提交）", () => {
    const onCommit = vi.fn();
    render(<NodeFormulaField value="" onCommit={onCommit} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    fireEvent.keyDown(input, { key: "Tab" });
    expect(input.value).toBe("count");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("输入单字符运算符（+）→ 直接插入不弹框", () => {
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "count ", selectionStart: 6 } });
    fireEvent.change(input, { target: { value: "count +", selectionStart: 7 } });
    expect(input.value).toBe("count +");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("输入运算符前缀（<）→ 弹运算符选择框", () => {
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "count <", selectionStart: 7 } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    expect(screen.getByText("<=")).toBeTruthy();
  });

  it("Enter 在弹层打开时选中候选（提交语义由后续 blur 完成）", () => {
    const onCommit = vi.fn();
    render(<NodeFormulaField value="" onCommit={onCommit} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input.value).toBe("count");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("Esc 关闭弹层（草稿保留，不提交不取消）", () => {
    const onCommit = vi.fn();
    render(<NodeFormulaField value="0" onCommit={onCommit} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(input.value).toBe("cou"); // 草稿保留
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("Store. 输入 → member 候选（storeSignals）", () => {
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "ThemeStore.", selectionStart: 11 } });
    expect(screen.getByText("ThemeStore.mode")).toBeTruthy();
  });

  it("member 上下文选中候选：整段替换 Store. 前缀（不重复）", () => {
    // E2E 缺陷 2 回归：修复前选中产生 `demostore.demostore.newSignal` 前缀重复
    const candsStore: FormulaCandidateSource = {
      filter: () => [],
      storeSignals: (name) => (name === "demostore" ? [{ value: "demostore.newSignal", label: "demostore.newSignal" }] : []),
    };
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={candsStore} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "demostore.", selectionStart: 10 } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    fireEvent.keyDown(input, { key: "Tab" });
    expect(input.value).toBe("demostore.newSignal");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("↑↓ 导航候选，Enter 选中第二项", () => {
    const cands2: FormulaCandidateSource = {
      filter: () => [{ value: "a", label: "a" }, { value: "b", label: "b" }],
      storeSignals: () => [],
    };
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={cands2} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "", selectionStart: 0 } });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input.value).toBe("b");
  });

  it("IME 组合输入期间不弹候选", () => {
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.compositionStart(input);
    fireEvent.change(input, { target: { value: "中", selectionStart: 1 } });
    expect(screen.queryByRole("listbox")).toBeNull();
    fireEvent.compositionEnd(input);
  });

  it("无 candidates → 退化纯文本（不弹层不报错）", () => {
    const onCommit = vi.fn();
    render(<NodeFormulaField value="0" onCommit={onCommit} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值");
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    expect(screen.queryByRole("listbox")).toBeNull();
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledWith("cou");
  });

  it("运算符候选选择不重复前缀（replaceToken 替换 token）", () => {
    render(<NodeFormulaField value="" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "count <", selectionStart: 7 } });
    // 默认第一项 "<" → 替换后值不变（insertAt 会得到 "count <<"）
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input.value).toBe("count <");
    // 重开弹层，导航到 "<=" 选中 → 前缀不重复
    fireEvent.change(input, { target: { value: "count <", selectionStart: 7 } });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input.value).toBe("count <=");
  });

  it("鼠标点击候选不提前提交（菜单项 onMouseDown 防夺焦）", () => {
    const onCommit = vi.fn();
    render(<NodeFormulaField value="" onCommit={onCommit} candidates={cands} ariaLabel="初始值" />);
    const input = screen.getByLabelText("初始值") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    const option = screen.getByRole("option");
    fireEvent.mouseDown(option);
    fireEvent.click(option);
    expect(onCommit).not.toHaveBeenCalled(); // 未完成草稿不得被 blur 提前提交
    expect(input.value).toBe("count"); // 候选已替换进草稿
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});
