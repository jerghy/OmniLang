# View 属性三态保真 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 Poiesis 设计画布的 view 元素属性保真缺口——`class="x"` 不再丢失、裸布尔属性（disabled 等）可解析可编辑、`prop={expr}` 动态属性往返保真。

**Architecture:** 以「props 值三态约定」为核心（静态字符串 / `"{expr}"` 表达式字符串 / boolean），解析端（parser-view.ts）把三类值按约定写入 store，发射端（source-emitter.ts）按形态还原输出，UI（PropsSection.tsx）用芯片编辑器 + 布尔开关行编辑三态值。零 schema 改动。

**Tech Stack:** TypeScript / Vitest / React 19 / Zustand 5（poiesis 前端）

## Global Constraints

（来自 spec，逐条绑定所有任务）
- **值三态约定**：`props: Record<string, unknown>` 值——静态文本 `"Enter name"`；动态表达式 `"{count}"`（trim 后匹配 `/^\{(.+)\}$/s` 且 `$1` 非空）；`true`=裸布尔属性、`false`=不发射。
- **BOOLEAN_ATTRS 共享常量**：20 个——disabled, required, readonly, checked, selected, hidden, multiple, autofocus, novalidate, async, defer, ismap, itemscope, open, loop, muted, controls, autoplay, autocomplete（emitter 现有 19 个 + `autocomplete`）。
- **class 统一走 className→classRefs 管线**（parser-finalize.ts:58 已有转换），输出形态为 `class: a b`（`class="x"` 与 `class: x` 语义等价）。
- **`id={x}` 动态 id 范围外**（D7）：BRACE 分支对 id 保持旧行为（存裸值，丢花括号）。
- **只改真实路径**：`applyStyles` 是空函数（collect-styles.ts:15），发射只改 `elNode.props` 路径（source-emitter.ts:620-631），**不动** `style.htmlAttrs` 死分支（611-619）。
- **测试即契约**：解析与发射必须同时改同时测；往返闭环测试 = `parseOmniSource` → `designElements` → `generateOmniSource` → 断言源码。
- 现有测试破坏面已确认：`graph-parser-syntax.test.ts:219/228/237` 的 `props.value` 断言是 fn 级（codeblock），不受影响；`bind-value-roundtrip.test.ts` 走 bindings 路径，不受影响。
- 测试命令：`cd poiesis && npx vitest run <file>`；全量 `npx vitest run`。

---

### Task 1: BOOLEAN_ATTRS 共享常量

**Files:**
- Create: `poiesis/src/services/codegen/boolean-attrs.ts`
- Modify: `poiesis/src/services/codegen/source-emitter.ts:476-481`（删除私有静态集合，引用共享常量）
- Test: `poiesis/src/services/codegen/__tests__/boolean-attrs.test.ts`

**Interfaces:**
- Produces: `export const BOOLEAN_ATTRS: ReadonlySet<string>`（20 项，见 Global Constraints）——Task 2 与 Task 6 消费。

- [ ] **Step 1: Write the failing test**

```ts
// poiesis/src/services/codegen/__tests__/boolean-attrs.test.ts
import { describe, it, expect } from "vitest";
import { BOOLEAN_ATTRS } from "../boolean-attrs";

describe("BOOLEAN_ATTRS 共享常量", () => {
  it("包含全部 20 个布尔属性", () => {
    const expected = [
      "disabled", "required", "readonly", "checked", "selected",
      "hidden", "multiple", "autofocus", "novalidate",
      "async", "defer", "ismap", "itemscope", "open",
      "loop", "muted", "controls", "autoplay", "autocomplete",
    ];
    for (const name of expected) expect(BOOLEAN_ATTRS.has(name)).toBe(true);
    expect(BOOLEAN_ATTRS.size).toBe(20);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/boolean-attrs.test.ts`
Expected: FAIL — `Cannot find module '../boolean-attrs'`

- [ ] **Step 3: Implement the shared constant**

```ts
// poiesis/src/services/codegen/boolean-attrs.ts
// HTML 布尔属性全集（source-emitter 原私有集合 + autocomplete）。
// parser-view（裸布尔属性解析）与 source-emitter（发射）共用。
export const BOOLEAN_ATTRS: ReadonlySet<string> = new Set([
  "disabled", "required", "readonly", "checked", "selected",
  "hidden", "multiple", "autofocus", "novalidate",
  "async", "defer", "ismap", "itemscope", "open",
  "loop", "muted", "controls", "autoplay", "autocomplete",
]);
```

- [ ] **Step 4: Switch source-emitter to the shared constant + regression**

Modify `source-emitter.ts:475-481`：

