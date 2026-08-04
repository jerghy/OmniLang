# CodeFlowCanvas 语法节点扩展实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 CodeFlowCanvas（节点编辑器）新增 4 种语句级语法节点（let / assign / store-call / call），贯穿 UI、编译、解析全链路，round-trip 稳定。

**Architecture:** 新增 `syntax-nodes.ts` 纯函数模块（props → 代码文本的单一来源 `formatSyntaxNode`），GraphCompiler 的 4 个 emit case 与 UI 预览共用它；GraphParser 扩展 `detectSegmentType` 语句分类 + 元素构建分支提取 props。`// @pid:xxx` 注释机制、`pipeline.ts`、`source-emitter.ts`、`parser.ts` 调用方零改动。

**Tech Stack:** TypeScript / React 19 / ReactFlow (@xyflow/react) / Zustand / vitest (jsdom) / @testing-library/react

**Spec:** `docs/superpowers/specs/2026-08-04-codeflow-syntax-nodes-design.md`

## Global Constraints

- 所有改动在 `poiesis/src` 下；测试命令 `cd poiesis && npx vitest run <path>`（include 模式 `src/**/__tests__/**/*.test.ts(x)`）
- 严格复用 `// @pid:xxx` 注释 round-trip 机制；**不**新增 `// @<type>` 类型注释（`annotation-registry.ts` 的 TYPE_REGEX/VALID_TYPES 不动）
- 不触碰遗留 operations/SlotValue 体系（`code-op.ts` / `code-op-parser.ts` 不改）
- 新节点 props 存结构化字段（`CodeblockElementData.props`），`source` 为派生缓存（解析时 = `formatSyntaxNode(tag, props)`）
- 新节点禁用双击打开 CodeBlockEditor（Monaco 编辑的是 source，GraphCompiler 从 props 生成代码，改 source 不生效）
- 提交信息格式 `feat:|test:|refactor:|docs:`，无 attribution 尾注（用户全局配置禁用）
- 每任务结束时运行 `npx vitest run <任务测试文件>`，确认无回归（全量在 Task 7）

---

### Task 1: syntax-nodes 纯函数模块

**Files:**
- Create: `poiesis/src/services/codegen/syntax-nodes.ts`
- Test: `poiesis/src/services/codegen/__tests__/syntax-nodes.test.ts`

**Interfaces:**
- Produces: `SYNTAX_NODE_TAGS: readonly ["let","assign","store-call","call"]`；`SyntaxNodeProps` 接口；`ASSIGN_OPERATORS: string[]`；`formatSyntaxNode(tag: string, props: SyntaxNodeProps): string`（props → 无缩进无注释的 OmniLang 语句文本，带尾分号）；`defaultSyntaxProps(tag: string): SyntaxNodeProps`（learn-omniview 占位默认值）

- [ ] **Step 1: 写失败测试**

```typescript
// poiesis/src/services/codegen/__tests__/syntax-nodes.test.ts
import { describe, it, expect } from "vitest";
import { formatSyntaxNode, defaultSyntaxProps, ASSIGN_OPERATORS } from "../syntax-nodes";

describe("formatSyntaxNode", () => {
  it("formats let without mut", () => {
    expect(formatSyntaxNode("let", { name: "x", mutable: false, value: "0" })).toBe("let x = 0;");
  });
  it("formats let mut with type annotation", () => {
    expect(formatSyntaxNode("let", { name: "items", mutable: true, varType: "Array<String>", value: "[]" })).toBe("let mut items: Array<String> = [];");
  });
  it("formats let without initial value", () => {
    expect(formatSyntaxNode("let", { name: "x" })).toBe("let x;");
  });
  it("formats assign with every operator", () => {
    for (const op of ASSIGN_OPERATORS) {
      expect(formatSyntaxNode("assign", { target: "count", operator: op, value: "1" })).toBe(`count ${op} 1;`);
    }
  });
  it("formats store-call", () => {
    expect(formatSyntaxNode("store-call", { storeName: "TodoStore", action: "add", args: "text" })).toBe("TodoStore.add(text);");
  });
  it("formats call without emit", () => {
    expect(formatSyntaxNode("call", { fnName: "close", args: "", emit: false })).toBe("close();");
  });
  it("formats call with emit prefix", () => {
    expect(formatSyntaxNode("call", { fnName: "onSelect", args: "item", emit: true })).toBe("emit onSelect(item);");
  });
});

describe("defaultSyntaxProps", () => {
  it("provides defaults that format to non-empty code for all four tags", () => {
    for (const tag of ["let", "assign", "store-call", "call"]) {
      expect(formatSyntaxNode(tag, defaultSyntaxProps(tag))).not.toBe("");
    }
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/syntax-nodes.test.ts`
Expected: FAIL（`Cannot find module '../syntax-nodes'`）

- [ ] **Step 3: 实现**

