# CodeFlowCanvas 成熟化实现计划（自动整理布局 / 循环节点 / Notion 式公式框）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 CodeFlowCanvas 更成熟——一键自动整理布局（自上而下分层）、while/for-in 循环节点（round-trip 稳定）、Notion 式公式框（表达式 token 化编辑）。

**Architecture:** 三块全部落在 poiesis 前端（services/codegen 纯函数层 + components/canvas/code UI 层），编译器（omnilang crates）零改动。循环节点复用既有 `@pid` round-trip 管线（loop = 单分支 condition 变体）；公式框保持 NodeFormulaField props 契约不变、内部实现替换（提交仍是字符串，持久化零改动）；布局是纯函数 computeLayeredLayout + setElementsAndWires 批量写入（带 undo）。

**Tech Stack:** React 19 + TypeScript + Zustand + @xyflow/react v12 + vitest（jsdom），零新依赖。

## Global Constraints

- 用户定性：代码的节点编辑器（节点=语句、连线=执行序）；**不做**数据端口/值连线、多选/框选/复制粘贴/折叠等通用交互打磨
- **零新依赖**（不引入 dagre/elkjs/contenteditable 库）
- **`@pid` round-trip 契约零改动**：注释格式、graph-parser 分支 id 扫描序（`br${branchIndex}`）、return 表达式 `el.source` 契约不变
- 运算符候选表以编译器 `infix.rs` 为权威（`+ - * / % == != < > <= >= && || ?? is ?.` 与一元 `!`），UI 不自行定义
- 语言合法性优先：生成源码必须过 WASM 语义验证（保存门禁最终裁决）；循环 body 内嵌套 fn 声明非法（E0001）
- 中文注释（项目惯例）；测试基线 440/440 vitest + `tsc --noEmit` 零错误
- 提交走 conventional commits；每任务独立 commit

---

### Task 1: syntax-nodes 扩展（while / for-in 单一来源）

**Files:**
- Modify: `poiesis/src/services/codegen/syntax-nodes.ts`
- Test: `poiesis/src/services/codegen/__tests__/syntax-nodes.test.ts`

**Interfaces:**
- Consumes: `SyntaxNodeProps`（既有接口，新增 `condition`/`varName`/`iterable` 字段）
- Produces: `formatSyntaxNode(tag, props)` 支持 `"while"` / `"for-in"`；`defaultSyntaxProps(tag)` 返回对应默认值

- [ ] **Step 1: 写失败测试**

在 `syntax-nodes.test.ts` 追加：

