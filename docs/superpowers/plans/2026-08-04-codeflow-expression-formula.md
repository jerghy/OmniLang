# CodeFlowCanvas 表达式公式框 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** assign/let 节点的 value 字段升级为「文本 + 触发式自动补全」公式框（变量/函数候选 + 运算符字符触发插入），assign target 改下拉；附带修复 F1（组件 fn 画布 reload 物化）与 F2（fn 候选恒空）。

**Architecture:** 纯函数触发规则（expression-trigger）+ 受控候选弹层（NodeCandidateMenu）+ 草稿提交语义公式框（NodeFormulaField，复用 NodeTextField 模式），三件套接入 SyntaxNode；数据侧：parser 登记 fnRegistry（F2）并合成 fn 元素（F1 解析侧），App.tsx 物化 codeFlowGraphs 进 store，DesignPage 进入 codeflow 时按组件合并物化。

**Tech Stack:** React 19、TypeScript、Zustand 5、@xyflow/react 12、vitest + @testing-library/react（jsdom）、无新依赖。

## Global Constraints

- **319 vitest 基线全绿**（`cd poiesis && npx vitest run`）；`npx tsc --noEmit` 零新增错误
- **零改动**：`fields/NodeField.tsx`、`services/codegen/syntax-nodes.ts`（formatSyntaxNode）、`graph-compiler.ts`、`graph-parser.ts` 的 parse/compile 契约、`@pid` round-trip 机制
- **运算符集合以编译器权威为准**（`crates/omnilang-core/src/parser/expr/infix.rs`）：`+ - * / %`、`== != < > <= >=`、`&& ||`、`??`、`is`、`?.`、一元 `!`；**`=` 单等号在表达式中不合法，弹框只提供 `==`**
- **只做 assign value / let value 公式框 + assign target 下拉**；condition/return/call/store-call args 字段不动
- 所有内联控件 `class="nodrag"`；中文输入法组合期间不触发候选（`isComposing` 守卫）
- 候选插入是**草稿的一部分**：不立即提交，Enter/blur 才 `onCommit`（props 仍是唯一事实来源）
- 代码与注释风格匹配现有文件（中文注释、命名 camelCase、纯函数可测）
- 提交消息遵循仓库既有格式（`feat(canvas): 中文描述` 风格）

---

### Task 1: expression-trigger 纯函数（触发规则表驱动）

**Files:**
- Create: `poiesis/src/components/canvas/code/fields/expression-trigger.ts`
- Test: `poiesis/src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`

**Interfaces:**
- Produces:
  ```ts
  export const SINGLE_CHAR_OPERATORS = ["+", "-", "*", "/", "%"];
  export const OPERATOR_PREFIX_CANDIDATES: Record<string, string[]> = {
    "<": ["<", "<="],
    "=": ["=="],
    "!": ["!=", "!"],
    "&": ["&&"],
    "|": ["||"],
    "?": ["??", "?."],
  };
  export interface TriggerContext {
    kind: "operand" | "member" | "operator" | "none";
    /** 光标前 token 文本（operand=标识符串；operator=运算符串；member=空串） */
    token: string;
    /** token 起始偏移（供替换） */
    tokenStart: number;
    /** member 时：点前的标识符（store 名），否则 undefined */
    storeName?: string;
  }
  export function getTriggerContext(value: string, cursor: number): TriggerContext;
  /** 运算符前缀 → 候选（OPERATOR_PREFIX_CANDIDATES 查表，未收录前缀返回 []） */
  export function operatorCandidates(prefix: string): Array<{ value: string; label: string }>;
  /** 光标处插入文本（不替换既有内容） */
  export function insertAt(value: string, cursor: number, insert: string): { value: string; cursor: number };
  /** 用 insert 替换 [tokenStart, cursor) 区间 */
  export function replaceToken(value: string, tokenStart: number, cursor: number, insert: string): { value: string; cursor: number };
  /** 轻量括号配对校验：未配对的 ( 数量为正、) 数量为负、配对为 0（跳过字符串字面量内的括号） */
  export function countUnbalancedParens(value: string): number;
  ```

- [ ] **Step 1: 写失败测试**（表驱动，覆盖规则全集 + 括号配对）

```ts
import { describe, it, expect } from "vitest";
import { getTriggerContext, operatorCandidates, insertAt, replaceToken, countUnbalancedParens, SINGLE_CHAR_OPERATORS } from "../expression-trigger";

describe("getTriggerContext", () => {
  it("空输入 → none", () => {
    expect(getTriggerContext("", 0)).toEqual({ kind: "none", token: "", tokenStart: 0 });
  });
  it("操作数位置输入字母 → operand", () => {
    expect(getTriggerContext("cou", 3)).toEqual({ kind: "operand", token: "cou", tokenStart: 0 });
  });
  it("运算符后的新操作数 → operand（token 空串）", () => {
    expect(getTriggerContext("count + ", 8)).toEqual({ kind: "operand", token: "", tokenStart: 8 });
  });
  it("运算符字符 → operator", () => {
    expect(getTriggerContext("count <", 6)).toEqual({ kind: "operator", token: "<", tokenStart: 6 });
  });
  it("单个运算符字符（+）→ operator 且 token 为单字符", () => {
    expect(getTriggerContext("count +", 6)).toEqual({ kind: "operator", token: "+", tokenStart: 6 });
  });
  it("成员点（Store.）→ member，带 storeName", () => {
    expect(getTriggerContext("ThemeStore.", 11)).toEqual({ kind: "member", token: "", tokenStart: 11, storeName: "ThemeStore" });
  });
  it("数字字面量 → operand", () => {
    expect(getTriggerContext("count + 5", 9)).toEqual({ kind: "operand", token: "5", tokenStart: 8 });
  });
  it("左括号后 → operand", () => {
    expect(getTriggerContext("max(", 4)).toEqual({ kind: "operand", token: "", tokenStart: 4 });
  });
  it("非表达式字符（右括号/分号）→ none", () => {
    expect(getTriggerContext("a)", 2)).toEqual({ kind: "none", token: "", tokenStart: 2 });
  });
});

describe("operatorCandidates", () => {
  it("前缀过滤：< → [<, <=]", () => {
    expect(operatorCandidates("<").map((o) => o.value)).toEqual(["<", "<="]);
  });
  it("= → 只提供 ==（不提供赋值 =）", () => {
    expect(operatorCandidates("=").map((o) => o.value)).toEqual(["=="]);
  });
  it("! → [!=, !]", () => {
    expect(operatorCandidates("!").map((o) => o.value)).toEqual(["!=", "!"]);
  });
  it("未收录前缀（如 .）→ []", () => {
    expect(operatorCandidates(".")).toEqual([]);
  });
});

describe("insertAt / replaceToken", () => {
  it("insertAt 光标处插入", () => {
    expect(insertAt("count ", 6, "+ ")).toEqual({ value: "count + ", cursor: 8 });
  });
  it("replaceToken 替换当前 token", () => {
    expect(replaceToken("cou", 0, 3, "count")).toEqual({ value: "count", cursor: 5 });
  });
  it("replaceToken 替换中间 token（保留两侧）", () => {
    expect(replaceToken("a + cou + b", 4, 7, "count")).toEqual({ value: "a + count + b", cursor: 9 });
  });
});

describe("countUnbalancedParens（轻量括号配对校验，spec §4.5）", () => {
  it("配对完整 → 0", () => {
    expect(countUnbalancedParens("max(count, 0) + 1")).toBe(0);
  });
  it("多一个左括号 → 1", () => {
    expect(countUnbalancedParens("(count + 1")).toBe(1);
  });
  it("多一个右括号 → -1", () => {
    expect(countUnbalancedParens("count + 1)")).toBe(-1);
  });
  it("字符串字面量内的括号不计数（简单引号跳过）", () => {
    expect(countUnbalancedParens('"(" + ")"')).toBe(0);
  });
  it("空串 → 0", () => {
    expect(countUnbalancedParens("")).toBe(0);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: FAIL — `Cannot find module '../expression-trigger'`

- [ ] **Step 3: 最小实现**

```ts
// poiesis/src/components/canvas/code/fields/expression-trigger.ts
// 表达式公式框的触发规则纯函数（表驱动，可单测）。
// 运算符集合以编译器 infix.rs 为权威：+ - * / % == != < > <= >= && || ?? is ?. 与一元 !。

export const SINGLE_CHAR_OPERATORS = ["+", "-", "*", "/", "%"];

export const OPERATOR_PREFIX_CANDIDATES: Record<string, string[]> = {
  "<": ["<", "<="],
  "=": ["=="],
  "!": ["!=", "!"],
  "&": ["&&"],
  "|": ["||"],
  "?": ["??", "?."],
};

export interface TriggerContext {
  kind: "operand" | "member" | "operator" | "none";
  token: string;
  tokenStart: number;
  storeName?: string;
}

