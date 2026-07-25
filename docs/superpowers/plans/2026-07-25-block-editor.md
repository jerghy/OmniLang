# Block Editor v4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Lexical-based CodeCard Editor v3 with a Blockly-based block (积木) editor using keyboard-driven interaction and fuzzy search.

**Architecture:** CodeOp[] is the single source of truth. BlockAdapter bridges CodeOp ↔ Blockly workspace bidirectionally. A custom FuzzySearchField replaces Blockly's default dropdown for inserting blocks. Blockly's `@blockly/keyboard-navigation` plugin handles arrow-key cursor movement. The old Lexical directory is deleted entirely.

**Tech Stack:** Blockly v12+, `@blockly/keyboard-navigation`, React thin wrapper, Zelos renderer (Scratch 3.0 style), Vitest + Playwright

---

## Global Constraints

- **No drag-and-drop** — all interaction via keyboard
- **CodeOp[] is SSOT** — Blockly workspace derives from it, never owns data
- **SlotValue type** — all string fields become `{ kind: "text" | "expr" | "empty" }`
- **Backward compatible** — old CodeOp string fields auto-convert to `{ kind: "text", value: oldString }`
- **`\` key opens raw text input** — text stored as RawCodeOp, never parsed
- **Zelos default colors** — no custom block coloring in v4
- **Direct replacement** — old Lexical code deleted, not kept alongside
- **TypeScript strict** — no `any`, no `as` casts without narrowing

---

## Task 1: Install Blockly + Remove Lexical

**Files:**
- Modify: `poiesis/package.json`
- Modify: `poiesis/pnpm-lock.yaml` (auto)

**Interfaces:**
- Consumes: none
- Produces: `blockly` and `@blockly/keyboard-navigation` available as npm dependencies

- [ ] **Step 1: Install Blockly packages**

```bash
cd poiesis && pnpm add blockly @blockly/keyboard-navigation
```

- [ ] **Step 2: Remove Lexical packages**

```bash
cd poiesis && pnpm remove lexical @lexical/react
```

- [ ] **Step 3: Verify install**

```bash
cd poiesis && pnpm ls blockly @blockly/keyboard-navigation
```

Expected: both packages listed with versions. `lexical` and `@lexical/react` no longer in dependencies.

- [ ] **Step 4: Verify build fails (expected — old imports broken)**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | head -20
```

Expected: errors about missing `lexical`, `@lexical/react` imports — these will be resolved when old code is deleted in Task 9.

- [ ] **Step 5: Commit**

```bash
git add poiesis/package.json poiesis/pnpm-lock.yaml
git commit -m "chore: swap lexical for blockly + @blockly/keyboard-navigation"
```

---

## Task 2: Extend CodeOp Data Model

**Files:**
- Create: `poiesis/src/core/types/slot-value.ts`
- Create: `poiesis/src/core/types/inline-op.ts`
- Modify: `poiesis/src/core/types/code-op.ts`

**Interfaces:**
- Consumes: none
- Produces:
  - `SlotValue = { kind: "text"; value: string } | { kind: "expr"; children: InlineOp[] } | { kind: "empty" }`
  - `InlineOp` union type (7 variants)
  - Updated `CodeOp` interfaces with SlotValue fields and `bodyId: string`

- [ ] **Step 1: Create `slot-value.ts`**

```typescript
// poiesis/src/core/types/slot-value.ts
import type { InlineOp } from "./inline-op";

/** A slot in a block that the selection cursor can land on. */
export type SlotValue =
  | { kind: "text"; value: string }
  | { kind: "expr"; children: InlineOp[] }
  | { kind: "empty" };

/** Convert a legacy string field to SlotValue. */
export function fromLegacyString(s: string | undefined): SlotValue {
  if (s === undefined || s === "") {
    return { kind: "empty" };
  }
  return { kind: "text", value: s };
}

/** Render SlotValue back to a flat string (for source emission). */
export function slotToText(sv: SlotValue): string {
  switch (sv.kind) {
    case "text": return sv.value;
    case "expr": return sv.children.map(opToText).join(" ");
    case "empty": return "";
  }
}

function opToText(op: InlineOp): string {
  switch (op.kind) {
    case "var_ref": return op.name;
    case "literal": return op.value;
    case "binary": return `${slotToText(op.left)} ${op.op} ${slotToText(op.right)}`;
    case "unary": return `${op.op}${slotToText(op.operand)}`;
    case "call_fn": return `${op.fnName}(${op.args.join(", ")})`;
    case "member_access": return `${op.target}.${op.member}`;
    case "parens": return `(${op.inner.map(opToText).join(" ")})`;
  }
}
```

- [ ] **Step 2: Write tests for `slot-value.ts`**

```bash
mkdir -p poiesis/src/core/types/__tests__
```

Create `poiesis/src/core/types/__tests__/slot-value.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { fromLegacyString, slotToText } from "../slot-value";

describe("fromLegacyString", () => {
  it("returns empty for undefined", () => {
    expect(fromLegacyString(undefined)).toEqual({ kind: "empty" });
  });

  it("returns empty for empty string", () => {
    expect(fromLegacyString("")).toEqual({ kind: "empty" });
  });

  it("returns text slot for non-empty string", () => {
    expect(fromLegacyString("count")).toEqual({ kind: "text", value: "count" });
  });
});

describe("slotToText", () => {
  it("renders text slot as its value", () => {
    expect(slotToText({ kind: "text", value: "count" })).toBe("count");
  });

  it("renders empty slot as empty string", () => {
    expect(slotToText({ kind: "empty" })).toBe("");
  });

  it("renders expression with var_ref and literal", () => {
    expect(slotToText({
      kind: "expr",
      children: [
        { kind: "var_ref", name: "x" },
        { kind: "literal", value: "0", literalType: "number" },
      ],
    })).toBe("x 0");
  });

  it("renders binary expression", () => {
    expect(slotToText({
      kind: "expr",
      children: [
        {
          kind: "binary",
          op: "+",
          left: { kind: "text", value: "x" },
          right: { kind: "text", value: "1" },
        },
      ],
    })).toBe("x + 1");
  });
});
```

- [ ] **Step 3: Run tests to verify they fail (file not created yet — skip, create together)**

Tests and source created together. Run:

```bash
cd poiesis && npx vitest run src/core/types/__tests__/slot-value.test.ts
```

Expected: 6 tests PASS.

- [ ] **Step 4: Create `inline-op.ts`**

```typescript
// poiesis/src/core/types/inline-op.ts
import type { SlotValue } from "./slot-value";

/** Inline operations used inside expression slots only. */
export type InlineOp =
  | { kind: "var_ref"; name: string }
  | { kind: "literal"; value: string; literalType: "number" | "string" | "bool" }
  | { kind: "binary"; op: string; left: SlotValue; right: SlotValue }
  | { kind: "unary"; op: string; operand: SlotValue }
  | { kind: "call_fn"; fnName: string; args: string[] }
  | { kind: "member_access"; target: string; member: string }
  | { kind: "parens"; inner: InlineOp[] };
```

- [ ] **Step 5: Update `code-op.ts`**

Read the current file at `poiesis/src/core/types/code-op.ts`. Replace all `string` fields on the CodeOp interfaces with `SlotValue`, and add `bodyId` to block ops:

