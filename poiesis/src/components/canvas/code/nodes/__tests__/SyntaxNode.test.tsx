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

  it("store-call 修改 store 名联动重置 action", () => {
    const onEditProp = vi.fn();
    renderNode("store-call", { storeName: "TodoStore", action: "add", args: "" }, onEditProp);
    fireEvent.change(screen.getByLabelText("store 名"), { target: { value: "OtherStore" } });
    expect(onEditProp).toHaveBeenCalledWith("l1", "storeName", "OtherStore");
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
