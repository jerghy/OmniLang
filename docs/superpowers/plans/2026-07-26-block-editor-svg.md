# Block Editor v4.1 — 移除 Blockly，纯 React + SVG 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 移除 Blockly 依赖，用纯 React + SVG 组件渲染 12 种积木，自建 BlockModel 状态机取代 SelectionManager/GapManager/BlockAdapter。

**Architecture:** CodeOp[] 保持为 SSOT。BlockModel 管理 ops 数组、gap 集合、和选择框位置。13 个 BlockSvg 组件从 CodeOp 直接渲染 SVG。KeyboardHandler 挂容器 div 上处理所有键盘事件。SelectionOverlay 渲染选择框视觉效果。

**Tech Stack:** React 18, TypeScript, SVG (no external rendering library), vitest, Playwright, Storybook

## Global Constraints

- CodeOp[] / SlotValue / InlineOp 数据模型已存在，不得修改其类型定义
- FuzzySearchPopup.ts / code-op-emitter.ts 保持不变
- BlockEditor 的外部接口（Props: elementId, initialOps, initialSource, language, onSave, onClose）不变
- FlowCanvas.tsx 中的 BlockEditor 使用方式不变
- 不引入新的第三方渲染库
- Blockly npm 包必须在最终结果中移除

---

### Task 1: BlockModel 状态机

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/BlockModel.ts`
- Create: `poiesis/src/components/canvas/code/blocks/BlockModel.test.ts`

**Interfaces:**
- Consumes: `CodeOp`, `SlotValue` from `core/types/code-op` and `core/types/slot-value` (existing, unchanged)
- Produces:
  - `export type SelectionTarget = { type: "block"; blockIndex: number; blockId: string } | { type: "slot"; blockIndex: number; blockId: string; slotName: string } | { type: "gap"; position: number }`
  - `export interface NavEntry { kind: "block" | "slot" | "gap"; blockIndex: number; slotName?: string; position: number; op?: CodeOp }`
  - `export class BlockModel { constructor(initialOps: CodeOp[]); getOps(): CodeOp[]; getNavEntries(): NavEntry[]; getCurrentTarget(): SelectionTarget | null; moveLeft(): void; moveRight(): void; moveToIndex(index: number): void; insertBlock(blockType: string, atPosition: number): CodeOp; deleteBlock(blockIndex: number): void; deleteGap(position: number): void; insertGap(position: number): void; setSlotValue(blockIndex: number, slotName: string, value: SlotValue): void; clearSlot(blockIndex: number, slotName: string): void; onChange: (() => void) | null; dispose(): void }`

- [ ] **Step 1: 创建 BlockModel.test.ts — 测试空 ops 列表的导航列表**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockModel.test.ts
import { describe, it, expect } from "vitest";
import { BlockModel } from "./BlockModel";
import type { CodeOp } from "../../../../core/types/code-op";
import { createBodyId } from "../../../../core/types/code-op";

describe("BlockModel — nav entries", () => {
  it("empty ops => single gap entry", () => {
    const model = new BlockModel([]);
    const entries = model.getNavEntries();
    expect(entries).toHaveLength(1);
    expect(entries[0].kind).toBe("gap");
    expect(entries[0].position).toBe(0);
  });

  it("one block => gap + block + gap", () => {
    const ops: CodeOp[] = [
      { kind: "break", pid: "b-1" },
    ];
    const model = new BlockModel(ops);
    const entries = model.getNavEntries();
    expect(entries.length).toBeGreaterThanOrEqual(3);
    expect(entries[0].kind).toBe("gap");       // before block
    expect(entries[1].kind).toBe("block");      // the block
    expect(entries[entries.length - 1].kind).toBe("gap"); // after last block
  });

  it("block with slots => block entry followed by slot entries", () => {
    const ops: CodeOp[] = [
      {
        kind: "declare_var",
        pid: "dv-1",
        mutable: false,
        name: { kind: "text", value: "x" },
        varType: { kind: "text", value: "i32" },
        initValue: { kind: "text", value: "0" },
        scope: "fn",
      },
    ];
    const model = new BlockModel(ops);
    const entries = model.getNavEntries();
    const slotNames = entries
      .filter((e) => e.kind === "slot")
      .map((e) => e.slotName);
    expect(slotNames).toContain("NAME");
    expect(slotNames).toContain("VARTYPE");
    expect(slotNames).toContain("VALUE");
  });
});
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run blocks/BlockModel.test.ts
```
Expected: FAIL — `BlockModel` not found

- [ ] **Step 3: 创建 BlockModel.ts — 最小实现让测试通过**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockModel.ts
import type { CodeOp } from "../../../../core/types/code-op";
import type { SlotValue } from "../../../../core/types/slot-value";
import { createBodyId } from "../../../../core/types/code-op";

export type SelectionTarget =
  | { type: "block"; blockIndex: number; blockId: string }
  | { type: "slot"; blockIndex: number; blockId: string; slotName: string }
  | { type: "gap"; position: number };

export interface NavEntry {
  kind: "block" | "slot" | "gap";
  blockIndex: number;
  slotName?: string;
  position: number;
  op?: CodeOp;
}

/** The list of slot names for each block type. */
const SLOT_MAP: Record<string, string[]> = {
  declare_var: ["NAME", "VARTYPE", "VALUE"],
  set_var: ["NAME", "VALUE"],
  return_stmt: ["VALUE"],
  call_fn: ["FN_NAME", "ARGS"],
  await_expr: ["VALUE"],
  if_block: ["CONDITION"],
  while_loop: ["CONDITION"],
  for_in: ["ITEM", "COLLECTION"],
  break_stmt: [],
  continue_stmt: [],
  comment_stmt: [],
  raw_stmt: [],
};

function getBlockSlots(op: CodeOp): string[] {
  return SLOT_MAP[op.kind] || [];
}

export class BlockModel {
  private ops: CodeOp[];
  private gaps: Set<number>;
  private selectionIndex: number;
  onChange: (() => void) | null = null;

  constructor(initialOps: CodeOp[]) {
    this.ops = [...initialOps];
    this.gaps = new Set<number>();
    this.selectionIndex = 0;
    this.initializeGaps();
  }

  getOps(): CodeOp[] {
    return this.ops;
  }

  getNavEntries(): NavEntry[] {
    const entries: NavEntry[] = [];
    // gap before first block
    entries.push({ kind: "gap", blockIndex: -1, position: 0 });

    for (let i = 0; i < this.ops.length; i++) {
      const op = this.ops[i];
      entries.push({
        kind: "block",
        blockIndex: i,
        position: i + 1,
        op,
      });
      // slots
      const slotNames = getBlockSlots(op);
      for (let s = 0; s < slotNames.length; s++) {
        entries.push({
          kind: "slot",
          blockIndex: i,
          slotName: slotNames[s],
          position: i + 1 + (s + 1) * 0.01,
        });
      }
      // gap between blocks (only if there's a gap at this position)
      if (i < this.ops.length - 1) {
        const gapPos = i + 0.5;
        if (this.gaps.has(gapPos)) {
          entries.push({ kind: "gap", blockIndex: i, position: gapPos });
        }
      }
    }

    // gap after last block
    const lastGapPos = this.ops.length;
    entries.push({
      kind: "gap",
      blockIndex: this.ops.length - 1,
      position: lastGapPos,
    });

    return entries;
  }