const IDENT_CHAR = /[A-Za-z0-9_]/;
const OPERATOR_CHAR = /[+\-*/%<>=!&|?]/;

export function getTriggerContext(value: string, cursor: number): TriggerContext {
  const before = value.slice(0, cursor);
  // 成员访问：光标前一字符是 "."
  if (before.endsWith(".")) {
    const storeMatch = before.slice(0, -1).match(/[A-Za-z_]\w*$/);
    return { kind: "member", token: "", tokenStart: cursor, storeName: storeMatch?.[0] };
  }
  // 标识符 token：从光标向前收集字母数字下划线
  let i = cursor;
  while (i > 0 && IDENT_CHAR.test(before[i - 1]!)) i--;
  if (i < cursor) return { kind: "operand", token: before.slice(i, cursor), tokenStart: i };
  // 运算符串：从光标向前收集运算符字符
  while (i > 0 && OPERATOR_CHAR.test(before[i - 1]!)) i--;
  if (i < cursor) return { kind: "operator", token: before.slice(i, cursor), tokenStart: i };
  // 空/其它字符（右括号等）→ 操作数位置（允许空 token 弹全量候选）
  if (cursor === 0 || /[(\s,=]/.test(before[cursor - 1]!)) {
    return { kind: "operand", token: "", tokenStart: cursor };
  }
  return { kind: "none", token: "", tokenStart: cursor };
}

export function operatorCandidates(prefix: string): Array<{ value: string; label: string }> {
  return (OPERATOR_PREFIX_CANDIDATES[prefix] ?? []).map((v) => ({ value: v, label: v }));
}

export function insertAt(value: string, cursor: number, insert: string): { value: string; cursor: number } {
  return { value: value.slice(0, cursor) + insert + value.slice(cursor), cursor: cursor + insert.length };
}

export function replaceToken(value: string, tokenStart: number, cursor: number, insert: string): { value: string; cursor: number } {
  const next = value.slice(0, tokenStart) + insert + value.slice(cursor);
  return { value: next, cursor: tokenStart + insert.length };
}

/** 轻量括号配对校验：跳过字符串字面量（单/双引号）内的括号。 */
export function countUnbalancedParens(value: string): number {
  let depth = 0;
  let quote: "'" | '"' | null = null;
  for (let i = 0; i < value.length; i++) {
    const ch = value[i]!;
    if (quote) {
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === "'" || ch === '"') { quote = ch; continue; }
    if (ch === "(") depth++;
    else if (ch === ")") depth--;
  }
  return depth;
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: PASS（17 用例全绿）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/fields/expression-trigger.ts poiesis/src/components/canvas/code/fields/__tests__/expression-trigger.test.ts
git commit -m "feat(canvas): 表达式公式框触发规则纯函数（表驱动：operand/member/operator 上下文 + 运算符前缀候选 + 括号配对校验）"
```

---

### Task 2: NodeCandidateMenu 候选弹层（受控组件）

**Files:**
- Create: `poiesis/src/components/canvas/code/fields/NodeCandidateMenu.tsx` + `NodeCandidateMenu.module.css`
- Test: `poiesis/src/components/canvas/code/fields/__tests__/NodeCandidateMenu.test.tsx`

**Interfaces:**
- Consumes: Task 1 的 `operatorCandidates`（可选，本任务不直接依赖——菜单是纯展示）
- Produces:
  ```ts
  export interface CandidateItem { value: string; label: string }
  export interface CandidateGroup { label: string; items: CandidateItem[] }
  interface Props {
    x: number; y: number;              // 弹层定位（fixed）
    groups: CandidateGroup[];          // 分组候选
    activeIdx: number;                 // 扁平索引（父组件持有键盘导航状态）
    onSelect: (item: CandidateItem) => void;
    onMouseEnter: (idx: number) => void;
    onClose: () => void;
  }
  export function NodeCandidateMenu(props: Props): JSX.Element;
  ```

- [ ] **Step 1: 写失败测试**

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeCandidateMenu } from "../NodeCandidateMenu";

const groups = [
  { label: "变量", items: [{ value: "count", label: "count" }, { value: "total", label: "total" }] },
  { label: "信号", items: [{ value: "ThemeStore.mode", label: "ThemeStore.mode" }] },
];

describe("NodeCandidateMenu", () => {
  it("渲染分组与选项（role=listbox/option + aria-selected）", () => {
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByRole("listbox")).toBeTruthy();
    const opts = screen.getAllByRole("option");
    expect(opts).toHaveLength(3);
    expect(opts[0]).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("变量")).toBeTruthy();
    expect(screen.getByText("信号")).toBeTruthy();
  });
  it("activeIdx 高亮对应项", () => {
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={2} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getAllByRole("option")[2]).toHaveAttribute("aria-selected", "true");
  });
  it("点击选项 → onSelect(item)", () => {
    const onSelect = vi.fn();
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={onSelect} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    fireEvent.click(screen.getByText("total"));
    expect(onSelect).toHaveBeenCalledWith({ value: "total", label: "total" });
  });
  it("鼠标悬停 → onMouseEnter(idx)", () => {
    const onMouseEnter = vi.fn();
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={vi.fn()} onMouseEnter={onMouseEnter} onClose={vi.fn()} />);
    fireEvent.mouseEnter(screen.getByText("ThemeStore.mode"));
    expect(onMouseEnter).toHaveBeenCalledWith(2);
  });
  it("无候选 → 显示无匹配项（不渲染 option）", () => {
    render(<NodeCandidateMenu x={0} y={0} groups={[]} activeIdx={0} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByText("无匹配项")).toBeTruthy();
    expect(screen.queryAllByRole("option")).toHaveLength(0);
  });
  it("点击菜单外 → onClose", () => {
    const onClose = vi.fn();
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={onClose} />);
    fireEvent.mouseDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeCandidateMenu.test.tsx`
Expected: FAIL — module not found

- [ ] **Step 3: 最小实现**

```tsx
// poiesis/src/components/canvas/code/fields/NodeCandidateMenu.tsx
// 公式框候选弹层（受控）：父组件（NodeFormulaField）持有键盘导航状态，本组件纯展示。
// 交互模式复刻 NodeSearchMenu（role=listbox/option、aria-selected、命中项高亮）。
import { useEffect, useRef } from "react";
import styles from "./NodeCandidateMenu.module.css";

export interface CandidateItem { value: string; label: string }
export interface CandidateGroup { label: string; items: CandidateItem[] }

interface Props {
  x: number;
  y: number;
  groups: CandidateGroup[];
  activeIdx: number;
  onSelect: (item: CandidateItem) => void;
  onMouseEnter: (idx: number) => void;
  onClose: () => void;
}

export function NodeCandidateMenu({ x, y, groups, activeIdx, onSelect, onMouseEnter, onClose }: Props) {
  const rootRef = useRef<HTMLDivElement>(null);
  const flat = groups.flatMap((g) => g.items);

  // 点击菜单外关闭
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  // 选中项滚动可见
  useEffect(() => {
    rootRef.current?.querySelector(`[data-cand-index="${activeIdx}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [activeIdx]);

  return (
    <div ref={rootRef} className={styles.menu} style={{ left: x, top: y }} role="listbox" aria-label="表达式候选">
      {flat.length === 0 ? (
        <div className={styles.empty}>无匹配项</div>
      ) : (
        groups.map((g) => (
          <div key={g.label}>
            <div className={styles.sectionLabel}>{g.label}</div>
            {g.items.map((item) => {
              const idx = flat.indexOf(item);
              return (
                <button
                  type="button"
                  key={item.value}
                  data-cand-index={idx}
                  role="option"
                  aria-selected={idx === activeIdx}
                  className={`${styles.item} ${idx === activeIdx ? styles.itemActive : ""}`}
                  onClick={() => onSelect(item)}
                  onMouseEnter={() => onMouseEnter(idx)}
                >
                  <span>{item.label}</span>
                </button>
              );
            })}
          </div>
        ))
      )}
    </div>
  );
}
```

```css
/* NodeCandidateMenu.module.css */
.menu {
  position: fixed;
  z-index: 10002;
  min-width: 200px;
  max-width: 320px;
  max-height: 240px;
  overflow-y: auto;
  background: var(--ol-surface, #fff);
  border: 1px solid var(--ol-border, #e2e8f0);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  padding: 4px 0;
  font-size: 12px;
}
.sectionLabel {
  padding: 4px 10px;
  color: var(--ol-text-muted, #94a3b8);
  font-size: 11px;
}
.item {
  display: block;
  width: 100%;
  text-align: left;
  padding: 5px 10px;
  border: none;
  background: transparent;
  cursor: pointer;
  font-family: inherit;
  font-size: 12px;
  color: var(--ol-text, #1e293b);
}
.item:hover { background: var(--ol-gray-100, #f1f5f9); }
.itemActive { background: var(--ol-accent-soft, #eef2ff); }
.empty { padding: 8px 10px; color: var(--ol-text-muted, #94a3b8); }
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeCandidateMenu.test.tsx`
Expected: PASS（6 用例全绿）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/fields/NodeCandidateMenu.tsx poiesis/src/components/canvas/code/fields/NodeCandidateMenu.module.css poiesis/src/components/canvas/code/fields/__tests__/NodeCandidateMenu.test.tsx
git commit -m "feat(canvas): 公式框候选弹层 NodeCandidateMenu（受控 listbox，复刻 NodeSearchMenu 交互）"
```

---

### Task 3: NodeFormulaField 公式框控件（草稿提交 + 触发补全）

**Files:**
- Create: `poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx` + `NodeFormulaField.module.css`
- Test: `poiesis/src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`

**Interfaces:**
- Consumes: Task 1（`getTriggerContext`/`operatorCandidates`/`insertAt`/`replaceToken`/`SINGLE_CHAR_OPERATORS`）、Task 2（`NodeCandidateMenu`/`CandidateItem`/`CandidateGroup`）
- Produces:
  ```ts
  export interface FormulaCandidateSource {
    /** operand 上下文候选（query 前缀过滤，空 query 返回全量前 8） */
    filter(query: string): CandidateItem[];
    /** member 上下文候选（storeName 点后） */
    storeSignals(storeName: string): CandidateItem[];
  }
  interface Props {
    value: string;
    onCommit: (v: string) => void;
    candidates?: FormulaCandidateSource;   // 缺省退化为纯文本输入（无弹层）
    placeholder?: string;
    ariaLabel?: string;
    className?: string;
    invalid?: boolean;
    onCancel?: () => void;
    onBlur?: () => void;
  }
  export function NodeFormulaField(props: Props): JSX.Element;
  ```

- [ ] **Step 1: 写失败测试**

```tsx
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
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`
Expected: FAIL — module not found

- [ ] **Step 3: 最小实现**

```tsx
// poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx
// 表达式公式框：NodeTextField 的草稿提交语义 + 触发式自动补全（字符触发+上下文候选）。
// 候选插入是草稿的一部分：Enter/blur 才 onCommit（props 仍是唯一事实来源）。
import { useEffect, useRef, useState, type CompositionEvent } from "react";
import { getTriggerContext, operatorCandidates, insertAt, replaceToken, SINGLE_CHAR_OPERATORS } from "./expression-trigger";
import { NodeCandidateMenu, type CandidateItem } from "./NodeCandidateMenu";
import styles from "./NodeField.module.css";

export interface FormulaCandidateSource {
  filter(query: string): CandidateItem[];
  storeSignals(storeName: string): CandidateItem[];
}

interface Props {
  value: string;
  onCommit: (v: string) => void;
  candidates?: FormulaCandidateSource;
  placeholder?: string;
  ariaLabel?: string;
  className?: string;
  invalid?: boolean;
  onCancel?: () => void;
  onBlur?: () => void;
}

interface MenuState {
  x: number;
  y: number;
  groups: Array<{ label: string; items: CandidateItem[] }>;
  activeIdx: number;
  replace: { tokenStart: number; cursor: number } | null; // null = 光标处插入（运算符）
}

export function NodeFormulaField({ value, onCommit, candidates, placeholder, ariaLabel, className, invalid, onCancel, onBlur }: Props) {
  const [draft, setDraft] = useState(value);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const focusedRef = useRef(false);
  const dirtyRef = useRef(false);
  const composingRef = useRef(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!focusedRef.current) setDraft(value);
  }, [value]);

  const commit = () => {
    if (dirtyRef.current && draft !== value) onCommit(draft);
    dirtyRef.current = false;
    setMenu(null);
  };
  const cancel = () => {
    focusedRef.current = false;
    dirtyRef.current = false;
    setMenu(null);
    setDraft(value);
    onCancel?.();
  };

  /** 输入变化 → 触发规则 → 更新草稿 + 弹层状态 */
  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const next = e.target.value;
    const cursor = e.target.selectionStart ?? next.length;
    setDraft(next);
    dirtyRef.current = true;
    if (composingRef.current || !candidates) {
      setMenu(null);
      return;
    }
    const ctx = getTriggerContext(next, cursor);
    if (ctx.kind === "none") { setMenu(null); return; }
    if (ctx.kind === "operator") {
      if (SINGLE_CHAR_OPERATORS.includes(ctx.token)) {
        // 单字符运算符直接插入（光标在尾部时；否则不处理——由下一次输入收敛）
        if (cursor === next.length) {
          const { value: v, cursor: c } = insertAt(next, cursor, ctx.token);
          setDraft(v);
          e.target.value = v;
          e.target.setSelectionRange(c, c);
        }
        setMenu(null);
        return;
      }
      const items = operatorCandidates(ctx.token);
      if (items.length === 0) { setMenu(null); return; }
      openMenu(tokenStart, cursor, [{ label: "运算符", items }], null);
      return;
    }
    if (ctx.kind === "member") {
      const items = candidates.storeSignals(ctx.storeName ?? "");
      if (items.length === 0) { setMenu(null); return; }
      openMenu(tokenStart, cursor, [{ label: ctx.storeName ? `${ctx.storeName} 信号` : "成员", items }], { tokenStart: ctx.tokenStart, cursor });
      return;
    }
    // operand：变量/函数候选（前缀过滤）
    const items = candidates.filter(ctx.token);
    if (items.length === 0) { setMenu(null); return; }
    openMenu(tokenStart, cursor, [{ label: "候选", items }], { tokenStart: ctx.tokenStart, cursor });
  };

  const openMenu = (tokenStart: number, cursor: number, groups: MenuState["groups"], replace: MenuState["replace"]) => {
    const rect = inputRef.current?.getBoundingClientRect();
    if (!rect) { setMenu(null); return; }
    setMenu({ x: rect.left, y: rect.bottom + 4, groups, activeIdx: 0, replace });
  };

  /** 选中候选：替换 token 或光标处插入；关闭弹层，保持聚焦与草稿 */
  const selectItem = (item: CandidateItem) => {
    if (!menu) return;
    const cursor = inputRef.current?.selectionStart ?? draft.length;
    const { value: v, cursor: c } = menu.replace
      ? replaceToken(draft, menu.replace.tokenStart, menu.replace.cursor, item.value)
      : insertAt(draft, cursor, item.value);
    setDraft(v);
    setMenu(null);
    requestAnimationFrame(() => {
      const el = inputRef.current;
      if (el) { el.focus(); el.setSelectionRange(c, c); }
    });
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!menu) {
      if (e.key === "Enter") { e.preventDefault(); commit(); }
      if (e.key === "Escape") { e.preventDefault(); cancel(); }
      return;
    }
    const flat = menu.groups.flatMap((g) => g.items);
    if (e.key === "ArrowDown") { e.preventDefault(); setMenu((m) => m ? { ...m, activeIdx: Math.min(m.activeIdx + 1, flat.length - 1) } : m); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setMenu((m) => m ? { ...m, activeIdx: Math.max(m.activeIdx - 1, 0) } : m); }
    else if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); selectItem(flat[menu.activeIdx] ?? { value: "", label: "" }); }
    else if (e.key === "Escape") { e.preventDefault(); setMenu(null); } // 只关弹层，草稿保留
  };

  return (
    <>
      <input
        ref={inputRef}
        data-field-kind="formula"
        className={`nodrag ${styles.field} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
        value={draft}
        placeholder={placeholder}
        aria-label={ariaLabel}
        aria-invalid={invalid || undefined}
        aria-expanded={menu ? true : undefined}
        aria-controls={menu ? "node-candidate-results" : undefined}
        role="combobox"
        onFocus={() => { focusedRef.current = true; }}
        onCompositionStart={() => { composingRef.current = true; }}
        onCompositionEnd={(e: CompositionEvent<HTMLInputElement>) => {
          composingRef.current = false;
          onInputChange(e as unknown as React.ChangeEvent<HTMLInputElement>);
        }}
        onChange={onInputChange}
        onBlur={() => {
          const wasDirty = dirtyRef.current;
          commit();
          focusedRef.current = false;
          if (!wasDirty) setDraft(value);
          onBlur?.();
        }}
        onKeyDown={onKeyDown}
      />
      {menu && (
        <NodeCandidateMenu
          x={menu.x}
          y={menu.y}
          groups={menu.groups}
          activeIdx={menu.activeIdx}
          onSelect={selectItem}
          onMouseEnter={(idx) => setMenu((m) => (m ? { ...m, activeIdx: idx } : m))}
          onClose={() => setMenu(null)}
        />
      )}
    </>
  );
}
```

```css
/* NodeFormulaField.module.css —— 无额外样式，复用 NodeField.module.css 的 .field */
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`
Expected: PASS（11 用例全绿）。若 "输入单字符运算符" 或 "IME" 用例有 jsdom selectionStart 差异，用 `Object.defineProperty` 或直接传 selectionStart 的 target 对象（现有测试风格：`fireEvent.change(input, { target: { value, selectionStart } })`——jsdom input 的 selectionStart 由 value 派生，fireEvent.change 的 target 合并生效；若断言失败，改用 `input.setSelectionRange(3, 3)` 后触发 change 的写法）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx poiesis/src/components/canvas/code/fields/NodeFormulaField.module.css poiesis/src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx
git commit -m "feat(canvas): NodeFormulaField 公式框——草稿提交语义 + 字符触发自动补全（operand/member/operator 上下文）"
```

---

### Task 4: 候选数据组装 + letVars 按 fn 拓扑归属

**Files:**
- Create: `poiesis/src/services/codegen/fn-topology.ts`
- Modify: `poiesis/src/services/codegen/flow-scope.ts`（追加候选组装函数，不动既有导出）
- Test: `poiesis/src/services/codegen/__tests__/fn-topology.test.ts`、`poiesis/src/services/codegen/__tests__/flow-scope.test.ts`（追加 describe）

**Interfaces:**
- Consumes: `CodeblockElementData`/`WireData`（core/types）、`FlowScope`（flow-scope.ts 既有）
- Produces:
  ```ts
  // fn-topology.ts
  export function collectLetVarsPerFn(elements: CodeblockElementData[], wires: WireData[]): Map<string, Array<{ name: string }>>;
  /** 从 elId 沿 flow-in wires 反推所属 fn 元素 id（无 fn 可达 → null） */
  export function findOwnerFnId(elId: string, elements: CodeblockElementData[], wires: WireData[]): string | null;
  // flow-scope.ts 追加（既有导出不动）
  export function buildExpressionCandidates(scope: FlowScope, letVars: Array<{ name: string }>): FormulaGroups;  // 见下方类型
  export function buildTargetOptions(scope: FlowScope, letVars: Array<{ name: string }>): Array<{ value: string; label: string }>;
  ```
  其中 `FormulaGroups = Array<{ label: string; items: Array<{ value: string; label: string }> }>`；`buildTargetOptions` 输出变量类（letVars + fnParams + signals + props + computeds）+ store 信号（`Store.信号`）。
  > 结构兼容说明：`FormulaCandidate`（flow-scope，服务层）与 `CandidateItem`（NodeCandidateMenu，UI 层）形状相同（`{ value, label }`），TS 结构类型直接兼容，服务层不依赖 UI 组件类型。

- [ ] **Step 1: 写失败测试**

```ts
// fn-topology.test.ts
import { describe, it, expect } from "vitest";
import { collectLetVarsPerFn, findOwnerFnId } from "../fn-topology";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

const el = (id: string, tag: string, name?: string): CodeblockElementData => ({
  id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {},
});
const wire = (id: string, fromId: string, toId: string): WireData => ({ id, fromId, fromPort: "flow-out", toId, toPort: "flow-in" });

describe("collectLetVarsPerFn", () => {
  it("单 fn：收集其 let 节点，忽略 assign/其它", () => {
    const elements = [el("fn1", "fn"), el("let1", "let", "x"), el("let2", "let", "y"), el("a1", "assign")];
    const wires = [wire("w1", "fn1", "let1"), wire("w2", "let1", "a1"), wire("w3", "a1", "let2")];
    const map = collectLetVarsPerFn(elements, wires);
    expect(map.get("fn1")?.map((v) => v.name)).toEqual(["x", "y"]);
  });
  it("多 fn 隔离：BFS 不跨越 fn 边界", () => {
    const elements = [el("fn1", "fn"), el("let1", "let", "x"), el("fn2", "fn"), el("let2", "let", "z")];
    const wires = [wire("w1", "fn1", "let1"), wire("w2", "fn2", "let2")];
    const map = collectLetVarsPerFn(elements, wires);
    expect(map.get("fn1")?.map((v) => v.name)).toEqual(["x"]);
    expect(map.get("fn2")?.map((v) => v.name)).toEqual(["z"]);
  });
  it("经过 condition 分支口（fromPort=br0）仍可达", () => {
    const elements = [el("fn1", "fn"), el("c1", "condition"), el("let1", "let", "x")];
    const wires = [wire("w1", "fn1", "c1"), { id: "w2", fromId: "c1", fromPort: "br0", toId: "let1", toPort: "flow-in" }];
    expect(collectLetVarsPerFn(elements, wires).get("fn1")?.map((v) => v.name)).toEqual(["x"]);
  });
  it("孤岛节点（无 fn 可达）不归属任何 fn", () => {
    const elements = [el("fn1", "fn"), el("let1", "let", "x"), el("orphan", "let", "o")];
    const wires = [wire("w1", "fn1", "let1")];
    const map = collectLetVarsPerFn(elements, wires);
    expect(map.get("fn1")?.map((v) => v.name)).toEqual(["x"]);
    expect(map.get("orphan")).toBeUndefined();
  });
});

describe("findOwnerFnId", () => {
  it("沿 flow-in 链反推所属 fn", () => {
    const elements = [el("fn1", "fn"), el("c1", "condition"), el("let1", "let", "x")];
    const wires = [wire("w1", "fn1", "c1"), wire("w2", "c1", "let1")];
    expect(findOwnerFnId("let1", elements, wires)).toBe("fn1");
  });
  it("fn 自身 → 自身", () => {
    const elements = [el("fn1", "fn")];
    expect(findOwnerFnId("fn1", elements, [])).toBe("fn1");
  });
  it("无 fn 可达 → null", () => {
    const elements = [el("c1", "condition"), el("let1", "let", "x")];
    const wires = [wire("w1", "c1", "let1")];
    expect(findOwnerFnId("let1", elements, wires)).toBeNull();
  });
});
```

```ts
// flow-scope.test.ts 追加（文件末尾新 describe）
import { buildExpressionCandidates, buildTargetOptions } from "../flow-scope";
import type { FlowScope } from "../flow-scope";

const scope: FlowScope = {
  signals: [{ name: "count", type: "i32" }],
  props: [{ name: "title", type: "String" }],
  computeds: [{ name: "double", returnType: "i32" }],
  events: [],
  fns: [{ name: "close", params: [] }],
  stores: [{ name: "ThemeStore", actions: [{ name: "toggle", params: [] }], signals: [{ name: "mode", type: "String" }] }],
  fnParams: [{ name: "step", type: "i32" }],
  letVars: [{ name: "total" }],
};

describe("buildExpressionCandidates", () => {
  it("分组完整：变量/参数/信号/属性/计算属性/store/函数", () => {
    const groups = buildExpressionCandidates(scope, [{ name: "total" }]);
    const all = groups.flatMap((g) => g.items).map((i) => i.value);
    expect(all).toContain("total");          // let 变量
    expect(all).toContain("step");           // fn 参数
    expect(all).toContain("count");          // 信号
    expect(all).toContain("title");          // 属性
    expect(all).toContain("double");         // 计算属性
    expect(all).toContain("ThemeStore.mode"); // store 信号
    expect(all).toContain("close");          // component fn（F2 修复后非空）
    expect(all).toContain("ThemeStore.toggle"); // store action
  });
  it("候选带类型/签名 label", () => {
    const groups = buildExpressionCandidates(scope, [{ name: "total" }]);
    const fnItem = groups.flatMap((g) => g.items).find((i) => i.value === "ThemeStore.toggle");
    expect(fnItem?.label).toContain("toggle()");
  });
});

describe("buildTargetOptions", () => {
  it("变量类 + store 信号", () => {
    const opts = buildTargetOptions(scope, [{ name: "total" }]);
    const values = opts.map((o) => o.value);
    expect(values).toContain("total");
    expect(values).toContain("count");
    expect(values).toContain("ThemeStore.mode");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/fn-topology.test.ts src/services/codegen/__tests__/flow-scope.test.ts`
Expected: FAIL — module not found / 新 describe 未定义

- [ ] **Step 3: 最小实现**

```ts
// poiesis/src/services/codegen/fn-topology.ts
// 按 wire 拓扑推断 let 变量归属（多 fn 隔离）：从每个 fn 元素沿 flow-out 正向 BFS，
// 收集 tag==="let" 节点；findOwnerFnId 沿 flow-in 反向反推所属 fn。
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";

/** 每个 fn 元素可达的 let 节点（按拓扑顺序）。孤岛节点不入任何 fn 的集合。 */
export function collectLetVarsPerFn(elements: CodeblockElementData[], wires: WireData[]): Map<string, Array<{ name: string }>> {
  const out = new Map<string, Array<{ name: string }>>();
  const outgoing = new Map<string, WireData[]>();
  for (const w of wires) {
    const list = outgoing.get(w.fromId) ?? [];
    list.push(w);
    outgoing.set(w.fromId, list);
  }
  const fnEls = elements.filter((e) => e.tag === "fn");
  for (const fnEl of fnEls) {
    const visited = new Set<string>([fnEl.id]);
    const queue = [fnEl.id];
    const lets: Array<{ name: string }> = [];
    while (queue.length > 0) {
      const cur = queue.shift()!;
      for (const w of outgoing.get(cur) ?? []) {
        if (visited.has(w.toId)) continue;
        visited.add(w.toId);
        const target = elements.find((e) => e.id === w.toId);
        if (target?.tag === "let") {
          const name = (target.props?.name as string) ?? "";
          if (name) lets.push({ name });
        }
        queue.push(w.toId);
      }
    }
    out.set(fnEl.id, lets);
  }
  return out;
}

/** 从 elId 沿 flow-in 反向 BFS 反推所属 fn 元素 id；fn 自身返回自身；无 fn 可达返回 null。 */
export function findOwnerFnId(elId: string, elements: CodeblockElementData[], wires: WireData[]): string | null {
  const incoming = new Map<string, WireData[]>();
  for (const w of wires) {
    const list = incoming.get(w.toId) ?? [];
    list.push(w);
    incoming.set(w.toId, list);
  }
  const visited = new Set<string>([elId]);
  const queue = [elId];
  while (queue.length > 0) {
    const cur = queue.shift()!;
    if (cur !== elId && elements.find((e) => e.id === cur)?.tag === "fn") return cur;
    for (const w of incoming.get(cur) ?? []) {
      if (visited.has(w.fromId)) continue;
      visited.add(w.fromId);
      queue.push(w.fromId);
    }
  }
  return null;
}
```

```ts
// flow-scope.ts 末尾追加（不动既有导出）
export interface FormulaCandidate { value: string; label: string }
export interface FormulaGroup { label: string; items: FormulaCandidate[] }

/** 公式框候选分组：变量/参数/信号/属性/计算属性/store 信号/store action/component fn。 */
export function buildExpressionCandidates(scope: FlowScope, letVars: Array<{ name: string }>): FormulaGroup[] {
  const groups: FormulaGroup[] = [];
  const vars = [
    ...letVars.map((v) => v.name),
    ...scope.fnParams.map((p) => p.name),
  ].filter(Boolean);
  if (vars.length > 0) groups.push({ label: "变量", items: vars.map((n) => ({ value: n, label: n })) });
  if (scope.signals.length > 0) groups.push({ label: "信号", items: scope.signals.map((s) => ({ value: s.name, label: `${s.name}: ${s.type}` })) });
  if (scope.props.length > 0) groups.push({ label: "属性", items: scope.props.map((p) => ({ value: p.name, label: `${p.name}: ${p.type}` })) });
  if (scope.computeds.length > 0) groups.push({ label: "计算属性", items: scope.computeds.map((c) => ({ value: c.name, label: `${c.name}: ${c.returnType}` })) });
  const storeSignals = scope.stores.flatMap((s) => s.signals.map((sig) => ({ value: `${s.name}.${sig.name}`, label: `${s.name}.${sig.name}: ${sig.type}` })));
  if (storeSignals.length > 0) groups.push({ label: "store 信号", items: storeSignals });
  const calls = [
    ...scope.fns.map((f) => ({ value: f.name, label: `${f.name}(${f.params.map((p) => p.name).join(", ")})` })),
    ...scope.stores.flatMap((s) => s.actions.map((a) => ({ value: `${s.name}.${a.name}`, label: `${s.name}.${a.name}(${a.params.map((p) => `${p.name}: ${p.type}`).join(", ")})` }))),
  ];
  if (calls.length > 0) groups.push({ label: "函数", items: calls });
  return groups;
}

/** assign target 下拉候选：变量类 + store 信号（Store.信号 格式）。 */
export function buildTargetOptions(scope: FlowScope, letVars: Array<{ name: string }>): FormulaCandidate[] {
  const names = [
    ...letVars.map((v) => v.name),
    ...scope.fnParams.map((p) => p.name),
    ...scope.signals.map((s) => s.name),
    ...scope.props.map((p) => p.name),
    ...scope.computeds.map((c) => c.name),
  ].filter(Boolean);
  const storeSignals = scope.stores.flatMap((s) => s.signals.map((sig) => `${s.name}.${sig.name}`));
  return [...new Set([...names, ...storeSignals])].map((n) => ({ value: n, label: n }));
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/fn-topology.test.ts src/services/codegen/__tests__/flow-scope.test.ts`
Expected: PASS（fn-topology 7 用例 + flow-scope 追加 3 用例全绿；既有 flow-scope 断言不动）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/codegen/fn-topology.ts poiesis/src/services/codegen/flow-scope.ts poiesis/src/services/codegen/__tests__/fn-topology.test.ts poiesis/src/services/codegen/__tests__/flow-scope.test.ts
git commit -m "feat(canvas): 候选数据组装（buildExpressionCandidates/buildTargetOptions）+ letVars 按 fn 拓扑归属（多 fn 隔离）"
```

---

### Task 5: F2 解析侧——parser 登记 fnRegistry

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`（parseFn 552-586、buildResult 1477-1487、类字段区 53 附近）
- Modify: `poiesis/src/services/codegen/types.ts`（ParseResult 追加字段）
- Test: `poiesis/src/services/codegen/__tests__/fn-registry.test.ts`（新建）

**Interfaces:**
- Consumes: `FnDef`（`core/types/project-json.ts:123`：`{ pid, name, params: Array<{name,type}>, returnType }`）
- Produces:
  ```ts
  // ParseResult 追加（types.ts:303 之后）
  /** 组件 → fn 元数据（F2：componentDefs.fns 物化来源）。fn 名含组件上下文。 */
  fnRegistry?: Record<string, Array<{ pid: string; name: string; params: Array<{ name: string; type: string }>; returnType: string }>>;
  ```

- [ ] **Step 1: 写失败测试**

```ts
// fn-registry.test.ts
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parseFn fnRegistry（F2）", () => {
  it("组件 fn → fnRegistry[组件名] 含 name/params/returnType", () => {
    const src = `
// @pid:fn_a1
component Counter {
    signal count: i32 = 0;
    // @pid:fn_a1
    fn increment(step: i32) -> i32 {
        // @pid:cb_b2
        count = count + step;
        return count;
    }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    const fns = result.fnRegistry?.["Counter"];
    expect(fns).toHaveLength(1);
    expect(fns?.[0]).toMatchObject({ name: "increment", params: [{ name: "step", type: "i32" }], returnType: "i32" });
  });
  it("无 fn 的组件不出现在 fnRegistry", () => {
    const src = `
component Empty {
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    expect(result.fnRegistry).toBeUndefined();
  });
  it("多组件各自登记", () => {
    const src = `
component A {
    fn fa() { }
    view { div { } }
}
component B {
    fn fb() { }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    expect(result.fnRegistry?.["A"]?.map((f) => f.name)).toEqual(["fa"]);
    expect(result.fnRegistry?.["B"]?.map((f) => f.name)).toEqual(["fb"]);
  });
});
```

> `parseOmniSource` 从 `omni-parser.ts` 导出（既有测试调用方式，参考 `component-members-roundtrip.test.ts:6`）。

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/fn-registry.test.ts`
Expected: FAIL — `result.fnRegistry` undefined

- [ ] **Step 3: 最小实现**

```ts
// parser.ts
// 类字段（53 行 parsedFnBodies 附近追加）：
private fnRegistry = new Map<string, Array<{ pid: string; name: string; params: Array<{ name: string; type: string }>; returnType: string }>>();

// parseFn（585 行 parsedFnBodies.set 之后追加）：
// F2：登记 fn 元数据（组件维度）——componentDefs.fns 物化来源。
// @pid 注释在成员分发前已存入 ctx.pendingPid（parser.ts:236-239）。
const fnPid = this.ctx.pendingPid || "";
this.ctx.pendingPid = null;
const rawParams = sigMatch?.[2] || "";
const params = rawParams.split(",").map((p) => {
  const [pName, pType] = p.trim().split(":").map((s) => s.trim());
  return { name: pName || "", type: pType || "String" };
});
const returnType = fnSignature.match(/->\s*(\w+)/)?.[1] || "void";
const comp = this.ctx.currentComponent;
if (comp) {
  const list = this.fnRegistry.get(comp) ?? [];
  list.push({ pid: fnPid, name: fnName, params, returnType });
  this.fnRegistry.set(comp, list);
}

// buildResult（1484 行 codeFlowGraphs 之后追加）：
if (this.fnRegistry.size > 0) {
  result.fnRegistry = Object.fromEntries(this.fnRegistry);
}
```

```ts
// types.ts（303 行 codeFlowGraphs 之后追加）
/** 组件 → fn 元数据（F2：componentDefs.fns 物化来源）。 */
fnRegistry?: Record<string, Array<{ pid: string; name: string; params: Array<{ name: string; type: string }>; returnType: string }>>;
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/fn-registry.test.ts`
Expected: PASS（3 用例）。随后跑全量确认无回归：`npx vitest run`（基线 319 应全绿——fnRegistry 是 Additive 字段）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/__tests__/fn-registry.test.ts
git commit -m "feat(canvas): F2 解析侧——parseFn 登记 fnRegistry（组件维度，Additive 字段）"
```

---

### Task 6: F1 解析侧——parseFn 合成 fn 元素（镜像 store action 模式）

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`（parseFn 552-586）
- Test: `poiesis/src/services/codegen/__tests__/fn-synthesis.test.ts`（新建）

**Interfaces:**
- Consumes: Task 5 的 fnPid 提取（`this.ctx.pendingPid`）
- Produces: `ParseResult.codeFlowGraphs[fnName].elements` 恒含 fn 元素（id=@pid 或 `fn_${name}_${uid}`），wires 首元素 wire 到 fn；**行为变化**：既有断言 codeFlowGraphs 形状的测试需同步更新（graph-parser-syntax/roundtrip 相关）

- [ ] **Step 1: 写失败测试**

```ts
// fn-synthesis.test.ts
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parseFn 合成 fn 元素（F1 解析侧）", () => {
  it("codeFlowGraphs 含 fn 元素（id=@pid）并 wire 到首语句", () => {
    const src = `
// @pid:fn_a1
component Counter {
    signal count: i32 = 0;
    // @pid:fn_a1
    fn increment(step: i32) {
        // @pid:cb_b2
        count = count + step;
    }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    const graph = result.codeFlowGraphs?.["increment"];
    const fnEl = graph?.elements.find((e) => e.tag === "fn");
    expect(fnEl?.id).toBe("fn_a1");
    expect(fnEl?.props?.name).toBe("increment");
    expect(graph?.wires.some((w) => w.fromId === "fn_a1" && w.toId === "cb_b2")).toBe(true);
  });
  it("空 body 的 fn：仅 fn 元素，无 wire", () => {
    const src = `
component A {
    fn empty() { }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    const graph = result.codeFlowGraphs?.["empty"];
    expect(graph?.elements).toHaveLength(1);
    expect(graph?.elements[0]?.tag).toBe("fn");
    expect(graph?.wires).toHaveLength(0);
  });
  it("重复解析幂等：二次 parse 不叠加 fn 元素", () => {
    const src = `
// @pid:fn_a1
component Counter {
    // @pid:fn_a1
    fn increment() {
        // @pid:cb_b2
        count = 1;
    }
    view { div { } }
}
`;
    const r1 = parseOmniSource(src);
    const graph = r1.codeFlowGraphs?.["increment"];
    expect(graph?.elements.filter((e) => e.tag === "fn")).toHaveLength(1);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/fn-synthesis.test.ts`
Expected: FAIL — `graph.elements` 无 fn 元素（目前 parseFn 只解析 body）

- [ ] **Step 3: 最小实现**

```ts
// parser.ts parseFn（583-585 行，parseCodeFlowGraph 调用之后）：
// 镜像 store action 的 fn 元素合成（parser.ts:697-720 模式）：
// 组件 fn 的 @pid 注释在成员分发前已入 ctx.pendingPid；body 解析不含 fn 头，
// 故从签名合成 fn 元素 + 首语句 wire，保证保存时 pipeline 能找到 fn 入口编译。
const { elements, wires } = parseCodeFlowGraph(fnBody);
if (!elements.some((e) => e.tag === "fn")) {
  const fnPid = this.ctx.pendingPid || `fn_${fnName}_${uid()}`;
  this.ctx.pendingPid = null;
  const rawParams = sigMatch?.[2] || "";
  const params = rawParams.split(",").map((p) => {
    const [pName, pType] = p.trim().split(":").map((s) => s.trim());
    return { name: pName || "", type: pType || "String" };
  });
  const returnType = fnSignature.match(/->\s*(\w+)/)?.[1] || "void";
  elements.unshift({
    id: fnPid,
    nodeType: "codeblock",
    tag: "fn",
    parentId: null,
    props: { name: fnName, params: rawParams, returnType },
    source: "",
  });
  if (elements.length > 1) {
    wires.push({ id: `w_${fnPid}_0`, fromId: fnPid, fromPort: "flow-out", toId: elements[1]!.id, toPort: "flow-in" });
  }
}
this.parsedFnBodies.set(fnName, { elements, wires });
```

> `uid` 已由 parser.ts 顶部导入（既有 `uid("sgn")` 等用法）；若未导入，补 `import { uid } from "../../core/utils/id";`。
> **注意**：Task 5 的 fnPid/params 提取与本 Task 重合——实现时把 Task 5 的登记逻辑收拢到合成之后（fnPid 由合成产物决定，registry 的 pid 与元素 id 一致），Task 5 测试继续通过。

- [ ] **Step 4: 运行确认通过 + 回归修正**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/fn-synthesis.test.ts`
Expected: PASS（3 用例）
随后跑全量：`npx vitest run`——若 graph-parser/roundtrip 既有断言因 codeFlowGraphs 多出 fn 元素 + wire 而失败，**更新既有断言**（新增 fn 元素是预期行为，不是回归）：在断言 graph.elements 处补 `filter((e) => e.tag !== "fn")` 或按新形状断言。逐处确认是"形状变化"而非"语义变化"。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/__tests__/fn-synthesis.test.ts
git commit -m "feat(canvas): F1 解析侧——parseFn 合成 fn 元素（@pid 稳定 id + 首语句 wire，镜像 store action 模式）"
```

---

### Task 7: F1/F2 物化——App.tsx 加载（codeFlowGraphs + componentDefs.fns）

**Files:**
- Modify: `poiesis/src/App.tsx`（loadOmniFiles 252-316 循环：284/295 行 `fns: existing?.fns ?? []` 两处、stateUpdate 类型与赋值）
- Modify: `poiesis/src/services/codegen/graph-parser.ts`（末尾追加纯函数 `mergeCodeFlowGraphs`，不动既有导出）
- Test: `poiesis/src/services/codegen/__tests__/merge-codeflow-graphs.test.ts`（新建）

**Interfaces:**
- Consumes: Task 5 的 `fnRegistry`（ParseResult 字段）
- Produces:
  ```ts
  // graph-parser.ts 末尾追加
  /** 合并多文件 codeFlowGraphs（fn 名 → 图），同名后者覆盖（跨文件同名 fn 为既有限制）。 */
  export function mergeCodeFlowGraphs(graphs: Array<Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> | undefined>): Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>;
  ```

- [ ] **Step 1: 写失败测试**

```ts
// merge-codeflow-graphs.test.ts
import { describe, it, expect } from "vitest";
import { mergeCodeFlowGraphs } from "../graph-parser";

describe("mergeCodeFlowGraphs", () => {
  it("多文件合并，fn 名去重", () => {
    const a = { increment: { elements: [{ id: "fn1", nodeType: "codeblock", tag: "fn", parentId: null, source: "", props: {} } as never], wires: [] as never[] } };
    const b = { close: { elements: [], wires: [] } };
    const merged = mergeCodeFlowGraphs([a, b]);
    expect(Object.keys(merged)).toEqual(["increment", "close"]);
  });
  it("undefined 条目跳过", () => {
    const merged = mergeCodeFlowGraphs([undefined, { fa: { elements: [], wires: [] } }]);
    expect(Object.keys(merged)).toEqual(["fa"]);
  });
  it("同名 fn 后者覆盖", () => {
    const a = { fa: { elements: [{ id: "x", nodeType: "codeblock", tag: "let", parentId: null, source: "", props: { name: "x" } } as never], wires: [] as never[] } };
    const b = { fa: { elements: [], wires: [] } };
    expect(mergeCodeFlowGraphs([a, b]).fa.elements).toEqual([]);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/merge-codeflow-graphs.test.ts`
Expected: FAIL — `mergeCodeFlowGraphs` not defined

- [ ] **Step 3: 最小实现**

```ts
// graph-parser.ts 末尾追加
/** 合并多文件 codeFlowGraphs（fn 名 → 图），同名后者覆盖（跨文件同名 fn 为既有限制）。 */
export function mergeCodeFlowGraphs(
  graphs: Array<Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> | undefined>,
): Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> {
  const out: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> = {};
  for (const g of graphs) {
    if (!g) continue;
    for (const [key, value] of Object.entries(g)) out[key] = value;
  }
  return out;
}
```

```ts
// App.tsx loadOmniFiles
// 0) 文件顶部 import 区追加：
import { mergeCodeFlowGraphs } from "./services/codegen/graph-parser";

// 1) stateUpdate 类型（241-250 行 Partial 对象）追加：
codeFlowGraphs: Record<string, { elements: import("./core/types/element").CodeblockElementData[]; wires: import("./core/types/wire").WireData[] }>;

// 2) mainParsed 定义之后收集全部文件的图合并结果：
const mergedGraphs = mergeCodeFlowGraphs(parsed.map((p) => p.parseResult.codeFlowGraphs));

// 3) 两处 `fns: existing?.fns ?? [],`（284 与 295 行）替换为：
fns: existing?.fns ?? (p.parseResult.fnRegistry?.[p.componentName] ?? []).map((f) => ({ pid: f.pid, name: f.name, params: f.params, returnType: f.returnType })),

// 4) 最终 setState（316 行附近）追加：
stateUpdate.codeFlowGraphs = mergedGraphs;
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/merge-codeflow-graphs.test.ts && npx tsc --noEmit`
Expected: merge 测试 PASS + tsc 零错误

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/App.tsx poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/__tests__/merge-codeflow-graphs.test.ts
git commit -m "feat(canvas): F1/F2 物化——App 加载写入 store.codeFlowGraphs + componentDefs.fns 物化（fnRegistry 消费）"
```

---

### Task 8: F1 物化触发——组件 tab 进入 codeflow 合并物化

**Files:**
- Modify: `poiesis/src/store/slices/element-crud.ts`（追加 action `materializeFnGraphs`）
- Modify: `poiesis/src/store/types.ts`（PoiesisState action 签名）
- Modify: `poiesis/src/pages/DesignPage.tsx`（codeflow 可见时触发物化的 effect）
- Test: `poiesis/src/store/__tests__/materialize-fn-graphs.test.ts`（新建）

**Interfaces:**
- Consumes: Task 6 的 fn 元素（codeFlowGraphs 形状）、Task 7 的 `store.codeFlowGraphs` + `componentDefs[comp].fns`
- Produces:
  ```ts
  // element-crud.ts 追加
  /** 进入组件 codeflow 时：把该组件的 fn 图合并进 elements/wires（pid 去重，已存在不覆盖，无 undo）。 */
  materializeFnGraphs: (componentName: string) => void;
  ```

- [ ] **Step 1: 写失败测试**

```ts
// materialize-fn-graphs.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeblockElementData } from "../../core/types/element";

const codeEl = (id: string, tag: string, name?: string): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {} });

const reset = () => usePoiesisStore.setState({
  elements: [], wires: [], codeFlowGraphs: {}, componentDefs: {},
});

describe("materializeFnGraphs（F1 物化）", () => {
  beforeEach(reset);

  it("合并该组件 fn 图：view 元素保留 + 新 codeblock 加入", () => {
    usePoiesisStore.setState({
      elements: [{ id: "view1", nodeType: "view", tag: "div", parentId: null, source: "" } as never],
      wires: [],
      codeFlowGraphs: {
        increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x")], wires: [{ id: "w1", fromId: "fn_a1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }] },
      },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const els = usePoiesisStore.getState().elements;
    expect(els.map((e) => e.id)).toEqual(["view1", "fn_a1", "cb_b2"]);
    expect(usePoiesisStore.getState().wires).toHaveLength(1);
  });

  it("已存在的 pid 不覆盖（保留画布编辑）", () => {
    usePoiesisStore.setState({
      elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "EDITED")],
      wires: [],
      codeFlowGraphs: { increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "original")], wires: [] } },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const el = usePoiesisStore.getState().elements.find((e) => e.id === "cb_b2");
    expect((el?.props?.name as string)).toBe("EDITED");
  });

  it("无 fn 元数据的组件：no-op", () => {
    usePoiesisStore.setState({ componentDefs: { Empty: { name: "Empty", fns: [] } as never } });
    expect(() => usePoiesisStore.getState().materializeFnGraphs("Empty")).not.toThrow();
    expect(usePoiesisStore.getState().elements).toEqual([]);
  });

  it("codeFlowGraphs 无对应 fn：no-op", () => {
    usePoiesisStore.setState({
      codeFlowGraphs: { other: { elements: [codeEl("z1", "let", "z")], wires: [] } },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    expect(usePoiesisStore.getState().elements).toEqual([]);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/store/__tests__/materialize-fn-graphs.test.ts`
Expected: FAIL — `materializeFnGraphs is not a function`

- [ ] **Step 3: 最小实现**

```ts
// element-crud.ts 追加（遵循既有 action 风格，含 set/get 签名——参考文件顶部既有 action 写法）
materializeFnGraphs: (componentName: string) => {
  const s = get();
  const fns = s.componentDefs[componentName]?.fns ?? [];
  if (fns.length === 0) return;
  const graphs = fns
    .map((f) => s.codeFlowGraphs[f.name])
    .filter((g): g is { elements: CodeblockElementData[]; wires: WireData[] } => Boolean(g));
  if (graphs.length === 0) return;
  const existIds = new Set(s.elements.map((e) => e.id));
  const newElements = graphs.flatMap((g) => g.elements).filter((e) => !existIds.has(e.id));
  const existWireIds = new Set(s.wires.map((w) => w.id));
  const newWires = graphs.flatMap((g) => g.wires).filter((w) => !existWireIds.has(w.id));
  // 加载时物化：直接 set（无 undo 条目，同 setElementPosition 哲学）
  set({ elements: [...s.elements, ...newElements], wires: [...s.wires, ...newWires] });
},
```

```ts
// DesignPage.tsx —— 组件 tab 进入 codeflow 时物化当前组件的 fn 图。
// 物化是幂等的（pid 去重、已存在不覆盖），可安全地在每次 codeflow 可见时触发。
const materializeFnGraphs = usePoiesisStore((s) => s.materializeFnGraphs);
const activeTabComponentName = activeTab?.componentName;
useEffect(() => {
  if (codeFlowVisible && activeTabType === "component" && activeTabComponentName) {
    materializeFnGraphs(activeTabComponentName);
  }
  // eslint-disable-next-line react-hooks/exhaustive-deps -- 物化幂等，依赖仅触发时机
}, [codeFlowVisible, activeTabType, activeTabComponentName]);
```

> 实现前先确认 DesignPage 顶部已有 `usePoiesisStore`/`useEffect`/`activeTab`/`activeTabType`/`codeFlowVisible` 的获取方式（DesignPage.tsx 296-337 附近存在 `activeTab`/`codeFlowVisible`/`toggleCodeFlow` 的用法，按其既有模式接线）。

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/store/__tests__/materialize-fn-graphs.test.ts && npx tsc --noEmit`
Expected: 4 用例 PASS + tsc 零错误

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/store/slices/element-crud.ts poiesis/src/store/types.ts poiesis/src/pages/DesignPage.tsx poiesis/src/store/__tests__/materialize-fn-graphs.test.ts
git commit -m "feat(canvas): F1 物化触发——materializeFnGraphs 合并进画布（pid 去重保留编辑）+ DesignPage 进入 codeflow 触发"
```

---

### Task 9: SyntaxNode 集成（value 公式框 + target 下拉 + FlowCanvas letVars 注入）

**Files:**
- Modify: `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx`
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（nodes data 注入 letVars）
- Test: `poiesis/src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx`（追加 describe）、`poiesis/src/components/canvas/code/__tests__/FlowCanvasDrag.test.tsx`（追加用例）

**Interfaces:**
- Consumes: Task 3 `NodeFormulaField`/`FormulaCandidateSource`、Task 4 `buildExpressionCandidates`/`buildTargetOptions`/`collectLetVarsPerFn`/`findOwnerFnId`、Task 2 `NodeSelect`（NodeField 既有）
- Produces: SyntaxNode data 追加 `letVars?: Array<{ name: string }>`（FlowCanvas 注入）

- [ ] **Step 1: 写失败测试**

```tsx
// SyntaxNode.test.tsx 追加 describe（沿用既有 render 方式；mock flow-scope 依赖）
import { NodeFormulaField } from "../../fields/NodeFormulaField";
// ……（在既有 import 处追加）

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
    expect(screen.getByText("total")).toBeTruthy();
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
```

```tsx
// FlowCanvasLetVars.test.tsx（新建——独立 mock，不动 FlowCanvasDrag 既有 mock 结构）
// FlowCanvas 向语法节点 data 注入所属 fn 的 letVars（wire 拓扑推断）。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { FlowCanvas } from "../FlowCanvas";

const { mockRfProps } = vi.hoisted(() => ({ mockRfProps: { current: null as null | Record<string, unknown> } }));

vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    mockRfProps.current = props;
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
  Background: () => null,
  BackgroundVariant: { Dots: "dots", Lines: "lines", Cross: "cross" },
  Controls: () => null,
  Handle: () => null,
  Position: { Top: "top", Bottom: "bottom", Right: "right", Left: "left" },
}));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) =>
    selector({
      elements: [
        { id: "fn1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "increment", params: "" }, source: "" },
        { id: "let1", nodeType: "codeblock", tag: "let", parentId: null, props: { name: "x", mutable: false, value: "0" }, source: "", position: { x: 0, y: 0 } },
        { id: "a1", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "count", operator: "=", value: "" }, source: "", position: { x: 0, y: 120 } },
      ],
      wires: [
        { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "let1", toPort: "flow-in" },
        { id: "w2", fromId: "let1", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
      ],
      addElement: () => {}, removeElement: () => {}, setElementPosition: () => {},
      setProp: () => {}, setElementSource: () => {}, setNodeMeta: () => {}, addWire: () => {},
      componentDefs: {}, storeReferences: {}, storeDefs: {}, inlineStoreDefs: {},
      designPage: { codeFlowElementId: "main", previewOutput: null, activeTabId: "t1", openTabs: [{ id: "t1", componentName: "App", filePath: "App.omni" }] },
    }),
}));

beforeEach(() => { mockRfProps.current = null; });

describe("FlowCanvas letVars 注入（wire 拓扑）", () => {
  it("assign 节点 data.letVars 含所属 fn 的 let 变量", () => {
    render(<FlowCanvas />);
    const nodes = mockRfProps.current?.nodes as Array<{ id: string; data: Record<string, unknown> }>;
    const assignNode = nodes.find((n) => n.id === "a1");
    expect(assignNode?.data.letVars).toEqual([{ name: "x" }]);
  });
  it("fn 节点自身携带其 fn 的 letVars（不渲染公式框，无害）", () => {
    render(<FlowCanvas />);
    const nodes = mockRfProps.current?.nodes as Array<{ id: string; data: Record<string, unknown> }>;
    const fnNode = nodes.find((n) => n.id === "fn1");
    expect(fnNode?.data.letVars).toEqual([{ name: "x" }]);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx src/components/canvas/code/__tests__/FlowCanvasLetVars.test.tsx`
Expected: FAIL — 无公式框/无 letVars（当前渲染纯文本输入）

- [ ] **Step 3: 最小实现**

```tsx
// SyntaxNode.tsx
// 1) import 追加：
import { NodeFormulaField, type FormulaCandidateSource } from "../fields/NodeFormulaField";
import { buildExpressionCandidates, buildTargetOptions, type FlowScope } from "../../../../services/codegen/flow-scope";
import { countUnbalancedParens } from "../fields/expression-trigger";

// 2) Props 追加：
letVars?: Array<{ name: string }>;

// 3) 组件内追加（fnCandidates useMemo 之后）：
const formulaSource = useMemo<FormulaCandidateSource | undefined>(() => {
  if (!scope) return undefined;
  const all = buildExpressionCandidates(scope, letVars ?? []);
  const flat = all.flatMap((g) => g.items);
  const flatStore = all.find((g) => g.label === "store 信号")?.items ?? [];
  return {
    filter: (q) => {
      const t = q.trim();
      if (!t) return flat.slice(0, 8);
      return flat.filter((c) => c.value.startsWith(t)).slice(0, 8);
    },
    storeSignals: (storeName) => flatStore.filter((c) => c.value.startsWith(`${storeName}.`)).slice(0, 8),
  };
}, [scope, letVars]);

// 4) assign 渲染（value 字段）替换 NodeTextField 为：
<NodeFormulaField
  value={(props.value as string) ?? ""}
  candidates={formulaSource}
  onCommit={(v) => set("value", v)}
  placeholder="表达式（变量/值/运算）"
  ariaLabel="表达式"
  className={styles.grow}
/>

// 5) assign target 字段替换为下拉：
<NodeSelect
  value={(props.target as string) ?? ""}
  options={buildTargetOptions(scope, letVars ?? [])}
  onCommit={(v) => set("target", v)}
  ariaLabel="目标"
  invalid={errorFields.has("target")}
  allowCustom
  className={styles.grow}
/>

// 6) let value 字段替换 NodeTextField 为 NodeFormulaField（同上，ariaLabel="初始值"，placeholder="初始值（变量/值/运算）"）

// 7) 括号配对轻校验（spec §4.5：只提示不阻塞）。errors 计算（42-46 行）后追加：
const parenIssue = useMemo(() => {
  if (tag !== "assign" && tag !== "let") return null;
  const v = (props.value as string) ?? "";
  if (!v) return null;
  const n = countUnbalancedParens(v);
  if (n > 0) return { field: "value", message: `表达式有 ${n} 个未闭合的括号` };
  if (n < 0) return { field: "value", message: `表达式有 ${-n} 个多余的右括号` };
  return null;
}, [tag, props.value]);
const allErrors = parenIssue ? [...errors, parenIssue] : errors;

// 8) 警告条渲染改用 allErrors（192-196 行 errors 两处引用替换）：
{allErrors.length > 0 && (
  <div className={styles.warnBar}>
    {allErrors.map((e) => <div key={e.field}>{e.message}</div>)}
  </div>
)}
```

```tsx
// FlowCanvas.tsx
// 1) import 追加：
import { collectLetVarsPerFn, findOwnerFnId } from "../../../services/codegen/fn-topology";

// 2) nodes useMemo 前（codeElements 定义后）：
const letVarsByFn = useMemo(() => collectLetVarsPerFn(codeElements, wires), [codeElements, wires]);

// 3) nodes map 内 data 展开处（SyntaxNode 分支）追加：
...(CODE_TAGS.has(el.tag) && el.tag !== "code-block" ? {
  id: el.id,
  scope: flowScope ?? undefined,
  letVars: letVarsByFn.get(findOwnerFnId(el.id, codeElements, wires) ?? "") ?? [],
  onEditProp: (id: string, prop: string, value: unknown) => setProp(id, prop, value),
} : {}),
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx src/components/canvas/code/__tests__/FlowCanvasLetVars.test.tsx`
Expected: 追加用例 PASS（SyntaxNode 5 新用例 + letVars 2 用例）+ 既有用例不回归

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx poiesis/src/components/canvas/code/__tests__/FlowCanvasLetVars.test.tsx
git commit -m "feat(canvas): SyntaxNode 集成公式框（assign/let value + 括号配对提示）+ target 下拉 + FlowCanvas letVars 注入"
```

---

### Task 10: 全量回归 + tsc + E2E 验证

**Files:** 无代码改动（除回归修复）

- [ ] **Step 1: 全量测试**

Run: `cd poiesis && npx vitest run`
Expected: 全绿（319 基线 + 各任务新增用例）。失败处按 Task 6 的回归修正纪律处理（区分"预期形状变化"与"真回归"）

- [ ] **Step 2: 类型检查**

Run: `cd poiesis && npx tsc --noEmit`
Expected: 零错误

- [ ] **Step 3: 真机 E2E（Victauri）**

按顺序验证（`get_plugin_info` 确认连到 Poiesis；必要时 `logs {action:'clear'}` 清 IPC 日志）：

1. **公式框候选**：进入组件 codeflow（双击组件元素）→ 双击 assign 节点 value 字段 → 输入字母 `c` → 断言候选弹层出现（signals/变量分组）
2. **运算符选择框**：value 字段输入 `count <` → 断言弹层含 `<`、`<=` → Tab/Enter 选中 `<` → 文本变为 `count <`
3. **单字符直插**：输入 `count + ` → 断言无弹层、文本直接为 `count +`
4. **store 信号候选**：输入 `ThemeStore.` → 断言弹层含 `ThemeStore.mode`
5. **target 下拉**：assign target 字段 → 断言为下拉（含 `ThemeStore.mode` 等候选）
6. **F1**：保存 → 刷新应用（reload）→ 重新进入组件 codeflow → **断言画布恢复 fn 图（不再为空）**
7. **F2**：call 节点 fnName 下拉 → 断言出现 component fn 候选（如 increment/close）
8. **round-trip**：编辑 value → 保存 → 检查 `.omni` 含正确表达式文本与 `@pid` 注释 → reload → 类型与文本保持
9. **回归抽查**：拖拽节点跟随鼠标（ADR-058 不回归）、空格唤出搜索菜单、拖出创建自动连线

- [ ] **Step 4: 文档更新**

更新 memory（新会话记录 + poiesis-features.md 的 CodeFlowCanvas 行）与 ADR（新增 ADR-059 或扩展 ADR-057，记录公式框 + F1/F2 修复决策与「表达式运算符以 infix.rs 为权威、`=` 不提供」等约束）

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "chore(canvas): 全量回归通过 + E2E 验证（公式框/运算符触发/F1/F2/round-trip）"
```

---

## 任务依赖图

```
Task 1 (expression-trigger) ─┬─→ Task 3 (NodeFormulaField) ─┐
Task 2 (NodeCandidateMenu) ──┘                              ├─→ Task 9 (SyntaxNode 集成)
Task 4 (候选组装 + fn 拓扑) ───────────────────────────────────┘
Task 5 (fnRegistry) ─→ Task 6 (fn 元素合成) ─→ Task 7 (App 物化) ─→ Task 8 (DesignPage 触发物化)
Task 10 (回归 + E2E) ─ 依赖全部
```

Task 1-4 与 Task 5-8 两条链互不依赖，可并行推进；Task 9 依赖 1-4；Task 10 收尾。