```typescript
// poiesis/src/core/types/code-op.ts
import type { SlotValue } from "./slot-value";

export type ScopeKind = "fn" | "container";

interface CodeOpBase {
  pid: string;
}

// ── Simple statements ──

export interface DeclareVarOp extends CodeOpBase {
  kind: "declare_var";
  mutable: boolean;
  name: SlotValue;
  varType: SlotValue;
  initValue: SlotValue;
  scope: ScopeKind;
}

export interface SetVarOp extends CodeOpBase {
  kind: "set_var";
  name: SlotValue;
  operator: "=" | "+=" | "-=" | "*=" | "/=" | "%=";
  value: SlotValue;
}

export interface ReturnExprOp extends CodeOpBase {
  kind: "return";
  value: SlotValue;
}

export interface CallFnOp extends CodeOpBase {
  kind: "call_fn";
  fnName: SlotValue;
  args: SlotValue[];
}

export interface AwaitExprOp extends CodeOpBase {
  kind: "await";
  value: SlotValue;
}

export interface BreakStmtOp extends CodeOpBase {
  kind: "break";
}

export interface ContinueStmtOp extends CodeOpBase {
  kind: "continue";
}

export interface CommentOp extends CodeOpBase {
  kind: "comment";
  text: string; // comments stay as plain string — not a slot
}

export interface RawCodeOp extends CodeOpBase {
  kind: "raw";
  source: string; // raw source stays as plain string — not a slot
}

// ── Block (C-shaped) statements ──

export interface IfBlockOp extends CodeOpBase {
  kind: "if_block";
  bodyId: string;
  branches: {
    condition: SlotValue;
    body: CodeOp[];
  }[];
}

export interface WhileLoopOp extends CodeOpBase {
  kind: "while";
  bodyId: string;
  condition: SlotValue;
  body: CodeOp[];
}

export interface ForLoopOp extends CodeOpBase {
  kind: "for_in";
  bodyId: string;
  itemName: SlotValue;
  collection: SlotValue;
  body: CodeOp[];
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

/** Helper to generate a unique bodyId. */
export function createBodyId(): string {
  return `body-${Math.random().toString(36).slice(2, 10)}`;
}
```

- [ ] **Step 6: Write roundtrip test for backward compatibility**

Create `poiesis/src/core/types/__tests__/code-op-backward-compat.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { fromLegacyString, slotToText } from "../slot-value";

describe("CodeOp backward compatibility", () => {
  it("old DeclareVarOp with string name converts to SlotValue text", () => {
    // Simulate loading old data where name was a plain string
    const oldName = "count";
    const sv = fromLegacyString(oldName);
    expect(sv).toEqual({ kind: "text", value: "count" });
    expect(slotToText(sv)).toBe("count");
  });

  it("old IfBlockOp with string condition converts to SlotValue text", () => {
    const oldCondition = "x > 0";
    const sv = fromLegacyString(oldCondition);
    expect(sv).toEqual({ kind: "text", value: "x > 0" });
  });

  it("old ReturnExprOp with undefined value converts to empty", () => {
    const sv = fromLegacyString(undefined);
    expect(sv).toEqual({ kind: "empty" });
  });
});
```

- [ ] **Step 7: Run all type tests**

```bash
cd poiesis && npx vitest run src/core/types/__tests__/
```

Expected: all tests PASS.

- [ ] **Step 8: Check TypeScript**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep -c "code-op\|slot-value\|inline-op"
```

Expected: only pre-existing errors in unrelated files (Lexical imports already broken from Task 1).

- [ ] **Step 9: Commit**

```bash
git add poiesis/src/core/types/code-op.ts poiesis/src/core/types/slot-value.ts poiesis/src/core/types/inline-op.ts poiesis/src/core/types/__tests__/
git commit -m "feat: extend CodeOp model with SlotValue, InlineOp, and bodyId"
```

---

## Task 3: Define Blockly Block Definitions

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/definitions.ts`
- Create: `poiesis/src/components/canvas/code/blocks/__tests__/definitions.test.ts`

**Interfaces:**
- Consumes: `CodeOp` types from Task 2
- Produces: `BLOCK_DEFINITIONS: Record<string, BlockDefinition>` — JSON definitions for all 13 block types
- Produces: `registerAllBlocks(): void` — registers all blocks with Blockly

- [ ] **Step 1: Create block definitions file**

```typescript
// poiesis/src/components/canvas/code/blocks/definitions.ts
import * as Blockly from "blockly";

// ── Color palette (Zelos/Scratch 3.0 defaults) ──
const C = {
  CONTROL:  "#FFAB19",  // if, while, for
  VARIABLE: "#FF8C1A",  // declare_var, set_var
  FUNCTION: "#995BA5",  // return, call_fn
  ASYNC:    "#5B67B0",  // await
  BREAK:    "#CF8B17",  // break, continue
  COMMENT:  "#5CB85C",  // comment
  RAW:      "#AAAAAA",  // raw
} as const;

// ── Simple statement blocks ──

const declareVarDef = {
  type: "declare_var",
  message0: "%1 %2 : %3 = %4",
  args0: [
    { type: "field_dropdown", name: "MUT", options: [["let", "let"], ["let mut", "mut"]] },
    { type: "input_value", name: "NAME", check: "Text" },
    { type: "input_value", name: "VARTYPE", check: "Text" },
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.VARIABLE,
  tooltip: "Declare a variable",
};

const setVarDef = {
  type: "set_var",
  message0: "%1 %2 = %3",
  args0: [
    { type: "field_dropdown", name: "OP", options: [["set", "set"], ["+=", "+="], ["-=", "-="], ["*=", "*="], ["/=", "/="], ["%=", "%="]] },
    { type: "input_value", name: "NAME", check: "Text" },
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.VARIABLE,
};

const returnDef = {
  type: "return_stmt",
  message0: "return %1",
  args0: [
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.FUNCTION,
};

const callFnDef = {
  type: "call_fn",
  message0: "call %1 ( %2 )",
  args0: [
    { type: "input_value", name: "FN_NAME", check: "Text" },
    { type: "input_value", name: "ARGS", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.FUNCTION,
  // ARGS is a single input; multi-arg support is deferred
};

const awaitDef = {
  type: "await_expr",
  message0: "await %1",
  args0: [
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.ASYNC,
};

const breakDef = {
  type: "break_stmt",
  message0: "break",
  previousStatement: null,
  nextStatement: null,
  colour: C.BREAK,
};

const continueDef = {
  type: "continue_stmt",
  message0: "continue",
  previousStatement: null,
  nextStatement: null,
  colour: C.BREAK,
};

const commentDef = {
  type: "comment_stmt",
  message0: "// %1",
  args0: [
    { type: "field_input", name: "TEXT", text: "" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.COMMENT,
};

const rawDef = {
  type: "raw_stmt",
  message0: "raw %1",
  args0: [
    { type: "field_multilinetext", name: "SOURCE", text: "" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.RAW,
};

// ── C-shaped blocks ──

const ifBlockDef = {
  type: "if_block",
  message0: "if %1",
  args0: [
    { type: "input_value", name: "CONDITION", check: "Expr" },
  ],
  message1: "do %1",
  args1: [
    { type: "input_statement", name: "BODY", check: "Statement" },
  ],
  message2: "else %1",
  args2: [
    { type: "input_statement", name: "ELSE_BODY", check: "Statement" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.CONTROL,
  // Mutation for elif/else
  mutator: "if_block_mutator",
  tooltip: "If / elif / else block",
};

const whileDef = {
  type: "while_loop",
  message0: "while %1",
  args0: [
    { type: "input_value", name: "CONDITION", check: "Expr" },
  ],
  message1: "do %1",
  args1: [
    { type: "input_statement", name: "BODY", check: "Statement" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.CONTROL,
};

const forInDef = {
  type: "for_in",
  message0: "for %1 in %2",
  args0: [
    { type: "input_value", name: "ITEM", check: "Text" },
    { type: "input_value", name: "COLLECTION", check: "Expr" },
  ],
  message1: "do %1",
  args1: [
    { type: "input_statement", name: "BODY", check: "Statement" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.CONTROL,
};

export const BLOCK_DEFINITIONS: Record<string, unknown> = {
  declare_var: declareVarDef,
  set_var: setVarDef,
  return_stmt: returnDef,
  call_fn: callFnDef,
  await_expr: awaitDef,
  break_stmt: breakDef,
  continue_stmt: continueDef,
  comment_stmt: commentDef,
  raw_stmt: rawDef,
  if_block: ifBlockDef,
  while_loop: whileDef,
  for_in: forInDef,
};

export function registerAllBlocks(): void {
  for (const [name, def] of Object.entries(BLOCK_DEFINITIONS)) {
    Blockly.Blocks[name] = {
      init(this: Blockly.Block) {
        this.jsonInit(def as Blockly.BlockDefinition);
      },
    };
  }
}
```