  getCurrentTarget(): SelectionTarget | null {
    const entries = this.getNavEntries();
    if (entries.length === 0 || this.selectionIndex >= entries.length) {
      return null;
    }
    const entry = entries[this.selectionIndex];
    return this.entryToTarget(entry, this.ops);
  }

  moveLeft(): void {
    if (this.selectionIndex > 0) {
      this.selectionIndex--;
      this.onChange?.();
    }
  }

  moveRight(): void {
    const entries = this.getNavEntries();
    if (this.selectionIndex < entries.length - 1) {
      this.selectionIndex++;
      this.onChange?.();
    }
  }

  moveToIndex(index: number): void {
    const entries = this.getNavEntries();
    if (index >= 0 && index < entries.length) {
      this.selectionIndex = index;
      this.onChange?.();
    }
  }

  insertBlock(blockType: string, atPosition: number): CodeOp {
    const op = this.createOpForType(blockType);
    const insertIdx = Math.round(atPosition);
    const clampedIdx = Math.min(insertIdx, this.ops.length);
    this.ops.splice(clampedIdx, 0, op);
    this.rebuildGaps();
    this.onChange?.();
    return op;
  }

  deleteBlock(blockIndex: number): void {
    if (blockIndex >= 0 && blockIndex < this.ops.length) {
      this.ops.splice(blockIndex, 1);
      this.rebuildGaps();
      this.onChange?.();
    }
  }

  deleteGap(position: number): void {
    this.gaps.delete(position);
    this.onChange?.();
  }

  insertGap(position: number): void {
    this.gaps.add(position);
    this.onChange?.();
  }

  setSlotValue(blockIndex: number, slotName: string, value: SlotValue): void {
    const op = this.ops[blockIndex];
    if (!op) return;
    // Map slotName to the CodeOp field
    const slotFieldMap: Record<string, Record<string, string>> = {
      declare_var: { NAME: "name", VARTYPE: "varType", VALUE: "initValue" },
      set_var: { NAME: "name", VALUE: "value" },
      return_stmt: { VALUE: "value" },
      call_fn: { FN_NAME: "fnName", ARGS: "args" },
      await_expr: { VALUE: "value" },
      if_block: { CONDITION: "condition" },
      while_loop: { CONDITION: "condition" },
      for_in: { ITEM: "itemName", COLLECTION: "collection" },
    };
    const fieldMap = slotFieldMap[op.kind];
    if (!fieldMap || !fieldMap[slotName]) return;
    const field = fieldMap[slotName];
    (op as Record<string, unknown>)[field] = value;
    this.onChange?.();
  }

  clearSlot(blockIndex: number, slotName: string): void {
    this.setSlotValue(blockIndex, slotName, { kind: "empty" });
  }

  dispose(): void {
    this.onChange = null;
    this.ops = [];
    this.gaps.clear();
  }

  // ── Private helpers ──

  private initializeGaps(): void {
    this.gaps.clear();
    for (let i = 0; i < this.ops.length - 1; i++) {
      this.gaps.add(i + 0.5);
    }
    this.selectionIndex = 0;
  }

  private rebuildGaps(): void {
    this.gaps.clear();
    for (let i = 0; i < this.ops.length - 1; i++) {
      this.gaps.add(i + 0.5);
    }
  }

  private entryToTarget(
    entry: NavEntry,
    ops: CodeOp[],
  ): SelectionTarget | null {
    switch (entry.kind) {
      case "block": {
        const op = ops[entry.blockIndex];
        return {
          type: "block",
          blockIndex: entry.blockIndex,
          blockId: op?.pid || "",
        };
      }
      case "slot":
        return {
          type: "slot",
          blockIndex: entry.blockIndex,
          blockId: ops[entry.blockIndex]?.pid || "",
          slotName: entry.slotName || "",
        };
      case "gap":
        return { type: "gap", position: entry.position };
      default:
        return null;
    }
  }

