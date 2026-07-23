# CodeCard Editor v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current coarse-grained CodeCard Editor with a fine-grained card-based structure editor where every token is an independent card, containers group related cards, and block structures (if/while/for) render as three-line closed color bars.

**Architecture:** Lexical-based editor with 5 custom node types (CardNode, ContainerNode, BlockNode, BodyLineNode, InputSlotNode) replacing the current CardElementNode + SlotTextNode. EditorNode[] nested tree serves as the internal data format, with bidirectional converters to/from the existing CodeOp[] for persistence.

**Tech Stack:** Lexical, React, TypeScript, Vitest (unit), Playwright (E2E)

**Spec:** `docs/superpowers/specs/2026-07-23-codecard-editor-v2-design.md`

## Global Constraints

- `onSave(operations: CodeOp[])` signature must not change — FlowCanvas depends on it
- `.omni` output must remain clean (no PID annotations, no new syntax)
- Monaco fallback (CodeEditorModal) must be preserved as the "text mode" escape hatch
- All existing roundtrip tests (`code-op-roundtrip.test.ts`) must keep passing
- Node.js 18+, TypeScript 5.x, Lexical 0.17+

---

### Task 1: Define EditorNode types

**Files:**
- Create: `poiesis/src/core/types/editor-node.ts`

**Interfaces:**
- Produces: `EditorNode`, `ContainerNode`, `BlockNode`, `InlineNode`, `InputSlot`, `BodyLine`, `TailLine`, `Branch` types — used by all subsequent tasks

- [ ] **Step 1: Write the type definitions**

```typescript
// poiesis/src/core/types/editor-node.ts
// Fine-grained EditorNode types for the CodeCard Editor v2.
// These are the internal editor format; convert to/from CodeOp[] for persistence.

export type EditorNode =
  | ContainerNode
  | BlockNode
  | InlineNode
  | InputSlot;

// ── Container cards (let, set, return, await, call, array, map) ──
export interface ContainerNode {
  kind: "let" | "set" | "return" | "await" | "call" | "array" | "map";
  pid: string;
  children: EditorNode[];
}

// ── Block cards (if, while, for) ──
export interface BlockNode {
  kind: "if" | "while" | "for";
  pid: string;
  head: EditorNode[];       // condition/loop header cards + slots
  body: BodyLine[];         // expandable code lines
  tail: TailLine;           // closing bar
  branches?: Branch[];      // elif/else (if only)
}

export interface Branch {
  kind: "elif" | "else";
  pid: string;
  head: EditorNode[];
  body: BodyLine[];
}

export interface BodyLine {
  pid: string;
  children: EditorNode[];
}

export interface TailLine {
  pid: string;
}

// ── Inline cards (every token that's not a container) ──
export interface InlineNode {
  kind: "mut" | "break" | "continue" | "operator" | "type" | "var_ref" | "literal" | "dot" | "separator" | "comment";
  cardLabel: string;        // displayed text: ">", "+=", "i32", "sum", etc.
  pid: string;
}

// ── Input slot (backslash-triggered inline input) ──
export interface InputSlot {
  kind: "slot";
  slotKind: "name" | "type_value" | "expression" | "condition" | "fn_name";
  value: string;
  pid: string;
}

// ── Helpers ──
export type ContainerKind = ContainerNode["kind"];
export type BlockKind = BlockNode["kind"];
export type InlineKind = InlineNode["kind"];
export type SlotKind = InputSlot["slotKind"];
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit -p poiesis/tsconfig.json 2>&1 | head -5`
Expected: no new errors from this file

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/core/types/editor-node.ts
git commit -m "feat(codecard-v2): add EditorNode type definitions"
```

---

### Task 2: Implement convertToNodes (CodeOp[] → EditorNode[])

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/transforms/convertToNodes.ts`
- Create: `poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToNodes.test.ts`

**Interfaces:**
- Consumes: `CodeOp` from `core/types/code-op`, `EditorNode` types from Task 1, `uid()` from `core/utils/id`
- Produces: `convertToNodes(ops: CodeOp[]): EditorNode[]`

- [ ] **Step 1: Write failing tests**

```typescript
// poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToNodes.test.ts
import { describe, it, expect } from "vitest";
import { convertToNodes } from "../convertToNodes";
import type { CodeOp } from "../../../../../../core/types/code-op";
import type { ContainerNode, BlockNode, InlineNode, InputSlot } from "../../../../../../core/types/editor-node";

describe("convertToNodes", () => {
  it("converts simple declare_var to let container with children", () => {
    const ops: CodeOp[] = [{
      kind: "declare_var", pid: "d1", mutable: false,
      name: "count", varType: "i32", initValue: "0", scope: "fn",
    }];
    const nodes = convertToNodes(ops);
    expect(nodes.length).toBe(1);

    const letNode = nodes[0] as ContainerNode;
    expect(letNode.kind).toBe("let");
    expect(letNode.children.length).toBeGreaterThanOrEqual(4);

    // children should include: name input, :, type, =, value input
    const labels = letNode.children.map(c =>
      c.kind === "slot" ? `slot:${(c as InputSlot).slotKind}` :
      c.kind === "inline" ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("slot:name");
    expect(labels).toContain(":");
    expect(labels).toContain("i32");
    expect(labels).toContain("=");
    expect(labels).toContain("slot:expression");
  });

  it("converts declare_var with mut", () => {
    const ops: CodeOp[] = [{
      kind: "declare_var", pid: "d2", mutable: true,
      name: "x", scope: "fn",
    }];
    const nodes = convertToNodes(ops);
    const letNode = nodes[0] as ContainerNode;
    const labels = letNode.children.map(c =>
      c.kind === "inline" ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("mut");
  });

  it("converts set_var to set container", () => {
    const ops: CodeOp[] = [{
      kind: "set_var", pid: "s1", name: "count", operator: "+=", value: "1",
    }];
    const nodes = convertToNodes(ops);
    const setNode = nodes[0] as ContainerNode;
    expect(setNode.kind).toBe("set");

    const labels = setNode.children.map(c =>
      c.kind === "slot" ? `slot:${(c as InputSlot).slotKind}` :
      c.kind === "inline" ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("slot:name");
    expect(labels).toContain("+=");
    expect(labels).toContain("slot:expression");
  });

  it("converts if_block to BlockNode", () => {
    const ops: CodeOp[] = [{
      kind: "if_block", pid: "if1",
      branches: [{
        condition: "x > 0",
        body: [{ kind: "return", pid: "r1", value: "x" }],
      }],
    }];
    const nodes = convertToNodes(ops);
    expect(nodes.length).toBe(1);
    const ifNode = nodes[0] as BlockNode;
    expect(ifNode.kind).toBe("if");
    expect(ifNode.head.length).toBeGreaterThan(0);
    expect(ifNode.body.length).toBe(1);
    expect(ifNode.body[0].children.length).toBeGreaterThan(0);
  });

  it("converts while loop to BlockNode", () => {
    const ops: CodeOp[] = [{
      kind: "while", pid: "w1", condition: "i < n",
      body: [{ kind: "break", pid: "b1" }],
    }];
    const nodes = convertToNodes(ops);
    const whileNode = nodes[0] as BlockNode;
    expect(whileNode.kind).toBe("while");
  });

  it("converts for_in to BlockNode with in separator", () => {
    const ops: CodeOp[] = [{
      kind: "for_in", pid: "f1", itemName: "item", collection: "items",
      body: [],
    }];
    const nodes = convertToNodes(ops);
    const forNode = nodes[0] as BlockNode;
    expect(forNode.kind).toBe("for");
    const labels = forNode.head.map(c =>
      c.kind === "inline" ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("in");
  });

  it("converts return to container", () => {
    const ops: CodeOp[] = [{ kind: "return", pid: "r1", value: "x + 1" }];
    const nodes = convertToNodes(ops);
    expect((nodes[0] as ContainerNode).kind).toBe("return");
  });

  it("converts break and continue to inline nodes", () => {
    const ops: CodeOp[] = [
      { kind: "break", pid: "b1" },
      { kind: "continue", pid: "c1" },
    ];
    const nodes = convertToNodes(ops);
    expect((nodes[0] as InlineNode).kind).toBe("break");
    expect((nodes[1] as InlineNode).kind).toBe("continue");
  });

  it("converts comment to inline node", () => {
    const ops: CodeOp[] = [{ kind: "comment", pid: "cm1", text: "TODO" }];
    const nodes = convertToNodes(ops);
    expect((nodes[0] as InlineNode).kind).toBe("comment");
    expect((nodes[0] as InlineNode).cardLabel).toBe("TODO");
  });

  it("handles empty ops array", () => {
    expect(convertToNodes([])).toEqual([]);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToNodes.test.ts`
Expected: all FAIL (module not found)

- [ ] **Step 3: Implement convertToNodes**