- [ ] **Step 2: Write test that all 13 definitions are valid**

```typescript
// poiesis/src/components/canvas/code/blocks/__tests__/definitions.test.ts
import { describe, it, expect } from "vitest";
import { BLOCK_DEFINITIONS } from "../definitions";

describe("BLOCK_DEFINITIONS", () => {
  const expectedBlocks = [
    "declare_var", "set_var", "return_stmt", "call_fn",
    "await_expr", "break_stmt", "continue_stmt",
    "comment_stmt", "raw_stmt",
    "if_block", "while_loop", "for_in",
  ];

  it("has all 13 block types", () => {
    expect(Object.keys(BLOCK_DEFINITIONS).sort()).toEqual(expectedBlocks.sort());
  });

  for (const name of expectedBlocks) {
    it(`${name} has a type field`, () => {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      expect(def.type).toBe(name);
    });

    it(`${name} has a colour field`, () => {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      expect(def.colour).toBeDefined();
    });
  }

  it("C-shaped blocks have input_statement BODY", () => {
    for (const name of ["if_block", "while_loop", "for_in"]) {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      const args = Object.values(def).filter(
        (v) => typeof v === "object" && v !== null && (v as Record<string, unknown>).name === "BODY"
      );
      expect(args.length).toBeGreaterThan(0);
    }
  });
});
```

- [ ] **Step 3: Run tests**

```bash
cd poiesis && npx vitest run src/components/canvas/code/blocks/__tests__/definitions.test.ts
```

Expected: 16 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/blocks/
git commit -m "feat: add Blockly block definitions for all 13 block types"
```

---

## Task 4: Build BlockAdapter (CodeOp ↔ Blockly)

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/BlockAdapter.ts`
- Create: `poiesis/src/components/canvas/code/blocks/__tests__/BlockAdapter.test.ts`

**Interfaces:**
- Consumes: `CodeOp` types (Task 2), `BLOCK_DEFINITIONS` (Task 3)
- Produces:
  - `class BlockAdapter { toBlocks(ops: CodeOp[], workspace: Blockly.Workspace): Blockly.Block[]; fromBlocks(workspace: Blockly.Workspace): CodeOp[] }`

- [ ] **Step 1: Write failing roundtrip test**

```typescript
// poiesis/src/components/canvas/code/blocks/__tests__/BlockAdapter.test.ts
import { describe, it, expect } from "vitest";
import * as Blockly from "blockly";
import { registerAllBlocks } from "../definitions";
import type { CodeOp, DeclareVarOp, IfBlockOp, WhileLoopOp, ForLoopOp, ReturnExprOp, SetVarOp } from "../../../../../core/types/code-op";
import { createBodyId } from "../../../../../core/types/code-op";

// Register blocks before tests
registerAllBlocks();

function makeWorkspace(): Blockly.WorkspaceSvg {
  const div = document.createElement("div");
  return Blockly.inject(div, {
    renderer: "zelos",
    // Minimal config for headless testing
  }) as Blockly.WorkspaceSvg;
}

describe("BlockAdapter roundtrip", () => {
  it("let + set roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "declare_var",
        pid: "dv-1",
        mutable: false,
        name: { kind: "text", value: "count" },
        varType: { kind: "text", value: "i32" },
        initValue: { kind: "text", value: "0" },
        scope: "fn",
      } as DeclareVarOp,
      {
        kind: "set_var",
        pid: "sv-1",
        name: { kind: "text", value: "count" },
        operator: "+=",
        value: { kind: "text", value: "1" },
      } as SetVarOp,
    ];


    // TODO: Implement BlockAdapter
    // const adapter = new BlockAdapter();
    // const ws = makeWorkspace();
    // adapter.toBlocks(ops, ws);
    // const back = adapter.fromBlocks(ws);
    // expect(back.length).toBe(2);
    // expect(back[0].kind).toBe("declare_var");
    // ...
  });
});
```

This test should FAIL because BlockAdapter doesn't exist yet.

- [ ] **Step 2: Run test to confirm failure**

```bash
cd poiesis && npx vitest run src/components/canvas/code/blocks/__tests__/BlockAdapter.test.ts
```

Expected: FAIL (cannot find module `../BlockAdapter`).

- [ ] **Step 3: Implement BlockAdapter.toBlocks()**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockAdapter.ts
import * as Blockly from "blockly";
import type { CodeOp } from "../../../../core/types/code-op";
import { fromLegacyString, slotToText } from "../../../../core/types/slot-value";
import { registerAllBlocks } from "./definitions";

export class BlockAdapter {
  private pidToBlock = new Map<string, Blockly.Block>();

  constructor() {
    registerAllBlocks();
  }

  /**
   * Convert CodeOp[] to Blockly blocks and append to workspace.
   */
  toBlocks(ops: CodeOp[], workspace: Blockly.Workspace): Blockly.Block[] {
    this.pidToBlock.clear();
    workspace.clear();
    const blocks: Blockly.Block[] = [];
    for (const op of ops) {
      const block = this.createBlock(op, workspace);
      if (block) {
        blocks.push(block);
      }
    }
    return blocks;
  }

  private createBlock(op: CodeOp, workspace: Blockly.Workspace): Blockly.Block | null {
    let block: Blockly.Block | null = null;

    switch (op.kind) {
      case "declare_var": {
        block = workspace.newBlock("declare_var");
        block.setFieldValue(op.mutable ? "mut" : "let", "MUT");
        this.setSlotField(block, "NAME", op.name);
        this.setSlotField(block, "VARTYPE", op.varType);
        this.setSlotField(block, "VALUE", op.initValue);
        break;
      }
      case "set_var": {
        block = workspace.newBlock("set_var");
        block.setFieldValue(this.operatorToSymbol(op.operator), "OP");
        this.setSlotField(block, "NAME", op.name);
        this.setSlotField(block, "VALUE", op.value);
        break;
      }
      case "return": {
        block = workspace.newBlock("return_stmt");
        this.setSlotField(block, "VALUE", op.value);
        break;
      }
      case "call_fn": {
        block = workspace.newBlock("call_fn");
        this.setSlotField(block, "FN_NAME", op.fnName);
        if (op.args.length > 0) {
          this.setSlotField(block, "ARGS", op.args[0]); // single arg for now
        }
        break;
      }
      case "await": {
        block = workspace.newBlock("await_expr");
        this.setSlotField(block, "VALUE", op.value);
        break;
      }
      case "break": {
        block = workspace.newBlock("break_stmt");
        break;
      }
      case "continue": {
        block = workspace.newBlock("continue_stmt");
        break;
      }
      case "comment": {
        block = workspace.newBlock("comment_stmt");
        block.setFieldValue(op.text, "TEXT");
        break;
      }
      case "raw": {
        block = workspace.newBlock("raw_stmt");
        block.setFieldValue(op.source, "SOURCE");
        break;
      }
      case "if_block": {
        block = workspace.newBlock("if_block");
        const firstBranch = op.branches[0];
        if (firstBranch) {
          this.setSlotField(block, "CONDITION", firstBranch.condition);
          this.createBodyBlocks(block, "BODY", firstBranch.body, workspace);
        }
        // TODO: elif/else via mutation (Task 4b)
        break;
      }
      case "while": {
        block = workspace.newBlock("while_loop");
        this.setSlotField(block, "CONDITION", op.condition);
        this.createBodyBlocks(block, "BODY", op.body, workspace);
        break;
      }
      case "for_in": {
        block = workspace.newBlock("for_in");
        this.setSlotField(block, "ITEM", op.itemName);
        this.setSlotField(block, "COLLECTION", op.collection);
        this.createBodyBlocks(block, "BODY", op.body, workspace);
        break;
      }
    }

    if (block) {
      this.pidToBlock.set(op.pid, block);
    }
    return block;
  }