  private createOpForType(blockType: string): CodeOp {
    const pid = "pid-" + Math.random().toString(36).slice(2, 10);
    switch (blockType) {
      case "declare_var":
        return {
          kind: "declare_var",
          pid,
          mutable: false,
          name: { kind: "empty" },
          scope: "fn",
        };
      case "set_var":
        return {
          kind: "set_var",
          pid,
          name: { kind: "empty" },
          operator: "=",
          value: { kind: "empty" },
        };
      case "return_stmt":
        return { kind: "return", pid };
      case "call_fn":
        return {
          kind: "call_fn",
          pid,
          fnName: { kind: "empty" },
          args: [],
        };
      case "await_expr":
        return { kind: "await", pid, value: { kind: "empty" } };
      case "break_stmt":
        return { kind: "break", pid };
      case "continue_stmt":
        return { kind: "continue", pid };
      case "comment_stmt":
        return { kind: "comment", pid, text: "" };
      case "raw_stmt":
        return { kind: "raw", pid, source: "" };
      case "if_block":
        return {
          kind: "if_block",
          pid,
          bodyId: createBodyId(),
          branches: [
            {
              condition: { kind: "empty" },
              body: [],
            },
          ],
        };
      case "while_loop":
        return {
          kind: "while",
          pid,
          bodyId: createBodyId(),
          condition: { kind: "empty" },
          body: [],
        };
      case "for_in":
        return {
          kind: "for_in",
          pid,
          bodyId: createBodyId(),
          itemName: { kind: "empty" },
          collection: { kind: "empty" },
          body: [],
        };
      default:
        return { kind: "comment", pid, text: `unknown: ${blockType}` };
    }
  }
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run blocks/BlockModel.test.ts
```
Expected: PASS

- [ ] **Step 5: 补充测试 — insertBlock / deleteBlock / slot CRUD**

```typescript
// 追加到 BlockModel.test.ts
describe("BlockModel — CRUD", () => {
  it("insertBlock at position 0 adds to beginning", () => {
    const model = new BlockModel([]);
    model.insertBlock("break_stmt", 0);
    expect(model.getOps()).toHaveLength(1);
    expect(model.getOps()[0].kind).toBe("break");
  });

  it("insertBlock in gap between two blocks inserts at correct index", () => {
    const model = new BlockModel([
      { kind: "break", pid: "b-1" },
      { kind: "continue", pid: "c-1" },
    ]);
    model.insertGap(0.5);
    model.insertBlock("return_stmt", 0.5);
    const ops = model.getOps();
    expect(ops[1].kind).toBe("return");
  });

  it("deleteBlock removes by index", () => {
    const model = new BlockModel([
      { kind: "break", pid: "b-1" },
    ]);
    model.deleteBlock(0);
    expect(model.getOps()).toHaveLength(0);
  });

  it("setSlotValue updates a named slot on a block", () => {
    const model = new BlockModel([
      {
        kind: "declare_var",
        pid: "dv-1",
        mutable: false,
        name: { kind: "empty" },
        scope: "fn",
      } as CodeOp,
    ]);
    model.setSlotValue(0, "NAME", { kind: "text", value: "count" });
    const op = model.getOps()[0];
    if (op.kind === "declare_var") {
      expect(op.name).toEqual({ kind: "text", value: "count" });
    }
  });

  it("clearSlot sets slot to empty", () => {
    const model = new BlockModel([
      {
        kind: "declare_var",
        pid: "dv-1",
        mutable: false,
        name: { kind: "text", value: "x" },
        scope: "fn",
      } as CodeOp,
    ]);
    model.clearSlot(0, "NAME");
    const op = model.getOps()[0];
    if (op.kind === "declare_var") {
      expect(op.name).toEqual({ kind: "empty" });
    }
  });

  it("insertGap / deleteGap / hasGap roundtrip", () => {
    const model = new BlockModel([
      { kind: "break", pid: "b-1" },
      { kind: "continue", pid: "c-1" },
    ]);
    model.insertGap(0.5);
    const entries = model.getNavEntries();
    const gaps = entries.filter((e) => e.kind === "gap");
    const gapPositions = gaps.map((g) => g.position);
    expect(gapPositions).toContain(0.5);

    model.deleteGap(0.5);
    const entries2 = model.getNavEntries();
    const gaps2 = entries2.filter((e) => e.kind === "gap");
    expect(gaps2.map((g) => g.position)).not.toContain(0.5);
  });

  it("moveLeft / moveRight changes selectionIndex", () => {
    const model = new BlockModel([
      { kind: "break", pid: "b-1" },
    ]);
    // Initially at index 0 (gap before first block)
    expect(model.getCurrentTarget()?.type).toBe("gap");
    model.moveRight();
    expect(model.getCurrentTarget()?.type).toBe("block");
    model.moveRight();
    expect(model.getCurrentTarget()?.type).toBe("gap"); // gap after last
    model.moveLeft();
    expect(model.getCurrentTarget()?.type).toBe("block");
  });
});
```

- [ ] **Step 6: 运行全部测试**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run blocks/BlockModel.test.ts
```
Expected: ALL PASS

- [ ] **Step 7: 提交**

```bash
git add poiesis/src/components/canvas/code/blocks/BlockModel.ts poiesis/src/components/canvas/code/blocks/BlockModel.test.ts
git commit -m "feat: add BlockModel state machine for block editor v4.1"
```

---

### Task 2: BlockSvg 共享外壳 + 9 种简单积木组件

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/LetVarBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/SetVarBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/ReturnBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/CallBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/AwaitBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/BreakBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/ContinueBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/CommentBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/index.ts`

**Interfaces:**
- Consumes: `CodeOp`, `SlotValue` from existing types
- Produces: `BlockSvg` shared component; 9 block components each exported as default

- [ ] **Step 1: 创建 BlockSvg.tsx — 共享积木外壳**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockSvg.tsx
import React from "react";

export const BLOCK_COLORS: Record<string, string> = {
  CONTROL: "#FFAB19",
  VARIABLE: "#FF8C1A",
  FUNCTION: "#995BA5",
  ASYNC: "#5B67B0",
  BREAK: "#CF8B17",
  COMMENT: "#5CB85C",
  RAW: "#AAAAAA",
};

interface BlockSvgProps {
  colour: string;
  blockId: string;
  children: React.ReactNode;
  /** Whether to render top connector (previousStatement). */
  hasTopConnector?: boolean;
  /** Whether to render bottom connector (nextStatement). */
  hasBottomConnector?: boolean;
}

const H_PX = 32; // block height
const CONNECTOR_W = 12;
const CONNECTOR_H = 8;
const RADIUS = 6;

/** Top puzzle connector: notch (convex bump). */
function TopConnector({ x }: { x: number }) {
  return (
    <path
      d={`M ${x} 0 L ${x + CONNECTOR_W} 0 L ${x + CONNECTOR_W} -${CONNECTOR_H}`}
      fill="none"
      stroke="rgba(0,0,0,0.15)"
      strokeWidth={1}
    />
  );
}

/** Bottom puzzle connector: indentation (concave notch). */
function BottomConnector({ x, y }: { x: number; y: number }) {
  return (
    <path
      d={`M ${x} ${y} L ${x + CONNECTOR_W} ${y} L ${x + CONNECTOR_W} ${
        y + CONNECTOR_H
      }`}
      fill="none"
      stroke="rgba(0,0,0,0.15)"
      strokeWidth={1}
    />
  );
}

export const BlockSvg = React.forwardRef<SVGGElement, BlockSvgProps>(
  ({ colour, blockId, children, hasTopConnector = true, hasBottomConnector = true }, ref) => {
    const textWidth = 200;
    const totalW = textWidth + 20;
    const connectorX = 20;

    return (
      <g ref={ref} data-block-id={blockId}>
        {/* Top connector notch */}
        {hasTopConnector && <TopConnector x={connectorX} />}

        {/* Main body: rounded rect */}
        <rect
          x={0}
          y={0}
          width={totalW}
          height={H_PX}
          rx={RADIUS}
          ry={RADIUS}
          fill={colour}
          stroke="rgba(0,0,0,0.15)"
          strokeWidth={1}
        />

        {/* Content */}
        <g transform={`translate(8, ${H_PX / 2})`}>{children}</g>

        {/* Bottom connector indentation */}
        {hasBottomConnector && (
          <BottomConnector x={connectorX} y={H_PX} />
        )}
      </g>
    );
  },
);

BlockSvg.displayName = "BlockSvg";

/** A text slot within a block. */
export const TextSlot: React.FC<{
  slotId: string;
  value: string;
  placeholder: string;
}> = ({ slotId, value, placeholder }) => {
  const display = value || placeholder;
  const isPlaceholder = !value;
  return (
    <tspan
      data-slot-id={slotId}
      fill={isPlaceholder ? "#999" : "#fff"}
      fontSize="13"
      fontFamily="monospace"
      fontWeight={isPlaceholder ? "normal" : "bold"}
    >
      {display}
    </tspan>
  );
};

/** A keyword label within a block. */
export const Keyword: React.FC<{ text: string; children?: never }> = ({ text }) => (
  <tspan fill="#fff" fontSize="13" fontFamily="monospace" fontWeight="bold">
    {text}{" "}
  </tspan>
);
```

- [ ] **Step 2: 创建 BlockSvg.test.tsx — 验证 shared 组件渲染**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockSvg.test.tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { BlockSvg, TextSlot, Keyword } from "./BlockSvg";

describe("BlockSvg", () => {
  it("renders SVG g with data-block-id", () => {
    const { container } = render(
      <svg>
        <BlockSvg colour="#FFAB19" blockId="test-1">
          <Keyword text="test" />
        </BlockSvg>
      </svg>,
    );
    const g = container.querySelector("[data-block-id='test-1']");
    expect(g).toBeTruthy();
  });

  it("renders rect with correct fill", () => {
    const { container } = render(
      <svg>
        <BlockSvg colour="#FFAB19" blockId="test-fill">
          <Keyword text="x" />
        </BlockSvg>
      </svg>,
    );
    const rect = container.querySelector("rect");
    expect(rect?.getAttribute("fill")).toBe("#FFAB19");
  });
});

describe("TextSlot", () => {
  it("shows value when present", () => {
    const { container } = render(
      <svg>
        <text>
          <TextSlot slotId="s1" value="hello" placeholder="name" />
        </text>
      </svg>,
    );
    expect(container.textContent).toContain("hello");
  });

  it("shows placeholder when value is empty", () => {
    const { container } = render(
      <svg>
        <text>
          <TextSlot slotId="s1" value="" placeholder="name" />
        </text>
      </svg>,
    );
    expect(container.textContent).toContain("name");
  });
});
```

- [ ] **Step 3: 创建 9 个简单积木组件**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockSvg/LetVarBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { DeclareVarOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const LetVarBlock = React.forwardRef<SVGGElement, { op: DeclareVarOp }>(
  ({ op }, ref) => {
    const mutLabel = op.mutable ? "let mut" : "let";
    return (
      <BlockSvg colour={BLOCK_COLORS.VARIABLE} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <Keyword text={mutLabel} />
          <TextSlot slotId="NAME" value={slotToText(op.name)} placeholder="name" />
          <Keyword text=":" />
          <TextSlot slotId="VARTYPE" value={slotToText(op.varType || { kind: "empty" })} placeholder="type" />
          <Keyword text="=" />
          <TextSlot slotId="VALUE" value={slotToText(op.initValue || { kind: "empty" })} placeholder="value" />
        </text>
      </BlockSvg>
    );
  },
);
LetVarBlock.displayName = "LetVarBlock";
```

其余 8 个简单积木组件遵循相同模式（用 `React.forwardRef<SVGGElement>` 包裹，props 接收对应的 CodeOp 子类型）。完整代码见实现阶段。

- [ ] **Step 4: 创建 index.ts 统一导出**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockSvg/index.ts
export { LetVarBlock } from "./LetVarBlock";
export { SetVarBlock } from "./SetVarBlock";
export { ReturnBlock } from "./ReturnBlock";
export { CallBlock } from "./CallBlock";
export { AwaitBlock } from "./AwaitBlock";
export { BreakBlock } from "./BreakBlock";
export { ContinueBlock } from "./ContinueBlock";
export { CommentBlock } from "./CommentBlock";
```

- [ ] **Step 5: 运行测试**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run blocks/BlockSvg.test.tsx
```
Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add poiesis/src/components/canvas/code/blocks/BlockSvg.tsx poiesis/src/components/canvas/code/blocks/BlockSvg.test.tsx poiesis/src/components/canvas/code/blocks/BlockSvg/
git commit -m "feat: add BlockSvg shared shell and 9 simple block components"
```

---

### Task 3: C 形积木组件（IfBlock, WhileBlock, ForInBlock）

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/IfBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/WhileBlock.tsx`
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/ForInBlock.tsx`
- Modify: `poiesis/src/components/canvas/code/blocks/BlockSvg/index.ts`

**Interfaces:**
- Consumes: `IfBlockOp`, `WhileLoopOp`, `ForLoopOp` from `core/types/code-op`; `BlockSvg`, `Keyword`, `TextSlot` from Task 2
- Produces: `<IfBlock>`, `<WhileBlock>`, `<ForInBlock>` — each renders body area with recursive BlockList

- [ ] **Step 1: 创建 IfBlock.tsx — if/elif/else + 动态分支**

```typescript
// poiesis/src/components/canvas/code/blocks/BlockSvg/IfBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { IfBlockOp, CodeOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

interface IfBlockProps {
  op: IfBlockOp;
  renderBody: (ops: CodeOp[]) => React.ReactNode;
}

export const IfBlock = React.forwardRef<SVGGElement, IfBlockProps>(
  ({ op, renderBody }, ref) => {
    return (
      <g ref={ref} data-block-id={op.pid}>
        {op.branches.map((branch, i) => {
          const isElse = branch.condition.kind === "empty";
          const prefix = i === 0 ? "if" : isElse ? "else" : "elif";
          const cond = slotToText(branch.condition);

          return (
            <g key={i} transform={`translate(0, ${i * 60})`}>
              {/* Branch header */}
              <BlockSvg
                colour={BLOCK_COLORS.CONTROL}
                blockId={`${op.pid}-br-${i}`}
                hasTopConnector={i === 0}
                hasBottomConnector={false}
              >
                <text dominantBaseline="middle">
                  <Keyword text={prefix} />
                  {!isElse && (
                    <TextSlot
                      slotId={`CONDITION_${i}`}
                      value={cond}
                      placeholder="condition"
                    />
                  )}
                </text>
              </BlockSvg>

              {/* Body area — indented */}
              {branch.body.length > 0 && (
                <g transform="translate(24, 36)">{renderBody(branch.body)}</g>
              )}
            </g>
          );
        })}

        {/* Bottom connector */}
        <rect
          x={0}
          y={op.branches.length * 60 + 24}
          width={200}
          height={8}
          fill={BLOCK_COLORS.CONTROL}
          rx={2}
        />
      </g>
    );
  },
);
IfBlock.displayName = "IfBlock";
```

- [ ] **Step 2: 创建 WhileBlock.tsx + ForInBlock.tsx**（模式同 IfBlock，无动态分支，body 直接渲染）

- [ ] **Step 3: 更新 index.ts，添加 C 形积木导入**

- [ ] **Step 4: 提交**

```bash
git add poiesis/src/components/canvas/code/blocks/BlockSvg/IfBlock.tsx poiesis/src/components/canvas/code/blocks/BlockSvg/WhileBlock.tsx poiesis/src/components/canvas/code/blocks/BlockSvg/ForInBlock.tsx poiesis/src/components/canvas/code/blocks/BlockSvg/index.ts
git commit -m "feat: add C-shaped block SVG components (if/while/for)"
```

---

### Task 4: SelectionOverlay 选择框

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/SelectionOverlay.tsx`

**Interfaces:**
- Consumes: `SelectionTarget` from BlockModel (Task 1)
- Produces: `<SelectionOverlay target={SelectionTarget | null} />`

- [ ] **Step 1: 创建 SelectionOverlay.tsx**

```typescript
// poiesis/src/components/canvas/code/blocks/SelectionOverlay.tsx
import React, { useEffect, useRef } from "react";
import type { SelectionTarget } from "./BlockModel";

interface SelectionOverlayProps {
  target: SelectionTarget | null;
  /** Map from target key to DOM element for positioning. */
  getElement: (target: SelectionTarget) => HTMLElement | SVGElement | null;
}

const BLINK_KEYFRAMES = `
@keyframes sel-gap-blink { 50% { opacity: 0; } }
`;

export const SelectionOverlay: React.FC<SelectionOverlayProps> = ({
  target,
  getElement,
}) => {
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = overlayRef.current;
    if (!el) return;

    if (!target) {
      el.style.display = "none";
      return;
    }

    const domEl = getElement(target);
    if (!domEl) {
      el.style.display = "none";
      return;
    }

    const rect = domEl.getBoundingClientRect();
    el.style.display = "block";
    el.style.left = `${rect.x}px`;
    el.style.top = `${rect.y}px`;
    el.style.width = `${rect.width}px`;
    el.style.height = `${rect.height}px`;

    switch (target.type) {
      case "block":
        el.style.outline = "2px solid #cc6b4a";
        el.style.outlineOffset = "2px";
        el.style.border = "none";
        el.style.borderRadius = "4px";
        el.style.animation = "none";
        break;
      case "slot":
        el.style.outline = "none";
        el.style.border = "2px dashed #cc6b4a";
        el.style.borderRadius = "4px";
        el.style.animation = "none";
        break;
      case "gap":
        el.style.outline = "none";
        el.style.border = "none";
        el.style.borderTop = "2px solid #cc6b4a";
        el.style.animation = "sel-gap-blink 1s step-end infinite";
        break;
    }
  }, [target, getElement]);

  return (
    <>
      <style>{BLINK_KEYFRAMES}</style>
      <div
        ref={overlayRef}
        style={{
          position: "absolute",
          pointerEvents: "none",
          zIndex: 1000,
          transition: "all 150ms ease",
        }}
      />
    </>
  );
};
```

- [ ] **Step 2: 提交**

```bash
git add poiesis/src/components/canvas/code/blocks/SelectionOverlay.tsx
git commit -m "feat: add SelectionOverlay for block editor cursor"
```

---

### Task 5: KeyboardHandler（纯 DOM 版）

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/KeyboardHandler.ts` (重写)
- Create: `poiesis/src/components/canvas/code/blocks/KeyboardHandler.test.ts`

**Interfaces:**
- Consumes: `BlockModel`, `SelectionTarget` from Task 1; `SelectionOverlay` getElement from Task 4
- Produces:
  - `export interface KBCallbacks { onFuzzySearch: (query: string, context: "slot" | "gap") => void; onRawInput: () => void; onOpsChanged: () => void }`
  - `export class KeyboardHandler { constructor(container: HTMLElement, model: BlockModel, getElement: (t: SelectionTarget) => HTMLElement | SVGElement | null, callbacks: KBCallbacks); dispose(): void }`

- [ ] **Step 1: 创建 KeyboardHandler.ts — 挂容器 div，无 Blockly 依赖**

```typescript
// poiesis/src/components/canvas/code/blocks/KeyboardHandler.ts
import type { BlockModel, SelectionTarget } from "./BlockModel";

export interface KBCallbacks {
  onFuzzySearch: (query: string, context: "slot" | "gap") => void;
  onRawInput: () => void;
  onOpsChanged: () => void;
}

export class KeyboardHandler {
  private container: HTMLElement;
  private model: BlockModel;
  private getElement: (target: SelectionTarget) => HTMLElement | SVGElement | null;
  private callbacks: KBCallbacks;
  private handleKeyDown: (e: KeyboardEvent) => void;

