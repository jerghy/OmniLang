# 调试面板实现计划（Debug Panel）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 Design Canvas 的 ComponentDefPanel 重构成以调试为主的信号为中心调试台（三 Tab：界面信号调试/信号调试/定义），支持 prop/signal/store 调试值覆盖驱动预览，服务 @if/@for 语法开发。

**Architecture:** 保留「编译前正则覆盖源码默认值」管线（方案 A）。新增信号分析器（对生成的组件源码文本做分析，聚合信号 → 界面用途 + @if 链 + @for 列表）、类型感知解析器、成真/成假合成器、代码样本提取器四个纯函数模块；store 数据模型换成 `debugValues`（ownerKey 键控 + 校验态）+ `debugOverridesEnabled` 总开关；UI 三 Tab 重构。

**Tech Stack:** React 19 / Zustand 5 / TypeScript / Vitest + @testing-library/react / 中文注释风格（与现有代码一致）

**Spec:** `docs/superpowers/specs/2026-08-06-debug-panel-design.md`（决策 D1-D12）

## Global Constraints

- 不碰 Rust/wasm crate（方案 A 铁律）
- 调试值永不落盘、不 dirty 项目（store 注释注明）
- 现有 `debugSignalValues`/`setDebugSignalValue` 在 Task 7 前保留（双字段共存过渡，避免中间态编译破）
- sources 文件 key 约定：main → `src/main.omni`；组件 X → `src/components/X.omni`；独立 store S → `src/stores/S.omni`；内联 store 在组件文件内
- 内联 store key 为 qualified `"Component.Store"`（store-crud.ts 已确认）
- 每个任务结束：vitest 通过 + tsc 干净 + 提交
- 测试运行命令（poiesis 目录）：`npx vitest run <file> --reporter=verbose`；全量 `npx vitest run`

---

### Task 1: Store 数据模型——debugValues + 总开关

**Files:**
- Modify: `poiesis/src/store/types.ts`（75、189 行附近）
- Modify: `poiesis/src/store/index.ts`（36 行附近）
- Modify: `poiesis/src/store/slices/component-lifecycle.ts`（93-103 行附近，替换 setDebugSignalValue 实现）
- Test: `poiesis/src/store/__tests__/debug-values.test.ts`（新建）

**Interfaces:**
- Consumes: 无（第一个任务）
- Produces:
  ```ts
  // store/types.ts
  interface DebugValueEntry { text: string; valid: boolean }
  debugValues: Record<string, Record<string, DebugValueEntry>>;
  debugOverridesEnabled: boolean;
  setDebugValue: (ownerKey: string, name: string, text: string, valid: boolean) => void;
  clearDebugValues: () => void;
  setDebugOverridesEnabled: (enabled: boolean) => void;
  ```

- [ ] **Step 1: 写失败测试**

`poiesis/src/store/__tests__/debug-values.test.ts`：

```ts
// debugValues 数据模型测试：ownerKey 键控 + 校验态 + 总开关。
import { describe, it, expect } from "vitest";
import { usePoiesisStore } from "../index";

function reset() {
  usePoiesisStore.setState({ debugValues: {}, debugOverridesEnabled: true });
}

describe("debugValues", () => {
  it("setDebugValue 写入 ownerKey 分组条目（含校验态）", () => {
    reset();
    usePoiesisStore.getState().setDebugValue("main", "status", "loading", true);
    const v = usePoiesisStore.getState().debugValues["main"]?.["status"];
    expect(v).toEqual({ text: "loading", valid: true });
    // 不 dirty 项目
    expect(usePoiesisStore.getState().isDirty).toBe(false);
  });

  it("不同 ownerKey 同名互不覆盖", () => {
    reset();
    const s = usePoiesisStore.getState();
    s.setDebugValue("main", "status", "loading", true);
    s.setDebugValue("TodoStore", "status", "done", true);
    expect(usePoiesisStore.getState().debugValues["main"]?.["status"]?.text).toBe("loading");
    expect(usePoiesisStore.getState().debugValues["TodoStore"]?.["status"]?.text).toBe("done");
  });

  it("clearDebugValues 清空全部", () => {
    reset();
    const s = usePoiesisStore.getState();
    s.setDebugValue("main", "count", "3", true);
    s.setDebugValue("TodoStore", "items", "a,b", true);
    s.clearDebugValues();
    expect(usePoiesisStore.getState().debugValues).toEqual({});
  });

  it("总开关默认开启，可切换", () => {
    reset();
    expect(usePoiesisStore.getState().debugOverridesEnabled).toBe(true);
    usePoiesisStore.getState().setDebugOverridesEnabled(false);
    expect(usePoiesisStore.getState().debugOverridesEnabled).toBe(false);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/store/__tests__/debug-values.test.ts --reporter=verbose`
Expected: FAIL（`debugValues` 未定义 / `setDebugValue` 不存在）

- [ ] **Step 3: 实现**

`poiesis/src/store/types.ts` — 在 debugSignalValues 声明（75 行）旁边加：

```ts
/** 调试值条目：text=输入框回显原文，valid=类型校验结果（false 不参与编译） */
interface DebugValueEntry {
  text: string;
  valid: boolean;
}
/** 调试值覆盖（ownerKey = 组件名 | store 名 | 内联 "Comp.Store"）——调试专用，不 dirty 项目 */
debugValues: Record<string, Record<string, DebugValueEntry>>;
/** 调试总开关：off 时调试值不参与编译但输入保留（对比原始渲染） */
debugOverridesEnabled: boolean;
```

types.ts 的 PoiesisState 接口（189 行附近）加方法签名：

```ts
setDebugValue: (ownerKey: string, name: string, text: string, valid: boolean) => void;
clearDebugValues: () => void;
setDebugOverridesEnabled: (enabled: boolean) => void;
```

`poiesis/src/store/index.ts`（36 行附近）初始化：

```ts
debugValues: {},
debugOverridesEnabled: true,
```

`poiesis/src/store/slices/component-lifecycle.ts` — 在 setDebugSignalValue 实现（93-103 行）后追加（旧实现保留到 Task 7）：

```ts
    // ── Debug value overrides（新模型，Task 1+）──
    setDebugValue: (ownerKey: string, name: string, text: string, valid: boolean) => set((s) => {
      const ownerValues = { ...(s.debugValues[ownerKey] || {}) };
      ownerValues[name] = { text, valid };
      return {
        debugValues: { ...s.debugValues, [ownerKey]: ownerValues },
        // Debug values don't dirty the project
      };
    }),
    clearDebugValues: () => set({ debugValues: {} }),
    setDebugOverridesEnabled: (enabled: boolean) => set({ debugOverridesEnabled: enabled }),
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/store/__tests__/debug-values.test.ts --reporter=verbose`
Expected: PASS（4 个用例）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/store/types.ts poiesis/src/store/index.ts poiesis/src/store/slices/component-lifecycle.ts poiesis/src/store/__tests__/debug-values.test.ts
git commit -m "feat(poiesis): debugValues 数据模型——ownerKey 键控 + 校验态 + 总开关（Task 1/11）"
```

---

### Task 2: 类型感知解析器 parseDebugValue

**Files:**
- Create: `poiesis/src/services/debug/parse-debug-value.ts`
- Test: `poiesis/src/services/debug/__tests__/parse-debug-value.test.ts`

**Interfaces:**
- Consumes: 无
- Produces:
  ```ts
  export interface ParsedDebugValue { literal: string; valid: boolean }
  export function parseDebugValue(type: string, text: string): ParsedDebugValue
  // type 例："String" | "bool" | "i32" | "Array<String>" | "RowItem[]" | "Map<String, i32>"
  ```

- [ ] **Step 1: 写失败测试**

`poiesis/src/services/debug/__tests__/parse-debug-value.test.ts`：

```ts
// 类型感知解析器：按声明类型把输入文本转成 OmniLang 字面量；字面量透传识别。
import { describe, it, expect } from "vitest";
import { parseDebugValue } from "../parse-debug-value";

