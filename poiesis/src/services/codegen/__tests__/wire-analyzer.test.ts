import { describe, it, expect } from "vitest";
import { analyzeWires } from "../wire-analyzer";
import type { ElementData, WireData } from "../../../core/types";

describe("analyzeWires", () => {
  it("should detect signal bindings from signal/value to view props", () => {
    const elements: ElementData[] = [
      {
        id: "sig-1",
        nodeType: "codeblock",
        tag: "signal",
        props: { name: "count", typeAnnotation: "number" },
        parentId: null,
        source: "",
      },
      {
        id: "view-1",
        nodeType: "view",
        tag: "Text",
        props: { label: "0" },
        children: [],
        parentId: null,
      },
    ];

    const wires: WireData[] = [
      {
        id: "w-1",
        fromId: "sig-1",
        fromPort: "value",
        toId: "view-1",
        toPort: "label",
      },
    ];

    const result = analyzeWires(elements, wires);

    // Signal "count" should be bound to view-1's label prop
    expect(result.signalBindings.has("count")).toBe(true);
    expect(result.signalBindings.get("count")).toEqual([
      { elementId: "view-1", propName: "label" },
    ]);

    // The wired prop "label" should NOT appear in unwiredProps
    const unwiredLabel = result.unwiredProps.filter(
      (p) => p.elementId === "view-1" && p.propName === "label",
    );
    expect(unwiredLabel).toHaveLength(0);

    // storeRefs and computedDeps should be empty for this test
    expect(result.storeRefs.size).toBe(0);
    expect(result.computedDeps.size).toBe(0);
  });

  it("should detect computed signal bindings from computed/value to view props", () => {
    const elements: ElementData[] = [
      {
        id: "comp-1",
        nodeType: "codeblock",
        tag: "computed",
        props: { name: "isDone" },
        parentId: null,
        source: "items.length > 0",
      },
      {
        id: "view-1",
        nodeType: "view",
        tag: "Text",
        props: { label: "" },
        children: [],
        parentId: null,
      },
    ];

    const wires: WireData[] = [
      {
        id: "w-1",
        fromId: "comp-1",
        fromPort: "value",
        toId: "view-1",
        toPort: "label",
      },
    ];

    const result = analyzeWires(elements, wires);

    expect(result.signalBindings.has("isDone")).toBe(true);
    expect(result.signalBindings.get("isDone")).toEqual([
      { elementId: "view-1", propName: "label" },
    ]);
  });

  it("should detect computed deps from signals wired to computed.deps", () => {
    const elements: ElementData[] = [
      {
        id: "sig-1",
        nodeType: "codeblock",
        tag: "signal",
        props: { name: "items", typeAnnotation: "string[]" },
        parentId: null,
        source: "",
      },
      {
        id: "comp-1",
        nodeType: "codeblock",
        tag: "computed",
        props: { name: "hasItems" },
        parentId: null,
        source: "items.length > 0",
      },
    ];

    const wires: WireData[] = [
      {
        id: "w-1",
        fromId: "sig-1",
        fromPort: "value",
        toId: "comp-1",
        toPort: "deps",
      },
    ];

    const result = analyzeWires(elements, wires);

    // Signal "items" should be a dep of computed "hasItems"
    expect(result.computedDeps.has("hasItems")).toBe(true);
    expect(result.computedDeps.get("hasItems")).toEqual(["items"]);
  });

  it("should detect store references from store/items to view", () => {
    const elements: ElementData[] = [
      {
        id: "store-1",
        nodeType: "codeblock",
        tag: "store",
        props: { name: "TodoStore" },
        parentId: null,
        source: "",
      },
      {
        id: "view-1",
        nodeType: "view",
        tag: "List",
        props: {},
        children: [],
        parentId: null,
      },
    ];

    const wires: WireData[] = [
      {
        id: "w-1",
        fromId: "store-1",
        fromPort: "items",
        toId: "view-1",
        toPort: "data",
      },
    ];

    const result = analyzeWires(elements, wires);

    // Store "TodoStore" should be referenced by view-1
    expect(result.storeRefs.has("TodoStore")).toBe(true);
    expect(result.storeRefs.get("TodoStore")).toEqual(["view-1"]);
  });

  it("should return empty bindings for empty wires array", () => {
    const result = analyzeWires([], []);

    expect(result.signalBindings.size).toBe(0);
    expect(result.storeRefs.size).toBe(0);
    expect(result.computedDeps.size).toBe(0);
    expect(result.unwiredProps).toHaveLength(0);
  });

  it("should ignore wires referencing non-existent elements", () => {
    const elements: ElementData[] = [
      {
        id: "view-1",
        nodeType: "view",
        tag: "div",
        props: {},
        children: [],
        parentId: null,
      },
    ];

    const wires: WireData[] = [
      {
        id: "w-1",
        fromId: "nonexistent",
        fromPort: "value",
        toId: "view-1",
        toPort: "label",
      },
    ];

    const result = analyzeWires(elements, wires);

    // The wire should be silently ignored
    expect(result.signalBindings.size).toBe(0);
    expect(result.storeRefs.size).toBe(0);
  });
});