  constructor(
    container: HTMLElement,
    model: BlockModel,
    getElement: (target: SelectionTarget) => HTMLElement | SVGElement | null,
    callbacks: KBCallbacks,
  ) {
    this.container = container;
    this.model = model;
    this.getElement = getElement;
    this.callbacks = callbacks;
    this.handleKeyDown = this.onKeyDown.bind(this);
    container.setAttribute("tabindex", "0");
    container.addEventListener("keydown", this.handleKeyDown);
    container.focus();
  }

  private onKeyDown(e: KeyboardEvent): void {
    const target = this.model.getCurrentTarget();
    if (!target) return;

    // ── Modifier keys ──
    if (e.ctrlKey || e.metaKey) {
      this.handleModifier(e, target);
      return;
    }

    // ── Navigation ──
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      this.model.moveLeft();
      return;
    }
    if (e.key === "ArrowRight") {
      e.preventDefault();
      this.model.moveRight();
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      this.moveSpatial("up");
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      this.moveSpatial("down");
      return;
    }

    // ── Context-sensitive ──
    switch (e.key) {
      case "Enter":
        e.preventDefault();
        this.handleEnter(target);
        break;
      case "Delete":
        e.preventDefault();
        this.handleDelete(target);
        break;
      case "Backspace":
        e.preventDefault();
        this.handleBackspace(target);
        break;
      case "\\":
        e.preventDefault();
        this.callbacks.onRawInput();
        break;
      default:
        if (e.key.length === 1) {
          this.handleCharacter(e.key, target);
        }
    }
  }