describe("parseDebugValue", () => {
  it("String：任意文本加引号并转义", () => {
    expect(parseDebugValue("String", 'hello "world"')).toEqual({ literal: '"hello \\"world\\""', valid: true });
  });

  it("String：空输入 = 空字符串字面量", () => {
    expect(parseDebugValue("String", "")).toEqual({ literal: '""', valid: true });
  });

  it("bool：仅 true/false 合法", () => {
    expect(parseDebugValue("bool", "true")).toEqual({ literal: "true", valid: true });
    expect(parseDebugValue("bool", "false")).toEqual({ literal: "false", valid: true });
    expect(parseDebugValue("bool", "yes").valid).toBe(false);
  });

  it("i32/f64：数字校验", () => {
    expect(parseDebugValue("i32", "42")).toEqual({ literal: "42", valid: true });
    expect(parseDebugValue("f64", "-3.14")).toEqual({ literal: "-3.14", valid: true });
    expect(parseDebugValue("i32", "abc").valid).toBe(false);
    expect(parseDebugValue("i32", "3.5").valid).toBe(false);
  });

  it("Array<String>：逗号分隔转数组字面量", () => {
    expect(parseDebugValue("Array<String>", 'a, b"c')).toEqual({ literal: '["a", "b\\"c"]', valid: true });
  });

  it("Array<i32>：逐项数字校验", () => {
    expect(parseDebugValue("Array<i32>", "1, 2, 3")).toEqual({ literal: "[1, 2, 3]", valid: true });
    expect(parseDebugValue("Array<i32>", "1, x").valid).toBe(false);
  });

  it("Array<String>：`String[]` 语法糖同解析", () => {
    expect(parseDebugValue("String[]", "a, b").literal).toBe('["a", "b"]');
  });

  it("字面量透传：已写 OmniLang 字面量原样过", () => {
    expect(parseDebugValue("Array<String>", '["a", "b"]')).toEqual({ literal: '["a", "b"]', valid: true });
    expect(parseDebugValue("Map<String, i32>", '{ "a": 1 }')).toEqual({ literal: '{ "a": 1 }', valid: true });
    expect(parseDebugValue("String", "3")).toEqual({ literal: "3", valid: true }); // 数字开头整体透传
  });

  it("struct 数组：文本输入拒绝（用代码样本胶囊）", () => {
    expect(parseDebugValue("RowItem[]", "a, b").valid).toBe(false);
  });

  it("未知类型：原样透传不校验", () => {
    expect(parseDebugValue("Map<String, i32>", "anything")).toEqual({ literal: "anything", valid: true });
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/parse-debug-value.test.ts --reporter=verbose`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现**

`poiesis/src/services/debug/parse-debug-value.ts`：

```ts
// 类型感知调试值解析：把用户输入文本按声明类型转成 OmniLang 字面量。
// 规则（spec 4.3）：String 加引号转义 / bool 严格 / 数字校验 / Array 逗号分隔逐项转义 /
// 字面量透传识别（以 [ { " ' 开头或整体为数字/bool → 原样过）/ struct 数组拒绝 / 未知类型透传。

export interface ParsedDebugValue {
  literal: string;
  valid: boolean;
}

const NUMBER_RE = /^-?\d+(\.\d+)?$/;
const BOOL_RE = /^(true|false)$/;
// 字面量透传：以数组/对象/引号开头，或整体是数字/bool
function isLiteralText(text: string): boolean {
  const t = text.trim();
  if (t.length === 0) return false;
  const first = t[0];
  if (first === "[" || first === "{" || first === '"' || first === "'") return true;
  return NUMBER_RE.test(t) || BOOL_RE.test(t);
}

function escapeString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function elementTypeOf(arrayType: string): string | null {
  // "Array<String>" → "String"；"String[]" → "String"
  const m = arrayType.match(/^Array<(.+)>$/);
  if (m) return m[1];
  const m2 = arrayType.match(/^(.+)\[\]$/);
  return m2 ? m2[1] : null;
}

function parseArrayItems(elemType: string, text: string): { literal: string; valid: boolean } {
  const items = text.split(",").map((s) => s.trim()).filter((s) => s.length > 0);
  const literals: string[] = [];
  for (const item of items) {
    const parsed = parseDebugValue(elemType, item);
    if (!parsed.valid) return { literal: "", valid: false };
    literals.push(parsed.literal);
  }
  return { literal: `[${literals.join(", ")}]`, valid: true };
}

export function parseDebugValue(type: string, text: string): ParsedDebugValue {
  const t = text.trim();
  if (t.length === 0) {
    // 空输入：String → ""，其余 → 无效（避免覆盖成空）
    return type === "String" ? { literal: '""', valid: true } : { literal: "", valid: false };
  }
  // 字面量透传
  if (isLiteralText(t)) return { literal: t, valid: true };

  if (type === "String") return { literal: `"${escapeString(t)}"`, valid: true };
  if (type === "bool") return BOOL_RE.test(t) ? { literal: t, valid: true } : { literal: "", valid: false };
  if (type === "i32" || type === "i64" || type === "f64" || type === "f32") {
    return NUMBER_RE.test(t) ? { literal: t, valid: true } : { literal: "", valid: false };
  }
  const elemType = elementTypeOf(type);
  if (elemType) {
    // struct 数组拒绝（复合类型数组用代码样本胶囊）
    if (elemType !== "String" && elemType !== "i32" && elemType !== "i64" && elemType !== "f64" && elemType !== "f32" && elemType !== "bool") {
      return { literal: "", valid: false };
    }
    return parseArrayItems(elemType, t);
  }
  // 未知类型：原样透传不校验
  return { literal: t, valid: true };
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/parse-debug-value.test.ts --reporter=verbose`
Expected: PASS（10 个用例）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/debug/parse-debug-value.ts poiesis/src/services/debug/__tests__/parse-debug-value.test.ts
git commit -m "feat(poiesis): parseDebugValue 类型感知解析器——String/bool/数字/数组/字面量透传（Task 2/11）"
```

---

### Task 3: 文件级覆盖管线 applyDebugOverrides

**Files:**
- Create: `poiesis/src/services/debug/debug-overrides.ts`
- Test: `poiesis/src/services/debug/__tests__/debug-overrides.test.ts`

**Interfaces:**
- Consumes: `DebugValueEntry`（store/types.ts，Task 1）
- Produces:
  ```ts
  export interface OverrideCtx { componentName: string; storePaths: Record<string, string> }
  export function resolveOwnerFile(ownerKey: string, ctx: OverrideCtx): string | null
  export function applyDebugOverrides(
    sources: Record<string, string>,
    debugValues: Record<string, Record<string, DebugValueEntry>>,
    ctx: OverrideCtx,
  ): Record<string, string>
  // 只替换 valid 条目；返回新对象（sources 原样不可变）
  ```

- [ ] **Step 1: 写失败测试**

`poiesis/src/services/debug/__tests__/debug-overrides.test.ts`：

```ts
// 文件级调试覆盖：ownerKey → 目标文件精确定位，修复跨组件同名冲突。
import { describe, it, expect } from "vitest";
import { applyDebugOverrides, resolveOwnerFile } from "../debug-overrides";

const C = { componentName: "Card", storePaths: { TodoStore: "src/stores/TodoStore.omni" } };

describe("resolveOwnerFile", () => {
  it("组件名 → 组件文件", () => {
    expect(resolveOwnerFile("Card", C)).toBe("src/components/Card.omni");
  });
  it("main → src/main.omni", () => {
    expect(resolveOwnerFile("main", { ...C, componentName: "main" })).toBe("src/main.omni");
  });
  it("独立 store → store 文件", () => {
    expect(resolveOwnerFile("TodoStore", C)).toBe("src/stores/TodoStore.omni");
  });
  it("内联 store（Component.Store）→ 组件文件", () => {
    expect(resolveOwnerFile("Card.CartStore", C)).toBe("src/components/Card.omni");
  });
  it("未知 owner → null", () => {
    expect(resolveOwnerFile("Nope", C)).toBeNull();
  });
});

describe("applyDebugOverrides", () => {
  const sources = {
    "src/components/Card.omni": [
      "component Card {",
      '  prop title: String = "Hello";',
      "  signal count: i32 = 0;",
      "  signal status: String;",
      "}",
    ].join("\n"),
    "src/main.omni": "signal status: String = 'x';",
  };

  it("按文件精确替换 signal 默认值（同名跨文件不冲突）", () => {
    const values = {
      Card: {
        status: { text: '"loading"', valid: true },
      },
    };
    const out = applyDebugOverrides(sources, values, C);
    expect(out["src/components/Card.omni"]).toContain('signal status: String = "loading";');
    expect(out["src/main.omni"]).toBe("signal status: String = 'x';"); // 未动
  });

  it("prop 同样支持覆盖", () => {
    const out = applyDebugOverrides(sources, { Card: { title: { text: '"New"', valid: true } } }, C);
    expect(out["src/components/Card.omni"]).toContain('prop title: String = "New";');
  });

  it("无默认值 signal 补 = 值", () => {
    const out = applyDebugOverrides(sources, { Card: { status: { text: '"ready"', valid: true } } }, C);
    expect(out["src/components/Card.omni"]).toContain('signal status: String = "ready";');
  });

  it("valid=false 不参与覆盖（源码保持）", () => {
    const out = applyDebugOverrides(sources, { Card: { count: { text: "abc", valid: false } } }, C);
    expect(out["src/components/Card.omni"]).toContain("signal count: i32 = 0;");
  });

  it("store 文件内覆盖（qualified 键）", () => {
    const src = { "src/stores/TodoStore.omni": "store TodoStore {\n  signal items: Array<String> = [];\n}" };
    const out = applyDebugOverrides(src, { TodoStore: { items: { text: '["a"]', valid: true } } }, C);
    expect(out["src/stores/TodoStore.omni"]).toContain('signal items: Array<String> = ["a"];');
  });

  it("覆盖目标不存在 → 该文件原样返回", () => {
    const out = applyDebugOverrides(sources, { Card: { ghost: { text: '"x"', valid: true } } }, C);
    expect(out["src/components/Card.omni"]).toBe(sources["src/components/Card.omni"]);
  });

  it("返回新对象，不修改输入 sources", () => {
    const values = { Card: { count: { text: "5", valid: true } } };
    const before = { ...sources };
    const out = applyDebugOverrides(sources, values, C);
    expect(out).not.toBe(sources);
    expect(sources).toEqual(before);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/debug-overrides.test.ts --reporter=verbose`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现**

`poiesis/src/services/debug/debug-overrides.ts`：

```ts
// 文件级调试覆盖管线（spec 4.3）：ownerKey → 目标文件精确定位，
// 正则替换 (signal|prop) <name>: Type = 值 → 调试字面量；无默认值补 = 值。
// 文件级键控修复旧 flatDebug 拍平导致的跨组件同名信号冲突。

import type { DebugValueEntry } from "../../store/types";

export interface OverrideCtx {
  componentName: string;
  /** 独立 store 名 → 文件路径（含内联 store 的组件文件由前缀推导，不入此表） */
  storePaths: Record<string, string>;
}

export function resolveOwnerFile(ownerKey: string, ctx: OverrideCtx): string | null {
  // 内联 store "Comp.Store" → 组件文件
  if (ownerKey.includes(".")) {
    const comp = ownerKey.slice(0, ownerKey.indexOf("."));
    return comp === "main" ? "src/main.omni" : `src/components/${comp}.omni`;
  }
  // 独立 store
  if (ownerKey in ctx.storePaths) return ctx.storePaths[ownerKey];
  // 组件
  if (ownerKey === ctx.componentName) {
    return ownerKey === "main" ? "src/main.omni" : `src/components/${ownerKey}.omni`;
  }
  return null;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function replaceDecl(source: string, name: string, literal: string): string {
  const kw = "(signal|prop)";
  const withDefault = new RegExp(
    `${kw}\\s+${escapeRegex(name)}\\s*:\\s*\\S+\\s*=\\s*[^;\\n]+;`,
    "g",
  );
  if (withDefault.test(source)) {
    withDefault.lastIndex = 0;
    return source.replace(withDefault, `$1 ${name}: ${literal};`);
  }
  // 无默认值：signal x: Type;（类型可能带泛型，用非贪婪到 ; 前）
  const noDefault = new RegExp(
    `${kw}\\s+${escapeRegex(name)}\\s*:\\s*[^;\\n]+;`,
    "g",
  );
  noDefault.lastIndex = 0;
  return source.replace(noDefault, `$1 ${name}: ${literal};`);
}

export function applyDebugOverrides(
  sources: Readonly<Record<string, string>>,
  debugValues: Readonly<Record<string, Record<string, DebugValueEntry>>>,
  ctx: OverrideCtx,
): Record<string, string> {
  const result: Record<string, string> = {};
  for (const path of Object.keys(sources)) {
    let text = sources[path];
    for (const [ownerKey, entries] of Object.entries(debugValues)) {
      const targetFile = resolveOwnerFile(ownerKey, ctx);
      if (targetFile !== path) continue;
      for (const [name, entry] of Object.entries(entries)) {
        if (!entry.valid) continue;
        const replaced = replaceDecl(text, name, entry.text);
        if (replaced !== text) text = replaced;
      }
    }
    result[path] = text;
  }
  return result;
}
```

> 注：`replaceDecl` 中 `$1` 保留 `signal`/`prop` 关键字；类型标注保留原样（只替换 `=` 后的默认值段）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/debug-overrides.test.ts --reporter=verbose`
Expected: PASS（10 个用例）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/debug/debug-overrides.ts poiesis/src/services/debug/__tests__/debug-overrides.test.ts
git commit -m "feat(poiesis): applyDebugOverrides 文件级覆盖——ownerKey→文件映射 + 正则替换（Task 3/11）"
```

---

### Task 4: 成真/成假合成器 synthesizeConditionValue

**Files:**
- Create: `poiesis/src/services/debug/synthesize-condition.ts`
- Test: `poiesis/src/services/debug/__tests__/synthesize-condition.test.ts`

**Interfaces:**
- Consumes: 无
- Produces:
  ```ts
  export type ComparisonOp = "==" | "!=" | ">" | ">=" | "<" | "<=" | null;
  export interface ConditionComparison { sig: string; op: ComparisonOp; literal?: string }
  export type TruthGoal = "true" | "false";
  export function synthesizeConditionValue(cmp: ConditionComparison, goal: TruthGoal): { literal: string; valid: boolean }
  // sig 可带 Store. 限定（spec D11）；literal 不带引号（原始字面量，如 loading / 0 / false）
  ```

- [ ] **Step 1: 写失败测试**

`poiesis/src/services/debug/__tests__/synthesize-condition.test.ts`：

```ts
// 成真/成假合成规则（spec 4.4 表）：点选分支 → 合成使条件为真/假的值。
import { describe, it, expect } from "vitest";
import { synthesizeConditionValue } from "../synthesize-condition";

describe("synthesizeConditionValue", () => {
  it("裸 bool：成真 true / 成假 false", () => {
    expect(synthesizeConditionValue({ sig: "isActive", op: null }, "true")).toEqual({ literal: "true", valid: true });
    expect(synthesizeConditionValue({ sig: "isActive", op: null }, "false")).toEqual({ literal: "false", valid: true });
  });

  it("==：成真取字面量", () => {
    expect(synthesizeConditionValue({ sig: "status", op: "==", literal: "loading" }, "true"))
      .toEqual({ literal: "loading", valid: true });
  });

  it("== 成假：bool 取反", () => {
    expect(synthesizeConditionValue({ sig: "flag", op: "==", literal: "true" }, "false"))
      .toEqual({ literal: "false", valid: true });
    expect(synthesizeConditionValue({ sig: "flag", op: "==", literal: "false" }, "false"))
      .toEqual({ literal: "true", valid: true });
  });

  it("== 成假：数字 0 取 1，非 0 取 0", () => {
    expect(synthesizeConditionValue({ sig: "count", op: "==", literal: "0" }, "false")).toEqual({ literal: "1", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "==", literal: "5" }, "false")).toEqual({ literal: "0", valid: true });
  });

  it("== 成假：字符串非空取空、空取 x", () => {
    expect(synthesizeConditionValue({ sig: "status", op: "==", literal: "loading" }, "false")).toEqual({ literal: "", valid: true });
    expect(synthesizeConditionValue({ sig: "status", op: "==", literal: "" }, "false")).toEqual({ literal: "x", valid: true });
  });

  it("!=：成真取「成假」值，成假取字面量", () => {
    expect(synthesizeConditionValue({ sig: "count", op: "!=", literal: "0" }, "true")).toEqual({ literal: "1", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "!=", literal: "0" }, "false")).toEqual({ literal: "0", valid: true });
  });

  it("比较链：> n 成真 n+1、成假 n", () => {
    expect(synthesizeConditionValue({ sig: "count", op: ">", literal: "10" }, "true")).toEqual({ literal: "11", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: ">", literal: "10" }, "false")).toEqual({ literal: "10", valid: true });
  });

  it("比较链：>= 成真 n、成假 n-1；< 成真 n-1、成假 n；<= 成真 n、成假 n+1", () => {
    expect(synthesizeConditionValue({ sig: "count", op: ">=", literal: "10" }, "true")).toEqual({ literal: "10", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: ">=", literal: "10" }, "false")).toEqual({ literal: "9", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<", literal: "10" }, "true")).toEqual({ literal: "9", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<", literal: "10" }, "false")).toEqual({ literal: "10", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<=", literal: "10" }, "true")).toEqual({ literal: "10", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<=", literal: "10" }, "false")).toEqual({ literal: "11", valid: true });
  });

  it("无字面量比较（sig > 但无 literal）→ invalid", () => {
    expect(synthesizeConditionValue({ sig: "count", op: ">" }, "true").valid).toBe(false);
  });

  it("store qualified sig 不参与合成逻辑差异（sig 仅作标识）", () => {
    expect(synthesizeConditionValue({ sig: "NotificationStore.unreadCount", op: "==", literal: "0" }, "true"))
      .toEqual({ literal: "0", valid: true });
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/synthesize-condition.test.ts --reporter=verbose`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现**

`poiesis/src/services/debug/synthesize-condition.ts`：

```ts
// 成真/成假合成器（spec 4.4）：点选 @if 分支 → 合成使该条件为真/假的值。
// 只支持单信号简单比较；复合条件/函数调用由调用方判定为 invalid 置灰。

export type ComparisonOp = "==" | "!=" | ">" | ">=" | "<" | "<=" | null;

export interface ConditionComparison {
  /** 信号 key，可带 Store. 限定（如 NotificationStore.unreadCount） */
  sig: string;
  op: ComparisonOp;
  /** 比较字面量（原始文本，不带引号）："loading" / 0 / false */
  literal?: string;
}

export type TruthGoal = "true" | "false";

const NUM_RE = /^-?\d+(\.\d+)?$/;
const BOOL_RE = /^(true|false)$/;

/** 求「x == literal」为假的值（即 != literal 为真的值） */
function falsifyEquals(literal: string): string {
  if (BOOL_RE.test(literal)) return literal === "true" ? "false" : "true";
  if (NUM_RE.test(literal)) return Number(literal) === 0 ? "1" : "0";
  // 字符串
  return literal === "" ? "x" : "";
}

export function synthesizeConditionValue(
  cmp: ConditionComparison,
  goal: TruthGoal,
): { literal: string; valid: boolean } {
  const { op, literal } = cmp;
  if (op === null) {
    return { literal: goal === "true" ? "true" : "false", valid: true };
  }
  if (literal === undefined || literal === "") {
    // 无字面量：只能处理 ==/!= 与空字符串比较或裸比较 —— 一律视为复杂，交由 UI 置灰
    return { literal: "", valid: false };
  }
  switch (op) {
    case "==":
      return goal === "true"
        ? { literal, valid: true }
        : { literal: falsifyEquals(literal), valid: true };
    case "!=":
      return goal === "true"
        ? { literal: falsifyEquals(literal), valid: true }
        : { literal, valid: true };
    case ">":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal: String(Number(literal) + 1), valid: true }
        : { literal, valid: true };
    case ">=":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal, valid: true }
        : { literal: String(Number(literal) - 1), valid: true };
    case "<":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal: String(Number(literal) - 1), valid: true }
        : { literal, valid: true };
    case "<=":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal, valid: true }
        : { literal: String(Number(literal) + 1), valid: true };
    default:
      return { literal: "", valid: false };
  }
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/synthesize-condition.test.ts --reporter=verbose`
Expected: PASS（9 个用例）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/debug/synthesize-condition.ts poiesis/src/services/debug/__tests__/synthesize-condition.test.ts
git commit -m "feat(poiesis): synthesizeConditionValue 成真/成假合成器——8 种比较形式规则表（Task 4/11）"
```

---

### Task 5: 代码样本提取器 extractAssignSamples

**Files:**
- Create: `poiesis/src/services/debug/sample-extractor.ts`
- Test: `poiesis/src/services/debug/__tests__/sample-extractor.test.ts`

**Interfaces:**
- Consumes: 无
- Produces:
  ```ts
  export interface SampleCandidate { label: string; literal: string }
  export function extractAssignSamples(fnSources: string[], signalName: string): SampleCandidate[]
  // fnSources：组件所有 fn 体源码文本数组；label 如 "initRows() 10 项"
  ```

- [ ] **Step 1: 写失败测试**

`poiesis/src/services/debug/__tests__/sample-extractor.test.ts`：

```ts
// 代码样本提取（spec D10）：从 fn 赋值字面量提取 struct 数组的真实样本。
import { describe, it, expect } from "vitest";
import { extractAssignSamples } from "../sample-extractor";

const INIT_ROWS = `
fn initRows() {
    rows = [
        RowItem { id: 1, name: "Alpha", status: "Active", score: 95 },
        RowItem { id: 2, name: "Bravo", status: "Active", score: 87 },
        RowItem { id: 3, name: "Charlie", status: "Inactive", score: 72 },
    ];
}
`;

describe("extractAssignSamples", () => {
  it("提取 name = [ ... ] 数组字面量（平衡括号）", () => {
    const samples = extractAssignSamples([INIT_ROWS], "rows");
    expect(samples.length).toBe(1);
    expect(samples[0].label).toContain("3 项");
    expect(samples[0].literal).toContain("RowItem { id: 1");
  });

  it("无该信号的赋值 → 空数组", () => {
    expect(extractAssignSamples([INIT_ROWS], "other")).toEqual([]);
  });

  it("赋值含函数调用/展开 → 跳过", () => {
    const src = "fn add() {\n  rows = [...rows, RowItem { id: 9 }];\n}\n";
    expect(extractAssignSamples([src], "rows")).toEqual([]);
  });

  it("多 fn 各自提取（最多 3 个样本）", () => {
    const srcs = [
      "fn a() {\n  rows = [RowItem { id: 1 }];\n}\n",
      "fn b() {\n  rows = [RowItem { id: 2 }, RowItem { id: 3 }];\n}\n",
    ];
    const samples = extractAssignSamples(srcs, "rows");
    expect(samples.length).toBe(2);
    expect(samples[0].label).toContain("1 项");
    expect(samples[1].label).toContain("2 项");
  });

  it("样本字面量去重（相同内容只留一个）", () => {
    const srcs = [
      "fn a() {\n  rows = [RowItem { id: 1 }];\n}\n",
      "fn b() {\n  rows = [RowItem { id: 1 }];\n}\n",
    ];
    expect(extractAssignSamples(srcs, "rows").length).toBe(1);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/sample-extractor.test.ts --reporter=verbose`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现**

`poiesis/src/services/debug/sample-extractor.ts`：

```ts
// 代码样本提取（spec D10）：对 Array<复合类型> 信号，从组件 fn 源码中
// `name = [ ... ]` 的赋值提取数组字面量作为调试候选（struct 数组无法文本输入）。
// 原样注入不解析；含函数调用/展开的赋值跳过（无法确定静态内容）。

export interface SampleCandidate {
  label: string;
  literal: string;
}

const MAX_SAMPLES = 3;

/** 提取 `name = [` 后到配平 `]` 的数组字面量文本；失败（含调用/无法配平）返回 null */
function extractArrayLiteral(fnSource: string, name: string): string | null {
  // 匹配 name = [ 或 name = [ ... （允许空白）
  const startRe = new RegExp(`\\b${name}\\s*=\\s*\\[`);
  const start = fnSource.search(startRe);
  if (start < 0) return null;
  const openIdx = fnSource.indexOf("[", start);
  if (openIdx < 0) return null;

  // 平衡括号扫描
  let depth = 0;
  let inString = false;
  let quote = "";
  for (let i = openIdx; i < fnSource.length; i++) {
    const ch = fnSource[i];
    if (inString) {
      if (ch === "\\") { i++; continue; }
      if (ch === quote) inString = false;
      continue;
    }
    if (ch === '"' || ch === "'") { inString = true; quote = ch; continue; }
    if (ch === "[") { depth++; continue; }
    if (ch === "]") {
      depth--;
      if (depth === 0) {
        const literal = fnSource.slice(openIdx, i + 1);
        // 字面量内不允许出现函数调用/方法调用（含 ( 或 .filter 等）
        if (/[a-zA-Z_]\s*\(/.test(literal)) return null;
        if (/\.\w+\(/.test(literal)) return null;
        return literal;
      }
    }
  }
  return null;
}

export function extractAssignSamples(fnSources: string[], signalName: string): SampleCandidate[] {
  const seen = new Set<string>();
  const samples: SampleCandidate[] = [];
  for (const src of fnSources) {
    if (samples.length >= MAX_SAMPLES) break;
    const literal = extractArrayLiteral(src, signalName);
    if (!literal || seen.has(literal)) continue;
    seen.add(literal);
    const itemCount = (literal.match(/\{[^}]*\}/g) || []).length;
    // label：函数名（从 "fn <name>" 或调用处推）——简化为项数
    const label = `样本 ${itemCount} 项`;
    samples.push({ label, literal });
  }
  return samples;
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/sample-extractor.test.ts --reporter=verbose`
Expected: PASS（5 个用例）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/debug/sample-extractor.ts poiesis/src/services/debug/__tests__/sample-extractor.test.ts
git commit -m "feat(poiesis): extractAssignSamples 代码样本提取——fn 赋值数组字面量平衡括号扫描（Task 5/11）"
```

---

### Task 6: 信号分析器 analyzeSignalUsage

**Files:**
- Create: `poiesis/src/services/debug/signal-analysis.ts`
- Test: `poiesis/src/services/debug/__tests__/signal-analysis.test.ts`

**Interfaces:**
- Consumes: `ConditionComparison`/`ComparisonOp`（synthesize-condition.ts，Task 4）
- Produces:
  ```ts
  export interface IfBranch { exprText: string | null; isElse: boolean; comparisons: ConditionComparison[] }
  export interface IfChain { branches: IfBranch[] }
  export interface ForUsage { collectionText: string; hasEmpty: boolean; viaTransform: boolean }
  export interface SignalUsage {
    key: string;                 // 信号 key（可带 Store. 限定）
    declaredType: string;
    conditions: IfChain[];       // 该信号参与的所有 @if 链（多链平铺）
    loops: ForUsage[];
    uses: { interpolations: number; binds: string[]; classGates: number; styleRefs: number; computedDeps: number };
  }
  export interface SignalDecl { key: string; type: string }
  export function analyzeSignalUsage(
    componentSource: string,          // 组件文件源码（view + 全部成员）
    signals: SignalDecl[],            // 组件信号（key = 名字）
    storeSignals: SignalDecl[],       // store 信号（key = "Store.sig"）
  ): { interfaceSignals: SignalUsage[]; dataSignals: SignalUsage[] }
  ```

- [ ] **Step 1: 写失败测试**

`poiesis/src/services/debug/__tests__/signal-analysis.test.ts`：

```ts
// 信号分析器（spec 4.2）：对组件源码文本聚合信号 → 界面用途 + 多 @if 链 + @for。
import { describe, it, expect } from "vitest";
import { analyzeSignalUsage } from "../signal-analysis";

const SRC = `
component NotificationsPanel {
    signal badgeVisible: bool = false;
    signal count: i32 = 0;
    @store NotificationStore;

    view {
        <h3>
            Notifications
            @if badgeVisible {
                <span>New</span>
            }
        </h3>
        @if NotificationStore.unreadCount == 0 {
            <p>No notifications</p>
        } @else {
            <p>Total: {NotificationStore.unreadCount}</p>
        }
        @for item in NotificationStore.notifications {
            <span>{item}</span>
        }
        <p>Count: {count} · badge {badgeVisible}</p>
        <input bind:value={input} />
    }
}
`;

const SIGNALS = [{ key: "badgeVisible", type: "bool" }, { key: "count", type: "i32" }];
const STORE_SIGNALS = [
  { key: "NotificationStore.unreadCount", type: "i32" },
  { key: "NotificationStore.notifications", type: "Array<String>" },
];

describe("analyzeSignalUsage", () => {
  it("界面信号：@if 条件出现 → conditions 链提取（含多分支）", () => {
    const { interfaceSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const badge = interfaceSignals.find((s) => s.key === "badgeVisible");
    expect(badge?.conditions.length).toBe(1);
    expect(badge?.conditions[0].branches[0].exprText).toBe("badgeVisible");
    // 单分支无 else → isElse 只一条
    expect(badge?.conditions[0].branches).toHaveLength(1);
  });

  it("store qualified 信号：== 比较 + 多分支链 + 插值计数", () => {
    const { interfaceSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const unread = interfaceSignals.find((s) => s.key === "NotificationStore.unreadCount");
    expect(unread?.conditions[0].branches.map((b) => b.isElse)).toEqual([false, true]);
    expect(unread?.conditions[0].branches[0].comparisons).toEqual([
      { sig: "NotificationStore.unreadCount", op: "==", literal: "0" },
    ]);
    expect(unread?.uses.interpolations).toBe(1);
  });

  it("@for collection 信号 → loops（含 @empty 标志）", () => {
    const { interfaceSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const items = interfaceSignals.find((s) => s.key === "NotificationStore.notifications");
    expect(items?.loops.length).toBe(1);
    expect(items?.loops[0].collectionText).toContain("NotificationStore.notifications");
    expect(items?.loops[0].hasEmpty).toBe(false);
  });

  it("纯数据信号（插值/bind）→ dataSignals；插值与绑定计数", () => {
    const { dataSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const count = dataSignals.find((s) => s.key === "count");
    expect(count?.uses.interpolations).toBe(1);
    const badge = dataSignals.find((s) => s.key === "badgeVisible");
    expect(badge).toBeUndefined(); // badgeVisible 驱动 @if → 界面信号
  });

  it("bind:value 绑定计数", () => {
    const src = `component C {\n signal input: String = "";\n view {\n  <input bind:value={input} />\n }\n}`;
    const { dataSignals } = analyzeSignalUsage(src, [{ key: "input", type: "String" }], []);
    expect(dataSignals[0]?.uses.binds).toContain("value");
  });

  it("复杂条件（函数调用）→ comparisons 空 + 标记 complex", () => {
    const src = `component C {\n signal items: Array<String> = [];\n view {\n  @if items.len() > 0 {\n   <p>has</p>\n  }\n }\n}`;
    const { interfaceSignals } = analyzeSignalUsage(src, [{ key: "items", type: "Array<String>" }], []);
    const items = interfaceSignals[0];
    // 条件含函数调用 → 该分支 comparisons 为空（UI 据此置灰胶囊）
    expect(items?.conditions[0].branches[0].comparisons).toEqual([]);
  });

  it("computed 依赖计入 uses.computedDeps", () => {
    const src = `component C {\n signal count: i32 = 0;\n computed double: i32 = count * 2;\n view { <p>{double}</p> }\n}`;
    const { dataSignals } = analyzeSignalUsage(src, [{ key: "count", type: "i32" }], []);
    expect(dataSignals[0]?.uses.computedDeps).toBe(1);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/signal-analysis.test.ts --reporter=verbose`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现**

`poiesis/src/services/debug/signal-analysis.ts`：

```ts
// 信号分析器（spec 4.2）：对组件源码文本做词法级分析，按信号聚合其界面用途。
// 只看 view 树（@if/@else/@for/@empty）+ 插值/bind/class/style/computed 依赖；
// fn 内的 if 是逻辑分支不驱动渲染结构，不参与分析。

import type { ConditionComparison, ComparisonOp } from "./synthesize-condition";

export interface IfBranch {
  exprText: string | null;   // else 分支为 null
  isElse: boolean;
  comparisons: ConditionComparison[];  // 空 = 复杂条件（UI 置灰胶囊）
}
export interface IfChain { branches: IfBranch[] }
export interface ForUsage {
  collectionText: string;
  hasEmpty: boolean;
  viaTransform: boolean;     // collection 含 filter/map/函数调用
}
export interface SignalUsage {
  key: string;
  declaredType: string;
  conditions: IfChain[];
  loops: ForUsage[];
  uses: { interpolations: number; binds: string[]; classGates: number; styleRefs: number; computedDeps: number };
}
export interface SignalDecl { key: string; type: string }

const COMPARISON_RE = /^(.+?)\s*(==|!=|>=|<=|>|<)\s*([^&|!]+)$/;

/** 把条件表达式拆成「参与信号 + 比较形式」；只认单信号简单比较 */
function parseComparisons(expr: string, signalKeys: string[]): ConditionComparison[] {
  const trimmed = expr.trim();
  // 裸信号（bool）或 !sig
  const bare = trimmed.match(/^!?\s*([A-Za-z_][\w.]*)$/);
  if (bare) {
    const sig = bare[1];
    if (signalKeys.includes(sig)) return [{ sig, op: null }];
    return [];
  }
  const m = trimmed.match(COMPARISON_RE);
  if (!m) return [];
  const left = m[1].trim();
  const op = m[2] as ComparisonOp;
  const right = m[3].trim();
  // 左边必须是信号；右边必须是无函数调用的字面量/变量（有调用 → 复杂）
  if (!signalKeys.includes(left)) return [];
  if (/[a-zA-Z_]\s*\(/.test(right)) return [];
  // 字面量剥引号：'loading' → loading；数字/bool 原样
  const literal = right.replace(/^["']|["']$/g, "");
  return [{ sig: left, op, literal }];
}

/** 提取 view 块文本（第一个 view { ... } 配平括号） */
function extractViewBlock(source: string): string {
  const start = source.search(/\bview\s*\{/);
  if (start < 0) return "";
  const openIdx = source.indexOf("{", start);
  let depth = 0;
  let inString = false;
  let quote = "";
  for (let i = openIdx; i < source.length; i++) {
    const ch = source[i];
    if (inString) {
      if (ch === "\\") { i++; continue; }
      if (ch === quote) inString = false;
      continue;
    }
    if (ch === '"' || ch === "'") { inString = true; quote = ch; continue; }
    if (ch === "{") depth++;
    if (ch === "}") { depth--; if (depth === 0) return source.slice(openIdx + 1, i); }
  }
  return "";
}

/** 扫描 view 文本，逐段识别 @if 链 / @for / 插值 / bind / class / style */
function scanView(view: string, signalKeys: string[], storeSignalKeys: string[]): {
  ifChains: IfChain[];
  loops: ForUsage[];
  uses: Record<string, { interpolations: number; binds: string[]; classGates: number; styleRefs: number }>;
} {
  const allKeys = [...signalKeys, ...storeSignalKeys];
  const uses: Record<string, { interpolations: number; binds: string[]; classGates: number; styleRefs: number }> = {};
  for (const k of allKeys) uses[k] = { interpolations: 0, binds: [], classGates: 0, styleRefs: 0 };

  const ifChains: IfChain[] = [];
  const loops: ForUsage[] = [];

  // @if ... { } (@else if ... { })* (@else { })? ——逐 token 扫描
  const tokens: Array<{ type: "if" | "else" | "for" | "empty" | "text"; value: string }> = [];
  let rest = view;
  const ifRe = /@if\s+([^{]*?)\s*(\{|mode:)/g;
  const elseRe = /@else\s*(?:(if)\s+([^{]*?)\s*\{|\{)/g;
  const forRe = /@for\s+([A-Za-z_]\w*)\s+in\s+([^{]*?)\s*(\{|key=)/g;
  const emptyRe = /@empty\s*\{/g;

  // 简化策略：按位置切分指令段（顺序扫描，分别处理 if 链、for、插值计数）
  // ── 1) 提取 @if 链 ──
  const chainRe = /@if([^{]*?)\{(.*?)\}(?=@else|@if|@for|@empty|\s*<\/|\s*$)/gs;
  // 链式解析：先按 @else 切分整体
  const fullChainRe = /(@if[^{]*\{[\s\S]*?(?=@else(?:\s*@?if|\s*\{)|$))/g;
  // 简化实现：先粗分 if 段，再处理 else 段
  const ifSegments: Array<{ expr: string; isElse: boolean }> = [];
  const segmentRe = /@(?:if|else\s*@?if?)\s*([^{]*?)\s*\{/g;
  let m: RegExpExecArray | null;
  let lastChainStart = -1;
  // 完整实现见下方 buildIfChains——此处以顺序扫描为准
  while ((m = segmentRe.exec(view)) !== null) {
    ifSegments.push({ expr: m[1].trim(), isElse: view.slice(segmentRe.lastIndex - m[0].length, segmentRe.lastIndex).includes("else") });
  }
  // 组装链：连续 if/else 段归一条链（段间无其他指令）
  let chain: IfChain | null = null;
  for (const seg of ifSegments) {
    if (!seg.isElse && chain) ifChains.push(chain);
    if (!seg.isElse) chain = { branches: [] };
    const comparisons = parseComparisons(seg.expr, allKeys);
    chain!.branches.push({ exprText: seg.expr || null, isElse: seg.isElse, comparisons });
  }
  if (chain) ifChains.push(chain);

  // ── 2) @for ──
  while ((m = forRe.exec(view)) !== null) {
    const item = m[1];
    const collectionRaw = m[2].trim();
    // 找该 @for 体后是否有 @empty
    const after = view.slice(forRe.lastIndex);
    const hasEmpty = /@empty\s*\{/.test(after.split(/\}@for|@if/)[0]);
    const viaTransform = /[a-zA-Z_]\s*\(/.test(collectionRaw);
    loops.push({ collectionText: collectionRaw, hasEmpty, viaTransform });
    void item;
  }

  // ── 3) 插值 / bind / class / style 计数 ──
  const interpRe = /\{([^{}]+)\}/g;
  while ((m = interpRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[1].includes(key)) uses[key].interpolations++;
    }
  }
  const bindRe = /bind:(\w+)=\{([^}]+)\}/g;
  while ((m = bindRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[2].includes(key)) uses[key].binds.push(m[1]);
    }
  }
  const classGateRe = /class:\s*\{([^}]*)\}/g;
  while ((m = classGateRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[1].includes(key)) uses[key].classGates++;
    }
  }
  const styleRe = /style:\s*\{([^}]*)\}/g;
  while ((m = styleRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[1].includes(key)) uses[key].styleRefs++;
    }
  }

  return { ifChains, loops, uses };
}

/** 提取 computed 表达式中的信号依赖（computed x: T = expr;） */
function scanComputedDeps(source: string, signalKeys: string[]): Record<string, number> {
  const deps: Record<string, number> = {};
  for (const key of signalKeys) deps[key] = 0;
  const computedRe = /computed\s+\w+\s*:\s*[^=]+=\s*([^;]+);/g;
  let m: RegExpExecArray | null;
  while ((m = computedRe.exec(source)) !== null) {
    for (const key of signalKeys) {
      if (m[1].includes(key)) deps[key]++;
    }
  }
  return deps;
}

export function analyzeSignalUsage(
  componentSource: string,
  signals: SignalDecl[],
  storeSignals: SignalDecl[],
): { interfaceSignals: SignalUsage[]; dataSignals: SignalUsage[] } {
  const view = extractViewBlock(componentSource);
  const signalKeys = signals.map((s) => s.key);
  const storeKeys = storeSignals.map((s) => s.key);
  const allKeys = [...signalKeys, ...storeKeys];
  const allDecls: SignalDecl[] = [...signals, ...storeSignals];

  const { ifChains, loops, uses } = scanView(view, signalKeys, storeKeys);
  const computedDeps = scanComputedDeps(componentSource, signalKeys);

  const usages: SignalUsage[] = allDecls.map((decl) => {
    const u = uses[decl.key] || { interpolations: 0, binds: [], classGates: 0, styleRefs: 0 };
    return {
      key: decl.key,
      declaredType: decl.type,
      conditions: ifChains.filter((chain) =>
        chain.branches.some((b) => b.comparisons.some((c) => c.sig === decl.key)),
      ),
      loops: loops.filter((l) => l.collectionText.includes(decl.key)),
      uses: {
        interpolations: u.interpolations,
        binds: u.binds,
        classGates: u.classGates,
        styleRefs: u.styleRefs,
        computedDeps: computedDeps[decl.key] || 0,
      },
    };
  });

  const interfaceSignals = usages.filter((u) => u.conditions.length > 0 || u.loops.length > 0);
  const dataSignals = usages.filter((u) => u.conditions.length === 0 && u.loops.length === 0);
  return { interfaceSignals, dataSignals };
}
```

> 注：`scanView` 中 `fullChainRe`/`chainRe`/`tokens` 为未使用草稿变量，实现时删除（保持 lint 干净）。`@if ... mode: "fade"` 形式由 `ifRe` 兼容（`mode:` 也匹配 `[^{]*?` 直到 `{`）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/services/debug/__tests__/signal-analysis.test.ts --reporter=verbose`
Expected: PASS（8 个用例）。若 @else 分支归属断言失败，调整 `scanView` 的链组装（以测试为契约）。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/debug/signal-analysis.ts poiesis/src/services/debug/__tests__/signal-analysis.test.ts
git commit -m "feat(poiesis): analyzeSignalUsage 信号分析器——view 树聚合多 @if 链/@for/用途计数（Task 6/11）"
```

---

### Task 7: useWasmCompile 接线（新管线 + 总开关 + 删旧字段）

**Files:**
- Modify: `poiesis/src/components/design/useWasmCompile.ts`（44、116-121、198 行附近）
- Modify: `poiesis/src/store/types.ts`（删除 debugSignalValues/setDebugSignalValue）
- Modify: `poiesis/src/store/index.ts`（删除 debugSignalValues 初始化）
- Modify: `poiesis/src/store/slices/component-lifecycle.ts`（删除旧 setDebugSignalValue 实现）

**Interfaces:**
- Consumes: `applyDebugOverrides`/`OverrideCtx`（Task 3）、`parseDebugValue`（Task 2）、`DebugValueEntry`（Task 1）
- Produces: 无（内部改造）

- [ ] **Step 1: 写失败测试**

`poiesis/src/components/design/__tests__/useWasmCompile-debug.test.ts`：

```ts
// useWasmCompile 调试覆盖接线：debugValues → 文件级覆盖 → WASM 编译输入。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

const { mockResolve } = vi.hoisted(() => ({ mockResolve: vi.fn() }));

vi.mock("../../commands/rustCompiler", () => ({
  rustCompiler: {
    isReady: true,
    resolveStylesMultiCached: (sources: Record<string, string>) => mockResolve(sources),
  },
}));

vi.mock("../../services/codegen/pipeline", () => ({
  generateOmniFiles: () => ({
    value: {
      "src/main.omni": 'signal status: String = "loading";\ncomponent main {\n  signal status: String = "loading";\n  view { <p>{status}</p> }\n}',
    },
    errors: [],
  }),
  generateOmniFragment: () => ({ value: null, errors: [] }),
}));

vi.mock("../../services/codegen/design-emitter", () => ({ emitDesignSource: () => null }));

vi.mock("../../store", () => ({
  usePoiesisStore: (sel: (s: unknown) => unknown) => {
    const state = {
      elements: [{ id: "e1", nodeType: "text", tag: "p", parentId: null, props: {}, source: "", position: { x: 0, y: 0 } }],
      wires: [],
      activeDesign: null,
      componentDefs: {},
      codeFlowVisible: false,
      debugValues: {
        main: { status: { text: '"error"', valid: true } },
      },
      debugOverridesEnabled: true,
      setPreviewOutput: () => {},
    };
    return sel(state);
  },
  useElements: () => [{ id: "e1", nodeType: "text", tag: "p", parentId: null, props: {}, source: "", position: { x: 0, y: 0 } }],
  useRootOrder: () => [],
  useComponentDefs: () => ({}),
}));

describe("useWasmCompile 调试覆盖接线", () => {
  beforeEach(() => mockResolve.mockClear());

  it("debugValues 覆盖后的源码传给 WASM（status → error）", async () => {
    renderHook(() => {
      const { useWasmCompile } = require("../useWasmCompile") as typeof import("../useWasmCompile");
      return useWasmCompile();
    });
    await act(async () => { await new Promise((r) => setTimeout(r, 200)); });
    const sources = mockResolve.mock.calls[0]?.[0] as Record<string, string>;
    expect(sources["src/main.omni"]).toContain('signal status: String = "error";');
  });

  it("总开关 off → 调试值不参与覆盖", async () => {
    vi.mocked(usePoiesisStore).mockImplementationOnce((sel) =>
      sel({ elements: [], wires: [], activeDesign: null, componentDefs: {}, codeFlowVisible: false, debugValues: { main: { status: { text: '"error"', valid: true } } }, debugOverridesEnabled: false, setPreviewOutput: () => {} }),
    );
    renderHook(() => {
      const { useWasmCompile } = require("../useWasmCompile") as typeof import("../useWasmCompile");
      return useWasmCompile();
    });
    await act(async () => { await new Promise((r) => setTimeout(r, 200)); });
    const sources = mockResolve.mock.calls[0]?.[0] as Record<string, string>;
    expect(sources["src/main.omni"]).toContain('signal status: String = "loading";');
  });
});
```

> 注：mock store 时若 `usePoiesisStore` 为 hook 调用形式无法 `mockImplementationOnce`，则第二个用例改为通过 zustand `usePoiesisStore.setState` 切总开关后再触发重渲染（renderHook 包一层 wrapper 订阅）。以实际 store 形态为准调整，契约不变：**总开关 off 时覆盖不生效**。

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/useWasmCompile-debug.test.ts --reporter=verbose`
Expected: FAIL（useWasmCompile 仍用旧 debugSignalValues，覆盖不生效）

- [ ] **Step 3: 实现**

`poiesis/src/components/design/useWasmCompile.ts`：

```ts
// 44 行：usePoiesisStore((s) => s.debugSignalValues) → debugValues + debugOverridesEnabled
const debugValues = usePoiesisStore((s) => s.debugValues);
const debugOverridesEnabled = usePoiesisStore((s) => s.debugOverridesEnabled);
```

```ts
// 116-121 行：替换 applyDebugSignalOverrides 调用块
const effectiveSources =
  debugOverridesEnabled && Object.keys(debugValues).length > 0
    ? applyDebugOverrides(sources, debugValues, {
        componentName: "main",
        storePaths: buildStorePaths(componentDefs, inlineStoreDefs, sources),
      })
    : sources;
```

新增辅助（文件内）：

```ts
/** 独立 store 名 → 文件路径（优先 StoreDefinition.filePath，兜底 src/stores/<Name>.omni） */
function buildStorePaths(
  componentDefs: Record<string, unknown>,
  inlineStoreDefs: Record<string, unknown>,
  sources: Record<string, string>,
): Record<string, string> {
  const paths: Record<string, string> = {};
  // 从 sources key 反推：src/stores/<Name>.omni → <Name>
  for (const path of Object.keys(sources)) {
    const m = path.match(/^src\/stores\/(.+)\.omni$/);
    if (m) paths[m[1]] = path;
  }
  return paths;
}
```

依赖：useWasmCompile 顶部 import 改为：

```ts
import { applyDebugOverrides } from "../../services/debug/debug-overrides";
```

effect 依赖数组（198 行）加 `debugOverridesEnabled`。删除文件内旧的 `applyDebugSignalOverrides`/`escapeRegex`/`formatDebugValue`（209-260 行）。

删除旧字段：
- `poiesis/src/store/types.ts`：删 `debugSignalValues`（75 行）与 `setDebugSignalValue`（189 行）
- `poiesis/src/store/index.ts`：删 `debugSignalValues: {}`（36 行）
- `poiesis/src/store/slices/component-lifecycle.ts`：删旧 `setDebugSignalValue` 实现（93-103 行）
- 全局 grep 确认无残留引用（`debugSignalValues` / `setDebugSignalValue` / `applyDebugSignalOverrides`）

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/useWasmCompile-debug.test.ts --reporter=verbose && npx vitest run src/store/__tests__/debug-values.test.ts --reporter=verbose`
Expected: PASS（2 个接线用例 + 4 个 store 用例）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/design/useWasmCompile.ts poiesis/src/store/types.ts poiesis/src/store/index.ts poiesis/src/store/slices/component-lifecycle.ts poiesis/src/components/design/__tests__/useWasmCompile-debug.test.ts
git commit -m "feat(poiesis): useWasmCompile 接线新覆盖管线——总开关门控 + 删旧 debugSignalValues 字段（Task 7/11）"
```

---

### Task 8: ComponentDefPanel 三 Tab 骨架 + 定义 Tab 迁移

**Files:**
- Modify: `poiesis/src/components/design/ComponentDefPanel.tsx`（结构重构）
- Create: `poiesis/src/components/design/ComponentDefPanel.css`（样式从 S 对象迁出，三 Tab 布局）
- Test: `poiesis/src/components/design/__tests__/ComponentDefPanel.test.tsx`（新建）

**Interfaces:**
- Consumes: `usePoiesisStore`（debugValues/clearDebugValues/setDebugOverridesEnabled，Task 1）、`useCanvasStore`（panelPosition/isCollapsed，现有）
- Produces: `DebugSignalRow`（信号调试 Tab 行，Task 9 消费）、`InterfaceSignalCard`（界面信号调试卡，Task 10 消费）——本任务先渲染 Tab 骨架与定义 Tab，行/卡组件由后续任务填充

- [ ] **Step 1: 写失败测试**

`poiesis/src/components/design/__tests__/ComponentDefPanel.test.tsx`：

```ts
// ComponentDefPanel 三 Tab 骨架：默认界面信号调试 + Tab 切换 + 定义 Tab 内容。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ComponentDefPanel } from "../ComponentDefPanel";

// mock store：最小形态
const { mockState } = vi.hoisted(() => ({
  mockState: {
    activeTabId: "t1",
    openTabs: [{ id: "t1", componentName: "Card", filePath: "Card.omni", tabType: "component" }],
    componentDefs: { Card: { signals: [{ pid: "s1", name: "count", type: "i32", defaultValue: "0" }], props: [], computeds: [] } },
    debugValues: {},
    debugOverridesEnabled: true,
    clearDebugValues: vi.fn(),
    setDebugOverridesEnabled: vi.fn(),
  },
}));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (sel: (s: unknown) => unknown) => sel(mockState),
}));

vi.mock("../../../canvas/canvas-store", () => ({
  useCanvasStore: (sel: (s: unknown) => unknown) =>
    sel({ panelPosition: { x: 400, y: 20 }, isPanelCollapsed: false, toggleCollapsed: vi.fn(), setPanelPosition: vi.fn() }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ComponentDefPanel 三 Tab 骨架", () => {
  it("默认显示界面信号调试 Tab", () => {
    render(<ComponentDefPanel />);
    expect(screen.getByRole("tab", { name: /界面信号调试/ })).toHaveAttribute("aria-selected", "true");
  });

  it("Tab 切换：点「定义」显示 Props/Signals/Computed 段", () => {
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /定义/ }));
    expect(screen.getByRole("tab", { name: /定义/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("count")).toBeTruthy();
  });

  it("Header 显示组件名与用途徽章", () => {
    render(<ComponentDefPanel />);
    expect(screen.getByText("Card")).toBeTruthy();
  });

  it("总开关 🔧 点击切换 setDebugOverridesEnabled", () => {
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByTitle(/调试/));
    expect(mockState.setDebugOverridesEnabled).toHaveBeenCalledWith(false);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/ComponentDefPanel.test.tsx --reporter=verbose`
Expected: FAIL（无 Tab 结构）

- [ ] **Step 3: 实现**

`ComponentDefPanel.tsx` 重构（保留 `S` 对象样式与既有子组件 `DefSection`/`DefEntryRow`/`ComputedEntryRow`/`AddForm`，改动点）：

1. 顶部 import 追加 `useState` 已有；新增 Tab 常量：
```ts
type PanelTab = "interface" | "signals" | "definitions";
const [tab, setTab] = useState<PanelTab>("interface");
```

2. 替换返回 JSX 为三 Tab 结构（Header 保留拖拽 + 🔧 + 折叠）：

```tsx
  // 总开关（🔧）：on → 调试值生效；off → 暂停（保留输入）
  const debugOverridesEnabled = usePoiesisStore((s) => s.debugOverridesEnabled);
  const setDebugOverridesEnabled = usePoiesisStore((s) => s.setDebugOverridesEnabled);
  const clearDebugValues = usePoiesisStore((s) => s.clearDebugValues);
  // 用途计数徽章：界面信号数（Task 10 填充真实计数，先显示信号总数）
  const totalSignals = signals.length + props.length;

  return (
    <div style={{ ...S.panel, left: panelPosition.x, top: panelPosition.y }}
      onPointerDown={(e) => e.stopPropagation()}>
      {/* Header */}
      <div style={S.panelHeader} onPointerDown={handlePointerDown} onPointerMove={handlePointerMove} onPointerUp={handlePointerUp}>
        <span style={S.panelHeaderTitle}>{componentName}</span>
        <button type="button" style={S.debugToggle(debugOverridesEnabled)} onClick={() => setDebugOverridesEnabled(!debugOverridesEnabled)}
          title="调试总开关：off 时调试值不参与编译（对比原始渲染）">
          {debugOverridesEnabled ? "🔧 ON" : "🔧 OFF"}
        </button>
        <button type="button" style={S.collapseBtn} onClick={toggleCollapsed}>{isCollapsed ? "□" : "－"}</button>
      </div>

      {!isCollapsed && (
        <>
          {/* Tab 栏 */}
          <div style={S.tabBar} role="tablist">
            {([
              ["interface", "界面信号调试"],
              ["signals", "信号调试"],
              ["definitions", "定义"],
            ] as Array<[PanelTab, string]>).map(([id, label]) => (
              <button key={id} type="button" role="tab" aria-selected={tab === id}
                style={S.tabButton(tab === id)} onClick={() => setTab(id)}>
                {label}
              </button>
            ))}
          </div>

          {/* Tab 内容 */}
          {tab === "definitions" ? (
            <div style={S.panelBody}>
              <DefSection title="Props" ... />   {/* 现内容原样迁入 */}
              <DefSection title="Signals" ... />
              <DefSection title="Computed" ... />
            </div>
          ) : tab === "signals" ? (
            <div style={S.panelBody} data-testid="signals-tab">
              <p style={S.placeholder}>信号调试（Task 9 实现）</p>
            </div>
          ) : (
            <div style={S.panelBody} data-testid="interface-tab">
              <p style={S.placeholder}>界面信号调试（Task 10 实现）</p>
            </div>
          )}
        </>
      )}
    </div>
  );
```

3. `S` 对象新增（样式对象内）：

```ts
  tabBar: {
    display: "flex", borderBottom: "1px solid var(--ol-border)", background: "var(--ol-gray-50)",
  } as React.CSSProperties,
  tabButton: (active: boolean): React.CSSProperties => ({
    flex: 1, padding: "6px 4px", fontSize: "var(--ol-font-size-11)", fontWeight: "var(--ol-font-weight-600)",
    border: "none", borderBottom: active ? "2px solid var(--ol-color-primary)" : "2px solid transparent",
    background: "none", color: active ? "var(--ol-color-primary)" : "var(--ol-text-secondary)",
    cursor: "pointer", whiteSpace: "nowrap",
  }),
  placeholder: { padding: "16px 12px", color: "var(--ol-text-muted)", fontSize: "var(--ol-font-size-11)", fontStyle: "italic" },
```

> 注：调试行（signal 行内 debugValueInput 逻辑）在本任务从 `DefEntryRow` 中**移除**（`debugMode`/`showDebugInput` 相关 props 与代码删除）——调试输入不再存在于定义 Tab，改由 Task 9/10 的专用行承载。同时删除 `debugMode` state 与 `debugSignalValues` 引用（Task 7 已删字段）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/ComponentDefPanel.test.tsx --reporter=verbose`
Expected: PASS（4 个用例）。再跑全量 `npx vitest run` 确认无回归。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/design/ComponentDefPanel.tsx poiesis/src/components/design/__tests__/ComponentDefPanel.test.tsx
git commit -m "feat(poiesis): ComponentDefPanel 三 Tab 骨架——默认界面信号调试 + 定义 Tab 迁移 + 总开关（Task 8/11）"
```

---

### Task 9: 信号调试 Tab（数据信号行）

**Files:**
- Modify: `poiesis/src/components/design/ComponentDefPanel.tsx`（signals tab 分支）
- Modify: `poiesis/src/components/design/ComponentDefPanel.css`（行样式）
- Test: `poiesis/src/components/design/__tests__/ComponentDefPanel.test.tsx`（追加）

**Interfaces:**
- Consumes: `parseDebugValue`（Task 2）、`setDebugValue`/`debugValues`（Task 1）、`analyzeSignalUsage`（Task 6，本任务用于取 dataSignals 列表）
- Produces: 无

- [ ] **Step 1: 追加失败测试**

`ComponentDefPanel.test.tsx` 追加：

```ts
describe("信号调试 Tab（数据信号行）", () => {
  it("列出数据信号：名称/类型/输入框；输入 → setDebugValue", () => {
    mockState.debugValues = {};
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /信号调试/ }));
    // mock 组件只有 count（i32，纯数据）
    const input = screen.getByLabelText("count 值");
    fireEvent.change(input, { target: { value: "5" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "count", "5", true);
  });

  it("非法输入 → 标红且 valid=false 传给 store", () => {
    mockState.debugValues = {};
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /信号调试/ }));
    fireEvent.change(screen.getByLabelText("count 值"), { target: { value: "abc" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "count", "abc", false);
    expect(screen.getByLabelText("count 值").className).toContain("invalid");
  });

  it("回显已有调试值", () => {
    mockState.debugValues = { Card: { count: { text: "7", valid: true } } };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /信号调试/ }));
    expect((screen.getByLabelText("count 值") as HTMLInputElement).value).toBe("7");
  });

  it("重置全部按钮 → clearDebugValues", () => {
    mockState.debugValues = { Card: { count: { text: "7", valid: true } } };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /信号调试/ }));
    fireEvent.click(screen.getByRole("button", { name: /重置全部/ }));
    expect(mockState.clearDebugValues).toHaveBeenCalled();
  });
});
```

> 注：mockState 需追加 `setDebugValue: vi.fn()`（测试中通过 `(mockState.setDebugValue as Mock)` 断言）。信号分析在此 Tab 只取 `dataSignals`——测试中组件无 @if/@for，count 归数据信号。

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/ComponentDefPanel.test.tsx --reporter=verbose`
Expected: FAIL（signals tab 是占位）

- [ ] **Step 3: 实现**

`ComponentDefPanel.tsx` 的 signals tab 分支替换为：

```tsx
import { parseDebugValue } from "../../services/debug/parse-debug-value";
import { analyzeSignalUsage } from "../../services/debug/signal-analysis";
import type { SignalUsage } from "../../services/debug/signal-analysis";
```

组件内（渲染前）：

```tsx
  // 数据信号列表：分析组件 view 源码（本任务只取 dataSignals；源码为空时退化为全量信号）
  const { dataSignals } = useMemo(() => {
    const signalDecls = signals.map((s) => ({ key: s.name, type: s.type }));
    const storeDecls: Array<{ key: string; type: string }> = [];
    // 组件 store 引用的 store 信号（Task 10 完善 store 信号收集，本任务先空）
    return analyzeSignalUsage("", signalDecls, storeDecls);
  }, [signals]);

  const signalRows = dataSignals.length > 0 ? dataSignals
    : signals.map((s) => ({ key: s.name, declaredType: s.type, conditions: [], loops: [], uses: { interpolations: 0, binds: [], classGates: 0, styleRefs: 0, computedDeps: 0 } }));
```

渲染：

```tsx
          ) : tab === "signals" ? (
            <div style={S.panelBody} data-testid="signals-tab">
              <div style={S.tabHeaderRow}>
                <span style={S.tabHeaderTitle}>🎛 信号调试</span>
                <button type="button" style={S.resetBtn} onClick={clearDebugValues}>↺ 重置全部</button>
              </div>
              {signalRows.map((u) => (
                <SignalDebugRow key={u.key} usage={u} />
              ))}
              {signalRows.length === 0 && <p style={S.placeholder}>无数据信号</p>}
            </div>
```

新增子组件（文件内，Task 9）：

```tsx
interface SignalDebugRowProps {
  usage: SignalUsage;
}

function SignalDebugRow({ usage }: SignalDebugRowProps) {
  const setDebugValue = usePoiesisStore((s) => s.setDebugValue);
  const debugValues = usePoiesisStore((s) => s.debugValues);
  const entry = debugValues[usage.key] ?? debugValues[usage.key.split(".").pop()!];
  const text = entry?.text ?? "";
  const valid = entry ? entry.valid : true;

  const handleChange = (value: string) => {
    const parsed = parseDebugValue(usage.declaredType, value);
    setDebugValue(usage.key, usage.key, parsed.literal, parsed.valid);
    void valid;
  };

  const isNumeric = ["i32", "i64", "f64", "f32"].includes(usage.declaredType);

  return (
    <div style={S.debugRow}>
      <span style={S.debugName}>{usage.key}</span>
      <span style={S.debugType}>{usage.declaredType}</span>
      <input
        aria-label={`${usage.key} 值`}
        style={{ ...S.debugValueInput, ...(entry && !entry.valid ? S.invalidInput : {}) }}
        className={entry && !entry.valid ? "invalid" : undefined}
        value={text}
        placeholder="默认值"
        onChange={(e) => handleChange(e.target.value)}
      />
      {isNumeric && (
        <button type="button" style={S.stepBtn} onClick={() => {
          const cur = text === "" ? "0" : text;
          const next = String(Number(cur) + 1);
          handleChange(next);
        }}>＋</button>
      )}
    </div>
  );
}
```

`S` 追加：

```ts
  tabHeaderRow: { display: "flex", alignItems: "center", justifyContent: "space-between", padding: "8px 12px" },
  tabHeaderTitle: { fontSize: "var(--ol-font-size-11)", fontWeight: "var(--ol-font-weight-600)", color: "var(--ol-text-secondary)", textTransform: "uppercase", letterSpacing: "0.05em" },
  resetBtn: { ...S.addBtn },
  debugRow: { display: "flex", alignItems: "center", gap: "var(--ol-space-1p5)", padding: "5px 12px", borderTop: "1px solid var(--ol-border-subtle)", fontSize: "var(--ol-font-size-12)" },
  debugName: { fontWeight: "var(--ol-font-weight-600)", flexShrink: 0, maxWidth: 96, overflow: "hidden", textOverflow: "ellipsis" },
  debugType: { color: "var(--ol-text-secondary)", fontSize: "var(--ol-font-size-11)", flexShrink: 0 },
  invalidInput: { borderColor: "var(--ol-color-danger)", background: "var(--ol-color-danger-subtle)" },
  stepBtn: { ...S.addBtn, padding: "0 6px" },
```

> 注：`setDebugValue(usage.key, usage.key, ...)` 的 ownerKey 即信号 key（组件信号 key=名字；store 信号 key="Store.sig"——store 信号归 Task 10 的界面卡，本任务行仅组件信号，ownerKey 与 name 相同）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/ComponentDefPanel.test.tsx --reporter=verbose`
Expected: PASS（4 骨架 + 4 新用例）。全量 `npx vitest run` 无回归。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/design/ComponentDefPanel.tsx poiesis/src/components/design/__tests__/ComponentDefPanel.test.tsx
git commit -m "feat(poiesis): 信号调试 Tab——数据信号行 + 类型校验 + 步进 + 重置全部（Task 9/11）"
```

---

### Task 10: 界面信号调试 Tab（信号卡 + 分支胶囊 + 列表）

**Files:**
- Modify: `poiesis/src/components/design/ComponentDefPanel.tsx`（interface tab 分支）
- Test: `poiesis/src/components/design/__tests__/ComponentDefPanel.test.tsx`（追加）
- Test: `poiesis/src/services/debug/__tests__/signal-analysis.test.ts`（如发现归属边界问题在此补）

**Interfaces:**
- Consumes: `analyzeSignalUsage`（Task 6，interfaceSignals）、`synthesizeConditionValue`（Task 4）、`extractAssignSamples`（Task 5）、`parseDebugValue`（Task 2）、`setDebugValue`/`debugValues`/`debugOverridesEnabled`（Task 1）
- Produces: 无（UI 收口）

- [ ] **Step 1: 追加失败测试**

`ComponentDefPanel.test.tsx` 追加：

```ts
describe("界面信号调试 Tab（信号卡）", () => {
  it("界面信号卡：@if 分支胶囊 + 条件文本 + 点选驱动合成值", () => {
    // mock 组件 view 有 @if status == "loading" / @else → analyzeSignalUsage 产出分支
    mockState.componentDefs = {
      Card: {
        signals: [{ pid: "s1", name: "status", type: "String", defaultValue: '"x"' }],
        props: [], computeds: [],
      },
    };
    render(<ComponentDefPanel />);
    // 默认即界面信号调试 Tab
    expect(screen.getByTestId("interface-tab")).toBeTruthy();
    expect(screen.getByText(/status/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /loading/ }));
    expect(mockState.setDebugValue).toHaveBeenCalledWith("status", "status", "loading", true);
  });

  it("复杂条件分支胶囊置灰", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "items", type: "Array<String>", defaultValue: "[]" }], props: [], computeds: [] },
    };
    render(<ComponentDefPanel />);
    const btn = screen.getByRole("button", { name: /len\(\)/i });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });

  it("总开关 off → 分支胶囊禁用", () => {
    mockState.debugOverridesEnabled = false;
    render(<ComponentDefPanel />);
    expect((screen.getByRole("button", { name: /loading/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("列表卡：@for 信号 + 空数组候选 + 手动输入", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "items", type: "Array<String>", defaultValue: "[]" }], props: [], computeds: [] },
    };
    render(<ComponentDefPanel />);
    expect(screen.getByText(/@for/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /空/ }));
    expect(mockState.setDebugValue).toHaveBeenCalledWith("items", "items", "[]", true);
  });
});
```

> 注：mock 里组件源码为空串时 `analyzeSignalUsage("", ...)` 产出空 interfaceSignals——测试需让分析器拿到 view 源码。实现时 ComponentDefPanel 需要**当前组件的生成源码**：从 `useWasmCompile` 拿不到（它内部 state），改为从 `generateOmniFragment` 在面板内惰性生成（仅当 tab==="interface" 且 componentDefs[componentName] 有元素时），或复用 `DesignPage` 的 `buildCodeSource` 思路。测试通过 `mockState.elements`/`componentDefs` 提供数据，面板内调用 `generateOmniFragment` 生成组件源码喂给分析器。若生成链在测试环境过重，ComponentDefPanel 改为接收可注入的 `getComponentSource?: () => string` 属性（DesignPage 传 `buildCodeSource` 复用），测试传 fixture 源码——**以可测性为准，二选一**（计划默认：面板内调 `generateOmniFragment`，mock 其 pipeline 依赖）。

- [ ] **Step 2: 跑测试确认失败**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/ComponentDefPanel.test.tsx --reporter=verbose`
Expected: FAIL（interface tab 是占位）

- [ ] **Step 3: 实现**

`ComponentDefPanel.tsx` interface tab 分支替换为：

```tsx
  // 界面信号分析：组件源码 → interfaceSignals（含多 @if 链 / @for / 用途徽章）
  const { interfaceSignals } = useMemo(() => {
    const signalDecls = signals.map((s) => ({ key: s.name, type: s.type }));
    // store 信号：@store 引用 + 内联 store（ownerKey qualified）
    const storeRefs: Array<{ key: string; type: string }> = [];
    for (const [storeKey, storeDef] of Object.entries(inlineStoreDefs)) {
      for (const sig of storeDef.signals || []) {
        storeRefs.push({ key: `${storeKey}.${sig.name}`, type: sig.type });
      }
    }
    for (const [storeName, storeDef] of Object.entries(storeDefs)) {
      for (const sig of storeDef.signals || []) {
        storeRefs.push({ key: `${storeName}.${sig.name}`, type: sig.type });
      }
    }
    const src = getComponentSource();
    return analyzeSignalUsage(src, signalDecls, storeRefs);
  }, [signals, inlineStoreDefs, storeDefs, getComponentSource]);
```

界面卡渲染：

```tsx
          ) : (
            <div style={S.panelBody} data-testid="interface-tab">
              <div style={S.tabHeaderRow}>
                <span style={S.tabHeaderTitle}>🎛 界面信号调试</span>
                <button type="button" style={S.resetBtn} onClick={clearDebugValues}>↺ 重置全部</button>
              </div>
              {interfaceSignals.map((u) => (
                <InterfaceSignalCard key={u.key} usage={u} />
              ))}
              {interfaceSignals.length === 0 && <p style={S.placeholder}>无界面驱动信号（@if/@for 未用信号）</p>}
            </div>
```

新增子组件：

```tsx
interface InterfaceSignalCardProps {
  usage: SignalUsage;
}

function InterfaceSignalCard({ usage }: InterfaceSignalCardProps) {
  const setDebugValue = usePoiesisStore((s) => s.setDebugValue);
  const debugValues = usePoiesisStore((s) => s.debugValues);
  const debugOverridesEnabled = usePoiesisStore((s) => s.debugOverridesEnabled);
  const entry = debugValues[usage.key];
  const text = entry?.text ?? "";
  const valid = entry ? entry.valid : true;

  const applyValue = (raw: string) => {
    const parsed = parseDebugValue(usage.declaredType, raw);
    setDebugValue(usage.key, usage.key, parsed.literal, parsed.valid);
  };

  // 分支胶囊：单信号简单比较可合成；复杂置灰
  const branchButtons = usage.conditions.flatMap((chain, chainIdx) =>
    chain.branches.map((branch, branchIdx) => {
      const cmp = branch.comparisons[0];
      const synthesizable = cmp !== undefined && synthesizeConditionValue(cmp, "true").valid;
      const label = branch.isElse ? "else" : branch.exprText || "?";
      const handleClick = () => {
        if (!cmp || !debugOverridesEnabled) return;
        const result = synthesizeConditionValue(cmp, "true");
        if (!result.valid) return;
        // 字面量按声明类型补引号（String 信号）
        const raw = usage.declaredType === "String" ? `"${result.literal}"` : result.literal;
        applyValue(raw);
      };
      return (
        <button key={`${chainIdx}-${branchIdx}`} type="button"
          style={S.branchChip(valid && entry !== undefined)}
          disabled={!synthesizable || !debugOverridesEnabled}
          title={!synthesizable ? "复杂条件，请在输入框手动调" : `驱动该分支成真`}
          onClick={handleClick}>
          {label}
        </button>
      );
    }),
  );

  // 列表胶囊：空数组 + 代码样本（struct 数组）+ 手动输入
  const samples = usage.loops.length > 0 && /Array</.test(usage.declaredType)
    ? extractAssignSamples(fnSourcesFor(usage), usage.key.split(".").pop()!)
    : [];

  const badges = [
    usage.conditions.length > 0 ? `条件 ${usage.conditions.length}` : "",
    usage.loops.length > 0 ? `列表 ${usage.loops.length}` : "",
    usage.uses.interpolations > 0 ? `插值 ${usage.uses.interpolations}` : "",
    usage.uses.computedDeps > 0 ? `computed ${usage.uses.computedDeps}` : "",
  ].filter(Boolean).join(" · ");

  return (
    <div style={S.signalCard}>
      <div style={S.signalCardHeader}>
        <span style={S.debugName}>{usage.key}</span>
        <span style={S.debugType}>{usage.declaredType}</span>
        {badges && <span style={S.badgeText}>{badges}</span>}
      </div>

      {usage.conditions.length > 0 && (
        <div style={S.branchGroup}>
          {usage.conditions.map((chain, i) => (
            <div key={i} style={S.branchRow}>
              <span style={S.chainPrefix}>@if</span>
              {chain.branches.map((b, j) => (
                <span key={j} style={S.branchChipWrap}>{branchButtons[i * chain.branches.length + j]}</span>
              ))}
            </div>
          ))}
        </div>
      )}

      {usage.loops.length > 0 && (
        <div style={S.loopGroup}>
          {usage.loops.map((loop, i) => (
            <div key={i} style={S.loopRow}>
              <span style={S.loopText}>@for in {loop.collectionText}</span>
              {loop.hasEmpty && <span style={S.loopEmptyTag}>@empty</span>}
              {loop.viaTransform && <span style={S.loopTransformTag}>经转换</span>}
            </div>
          ))}
          <div style={S.chipRow}>
            <button type="button" style={S.quickChip} disabled={!debugOverridesEnabled}
              onClick={() => applyValue("[]")}>空</button>
            {samples.map((s) => (
              <button key={s.label} type="button" style={S.quickChip} disabled={!debugOverridesEnabled}
                onClick={() => setDebugValue(usage.key, usage.key, s.literal, true)}>
                {s.label}
              </button>
            ))}
          </div>
        </div>
      )}

      <div style={S.debugRow}>
        <span style={S.valueLabel}>值</span>
        <input
          aria-label={`${usage.key} 值`}
          style={{ ...S.debugValueInput, ...(entry && !entry.valid ? S.invalidInput : {}) }}
          className={entry && !entry.valid ? "invalid" : undefined}
          value={text}
          placeholder="默认值"
          onChange={(e) => applyValue(e.target.value)}
        />
        {!valid && <span style={S.invalidHint}>⚠ 格式不符</span>}
      </div>
    </div>
  );
}
```

辅助：`fnSourcesFor(usage)` —— 从组件元素中收集 fn 节点源码（`elements.filter((e) => e.nodeType === "fn").map((e) => "source" in e ? String(e.source) : "")`），信号 key 剥 store 前缀。`getComponentSource` —— 面板内惰性生成组件源码：

```tsx
  const getComponentSource = useCallback(() => {
    const compDef = componentDefs[componentName];
    if (!compDef?.elements?.length) return "";
    const result = generateOmniFragment({
      componentName, designElements: compDef.elements, designWires: compDef.wires || [],
      codeTabs: [{ id: "fragment", type: "logic", label: "Fragment", icon: "📄", graph: { elements: compDef.elements, wires: compDef.wires || [] }, variables: [] }],
      rootOrder: compDef.rootOrder, fromDefinition: true, inlineStores: inlineStoresFor(componentName),
    });
    return result.value ?? "";
  }, [componentDefs, componentName, inlineStoreDefs]);
```

`S` 追加样式：`signalCard`（卡片 border + 圆角 + 内边距 + 背景 `var(--ol-gray-50)`）、`signalCardHeader`、`badgeText`（小号 muted）、`branchGroup`/`branchRow`/`chainPrefix`（等宽 muted）、`branchChip`（胶囊：border 圆角 hover 主色，激活高亮）、`chipRow`/`quickChip`、`loopGroup`/`loopRow`/`loopText`（等宽小字）/`loopEmptyTag`/`loopTransformTag`、`valueLabel`、`invalidHint`（danger 色小字）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cd poiesis && npx vitest run src/components/design/__tests__/ComponentDefPanel.test.tsx --reporter=verbose`
Expected: PASS（8 骨架/信号 + 4 界面新用例）。全量 `npx vitest run` + `npx tsc --noEmit` 无回归无类型错误。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/design/ComponentDefPanel.tsx poiesis/src/components/design/__tests__/ComponentDefPanel.test.tsx
git commit -m "feat(poiesis): 界面信号调试 Tab——信号卡 + 分支胶囊（成真合成）+ 列表胶囊（空/样本）（Task 10/11）"
```

---

### Task 11: 全量验证与收尾

**Files:**
- 验证：`poiesis/src/services/debug/`、`ComponentDefPanel.tsx`、`useWasmCompile.ts`

**Interfaces:**
- Consumes: 全部任务产物

- [ ] **Step 1: 全量测试 + 类型 + lint**

Run（poiesis 目录）：
```bash
npx vitest run
npx tsc --noEmit
npx eslint src/services/debug src/components/design/ComponentDefPanel.tsx src/components/design/useWasmCompile.ts
```
Expected: 全部通过；新代码零告警。

- [ ] **Step 2: 残留清理检查**

```bash
grep -rn "debugSignalValues\|setDebugSignalValue\|applyDebugSignalOverrides" poiesis/src
```
Expected: 零匹配（Task 7 已删）。残留则清理。

- [ ] **Step 3: 手工验证（Victauri，可选——poiesis 运行中）**

启动 poiesis（如未运行），Victauri 打开任意含 @if 的组件（如 learn-omniview 导入后 NotificationsPanel），验证：
1. 默认 Tab = 界面信号调试；信号卡显示 @if 分支胶囊
2. 点分支胶囊 → 预览切换分支
3. 输入非法值 → 红框 + 不编译
4. 🔧 off → 预览回原始渲染；on → 恢复
5. 退出组件再进：调试值保留

- [ ] **Step 4: 提交（如有残留改动）**

```bash
git add -A
git commit -m "chore(poiesis): 调试面板收尾——全量验证通过"
```

---

## Self-Review（写完后执行）

1. **Spec 覆盖检查**：D1（store 全覆盖）→ Task 1/7/10 ✓；D2 类型感知 → Task 2 ✓；D3/D4 三 Tab 默认界面信号调试 → Task 8 ✓；D5/D6 分支胶囊成真合成 → Task 4/10 ✓；D7 信号为中心 → Task 6/10 ✓；D8 方案 A → 全计划（零 Rust 改动）✓；D9 输入框主角+字面量透传 → Task 2/9 ✓；D10 代码样本 → Task 5/10 ✓；D11 Store.sig 限定 → Task 4/6 ✓；D12 单分支二态 → Task 4（裸 bool 成真/成假）✓；总开关 → Task 1/7/8/10 ✓；错误处理表 → Task 2（校验）/4（invalid）/10（置灰）✓；测试计划 → Task 2-10 对应测试 ✓
2. **占位符扫描**：无 TBD/TODO；Task 10 的「以可测性为准二选一」有明确默认（面板内 generateOmniFragment + mock pipeline），非占位
3. **类型一致性**：`DebugValueEntry`（Task 1 定义，Task 3/7/9/10 消费）一致；`ConditionComparison`（Task 4 定义，Task 6 Produces）一致；`SignalUsage`（Task 6 定义，Task 9/10 消费）一致；`applyDebugOverrides(debugValues, ctx)` 签名跨 Task 3/7 一致
