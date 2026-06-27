/** 各 Command 类的 execute/undo 单元测试。*/

import { describe, it, expect } from "vitest";
import type { CommandContext } from "../types";
import type { ElementData, WireData } from "../../types";
import { AddElement } from "../AddElement";
import { RemoveElement } from "../RemoveElement";
import { AddWire } from "../AddWire";
import { RemoveWire } from "../RemoveWire";
import { SetProp } from "../SetProp";
import { SetElementsAndWires } from "../SetElementsAndWires";

function makeEl(overrides: Partial<ElementData> = {}): ElementData {
  return {
    id: "el-1",
    nodeType: "codeblock",
    tag: "signal",
    props: { name: "count" },
    children: [],
    parentId: null,
    ...overrides,
  };
}

function makeWire(overrides: Partial<WireData> = {}): WireData {
  return {
    id: "w-1",
    fromId: "el-1",
    fromPort: "value",
    toId: "el-2",
    toPort: "deps",
    ...overrides,
  };
}

function makeCtx(elements: ElementData[] = [], wires: WireData[] = []): CommandContext {
  const ctxElements = [...elements];
  const ctxWires = [...wires];
  return {
    elements: ctxElements,
    wires: ctxWires,
    getElement: (id: string) => ctxElements.find((e) => e.id === id),
    addElement: (el: ElementData) => {
      ctxElements.push(el);
    },
    removeElement: (id: string) => {
      const idx = ctxElements.findIndex((e) => e.id === id);
      if (idx >= 0) ctxElements.splice(idx, 1);
    },
    setProp: (id: string, prop: string, value: unknown) => {
      const el = ctxElements.find((e) => e.id === id);
      if (el) (el.props as Record<string, unknown>)[prop] = value;
    },
    getProp: (id: string, prop: string) => {
      const el = ctxElements.find((e) => e.id === id);
      return el ? (el.props as Record<string, unknown>)[prop] : undefined;
    },
    addWire: (w: WireData) => {
      ctxWires.push(w);
    },
    removeWire: (id: string) => {
      const idx = ctxWires.findIndex((w) => w.id === id);
      if (idx >= 0) ctxWires.splice(idx, 1);
    },
    setElementsAndWires: (els: ElementData[], ws: WireData[]) => {
      ctxElements.length = 0;
      ctxElements.push(...els);
      ctxWires.length = 0;
      ctxWires.push(...ws);
    },
    reorderElement: (_elementId: string, _targetSiblingId: string, _position: "before" | "after") => {},
    moveElement: (_elementId: string, _newParentId: string | null, _targetSiblingId: string | null, _position: "before" | "after") => {},
  };
}

// ── AddElement ──────────────────────────────────────────────────────

describe("AddElement", () => {
  it("execute adds the element", () => {
    const el = makeEl({ id: "a" });
    const ctx = makeCtx();
    const cmd = new AddElement(el);
    cmd.execute(ctx);
    expect(ctx.elements).toHaveLength(1);
    expect(ctx.elements[0].id).toBe("a");
  });

  it("undo removes the element", () => {
    const el = makeEl({ id: "a" });
    const ctx = makeCtx();
    const cmd = new AddElement(el);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect(ctx.elements).toHaveLength(0);
  });

  it("description includes the tag name", () => {
    const cmd = new AddElement(makeEl({ tag: "button" }));
    expect(cmd.description).toContain("button");
  });
});

// ── RemoveElement ───────────────────────────────────────────────────

describe("RemoveElement", () => {
  it("execute snapshots and removes the element", () => {
    const el = makeEl({ id: "a" });
    const ctx = makeCtx([el]);
    const cmd = new RemoveElement("a");
    cmd.execute(ctx);
    expect(ctx.elements).toHaveLength(0);
  });

  it("undo restores the snapshot", () => {
    const el = makeEl({ id: "a" });
    const ctx = makeCtx([el]);
    const cmd = new RemoveElement("a");
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect(ctx.elements).toHaveLength(1);
    expect(ctx.elements[0].id).toBe("a");
  });

  it("is no-op when element does not exist", () => {
    const ctx = makeCtx();
    const cmd = new RemoveElement("nonexistent");
    cmd.execute(ctx);
    expect(ctx.elements).toHaveLength(0);
  });
});

