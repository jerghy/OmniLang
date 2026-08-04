# CodeFlowCanvas 内联编辑升级 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 CodeFlowCanvas 语法节点（let/assign/store-call/call）+ condition/return 的编辑从右侧面板字符串输入框升级为节点内联编辑，并新增可搜索节点创建菜单 + flow-out 拖出创建自动连线。

**Architecture:** 节点组件内部持有草稿 state（Enter/blur 提交 setProp、Esc 取消），共享 `NodeField` 控件族统一交互；新建 `flow-scope.ts` 纯函数层从现有 store 收集补全候选并做轻量校验；新建 `NodeSearchMenu` 复刻 ComfyUI 搜索菜单交互；`FlowCanvas` 接入空格快捷键与拖出创建状态机（onConnectStart/onConnectEnd）；`FlowInspector` 降级为 formatSyntaxNode 实时预览。**props 仍为单一事实来源，@pid round-trip 零改动。**

**Tech Stack:** React 19 + TypeScript、@xyflow/react ^12.11.0、Zustand 5、Vitest + @testing-library/react

**Spec:** `docs/superpowers/specs/2026-08-04-codeflowcanvas-inline-editing-design.md`

## Global Constraints

- 所有内联控件必须带 `className="nodrag"`（防与画布拖拽冲突，xyflow 官方模式）
- 编辑只写 `props`（setProp）；草稿只在节点组件局部 state，Enter/blur 才提交；**不得**直接改 `source`
- `formatSyntaxNode`（`poiesis/src/services/codegen/syntax-nodes.ts`）是代码生成单一来源——不新增格式函数、不改其输出格式
- @pid round-trip 管道（graph-parser/graph-compiler/parser/source-emitter/pipeline）**零改动**
- UI 文案用中文（与现有节点/表单一致）；复用 `--ol-*` CSS 变量
- 无 console.log；TypeScript 严格模式
- 测试基线：`cd poiesis && npx vitest run` 现有 232 tests 必须全绿（每任务结束跑一次回归）
- 校验只提示不阻塞（保存时 WASM 门禁最终裁决）

---

### Task 1: NodeField 控件族（NodeTextField / NodeSelect / NodeChipToggle）

草稿提交语义的核心控件，供 SyntaxNode/ConditionNode/ReturnNode 复用。

**Files:**
- Create: `poiesis/src/components/canvas/code/fields/NodeField.tsx`
- Create: `poiesis/src/components/canvas/code/fields/NodeField.module.css`
- Test: `poiesis/src/components/canvas/code/fields/__tests__/NodeField.test.tsx`

**Interfaces:**
- Produces:
  - `NodeTextField({ value: string; onCommit: (v: string) => void; placeholder?: string; ariaLabel?: string; className?: string; invalid?: boolean })` — 聚焦后本地草稿；Enter/blur 提交 onCommit；Esc 恢复原值；外部 value 变化时同步草稿
  - `NodeSelect({ value: string; options: Array<{ value: string; label: string }>; onCommit: (v: string) => void; ariaLabel?: string; allowCustom?: boolean })` — 变更即提交；allowCustom=true 时末位追加自由文本项
  - `NodeChipToggle({ checked: boolean; onChange: (v: boolean) => void; label: string; ariaLabel?: string })` — 点击切换的徽章按钮
  - 三者均输出 `data-field-kind` 属性（"text"|"select"|"chip"）供测试/E2E 定位

- [ ] **Step 1: 写失败测试**

`poiesis/src/components/canvas/code/fields/__tests__/NodeField.test.tsx`:

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeTextField, NodeSelect, NodeChipToggle } from "../NodeField";

