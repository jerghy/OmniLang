import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConditionNode } from "../ConditionNode";
import { ReturnNode } from "../ReturnNode";
import type { FlowScope } from "../../../../../services/codegen/flow-scope";

// 公式框测试共用 scope fixture（与 SyntaxNode.test.tsx 同形）
const formulaScope: FlowScope = {
  signals: [{ name: "count", type: "i32" }],
  props: [], computeds: [], events: [],
  fns: [], stores: [], fnParams: [{ name: "step", type: "i32" }],
  letVars: [],
};

describe("ConditionNode 内联编辑", () => {
  it("点击 if 表达式进入编辑，blur 提交 onEditCondition", () => {
    const onEditCondition = vi.fn();
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br0", type: "if", condition: "" }], onEditCondition }} />);
    fireEvent.click(screen.getByText("..."));
    const input = screen.getByLabelText("编辑条件表达式");
    const ta = input.querySelector("textarea") as HTMLTextAreaElement;
    // 新契约：表达式以胶囊标记段输入（raw 自由文本会按字符串字面量加引号提交）
    ta.value = "\uE000count\uE001 \uE000>\uE001 0";
    ta.setSelectionRange(13, 13); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(ta, { target: { value: "\uE000count\uE001 \uE000>\uE001 0" } });
    fireEvent.blur(ta);
    expect(onEditCondition).toHaveBeenCalledWith("br0", "count > 0");
  });

  it("已有条件直接显示（TokenPreview 胶囊），点击进入编辑", () => {
    const { container } = render(<ConditionNode data={{ label: "判断", branches: [{ id: "br0", type: "if", condition: "x == 1" }], onEditCondition: vi.fn() }} />);
    // token 结构断言：identifier/operator/number 分胶囊（空白被 tokenizer 跳过，容器文本 "x==1"）
    const preview = container.querySelector('[data-testid="token-preview"]');
    expect(preview?.textContent).toBe("x==1");
    expect(container.querySelector('[data-token-kind="identifier"]')?.textContent).toBe("x");
    expect(container.querySelectorAll('[data-token-kind="operator"]')).toHaveLength(1);
    fireEvent.click(preview!);
    expect(screen.getByLabelText("编辑条件表达式")).toBeTruthy();
  });

  it("else 分支不可编辑", () => {
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br1", type: "else", condition: "" }], onEditCondition: vi.fn() }} />);
    expect(screen.queryByLabelText("编辑条件表达式")).toBeFalsy();
  });

  it("Esc 取消退出编辑态恢复显示", () => {
    const { container } = render(<ConditionNode data={{ label: "判断", branches: [{ id: "br0", type: "if", condition: "x == 1" }], onEditCondition: vi.fn() }} />);
    fireEvent.click(screen.getByTestId("token-preview"));
    const input = screen.getByLabelText("编辑条件表达式").querySelector("textarea") as HTMLTextAreaElement;
    fireEvent.keyDown(input, { key: "Escape" });
    expect(screen.queryByLabelText("编辑条件表达式")).toBeFalsy();
    expect(container.querySelector('[data-testid="token-preview"]')?.textContent).toBe("x==1");
  });
});

describe("ReturnNode 内联编辑", () => {
  it("显示 return 表达式（TokenPreview 胶囊）", () => {
    const { container } = render(<ReturnNode data={{ label: "返回", returnExpr: "items.len()" }} />);
    // token 结构断言：member 整体胶囊 + paren 符号
    expect(container.querySelector('[data-token-kind="member"]')?.textContent).toBe("items.len");
    expect(container.querySelector('[data-token-kind="paren"]')?.textContent).toBe("(");
  });

  it("点击表达式区进入编辑，Enter 提交", () => {
    const onCommit = vi.fn();
    const { container } = render(<ReturnNode data={{ label: "返回", returnExpr: "x", onCommitReturn: onCommit }} />);
    fireEvent.click(container.querySelector('[data-testid="token-preview"]')!);
    const input = screen.getByLabelText("return 表达式");
    const ta = input.querySelector("textarea") as HTMLTextAreaElement;
    // 新契约：标识符以胶囊标记段输入（raw "y" 会按字符串字面量提交为 \"y\"）
    ta.value = "\uE000y\uE001";
    ta.setSelectionRange(3, 3); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(ta, { target: { value: "\uE000y\uE001" } });
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("y");
  });

  it("Esc 取消退出编辑态恢复显示", () => {
    const { container } = render(<ReturnNode data={{ label: "返回", returnExpr: "x", onCommitReturn: vi.fn() }} />);
    fireEvent.click(container.querySelector('[data-testid="token-preview"]')!);
    expect(screen.getByLabelText("return 表达式")).toBeTruthy();
    fireEvent.keyDown(screen.getByLabelText("return 表达式").querySelector("textarea") as HTMLTextAreaElement, { key: "Escape" });
    expect(screen.queryByLabelText("return 表达式")).toBeFalsy();
    expect(container.querySelector('[data-testid="token-preview"]')?.textContent).toBe("x");
  });
});