// ── AddWire ─────────────────────────────────────────────────────────

describe("AddWire", () => {
  it("execute adds the wire", () => {
    const w = makeWire();
    const ctx = makeCtx();
    const cmd = new AddWire(w);
    cmd.execute(ctx);
    expect(ctx.wires).toHaveLength(1);
    expect(ctx.wires[0].id).toBe("w-1");
  });

  it("undo removes the wire", () => {
    const w = makeWire();
    const ctx = makeCtx();
    const cmd = new AddWire(w);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect(ctx.wires).toHaveLength(0);
  });
});

// ── RemoveWire ──────────────────────────────────────────────────────

describe("RemoveWire", () => {
  it("execute snapshots and removes the wire", () => {
    const w = makeWire();
    const ctx = makeCtx([], [w]);
    const cmd = new RemoveWire("w-1");
    cmd.execute(ctx);
    expect(ctx.wires).toHaveLength(0);
  });

  it("undo restores the snapshot", () => {
    const w = makeWire();
    const ctx = makeCtx([], [w]);
    const cmd = new RemoveWire("w-1");
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect(ctx.wires).toHaveLength(1);
  });

  it("is no-op when wire does not exist", () => {
    const ctx = makeCtx();
    const cmd = new RemoveWire("nonexistent");
    cmd.execute(ctx);
    expect(ctx.wires).toHaveLength(0);
  });
});

// ── SetProp ─────────────────────────────────────────────────────────

describe("SetProp", () => {
  it("execute changes the prop value", () => {
    const el = makeEl({ id: "a", props: { color: "red" } });
    const ctx = makeCtx([el]);
    const cmd = new SetProp("a", "color", "blue");
    cmd.execute(ctx);
    expect(ctx.elements[0].props.color).toBe("blue");
  });

  it("undo restores the old value", () => {
    const el = makeEl({ id: "a", props: { color: "red" } });
    const ctx = makeCtx([el]);
    const cmd = new SetProp("a", "color", "blue");
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect(ctx.elements[0].props.color).toBe("red");
  });

  it("description includes the prop name", () => {
    const cmd = new SetProp("a", "padding", "16px");
    expect(cmd.description).toContain("padding");
  });
});

// ── SetElementsAndWires ─────────────────────────────────────────────

describe("SetElementsAndWires", () => {
  it("execute snapshots old state and sets new state", () => {
    const oldEl = makeEl({ id: "old" });
    const ctx = makeCtx([oldEl], [makeWire({ id: "old-w" })]);

    const newEl = makeEl({ id: "new" });
    const cmd = new SetElementsAndWires([newEl], []);
    cmd.execute(ctx);

    expect(ctx.elements).toHaveLength(1);
    expect(ctx.elements[0].id).toBe("new");
    expect(ctx.wires).toHaveLength(0);
  });

  it("undo restores the snapshot", () => {
    const oldEl = makeEl({ id: "old" });
    const ctx = makeCtx([oldEl], []);

    const cmd = new SetElementsAndWires([makeEl({ id: "new" })], []);
    cmd.execute(ctx);
    cmd.undo(ctx);

    expect(ctx.elements).toHaveLength(1);
    expect(ctx.elements[0].id).toBe("old");
  });

  it("second execute does not re-snapshot (uses same prev state)", () => {
    const oldEl = makeEl({ id: "old" });
    const ctx = makeCtx([oldEl], []);

    const cmd = new SetElementsAndWires([makeEl({ id: "new" })], []);
    cmd.execute(ctx);
    // Change ctx externally
    ctx.addElement(makeEl({ id: "external" }));
    cmd.execute(ctx); // redo scenario — should NOT re-snapshot

    cmd.undo(ctx);
    // Should restore to original (old), not the intermediate state
    expect(ctx.elements).toHaveLength(1);
    expect(ctx.elements[0].id).toBe("old");
  });
});