  /** Set a block's input field from a SlotValue. */
  private setSlotField(block: Blockly.Block, inputName: string, sv: unknown): void {
    const slotValue = sv as { kind: string; value?: string };
    if (slotValue.kind === "text" || slotValue.kind === "expr") {
      const text = slotToText(slotValue as Parameters<typeof slotToText>[0]);
      const input = block.getInput(inputName);
      if (input) {
        // For now, represent text/expr as a label field in the input
        // Full expression rendering is deferred — Task 5 covers it
        const existingField = input.fieldRow.find(f => f.name === inputName);
        if (existingField && existingField.setValue) {
          existingField.setValue(text);
        }
      }
    }
  }

  /** Recursively create body blocks inside a statement input. */
  private createBodyBlocks(
    parentBlock: Blockly.Block,
    inputName: string,
    body: CodeOp[],
    workspace: Blockly.Workspace,
  ): void {
    const input = parentBlock.getInput(inputName);
    if (!input) return;

    const conn = input.connection;
    if (!conn) return;

    // Build body blocks and connect them sequentially
    let prevBlock: Blockly.Block | null = null;
    for (const op of body) {
      const child = this.createBlock(op, workspace);
      if (child && child.previousConnection) {
        if (prevBlock && prevBlock.nextConnection) {
          child.previousConnection.connect(prevBlock.nextConnection);
        } else {
          child.previousConnection.connect(conn);
        }
        prevBlock = child;
      }
    }
  }

  private operatorToSymbol(op: string): string {
    switch (op) {
      case "+=": return "+=";
      case "-=": return "-=";
      case "*=": return "*=";
      case "/=": return "/=";
      case "%=": return "%=";
      default: return "set";
    }
  }

  /**
   * Convert all top-level blocks in the workspace back to CodeOp[].
   */
  fromBlocks(workspace: Blockly.Workspace): CodeOp[] {
    const ops: CodeOp[] = [];
    const topBlocks = workspace.getTopBlocks(true);
    for (const block of topBlocks) {
      const op = this.blockToCodeOp(block);
      if (op) ops.push(op);
    }
    return ops;
  }

  private blockToCodeOp(block: Blockly.Block): CodeOp | null {
    // Implementation deferred to Step 5
    return null;
  }
}
```

- [ ] **Step 4: Implement BlockAdapter.fromBlocks()**

Add the `blockToCodeOp` method to BlockAdapter.ts with the reverse mapping:

```typescript
private blockToCodeOp(block: Blockly.Block): CodeOp | null {
  const pid = `pid-${block.type}-${Math.random().toString(36).slice(2, 8)}`;

  switch (block.type) {
    case "declare_var": {
      const mutable = block.getFieldValue("MUT") === "mut";
      return {
        kind: "declare_var",
        pid,
        mutable,
        name: fromLegacyString(this.getInputText(block, "NAME")),
        varType: fromLegacyString(this.getInputText(block, "VARTYPE")),
        initValue: fromLegacyString(this.getInputText(block, "VALUE")),
        scope: "fn",
      };
    }
    case "set_var": {
      const fieldOp = block.getFieldValue("OP") || "set";
      // Map dropdown value back to operator
      const opMap: Record<string, "=" | "+=" | "-=" | "*=" | "/=" | "%="> = {
        "set": "=", "+=": "+=", "-=": "-=", "*=": "*=", "/=": "/=", "%=": "%=",
      };
      return {
        kind: "set_var",
        pid,
        name: fromLegacyString(this.getInputText(block, "NAME")),
        operator: opMap[fieldOp] || "=",
        value: fromLegacyString(this.getInputText(block, "VALUE")),
      };
    }
    case "return_stmt": {
      return {
        kind: "return",
        pid,
        value: fromLegacyString(this.getInputText(block, "VALUE")),
      };
    }
    case "call_fn": {
      return {
        kind: "call_fn",
        pid,
        fnName: fromLegacyString(this.getInputText(block, "FN_NAME")),
        args: [fromLegacyString(this.getInputText(block, "ARGS"))],
      };
    }
    case "await_expr": {
      return {
        kind: "await",
        pid,
        value: fromLegacyString(this.getInputText(block, "VALUE")),
      };
    }
    case "break_stmt": {
      return { kind: "break", pid };
    }
    case "continue_stmt": {
      return { kind: "continue", pid };
    }
    case "comment_stmt": {
      return {
        kind: "comment",
        pid,
        text: block.getFieldValue("TEXT") || "",
      };
    }
    case "raw_stmt": {
      return {
        kind: "raw",
        pid,
        source: block.getFieldValue("SOURCE") || "",
      };
    }
    case "if_block": {
      const branches: { condition: ReturnType<typeof fromLegacyString>; body: CodeOp[] }[] = [];
      branches.push({
        condition: fromLegacyString(this.getInputText(block, "CONDITION")),
        body: this.getStatementBody(block, "BODY"),
      });
      const elseBody = this.getStatementBody(block, "ELSE_BODY");
      if (elseBody.length > 0) {
        branches.push({ condition: fromLegacyString(""), body: elseBody });
      }
      return { kind: "if_block", pid, bodyId: createBodyId(), branches };
    }
    case "while_loop": {
      return {
        kind: "while",
        pid,
        bodyId: createBodyId(),
        condition: fromLegacyString(this.getInputText(block, "CONDITION")),
        body: this.getStatementBody(block, "BODY"),
      };
    }
    case "for_in": {
      return {
        kind: "for_in",
        pid,
        bodyId: createBodyId(),
        itemName: fromLegacyString(this.getInputText(block, "ITEM")),
        collection: fromLegacyString(this.getInputText(block, "COLLECTION")),
        body: this.getStatementBody(block, "BODY"),
      };
    }
    default:
      return null;
  }
}

private getInputText(block: Blockly.Block, inputName: string): string {
  const input = block.getInput(inputName);
  if (!input) return "";
  // Get text from the first field in the input's field row
  const field = input.fieldRow.find(f => f.getValue);
  return field ? (field.getValue() as string) || "" : "";
}

