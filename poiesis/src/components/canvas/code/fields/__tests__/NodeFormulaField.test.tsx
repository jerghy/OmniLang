import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeFormulaField, type FormulaCandidateSource } from "../NodeFormulaField";

// contenteditable 测试以 jsdom 可实现为准：真实键盘编辑无法模拟，用
// `fireEvent.input(el, { target: { textContent } })` 直设文本 + input 事件
// （testing-library 对 target 的 textContent 做 Object.assign，等同真实键入）。
// 光标位置无 Selection 时回退到文本末尾（caretOffset 契约），与各用例光标在尾一致。

const cands: FormulaCandidateSource = {
  filter: (q) => (q === "" ? [{ value: "count", label: "count" }] : [{ value: "count", label: "count" }].filter((c) => c.value.startsWith(q))),
  storeSignals: (name) => (name === "ThemeStore" ? [{ value: "ThemeStore.mode", label: "ThemeStore.mode" }] : []),
};

function setup(initial = "", candidates: FormulaCandidateSource | undefined = cands, onCommit = vi.fn()) {
  const { container } = render(<NodeFormulaField value={initial} onCommit={onCommit} candidates={candidates} ariaLabel="初始值" />);
  return { container, el: container.querySelector("[contenteditable]") as HTMLElement, onCommit };
}

describe("NodeFormulaField Notion 式分词", () => {
  it("输入即分词：键入 `count + 1` 渲染 token 胶囊", () => {
    const { container, el } = setup();
    fireEvent.input(el, { target: { textContent: "count + 1" } });
    expect(container.querySelectorAll('[data-token-kind="identifier"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-token-kind="operator"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-token-kind="number"]')).toHaveLength(1);
  });

  it("初始 value 挂载即分词渲染（结构同 TokenPreview）", () => {
    const { container } = setup("TodoStore.total == 1");
    expect(container.querySelector('[data-token-kind="member"]')?.textContent).toBe("TodoStore.total");
    expect(container.querySelector('[data-token-kind="operator"]')?.textContent).toBe("==");
    expect(container.querySelector('[data-token-kind="number"]')?.textContent).toBe("1");
  });

  it("敲运算符字符：直插成符号 token 且弹候选（多字符变体）", () => {
    const { container, el } = setup("count <");
    // 输入 `<` 前缀后继续输入 `=` → 完整 `<=` token
    fireEvent.input(el, { target: { textContent: "count <=" } });
    expect(container.querySelector('[data-token-kind="operator"]')?.textContent).toBe("<=");
  });

  it("Enter 提交字符串（草稿语义不变）", () => {
    const { onCommit, el } = setup();
    fireEvent.input(el, { target: { textContent: "count + 1" } });
    fireEvent.keyDown(el, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("count + 1");
  });

  it("IME 组合期间不分词（compositionstart → 原样文本），组合结束恢复分词", () => {
    const { container, el } = setup();
    fireEvent.compositionStart(el);
    fireEvent.input(el, { target: { textContent: "变量名" } });
    expect(container.querySelectorAll('[data-token-kind="identifier"]')).toHaveLength(0);
    fireEvent.compositionEnd(el, { target: { textContent: "变量名" } });
    expect(el.textContent).toBe("变量名");
    expect(container.querySelectorAll("[data-token-kind]").length).toBeGreaterThan(0);
  });

  it("无 candidates 退化纯文本输入（无分词无弹层，blur 仍提交）", () => {
    // 注意：不能经 setup() 传 undefined（默认参数会兜底成 cands），直接渲染
    const onCommit = vi.fn();
    const { container } = render(<NodeFormulaField value="" onCommit={onCommit} ariaLabel="初始值" />);
    const el = container.querySelector("[contenteditable]") as HTMLElement;
    fireEvent.input(el, { target: { textContent: "abc" } });
    expect(container.querySelectorAll("[data-token-kind]")).toHaveLength(0);
    expect(screen.queryByRole("listbox")).toBeNull();
    fireEvent.blur(el);
    expect(onCommit).toHaveBeenCalledWith("abc");
  });

  it("光标在数字字面量内不弹候选（字面量无补全语义，Enter 提交契约的基础）", () => {
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: "count + 1" } });
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("Backspace 在标识符 token 末尾整块删除", () => {
    const { el } = setup("count");
    fireEvent.keyDown(el, { key: "Backspace" });
    expect(el.textContent).toBe("");
  });
});

