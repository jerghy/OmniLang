// poiesis/src/components/canvas/code/blocks/BlockModel.ts
import type { CodeOp } from "../../../../core/types/code-op";
import type { SlotValue } from "../../../../core/types/slot-value";
import { createBodyId } from "../../../../core/types/code-op";

export type SelectionTarget =
  | { type: "block"; blockIndex: number; blockId: string; bodyPath?: number[] }
  | { type: "slot"; blockIndex: number; blockId: string; slotName: string; bodyPath?: number[] }
  | { type: "gap"; position: number };

export interface NavEntry {
  kind: "block" | "slot" | "gap";
  blockIndex: number;
  slotName?: string;
  position: number;
  op?: CodeOp;
  bodyPath?: number[];
}

/** The list of slot names for each block type. */
const SLOT_MAP: Record<string, string[]> = {
  declare_var: ["NAME", "VARTYPE", "VALUE"],
  set_var: ["NAME", "VALUE"],
  return: ["VALUE"],
  call_fn: ["FN_NAME", "ARGS"],
  await: ["VALUE"],
  if_block: ["CONDITION"],
  while: ["CONDITION"],
  for_in: ["ITEM", "COLLECTION"],
  break: [],
  continue: [],
  comment: [],
  raw: [],
};

function getBlockSlots(op: CodeOp): string[] {
  return SLOT_MAP[op.kind] || [];
}

// ── Block path helpers ──

interface BlockInfo {
  op: CodeOp;
  topIndex: number;
  bodyPath: number[];
}

/**
 * Flatten all blocks (top-level + nested body blocks) into a list.
 * bodyPath identifies the location: empty for top-level, [branchIdx, bodyIdx] for if body, [bodyIdx] for while/for body.
 */
function collectAllBlocks(ops: CodeOp[]): BlockInfo[] {
  const result: BlockInfo[] = [];

  function walk(opList: CodeOp[], topIndex: number, pathPrefix: number[]) {
    for (let i = 0; i < opList.length; i++) {
      const op = opList[i];
      result.push({ op, topIndex, bodyPath: [...pathPrefix] });

      // Recurse into body blocks
      if (op.kind === "if_block") {
        for (let bi = 0; bi < op.branches.length; bi++) {
          walk(op.branches[bi].body, topIndex, [...pathPrefix, bi, ...op.branches[bi].body]);
          // Actually, we need to walk each branch's body
        }
        for (let bi = 0; bi < op.branches.length; bi++) {
          for (let bbi = 0; bbi < op.branches[bi].body.length; bbi++) {
            walk([op.branches[bi].body[bbi]], result.length > 0 ? result[result.length - 1].topIndex : 0, [bi, bbi]);
          }
        }
      } else if (op.kind === "while" || op.kind === "for_in") {
        const body = (op as { body: CodeOp[] }).body;
        for (let bbi = 0; bbi < body.length; bbi++) {
          walk([body[bbi]], result.length > 0 ? topIndex : 0, [bbi]);
        }
      }
    }
  }

  // Collect top-level blocks with empty bodyPath
  for (let i = 0; i < ops.length; i++) {
    const op = ops[i];
    result.push({ op, topIndex: i, bodyPath: [] });

    if (op.kind === "if_block") {
      for (let bi = 0; bi < op.branches.length; bi++) {
        for (let bbi = 0; bbi < op.branches[bi].body.length; bbi++) {
          result.push({ op: op.branches[bi].body[bbi], topIndex: i, bodyPath: [bi, bbi] });
        }
      }
    } else if (op.kind === "while" || op.kind === "for_in") {
      const body = (op as { body: CodeOp[] }).body;
      for (let bbi = 0; bbi < body.length; bbi++) {
        result.push({ op: body[bbi], topIndex: i, bodyPath: [bbi] });
      }
    }
  }

  return result;
}

/**
 * Get the body blocks for a given CodeOp.
 */