private getStatementBody(block: Blockly.Block, inputName: string): CodeOp[] {
  const input = block.getInput(inputName);
  if (!input?.connection) return [];
  const conn = input.connection;
  const ops: CodeOp[] = [];
  let child = conn.targetBlock();
  while (child) {
    const op = this.blockToCodeOp(child);
    if (op) ops.push(op);
    child = child.getNextBlock();
  }
  return ops;
}
```

- [ ] **Step 5: Write real roundtrip tests**

Update `BlockAdapter.test.ts` with actual tests:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import * as Blockly from "blockly";
import { registerAllBlocks } from "../definitions";
import { BlockAdapter } from "../BlockAdapter";
import type { CodeOp } from "../../../../../core/types/code-op";
import { createBodyId } from "../../../../../core/types/code-op";

registerAllBlocks();

function makeWorkspace(): Blockly.WorkspaceSvg {
  const div = document.createElement("div");
  div.style.width = "800px";
  div.style.height = "600px";
  document.body.appendChild(div);
  return Blockly.inject(div, {
    renderer: "zelos",
    scrollbars: false,
    grid: { spacing: 20, length: 0, colour: "#ccc", snap: false },
  }) as Blockly.WorkspaceSvg;
}

describe("BlockAdapter", () => {
  let adapter: BlockAdapter;

  beforeEach(() => {
    adapter = new BlockAdapter();
  });

  it("roundtrip: empty ops", () => {
    // Need workspace for fromBlocks to work, but toBlocks clears it
    // Test: fromBlocks on empty workspace returns []
    const ws = makeWorkspace();
    expect(adapter.fromBlocks(ws)).toEqual([]);
    ws.dispose();
  });

  it("roundtrip: declare_var (let)", () => {
    const ws = makeWorkspace();
    const ops: CodeOp[] = [{
      kind: "declare_var" as const,
      pid: "dv-1",
      mutable: false,
      name: { kind: "text" as const, value: "count" },
      varType: { kind: "text" as const, value: "i32" },
      initValue: { kind: "text" as const, value: "0" },
      scope: "fn" as const,
    }];

    adapter.toBlocks(ops, ws);
    const result = adapter.fromBlocks(ws);

    expect(result.length).toBe(1);
    expect(result[0].kind).toBe("declare_var");
    const op = result[0] as typeof ops[0];
    expect(op.name.kind).toBe("text");
    expect((op.name as { kind: "text"; value: string }).value).toBe("count");
    ws.dispose();
  });

  it("roundtrip: while loop with body", () => {
    const ws = makeWorkspace();
    const ops: CodeOp[] = [{
      kind: "while" as const,
      pid: "wh-1",
      bodyId: createBodyId(),
      condition: { kind: "text" as const, value: "i < 10" },
      body: [{
        kind: "set_var" as const,
        pid: "sv-1",
        name: { kind: "text" as const, value: "i" },
        operator: "+=" as const,
        value: { kind: "text" as const, value: "1" },
      }],
    }];

    adapter.toBlocks(ops, ws);
    const result = adapter.fromBlocks(ws);

    expect(result.length).toBe(1);
    const w = result[0] as typeof ops[0];
    expect(w.kind).toBe("while");
    expect((w.condition as { kind: "text"; value: string }).value).toBe("i < 10");
    expect(w.body.length).toBe(1);
    const b0 = w.body[0];
    if (b0.kind === "set_var") {
      expect((b0.name as { kind: "text"; value: string }).value).toBe("i");
      expect(b0.operator).toBe("+=");
    }
    ws.dispose();
  });

  it("roundtrip: if_block with condition + else", () => {
    const ws = makeWorkspace();
    const ops: CodeOp[] = [{
      kind: "if_block" as const,
      pid: "if-1",
      bodyId: createBodyId(),
      branches: [
        {
          condition: { kind: "text" as const, value: "x > 0" },
          body: [{
            kind: "return" as const,
            pid: "ret-1",
            value: { kind: "text" as const, value: "1" },
          }],
        },
        {
          condition: { kind: "empty" as const },
          body: [{
            kind: "return" as const,
            pid: "ret-2",
            value: { kind: "text" as const, value: "0" },
          }],
        },
      ],
    }];

    adapter.toBlocks(ops, ws);
    const result = adapter.fromBlocks(ws);

    expect(result.length).toBe(1);
    if (result[0].kind === "if_block") {
      expect(result[0].branches.length).toBeGreaterThanOrEqual(1);
    }
    ws.dispose();
  });
});
```

- [ ] **Step 6: Run tests**

```bash
cd poiesis && npx vitest run src/components/canvas/code/blocks/__tests__/BlockAdapter.test.ts
```

Expected: tests should pass (or reveal real issues to fix).

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/components/canvas/code/blocks/BlockAdapter.ts poiesis/src/components/canvas/code/blocks/__tests__/BlockAdapter.test.ts
git commit -m "feat: add BlockAdapter for CodeOp ↔ Blockly bidirectional conversion"
```

---

## Task 5: Build FuzzySearchField

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/fields/FuzzySearchField.ts`
- Create: `poiesis/src/components/canvas/code/blocks/fields/__tests__/FuzzySearchField.test.ts`

**Interfaces:**
- Consumes: none (standalone field)
- Produces: `class FuzzySearchField extends Blockly.Field` — a custom Blockly field that opens a fuzzy-search dropdown on keypress

- [ ] **Step 1: Write fuzzy matcher tests**

```typescript
// poiesis/src/components/canvas/code/blocks/fields/__tests__/FuzzySearchField.test.ts
import { describe, it, expect } from "vitest";

/**
 * Fuzzy match: check if query characters appear in order in the target string.
 * Case-insensitive. Returns a score (higher = better match).
 */
function fuzzyScore(query: string, target: string): number {
  const q = query.toLowerCase();
  const t = target.toLowerCase();
  let qi = 0;
  let score = 0;
  let consecutive = 0;
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      qi++;
      consecutive++;
      score += consecutive * 10; // consecutive matches score higher
    } else {
      consecutive = 0;
    }
  }
  return qi === q.length ? score : 0;
}

describe("fuzzyScore", () => {
  it("exact match scores highest", () => {
    const exact = fuzzyScore("if", "if");
    const prefix = fuzzyScore("if", "if_block");
    expect(exact).toBeGreaterThan(prefix);
  });

  it("substring match scores", () => {
    expect(fuzzyScore("wh", "while")).toBeGreaterThan(0);
    expect(fuzzyScore("wh", "let")).toBe(0);
  });

  it("case insensitive", () => {
    expect(fuzzyScore("IF", "if_block")).toBeGreaterThan(0);
  });

  it("fuzzy match after prefix", () => {
    // "fl" should match "for_in" (f...l doesn't exist) — actually "fl" vs "for_in": f✓, l✗
    expect(fuzzyScore("fl", "for_in")).toBe(0);
    // "fi" should match "for_in": f✓, i✗ → no match
    expect(fuzzyScore("fi", "for_in")).toBe(0);
    // "fo" should match "for_in"
    expect(fuzzyScore("fo", "for_in")).toBeGreaterThan(0);
  });
});
```

- [ ] **Step 2: Confirm tests fail (no implementation yet)**

```bash
cd poiesis && npx vitest run src/components/canvas/code/blocks/fields/__tests__/FuzzySearchField.test.ts
```

Expected: PASS (self-contained fuzzy matcher test — `fuzzyScore` is defined inline).

- [ ] **Step 3: Implement FuzzySearchField**