```typescript
// poiesis/src/services/codegen/syntax-nodes.ts
// 语法节点（let/assign/store-call/call）的 props 结构与代码生成。
// CodeFlowCanvas 中"一个语法一个节点"的语句级节点核心：
//  - formatSyntaxNode: props → OmniLang 语句文本（GraphCompiler 与 UI 预览共用，单一来源）
//  - defaultSyntaxProps: 新节点默认值（占位符取自 learn-omniview 真实使用统计）

export const SYNTAX_NODE_TAGS = ["let", "assign", "store-call", "call"] as const;
export type SyntaxNodeTag = (typeof SYNTAX_NODE_TAGS)[number];

export interface SyntaxNodeProps {
  /** let: 变量名；assign: 赋值目标（signal/变量/store 信号） */
  name?: string;
  mutable?: boolean;
  varType?: string;
  value?: string;
  target?: string;
  operator?: string;
  storeName?: string;
  action?: string;
  args?: string;
  emit?: boolean;
  fnName?: string;
}

export const ASSIGN_OPERATORS = ["=", "+=", "-=", "*=", "/=", "%="];

/** 新节点的默认 props（learn-omniview 高频模式：let 68×、let mut 21×、store 动作 6+、emit 4×）。 */
export function defaultSyntaxProps(tag: string): SyntaxNodeProps {
  switch (tag) {
    case "let":
      return { name: "x", mutable: false, value: "0" };
    case "assign":
      return { target: "count", operator: "=", value: "0" };
    case "store-call":
      return { storeName: "TodoStore", action: "add", args: "" };
    case "call":
      return { fnName: "close", args: "", emit: false };
    default:
      return {};
  }
}

/** props → OmniLang 语句文本（无缩进、无 @pid 注释，带尾分号）。 */
export function formatSyntaxNode(tag: string, props: SyntaxNodeProps): string {
  switch (tag) {
    case "let": {
      const mut = props.mutable ? " mut" : "";
      const type = props.varType ? `: ${props.varType}` : "";
      const value = props.value != null && props.value !== "" ? ` = ${props.value}` : "";
      return `let${mut} ${props.name ?? "x"}${type}${value};`;
    }
    case "assign":
      return `${props.target ?? "x"} ${props.operator ?? "="} ${props.value ?? ""};`;
    case "store-call":
      return `${props.storeName ?? "Store"}.${props.action ?? "action"}(${props.args ?? ""});`;
    case "call":
      return `${props.emit ? "emit " : ""}${props.fnName ?? "fn"}(${props.args ?? ""});`;
    default:
      return "";
  }
}
```

- [ ] **Step 4: 运行验证通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/syntax-nodes.test.ts`
Expected: PASS（8 tests）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/codegen/syntax-nodes.ts poiesis/src/services/codegen/__tests__/syntax-nodes.test.ts
git commit -m "feat: syntax-nodes 纯函数模块（formatSyntaxNode/defaultSyntaxProps）"
```

---

### Task 2: GraphCompiler emit 4 个语法节点 case

**Files:**
- Modify: `poiesis/src/services/codegen/graph-compiler.ts`（import 区 + `emitNode` 的 switch，在 `case "return"` 之前插入 4 case）
- Test: `poiesis/src/services/codegen/__tests__/graph-compiler-syntax.test.ts`

**Interfaces:**
- Consumes: `formatSyntaxNode(tag, props)` from `./syntax-nodes`（Task 1）
- Produces: `compileCodeFlowGraph`（既有导出，签名不变）现在能 emit 4 种新节点；每个新节点 emit `// @pid:<id>` 注释行 + 缩进代码行；flow-out 后继递归逻辑与 code-block 相同

- [ ] **Step 1: 写失败测试**

```typescript
// poiesis/src/services/codegen/__tests__/graph-compiler-syntax.test.ts
import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

function el(id: string, tag: string, props: Record<string, unknown>, source = ""): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, props, parentId: null, source };
}

function wire(fromId: string, toId: string): WireData {
  return { id: `w-${fromId}-${toId}`, fromId, fromPort: "flow-out", toId, toPort: "flow-in" };
}

describe("compileCodeFlowGraph — syntax nodes", () => {
  it("emits let/assign/store-call/call in a linear fn chain", () => {
    const elements = [
      el("fn1", "fn", { name: "handleAdd", params: "text: String" }),
      el("l1", "let", { name: "items", mutable: false, value: "[]" }),
      el("a1", "assign", { target: "count", operator: "=", value: "count + step" }),
      el("s1", "store-call", { storeName: "TodoStore", action: "add", args: "text" }),
      el("c1", "call", { fnName: "close", args: "", emit: false }),
      el("r1", "return", {}, ""),
    ];
    const wires = [
      wire("fn1", "l1"), wire("l1", "a1"), wire("a1", "s1"), wire("s1", "c1"), wire("c1", "r1"),
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("fn handleAdd(text: String) {");
    expect(out).toContain("// @pid:l1\n  let items = [];");
    expect(out).toContain("count = count + step;");
    expect(out).toContain("TodoStore.add(text);");
    expect(out).toContain("close();");
    expect(out).toContain("return;");
  });

  it("emits syntax nodes inside condition branches with branch indent", () => {
    const elements = [
      el("fn1", "fn", { name: "check", params: "" }),
      el("cond1", "condition", { branches: [
        { id: "br0", type: "if", condition: "x > 0" },
        { id: "br1", type: "else", condition: "" },
      ] }),
      el("l1", "let", { name: "y", mutable: true, value: "1" }),
      el("a1", "assign", { target: "y", operator: "+=", value: "1" }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cond1", toPort: "flow-in" },
      { id: "w2", fromId: "cond1", fromPort: "br0", toId: "l1", toPort: "flow-in" },
      { id: "w3", fromId: "cond1", fromPort: "br1", toId: "a1", toPort: "flow-in" },
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("if x > 0 {");
    expect(out).toContain("  let mut y = 1;");
    expect(out).toContain("} else {");
    expect(out).toContain("  y += 1;");
  });

  it("emits emit-call with the emit prefix", () => {
    const elements = [
      el("fn1", "fn", { name: "select", params: "" }),
      el("c1", "call", { fnName: "onSelect", args: "item", emit: true }),
    ];
    const wires = [wire("fn1", "c1")];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("emit onSelect(item);");
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-compiler-syntax.test.ts`
Expected: FAIL（emitNode 无 `case "let"` 等分支 → 输出空）

- [ ] **Step 3: 实现**

在 `graph-compiler.ts` 顶部 import 区加：

```typescript
import { formatSyntaxNode, type SyntaxNodeProps } from "./syntax-nodes";
```

在 `emitNode` 的 switch 中、`case "return"` 之前插入（语法节点是单行语句，无 brace，成功后继与 code-block 相同）：

```typescript
    case "let":
    case "assign":
    case "store-call":
    case "call": {
      let out = emitPidComment(pid);
      out += `${pad}${formatSyntaxNode(tag, (node.element.props ?? {}) as SyntaxNodeProps)}\n`;
      // Emit successor (control flow continues)
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent, visited);
      }
      return out;
    }
```

