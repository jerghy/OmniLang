# CodeFlow 公式框重新设计实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 CodeFlow 公式框从 contentEditable 自动分词架构重写为 textarea + 覆盖层胶囊渲染 + 显式斜杠/运算符触发胶囊 + 字面量判定，根治光标跳开头类 bug。

**Architecture:** textarea 是唯一编辑层（光标/IME/选区浏览器原生）；胶囊以私有区字符标记段 `\uE000<text>\uE001` 存于 textarea，覆盖层把标记段渲染为高亮胶囊块并给结构字符着色；提交时逐段拼接（胶囊/结构字符原样、自由文本按字面量判定）；重开时反向结构化。纯函数核心（tokenize/判定/拼接/反向结构化）集中在 `expression-trigger.ts`，组件只做渲染与交互。

**Tech Stack:** React 19 + TypeScript + vitest + @testing-library/react（jsdom）；`NodeCandidateMenu`/`buildFormulaSource`/`OPERATOR_PREFIX_CANDIDATES` 复用，编译器与 `.omni` 格式零改动。

## Global Constraints

- 提交仍是字符串：`onCommit(v: string)` 契约不变；`.omni` 持久化格式、编译器零改动（ADR-059）
- 运算符集合以编译器 `infix.rs` 为权威，UI 不自行定义；`=` 只提供 `==`
- 候选数据零新数据源（只消费既有 `buildFormulaSource`/`buildExpressionCandidates`）
- 胶囊只属于语法元素（引用/运算符）；结构字符 `( ) [ ] { } , : .` 非胶囊、提交原样
- 自由文本 = 字面量：纯数字→number、`true`/`false`→bool、`null`→null、其余→string（显示无引号，提交 `"..."` 加引号转义）；`.` 后的自由文本段（member 段）原样
- 所有代码步骤后运行 `npx vitest run <路径>` 与提交，遵循 TDD（先写测试验证失败再实现）
- 工作目录：`D:\space\OmniLang\poiesis`（所有命令在此目录下运行）

---

## 文件结构

| 文件 | 职责 |
|---|---|
| `src/components/canvas/code/fields/expression-trigger.ts`（重写） | 纯函数核心：段模型/解析、字面量判定、提交拼接、反向结构化、边界判定、运算符候选表 |
| `src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`（重写） | 上述纯函数的单测 + round-trip 双向语料 |
| `src/components/canvas/code/fields/NodeFormulaField.tsx`（重写） | textarea + 覆盖层渲染 + 弹框/胶囊/提交交互（组件层，无业务规则） |
| `src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`（重写） | 组件交互测试 |
| `src/components/canvas/code/fields/NodeFormulaField.module.css`（修改） | 胶囊块/结构字符着色/透明文本层样式 |

**不动**：`NodeCandidateMenu.tsx`（复用）、`flow-scope.ts`（复用）、`SyntaxNode.tsx`/`ConditionNode.tsx`/`ReturnNode.tsx`（接口不变，零改动）、`.omni`、编译器。

**组件对外接口保持不变**（三使用点零改动）：
```ts
interface Props {
  value: string;                       // omni 表达式文本
  onCommit: (v: string) => void;
  candidates?: FormulaCandidateSource; // { filter(q): CandidateItem[]; storeSignals(storeName): CandidateItem[] }
  placeholder?: string;
  ariaLabel?: string;
  className?: string;
  invalid?: boolean;
  onCancel?: () => void;
  onBlur?: () => void;
}
```

---

### Task 1: 段模型与解析（parseMarkup + 结构字符集）

**Files:**
- Modify: `src/components/canvas/code/fields/expression-trigger.ts`
- Test: `src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`

**Interfaces:**
- Produces: `CAPSULE_START`/`CAPSULE_END` 常量、`STRUCT_CHARS` 集合、`Segment` 类型、`parseMarkup(markup: string): Segment[]`

- [ ] **Step 1: 写失败测试**

在 `expression-trigger.test.ts` 顶部追加：

```ts
import { parseMarkup, CAPSULE_START, CAPSULE_END } from "../expression-trigger";

describe("parseMarkup", () => {
  it("解析胶囊标记段", () => {
    const segs = parseMarkup(`${CAPSULE_START}count${CAPSULE_END} + 1`);
    expect(segs).toEqual([
      { kind: "capsule", text: "count", start: 0, end: CAPSULE_START.length + 5 + CAPSULE_END.length },
      { kind: "text", text: " + 1", start: segs[0]!.end, end: segs[0]!.end + 4 },
    ]);
  });
  it("结构字符独立成段（连续合并）", () => {
    const segs = parseMarkup(`a(1, 2)`);
    expect(segs.map((s) => [s.kind, s.text])).toEqual([
      ["text", "a"],
      ["struct", "("],
      ["text", "1"],
      ["struct", ","],
      ["text", " 2"],
      ["struct", ")"],
    ]);
  });
  it("结构字符集覆盖 ()[]{}.,:", () => {
    const segs = parseMarkup(`[10].{x:2}`);
    expect(segs.filter((s) => s.kind === "struct").map((s) => s.text).join("")).toBe("[].{x:2}");
  });
  it("未闭合标记容错：截断到末尾", () => {
    const segs = parseMarkup(`${CAPSULE_START}abc`);
    expect(segs[0]).toEqual({ kind: "capsule", text: "abc", start: 0, end: 3 + CAPSULE_START.length });
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: FAIL（`parseMarkup` 未定义）

- [ ] **Step 3: 实现 parseMarkup**

在 `expression-trigger.ts` 顶部（保留现有 `OPERATOR_PREFIX_CANDIDATES`、`tokenizeFormula`、`countUnbalancedParens` 等既有导出不动，后续任务再改）追加：

```ts
/** 胶囊标记：textarea 中胶囊段用私有区字符包裹（React Mentions 同款）。 */
export const CAPSULE_START = "\uE000";
export const CAPSULE_END = "\uE001";

/** 结构字符：非胶囊的表达式标点，着色渲染、提交原样。 */
export const STRUCT_CHARS = new Set(["(", ")", "[", "]", "{", "}", ",", ":", "."]);

