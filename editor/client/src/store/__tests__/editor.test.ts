/** Editor store 单元测试。*/

import { describe, it, expect, beforeEach } from "vitest";
import { useEditorStore } from "../editor";
import type { ElementData, WireData } from "../editor";

function makeEl(overrides: Partial<ElementData> = {}): ElementData {
  return {
    id: "el-1",
    nodeType: "codeblock",
    tag: "signal",
    props: {},
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

beforeEach(() => {
  useEditorStore.setState({
    elements: [],
    wires: [],
    selectedElementIds: [],
  });
});

describe("addElement", () => {
  it("adds an element to the array", () => {
    const el = makeEl();
    useEditorStore.getState().addElement(el);
    expect(useEditorStore.getState().elements).toHaveLength(1);
    expect(useEditorStore.getState().elements[0]).toEqual(el);
  });

  it("adds multiple elements independently", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a" }));
    useEditorStore.getState().addElement(makeEl({ id: "b" }));
    expect(useEditorStore.getState().elements).toHaveLength(2);
  });
});

describe("removeElement", () => {
  it("removes an element by id", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a" }));
    useEditorStore.getState().addElement(makeEl({ id: "b" }));
    useEditorStore.getState().removeElement("a");
    expect(useEditorStore.getState().elements).toHaveLength(1);
    expect(useEditorStore.getState().elements[0].id).toBe("b");
  });

  it("is a no-op when removing a non-existent element", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a" }));
    useEditorStore.getState().removeElement("nonexistent");
    expect(useEditorStore.getState().elements).toHaveLength(1);
  });
});

describe("setProp", () => {
  it("immutably updates a prop on an element", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a", props: { name: "old" } }));
    const prevEl = useEditorStore.getState().elements[0];
    useEditorStore.getState().setProp("a", "name", "new");
    const nextEl = useEditorStore.getState().elements[0];
    expect(nextEl.props.name).toBe("new");
    // Immutability: prev element object should be different
    expect(prevEl).not.toBe(nextEl);
  });

  it("does not mutate other elements", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a", props: { x: 1 } }));
    useEditorStore.getState().addElement(makeEl({ id: "b", props: { x: 2 } }));
    useEditorStore.getState().setProp("a", "x", 10);
    const b = useEditorStore.getState().elements.find((e) => e.id === "b");
    expect(b?.props.x).toBe(2);
  });
});

describe("getProp", () => {
  it("returns the value of a prop", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a", props: { color: "red" } }));
    expect(useEditorStore.getState().getProp("a", "color")).toBe("red");
  });

  it("returns undefined for non-existent element", () => {
    expect(useEditorStore.getState().getProp("nonexistent", "x")).toBeUndefined();
  });
});

describe("selectElement", () => {
  it("sets selectedElementIds to single element", () => {
    useEditorStore.getState().selectElement("el-1");
    expect(useEditorStore.getState().selectedElementIds).toEqual(["el-1"]);
  });

  it("clears selection with null", () => {
    useEditorStore.getState().selectElement("el-1");
    useEditorStore.getState().selectElement(null);
    expect(useEditorStore.getState().selectedElementIds).toEqual([]);
  });

  it("adds to selection with multi=true", () => {
    useEditorStore.getState().selectElement("el-1");
    useEditorStore.getState().selectElement("el-2", true);
    expect(useEditorStore.getState().selectedElementIds).toEqual(["el-1", "el-2"]);
  });

  it("toggles off with multi=true when already selected", () => {
    useEditorStore.getState().selectElement("el-1");
    useEditorStore.getState().selectElement("el-2", true);
    useEditorStore.getState().selectElement("el-1", true); // toggle off
    expect(useEditorStore.getState().selectedElementIds).toEqual(["el-2"]);
  });
});

describe("selectAll / clearSelection", () => {
  it("selects all element ids", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a" }));
    useEditorStore.getState().addElement(makeEl({ id: "b" }));
    useEditorStore.getState().selectAll();
    expect(useEditorStore.getState().selectedElementIds).toEqual(["a", "b"]);
  });

  it("clears selection", () => {
    useEditorStore.getState().selectElement("el-1");
    useEditorStore.getState().clearSelection();
    expect(useEditorStore.getState().selectedElementIds).toEqual([]);
  });
});

describe("addWire / removeWire", () => {
  it("adds and removes wires", () => {
    const w = makeWire();
    useEditorStore.getState().addWire(w);
    expect(useEditorStore.getState().wires).toHaveLength(1);

    useEditorStore.getState().removeWire(w.id);
    expect(useEditorStore.getState().wires).toHaveLength(0);
  });

  it("removeWire on non-existent is a no-op", () => {
    useEditorStore.getState().addWire(makeWire({ id: "w-1" }));
    useEditorStore.getState().removeWire("nonexistent");
    expect(useEditorStore.getState().wires).toHaveLength(1);
  });
});

describe("setElementPosition", () => {
  it("updates the position of an element", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a", position: { x: 0, y: 0 } }));
    useEditorStore.getState().setElementPosition("a", { x: 100, y: 200 });
    const el = useEditorStore.getState().elements[0];
    expect(el.position).toEqual({ x: 100, y: 200 });
  });
});

describe("setElementsAndWires", () => {
  it("atomically replaces all elements and wires", () => {
    useEditorStore.getState().addElement(makeEl({ id: "old" }));
    useEditorStore.getState().addWire(makeWire({ id: "old-w" }));

    const newElements = [makeEl({ id: "new" })];
    const newWires = [makeWire({ id: "new-w" })];
    useEditorStore.getState().setElementsAndWires(newElements, newWires);

    expect(useEditorStore.getState().elements).toEqual(newElements);
    expect(useEditorStore.getState().wires).toEqual(newWires);
    expect(useEditorStore.getState().selectedElementIds).toEqual([]);
  });
});

describe("setBlockValue", () => {
  it("updates outputValue", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a" }));
    useEditorStore.getState().setBlockValue("a", 42);
    expect(useEditorStore.getState().elements[0].outputValue).toBe(42);
  });

  it("skips update when value has not changed", () => {
    useEditorStore.getState().addElement(makeEl({ id: "a", outputValue: 42 }));
    const prev = useEditorStore.getState().elements;
    useEditorStore.getState().setBlockValue("a", 42); // same value
    // Should return the same reference (no re-render)
    expect(useEditorStore.getState().elements).toBe(prev);
  });
});
