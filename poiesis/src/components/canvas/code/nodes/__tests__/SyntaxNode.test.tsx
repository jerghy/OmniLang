import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SyntaxNode } from "../SyntaxNode";
import type { FlowScope } from "../../../../../services/codegen/flow-scope";

const scope: FlowScope = {
  signals: [{ name: "count", type: "i32" }],
  props: [], computeds: [], events: [{ name: "onSelect" }],
  fns: [{ name: "close", params: [] }],
  stores: [{ name: "TodoStore", actions: [{ name: "add", params: [{ name: "text", type: "String" }] }], signals: [] }],
  fnParams: [], letVars: [],
};

function renderNode(tag: string, props: Record<string, unknown>, onEditProp = vi.fn()) {
  return render(
    <SyntaxNode data={{ id: "l1", label: tag, tag, props, scope, onEditProp }} />
  );
}

describe("SyntaxNode 内联编辑", () => {
  beforeEach(() => vi.clearAllMocks());

  it("let 节点渲染内联字段：name/value/mut", () => {
    renderNode("let", { name: "x", mutable: false, value: "0" });
    expect(screen.getByLabelText("变量名")).toBeTruthy();
    expect(screen.getByLabelText("初始值")).toBeTruthy();
    expect(screen.getByRole("button", { name: "mut" })).toBeTruthy();
  });

  it("let 修改变量名：blur 提交 onEditProp", () => {
    const onEditProp = vi.fn();
    renderNode("let", { name: "x", mutable: false, value: "0" }, onEditProp);
    const input = screen.getByLabelText("变量名");
    fireEvent.change(input, { target: { value: "count" } });
    fireEvent.blur(input);
    expect(onEditProp).toHaveBeenCalledWith("l1", "name", "count");
  });

  it("let mut 徽章切换提交 mutable", () => {
    const onEditProp = vi.fn();
    renderNode("let", { name: "x", mutable: false, value: "0" }, onEditProp);
    fireEvent.click(screen.getByRole("button", { name: "mut" }));
    expect(onEditProp).toHaveBeenCalledWith("l1", "mutable", true);
  });

  it("store-call 联动：action 下拉只含所选 store 的 actions", () => {
    renderNode("store-call", { storeName: "TodoStore", action: "add", args: "" });
    const select = screen.getByLabelText("action") as HTMLSelectElement;
    const opts = [...select.options].map((o) => o.value);
    expect(opts).toContain("add");
    expect(opts).not.toContain("nope");
  });

  it("store-call 修改 store 名 blur 提交并联动重置 action", () => {
    const onEditProp = vi.fn();
    renderNode("store-call", { storeName: "TodoStore", action: "add", args: "" }, onEditProp);
    const input = screen.getByLabelText("store 名");
    fireEvent.change(input, { target: { value: "OtherStore" } });
    expect(onEditProp).not.toHaveBeenCalled(); // 草稿阶段不提交
    fireEvent.blur(input);
    expect(onEditProp).toHaveBeenCalledWith("l1", "storeName", "OtherStore");
    expect(onEditProp).toHaveBeenCalledWith("l1", "action", "");
  });

  it("新建 store-call 节点可经 + 添加参数（接缝集成）", () => {
    const onEditProp = vi.fn();
    renderNode("store-call", { storeName: "TodoStore", action: "add", args: "" }, onEditProp);
    expect(screen.queryByLabelText("新参数")).toBeFalsy();
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    const input = screen.getByLabelText("新参数");
    fireEvent.change(input, { target: { value: "text" } });
    fireEvent.blur(input);
    expect(onEditProp).toHaveBeenCalledWith("l1", "args", "text");
  });

  it("call 的 emit 徽章显示 emit 前缀", () => {
    renderNode("call", { fnName: "onSelect", args: "x", emit: true });
    expect(screen.getAllByText("emit").length).toBeGreaterThan(0);
  });

  it("assign 运算符下拉变更提交", () => {
    const onEditProp = vi.fn();
    renderNode("assign", { target: "count", operator: "=", value: "0" }, onEditProp);
    fireEvent.change(screen.getByLabelText("运算符"), { target: { value: "+=" } });
    expect(onEditProp).toHaveBeenCalledWith("l1", "operator", "+=");
  });

  it("校验失败显示警告条", () => {
    renderNode("let", { name: "1bad", mutable: false });
    expect(screen.getByText(/变量名需为合法标识符/)).toBeTruthy();
  });

  it("预览代码仍为 formatSyntaxNode 输出", () => {
    renderNode("let", { name: "x", mutable: true, value: "0" });
    expect(screen.getByText("let mut x = 0;")).toBeTruthy();
  });

  it("store 名 datalist 提供候选", () => {
    renderNode("store-call", { storeName: "TodoStore", action: "add", args: "" });
    const list = document.querySelector("#syntax-store-candidates");
    expect(list).toBeTruthy();
    expect([...list!.querySelectorAll("option")].map((o) => o.value)).toContain("TodoStore");
  });

  it("action 校验失败时 select 带 invalid 样式", () => {
    renderNode("store-call", { storeName: "TodoStore", action: "nope", args: "" });
    const select = screen.getByLabelText("action");
    expect(select.className).toContain("invalid");
  });
});

