import { describe, it, expect } from "vitest";
import { buildTreeNode } from "../tree-utils";
import type { ElementData, ConditionalBlockData, ViewElementData } from "../../../core/types/element";

describe("buildTreeNode", () => {
  it("handles ConditionalBlockData with @if and @else branches", () => {
    const childA: ViewElementData = {
      id: "child-a", nodeType: "view", tag: "div", props: { label: "A" },
      children: [], parentId: "cond-1",
    };
    const childB: ViewElementData = {
      id: "child-b", nodeType: "view", tag: "span", props: { label: "B" },
      children: [], parentId: "cond-1",
    };
    const condBlock: ConditionalBlockData = {
      id: "cond-1", nodeType: "conditional", parentId: "root",
      branches: [
        { type: "if", condition: "x > 0", children: ["child-a"] },
        { type: "else", condition: "", children: ["child-b"] },
      ],
      children: ["child-a", "child-b"],
    };

    const elMap = new Map<string, ElementData>();
    elMap.set(condBlock.id, condBlock);
    elMap.set(childA.id, childA);
    elMap.set(childB.id, childB);

    const node = buildTreeNode(condBlock, elMap);

    // Conditional block node
    expect(node.id).toBe("cond-1");
    expect(node.tag).toBe("条件渲染");
    expect(node.htmlTag).toBe("conditional");
    expect(node.nodeType).toBe("conditional");

    // Branch nodes
    expect(node.children).toHaveLength(2);

    // @if branch
    const br0 = node.children[0];
    expect(br0.id).toBe("cond-1.br0");
    expect(br0.tag).toBe("(x > 0)");
    expect(br0.nodeType).toBe("conditional-branch");
    expect(br0.branchType).toBe("if");
    expect(br0.condition).toBe("x > 0");
    expect(br0.branchIndex).toBe(0);

    // @else branch
    const br1 = node.children[1];
    expect(br1.id).toBe("cond-1.br1");
    expect(br1.tag).toBe("else");
    expect(br1.nodeType).toBe("conditional-branch");
    expect(br1.branchType).toBe("else");
    expect(br1.branchIndex).toBe(1);

    // Nested children
    expect(br0.children).toHaveLength(1);
    expect(br0.children[0].id).toBe("child-a");
    expect(br0.children[0].tag).toBe("div");

    expect(br1.children).toHaveLength(1);
    expect(br1.children[0].id).toBe("child-b");
    expect(br1.children[0].tag).toBe("span");
  });

  it("handles empty @if condition with (条件) placeholder", () => {
    const condBlock: ConditionalBlockData = {
      id: "cond-2", nodeType: "conditional", parentId: "root",
      branches: [
        { type: "if", condition: "", children: [] },
        { type: "else", condition: "", children: [] },
      ],
      children: [],
    };

    const elMap = new Map<string, ElementData>();
    elMap.set(condBlock.id, condBlock);

    const node = buildTreeNode(condBlock, elMap);
    expect(node.children[0].tag).toBe("(条件)");
    expect(node.children[0].displayLabel).toBe("");
  });

  it("handles @else if branch with condition", () => {
    const condBlock: ConditionalBlockData = {
      id: "cond-3", nodeType: "conditional", parentId: "root",
      branches: [
        { type: "if", condition: "a > 0", children: [] },
        { type: "elif", condition: "b > 0", children: [] },
        { type: "else", condition: "", children: [] },
      ],
      children: [],
    };

    const elMap = new Map<string, ElementData>();
    elMap.set(condBlock.id, condBlock);

    const node = buildTreeNode(condBlock, elMap);
    const branches = node.children;

    expect(branches).toHaveLength(3);

    // @else if branch
    expect(branches[1].id).toBe("cond-3.br1");
    expect(branches[1].tag).toBe("(b > 0)");
    expect(branches[1].branchType).toBe("elif");
    expect(branches[1].displayLabel).toBe("b > 0");

    // @else branch
    expect(branches[2].id).toBe("cond-3.br2");
    expect(branches[2].tag).toBe("else");
    expect(branches[2].branchType).toBe("else");
    expect(branches[2].displayLabel).toBe("");
  });

  it("preserves displayLabel for @if branch from condition text", () => {
    const condBlock: ConditionalBlockData = {
      id: "cond-4", nodeType: "conditional", parentId: "root",
      branches: [
        { type: "if", condition: "isVisible", children: [] },
        { type: "else", condition: "", children: [] },
      ],
      children: [],
    };

    const elMap = new Map<string, ElementData>();
    elMap.set(condBlock.id, condBlock);

    const node = buildTreeNode(condBlock, elMap);
    expect(node.children[0].displayLabel).toBe("isVisible");
    expect(node.children[1].displayLabel).toBe("");
  });

  it("builds root tree mixing view and conditional block elements", () => {
    const viewEl: ViewElementData = {
      id: "view-1", nodeType: "view", tag: "div", props: {},
      children: [], parentId: "root",
    };
    const condBlock: ConditionalBlockData = {
      id: "cond-5", nodeType: "conditional", parentId: "root",
      branches: [
        { type: "if", condition: "ok", children: [] },
        { type: "else", condition: "", children: [] },
      ],
      children: [],
    };

    const elMap = new Map<string, ElementData>();
    elMap.set(viewEl.id, viewEl);
    elMap.set(condBlock.id, condBlock);

    const rootIds = ["view-1", "cond-5"];
    const tree = rootIds
      .map((id) => elMap.get(id))
      .filter((e): e is ElementData => e != null)
      .map((e) => buildTreeNode(e, elMap));

    expect(tree).toHaveLength(2);
    expect(tree[0].id).toBe("view-1");
    expect(tree[0].tag).toBe("div");
    expect(tree[1].id).toBe("cond-5");
    expect(tree[1].tag).toBe("条件渲染");
    expect(tree[1].nodeType).toBe("conditional");
  });
});
