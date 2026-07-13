import { describe, it, expect } from "vitest";
import { buildViewTree } from "../view-tree-builder";
import type { ElementData, ConditionalBlockData, ViewElementData } from "../../../core/types/element";

describe("buildViewTree with conditional blocks", () => {
  it("converts ConditionalBlockData to IfViewNode", () => {
    const childA: ViewElementData = {
      id: "el-a", nodeType: "view", tag: "div", props: { label: "A" },
      children: [], parentId: "cond-1",
    };
    const childB: ViewElementData = {
      id: "el-b", nodeType: "view", tag: "span", props: { label: "B" },
      children: [], parentId: "cond-1",
    };
    const condBlock: ConditionalBlockData = {
      id: "cond-1", nodeType: "conditional", parentId: null,
      branches: [
        { type: "if", condition: "x > 0", children: ["el-a"] },
        { type: "else", condition: "", children: ["el-b"] },
      ],
      children: ["el-a", "el-b"],
    };

    const elements: ElementData[] = [condBlock, childA, childB];
    const result = buildViewTree(elements, "Test");

    expect(result.roots).toHaveLength(1);
    const root = result.roots[0];
    expect(root.kind).toBe("if");
    if (root.kind === "if") {
      expect(root.condition).toBe("x > 0");
      // branches[] now only contains if + elif (not else)
      expect(root.branches).toHaveLength(1);
      expect(root.branches[0].condition).toBe("x > 0");
      expect(root.branches[0].body).toHaveLength(1);
      // else branch lives exclusively in fallback
      expect(root.fallback).toBeDefined();
      expect(root.fallback!.length).toBe(1);
    }
  });
});
