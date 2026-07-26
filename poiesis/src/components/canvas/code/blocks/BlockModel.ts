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
    return [...this.ops];
  }

  getNavEntries(): NavEntry[] {
    const entries: NavEntry[] = [];
    if (this.ops.length === 0) {
      entries.push({ kind: "gap", blockIndex: -1, position: 0 });
      return entries;
    }

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

  /** Insert an already-constructed CodeOp (e.g., from paste) directly into ops at the position. */
  insertCodeOp(op: CodeOp, atPosition: number): void {
    const insertIdx = Math.round(atPosition);
    const clampedIdx = Math.min(insertIdx, this.ops.length);
    this.ops.splice(clampedIdx, 0, op);
    this.rebuildGaps();
    this.onChange?.();
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
      while_loop: { CONDITION: "condition" },
      for_in: { ITEM: "itemName", COLLECTION: "collection" },
    };

    // Special handling for if_block: condition lives in branches[0].condition
    if (op.kind === "if_block") {
      if (slotName === "CONDITION") {
        this.ops[blockIndex] = {
          ...op,
          branches: [
            { ...op.branches[0], condition: value },
            ...op.branches.slice(1),
          ],
        } as CodeOp;
        this.onChange?.();
      }
      return;
    }

    const fieldMap = slotFieldMap[op.kind];
    if (!fieldMap || !fieldMap[slotName]) return;
    const field = fieldMap[slotName];
    // Create a new object (immutable update) instead of mutating in-place
    this.ops[blockIndex] = { ...op, [field]: value } as CodeOp;
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
        } as CodeOp;
      case "set_var":
        return {
          kind: "set_var",
          pid,
          name: { kind: "empty" },
          operator: "=",
          value: { kind: "empty" },
        } as CodeOp;
      case "return_stmt":
        return { kind: "return", pid } as CodeOp;
      case "call_fn":
        return {
          kind: "call_fn",
          pid,
          fnName: { kind: "empty" },
          args: [],
        } as CodeOp;
      case "await_expr":
        return { kind: "await", pid, value: { kind: "empty" } } as CodeOp;
      case "break_stmt":
        return { kind: "break", pid } as CodeOp;
      case "continue_stmt":
        return { kind: "continue", pid } as CodeOp;
      case "comment_stmt":
        return { kind: "comment", pid, text: "" } as CodeOp;
      case "raw_stmt":
        return { kind: "raw", pid, source: "" } as CodeOp;
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
        } as CodeOp;
      case "while_loop":
        return {
          kind: "while",
          pid,
          bodyId: createBodyId(),
          condition: { kind: "empty" },
          body: [],
        } as CodeOp;
      case "for_in":
        return {
          kind: "for_in",
          pid,
          bodyId: createBodyId(),
          itemName: { kind: "empty" },
          collection: { kind: "empty" },
          body: [],
        } as CodeOp;
      default:
        return { kind: "comment", pid, text: `unknown: ${blockType}` } as CodeOp;
    }
  }
}