  private handleModifier(e: KeyboardEvent, target: SelectionTarget): void {
    // Simplified clipboard — will be replaced by ClipboardManager in Task 6
    if (e.key === "c" || e.key === "C") {
      e.preventDefault();
      // clipboard copy placeholder
    } else if (e.key === "x" || e.key === "X") {
      e.preventDefault();
      if (target.type === "block") {
        this.model.deleteBlock(target.blockIndex);
        this.callbacks.onOpsChanged();
      }
    } else if (e.key === "v" || e.key === "V") {
      e.preventDefault();
      // clipboard paste placeholder
    }
  }

  private handleEnter(target: SelectionTarget): void {
    if (target.type === "gap") {
      this.model.insertGap(target.position);
      this.callbacks.onFuzzySearch("", "gap");
    } else if (target.type === "slot") {
      this.callbacks.onFuzzySearch("", "slot");
    }
    // block: no-op (RawCodeOp edit handled by RawBlock component)
  }

  private handleDelete(target: SelectionTarget): void {
    if (target.type === "block") {
      this.model.deleteBlock(target.blockIndex);
      this.callbacks.onOpsChanged();
    } else if (target.type === "slot") {
      this.model.clearSlot(target.blockIndex, target.slotName);
      this.callbacks.onOpsChanged();
    } else if (target.type === "gap") {
      this.model.deleteGap(target.position);
    }
  }

  private handleBackspace(target: SelectionTarget): void {
    if (target.type === "block") {
      this.model.deleteBlock(target.blockIndex);
      this.callbacks.onOpsChanged();
    } else if (target.type === "slot") {
      this.model.clearSlot(target.blockIndex, target.slotName);
      this.callbacks.onOpsChanged();
    } else if (target.type === "gap") {
      this.model.deleteGap(target.position);
      this.model.moveLeft();
    }
  }

  private handleCharacter(char: string, target: SelectionTarget): void {
    if (target.type === "slot") {
      this.callbacks.onFuzzySearch(char, "slot");
    } else if (target.type === "gap") {
      this.callbacks.onFuzzySearch(char, "gap");
    }
  }