describe("NodeFormulaField 草稿提交与候选菜单", () => {
  it("blur 提交 onCommit（草稿语义不变）", () => {
    const { el, onCommit } = setup("0");
    fireEvent.input(el, { target: { textContent: "count + 1" } });
    expect(onCommit).not.toHaveBeenCalled();
    fireEvent.blur(el);
    expect(onCommit).toHaveBeenCalledWith("count + 1");
  });

  it("输入字母 → 弹候选（operand 上下文）", () => {
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: "cou" } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    expect(screen.getByText("count")).toBeTruthy();
  });

  it("Tab 选中候选 → 替换当前 token 并关闭弹层（不提交）", () => {
    const { el, onCommit } = setup();
    fireEvent.input(el, { target: { textContent: "cou" } });
    fireEvent.keyDown(el, { key: "Tab" });
    expect(el.textContent).toBe("count");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("输入单字符运算符（+）→ 直接插入不弹框（无多字符变体）", () => {
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: "count +" } });
    expect(el.textContent).toBe("count +");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("输入运算符前缀（<）→ 弹运算符选择框", () => {
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: "count <" } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    expect(screen.getByText("<=")).toBeTruthy();
  });

  it("完整运算符串（<=）无变体候选 → 不弹层（不产生截断替换）", () => {
    // 完整 "<=" 在候选表中无前缀 → 弹层关闭，避免选中候选只替换前半段、残留 "="（"count <==" 污染）
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: "count <=" } });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(el.textContent).toBe("count <=");
  });

  it("Enter 在弹层打开时选中候选（提交语义由后续 blur 完成）", () => {
    const { el, onCommit } = setup();
    fireEvent.input(el, { target: { textContent: "cou" } });
    fireEvent.keyDown(el, { key: "Enter" });
    expect(el.textContent).toBe("count");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("Esc 关闭弹层（草稿保留，不提交不取消）", () => {
    const { el, onCommit } = setup("0");
    fireEvent.input(el, { target: { textContent: "cou" } });
    fireEvent.keyDown(el, { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(el.textContent).toBe("cou"); // 草稿保留
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("Store. 输入 → member 候选（storeSignals）", () => {
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: "ThemeStore." } });
    expect(screen.getByText("ThemeStore.mode")).toBeTruthy();
  });

  it("member 上下文选中候选：整段替换 Store. 前缀（不重复）", () => {
    // E2E 缺陷 2 回归：修复前选中产生 `demostore.demostore.newSignal` 前缀重复
    const candsStore: FormulaCandidateSource = {
      filter: () => [],
      storeSignals: (name) => (name === "demostore" ? [{ value: "demostore.newSignal", label: "demostore.newSignal" }] : []),
    };
    const { el } = setup("", candsStore);
    fireEvent.input(el, { target: { textContent: "demostore." } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    fireEvent.keyDown(el, { key: "Tab" });
    expect(el.textContent).toBe("demostore.newSignal");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("↑↓ 导航候选，Enter 选中第二项", () => {
    const cands2: FormulaCandidateSource = {
      filter: () => [{ value: "a", label: "a" }, { value: "b", label: "b" }],
      storeSignals: () => [],
    };
    const { el } = setup("", cands2);
    fireEvent.input(el, { target: { textContent: "" } });
    fireEvent.keyDown(el, { key: "ArrowDown" });
    fireEvent.keyDown(el, { key: "Enter" });
    expect(el.textContent).toBe("b");
  });

  it("IME 组合输入期间不弹候选", () => {
    const { el } = setup();
    fireEvent.compositionStart(el);
    // 组合期间 input 的 isComposing 为 true（真实浏览器语义；jsdom 默认 false 需显式给）
    fireEvent.input(el, { target: { textContent: "中" }, isComposing: true });
    expect(screen.queryByRole("listbox")).toBeNull();
    fireEvent.compositionEnd(el, { target: { textContent: "中" } });
  });

  it("compositionend 丢失后非组合 input 复位 IME 状态——候选弹层恢复（卡死兜底）", () => {
    // RED（2026-08-06 真机）：React 委托可能收到 compositionstart 却丢失
    // compositionend → composingRef 卡 true → 所有输入跳过光标恢复与候选弹层
    // （光标每次输入跳回开头）。修复：非组合 input（isComposing === false）即复位。
    const { el } = setup();
    fireEvent.compositionStart(el);
    fireEvent.input(el, { target: { textContent: "中" }, isComposing: true });
    expect(screen.queryByRole("listbox")).toBeNull(); // 组合中不弹
    // 组合结束后的提交 input：真实浏览器 isComposing=false → 复位 composingRef
    fireEvent.input(el, { target: { textContent: "变量" }, isComposing: false });
    expect(screen.getByRole("listbox")).toBeTruthy(); // 弹层恢复
  });

  it("IME 组合结束保留组合文本外的既有字符（真实浏览器 DOM 已上屏，不整树覆盖）", () => {
    // RED（Task 12 E2E）：onCompositionEnd 里 `textContent = e.data` 整树覆盖，
    // 吞掉组合文本前的既有字符（字符串左引号）——"你好" 变 "你好"，字面量残缺。
    // 真实浏览器 compositionend 时 DOM 已自动上屏为 前置字符+组合结果，无需手动覆盖。
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: '"' } }); // 左引号
    fireEvent.compositionStart(el);
    fireEvent.input(el, { target: { textContent: '"nihao' } }); // 组合期拼音
    // 组合结束：浏览器 DOM 已是 '"你好'（前置引号保留）
    fireEvent.compositionEnd(el, { target: { textContent: '"你好' }, data: "你好" });
    expect(el.textContent).toBe('"你好');
  });

  it("string token 渲染保留引号（textContent 与 draft 双向一致，编辑不丢引号）", () => {
    // RED（Task 12 E2E）：渲染用 t.text（Task 7 契约 text 不含引号）→ string token 剥引号
    // → textContent ≠ draft → 后续编辑读 textContent 丢引号（"你好" + 变 你好+）。
    // 与 TokenPreview 对齐：渲染统一取 draft.slice(t.start, t.end) 原文（含引号）。
    const { container, el } = setup();
    fireEvent.input(el, { target: { textContent: '"你好"' } });
    const str = container.querySelector('[data-token-kind="string"]');
    expect(str?.textContent).toBe('"你好"');
    // 渲染后继续编辑：textContent 含引号 → draft 不丢
    fireEvent.input(el, { target: { textContent: '"你好" +' } });
    expect(el.textContent).toBe('"你好" +');
  });

  it("运算符候选选择不重复前缀（replaceToken 替换 token）", () => {
    const { el } = setup();
    fireEvent.input(el, { target: { textContent: "count <" } });
    // 默认第一项 "<" → 替换后值不变（insertAt 会得到 "count <<"）
    fireEvent.keyDown(el, { key: "Enter" });
    expect(el.textContent).toBe("count <");
    // 重开弹层，导航到 "<=" 选中 → 前缀不重复
    fireEvent.input(el, { target: { textContent: "count <" } });
    fireEvent.keyDown(el, { key: "ArrowDown" });
    fireEvent.keyDown(el, { key: "Enter" });
    expect(el.textContent).toBe("count <=");
  });

  it("鼠标点击候选不提前提交（菜单项 onMouseDown 防夺焦）", () => {
    const { el, onCommit } = setup();
    fireEvent.input(el, { target: { textContent: "cou" } });
    const option = screen.getByRole("option");
    fireEvent.mouseDown(option);
    fireEvent.click(option);
    expect(onCommit).not.toHaveBeenCalled(); // 未完成草稿不得被 blur 提前提交
    expect(el.textContent).toBe("count"); // 候选已替换进草稿
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});

describe("NodeFormulaField memo 隔离（React 19 无条件重设 innerHTML 的光标保护）", () => {
  it("父组件重渲染（数据 props 引用不变）不重设字段 innerHTML", () => {
    // RED（2026-08-06 真机）：reactflow 节点选中触发父渲染 → React 19 updateProperties
    // 对 dangerouslySetInnerHTML 无条件重设 → 光标重置到开头。memo + 自定义比较器
    // （忽略函数 props）应跳过渲染——innerHTML 不变是光标保留的前提。
    const { container, rerender } = render(
      <NodeFormulaField value="count + 1" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />
    );
    const el = container.querySelector("[contenteditable]") as HTMLElement;
    const htmlBefore = el.innerHTML;
    // 父渲染：onCommit 换成新函数引用（reactflow 节点渲染常态），数据 props 不变
    rerender(
      <NodeFormulaField value="count + 1" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />
    );
    expect(el.innerHTML).toBe(htmlBefore); // 无 innerHTML 重设 = 无光标重置
  });

  it("数据 props 变化仍正常重渲染（value 更新）", () => {
    const { container, rerender } = render(
      <NodeFormulaField value="count" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />
    );
    const el = container.querySelector("[contenteditable]") as HTMLElement;
    rerender(
      <NodeFormulaField value="count + 2" onCommit={vi.fn()} candidates={cands} ariaLabel="初始值" />
    );
    expect(el.textContent).toBe("count + 2");
  });
});