```ts
import { describe, it, expect } from "vitest";
import { formatSyntaxNode, defaultSyntaxProps } from "../syntax-nodes";

describe("loop 语法节点", () => {
  it("while 生成 `while <condition> {`（无尾分号，块头由编译器补）", () => {
    expect(formatSyntaxNode("while", { condition: "n < 10" })).toBe("while n < 10 {");
  });
  it("for-in 生成 `for <var> in <iterable> {`", () => {
    expect(formatSyntaxNode("for-in", { varName: "item", iterable: "items" })).toBe("for item in items {");
  });
  it("defaultSyntaxProps 提供占位默认值", () => {
    expect(defaultSyntaxProps("while")).toEqual({ condition: "count < 10" });
    expect(defaultSyntaxProps("for-in")).toEqual({ varName: "item", iterable: "items" });
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/services/codegen/__tests__/syntax-nodes.test.ts`
Expected: FAIL（`formatSyntaxNode` 对 while 走 default 分支返回 `""`）

- [ ] **Step 3: 最小实现**

在 `syntax-nodes.ts` 的 `SyntaxNodeProps` 追加字段（带注释）：

```ts
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
  /** while: 循环条件表达式 */
  condition?: string;
  /** for-in: 循环变量名 */
  varName?: string;
  /** for-in: 可迭代表达式 */
  iterable?: string;
}
```

`defaultSyntaxProps` 的 switch 加：

```ts
    case "while":
      return { condition: "count < 10" };
    case "for-in":
      return { varName: "item", iterable: "items" };
```

`formatSyntaxNode` 的 switch 加（在 default 之前）：

```ts
    case "while":
      return `while ${props.condition ?? "true"} {`;
    case "for-in":
      return `for ${props.varName ?? "item"} in ${props.iterable ?? "items"} {`;
```

- [ ] **Step 4: 运行确认通过**

Run: `npx vitest run src/services/codegen/__tests__/syntax-nodes.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/syntax-nodes.ts poiesis/src/services/codegen/__tests__/syntax-nodes.test.ts
git commit -m "feat(codeflow): syntax-nodes 支持 while/for-in（formatSyntaxNode 单一来源）"
```

---

### Task 2: graph-parser 识别与 props 提取（detectSegmentType / parseSyntaxProps）

**Files:**
- Modify: `poiesis/src/services/codegen/graph-parser.ts`
- Test: `poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts`

**Interfaces:**
- Consumes: Task 1 的 `formatSyntaxNode`（元素构造侧 source 派生）；既有 `parseSyntaxProps(type, code)`
- Produces: `SegmentType` 联合新增 `"while" | "for-in"`；`parseSyntaxProps` 支持两类型；`detectSegmentType` 识别 OmniLang 风格循环段头

- [ ] **Step 1: 写失败测试**

在 `graph-parser-syntax.test.ts` 追加：

```ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";

function parseProps(src: string) {
  const g = parseCodeFlowGraph(src);
  const el = g.elements.find((e) => e.tag === "while" || e.tag === "for-in");
  return el ? { tag: el.tag, props: el.props } : null;
}

describe("graph-parser 循环段", () => {
  it("while 段头识别为 while 节点并提取 condition", () => {
    const r = parseProps(`fn f() {
  // @pid:w-1
  while n < 10 {
    // @pid:b-2
    n = n + 1;
  }
}`);
    expect(r).not.toBeNull();
    expect(r!.tag).toBe("while");
    expect(r!.props.condition).toBe("n < 10");
  });
  it("for-in 段头识别并提取 varName/iterable", () => {
    const r = parseProps(`fn f() {
  // @pid:fi-1
  for item in items {
    // @pid:b-2
    total = total + item;
  }
}`);
    expect(r!.tag).toBe("for-in");
    expect(r!.props.varName).toBe("item");
    expect(r!.props.iterable).toBe("items");
  });
  it("C 风格括号循环（`for (…`) 保持 code-block（防御性，非 OmniLang 语法）", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:c-1
  for (let i = 0; i < n; i++) {
    x = i;
  }
}`);
    const el = g.elements.find((e) => e.id === "c-1");
    expect(el?.tag).toBe("code-block");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/services/codegen/__tests__/graph-parser-syntax.test.ts`
Expected: FAIL（while/for-in 段头目前归 code-block；props 为空）

- [ ] **Step 3: 最小实现**

`graph-parser.ts` 三处修改：

**a) `SegmentType` 联合（L159）**：

```ts
type SegmentType = "fn" | "code-block" | "condition" | "return" | "let" | "assign" | "store-call" | "call" | "while" | "for-in";
```

**b) `detectSegmentType`（L162-189）**：在 condition/return 判定之后、既有 C 风格 for/while 括号规则（`/^(for|while)\s*\(/`）**之前**插入：

```ts
  // OmniLang 风格循环段头（无括号）：`while <expr> {` / `for <ident> in <expr> {`
  if (/^while\s+/.test(trimmed)) return "while";
  if (/^for\s+[a-zA-Z_]\w*\s+in\s+/.test(trimmed)) return "for-in";
```

（既有 `if (/^(for|while)\s*\(/.test(trimmed)) return "code-block";` 保留——C 风格防御，不匹配 OmniLang 语法。）

**c) `parseSyntaxProps`（L690）**：参数类型联合扩展 + switch 加两 case（复用既有 stmtEnd 截断 + 单行归一逻辑，只改 switch）：

```ts
function parseSyntaxProps(
  type: "let" | "assign" | "store-call" | "call" | "while" | "for-in",
  code: string,
): Record<string, unknown> {
  // ...既有 stmtEnd 截断与 t 归一不变...
  switch (type) {
    case "while": {
      const m = t.match(/^while\s+(.*?)\s*\{?\s*$/);
      return m ? { condition: m[1].trim() } : {};
    }
    case "for-in": {
      const m = t.match(/^for\s+([a-zA-Z_]\w*)\s+in\s+(.*?)\s*\{?\s*$/);
      return m ? { varName: m[1], iterable: m[2].trim() } : {};
    }
    // ...既有 case 不变...
  }
}
```

- [ ] **Step 4: 运行确认通过**

Run: `npx vitest run src/services/codegen/__tests__/graph-parser-syntax.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts
git commit -m "feat(codeflow): graph-parser 识别 while/for-in 段头并提取 props"
```

---

### Task 3: graph-parser 线形重建（loop 压栈 + body/flow-out 接线）

**Files:**
- Modify: `poiesis/src/services/codegen/graph-parser.ts`（`assignBranchContext` 压栈、`parseCodeFlowGraph` 元素构造与续接循环）
- Test: `poiesis/src/services/codegen/__tests__/graph-parser-midchain.test.ts`（追加 describe 块）

**Interfaces:**
- Consumes: Task 2 的 SegmentType/parseSyntaxProps；既有 branchCtx 机制
- Produces: loop 段 body 段 wire 到 `br0`、后置段 wire 到 `flow-out`（与 condition 对称）

- [ ] **Step 1: 写失败测试**

在 `graph-parser-midchain.test.ts` 追加：

```ts
describe("graph-parser 中链 loop", () => {
  const MIDCHAIN_LOOP = `fn handleAdd() -> void {
    // @pid:let-1
      let total = 0;
    // @pid:w-2
      while total < 10 {
    // @pid:assign-3
        total = total + 1;
      }
    // @pid:ret-4
      return total;
  }`;

  it("中链 while：入链 + body(br0) + flow-out 续接完整", () => {
    const g = parseCodeFlowGraph(MIDCHAIN_LOOP);
    const w = wireList(g.wires);
    expect(w).toContain("let-1(flow-out)->w-2(flow-in)");
    expect(w).toContain("w-2(br0)->assign-3(flow-in)");
    expect(w).toContain("w-2(flow-out)->ret-4(flow-in)");
  });

  it("loop 嵌套在 condition 分支体内（branchCtx 栈复用）", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:cond-1
    if ok {
  // @pid:w-2
      while n > 0 {
  // @pid:a-3
        n = n - 1;
      }
    }
  // @pid:ret-4
    return;
  }`);
    const w = wireList(g.wires);
    expect(w).toContain("cond-1(br0)->w-2(flow-in)");
    expect(w).toContain("w-2(br0)->a-3(flow-in)");
  });

  it("for-in 段元素 props/source 派生（round-trip 用）", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:fi-1
    for item in items {
  // @pid:a-2
      total = total + item;
    }
  }`);
    const el = g.elements.find((e) => e.id === "fi-1");
    expect(el?.tag).toBe("for-in");
    expect(el?.props.varName).toBe("item");
    expect(el?.props.iterable).toBe("items");
    expect(el?.source).toBe("for item in items {");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/services/codegen/__tests__/graph-parser-midchain.test.ts`
Expected: FAIL（body 段无 ctx → 落入顶层线形通道，`w-2(br0)` 缺失）

- [ ] **Step 3: 最小实现**

`graph-parser.ts` 三处修改：

**a) `assignBranchContext` 压栈（L400）**——condition 与 loop 同为块头节点压栈（loop 单分支，branchCount 1，无 elif 延续语义，2a/2b/2c/2d 对 loop 天然无触发）：

```ts
    if ((seg.type === "condition" || seg.type === "while" || seg.type === "for-in") && !isBranchContinuation) {
      const branches = seg.type === "condition" ? parseConditionBranches(seg.code) : [{ id: "br0", type: "if" as const, condition: "" }];
      conditionStack.push({
        pid: seg.pid,
        baseDepth: startDepth,
        branchCount: branches.length,
        currentBranch: 0,
        pendingTransition: false,
      });
    }
```

**b) `parseCodeFlowGraph` 元素构造（L513 的 else-if 链）**——把 while/for-in 并入 syntax props 分支：

```ts
    } else if (
      seg.type === "let" ||
      seg.type === "assign" ||
      seg.type === "store-call" ||
      seg.type === "call" ||
      seg.type === "while" ||
      seg.type === "for-in"
    ) {
      const parsed = parseSyntaxProps(seg.type, seg.code);
      props = parsed;
      source = Object.keys(parsed).length > 0 ? formatSyntaxNode(seg.type, parsed as SyntaxNodeProps) : seg.code;
```

（`formatSyntaxNode` 已由 Task 1 支持 loop；需确认文件顶部已 import `formatSyntaxNode`——L524 既有使用，无新增 import。）

**c) 续接循环（L609）**——loop 段与 condition 段同样做 flow-out 续接：

```ts
  for (let i = 0; i < segments.length; i++) {
    const segType = segments[i].type;
    if (segType !== "condition" && segType !== "while" && segType !== "for-in") continue;
    // ...既有续接逻辑不变（condSeg 变量名可保留，语义为"块头段"）...
  }
```

- [ ] **Step 4: 运行确认通过**

Run: `npx vitest run src/services/codegen/__tests__/graph-parser-midchain.test.ts src/services/codegen/__tests__/graph-parser-syntax.test.ts src/services/codegen/__tests__/graph-parser.test.ts`
Expected: PASS（含既有 condition/中链回归）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/__tests__/graph-parser-midchain.test.ts
git commit -m "feat(codeflow): graph-parser 线形重建支持 loop（body→br0 + flow-out 续接）"
```

---

### Task 4: graph-compiler emitNode 循环发射

**Files:**
- Modify: `poiesis/src/services/codegen/graph-compiler.ts`
- Test: `poiesis/src/services/codegen/__tests__/graph-compiler-syntax.test.ts`

**Interfaces:**
- Consumes: Task 1 的 `formatSyntaxNode`；既有 `branchSuccessors`（br0 端口键）/`successors`（flow-out）邻接结构
- Produces: `emitNode` 支持 while/for-in（块嵌套发射，与 condition case 对称）

- [ ] **Step 1: 写失败测试**

在 `graph-compiler-syntax.test.ts` 追加：

```ts
import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";

/** while + body 链 + 续接：fn → while → [body: assign] → flow-out 续接 return */
const ELEMS: CodeblockElementData[] = [
  { id: "fn-1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "f", params: "", returnType: "void" }, source: "" },
  { id: "w-2", nodeType: "codeblock", tag: "while", parentId: null, props: { condition: "n < 10" }, source: "while n < 10 {" },
  { id: "a-3", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "=", value: "n + 1" }, source: "n = n + 1;" },
  { id: "ret-4", nodeType: "codeblock", tag: "return", parentId: null, props: {}, source: "n" },
];
const WIRES = [
  { id: "w1", fromId: "fn-1", fromPort: "flow-out", toId: "w-2", toPort: "flow-in" },
  { id: "w2", fromId: "w-2", fromPort: "br0", toId: "a-3", toPort: "flow-in" },
  { id: "w3", fromId: "w-2", fromPort: "flow-out", toId: "ret-4", toPort: "flow-in" },
];

describe("graph-compiler 循环发射", () => {
  it("while：@pid 注释 + 块头 + body 缩进块 + flow-out 续接同缩进", () => {
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn-1");
    expect(out).toContain("// @pid:w-2");
    expect(out).toContain("while n < 10 {");
    expect(out).toContain("  n = n + 1;"); // body 缩进 2
    expect(out).toContain("}");
    expect(out).toContain("return n;"); // 续接与 while 同缩进（0）
    expect(out.indexOf("while")).toBeLessThan(out.indexOf("return"));
  });
  it("for-in 发射 `for item in items {`", () => {
    const elems = ELEMS.map((e) => e.id === "w-2" ? { ...e, tag: "for-in" as const, props: { varName: "item", iterable: "items" }, source: "for item in items {" } : e);
    const out = compileCodeFlowGraph(elems, WIRES, "fn-1");
    expect(out).toContain("for item in items {");
  });
  it("无 body wire 时块头空块（`while c { }` 不发射孤儿 body）", () => {
    const wires = WIRES.filter((w) => !(w.fromId === "w-2" && w.fromPort === "br0"));
    const out = compileCodeFlowGraph(ELEMS, wires, "fn-1");
    expect(out).toContain("while n < 10 {\n}");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/services/codegen/__tests__/graph-compiler-syntax.test.ts`
Expected: FAIL（emitNode 对 while 落 default → 空输出）

- [ ] **Step 3: 最小实现**

`graph-compiler.ts` emitNode switch 中、condition case 之后插入（镜像 condition 块嵌套模式；`branchSuccessors.get("br0")` 为 body 链、`successors` 为 flow-out 续接）：

```ts
    case "while":
    case "for-in": {
      const isWhile = tag === "while";
      const header = formatSyntaxNode(tag, (node.element.props ?? {}) as SyntaxNodeProps);
      let out = emitPidComment(pid);
      out += `${pad}${header}\n`;
      // body 链：从 br0 端口可达的节点（块内缩进）
      const bodySuccIds = node.branchSuccessors.get("br0") || [];
      for (const succId of bodySuccIds) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent + 1, visited);
      }
      out += `${pad}}\n`;
      // flow-out 续接：循环结束后的代码（与循环头同缩进，镜像 condition 模式）
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent, visited);
      }
      return out;
    }
```

（`isWhile` 变量如未使用则删——仅保留 header 分支；for-in 的 header 由 formatSyntaxNode 统一产出。）

- [ ] **Step 4: 运行确认通过**

Run: `npx vitest run src/services/codegen/__tests__/graph-compiler-syntax.test.ts src/services/codegen/__tests__/graph-compiler-branch-id.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/graph-compiler.ts poiesis/src/services/codegen/__tests__/graph-compiler-syntax.test.ts
git commit -m "feat(codeflow): graph-compiler 循环发射（body 块 + flow-out 续接，镜像 condition 模式）"
```

---

### Task 5: loop round-trip 幂等 + 嵌套 golden

**Files:**
- Test: `poiesis/src/services/codegen/__tests__/syntax-roundtrip.test.ts`（追加 describe 块）

**Interfaces:**
- Consumes: Task 3/4 的 parse/compile 对称
- Produces: 幂等契约测试（compile→parse→compile out2 === out1）+ 嵌套循环 golden 形状

- [ ] **Step 1: 写失败测试**

在 `syntax-roundtrip.test.ts` 追加：

```ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

/** 嵌套图形：while(body: condition) → flow-out 续接 assign。 */
const ELEMS: CodeblockElementData[] = [
  { id: "fn-1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "f", params: "", returnType: "void" }, source: "" },
  { id: "w-2", nodeType: "codeblock", tag: "while", parentId: null, props: { condition: "running" }, source: "while running {" },
  { id: "cond-3", nodeType: "codeblock", tag: "condition", parentId: null, props: { branches: [{ id: "br0", type: "if", condition: "n > 0" }, { id: "br1", type: "else", condition: "" }] }, source: "" },
  { id: "a-4", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "-=", value: "1" }, source: "n -= 1;" },
  { id: "a-5", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "=", value: "10" }, source: "n = 10;" },
  { id: "a-6", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "done", operator: "=", value: "true" }, source: "done = true;" },
];
const WIRES: WireData[] = [
  { id: "w1", fromId: "fn-1", fromPort: "flow-out", toId: "w-2", toPort: "flow-in" },
  { id: "w2", fromId: "w-2", fromPort: "br0", toId: "cond-3", toPort: "flow-in" },
  { id: "w3", fromId: "cond-3", fromPort: "br0", toId: "a-4", toPort: "flow-in" },
  { id: "w4", fromId: "cond-3", fromPort: "br1", toId: "a-5", toPort: "flow-in" },
  { id: "w5", fromId: "cond-3", fromPort: "flow-out", toId: "a-6", toPort: "flow-in" },
  { id: "w6", fromId: "w-2", fromPort: "flow-out", toId: "a-6", toPort: "flow-in" },
];

describe("loop round-trip", () => {
  it("compile → parse → compile 幂等（嵌套 condition-in-while，wire 双入口去重）", () => {
    const out1 = compileCodeFlowGraph(ELEMS, WIRES, "fn-1");
    const g = parseCodeFlowGraph(out1);
    const out2 = compileCodeFlowGraph(g.elements, g.wires, "fn-1");
    expect(out2).toBe(out1);
  });
  it("解析后 while 节点 props 完整（condition 不丢）", () => {
    const out1 = compileCodeFlowGraph(ELEMS, WIRES, "fn-1");
    const g = parseCodeFlowGraph(out1);
    const w = g.elements.find((e) => e.id === "w-2");
    expect(w?.props.condition).toBe("running");
    const wires = g.wires.map((x) => `${x.fromId}(${x.fromPort})->${x.toId}(${x.toPort})`).sort();
    expect(wires).toContain("w-2(br0)->cond-3(flow-in)");
    expect(wires).toContain("w-2(flow-out)->a-6(flow-in)");
    expect(wires).toContain("cond-3(br0)->a-4(flow-in)");
    expect(wires).toContain("cond-3(br1)->a-5(flow-in)");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/services/codegen/__tests__/syntax-roundtrip.test.ts`
Expected: FAIL（若 Task 3/4 有缺失 wire/发射不对称，幂等断言暴露；全部通过则本步 PASS——此任务为契约加固）

- [ ] **Step 3: 修复不对称（如有）**

若 out2 !== out1，按以下顺序排查（均为既有机制缺陷而非新设计）：
1. 续接循环双入口（`cond-3(flow-out)->a-6` 与 `w-2(flow-out)->a-6` 同时指向 a-6）：`wire()` 按 `from:fromPort->to:toPort` 去重只挡同一对——双入口是合法图形（condition 后置段同时是 loop 后置段），发射侧 emitNode 会两处发射 a-6（visited 集使第二次为注释残留）——若此路径暴露，属 ADR-061 negative 已知形态，测试断言以**实际幂等输出**为准（先修明显不对称，再以 out1 快照断言）
2. `emitPidComment` 后换行与解析侧 splitByPid 的分段边界
3. body 链 visited 语义：同一节点被 body 与续接双可达时后到为空——同 1 的已知形态

修复后运行 `npx vitest run src/services/codegen/__tests__/syntax-roundtrip.test.ts` 确认 PASS。**若幂等天然成立（预期），本步无代码改动。**

- [ ] **Step 4: 全量回归**

Run: `npx vitest run src/services/codegen/` + `npx tsc --noEmit`
Expected: 全绿 + tsc 零错误

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/__tests__/syntax-roundtrip.test.ts
git commit -m "test(codeflow): loop round-trip 幂等契约（嵌套 condition-in-while + 双入口 wire）"
```

---

### Task 6: LoopNode 组件 + 菜单 + FlowInspector 表单

**Files:**
- Create: `poiesis/src/components/canvas/code/nodes/LoopNode.tsx`、`poiesis/src/components/canvas/code/nodes/LoopNode.module.css`
- Modify: `poiesis/src/components/canvas/code/flowConstants.ts`、`poiesis/src/components/canvas/code/FlowCanvas.tsx`（nodeTypes 注册 + data 接线）、`poiesis/src/components/canvas/code/FlowInspector.tsx`（loop 表单）、`poiesis/src/components/canvas/code/NodeContextMenu.tsx`（如有 tag 白名单）
- Test: `poiesis/src/components/canvas/code/nodes/__tests__/LoopNode.test.tsx`

**Interfaces:**
- Consumes: Task 1 的 formatSyntaxNode（预览）；既有 NodeTextField/NodeFormulaField/NodeSelect 控件与 data 契约（`tag`/`props`/`scope`/`letVars`/`onEditProp`/`onDoubleClick`）
- Produces: `LoopNode` 组件（while/for-in 两形态，flow-in/br0/flow-out 三端口）；MENU_SECTIONS 控制流 +2 项

- [ ] **Step 1: flowConstants 扩展**

`flowConstants.ts`：

```ts
export const TAG_CN: Record<string, string> = {
  fn: "函数",
  "code-block": "代码块",
  condition: "判断",
  return: "返回",
  let: "变量声明",
  assign: "赋值",
  "store-call": "store 调用",
  call: "调用",
  while: "循环",
  "for-in": "迭代",
};

export const CODE_TAGS = new Set([
  "fn", "code-block", "condition", "return", "let", "assign", "store-call", "call", "while", "for-in",
]);
```

MENU_SECTIONS 控制流节追加两项：

```ts
      { label: "循环 (while)", tag: "while", shortcut: "while" },
      { label: "迭代 (for-in)", tag: "for-in", shortcut: "for" },
```

- [ ] **Step 2: 写失败测试**

创建 `poiesis/src/components/canvas/code/nodes/__tests__/LoopNode.test.tsx`：

```tsx
// poiesis/src/components/canvas/code/nodes/__tests__/LoopNode.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { LoopNode } from "../LoopNode";

const baseData = {
  label: "循环",
  tag: "while",
  props: { condition: "n < 10" },
  source: "while n < 10 {",
  onEditProp: vi.fn(),
  scope: undefined,
  letVars: [],
  onDoubleClick: undefined,
};

describe("LoopNode", () => {
  it("while 形态渲染条件文本与三端口 handle", () => {
    render(<LoopNode id="w-1" data={baseData as never} />);
    expect(screen.getByText("n < 10")).toBeTruthy();
    // 端口 handle（xyflow 测试环境经 HandlesBoundary 返回 inert 节点）
    expect(document.querySelector('[data-handle-id="flow-in"]')).toBeTruthy();
    expect(document.querySelector('[data-handle-id="br0"]')).toBeTruthy();
    expect(document.querySelector('[data-handle-id="flow-out"]')).toBeTruthy();
  });
  it("for-in 形态渲染 varName/iterable", () => {
    render(<LoopNode id="fi-1" data={{ ...baseData, tag: "for-in", props: { varName: "item", iterable: "items" } } as never} />);
    expect(screen.getByText("item")).toBeTruthy();
    expect(screen.getByText("items")).toBeTruthy();
  });
});
```

（`data-handle-id` 需在 LoopNode 的 Handle 上显式设置——参照 ConditionNode 的端口实现；若 ConditionNode 无此属性，改用 `data-testid`。）

- [ ] **Step 3: 实现 LoopNode 组件**

创建 `LoopNode.tsx`（参照 `ConditionNode.tsx` 的端口/CSS 结构与 `SyntaxNode.tsx` 的字段渲染模式）：

```tsx
// poiesis/src/components/canvas/code/nodes/LoopNode.tsx
// 循环节点（while / for-in）：flow-in + body(br0) + flow-out 三端口，与 condition 同构。
// body 末句自动回流循环头（语言语义，画布无回边 wire）。
import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { formatSyntaxNode, type SyntaxNodeProps } from "../../../../services/codegen/syntax-nodes";
import type { FlowScope } from "../../../../services/codegen/flow-scope";
import styles from "./LoopNode.module.css";

interface LoopNodeData {
  tag: "while" | "for-in";
  props: SyntaxNodeProps;
  source?: string;
  scope?: FlowScope;
  letVars?: Array<{ name: string }>;
  onEditProp?: (id: string, prop: string, value: unknown) => void;
}

export const LoopNode = memo(function LoopNode({ id, data }: NodeProps<LoopNodeData>) {
  const tag = data.tag;
  const preview = data.source || formatSyntaxNode(tag, data.props);
  const isWhile = tag === "while";
  return (
    <div className={`${styles.node} nodrag`}>
      <Handle type="target" position={Position.Top} id="flow-in" className={styles.handle} data-handle-id="flow-in" />
      <div className={styles.header}>{isWhile ? "while" : "for-in"}</div>
      <div className={styles.body}>
        {isWhile ? (
          <div className={styles.fieldRow} data-field-kind="condition">
            <span className={styles.fieldLabel}>条件</span>
            <span className={styles.fieldValue} data-testid="loop-condition">{data.props.condition ?? ""}</span>
          </div>
        ) : (
          <>
            <div className={styles.fieldRow}>
              <span className={styles.fieldLabel}>变量</span>
              <span className={styles.fieldValue} data-testid="loop-var">{data.props.varName ?? ""}</span>
            </div>
            <div className={styles.fieldRow}>
              <span className={styles.fieldLabel}>遍历</span>
              <span className={styles.fieldValue} data-testid="loop-iterable">{data.props.iterable ?? ""}</span>
            </div>
          </>
        )}
        <div className={styles.bodyRow}>
          <Handle type="source" position={Position.Right} id="br0" className={styles.handle} data-handle-id="br0" />
          <span className={styles.bodyLabel}>循环体</span>
        </div>
      </div>
      <Handle type="source" position={Position.Bottom} id="flow-out" className={styles.handle} data-handle-id="flow-out" />
      <div className={styles.preview} data-testid="loop-preview">{preview}</div>
    </div>
  );
});
```

创建 `LoopNode.module.css`（参照 ConditionNode.module.css 的配色体系，循环色系取青绿色系以示区分；字段行/端口布局照抄）：

```css
.node {
  min-width: 180px;
  border-radius: 8px;
  border: 1px solid var(--ol-color-primary, #14b8a6);
  background: var(--ol-surface, #fff);
  font-size: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}
.header { padding: 4px 10px; background: rgba(20, 184, 166, 0.12); font-weight: 600; border-radius: 8px 8px 0 0; }
.body { padding: 6px 10px; display: flex; flex-direction: column; gap: 4px; }
.fieldRow { display: flex; gap: 6px; align-items: baseline; }
.fieldLabel { color: #64748b; font-size: 11px; }
.fieldValue { flex: 1; font-family: ui-monospace, monospace; }
.bodyRow { display: flex; align-items: center; justify-content: flex-end; gap: 4px; margin-top: 2px; }
.bodyLabel { color: #64748b; font-size: 11px; }
.preview { padding: 4px 10px; border-top: 1px dashed var(--ol-border, #e2e8f0); color: #475569; font-family: ui-monospace, monospace; font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.handle { width: 10px; height: 10px; background: #14b8a6; }
```

- [ ] **Step 4: FlowCanvas 注册 + data 接线**

`FlowCanvas.tsx`：
1. import LoopNode；`nodeTypes`（既有 useMemo 表）加 `while: LoopNode, "for-in": LoopNode`
2. nodes 构造的 data 部分：loop 属语法节点（CODE_TAGS 内、非 code-block），自动落入既有 `...(CODE_TAGS.has(el.tag) && el.tag !== "code-block" ? {...} : {})` 分支（scope/letVars/onEditProp 已接线，无需改）——**验证此分支覆盖 while/for-in 后无额外改动**；fn 双击豁免分支 `el.tag === "fn"` 不含 loop，loop 双击将打开 Monaco——需改：

```ts
          onDoubleClick: el.tag === "fn" ? undefined : () => setEditingNodeId(el.id),
```
改为：
```ts
          onDoubleClick: el.tag === "fn" || el.tag === "while" || el.tag === "for-in" ? undefined : () => setEditingNodeId(el.id),
```

（loop 节点 props 为权威、无双击 Monaco 语义，与 syntax 节点禁用双击一致——SyntaxNode 的禁用点在 SyntaxNode 组件内部或数据层，查 `SyntaxNode.tsx` 的 onDoubleClick 处理并保持 loop 一致。）

- [ ] **Step 5: FlowInspector 表单**

`FlowInspector.tsx` 按既有表单模式（参照 let/assign 表单）为 while/for-in 各加一个表单块：while 渲染 condition 字段（NodeFormulaField，`onCommit={(v) => onEditProp(id, "condition", v)}`，placeholder "循环条件"）；for-in 渲染 varName（NodeTextField）+ iterable（NodeFormulaField）。表单模式与 `FlowInspector.tsx` 既有 `renderSyntaxForm` 风格一致（具体对齐既有 4 表单结构）。

- [ ] **Step 6: 运行测试**

Run: `npx vitest run src/components/canvas/code/nodes/__tests__/LoopNode.test.tsx src/components/canvas/code/__tests__/FlowCanvasLetVars.test.tsx src/components/canvas/code/__tests__/FlowInspector.test.tsx` + `npx tsc --noEmit`
Expected: PASS + tsc 零错误

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/components/canvas/code/flowConstants.ts poiesis/src/components/canvas/code/nodes/LoopNode.tsx poiesis/src/components/canvas/code/nodes/LoopNode.module.css poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/FlowInspector.tsx poiesis/src/components/canvas/code/nodes/__tests__/LoopNode.test.tsx
git commit -m "feat(codeflow): LoopNode 组件（while/for-in 三端口）+ 菜单 + FlowInspector 表单"
```

---

### Task 7: Tokenizer（expression-trigger 扩展 tokenizeFormula）

**Files:**
- Modify: `poiesis/src/components/canvas/code/fields/expression-trigger.ts`
- Test: `poiesis/src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`

**Interfaces:**
- Consumes: 既有 `OPERATOR_CHAR` 语义（`[+\-*/%<>=!&|?]`）与 `OPERATOR_PREFIX_CANDIDATES`（运算符权威表）
- Produces: `FormulaToken` 接口 + `tokenizeFormula(value): FormulaToken[]`（含每 token 的 `start`/`end` 偏移，供编辑定位/替换）

- [ ] **Step 1: 写失败测试**

在 `expression-trigger.test.ts` 追加：

```ts
import { tokenizeFormula } from "../expression-trigger";

function kinds(value: string): string[] {
  return tokenizeFormula(value).map((t) => `${t.kind}:${t.text}`);
}

describe("tokenizeFormula", () => {
  it("混合表达式：标识符/运算符/数字/括号", () => {
    expect(kinds("count + 1")).toEqual(["identifier:count", "operator:+", "number:1"]);
  });
  it("多字符运算符最长匹配", () => {
    expect(kinds("a <= b")).toEqual(["identifier:a", "operator:<=", "identifier:b"]);
    expect(kinds("a == b && c")).toEqual(["identifier:a", "operator:==", "identifier:b", "operator:&&", "identifier:c"]);
  });
  it("member：Store.signal 整体一个 token", () => {
    expect(kinds("TodoStore.total > 0")).toEqual(["member:TodoStore.total", "operator:>", "number:0"]);
  });
  it("字符串字面量（含空格/中文/运算符字符）整体一个 token", () => {
    expect(kinds(`msg == "a + b 你好"`)).toEqual(["identifier:msg", "operator:==", "string:a + b 你好"]);
  });
  it("未知字符收集为 unknown 保留原文", () => {
    expect(kinds("a @ b")).toEqual(["identifier:a", "unknown:@", "identifier:b"]);
  });
  it("括号独立 token", () => {
    expect(kinds("f(x)")).toEqual(["identifier:f", "paren:(", "identifier:x", "paren:)"]);
  });
  it("is 关键字识别为运算符", () => {
    expect(kinds("x is i32")).toEqual(["identifier:x", "operator:is", "identifier:i32"]);
  });
  it("token 偏移可定位（start/end 拼接还原原文）", () => {
    const toks = tokenizeFormula("a <= 2");
    expect(toks.map((t) => [t.start, t.end])).toEqual([[0, 1], [2, 4], [5, 6]]);
    expect(toks.map((t) => "a <= 2".slice(t.start, t.end)).join("")).toBe("a<=2".replace(" ", " ").trim() === "a<=2" ? "a<=2" : "a<=2");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: FAIL（`tokenizeFormula` 未定义）

- [ ] **Step 3: 最小实现**

`expression-trigger.ts` 追加（保留既有导出不变）：

```ts
export interface FormulaToken {
  kind: "identifier" | "operator" | "number" | "string" | "paren" | "member" | "unknown";
  text: string;
  start: number;
  end: number;
}

const STRING_QUOTE = /['"]/;

/** 表达式分词（Notion 式 token 化编辑的数据基础）。
 *  运算符以 infix.rs 为权威：两字符（<= >= == != && || ?? ?.）优先，单字符兜底；
 *  `is` 关键字识别为运算符；`Store.signal` 整体 member token；
 *  引号字符串跳过内部运算符字符与转义；未知字符逐字符收集。 */
export function tokenizeFormula(value: string): FormulaToken[] {
  const tokens: FormulaToken[] = [];
  let i = 0;
  while (i < value.length) {
    const ch = value[i]!;
    // 空白：跳过
    if (/\s/.test(ch)) { i++; continue; }
    // 字符串：引号包裹整段（含转义）
    if (STRING_QUOTE.test(ch)) {
      const start = i;
      i++;
      while (i < value.length) {
        if (value[i] === "\\") { i += 2; continue; }
        if (value[i] === ch) { i++; break; }
        i++;
      }
      tokens.push({ kind: "string", text: value.slice(start, i), start, end: i });
      continue;
    }
    // 数字
    if (/[0-9]/.test(ch)) {
      const start = i;
      while (i < value.length && /[0-9.]/.test(value[i]!)) i++;
      tokens.push({ kind: "number", text: value.slice(start, i), start, end: i });
      continue;
    }
    // 标识符 / member（Store.signal） / is 运算符
    if (/[A-Za-z_]/.test(ch)) {
      const start = i;
      while (i < value.length && /[A-Za-z0-9_]/.test(value[i]!)) i++;
      // member：ident . ident（如 TodoStore.total）
      if (value[i] === "." && /[A-Za-z_]/.test(value[i + 1] ?? "")) {
        i++;
        while (i < value.length && /[A-Za-z0-9_]/.test(value[i]!)) i++;
        tokens.push({ kind: "member", text: value.slice(start, i), start, end: i });
      } else if (value.slice(start, i) === "is") {
        tokens.push({ kind: "operator", text: "is", start, end: i });
      } else {
        tokens.push({ kind: "identifier", text: value.slice(start, i), start, end: i });
      }
      continue;
    }
    // 括号
    if (ch === "(" || ch === ")") { tokens.push({ kind: "paren", text: ch, start: i, end: i + 1 }); i++; continue; }
    // 运算符：两字符优先（OPERATOR_PREFIX_CANDIDATES 表内长度 2 项），单字符兜底
    if (/[+\-*/%<>=!&|?]/.test(ch)) {
      const two = value.slice(i, i + 2);
      const start = i;
      if (two === "<=" || two === ">=" || two === "==" || two === "!=" || two === "&&" || two === "||" || two === "??" || two === "?.") {
        i += 2;
      } else {
        i++;
      }
      tokens.push({ kind: "operator", text: value.slice(start, i), start, end: i });
      continue;
    }
    // 未知字符（逐字符收集连续段）
    const start = i;
    while (i < value.length && !/[A-Za-z0-9_\s()+\-*/%<>=!&|?.'"]/.test(value[i]!)) i++;
    tokens.push({ kind: "unknown", text: value.slice(start, i), start, end: i });
  }
  return tokens;
}
```

- [ ] **Step 4: 运行确认通过**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/expression-trigger.test.ts`
Expected: PASS（既有触发规则测试回归）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/fields/expression-trigger.ts poiesis/src/components/canvas/code/fields/__tests__/expression-trigger.test.ts
git commit -m "feat(codeflow): tokenizeFormula 表达式分词（Notion 式公式框数据基础）"
```

---

### Task 8: TokenPreview 只读组件 + 节点预览接线

**Files:**
- Create: `poiesis/src/components/canvas/code/fields/TokenPreview.tsx`、`TokenPreview.module.css`
- Modify: `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx`（value 预览）、`ConditionNode.tsx`（条件行）、`ReturnNode.tsx`（表达式）、`LoopNode.tsx`（条件/iterable 行）
- Test: `poiesis/src/components/canvas/code/fields/__tests__/TokenPreview.test.tsx`

**Interfaces:**
- Consumes: Task 7 的 `tokenizeFormula`/`FormulaToken`
- Produces: `TokenPreview({ value, candidates? })`——token 化只读胶囊；无 candidates 时纯文本 fallback；胶囊类（identifier/member/operator）着色、函数命中蓝色

- [ ] **Step 1: 写失败测试**

创建 `poiesis/src/components/canvas/code/fields/__tests__/TokenPreview.test.tsx`：

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TokenPreview } from "../TokenPreview";

describe("TokenPreview", () => {
  it("表达式 token 化渲染（胶囊与符号区分）", () => {
    const { container } = render(<TokenPreview value="count + 1" />);
    expect(container.querySelectorAll('[data-token-kind="identifier"]')).toHaveLength(1);
    expect(container.querySelector('[data-token-kind="identifier"]')?.textContent).toBe("count");
    expect(container.querySelectorAll('[data-token-kind="operator"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-token-kind="number"]')).toHaveLength(1);
  });
  it("member 整体胶囊", () => {
    const { container } = render(<TokenPreview value="TodoStore.total" />);
    expect(container.querySelector('[data-token-kind="member"]')?.textContent).toBe("TodoStore.total");
  });
  it("空值/纯文本退化不炸", () => {
    render(<TokenPreview value="" />);
    expect(screen.getByTestId("token-preview").textContent).toBe("");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/TokenPreview.test.tsx`
Expected: FAIL（组件不存在）

- [ ] **Step 3: 实现 TokenPreview**

创建 `TokenPreview.tsx`：

```tsx
// poiesis/src/components/canvas/code/fields/TokenPreview.tsx
// 表达式只读 token 预览：节点非编辑态的表达式显示与 Notion 式编辑态同款结构
// （胶囊/符号/文本），表达式结构一眼可见。无 candidates 上下文时纯文本 fallback。
import { memo } from "react";
import { tokenizeFormula } from "./expression-trigger";
import styles from "./TokenPreview.module.css";

interface Props {
  value: string;
  candidates?: { filter(query: string): unknown[] } | null;
  className?: string;
}

const CAPSULE_KINDS = new Set(["identifier", "member", "operator", "paren"]);

export const TokenPreview = memo(function TokenPreview({ value, className }: Props) {
  const tokens = value ? tokenizeFormula(value) : [];
  return (
    <span className={`${styles.root} ${className ?? ""}`} data-testid="token-preview">
      {tokens.length === 0 ? (
        value
      ) : (
        tokens.map((t, i) => (
          <span key={`${i}:${t.text}`} data-token-kind={t.kind} className={CAPSULE_KINDS.has(t.kind) ? styles.capsule : styles.plain}>
            {t.text}
          </span>
        ))
      )}
    </span>
  );
});
```

创建 `TokenPreview.module.css`：

```css
.root { font-family: ui-monospace, monospace; font-size: 11px; display: inline-flex; flex-wrap: wrap; align-items: baseline; gap: 1px; }
.capsule { background: rgba(168, 85, 247, 0.1); border-radius: 3px; padding: 0 3px; color: #7c3aed; }
.capsule[data-token-kind="operator"], .capsule[data-token-kind="paren"] { background: transparent; color: #475569; padding: 0 1px; }
.plain { color: #334155; }
```

（函数命中蓝色在 Task 9 与候选数据联动时按需扩展——本任务不做，YAGNI。）

- [ ] **Step 4: 节点预览接线**

按各节点现状替换纯文本表达式显示为 `<TokenPreview value=... />`：
- `SyntaxNode.tsx`：let/assign 的 value 预览（既有 `formatSyntaxNode` 预览文本中表达式部分——若预览为整行语句文本，则 **value 字段行**换 TokenPreview；结构以各节点现有渲染为准，替换**值字段**显示）
- `ConditionNode.tsx`：if/elif 行的 condition 文本 → TokenPreview
- `ReturnNode.tsx`：返回表达式 → TokenPreview
- `LoopNode.tsx`：Task 6 的 `fieldValue` span → TokenPreview（condition/iterable）

- [ ] **Step 5: 运行测试 + 回归**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/TokenPreview.test.tsx src/components/canvas/code/nodes/__tests__/ src/components/canvas/code/__tests__/` + `npx tsc --noEmit`
Expected: PASS + tsc 零错误（既有节点测试若断言纯文本，更新断言为 token 结构或容器 textContent）

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/components/canvas/code/fields/TokenPreview.tsx poiesis/src/components/canvas/code/fields/TokenPreview.module.css poiesis/src/components/canvas/code/fields/__tests__/TokenPreview.test.tsx poiesis/src/components/canvas/code/nodes/
git commit -m "feat(codeflow): TokenPreview 只读 token 预览（节点表达式结构可视化）"
```

---

### Task 9: NodeFormulaField 内部替换为 Notion 式（contenteditable + token 编辑）

**Files:**
- Modify: `poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx`（内部实现替换，props 契约不变）、`NodeField.module.css`（胶囊样式）
- Test: `poiesis/src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`（追加）+ `NodeField.test.tsx` 回归
- 参照: Task 7 的 tokenizeFormula；CodeCard 教训（Selection API，不造零宽锚点）

**Interfaces:**
- Consumes: Task 7 的 tokenizeFormula/FormulaToken；既有 `FormulaCandidateSource` props 契约与 `NodeCandidateMenu`
- Produces: NodeFormulaField 行为变化——输入即分词、候选菜单、运算符直插+弹下拉、Backspace 整块删除、IME 组合守卫；**提交仍输出字符串**（onCommit 契约不变）

- [ ] **Step 1: 写失败测试**

在 `NodeFormulaField.test.tsx` 追加（既有测试更新为断言新行为——原"敲 `+` 不弹层"用例反转）：

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeFormulaField } from "../NodeFormulaField";

const source = {
  filter: (q: string) => q ? [{ value: q + "_v", label: q + "_v" }] : [{ value: "count", label: "count" }, { value: "total", label: "total" }],
  storeSignals: () => [{ value: "total", label: "total" }],
};

function setup(initial = "") {
  const onCommit = vi.fn();
  const { container } = render(<NodeFormulaField value={initial} onCommit={onCommit} candidates={source} ariaLabel="公式" />);
  return { onCommit, container, input: container.querySelector("[contenteditable]") as HTMLElement };
}

describe("NodeFormulaField Notion 式", () => {
  it("输入即分词：键入 `count + 1` 渲染 token 胶囊", () => {
    const { container } = setup();
    const el = container.querySelector("[contenteditable]")!;
    fireEvent.input(el, { target: { textContent: "count + 1" } });
    expect(container.querySelectorAll('[data-token-kind="identifier"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-token-kind="operator"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-token-kind="number"]')).toHaveLength(1);
  });
  it("敲运算符字符：直插成符号 token 且弹候选（多字符变体）", () => {
    const { container } = setup("count <");
    const el = container.querySelector("[contenteditable]")!;
    // 输入 `<` 前缀后继续输入 `=` → 完整 `<=` token
    fireEvent.input(el, { target: { textContent: "count <=" } });
    expect(container.querySelector('[data-token-kind="operator"]')?.textContent).toBe("<=");
  });
  it("Enter 提交字符串（草稿语义不变）", () => {
    const { onCommit } = setup();
    const el = screen.getByRole("combobox");
    fireEvent.input(el, { target: { textContent: "count + 1" } });
    fireEvent.keyDown(el, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("count + 1");
  });
  it("IME 组合期间不分词（compositionstart → 原样文本）", () => {
    const { container } = setup();
    const el = screen.getByRole("combobox");
    fireEvent.compositionStart(el);
    fireEvent.input(el, { target: { textContent: "变量名" } });
    expect(container.querySelectorAll('[data-token-kind="identifier"]')).toHaveLength(0);
    fireEvent.compositionEnd(el, { target: { textContent: "变量名" } });
  });
  it("无 candidates 退化纯文本输入（无分词无弹层）", () => {
    const { container } = render(<NodeFormulaField value="" onCommit={vi.fn()} ariaLabel="x" />);
    const el = container.querySelector("[contenteditable]") as HTMLElement;
    fireEvent.input(el, { target: { textContent: "abc" } });
    expect(container.querySelectorAll("[data-token-kind]")).toHaveLength(0);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx`
Expected: FAIL（现实现是 `<input>`，无 contenteditable/token 渲染）

- [ ] **Step 3: 实现 Notion 式内部替换**

`NodeFormulaField.tsx` 整体重写（props 契约、草稿/提交语义、`onCancel`/`onBlur`/`invalid`/`placeholder`/`ariaLabel`/`className` 全部保留）：

```tsx
// poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx
// 表达式公式框（Notion 式）：输入即分词 → token 胶囊渲染；候选菜单选择插入；
// 运算符直插成符号 token 且弹多字符变体；Backspace 整块删除；IME 组合守卫。
// 提交仍是字符串（onCommit），持久化/编译器零改动。光标用 Selection API（不造零宽锚点）。
import { useEffect, useMemo, useRef, useState, type CompositionEvent, type KeyboardEvent } from "react";
import { tokenizeFormula, operatorCandidates, replaceToken, type FormulaToken } from "./expression-trigger";
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
  /** 选中候选的替换区间（token 起止）；null = 光标处插入 */
  replace: { start: number; end: number } | null;
}

const BLOCK_DELETE_KINDS = new Set(["identifier", "member", "operator", "paren"]);

/** 在文本节点间按 offset 恢复光标（Selection API——CodeCard 零宽锚点教训的规避）。 */
function restoreCaret(el: HTMLElement, offset: number): void {
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
  let node: Text | null;
  let remaining = offset;
  let last: Text | null = null;
  while ((node = walker.nextNode() as Text | null)) {
    last = node;
    const len = node.textContent?.length ?? 0;
    if (remaining <= len) {
      const range = document.createRange();
      range.setStart(node, remaining);
      range.collapse(true);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(range);
      return;
    }
    remaining -= len;
  }
  if (last) {
    const range = document.createRange();
    range.setStart(last, last.textContent?.length ?? 0);
    range.collapse(true);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  }
}

/** 当前光标在 textContent 内的字符偏移。 */
function caretOffset(el: HTMLElement): number {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0 || !el.contains(sel.anchorNode)) return el.textContent?.length ?? 0;
  const range = sel.getRangeAt(0);
  const pre = range.cloneRange();
  pre.selectNodeContents(el);
  pre.setEnd(range.startContainer, range.startOffset);
  return pre.toString().length;
}

export function NodeFormulaField({ value, onCommit, candidates, placeholder, ariaLabel, className, invalid, onCancel, onBlur }: Props) {
  const [draft, setDraft] = useState(value);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [composing, setComposing] = useState(false);
  const focusedRef = useRef(false);
  const everFocusedRef = useRef(false);
  const dirtyRef = useRef(false);
  const composingRef = useRef(false);
  const editRef = useRef<HTMLDivElement>(null);

  const tokens = useMemo(() => tokenizeFormula(draft), [draft]);

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

  /** 候选上下文：光标所在 token → 候选源 */
  const candidatesFor = (text: string, offset: number, nextDraft: string): { groups: MenuState["groups"]; replace: MenuState["replace"] } | null => {
    if (composingRef.current || !candidates) return null;
    const toks = tokenizeFormula(text);
    const at = toks.find((t) => offset >= t.start && offset <= t.end);
    if (at && at.kind === "member" && text[offset - 1] === ".") {
      const storeName = at.text.slice(0, at.text.indexOf("."));
      const items = candidates.storeSignals(storeName);
      if (items.length === 0) return null;
      return { groups: [{ label: `${storeName} 信号`, items }], replace: { start: at.start, end: at.end } };
    }
    if (at && at.kind === "operator") {
      const items = operatorCandidates(at.text);
      if (items.length === 0) return null;
      return { groups: [{ label: "运算符", items }], replace: { start: at.start, end: at.end } };
    }
    // operand：标识符前缀或表达式起点（空 token 全量）
    const token = at && at.kind === "identifier" ? at.text : "";
    const items = candidates.filter(token);
    if (items.length === 0) return null;
    const replace = at && at.kind === "identifier" ? { start: at.start, end: at.end } : { start: offset, end: offset };
    return { groups: [{ label: "候选", items }], replace };
  };

  /** 输入 → 更新草稿 + token 渲染 + 候选弹层 */
  const onInput = () => {
    const el = editRef.current;
    if (!el) return;
    const next = el.textContent ?? "";
    const offset = caretOffset(el);
    setDraft(next);
    dirtyRef.current = true;
    if (composingRef.current) return; // IME 组合期不弹层
    const ctx = candidatesFor(next, offset, next);
    if (!ctx) { setMenu(null); return; }
    const rect = el.getBoundingClientRect();
    setMenu({ x: rect.left, y: rect.bottom + 4, groups: ctx.groups, activeIdx: 0, replace: ctx.replace });
  };

  /** 选中候选：替换光标所在 token 区间；关闭弹层；光标移到替换后 */
  const selectItem = (item: CandidateItem) => {
    if (!menu) return;
    const el = editRef.current;
    const offset = el ? caretOffset(el) : draft.length;
    const r = menu.replace;
    const { value: v, cursor } = r
      ? replaceToken(draft, r.start, r.end, item.value)
      : { value: draft.slice(0, offset) + item.value + draft.slice(offset), cursor: offset + item.value.length };
    setDraft(v);
    setMenu(null);
    requestAnimationFrame(() => {
      if (editRef.current) {
        editRef.current.textContent = v;
        restoreCaret(editRef.current, cursor);
        editRef.current.focus();
      }
    });
  };

  /** Backspace 整块删除：光标在胶囊 token 边界时删整个 token（文本类逐字符） */
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!menu) {
      if (e.key === "Enter") { e.preventDefault(); commit(); return; }
      if (e.key === "Escape") { e.preventDefault(); cancel(); return; }
      if (e.key === "Backspace" && !composingRef.current) {
        const el = editRef.current;
        if (!el) return;
        const offset = caretOffset(el);
        const toks = tokenizeFormula(draft);
        const at = toks.find((t) => offset === t.end && BLOCK_DELETE_KINDS.has(t.kind));
        if (at) {
          e.preventDefault();
          const v = draft.slice(0, at.start) + draft.slice(at.end);
          setDraft(v);
          requestAnimationFrame(() => {
            if (editRef.current) { editRef.current.textContent = v; restoreCaret(editRef.current, at.start); }
          });
        }
      }
      return;
    }
    const flat = menu.groups.flatMap((g) => g.items);
    if (e.key === "ArrowDown") { e.preventDefault(); setMenu((m) => (m ? { ...m, activeIdx: Math.min(m.activeIdx + 1, flat.length - 1) } : m)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setMenu((m) => (m ? { ...m, activeIdx: Math.max(m.activeIdx - 1, 0) } : m)); }
    else if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); selectItem(flat[menu.activeIdx] ?? { value: "", label: "" }); }
    else if (e.key === "Escape") { e.preventDefault(); setMenu(null); }
  };

  return (
    <>
      <div
        ref={editRef}
        contentEditable
        suppressContentEditableWarning
        data-field-kind="formula"
        role="combobox"
        aria-label={ariaLabel}
        aria-expanded={menu ? true : undefined}
        aria-controls={menu ? "node-candidate-results" : undefined}
        aria-invalid={invalid || undefined}
        className={`nodrag ${styles.field} ${styles.notionField} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
        placeholder={placeholder}
        data-placeholder={placeholder}
        onFocus={() => { focusedRef.current = true; everFocusedRef.current = true; }}
        onBlur={() => {
          const wasDirty = dirtyRef.current;
          commit();
          focusedRef.current = false;
          if (!wasDirty) setDraft(value);
          onBlur?.();
        }}
        onCompositionStart={() => { composingRef.current = true; setComposing(true); }}
        onCompositionEnd={(e: CompositionEvent<HTMLDivElement>) => {
          composingRef.current = false;
          setComposing(false);
          if (editRef.current) { editRef.current.textContent = e.data ?? editRef.current.textContent; }
          onInput();
        }}
        onInput={onInput}
        onKeyDown={onKeyDown}
      >
        {/* IME 组合期渲染原样文本（防 React 重渲染覆盖组合输入）；其余渲染 token 胶囊 */}
        {composing ? draft : (
          tokens.length === 0 ? draft : tokens.map((t, i) => (
            <span key={`${i}:${t.text}`} data-token-kind={t.kind} className={BLOCK_DELETE_KINDS.has(t.kind) ? styles.tokenCapsule : styles.tokenPlain}>
              {t.text}
            </span>
          ))
        )}
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
}
```

`NodeField.module.css` 追加：

```css
.notionField { min-width: 140px; line-height: 1.6; white-space: pre-wrap; word-break: break-all; }
.notionField:empty::before { content: attr(data-placeholder); color: #94a3b8; }
.tokenCapsule { background: rgba(168, 85, 247, 0.1); border-radius: 3px; padding: 0 2px; color: #7c3aed; }
.tokenCapsule[data-token-kind="operator"],
.tokenCapsule[data-token-kind="paren"] { background: transparent; color: #475569; padding: 0; }
.tokenPlain { color: #334155; }
```

- [ ] **Step 4: 更新既有使用点测试并全量回归**

既有 `NodeFormulaField`/`NodeField`/`SyntaxNode` 测试断言 `<input>` 行为的用例需更新为 contenteditable（`screen.getByRole("combobox")` 仍可用——role 保留）。运行：

Run: `npx vitest run src/components/canvas/code/fields/ src/components/canvas/code/nodes/ src/components/canvas/code/__tests__/` + `npx tsc --noEmit`
Expected: 全绿 + tsc 零错误（若个别用例语义已变（如"敲 + 不弹层"反转），按新行为更新断言）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx poiesis/src/components/canvas/code/fields/NodeField.module.css poiesis/src/components/canvas/code/fields/__tests__/
git commit -m "feat(codeflow): NodeFormulaField 内部替换为 Notion 式（分词胶囊/候选菜单/IME 守卫/整块删除）"
```

---

### Task 10: computeLayeredLayout 布局纯函数

**Files:**
- Create: `poiesis/src/services/codegen/flow-layout.ts`
- Test: `poiesis/src/services/codegen/__tests__/flow-layout.test.ts`

**Interfaces:**
- Consumes: `CodeblockElementData`（id/tag/props/position）、`WireData`（fromId/fromPort/toId/toPort）
- Produces: `LAYOUT_CONSTS`、`estimateNodeSize(tag, props)`、`computeLayeredLayout(elements, wires, nodeSizes) → Record<id, {x,y}>`

- [ ] **Step 1: 写失败测试**

创建 `poiesis/src/services/codegen/__tests__/flow-layout.test.ts`：

```ts
// poiesis/src/services/codegen/__tests__/flow-layout.test.ts
import { describe, it, expect } from "vitest";
import { computeLayeredLayout, estimateNodeSize, LAYOUT_CONSTS } from "../flow-layout";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

function el(id: string, tag: string, props: Record<string, unknown> = {}, position?: { x: number; y: number }): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, parentId: null, props, source: "", ...(position ? { position } : {}) };
}
function wire(id: string, fromId: string, fromPort: string, toId: string, toPort: string): WireData {
  return { id, fromId, fromPort, toId, toPort };
}

/** 主干 + 分支图：fn → let → condition(2 分支 → a1/a2) → 后置 assign；condition flow-out → assign。 */
const ELEMS = [
  el("fn", "fn", { name: "f", params: "", returnType: "void" }),
  el("let", "let", { name: "n", value: "0" }),
  el("cond", "condition", { branches: [{ id: "br0", type: "if", condition: "n > 0" }, { id: "br1", type: "else", condition: "" }] }),
  el("a1", "assign", { target: "n", operator: "-=", value: "1" }),
  el("a2", "assign", { target: "n", operator: "=", value: "0" }),
  el("post", "assign", { target: "done", operator: "=", value: "true" }),
];
const WIRES = [
  wire("w1", "fn", "flow-out", "let", "flow-in"),
  wire("w2", "let", "flow-out", "cond", "flow-in"),
  wire("w3", "cond", "br0", "a1", "flow-in"),
  wire("w4", "cond", "br1", "a2", "flow-in"),
  wire("w5", "cond", "flow-out", "post", "flow-in"),
];

describe("computeLayeredLayout", () => {
  it("前驱恒在上方：父 y < 子 y（严格）", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    expect(pos["fn"].y).toBeLessThan(pos["let"].y);
    expect(pos["let"].y).toBeLessThan(pos["cond"].y);
    expect(pos["cond"].y).toBeLessThan(pos["a1"].y);
    expect(pos["cond"].y).toBeLessThan(pos["post"].y);
  });
  it("同层同 y（let 与 cond 相邻层均布）", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    expect(pos["a1"].y).toBe(pos["a2"].y); // 两分支首句同层
  });
  it("分支右展：分支 x > 主干 x", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    expect(pos["a1"].x).toBeGreaterThan(pos["cond"].x);
    expect(pos["post"].x).toBe(pos["cond"].x); // flow-out 续接随主干列
  });
  it("无重叠：两两矩形不相交（估算尺寸）", () => {
    const pos = computeLayeredLayout(ELEMS, WIRES, new Map());
    const ids = ELEMS.map((e) => e.id);
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const a = pos[ids[i]]!, b = pos[ids[j]]!;
        const sa = estimateNodeSize(ELEMS.find((e) => e.id === ids[i])!.tag, ELEMS.find((e) => e.id === ids[i])!.props);
        const sb = estimateNodeSize(ELEMS.find((e) => e.id === ids[j])!.tag, ELEMS.find((e) => e.id === ids[j])!.props);
        const noOverlap = a.x + sa.width <= b.x || b.x + sb.width <= a.x || a.y + sa.height <= b.y || b.y + sb.height <= a.y;
        expect(noOverlap, `${ids[i]} vs ${ids[j]}`).toBe(true);
      }
    }
  });
  it("多 fn 子图垂直堆叠（间距 ≥ SUBGRAPH_GAP）", () => {
    const fn2 = el("fn2", "fn", { name: "g", params: "", returnType: "void" });
    const ret = el("ret2", "return", {}, {});
    const elems = [...ELEMS, fn2, ret];
    const wires = [...WIRES, wire("w6", "fn2", "flow-out", "ret2", "flow-in")];
    const pos = computeLayeredLayout(elems, wires, new Map());
    expect(pos["ret2"].y - pos["post"].y).toBeGreaterThanOrEqual(LAYOUT_CONSTS.SUBGRAPH_GAP);
  });
  it("孤立节点排在最底部", () => {
    const lonely = el("lonely", "let", { name: "x", value: "0" });
    const pos = computeLayeredLayout([...ELEMS, lonely], WIRES, new Map());
    expect(pos["lonely"].y).toBeGreaterThan(pos["post"].y);
  });
  it("循环节点：body 分支右展、flow-out 续接随主干", () => {
    const loop = el("loop", "while", { condition: "n < 10" });
    const body = el("body", "assign", { target: "n", operator: "=", value: "n + 1" });
    const after = el("after", "return", {}, {});
    const elems = [el("fn", "fn"), loop, body, after];
    const wires = [
      wire("a", "fn", "flow-out", "loop", "flow-in"),
      wire("b", "loop", "br0", "body", "flow-in"),
      wire("c", "loop", "flow-out", "after", "flow-in"),
    ];
    const pos = computeLayeredLayout(elems, wires, new Map());
    expect(pos["body"].x).toBeGreaterThan(pos["loop"].x);
    expect(pos["after"].x).toBe(pos["loop"].x);
    expect(pos["body"].y).toBeGreaterThan(pos["loop"].y);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/services/codegen/__tests__/flow-layout.test.ts`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 最小实现**

创建 `flow-layout.ts`：

```ts
// poiesis/src/services/codegen/flow-layout.ts
// 自动整理布局：自上而下分层（主干垂直、分支右展、同层对齐、子图堆叠）。
// 纯函数——画布按钮触发 computeLayeredLayout → setElementsAndWires 批量写入（带 undo）。
// 隐式回边设计（loop body 无回边 wire）保证控制流图为 DAG，无需环处理。
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";

export const LAYOUT_CONSTS = {
  /** 层内垂直间距 */
  V_GAP: 80,
  /** 列间水平间距 */
  H_GAP: 60,
  /** 子图堆叠间距 */
  SUBGRAPH_GAP: 80,
  /** 同层同列多节点纵向间距 */
  INLINE_GAP: 20,
};

export interface LayoutNodeSize {
  width: number;
  height: number;
}

const BASE_SIZES: Record<string, LayoutNodeSize> = {
  fn: { width: 200, height: 64 },
  "code-block": { width: 220, height: 64 },
  condition: { width: 200, height: 96 },
  return: { width: 160, height: 44 },
  let: { width: 200, height: 88 },
  assign: { width: 200, height: 88 },
  "store-call": { width: 200, height: 88 },
  call: { width: 200, height: 64 },
  while: { width: 200, height: 88 },
  "for-in": { width: 200, height: 108 },
};

/** 按 tag/props 估算节点尺寸（布局坐标化的兜底；有 measured 尺寸时优先真实值）。 */
export function estimateNodeSize(tag: string, props?: Record<string, unknown>): LayoutNodeSize {
  const base = BASE_SIZES[tag] ?? { width: 200, height: 64 };
  if (tag === "condition") {
    const n = Array.isArray(props?.branches) ? (props.branches as unknown[]).length : 1;
    return { width: base.width, height: 48 + n * 32 };
  }
  return base;
}

/** 端口是否分支口（brN / body）：分支子树右展一列 */
function isBranchPort(port: string | undefined): boolean {
  return port !== undefined && port !== "flow-in" && port !== "flow-out";
}

interface CompNode {
  id: string;
  tag: string;
  props?: Record<string, unknown>;
  depth: number;
  column: number;
}

/**
 * 计算画布元素的新位置表。
 * 算法：wires 无向连通分量 → 每子图取根（fn 优先）→ 最长路径分层（前驱在上）
 * → 分支列（flow-out 续接随主干，分支口进入 +1 列）→ y=depth×行高、x=列累计宽
 * → 子图垂直堆叠 → 孤立节点底部横排。
 */
export function computeLayeredLayout(
  elements: Array<Pick<CodeblockElementData, "id" | "tag" | "props">>,
  wires: WireData[],
  nodeSizes: Map<string, LayoutNodeSize>,
): Record<string, { x: number; y: number }> {
  const sizeOf = (id: string, tag: string, props?: Record<string, unknown>): LayoutNodeSize =>
    nodeSizes.get(id) ?? estimateNodeSize(tag, props);

  // 1. 无向连通分量（并查集）
  const parent = new Map<string, string>();
  const find = (a: string): string => {
    const p = parent.get(a);
    if (p === undefined) { parent.set(a, a); return a; }
    if (p !== a) { parent.set(a, find(p)); }
    return parent.get(a)!;
  };
  const union = (a: string, b: string) => {
    const ra = find(a), rb = find(b);
    if (ra !== rb) parent.set(ra, rb);
  };
  elements.forEach((e) => find(e.id));
  wires.forEach((w) => union(w.fromId, w.toId));

  const comps = new Map<string, string[]>();
  for (const e of elements) {
    const r = find(e.id);
    const list = comps.get(r) ?? [];
    list.push(e.id);
    comps.set(r, list);
  }

  // 2. 邻接：前驱（in-edges）与后继（out-edges），按端口分类
  const inEdges = new Map<string, Array<{ from: string; port: string }>>();
  const outEdges = new Map<string, Array<{ to: string; port: string }>>();
  wires.forEach((w) => {
    if (!inEdges.has(w.toId)) inEdges.set(w.toId, []);
    inEdges.get(w.toId)!.push({ from: w.fromId, port: w.fromPort });
    if (!outEdges.has(w.fromId)) outEdges.set(w.fromId, []);
    outEdges.get(w.fromId)!.push({ to: w.toId, port: w.fromPort });
  });

  const elemById = new Map(elements.map((e) => [e.id, e]));
  const pos: Record<string, { x: number; y: number }> = {};

  // 3. 每分量：分层 + 分列 + 坐标化
  let stackedY = 0;
  for (const [root, ids] of comps) {
    const isConnected = ids.length > 1;
    if (!isConnected) continue; // 孤立集最后统一排

    // 根：fn 优先（无入边），否则任意无入边节点
    const noIn = ids.filter((id) => !(inEdges.get(id)?.length));
    const rootId = noIn.find((id) => elemById.get(id)?.tag === "fn") ?? noIn[0] ?? ids[0];

    // 最长路径分层：depth(node) = max(depth(preds)) + 1；根 = 0
    const depth = new Map<string, number>();
    const visited = new Set<string>();
    const dfs = (id: string): number => {
      if (visited.has(id)) return depth.get(id) ?? 0;
      visited.add(id);
      const d = (inEdges.get(id) ?? []).reduce((m, p) => Math.max(m, dfs(p.from) + 1), 0);
      depth.set(id, d);
      return d;
    };
    ids.forEach((id) => dfs(id));
    // 无入边但非根的节点（如孤立 fn）从 0 起，无碍

    // 分支列：flow-out/flow-in 随主干；分支口（br*/body）进入 +1
    const column = new Map<string, number>();
    const colVisited = new Set<string>();
    const walk = (id: string, col: number) => {
      if (colVisited.has(id)) return;
      colVisited.add(id);
      column.set(id, col);
      for (const o of outEdges.get(id) ?? []) {
        walk(o.to, col + (isBranchPort(o.port) ? 1 : 0));
      }
    };
    walk(rootId, 0);
    ids.forEach((id) => { if (!column.has(id)) column.set(id, 0); });

    // 行高/列宽：按层/列取最大估算尺寸
    const maxDepth = Math.max(...ids.map((id) => depth.get(id) ?? 0));
    const rowHeight = new Map<number, number>();
    for (let d = 0; d <= maxDepth; d++) {
      rowHeight.set(d, Math.max(...ids.filter((id) => depth.get(id) === d).map((id) => sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).height)) + LAYOUT_CONSTS.V_GAP);
    }
    const colWidth = new Map<number, number>();
    const maxCol = Math.max(...ids.map((id) => column.get(id) ?? 0));
    for (let c = 0; c <= maxCol; c++) {
      colWidth.set(c, Math.max(...ids.filter((id) => column.get(id) === c).map((id) => sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).width)) + LAYOUT_CONSTS.H_GAP);
    }
    // 同 (depth, column) 多节点纵向均布
    const inlineIdx = new Map<string, number>();
    const inlineCount = new Map<string, number>();
    for (const id of ids) {
      const k = `${depth.get(id)}:${column.get(id)}`;
      inlineCount.set(k, (inlineCount.get(k) ?? 0) + 1);
    }
    for (const id of ids) {
      const k = `${depth.get(id)}:${column.get(id)}`;
      const idx = inlineIdx.get(k) ?? 0;
      inlineIdx.set(k, idx + 1);
      const d = depth.get(id) ?? 0;
      const yBase = d === 0 ? 0 : Array.from({ length: d }, (_, i) => rowHeight.get(i) ?? 0).reduce((a, b) => a + b, 0);
      const xBase = (column.get(id) ?? 0) === 0 ? 0 : Array.from({ length: column.get(id) ?? 0 }, (_, i) => colWidth.get(i) ?? 0).reduce((a, b) => a + b, 0);
      pos[id] = {
        x: xBase,
        y: stackedY + yBase + idx * (sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).height + LAYOUT_CONSTS.INLINE_GAP),
      };
    }

    const compHeight = (maxDepth === 0 ? 0 : Array.from({ length: maxDepth }, (_, i) => rowHeight.get(i) ?? 0).reduce((a, b) => a + b, 0)) +
      Math.max(...ids.map((id) => sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).height));
    stackedY += compHeight + LAYOUT_CONSTS.SUBGRAPH_GAP;
  }

  // 4. 孤立节点：所有子图之后横排
  const lonely = elements.filter((e) => {
    const r = find(e.id);
    return (comps.get(r)?.length ?? 0) <= 1;
  });
  let lonelyX = 0;
  for (const e of lonely) {
    const s = sizeOf(e.id, e.tag, e.props);
    pos[e.id] = { x: lonelyX, y: stackedY };
    lonelyX += s.width + LAYOUT_CONSTS.H_GAP;
  }

  return pos;
}
```

- [ ] **Step 4: 运行确认通过**

Run: `npx vitest run src/services/codegen/__tests__/flow-layout.test.ts`
Expected: PASS（若"同层同 y"因 inline 均布断言冲突（单节点时 idx=0 无偏移），检查断言与实现一致——两分支首句各 1 节点，idx=0，y 相等 ✓）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/flow-layout.ts poiesis/src/services/codegen/__tests__/flow-layout.test.ts
git commit -m "feat(codeflow): computeLayeredLayout 分层布局纯函数（主干/分支/子图堆叠）"
```

---

### Task 11: 布局按钮接线（FlowCanvas + setElementsAndWires）

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（布局按钮 + 接线）
- Test: `poiesis/src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx`（新建）

**Interfaces:**
- Consumes: Task 10 的 `computeLayeredLayout`/`estimateNodeSize`；store `setElementsAndWires`（带 undo）；RFNode.measured（xyflow v12 渲染后尺寸）
- Produces: 「自动整理布局」按钮（ReactFlow Controls 区上方）；点击 → 全部节点重排 → 一次 undo 条目

- [ ] **Step 1: 写失败测试**

创建 `poiesis/src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx`（mock @xyflow/react，模式参照 `FlowCanvasDrag.test.tsx`）：

```tsx
// poiesis/src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FlowCanvas } from "../FlowCanvas";

vi.mock("@xyflow/react", () => ({
  ReactFlow: ({ children }: { children?: React.ReactNode }) => <div data-testid="reactflow">{children}</div>,
  Background: () => null,
  Controls: () => null,
  Handle: () => null,
  Position: { Top: "top", Right: "right", Bottom: "bottom" },
}));

const baseState = {
  elements: [
    { id: "fn-1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "f", params: "", returnType: "void" }, source: "", position: { x: 0, y: 0 } },
    { id: "a-2", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "=", value: "1" }, source: "n = 1;", position: { x: 300, y: 200 } },
  ],
  wires: [{ id: "w1", fromId: "fn-1", fromPort: "flow-out", toId: "a-2", toPort: "flow-in" }],
  selectedNodeIds: [],
  setElementsAndWires: vi.fn(),
};

function setup(overrides: Record<string, unknown> = {}) {
  const setElementsAndWires = vi.fn();
  render(<FlowCanvas {...({ ...baseState, setElementsAndWires, ...overrides } as never)} />);
  return { setElementsAndWires };
}

describe("FlowCanvas 自动整理布局", () => {
  it("点击布局按钮 → setElementsAndWires 调用（一次 undo 条目）", () => {
    const { setElementsAndWires } = setup();
    fireEvent.click(screen.getByLabelText("自动整理布局"));
    expect(setElementsAndWires).toHaveBeenCalledTimes(1);
    const [elements, wires] = setElementsAndWires.mock.calls[0] as [unknown[], unknown[]];
    expect(elements).toHaveLength(2);
    expect(wires).toHaveLength(1);
    // 重排后 fn 在前、位置有序（y 递增）
    const fnEl = (elements as Array<{ id: string; position: { y: number } }>).find((e) => e.id === "fn-1")!;
    const aEl = (elements as Array<{ id: string; position: { y: number } }>).find((e) => e.id === "a-2")!;
    expect(fnEl.position.y).toBeLessThan(aEl.position.y);
  });
  it("空画布布局按钮禁用", () => {
    setup({ elements: [], wires: [] });
    expect((screen.getByLabelText("自动整理布局") as HTMLButtonElement).disabled).toBe(true);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `npx vitest run src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx`
Expected: FAIL（无按钮）

- [ ] **Step 3: 实现接线**

`FlowCanvas.tsx` 修改：

**a) store 取 setElementsAndWires**（既有 select 区追加）：

```ts
  const setElementsAndWires = usePoiesisStore((s) => s.setElementsAndWires);
```

**b) 布局处理器**（在 handleNodesChange 附近）：

```ts
  // ── 自动整理布局：computeLayeredLayout 纯函数 → setElementsAndWires 批量写入（一次 undo 条目）──
  const handleAutoLayout = useCallback(() => {
    if (codeElements.length === 0) return;
    const sizes = new Map<string, { width: number; height: number }>();
    for (const el of codeElements) {
      const n = nodes.find((nd) => nd.id === el.id);
      if (n?.measured?.width && n?.measured?.height) {
        sizes.set(el.id, { width: n.measured.width, height: n.measured.height });
      }
    }
    const layout = computeLayeredLayout(codeElements, wires, sizes);
    const nextElements = elements.map((e) =>
      e.nodeType === "codeblock" && CODE_TAGS.has(e.tag) && layout[e.id]
        ? { ...e, position: { ...layout[e.id] } } as ElementData
        : e,
    );
    setElementsAndWires(nextElements, wires);
  }, [codeElements, elements, wires, nodes, setElementsAndWires]);
```

**c) 按钮渲染**（ReactFlow 同级、Controls 上方 absolute 定位，模式参照 Controls 的 bottom-right）：

```tsx
          <button
            type="button"
            onClick={handleAutoLayout}
            disabled={codeElements.length === 0}
            aria-label="自动整理布局"
            title="自动整理布局（自上而下分层）"
            style={{
              position: "absolute",
              right: 14,
              bottom: 86, // Controls（bottom-right）上方
              zIndex: 5,
              padding: "4px 8px",
              fontSize: 12,
              border: "1px solid var(--ol-border)",
              borderRadius: 6,
              background: "var(--ol-surface)",
              cursor: codeElements.length === 0 ? "not-allowed" : "pointer",
              opacity: codeElements.length === 0 ? 0.5 : 1,
            }}
          >
            ⤢ 整理布局
          </button>
```

（按钮放在 `{allNodes.length === 0 ? <EmptyState/> : <ReactFlow>…</ReactFlow>}` 结构的 ReactFlow 之后、`menuState` 之前，外层已是 `position: relative` 容器——若 flowWrapper 无 relative，给按钮外层包一个 `position: relative` div。）

**d) imports**：`computeLayeredLayout` from `../../services/codegen/flow-layout`；`ElementData` type（若未导入）。

- [ ] **Step 4: 运行确认通过 + 回归**

Run: `npx vitest run src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx src/components/canvas/code/__tests__/FlowCanvasDrag.test.tsx src/components/canvas/code/__tests__/FlowCanvasLetVars.test.tsx` + `npx tsc --noEmit`
Expected: PASS + tsc 零错误

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx
git commit -m "feat(codeflow): 自动整理布局按钮（computeLayeredLayout → setElementsAndWires 可 undo）"
```

---

### Task 12: E2E 真机验证轮（Victauri）

**Files:**
- 验证目标：布局按钮、loop 节点、Notion 式公式框全链路
- 记录: `.superpowers/sdd/2026-08-05-codeflow-maturity/`（progress + task-12-report）

**Interfaces:**
- Consumes: Task 1-11 全部交付；Poiesis 运行实例（start-poiesis）+ Victauri MCP

- [ ] **Step 1: 全量基线**

Run: `npx vitest run` + `npx tsc --noEmit`
Expected: 全绿 + tsc 零错误（记录测试总数，预计 480+）

- [ ] **Step 2: 启动应用**

Run: `bash /d/space/OmniLang/.claude/skills/start-poiesis/start-poiesis.sh`（若未运行）；`get_plugin_info` 确认 Victauri 已连接（app.identifier 匹配）

- [ ] **Step 3: 循环节点 E2E**

1. 进入 demo2 组件 codeflow（既有 TestCodeFlow 项目）
2. 右键菜单确认「循环 (while)」「迭代 (for-in)」两项存在
3. 添加 while 节点 → 填条件 `total < 10` → 从循环体端口拖出创建 assign `total = total + 1`
4. 添加 for-in 节点 → `item in items` → 循环体 `total = total + item`
5. flow-out 续接 return
6. 保存 → **真实进程重启**（kill + relaunch，非 location.reload——ADR-060 教训）→ 重进 codeflow：
   - while/for-in 节点形态保持（tag/props 逐字）
   - body wire（br0）与 flow-out 续接 wire 完整
7. `.omni` 磁盘产物核验：`while total < 10 {` / `for item in items {` 含 `@pid` 注释；文件 id 与画布元素逐字吻合
8. 编译产物过 WASM 验证（保存门禁无拦截）

- [ ] **Step 4: Notion 式公式框 E2E**

1. assign 节点 value 框：输入 `count + 1` → 观察 token 胶囊（identifier 紫 / operator 灰 / number 文本）
2. 敲 `<` → 直接成符号 + 弹候选（`<`/`<=`）→ ↑↓ Enter 选择 `<=` 替换
3. 敲 `+` → 成符号 + 弹操作数候选（变量列表）→ Enter 选 `count`
4. 输入 `T` → 候选 [total, TodoStore...] → Enter 插入
5. `TodoStore.` 点后 → member 候选（信号列表）→ 选中插入
6. **中文 IME**：输入法组合输入字符串字面量 `"你好"` → 组合期不跳动、组合结束成 string token
7. Backspace 在胶囊边界 → 整块删除（删整个变量胶囊而非单字符）
8. condition 条件/return 表达式/args 行均为同一 Notion 式行为
9. 节点预览（非编辑态）TokenPreview 胶囊渲染
10. 保存 → 重启 → 表达式字符串逐字保持（round-trip）

- [ ] **Step 5: 布局按钮 E2E**

1. 手动把节点拖乱 → 点「自动整理布局」
2. 断言：主干垂直、分支右展、同层对齐、无重叠；fn 在顶
3. undo（Ctrl+Z）→ 恢复乱布局；redo → 恢复整理后
4. 保存 → 真实进程重启 → 重进 codeflow → 布局保持（persistedPositions）
5. 多 fn 子图垂直堆叠；孤立节点底部

- [ ] **Step 6: 回归抽查**

- 既有 8 节点添加/编辑/连线/保存/重启回归
- 中链 condition、位置持久化、redo 全动作（Task 12 新功能不破坏）
- 空画布布局按钮禁用

- [ ] **Step 7: 缺陷修复轮（如有）+ 报告**

E2E 发现的缺陷按项目惯例：根因链记录 → 修复 → 复验 → 全量测试。报告写入 `task-12-report.md`（含磁盘产物核验证据），progress.md 收口。

- [ ] **Step 8: Commit（如有修复）**

```bash
git add -A poiesis/ .superpowers/sdd/2026-08-05-codeflow-maturity/
git commit -m "fix(codeflow): E2E 缺陷修复（如有）+ 验证报告"
```

---

## Self-Review 记录

**Spec 覆盖核对：**
- 方向一布局：Task 10（纯函数）+ Task 11（按钮/写入/undo）+ Task 12（E2E 重启保持）✓；「全部重排含手动拖过」→ Task 11 map 全量替换 ✓；「结果存 persistedPositions」→ setElementsAndWires 后保存侧既有收集逻辑 ✓（零改动）
- 方向二循环：Task 1（单一来源）→ 2（识别/props）→ 3（线形重建）→ 4（发射）→ 5（幂等）→ 6（UI）→ 12（E2E）✓；break/continue 范围外（code-block 手写）✓；语言合法性 → 门禁兜底 + E2E Step 3.8 ✓
- 方向三公式框：Task 7（tokenizer）→ 8（TokenPreview 预览）→ 9（内部替换）→ 12（E2E 含 IME）✓；props 契约不变 → 使用点零改动 ✓；运算符权威表 → OPERATOR_PREFIX_CANDIDATES 复用 ✓；「直接敲成符号+弹下拉」→ Task 9 onInput 候选上下文 + 测试用例 ✓
- 约束：零新依赖（全部手写）✓；`@pid` 契约零改动（loop 端口复用 br0，Task 3 声明）✓；用户定性（无交互打磨/无数据端口）✓

**占位符扫描：** 无 TBD/TODO；所有步骤含真实代码或可执行命令；Task 5 Step 3 的"若幂等天然成立无代码改动"为条件分支（两种结果都有明确动作），非占位。

**类型一致性核对：** `formatSyntaxNode(tag, props)` Task 1 定义 → Task 3/4 消费 ✓；`parseSyntaxProps` 签名扩展 Task 2 → Task 3 消费 ✓；`tokenizeFormula`/`FormulaToken` Task 7 → Task 8/9 消费（start/end 偏移一致）✓；`computeLayeredLayout(elements, wires, nodeSizes)` Task 10 → Task 11 消费（Map<string,{width,height}>）✓；`setElementsAndWires(elements, wires)` 既有签名 Task 11 直用 ✓；LoopNode data 契约（tag/props/source/scope/letVars/onEditProp）Task 6 → FlowCanvas 既有语法节点分支 ✓。

**已知风险（计划内缓解）：** Task 9 contenteditable 是最大工程——IME 组合渲染分支 + Selection API 恢复 + 既有测试更新面广；若 jsdom 对 contenteditable 的 `onInput` 模拟与真实行为偏差，Task 9 Step 4 以真实浏览器 E2E（Task 12 Step 4）为准修订断言。
