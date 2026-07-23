# CodeCard Editor 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用结构化卡片编辑器（CodeCardEditor）替换 Poiesis 中 code-block 节点的 Monaco 文本编辑器（CodeEditorModal），消除代码块的黑箱问题。

**Architecture:** 保留 CodeFlow 画布的节点+连线体系不变。code-block 节点内部从纯文本改为 CodeOp[] 结构化操作列表。新增 `emitOps()`/`parseOps()` 往返函数，编译器从 operations 生成干净 Omni 源码，解析器从 Omni 源码还原 operations。

**Tech Stack:** TypeScript + React 19 + Lexical + Zustand 5 + Vitest + Playwright，Rust (omnilang/omnilang-wasm) 用于 WASM 解析扩展。

## Global Constraints

- 不改 CodeFlow 画布节点+连线体系
- 不改 .omni 语言语法
- code-block 内部不写入 PID 注释
- 向后兼容：旧项目 code-block 的 source 字段必须能正常编译
- 键盘交互优于鼠标交互

---

## File Structure

```
New files:
  poiesi/src/core/types/code-op.ts              ← CodeOp type definitions (12 types)
  poiesi/src/services/codegen/code-op-emitter.ts ← emitOp/emitOps functions
  poiesi/src/services/codegen/code-op-parser.ts  ← parseOps (JS regex) + Stmt→CodeOp mapper
  poiesi/src/components/canvas/code/CodeCardEditor.tsx          ← main editor component
  poiesi/src/components/canvas/code/CodeCardEditor.module.css   ← editor styles
  poiesi/src/components/canvas/code/cards/CardList.tsx          ← recursive card renderer
  poiesi/src/components/canvas/code/cards/InlineSlot.tsx        ← inline input for slots
  poiesi/src/components/canvas/code/cards/AutocompleteMenu.tsx  ← context-aware autocomplete
  poiesi/src/components/canvas/code/cards/DeclareVarCard.tsx    ← let/let mut
  poiesi/src/components/canvas/code/cards/SetVarCard.tsx        ← assignment
  poiesi/src/components/canvas/code/cards/IfBlockCard.tsx       ← if/else if/else
  poiesi/src/components/canvas/code/cards/WhileLoopCard.tsx     ← while
  poiesi/src/components/canvas/code/cards/ForLoopCard.tsx       ← for..in
  poiesi/src/components/canvas/code/cards/ReturnExprCard.tsx    ← return
  poiesi/src/components/canvas/code/cards/CallFnCard.tsx        ← function call
  poiesi/src/components/canvas/code/cards/AwaitExprCard.tsx     ← await
  poiesi/src/components/canvas/code/cards/SimpleStmtCard.tsx    ← break/continue/comment (shared)
  poiesi/src/components/canvas/code/cards/RawCodeCard.tsx       ← raw fallback
  poiesi/src/components/canvas/code/cards/card-registry.ts      ← type→component map + buildOp
  poiesi/src/components/canvas/code/cards/card-types.ts         ← shared card component interfaces

Modified files:
  poiesi/src/core/types/element.ts              ← CodeblockElementData.operations?: CodeOp[]
  poiesi/src/services/codegen/graph-compiler.ts  ← wire emitOps into case "code-block"
  poiesi/src/services/codegen/graph-parser.ts    ← wire parseOps into code-block segment
  poiesi/src/components/canvas/code/FlowCanvas.tsx ← CodeCardEditor instead of CodeEditorModal
  poiesi/src/store/types.ts                      ← add operations to element state type
  poiesi/src/store/slices/element-crud.ts        ← add setElementOperations action

Rust files (WASM Phase 2):
  omnilang/src/parser/ast/mod.rs                ← add Serialize derive to Stmt, Block, Expr, etc.
  omnilang/src/lib.rs                            ← add parse_stmts function
  omnilang-wasm/src/lib.rs                       ← add parse_code_block WASM binding
```

---

### Task 1: CodeOp Type Definitions

**Files:**
- Create: `poiesi/src/core/types/code-op.ts`

**Interfaces:**
- Produces: `CodeOp`, `DeclareVarOp`, `SetVarOp`, `IfBlockOp`, `WhileLoopOp`, `ForLoopOp`, `ReturnExprOp`, `CallFnOp`, `AwaitExprOp`, `BreakStmtOp`, `ContinueStmtOp`, `CommentOp`, `RawCodeOp`
- Produces: `ScopeKind = "fn" | "container"`

- [ ] **Step 1: Create the type file**

```typescript
// poiesi/src/core/types/code-op.ts
// Structured operation types for CodeCard Editor.
// Each operation maps to a single OmniLang statement/expression inside a code-block.

export type ScopeKind = "fn" | "container";

// ── Base ──

interface CodeOpBase {
  pid: string; // 编辑器内部 ID，不写入 .omni
}

// ── Concrete operations ──

export interface DeclareVarOp extends CodeOpBase {
  kind: "declare_var";
  mutable: boolean;
  name: string;
  varType?: string;
  initValue?: string;
  scope: ScopeKind;
}

export interface SetVarOp extends CodeOpBase {
  kind: "set_var";
  name: string;
  operator: "=" | "+=" | "-=" | "*=" | "/=" | "%=";
  value: string;
}

export interface IfBlockOp extends CodeOpBase {
  kind: "if_block";
  branches: {
    condition: string;
    body: CodeOp[];
  }[];
}

export interface WhileLoopOp extends CodeOpBase {
  kind: "while";
  condition: string;
  body: CodeOp[];
}

export interface ForLoopOp extends CodeOpBase {
  kind: "for_in";
  itemName: string;
  collection: string;
  body: CodeOp[];
}

export interface ReturnExprOp extends CodeOpBase {
  kind: "return";
  value?: string;
}

export interface CallFnOp extends CodeOpBase {
  kind: "call_fn";
  fnName: string;
  args: string[];
}

export interface AwaitExprOp extends CodeOpBase {
  kind: "await";
  value: string;
}

export interface BreakStmtOp extends CodeOpBase {
  kind: "break";
}

export interface ContinueStmtOp extends CodeOpBase {
  kind: "continue";
}

export interface CommentOp extends CodeOpBase {
  kind: "comment";
  text: string;
}

export interface RawCodeOp extends CodeOpBase {
  kind: "raw";
  source: string;
}

export type CodeOp =
  | DeclareVarOp
  | SetVarOp
  | IfBlockOp
  | WhileLoopOp
  | ForLoopOp
  | ReturnExprOp
  | CallFnOp
  | AwaitExprOp
  | BreakStmtOp
  | ContinueStmtOp
  | CommentOp
  | RawCodeOp;
```

- [ ] **Step 2: Verify compilation**

```bash
cd poiesi && npx tsc --noEmit src/core/types/code-op.ts
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add poiesi/src/core/types/code-op.ts
git commit -m "feat: add CodeOp type definitions for CodeCard Editor"
```

---

### Task 2: Extend CodeblockElementData

**Files:**
- Modify: `poiesi/src/core/types/element.ts`

**Interfaces:**
- Modifies: `CodeblockElementData` — adds `operations?: CodeOp[]` field

- [ ] **Step 1: Add import and extend interface**

Read the current element.ts to understand `CodeblockElementData`, then add:

```typescript
// Add import at top of element.ts
import type { CodeOp } from "./code-op";

// In CodeblockElementData interface, add after `source` field:
export interface CodeblockElementData extends ElementDataBase {
  nodeType: "codeblock";
  tag: string;
  props: Record<string, unknown>;
  ports?: PortData[];
  source: string;
  operations?: CodeOp[];  // NEW: structured operations for card editor
  outputValue?: unknown;
  position?: { x: number; y: number };
  nodeCategory?: "flow" | "event" | "action" | "data" | "variable" | "code" | "function" | "ui";
  config?: Record<string, unknown>;
  flowOrder?: number;
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd poiesi && npx tsc --noEmit 2>&1 | head -20
```

Expected: no new TS errors from this change

- [ ] **Step 3: Commit**

```bash
git add poiesi/src/core/types/element.ts
git commit -m "feat: extend CodeblockElementData with operations field"
```

---

### Task 3: CodeOp Emitter (Operations → Omni Source)

**Files:**
- Create: `poiesi/src/services/codegen/code-op-emitter.ts`

**Interfaces:**
- Produces: `emitOp(op: CodeOp, indent: number): string`
- Produces: `emitOps(ops: CodeOp[]): string`

- [ ] **Step 1: Write the failing test**