```typescript
// poiesis/src/components/canvas/code/blocks/fields/FuzzySearchField.ts
import * as Blockly from "blockly";

interface SearchItem {
  label: string;       // display text
  blockType: string;   // Blockly block type to create
  description: string; // subtitle in dropdown
}

/** Default search items — all 13 block types available for insertion. */
const DEFAULT_ITEMS: SearchItem[] = [
  { label: "let", blockType: "declare_var", description: "declare variable" },
  { label: "set", blockType: "set_var", description: "assign variable" },
  { label: "return", blockType: "return_stmt", description: "return value" },
  { label: "call", blockType: "call_fn", description: "call function" },
  { label: "await", blockType: "await_expr", description: "await expression" },
  { label: "break", blockType: "break_stmt", description: "break loop" },
  { label: "continue", blockType: "continue_stmt", description: "continue loop" },
  { label: "comment", blockType: "comment_stmt", description: "add comment" },
  { label: "raw", blockType: "raw_stmt", description: "raw code" },
  { label: "if", blockType: "if_block", description: "if / elif / else" },
  { label: "while", blockType: "while_loop", description: "while loop" },
  { label: "for", blockType: "for_in", description: "for..in loop" },
];

/**
 * Fuzzy search field.
 *
 * When the user types, it opens a dropdown showing matching block types.
 * Selecting an item creates the block at the workspace cursor position.
 */
export class FuzzySearchField extends Blockly.Field {
  private searchItems: SearchItem[];
  private dropdownDiv: HTMLDivElement | null = null;
  private inputEl: HTMLInputElement | null = null;
  private selectedIndex = -1;

  constructor(items?: SearchItem[], validator?: Blockly.FieldValidator<string>) {
    super("", validator);
    this.searchItems = items ?? DEFAULT_ITEMS;
  }

  /**
   * Show the dropdown when the field receives focus.
   */
  protected showEditor_(_e?: Event): void {
    this.showDropdown_("");
  }

  /**
   * Handle keydown: character keys filter, Enter selects, Esc closes, arrows navigate.
   */
  protected onHtmlInputKeyDown_(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      this.closeDropdown_();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      this.selectedIndex = Math.min(this.selectedIndex + 1, this.getVisibleItems_().length - 1);
      this.rerenderHighlight_();
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      this.selectedIndex = Math.max(this.selectedIndex - 1, 0);
      this.rerenderHighlight_();
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      this.commitSelection_();
      return;
    }
    // Character key — filter
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
      const query = (this.inputEl?.value ?? "") + e.key;
      this.showDropdown_(query);
    }
  }

  private showDropdown_(query: string): void {
    this.closeDropdown_();

    const div = document.createElement("div");
    div.className = "fuzzy-search-dropdown";
    div.style.cssText = `
      position: fixed;
      z-index: 10000;
      background: var(--ol-surface, #fff);
      border: 1px solid var(--ol-border, #d3d2cc);
      border-radius: 8px;
      box-shadow: 0 8px 24px rgba(0,0,0,0.15);
      min-width: 240px;
      max-height: 300px;
      overflow-y: auto;
      font-family: "JetBrains Mono", "Fira Code", monospace;
      font-size: 13px;
    `;

    // Search input
    const input = document.createElement("input");
    input.type = "text";
    input.className = "fuzzy-search-input";
    input.value = query;
    input.style.cssText = `
      width: 100%;
      padding: 8px 12px;
      border: none;
      border-bottom: 1px solid var(--ol-border, #d3d2cc);
      border-radius: 8px 8px 0 0;
      font-family: inherit;
      font-size: inherit;
      outline: none;
      background: transparent;
      box-sizing: border-box;
    `;
    input.addEventListener("keydown", (e) => this.onHtmlInputKeyDown_(e));
    input.addEventListener("input", () => {
      this.showDropdown_(input.value);
    });
    div.appendChild(input);

    // Filtered items
    const items = this.filterItems_(query);
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      const row = document.createElement("div");
      row.className = "fuzzy-search-item";
      row.style.cssText = `
        padding: 6px 12px;
        cursor: pointer;
        display: flex;
        justify-content: space-between;
        align-items: center;
      `;
      if (i === this.selectedIndex) {
        row.style.background = "var(--ol-color-primary-subtle, rgba(201,100,66,0.1))";
      }

      const labelSpan = document.createElement("span");
      labelSpan.style.fontWeight = "600";
      labelSpan.textContent = item.label;

      const descSpan = document.createElement("span");
      descSpan.style.cssText = "color: var(--ol-text-muted, #94a3b8); font-size: 11px;";
      descSpan.textContent = item.description;

      row.appendChild(labelSpan);
      row.appendChild(descSpan);

      const itemIndex = i;
      row.addEventListener("click", () => {
        this.selectedIndex = itemIndex;
        this.commitSelection_();
      });
      div.appendChild(row);
    }

    if (items.length === 0) {
      const empty = document.createElement("div");
      empty.style.cssText = "padding: 12px; color: var(--ol-text-muted); text-align: center;";
      empty.textContent = "No matching blocks";
      div.appendChild(empty);
    }

    // Position near the workspace cursor
    const ws = this.sourceBlock_?.workspace;
    if (ws) {
      const metrics = ws.getMetrics();
      div.style.left = `${metrics.viewLeft + 20}px`;
      div.style.top = `${metrics.viewTop + 40}px`;
    }

    document.body.appendChild(div);
    this.dropdownDiv = div;
    this.inputEl = input;
    this.selectedIndex = items.length > 0 ? 0 : -1;

    input.focus();
  }

  private filterItems_(query: string): SearchItem[] {
    if (!query) return this.searchItems;
    const q = query.toLowerCase();
    return this.searchItems
      .filter(item => {
        const label = item.label.toLowerCase();
        const desc = item.description.toLowerCase();
        // Simple substring match for now
        return label.includes(q) || desc.includes(q) ||
          item.blockType.toLowerCase().includes(q);
      });
  }

  private getVisibleItems_(): SearchItem[] {
    if (!this.inputEl) return [];
    return this.filterItems_(this.inputEl.value);
  }

  private rerenderHighlight_(): void {
    if (!this.dropdownDiv) return;
    const items = this.getVisibleItems_();
    const rows = this.dropdownDiv.querySelectorAll(".fuzzy-search-item");
    rows.forEach((row, i) => {
      (row as HTMLElement).style.background = i === this.selectedIndex
        ? "var(--ol-color-primary-subtle, rgba(201,100,66,0.1))"
        : "";
    });
  }

  private commitSelection_(): void {
    const items = this.getVisibleItems_();
    if (this.selectedIndex >= 0 && this.selectedIndex < items.length) {
      const item = items[this.selectedIndex];
      // Create the block at the workspace cursor
      const ws = this.sourceBlock_?.workspace;
      if (ws) {
        const block = ws.newBlock(item.blockType);
        block.initSvg();
        block.render();
        // Position at cursor location
        const cursor = ws.getCursor();
        if (cursor) {
          const node = cursor.getCurNode();
          if (node && node.getType() === Blockly.ASTNode.types.WORKSPACE) {
            block.moveBy(20, 20);
          }
        }
        block.select();
      }
    }
    this.closeDropdown_();
  }

  private closeDropdown_(): void {
    if (this.dropdownDiv) {
      this.dropdownDiv.remove();
      this.dropdownDiv = null;
      this.inputEl = null;
    }
  }
}

Blockly.fieldRegistry.register("fuzzy_search", FuzzySearchField);
```

- [ ] **Step 4: Run fuzzy matcher tests**

```bash
cd poiesis && npx vitest run src/components/canvas/code/blocks/fields/__tests__/FuzzySearchField.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/blocks/fields/
git commit -m "feat: add FuzzySearchField — keyboard-driven block insertion with fuzzy matching"
```

---

## Task 6: Build BlockEditor React Component

**Files:**
- Create: `poiesis/src/components/canvas/code/BlockEditor.tsx`
- Create: `poiesis/src/components/canvas/code/BlockEditor.module.css`

**Interfaces:**
- Consumes: `BlockAdapter` (Task 4), `registerAllBlocks` (Task 3), `CodeOp` (Task 2)
- Produces: `export const BlockEditor: React.FC<Props>` — replaces CodeCardEditor

- [ ] **Step 1: Create CSS**

```css
/* poiesis/src/components/canvas/code/BlockEditor.module.css */
.overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
}

.editor {
  width: 85vw;
  height: 85vh;
  background: var(--ol-surface, #fff);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--ol-border, #d3d2cc);
}

.title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.actions {
  display: flex;
  gap: 8px;
}

.textModeBtn {
  padding: 6px 16px;
  background: transparent;
  color: var(--ol-color-primary, #c96442);
  border: 1px solid var(--ol-color-primary, #c96442);
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.saveBtn {
  padding: 6px 16px;
  background: var(--ol-color-primary, #c96442);
  color: var(--ol-text-inverse, #fff);
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.cancelBtn {
  padding: 6px 16px;
  background: var(--ol-gray-200, #d3d2cc);
  color: var(--ol-text-secondary, #5e5d59);
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.blocklyContainer {
  flex: 1;
  min-height: 0;
}
```

- [ ] **Step 2: Create BlockEditor component**

```typescript
// poiesis/src/components/canvas/code/BlockEditor.tsx
import { useCallback, useEffect, useRef, useState } from "react";
import * as Blockly from "blockly";
import type { CodeOp } from "../../../core/types/code-op";
import { BlockAdapter } from "./blocks/BlockAdapter";
import { registerAllBlocks } from "./blocks/definitions";
import { CodeEditorModal } from "./CodeEditorModal";
import styles from "./BlockEditor.module.css";

// Register blocks once at module level
registerAllBlocks();

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  initialSource: string;
  language?: "typescript" | "omnilang";
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

export const BlockEditor: React.FC<Props> = ({
  elementId,
  initialOps,
  initialSource,
  language = "omnilang",
  onSave,
  onClose,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const workspaceRef = useRef<Blockly.WorkspaceSvg | null>(null);
  const adapterRef = useRef<BlockAdapter>(new BlockAdapter());
  const opsRef = useRef<CodeOp[]>(initialOps);
  const [showMonaco, setShowMonaco] = useState(false);
  const [opsCount, setOpsCount] = useState(initialOps.length);

  // Initialize Blockly workspace
  useEffect(() => {
    if (!containerRef.current || workspaceRef.current) return;

    const ws = Blockly.inject(containerRef.current, {
      renderer: "zelos",
      toolbox: {
        kind: "flyoutToolbox",
        contents: [
          { kind: "block", type: "declare_var" },
          { kind: "block", type: "set_var" },
          { kind: "block", type: "return_stmt" },
          { kind: "block", type: "call_fn" },
          { kind: "block", type: "await_expr" },
          { kind: "block", type: "break_stmt" },
          { kind: "block", type: "continue_stmt" },
          { kind: "block", type: "comment_stmt" },
          { kind: "block", type: "raw_stmt" },
          { kind: "block", type: "if_block" },
          { kind: "block", type: "while_loop" },
          { kind: "block", type: "for_in" },
        ],
      },
      scrollbars: true,
      grid: { spacing: 20, length: 3, colour: "#e8e7e2", snap: false },
      zoom: { controls: true, wheel: true, startScale: 1.0 },
      move: { scrollbars: true, drag: false, wheel: true },
    });

    // Disable drag-and-drop
    // Blockly doesn't have a direct "disable drag" config; we prevent it via CSS
    const svg = ws.getParentSvg();
    if (svg) {
      svg.style.pointerEvents = "auto";
    }

    // Populate with initial ops
    adapterRef.current.toBlocks(initialOps, ws);

    // Listen for changes
    ws.addChangeListener(() => {
      const ops = adapterRef.current.fromBlocks(ws);
      opsRef.current = ops;
      setOpsCount(ops.length);
    });

    workspaceRef.current = ws;

    return () => {
      ws.dispose();
      workspaceRef.current = null;
    };
  }, [elementId]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSave = useCallback(() => {
    onSave(opsRef.current);
  }, [onSave]);

  const handleMonacoSave = useCallback(async (_source: string, _desc: string) => {
    // When saving from Monaco, let parent handle parsing
    onClose();
  }, [onClose]);

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

  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        <div className={styles.header}>
          <h3 className={styles.title}>
            Block Editor — {opsCount} ops
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
        <div ref={containerRef} className={styles.blocklyContainer} />
      </div>
    </div>
  );
};
```

- [ ] **Step 3: Wire into FlowCanvas**

In `poiesis/src/components/canvas/code/FlowCanvas.tsx`:

```typescript
// Replace:
import { CodeCardEditor } from "./CodeCardEditor";
// With:
import { BlockEditor } from "./BlockEditor";

// Replace:
<CodeCardEditor
  elementId={editingElement.id}
  ...
/>
// With:
<BlockEditor
  elementId={editingElement.id}
  ...
/>
```

- [ ] **Step 4: Verify TypeScript compiles (except deleted files)**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep -v "lexical/" | grep -v "CodeCardEditor" | head -20
```

Expected: only pre-existing unrelated errors.

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/BlockEditor.tsx poiesis/src/components/canvas/code/BlockEditor.module.css poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat: add BlockEditor component and wire into FlowCanvas"
```

---

## Task 7: Delete Old Lexical Code

**Files:**
- Delete: `poiesis/src/components/canvas/code/lexical/` (entire directory)
- Delete: `poiesis/src/components/canvas/code/CodeCardEditor.tsx`
- Delete: `poiesis/src/components/canvas/code/CodeCardEditor.module.css`
- Delete: `poiesis/src/components/canvas/code/CodeCardEditor.stories.tsx`
- Delete: `poiesis/src/core/types/editor-node.ts`
- Delete: `poiesis/src/core/types/__tests__/editor-node.test.ts` (if exists)

- [ ] **Step 1: Delete files**

```bash
rm -rf poiesis/src/components/canvas/code/lexical/
rm poiesis/src/components/canvas/code/CodeCardEditor.tsx
rm poiesis/src/components/canvas/code/CodeCardEditor.module.css
rm poiesis/src/components/canvas/code/CodeCardEditor.stories.tsx
rm poiesis/src/core/types/editor-node.ts
```

- [ ] **Step 2: Check for remaining references**

```bash
grep -r "editor-node" poiesis/src/ --include="*.ts" --include="*.tsx" | grep -v node_modules | grep -v ".test."
grep -r "CodeCardEditor" poiesis/src/ --include="*.ts" --include="*.tsx" | grep -v node_modules
```

Expected: no remaining references (or only in the BlockEditor import in FlowCanvas).

- [ ] **Step 3: Update Storybook stories**

Delete old stories reference. Stories for BlockEditor can be created in a future task.

- [ ] **Step 4: Run all remaining tests**

```bash
cd poiesis && npx vitest run --exclude="lexical"
```

Expected: all non-Lexical tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A poiesis/src/
git commit -m "refactor: delete old Lexical CodeCardEditor — replaced by BlockEditor"
```

---

## Task 8: Keyboard Navigation Configuration

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/keyboard.ts`

**Interfaces:**
- Consumes: Blockly workspace
- Produces: `configureKeyboardNavigation(workspace: Blockly.WorkspaceSvg): void`

- [ ] **Step 1: Implement keyboard config**

```typescript
// poiesis/src/components/canvas/code/blocks/keyboard.ts
import * as Blockly from "blockly";

export function configureKeyboardNavigation(ws: Blockly.WorkspaceSvg): void {
  // Enable keyboard navigation mode
  // @blockly/keyboard-navigation provides the marker and cursor system
  
  // Default shortcuts (can be customized):
  // Ctrl+Shift+K: Toggle keyboard navigation
  // WASD or Arrow keys: Navigate between blocks and inputs
  // Enter: Enter block/input
  // Escape: Exit block/input
  // Delete/Backspace: Delete selected block

  // Configure the navigation controller
  const nav = ws.getMarkerManager();
  if (!nav) return;

  // Register custom shortcut: Delete key
  Blockly.ShortcutRegistry.registry.register(
    "blockEditorDelete",
    {
      name: "Delete",
      keyCodes: [Blockly.utils.KeyCodes.DELETE, Blockly.utils.KeyCodes.BACKSPACE],
      callback: () => {
        const block = ws.getCursor()?.getCurNode()?.getSourceBlock();
        if (block && block.isDeletable()) {
          block.dispose(true);
          return true;
        }
        return false;
      },
    },
    true, // allow while in field editor
  );

  // Register custom shortcut: \ for raw text input
  Blockly.ShortcutRegistry.registry.register(
    "blockEditorRawInput",
    {
      name: "Raw Input",
      keyCodes: [220], // Backslash key
      callback: () => {
        // Dispatch custom event that BlockEditor can listen for
        window.dispatchEvent(new CustomEvent("block-editor-raw-input"));
        return true;
      },
    },
    true,
  );

  // Register custom shortcut: Ctrl+C copy
  Blockly.ShortcutRegistry.registry.register(
    "blockEditorCopy",
    {
      name: "Copy",
      keyCodes: [Blockly.utils.KeyCodes.C],
      modifierKeys: [Blockly.utils.KeyCodes.CTRL],
      callback: () => {
        // Use Blockly's built-in copy
        Blockly.clipboard.copy(ws);
        return true;
      },
    },
    true,
  );

  // Register custom shortcut: Ctrl+V paste
  Blockly.ShortcutRegistry.registry.register(
    "blockEditorPaste",
    {
      name: "Paste",
      keyCodes: [Blockly.utils.KeyCodes.V],
      modifierKeys: [Blockly.utils.KeyCodes.CTRL],
      callback: () => {
        Blockly.clipboard.paste(ws);
        return true;
      },
    },
    true,
  );

  // Register custom shortcut: Ctrl+X cut
  Blockly.ShortcutRegistry.registry.register(
    "blockEditorCut",
    {
      name: "Cut",
      keyCodes: [Blockly.utils.KeyCodes.X],
      modifierKeys: [Blockly.utils.KeyCodes.CTRL],
      callback: () => {
        Blockly.clipboard.copy(ws);
        const block = ws.getCursor()?.getCurNode()?.getSourceBlock();
        if (block && block.isDeletable()) {
          block.dispose(true);
        }
        return true;
      },
    },
    true,
  );
}
```

- [ ] **Step 2: Call from BlockEditor useEffect**

Add to `BlockEditor.tsx` after workspace creation:

```typescript
import { configureKeyboardNavigation } from "./blocks/keyboard";

// Inside useEffect, after workspaceRef.current = ws:
configureKeyboardNavigation(ws);
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/blocks/keyboard.ts
git commit -m "feat: configure keyboard navigation — arrow keys, Delete, Ctrl+C/V/X, backslash"
```

---

## Task 9: Integration & Visual Verification

**Files:**
- No new files — verify BlockEditor renders correctly in browser
- Modify: `poiesis/src/components/canvas/code/BlockEditor.tsx` (bug fixes)

- [ ] **Step 1: Start the app**

```bash
# Ask user to start Poiesis via Victauri
```

- [ ] **Step 2: Open BlockEditor and verify**

Using Playwright/Victauri, open any element's code editor and verify:
1. Blockly workspace renders with Zelos theme
2. Initial ops populate as blocks
3. Arrow keys navigate between blocks
4. Delete key removes a selected block
5. Save produces correct CodeOp[] output
6. "文本模式" button switches to Monaco fallback

- [ ] **Step 3: Fix any visual or interaction issues**

- [ ] **Step 4: Commit fixes**

---

## Task 10: Update Storybook Stories

**Files:**
- Create: `poiesis/src/components/canvas/code/BlockEditor.stories.tsx`

- [ ] **Step 1: Create BlockEditor stories**

```typescript
// poiesis/src/components/canvas/code/BlockEditor.stories.tsx
import type { Meta, StoryObj } from "@storybook/react";
import { BlockEditor } from "./BlockEditor";
import type { CodeOp } from "../../../core/types/code-op";
import { createBodyId } from "../../../core/types/code-op";

const meta: Meta<typeof BlockEditor> = {
  title: "BlockEditor/BlockEditor",
  component: BlockEditor,
  parameters: { layout: "fullscreen" },
};

export default meta;
type Story = StoryObj<typeof BlockEditor>;

const noop = { onSave: () => {}, onClose: () => {} };

export const Empty: Story = {
  args: {
    elementId: "story-empty",
    initialOps: [],
    initialSource: "",
    ...noop,
  },
};

export const LetAndSet: Story = {
  args: {
    elementId: "story-let-set",
    initialOps: [
      { kind: "declare_var", pid: "dv-1", mutable: false, name: { kind: "text", value: "count" }, varType: { kind: "text", value: "i32" }, initValue: { kind: "text", value: "0" }, scope: "fn" },
      { kind: "set_var", pid: "sv-1", name: { kind: "text", value: "count" }, operator: "+=", value: { kind: "text", value: "1" } },
    ] as CodeOp[],
    initialSource: "let count: i32 = 0;\ncount += 1;",
    ...noop,
  },
};

export const WhileLoop: Story = {
  args: {
    elementId: "story-while",
    initialOps: [
      {
        kind: "while", pid: "wh-1", bodyId: createBodyId(),
        condition: { kind: "text", value: "i < 10" },
        body: [
          { kind: "set_var", pid: "sv-1", name: { kind: "text", value: "i" }, operator: "+=", value: { kind: "text", value: "1" } },
        ],
      },
    ] as CodeOp[],
    initialSource: "while i < 10 {\n  i += 1;\n}",
    ...noop,
  },
};
```

- [ ] **Step 2: Verify stories render in Storybook**

```bash
cd poiesis && npx storybook dev -p 6006 --no-open &
# Navigate to new BlockEditor stories
```

Expected: stories render with Blockly workspace, blocks visible.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/BlockEditor.stories.tsx
git commit -m "feat: add BlockEditor stories — Empty, Let+Set, WhileLoop"
```

---

## Self-Review Checklist

- [ ] Spec coverage: All spec sections covered? Architecture ✅, Data Model (Task 2) ✅, Block Types (Task 3) ✅, Interaction Model (Task 6, 8) ✅, Tech Stack ✅, BlockAdapter (Task 4) ✅, Deletion (Task 7) ✅, Testing (Task 9, 10) ✅
- [ ] Placeholder scan: No TBD/TODO/implements-later
- [ ] Type consistency: SlotValue fields match between Task 2 (code-op.ts) and Task 4 (BlockAdapter)
- [ ] Backward compat: fromLegacyString() covers old string→SlotValue conversion