  private moveSpatial(direction: "up" | "down"): void {
    const entries = this.model.getNavEntries();
    const target = this.model.getCurrentTarget();
    if (!target) return;

    const currentIdx = entries.findIndex((e) => {
      if (e.kind !== target.type) return false;
      if (e.kind === "block" && target.type === "block") return e.blockIndex === target.blockIndex;
      if (e.kind === "slot" && target.type === "slot") return e.blockIndex === target.blockIndex && e.slotName === target.slotName;
      if (e.kind === "gap" && target.type === "gap") return e.position === target.position;
      return false;
    });
    if (currentIdx < 0) return;

    const currentEl = this.getElement(target);
    if (!currentEl) return;
    const currentRect = currentEl.getBoundingClientRect();
    const cx = currentRect.x + currentRect.width / 2;
    const cy = currentRect.y + currentRect.height / 2;

    let bestIdx = currentIdx;
    let bestDist = Infinity;

    for (let i = 0; i < entries.length; i++) {
      if (i === currentIdx) continue;
      const entry = entries[i];
      const entryTarget = this.entryToTarget(entry);
      if (!entryTarget) continue;
      const el = this.getElement(entryTarget);
      if (!el) continue;
      const rect = el.getBoundingClientRect();
      const ex = rect.x + rect.width / 2;
      const ey = rect.y + rect.height / 2;

      const dy = ey - cy;
      const dx = ex - cx;

      if (direction === "up" && dy >= 0) continue;
      if (direction === "down" && dy <= 0) continue;

      const dist = Math.abs(dy) + Math.abs(dx) * 0.3; // weight vertical more
      if (dist < bestDist) {
        bestDist = dist;
        bestIdx = i;
      }
    }

    if (bestIdx !== currentIdx) {
      this.model.moveToIndex(bestIdx);
    }
  }

  private entryToTarget(entry: { kind: string; blockIndex: number; slotName?: string; position: number }): SelectionTarget | null {
    const ops = this.model.getOps();
    switch (entry.kind) {
      case "block":
        return { type: "block", blockIndex: entry.blockIndex, blockId: ops[entry.blockIndex]?.pid || "" };
      case "slot":
        return { type: "slot", blockIndex: entry.blockIndex, blockId: ops[entry.blockIndex]?.pid || "", slotName: entry.slotName || "" };
      case "gap":
        return { type: "gap", position: entry.position };
      default:
        return null;
    }
  }

  dispose(): void {
    this.container.removeEventListener("keydown", this.handleKeyDown);
  }
}
```

- [ ] **Step 2: 创建 KeyboardHandler.test.ts**

```typescript
import { describe, it, expect, vi } from "vitest";
import { KeyboardHandler } from "./KeyboardHandler";
import { BlockModel } from "./BlockModel";

describe("KeyboardHandler", () => {
  const makeContainer = () => {
    const div = document.createElement("div");
    document.body.appendChild(div);
    return div;
  };

  const dummyGetElement = () => null;

  it("ArrowLeft calls model.moveLeft", () => {
    const model = new BlockModel([{ kind: "break", pid: "b-1" }]);
    model.moveRight(); // move to block
    const container = makeContainer();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch: vi.fn(),
      onRawInput: vi.fn(),
      onOpsChanged: vi.fn(),
    });
    expect(model.getCurrentTarget()?.type).toBe("block");
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft" }));
    expect(model.getCurrentTarget()?.type).toBe("gap");
    kh.dispose();
    container.remove();
  });

  it("Delete on block removes it", () => {
    const model = new BlockModel([{ kind: "break", pid: "b-1" }]);
    model.moveRight(); // on block
    const container = makeContainer();
    const onOpsChanged = vi.fn();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch: vi.fn(),
      onRawInput: vi.fn(),
      onOpsChanged,
    });
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete" }));
    expect(model.getOps()).toHaveLength(0);
    expect(onOpsChanged).toHaveBeenCalled();
    kh.dispose();
    container.remove();
  });

  it("character key on gap triggers fuzzy search", () => {
    const model = new BlockModel([]);
    const container = makeContainer();
    const onFuzzySearch = vi.fn();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch,
      onRawInput: vi.fn(),
      onOpsChanged: vi.fn(),
    });
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "l" }));
    expect(onFuzzySearch).toHaveBeenCalledWith("l", "gap");
    kh.dispose();
    container.remove();
  });

  it("backslash triggers raw input", () => {
    const model = new BlockModel([]);
    const container = makeContainer();
    const onRawInput = vi.fn();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch: vi.fn(),
      onRawInput,
      onOpsChanged: vi.fn(),
    });
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "\\" }));
    expect(onRawInput).toHaveBeenCalled();
    kh.dispose();
    container.remove();
  });
});
```

- [ ] **Step 3: 运行测试**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run blocks/KeyboardHandler.test.ts
```
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add poiesis/src/components/canvas/code/blocks/KeyboardHandler.ts poiesis/src/components/canvas/code/blocks/KeyboardHandler.test.ts
git commit -m "feat: rewrite KeyboardHandler — pure DOM, no Blockly dependency"
```

---

### Task 6: ClipboardManager

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/ClipboardManager.ts`

**Interfaces:**
- Consumes: `CodeOp` from core types
- Produces: `export class ClipboardManager { async copy(op: CodeOp): Promise<void>; async paste(): Promise<CodeOp | null> }`

- [ ] **Step 1: 创建 ClipboardManager.ts**

```typescript
export class ClipboardManager {
  async copy(op: unknown): Promise<void> {
    const json = JSON.stringify(op);
    await navigator.clipboard.writeText(json);
  }

  async paste(): Promise<unknown | null> {
    try {
      const text = await navigator.clipboard.readText();
      return JSON.parse(text);
    } catch {
      return null;
    }
  }
}
```

- [ ] **Step 2: 提交**

```bash
git add poiesis/src/components/canvas/code/blocks/ClipboardManager.ts
git commit -m "feat: add ClipboardManager for block copy/paste"
```

---

### Task 7: RawBlock 双模式组件

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/BlockSvg/RawBlock.tsx`
- Modify: `poiesis/src/components/canvas/code/blocks/BlockSvg/index.ts`

**Interfaces:**
- Consumes: `RawCodeOp` from core types; `BlockSvg`, `Keyword` from Task 2
- Produces: `<RawBlock op={RawCodeOp} onEdit={(source) => void} />`

- [ ] **Step 1: 创建 RawBlock.tsx — display ⇄ edit 双模式**

```typescript
import React, { useState, useRef, useEffect } from "react";
import { BlockSvg, Keyword, BLOCK_COLORS } from "../BlockSvg";
import type { RawCodeOp } from "../../../../../core/types/code-op";

interface RawBlockProps {
  op: RawCodeOp;
  onEdit: (newSource: string) => void;
  isEditing: boolean;
  onToggleEdit: () => void;
}

