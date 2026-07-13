import { describe, it, expect, expectTypeOf } from "vitest";
import type {
  ElementData,
  ConditionalBlockData,
  ViewElementData,
} from "../element";
import type { TreeNode } from "../../../components/panels/tree-utils";

describe("ConditionalBlockData", () => {
  it("should create a conditional block with @if and @else branches", () => {
    const block: ConditionalBlockData = {
      id: "cond-1",
      parentId: "parent-1",
      nodeType: "conditional",
      branches: [
        { type: "if", condition: "isVisible", children: ["el-1", "el-2"] },
        { type: "else", condition: "", children: ["el-3"] },
      ],
      children: ["el-1", "el-2", "el-3"],
    };

    expect(block.nodeType).toBe("conditional");
    expect(block.branches).toHaveLength(2);
    expect(block.branches[0].type).toBe("if");
    expect(block.branches[0].condition).toBe("isVisible");
    expect(block.branches[1].type).toBe("else");
    expect(block.branches[1].condition).toBe("");
    expect(block.children).toEqual(["el-1", "el-2", "el-3"]);
  });

  it("should create a conditional block with @if, @elif, and @else branches", () => {
    const block: ConditionalBlockData = {
      id: "cond-2",
      parentId: "parent-1",
      nodeType: "conditional",
      branches: [
        { type: "if", condition: "isAdmin", children: ["el-admin"] },
        { type: "elif", condition: "isUser", children: ["el-user"] },
        { type: "else", condition: "", children: ["el-guest"] },
      ],
      children: ["el-admin", "el-user", "el-guest"],
      isFolded: false,
      position: { x: 100, y: 200 },
    };

    expect(block.branches).toHaveLength(3);
    expect(block.branches[1].type).toBe("elif");
    expect(block.branches[1].condition).toBe("isUser");
    expect(block.isFolded).toBe(false);
    expect(block.position).toEqual({ x: 100, y: 200 });
  });

  it("should be assignable to ElementData union type", () => {
    const elements: ElementData[] = [];

    const viewEl: ViewElementData = {
      id: "view-1",
      parentId: null,
      nodeType: "view",
      tag: "div",
      props: {},
      children: [],
    };

    const condBlock: ConditionalBlockData = {
      id: "cond-1",
      parentId: null,
      nodeType: "conditional",
      branches: [{ type: "if", condition: "isVisible", children: ["view-1"] }],
      children: ["view-1"],
    };

    elements.push(viewEl);
    elements.push(condBlock);

    expect(elements).toHaveLength(2);
    expect(elements[0].nodeType).toBe("view");
    expect(elements[1].nodeType).toBe("conditional");
  });

  it("should allow type narrowing on ElementData", () => {
    const block: ElementData = {
      id: "cond-1",
      parentId: null,
      nodeType: "conditional",
      branches: [{ type: "if", condition: "x > 0", children: [] }],
      children: [],
    };

    // Type narrowing test — verify that discriminated union works
    if (block.nodeType === "conditional") {
      expect(block.branches[0].condition).toBe("x > 0");
    } else {
      // TypeScript should narrow this away — test just validates the type compiles
      throw new Error("Expected conditional nodeType");
    }
  });

  it("should have correct type structure", () => {
    // Type-level assertions (compile-time checks)
    expectTypeOf<ConditionalBlockData>().toHaveProperty("nodeType");
    expectTypeOf<ConditionalBlockData>().toHaveProperty("branches");
    expectTypeOf<ConditionalBlockData>().toHaveProperty("children");
    expectTypeOf<ConditionalBlockData>().toHaveProperty("id");
    expectTypeOf<ConditionalBlockData>().toHaveProperty("parentId");

    // Verify branch type structure
    type Branch = ConditionalBlockData["branches"][number];
    expectTypeOf<Branch>().toHaveProperty("type");
    expectTypeOf<Branch>().toHaveProperty("condition");
    expectTypeOf<Branch>().toHaveProperty("children");
  });
});

describe("TreeNode type extension", () => {
  it("should support conditional rendering fields in TreeNode", () => {
    const conditionalNode: TreeNode = {
      id: "cond-1",
      tag: "conditional",
      htmlTag: "conditional",
      displayLabel: "IF isVisible",
      nodeType: "conditional",
      children: [
        {
          id: "branch-0",
          tag: "conditional-branch",
          htmlTag: "conditional-branch",
          displayLabel: "@if isVisible",
          nodeType: "conditional-branch",
          branchType: "if",
          condition: "isVisible",
          branchIndex: 0,
          children: [
            {
              id: "el-1",
              tag: "div",
              htmlTag: "div",
              displayLabel: "Hello",
              children: [],
            },
          ],
        },
        {
          id: "branch-1",
          tag: "conditional-branch",
          htmlTag: "conditional-branch",
          displayLabel: "@else",
          nodeType: "conditional-branch",
          branchType: "else",
          condition: "",
          branchIndex: 1,
          children: [
            {
              id: "el-2",
              tag: "p",
              htmlTag: "p",
              displayLabel: "World",
              children: [],
            },
          ],
        },
      ],
    };

    expect(conditionalNode.nodeType).toBe("conditional");
    expect(conditionalNode.children).toHaveLength(2);

    const ifBranch = conditionalNode.children[0];
    expect(ifBranch.nodeType).toBe("conditional-branch");
    expect(ifBranch.branchType).toBe("if");
    expect(ifBranch.condition).toBe("isVisible");
    expect(ifBranch.branchIndex).toBe(0);

    const elseBranch = conditionalNode.children[1];
    expect(elseBranch.nodeType).toBe("conditional-branch");
    expect(elseBranch.branchType).toBe("else");
    expect(elseBranch.condition).toBe("");
    expect(elseBranch.branchIndex).toBe(1);
  });

  it("should create a plain view node without optional fields", () => {
    const viewNode: TreeNode = {
      id: "view-1",
      tag: "div",
      htmlTag: "div",
      displayLabel: "Hello",
      children: [],
    };

    expect(viewNode.nodeType).toBeUndefined();
    expect(viewNode.condition).toBeUndefined();
    expect(viewNode.branchType).toBeUndefined();
    expect(viewNode.branchIndex).toBeUndefined();
  });

  it("should have correct optional type structure on TreeNode", () => {
    // Type-level assertions verifying optional fields exist
    expectTypeOf<TreeNode>().toHaveProperty("nodeType");
    expectTypeOf<TreeNode>().toHaveProperty("condition");
    expectTypeOf<TreeNode>().toHaveProperty("branchType");
    expectTypeOf<TreeNode>().toHaveProperty("branchIndex");
  });
});