```ts
  /** HTML boolean attributes — emitted as bare attribute name. */
  // (共享常量：boolean-attrs.ts，parser-view 同源)
```
（删除 `private static readonly BOOLEAN_ATTRS = new Set([...])` 的 Set 字面量，改为引用）
```ts
import { BOOLEAN_ATTRS } from "./boolean-attrs";
```
并把 614 行 `SourceEmitter.BOOLEAN_ATTRS.has(attr)` 改为 `BOOLEAN_ATTRS.has(attr)`。

Run: `npx vitest run src/services/codegen/__tests__/bind-value-roundtrip.test.ts src/services/codegen/__tests__/roundtrip.test.ts`
Expected: PASS（发射行为不变）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/boolean-attrs.ts poiesis/src/services/codegen/__tests__/boolean-attrs.test.ts poiesis/src/services/codegen/source-emitter.ts
git commit -m "feat(poiesis): BOOLEAN_ATTRS 共享常量（20 项）——parser/emitter 同源（spec D4）"
```

---

### Task 2: 解析端——BRACE 任意表达式 + 裸布尔属性（D2+D3）

**Files:**
- Modify: `poiesis/src/services/codegen/parser-view.ts:343-451`（IDENTIFIER 属性分支）
- Test: `poiesis/src/services/codegen/__tests__/parser-view-attrs.test.ts`

**Interfaces:**
- Consumes: `BOOLEAN_ATTRS` from `./boolean-attrs`（Task 1）
- Produces: 解析约定——`value={count}` → `props.value = "{count}"`；`<input disabled>` → `props.disabled = true`；`id={x}` → `el.id = "x"`（旧行为，D7）。Task 4 发射端消费此约定。

- [ ] **Step 1: Write the failing tests**

```ts
// poiesis/src/services/codegen/__tests__/parser-view-attrs.test.ts
// view 元素属性解析（spec D2/D3）：{expr} 花括号保真 + 裸布尔属性 + break 修复。
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import type { ViewElementData } from "../../../core/types/element";

const COMP = (body: string) => `import omniview;

component Foo {
    view {
${body}
    }
}`;

function firstEl(source: string): ViewElementData {
  const result = parseOmniSource(source);
  const el = result.designElements?.find((e) => e.nodeType === "view") as ViewElementData | undefined;
  if (!el) throw new Error("no view element parsed");
  return el;
}

describe("view 元素属性解析：{expr} 动态属性（D2）", () => {
  it("value={count} → props.value === \"{count}\"（花括号保真）", () => {
    const el = firstEl(COMP(`        <input value={count} />`));
    expect(el.props.value).toBe("{count}");
  });

  it("value={inputText.to_string()} 任意表达式保真（原解析失败）", () => {
    const el = firstEl(COMP(`        <input value={inputText.to_string()} />`));
    expect(el.props.value).toBe("{inputText.to_string()}");
  });

  it("嵌套花括号表达式（if 表达式）配平收集", () => {
    const el = firstEl(COMP(`        <input placeholder={if x { "a" } else { "b" }} />`));
    expect(el.props.placeholder).toBe('{if x { "a" } else { "b" }}');
  });

  it("checked={agreed} 布尔属性的表达式形态", () => {
    const el = firstEl(COMP(`        <input type="checkbox" checked={agreed} />`));
    expect(el.props.checked).toBe("{agreed}");
  });

  it("id={x} 保持旧行为（D7：动态 id 范围外）", () => {
    const el = firstEl(COMP(`        <input id={dynId} />`));
    expect(el.id).toBe("dynId");
  });
});