- [ ] **Step 4: 运行验证通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-compiler-syntax.test.ts src/services/codegen/__tests__/graph-parser.test.ts`
Expected: PASS（3 个新测试 + 既有 graph-parser 测试通过——本任务只改 emit 方向，parse 方向未动）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/codegen/graph-compiler.ts poiesis/src/services/codegen/__tests__/graph-compiler-syntax.test.ts
git commit -m "feat: graph-compiler emit 语法节点（let/assign/store-call/call）"
```

---

### Task 3: GraphParser 语句分类 + props 提取（含现有测试更新）

**Files:**
- Modify: `poiesis/src/services/codegen/graph-parser.ts`（`ParsedSegment.type` 联合、`detectSegmentType`、元素构建循环 464-514 行区域）
- Modify: `poiesis/src/services/codegen/__tests__/graph-parser.test.ts`（2 处受影响的既有断言）
- Test: `poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts`

**Interfaces:**
- Consumes: `formatSyntaxNode(tag, props)` from `./syntax-nodes`（Task 1，用于派生 source）
- Produces: `parseCodeFlowGraph`（既有导出，签名不变）现在返回 8 种 tag 的元素；新节点 `props` 结构化、`source = formatSyntaxNode(tag, props)`

- [ ] **Step 1: 写失败测试**

```typescript
// poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";

describe("parseCodeFlowGraph — syntax nodes", () => {
  it("parses let statement into a let node with extracted props", () => {
    const fnBody = `// @pid:l1
let mut items: Array<String> = [];`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("let");
    expect(el.props).toEqual({ name: "items", mutable: true, varType: "Array<String>", value: "[]" });
    expect(el.source).toBe("let mut items: Array<String> = [];");
  });

  it("parses signal assignment into an assign node", () => {
    const fnBody = `// @pid:a1