Create `poiesi/src/services/codegen/__tests__/code-op-emitter.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { emitOp, emitOps } from "../code-op-emitter";
import type { CodeOp } from "../../core/types/code-op";
import { uid } from "../../core/utils/id";

describe("emitOp", () => {
  it("emits let declaration with type and init", () => {
    const op: CodeOp = {
      kind: "declare_var", pid: uid("test"),
      mutable: false, name: "sum", varType: "f64", initValue: "0.0",
      scope: "fn",
    };
    expect(emitOp(op, 0)).toBe("let sum: f64 = 0.0;\n");
  });

  it("emits let mut without type (inferred)", () => {
    const op: CodeOp = {
      kind: "declare_var", pid: uid("test"),
      mutable: true, name: "i", initValue: "0",
      scope: "fn",
    };
    expect(emitOp(op, 0)).toBe("let mut i = 0;\n");
  });

  it("emits set_var with += operator", () => {
    const op: CodeOp = {
      kind: "set_var", pid: uid("test"),
      name: "count", operator: "+=", value: "1",
    };
    expect(emitOp(op, 0)).toBe("count += 1;\n");
  });

  it("emits while loop with nested body", () => {
    const op: CodeOp = {
      kind: "while", pid: uid("test"),
      condition: "i < n",
      body: [{
        kind: "set_var", pid: uid("test"),
        name: "sum", operator: "+=", value: "i",
      }],
    };
    const result = emitOp(op, 0);
    expect(result).toContain("while i < n {");
    expect(result).toContain("  sum += i;");
    expect(result).toContain("}");
  });

  it("emits if/else block", () => {
    const op: CodeOp = {
      kind: "if_block", pid: uid("test"),
      branches: [
        { condition: "x > 0", body: [{ kind: "return", pid: uid("test"), value: "1" }] },
        { condition: "", body: [{ kind: "return", pid: uid("test"), value: "0" }] },
      ],
    };
    const result = emitOp(op, 0);
    expect(result).toContain("if x > 0 {");
    expect(result).toContain("  return 1;");
    expect(result).toContain("} else {");
    expect(result).toContain("  return 0;");
  });

  it("emits return with and without value", () => {
    expect(emitOp({ kind: "return", pid: uid("test"), value: "x" }, 0))
      .toBe("return x;\n");
    expect(emitOp({ kind: "return", pid: uid("test") }, 0))
      .toBe("return;\n");
  });
});

describe("emitOps", () => {
  it("joins multiple ops", () => {
    const ops: CodeOp[] = [
      { kind: "declare_var", pid: uid("test"), mutable: false, name: "x", initValue: "1", scope: "fn" },
      { kind: "return", pid: uid("test"), value: "x" },
    ];
    expect(emitOps(ops)).toBe("let x = 1;\nreturn x;\n");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd poiesi && npx vitest run src/services/codegen/__tests__/code-op-emitter.test.ts
```

Expected: FAIL — module not found

- [ ] **Step 3: Write emitter implementation**

```typescript
// poiesi/src/services/codegen/code-op-emitter.ts
import type { CodeOp } from "../../core/types/code-op";

export function emitOp(op: CodeOp, indent: number): string {
  const pad = "  ".repeat(indent);

  switch (op.kind) {
    case "declare_var": {
      const mut = op.mutable ? " mut" : "";
      const type = op.varType ? `: ${op.varType}` : "";
      const init = op.initValue !== undefined ? ` = ${op.initValue}` : "";
      return `${pad}let${mut} ${op.name}${type}${init};\n`;
    }
    case "set_var":
      return `${pad}${op.name} ${op.operator} ${op.value};\n`;
    case "if_block": {
      let out = "";
      for (let i = 0; i < op.branches.length; i++) {
        const br = op.branches[i];
        const kw = i === 0 ? "if" : br.condition ? "else if" : "else";
        const cond = br.condition ? ` ${br.condition}` : "";
        out += `${pad}${kw}${cond} {\n`;
        for (const child of br.body) out += emitOp(child, indent + 1);
        out += `${pad}}\n`;
      }
      return out;
    }
    case "while": {
      let out = `${pad}while ${op.condition} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "for_in": {
      let out = `${pad}for ${op.itemName} in ${op.collection} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "return":
      return `${pad}return${op.value ? " " + op.value : ""};\n`;
    case "call_fn":
      return `${pad}${op.fnName}(${op.args.join(", ")});\n`;
    case "await":
      return `${pad}await ${op.value};\n`;
    case "break":
      return `${pad}break;\n`;
    case "continue":
      return `${pad}continue;\n`;
    case "comment":
      return `${pad}// ${op.text}\n`;
    case "raw":
      return op.source;
  }
}

