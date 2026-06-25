/** Style Command 类的 execute/undo 单元测试。*/

import { describe, it, expect } from "vitest";
import { AddStyleEntry } from "../AddStyleEntry";
import { RemoveStyleEntry } from "../RemoveStyleEntry";
import { ReorderStyleEntry } from "../ReorderStyleEntry";
import { UpdateStyleProperty } from "../UpdateStyleProperty";
import { SetStyleEntryType } from "../SetStyleEntryType";
import { RenameStyleEntry } from "../RenameStyleEntry";
import type { CommandContext } from "../types";
import type { ElementData, WireData } from "../../store/editor";
import type { StyleEntry } from "../../constants/style-properties";

function makeCtx(elements: ElementData[] = [], wires: WireData[] = []): CommandContext {
  const ctxElements = [...elements];
  const ctxWires = [...wires];
  return {
    elements: ctxElements,
    wires: ctxWires,
    getElement: (id: string) => ctxElements.find((e) => e.id === id),
    addElement: (el: ElementData) => { ctxElements.push(el); },
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
    addWire: (w: WireData) => { ctxWires.push(w); },
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
    reorderElement: () => {},
    moveElement: () => {},
  };
}

const baseEl: ElementData = {
  id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view",
};

describe("AddStyleEntry", () => {
  it("execute adds entry", () => {
    const el = { ...baseEl, styles: [] as StyleEntry[] };
    const ctx = makeCtx([el]);
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "16px" }] };
    new AddStyleEntry("a", entry).execute(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(1);
  });

  it("undo removes the entry", () => {
    const el = { ...baseEl, styles: [] as StyleEntry[] };
    const ctx = makeCtx([el]);
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const cmd = new AddStyleEntry("a", entry);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(0);
  });
});

describe("RemoveStyleEntry", () => {
  it("execute removes entry at index", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new RemoveStyleEntry("a", entry, 0).execute(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(0);
  });

  it("undo restores entry at original index", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new RemoveStyleEntry("a", entry, 0);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(1);
  });
});

describe("ReorderStyleEntry", () => {
  it("execute reorders entries", () => {
    const s1: StyleEntry = { id: "s1", type: "class", name: "first", properties: [] };
    const s2: StyleEntry = { id: "s2", type: "class", name: "second", properties: [] };
    const el = { ...baseEl, styles: [s1, s2] };
    const ctx = makeCtx([el]);
    new ReorderStyleEntry("a", ["s1", "s2"], ["s2", "s1"]).execute(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("second");
  });

  it("undo restores original order", () => {
    const s1: StyleEntry = { id: "s1", type: "class", name: "first", properties: [] };
    const s2: StyleEntry = { id: "s2", type: "class", name: "second", properties: [] };
    const el = { ...baseEl, styles: [s1, s2] };
    const ctx = makeCtx([el]);
    const cmd = new ReorderStyleEntry("a", ["s1", "s2"], ["s2", "s1"]);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("first");
  });
});

describe("UpdateStyleProperty", () => {
  it("execute modifies a property value", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "8px" }] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new UpdateStyleProperty("a", "s1", 0, { name: "padding", value: "8px" }, { name: "padding", value: "16px" }).execute(ctx);
    expect((ctx.elements[0] as any).styles[0].properties[0].value).toBe("16px");
  });

  it("undo reverts the property", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "8px" }] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new UpdateStyleProperty("a", "s1", 0, { name: "padding", value: "8px" }, { name: "padding", value: "16px" });
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].properties[0].value).toBe("8px");
  });

  it("propertyIndex -1 means add new property", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new UpdateStyleProperty("a", "s1", -1, null, { name: "color", value: "#333" }).execute(ctx);
    expect((ctx.elements[0] as any).styles[0].properties).toHaveLength(1);
  });
});

describe("SetStyleEntryType", () => {
  it("execute changes type from class to variable", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new SetStyleEntryType("a", "s1", "class", "variable").execute(ctx);
    expect((ctx.elements[0] as any).styles[0].type).toBe("variable");
  });

  it("undo reverts type", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new SetStyleEntryType("a", "s1", "class", "variable");
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].type).toBe("class");
  });
});

describe("RenameStyleEntry", () => {
  it("execute changes the name", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "old-name", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new RenameStyleEntry("a", "s1", "old-name", "new-name").execute(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("new-name");
  });

  it("undo reverts name", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "old-name", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new RenameStyleEntry("a", "s1", "old-name", "new-name");
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("old-name");
  });
});