describe("ConditionNode 公式框", () => {
  const data = {
    label: "判断",
    branches: [{ id: "br0", type: "if" as const, condition: "a > 1" }],
    scope: formulaScope,
    letVars: [{ name: "total" }],
    onEditCondition: vi.fn(),
  };

  it("条件字段渲染公式框（data-field-kind=formula）", () => {
    const { container } = render(<ConditionNode data={data} />);
    fireEvent.click(screen.getByTestId("token-preview")); // 点击进入编辑态才挂载公式框
    expect(container.querySelector('[data-field-kind="formula"]')).toBeTruthy();
  });

  it("条件斜杠键入弹候选（新契约：触发字符 / 弹全量候选）", () => {
    const { container } = render(<ConditionNode data={data} />);
    fireEvent.click(screen.getByTestId("token-preview"));
    const input = container.querySelector('[data-field-kind="formula"] textarea') as HTMLTextAreaElement;
    input.value = "/";
    input.setSelectionRange(1, 1); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(input, { target: { value: "/" } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    // 信号候选 label 为 "count: i32"，限定菜单作用域断言包含 count
    expect(container.querySelector('[role="listbox"]')?.textContent).toContain("count");
  });

  it("提交走 onEditCondition（条件文本更新到 branches）", () => {
    const onEditCondition = vi.fn();
    const { container } = render(<ConditionNode data={{ ...data, onEditCondition }} />);
    fireEvent.click(screen.getByTestId("token-preview"));
    const input = container.querySelector('[data-field-kind="formula"] textarea') as HTMLTextAreaElement;
    // 新契约：表达式以胶囊标记段输入（raw 自由文本会按字符串字面量加引号提交）
    input.value = "\uE000count\uE001 \uE000>\uE001 0";
    input.setSelectionRange(13, 13); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(input, { target: { value: "\uE000count\uE001 \uE000>\uE001 0" } });
    fireEvent.blur(input);
    expect(onEditCondition).toHaveBeenCalledWith("br0", "count > 0");
  });

  it("无 scope → 退化纯文本不弹层", () => {
    const { container } = render(<ConditionNode data={{ ...data, scope: undefined }} />);
    fireEvent.click(screen.getByTestId("token-preview"));
    const input = container.querySelector('[data-field-kind="formula"] textarea') as HTMLTextAreaElement;
    input.value = "/";
    input.setSelectionRange(1, 1); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(input, { target: { value: "/" } });
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});

describe("ReturnNode 公式框", () => {
  it("表达式字段渲染公式框且提交走 onCommitReturn", () => {
    const onCommitReturn = vi.fn();
    const { container } = render(
      <ReturnNode data={{ label: "返回", returnExpr: "", scope: formulaScope, letVars: [], onCommitReturn }} />,
    );
    fireEvent.click(screen.getByText("return")); // 点击进入编辑态才挂载公式框
    const input = container.querySelector('[data-field-kind="formula"] textarea') as HTMLTextAreaElement;
    // 新契约：表达式以胶囊标记段输入（raw 自由文本会按字符串字面量加引号提交）
    input.value = "\uE000count\uE001 \uE000+\uE001 1";
    input.setSelectionRange(13, 13); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(input, { target: { value: "\uE000count\uE001 \uE000+\uE001 1" } });
    fireEvent.blur(input);
    expect(onCommitReturn).toHaveBeenCalledWith("count + 1");
  });
});