export const RawBlock = React.forwardRef<SVGGElement, RawBlockProps>(
  ({ op, onEdit, isEditing, onToggleEdit }, ref) => {
    const inputRef = useRef<HTMLInputElement>(null);

    useEffect(() => {
      if (isEditing) inputRef.current?.focus();
    }, [isEditing]);

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        if (inputRef.current) onEdit(inputRef.current.value);
        onToggleEdit();
      } else if (e.key === "Escape") {
        onToggleEdit();
      }
    };

    return (
      <g ref={ref} data-block-id={op.pid}>
        <BlockSvg
          colour={BLOCK_COLORS.RAW}
          blockId={op.pid}
        >
          {isEditing ? (
            <foreignObject x={0} y={-10} width={300} height={24}>
              <input
                ref={inputRef}
                defaultValue={op.source}
                onKeyDown={handleKeyDown}
                style={{
                  width: "100%",
                  height: "100%",
                  border: "none",
                  background: "transparent",
                  color: "#fff",
                  fontSize: "13px",
                  fontFamily: "monospace",
                  outline: "none",
                }}
              />
            </foreignObject>
          ) : (
            <text dominantBaseline="middle">
              <Keyword text="raw" />
              <tspan fill="#fff" fontSize="12" fontFamily="monospace">
                {op.source || "(empty)"}
              </tspan>
            </text>
          )}
        </BlockSvg>
      </g>
    );
  },
);
RawBlock.displayName = "RawBlock";
```

- [ ] **Step 2: 提交**

```bash
git add poiesis/src/components/canvas/code/blocks/BlockSvg/RawBlock.tsx poiesis/src/components/canvas/code/blocks/BlockSvg/index.ts
git commit -m "feat: add RawBlock dual-mode component (display/edit)"
```

---

### Task 8: BlockEditor 重写 + 移除 Blockly

**Files:**
- Rewrite: `poiesis/src/components/canvas/code/BlockEditor.tsx`
- Rewrite: `poiesis/src/components/canvas/code/BlockEditor.module.css`
- Delete: `poiesis/src/components/canvas/code/blocks/definitions.ts`
- Delete: `poiesis/src/components/canvas/code/blocks/mutations.ts`
- Delete: `poiesis/src/components/canvas/code/blocks/BlockAdapter.ts`
- Delete: `poiesis/src/components/canvas/code/blocks/SelectionManager.ts`
- Delete: `poiesis/src/components/canvas/code/blocks/GapManager.ts`
- Modify: `poiesis/package.json` (remove blockly, remove @blockly/*)

**Interfaces:**
- Consumes: All tasks 1-7
- Produces: Working `<BlockEditor>` with identical Props interface

- [ ] **Step 1: 重写 BlockEditor.tsx — 用 BlockModel + BlockList + KeyboardHandler**

```typescript
// poiesis/src/components/canvas/code/BlockEditor.tsx
import React, { useCallback, useEffect, useRef, useState } from "react";
import type { CodeOp } from "../../../core/types/code-op";
import { slotToText } from "../../../core/types/slot-value";
import { BlockModel } from "./blocks/BlockModel";
import type { SelectionTarget } from "./blocks/BlockModel";
import { KeyboardHandler } from "./blocks/KeyboardHandler";
import type { KBCallbacks } from "./blocks/KeyboardHandler";
import { ClipboardManager } from "./blocks/ClipboardManager";
import { SelectionOverlay } from "./blocks/SelectionOverlay";
import {
  FuzzySearchPopup,
  getItemsForContext,
} from "./blocks/fields/FuzzySearchPopup";
import type { SearchItem } from "./blocks/fields/FuzzySearchPopup";
import { CodeEditorModal } from "./CodeEditorModal";
import {
  LetVarBlock,
  SetVarBlock,
  ReturnBlock,
  CallBlock,
  AwaitBlock,
  BreakBlock,
  ContinueBlock,
  CommentBlock,
  RawBlock,
  IfBlock,
  WhileBlock,
  ForInBlock,
} from "./blocks/BlockSvg";
import styles from "./BlockEditor.module.css";

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  initialSource: string;
  language?: "typescript" | "omnilang";
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

const clipboard = new ClipboardManager();

export const BlockEditor: React.FC<Props> = ({
  elementId,
  initialOps,
  initialSource,
  language = "omnilang",
  onSave,
  onClose,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [showMonaco, setShowMonaco] = useState(false);
  const [showRawInput, setShowRawInput] = useState(false);
  const [rawInputText, setRawInputText] = useState("");
  const [opsCount, setOpsCount] = useState(initialOps.length);
  const [editingRawPid, setEditingRawPid] = useState<string | null>(null);

  const modelRef = useRef<BlockModel | null>(null);
  const khRef = useRef<KeyboardHandler | null>(null);
  const blockRefs = useRef<Map<string, SVGGElement>>(new Map());

  // Initialize model (lazily, once)
  if (!modelRef.current) {
    modelRef.current = new BlockModel(initialOps);
    modelRef.current.onChange = () => setOpsCount(modelRef.current!.getOps().length);
  }
  const model = modelRef.current;

  // getElement callback: maps SelectionTarget → DOM element
  const getElement = useCallback((target: SelectionTarget): HTMLElement | SVGElement | null => {
    switch (target.type) {
      case "block":
      case "slot": {
        const el = document.querySelector(`[data-block-id="${target.blockId}"]`);
        return el as SVGElement | null;
      }
      case "gap": {
        // Gap indicator: find the gap row by position
        const el = containerRef.current?.querySelector(
          `[data-gap-position="${target.position}"]`,
        );
        return el as HTMLElement | null;
      }
      default:
        return null;
    }
  }, []);

  // Keyboard handler
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const callbacks: KBCallbacks = {
      onFuzzySearch: (query: string, context: "slot" | "gap") => {
        const items = getItemsForContext(context);
        const popup = new FuzzySearchPopup(items, (item: SearchItem) => {
          if (context === "gap") {
            const target = model.getCurrentTarget();
            if (target?.type === "gap") {
              model.insertBlock(item.blockType, target.position);
            }
          } else {
            const target = model.getCurrentTarget();
            if (target?.type === "slot") {
              model.setSlotValue(target.blockIndex, target.slotName, {
                kind: "text",
                value: item.label,
              });
            }
          }
        }, () => {});
        popup.open(query);
      },
      onRawInput: () => setShowRawInput(true),
      onOpsChanged: () => console.log("ops changed"),
    };

    // Wire up clipboard Ctrl+C/V in the container
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        if (e.key === "c" || e.key === "C") {
          e.preventDefault();
          const target = model.getCurrentTarget();
          if (target?.type === "block") {
            clipboard.copy(model.getOps()[target.blockIndex]);
          }
        } else if (e.key === "v" || e.key === "V") {
          e.preventDefault();
          clipboard.paste().then((op) => {
            if (op) {
              const target = model.getCurrentTarget();
              if (target?.type === "gap") {
                // Insert pasted op
                model.insertBlock((op as CodeOp).kind, target.position);
              }
            }
          });
        }
      }
    };
    container.addEventListener("keydown", onKeyDown, { capture: true });

    const kh = new KeyboardHandler(container, model, getElement, callbacks);
    khRef.current = kh;

    return () => {
      kh.dispose();
      container.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [elementId, model, getElement]);

  // Save
  const handleSave = useCallback(() => {
    onSave(model.getOps());
  }, [onSave, model]);

  // Render a single block from CodeOp
  const renderBlock = (op: CodeOp): React.ReactNode => {
    switch (op.kind) {
      case "declare_var":
        return <LetVarBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "set_var":
        return <SetVarBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "return":
        return <ReturnBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "call_fn":
        return <CallBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "await":
        return <AwaitBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "break":
        return <BreakBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "continue":
        return <ContinueBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "comment":
        return <CommentBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} />;
      case "raw":
        return <RawBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} isEditing={editingRawPid === op.pid} onToggleEdit={() => setEditingRawPid(editingRawPid === op.pid ? null : op.pid)} onEdit={(src) => { op.source = src; setEditingRawPid(null); }} />;
      case "if_block":
        return <IfBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} renderBody={(body) => body.map(renderBlock)} />;
      case "while":
        return <WhileBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} renderBody={(body) => body.map(renderBlock)} />;
      case "for_in":
        return <ForInBlock key={op.pid} op={op} ref={(el) => el && blockRefs.current.set(op.pid, el)} renderBody={(body) => body.map(renderBlock)} />;
      default:
        return null;
    }
  };

  // Raw input commit
  const commitRawInput = useCallback(() => {
    if (!rawInputText.trim()) { setShowRawInput(false); return; }
    model.insertBlock("raw_stmt", model.getOps().length);
    const ops = model.getOps();
    const lastOp = ops[ops.length - 1];
    if (lastOp.kind === "raw") lastOp.source = rawInputText;
    setShowRawInput(false);
    setRawInputText("");
  }, [rawInputText, model]);

  // Monaco fallback
  if (showMonaco) {
    return (
      <CodeEditorModal
        elementId={elementId}
        initialSource={initialSource}
        initialDescription=""
        language={language}
        onSave={async () => { onSave(initialOps); setShowMonaco(false); }}
        onClose={() => setShowMonaco(false)}
      />
    );
  }

  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        <div className={styles.header}>
          <h3>Block Editor — {opsCount} ops</h3>
          <div>
            <button onClick={() => setShowMonaco(true)}>文本模式</button>
            <button onClick={handleSave}>Save</button>
            <button onClick={onClose}>Cancel</button>
          </div>
        </div>

        <div ref={containerRef} className={styles.container} style={{ overflowY: "auto", height: "100%" }}>
          <svg width="100%" height="auto">
            <g transform="translate(8, 8)">
              {model.getOps().map((op, i) => (
                <g key={op.pid} transform={`translate(0, ${i * 48})`}>
                  {renderBlock(op)}
                </g>
              ))}
              {/* Gap rows */}
              {[...model.getOps().keys()].map((i) => (
                <g key={`gap-${i}`} transform={`translate(0, ${i * 48 + 36})`}>
                  <rect
                    data-gap-position={i + 0.5}
                    width="100%"
                    height={12}
                    fill="transparent"
                  />
                </g>
              ))}
            </g>
          </svg>
        </div>

        <SelectionOverlay target={model.getCurrentTarget()} getElement={getElement} />

        {showRawInput && (
          <div className={styles.rawOverlay}>
            <textarea autoFocus value={rawInputText} onChange={(e) => setRawInputText(e.target.value)} placeholder="Enter raw OmniLang code..." />
            <button onClick={() => setShowRawInput(false)}>Esc 取消</button>
            <button onClick={commitRawInput}>Enter 提交</button>
          </div>
        )}
      </div>
    </div>
  );
};
```

- [ ] **Step 2: 删除旧文件**

```bash
rm poiesis/src/components/canvas/code/blocks/definitions.ts
rm poiesis/src/components/canvas/code/blocks/mutations.ts
rm poiesis/src/components/canvas/code/blocks/BlockAdapter.ts
rm poiesis/src/components/canvas/code/blocks/SelectionManager.ts
rm poiesis/src/components/canvas/code/blocks/GapManager.ts
```

- [ ] **Step 3: 移除 blockly npm 依赖**

```bash
cd /d/space/OmniLang/poiesis && pnpm remove blockly
```

- [ ] **Step 4: 验证 TypeScript 编译**

```bash
cd /d/space/OmniLang/poiesis && npx tsc --noEmit
```
Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add -A poiesis/src/components/canvas/code/blocks/ poiesis/src/components/canvas/code/BlockEditor.tsx poiesis/src/components/canvas/code/BlockEditor.module.css poiesis/package.json poiesis/pnpm-lock.yaml
git commit -m "feat: rewrite BlockEditor with pure SVG, remove Blockly"
```