count = count + step;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "count", operator: "=", value: "count + step" });
  });

  it("parses compound assignment operator", () => {
    const fnBody = `// @pid:a1
y += 1;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "y", operator: "+=", value: "1" });
  });

  it("parses store call into a store-call node", () => {
    const fnBody = `// @pid:s1
TodoStore.add(text);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("store-call");
    expect(result.elements[0].props).toEqual({ storeName: "TodoStore", action: "add", args: "text" });
  });

  it("parses store signal write as assign (not store-call)", () => {
    const fnBody = `// @pid:a1
ThemeStore.mode = "dark";`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "ThemeStore.mode", operator: "=", value: '"dark"' });
  });

  it("parses emit into a call node with emit=true", () => {
    const fnBody = `// @pid:c1
emit onSelect(item);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: true, fnName: "onSelect", args: "item" });
  });

  it("parses plain function call into a call node", () => {
    const fnBody = `// @pid:c1
close();`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: false, fnName: "close", args: "" });
  });

  it("parses array method call (lowercase) into a call node", () => {
    const fnBody = `// @pid:c1
items.push(x);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: false, fnName: "items.push", args: "x" });
  });

  it("keeps for-loop as code-block (starts with 'for', not 'let')", () => {
    const fnBody = `// @pid:cb1
for (let i = 0; i < n; i++) {
  process(i);
}`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("code-block");
  });

  it("wires syntax nodes linearly like code-blocks", () => {
    const fnBody = `// @pid:l1
let x = 0;
// @pid:a1
x = x + 1;
// @pid:c1
close();`;
    const result = parseCodeFlowGraph(fnBody);
    const lw = result.wires.find((w) => w.fromId === "l1" && w.fromPort === "flow-out");
    expect(lw?.toId).toBe("a1");
    const aw = result.wires.find((w) => w.fromId === "a1" && w.fromPort === "flow-out");
    expect(aw?.toId).toBe("c1");
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-parser-syntax.test.ts`
Expected: FAIL（所有 let/assign/call 语句仍解析为 code-block）

- [ ] **Step 3: 更新现有测试断言（行为变化的预期部分）**

`graph-parser.test.ts` L48-74「parses a simple code-block chain with return」中 `let x = 1` 段：把 code-block 断言改为 let 节点断言：

```typescript
    expect(codeEl.tag).toBe("let");
    expect(codeEl.props).toEqual({ name: "x", mutable: false, value: "1" });
    expect(codeEl.source).toBe("let x = 1;");
```

`graph-parser.test.ts` L230+「round-trips: compile → parse returns equivalent graph」：图中 code-block 的 source 含 `let x = 1`、`process(x)`、`skip()`、`done()`——这些在新规则下会被重分类为 let/call 节点，破坏 round-trip 断言。把 4 个 code-block 的 source 换成中性语句（注释行，不匹配任何新规则）：

```typescript
        source: "  // 初始化变量",   // code1（原 "  let x = 1"）
        source: "  // then 分支",    // then1（原 "  process(x)"）
        source: "  // else 分支",    // else1（原 "  skip()"）
        source: "  // 收尾",         // post1（原 "  done()"）
```

（测试意图是验证 round-trip 稳定性而非语句类型；注释行保持 code-block 语义不变。）

- [ ] **Step 4: 实现 graph-parser**

`ParsedSegment.type` 联合扩展（L29）：

```typescript
  type: "fn" | "code-block" | "condition" | "return" | "let" | "assign" | "store-call" | "call";
```

`detectSegmentType` 返回类型与函数体替换（L158-176）。判定顺序严格按 spec 6.1（先调用后赋值，`ThemeStore.mode = "dark"` 落赋值）：

```typescript
type SegmentType = "fn" | "code-block" | "condition" | "return" | "let" | "assign" | "store-call" | "call";

/** Determine segment type from first non-empty line of code. */
function detectSegmentType(
  lines: string[],
): SegmentType {
  const firstLine = lines.find((l) => l.trim()) || "";
  const trimmed = firstLine.trim();

  if (trimmed.startsWith("fn ")) return "fn";
  if (
    trimmed.startsWith("if ") ||
    trimmed.startsWith("else if ") ||
    trimmed === "else" ||
    trimmed.startsWith("else {")
  )
    return "condition";
  if (trimmed.startsWith("return ") || trimmed === "return;" || trimmed === "return")
    return "return";
  if (/^let\s+(mut\s+)?[a-zA-Z_]\w*/.test(trimmed)) return "let";
  if (trimmed.startsWith("emit ")) return "call";
  if (/^[A-Z]\w*\.[a-zA-Z_]\w*\s*\(/.test(trimmed)) return "store-call";
  if (/^[a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)?\s*\(/.test(trimmed)) return "call";
  if (/^[a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)?\s*(?:=|\+=|-=|\*=|/=|%=)\s*/.test(trimmed)) return "assign";
  return "code-block";
}
```

元素构建循环（L464-514）新增分支（插在 `seg.type === "code-block"` 分支之后）：

```typescript
    } else if (
      seg.type === "let" ||
      seg.type === "assign" ||
      seg.type === "store-call" ||
      seg.type === "call"
    ) {
      props = parseSyntaxProps(seg.type, seg.code);
      source = formatSyntaxNode(seg.type, props as SyntaxNodeProps);
    } else if (seg.type === "return") {
```

文件顶部 import 区加：

```typescript
import { formatSyntaxNode, type SyntaxNodeProps } from "./syntax-nodes";
```

文件底部（`normalizeAnnotations` 之后）新增：

```typescript
// ---------------------------------------------------------------------------
// parseSyntaxProps – extract syntax node props from code text
// ---------------------------------------------------------------------------

/**
 * Extract props for a syntax node from its code text.
 * Inverse of formatSyntaxNode (round-trip stable): every emitted line
 * parses back into the same props, modulo trailing semicolons.
 */
function parseSyntaxProps(
  type: "let" | "assign" | "store-call" | "call",
  code: string,
): Record<string, unknown> {
  const line = code.split("\n").find((l) => l.trim()) || "";
  const t = line.trim();

  switch (type) {
    case "let": {
      const m = t.match(/^let\s+(mut\s+)?([a-zA-Z_]\w*)(?::\s*([\w<>, ]+?))?\s*=\s*(.*?);?\s*$/);
      if (!m) return { name: "x", mutable: false };
      return { name: m[2], mutable: !!m[1], varType: m[3] || undefined, value: m[4] };
    }
    case "assign": {
      const m = t.match(/^([a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)?)\s*(=|\+=|-=|\*=|/=|%=)\s*(.*?);?\s*$/);
      if (!m) return {};
      return { target: m[1], operator: m[2], value: m[3] };
    }
    case "store-call": {
      const m = t.match(/^([A-Z]\w*)\.([a-zA-Z_]\w*)\s*\((.*?)\)\s*;?\s*$/);
      if (!m) return {};
      return { storeName: m[1], action: m[2], args: m[3] };
    }
    case "call": {
      const m = t.match(/^(emit\s+)?([a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)?)\s*\((.*?)\)\s*;?\s*$/);
      if (!m) return { fnName: "fn", args: "" };
      return { emit: !!m[1], fnName: m[2], args: m[3] };
    }
  }
}
```

注意：`formatSyntaxNode` 的 let 正则与 emit 输出互逆验证——`let x = 0;` → `(.*?);?\s*$` 捕获 `0`；`emit onSelect(item);` → call 正则 `^(emit\s+)?` 捕获 `emit `。

- [ ] **Step 5: 运行验证通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-parser-syntax.test.ts src/services/codegen/__tests__/graph-parser.test.ts src/services/codegen/__tests__/roundtrip-annotation.test.ts`
Expected: PASS（10 个新测试 + 更新后的既有测试；roundtrip-annotation 用 `@code-block` 类型注释，TYPE_TO_SEGMENT 强制 code-block，不受影响）

- [ ] **Step 6: 提交**

```bash
git add poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts poiesis/src/services/codegen/__tests__/graph-parser.test.ts
git commit -m "feat: graph-parser 识别语法节点语句（round-trip 分类 + props 提取）"
```

---

### Task 4: 端到端 round-trip 稳定性测试

**Files:**
- Test: `poiesis/src/services/codegen/__tests__/syntax-roundtrip.test.ts`

**Interfaces:**
- Consumes: `compileCodeFlowGraph`（Task 2）+ `parseCodeFlowGraph`（Task 3）
- Produces: 无新接口——验证双向稳定：parse→compile→parse 等价；compile→parse→compile 文本等价

- [ ] **Step 1: 写测试（本轮无失败预期——纯验证测试，但先确认能运行）**

```typescript
// poiesis/src/services/codegen/__tests__/syntax-roundtrip.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

function el(id: string, tag: string, props: Record<string, unknown>, source = ""): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, props, parentId: null, source };
}

/** Compile a graph then re-parse it; return the parsed elements. */
function compileThenParse(elements: CodeblockElementData[], wires: WireData[], fnId: string) {
  const compiled = compileCodeFlowGraph(elements, wires, fnId);
  return { compiled, parsed: parseCodeFlowGraph(compiled) };
}

describe("syntax node round-trip", () => {
  it("compile → parse preserves node types and props for all four tags", () => {
    const elements = [
      el("fn1", "fn", { name: "roundtrip", params: "" }),
      el("l1", "let", { name: "items", mutable: true, varType: "Array<String>", value: "[]" }),
      el("a1", "assign", { target: "count", operator: "+=", value: "1" }),
      el("s1", "store-call", { storeName: "TodoStore", action: "add", args: "text" }),
      el("c1", "call", { fnName: "onSelect", args: "item", emit: true }),
      el("r1", "return", {}, ""),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "l1", toPort: "flow-in" },
      { id: "w2", fromId: "l1", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
      { id: "w3", fromId: "a1", fromPort: "flow-out", toId: "s1", toPort: "flow-in" },
      { id: "w4", fromId: "s1", fromPort: "flow-out", toId: "c1", toPort: "flow-in" },
      { id: "w5", fromId: "c1", fromPort: "flow-out", toId: "r1", toPort: "flow-in" },
    ];
    const { parsed } = compileThenParse(elements, wires, "fn1");
    expect(parsed.elements.find((e) => e.id === "l1")?.tag).toBe("let");
    expect(parsed.elements.find((e) => e.id === "l1")?.props).toEqual({ name: "items", mutable: true, varType: "Array<String>", value: "[]" });
    expect(parsed.elements.find((e) => e.id === "a1")?.tag).toBe("assign");
    expect(parsed.elements.find((e) => e.id === "a1")?.props).toEqual({ target: "count", operator: "+=", value: "1" });
    expect(parsed.elements.find((e) => e.id === "s1")?.tag).toBe("store-call");
    expect(parsed.elements.find((e) => e.id === "s1")?.props).toEqual({ storeName: "TodoStore", action: "add", args: "text" });
    expect(parsed.elements.find((e) => e.id === "c1")?.tag).toBe("call");
    expect(parsed.elements.find((e) => e.id === "c1")?.props).toEqual({ emit: true, fnName: "onSelect", args: "item" });
    // Wiring preserved
    expect(parsed.wires.some((w) => w.fromId === "l1" && w.toId === "a1")).toBe(true);
    expect(parsed.wires.some((w) => w.fromId === "s1" && w.toId === "c1")).toBe(true);
  });

  it("compile → parse → compile produces identical source", () => {
    const elements = [
      el("fn1", "fn", { name: "stable", params: "n: i32" }),
      el("l1", "let", { name: "total", mutable: false, value: "0" }),
      el("a1", "assign", { target: "total", operator: "=", value: "total + n" }),
      el("r1", "return", {}, ""),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "l1", toPort: "flow-in" },
      { id: "w2", fromId: "l1", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
      { id: "w3", fromId: "a1", fromPort: "flow-out", toId: "r1", toPort: "flow-in" },
    ];
    const { compiled, parsed } = compileThenParse(elements, wires, "fn1");
    // Recompile the parsed graph — source must be identical (idempotent)
    const parsedFnId = parsed.elements.find((e) => e.tag === "fn")!.id;
    const recompiled = compileCodeFlowGraph(parsed.elements, parsed.wires, parsedFnId);
    expect(recompiled).toBe(compiled);
  });

  it("hand-written source with @pid comments round-trips through the pipeline", () => {
    const fnBody = `// @pid:fn1
fn filter(items: Array<String>) {
  // @pid:l1
  let mut result = [];
  // @pid:a1
  result = items.filter(|i| i != "x");
  // @pid:r1
  return result;
}`;
    const parsed = parseCodeFlowGraph(fnBody);
    const fnId = parsed.elements.find((e) => e.tag === "fn")!.id;
    const recompiled = compileCodeFlowGraph(parsed.elements, parsed.wires, fnId);
    const reparsed = parseCodeFlowGraph(recompiled);
    // Types preserved after the full cycle
    expect(reparsed.elements.find((e) => e.id === "l1")?.tag).toBe("let");
    expect(reparsed.elements.find((e) => e.id === "a1")?.tag).toBe("assign");
    expect(reparsed.elements.find((e) => e.id === "r1")?.tag).toBe("return");
  });
});
```

- [ ] **Step 2: 运行验证**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/syntax-roundtrip.test.ts`
Expected: PASS（3 tests；如失败，说明 Task 2/3 的 emit/parse 正则不互逆，修复对应正则）

- [ ] **Step 3: 提交**

```bash
git add poiesis/src/services/codegen/__tests__/syntax-roundtrip.test.ts
git commit -m "test: 语法节点端到端 round-trip 稳定性测试"
```

---

### Task 5: 菜单与节点 UI（flowConstants + SyntaxNode 组件 + FlowCanvas 注册）

**Files:**
- Modify: `poiesis/src/components/canvas/code/flowConstants.ts`（CODE_TAGS、TAG_CN、MENU_SECTIONS）
- Create: `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx` + `SyntaxNode.module.css`
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（nodeTypes 注册、handleAddNode 默认 props、data 传递）

**Interfaces:**
- Consumes: `defaultSyntaxProps` from `../../../services/codegen/syntax-nodes`（Task 1）
- Produces: 4 个新 tag 可添加到画布并渲染；右键菜单新增「变量与赋值」「调用」两分类

- [ ] **Step 1: 实现 flowConstants 扩展**

`flowConstants.ts`：

```typescript
export const TAG_CN: Record<string, string> = {
  fn: "函数",
  "code-block": "代码块",
  condition: "判断",
  return: "返回",
  let: "变量声明",
  assign: "赋值",
  "store-call": "store 调用",
  call: "调用",
};

export const CODE_TAGS = new Set([
  "fn",
  "code-block",
  "condition",
  "return",
  "let",
  "assign",
  "store-call",
  "call",
]);

export const MENU_SECTIONS: MenuSection[] = [
  {
    label: "控制流",
    items: [
      { label: "函数定义 (fn)", tag: "fn", shortcut: "fn" },
      { label: "判断 (condition)", tag: "condition", shortcut: "cond" },
      { label: "代码块 (code-block)", tag: "code-block", shortcut: "cb" },
      { label: "返回 (return)", tag: "return", shortcut: "ret" },
    ],
  },
  {
    label: "变量与赋值",
    items: [
      { label: "变量声明 (let)", tag: "let", shortcut: "let" },
      { label: "赋值 (assign)", tag: "assign", shortcut: "=" },
    ],
  },
  {
    label: "调用",
    items: [
      { label: "store 调用", tag: "store-call", shortcut: "st" },
      { label: "函数调用 (call)", tag: "call", shortcut: "fn()" },
    ],
  },
];
```

- [ ] **Step 2: 创建 SyntaxNode 组件**

```tsx
// poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx
import { Handle, Position } from "@xyflow/react";
import React from "react";
import { formatSyntaxNode } from "../../../../services/codegen/syntax-nodes";
import styles from "./SyntaxNode.module.css";

interface Props {
  data: {
    label: string;
    tag: string;
    props?: Record<string, unknown>;
  };
}

/** 语法节点（let/assign/store-call/call）：卡片显示 props 生成的代码预览。 */
const SyntaxNode = React.memo(function SyntaxNode({ data }: Props) {
  const code = formatSyntaxNode(data.tag, (data.props ?? {}) as never);
  return (
    <div
      className={`flow-node ${styles.node}`}
      data-node-type={data.tag}
      data-syntax-code={code}
    >
      <code>{code}</code>
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "var(--ol-text-muted)" }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "var(--ol-text-muted)" }}
      />
    </div>
  );
});
export { SyntaxNode };
```

```css
/* poiesis/src/components/canvas/code/nodes/SyntaxNode.module.css */
.node {
  border-radius: var(--ol-radius-md);
  padding: var(--ol-space-2p5) 14px;
  min-width: 180px;
  max-width: 280px;
  font-size: var(--ol-font-size-12);
  font-family: var(--ol-font-family-mono);
  cursor: pointer;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: var(--ol-line-height-normal);
}
/* tag 色系：let 蓝（primary）、assign 青（info）、store-call 橙（warning）、call 绿（success） */
.node[data-node-type="let"] {
  background: var(--ol-color-primary-subtle);
  border: 1.5px solid var(--ol-color-primary);
  color: var(--ol-color-primary-active);
}
.node[data-node-type="assign"] {
  background: var(--ol-color-info-muted);
  border: 1.5px solid var(--ol-color-info);
  color: var(--ol-gray-700);
}
.node[data-node-type="store-call"] {
  background: var(--ol-color-warning-muted);
  border: 1.5px solid var(--ol-color-warning);
  color: var(--ol-gray-700);
}
.node[data-node-type="call"] {
  background: var(--ol-color-success-muted);
  border: 1.5px solid var(--ol-color-success);
  color: var(--ol-gray-700);
}
```

（先用 tokens 中确认存在的变量；`--ol-color-info-muted` / `--ol-color-warning-muted` / `--ol-color-success-muted` 若不存在，用 `-subtle` 变体替换——先 `grep` tokens.css 确认。）

- [ ] **Step 3: FlowCanvas 注册与默认 props**

`FlowCanvas.tsx`：

nodeTypes 对象（L32-38）加 4 项：

```typescript
const nodeTypes = {
  "fn": FnNode,
  "code-block": CodeBlockNode,
  "condition": ConditionNode,
  "return": ReturnNode,
  "design-preview": DesignPreviewNode,
  "let": SyntaxNode,
  "assign": SyntaxNode,
  "store-call": SyntaxNode,
  "call": SyntaxNode,
};
```

import 区加：

```typescript
import { SyntaxNode } from "./nodes/SyntaxNode";
import { defaultSyntaxProps } from "../../../services/codegen/syntax-nodes";
```

`handleAddNode`（L95-123）中，在 `if (tag === "condition")` 分支后加：

```typescript
      if (["let", "assign", "store-call", "call"].includes(tag)) {
        props = { ...defaultSyntaxProps(tag) };
      }
```

节点 data 传递（L247-268）：节点创建处需要把 `el.props` 传给 SyntaxNode。在 nodes map 的 data 对象里加（现有 data 没有 props 字段）：

```typescript
        data: {
          label: TAG_CN[el.tag] || el.tag,
          tag: el.tag,
          props: el.props,
          source: el.source,
          ...
```

并保持 `onDoubleClick: () => setEditingNodeId(el.id)` 只对 code-block 生效——SyntaxNode 组件不绑定 onDoubleClick（组件内无该属性），天然禁用双击编辑。

- [ ] **Step 4: 验证类型与现有测试**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/ && npx tsc --noEmit`
Expected: 全部 PASS + 无类型错误。（`npx tsc --noEmit` 若项目无该脚本配置则跳过，改用 `npx tsc --noEmit -p tsconfig.json`。）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/flowConstants.ts poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx poiesis/src/components/canvas/code/nodes/SyntaxNode.module.css poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat: 菜单与节点 UI（语法节点菜单分类 + SyntaxNode 组件）"
```

---

### Task 6: FlowInspector 语法节点表单

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowInspector.tsx`（新增 4 个 Inspector 组件 + switch 分支 + storeReferences）
- Test: `poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx`

**Interfaces:**
- Consumes: `ASSIGN_OPERATORS` from `../../../services/codegen/syntax-nodes`；`usePoiesisStore`（既有）；`storeReferences: Record<string, string[]>`（store 名 → 引用组件列表，键即 store 名下拉）
- Produces: 点选 let/assign/store-call/call 节点时右侧面板显示对应表单；修改调用 `setProp(el.id, field, value)`

- [ ] **Step 1: 写渲染测试**

```tsx
// poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FlowInspector } from "../FlowInspector";
import type { CodeblockElementData } from "../../../../core/types/element";

// vi.mock 会被提升到 import 之前执行，mock 中引用的变量必须用 vi.hoisted 创建。
const { mockSetProp } = vi.hoisted(() => ({ mockSetProp: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) =>
    selector({
      elements: [
        {
          id: "l1", nodeType: "codeblock", tag: "let", parentId: null,
          props: { name: "x", mutable: false, value: "0" },
          source: "let x = 0;",
        },
        {
          id: "s1", nodeType: "codeblock", tag: "store-call", parentId: null,
          props: { storeName: "TodoStore", action: "add", args: "" },
          source: "TodoStore.add();",
        },
      ],
      setProp: (id: string, prop: string, value: unknown) => {
        mockSetProp(id, prop, value);
      },
      setElementSource: () => {},
      storeReferences: { TodoStore: ["App"], CartStore: ["App"] },
    }),
}));

beforeEach(() => {
  mockSetProp.mockClear();
});

describe("FlowInspector syntax node forms", () => {
  it("renders LetInspector fields for a let node", () => {
    render(<FlowInspector selectedNodeIds={["l1"]} />);
    expect(screen.getByLabelText("变量名")).toBeTruthy();
    expect(screen.getByLabelText("可变 (mut)")).toBeTruthy();
    expect(screen.getByLabelText("初始值")).toBeTruthy();
  });

  it("updates let props through setProp", () => {
    render(<FlowInspector selectedNodeIds={["l1"]} />);
    fireEvent.change(screen.getByLabelText("变量名"), { target: { value: "count" } });
    expect(mockSetProp).toHaveBeenCalledWith("l1", "name", "count");
  });

  it("renders store-call form with store name datalist from storeReferences", () => {
    render(<FlowInspector selectedNodeIds={["s1"]} />);
    expect(screen.getByLabelText("store 名")).toBeTruthy();
    expect(screen.getByLabelText("action")).toBeTruthy();
  });
});
```

（注意：`usePoiesisStore` 的实际导出签名见 `poiesis/src/store/index.ts`；若 store 模块在 jsdom 可直接初始化（test-setup 已 mock localStorage），也可去掉 vi.mock 改用真实 store 并 `act()` 设置状态——以运行通过为准。mock 路径 `../../../../store` 从 `__tests__/` 目录算起。）

- [ ] **Step 2: 运行验证失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/FlowInspector.test.tsx`
Expected: FAIL（表单尚未实现）

- [ ] **Step 3: 实现 FlowInspector 扩展**

在 `FlowInspector` 主体 switch 中加 4 个分支：

```typescript
  if (el.tag === "fn") {
    return <FnInspector el={el} setProp={setProp} />;
  }

  if (el.tag === "return") {
    return <ReturnInspector el={el} setElementSource={setElementSource} />;
  }

  if (el.tag === "let") {
    return <LetInspector el={el} setProp={setProp} />;
  }

  if (el.tag === "assign") {
    return <AssignInspector el={el} setProp={setProp} />;
  }

  if (el.tag === "store-call") {
    return <StoreCallInspector el={el} setProp={setProp} storeReferences={storeReferences} />;
  }

  if (el.tag === "call") {
    return <CallInspector el={el} setProp={setProp} />;
  }
```

`FlowInspector` 组件顶部加 storeReferences 读取（L98-100 区域）：

```typescript
  const storeReferences = usePoiesisStore((s) => s.storeReferences);
```

新增 4 个表单组件（复用 S.* 样式常量，字段模式与 FnInspector 一致——label 用中文并加 `aria-label` 供测试定位）：

```tsx
// ────────────────────────────────────────────
// Syntax node inspectors (let / assign / store-call / call)
// ────────────────────────────────────────────

const SYNTAX_FIELD: React.CSSProperties = { ...S.field };
const SYNTAX_LABEL: React.CSSProperties = { ...S.label };
const SYNTAX_INPUT: React.CSSProperties = { ...S.input };
const SYNTAX_SELECT: React.CSSProperties = { ...S.select };

function LetInspector({ el, setProp }: { el: CodeblockElementData; setProp: FnInspectorProps["setProp"] }) {
  const name = (el.props?.name as string) || "";
  const mutable = !!el.props?.mutable;
  const varType = (el.props?.varType as string) || "";
  const value = (el.props?.value as string) || "";
  return (
    <div style={S.panel}>
      <div style={S.header}>let 变量声明</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>变量名</div>
        <input aria-label="变量名" style={SYNTAX_INPUT} value={name}
          onChange={(e) => setProp(el.id, "name", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>类型（可选）</div>
        <select aria-label="类型" style={SYNTAX_SELECT} value={varType}
          onChange={(e) => setProp(el.id, "varType", e.target.value || undefined)}>
          <option value="">（推断）</option>
          {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          {varType && !OMNI_TYPES.includes(varType) && <option value={varType}>{varType}</option>}
        </select>
      </div>
      <div style={SYNTAX_FIELD}>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--ol-font-size-12)" }}>
          <input type="checkbox" aria-label="可变 (mut)" checked={mutable}
            onChange={(e) => setProp(el.id, "mutable", e.target.checked)} />
          可变 (mut)
        </label>
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>初始值</div>
        <input aria-label="初始值" style={SYNTAX_INPUT} value={value}
          placeholder="e.g. 0 / [] / items.filter(...)"
          onChange={(e) => setProp(el.id, "value", e.target.value)} />
      </div>
    </div>
  );
}

function AssignInspector({ el, setProp }: { el: CodeblockElementData; setProp: FnInspectorProps["setProp"] }) {
  const target = (el.props?.target as string) || "";
  const operator = (el.props?.operator as string) || "=";
  const value = (el.props?.value as string) || "";
  return (
    <div style={S.panel}>
      <div style={S.header}>assign 赋值</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>目标（signal/变量/store 信号）</div>
        <input aria-label="目标" style={SYNTAX_INPUT} value={target}
          placeholder="e.g. count / ThemeStore.mode"
          onChange={(e) => setProp(el.id, "target", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>运算符</div>
        <select aria-label="运算符" style={SYNTAX_SELECT} value={operator}
          onChange={(e) => setProp(el.id, "operator", e.target.value)}>
          {ASSIGN_OPERATORS.map((op) => <option key={op} value={op}>{op}</option>)}
        </select>
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>表达式</div>
        <input aria-label="表达式" style={SYNTAX_INPUT} value={value}
          placeholder="e.g. count + step"
          onChange={(e) => setProp(el.id, "value", e.target.value)} />
      </div>
    </div>
  );
}

function StoreCallInspector({ el, setProp, storeReferences }: {
  el: CodeblockElementData;
  setProp: FnInspectorProps["setProp"];
  storeReferences: Record<string, string[]>;
}) {
  const storeName = (el.props?.storeName as string) || "";
  const action = (el.props?.action as string) || "";
  const args = (el.props?.args as string) || "";
  const storeNames = Object.keys(storeReferences);
  return (
    <div style={S.panel}>
      <div style={S.header}>store 调用</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>store 名</div>
        <input aria-label="store 名" list="syntax-store-names" style={SYNTAX_INPUT} value={storeName}
          placeholder="e.g. TodoStore"
          onChange={(e) => setProp(el.id, "storeName", e.target.value)} />
        <datalist id="syntax-store-names">
          {storeNames.map((n) => <option key={n} value={n} />)}
        </datalist>
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>action</div>
        <input aria-label="action" style={SYNTAX_INPUT} value={action}
          placeholder="e.g. add"
          onChange={(e) => setProp(el.id, "action", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>参数（逗号分隔）</div>
        <input aria-label="参数" style={SYNTAX_INPUT} value={args}
          placeholder="e.g. text, 2"
          onChange={(e) => setProp(el.id, "args", e.target.value)} />
      </div>
    </div>
  );
}

function CallInspector({ el, setProp }: { el: CodeblockElementData; setProp: FnInspectorProps["setProp"] }) {
  const fnName = (el.props?.fnName as string) || "";
  const args = (el.props?.args as string) || "";
  const emit = !!el.props?.emit;
  return (
    <div style={S.panel}>
      <div style={S.header}>call 调用</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>函数名</div>
        <input aria-label="函数名" style={SYNTAX_INPUT} value={fnName}
          placeholder="e.g. close / onSelect / console_log"
          onChange={(e) => setProp(el.id, "fnName", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>参数（逗号分隔）</div>
        <input aria-label="参数" style={SYNTAX_INPUT} value={args}
          placeholder="e.g. item"
          onChange={(e) => setProp(el.id, "args", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--ol-font-size-12)" }}>
          <input type="checkbox" aria-label="emit 事件" checked={emit}
            onChange={(e) => setProp(el.id, "emit", e.target.checked)} />
          emit 事件
        </label>
      </div>
    </div>
  );
}
```

文件顶部 import 区加：

```typescript
import { ASSIGN_OPERATORS } from "../../../services/codegen/syntax-nodes";
```

- [ ] **Step 4: 运行验证通过**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/FlowInspector.test.tsx`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/FlowInspector.tsx poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx
git commit -m "feat: FlowInspector 语法节点表单（let/assign/store-call/call）"
```

---

### Task 7: 全量验证（vitest + tsc + Victauri 手动）

**Files:** 无新文件

**Interfaces:** 验证 Task 1-6 产物集成正确，无回归

- [ ] **Step 1: 全量单测**

Run: `cd poiesis && npx vitest run`
Expected: 全部 PASS（既有 ~60+ 测试 + 新增 ~24 测试）

- [ ] **Step 2: 类型检查**

Run: `cd poiesis && npx tsc --noEmit`
Expected: 0 errors

- [ ] **Step 3: Victauri 手动验证（组件代码流）**

1. 打开 poiesis 设计页 → 进入 CodeFlowCanvas（codeFlow 模式）
2. 画布空白处右键 → 确认新分类「变量与赋值」「调用」存在（4 个新菜单项）
3. 添加 let 节点 → 点选 → 右侧表单出现 → 改名 `total`、值 `0`
4. 添加 assign 节点 → target `count`、值 `count + step`
5. 添加 store-call 节点 → store 名下拉出现 `TodoStore`（storeReferences 有数据时）
6. 添加 call 节点 → 勾选 emit → fnName `onSelect`
7. 连线 fn → let → assign → store-call → call → return
8. 保存（Ctrl+S）→ 重新进入该组件代码流 → **节点类型保持**（let/assign/store-call/call），代码文本正确
9. 打开生成的 .omni 源码确认：`let total = 0;` / `count = count + step;` / `TodoStore.add(...)` / `emit onSelect(...);` 均带 `// @pid:xxx` 注释
10. Debug window 或画布预览确认组件编译无错误（新增代码被真实编译）

- [ ] **Step 4: Victauri 手动验证（store action 代码流）**

1. StorePage → 打开任一 store 的 action 代码流
2. 添加 assign / store-call 节点并连线
3. 保存 → 重新打开 → 节点类型保持
4. 生成的 store .omni 中 action 体包含新语句

- [ ] **Step 5: 提交（如有修复）**

```bash
git add -A poiesis/src
git commit -m "fix: 语法节点集成修复（按全量验证结果）"
```

---

## 任务依赖图

```
Task 1 (syntax-nodes 纯函数)
   ├── Task 2 (GraphCompiler emit)
   ├── Task 3 (GraphParser 分类)
   │      └── Task 4 (round-trip 测试，验证 2+3 互逆)
   └── Task 5 (UI 菜单/节点组件)
          └── Task 6 (FlowInspector 表单)
                 └── Task 7 (全量验证)
```

## 风险与注意

- **现有测试破坏**（Task 3 已处理）：`graph-parser.test.ts` 中 `let x = 1` / `process(x)` / `done()` 等语句在新分类规则下变为 let/call 节点——这是预期行为变化，断言已更新
- **正则互逆**（Task 4 验证）：emit 输出 `let x = 0;` 必须能被 parse 正则提取回 `{ name: "x", value: "0" }`；尾分号在提取时剥离
- **`formatSyntaxNode` 单一来源**：GraphCompiler 与 SyntaxNode 预览共用，禁止两处各自拼字符串
- **双击编辑禁用**：SyntaxNode 组件不绑定 onDoubleClick（CodeBlockEditor 改的是 source，语法节点以 props 为权威）
- **CSS 变量**：使用前 grep `tokens.css` 确认 `--ol-color-{info,warning,success}-muted` 存在，否则用 `-subtle` 变体
