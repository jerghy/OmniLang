/** Style store methods 单元测试。*/

import { describe, it, expect, beforeEach } from "vitest";
import { useEditorStore } from "../editor";
import type { StyleEntry } from "../../constants/style-properties";

function makeEntry(overrides: Partial<StyleEntry> = {}): StyleEntry {
  return {
    id: "s1",
    type: "class",
    name: "card-style",
    properties: [{ name: "padding", value: "16px" }],
    ...overrides,
  };
}

describe("Style store methods", () => {
  beforeEach(() => {
    useEditorStore.setState({
      elements: [],
      wires: [],
      selectedElementIds: [],
    });
  });

  it("addStyleEntry adds an entry to element.styles", () => {
    useEditorStore.getState().addElement({
      id: "a",
      tag: "Button",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view" as const,
    });

    const entry = makeEntry();
    useEditorStore.getState().addStyleEntry("a", entry);

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.styles).toHaveLength(1);
    expect(updated?.styles?.[0].name).toBe("card-style");
  });

  it("removeStyleEntry removes entry by id", () => {
    const entry = makeEntry();
    useEditorStore.getState().addElement({
      id: "a",
      tag: "Button",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view" as const,
      styles: [entry],
    });

    useEditorStore.getState().removeStyleEntry("a", "s1");

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.styles).toHaveLength(0);
  });

  it("reorderStyleEntries reorders entries", () => {
    const e1 = makeEntry({ id: "s1", name: "first" });
    const e2 = makeEntry({ id: "s2", name: "second" });
    const el = { id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view" as const, styles: [e1, e2] };
    useEditorStore.setState({ elements: [el] });

    useEditorStore.getState().reorderStyleEntries("a", ["s2", "s1"]);

    const updated = useEditorStore.getState().elements.find(e => e.id === "a");
    expect(updated?.styles?.[0].id).toBe("s2");
    expect(updated?.styles?.[1].id).toBe("s1");
  });

  it("reorderStyleEntries ignores stale IDs", () => {
    const e1 = makeEntry({ id: "s1", name: "first" });
    const el = { id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view" as const, styles: [e1] };
    useEditorStore.setState({ elements: [el] });

    // Pass a stale ID that doesn't exist
    useEditorStore.getState().reorderStyleEntries("a", ["s2", "s1"]);

    // Should not have changed
    const updated = useEditorStore.getState().elements.find(e => e.id === "a");
    expect(updated?.styles).toHaveLength(1);
    expect(updated?.styles?.[0].id).toBe("s1");
  });

  it("getStyles returns styles for element", () => {
    const entry = makeEntry();
    const el = { id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view" as const, styles: [entry] };
    useEditorStore.setState({ elements: [el] });

    const styles = useEditorStore.getState().getStyles("a");
    expect(styles).toHaveLength(1);
    expect(styles[0].name).toBe("card-style");
  });

  it("getStyles returns empty array for non-existent element", () => {
    const styles = useEditorStore.getState().getStyles("nonexistent");
    expect(styles).toEqual([]);
  });

  it("updateStyleEntry updates a specific entry", () => {
    const entry = makeEntry();
    useEditorStore.getState().addElement({
      id: "a",
      tag: "Button",
      props: {},
      children: [],
      parentId: null,
      nodeType: "view" as const,
      styles: [entry],
    });

    useEditorStore.getState().updateStyleEntry("a", "s1", { name: "new-name" });

    const updated = useEditorStore.getState().elements.find((e) => e.id === "a");
    expect(updated?.styles?.[0].name).toBe("new-name");
  });
});