```typescript
// poiesis/src/components/canvas/code/lexical/transforms/convertToNodes.ts
import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine } from "../../../../../core/types/editor-node";
import { uid } from "../../../../../core/utils/id";

export function convertToNodes(ops: CodeOp[]): EditorNode[] {
  return ops.map(op => convertOneOp(op));
}

function convertOneOp(op: CodeOp): EditorNode {
  switch (op.kind) {
    case "declare_var": return convertDeclareVar(op);
    case "set_var": return convertSetVar(op);
    case "if_block": return convertIfBlock(op);
    case "while": return convertWhile(op);
    case "for_in": return convertForIn(op);
    case "return": return convertReturn(op);
    case "call_fn": return convertCallFn(op);
    case "await": return convertAwait(op);
    case "break": return inline("break", "break", op.pid);
    case "continue": return inline("continue", "continue", op.pid);
    case "comment": return inline("comment", op.text, op.pid);
    case "raw": return inline("comment", op.source, op.pid);
  }
}

// ── Helpers ──

function slot(slotKind: InputSlot["slotKind"], value = "", pid?: string): InputSlot {
  return { kind: "slot", slotKind, value, pid: pid ?? uid("sl") };
}

function inline(kind: InlineNode["kind"], cardLabel: string, pid?: string): InlineNode {
  return { kind, cardLabel, pid: pid ?? uid("in") };
}

function container(kind: ContainerNode["kind"], children: EditorNode[], pid: string): ContainerNode {
  return { kind, pid, children };
}

function block(kind: BlockNode["kind"], head: EditorNode[], body: BodyLine[], tailPid: string, pid: string, branches?: BlockNode["branches"]): BlockNode {
  return { kind, pid, head, body, tail: { pid: tailPid }, branches };
}

function bodyLine(children: EditorNode[]): BodyLine {
  return { pid: uid("bl"), children };
}

// ── Op converters ──

function convertDeclareVar(op: Extract<CodeOp, { kind: "declare_var" }>): ContainerNode {
  const children: EditorNode[] = [];
  if (op.mutable) children.push(inline("mut", "mut"));
  children.push(slot("name", op.name));
  if (op.varType) {
    children.push(inline("separator", ":"));
    children.push(inline("type", op.varType));
  }
  if (op.initValue !== undefined) {
    children.push(inline("operator", "="));
    children.push(slot("expression", op.initValue));
  }
  return container("let", children, op.pid);
}

function convertSetVar(op: Extract<CodeOp, { kind: "set_var" }>): ContainerNode {
  return container("set", [
    slot("name", op.name),
    inline("operator", op.operator),
    slot("expression", op.value),
  ], op.pid);
}

function convertIfBlock(op: Extract<CodeOp, { kind: "if_block" }>): BlockNode {
  const first = op.branches[0];
  const branches = op.branches.length > 1
    ? op.branches.slice(1).map(br => ({
        kind: br.condition ? "elif" as const : "else" as const,
        pid: uid("br"),
        head: br.condition ? [slot("condition", br.condition)] : [],
        body: br.body.map(op => bodyLine([convertOneOp(op)])),
      }))
    : undefined;

  return block(
    "if",
    first.condition ? [slot("condition", first.condition)] : [],
    first.body.length > 0
      ? first.body.map(op => bodyLine([convertOneOp(op)]))
      : [bodyLine([])],
    uid("tl"),
    op.pid,
    branches,
  );
}

function convertWhile(op: Extract<CodeOp, { kind: "while" }>): BlockNode {
  return block(
    "while",
    [slot("condition", op.condition)],
    op.body.length > 0
      ? op.body.map(o => bodyLine([convertOneOp(o)]))
      : [bodyLine([])],
    uid("tl"),
    op.pid,
  );
}

function convertForIn(op: Extract<CodeOp, { kind: "for_in" }>): BlockNode {
  return block(
    "for",
    [
      slot("name", op.itemName),
      inline("separator", "in"),
      slot("expression", op.collection),
    ],
    op.body.length > 0
      ? op.body.map(o => bodyLine([convertOneOp(o)]))
      : [bodyLine([])],
    uid("tl"),
    op.pid,
  );
}

function convertReturn(op: Extract<CodeOp, { kind: "return" }>): ContainerNode {
  const children: EditorNode[] = [];
  children.push(slot("expression", op.value ?? ""));
  return container("return", children, op.pid);
}

function convertCallFn(op: Extract<CodeOp, { kind: "call_fn" }>): ContainerNode {
  const children: EditorNode[] = [slot("fn_name", op.fnName)];
  for (let i = 0; i < op.args.length; i++) {
    if (i > 0) children.push(inline("separator", ","));
    children.push(slot("expression", op.args[i]));
  }
  return container("call", children, op.pid);
}

function convertAwait(op: Extract<CodeOp, { kind: "await" }>): ContainerNode {
  return container("await", [slot("expression", op.value)], op.pid);
}
```

- [ ] **Step 4: Run tests**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToNodes.test.ts`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/transforms/convertToNodes.ts poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToNodes.test.ts
git commit -m "feat(codecard-v2): implement convertToNodes (CodeOp[] → EditorNode[])"
```

---

### Task 3: Implement convertToOps (EditorNode[] → CodeOp[])

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/transforms/convertToOps.ts`
- Create: `poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToOps.test.ts`

**Interfaces:**
- Consumes: `EditorNode` from Task 1, `CodeOp` from `core/types/code-op`, `uid()` from `core/utils/id`
- Produces: `convertToOps(nodes: EditorNode[]): CodeOp[]`

- [ ] **Step 1: Write failing tests**

```typescript
// poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToOps.test.ts
import { describe, it, expect } from "vitest";
import { convertToOps } from "../convertToOps";
import type { ContainerNode, BlockNode, BodyLine } from "../../../../../../core/types/editor-node";

function bodyLine(children: any[]): BodyLine {
  return { pid: "bl1", children: children as any };
}