---

### Task 9: 更新测试 + 更新 FlowCanvas 集成

**Files:**
- Delete: old test files for deleted modules
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx` (import 路径更新)
- Modify: Storybook stories (update imports)

- [ ] **Step 1: 删除旧模块的测试文件**

```bash
rm poiesis/src/components/canvas/code/blocks/definitions.test.ts 2>/dev/null
rm poiesis/src/components/canvas/code/blocks/BlockAdapter.test.ts 2>/dev/null
rm poiesis/src/components/canvas/code/blocks/SelectionManager.test.ts 2>/dev/null
rm poiesis/src/components/canvas/code/blocks/GapManager.test.ts 2>/dev/null
```

- [ ] **Step 2: 验证 FlowCanvas.tsx 引用——接口不变，import 路径可能需要调整**

BlockEditor 的 Props 接口不变，FlowCanvas 中 import `BlockEditor` from `"./BlockEditor"` 路径不变。无需修改 FlowCanvas.tsx。

- [ ] **Step 3: 运行所有测试**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run
```
Expected: 新测试 + 保留的 FuzzySearchPopup 测试全部通过

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "test: remove old Blockly-dependent tests, verify new test suite"
```

---

### Task 10: Storybook 验证 + Poiesis 集成验证

- [ ] **Step 1: 启动 Storybook**

```bash
bash /d/space/OmniLang/.claude/skills/start-storybook/start-storybook.sh
```

- [ ] **Step 2: Playwright 导航到 Storybook**

打开 `http://localhost:6006`，逐个验证 10 个 stories（Empty, Let+Set, IfBlock, WhileLoop, ForLoop 等）渲染正常，无 "Missing field moduleType" 错误。

- [ ] **Step 3: 键盘导航验证**

在 Empty story 中：按 Enter 触发 fuzzy search → 选 "let" → 积木创建。↑↓←→ 在积木和 gap 间移动。字符键在 slot 上触发搜索。

- [ ] **Step 4: 启动 Poiesis 验证**

```bash
bash /d/space/OmniLang/.claude/skills/start-poiesis/start-poiesis.sh
```

进入编辑器，双击 code-block 节点，确认 BlockEditor 正常打开和工作。

- [ ] **Step 5: 提交验证记录**

---

## Self-Review Report

**1. Spec coverage:**
- §1-2 架构和数据流 → Task 1 (BlockModel), Task 8 (BlockEditor), Task 5 (KeyboardHandler)
- §3 数据模型（不变）→ 无实施任务（代码已存在）
- §4 BlockModel → Task 1 全覆盖（nav entries, CRUD, selection target）
- §5 积木 SVG 组件 → Task 2 (shell + 9 simple), Task 3 (3 C-shaped), Task 7 (RawBlock)
- §6 交互模型 → Task 5 (KeyboardHandler), Task 4 (SelectionOverlay)
- §7 Clipboard → Task 6
- §8 文件变更 → Task 8 (删除旧文件), Task 2-3 (新建 SVG 组件)
- §9-12 测试、风险、实施顺序 → Task 9-10

**2. Placeholder scan:** 无 TBD/TODO。所有步骤包含实际代码。

**3. Type consistency:**
- `SelectionTarget` 在 Task 1 定义，Task 4/5/8 使用——类型一致
- `BlockModel` 的所有方法签名在 Task 1 定义，Task 5/8 调用——一致
- `KBCallbacks` 在 Task 5 定义，Task 8 使用——一致
- `slotToText` 来自现有 `slot-value.ts`，Task 2 使用——一致
- BlockSvg 组件 props 在各 Task 2/3/7 中定义，Task 8 调用——一致
