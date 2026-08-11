import { describe, it, expect } from "vitest";
import {
  buildTabSaveEntry,
  tabRestoreCanvasPartial,
} from "../tab-helpers";
import type { ElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";

describe("buildTabSaveEntry", () => {
  it("should produce a TabDataEntry with all fields from source", () => {
    const source = {
      elements: [] as ElementData[],
      wires: [] as WireData[],
      rootOrder: [] as string[],
      selectedElementIds: [] as string[],
    };

    const entry = buildTabSaveEntry(source);

    expect(entry).toEqual({
      elements: [],
      wires: [],
      rootOrder: [],
      selectedElementIds: [],
      filePath: undefined,
      selectedActionPid: null,
    });
  });

  it("should preserve non-empty fields from source", () => {
    const source = {
      elements: [
        {
          id: "el-1",
          nodeType: "view" as const,
          tag: "div",
          props: { className: "app" },
          children: [],
          parentId: null,
        },
      ],
      wires: [
        { id: "w-1", fromId: "sig-1", fromPort: "value", toId: "view-1", toPort: "label" },
      ],
      rootOrder: ["el-1"],
      selectedElementIds: ["el-1"],
    };

    const entry = buildTabSaveEntry(source);

    expect(entry.elements).toEqual(source.elements);
    expect(entry.wires).toEqual(source.wires);
    expect(entry.rootOrder).toEqual(source.rootOrder);
    expect(entry.selectedElementIds).toEqual(source.selectedElementIds);
  });
});

describe("tabRestoreCanvasPartial", () => {
  const dummyEl: ElementData = {
    id: "el-1",
    nodeType: "view",
    tag: "div",
    props: {},
    children: [],
    parentId: null,
  };

  it("should restore entry fields correctly", () => {
    const entry = {
      elements: [dummyEl],
      wires: [{ id: "w-1", fromId: "sig-1", fromPort: "value", toId: "view-1", toPort: "label" }],
      rootOrder: ["el-1"],
      selectedElementIds: ["el-1"],
    };

    const result = tabRestoreCanvasPartial(entry);

    expect(result.elements).toEqual([dummyEl]);
    expect(result.wires).toEqual([
      { id: "w-1", fromId: "sig-1", fromPort: "value", toId: "view-1", toPort: "label" },
    ]);
    expect(result.rootOrder).toEqual(["el-1"]);
    expect(result.selectedElementIds).toEqual(["el-1"]);
  });

  it("should use fallback elements when entry is undefined", () => {
    const fallback: ElementData[] = [dummyEl];

    const result = tabRestoreCanvasPartial(undefined, fallback);

    expect(result.elements).toEqual(fallback);
    // Other fields fall back to empty arrays
    expect(result.wires).toEqual([]);
    expect(result.rootOrder).toEqual([]);
    expect(result.selectedElementIds).toEqual([]);
  });

  it("should return empty arrays when entry and fallback are both missing (null tabId)", () => {
    const result = tabRestoreCanvasPartial(undefined);

    expect(result.elements).toEqual([]);
    expect(result.wires).toEqual([]);
    expect(result.rootOrder).toEqual([]);
    expect(result.selectedElementIds).toEqual([]);
  });
});