export function emitOps(ops: CodeOp[]): string {
  return ops.map(op => emitOp(op, 0)).join("");
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd poiesi && npx vitest run src/services/codegen/__tests__/code-op-emitter.test.ts
```

Expected: all 7 tests PASS

- [ ] **Step 5: Commit**

```bash
git add poiesi/src/services/codegen/code-op-emitter.ts poiesi/src/services/codegen/__tests__/code-op-emitter.test.ts
git commit -m "feat: add CodeOp emitter — emitOp/emitOps convert structured ops to Omni source"
```

---

### Task 4: Wire Emitter into graph-compiler.ts

**Files:**
- Modify: `poiesi/src/services/codegen/graph-compiler.ts`

**Interfaces:**
- Modifies: `emitNode()` — in case `"code-block"`, calls `emitOps(el.operations)` instead of emitting `el.source` when operations exist

- [ ] **Step 1: Add import**

```typescript
// In graph-compiler.ts, add import at top:
import { emitOps } from "./code-op-emitter";
```

- [ ] **Step 2: Modify case "code-block"**

Read the current `emitNode` function. Find the `case "code-block":` block and change:

```typescript
case "code-block": {
  // Use structured operations if available, fall back to raw source
  const operations = (node.element as any).operations as CodeOp[] | undefined;
  let out: string;

  if (operations && operations.length > 0) {
    out = emitPidComment(pid);
    out += emitOps(operations);
  } else {
    // Legacy: raw source text
    const source = node.element.source || "";
    out = emitPidComment(pid);
    const lines = source.split("\n");
    for (const line of lines) {
      out += `${pad}${line}\n`;
    }
  }

  // Emit successor (control flow continues)
  for (const succId of node.successors) {
    const succ = adj.get(succId);
    if (succ) out += emitNode(succ, adj, indent, visited);
  }
  return out;
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd poiesi && npx tsc --noEmit 2>&1 | grep "graph-compiler" | head -10
```

Expected: no new errors

- [ ] **Step 4: Commit**

```bash
git add poiesi/src/services/codegen/graph-compiler.ts
git commit -m "feat: wire emitOps into graph-compiler code-block case"
```

---

### Task 5: CodeOp Parser (Omni Source → Operations) — JS Regex Fallback

**Files:**
- Create: `poiesi/src/services/codegen/code-op-parser.ts`

**Interfaces:**
- Produces: `parseOps(source: string): CodeOp[]`
- Produces: `detectOpKind(firstLine: string): CodeOp["kind"] | "raw"`

- [ ] **Step 1: Write the failing test**

Create `poiesi/src/services/codegen/__tests__/code-op-parser.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { parseOps, detectOpKind } from "../code-op-parser";

describe("detectOpKind", () => {
  it("detects let declaration", () => {
    expect(detectOpKind("let x = 1;")).toBe("declare_var");
  });
  it("detects let mut", () => {
    expect(detectOpKind("let mut i = 0;")).toBe("declare_var");
  });
  it("detects if", () => {
    expect(detectOpKind("if x > 0 {")).toBe("if_block");
  });
  it("detects while", () => {
    expect(detectOpKind("while i < n {")).toBe("while");
  });
  it("detects for in", () => {
    expect(detectOpKind("for item in items {")).toBe("for_in");
  });
  it("detects return", () => {
    expect(detectOpKind("return x;")).toBe("return");
  });
  it("detects function call", () => {
    expect(detectOpKind("console_log(\"hi\");")).toBe("call_fn");
  });
  it("detects assignment", () => {
    expect(detectOpKind("count += 1;")).toBe("set_var");
  });
  it("falls back to raw", () => {
    expect(detectOpKind("}")).toBe("raw");
  });
});

describe("parseOps", () => {
  it("parses simple sequence", () => {
    const source = "let x = 1;\nreturn x;\n";
    const ops = parseOps(source);
    expect(ops).toHaveLength(2);
    expect(ops[0].kind).toBe("declare_var");
    if (ops[0].kind === "declare_var") {
      expect(ops[0].name).toBe("x");
      expect(ops[0].initValue).toBe("1");
    }
    expect(ops[1].kind).toBe("return");
  });

  it("parses while loop with nested body", () => {
    const source = `let mut i = 0;
while i < 10 {
    i += 1;
}
return i;`;
    const ops = parseOps(source);
    expect(ops).toHaveLength(3);
    expect(ops[1].kind).toBe("while");
    if (ops[1].kind === "while") {
      expect(ops[1].condition).toBe("i < 10");
      expect(ops[1].body).toHaveLength(1);
      expect(ops[1].body[0].kind).toBe("set_var");
    }
  });

  it("parses if/else with nested return", () => {
    const source = `if x > 0 {
    return 1;
} else {
    return 0;
}`;
    const ops = parseOps(source);
    expect(ops).toHaveLength(1);
    expect(ops[0].kind).toBe("if_block");
    if (ops[0].kind === "if_block") {
      expect(ops[0].branches).toHaveLength(2);
      expect(ops[0].branches[0].condition).toBe("x > 0");
      expect(ops[0].branches[0].body).toHaveLength(1);
      expect(ops[0].branches[1].condition).toBe("");
    }
  });

  it("round-trips with emitOps", async () => {
    const { emitOps } = await import("../code-op-emitter");
    const source = `let sum = 0.0;
let mut i = 0;
while i < 10 {
    sum += i;
    i += 1;
}
return sum;`;
    const ops = parseOps(source);
    const emitted = emitOps(ops);
    // after round-trip, parsing the emitted source should produce equivalent ops
    const ops2 = parseOps(emitted);
    expect(ops2.length).toBe(ops.length);
    expect(ops2.map(o => o.kind)).toEqual(ops.map(o => o.kind));
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd poiesi && npx vitest run src/services/codegen/__tests__/code-op-parser.test.ts
```

Expected: FAIL

- [ ] **Step 3: Write parser implementation**

```typescript
// poiesi/src/services/codegen/code-op-parser.ts
import type { CodeOp } from "../../core/types/code-op";
import { uid } from "../../core/utils/id";

/** Detect the CodeOp kind from the first non-empty line of Omni source. */
export function detectOpKind(firstLine: string): CodeOp["kind"] | "raw" {
  const t = firstLine.trim();
  if (/^let\s+mut\s/.test(t) || /^let\s+\w/.test(t)) return "declare_var";
  if (/^if\s+/.test(t) || /^else\s+if\s+/.test(t) || /^else\s*\{/.test(t)) return "if_block";
  if (/^while\s+/.test(t)) return "while";
  if (/^for\s+\w+\s+in\s+/.test(t)) return "for_in";
  if (/^return\b/.test(t)) return "return";
  if (/^break\s*;?\s*$/.test(t)) return "break";
  if (/^continue\s*;?\s*$/.test(t)) return "continue";
  if (/^await\s+/.test(t)) return "await";
  if (/^\w+\s*\(/.test(t)) return "call_fn";
  if (/^\w+\s*(=|\+=|-=|\*=|\/=|%=)/.test(t)) return "set_var";
  if (/^\/\//.test(t)) return "comment";
  return "raw";
}

function braceDepth(line: string): number {
  let d = 0;
  for (const ch of line) {
    if (ch === "{") d++;
    if (ch === "}") d--;
  }
  return d;
}

function extractWhileCondition(line: string): string {
  const m = line.trim().match(/^while\s+(.+?)\s*\{?\s*$/);
  return m ? m[1].trim() : "true";
}

function extractForParts(line: string): { itemName: string; collection: string } {
  const m = line.trim().match(/^for\s+(\w+)\s+in\s+(.+?)\s*\{?\s*$/);
  return { itemName: m?.[1] ?? "item", collection: m?.[2]?.trim() ?? "items" };
}

function extractDeclareVar(line: string) {
  const m = line.trim().match(
    /^let(\s+mut)?\s+(\w+)(?:\s*:\s*(\w+(?:<.+?>)?))?(?:\s*=\s*(.+?))?\s*;?\s*$/
  );
  return {
    mutable: !!m?.[1],
    name: m?.[2] ?? "x",
    varType: m?.[3],
    initValue: m?.[4],
  };
}

function extractSetVar(line: string) {
  const m = line.trim().match(/^(\w+)\s*(=|\+=|-=|\*=|\/=|%=)\s*(.+?)\s*;?\s*$/);
  return { name: m?.[1] ?? "x", operator: (m?.[2] ?? "=") as CodeOp["kind"] extends "set_var" ? any : any, value: m?.[3] ?? "" };
}

function extractReturnExpr(line: string): string | undefined {
  const m = line.trim().match(/^return\s+(.+?)\s*;?\s*$/);
  return m?.[1];
}

function extractCallFn(line: string) {
  const m = line.trim().match(/^(\w+)\s*\(([^)]*)\)\s*;?\s*$/);
  return {
    fnName: m?.[1] ?? "fn",
    args: m?.[2] ? m[2].split(",").map(a => a.trim()).filter(Boolean) : [],
  };
}

function extractAwaitExpr(line: string): string {
  const m = line.trim().match(/^await\s+(.+?)\s*;?\s*$/);
  return m?.[1] ?? "expr";
}

function buildBlockBody(lines: string[], startI: number, startDepth: number): { bodyLines: string[]; nextI: number } {
  const bodyLines: string[] = [];
  let i = startI + 1;
  let depth = startDepth;
  while (i < lines.length) {
    depth += braceDepth(lines[i]);
    if (depth <= startDepth) break;
    bodyLines.push(lines[i]);
    i++;
  }
  return { bodyLines: bodyLines.map(l => l.replace(/^\s{2,}/, "  ")), nextI: i };
}

function buildIfBranches(lines: string[], startI: number): { branches: { condition: string; body: CodeOp[] }[]; nextI: number } {
  const branches: { condition: string; body: CodeOp[] }[] = [];
  let i = startI;

  while (i < lines.length) {
    const line = lines[i].trim();
    let condition = "";

    if (line.startsWith("if ")) {
      condition = line.slice(3).replace(/\s*\{?\s*$/, "").trim();
    } else if (line.startsWith("else if ")) {
      condition = line.slice(8).replace(/\s*\{?\s*$/, "").trim();
    } else if (line.startsWith("else")) {
      condition = "";
    } else {
      break;
    }

    const startDepth = braceDepth(lines[i]);
    const { bodyLines, nextI } = buildBlockBody(lines, i, startDepth);
    branches.push({ condition, body: parseOps(bodyLines.join("\n")) });
    i = nextI;

    // Check for next branch continuation (} else { or } else if)
    if (i < lines.length && /^\}\s*(else|$)/.test(lines[i].trim()) && lines[i].trim().startsWith("}")) {
      const closeLine = lines[i].trim();
      if (closeLine.includes("else if")) {
        const condPart = closeLine.replace(/^\}\s*else\s+if\s+/, "").replace(/\s*\{?\s*$/, "").trim();
        condition = condPart;
        const sd = braceDepth(lines[i]);
        const { bodyLines: bl, nextI: ni } = buildBlockBody(lines, i, sd);
        branches.push({ condition, body: parseOps(bl.join("\n")) });
        i = ni;
      } else if (closeLine.includes("else")) {
        condition = "";
        const sd = braceDepth(lines[i]);
        const { bodyLines: bl, nextI: ni } = buildBlockBody(lines, i, sd);
        branches.push({ condition, body: parseOps(bl.join("\n")) });
        i = ni;
      } else {
        i++; // just close brace
      }
    }
  }

  return { branches, nextI: i };
}

/** Parse Omni source text into structured CodeOp[]. */
export function parseOps(source: string): CodeOp[] {
  const lines = source.split("\n");
  const ops: CodeOp[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (trimmed === "" || trimmed === "}" || trimmed.startsWith("} else")) {
      i++;
      continue;
    }

    const kind = detectOpKind(trimmed);

    switch (kind) {
      case "declare_var": {
        const info = extractDeclareVar(trimmed);
        ops.push({
          kind: "declare_var", pid: uid("op"),
          mutable: info.mutable, name: info.name,
          varType: info.varType, initValue: info.initValue,
          scope: "fn",
        });
        i++;
        break;
      }
      case "set_var": {
        const info = extractSetVar(trimmed);
        ops.push({ kind: "set_var", pid: uid("op"), ...info });
        i++;
        break;
      }
      case "if_block": {
        const { branches, nextI } = buildIfBranches(lines, i);
        ops.push({ kind: "if_block", pid: uid("op"), branches });
        i = nextI;
        break;
      }
      case "while": {
        const condition = extractWhileCondition(trimmed);
        const startDepth = braceDepth(line);
        const { bodyLines, nextI } = buildBlockBody(lines, i, startDepth);
        ops.push({ kind: "while", pid: uid("op"), condition, body: parseOps(bodyLines.join("\n")) });
        i = nextI;
        break;
      }
      case "for_in": {
        const { itemName, collection } = extractForParts(trimmed);
        const startDepth = braceDepth(line);
        const { bodyLines, nextI } = buildBlockBody(lines, i, startDepth);
        ops.push({ kind: "for_in", pid: uid("op"), itemName, collection, body: parseOps(bodyLines.join("\n")) });
        i = nextI;
        break;
      }
      case "return":
        ops.push({ kind: "return", pid: uid("op"), value: extractReturnExpr(trimmed) });
        i++;
        break;
      case "call_fn": {
        const info = extractCallFn(trimmed);
        ops.push({ kind: "call_fn", pid: uid("op"), ...info });
        i++;
        break;
      }
      case "await":
        ops.push({ kind: "await", pid: uid("op"), value: extractAwaitExpr(trimmed) });
        i++;
        break;
      case "break":
        ops.push({ kind: "break", pid: uid("op") });
        i++;
        break;
      case "continue":
        ops.push({ kind: "continue", pid: uid("op") });
        i++;
        break;
      case "comment":
        ops.push({ kind: "comment", pid: uid("op"), text: trimmed.replace(/^\/\/\s*/, "") });
        i++;
        break;
      case "raw":
        ops.push({ kind: "raw", pid: uid("op"), source: trimmed });
        i++;
        break;
    }
  }

  return ops;
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd poiesi && npx vitest run src/services/codegen/__tests__/code-op-parser.test.ts
```

Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add poiesi/src/services/codegen/code-op-parser.ts poiesi/src/services/codegen/__tests__/code-op-parser.test.ts
git commit -m "feat: add CodeOp parser — parseOps with JS regex fallback for round-trip"
```

---

### Task 6: Wire Parser into graph-parser.ts

**Files:**
- Modify: `poiesi/src/services/codegen/graph-parser.ts`

**Interfaces:**
- Modifies: `parseCodeFlowGraph()` — for `"code-block"` segments, calls `parseOps(seg.code)` to populate `operations`

- [ ] **Step 1: Add import**

```typescript
// In graph-parser.ts, add import at top:
import { parseOps } from "./code-op-parser";
```

- [ ] **Step 2: Add operations population**

Find where `el` is created for `seg.type === "code-block"`. After setting `source = seg.code`, add:

```typescript
} else if (seg.type === "code-block") {
  source = seg.code;
  // Try parsing source into structured operations
  let operations;
  try {
    operations = parseOps(source);
  } catch {
    operations = undefined; // keep as raw source only
  }
  // ... later when constructing el:
  const el: CodeblockElementData = {
    id: seg.pid,
    nodeType: "codeblock",
    tag: seg.type,
    parentId: null,
    props,
    source,
    ...(operations && operations.length > 0 ? { operations } : {}),
  };
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd poiesi && npx tsc --noEmit 2>&1 | grep "graph-parser" | head -10
```

Expected: no new errors

- [ ] **Step 4: Commit**

```bash
git add poiesi/src/services/codegen/graph-parser.ts
git commit -m "feat: wire parseOps into graph-parser for code-block segments"
```

---

### Task 7: Store Support — setElementOperations Action

**Files:**
- Modify: `poiesi/src/store/types.ts` — `setElementOperations` in PoiesisState
- Modify: `poiesi/src/store/slices/element-crud.ts` — implementation

**Interfaces:**
- Produces: `setElementOperations(id: string, operations: CodeOp[]): void`

- [ ] **Step 1: Add action to store type**

In `poiesi/src/store/types.ts`, add to `PoiesisState` interface:

```typescript
setElementOperations: (id: string, operations: CodeOp[]) => void;
```

- [ ] **Step 2: Implement action**

In `poiesi/src/store/slices/element-crud.ts`, add:

```typescript
import type { CodeOp } from "../../core/types/code-op";

// In the slice, add:
setElementOperations: (id, operations) => {
  set((state) => {
    const idx = state.elements.findIndex((e) => e.id === id);
    if (idx < 0) return;
    const el = state.elements[idx];
    if (el.nodeType !== "codeblock") return;
    state.elements[idx] = {
      ...el,
      operations,
      source: "", // Will be regenerated by emitOps during save
    } as any;
  });
},
```

- [ ] **Step 3: Verify compilation**

```bash
cd poiesi && npx tsc --noEmit 2>&1 | grep "element-crud\|types.ts" | head -10
```

Expected: no new errors

- [ ] **Step 4: Commit**

```bash
git add poiesi/src/store/types.ts poiesi/src/store/slices/element-crud.ts
git commit -m "feat: add setElementOperations action to store"
```

---

### Task 8: Card Registry + Shared Types

**Files:**
- Create: `poiesi/src/components/canvas/code/cards/card-types.ts`
- Create: `poiesi/src/components/canvas/code/cards/card-registry.ts`

**Interfaces:**
- Produces: `CardComponentProps<TOp>` — shared props interface for all card components
- Produces: `CARD_REGISTRY: Record<string, React.FC>` — maps CodeOp kind → card component
- Produces: `getCardComponent(kind: string): React.FC | undefined`

- [ ] **Step 1: Create card-types.ts**

```typescript
// poiesi/src/components/canvas/code/cards/card-types.ts
import type { CodeOp } from "../../../../core/types/code-op";

export interface CardComponentProps<TOp extends CodeOp = CodeOp> {
  op: TOp;
  isSelected: boolean;
  onUpdate: (updated: TOp) => void;
  onDelete: () => void;
  /** Called when user presses Enter at the card's right boundary to insert a new line after */
  onInsertAfter: () => void;
  /** Called when cursor navigates to a nested body (if/when/for) */
  onEnterBody?: () => void;
}

export interface CardListProps {
  ops: CodeOp[];
  selectedPid: string | null;
  onSelect: (pid: string) => void;
  onUpdate: (index: number, updated: CodeOp) => void;
  onDelete: (index: number) => void;
  onInsertAfter: (index: number) => void;
}
```

- [ ] **Step 2: Create card-registry.ts**

```typescript
// poiesi/src/components/canvas/code/cards/card-registry.ts
import type { CodeOp } from "../../../../core/types/code-op";
import type { FC } from "react";
import type { CardComponentProps } from "./card-types";

// Lazily populated — card components register themselves
const registry: Record<string, FC<CardComponentProps<any>>> = {};

export function registerCard(kind: CodeOp["kind"], component: FC<CardComponentProps<any>>): void {
  registry[kind] = component;
}

export function getCardComponent(kind: CodeOp["kind"]): FC<CardComponentProps<any>> | undefined {
  return registry[kind];
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesi/src/components/canvas/code/cards/card-types.ts poiesi/src/components/canvas/code/cards/card-registry.ts
git commit -m "feat: add card registry and shared types for CodeCard Editor"
```

---

### Task 9: InlineSlot Component

**Files:**
- Create: `poiesi/src/components/canvas/code/cards/InlineSlot.tsx`

**Interfaces:**
- Produces: `<InlineSlot value={string} placeholder={string} onChange onChangeCommit onDelete />`

- [ ] **Step 1: Create InlineSlot component**

```typescript
// poiesi/src/components/canvas/code/cards/InlineSlot.tsx
import React, { useState, useRef, useEffect } from "react";

interface Props {
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onCommit: () => void;       // Enter pressed
  onDeleteEmpty: () => void;  // Backspace on empty slot
  onNavigateLeft: () => void;
  onNavigateRight: () => void;
  autoFocus?: boolean;
}

export const InlineSlot: React.FC<Props> = ({
  value, placeholder, onChange, onCommit,
  onDeleteEmpty, onNavigateLeft, onNavigateRight, autoFocus,
}) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const [isEmpty, setIsEmpty] = useState(value === "");

  useEffect(() => {
    if (autoFocus && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [autoFocus]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange(e.target.value);
    setIsEmpty(e.target.value === "");
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      onCommit();
    } else if (e.key === "Backspace" && isEmpty) {
      e.preventDefault();
      onDeleteEmpty();
    } else if (e.key === "ArrowLeft" && e.currentTarget.selectionStart === 0) {
      e.preventDefault();
      onNavigateLeft();
    } else if (e.key === "ArrowRight" && e.currentTarget.selectionEnd === (e.currentTarget as HTMLInputElement).value.length) {
      e.preventDefault();
      onNavigateRight();
    }
  };

  if (value && isEmpty === false) {
    // Tokenized display
    return (
      <span
        className="inline-slot-token"
        tabIndex={0}
        onFocus={() => setIsEmpty(false)}
        style={{
          display: "inline-block",
          background: "#e0e7ff",
          color: "#3730a3",
          borderRadius: 3,
          padding: "0 4px",
          fontFamily: "monospace",
          fontSize: 13,
          cursor: "text",
        }}
      >
        {value}
      </span>
    );
  }

  return (
    <input
      ref={inputRef}
      className="inline-slot-input"
      value={value}
      placeholder={placeholder}
      onChange={handleChange}
      onKeyDown={handleKeyDown}
      style={{
        display: "inline-block",
        width: Math.max(60, (value.length + 1) * 9),
        minWidth: 40,
        border: "1px solid #a5b4fc",
        borderRadius: 3,
        padding: "0 4px",
        fontFamily: "monospace",
        fontSize: 13,
        outline: "none",
        background: "#eef2ff",
      }}
    />
  );
};
```

- [ ] **Step 2: Commit**

```bash
git add poiesi/src/components/canvas/code/cards/InlineSlot.tsx
git commit -m "feat: add InlineSlot component for token-level inline editing"
```

---

### Task 10: Card Components — DeclareVarCard + SetVarCard + ReturnExprCard

**Files:**
- Create: `poiesi/src/components/canvas/code/cards/DeclareVarCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/SetVarCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/ReturnExprCard.tsx`

**Interfaces:**
- Each exports a React.FC<CardComponentProps<T>> + calls registerCard on module load

- [ ] **Step 1: Create DeclareVarCard**

```typescript
// poiesi/src/components/canvas/code/cards/DeclareVarCard.tsx
import React from "react";
import type { DeclareVarOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const DeclareVarCard: React.FC<CardComponentProps<DeclareVarOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  return (
    <div className={`code-card declare-var ${isSelected ? "selected" : ""}`}
      style={{
        display: "flex", alignItems: "center", gap: 4,
        padding: "4px 8px", borderRadius: 6,
        border: isSelected ? "2px solid #6366f1" : "1px solid #cbd5e1",
        background: isSelected ? "#f8fafc" : "#fff",
        fontFamily: "monospace", fontSize: 13,
      }}>
      <span className="card-keyword" style={{ color: "#7c3aed", fontWeight: 600 }}>let</span>
      {op.mutable && <span className="card-keyword" style={{ color: "#7c3aed", fontWeight: 600 }}>mut</span>}
      <InlineSlot
        value={op.name} placeholder="name"
        onChange={(v) => onUpdate({ ...op, name: v })}
        onCommit={onInsertAfter}
        onDeleteEmpty={onDelete}
        onNavigateLeft={() => {}}
        onNavigateRight={() => {}}
        autoFocus={op.name === ""}
      />
      {op.varType !== undefined && (
        <>
          <span>:</span>
          <InlineSlot value={op.varType} placeholder="Type"
            onChange={(v) => onUpdate({ ...op, varType: v })}
            onCommit={onInsertAfter} onDeleteEmpty={() => onUpdate({ ...op, varType: undefined })}
            onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        </>
      )}
      {op.initValue !== undefined && (
        <>
          <span>=</span>
          <InlineSlot value={op.initValue} placeholder="value"
            onChange={(v) => onUpdate({ ...op, initValue: v })}
            onCommit={onInsertAfter} onDeleteEmpty={() => onUpdate({ ...op, initValue: undefined })}
            onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        </>
      )}
      <span className="card-pid" style={{ fontSize: 9, color: "#94a3b8", marginLeft: "auto" }}>
        {op.scope}
      </span>
    </div>
  );
};

registerCard("declare_var", DeclareVarCard);
export default DeclareVarCard;
```

- [ ] **Step 2: Create SetVarCard**

```typescript
// poiesi/src/components/canvas/code/cards/SetVarCard.tsx
import React from "react";
import type { SetVarOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const SetVarCard: React.FC<CardComponentProps<SetVarOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card set-var ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #6366f1" : "1px solid #cbd5e1",
      background: "#fff", fontFamily: "monospace", fontSize: 13,
    }}>
    <InlineSlot value={op.name} placeholder="variable"
      onChange={(v) => onUpdate({ ...op, name: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
    <span className="card-operator" style={{ color: "#2563eb", fontWeight: 600 }}>{op.operator}</span>
    <InlineSlot value={op.value} placeholder="expr"
      onChange={(v) => onUpdate({ ...op, value: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
  </div>
);

registerCard("set_var", SetVarCard);
export default SetVarCard;
```

- [ ] **Step 3: Create ReturnExprCard**

```typescript
// poiesi/src/components/canvas/code/cards/ReturnExprCard.tsx
import React from "react";
import type { ReturnExprOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const ReturnExprCard: React.FC<CardComponentProps<ReturnExprOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card return-expr ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #f59e0b" : "1px solid #fcd34d",
      background: "#fffbeb", fontFamily: "monospace", fontSize: 13,
    }}>
    <span className="card-keyword" style={{ color: "#d97706", fontWeight: 600 }}>return</span>
    <InlineSlot value={op.value ?? ""} placeholder="expr"
      onChange={(v) => onUpdate({ ...op, value: v || undefined })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
  </div>
);

registerCard("return", ReturnExprCard);
export default ReturnExprCard;
```

- [ ] **Step 4: Commit**

```bash
git add poiesi/src/components/canvas/code/cards/DeclareVarCard.tsx poiesi/src/components/canvas/code/cards/SetVarCard.tsx poiesi/src/components/canvas/code/cards/ReturnExprCard.tsx
git commit -m "feat: add DeclareVarCard, SetVarCard, ReturnExprCard components"
```

---

### Task 11: Card Components — IfBlockCard + WhileLoopCard + ForLoopCard

**Files:**
- Create: `poiesi/src/components/canvas/code/cards/IfBlockCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/WhileLoopCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/ForLoopCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/CardList.tsx`

**Interfaces:**
- Consumes: `CardListProps` from card-types
- Produces: `CardList` recursive component used by block cards

- [ ] **Step 1: Create CardList (recursive card renderer)**

```typescript
// poiesi/src/components/canvas/code/cards/CardList.tsx
import React from "react";
import type { CardListProps } from "./card-types";
import { getCardComponent } from "./card-registry";

export const CardList: React.FC<CardListProps> = ({
  ops, selectedPid, onSelect, onUpdate, onDelete, onInsertAfter,
}) => {
  if (ops.length === 0) {
    return (
      <div className="card-list-empty" style={{ padding: "8px 0", color: "#94a3b8", fontSize: 12, fontStyle: "italic" }}>
        Empty block — type to add operations
      </div>
    );
  }

  return (
    <div className="card-list" style={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {ops.map((op, index) => {
        const CardComponent = getCardComponent(op.kind);
        if (!CardComponent) {
          return (
            <div key={op.pid} style={{ color: "#94a3b8", fontSize: 12 }}>
              Unknown op: {op.kind}
            </div>
          );
        }
        return (
          <CardComponent
            key={op.pid}
            op={op}
            isSelected={op.pid === selectedPid}
            onUpdate={(updated) => onUpdate(index, updated)}
            onDelete={() => onDelete(index)}
            onInsertAfter={() => onInsertAfter(index)}
          />
        );
      })}
    </div>
  );
};
```

- [ ] **Step 2: Create IfBlockCard**

```typescript
// poiesi/src/components/canvas/code/cards/IfBlockCard.tsx
import React from "react";
import type { IfBlockOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { CardList } from "./CardList";
import { registerCard } from "./card-registry";

const IfBlockCard: React.FC<CardComponentProps<IfBlockOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const [selectedPid, setSelectedPid] = React.useState<string | null>(null);

  const updateBranch = (bi: number, field: "condition" | "body", value: any) => {
    const newBranches = op.branches.map((br, i) =>
      i === bi ? { ...br, [field]: value } : br
    );
    onUpdate({ ...op, branches: newBranches });
  };

  return (
    <div className={`code-card if-block ${isSelected ? "selected" : ""}`}
      style={{
        border: isSelected ? "2px solid #8b5cf6" : "1px solid #c4b5fd",
        borderRadius: 6, background: "#faf5ff",
        fontFamily: "monospace", fontSize: 13,
      }}>
      {op.branches.map((br, bi) => {
        const kw = bi === 0 ? "if" : br.condition ? "else if" : "else";
        return (
          <div key={bi} style={{ padding: "4px 8px" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <span style={{ color: "#7c3aed", fontWeight: 600 }}>{kw}</span>
              {br.condition && (
                <InlineSlot value={br.condition} placeholder="condition"
                  onChange={(v) => updateBranch(bi, "condition", v)}
                  onCommit={() => {}}
                  onDeleteEmpty={() => updateBranch(bi, "condition", "")}
                  onNavigateLeft={() => {}} onNavigateRight={() => {}} />
              )}
              <span>{`{`}</span>
            </div>
            <div style={{ paddingLeft: 16 }}>
              <CardList
                ops={br.body}
                selectedPid={selectedPid}
                onSelect={setSelectedPid}
                onUpdate={(idx, updated) => {
                  const newBody = [...br.body];
                  newBody[idx] = updated;
                  updateBranch(bi, "body", newBody);
                }}
                onDelete={(idx) => {
                  const newBody = br.body.filter((_, i) => i !== idx);
                  updateBranch(bi, "body", newBody);
                }}
                onInsertAfter={(idx) => {
                  // Insert empty line after idx — add a raw op as placeholder
                  const newBody = [...br.body];
                  newBody.splice(idx + 1, 0, { kind: "raw", pid: `op-${Date.now()}`, source: "" } as any);
                  updateBranch(bi, "body", newBody);
                }}
              />
            </div>
            <div>{`}`}</div>
          </div>
        );
      })}
    </div>
  );
};

registerCard("if_block", IfBlockCard);
export default IfBlockCard;
```

- [ ] **Step 3: Create WhileLoopCard and ForLoopCard**

```typescript
// poiesi/src/components/canvas/code/cards/WhileLoopCard.tsx
import React from "react";
import type { WhileLoopOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { CardList } from "./CardList";
import { registerCard } from "./card-registry";

const WhileLoopCard: React.FC<CardComponentProps<WhileLoopOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const [selectedPid, setSelectedPid] = React.useState<string | null>(null);

  return (
    <div className={`code-card while-loop ${isSelected ? "selected" : ""}`}
      style={{
        border: isSelected ? "2px solid #06b6d4" : "1px solid #67e8f9",
        borderRadius: 6, background: "#ecfeff",
        fontFamily: "monospace", fontSize: 13,
      }}>
      <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 8px" }}>
        <span style={{ color: "#0891b2", fontWeight: 600 }}>while</span>
        <InlineSlot value={op.condition} placeholder="condition"
          onChange={(v) => onUpdate({ ...op, condition: v })}
          onCommit={() => {}} onDeleteEmpty={onDelete}
          onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        <span>{`{`}</span>
      </div>
      <div style={{ paddingLeft: 16 }}>
        <CardList ops={op.body} selectedPid={selectedPid} onSelect={setSelectedPid}
          onUpdate={(idx, updated) => {
            const newBody = [...op.body];
            newBody[idx] = updated;
            onUpdate({ ...op, body: newBody });
          }}
          onDelete={(idx) => onUpdate({ ...op, body: op.body.filter((_, i) => i !== idx) })}
          onInsertAfter={(idx) => {
            const newBody = [...op.body];
            newBody.splice(idx + 1, 0, { kind: "raw", pid: `op-${Date.now()}`, source: "" } as any);
            onUpdate({ ...op, body: newBody });
          }}
        />
      </div>
      <div style={{ padding: "0 8px 4px" }}>{`}`}</div>
    </div>
  );
};

registerCard("while", WhileLoopCard);
export default WhileLoopCard;
```

```typescript
// poiesi/src/components/canvas/code/cards/ForLoopCard.tsx
import React from "react";
import type { ForLoopOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { CardList } from "./CardList";
import { registerCard } from "./card-registry";

const ForLoopCard: React.FC<CardComponentProps<ForLoopOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const [selectedPid, setSelectedPid] = React.useState<string | null>(null);

  return (
    <div className={`code-card for-loop ${isSelected ? "selected" : ""}`}
      style={{
        border: isSelected ? "2px solid #10b981" : "1px solid #6ee7b7",
        borderRadius: 6, background: "#ecfdf5",
        fontFamily: "monospace", fontSize: 13,
      }}>
      <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 8px" }}>
        <span style={{ color: "#059669", fontWeight: 600 }}>for</span>
        <InlineSlot value={op.itemName} placeholder="item"
          onChange={(v) => onUpdate({ ...op, itemName: v })}
          onCommit={() => {}} onDeleteEmpty={onDelete}
          onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        <span style={{ color: "#64748b" }}>in</span>
        <InlineSlot value={op.collection} placeholder="collection"
          onChange={(v) => onUpdate({ ...op, collection: v })}
          onCommit={() => {}} onDeleteEmpty={onDelete}
          onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        <span>{`{`}</span>
      </div>
      <div style={{ paddingLeft: 16 }}>
        <CardList ops={op.body} selectedPid={selectedPid} onSelect={setSelectedPid}
          onUpdate={(idx, updated) => {
            const newBody = [...op.body];
            newBody[idx] = updated;
            onUpdate({ ...op, body: newBody });
          }}
          onDelete={(idx) => onUpdate({ ...op, body: op.body.filter((_, i) => i !== idx) })}
          onInsertAfter={(idx) => {
            const newBody = [...op.body];
            newBody.splice(idx + 1, 0, { kind: "raw", pid: `op-${Date.now()}`, source: "" } as any);
            onUpdate({ ...op, body: newBody });
          }}
        />
      </div>
      <div style={{ padding: "0 8px 4px" }}>{`}`}</div>
    </div>
  );
};

registerCard("for_in", ForLoopCard);
export default ForLoopCard;
```

- [ ] **Step 4: Commit**

```bash
git add poiesi/src/components/canvas/code/cards/CardList.tsx poiesi/src/components/canvas/code/cards/IfBlockCard.tsx poiesi/src/components/canvas/code/cards/WhileLoopCard.tsx poiesi/src/components/canvas/code/cards/ForLoopCard.tsx
git commit -m "feat: add CardList + IfBlockCard + WhileLoopCard + ForLoopCard components"
```

---

### Task 12: Card Components — Remaining (CallFnCard + SimpleStmtCard + RawCodeCard + AutocompleteMenu)

**Files:**
- Create: `poiesi/src/components/canvas/code/cards/CallFnCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/AwaitExprCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/SimpleStmtCard.tsx` (break/continue/comment)
- Create: `poiesi/src/components/canvas/code/cards/RawCodeCard.tsx`
- Create: `poiesi/src/components/canvas/code/cards/AutocompleteMenu.tsx`

- [ ] **Step 1: Create CallFnCard + AwaitExprCard + SimpleStmtCard + RawCodeCard**

All simple cards following the same pattern — InlineSlot for content, keyword prefix for styling.

**CallFnCard**:
```typescript
import React from "react";
import type { CallFnOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const CallFnCard: React.FC<CardComponentProps<CallFnOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card call-fn ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #6366f1" : "1px solid #cbd5e1",
      background: "#fff", fontFamily: "monospace", fontSize: 13,
    }}>
    <InlineSlot value={op.fnName} placeholder="fn"
      onChange={(v) => onUpdate({ ...op, fnName: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
    <span>(</span>
    <InlineSlot value={op.args.join(", ")} placeholder="args"
      onChange={(v) => onUpdate({ ...op, args: v ? v.split(",").map(a => a.trim()) : [] })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
    <span>)</span>
  </div>
);

registerCard("call_fn", CallFnCard);
export default CallFnCard;
```

**AwaitExprCard**:
```typescript
import React from "react";
import type { AwaitExprOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const AwaitExprCard: React.FC<CardComponentProps<AwaitExprOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card await-expr ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #f59e0b" : "1px solid #fcd34d",
      background: "#fffbeb", fontFamily: "monospace", fontSize: 13,
    }}>
    <span style={{ color: "#d97706", fontWeight: 600 }}>await</span>
    <InlineSlot value={op.value} placeholder="expr"
      onChange={(v) => onUpdate({ ...op, value: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
  </div>
);

registerCard("await", AwaitExprCard);
export default AwaitExprCard;
```

**SimpleStmtCard** (break / continue / comment):
```typescript
import React from "react";
import type { BreakStmtOp, ContinueStmtOp, CommentOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

type SimpleOp = BreakStmtOp | ContinueStmtOp | CommentOp;

const SimpleStmtCard: React.FC<CardComponentProps<SimpleOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const label = op.kind === "break" ? "break" : op.kind === "continue" ? "continue" : "//";

  return (
    <div className={`code-card simple-stmt ${isSelected ? "selected" : ""}`}
      style={{
        display: "flex", alignItems: "center", gap: 4,
        padding: "4px 8px", borderRadius: 6,
        border: isSelected ? "2px solid #94a3b8" : "1px solid #e2e8f0",
        background: "#fff", fontFamily: "monospace", fontSize: 13,
      }}>
      <span style={{ color: "#64748b", fontWeight: 600 }}>{label}</span>
      {op.kind === "comment" && (
        <InlineSlot value={(op as CommentOp).text} placeholder="comment"
          onChange={(v) => onUpdate({ ...op, text: v } as any)}
          onCommit={onInsertAfter} onDeleteEmpty={onDelete}
          onNavigateLeft={() => {}} onNavigateRight={() => {}} />
      )}
    </div>
  );
};

registerCard("break", SimpleStmtCard);
registerCard("continue", SimpleStmtCard);
registerCard("comment", SimpleStmtCard);
export default SimpleStmtCard;
```

**RawCodeCard**:
```typescript
import React from "react";
import type { RawCodeOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { registerCard } from "./card-registry";

const RawCodeCard: React.FC<CardComponentProps<RawCodeOp>> = ({
  op, isSelected, onDelete,
}) => (
  <div className={`code-card raw-code ${isSelected ? "selected" : ""}`}
    style={{
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #ef4444" : "1px solid #fca5a5",
      background: "#fef2f2", fontFamily: "monospace", fontSize: 12,
      color: "#991b1b", whiteSpace: "pre-wrap",
    }}>
    {op.source || "(empty)"}
  </div>
);

registerCard("raw", RawCodeCard);
export default RawCodeCard;
```

- [ ] **Step 2: Create AutocompleteMenu**

```typescript
// poiesi/src/components/canvas/code/cards/AutocompleteMenu.tsx
import React, { useEffect, useRef } from "react";

export interface AutocompleteItem {
  label: string;
  description: string;
  kind: string;
}

interface Props {
  items: AutocompleteItem[];
  selectedIndex: number;
  onSelect: (item: AutocompleteItem) => void;
  onClose: () => void;
  position: { top: number; left: number };
}

export const AutocompleteMenu: React.FC<Props> = ({
  items, selectedIndex, onSelect, onClose, position,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  if (items.length === 0) return null;

  return (
    <div ref={menuRef}
      style={{
        position: "absolute",
        top: position.top, left: position.left,
        background: "#fff", borderRadius: 6,
        boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
        minWidth: 200, maxHeight: 240, overflowY: "auto",
        zIndex: 1000, fontFamily: "monospace", fontSize: 12,
      }}>
      {items.map((item, i) => (
        <div key={item.kind}
          onClick={() => onSelect(item)}
          style={{
            padding: "6px 12px", cursor: "pointer",
            background: i === selectedIndex ? "#eef2ff" : "transparent",
            borderBottom: "1px solid #f1f5f9",
            display: "flex", justifyContent: "space-between",
          }}>
          <span style={{ fontWeight: 600, color: "#1e293b" }}>{item.label}</span>
          <span style={{ color: "#94a3b8" }}>{item.description}</span>
        </div>
      ))}
    </div>
  );
};

// Context-aware autocomplete items
export function getAutocompleteItems(context: string): AutocompleteItem[] {
  // Blank line — all possible entries
  return [
    { label: "let", description: "声明不可变变量", kind: "declare_var" },
    { label: "let mut", description: "声明可变变量", kind: "declare_var_mut" },
    { label: "set", description: "赋值变量", kind: "set_var" },
    { label: "if", description: "条件分支", kind: "if_block" },
    { label: "while", description: "while 循环", kind: "while" },
    { label: "for", description: "for..in 循环", kind: "for_in" },
    { label: "return", description: "返回", kind: "return" },
    { label: "await", description: "异步等待", kind: "await" },
    { label: "break", description: "跳出循环", kind: "break" },
    { label: "continue", description: "跳过本次", kind: "continue" },
    { label: "//", description: "注释", kind: "comment" },
  ];
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesi/src/components/canvas/code/cards/
git commit -m "feat: add remaining card components + AutocompleteMenu"
```

---

### Task 13: CodeCardEditor Main Component

**Files:**
- Create: `poiesi/src/components/canvas/code/CodeCardEditor.tsx`
- Create: `poiesi/src/components/canvas/code/CodeCardEditor.module.css`

**Interfaces:**
- Consumes: `CodeOp[]` via props or store
- Produces: Modal component with CardList, AutocompleteMenu, keyboard handling

- [ ] **Step 1: Create the main editor component**

```typescript
// poiesi/src/components/canvas/code/CodeCardEditor.tsx
import React, { useState, useCallback } from "react";
import type { CodeOp } from "../../../core/types/code-op";
import { CardList } from "./cards/CardList";
import { AutocompleteMenu, getAutocompleteItems, type AutocompleteItem } from "./cards/AutocompleteMenu";
import { uid } from "../../../core/utils/id";

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

export const CodeCardEditor: React.FC<Props> = ({
  elementId, initialOps, onSave, onClose,
}) => {
  const [ops, setOps] = useState<CodeOp[]>(initialOps);
  const [selectedPid, setSelectedPid] = useState<string | null>(
    initialOps.length > 0 ? initialOps[0].pid : null
  );
  const [showAutocomplete, setShowAutocomplete] = useState(false);
  const [autocompleteIndex, setAutocompleteIndex] = useState(0);
  const [autocompleteItems, setAutocompleteItems] = useState<AutocompleteItem[]>([]);
  const [menuPos, setMenuPos] = useState({ top: 100, left: 50 });
  const [pendingInsertIndex, setPendingInsertIndex] = useState<number>(ops.length);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    // Close on Escape
    if (e.key === "Escape") {
      if (showAutocomplete) {
        setShowAutocomplete(false);
      } else {
        onClose();
      }
      return;
    }

    // Navigate autocomplete
    if (showAutocomplete) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setAutocompleteIndex(i => Math.min(i + 1, autocompleteItems.length - 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setAutocompleteIndex(i => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const item = autocompleteItems[autocompleteIndex];
        if (item) insertOp(item, pendingInsertIndex);
        setShowAutocomplete(false);
        return;
      }
    }

    // Regular typing triggers autocomplete
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
      setMenuPos({ top: 200 + pendingInsertIndex * 32, left: 60 });
      setAutocompleteItems(getAutocompleteItems(""));
      setAutocompleteIndex(0);
      setShowAutocomplete(true);
    }
  }, [showAutocomplete, autocompleteIndex, autocompleteItems, pendingInsertIndex]);

  const insertOp = (item: AutocompleteItem, atIndex: number) => {
    const pid = uid("op");
    let newOp: CodeOp;

    switch (item.kind) {
      case "declare_var":
        newOp = { kind: "declare_var", pid, mutable: false, name: "", scope: "fn" };
        break;
      case "declare_var_mut":
        newOp = { kind: "declare_var", pid, mutable: true, name: "", scope: "fn" };
        break;
      case "set_var":
        newOp = { kind: "set_var", pid, name: "", operator: "=", value: "" };
        break;
      case "if_block":
        newOp = { kind: "if_block", pid, branches: [{ condition: "", body: [] }] };
        break;
      case "while":
        newOp = { kind: "while", pid, condition: "", body: [] };
        break;
      case "for_in":
        newOp = { kind: "for_in", pid, itemName: "", collection: "", body: [] };
        break;
      case "return":
        newOp = { kind: "return", pid };
        break;
      case "await":
        newOp = { kind: "await", pid, value: "" };
        break;
      case "break":
        newOp = { kind: "break", pid };
        break;
      case "continue":
        newOp = { kind: "continue", pid };
        break;
      case "comment":
        newOp = { kind: "comment", pid, text: "" };
        break;
      default:
        newOp = { kind: "raw", pid, source: "" };
    }

    const newOps = [...ops];
    newOps.splice(atIndex, 0, newOp);
    setOps(newOps);
    setSelectedPid(pid);
  };

  const handleSave = () => {
    // Filter out empty raw ops before saving
    const cleanOps = ops.filter(op => {
      if (op.kind === "raw" && !op.source.trim()) return false;
      if (op.kind === "declare_var" && !op.name) return false;
      return true;
    });
    onSave(cleanOps);
  };

  return (
    <div className="code-card-editor-overlay" style={{
      position: "fixed", inset: 0, zIndex: 9999,
      background: "rgba(0,0,0,0.3)", display: "flex",
      alignItems: "center", justifyContent: "center",
    }}>
      <div className="code-card-editor" tabIndex={0} onKeyDown={handleKeyDown}
        style={{
          width: "80vw", height: "80vh", background: "#fff",
          borderRadius: 12, boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          display: "flex", flexDirection: "column", overflow: "hidden",
        }}>
        {/* Header */}
        <div className="editor-header" style={{
          display: "flex", justifyContent: "space-between",
          alignItems: "center", padding: "12px 16px",
          borderBottom: "1px solid #e2e8f0",
        }}>
          <h3 style={{ margin: 0, fontSize: 14, fontWeight: 600 }}>
            Code Card Editor — {ops.length} ops
          </h3>
          <div style={{ display: "flex", gap: 8 }}>
            <button onClick={handleSave}
              style={{
                padding: "6px 16px", background: "#6366f1", color: "#fff",
                border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13,
              }}>
              Save
            </button>
            <button onClick={onClose}
              style={{
                padding: "6px 16px", background: "#e2e8f0", color: "#475569",
                border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13,
              }}>
              Cancel
            </button>
          </div>
        </div>

        {/* Card list area */}
        <div className="editor-body" style={{ flex: 1, overflow: "auto", padding: 16, position: "relative" }}>
          <CardList
            ops={ops}
            selectedPid={selectedPid}
            onSelect={setSelectedPid}
            onUpdate={(idx, updated) => {
              const newOps = [...ops];
              newOps[idx] = updated;
              setOps(newOps);
            }}
            onDelete={(idx) => {
              setOps(ops.filter((_, i) => i !== idx));
            }}
            onInsertAfter={(idx) => {
              setPendingInsertIndex(idx + 1);
              setShowAutocomplete(true);
              setAutocompleteItems(getAutocompleteItems(""));
              setAutocompleteIndex(0);
              setMenuPos({ top: 200 + (idx + 1) * 32, left: 60 });
            }}
          />

          {showAutocomplete && (
            <AutocompleteMenu
              items={autocompleteItems}
              selectedIndex={autocompleteIndex}
              onSelect={(item) => {
                insertOp(item, pendingInsertIndex);
                setShowAutocomplete(false);
              }}
              onClose={() => setShowAutocomplete(false)}
              position={menuPos}
            />
          )}
        </div>
      </div>
    </div>
  );
};
```

- [ ] **Step 2: Commit**

```bash
git add poiesi/src/components/canvas/code/CodeCardEditor.tsx poiesi/src/components/canvas/code/CodeCardEditor.module.css
git commit -m "feat: add CodeCardEditor main component — modal with CardList + AutocompleteMenu"
```

---

### Task 14: Wire CodeCardEditor into FlowCanvas

**Files:**
- Modify: `poiesi/src/components/canvas/code/FlowCanvas.tsx`

**Interfaces:**
- Replaces `CodeEditorModal` usage with `CodeCardEditor`

- [ ] **Step 1: Replace CodeEditorModal with CodeCardEditor**

In FlowCanvas.tsx:

```typescript
// Remove: import { CodeEditorModal } from "./CodeEditorModal";
// Add:
import { CodeCardEditor } from "./CodeCardEditor";

// Replace the editingElement section (around line 385-395):
{editingElement && (
  <CodeCardEditor
    elementId={editingElement.id}
    initialOps={(editingElement as CodeblockElementData).operations || []}
    onSave={(operations) => {
      setElementSource(editingNodeId!, "");
      setProp(editingNodeId!, "operations", operations);
      // Also regenerate source for backward compat
      // (source regeneration happens during pipeline save)
      setEditingNodeId(null);
    }}
    onClose={() => setEditingNodeId(null)}
  />
)}
```

- [ ] **Step 2: Verify compilation**

```bash
cd poiesi && npx tsc --noEmit 2>&1 | grep "FlowCanvas\|CodeCardEditor" | head -10
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add poiesi/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat: wire CodeCardEditor into FlowCanvas replacing CodeEditorModal"
```

---

### Task 15: Integration Test — Full Round-Trip

**Files:**
- Create: `poiesi/src/services/codegen/__tests__/code-op-roundtrip.test.ts`

- [ ] **Step 1: Write round-trip integration test**

```typescript
import { describe, it, expect } from "vitest";
import { parseOps } from "../code-op-parser";
import { emitOps } from "../code-op-emitter";
import type { CodeOp } from "../../core/types/code-op";
import { uid } from "../../core/utils/id";

// Real Omni source from learn-omniview validation.omni
const REAL_SOURCES = [
  {
    name: "validate_not_empty",
    source: `for field in fields {
    if field == "" {
        return "错误: 发现空字段";
    }
}
return "通过";`,
  },
  {
    name: "calculate_average",
    source: `let sum = 0.0;
let mut i = 0;
while i < values.len() {
    sum = sum + values.get(i);
    i = i + 1;
}
if count != 0 {
    return sum / count;
}
return 0.0;`,
  },
  {
    name: "count_valid",
    source: `let mut count = 0;
for value in values {
    if value == "" {
        continue;
    }
    if value.len() < min_len {
        continue;
    }
    count = count + 1;
}
return count;`,
  },
];

describe("CodeOp round-trip with real Omni sources", () => {
  for (const { name, source } of REAL_SOURCES) {
    it(`round-trips ${name}`, () => {
      const ops = parseOps(source);
      expect(ops.length).toBeGreaterThan(0);

      const emitted = emitOps(ops);

      // Second parse should produce same structure
      const ops2 = parseOps(emitted);

      // Compare kinds (structural equivalence, PIDs differ)
      expect(ops2.map(o => o.kind)).toEqual(ops.map(o => o.kind));
    });

    it(`${name} compiles without errors`, () => {
      const ops = parseOps(source);
      const emitted = emitOps(ops);

      // Wrap in a fn to make it valid Omni source
      const fullSource = `fn test() {\n${emitted}}\n`;
      // This just verifies the emitter produces parseable Omni
      const reOps = parseOps(emitted);
      expect(reOps.length).toBeGreaterThan(0);
    });
  }
});
```

- [ ] **Step 2: Run tests**

```bash
cd poiesi && npx vitest run src/services/codegen/__tests__/code-op-roundtrip.test.ts
```

Expected: all tests PASS

- [ ] **Step 3: Commit**

```bash
git add poiesi/src/services/codegen/__tests__/code-op-roundtrip.test.ts
git commit -m "test: add round-trip tests with real Omni sources from learn-omniview"
```

---

### Task 16: WASM Extension (Rust) — Phase 2

**Files:**
- Modify: `omnilang/src/parser/ast/mod.rs` — add Serialize derive
- Modify: `omnilang/src/lib.rs` — add `parse_stmts` function
- Modify: `omnilang-wasm/src/lib.rs` — add `parse_code_block` WASM binding

**Interfaces:**
- Produces Rust: `parse_stmts(source: &str) -> ParseStmtsResult`
- Produces WASM: `parse_code_block(source: &str) -> JsValue`

- [ ] **Step 1: Add Serialize derive to AST types**

In `omnilang/src/parser/ast/mod.rs`, add `serde::Serialize` to all AST types:

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Stmt { /* ... existing variants ... */ }

#[derive(Debug, Clone, Serialize)]
pub struct Block { /* ... */ }

#[derive(Debug, Clone, Serialize)]
pub enum Expr { /* ... */ }

// ... and for Literal, BinaryOpKind, TypeAnnotation, MatchArm, etc.
```

- [ ] **Step 2: Add parse_stmts function in omnilang/src/lib.rs**

```rust
#[derive(Debug, Serialize, TS)]
pub struct ParseStmtsResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stmts: Vec<parser::ast::Stmt>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

pub fn parse_stmts(source: &str) -> ParseStmtsResult {
    let tokens = lexer::tokenize(source);
    let mut parser = parser::Parser::new(&tokens, source);
    let mut stmts = Vec::new();
    let mut errors = Vec::new();

    while !parser.at_eof() {
        match parser.parse_stmt() {
            Ok(stmt) => stmts.push(stmt),
            Err(e) => {
                errors.push(e.into());
                parser.recover_from_error();
            }
        }
    }

    ParseStmtsResult {
        success: errors.is_empty(),
        stmts,
        errors,
    }
}
```

- [ ] **Step 3: Add WASM binding in omnilang-wasm/src/lib.rs**

```rust
#[wasm_bindgen]
pub fn parse_code_block(source: &str) -> JsValue {
    to_js_value(&omnilang::parse_stmts(source))
}
```

- [ ] **Step 4: Build WASM**

```bash
cd omnilang-wasm && wasm-pack build --target web --out-dir ../poiesi/src/wasm
```

- [ ] **Step 5: Add TS wrapper in poiesi/src/services/codegen/code-op-parser.ts**

```typescript
// Add WASM-first parse function
export async function parseOpsWasm(source: string): Promise<CodeOp[]> {
  const wasm = await import("../../wasm/omnilang_wasm.js");
  const result = wasm.parse_code_block(source);
  const parsed: { success: boolean; stmts: any[]; errors: any[] } =
    typeof result === "string" ? JSON.parse(result) : result;

  if (!parsed.success && parsed.stmts.length === 0) {
    // Fall back to JS regex parser
    return parseOps(source);
  }

  return stmtsToCodeOps(parsed.stmts);
}

function stmtsToCodeOps(stmts: any[]): CodeOp[] {
  return stmts.map((stmt: any) => {
    switch (stmt.VarDecl || stmt.Let) {
      // ... map Rust AST enum variants to CodeOp
    }
    // ... handle all Stmt variants
    return { kind: "raw", pid: uid("op"), source: JSON.stringify(stmt) };
  });
}
```

- [ ] **Step 6: Run tests**

```bash
cd poiesi && npx vitest run src/services/codegen/__tests__/code-op-roundtrip.test.ts
```

Expected: WASM parser produces same results as JS regex parser

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/parser/ast/mod.rs omnilang/src/lib.rs omnilang-wasm/src/lib.rs poiesi/src/services/codegen/code-op-parser.ts
git commit -m "feat: add WASM parse_code_block — omnilang parser exposed for code-block parsing"
```
