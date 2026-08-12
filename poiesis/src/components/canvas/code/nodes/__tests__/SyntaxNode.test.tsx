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
    const input = screen.getByLabelText("新参数"); // store-call 传 candidates → 公式框（textarea 编辑层）
    const ta = input.querySelector("textarea") as HTMLTextAreaElement;
    // 新契约：标识符经斜杠逃生口（候选空 Enter → 引用胶囊），blur 提交
    ta.value = "/";
    ta.setSelectionRange(1, 1); // 触发字符落点必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(ta, { target: { value: "/" } }); // 第一步：斜杠触发菜单
    ta.value = "/text";
    ta.setSelectionRange(5, 5);
    fireEvent.input(ta, { target: { value: "/text" } }); // 第二步：菜单重过滤（候选空 → 仅运算符组）
    fireEvent.keyDown(ta, { key: "Enter" }); // 逃生口：引用胶囊
    expect(ta.value).toBe("\uE000text\uE001");
    fireEvent.blur(ta);
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

  it("let 值字段公式框内嵌胶囊渲染（新契约：data-seg-kind 覆盖层分段）", () => {
    const { container } = renderNode("let", { name: "x", mutable: false, value: "count + 1" });
    // NodeFormulaField 覆盖层分段渲染：identifier/operator 成胶囊；结构字符独立段；自由文本透明
    expect(container.querySelector('[data-testid="token-preview"]')).toBeNull(); // 预览行已删除
    expect(container.querySelectorAll('[data-seg-kind="capsule"]')).toHaveLength(2);
    expect(container.querySelector('[data-seg-kind="capsule"]')?.textContent).toBe("count");
    expect(container.querySelectorAll('[data-seg-kind="struct"]')).toHaveLength(0);
    // 覆盖层分段 join = 原始表达式（textarea 值=结构化 markup，jsdom textContent 含之，故限定覆盖层）
    const segText = [...container.querySelectorAll('[data-field-kind="formula"] [data-seg-kind]')].map((s) => s.textContent).join("");
    expect(segText).toBe("count + 1");
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

  it("assign value 斜杠键入弹候选（新契约：触发字符 / 弹全量候选）", () => {
    const { container } = render(
      <SyntaxNode data={{ id: "n1", label: "赋值", tag: "assign", props: { target: "count", operator: "=", value: "" }, scope, letVars: [{ name: "total" }], onEditProp: vi.fn() }} />,
    );
    const input = container.querySelector('[data-field-kind="formula"] textarea') as HTMLTextAreaElement;
    input.value = "/";
    input.setSelectionRange(1, 1); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(input, { target: { value: "/" } });
    // 候选菜单（role=listbox）内应出现 total；target 下拉也有同名 option（letVars 目标），故限定菜单作用域
    expect(document.querySelector('[role="listbox"]')?.textContent).toContain("total");
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
    const input = container.querySelector('[data-field-kind="formula"] textarea') as HTMLTextAreaElement;
    // 新契约：表达式以胶囊标记段输入（raw 自由文本会按字符串字面量加引号提交）
    input.value = "\uE000count\uE001 \uE000+\uE001 1";
    input.setSelectionRange(13, 13); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(input, { target: { value: "\uE000count\uE001 \uE000+\uE001 1" } });
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
