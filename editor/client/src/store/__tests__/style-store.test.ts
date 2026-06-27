/**
 * Tests for the new element-level style API on useEditorStore.
 *
 * Style registry operations (addClass, addVariable, etc.) live on
 * useStyleRegistryStore and are tested in style-registry.test.ts.
 * Here we test per-element classRef / inlineProperty mutations.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { useEditorStore } from "../editor";
import type { StyleProperty } from "../../core/types";

function makeProperty(overrides: Partial<StyleProperty> = {}): StyleProperty {
  return {
    id: "p1",
    name: "padding",
    segments: [{ type: "text", value: "16px" }],
    ...overrides,
  };
}

describe("Element style mutations on editor store", () => {
  beforeEach(() => {
    useEditorStore.setState({
      elements: [],
      wires: [],
      selectedElementIds: [],
    });
  });

  // ── classRef methods ──

  it("setElementClassRefs sets classRefs on an element", () => {
    useEditorStore.getState().addElement({
      id: "a",
      tag: "div",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view",
    });

    useEditorStore.getState().setElementClassRefs("a", ["card", "_v-gap"]);

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.style?.classRefs).toEqual(["card", "_v-gap"]);
  });

  it("addElementClassRef adds a single class ref", () => {
    useEditorStore.getState().addElement({
      id: "a",
      tag: "div",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view",
      style: { classRefs: ["card"], inlineProperties: [] },
    });

    useEditorStore.getState().addElementClassRef("a", "_v-gap");

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.style?.classRefs).toContain("_v-gap");
    expect(updated?.style?.classRefs).toHaveLength(2);
  });

  it("removeElementClassRef removes a single class ref", () => {
    useEditorStore.getState().addElement({
      id: "a",
      tag: "div",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view",
      style: { classRefs: ["card", "_v-gap"], inlineProperties: [] },
    });

    useEditorStore.getState().removeElementClassRef("a", "card");

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.style?.classRefs).toEqual(["_v-gap"]);
  });

  // ── inline property methods ──

  it("setElementInlineStyles sets inline properties", () => {
    const props = [makeProperty()];
    useEditorStore.getState().addElement({
      id: "a",
      tag: "div",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view",
    });

    useEditorStore.getState().setElementInlineStyles("a", props);

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.style?.inlineProperties).toHaveLength(1);
    expect(updated?.style?.inlineProperties[0].name).toBe("padding");
  });

  it("addElementInlineProperty adds a property", () => {
    useEditorStore.getState().addElement({
      id: "a",
      tag: "div",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view",
      style: { classRefs: [], inlineProperties: [] },
    });

    useEditorStore.getState().addElementInlineProperty("a", makeProperty({ id: "p1" }));

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.style?.inlineProperties).toHaveLength(1);
    expect(updated?.style?.inlineProperties[0].id).toBe("p1");
  });

  it("removeElementInlineProperty removes a property by id", () => {
    useEditorStore.getState().addElement({
      id: "a",
      tag: "div",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view",
      style: { classRefs: [], inlineProperties: [makeProperty({ id: "p1" })] },
    });

    useEditorStore.getState().removeElementInlineProperty("a", "p1");

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.style?.inlineProperties).toHaveLength(0);
  });

  it("updateElementInlineProperty patches a property", () => {
    useEditorStore.getState().addElement({
      id: "a",
      tag: "div",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view",
      style: { classRefs: [], inlineProperties: [makeProperty({ id: "p1", name: "padding", segments: [{ type: "text", value: "16px" }] })] },
    });

    useEditorStore.getState().updateElementInlineProperty("a", "p1", { name: "margin" });

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.style?.inlineProperties[0].name).toBe("margin");
  });
});