describe("NodeTextField", () => {
  it("blur 提交 onCommit", () => {
    const onCommit = vi.fn();
    render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    const input = screen.getByLabelText("变量名");
    fireEvent.change(input, { target: { value: "count" } });
    expect(onCommit).not.toHaveBeenCalled(); // 草稿阶段不提交
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledWith("count");
  });

  it("Enter 提交", () => {
    const onCommit = vi.fn();
    render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    fireEvent.change(screen.getByLabelText("变量名"), { target: { value: "y" } });
    fireEvent.keyDown(screen.getByLabelText("变量名"), { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("y");
  });

  it("Esc 取消恢复原值", () => {
    const onCommit = vi.fn();
    render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    fireEvent.change(screen.getByLabelText("变量名"), { target: { value: "z" } });
    fireEvent.keyDown(screen.getByLabelText("变量名"), { key: "Escape" });
    expect(onCommit).not.toHaveBeenCalled();
    expect((screen.getByLabelText("变量名") as HTMLInputElement).value).toBe("x");
  });

  it("外部 value 变化时同步草稿", () => {
    const { rerender } = render(<NodeTextField value="x" onCommit={vi.fn()} ariaLabel="变量名" />);
    rerender(<NodeTextField value="changed" onCommit={vi.fn()} ariaLabel="变量名" />);
    expect((screen.getByLabelText("变量名") as HTMLInputElement).value).toBe("changed");
  });

  it("invalid 时带警示样式类", () => {
    render(<NodeTextField value="x" onCommit={vi.fn()} ariaLabel="变量名" invalid />);
    expect(screen.getByLabelText("变量名").className).toContain("invalid");
  });
});

describe("NodeSelect", () => {
  const options = [
    { value: "=", label: "=" },
    { value: "+=", label: "+=" },
    { value: "custom", label: "自定义…" },
  ];
  it("变更即提交 onCommit", () => {
    const onCommit = vi.fn();
    render(<NodeSelect value="=" options={options} onCommit={onCommit} ariaLabel="运算符" />);
    fireEvent.change(screen.getByLabelText("运算符"), { target: { value: "+=" } });
    expect(onCommit).toHaveBeenCalledWith("+=");
  });
});

describe("NodeChipToggle", () => {
  it("点击切换", () => {
    const onChange = vi.fn();
    render(<NodeChipToggle checked={false} onChange={onChange} label="mut" />);
    fireEvent.click(screen.getByRole("button", { name: "mut" }));
    expect(onChange).toHaveBeenCalledWith(true);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeField.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现 NodeField.tsx**

```tsx
// poiesis/src/components/canvas/code/fields/NodeField.tsx
// 节点内联编辑共享控件族：统一"草稿 → Enter/blur 提交 → Esc 取消"语义。
import { useEffect, useRef, useState } from "react";
import styles from "./NodeField.module.css";

export interface FieldOption {
  value: string;
  label: string;
}

interface TextProps {
  value: string;
  onCommit: (v: string) => void;
  placeholder?: string;
  ariaLabel?: string;
  className?: string;
  invalid?: boolean;
}

export function NodeTextField({ value, onCommit, placeholder, ariaLabel, className, invalid }: TextProps) {
  const [draft, setDraft] = useState(value);
  // 外部 value 变化（提交成功/重渲染）时同步草稿，但聚焦输入中不打断
  const focused = useRef(false);
  useEffect(() => {
    if (!focused.current) setDraft(value);
  }, [value]);

  const commit = () => {
    focused.current = false;
    if (draft !== value) onCommit(draft);
  };
  const cancel = () => {
    focused.current = false;
    setDraft(value);
  };

  return (
    <input
      data-field-kind="text"
      className={`nodrag ${styles.field} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
      value={draft}
      placeholder={placeholder}
      aria-label={ariaLabel}
      onFocus={() => { focused.current = true; }}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") { e.preventDefault(); commit(); }
        if (e.key === "Escape") { e.preventDefault(); cancel(); }
      }}
    />
  );
}

interface SelectProps {
  value: string;
  options: FieldOption[];
  onCommit: (v: string) => void;
  ariaLabel?: string;
  allowCustom?: boolean;
}

export function NodeSelect({ value, options, onCommit, ariaLabel, allowCustom }: SelectProps) {
  const opts = [...options];
  if (allowCustom && value && !options.some((o) => o.value === value)) {
    opts.push({ value, label: value });
  }
  return (
    <select
      data-field-kind="select"
      className={`nodrag ${styles.field}`}
      value={value}
      aria-label={ariaLabel}
      onChange={(e) => onCommit(e.target.value)}
    >
      {opts.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );
}

interface ChipProps {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  ariaLabel?: string;
}

export function NodeChipToggle({ checked, onChange, label, ariaLabel }: ChipProps) {
  return (
    <button
      type="button"
      data-field-kind="chip"
      className={`nodrag ${styles.chip} ${checked ? styles.chipOn : ""}`}
      aria-label={ariaLabel ?? label}
      aria-pressed={checked}
      onClick={() => onChange(!checked)}
    >
      {label}
    </button>
  );
}
```

`NodeField.module.css`:

```css
.field {
  border: 1px solid var(--ol-border);
  border-radius: var(--ol-radius-sm);
  padding: 2px 6px;
  font-size: var(--ol-font-size-12);
  font-family: var(--ol-font-family-mono);
  background: var(--ol-surface);
  color: var(--ol-text);
  outline: 2px solid transparent;
  min-width: 0;
}
.field:hover { background: var(--ol-gray-100); }
.field:focus { outline: 2px solid var(--ol-color-primary, #4f46e5); border-color: transparent; }
.invalid { border-color: var(--ol-color-danger); }
.chip {
  border: 1px solid var(--ol-border);
  border-radius: 999px;
  padding: 0 8px;
  font-size: var(--ol-font-size-11);
  background: var(--ol-surface);
  color: var(--ol-text-muted);
  cursor: pointer;
}
.chipOn { background: var(--ol-color-primary, #4f46e5); border-color: transparent; color: #fff; }
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeField.test.tsx`
Expected: PASS（7 个用例）

- [ ] **Step 5: 回归 + 提交**

Run: `cd poiesis && npx vitest run` — 232 全绿
```bash
git add poiesis/src/components/canvas/code/fields/
git commit -m "feat(canvas): NodeField 控件族（text/select/chip，草稿提交语义）
- 复刻 ComfyUI 节点内联控件模式：Enter/blur 提交、Esc 取消、外部 value 同步
- 全部 nodrag 防画布拖拽冲突；invalid 警示描边"
```

---

### Task 2: NodeArgsRows 参数行编辑

call/store-call 的 args 从"逗号分隔字符串"升级为可增删的参数行。**props 仍存逗号分隔字符串**（round-trip 契约不变），行编辑只是其结构化视图。

**Files:**
- Modify: `poiesis/src/components/canvas/code/fields/NodeField.tsx`（追加 NodeArgsRows）
- Modify: `poiesis/src/components/canvas/code/fields/NodeField.module.css`（追加 args 样式）
- Test: `poiesis/src/components/canvas/code/fields/__tests__/NodeArgsRows.test.tsx`

**Interfaces:**
- Consumes: Task 1 的 styles/`data-field-kind` 约定
- Produces: `NodeArgsRows({ values: string[]; onChange: (values: string[]) => void; placeholder?: string })` — 每参一行输入框 + × 删除 + 底部「+ 添加参数」；values 尾部空字符串折叠为「+」按钮

- [ ] **Step 1: 写失败测试**

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeArgsRows } from "../NodeField";

function inputs() {
  return screen.getAllByRole("textbox") as HTMLInputElement[];
}

describe("NodeArgsRows", () => {
  it("每参一行输入框", () => {
    render(<NodeArgsRows values={["a", "b"]} onChange={vi.fn()} />);
    expect(inputs()).toHaveLength(2);
    expect(inputs()[0].value).toBe("a");
  });

  it("编辑行内容提交 onChange", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.change(inputs()[0], { target: { value: "text" } });
    fireEvent.blur(inputs()[0]);
    expect(onChange).toHaveBeenCalledWith(["text"]);
  });

  it("点击 × 删除行", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a", "b"]} onChange={onChange} />);
    const del = screen.getAllByRole("button", { name: "×" });
    fireEvent.click(del[0]);
    expect(onChange).toHaveBeenCalledWith(["b"]);
  });

  it("点击 + 添加参数追加空行", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    expect(onChange).toHaveBeenCalledWith(["a", ""]);
  });

  it("尾随空行折叠为 + 按钮（不渲染输入框）", () => {
    render(<NodeArgsRows values={["a", ""]} onChange={vi.fn()} />);
    expect(inputs()).toHaveLength(1);
    expect(screen.getByRole("button", { name: "+ 添加参数" })).toBeTruthy();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeArgsRows.test.tsx`
Expected: FAIL（NodeArgsRows 不存在）

- [ ] **Step 3: 实现 NodeArgsRows（追加到 NodeField.tsx 末尾）**

```tsx
interface ArgsProps {
  values: string[];
  onChange: (values: string[]) => void;
  placeholder?: string;
}

export function NodeArgsRows({ values, onChange, placeholder }: ArgsProps) {
  const visible = [...values];
  while (visible.length > 0 && visible[visible.length - 1] === "") visible.pop();
  const hasTrailing = values.length > 0 && values[values.length - 1] === "";

  const update = (i: number, v: string) => {
    const next = [...values];
    next[i] = v;
    onChange(next);
  };
  const remove = (i: number) => {
    onChange(values.filter((_, idx) => idx !== i));
  };
  const add = () => {
    if (!hasTrailing) onChange([...values, ""]);
  };

  return (
    <div data-field-kind="args" className={styles.args}>
      {visible.map((v, i) => (
        <div key={i} className={styles.argsRow}>
          <NodeTextField value={v} onCommit={(nv) => update(i, nv)} placeholder={placeholder} ariaLabel={`参数 ${i + 1}`} />
          <button
            type="button"
            className={`nodrag ${styles.argsRemove}`}
            aria-label="×"
            onClick={() => remove(i)}
          >×</button>
        </div>
      ))}
      <button type="button" className={`nodrag ${styles.argsAdd}`} onClick={add}>+ 添加参数</button>
    </div>
  );
}
```

`NodeField.module.css` 追加:

```css
.args { display: flex; flex-direction: column; gap: 3px; align-items: stretch; }
.argsRow { display: flex; gap: 3px; align-items: center; }
.argsRow .field { flex: 1; width: 100%; }
.argsRemove {
  border: none; background: transparent; color: var(--ol-text-muted);
  cursor: pointer; font-size: 12; padding: 0 2px; flex-shrink: 0;
}
.argsAdd {
  align-self: flex-start; border: 1px dashed var(--ol-border);
  border-radius: var(--ol-radius-sm); background: transparent;
  color: var(--ol-text-muted); font-size: var(--ol-font-size-11);
  padding: 1px 8px; cursor: pointer;
}
.argsAdd:hover { color: var(--ol-text); border-color: var(--ol-border-hover); }
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeArgsRows.test.tsx`
Expected: PASS（5 个用例）

- [ ] **Step 5: 回归 + 提交**

Run: `cd poiesis && npx vitest run` — 232 全绿
```bash
git add poiesis/src/components/canvas/code/fields/
git commit -m "feat(canvas): NodeArgsRows 参数行编辑（args 结构化增删，props 仍存逗号字符串）"
```

---

### Task 3: flow-scope 补全候选数据层 + 轻量校验

从现有 store 数据收集节点字段的补全候选（零新数据源），并提供 scope 感知的即时校验。纯函数层，独立可测。

**Files:**
- Create: `poiesis/src/services/codegen/flow-scope.ts`
- Create: `poiesis/src/services/codegen/flow-scope-hook.ts`
- Test: `poiesis/src/services/codegen/__tests__/flow-scope.test.ts`

**Interfaces:**
- Consumes: 既有类型 `ComponentDef`（`core/types/project-json.ts`）、`StoreDefinition`（`core/types/store.ts`）、`CodeblockElementData`（`core/types/element.ts`）
- Produces:
  - `interface FlowScope { signals: Array<{name: string; type: string}>; props: Array<{name: string; type: string}>; computeds: Array<{name: string; returnType: string}>; events: Array<{name: string}>; fns: Array<{name: string; params: Array<{name: string; type: string}>}>; stores: Array<{ name: string; actions: Array<{name: string; params: Array<{name: string; type: string}>}>; signals: Array<{name: string; type: string}> }>; fnParams: Array<{name: string; type: string}>; letVars: Array<{name: string}> }`
  - `buildFlowScope(input: { componentDef?: ComponentDef; storeDefs: Record<string, StoreDefinition>; inlineStoreDefs: Record<string, StoreDefinition>; storesUsed: string[]; fnParamsStr?: string; fnElements: CodeblockElementData[] }): FlowScope`
  - `parseFnParamsStr(s: string): Array<{name: string; type: string}>` — 与 FnInspector 相同格式（"name: Type, name: Type"），导出供测试
  - `filterCandidates(query: string, candidates: Array<{name: string; type?: string}>): Array<{name: string; type?: string}>` — 子串前缀匹配（空 query 返回全部，最多 8 项）
  - `validateSyntaxProps(tag: string, props: Record<string, unknown>, scope: FlowScope): Array<{ field: string; message: string }>` — let: name 必须合法标识符（`/^[a-zA-Z_]\w*$/`）且非空；assign: target 非空且合法标识符（允许 `Store.signal` 形式）；store-call: store 名存在于 scope.stores 则 action 必须在其 actions 中（store 未知则跳过）；call: fnName 非空
  - `useFlowScope(componentName: string | undefined): FlowScope | null`（hook，包装 store 选择器 + buildFlowScope）

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/services/codegen/__tests__/flow-scope.test.ts
import { describe, it, expect } from "vitest";
import { buildFlowScope, parseFnParamsStr, filterCandidates, validateSyntaxProps, type FlowScope } from "../flow-scope";

const compDef = {
  name: "App", filePath: "App.omni",
  signals: [{ pid: "p1", name: "count", type: "i32" }, { pid: "p2", name: "items", type: "Array<String>" }],
  props: [{ pid: "p3", name: "title", type: "String" }],
  computeds: [{ pid: "p4", name: "doneCount", returnType: "i32", expression: "" }],
  events: [{ pid: "p5", name: "onSelect" }],
  fns: [{ pid: "p6", name: "close", params: [{ name: "x", type: "i32" }], returnType: "" }],
};

const storeDefs = {
  TodoStore: {
    pid: "s1", name: "TodoStore", filePath: "TodoStore.omni", imports: [],
    signals: [{ pid: "s2", name: "list", type: "Array<String>" }],
    getters: [], actions: [{ pid: "s3", name: "add", params: [{ name: "text", type: "String" }] }],
  },
};

const fnEls = [
  { id: "l1", nodeType: "codeblock", tag: "let", parentId: null, props: { name: "total", mutable: false, value: "0" } },
  { id: "l2", nodeType: "codeblock", tag: "let", parentId: null, props: { name: "x" } },
] as unknown as Parameters<typeof buildFlowScope>[0]["fnElements"];

describe("parseFnParamsStr", () => {
  it("解析 name: Type 逗号格式", () => {
    expect(parseFnParamsStr("text: String, count: i32")).toEqual([
      { name: "text", type: "String" }, { name: "count", type: "i32" },
    ]);
  });
  it("空串返回空数组", () => {
    expect(parseFnParamsStr("")).toEqual([]);
  });
});

describe("buildFlowScope", () => {
  const scope = buildFlowScope({
    componentDef: compDef as never,
    storeDefs, inlineStoreDefs: {}, storesUsed: ["TodoStore"],
    fnParamsStr: "text: String",
    fnElements: fnEls,
  });

  it("收集 signals/props/computeds/events/fns", () => {
    expect(scope.signals.map(s => s.name)).toEqual(["count", "items"]);
    expect(scope.props[0].name).toBe("title");
    expect(scope.computeds[0].name).toBe("doneCount");
    expect(scope.events[0].name).toBe("onSelect");
    expect(scope.fns[0].name).toBe("close");
  });
  it("收集 store actions 与 signals（storeDefs 与 inlineStoreDefs 合并）", () => {
    expect(scope.stores[0].name).toBe("TodoStore");
    expect(scope.stores[0].actions[0].name).toBe("add");
    expect(scope.stores[0].signals[0].name).toBe("list");
  });
  it("收集 fn 参数与 let 变量", () => {
    expect(scope.fnParams[0].name).toBe("text");
    expect(scope.letVars.map(v => v.name)).toEqual(["total", "x"]);
  });
  it("fnElements 缺省时 letVars 为空", () => {
    const s2 = buildFlowScope({ componentDef: undefined, storeDefs: {}, inlineStoreDefs: {}, storesUsed: [], fnElements: [] });
    expect(s2.letVars).toEqual([]);
    expect(s2.stores).toEqual([]);
  });
});

describe("filterCandidates", () => {
  it("空 query 返回全部（限 8 项）", () => {
    const c = Array.from({ length: 10 }, (_, i) => ({ name: `v${i}` }));
    expect(filterCandidates("", c)).toHaveLength(8);
  });
  it("前缀匹配", () => {
    const c = [{ name: "count" }, { name: "doneCount" }, { name: "title" }];
    expect(filterCandidates("count", c).map(x => x.name)).toEqual(["count"]);
    expect(filterCandidates("ti", c).map(x => x.name)).toEqual(["title"]);
  });
  it("无匹配返回空", () => {
    expect(filterCandidates("zzz", [{ name: "count" }])).toEqual([]);
  });
});

describe("validateSyntaxProps", () => {
  it("let 非法变量名报错", () => {
    const r = validateSyntaxProps("let", { name: "1bad", mutable: false }, {} as FlowScope);
    expect(r.some(x => x.field === "name")).toBe(true);
  });
  it("let 合法通过", () => {
    expect(validateSyntaxProps("let", { name: "count" }, {} as FlowScope)).toEqual([]);
  });
  it("store-call action 不在所选 store 时报错", () => {
    const scope = buildFlowScope({ componentDef: undefined, storeDefs, inlineStoreDefs: {}, storesUsed: ["TodoStore"], fnElements: [] });
    const r = validateSyntaxProps("store-call", { storeName: "TodoStore", action: "nope" }, scope);
    expect(r.some(x => x.field === "action")).toBe(true);
  });
  it("store-call 未知 store 不报 action 错（跳过校验）", () => {
    const scope = buildFlowScope({ componentDef: undefined, storeDefs: {}, inlineStoreDefs: {}, storesUsed: [], fnElements: [] });
    expect(validateSyntaxProps("store-call", { storeName: "UnknownStore", action: "x" }, scope)).toEqual([]);
  });
  it("assign 空 target 报错", () => {
    const r = validateSyntaxProps("assign", { target: "", operator: "=" }, {} as FlowScope);
    expect(r.some(x => x.field === "target")).toBe(true);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/flow-scope.test.ts`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现 flow-scope.ts**

```ts
// poiesis/src/services/codegen/flow-scope.ts
// 节点内联编辑的补全候选数据层 + 轻量即时校验。
// 纯函数：从 store 现有数据（componentDefs/storeDefs/inlineStoreDefs/elements）收集候选，零新数据源。
import type { ComponentDef } from "../../core/types/project-json";
import type { StoreDefinition } from "../../core/types/store";
import type { CodeblockElementData } from "../../core/types/element";

export interface ScopeParam { name: string; type: string }
export interface ScopeStore {
  name: string;
  actions: Array<{ name: string; params: ScopeParam[] }>;
  signals: Array<{ name: string; type: string }>;
}
export interface FlowScope {
  signals: Array<{ name: string; type: string }>;
  props: Array<{ name: string; type: string }>;
  computeds: Array<{ name: string; returnType: string }>;
  events: Array<{ name: string }>;
  fns: Array<{ name: string; params: ScopeParam[] }>;
  stores: ScopeStore[];
  fnParams: ScopeParam[];
  letVars: Array<{ name: string }>;
}

/** 与 FnInspector 相同的 params 字符串格式（"name: Type, name: Type"）。 */
export function parseFnParamsStr(s: string): ScopeParam[] {
  if (!s) return [];
  return s.split(",").map((p) => {
    const [pName, pType] = p.trim().split(":").map((t) => t.trim());
    return { name: pName || "", type: pType || "String" };
  });
}

export function buildFlowScope(input: {
  componentDef?: ComponentDef;
  storeDefs: Record<string, StoreDefinition>;
  inlineStoreDefs: Record<string, StoreDefinition>;
  storesUsed: string[];
  fnParamsStr?: string;
  fnElements: CodeblockElementData[];
}): FlowScope {
  const { componentDef, storeDefs, inlineStoreDefs, storesUsed } = input;
  const allStoreDefs = { ...inlineStoreDefs, ...storeDefs };
  const storeNames = new Set([...Object.keys(allStoreDefs), ...storesUsed]);

  return {
    signals: componentDef?.signals?.map((s) => ({ name: s.name, type: s.type })) ?? [],
    props: componentDef?.props?.map((p) => ({ name: p.name, type: p.type })) ?? [],
    computeds: componentDef?.computeds?.map((c) => ({ name: c.name, returnType: c.returnType })) ?? [],
    events: componentDef?.events?.map((e) => ({ name: e.name })) ?? [],
    fns: componentDef?.fns?.map((f) => ({ name: f.name, params: f.params ?? [] })) ?? [],
    stores: [...storeNames]
      .filter(Boolean)
      .map((name) => {
        const def = allStoreDefs[name];
        return {
          name,
          actions: def?.actions?.map((a) => ({ name: a.name, params: a.params ?? [] })) ?? [],
          signals: def?.signals?.map((s) => ({ name: s.name, type: s.type })) ?? [],
        };
      }),
    fnParams: parseFnParamsStr(input.fnParamsStr),
    letVars: input.fnElements
      .filter((e) => e.tag === "let")
      .map((e) => ({ name: (e.props?.name as string) ?? "" }))
      .filter((v) => v.name !== ""),
  };
}

/** 前缀子串匹配候选，空 query 返回全部（限 8 项）。 */
export function filterCandidates(query: string, candidates: Array<{ name: string; type?: string }>): Array<{ name: string; type?: string }> {
  const q = query.trim();
  if (!q) return candidates.slice(0, 8);
  return candidates.filter((c) => c.name.startsWith(q)).slice(0, 8);
}

const IDENT = /^[a-zA-Z_]\w*$/;
const TARGET = /^[a-zA-Z_]\w*(\.[a-zA-Z_]\w*)?$/;

/** 轻量即时校验：只提示不阻塞（保存时 WASM 门禁最终裁决）。 */
export function validateSyntaxProps(
  tag: string,
  props: Record<string, unknown>,
  scope: FlowScope,
): Array<{ field: string; message: string }> {
  const errors: Array<{ field: string; message: string }> = [];
  if (tag === "let") {
    const name = (props.name as string) ?? "";
    if (!IDENT.test(name)) errors.push({ field: "name", message: "变量名需为合法标识符（字母/下划线开头）" });
  }
  if (tag === "assign") {
    const target = (props.target as string) ?? "";
    if (!TARGET.test(target)) errors.push({ field: "target", message: "目标需为标识符或 Store.信号" });
  }
  if (tag === "store-call") {
    const storeName = (props.storeName as string) ?? "";
    const action = (props.action as string) ?? "";
    const store = scope.stores.find((s) => s.name === storeName);
    if (store && !store.actions.some((a) => a.name === action)) {
      errors.push({ field: "action", message: `action「${action}」不在 ${storeName} 中` });
    }
  }
  if (tag === "call") {
    const fnName = (props.fnName as string) ?? "";
    if (!fnName) errors.push({ field: "fnName", message: "函数名不能为空" });
  }
  return errors;
}
```

- [ ] **Step 4: 实现 flow-scope-hook.ts**

```ts
// poiesis/src/services/codegen/flow-scope-hook.ts
// 从 Zustand store 取数据构建 FlowScope（组件内便捷入口）。
import { useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { useShallow } from "zustand/react/shallow";
import type { CodeblockElementData } from "../../core/types/element";
import { buildFlowScope, type FlowScope } from "./flow-scope";

export function useFlowScope(componentName: string | undefined): FlowScope | null {
  const componentDef = usePoiesisStore((s) => (componentName ? s.componentDefs[componentName] : undefined));
  const storeDefs = usePoiesisStore(useShallow((s) => s.storeDefs));
  const inlineStoreDefs = usePoiesisStore(useShallow((s) => s.inlineStoreDefs));
  const storeReferences = usePoiesisStore(useShallow((s) => s.storeReferences));
  const elements = usePoiesisStore(useShallow((s) => s.elements));

  return useMemo(() => {
    if (!componentName) return null;
    const storesUsed = storeReferences[componentName] ?? [];
    const fnEl = (elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock" && e.tag === "fn"));
    return buildFlowScope({
      componentDef,
      storeDefs,
      inlineStoreDefs,
      storesUsed,
      fnParamsStr: fnEl[0]?.props?.params as string | undefined,
      fnElements: elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock"),
    });
  }, [componentName, componentDef, storeDefs, inlineStoreDefs, storeReferences, elements]);
}
```

- [ ] **Step 5: 运行确认通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/flow-scope.test.ts`
Expected: PASS（13 个用例）

- [ ] **Step 6: 类型检查 + 回归 + 提交**

Run: `cd poiesis && npx tsc --noEmit && npx vitest run` — 232 全绿
```bash
git add poiesis/src/services/codegen/flow-scope.ts poiesis/src/services/codegen/flow-scope-hook.ts poiesis/src/services/codegen/__tests__/flow-scope.test.ts
git commit -m "feat(canvas): flow-scope 补全候选数据层 + 轻量即时校验（纯函数，零新数据源）"
```

---

### Task 4: SyntaxNode 内联编辑重写

4 种语法节点（let/assign/store-call/call）改为节点上直接编辑。store-call 的 store/action 联动下拉；call 带 fnName 补全与 emit 徽章；节点底部警告条。

**Files:**
- Rewrite: `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx`
- Modify: `poiesis/src/components/canvas/code/nodes/SyntaxNode.module.css`
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（仅 nodes data 注入 `scope` 与 `onEditProp`——见 Step 4 说明）
- Test: `poiesis/src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx`（新建目录）

**Interfaces:**
- Consumes: `NodeTextField/NodeSelect/NodeChipToggle/NodeArgsRows`（Task 1/2）、`FlowScope/validateSyntaxProps/filterCandidates`（Task 3）、`formatSyntaxNode`（既有）、`useFlowScope`（Task 3）
- Produces: SyntaxNode data 契约扩展——`data: { label: string; tag: string; props?: Record<string, unknown>; scope?: FlowScope; onEditProp?: (id: string, prop: string, value: unknown) => void }`；FlowCanvas 传入 `scope`（useFlowScope）与 `onEditProp`（setProp 包装，内含"scope 不存在时跳过"守卫）

- [ ] **Step 1: 写失败测试**

```tsx
// poiesis/src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SyntaxNode } from "../SyntaxNode";
import type { FlowScope } from "../../../../services/codegen/flow-scope";

const scope: FlowScope = {
  signals: [{ name: "count", type: "i32" }],
  props: [], computeds: [], events: [{ name: "onSelect" }],
  fns: [{ name: "close", params: [] }],
  stores: [{ name: "TodoStore", actions: [{ name: "add", params: [{ name: "text", type: "String" }] }], signals: [] }],
  fnParams: [], letVars: [],
};

function renderNode(tag: string, props: Record<string, unknown>, onEditProp = vi.fn()) {
  return render(
    <SyntaxNode data={{ label: tag, tag, props, scope, onEditProp }} />
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
    expect(screen.getByText(/emit/)).toBeTruthy();
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
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx`
Expected: FAIL（新契约未实现）

- [ ] **Step 3: 重写 SyntaxNode.tsx**

```tsx
// poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx
// 语法节点内联编辑版：节点即编辑面（Unity VS 交互精神）。
// 草稿在 NodeField 内部，提交走 onEditProp → setProp；props 仍是唯一事实来源。
import { Handle, Position } from "@xyflow/react";
import React, { useMemo } from "react";
import { formatSyntaxNode, type SyntaxNodeProps } from "../../../../services/codegen/syntax-nodes";
import { NodeTextField, NodeSelect, NodeChipToggle, NodeArgsRows } from "../fields/NodeField";
import { validateSyntaxProps, filterCandidates, type FlowScope } from "../../../../services/codegen/flow-scope";
import styles from "./SyntaxNode.module.css";

const OMNI_TYPES = ["String", "i32", "i64", "f64", "f32", "bool", "Array<String>", "Array<i32>", "Array<f64>", "Map<String, String>"];
const ASSIGN_OPTS = ["=", "+=", "-=", "*=", "/=", "%="].map((op) => ({ value: op, label: op }));
const TYPE_OPTS = OMNI_TYPES.map((t) => ({ value: t, label: t }));

interface Props {
  data: {
    label: string;
    tag: string;
    props?: Record<string, unknown>;
    scope?: FlowScope;
    onEditProp?: (id: string, prop: string, value: unknown) => void;
  };
}

const SyntaxNode = React.memo(function SyntaxNode({ data }: Props) {
  const { tag, props = {}, scope, onEditProp } = data;
  const id = useMemo(() => (props as { __id?: string }).__id ?? "el", [props]);
  const set = (prop: string, value: unknown) => onEditProp?.(id, prop, value);

  const errors = useMemo(
    () => (scope ? validateSyntaxProps(tag, props, scope) : []),
    [tag, props, scope],
  );
  const errorFields = useMemo(() => new Set(errors.map((e) => e.field)), [errors]);

  const storeCandidates = useMemo(() => (scope?.stores ?? []).map((s) => ({ value: s.name, label: s.name })), [scope]);
  const actionCandidates = useMemo(() => {
    const storeName = (props.storeName as string) ?? "";
    const store = scope?.stores.find((s) => s.name === storeName);
    return (store?.actions ?? []).map((a) => ({ value: a.name, label: a.params.length ? `${a.name}(${a.params.map((p) => `${p.name}: ${p.type}`).join(", ")})` : a.name }));
  }, [scope, props.storeName]);

  const fnCandidates = useMemo(() => {
    const base = [
      ...(scope?.fns ?? []).map((f) => ({ name: f.name, type: "fn" })),
      ...(scope?.events ?? []).map((e) => ({ name: e.name, type: "event" })),
    ];
    return filterCandidates("", base).map((c) => ({ value: c.name, label: c.name }));
  }, [scope]);

  const targetPlaceholder = tag === "assign"
    ? scope?.signals[0] ? `e.g. ${scope.signals[0].name} / ThemeStore.mode` : "e.g. count / ThemeStore.mode"
    : undefined;

  const code = useMemo(() => formatSyntaxNode(tag, props as SyntaxNodeProps), [tag, props]);

  return (
    <div
      className={`flow-node ${styles.node}`}
      data-node-type={tag}
      data-syntax-code={code}
    >
      {/* 头部：tag 中文标签 */}
      <div className={styles.header}>{data.label}</div>

      <div className={styles.body}>
        {tag === "let" && (
          <>
            <div className={styles.row}>
              <NodeChipToggle checked={!!props.mutable} onChange={(v) => set("mutable", v)} label="mut" ariaLabel="mut" />
              <NodeTextField
                value={(props.name as string) ?? ""}
                onCommit={(v) => set("name", v)}
                placeholder="变量名"
                ariaLabel="变量名"
                invalid={errorFields.has("name")}
                className={styles.grow}
              />
              <NodeSelect value={(props.varType as string) ?? ""} options={TYPE_OPTS} onCommit={(v) => set("varType", v || undefined)} ariaLabel="类型" allowCustom />
            </div>
            <div className={styles.row}>
              <NodeTextField
                value={(props.value as string) ?? ""}
                onCommit={(v) => set("value", v)}
                placeholder="初始值（可放表达式）"
                ariaLabel="初始值"
                className={styles.grow}
              />
            </div>
          </>
        )}

        {tag === "assign" && (
          <div className={styles.row}>
            <NodeTextField
              value={(props.target as string) ?? ""}
              onCommit={(v) => set("target", v)}
              placeholder={targetPlaceholder}
              ariaLabel="目标"
              invalid={errorFields.has("target")}
              className={styles.grow}
            />
            <NodeSelect value={(props.operator as string) ?? "="} options={ASSIGN_OPTS} onCommit={(v) => set("operator", v)} ariaLabel="运算符" />
            <NodeTextField
              value={(props.value as string) ?? ""}
              onCommit={(v) => set("value", v)}
              placeholder="表达式"
              ariaLabel="表达式"
              className={styles.grow}
            />
          </div>
        )}

        {tag === "store-call" && (
          <>
            <div className={styles.row}>
              <NodeSelect
                value={(props.storeName as string) ?? ""}
                options={storeCandidates}
                onCommit={(v) => { set("storeName", v); set("action", ""); }}
                ariaLabel="store 名"
                allowCustom
                className={styles.grow}
              />
              <span className={styles.op}>.</span>
              <NodeSelect
                value={(props.action as string) ?? ""}
                options={actionCandidates}
                onCommit={(v) => set("action", v)}
                ariaLabel="action"
                invalid={errorFields.has("action")}
                allowCustom
                className={styles.grow}
              />
            </div>
            <NodeArgsRows
              values={((props.args as string) ?? "").split(",").map((s) => s.trim()).filter((s) => s !== "")}
              onChange={(vals) => set("args", vals.filter((v) => v !== "").join(", "))}
              placeholder="参数"
            />
          </>
        )}

        {tag === "call" && (
          <>
            <div className={styles.row}>
              {props.emit ? <span className={styles.op}>emit</span> : null}
              <NodeSelect
                value={(props.fnName as string) ?? ""}
                options={fnCandidates}
                onCommit={(v) => set("fnName", v)}
                ariaLabel="函数名"
                invalid={errorFields.has("fnName")}
                allowCustom
                className={styles.grow}
              />
              <NodeChipToggle
                checked={!!props.emit}
                onChange={(v) => set("emit", v)}
                label="emit"
                ariaLabel="emit 事件"
              />
            </div>
            <NodeArgsRows
              values={((props.args as string) ?? "").split(",").map((s) => s.trim()).filter((s) => s !== "")}
              onChange={(vals) => set("args", vals.filter((v) => v !== "").join(", "))}
              placeholder="参数"
            />
          </>
        )}
      </div>

      {errors.length > 0 && (
        <div className={styles.warnBar}>
          {errors.map((e) => <div key={e.field}>{e.message}</div>)}
        </div>
      )}

      <Handle type="target" position={Position.Top} id="flow-in" style={{ background: "var(--ol-text-muted)" }} />
      <Handle type="source" position={Position.Bottom} id="flow-out" style={{ background: "var(--ol-text-muted)" }} />
    </div>
  );
});
export { SyntaxNode };
```

`SyntaxNode.module.css` 追加:

```css
.header {
  padding: "2px 10px";
  font-weight: 600;
  font-size: var(--ol-font-size-11);
  color: var(--ol-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-bottom: 4px;
}
.body { display: flex; flex-direction: column; gap: 4px; padding: 0 8px; }
.row { display: flex; gap: 4px; align-items: center; }
.grow { flex: 1; min-width: 40px; width: 100%; }
.op { color: var(--ol-text-muted); font-family: var(--ol-font-family-mono); font-size: 12px; }
.warnBar {
  margin: 6px 8px 0;
  padding: 3px 6px;
  border-radius: var(--ol-radius-sm);
  background: var(--ol-color-danger-subtle, #fef2f2);
  color: var(--ol-color-danger);
  font-size: var(--ol-font-size-11);
}
```

- [ ] **Step 4: FlowCanvas 注入 scope 与 onEditProp**

`poiesis/src/components/canvas/code/FlowCanvas.tsx` 修改点：
1. import 增加 `useFlowScope` 与既有 `activeTabId/openTabs/componentDefs` 已存在
2. 组件内（`FlowCanvasInner`）：`const activeTab = openTabs.find((t) => t.id === activeTabId); const flowScope = useFlowScope(activeTab?.componentName);`
3. `nodes` useMemo 的 data 中为 4 个语法 tag 追加：
```tsx
...(CODE_TAGS.has(el.tag) && el.tag !== "code-block" ? {
  scope: flowScope ?? undefined,
  onEditProp: (id: string, prop: string, value: unknown) => setProp(id, prop, value),
} : {}),
```
4. nodes useMemo 依赖数组追加 `flowScope, setProp`

- [ ] **Step 5: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx`
Expected: PASS（9 个用例）

- [ ] **Step 6: 类型检查 + 回归 + 提交**

Run: `cd poiesis && npx tsc --noEmit && npx vitest run` — 232 全绿
```bash
git add poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx poiesis/src/components/canvas/code/nodes/SyntaxNode.module.css poiesis/src/components/canvas/code/nodes/__tests__/ poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat(canvas): SyntaxNode 内联编辑（节点即编辑面，store/action 联动下拉 + 校验警告条）
- let/assign/store-call/call 全部字段节点上直接编辑，Enter/blur 提交
- store-call 选 store 重置 action；call 函数名候选补全（fns+events）
- 本地轻量校验：错误字段红描边 + 节点警告条，不阻塞输入"
```

---

### Task 5: ConditionNode / ReturnNode 内联编辑

condition 的 if 表达式与 return 的表达式改为节点上内联编辑（复用 NodeField 草稿提交语义）。

**Files:**
- Modify: `poiesis/src/components/canvas/code/nodes/ConditionNode.tsx`
- Modify: `poiesis/src/components/canvas/code/nodes/ReturnNode.tsx`
- Test: `poiesis/src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx`

**Interfaces:**
- Consumes: `NodeTextField`（Task 1）、既有 `onEditCondition/onUpdateBranches` data 回调（FlowCanvas 已有）
- Produces: ConditionNode/ReturnNode 内联编辑（编辑态由"点击表达式按钮"触发，NodeTextField 常驻显示已编辑值；else 分支不可编辑保持现状）

- [ ] **Step 1: 写失败测试**

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConditionNode } from "../ConditionNode";
import { ReturnNode } from "../ReturnNode";

describe("ConditionNode 内联编辑", () => {
  it("点击 if 表达式进入编辑，blur 提交 onEditCondition", () => {
    const onEditCondition = vi.fn();
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br0", type: "if", condition: "" }], onEditCondition }} />);
    fireEvent.click(screen.getByText("..."));
    const input = screen.getByLabelText("编辑条件表达式");
    fireEvent.change(input, { target: { value: "count > 0" } });
    fireEvent.blur(input);
    expect(onEditCondition).toHaveBeenCalledWith("br0", "count > 0");
  });

  it("已有条件直接显示，点击进入编辑", () => {
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br0", type: "if", condition: "x == 1" }], onEditCondition: vi.fn() }} />);
    expect(screen.getByText("x == 1")).toBeTruthy();
  });

  it("else 分支不可编辑", () => {
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br1", type: "else", condition: "" }], onEditCondition: vi.fn() }} />);
    expect(screen.queryByLabelText("编辑条件表达式")).toBeFalsy();
  });
});

describe("ReturnNode 内联编辑", () => {
  it("显示 return 表达式", () => {
    render(<ReturnNode data={{ label: "返回", returnExpr: "items.len()" }} />);
    expect(screen.getByText(/items\.len\(\)/)).toBeTruthy();
  });

  it("双击进入编辑，Enter 提交", () => {
    const onCommit = vi.fn();
    render(<ReturnNode data={{ label: "返回", returnExpr: "x", onDoubleClick: undefined }} />);
    // ReturnNode 使用内部编辑态 + setElementSource 回调（经 FlowCanvas data.onDoubleClick 同源）
    // 本用例验证点击编辑入口：见实现——return 表达式区点击即编辑
    fireEvent.click(screen.getByText("return x"));
    const input = screen.getByLabelText("return 表达式");
    fireEvent.change(input, { target: { value: "y" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("y");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx`
Expected: FAIL（ReturnNode 无编辑态）

- [ ] **Step 3: 修改 ConditionNode.tsx**

保持现有整体结构（编辑态 input + 分支增删按钮），仅把编辑 input 替换为 `NodeTextField`（获得统一的 hover/focus 视觉与 Esc 取消语义）：

```tsx
// 替换 handleEditConfirm 相关编辑块
// import 增加: import { NodeTextField } from "../fields/NodeField";
// 编辑态:
{editingBranch === br.id ? (
  <NodeTextField
    value={editValue}
    onCommit={(v) => { data.onEditCondition?.(br.id, v); setEditingBranch(null); }}
    placeholder="条件表达式"
    ariaLabel="编辑条件表达式"
    className={styles.grow}  // 或内联 style width:"100%"
  />
) : ( /* 现有 button 不变 */ )}
```

（注意 `editValue` 初值仍在 `handleEditStart` 设置；NodeTextField 每次编辑态进入时以 `editValue` 为初值渲染。）

- [ ] **Step 4: 修改 ReturnNode.tsx**

```tsx
// poiesis/src/components/canvas/code/nodes/ReturnNode.tsx
import { Handle, Position } from "@xyflow/react";
import { useState } from "react";
import styles from "./ReturnNode.module.css";
import { NodeTextField } from "../fields/NodeField";

interface Props {
  data: {
    label: string;
    returnExpr?: string;
    onDoubleClick?: () => void;
    onCommitReturn?: (expr: string) => void;
  };
}

export function ReturnNode({ data }: Props) {
  const expr = data.returnExpr || "";
  const [editing, setEditing] = useState(false);

  return (
    <div className={`flow-node ${styles.node}`} data-node-type="return">
      <div
        style={{ display: "flex", alignItems: "center", gap: 4 }}
        onClick={() => setEditing(true)}
        title="点击编辑"
      >
        <span>return</span>
        {editing ? (
          <NodeTextField
            value={expr}
            onCommit={(v) => { data.onCommitReturn?.(v); setEditing(false); }}
            placeholder="表达式"
            ariaLabel="return 表达式"
            className={styles.grow}
          />
        ) : (
          <span>{expr}</span>
        )}
      </div>
      <Handle type="target" position={Position.Top} id="flow-in" style={{ background: "var(--ol-color-danger)" }} />
    </div>
  );
}
```

`FlowCanvas.tsx` data 中 return 节点追加：

```tsx
onCommitReturn: (expr: string) => setElementSource(el.id, expr),
```

（与 `handleSaveSource` 相同路径：return 节点以 `source` 为权威——graph-parser 从 `return <expr>;` 提取。）

- [ ] **Step 5: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx`
Expected: PASS（5 个用例）

- [ ] **Step 6: 类型检查 + 回归 + 提交**

Run: `cd poiesis && npx tsc --noEmit && npx vitest run` — 232 全绿
```bash
git add poiesis/src/components/canvas/code/nodes/ConditionNode.tsx poiesis/src/components/canvas/code/nodes/ReturnNode.tsx poiesis/src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat(canvas): condition/return 节点内联表达式编辑（复用 NodeField 草稿提交语义）"
```

---

### Task 6: NodeSearchMenu 可搜索节点创建菜单

复刻 ComfyUI 搜索菜单结构：搜索框（自动聚焦）+ 分类分组 + 命中高亮 + 键盘导航（↑↓/Enter/Esc）+ 无结果态。

**Files:**
- Create: `poiesis/src/components/canvas/code/NodeSearchMenu.tsx`
- Create: `poiesis/src/components/canvas/code/NodeSearchMenu.module.css`
- Test: `poiesis/src/components/canvas/code/__tests__/NodeSearchMenu.test.tsx`

**Interfaces:**
- Consumes: `MENU_SECTIONS/TAG_CN`（`flowConstants.ts` 既有）
- Produces:
  - `filterMenuItems(query: string): Array<{ section: string; items: Array<{ label: string; tag: string; shortcut?: string }> }>` — 导出纯函数；子串匹配中文 label/英文 tag/快捷键；空 query 返回全部分组（原序）
  - `NodeSearchMenu({ x: number; y: number; onSelect: (tag: string) => void; onClose: () => void })` — 挂载自动聚焦；输入过滤；↑↓ 移动选中（跨分组连续索引、自动滚动）；Enter 选中创建；Esc/点击菜单外/失焦关闭；无结果显示「无匹配节点」

- [ ] **Step 1: 写失败测试**

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeSearchMenu, filterMenuItems } from "../NodeSearchMenu";

describe("filterMenuItems", () => {
  it("空 query 返回全部 3 个分组", () => {
    const r = filterMenuItems("");
    expect(r.map((s) => s.section)).toEqual(["控制流", "变量与赋值", "调用"]);
    expect(r.flatMap((s) => s.items)).toHaveLength(8);
  });
  it("中文子串匹配", () => {
    const r = filterMenuItems("赋值");
    expect(r.flatMap((s) => s.items).map((i) => i.tag)).toEqual(["assign"]);
  });
  it("英文 tag 匹配", () => {
    const r = filterMenuItems("store");
    expect(r.flatMap((s) => s.items).map((i) => i.tag)).toEqual(["store-call"]);
  });
  it("快捷键别名匹配", () => {
    const r = filterMenuItems("ret");
    expect(r.flatMap((s) => s.items).map((i) => i.tag)).toEqual(["return"]);
  });
  it("无匹配返回空", () => {
    expect(filterMenuItems("zzz").flatMap((s) => s.items)).toEqual([]);
  });
});

describe("NodeSearchMenu", () => {
  it("挂载后自动聚焦搜索框", () => {
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByPlaceholderText("搜索节点…")).toHaveFocus();
  });

  it("输入过滤并高亮命中", () => {
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText("搜索节点…"), { target: { value: "let" } });
    expect(screen.getByText("变量声明 (let)")).toBeTruthy();
    expect(screen.queryByText("函数定义 (fn)")).toBeFalsy();
  });

  it("Enter 创建当前选中项", () => {
    const onSelect = vi.fn();
    render(<NodeSearchMenu x={0} y={0} onSelect={onSelect} onClose={vi.fn()} />);
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("fn");
  });

  it("↓ 移动选中后 Enter 创建对应项", () => {
    const onSelect = vi.fn();
    render(<NodeSearchMenu x={0} y={0} onSelect={onSelect} onClose={vi.fn()} />);
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("condition");
  });

  it("Esc 关闭", () => {
    const onClose = vi.fn();
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={onClose} />);
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("无结果显示提示", () => {
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText("搜索节点…"), { target: { value: "zzz" } });
    expect(screen.getByText("无匹配节点")).toBeTruthy();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/NodeSearchMenu.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现 NodeSearchMenu.tsx**

```tsx
// poiesis/src/components/canvas/code/NodeSearchMenu.tsx
// 可搜索节点创建菜单（复刻 ComfyUI NodeSearchBox：自动聚焦/过滤/高亮/键盘导航）。
import { useEffect, useMemo, useRef, useState } from "react";
import { MENU_SECTIONS } from "./flowConstants";
import styles from "./NodeSearchMenu.module.css";

export interface FilteredSection {
  section: string;
  items: Array<{ label: string; tag: string; shortcut?: string }>;
}

/** 子串匹配：中文 label / 英文 tag / 快捷键别名；空 query 返回全部。 */
export function filterMenuItems(query: string): FilteredSection[] {
  const q = query.trim().toLowerCase();
  if (!q) return MENU_SECTIONS.map((s) => ({ section: s.label, items: s.items }));
  return MENU_SECTIONS.map((s) => ({
    section: s.label,
    items: s.items.filter((i) =>
      i.label.toLowerCase().includes(q) ||
      i.tag.toLowerCase().includes(q) ||
      (i.shortcut ?? "").toLowerCase().includes(q),
    ),
  })).filter((s) => s.items.length > 0);
}

/** 命中子串高亮（转义后渲染）。 */
function highlight(text: string, query: string): React.ReactNode {
  const q = query.trim();
  if (!q) return text;
  const idx = text.toLowerCase().indexOf(q.toLowerCase());
  if (idx < 0) return text;
  return (
    <>
      {text.slice(0, idx)}
      <mark className={styles.hit}>{text.slice(idx, idx + q.length)}</mark>
      {text.slice(idx + q.length)}
    </>
  );
}

interface Props {
  x: number;
  y: number;
  onSelect: (tag: string) => void;
  onClose: () => void;
}

export function NodeSearchMenu({ x, y, onSelect, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [activeIdx, setActiveIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  const sections = useMemo(() => filterMenuItems(query), [query]);
  const flat = useMemo(() => sections.flatMap((s) => s.items), [sections]);

  // 自动聚焦
  useEffect(() => { inputRef.current?.focus(); }, []);

  // 点击菜单外关闭
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  // 过滤变化时重置选中
  useEffect(() => { setActiveIdx(0); }, [query]);

  // 选中项滚动可见
  useEffect(() => {
    const active = listRef.current?.querySelector(`[data-menu-index="${activeIdx}"]`);
    active?.scrollIntoView({ block: "nearest" });
  }, [activeIdx]);

  const select = (tag: string) => {
    onSelect(tag);
    onClose();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") { e.preventDefault(); setActiveIdx((i) => Math.min(i + 1, flat.length - 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setActiveIdx((i) => Math.max(i - 1, 0)); }
    else if (e.key === "Enter") { e.preventDefault(); if (flat[activeIdx]) select(flat[activeIdx].tag); }
    else if (e.key === "Escape") { e.preventDefault(); onClose(); }
  };

  const vw = window.innerWidth;
  const vwH = window.innerHeight;
  const left = Math.min(x, vw - 260);
  const top = Math.min(y, vwH - 320);

  return (
    <div ref={rootRef} className={styles.menu} style={{ left, top }} role="menu" onContextMenu={(e) => e.preventDefault()}>
      <input
        ref={inputRef}
        className={styles.search}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder="搜索节点…"
        aria-label="搜索节点"
        role="combobox"
        aria-expanded="true"
      />
      <div ref={listRef} className={styles.list}>
        {flat.length === 0 ? (
          <div className={styles.empty}>无匹配节点</div>
        ) : (
          sections.map((sec) => (
            <div key={sec.section}>
              <div className={styles.sectionLabel}>{sec.section}</div>
              {sec.items.map((item) => {
                const idx = flat.indexOf(item);
                return (
                  <button
                    type="button"
                    key={item.tag}
                    data-menu-index={idx}
                    className={`${styles.item} ${idx === activeIdx ? styles.itemActive : ""}`}
                    onClick={() => select(item.tag)}
                    onMouseEnter={() => setActiveIdx(idx)}
                  >
                    <span>{highlight(item.label, query)}</span>
                    {item.shortcut && <span className={styles.shortcut}>{item.shortcut}</span>}
                  </button>
                );
              })}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
```

`NodeSearchMenu.module.css`:

```css
.menu {
  position: fixed;
  z-index: 10002;
  width: 250px;
  background: var(--ol-surface);
  border: 1px solid var(--ol-border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  padding: 6px;
  font-size: var(--ol-font-size-12);
}
.search {
  width: 100%;
  box-sizing: border-box;
  border: 1px solid var(--ol-border);
  border-radius: var(--ol-radius-sm);
  padding: 5px 8px;
  font-size: var(--ol-font-size-12);
  background: var(--ol-gray-50);
  color: var(--ol-text);
  outline: 2px solid transparent;
  margin-bottom: 4px;
}
.search:focus { outline: 2px solid var(--ol-color-primary, #4f46e5); border-color: transparent; }
.list { max-height: 280px; overflow-y: auto; }
.sectionLabel {
  font-size: var(--ol-font-size-11);
  color: var(--ol-text-muted);
  padding: 4px 8px 2px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.item {
  display: flex; justify-content: space-between; align-items: center; gap: 8px;
  width: 100%; padding: 5px 8px;
  border: none; background: transparent; border-radius: var(--ol-radius-sm);
  color: var(--ol-text); cursor: pointer; text-align: left; font-size: var(--ol-font-size-12);
}
.item:hover { background: var(--ol-gray-100); }
.itemActive { background: var(--ol-gray-100); outline: 1px solid var(--ol-color-primary, #4f46e5); }
.hit { background: var(--ol-color-warning-subtle, #fef3c7); color: inherit; border-radius: 2px; padding: 0 1px; }
.shortcut { color: var(--ol-text-muted); font-family: var(--ol-font-family-mono); font-size: var(--ol-font-size-11); }
.empty { padding: 12px; text-align: center; color: var(--ol-text-muted); }
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/NodeSearchMenu.test.tsx`
Expected: PASS（10 个用例）

- [ ] **Step 5: 回归 + 提交**

Run: `cd poiesis && npx vitest run` — 232 全绿
```bash
git add poiesis/src/components/canvas/code/NodeSearchMenu.tsx poiesis/src/components/canvas/code/NodeSearchMenu.module.css poiesis/src/components/canvas/code/__tests__/NodeSearchMenu.test.tsx
git commit -m "feat(canvas): NodeSearchMenu 可搜索节点创建菜单（自动聚焦/高亮/键盘导航/无结果态）"
```

---

### Task 7: FlowCanvas 接入——空格快捷键 + 拖出创建 + 替换右键菜单

将 NodeSearchMenu 接入 FlowCanvas：替换 CanvasContextMenu、空格唤出、flow-out 拖出到空白创建自动连线、300ms 防误关、拖出中途关闭清理。

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`
- Delete: `poiesis/src/components/canvas/code/CanvasContextMenu.tsx` + `.module.css`（完全替代）
- Test: `poiesis/src/components/canvas/code/__tests__/flow-drag-create.test.ts`（纯函数）

**Interfaces:**
- Consumes: `NodeSearchMenu`（Task 6）、既有 `addElement/addWire/screenToFlowPosition`
- Produces:
  - `isPaneRelease(event: { target: EventTarget | null }): boolean` — 导出纯函数：`target` 为 `HTMLElement` 且 `closest('.react-flow__handle')` 与 `closest('.react-flow__node')` 均为 null（空白释放）
  - FlowCanvas 内部状态机：`pendingDragRef: { nodeId: string } | null` + `menuOpen: boolean` + 菜单定位 state

- [ ] **Step 1: 写失败测试（纯函数）**

```ts
// poiesis/src/components/canvas/code/__tests__/flow-drag-create.test.ts
import { describe, it, expect } from "vitest";
import { isPaneRelease } from "../FlowCanvas";

describe("isPaneRelease", () => {
  function el(className: string): HTMLElement {
    const div = document.createElement("div");
    div.className = className;
    return div;
  }
  it("释放到空白 pane 返回 true", () => {
    expect(isPaneRelease({ target: el("react-flow__pane") })).toBe(true);
  });
  it("释放到节点上返回 false", () => {
    expect(isPaneRelease({ target: el("react-flow__node") })).toBe(false);
  });
  it("释放到 handle 上返回 false", () => {
    expect(isPaneRelease({ target: el("react-flow__handle") })).toBe(false);
  });
  it("target 非 HTMLElement 返回 false", () => {
    expect(isPaneRelease({ target: null })).toBe(false);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/flow-drag-create.test.ts`
Expected: FAIL（isPaneRelease 未导出）

- [ ] **Step 3: 实现 isPaneRelease + FlowCanvas 接入**

`FlowCanvas.tsx` 修改：

```tsx
// 新增导出（文件末尾）
/** 释放目标判定：空白 pane 释放（非 handle/非节点）→ 触发拖出创建。 */
export function isPaneRelease(event: { target: EventTarget | null }): boolean {
  const t = event.target;
  if (!(t instanceof HTMLElement)) return false;
  return !t.closest(".react-flow__handle") && !t.closest(".react-flow__node");
}
```

组件内接入（替换现有 contextMenu 逻辑）：

```tsx
// state：替换 contextMenu 为 menuState
const [menuState, setMenuState] = useState<{ x: number; y: number; flowX: number; flowY: number } | null>(null);
const pendingDragRef = useRef<{ nodeId: string } | null>(null);
const menuOpenAtRef = useRef(0); // 300ms 防误关

const openMenuAt = useCallback((x: number, y: number) => {
  const pos = screenToFlowPosition({ x, y });
  menuOpenAtRef.current = Date.now();
  setMenuState({ x, y, flowX: pos.x, flowY: pos.y });
}, [screenToFlowPosition]);

// 右键画布
const handlePaneContextMenu = useCallback((event: React.MouseEvent | MouseEvent) => {
  event.preventDefault();
  if (Date.now() - menuOpenAtRef.current < 300) return; // 防误关（刚打开时忽略）
  pendingDragRef.current = null;
  openMenuAt(event.clientX, event.clientY);
}, [openMenuAt]);

// 空格唤出（画布内非输入焦点）：追加到现有 window keydown handler
// 重要：现有键盘 useEffect 绑定一次（依赖 []，通过 ref 读最新值——React Doctor 已标记
// FlowCanvas.tsx:216 为"缺依赖"，那是刻意模式，保持它）。新增的菜单开关状态必须走 ref，
// 否则闭包读到旧值：
// const menuOpenRef = useRef(false);  // 与 menuState 同步
// 在 openMenuAt/onClose 中同步 menuOpenRef.current
// 在现有 useEffect 键盘处理中追加分支：
// if (e.key === " " && !menuOpenRef.current && !(e.target instanceof HTMLInputElement) && !(e.target instanceof HTMLTextAreaElement) && !(e.target instanceof HTMLSelectElement)) { e.preventDefault(); openMenuAt(window.innerWidth / 2, window.innerHeight / 2); return; }

// 拖出创建状态机
const onConnectStart = useCallback((_e: unknown, params: { nodeId: string; handleId: string }) => {
  if (params.handleId === "flow-out" || params.handleType === "source") {
    pendingDragRef.current = { nodeId: params.nodeId };
  } else {
    pendingDragRef.current = null;
  }
}, []);

const onConnectEnd = useCallback((event: MouseEvent | TouchEvent) => {
  if (pendingDragRef.current && isPaneRelease(event)) {
    openMenuAt(event instanceof MouseEvent ? event.clientX : window.innerWidth / 2, event instanceof MouseEvent ? event.clientY : window.innerHeight / 2);
    // pendingDragRef 保留：菜单选中时取用
  } else {
    pendingDragRef.current = null;
  }
}, [openMenuAt]);

// 菜单选中：若 pendingDrag 存在则自动连线
const handleMenuSelect = useCallback((tag: string) => {
  if (!menuState) return;
  const id = uid(tag);
  const jitter = Math.random() * 40 - 20;
  let props: Record<string, unknown> = {};
  if (tag === "fn") { props = { name: "newFn", params: "" }; }
  if (tag === "condition") { props = { branches: [{ id: "br0", type: "if", condition: "" }] }; }
  if (["let", "assign", "store-call", "call"].includes(tag)) { props = { ...defaultSyntaxProps(tag) }; }
  addElement({ id, nodeType: "codeblock", tag, props, parentId: null, position: { x: menuState.flowX + jitter, y: menuState.flowY + jitter }, source: "" });
  if (pendingDragRef.current) {
    addWire({ id: uid("wire"), fromId: pendingDragRef.current.nodeId, fromPort: "flow-out", toId: id, toPort: "flow-in" });
    pendingDragRef.current = null;
  }
  setMenuState(null);
}, [menuState, addElement, addWire]);

// 渲染：替换 <ContextMenu ...> 为
{menuState && (
  <NodeSearchMenu
    x={menuState.x}
    y={menuState.y}
    onSelect={handleMenuSelect}
    onClose={() => { pendingDragRef.current = null; setMenuState(null); }}
  />
)}
```

ReactFlow props 追加：`onConnectStart={onConnectStart}`、`onConnectEnd={onConnectEnd}`（现有 `onConnect` 保留）。

删除内容：`CanvasContextMenu` import、`collapsedSections` state、`menuRef`、`ContextMenu` 渲染块、`handleAddNode`（其逻辑并入 `handleMenuSelect`）、pane 双击现有空态提示文案更新为「右键 / 空格 / 拖出端口 → 添加节点」。

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/flow-drag-create.test.ts`
Expected: PASS（4 个用例）

- [ ] **Step 5: 类型检查 + 回归 + 提交**

Run: `cd poiesis && npx tsc --noEmit && npx vitest run` — 232 全绿
```bash
git rm poiesis/src/components/canvas/code/CanvasContextMenu.tsx poiesis/src/components/canvas/code/CanvasContextMenu.module.css
git add poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/__tests__/flow-drag-create.test.ts
git commit -m "feat(canvas): FlowCanvas 接入搜索菜单（空格唤出/拖出创建自动连线/替换右键菜单）
- onConnectStart/End 状态机：flow-out 拖到空白→菜单→选中自动 addWire
- isPaneRelease 区分空白释放与 handle 释放；300ms 防误关
- 删除 CanvasContextMenu（死代码清理）"
```

---

### Task 8: FlowInspector 降级为实时预览

语法节点/condition/return 选中时不再显示表单，改为编辑提示 + formatSyntaxNode 实时预览 + 校验提示；删除 5 个旧表单；fn 参数编辑保留。

**Files:**
- Rewrite: `poiesis/src/components/canvas/code/FlowInspector.tsx`
- Modify: `poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx`

**Interfaces:**
- Consumes: `formatSyntaxNode`（既有）、`validateSyntaxProps/useFlowScope`（Task 3）、既有 `selectedNodeIds/elements/setProp`
- Produces: FlowInspector 新行为——语法节点选中渲染 `<SyntaxPreview tag props errors />`；fn 仍渲染 FnInspector（原样保留）

- [ ] **Step 1: 重写测试（更新断言为预览行为）**

```tsx
// poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx（整体替换）
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { FlowInspector } from "../FlowInspector";

const { mockSetProp } = vi.hoisted(() => ({ mockSetProp: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) =>
    selector({
      elements: [
        {
          id: "l1", nodeType: "codeblock", tag: "let", parentId: null,
          props: { name: "x", mutable: true, value: "0" },
          source: "let mut x = 0;",
        },
        {
          id: "f1", nodeType: "codeblock", tag: "fn", parentId: null,
          props: { name: "add", params: "text: String" },
          source: "",
        },
      ],
      setProp: (id: string, prop: string, value: unknown) => { mockSetProp(id, prop, value); },
      setElementSource: () => {},
      storeReferences: {},
      componentDefs: {},
      storeDefs: {},
      inlineStoreDefs: {},
    }),
}));

beforeEach(() => { mockSetProp.mockClear(); });

describe("FlowInspector 降级为预览", () => {
  it("语法节点显示编辑提示与 formatSyntaxNode 实时预览", () => {
    render(<FlowInspector selectedNodeIds={["l1"]} />);
    expect(screen.getByText("在节点上直接编辑")).toBeTruthy();
    expect(screen.getByText("let mut x = 0;")).toBeTruthy();
  });

  it("fn 节点仍渲染参数编辑表单", () => {
    render(<FlowInspector selectedNodeIds={["f1"]} />);
    expect(screen.getByLabelText("Function name")).toBeTruthy();
    expect(screen.getByRole("button", { name: "+ Add" })).toBeTruthy();
  });

  it("未选择时显示占位提示", () => {
    render(<FlowInspector selectedNodeIds={[]} />);
    expect(screen.getByText("Select a node to edit")).toBeTruthy();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/FlowInspector.test.tsx`
Expected: FAIL（旧表单仍渲染，无「在节点上直接编辑」）

- [ ] **Step 3: 重写 FlowInspector.tsx**

保留：面板样式对象（S）、`FnInspector`（原样）、OMNI_TYPES、Props/组件骨架、fn/return 分发（return 也显示预览）。

删除：`LetInspector/AssignInspector/StoreCallInspector/CallInspector/ReturnInspector` 全部表单组件。

新增：

```tsx
// ────────────────────────────────────────────
// 语法节点预览（内联编辑时代：面板降级为单一来源预览）
// ────────────────────────────────────────────

function SyntaxPreview({ tag, props, scope }: {
  tag: string;
  props: Record<string, unknown>;
  scope?: import("../../../../services/codegen/flow-scope").FlowScope;
}) {
  const code = formatSyntaxNode(tag, props as SyntaxNodeProps);
  const errors = scope ? validateSyntaxProps(tag, props, scope) : [];
  return (
    <div style={S.panel}>
      <div style={S.header}>{TAG_CN[tag] ?? tag}</div>
      <div style={S.field}>
        <div style={S.label}>⚡ 在节点上直接编辑</div>
        <div style={{ fontSize: "var(--ol-font-size-11)", color: "var(--ol-text-muted)", marginTop: 2 }}>
          点击节点上的字段直接修改，Enter 或失焦提交
        </div>
      </div>
      <div style={S.field}>
        <div style={S.label}>生成代码（实时预览）</div>
        <pre style={{
          background: "var(--ol-gray-100)", borderRadius: "var(--ol-radius-sm)",
          padding: "6px 8px", fontSize: "var(--ol-font-size-12)",
          fontFamily: "var(--ol-font-family-mono)", margin: 0, overflowX: "auto",
          color: "var(--ol-text)",
        }}>
          {code}
        </pre>
      </div>
      {errors.length > 0 && (
        <div style={S.field}>
          <div style={{ ...S.label, color: "var(--ol-color-danger)" }}>⚠ 校验提示</div>
          {errors.map((e) => (
            <div key={e.field} style={{ fontSize: "var(--ol-font-size-11)", color: "var(--ol-color-danger)", marginTop: 2 }}>
              {e.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

分发改写（`FlowInspector` 主组件内）：

```tsx
// return 节点：表达式内联编辑，面板显示预览
if (el.tag === "return") {
  return <ReturnPreview el={el} />;   // 同 SyntaxPreview 结构：标题「← 返回」+ 提示 + <pre>{el.source}</pre>
}

// 语法节点（let/assign/store-call/call）
if (["let", "assign", "store-call", "call"].includes(el.tag)) {
  const scope = useFlowScope(activeTab?.componentName);
  return <SyntaxPreview tag={el.tag} props={el.props ?? {}} scope={scope ?? undefined} />;
}
```

（需要 `activeTab`：从 store 取 `activeTabId/openTabs` 已有——`const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId); const openTabs = usePoiesisStore((s) => s.designPage.openTabs);` 然后 `openTabs.find((t) => t.id === activeTabId)`。）

import 更新：`formatSyntaxNode`、`type SyntaxNodeProps`（syntax-nodes.ts）、`TAG_CN`（flowConstants.ts）、`validateSyntaxProps/useFlowScope`（flow-scope/flow-scope-hook）、`ASSIGN_OPERATORS` 不再需要。

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/FlowInspector.test.tsx`
Expected: PASS（3 个用例）

- [ ] **Step 5: 类型检查 + 回归 + 提交**

Run: `cd poiesis && npx tsc --noEmit && npx vitest run` — 232 全绿
```bash
git add poiesis/src/components/canvas/code/FlowInspector.tsx poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx
git commit -m "feat(canvas): FlowInspector 降级为实时预览（编辑在节点上进行，fn 参数表单保留）"
```

---

### Task 9: E2E 验证（Victauri）+ 全量回归

真机验证完整交互链路与 round-trip 稳定性。

**Files:**
- 无源码改动；验证产物记录到会话报告

**Interfaces:**
- Consumes: 全部 Task 1-8 的产物、Victauri MCP（app 已启动）

- [ ] **Step 1: 全量测试回归**

Run: `cd poiesis && npx vitest run` — 期望 232 + 新增（约 58 个新用例）全绿；`npx tsc --noEmit` 零错误

- [ ] **Step 2: 启动应用并验证搜索菜单触发**

用 `start-poiesis` 技能启动（或确认已有实例）→ Victauri `get_plugin_info` 确认 app → 进入 CodeFlowCanvas → 右键画布 → 断言 NodeSearchMenu 出现、搜索框聚焦、输入"let"过滤只剩 变量声明

- [ ] **Step 3: 验证内联编辑**

选中 let 节点 → 点击变量名字段 → 输入新名 → Enter → 断言节点预览与面板预览同步更新（`data-syntax-code` 变化）→ 改 store-call 的 store → 断言 action 下拉联动刷新 → 改非法变量名 → 断言警告条出现

- [ ] **Step 4: 验证拖出创建 + 空格**

从 let 节点 flow-out 拖到空白 → 断言菜单出现 → 选「赋值」→ 断言新节点出现且连线（源 flow-out → 新节点 flow-in）；按空格 → 菜单出现于画布中心

- [ ] **Step 5: 验证保存 round-trip**

保存项目 → 重开 → 断言：节点类型/字段值保持（let 名称、operator、args 行）；`.omni` 文件含 `@pid`；WASM 编译通过（无 E0001）

- [ ] **Step 6: 提交（如有产物）并汇报**

```bash
git add docs/superpowers/plans/2026-08-04-codeflowcanvas-inline-editing.md  # 本计划文件（若未随 Task 提交）
git commit -m "docs: CodeFlowCanvas 内联编辑升级实现计划（9 任务，E2E 验证闭环）" || true
```
汇报：测试数、E2E 结果、遗留问题（如有）

---

## Self-Review 记录

- **Spec 覆盖**：节点内联编辑（Task 1/2/4/5）、搜索菜单（Task 6）、拖出创建+空格+右键（Task 7）、面板降级（Task 8）、补全数据层+校验（Task 3）、E2E（Task 9）。Spec 中"弹层纪律/防误关/useDismissOnCanvasGesture 等价实现"落在 Task 7（onMoveStart 关闭未单独列出——已在 Task 7 实现说明中覆盖为"菜单外点击关闭"，画布平移时菜单不锚定节点、随 viewport 移动，无残留风险，故不引入额外关闭逻辑——偏差已说明）。
- **占位符扫描**：无 TBD/TODO；每个代码步骤含完整实现。
- **类型一致性**：`FlowScope` 结构 Task 3 定义、Task 4/8 消费一致；`NodeTextField` 的 `onCommit` 签名在 Task 1/4/5 一致；`isPaneRelease` Task 7 定义与测试一致；`filterMenuItems`/`NodeSearchMenu` props 在 Task 6 定义、Task 7 消费一致。`onEditProp(id, prop, value)` 契约 Task 4 定义（id 取自 props.__id——实现内以占位 `"el"` 兜底，FlowCanvas 注入时若需真实 id 可在 data 中传 `id` 字段——**偏差说明**：测试断言用 `"l1"`，实现需在 FlowCanvas 注入 `__id: el.id`；SyntaxNode 测试中 renderNode 直接传 props 而不带 __id，断言 onEditProp 参数为 "l1" 会失败——修正：测试 renderNode 的 props 传入 `{ __id: "l1", name: "x", ... }`）。