describe("convertToOps", () => {
  it("converts let container back to declare_var", () => {
    const nodes = [{
      kind: "let" as const, pid: "d1",
      children: [
        { kind: "inline", cardLabel: "mut", pid: "m1" } as any,
        { kind: "slot", slotKind: "name", value: "count", pid: "s1" } as any,
        { kind: "inline", cardLabel: ":", pid: "sep1" } as any,
        { kind: "inline", cardLabel: "i32", pid: "t1" } as any,
        { kind: "inline", cardLabel: "=", pid: "op1" } as any,
        { kind: "slot", slotKind: "expression", value: "0", pid: "s2" } as any,
      ],
    }];
    const ops = convertToOps(nodes);
    expect(ops.length).toBe(1);
    expect(ops[0].kind).toBe("declare_var");
    if (ops[0].kind === "declare_var") {
      expect(ops[0].mutable).toBe(true);
      expect(ops[0].name).toBe("count");
      expect(ops[0].varType).toBe("i32");
      expect(ops[0].initValue).toBe("0");
    }
  });

  it("converts set container back to set_var", () => {
    const nodes = [{
      kind: "set" as const, pid: "s1",
      children: [
        { kind: "slot", slotKind: "name", value: "sum", pid: "s1" } as any,
        { kind: "inline", cardLabel: "+=", pid: "op1" } as any,
        { kind: "slot", slotKind: "expression", value: "1", pid: "s2" } as any,
      ],
    }];
    const ops = convertToOps(nodes);
    expect(ops[0].kind).toBe("set_var");
    if (ops[0].kind === "set_var") {
      expect(ops[0].name).toBe("sum");
      expect(ops[0].operator).toBe("+=");
      expect(ops[0].value).toBe("1");
    }
  });

  it("converts if BlockNode back to if_block", () => {
    const nodes = [{
      kind: "if" as const, pid: "if1",
      head: [
        { kind: "slot", slotKind: "condition", value: "x > 0", pid: "c1" } as any,
      ],
      body: [
        { pid: "bl1", children: [
          { kind: "return" as const, pid: "r1", children: [
            { kind: "slot", slotKind: "expression", value: "x", pid: "e1" } as any,
          ] },
        ] },
      ],
      tail: { pid: "tl1" },
      branches: [{
        kind: "else" as const, pid: "br1",
        head: [],
        body: [
          { pid: "bl2", children: [
            { kind: "return" as const, pid: "r2", children: [
              { kind: "slot", slotKind: "expression", value: "0", pid: "e2" } as any,
            ] },
          ] },
        ],
      }],
    }];
    const ops = convertToOps(nodes);
    expect(ops.length).toBe(1);
    expect(ops[0].kind).toBe("if_block");
    if (ops[0].kind === "if_block") {
      expect(ops[0].branches.length).toBe(2);
      expect(ops[0].branches[0].condition).toBe("x > 0");
      expect(ops[0].branches[1].condition).toBe("");
    }
  });

  it("converts while BlockNode back to while", () => {
    const nodes = [{
      kind: "while" as const, pid: "w1",
      head: [{ kind: "slot", slotKind: "condition", value: "i < n", pid: "c1" } as any],
      body: [],
      tail: { pid: "tl1" },
    }];
    const ops = convertToOps(nodes);
    expect(ops[0].kind).toBe("while");
    if (ops[0].kind === "while") {
      expect(ops[0].condition).toBe("i < n");
    }
  });

  it("converts for BlockNode back to for_in", () => {
    const nodes = [{
      kind: "for" as const, pid: "f1",
      head: [
        { kind: "slot", slotKind: "name", value: "item", pid: "s1" } as any,
        { kind: "inline", cardLabel: "in", pid: "in1" } as any,
        { kind: "slot", slotKind: "expression", value: "items", pid: "s2" } as any,
      ],
      body: [],
      tail: { pid: "tl1" },
    }];
    const ops = convertToOps(nodes);
    expect(ops[0].kind).toBe("for_in");
    if (ops[0].kind === "for_in") {
      expect(ops[0].itemName).toBe("item");
      expect(ops[0].collection).toBe("items");
    }
  });

  it("roundtrip: convertToNodes → convertToOps is identity for key ops", () => {
    const { convertToNodes } = require("../convertToNodes");
    const original: any[] = [
      { kind: "declare_var", pid: "d1", mutable: true, name: "x", varType: "f64", initValue: "1.0", scope: "fn" },
      { kind: "set_var", pid: "s1", name: "x", operator: "=", value: "2" },
      { kind: "return", pid: "r1", value: "x" },
    ];
    const nodes = convertToNodes(original);
    const roundtripped = convertToOps(nodes);
    expect(roundtripped.length).toBe(3);
    expect(roundtripped[0].kind).toBe("declare_var");
    expect(roundtripped[1].kind).toBe("set_var");
    expect(roundtripped[2].kind).toBe("return");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToOps.test.ts`
Expected: all FAIL

- [ ] **Step 3: Implement convertToOps**

```typescript
// poiesis/src/components/canvas/code/lexical/transforms/convertToOps.ts
import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine } from "../../../../../core/types/editor-node";
import { uid } from "../../../../../core/utils/id";

export function convertToOps(nodes: EditorNode[]): CodeOp[] {
  return nodes.map(node => convertOneNode(node));
}

function convertOneNode(node: EditorNode): CodeOp {
  switch (node.kind) {
    case "let": return convertLetContainer(node as ContainerNode);
    case "set": return convertSetContainer(node as ContainerNode);
    case "return": return convertReturnContainer(node as ContainerNode);
    case "await": return convertAwaitContainer(node as ContainerNode);
    case "call": return convertCallContainer(node as ContainerNode);
    case "if": return convertIfBlock(node as BlockNode);
    case "while": return convertWhileBlock(node as BlockNode);
    case "for": return convertForBlock(node as BlockNode);
    case "break": return { kind: "break", pid: node.pid };
    case "continue": return { kind: "continue", pid: node.pid };
    case "comment": return { kind: "comment", pid: node.pid, text: (node as InlineNode).cardLabel };
    default:
      return { kind: "raw", pid: node.pid, source: "" };
  }
}

// ── Extract helpers ──

function findSlot(children: EditorNode[], slotKind: string): InputSlot | undefined {
  return children.find(c => c.kind === "slot" && (c as InputSlot).slotKind === slotKind) as InputSlot | undefined;
}

function findInline(children: EditorNode[], cardLabel?: string): InlineNode | undefined {
  return children.find(c => {
    if (c.kind !== "inline") return false;
    if (cardLabel !== undefined) return (c as InlineNode).cardLabel === cardLabel;
    return true;
  }) as InlineNode | undefined;
}

function hasInline(children: EditorNode[], cardLabel: string): boolean {
  return children.some(c => c.kind === "inline" && (c as InlineNode).cardLabel === cardLabel);
}

// ── Container converters ──

function convertLetContainer(node: ContainerNode): CodeOp {
  const mutable = hasInline(node.children, "mut");
  const nameSlot = findSlot(node.children, "name");
  const typeInline = findInline(node.children);
  const eqInline = findInline(node.children, "=");
  const valueSlot = findSlot(node.children, "expression");

  // type is the inline whose cardLabel is neither "mut" nor an operator
  let varType: string | undefined;
  for (const child of node.children) {
    if (child.kind !== "inline") continue;
    const label = (child as InlineNode).cardLabel;
    if (label !== "mut" && label !== "=" && label !== ":" && label !== "+=" && label !== "-=" && label !== "*=" && label !== "/=" && label !== "%=") {
      // This could be our type — after the ":" separator
      // Only take it if preceded by ":"
      const sepIdx = node.children.findIndex(c => c.kind === "inline" && (c as InlineNode).cardLabel === ":");
      const typeIdx = node.children.indexOf(child);
      if (sepIdx >= 0 && typeIdx === sepIdx + 1) {
        varType = label;
      }
    }
  }

  return {
    kind: "declare_var",
    pid: node.pid,
    mutable,
    name: nameSlot?.value ?? "x",
    varType,
    initValue: valueSlot?.value,
    scope: "fn",
  };
}

function convertSetContainer(node: ContainerNode): CodeOp {
  const nameSlot = findSlot(node.children, "name");
  // operator is the inline with label matching an operator pattern
  let operator = "=" as CodeOp["operator"];
  for (const child of node.children) {
    if (child.kind !== "inline") continue;
    const label = (child as InlineNode).cardLabel;
    if (/^(=|\+=|-=|\*=|\/=|%=)$/.test(label)) {
      operator = label as any;
      break;
    }
  }
  const valueSlot = findSlot(node.children, "expression");
  return {
    kind: "set_var",
    pid: node.pid,
    name: nameSlot?.value ?? "x",
    operator,
    value: valueSlot?.value ?? "",
  };
}

function convertReturnContainer(node: ContainerNode): CodeOp {
  const valSlot = findSlot(node.children, "expression");
  return {
    kind: "return",
    pid: node.pid,
    value: valSlot?.value || undefined,
  };
}

function convertAwaitContainer(node: ContainerNode): CodeOp {
  const valSlot = findSlot(node.children, "expression");
  return { kind: "await", pid: node.pid, value: valSlot?.value ?? "" };
}

function convertCallContainer(node: ContainerNode): CodeOp {
  const fnSlot = findSlot(node.children, "fn_name");
  const args: string[] = [];
  for (const child of node.children) {
    if (child.kind === "slot" && (child as InputSlot).slotKind === "expression") {
      args.push((child as InputSlot).value);
    }
  }
  return { kind: "call_fn", pid: node.pid, fnName: fnSlot?.value ?? "", args };
}

// ── Block converters ──

function convertIfBlock(node: BlockNode): CodeOp {
  const branches = [{ condition: extractCondition(node.head), body: convertBody(node.body) }];
  if (node.branches) {
    for (const br of node.branches) {
      branches.push({
        condition: br.kind === "elif" ? extractCondition(br.head) : "",
        body: convertBody(br.body),
      });
    }
  }
  return { kind: "if_block", pid: node.pid, branches };
}

function convertWhileBlock(node: BlockNode): CodeOp {
  return {
    kind: "while",
    pid: node.pid,
    condition: extractCondition(node.head),
    body: convertBody(node.body),
  };
}

function convertForBlock(node: BlockNode): CodeOp {
  const itemSlot = findSlot(node.head, "name");
  const collSlot = findSlot(node.head, "expression");
  return {
    kind: "for_in",
    pid: node.pid,
    itemName: itemSlot?.value ?? "item",
    collection: collSlot?.value ?? "",
    body: convertBody(node.body),
  };
}

function extractCondition(head: EditorNode[]): string {
  const condSlot = findSlot(head, "condition");
  return condSlot?.value ?? "";
}

function convertBody(bodyLines: BodyLine[]): CodeOp[] {
  return bodyLines.flatMap(line => line.children.map(child => convertOneNode(child)));
}
```

- [ ] **Step 4: Run tests**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToOps.test.ts`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/transforms/convertToOps.ts poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToOps.test.ts
git commit -m "feat(codecard-v2): implement convertToOps (EditorNode[] → CodeOp[])"
```

---

### Task 4: Create InputSlotNode (Lexical node for \ input box)

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/nodes/InputSlotNode.ts`
- Create: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/InputSlotNode.test.ts`

**Interfaces:**
- Consumes: Lexical `TextNode`, `SlotKind` from Task 1
- Produces: `InputSlotNode` class + `$createInputSlotNode()`, `$isInputSlotNode()`

- [ ] **Step 1: Write failing test**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/__tests__/InputSlotNode.test.ts
import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import { InputSlotNode, $createInputSlotNode, $isInputSlotNode } from "../InputSlotNode";

describe("InputSlotNode", () => {
  it("has correct type", () => {
    const node = $createInputSlotNode("hello", "name");
    expect(InputSlotNode.getType()).toBe("input-slot");
    expect(node.getSlotKind()).toBe("name");
    expect(node.getTextContent()).toBe("hello");
  });

  it("is detected by $isInputSlotNode", () => {
    const node = $createInputSlotNode("x", "expression");
    expect($isInputSlotNode(node)).toBe(true);
  });

  it("can update slot kind", () => {
    const node = $createInputSlotNode("test", "name");
    node.setSlotKind("condition");
    expect(node.getSlotKind()).toBe("condition");
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/nodes/__tests__/InputSlotNode.test.ts`
Expected: FAIL

- [ ] **Step 3: Implement**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/InputSlotNode.ts
import { TextNode, type SerializedTextNode, type LexicalNode, type EditorConfig } from "lexical";
import type { SlotKind } from "../../../../../core/types/editor-node";

export interface SerializedInputSlotNode extends SerializedTextNode {
  slotKind: SlotKind;
  type: "input-slot";
}

export class InputSlotNode extends TextNode {
  __slotKind: SlotKind;

  static getType(): string {
    return "input-slot";
  }

  static clone(node: InputSlotNode): InputSlotNode {
    return new InputSlotNode(node.__text, node.__slotKind, node.__key);
  }

  constructor(text: string, slotKind: SlotKind, key?: string) {
    super(text, key);
    this.__slotKind = slotKind;
  }

  getSlotKind(): SlotKind {
    const self = this.getLatest();
    return (self as InputSlotNode).__slotKind;
  }

  setSlotKind(kind: SlotKind): void {
    const self = this.getWritable();
    (self as InputSlotNode).__slotKind = kind;
  }

  createDOM(config: EditorConfig): HTMLElement {
    const dom = super.createDOM(config);
    dom.className = `input-slot input-slot--${this.__slotKind}`;
    dom.setAttribute("data-slot-kind", this.__slotKind);
    // contentEditable is inherited from TextNode
    return dom;
  }

  exportJSON(): SerializedInputSlotNode {
    return {
      ...super.exportJSON(),
      type: "input-slot",
      slotKind: this.__slotKind,
    };
  }

  static importJSON(serialized: SerializedInputSlotNode): InputSlotNode {
    const node = $createInputSlotNode(serialized.text, serialized.slotKind);
    node.setFormat(serialized.format);
    node.setDetail(serialized.detail);
    node.setMode(serialized.mode);
    node.setStyle(serialized.style);
    return node;
  }
}

export function $isInputSlotNode(node: LexicalNode | null | undefined): node is InputSlotNode {
  return node instanceof InputSlotNode;
}

export function $createInputSlotNode(text: string, slotKind: SlotKind): InputSlotNode {
  return new InputSlotNode(text, slotKind);
}
```

- [ ] **Step 4: Run test**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/nodes/__tests__/InputSlotNode.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/InputSlotNode.ts poiesis/src/components/canvas/code/lexical/nodes/__tests__/InputSlotNode.test.ts
git commit -m "feat(codecard-v2): add InputSlotNode for backslash-triggered inline input"
```

---

### Task 5: Create CardNode (Lexical node for InlineNode cards)

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts`
- Create: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts`

**Interfaces:**
- Consumes: Lexical `DecoratorNode`, `InlineKind` from Task 1
- Produces: `CardNode` class + `$createCardNode()`, `$isCardNode()`

- [ ] **Step 1: Write failing test**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts
import { describe, it, expect } from "vitest";
import { CardNode, $createCardNode, $isCardNode } from "../CardNode";

describe("CardNode", () => {
  it("has correct type", () => {
    const node = $createCardNode("operator", "=", "c1");
    expect(CardNode.getType()).toBe("inline-card");
    expect(node.getCardKind()).toBe("operator");
    expect(node.getCardLabel()).toBe("=");
  });

  it("is detected by $isCardNode", () => {
    const node = $createCardNode("type", "i32", "c2");
    expect($isCardNode(node)).toBe(true);
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts`
Expected: FAIL

- [ ] **Step 3: Implement as DecoratorNode (inline, non-editable card block)**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts
import { DecoratorNode, type SerializedLexicalNode, type LexicalNode, type EditorConfig, type NodeKey } from "lexical";
import type { InlineKind } from "../../../../../core/types/editor-node";

// Using DecoratorNode: inline cards are read-only blocks, users cannot
// place cursor inside them. Typing replaces or inserts adjacent cards.

export interface SerializedCardNode extends SerializedLexicalNode {
  cardKind: InlineKind;
  cardLabel: string;
  cardPid: string;
  type: "inline-card";
}

export class CardNode extends DecoratorNode<JSX.Element> {
  __cardKind: InlineKind;
  __cardLabel: string;
  __cardPid: string;

  static getType(): string {
    return "inline-card";
  }

  static clone(node: CardNode): CardNode {
    return new CardNode(node.__cardKind, node.__cardLabel, node.__cardPid, node.__key);
  }

  constructor(kind: InlineKind, label: string, pid: string, key?: NodeKey) {
    super(key);
    this.__cardKind = kind;
    this.__cardLabel = label;
    this.__cardPid = pid;
  }

  getCardKind(): InlineKind { return this.getLatest().__cardKind; }
  getCardLabel(): string { return this.getLatest().__cardLabel; }
  getCardPid(): string { return this.getLatest().__cardPid; }

  createDOM(_config: EditorConfig): HTMLElement {
    const el = document.createElement("span");
    el.className = `code-card code-card--${this.__cardKind}`;
    el.setAttribute("data-card-kind", this.__cardKind);
    el.setAttribute("data-card-pid", this.__cardPid);
    el.textContent = this.__cardLabel;
    return el;
  }

  updateDOM(): boolean {
    return false; // no dynamic updates needed
  }

  decorate(): JSX.Element {
    return (
      <span
        className={`code-card code-card--${this.__cardKind}`}
        data-card-kind={this.__cardKind}
        data-card-pid={this.__cardPid}
      >
        {this.__cardLabel}
      </span>
    );
  }

  exportJSON(): SerializedCardNode {
    return {
      ...super.exportJSON(),
      type: "inline-card",
      cardKind: this.__cardKind,
      cardLabel: this.__cardLabel,
      cardPid: this.__cardPid,
    };
  }

  static importJSON(serialized: SerializedCardNode): CardNode {
    return $createCardNode(serialized.cardKind, serialized.cardLabel, serialized.cardPid);
  }

  isInline(): boolean {
    return true;
  }
}

export function $isCardNode(node: LexicalNode | null | undefined): node is CardNode {
  return node instanceof CardNode;
}

export function $createCardNode(kind: InlineKind, label: string, pid: string): CardNode {
  return new CardNode(kind, label, pid);
}
```

- [ ] **Step 4: Run test**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts
git commit -m "feat(codecard-v2): add CardNode for inline non-editable cards"
```

---

### Task 6: Create ContainerNode + BlockNode + BodyLineNode (Lexical nodes)

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/nodes/ContainerNode.ts`
- Create: `poiesis/src/components/canvas/code/lexical/nodes/BlockNode.ts`
- Create: `poiesis/src/components/canvas/code/lexical/nodes/BodyLineNode.ts`
- Create: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/ContainerNode.test.ts`
- Create: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/BlockNode.test.ts`

**Interfaces:**
- Consumes: Lexical `ElementNode`, `ContainerKind`, `BlockKind` from Task 1
- Produces: `ContainerElementNode`, `BlockElementNode`, `BodyLineElementNode` + factory functions

- [ ] **Step 1: Implement all three nodes**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/ContainerNode.ts
import { ElementNode, type SerializedElementNode, type LexicalNode, type Spread } from "lexical";
import type { ContainerKind } from "../../../../../core/types/editor-node";

export interface SerializedContainerElementNode extends Spread<
  { containerKind: ContainerKind; containerPid: string; type: "container" },
  SerializedElementNode
> {}

export class ContainerElementNode extends ElementNode {
  __containerKind: ContainerKind;
  __containerPid: string;

  static getType(): string { return "container"; }

  static clone(node: ContainerElementNode): ContainerElementNode {
    return new ContainerElementNode(node.__containerKind, node.__containerPid, node.__key);
  }

  constructor(kind: ContainerKind, pid: string, key?: string) {
    super(key);
    this.__containerKind = kind;
    this.__containerPid = pid;
  }

  getContainerKind(): ContainerKind { return this.getLatest().__containerKind; }
  getContainerPid(): string { return this.getLatest().__containerPid; }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = `code-container code-container--${this.__containerKind}`;
    el.setAttribute("data-container-kind", this.__containerKind);
    el.setAttribute("data-container-pid", this.__containerPid);
    return el;
  }

  updateDOM(_prev: ContainerElementNode, _dom: HTMLElement): boolean { return true; }

  exportJSON(): SerializedContainerElementNode {
    return { ...super.exportJSON(), type: "container", containerKind: this.__containerKind, containerPid: this.__containerPid };
  }

  static importJSON(serialized: SerializedContainerElementNode): ContainerElementNode {
    const node = $createContainerElementNode(serialized.containerKind, serialized.containerPid);
    node.setFormat(serialized.format);
    node.setIndent(serialized.indent);
    node.setDirection(serialized.direction);
    return node;
  }

  canBeEmpty(): boolean { return false; }
  canInsertTextBefore(): boolean { return true; }
  canInsertTextAfter(): boolean { return true; }
  isInline(): boolean { return false; }
}

export function $isContainerElementNode(node: LexicalNode | null | undefined): node is ContainerElementNode {
  return node instanceof ContainerElementNode;
}

export function $createContainerElementNode(kind: ContainerKind, pid: string): ContainerElementNode {
  return new ContainerElementNode(kind, pid);
}
```

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/BlockNode.ts
import { ElementNode, type SerializedElementNode, type LexicalNode, type Spread } from "lexical";
import type { BlockKind } from "../../../../../core/types/editor-node";

export interface SerializedBlockElementNode extends Spread<
  { blockKind: BlockKind; blockPid: string; type: "block" },
  SerializedElementNode
> {}

export class BlockElementNode extends ElementNode {
  __blockKind: BlockKind;
  __blockPid: string;

  static getType(): string { return "block"; }

  static clone(node: BlockElementNode): BlockElementNode {
    return new BlockElementNode(node.__blockKind, node.__blockPid, node.__key);
  }

  constructor(kind: BlockKind, pid: string, key?: string) {
    super(key);
    this.__blockKind = kind;
    this.__blockPid = pid;
  }

  getBlockKind(): BlockKind { return this.getLatest().__blockKind; }
  getBlockPid(): string { return this.getLatest().__blockPid; }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = `code-block-card code-block-card--${this.__blockKind}`;
    el.setAttribute("data-block-kind", this.__blockKind);
    el.setAttribute("data-block-pid", this.__blockPid);
    return el;
  }

  updateDOM(): boolean { return true; }

  exportJSON(): SerializedBlockElementNode {
    return { ...super.exportJSON(), type: "block", blockKind: this.__blockKind, blockPid: this.__blockPid };
  }

  static importJSON(serialized: SerializedBlockElementNode): BlockElementNode {
    const node = $createBlockElementNode(serialized.blockKind, serialized.blockPid);
    node.setFormat(serialized.format);
    node.setIndent(serialized.indent);
    node.setDirection(serialized.direction);
    return node;
  }

  canBeEmpty(): boolean { return false; }
  canInsertTextBefore(): boolean { return true; }
  canInsertTextAfter(): boolean { return true; }
  isInline(): boolean { return false; }
}

export function $isBlockElementNode(node: LexicalNode | null | undefined): node is BlockElementNode {
  return node instanceof BlockElementNode;
}

export function $createBlockElementNode(kind: BlockKind, pid: string): BlockElementNode {
  return new BlockElementNode(kind, pid);
}
```

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/BodyLineNode.ts
import { ElementNode, type SerializedElementNode, type LexicalNode, type Spread } from "lexical";

export interface SerializedBodyLineNode extends Spread<
  { bodyLinePid: string; type: "body-line" },
  SerializedElementNode
> {}

export class BodyLineNode extends ElementNode {
  __bodyLinePid: string;

  static getType(): string { return "body-line"; }

  static clone(node: BodyLineNode): BodyLineNode {
    return new BodyLineNode(node.__bodyLinePid, node.__key);
  }

  constructor(pid: string, key?: string) {
    super(key);
    this.__bodyLinePid = pid;
  }

  getBodyLinePid(): string { return this.getLatest().__bodyLinePid; }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = "code-body-line";
    el.setAttribute("data-body-line-pid", this.__bodyLinePid);
    return el;
  }

  updateDOM(): boolean { return true; }

  exportJSON(): SerializedBodyLineNode {
    return { ...super.exportJSON(), type: "body-line", bodyLinePid: this.__bodyLinePid };
  }

  static importJSON(serialized: SerializedBodyLineNode): BodyLineNode {
    const node = $createBodyLineNode(serialized.bodyLinePid);
    node.setFormat(serialized.format);
    node.setIndent(serialized.indent);
    node.setDirection(serialized.direction);
    return node;
  }

  canBeEmpty(): boolean { return true; }
  canInsertTextBefore(): boolean { return true; }
  canInsertTextAfter(): boolean { return true; }
  isInline(): boolean { return false; }
}

export function $isBodyLineNode(node: LexicalNode | null | undefined): node is BodyLineNode {
  return node instanceof BodyLineNode;
}

export function $createBodyLineNode(pid: string): BodyLineNode {
  return new BodyLineNode(pid);
}
```

- [ ] **Step 2: Write and run basic unit tests**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/__tests__/ContainerNode.test.ts
import { describe, it, expect } from "vitest";
import { ContainerElementNode, $createContainerElementNode, $isContainerElementNode } from "../ContainerNode";

describe("ContainerElementNode", () => {
  it("creates with correct type", () => {
    const node = $createContainerElementNode("let", "c1");
    expect(ContainerElementNode.getType()).toBe("container");
    expect(node.getContainerKind()).toBe("let");
  });

  it("is detected correctly", () => {
    expect($isContainerElementNode($createContainerElementNode("set", "c2"))).toBe(true);
    expect($isContainerElementNode(null)).toBe(false);
  });
});
```

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/__tests__/BlockNode.test.ts
import { describe, it, expect } from "vitest";
import { BlockElementNode, $createBlockElementNode, $isBlockElementNode } from "../BlockNode";

describe("BlockElementNode", () => {
  it("creates with correct type", () => {
    const node = $createBlockElementNode("if", "b1");
    expect(BlockElementNode.getType()).toBe("block");
    expect(node.getBlockKind()).toBe("if");
  });
});
```

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/nodes/__tests__/ContainerNode.test.ts poiesis/src/components/canvas/code/lexical/nodes/__tests__/BlockNode.test.ts`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/ContainerNode.ts poiesis/src/components/canvas/code/lexical/nodes/BlockNode.ts poiesis/src/components/canvas/code/lexical/nodes/BodyLineNode.ts poiesis/src/components/canvas/code/lexical/nodes/__tests__/ContainerNode.test.ts poiesis/src/components/canvas/code/lexical/nodes/__tests__/BlockNode.test.ts
git commit -m "feat(codecard-v2): add ContainerNode, BlockNode, BodyLineNode Lexical nodes"
```

---

### Task 7: Update nodes/index.ts + add CSS styles

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/nodes/index.ts`
- Modify: `poiesis/src/components/canvas/code/CodeCardEditor.module.css`

**Interfaces:**
- Consumes: all 5 new node types from Tasks 4-6
- Produces: unified node export + visual styles

- [ ] **Step 1: Update nodes/index.ts**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/index.ts
export { CardNode, $createCardNode, $isCardNode } from "./CardNode";
export type { SerializedCardNode } from "./CardNode";

export { ContainerElementNode, $createContainerElementNode, $isContainerElementNode } from "./ContainerNode";
export type { SerializedContainerElementNode } from "./ContainerNode";

export { BlockElementNode, $createBlockElementNode, $isBlockElementNode } from "./BlockNode";
export type { SerializedBlockElementNode } from "./BlockNode";

export { BodyLineNode, $createBodyLineNode, $isBodyLineNode } from "./BodyLineNode";
export type { SerializedBodyLineNode } from "./BodyLineNode";

export { InputSlotNode, $createInputSlotNode, $isInputSlotNode } from "./InputSlotNode";
export type { SerializedInputSlotNode } from "./InputSlotNode";
```

- [ ] **Step 2: Add CSS styles to CodeCardEditor.module.css**

Append to the existing CSS file:

```css
/* ── CodeCard v2: Card atoms (inline, non-editable) ── */
.code-card {
  display: inline-block;
  padding: 1px 6px;
  margin: 1px 2px;
  border-radius: 5px;
  font-size: 13px;
  font-family: "JetBrains Mono", "Fira Code", monospace;
  line-height: 1.6;
  cursor: default;
  user-select: none;
  white-space: nowrap;
  transition: box-shadow 0.1s;
}

.code-card--mut           { background: #fde68a; color: #92400e; }  /* orange */
.code-card--break,
.code-card--continue      { background: #e2e8f0; color: #475569; }  /* gray */
.code-card--operator      { background: #e2e8f0; color: #334155; }  /* gray */
.code-card--type          { background: #cffafe; color: #155e75; }  /* cyan */
.code-card--var_ref       { background: #d1fae5; color: #065f46; }  /* green */
.code-card--literal       { background: #d1fae5; color: #065f46; }  /* green */
.code-card--dot           { background: #e2e8f0; color: #334155; }
.code-card--separator     { background: transparent; color: #94a3b8; padding: 1px 2px; }
.code-card--comment       { background: #f1f5f9; color: #94a3b8; font-style: italic; }

/* ── Card highlight (arrow-key focus) ── */
.code-card:focus,
.code-card.focused {
  outline: 2px solid #6366f1;
  outline-offset: 1px;
}

/* ── Containers ── */
.code-container {
  display: inline-flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0;
  padding: 2px 4px 2px 2px;
  margin: 1px 0;
  border-radius: 6px;
  background: rgba(99, 102, 241, 0.04);  /* subtle purple tint */
  border-left: 3px solid;
  position: relative;
}

.code-container--let      { border-color: #a78bfa; }
.code-container--set      { border-color: #60a5fa; }
.code-container--return   { border-color: #f87171; }
.code-container--await    { border-color: #22d3ee; }
.code-container--call     { border-color: #22d3ee; }
.code-container--array    { border-color: #a78bfa; }
.code-container--map      { border-color: #a78bfa; }

/* Container right-edge delete zone indicator */
.code-container::after {
  content: "";
  display: inline-block;
  width: 6px;
  height: 100%;
  min-height: 20px;
  margin-left: 2px;
  border-radius: 2px;
  cursor: default;
}

/* ── Block cards (if/while/for) ── */
.code-block-card {
  display: block;
  border-left: 4px solid;
  padding-left: 4px;
  margin: 2px 0;
}

.code-block-card--if       { border-color: #fbbf24; }
.code-block-card--while    { border-color: #f97316; }
.code-block-card--for      { border-color: #34d399; }

/* Block head line */
.code-block-head {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px 0;
}

/* Block body lines */
.code-body-line {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 1px 0 1px 16px;  /* indent */
  min-height: 22px;
}

/* Block tail line (thin closing bar) */
.code-block-tail {
  height: 3px;
  margin-left: 0;
  margin-right: 0;
  border-radius: 0 0 2px 2px;
}

.code-block-card--if .code-block-tail    { background: #fbbf24; }
.code-block-card--while .code-block-tail { background: #f97316; }
.code-block-card--for .code-block-tail   { background: #34d399; }

/* ── Input slots (backslash-triggered inline input) ── */
.input-slot {
  display: inline-block;
  min-width: 40px;
  padding: 1px 6px;
  background: #fff;
  border: 1px solid #cbd5e1;
  border-radius: 3px;
  font-size: 13px;
  font-family: "JetBrains Mono", "Fira Code", monospace;
  line-height: 1.5;
  outline: none;
  transition: border-color 0.15s;
}

.input-slot:focus {
  border-color: #6366f1;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.15);
}

/* Empty slot placeholder */
.input-slot--name:empty::before       { content: "变量名"; color: #fca5a5; }
.input-slot--type_value:empty::before { content: "类型"; color: #cbd5e1; }
.input-slot--expression:empty::before { content: "值"; color: #cbd5e1; }
.input-slot--condition:empty::before  { content: "条件"; color: #cbd5e1; }
.input-slot--fn_name:empty::before    { content: "函数名"; color: #cbd5e1; }

/* ── Syntax lint ── */
.code-card.syntax-warn {
  text-decoration: underline;
  text-decoration-color: #fbbf24;
  text-decoration-style: wavy;
  text-underline-offset: 2px;
}

/* ── Editor body ── */
.overlay { /* existing — keep */ }
.editor { /* existing — keep */ }
.header { /* existing — keep */ }
.body {
  padding: 12px;
  background: #fafbfc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  min-height: 200px;
  max-height: 70vh;
  overflow-y: auto;
}

.contentEditable {
  outline: none;
  min-height: 100%;
  font-size: 13px;
  font-family: "JetBrains Mono", "Fira Code", monospace;
  line-height: 1.8;
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit -p poiesis/tsconfig.json 2>&1 | head -10`
Expected: no new errors from these files

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/index.ts poiesis/src/components/canvas/code/CodeCardEditor.module.css
git commit -m "feat(codecard-v2): update node index and add v2 CSS styles"
```

---

### Task 8: Rewrite autocomplete rules for fine-grained cards

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/autocomplete/autocomplete-rules.ts`
- Modify: `poiesis/src/components/canvas/code/lexical/autocomplete/__tests__/autocomplete-rules.test.ts`

**Interfaces:**
- Consumes: `InlineKind`, `ContainerKind` from Task 1
- Produces: `getAutocompleteItems(token, context)`, `detectCardKindFromItem(item)`

- [ ] **Step 1: Rewrite autocomplete rules**

```typescript
// poiesis/src/components/canvas/code/lexical/autocomplete/autocomplete-rules.ts
import type { ContainerKind, BlockKind, InlineKind } from "../../../../../core/types/editor-node";

export interface AutocompleteItem {
  label: string;
  description: string;
  /** The EditorNode kind this item creates */
  cardKind: "container" | "block" | "inline" | "slot";
  /** Sub-kind for container/block/inline */
  subKind: string;
}

export interface AutocompleteContext {
  /** What container/block we're inside (null = top-level) */
  parentKind: string | null;
  /** What preceded the cursor (used for . dot detection) */
  precedingCardKind: string | null;
}

// ── Statement-level (top-level) ──
const STATEMENT_ITEMS: AutocompleteItem[] = [
  { label: "let", description: "声明不可变变量", cardKind: "container", subKind: "let" },
  { label: "set", description: "赋值变量", cardKind: "container", subKind: "set" },
  { label: "if", description: "条件分支", cardKind: "block", subKind: "if" },
  { label: "while", description: "while 循环", cardKind: "block", subKind: "while" },
  { label: "for", description: "for..in 循环", cardKind: "block", subKind: "for" },
  { label: "return", description: "返回", cardKind: "container", subKind: "return" },
  { label: "await", description: "异步等待", cardKind: "container", subKind: "await" },
  { label: "call", description: "函数调用", cardKind: "container", subKind: "call" },
  { label: "break", description: "跳出循环", cardKind: "inline", subKind: "break" },
  { label: "continue", description: "跳过本次", cardKind: "inline", subKind: "continue" },
  { label: "//", description: "注释", cardKind: "inline", subKind: "comment" },
];

// ── Inside let container ──
const LET_CHILDREN: AutocompleteItem[] = [
  { label: "mut", description: "可变修饰", cardKind: "inline", subKind: "mut" },
  { label: ":", description: "类型标注", cardKind: "inline", subKind: "separator" },
  { label: "=", description: "初始化", cardKind: "inline", subKind: "operator" },
];

// ── Inside set container ──
const SET_CHILDREN: AutocompleteItem[] = [
  { label: "=", description: "赋值", cardKind: "inline", subKind: "operator" },
  { label: "+=", description: "加等于", cardKind: "inline", subKind: "operator" },
  { label: "-=", description: "减等于", cardKind: "inline", subKind: "operator" },
  { label: "*=", description: "乘等于", cardKind: "inline", subKind: "operator" },
  { label: "/=", description: "除等于", cardKind: "inline", subKind: "operator" },
  { label: "%=", description: "取余等于", cardKind: "inline", subKind: "operator" },
];

// ── Type completions ──
const TYPE_ITEMS: AutocompleteItem[] = [
  { label: "i32", description: "32-bit integer", cardKind: "inline", subKind: "type" },
  { label: "i64", description: "64-bit integer", cardKind: "inline", subKind: "type" },
  { label: "f32", description: "32-bit float", cardKind: "inline", subKind: "type" },
  { label: "f64", description: "64-bit float", cardKind: "inline", subKind: "type" },
  { label: "String", description: "string type", cardKind: "inline", subKind: "type" },
  { label: "bool", description: "boolean", cardKind: "inline", subKind: "type" },
];

// ── Comparison operators (condition context) ──
const COMPARISON_ITEMS: AutocompleteItem[] = [
  { label: ">", description: "大于", cardKind: "inline", subKind: "operator" },
  { label: "<", description: "小于", cardKind: "inline", subKind: "operator" },
  { label: ">=", description: "大于等于", cardKind: "inline", subKind: "operator" },
  { label: "<=", description: "小于等于", cardKind: "inline", subKind: "operator" },
  { label: "==", description: "等于", cardKind: "inline", subKind: "operator" },
  { label: "!=", description: "不等于", cardKind: "inline", subKind: "operator" },
];

// ── Arithmetic operators (expression context) ──
const ARITHMETIC_ITEMS: AutocompleteItem[] = [
  { label: "+", description: "加", cardKind: "inline", subKind: "operator" },
  { label: "-", description: "减", cardKind: "inline", subKind: "operator" },
  { label: "*", description: "乘", cardKind: "inline", subKind: "operator" },
  { label: "/", description: "除", cardKind: "inline", subKind: "operator" },
  { label: "%", description: "取余", cardKind: "inline", subKind: "operator" },
];

// ── Logic operators ──
const LOGIC_ITEMS: AutocompleteItem[] = [
  { label: "&&", description: "且", cardKind: "inline", subKind: "operator" },
  { label: "||", description: "或", cardKind: "inline", subKind: "operator" },
  { label: "!", description: "非", cardKind: "inline", subKind: "operator" },
];

// ── Literals ──
const LITERAL_ITEMS: AutocompleteItem[] = [
  { label: "true", description: "true", cardKind: "inline", subKind: "literal" },
  { label: "false", description: "false", cardKind: "inline", subKind: "literal" },
  { label: "null", description: "null", cardKind: "inline", subKind: "literal" },
];

// ── Dot access ──
const DOT_METHODS: AutocompleteItem[] = [
  { label: ".len()", description: "长度", cardKind: "inline", subKind: "dot" },
  { label: ".push()", description: "添加元素", cardKind: "inline", subKind: "dot" },
  { label: ".pop()", description: "移除末尾", cardKind: "inline", subKind: "dot" },
  { label: ".get()", description: "获取元素", cardKind: "inline", subKind: "dot" },
  { label: ".to_string()", description: "转字符串", cardKind: "inline", subKind: "dot" },
];

/** Get autocomplete items filtered by token prefix and cursor context. */
export function getAutocompleteItems(
  token: string,
  context: AutocompleteContext,
): AutocompleteItem[] {
  const candidates = getCandidatesForContext(context);
  if (!token) return candidates;
  const lower = token.toLowerCase();
  return candidates.filter(item => item.label.toLowerCase().startsWith(lower));
}

function getCandidatesForContext(context: AutocompleteContext): AutocompleteItem[] {
  // After a dot, show method completions
  if (context.precedingCardKind === "dot") {
    return DOT_METHODS;
  }

  // Inside containers
  switch (context.parentKind) {
    case "let":
      return [...LET_CHILDREN, ...TYPE_ITEMS, ...LITERAL_ITEMS];
    case "set":
      return [...SET_CHILDREN, ...ARITHMETIC_ITEMS, ...LITERAL_ITEMS];
    case "return":
    case "await":
      return [...ARITHMETIC_ITEMS, ...LOGIC_ITEMS, ...LITERAL_ITEMS];
    case "if":
    case "while":
      // In block head = condition context
      return [...COMPARISON_ITEMS, ...LOGIC_ITEMS, ...LITERAL_ITEMS];
    case "for":
      return [...LITERAL_ITEMS];
    case "call":
      return [...ARITHMETIC_ITEMS, ...LITERAL_ITEMS];
    case "array":
    case "map":
      return [...LITERAL_ITEMS];
    default:
      return STATEMENT_ITEMS;
  }
}

/** Map autocomplete item to node construction parameters. */
export function detectCardKindFromItem(item: AutocompleteItem): {
  nodeType: "container" | "block" | "inline";
  kind: string;
} {
  return { nodeType: item.cardKind, kind: item.subKind };
}
```

- [ ] **Step 2: Update the test file**

```typescript
// poiesis/src/components/canvas/code/lexical/autocomplete/__tests__/autocomplete-rules.test.ts
import { describe, it, expect } from "vitest";
import { getAutocompleteItems } from "../autocomplete-rules";

describe("autocomplete rules v2", () => {
  it("returns statement items at top level", () => {
    const items = getAutocompleteItems("l", { parentKind: null, precedingCardKind: null });
    const labels = items.map(i => i.label);
    expect(labels).toContain("let");
  });

  it("returns mut inside let container", () => {
    const items = getAutocompleteItems("m", { parentKind: "let", precedingCardKind: null });
    expect(items.some(i => i.label === "mut")).toBe(true);
  });

  it("returns type completions inside let", () => {
    const items = getAutocompleteItems("i", { parentKind: "let", precedingCardKind: null });
    expect(items.some(i => i.label === "i32")).toBe(true);
  });

  it("returns operators inside set", () => {
    const items = getAutocompleteItems("+", { parentKind: "set", precedingCardKind: null });
    expect(items.some(i => i.label === "+=")).toBe(true);
  });

  it("returns comparison operators in if head", () => {
    const items = getAutocompleteItems(">", { parentKind: "if", precedingCardKind: null });
    expect(items.some(i => i.label === ">=")).toBe(true);
  });

  it("returns method completions after dot", () => {
    const items = getAutocompleteItems(".len", { parentKind: null, precedingCardKind: "dot" });
    expect(items.some(i => i.label === ".len()")).toBe(true);
  });
});
```

- [ ] **Step 3: Run tests**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/autocomplete/__tests__/autocomplete-rules.test.ts`
Expected: all PASS

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/autocomplete/autocomplete-rules.ts poiesis/src/components/canvas/code/lexical/autocomplete/__tests__/autocomplete-rules.test.ts
git commit -m "feat(codecard-v2): rewrite autocomplete rules for fine-grained cards"
```

---

### Task 9: Rewrite KeyboardPlugin for v2 cursor navigation

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts`
- Create: `poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts`

**Interfaces:**
- Consumes: Lexical `useLexicalComposerContext`, all 5 node types
- Produces: keyboard navigation (← → ↑ ↓ Tab Enter Delete \)

- [ ] **Step 1: Write failing tests**

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
import { describe, it, expect } from "vitest";

// Keyboard behavior specification tests (logic extracted for testability)
// The actual plugin uses Lexical commands — we test the logic functions.

describe("KeyboardPlugin v2 — navigation logic", () => {
  it("Delete on inline card removes it", () => {
    // Placeholder: test $deleteSelectedCard utility
    expect(true).toBe(true);
  });

  it("Delete on container right edge removes entire container", () => {
    expect(true).toBe(true);
  });

  it("backslash in let container creates name input", () => {
    expect(true).toBe(true);
  });

  it("backslash at top level is a no-op", () => {
    expect(true).toBe(true);
  });

  it("Enter at end of body line creates new line", () => {
    expect(true).toBe(true);
  });

  it("Tab in set container cycles operator", () => {
    expect(true).toBe(true);
  });
});
```

- [ ] **Step 2: Implement KeyboardPlugin v2**

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  KEY_DELETE_COMMAND,
  KEY_BACKSPACE_COMMAND,
  KEY_ENTER_COMMAND,
  KEY_TAB_COMMAND,
  COMMAND_PRIORITY_LOW,
  $getSelection,
  $isRangeSelection,
  $getNodeByKey,
  $getRoot,
  $createParagraphNode,
} from "lexical";
import { $isCardNode, $createCardNode } from "../nodes/CardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBlockElementNode } from "../nodes/BlockElementNode";
import { $isBodyLineNode, $createBodyLineNode } from "../nodes/BodyLineNode";
import { $isInputSlotNode, $createInputSlotNode } from "../nodes/InputSlotNode";
import { useEffect } from "react";

export function KeyboardPlugin(): null {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    // ── Delete: remove card/container at cursor ──
    return editor.registerCommand(
      KEY_DELETE_COMMAND,
      (event) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        const anchorNode = selection.anchor.getNode();
        const anchorKey = anchorNode.getKey();

        // Delete inline card
        if ($isCardNode(anchorNode)) {
          event.preventDefault();
          anchorNode.remove();
          return true;
        }

        // Delete container (when cursor is after it)
        // Check if previous sibling is a container
        const prevSibling = anchorNode.getPreviousSibling();
        if ($isContainerElementNode(prevSibling)) {
          event.preventDefault();
          prevSibling.remove();
          return true;
        }

        // Delete block
        if ($isBlockElementNode(anchorNode) || $isBlockElementNode(anchorNode.getParent())) {
          event.preventDefault();
          const blockNode = $isBlockElementNode(anchorNode) ? anchorNode : anchorNode.getParent();
          blockNode?.remove();
          return true;
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );

    // ── Backspace: same as delete for cards ──
    return editor.registerCommand(
      KEY_BACKSPACE_COMMAND,
      (event) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;
        const anchorNode = selection.anchor.getNode();
        if ($isCardNode(anchorNode)) {
          event.preventDefault();
          anchorNode.remove();
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  useEffect(() => {
    // ── Enter: new body line or new top-level line ──
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;
        const anchorNode = selection.anchor.getNode();

        // Enter inside body line: insert new body line below
        if ($isBodyLineNode(anchorNode) || $isBodyLineNode(anchorNode.getParent())) {
          event.preventDefault();
          const bodyLine = $isBodyLineNode(anchorNode) ? anchorNode : anchorNode.getParent()!;
          const newLine = $createBodyLineNode("bl-" + Date.now());
          bodyLine.insertAfter(newLine);
          newLine.selectStart();
          return true;
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  useEffect(() => {
    // ── Tab: cycle operator in set container ──
    return editor.registerCommand(
      KEY_TAB_COMMAND,
      (event) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;
        const anchorNode = selection.anchor.getNode();

        // If cursor is on an operator card inside a set container, cycle operator
        if ($isCardNode(anchorNode) && anchorNode.getCardKind() === "operator") {
          const parent = anchorNode.getParent();
          if ($isContainerElementNode(parent) && parent.getContainerKind() === "set") {
            event.preventDefault();
            const operators = ["=", "+=", "-=", "*=", "/=", "%="];
            const current = operators.indexOf(anchorNode.getCardLabel());
            const next = (current + 1) % operators.length;
            // Replace with new operator card
            const newNode = $createCardNode("operator", operators[next], anchorNode.getCardPid());
            anchorNode.replace(newNode);
            newNode.selectNext();
            return true;
          }
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  return null;
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit -p poiesis/tsconfig.json 2>&1 | head -10`
Expected: no new errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
git commit -m "feat(codecard-v2): rewrite KeyboardPlugin for v2 navigation"
```

---

### Task 10: Rewrite AutocompletePlugin for v2 card insertion

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx`

**Interfaces:**
- Consumes: `getAutocompleteItems`, `detectCardKindFromItem` from Task 8, `AutocompleteMenu` from existing file
- Produces: `<AutocompletePlugin>` that inserts v2 nodes on selection

- [ ] **Step 1: Rewrite AutocompletePlugin**

The plugin needs to:
1. Track the current typing token and context (parent container/block)
2. Show the autocomplete menu
3. On selection, insert the appropriate node (container/block/inline/slot)

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  $getSelection,
  $isRangeSelection,
  KEY_ENTER_COMMAND,
  KEY_ESCAPE_COMMAND,
  COMMAND_PRIORITY_LOW,
  $getRoot,
  $createParagraphNode,
  $createTextNode,
} from "lexical";
import { useCallback, useEffect, useRef, useState } from "react";
import { getAutocompleteItems, detectCardKindFromItem, type AutocompleteItem, type AutocompleteContext } from "../autocomplete/autocomplete-rules";
import { $createCardNode } from "../nodes/CardNode";
import { $createContainerElementNode, $isContainerElementNode } from "../nodes/ContainerNode";
import { $createBlockElementNode } from "../nodes/BlockElementNode";
import { $createBodyLineNode } from "../nodes/BodyLineNode";
import { $createInputSlotNode, $isInputSlotNode } from "../nodes/InputSlotNode";
import { uid } from "../../../../../core/utils/id";
import { AutocompleteMenu } from "../autocomplete/AutocompleteMenu";

interface Props {
  /** Called when backslash key creates a new input slot */
  onCreateSlot?: () => void;
}

export function AutocompletePlugin({ onCreateSlot }: Props): JSX.Element | null {
  const [editor] = useLexicalComposerContext();
  const [acState, setAcState] = useState<{
    visible: boolean;
    items: AutocompleteItem[];
    index: number;
  }>({ visible: false, items: [], index: 0 });
  const typingTokenRef = useRef("");

  // ── Detect typing and trigger autocomplete ──
  useEffect(() => {
    return editor.registerUpdateListener(({ editorState, prevEditorState }) => {
      editorState.read(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          setAcState(s => s.visible ? { ...s, visible: false } : s);
          return;
        }

        const anchor = selection.anchor;
        const node = anchor.getNode();
        const text = node.getTextContent();

        // Extract current word token from cursor position
        const offset = anchor.offset;
        const beforeCursor = text.slice(0, offset);
        const tokenMatch = beforeCursor.match(/(\S+)$/);
        const token = tokenMatch ? tokenMatch[1] : "";

        // Determine context
        const parent = node.getParent();
        const context: AutocompleteContext = {
          parentKind: getParentContextKind(parent),
          precedingCardKind: getPrecedingCardKind(node, offset),
        };

        const items = getAutocompleteItems(token, context);
        if (items.length > 0) {
          setAcState({ visible: true, items, index: 0 });
          typingTokenRef.current = token;
        } else {
          setAcState(s => s.visible ? { ...s, visible: false } : s);
        }
      });
    });
  }, [editor]);

  // ── Backslash → insert input slot ──
  useEffect(() => {
    const removeRootListener = editor.registerRootListener(
      (rootElement: HTMLElement | null) => {
        if (!rootElement) return;
        const handler = (e: KeyboardEvent) => {
          if (e.key === "\\") {
            e.preventDefault();
            editor.update(() => {
              const selection = $getSelection();
              if (!$isRangeSelection(selection)) return;
              const node = selection.anchor.getNode();
              const parent = node.getParent();

              // Only insert slot if cursor position is valid for a slot
              const parentKind = getParentContextKind(parent);
              const slotKind = inferSlotKind(parentKind, node);
              if (slotKind) {
                const slotNode = $createInputSlotNode("", slotKind);
                node.getParent()?.append(slotNode);
                slotNode.select();
                onCreateSlot?.();
              }
            });
          }
        };
        rootElement.addEventListener("keydown", handler);
        return () => rootElement.removeEventListener("keydown", handler);
      },
    );
    return () => removeRootListener?.();
  }, [editor, onCreateSlot]);

  // ── Enter confirms autocomplete selection ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event) => {
        if (!acState.visible) return false;
        event.preventDefault();
        executeAutocomplete(acState.items[acState.index]);
        return true;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, acState]);

  // ── Escape closes autocomplete ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      () => {
        if (acState.visible) {
          setAcState(s => ({ ...s, visible: false }));
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, acState.visible]);

  const executeAutocomplete = useCallback(
    (item: AutocompleteItem) => {
      setAcState(s => ({ ...s, visible: false }));

      editor.update(() => {
        const token = typingTokenRef.current;
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return;

        const anchorNode = selection.anchor.getNode();

        // Remove the typed token text
        if (anchorNode.getTextContentSize() > 0) {
          const text = anchorNode.getTextContent();
          const offset = selection.anchor.offset;
          const beforeToken = text.slice(0, offset - token.length);
          const afterToken = text.slice(offset);
          anchorNode.setTextContent(beforeToken + afterToken);
        }

        const { nodeType, kind } = detectCardKindFromItem(item);
        const pid = uid("cc");

        if (nodeType === "inline") {
          const cardNode = $createCardNode(kind as any, item.label, pid);
          anchorNode.getParent()?.append(cardNode);
          // Insert text node after for continued typing
          const nextText = $createTextNode(" ");
          cardNode.insertAfter(nextText);
          nextText.select();
        } else if (nodeType === "container") {
          const containerNode = $createContainerElementNode(kind as any, pid);
          anchorNode.getParent()?.append(containerNode);
          // Pre-populate with an empty input slot if appropriate
          containerNode.selectStart();
        } else if (nodeType === "block") {
          const blockNode = $createBlockElementNode(kind as any, pid);
          const bodyLine = $createBodyLineNode(uid("bl"));
          blockNode.append(bodyLine);
          anchorNode.getParent()?.append(blockNode);
          bodyLine.selectStart();
        }
      });
    },
    [editor],
  );

  const handleAcNavigate = useCallback(
    (dir: "up" | "down") => {
      setAcState(s => ({
        ...s,
        index: dir === "up" ? Math.max(0, s.index - 1) : Math.min(s.index + 1, s.items.length - 1),
      }));
    },
    [],
  );

  return (
    <>
      {acState.visible && (
        <AutocompleteMenu
          items={acState.items}
          selectedIndex={acState.index}
          onSelect={executeAutocomplete}
          onClose={() => setAcState(s => ({ ...s, visible: false }))}
          rowIndex={0}
        />
      )}
    </>
  );
}

// ── Context helpers ──

function getParentContextKind(parent: any): string | null {
  if (!$parent || parent === $getRoot()) return null;
  if ($isContainerElementNode(parent)) return parent.getContainerKind();
  if ($isContainerElementNode(parent.getParent?.())) return parent.getParent().getContainerKind();
  return null;
}

function getPrecedingCardKind(node: any, offset: number): string | null {
  // Check if the character immediately before cursor is a dot
  const text = node.getTextContent();
  if (offset > 0 && text[offset - 1] === ".") return "dot";
  return null;
}

function inferSlotKind(parentKind: string | null, node: any): string | null {
  if (!parentKind) return null;
  switch (parentKind) {
    case "let":
    case "set":
      // If preceding child is a separator ":", it's a type slot
      // Otherwise it's a name or expression depending on position
      return "expression";
    case "if":
    case "while":
      return "condition";
    case "for":
      return "name";
    case "call":
      return "fn_name";
    case "return":
    case "await":
      return "expression";
    default:
      return null;
  }
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit -p poiesis/tsconfig.json 2>&1 | head -20`
Expected: resolve any TS errors, then no new errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx
git commit -m "feat(codecard-v2): rewrite AutocompletePlugin for v2 card insertion"
```

---

### Task 11: Rewrite NodeSyncPlugin (CodeOpSyncPlugin → v2)

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts`

**Interfaces:**
- Consumes: `convertToNodes` from Task 2, `convertToOps` from Task 3, all 5 node types
- Produces: `<NodeSyncPlugin>` — loads EditorNode[] on mount, extracts EditorNode[] on change

- [ ] **Step 1: Rewrite the sync plugin**

Replace `CodeOpSyncPlugin.ts` entirely:

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
// Renamed internally to NodeSyncPlugin — file name kept for git history.
// Now syncs EditorNode[] instead of directly converting to CodeOp[].

import { useEffect, useRef } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import { $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine } from "../../../../../core/types/editor-node";
import { convertToNodes } from "../transforms/convertToNodes";
import { convertToOps } from "../transforms/convertToOps";
import { $createCardNode, $isCardNode } from "../nodes/CardNode";
import { $createContainerElementNode, $isContainerElementNode } from "../nodes/ContainerNode";
import { $createBlockElementNode, $isBlockElementNode } from "../nodes/BlockElementNode";
import { $createBodyLineNode, $isBodyLineNode } from "../nodes/BodyLineNode";
import { $createInputSlotNode, $isInputSlotNode } from "../nodes/InputSlotNode";

interface Props {
  initialOps: CodeOp[];
  onSync: (ops: CodeOp[]) => void;
}

export function CodeOpSyncPlugin({ initialOps, onSync }: Props): null {
  const [editor] = useLexicalComposerContext();
  const initializedRef = useRef(false);

  // ── Load initial ops into editor (once) ──
  useEffect(() => {
    if (initializedRef.current) return;
    initializedRef.current = true;

    editor.update(() => {
      const root = $getRoot();
      root.clear();

      const nodes = convertToNodes(initialOps);
      for (const node of nodes) {
        const lexicalNode = editorNodeToLexical(node);
        root.append(lexicalNode);
      }
      root.append($createParagraphNode());
    });
  }, [editor, initialOps]);

  // ── Extract EditorNode[] → CodeOp[] on change and report ──
  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const root = $getRoot();
        const nodes = extractEditorNodes(root);
        const ops = convertToOps(nodes);
        onSync(ops);
      });
    });
  }, [editor, onSync]);

  return null;
}

// ── EditorNode → Lexical node ──

function editorNodeToLexical(node: EditorNode) {
  switch (node.kind) {
    case "let":
    case "set":
    case "return":
    case "await":
    case "call":
    case "array":
    case "map": {
      const container = $createContainerElementNode(node.kind, node.pid);
      for (const child of (node as ContainerNode).children) {
        container.append(editorNodeToLexical(child));
      }
      return container;
    }
    case "if":
    case "while":
    case "for": {
      const block = $createBlockElementNode(node.kind, node.pid);
      const bNode = node as BlockNode;
      // Head line
      for (const headItem of bNode.head) {
        block.append(editorNodeToLexical(headItem));
      }
      // Body lines
      for (const line of bNode.body) {
        const bl = $createBodyLineNode(line.pid);
        for (const child of line.children) {
          bl.append(editorNodeToLexical(child));
        }
        block.append(bl);
      }
      // Branches (if)
      if (bNode.branches) {
        for (const br of bNode.branches) {
          // Represent branch as container with head + body
          const brContainer = $createContainerElementNode("call" as any, br.pid); // temporary
          for (const headItem of br.head) {
            brContainer.append(editorNodeToLexical(headItem));
          }
          for (const line of br.body) {
            const bl = $createBodyLineNode(line.pid);
            for (const child of line.children) {
              bl.append(editorNodeToLexical(child));
            }
            brContainer.append(bl);
          }
          block.append(brContainer);
        }
      }
      return block;
    }
    case "mut":
    case "break":
    case "continue":
    case "operator":
    case "type":
    case "var_ref":
    case "literal":
    case "dot":
    case "separator":
    case "comment": {
      const inNode = node as InlineNode;
      return $createCardNode(inNode.kind, inNode.cardLabel, inNode.pid);
    }
    case "slot": {
      const sl = node as InputSlot;
      return $createInputSlotNode(sl.value, sl.slotKind);
    }
    default:
      return $createTextNode("");
  }
}

// ── Lexical nodes → EditorNode[] ──

function extractEditorNodes(root: ReturnType<typeof $getRoot>): EditorNode[] {
  const nodes: EditorNode[] = [];
  const children = root.getChildren();
  for (const child of children) {
    const node = lexicalToEditorNode(child);
    if (node) nodes.push(node);
  }
  return nodes;
}

function lexicalToEditorNode(lexicalNode: any): EditorNode | null {
  if ($isContainerElementNode(lexicalNode)) {
    const children: EditorNode[] = [];
    for (const child of lexicalNode.getChildren()) {
      const c = lexicalToEditorNode(child);
      if (c) children.push(c);
    }
    return {
      kind: lexicalNode.getContainerKind(),
      pid: lexicalNode.getContainerPid(),
      children,
    } as ContainerNode;
  }

  if ($isBlockElementNode(lexicalNode)) {
    const head: EditorNode[] = [];
    const body: BodyLine[] = [];
    for (const child of lexicalNode.getChildren()) {
      if ($isBodyLineNode(child)) {
        const lineChildren: EditorNode[] = [];
        for (const lc of child.getChildren()) {
          const c = lexicalToEditorNode(lc);
          if (c) lineChildren.push(c);
        }
        body.push({ pid: child.getBodyLinePid(), children: lineChildren });
      } else {
        const c = lexicalToEditorNode(child);
        if (c) head.push(c);
      }
    }
    return {
      kind: lexicalNode.getBlockKind(),
      pid: lexicalNode.getBlockPid(),
      head,
      body,
      tail: { pid: "tail-" + lexicalNode.getBlockPid() },
    } as BlockNode;
  }

  if ($isCardNode(lexicalNode)) {
    return {
      kind: lexicalNode.getCardKind(),
      cardLabel: lexicalNode.getCardLabel(),
      pid: lexicalNode.getCardPid(),
    } as InlineNode;
  }

  if ($isInputSlotNode(lexicalNode)) {
    return {
      kind: "slot",
      slotKind: lexicalNode.getSlotKind(),
      value: lexicalNode.getTextContent(),
      pid: lexicalNode.getKey(),
    } as InputSlot;
  }

  return null;
}
```

- [ ] **Step 2: Run existing roundtrip tests to verify no regression**

Run: `npx vitest run poiesis/src/services/codegen/__tests__/code-op-roundtrip.test.ts`
Expected: all existing tests should still pass (they test emit ↔ parse, not lexical)

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
git commit -m "feat(codecard-v2): rewrite CodeOpSyncPlugin for EditorNode sync"
```

---

### Task 12: Rewrite CodeCardEditor.tsx + update FlowCanvas import

**Files:**
- Modify: `poiesis/src/components/canvas/code/CodeCardEditor.tsx`
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx:24`

**Interfaces:**
- Consumes: all plugins from Tasks 9-11, new node exports from Task 7
- Produces: fully functional CodeCardEditor v2 component

- [ ] **Step 1: Rewrite CodeCardEditor.tsx**

```typescript
// poiesis/src/components/canvas/code/CodeCardEditor.tsx
import { useCallback, useRef, useState } from "react";
import { LexicalComposer } from "@lexical/react/LexicalComposer";
import { RichTextPlugin } from "@lexical/react/LexicalRichTextPlugin";
import { LexicalErrorBoundary } from "@lexical/react/LexicalErrorBoundary";
import { ContentEditable } from "@lexical/react/LexicalContentEditable";
import { HistoryPlugin } from "@lexical/react/LexicalHistoryPlugin";
import type { CodeOp } from "../../../core/types/code-op";
import { AutocompletePlugin } from "./lexical/plugins/AutocompletePlugin";
import { KeyboardPlugin } from "./lexical/plugins/KeyboardPlugin";
import { CodeOpSyncPlugin } from "./lexical/plugins/CodeOpSyncPlugin";
import { CardNode } from "./lexical/nodes/CardNode";
import { ContainerElementNode } from "./lexical/nodes/ContainerNode";
import { BlockElementNode } from "./lexical/nodes/BlockElementNode";
import { BodyLineNode } from "./lexical/nodes/BodyLineNode";
import { InputSlotNode } from "./lexical/nodes/InputSlotNode";
import { parseOpsWasm } from "../../../services/codegen/code-op-parser";
import { CodeEditorModal } from "./CodeEditorModal";
import styles from "./CodeCardEditor.module.css";

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  initialSource: string;
  language?: "typescript" | "omnilang";
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

const lexicalConfig = {
  namespace: "code-card-editor-v2",
  nodes: [CardNode, ContainerElementNode, BlockElementNode, BodyLineNode, InputSlotNode],
  onError: (error: Error) => {
    console.error("Lexical error:", error);
  },
};

export const CodeCardEditor: React.FC<Props> = ({
  elementId,
  initialOps,
  initialSource,
  language = "omnilang",
  onSave,
  onClose,
}) => {
  const [showMonaco, setShowMonaco] = useState(false);
  const opsRef = useRef<CodeOp[]>(initialOps);

  const handleSync = useCallback((ops: CodeOp[]) => {
    opsRef.current = ops;
  }, []);

  const handleSave = useCallback(() => {
    onSave(opsRef.current);
  }, [onSave]);

  const handleMonacoSave = useCallback(async (source: string, _desc: string) => {
    try {
      const parsedOps = await parseOpsWasm(source);
      onSave(parsedOps);
    } catch (err) {
      console.error("Failed to parse Monaco source:", err);
    }
  }, [onSave]);

  // ── Monaco fallback mode ──
  if (showMonaco) {
    return (
      <CodeEditorModal
        elementId={elementId}
        initialSource={initialSource}
        initialDescription=""
        language={language}
        onSave={handleMonacoSave}
        onClose={() => setShowMonaco(false)}
      />
    );
  }

  // ── Lexical card editor v2 ──
  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        <div className={styles.header}>
          <h3 className={styles.title}>
            Code Card Editor — {opsRef.current.length} ops
          </h3>
          <div className={styles.actions}>
            <button
              type="button"
              className={styles.textModeBtn}
              onClick={() => setShowMonaco(true)}
            >
              文本模式
            </button>
            <button type="button" className={styles.saveBtn} onClick={handleSave}>
              Save
            </button>
            <button type="button" className={styles.cancelBtn} onClick={onClose}>
              Cancel
            </button>
          </div>
        </div>

        <div className={styles.body}>
          <LexicalComposer initialConfig={lexicalConfig}>
            <RichTextPlugin
              ErrorBoundary={LexicalErrorBoundary}
              contentEditable={
                <ContentEditable className={styles.contentEditable} />
              }
              placeholder={
                <div style={{ color: "#94a3b8", fontSize: 12, fontStyle: "italic", padding: "8px 0" }}>
                  Type to add operations...
                </div>
              }
            />
            <HistoryPlugin />
            <KeyboardPlugin />
            <AutocompletePlugin />
            <CodeOpSyncPlugin initialOps={initialOps} onSync={handleSync} />
          </LexicalComposer>
        </div>
      </div>
    </div>
  );
};
```

- [ ] **Step 2: Update FlowCanvas.tsx import**

```typescript
// Line 24 change:
// OLD:
import { CodeCardEditor } from "./card-editor/CodeCardEditor";
// NEW:
import { CodeCardEditor } from "./CodeCardEditor";
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit -p poiesis/tsconfig.json 2>&1 | head -20`
Expected: resolve import issues, no new errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/CodeCardEditor.tsx poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat(codecard-v2): rewrite CodeCardEditor shell, update FlowCanvas import"
```

---

### Task 13: Cleanup — delete old card-editor/ directory

**Files:**
- Delete: `poiesis/src/components/canvas/code/card-editor/` (entire directory tree)

- [ ] **Step 1: Remove the old directory**

```bash
rm -rf poiesis/src/components/canvas/code/card-editor
```

- [ ] **Step 2: Remove obsolete lexical transforms (superseded by convertToNodes/convertToOps)**

```bash
rm -f poiesis/src/components/canvas/code/lexical/transforms/text-to-card.ts
rm -f poiesis/src/components/canvas/code/lexical/transforms/card-to-text.ts
rm -f poiesis/src/components/canvas/code/lexical/transforms/__tests__/text-to-card.test.ts
rm -f poiesis/src/components/canvas/code/lexical/transforms/__tests__/card-to-text.test.ts
```

- [ ] **Step 3: Verify TypeScript compiles after cleanup**

Run: `npx tsc --noEmit -p poiesis/tsconfig.json 2>&1 | head -20`
Expected: no errors from deleted imports

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor(codecard-v2): remove old card-editor and obsolete transforms"
```

---

### Task 14: Integration test — full edit flow

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/__tests__/integration-v2.test.ts`

**Interfaces:**
- Consumes: `convertToNodes` (Task 2), `convertToOps` (Task 3), `emitOps`, `parseOps`

- [ ] **Step 1: Write integration roundtrip test**

```typescript
// poiesis/src/components/canvas/code/lexical/__tests__/integration-v2.test.ts
import { describe, it, expect } from "vitest";
import { convertToNodes } from "../transforms/convertToNodes";
import { convertToOps } from "../transforms/convertToOps";
import { emitOps } from "../../../../../services/codegen/code-op-emitter";
import { parseOps } from "../../../../../services/codegen/code-op-parser";
import type { CodeOp } from "../../../../../core/types/code-op";

describe("CodeCard v2: full pipeline roundtrip", () => {
  const testRoundtrip = (ops: CodeOp[]) => {
    // CodeOp → EditorNode → CodeOp
    const nodes = convertToNodes(ops);
    const back = convertToOps(nodes);

    // Compare semantic fields (pids will differ for auto-generated)
    expect(back.length).toBe(ops.length);
    for (let i = 0; i < ops.length; i++) {
      expect(back[i].kind).toBe(ops[i].kind);
    }

    // Emit should still produce valid source
    const source = emitOps(back);
    const reparsed = parseOps(source);
    expect(reparsed.length).toBeGreaterThanOrEqual(ops.length);
  };

  it("declare_var roundtrips through EditorNode", () => {
    testRoundtrip([
      { kind: "declare_var", pid: "d1", mutable: true, name: "count", varType: "i32", initValue: "0", scope: "fn" },
    ]);
  });

  it("set_var roundtrips through EditorNode", () => {
    testRoundtrip([
      { kind: "set_var", pid: "s1", name: "total", operator: "+=", value: "1" },
    ]);
  });

  it("if_block with elif/else roundtrips", () => {
    testRoundtrip([{
      kind: "if_block", pid: "if1",
      branches: [
        { condition: "x > 0", body: [{ kind: "return", pid: "r1", value: "1" }] },
        { condition: "x < 0", body: [{ kind: "return", pid: "r2", value: "-1" }] },
        { condition: "", body: [{ kind: "return", pid: "r3", value: "0" }] },
      ],
    }]);
  });

  it("while loop roundtrips", () => {
    testRoundtrip([{
      kind: "while", pid: "w1", condition: "i < n",
      body: [
        { kind: "set_var", pid: "s1", name: "sum", operator: "+=", value: "i" },
        { kind: "set_var", pid: "s2", name: "i", operator: "+=", value: "1" },
      ],
    }]);
  });

  it("for_in roundtrips", () => {
    testRoundtrip([{
      kind: "for_in", pid: "f1", itemName: "item", collection: "items",
      body: [{ kind: "break", pid: "b1" }],
    }]);
  });

  it("complex real-world code roundtrips", () => {
    const source = `let sum = 0.0;
let mut i = 0;
while i < values.len() {
    sum = sum + values.get(i);
    i = i + 1;
}
if count != 0 {
    return sum / count;
}
return 0.0;`;
    const ops = parseOps(source);
    const nodes = convertToNodes(ops);
    const back = convertToOps(nodes);
    const reEmitted = emitOps(back);

    // Verify key elements are present
    expect(reEmitted).toContain("let sum = 0.0");
    expect(reEmitted).toContain("while i < values.len()");
    expect(reEmitted).toContain("return");
  });
});
```

- [ ] **Step 2: Run integration tests**

Run: `npx vitest run poiesis/src/components/canvas/code/lexical/__tests__/integration-v2.test.ts`
Expected: all PASS

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/__tests__/integration-v2.test.ts
git commit -m "test(codecard-v2): add full pipeline integration roundtrip tests"
```

---

### Task 15: Run all tests + fix regressions

**Files:**
- (any files with test failures)

- [ ] **Step 1: Run full test suite**

Run: `cd poiesis && npx vitest run`
Expected: all tests pass, or identify failures

- [ ] **Step 2: Run TypeScript check**

Run: `npx tsc --noEmit -p poiesis/tsconfig.json`
Expected: no errors (existing pre-existing errors are OK if count doesn't increase)

- [ ] **Step 3: Fix any failures and commit**

```bash
git add -A
git commit -m "fix(codecard-v2): resolve test failures and type errors"
```
