// poiesis/src/components/canvas/code/blocks/BlockModel.test.ts
import { describe, it, expect } from "vitest";
import { BlockModel } from "../BlockModel";
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