describe("view 元素属性解析：裸布尔属性（D3）", () => {
  it("<input disabled> → props.disabled === true", () => {
    const el = firstEl(COMP(`        <input disabled />`));
    expect(el.props.disabled).toBe(true);
  });

  it("break 修复：<input disabled placeholder=\"x\"> 后续属性不丢", () => {
    const el = firstEl(COMP(`        <input disabled placeholder="x" />`));
    expect(el.props.disabled).toBe(true);
    expect(el.props.placeholder).toBe("x");
  });

  it("多个裸布尔：<button disabled required> 全部解析", () => {
    const el = firstEl(COMP(`        <button disabled required />`));
    expect(el.props.disabled).toBe(true);
    expect(el.props.required).toBe(true);
  });

  it("未知裸属性维持容错（不崩溃、属性循环停止）", () => {
    const el = firstEl(COMP(`        <input unknownattr />`));
    expect(el.tag).toBe("input");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/parser-view-attrs.test.ts`
Expected: FAIL——`value={count}` 断言收到 `"count"`（无花括号）、`disabled` 用例收到 undefined。

- [ ] **Step 3: Implement BRACE 重写 + 裸布尔分支**

Modify `parser-view.ts`。先加 import（文件头第 8 行附近）：

```ts
import { BOOLEAN_ATTRS } from "./boolean-attrs";
```

重写 BRACE 分支（现 371-382 行）：

```ts
        if (valTok.kind === "BRACE_OPEN") {
          // {expr} 动态属性值（D2）：配平花括号收集任意表达式，存 "{expr}"。
          // 与事件 handler（315-329）同一收集模式；id 特例保持旧行为（D7）。
          internals.stream.consume(); // {
          let depth = 1;
          let expr = "";
          while (depth > 0 && !internals.stream.isEof()) {
            const t = internals.stream.peek();
            if (t.kind === "BRACE_OPEN") depth++;
            else if (t.kind === "BRACE_CLOSE") depth--;
            if (depth === 0) {
              internals.stream.consume(); // }
              break;
            }
            internals.stream.consume();
            expr += t.text;
          }
          expr = expr.trim();
          if (attrName === "id") {
            result.id = expr;
          } else if (attrName === "class") {
            // 动态 class={expr} 不在本次范围（class 走 className 管线）——跳过
          } else {
            result.attrs[attrName] = expr.length > 0 ? `{${expr}}` : "";
          }
          continue;
        }
```

在 `// There must be EQUALS or COLON after the attribute name` 注释前插入裸布尔分支（现 357-359 行之间）：

```ts
      // 裸布尔属性（D3）：<input disabled> → attrs.disabled = true。
      // 必须 continue——现状 break 会终止整个属性循环，后续属性全部丢失。
      if (next.kind !== "EQUALS" && next.kind !== "COLON") {
        if (BOOLEAN_ATTRS.has(attrName)) {
          result.attrs[attrName] = true;
          continue;
        }
        // 未知裸属性：维持旧容错（break）
        break;
      }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/services/codegen/__tests__/parser-view-attrs.test.ts src/services/codegen/__tests__/parser-view-conditional.test.ts src/services/codegen/__tests__/parser-view-for.test.ts`
Expected: PASS（新增全绿 + 既有 view 解析回归）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/parser-view.ts poiesis/src/services/codegen/__tests__/parser-view-attrs.test.ts
git commit -m "feat(poiesis): 属性解析 {expr} 花括号保真 + 裸布尔属性（D2/D3，修复 break 丢后续属性）"
```

---

### Task 3: 解析端——class 双形式（D5）

**Files:**
- Modify: `poiesis/src/services/codegen/parser-view.ts:384-392`（EQUALS 分支 class）`;:547-602`（parseClassName STRING 分支）
- Test: `poiesis/src/services/codegen/__tests__/parser-view-class.test.ts`

**Interfaces:**
- Consumes: Task 2 的 `parser-view.ts`（同一文件，顺序执行）
- Produces: `class="x"` 与 `class: "a b"` 都进 `attrs.className` → 现有 finalize 管线转 `el.style.classRefs`。Task 7 真机验证消费。

- [ ] **Step 1: Write the failing tests**

```ts
// poiesis/src/services/codegen/__tests__/parser-view-class.test.ts
// class 双形式解析（spec D5）：class="x" 属性形式 + class: "a b" 字符串形式 → classRefs。
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import type { ViewElementData } from "../../../core/types/element";

const COMP = (body: string) => `import omniview;

component Foo {
    view {
${body}
    }
}`;

function firstEl(source: string): ViewElementData {
  const result = parseOmniSource(source);
  const el = result.designElements?.find((e) => e.nodeType === "view") as ViewElementData | undefined;
  if (!el) throw new Error("no view element parsed");
  return el;
}

describe("class 属性解析（D5）", () => {
  it("class=\"app\" → classRefs 包含 app（不再静默丢弃）", () => {
    const el = firstEl(COMP(`        <div class="app" />`));
    expect(el.style?.classRefs).toContain("app");
  });

  it("class=\"a b\" 空白拆分为多个类名", () => {
    const el = firstEl(COMP(`        <div class="a b" />`));
    expect(el.style?.classRefs).toEqual(expect.arrayContaining(["a", "b"]));
  });

  it("class: \"a b\" 字符串形式 → classRefs（原 STRING 分支丢失）", () => {
    const el = firstEl(COMP(`        <div class: "a b" />`));
    expect(el.style?.classRefs).toEqual(expect.arrayContaining(["a", "b"]));
  });

  it("class: LearnDesign.Card 设计类引用回归（IDENTIFIER 路径不变）", () => {
    const el = firstEl(COMP(`        <div class: LearnDesign.Card />`));
    expect(el.style?.classRefs).toContain("LearnDesign.Card");
  });

  it("class=\"x\" 与后续属性共存", () => {
    const el = firstEl(COMP(`        <div class="app" id="main" />`));
    expect(el.style?.classRefs).toContain("app");
    expect(el.id).toBe("main");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/parser-view-class.test.ts`
Expected: FAIL——`class="app"` 用例 classRefs 为空。

- [ ] **Step 3: Implement**

Modify EQUALS 分支的 STRING 处理（现 384-392 行）：

```ts
        if (valTok.kind === "STRING") {
          const strVal = internals.stream.consume().text.slice(1, -1); // strip quotes
          if (attrName === "id") {
            result.id = strVal;
          } else if (attrName === "class") {
            // D5：class="x" 不再静默丢弃——与 class: 形式统一走 className 管线
            result.className = strVal.trim();
          } else {
            result.attrs[attrName] = strVal;
          }
          continue;
        }
```

修改 parseClassName（现 595-598 的 else 分支前插入 STRING 处理）：

```ts
    } else if (t.kind === "STRING") {
      // D5：class: "a b" 字符串形式——拆空白并入类名列表
      internals.stream.consume();
      for (const cls of t.text.slice(1, -1).split(/\s+/)) {
        if (cls) classes.push(cls);
      }
    } else {
      // End of class list — stop at end-of-tag tokens
      break;
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/services/codegen/__tests__/parser-view-class.test.ts src/services/codegen/__tests__/parser-view-attrs.test.ts src/services/codegen/__tests__/roundtrip.test.ts`
Expected: PASS（新增全绿 + 往返回归）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/parser-view.ts poiesis/src/services/codegen/__tests__/parser-view-class.test.ts
git commit -m "feat(poiesis): class 双形式解析——class=\"x\" 与 class: \"a b\" 统一走 className 管线（D5）"
```

---

### Task 4: 发射端——动态属性 + 布尔输出 + 闭环（D6）

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts:620-631`（props 路径属性发射）
- Test: `poiesis/src/services/codegen/__tests__/source-emitter-attrs.test.ts`

**Interfaces:**
- Consumes: Task 1 `BOOLEAN_ATTRS`、Task 2/3 解析约定（`"{expr}"` 字符串 / boolean / className→classRefs）
- Produces: 发射形态——`"{count}"` → `value={count}`；`true` → 裸属性；`false` → 无输出。

- [ ] **Step 1: Write the failing tests**

```ts
// poiesis/src/services/codegen/__tests__/source-emitter-attrs.test.ts
// 发射端属性三态输出（spec D6）：{expr} 动态 / boolean 裸 / 静态字符串。
import { describe, test, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { generateOmniSource } from "../pipeline";
import type { ElementData, CodeTab } from "../../../core/types";

const basicCodeTabs: CodeTab[] = [
  { id: "main", type: "logic", label: "Main", icon: "code", graph: { elements: [], wires: [] }, variables: [] },
];

function emit(elements: ElementData[], componentName = "TestComponent") {
  const result = generateOmniSource({ designElements: elements, designWires: [], codeTabs: basicCodeTabs, componentName });
  const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
  expect(fatalErrors).toHaveLength(0);
  return result.value ?? "";
}

function viewEl(props: Record<string, unknown>, tag = "input"): ElementData {
  return { id: "el-1", nodeType: "view", tag, props, children: [], parentId: null };
}

describe("发射端属性三态（D6）", () => {
  test("{count} 形态 → value={count}", () => {
    const out = emit([viewEl({ type: "text", value: "{count}" })]);
    expect(out).toContain("value={count}");
  });

  test("任意表达式 {fn(x)} 原样输出", () => {
    const out = emit([viewEl({ value: "{inputText.to_string()}" })]);
    expect(out).toContain("value={inputText.to_string()}");
  });

  test("静态字符串 → value=\"Enter name\"（回归）", () => {
    const out = emit([viewEl({ value: "Enter name" })]);
    expect(out).toContain('value="Enter name"');
  });

  test("boolean true → 裸属性 disabled", () => {
    const out = emit([viewEl({ disabled: true })]);
    expect(out).toContain(" disabled");
  });

  test("boolean false → 不发射", () => {
    const out = emit([viewEl({ disabled: false, type: "text" })]);
    expect(out).not.toContain("disabled");
  });

  test("表达式布尔 checked={agreed} 往返", () => {
    const out = emit([viewEl({ type: "checkbox", checked: "{agreed}" })]);
    expect(out).toContain("checked={agreed}");
  });

  test("闭环：<input disabled placeholder=\"x\"> 解析→发射保真", () => {
    const parsed = parseOmniSource(`import omniview;

component Foo {
    view {
        <input disabled placeholder="x" />
    }
}`);
    const out = emit((parsed.designElements ?? []) as ElementData[], "Foo");
    expect(out).toContain("disabled");
    expect(out).toContain('placeholder="x"');
  });

  test("闭环：value={count} 解析→发射保真", () => {
    const parsed = parseOmniSource(`import omniview;

component Foo {
    view {
        <input value={count} />
    }
}`);
    const out = emit((parsed.designElements ?? []) as ElementData[], "Foo");
    expect(out).toContain("value={count}");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/source-emitter-attrs.test.ts`
Expected: FAIL——`value={count}` 被输出为 `value="{count}"`（静态转义）；`false` 输出 `disabled="false"`。

- [ ] **Step 3: Implement（source-emitter.ts:620-631 重写）**

```ts
    } else {
      // Derive HTML attributes from node.props (used when applyStyles is no-op)
      for (const [key, val] of Object.entries(elNode.props)) {
        if (key === "className" || key === "style" || key === "key" || key === "label") continue;
        if (key.startsWith("on")) continue; // event handlers handled below
        if (typeof val === "boolean") {
          if (val) parts.push(` ${key}`);
        } else if (typeof val === "string") {
          // D6：{expr} 形态 → 动态属性输出；否则静态字符串
          const exprMatch = /^\{(.+)\}$/s.exec(val.trim());
          if (exprMatch && exprMatch[1].trim().length > 0) {
            parts.push(` ${key}={${exprMatch[1]}}`);
          } else {
            parts.push(` ${key}="${escapeAttr(val)}"`);
          }
        } else if (typeof val === "number") {
          parts.push(` ${key}="${val}"`);
        }
      }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/services/codegen/__tests__/source-emitter-attrs.test.ts src/services/codegen/__tests__/bind-value-roundtrip.test.ts src/services/codegen/__tests__/conditional-roundtrip.test.ts`
Expected: PASS（新增全绿 + 发射回归）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/source-emitter.ts poiesis/src/services/codegen/__tests__/source-emitter-attrs.test.ts
git commit -m "feat(poiesis): 发射端属性三态输出——{expr} 动态属性 + boolean 裸属性（D6）"
```

---

### Task 5: UI——芯片编辑器推广（D8）

**Files:**
- Modify: `poiesis/src/components/panels/inspector/PropsSection.tsx`
- Test: `poiesis/src/components/panels/inspector/__tests__/PropsSection.test.tsx`（新建）

**Interfaces:**
- Consumes: Task 2 的 `"{expr}"` 值约定（UI 显示对象）
- Produces: `ChipEditor`（LabelEditor 重命名，逻辑不变）+ `ChipPropRow`；relevantProps 全走 chip 行；id 行保持 PropRow。Task 6 在 ChipPropRow 之上加布尔行。

- [ ] **Step 1: Write the failing tests**

UI 测试用 vi.hoisted mock 模式（参考 `components/design/__tests__/ComponentDefPanel.test.tsx:51-103` 的 useSyncExternalStore 订阅模式）：

```tsx
// poiesis/src/components/panels/inspector/__tests__/PropsSection.test.tsx
// PropsSection 芯片编辑器（spec D8）：{expr} 段渲染紫色芯片 + label 回归。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { PropsSection } from "../PropsSection";
import type { ViewElementData } from "../../../../core/types/element";

const { mockSetProp } = vi.hoisted(() => ({ mockSetProp: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (sel: (s: any) => unknown) => sel({ setProp: mockSetProp }),
}));

const el = (props: Record<string, unknown>): ViewElementData => ({
  id: "el-1", nodeType: "view", tag: "input", props, children: [], parentId: null,
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
    render(<PropsSection element={el({ label: "Hello {count}" })} />);
    expect(screen.getByText("{count}")).toBeTruthy();
    expect(screen.getByDisplayValue("Hello ")).toBeTruthy();
  });

  it("编辑芯片后提交 → setProp 收到新值", () => {
    render(<PropsSection element={el({ value: "{count}" })} />);
    fireEvent.click(screen.getByText("{count}"));
    const editInput = screen.getByLabelText("Edit segment");
    fireEvent.change(editInput, { target: { value: "{count + 1}" } });
    fireEvent.keyDown(editInput, { key: "Enter" });
    expect(mockSetProp).toHaveBeenCalledWith("el-1", "value", "{count + 1}");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd poiesis && npx vitest run src/components/panels/inspector/__tests__/PropsSection.test.tsx`
Expected: FAIL——value 属性行当前渲染为普通输入框（`{count}` 是字面文本，getByDisplayValue 命中文本值而非芯片；label 用例因组件缺失而失败）。若 `@testing-library/react` 未装，先 `npm i -D @testing-library/react`（查 package.json devDependencies 确认）。

- [ ] **Step 3: Implement（PropsSection.tsx 重构）**

1. 重命名导出：`LabelEditor` → `ChipEditor`，`LabelPropRow` → `ChipPropRow`（组件体与逻辑逐字不变，props 签名不变：`{ value, onChange, onSlashRequest }`）。
2. 渲染处（247-269 行）——relevantProps 全部改用 ChipPropRow，id 保持 PropRow：

```tsx
        {relevantProps.map((prop) => (
          <ChipPropRow
            key={prop}
            label={prop}
            value={localProps[prop] ?? ""}
            onChange={(v) => handleChange(prop, v)}
            onSlash={(el) => handleSlash(prop, el)}
          />
        ))}
```

3. 删除原 label 特判分支（249-258 行）——ChipPropRow 对 label 行为与原来完全一致（chip 编辑器本就是 label 的编辑器，行为逐字保留）。
4. `buildLocalProps`（78-88 行）：值类型扩展为 `string | boolean`：

```ts
function buildLocalProps(element: ElementData): Record<string, string | boolean> {
  const initial: Record<string, string | boolean> = {};
  if (!("props" in element)) return initial;
  for (const key of KNOWN_PROPS) {
    const val = element.props[key];
    if (val !== undefined) {
      initial[key] = typeof val === "boolean" ? val : String(val);
    }
  }
  return initial;
}
```
（`localProps` state 类型同步改为 `Record<string, string | boolean>`；`handleChange` 的 value 参数类型 `string | boolean`；`String(val)` 只在需要回填输入框时用。）

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/components/panels/inspector/__tests__/PropsSection.test.tsx`
Expected: PASS。然后跑 tsc：`npx tsc --noEmit`（poiesis 目录）——确保 localProps 类型扩散无编译错误。

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/panels/inspector/PropsSection.tsx poiesis/src/components/panels/inspector/__tests__/PropsSection.test.tsx
git commit -m "feat(poiesis): 属性芯片编辑器推广——所有属性行支持 {expr} 段（D8，id 行除外）"
```

---

### Task 6: UI——布尔属性行 + 白名单（D9）

**Files:**
- Modify: `poiesis/src/components/panels/inspector/PropsSection.tsx`（BoolPropRow + getRelevantProps + buildLocalProps 布尔显示）
- Test: `poiesis/src/components/panels/inspector/__tests__/PropsSection.test.tsx`（追加用例）

**Interfaces:**
- Consumes: Task 1 `BOOLEAN_ATTRS`、Task 5 `ChipPropRow`/`localProps` 类型
- Produces: `BoolPropRow` 组件 + getRelevantProps 白名单补布尔属性（D9 前置）。

- [ ] **Step 1: Write the failing tests（追加到 PropsSection.test.tsx）**

```tsx
describe("布尔属性行（D9）", () => {
  it("input 白名单包含 checked/disabled/required/readonly（D9 前置）", () => {
    // 通过渲染断言：<input checked={agreed}> 解析后面板显示 checked 行
    render(<PropsSection element={el({ type: "checkbox", checked: "{agreed}" })} />);
    expect(screen.getByText("checked")).toBeTruthy();
  });

  it("boolean true → 显示开关且勾选", () => {
    render(<PropsSection element={el({ disabled: true })} />);
    const toggle = screen.getByRole("checkbox", { name: /disabled/ });
    expect(toggle.checked).toBe(true);
  });

  it("勾选开关 → setProp 收到 true", () => {
    render(<PropsSection element={el({})} />);
    const toggle = screen.getByRole("checkbox", { name: /disabled/ });
    fireEvent.click(toggle);
    expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", true);
  });

  it("点「表达式」→ 输入 {expr} → setProp 收到字符串", () => {
    render(<PropsSection element={el({ disabled: true })} />);
    fireEvent.click(screen.getByRole("button", { name: /表达式/ }));
    const input = screen.getByLabelText("disabled-expression");
    fireEvent.change(input, { target: { value: "{isLoading}" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", "{isLoading}");
  });

  it("表达式形态显示紫色芯片、开关隐藏", () => {
    render(<PropsSection element={el({ disabled: "{isLoading}" })} />);
    expect(screen.getByText("{isLoading}")).toBeTruthy();
    expect(screen.queryByRole("checkbox", { name: /disabled/ })).toBeNull();
  });

  it("删除表达式芯片 → 回开关态", () => {
    render(<PropsSection element={el({ disabled: "{isLoading}" })} />);
    fireEvent.click(screen.getByTitle("删除"));
    expect(mockSetProp).toHaveBeenCalledWith("el-1", "disabled", true);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd poiesis && npx vitest run src/components/panels/inspector/__tests__/PropsSection.test.tsx`
Expected: FAIL——checked 行不存在（白名单）、无开关控件、无表达式按钮。

- [ ] **Step 3: Implement**

1. **getRelevantProps 白名单补布尔（573-588 行）**：

```ts
function getRelevantProps(tag: string): string[] {
  const map: Record<string, string[]> = {
    input: ["type", "placeholder", "value", "name", "checked", "disabled", "required", "readonly"],
    button: ["label", "type", "name", "disabled"],
    img: ["src", "alt"],
    a: ["href", "label", "target", "disabled"],
    textarea: ["placeholder", "value", "name", "rows", "disabled", "readonly", "required"],
    select: ["name", "disabled", "required"],
    label: ["label"],
    h1: ["label"], h2: ["label"], h3: ["label"], p: ["label"],
  };
  return map[tag] ?? ["label"];
}
```

2. **BoolPropRow 组件**（布尔属性行：开关 ↔ 表达式互斥视图）：

```tsx
function BoolPropRow({
  label, value, onChange, onSlash,
}: {
  label: string;
  value: string | boolean | undefined;
  onChange: (v: boolean | string) => void;
  onSlash?: (inputEl: HTMLInputElement) => void;
}) {
  const isExpr = typeof value === "string";
  const [editingExpr, setEditingExpr] = useState(false);
  const [exprText, setExprText] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const commitExpr = useCallback(() => {
    const text = exprText.trim();
    if (text) onChange(text.startsWith("{") ? text : `{${text}}`);
    else onChange(true); // 空输入 → 回开关态（开）
    setEditingExpr(false);
    setExprText("");
  }, [exprText, onChange]);

  if (isExpr) {
    // 表达式形态：紫色芯片（点击编辑 / × 删除回开关态）
    return (
      <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 12, color: "var(--ol-text-secondary)", minWidth: 50, flexShrink: 0 }}>{label}</span>
        <button type="button" onClick={() => { setExprText(value.slice(1, -1)); setEditingExpr(true); }}
          style={{ ...EXPR_CHIP_STYLE, cursor: "pointer" }} title="点击编辑">
          {value}
          <span data-action="delete-segment" style={CHIP_DELETE_BTN} title="删除"
            onClick={(e) => { e.stopPropagation(); onChange(true); }}>&times;</span>
        </button>
      </label>
    );
  }
  if (editingExpr) {
    return (
      <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 12, color: "var(--ol-text-secondary)", minWidth: 50, flexShrink: 0 }}>{label}</span>
        <input ref={inputRef} type="text" aria-label={`${label}-expression`} value={exprText}
          autoFocus
          onChange={(e) => setExprText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commitExpr();
            else if (e.key === "Escape") setEditingExpr(false);
            else if (e.key === "/" && onSlash) {
              e.preventDefault();
              requestAnimationFrame(() => onSlash(e.currentTarget));
            }
          }}
          onBlur={commitExpr}
          style={{ flex: 1, padding: "3px 6px", fontSize: 12, border: "1px solid var(--ol-color-primary)", borderRadius: 4, fontFamily: "var(--dp-font-mono, monospace)" }} />
      </label>
    );
  }
  // 开关形态
  const isOn = value === true;
  return (
    <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <span style={{ fontSize: 12, color: "var(--ol-text-secondary)", minWidth: 50, flexShrink: 0 }}>{label}</span>
      <input type="checkbox" aria-label={`${label}-toggle`} checked={isOn}
        onChange={(e) => onChange(e.target.checked)} />
      <button type="button" onClick={() => setEditingExpr(true)} style={{ fontSize: 11, color: "var(--ol-text-secondary)", border: "1px solid var(--ol-border)", borderRadius: 4, padding: "1px 6px", background: "transparent", cursor: "pointer" }}>
        表达式
      </button>
    </label>
  );
}
```

3. **渲染处接线**（relevantProps map 内）：

```tsx
        {relevantProps.map((prop) =>
          BOOLEAN_ATTRS.has(prop) ? (
            <BoolPropRow
              key={prop}
              label={prop}
              value={localProps[prop]}
              onChange={(v) => handleChange(prop, v)}
              onSlash={(el) => handleSlash(prop, el)}
            />
          ) : (
            <ChipPropRow
              key={prop}
              label={prop}
              value={localProps[prop] ?? ""}
              onChange={(v) => handleChange(prop, v)}
              onSlash={(el) => handleSlash(prop, el)}
            />
          )
        )}
```

4. **handleChange 类型**（145-151 行）：`(propName: string, value: string | boolean)`——boolean 直存（不 String() 化）；`debouncedSetProp` 的 value 参数类型同步 `string | boolean`。布尔行的「表达式」按钮插入的信号（`/` 插入在 BoolPropRow 的编辑输入框内生效——通过 onSlash 复用 SignalSlashPicker；插入回调需要把文本写入 exprText：给 BoolPropRow 加 `onSlashInsert` 回调或复用 PropsSection 的 handleSlashSelect——计划实现时优先复用现有 handleSlashSelect 的「labelInsert 回调」机制，在 BoolPropRow 挂 `labelSlashCallbacks` 类似回调，把 `{sig}` 拼进 exprText）。

> 注意：`/` 插入到布尔表达式输入框的接线（SignalSlashPicker 复用）——现有 `labelSlashCallbacks` WeakMap 机制（PropsSection.tsx:16,180-186）可直接复用：BoolPropRow 的输入框注册回调（把信号名拼入 exprText 并提交），onSlash 触发父级 handleSlash。

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/components/panels/inspector/__tests__/PropsSection.test.tsx && npx tsc --noEmit`
Expected: PASS + tsc 零错误

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/panels/inspector/PropsSection.tsx poiesis/src/components/panels/inspector/__tests__/PropsSection.test.tsx
git commit -m "feat(poiesis): 布尔属性行——开关/表达式互斥视图 + getRelevantProps 白名单补全（D9）"
```

---

### Task 7: 全量回归 + 真机验证

**Files:**
- 无新代码（验证任务；如回归暴露问题则按 fix loop 处理）

**Interfaces:**
- Consumes: Task 1-6 全部产物

- [ ] **Step 1: 全量测试 + 类型 + lint**

Run: `cd poiesis && npx vitest run && npx tsc --noEmit`
Expected: 全绿（既有 606+ 测试 + 本次新增约 30 用例）

- [ ] **Step 2: 真机验证——learn-omniview 三个文件打开→保存无损**

启动 Poiesis（start-poiesis skill 或已有实例），用 Victauri 或手动操作：
1. 打开 `test/projects/learn-omniview/src/simple.omni`（`class="app"` 唯一使用点）→ 保存 → 源文件含 `class: app`（等价改写）且元素不丢
2. 打开 `src/components/FormPage.omni` → 保存 → `disabled`/`readonly` 属性保留、`disabled={false}` 保留
3. 打开 `src/components/FormControlsDemo.omni` → 保存 → `checked={agreed}` 保留
4. 属性面板交互：选中 input → value 输 `{count}` → 紫色芯片出现 → 保存 → 源码 `value={count}`

Expected: 三项文件的 diff 只含格式重排与 PID 注释（spec 验收标准 1）

- [ ] **Step 3: 验收标准逐条核对（spec 末尾）**

1. learn-omniview 62 个 view 文件：无 `value="sig"` 退化、无布尔丢失、无 class 丢失 ✓/✗
2. 解析/发射往返测试全绿 ✓/✗
3. 属性面板：任意属性可编辑动态表达式（芯片 + `/` 插入）、布尔属性开关/表达式双形态 ✓/✗

- [ ] **Step 4: 提交验证期间的修复（如有）并汇报**

```bash
git add -A poiesis/src
git commit -m "fix(poiesis): 真机验证修复——view 属性三态保真（如有）"
```

---

## Self-Review（已执行）

**Spec 覆盖**：
- D1 值三态约定 → Global Constraints + Task 2/4 测试矩阵 ✓
- D2 BRACE 任意表达式 → Task 2 ✓
- D3 裸布尔 + break 修复 → Task 2 ✓
- D4 BOOLEAN_ATTRS 共享 → Task 1 ✓
- D5 class 双形式 → Task 3 ✓
- D6 发射动态属性 → Task 4 ✓
- D7 id 边界 → Task 2（锁定旧行为测试）✓
- D8 芯片推广 → Task 5 ✓
- D9 布尔行 + 白名单前置 → Task 6 ✓
- D10 范围外 → 无任务（明确不做）✓
- spec 测试矩阵 9 行 → Task 2 5 行 + Task 3 5 行 + Task 4 8 行 + 闭环 2 行 ✓
- 真机验证 4 项 → Task 7 ✓

**占位符扫描**：无 TBD/TODO；所有步骤含真实代码。

**类型一致性**：`BOOLEAN_ATTRS`（ReadonlySet<string>）、`"{expr}"` 字符串约定、`bool` 属性名、`BoolPropRow`/`ChipPropRow`/`ChipEditor` 命名在 Task 间一致；`handleChange(propName, value: string | boolean)` 在 Task 5/6 间一致。