function getBodyBlocks(op: CodeOp): CodeOp[] {
  if (op.kind === "if_block") {
    const all: CodeOp[] = [];
    for (const branch of op.branches) {
      all.push(...branch.body);
    }
    return all;
  }
  if (op.kind === "while" || op.kind === "for_in") {
    return (op as { body: CodeOp[] }).body;
  }
  return [];
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
    const allBlocks = collectAllBlocks(this.ops);

    const entries: NavEntry[] = [];

    if (this.ops.length === 0) {
      entries.push({ kind: "gap", blockIndex: -1, position: 0 });
      return entries;
    }

    // gap before first block
    entries.push({ kind: "gap", blockIndex: -1, position: 0 });

    for (let i = 0; i < this.ops.length; i++) {
      const op = this.ops[i];

      // Block entry (top-level)
      entries.push({ kind: "block", blockIndex: i, position: i + 1, op, bodyPath: [] });

      // Slots for this block
      const slotNames = getBlockSlots(op);
      for (let s = 0; s < slotNames.length; s++) {
        entries.push({
          kind: "slot",
          blockIndex: i,
          slotName: slotNames[s],
          position: i + 1 + (s + 1) * 0.01,
          bodyPath: [],
        });
      }

      // ── Body entries for C-blocks (after slots, before gap) ──
      const bodyBlocks = getBodyBlocks(op);
      if (bodyBlocks.length > 0) {
        const bodyEntries = this.buildBodyNavEntries(op, i);
        entries.push(...bodyEntries);
      }

      // Gap between top-level blocks (only if there's a gap at this position)
      if (i < this.ops.length - 1) {
        const gapPos = i + 0.5;
        if (this.gaps.has(gapPos)) {
          entries.push({ kind: "gap", blockIndex: i, position: gapPos });
        }
      }
    }

    // gap after last block
    entries.push({ kind: "gap", blockIndex: this.ops.length - 1, position: this.ops.length });

    return entries;
  }

  private buildBodyNavEntries(parentOp: CodeOp, topIndex: number): NavEntry[] {
    const entries: NavEntry[] = [];

    if (parentOp.kind === "if_block") {
      for (let bi = 0; bi < parentOp.branches.length; bi++) {
        for (let bbi = 0; bbi < parentOp.branches[bi].body.length; bbi++) {
          const bodyOp = parentOp.branches[bi].body[bbi];
          const bodyPath = [bi, bbi];
          entries.push({
            kind: "block",
            blockIndex: topIndex,
            position: entries.length + 1,
            op: bodyOp,
            bodyPath,
          });
          // Slots for this body block — carry op ref so entryToTarget resolves blockId correctly
          const slotNames = getBlockSlots(bodyOp);
          for (let s = 0; s < slotNames.length; s++) {
            entries.push({
              kind: "slot",
              blockIndex: topIndex,
              slotName: slotNames[s],
              position: entries.length + 1 + (s + 1) * 0.01,
              op: bodyOp,
              bodyPath,
            });
          }
        }
      }
    } else if (parentOp.kind === "while" || parentOp.kind === "for_in") {
      const body = (parentOp as { body: CodeOp[] }).body;
      for (let bbi = 0; bbi < body.length; bbi++) {
        const bodyOp = body[bbi];
        const bodyPath = [bbi];
        entries.push({
          kind: "block",
          blockIndex: topIndex,
          position: entries.length + 1,
          op: bodyOp,
          bodyPath,
        });
        const slotNames = getBlockSlots(bodyOp);
        for (let s = 0; s < slotNames.length; s++) {
          entries.push({
            kind: "slot",
            blockIndex: topIndex,
            slotName: slotNames[s],
            position: entries.length + 1 + (s + 1) * 0.01,
            op: bodyOp,
            bodyPath,
          });
        }
      }
    }

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

  /** Delete a body block identified by its path from the parent block. */
  deleteBodyBlock(topIndex: number, bodyPath: number[]): void {
    const parentOp = this.ops[topIndex];
    if (!parentOp) return;

    if (parentOp.kind === "if_block" && bodyPath.length === 2) {
      const [bi, bbi] = bodyPath;
      if (bi >= 0 && bi < parentOp.branches.length) {
        parentOp.branches[bi].body.splice(bbi, 1);
        this.ops = [...this.ops]; // trigger immutability
      }
    } else if ((parentOp.kind === "while" || parentOp.kind === "for_in") && bodyPath.length === 1) {
      const body = (parentOp as { body: CodeOp[] }).body;
      body.splice(bodyPath[0], 1);
      this.ops = [...this.ops];
    }

    this.rebuildGaps();
    this.onChange?.();
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

    const slotFieldMap: Record<string, Record<string, string>> = {
      declare_var: { NAME: "name", VARTYPE: "varType", VALUE: "initValue" },
      set_var: { NAME: "name", VALUE: "value" },
      return: { VALUE: "value" },
      call_fn: { FN_NAME: "fnName", ARGS: "args" },
      await: { VALUE: "value" },
      while: { CONDITION: "condition" },
      for_in: { ITEM: "itemName", COLLECTION: "collection" },
    };

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

  entryToTarget(
    entry: NavEntry,
    ops: CodeOp[],
  ): SelectionTarget | null {
    // For body blocks/slots, use the entry's op (body block) instead of ops[blockIndex] (parent)
    const hasBodyPath = entry.bodyPath && entry.bodyPath.length > 0;

    switch (entry.kind) {
      case "block": {
        const blockId = hasBodyPath && entry.op
          ? entry.op.pid
          : (ops[entry.blockIndex]?.pid || "");
        return {
          type: "block",
          blockIndex: entry.blockIndex,
          blockId,
          bodyPath: entry.bodyPath,
        };
      }
      case "slot":
        return {
          type: "slot",
          blockIndex: entry.blockIndex,
          blockId: hasBodyPath && entry.op
            ? entry.op.pid
            : (ops[entry.blockIndex]?.pid || ""),
          slotName: entry.slotName || "",
          bodyPath: entry.bodyPath,
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