export interface Segment {
  kind: "capsule" | "struct" | "text";
  /** 段内容（capsule 不含标记字符） */
  text: string;
  /** 在 markup 中的起止偏移（覆盖层定位/光标定位用） */
  start: number;
  end: number;
}

/** 标记文本 → 段序列：胶囊标记段 / 连续结构字符段 / 连续普通文本段。 */
export function parseMarkup(markup: string): Segment[] {
  const segments: Segment[] = [];
  let i = 0;
  while (i < markup.length) {
    if (markup[i] === CAPSULE_START) {
      const end = markup.indexOf(CAPSULE_END, i + 1);
      const endPos = end === -1 ? markup.length : end + 1;
      segments.push({ kind: "capsule", text: markup.slice(i + 1, end === -1 ? endPos : end), start: i, end: endPos });
      i = endPos;
      continue;
    }
    if (STRUCT_CHARS.has(markup[i]!)) {
      let j = i;
      while (j < markup.length && STRUCT_CHARS.has(markup[j]!)) j++;
      segments.push({ kind: "struct", text: markup.slice(i, j), start: i, end: j });
      i = j;
      continue;
    }
    let j = i;
    while (j < markup.length && markup[j] !== CAPSULE_START && !STRUCT_CHARS.has(markup[j]!)) j++;
    segments.push({ kind: "text", text: markup.slice(i, j), start: i, end: j });
    i = j;
  }
  return segments;
}
```

- [ ] **Step 4: 运行验证通过**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: PASS（既有测试 + 新 4 条）

- [ ] **Step 5: 提交**

```bash
git add src/components/canvas/code/fields/expression-trigger.ts src/components/canvas/code/fields/__tests__/expression-trigger.test.ts
git commit -m "feat(codeflow): 公式框段模型——胶囊标记/结构字符/自由文本三段解析"
```

---

### Task 2: 字面量判定 + 提交拼接（classifyLiteral + serializeSegments）

**Files:**
- Modify: `src/components/canvas/code/fields/expression-trigger.ts`
- Test: `src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`

**Interfaces:**
- Consumes: `parseMarkup`/`Segment`（Task 1）
- Produces: `escapeString`/`unescapeString`、`LiteralClass`、`classifyLiteral(text, prevStructDot)`、`serializeSegments(markup): string`

- [ ] **Step 1: 写失败测试**

```ts
describe("serializeSegments 字面量判定与拼接", () => {
  const c = (s: string) => `${CAPSULE_START}${s}${CAPSULE_END}`;
  it("纯数字 → number 原样", () => {
    expect(serializeSegments(`${c("count")} + 1`)).toBe("count + 1");
  });
  it("带小数点数字原样（点属结构字符，拼接一致）", () => {
    expect(serializeSegments(`1.5`)).toBe("1.5");
  });
  it("true/false/null 原样", () => {
    expect(serializeSegments(`true`)).toBe("true");
    expect(serializeSegments(`false`)).toBe("false");
    expect(serializeSegments(`null`)).toBe("null");
  });
  it("普通文本 → 字符串加双引号（trim + 转义）", () => {
    expect(serializeSegments(`hello world`)).toBe('"hello world"');
    expect(serializeSegments(`user-settings`)).toBe('"user-settings"');
  });
  it("字符串内嵌引号转义", () => {
    expect(serializeSegments(`say "hi"`)).toBe('"say \\"hi\\""');
  });
  it("空白段原样保留（胶囊间距不变）", () => {
    expect(serializeSegments(`${c("count")} + ${c("count")}`)).toBe("count + count");
  });
  it("点号后的自由文本段 → member 原样（不判字符串）", () => {
    expect(serializeSegments(`${c("sizeBox")}.measure()`)).toBe("sizeBox.measure()");
    expect(serializeSegments(`${c("mirrorInput")}.value`)).toBe("mirrorInput.value");
  });
  it("member 链含参数与数字", () => {
    expect(serializeSegments(`${c("items")}.get(${c("items")}.len() - 1)`)).toBe("items.get(items.len() - 1)");
  });
  it("胶囊原样、结构字符原样", () => {
    expect(serializeSegments(`${c("+")}${c("count")}`)).toBe("+count");
  });
  it("escapeString/unescapeString 互逆", () => {
    expect(unescapeString(escapeString(`a\\b"c`))).toBe(`a\\b"c`);
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: FAIL（`serializeSegments` 等未定义）

- [ ] **Step 3: 实现**

```ts
/** 自由文本段提交分类：空白原样 / number / bool / null / member（`.` 后）原样 / string 加引号。 */
export type LiteralClass = "blank" | "number" | "bool" | "null" | "string" | "member";

export function escapeString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

export function unescapeString(s: string): string {
  return s.replace(/\\"/g, '"').replace(/\\\\/g, "\\");
}

/** 字面量判定：trim 后为空 → blank；前段以 `.` 结尾 → member；纯数字 → number；true/false/null 原样；其余 string。 */
export function classifyLiteral(text: string, prevStructDot: boolean): LiteralClass {
  const t = text.trim();
  if (t === "") return "blank";
  if (prevStructDot) return "member";
  if (/^\d+(\.\d+)?$/.test(t)) return "number";
  if (t === "true" || t === "false") return "bool";
  if (t === "null") return "null";
  return "string";
}

/** 标记文本 → omni 表达式（提交拼接）：胶囊/结构字符原样，自由文本按判定转换。 */
export function serializeSegments(markup: string): string {
  const segs = parseMarkup(markup);
  let out = "";
  for (let k = 0; k < segs.length; k++) {
    const s = segs[k]!;
    if (s.kind === "capsule" || s.kind === "struct") { out += s.text; continue; }
    const prev = k > 0 ? segs[k - 1]! : null;
    const cls = classifyLiteral(s.text, prev?.kind === "struct" && prev.text.endsWith("."));
    if (cls === "string") out += `"${escapeString(s.text.trim())}"`;
    else out += s.text;
  }
  return out;
}
```

- [ ] **Step 4: 运行验证通过**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add src/components/canvas/code/fields/expression-trigger.ts src/components/canvas/code/fields/__tests__/expression-trigger.test.ts
git commit -m "feat(codeflow): 公式框字面量判定 + 提交拼接（member 段/字符串转义/空白保留）"
```

---

### Task 3: 反向结构化（toMarkup + round-trip 双向）

**Files:**
- Modify: `src/components/canvas/code/fields/expression-trigger.ts`
- Test: `src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`

**Interfaces:**
- Consumes: `CAPSULE_START`/`CAPSULE_END`、`unescapeString`（Task 1/2）、现有 `tokenizeFormula`
- Produces: `toMarkup(omniText: string): string`；`tokenizeFormula` 的 paren 分支扩展为结构字符（`[]{}.,:` 归 `paren` kind）

- [ ] **Step 1: 写失败测试**

```ts
describe("toMarkup 反向结构化 + round-trip", () => {
  const c = (s: string) => `${CAPSULE_START}${s}${CAPSULE_END}`;
  it("标识符/运算符 → 胶囊；数字/空白原样", () => {
    expect(toMarkup("count + 1")).toBe(`${c("count")} + ${c("+")} 1`);
  });
  it("字符串剥引号反转义", () => {
    expect(toMarkup('"hello"')).toBe("hello");
    expect(toMarkup('"say \\"hi\\""')).toBe(`say "hi"`);
  });
  it("member token 拆分三段（引用胶囊+点+member 段）", () => {
    expect(toMarkup("mirrorInput.value")).toBe(`${c("mirrorInput")}.value`);
    expect(toMarkup("Store.signal")).toBe(`${c("Store")}.signal`);
  });
  it("方法链：member 拆段、.height 为点+胶囊（height 是独立 identifier token）", () => {
    expect(toMarkup("sizeBox.measure().height")).toBe(`${c("sizeBox")}.measure().${c("height")}`);
  });
  it("结构字符原样（括号/逗号/方括号/花括号/冒号）；结构体 key 是标识符 → 胶囊", () => {
    expect(toMarkup("foo(a, b)")).toBe(`${c("foo")}(${c("a")}, ${c("b")})`);
    expect(toMarkup("[10, 20]")).toBe(`[10, 20]`);
    expect(toMarkup("Point { x: 0 }")).toBe(`${c("Point")} { ${c("x")}: 0 }`);
  });
  it("is 运算符成胶囊", () => {
    expect(toMarkup("x is String")).toBe(`${c("x")} ${c("is")} ${c("String")}`);
  });
  it("round-trip 双向一致：serialize(toMarkup(t)) === t", () => {
    const cases = ["count + 1", "mirrorInput.value", '"hello world"', "user-settings", "items.get(items.len() - 1)", "sizeBox.measure().height", "Store.signal", "total > 0 && total < 10", 'say "hi" + name'];
    for (const t of cases) expect(serializeSegments(toMarkup(t))).toBe(t);
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: FAIL（`toMarkup` 未定义）

- [ ] **Step 3: 扩展 tokenizeFormula 的结构字符识别**

把 `expression-trigger.ts` 中 tokenizeFormula 的括号分支（`if (ch === "(" || ch === ")")`）替换为：

```ts
    // 结构字符（括号/逗号/方括号/花括号/冒号/点）：原样保留、不参与字面量判定
    if (/[()[\],:.{}]/.test(ch)) {
      tokens.push({ kind: "paren", text: ch, start: i, end: i + 1 });
      i++;
      continue;
    }
```

**注意**：删除原分支 `if (ch === "(" || ch === ")")`。这样 `[` `]` `{` `}` `,` `:` 与独立 `.` 从 unknown 归入 paren，`toMarkup` 原样输出；`1.5` 中 `.` 变 paren 段——`tokenizeFormula("1.5")` 产出 number(`1`)+paren(`.`)+number(`5`)，`toMarkup` 拼接 `1.5` 原样一致（验证 round-trip 语料含 `1.5` 时成立）。

- [ ] **Step 4: 实现 toMarkup**

```ts
/** omni 表达式 → 标记文本（重开反向结构化）：
 *  identifier/operator → 胶囊；member token 按 spec D5 拆三段（首段胶囊 + "." + member 段，
 *  可点击重选 store、逐字编辑 member）；string 剥引号；number/struct/空白原样。 */
export function toMarkup(omniText: string): string {
  const tokens = tokenizeFormula(omniText);
  let out = "";
  let pos = 0;
  for (const t of tokens) {
    if (t.start > pos) out += omniText.slice(pos, t.start);
    if (t.kind === "member") {
      const [head, ...rest] = t.text.split(".");
      out += CAPSULE_START + head + CAPSULE_END;
      if (rest.length > 0) out += "." + rest.join(".");
    } else if (t.kind === "identifier" || t.kind === "operator") {
      out += CAPSULE_START + t.text + CAPSULE_END;
    } else if (t.kind === "string") {
      out += unescapeString(t.text);
    } else {
      out += t.text;
    }
    pos = t.end;
  }
  if (pos < omniText.length) out += omniText.slice(pos);
  return out;
}
```

**注意**：member 拆段后提交拼接自动对齐——`Store.signal` 中 `signal` 前段是 struct `.`，`classifyLiteral` 判 member 原样输出 → `Store.signal`，round-trip 双向一致。

- [ ] **Step 5: 运行验证通过**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: PASS（含 round-trip 双向语料）

- [ ] **Step 6: 提交**

```bash
git add src/components/canvas/code/fields/expression-trigger.ts src/components/canvas/code/fields/__tests__/expression-trigger.test.ts
git commit -m "feat(codeflow): 公式框反向结构化 toMarkup——round-trip 双向一致（含 member/结构字符/转义语料）"
```

---

### Task 4: 边界判定 + 运算符触发（isBoundary + 候选表增补）

**Files:**
- Modify: `src/components/canvas/code/fields/expression-trigger.ts`
- Test: `src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`

**Interfaces:**
- Consumes: `CAPSULE_END`、`STRUCT_CHARS`（Task 1）
- Produces: `TRIGGER_CHARS`、`isBoundary(markup, offset): boolean`；`OPERATOR_PREFIX_CANDIDATES` 增补单字符运算符（`+ - * / %` 弹自身）

- [ ] **Step 1: 写失败测试**

```ts
describe("isBoundary 边界触发", () => {
  const c = (s: string) => `${CAPSULE_START}${s}${CAPSULE_END}`;
  it("开头/空白后/胶囊后/结构字符后 → 边界", () => {
    expect(isBoundary("", 0)).toBe(true);
    expect(isBoundary("count ", 6)).toBe(true);
    expect(isBoundary(`${c("count")}/`, c("count").length)).toBe(true); // slash 前是胶囊标记尾
    expect(isBoundary("(", 1)).toBe(true);
  });
  it("词中间（字母数字后）→ 非边界（字符串内容）", () => {
    expect(isBoundary("user", 4)).toBe(false);
    expect(isBoundary("a/b", 2)).toBe(false);
  });
});
describe("OPERATOR_PREFIX_CANDIDATES 增补", () => {
  it("单字符运算符弹自身", () => {
    expect(OPERATOR_PREFIX_CANDIDATES["+"]).toEqual(["+"]);
    expect(OPERATOR_PREFIX_CANDIDATES["-"]).toEqual(["-"]);
    expect(OPERATOR_PREFIX_CANDIDATES["/"]).toEqual(["/"]);
    expect(OPERATOR_PREFIX_CANDIDATES["%"]).toEqual(["%"]);
  });
  it("多字符前缀不变（infix.rs 权威）", () => {
    expect(OPERATOR_PREFIX_CANDIDATES["<"]).toEqual(["<", "<="]);
    expect(OPERATOR_PREFIX_CANDIDATES["="]).toEqual(["=="]);
    expect(OPERATOR_PREFIX_CANDIDATES["?"]).toEqual(["??", "?."]);
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: FAIL（`isBoundary` 未定义、单字符候选缺失）

- [ ] **Step 3: 实现**

修改 `OPERATOR_PREFIX_CANDIDATES` 为（增补单字符，其余不变）：

```ts
export const OPERATOR_PREFIX_CANDIDATES: Record<string, string[]> = {
  "+": ["+"],
  "-": ["-"],
  "*": ["*"],
  "/": ["/"],
  "%": ["%"],
  "<": ["<", "<="],
  ">": [">", ">="],
  "=": ["=="],
  "!": ["!=", "!"],
  "&": ["&&"],
  "|": ["||"],
  "?": ["??", "?."],
};
```

追加：

```ts
/** 触发弹框的字符：斜杠（胶囊入口）+ 运算符字符。 */
export const TRIGGER_CHARS = new Set(["+", "-", "*", "/", "%", "<", "=", "!", "&", "|", "?"]);

/** 边界判定：offset 前字符为空/空白/胶囊标记尾/结构字符 → 触发弹框；词中间不触发。 */
export function isBoundary(markup: string, offset: number): boolean {
  const prev = markup[offset - 1];
  if (prev === undefined || /\s/.test(prev)) return true;
  if (prev === CAPSULE_END) return true;
  if (STRUCT_CHARS.has(prev)) return true;
  return false;
}
```

- [ ] **Step 4: 运行验证通过**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add src/components/canvas/code/fields/expression-trigger.ts src/components/canvas/code/fields/__tests__/expression-trigger.test.ts
git commit -m "feat(codeflow): 公式框边界触发判定 + 单字符运算符候选增补"
```

---

### Task 5: NodeFormulaField 重写（textarea + 覆盖层 + 交互）

**Files:**
- Rewrite: `src/components/canvas/code/fields/NodeFormulaField.tsx`
- Modify: `src/components/canvas/code/fields/NodeFormulaField.module.css`
- Rewrite: `src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`

**Interfaces:**
- Consumes: `parseMarkup`/`serializeSegments`/`toMarkup`/`isBoundary`/`TRIGGER_CHARS`/`operatorCandidates`/`OPERATOR_PREFIX_CANDIDATES`（Task 1-4）、`NodeCandidateMenu`/`CandidateItem`（既有）、`FormulaCandidateSource`（既有）
- Produces: 与旧版完全相同的组件 Props（见文件结构表），三使用点零改动

- [ ] **Step 1: 写失败测试（组件交互契约）**

重写 `NodeFormulaField.test.tsx`：

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { NodeFormulaField, type FormulaCandidateSource } from "../NodeFormulaField";

/** 模拟 buildFormulaSource 的 filter 语义（空 query 全量、其余 startsWith 匹配）——
 *  逃生口测试依赖"无匹配 → 空候选"。 */
const source: FormulaCandidateSource = {
  filter: (q) => (q === "" || q.startsWith("count") ? [{ value: "count", label: "count" }] : []),
  storeSignals: () => [],
};

const el = (w: HTMLElement) => w.querySelector("textarea")!;

describe("NodeFormulaField", () => {
  it("渲染：胶囊块 + 结构字符着色 + 透明文本层", () => {
    const { container } = render(<NodeFormulaField value="count + 1" onCommit={() => {}} candidates={source} />);
    expect(container.querySelectorAll("[data-seg-kind='capsule']")).toHaveLength(2);
    expect(container.querySelectorAll("[data-seg-kind='struct']")).toHaveLength(0);
    expect(el(container).value).toBe("\uE000count\uE001 \uE000+\uE001 1");
  });

  it("斜杠边界键入 → 弹全量候选；Enter 选中 → 胶囊入框", async () => {
    const onCommit = vi.fn();
    const { container } = render(<NodeFormulaField value="" onCommit={onCommit} candidates={source} />);
    const ta = el(container);
    ta.value = "/";
    ta.setSelectionRange(1, 1); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(ta, { target: { value: "/" } });
    expect(await screen.findByRole("listbox")).toBeTruthy();
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("\uE000count\uE001");
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("count");
  });

  it("运算符边界键入 → 弹运算符候选；Enter 选中", async () => {
    const { container } = render(<NodeFormulaField value="count" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.value = "count +";
    ta.setSelectionRange(7, 7);
    fireEvent.input(ta, { target: { value: "count +" } });
    const box = await screen.findByRole("listbox");
    expect(box).toBeTruthy();
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("count \uE000+\uE001");
  });

  it("词中间运算符不弹框（字符串内容）", async () => {
    const { container } = render(<NodeFormulaField value="" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.value = "user-s";
    ta.setSelectionRange(6, 6);
    fireEvent.input(ta, { target: { value: "user-s" } });
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("斜杠逃生口：无匹配 Enter → 引用胶囊", async () => {
    const { container } = render(<NodeFormulaField value="" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.value = "/";
    ta.setSelectionRange(1, 1);
    fireEvent.input(ta, { target: { value: "/" } });
    expect(await screen.findByRole("listbox")).toBeTruthy();
    ta.value = "/s";
    ta.setSelectionRange(2, 2);
    fireEvent.input(ta, { target: { value: "/s" } }); // 菜单重过滤：候选空 → 仅运算符组
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("\uE000s\uE001");
  });

  it("点击胶囊 → 重新弹候选；Enter 替换", async () => {
    const { container } = render(<NodeFormulaField value="count + 1" onCommit={() => {}} candidates={source} />);
    const cap = container.querySelector("[data-seg-kind='capsule']") as HTMLElement;
    fireEvent.mouseDown(cap);
    expect(await screen.findByRole("listbox")).toBeTruthy();
    const ta = el(container);
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("\uE000count\uE001 \uE000+\uE001 1");
  });

  it("Backspace 在胶囊标记起点 → 整块删除", () => {
    const { container } = render(<NodeFormulaField value="count + 1" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.setSelectionRange(1, 1); // 光标在胶囊内
    fireEvent.keyDown(ta, { key: "Backspace" });
    expect(ta.value).toBe(" \uE000+\uE001 1");
  });

  it("Enter 提交（无候选框）；Esc 取消恢复 value", () => {
    const onCommit = vi.fn();
    const { container, rerender } = render(<NodeFormulaField value="count" onCommit={onCommit} candidates={source} />);
    const ta = el(container);
    ta.value = "\uE000count\uE001 \uE000+\uE001 1";
    ta.setSelectionRange(11, 11);
    fireEvent.input(ta, { target: { value: "\uE000count\uE001 \uE000+\uE001 1" } });
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("count + 1");
    // Esc 取消 → 恢复外部 value 的反向结构化形态
    ta.value = "\uE000count\uE001 \uE000+\uE001 2";
    ta.setSelectionRange(11, 11);
    fireEvent.input(ta, { target: { value: "count + 2" } });
    fireEvent.keyDown(ta, { key: "Escape" });
    expect(ta.value).toBe("\uE000count\uE001");
  });
});
```

**注意**：jsdom 的 textarea 需手动 `setSelectionRange` 且**必须在 `fireEvent.input` 之前设置**（onInput 同步读 selectionStart）；`data-seg-kind` 是覆盖层渲染时给胶囊块/结构字符加的属性。若 jsdom 中 `fireEvent.input` 与受控 textarea 组合有偏差（React 合成事件读 target.value 与 state 竞态），改用 `fireEvent.change` 先行同步，以真实通过为准。

- [ ] **Step 2: 运行验证失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`
Expected: FAIL（组件为旧版 contentEditable，无 textarea）

- [ ] **Step 3: 重写 NodeFormulaField.tsx**

```tsx
// poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx
// 表达式公式框（重新设计）：textarea 编辑层 + 覆盖层胶囊渲染。
// 光标/IME/选区浏览器原生；胶囊=私有区标记段 \uE000..\uE001，覆盖层渲染高亮块；
// 结构字符着色；自由文本为普通文本。提交=serializeSegments(markup)（字符串，编译器零改动）。
import { memo, useEffect, useMemo, useRef, useState, type KeyboardEvent, type CompositionEvent, type FormEvent } from "react";
import { parseMarkup, serializeSegments, toMarkup, isBoundary, TRIGGER_CHARS, operatorCandidates, tokenizeFormula, CAPSULE_START, CAPSULE_END } from "./expression-trigger";
import { NodeCandidateMenu, type CandidateItem } from "./NodeCandidateMenu";
import styles from "./NodeFormulaField.module.css";

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
  /** 触发字符区间（Escape 落点取触发字符用；选中替换用 triggerOffset..当前光标 动态区间） */
  replace: { start: number; end: number } | null;
  /** 逃生口：菜单打开时触发字符（如 "/"）在 markup 中的偏移 */
  triggerOffset: number;
}

/** memo 自定义比较：函数 props 引用变化不触发重渲染（父节点选中等不重置光标）。 */
export const NodeFormulaField = memo(function NodeFormulaField({ value, onCommit, candidates, placeholder, ariaLabel, className, invalid, onCancel, onBlur }: Props) {
  const [draft, setDraft] = useState(() => toMarkup(value));
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [composing, setComposing] = useState(false);
  const focusedRef = useRef(false);
  const dirtyRef = useRef(false);
  const composingRef = useRef(false);
  const taRef = useRef<HTMLTextAreaElement>(null);
  const menuRef = useRef<MenuState | null>(null);
  menuRef.current = menu;

  // 外部 value 变化（提交成功）时同步草稿；聚焦输入中不打断
  useEffect(() => {
    if (!focusedRef.current) setDraft(toMarkup(value));
  }, [value]);

  const segments = useMemo(() => parseMarkup(draft), [draft]);

  const commit = () => {
    if (dirtyRef.current) onCommit(serializeSegments(draft));
    dirtyRef.current = false;
    setMenu(null);
  };
  const cancel = () => {
    focusedRef.current = false;
    dirtyRef.current = false;
    setMenu(null);
    setDraft(toMarkup(value));
    onCancel?.();
  };

  const caret = (): number => taRef.current?.selectionStart ?? draft.length;

  /** 弹框上下文：光标处最后输入的触发字符 → 候选分组与替换区间。 */
  const openMenu = (markup: string) => {
    const el = taRef.current;
    if (!el || !candidates) return;
    const offset = caret();
    const ch = markup[offset - 1];
    if (!ch || !TRIGGER_CHARS.has(ch) || !isBoundary(markup, offset - 1)) return;
    const rect = el.getBoundingClientRect();
    if (ch === "/") {
      const all = candidates.filter("");
      if (all.length === 0) return;
      const groups: Array<{ label: string; items: CandidateItem[] }> = [];
      if (all.length > 0) groups.push({ label: "候选", items: all });
      const ops = operatorCandidates("/");
      if (ops.length > 0) groups.push({ label: "运算符", items: ops });
      setMenu({ x: rect.left, y: rect.bottom + 4, groups, activeIdx: 0, replace: { start: offset - 1, end: offset }, triggerOffset: offset - 1 });
    } else {
      const items = operatorCandidates(ch);
      if (items.length === 0) return;
      setMenu({ x: rect.left, y: rect.bottom + 4, groups: [{ label: "运算符", items }], activeIdx: 0, replace: { start: offset - 1, end: offset }, triggerOffset: offset - 1 });
    }
  };

  /** 输入（textarea 原生）→ 更新草稿；非组合期判定弹框。 */
  const onInput = (e: FormEvent<HTMLTextAreaElement>) => {
    const el = taRef.current;
    if (!el) return;
    const native = (e as unknown as { nativeEvent?: InputEvent })?.nativeEvent;
    if (native && native.isComposing === false) composingRef.current = false;
    const next = el.value;
    setDraft(next);
    dirtyRef.current = true;
    if (composingRef.current) return;
    if (menuRef.current) {
      // 菜单打开：斜杠触发 → 按触发后文本重新过滤（逃生口依赖候选组空）；运算符触发 → 字符已变则关框
      const m = menuRef.current;
      const offset = caret();
      if (next[m.triggerOffset] === "/") {
        const query = next.slice(m.triggerOffset + 1, offset);
        const items = candidates?.filter(query) ?? [];
        const groups: Array<{ label: string; items: CandidateItem[] }> = [];
        if (items.length > 0) groups.push({ label: "候选", items });
        const ops = operatorCandidates("/");
        if (ops.length > 0) groups.push({ label: "运算符", items: ops });
        if (groups.length === 0) setMenu(null);
        else setMenu({ ...m, groups, activeIdx: 0 });
      } else {
        setMenu(null);
      }
      return;
    }
    const prevCaret = caret();
    if (next[prevCaret - 1] && TRIGGER_CHARS.has(next[prevCaret - 1]!)) openMenu(next);
  };

  /** 逃生口：斜杠菜单、候选组空时 Enter → 触发字符+输入文本 → 引用胶囊（候选外标识符出路）。 */
  const escapeHatch = () => {
    if (!menu) return false;
    const cands = menu.groups.filter((g) => g.label === "候选").flatMap((g) => g.items);
    if (cands.length > 0) return false; // 候选组非空 → 正常选中
    if (draft[menu.triggerOffset] !== "/") return false; // 仅斜杠菜单有逃生口（运算符菜单无）
    const el = taRef.current;
    if (!el) return false;
    const offset = caret();
    const raw = draft.slice(menu.triggerOffset + 1, offset).trim();
    if (!raw) return false;
    const next = draft.slice(0, menu.triggerOffset) + CAPSULE_START + raw + CAPSULE_END + draft.slice(offset);
    setDraft(next);
    setMenu(null);
    const cursor = menu.triggerOffset + CAPSULE_START.length + raw.length + CAPSULE_END.length;
    requestAnimationFrame(() => { if (taRef.current) taRef.current.setSelectionRange(cursor, cursor); });
    return true;
  };

  /** 选中候选：替换 triggerOffset..光标 区间为胶囊（斜杠后已输入文本一并替换；运算符/点击胶囊场景区间即触发字符）。 */
  const selectItem = (item: CandidateItem) => {
    if (!menu) return;
    const el = taRef.current;
    const offset = el ? el.selectionStart ?? draft.length : draft.length;
    const start = menu.triggerOffset;
    const v = draft.slice(0, start) + CAPSULE_START + item.value + CAPSULE_END + draft.slice(offset);
    setDraft(v);
    setMenu(null);
    const cursor = start + CAPSULE_START.length + item.value.length + CAPSULE_END.length;
    requestAnimationFrame(() => { if (taRef.current) { taRef.current.setSelectionRange(cursor, cursor); taRef.current.focus(); } });
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    const m = menu;
    if (m) {
      const flat = m.groups.flatMap((g) => g.items);
      if (e.key === "ArrowDown") { e.preventDefault(); setMenu((s) => (s ? { ...s, activeIdx: Math.min(s.activeIdx + 1, flat.length - 1) } : s)); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setMenu((s) => (s ? { ...s, activeIdx: Math.max(s.activeIdx - 1, 0) } : s)); }
      else if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        if (escapeHatch()) return; // 候选空且斜杠菜单 → 引用胶囊
        if (flat.length === 0) return;
        selectItem(flat[m.activeIdx]!);
      }
      else if (e.key === "Escape") {
        e.preventDefault();
        // Esc 关框：触发字符落点为运算符胶囊（原样提交，/ 除法语义由编译器解释）
        const r = m.replace;
        if (r) {
          const ch = draft.slice(r.start, r.end);
          if (TRIGGER_CHARS.has(ch) && !ch.includes(CAPSULE_START)) {
            setDraft(draft.slice(0, r.start) + CAPSULE_START + ch + CAPSULE_END + draft.slice(r.end));
          }
        }
        setMenu(null);
      }
      return;
    }
    if (e.key === "Enter") { e.preventDefault(); commit(); return; }
    if (e.key === "Escape") { e.preventDefault(); cancel(); return; }
    if (e.key === "Backspace" && !composingRef.current) {
      const el = taRef.current;
      if (!el) return;
      const offset = el.selectionStart ?? draft.length;
      if (el.selectionEnd !== offset) return; // 有选区走原生删除
      const seg = segments.find((s) => s.kind === "capsule" && offset >= s.start && offset <= s.end);
      if (seg && offset > seg.start) {
        // 光标在胶囊标记段内或末尾 → 整块删除
        e.preventDefault();
        const v = draft.slice(0, seg.start) + draft.slice(seg.end);
        setDraft(v);
        requestAnimationFrame(() => { if (taRef.current) taRef.current.setSelectionRange(seg.start, seg.start); });
      }
    }
  };

  /** 点击胶囊 → 定位光标 + 弹该类型候选（引用→全量过滤，运算符→运算符候选）。 */
  const onCapsuleMouseDown = (seg: (typeof segments)[number], e: React.MouseEvent) => {
    e.preventDefault();
    const el = taRef.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(seg.end, seg.end);
    if (!candidates) return;
    const rect = el.getBoundingClientRect();
    // 胶囊类型以 tokenizer 为单一来源：operator token → 运算符候选；其余 → 引用候选
    const tok = tokenizeFormula(seg.text)[0];
    const ops = tok?.kind === "operator" ? operatorCandidates(tok.text) : [];
    if (ops.length > 0) {
      setMenu({ x: rect.left, y: rect.bottom + 4, groups: [{ label: "运算符", items: ops }], activeIdx: 0, replace: { start: seg.start, end: seg.end }, triggerOffset: seg.start });
      return;
    }
    const items = candidates.filter(seg.text);
    if (items.length === 0) return;
    setMenu({ x: rect.left, y: rect.bottom + 4, groups: [{ label: "候选", items }], activeIdx: 0, replace: { start: seg.start, end: seg.end }, triggerOffset: seg.start });
  };

  return (
    <>
      <div className={`nodrag ${styles.wrap} ${className ?? ""}`} data-field-kind="formula" role="combobox" aria-expanded={menu ? true : undefined} aria-controls={menu ? "node-candidate-results" : undefined} aria-invalid={invalid || undefined} aria-label={ariaLabel}>
        {/* 覆盖层：胶囊块 + 结构字符着色 + 自由文本透明（textarea 文字透明，内容由覆盖层呈现） */}
        <div className={styles.overlay} aria-hidden>
          {segments.map((s, i) =>
            s.kind === "capsule" ? (
              <span key={i} data-seg-kind="capsule" className={styles.capsule} onMouseDown={(e) => onCapsuleMouseDown(s, e)}>{s.text}</span>
            ) : s.kind === "struct" ? (
              <span key={i} data-seg-kind="struct" className={styles.struct}>{s.text}</span>
            ) : (
              <span key={i} data-seg-kind="text" className={styles.text}>{s.text}</span>
            )
          )}
        </div>
        <textarea
          ref={taRef}
          className={composing ? styles.inputComposing : styles.input}
          value={draft}
          placeholder={placeholder}
          data-placeholder={placeholder}
          onFocus={() => { focusedRef.current = true; }}
          onBlur={() => {
            const wasDirty = dirtyRef.current;
            commit();
            focusedRef.current = false;
            if (!wasDirty) setDraft(toMarkup(value));
            onBlur?.();
          }}
          onCompositionStart={() => { composingRef.current = true; setComposing(true); }}
          onCompositionEnd={(e: CompositionEvent<HTMLTextAreaElement>) => {
            composingRef.current = false;
            setComposing(false);
            onInput(e);
          }}
          onInput={onInput}
          onKeyDown={onKeyDown}
        />
      </div>
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
}, (prev, next) => {
  return (
    prev.value === next.value &&
    prev.candidates === next.candidates &&
    prev.placeholder === next.placeholder &&
    prev.ariaLabel === next.ariaLabel &&
    prev.className === next.className &&
    prev.invalid === next.invalid
  );
});
```

- [ ] **Step 4: 重写 NodeFormulaField.module.css**

```css
.wrap { position: relative; min-width: 140px; line-height: 1.6; }
.overlay {
  position: absolute; inset: 0;
  display: flex; flex-wrap: wrap; align-content: flex-start;
  white-space: pre-wrap; word-break: break-all;
  pointer-events: none; overflow: hidden;
  font: inherit; padding: inherit; border: inherit;
}
.overlay > span { white-space: pre-wrap; word-break: break-all; }
.capsule {
  pointer-events: auto;
  background: rgba(168, 85, 247, 0.12); border-radius: 3px; padding: 0 2px; margin: 0 -1px;
  color: #7c3aed; cursor: pointer; user-select: none;
}
.capsule:hover { background: rgba(168, 85, 247, 0.22); }
.struct { color: #94a3b8; }   /* 结构字符：中性灰 */
.text { color: transparent; } /* 自由文本由 textarea 呈现（同字形），本层透明占位 */
.input {
  position: relative; z-index: 1;
  width: 100%; min-width: 140px;
  font: inherit; line-height: 1.6;
  white-space: pre-wrap; word-break: break-all;
  color: transparent;            /* 文字透明，覆盖层呈现 */
  caret-color: #334155;          /* 光标可见 */
  background: transparent; border: none; outline: none;
  resize: none; overflow: hidden;
}
.input::placeholder { color: #94a3b8; }
.inputComposing { color: #334155; caret-color: #334155; } /* 组合期文字可见（拼音未上屏时覆盖层无对应内容，默认透明会看不到） */
```

**关键对齐约束**：`.wrap` 必须有确定的 padding/border（来自上层 `styles.field`/`styles.grow`），`.input` 与 `.overlay` 必须共享相同的 `font/padding/border/line-height/white-space/word-break`，否则覆盖层胶囊与 textarea 字符错位。若上层 `.field` 有 padding，把 padding 声明同时加到 `.input` 与 `.overlay`（实现时核对 `NodeField.module.css` 的 `.field` 实际值）。

- [ ] **Step 5: 运行验证通过**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`
Expected: PASS（若 jsdom 的 `fireEvent.input` 与受控 value 有偏差，用 `fireEvent.change` 先行同步，以测试真实通过为准，不改业务规则）

- [ ] **Step 6: 全量回归（公式框相关）**

Run: `npx vitest run src/components/canvas/code/fields/ src/components/canvas/code/nodes/ src/components/canvas/code/__tests__/`
Expected: PASS（SyntaxNode/ConditionNode/ReturnNode 使用点零改动兼容）

- [ ] **Step 7: 提交**

```bash
git add src/components/canvas/code/fields/NodeFormulaField.tsx src/components/canvas/code/fields/NodeFormulaField.module.css src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx
git commit -m "feat(codeflow): 公式框重写——textarea+覆盖层胶囊渲染、斜杠/运算符弹框、逃生口、点击重选"
```

---

### Task 6: 集成验证（tsc + 全量测试 + 真机 E2E）

**Files:**
- 无代码改动（验证任务）

**Interfaces:**
- Consumes: 全部前序任务产物

- [ ] **Step 1: tsc 类型检查**

Run: `npx tsc --noEmit`
Expected: 无计划外新增错误（既有基线允许项不变：诊断 `diagnostics.ts:151` 等历史基线）

- [ ] **Step 2: 全量测试**

Run: `npx vitest run`
Expected: 全部 PASS（旧公式框相关测试已随 Task 1-5 重写/替换，其余 600+ 基线不受影响）

- [ ] **Step 3: 真机 E2E（Victauri，会话内执行）**

启动 Poiesis（`/start-poiesis`）后经 Victauri 逐项验证：

1. **输入流**：打开 CodeFlow 画布 → 新建 assign 节点 → 点击 value 公式框 → 光标落在点击处（不复位）→ 输入 `count` → 斜杠弹全量候选 → Enter 选中 → 胶囊入框
2. **中文 IME**：输入法键入中文（`你好`）→ 组合过程无弹框、无光标跳变 → 提交 `"你好"`
3. **运算符弹框**：空格后按 `+` → 弹运算符候选 → Enter → 运算符胶囊；按 `<` → 弹 `<`/`<=` → 选 `<=`
4. **词中间不弹框**：输入 `user-settings` → `-` 不弹框 → 提交 `"user-settings"`
5. **member 链**：`sizeBox` 胶囊 → `.` → `measure` → `(` `)` → 提交 `sizeBox.measure()`
6. **点击胶囊重选**：点击变量胶囊 → 候选框弹出 → Enter 替换
7. **Backspace 整块删**：光标在胶囊尾 → Backspace 整块删除
8. **round-trip**：保存（Enter/失焦）→ 检查 `.omni` 文本含 `count + 1`（无 `\uE000` 残留）→ 重进画布 → 胶囊重新物化 → 再提交与 `.omni` 一致
9. **CodeFlowMidChain 语料**：加载 `CodeFlowMidChain.omni` 的 fn 图 → 逐个节点 value 显示为胶囊/文本正确 → 不改动直接保存 → `.omni` 逐字不变

- [ ] **Step 4: 提交（若有 E2E 暴露的修复）**

```bash
git add -A && git commit -m "fix(codeflow): E2E 暴露的公式框缺陷修复"  # 仅在 Step 3 发现缺陷时
```

---

## Self-Review 记录

- **Spec 覆盖**：D1（textarea+覆盖层）→ Task 5；D2（三段内容物）→ Task 1/5；D3（字面量判定）→ Task 2；D4（触发/逃生口/Esc 落点/点击/删除/提交）→ Task 4/5；D5（member 段）→ Task 2/3；D6（反向结构化）→ Task 3；测试/范围/复用 → Task 1-6。spec 已知取舍（复制标记字符、无结果提示、结构体 key 出范围）均已映射。
- **占位符扫描**：无 TBD/TODO；全部步骤含真实代码或可执行命令。
- **类型一致性**：`parseMarkup`/`serializeSegments`/`toMarkup`/`isBoundary`/`TRIGGER_CHARS`/`OPERATOR_PREFIX_CANDIDATES` 在 Task 1-5 间签名一致；组件 Props 与旧版逐字段一致（三使用点零改动）；`Segment.kind` 三值在 Task 1/5 间一致。

### 自审修正（2026-08-06，入计划时内联执行）

- **openMenu 闭包陈旧**（Task 5 Step 3）：原 `openMenu()` 读 `draft` 闭包——onInput 里 `setDraft(next)` 异步生效，首次键入触发字符（`/`、`+` 等）时 draft 仍是旧值（空框场景 draft=""），`draft[offset-1]` 取不到触发字符 → 菜单永不弹出。修正：`openMenu(markup: string)` 参数化，onInput 调用 `openMenu(next)`。
- **escapeHatch 组检查错误**（Task 5 Step 3）：原检查 `menu.groups[0].items` 非空即正常选中——斜杠菜单重过滤（输入无匹配）后 groups = `[运算符组]` 单组且 items 非空（`operatorCandidates("/")` 恒有 `/`），逃生口被误判"有候选"而永不触发。修正：只检查 `label === "候选"` 组的 items（`filter().flatMap()`）。
- **测试期望未算 `+` 也是胶囊**（Task 5 Step 1）：`toMarkup("count + 1")` = `count + 1`——t1 渲染断言 `toContain` → 精确 `toBe`；t5 点击重选后 `+` 胶囊保持；t6 Backspace 删 count 后剩 ` + 1`；t7 输入必须带胶囊标记（否则 `count + 1` 整个被判为字符串字面量，提交变 `"count + 1"` 带引号）且 `setSelectionRange(11, 11)` 在 fireEvent 前。
- **IME 组合期文字不可见**（Task 5 Step 3/4）：textarea 默认 `color: transparent`（覆盖层呈现），组合期拼音未上屏时覆盖层无对应内容 → 用户看不到正在输入的中文。修正：`className` 按 `composing` 切换 `styles.inputComposing`（文字恢复深色），移除无意义占位 `imeHint` span。