describe("SyntaxNode 公式框集成", () => {
  const scope = {
    signals: [{ name: "count", type: "i32" }],
    props: [], computeds: [], events: [],
    fns: [{ name: "close", params: [] }],
    stores: [{ name: "ThemeStore", actions: [], signals: [{ name: "mode", type: "String" }] }],
    fnParams: [{ name: "step", type: "i32" }],
    letVars: [],
  };

  it("assign value 渲染公式框（data-field-kind=formula）", () => {
    const onEditProp = vi.fn();
    const { container } = render(
      <SyntaxNode data={{ id: "n1", label: "赋值", tag: "assign", props: { target: "count", operator: "=", value: "0" }, scope, onEditProp }} />,
    );
    expect(container.querySelector('[data-field-kind="formula"]')).toBeTruthy();
  });

  it("assign value 输入字母弹候选（scope 信号/变量）", () => {
    const { container } = render(
      <SyntaxNode data={{ id: "n1", label: "赋值", tag: "assign", props: { target: "count", operator: "=", value: "" }, scope, letVars: [{ name: "total" }], onEditProp: vi.fn() }} />,
    );
    const input = container.querySelector('[data-field-kind="formula"]') as HTMLInputElement;
    fireEvent.change(input, { target: { value: "tot", selectionStart: 3 } });
    // 候选菜单（role=listbox）内应出现 total；target 下拉也有同名 option（letVars 目标），故限定菜单作用域
    expect(container.querySelector('[role="listbox"]')?.textContent).toContain("total");
  });

  it("assign target 渲染下拉（NodeSelect + store 信号候选）", () => {
    const { container } = render(
      <SyntaxNode data={{ id: "n1", label: "赋值", tag: "assign", props: { target: "", operator: "=", value: "0" }, scope, onEditProp: vi.fn() }} />,
    );
    const select = container.querySelector('select[data-field-kind="select"]') as HTMLSelectElement;
    expect(select).toBeTruthy();
    expect(Array.from(select.options).map((o) => o.value)).toContain("ThemeStore.mode");
  });

  it("let value 渲染公式框且提交走 onEditProp", () => {
    const onEditProp = vi.fn();
    const { container } = render(
      <SyntaxNode data={{ id: "n1", label: "变量声明", tag: "let", props: { name: "x", mutable: false, value: "0" }, scope, onEditProp }} />,
    );
    const input = container.querySelector('[data-field-kind="formula"]') as HTMLInputElement;
    fireEvent.change(input, { target: { value: "count + 1" } });
    fireEvent.blur(input);
    expect(onEditProp).toHaveBeenCalledWith("n1", "value", "count + 1");
  });

  it("value 括号不配对 → 警告条（只提示不阻塞）", () => {
    const { container } = render(
      <SyntaxNode data={{ id: "n1", label: "赋值", tag: "assign", props: { target: "count", operator: "=", value: "(count + 1" }, scope, onEditProp: vi.fn() }} />,
    );
    expect(screen.getByText(/未闭合的括号/)).toBeTruthy();
    // 不阻塞：公式框仍可输入
    expect(container.querySelector('[data-field-kind="formula"]')).toBeTruthy();
  });
});
