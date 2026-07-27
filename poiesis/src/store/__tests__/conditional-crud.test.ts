import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { ConditionalBlockData } from "../../core/types/element";
import { createInitialDesignPageState } from "../operations/tab-lifecycle";

describe("conditional block CRUD", () => {
  beforeEach(() => {
    // Reset store to known state
    usePoiesisStore.setState({
      elements: [
        { id: "root", nodeType: "view", tag: "div", props: {}, children: [], parentId: null },
      ],
      selectedElementIds: [],
      rootOrder: ["root"],
      isDirty: false,
      undoStacks: {},
      designPage: {
        ...createInitialDesignPageState(),
        activeTabId: "tab-1",
        openTabs: [{ id: "tab-1", filePath: "TestComp.omni", tabType: "component", componentName: "TestComp", isDirty: false, pidMap: { status: "ok", entries: {}, lineToPid: {} } }],
      },
    });
  });

  it("addConditionalBlock creates block with @if + @else branches", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state = usePoiesisStore.getState();
    expect(state.isDirty).toBe(true);
    const condBlock = state.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(condBlock).toBeDefined();
    expect(condBlock!.branches).toHaveLength(2);
    expect(condBlock!.branches[0].type).toBe("if");
    expect(condBlock!.branches[0].condition).toBe("");
    expect(condBlock!.branches[0].children).toEqual([]);
    expect(condBlock!.branches[1].type).toBe("else");
    expect(condBlock!.branches[1].condition).toBe("");
    // Root element should have the conditional block as a child
    const root = state.elements.find((e) => e.id === "root");
    expect(root).toBeDefined();
    expect("children" in root! && root!.children).toContain(condBlock!.id);
  });

  it("addConditionalBranch adds @else if between @if and @else", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;
    store.addConditionalBranch(condBlock.id, "elif");

    const state2 = usePoiesisStore.getState();
    const updated = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(updated.branches).toHaveLength(3);
    expect(updated.branches[0].type).toBe("if");
    expect(updated.branches[1].type).toBe("elif");
    expect(updated.branches[2].type).toBe("else");
  });

  it("updateBranchCondition changes condition expression", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;
    store.updateBranchCondition(condBlock.id, 0, "count > 5");

    const state2 = usePoiesisStore.getState();
    const updated = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(updated.branches[0].condition).toBe("count > 5");
  });

  it("removeConditionalBranch removes an @else if branch (not @if or @else)", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;

    // Add an @else if branch (index 1)
    store.addConditionalBranch(condBlock.id, "elif");

    const state2 = usePoiesisStore.getState();
    const beforeRemove = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(beforeRemove.branches).toHaveLength(3);
    expect(beforeRemove.branches[1].type).toBe("elif");

    // Remove the @else if branch at index 1
    store.removeConditionalBranch(condBlock.id, 1);

    const state3 = usePoiesisStore.getState();
    const afterRemove = state3.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterRemove.branches).toHaveLength(2);
    expect(afterRemove.branches[0].type).toBe("if");
    expect(afterRemove.branches[1].type).toBe("else");
  });

  it("undo restores addConditionalBranch", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;
    const prevBranches = condBlock.branches.map((b) => ({ ...b }));

    // Add an @else if branch
    store.addConditionalBranch(condBlock.id, "elif");

    const state2 = usePoiesisStore.getState();
    const afterAdd = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterAdd.branches).toHaveLength(3);

    // Undo should restore to 2 branches
    store.undo();
    const state3 = usePoiesisStore.getState();
    const afterUndo = state3.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterUndo.branches).toHaveLength(2);
    expect(afterUndo.branches[0].type).toBe(prevBranches[0].type);
    expect(afterUndo.branches[1].type).toBe(prevBranches[1].type);
  });

  it("undo restores updateBranchCondition", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;

    // Update condition
    store.updateBranchCondition(condBlock.id, 0, "count > 5");

    const state2 = usePoiesisStore.getState();
    const afterUpdate = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterUpdate.branches[0].condition).toBe("count > 5");

    // Undo should restore condition to ""
    store.undo();
    const state3 = usePoiesisStore.getState();
    const afterUndo = state3.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterUndo.branches[0].condition).toBe("");
  });

  it("undo restores addChildToBranch", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;

    // Add a child element and move it to branch 0
    store.addElement({
      id: "undo-child-1", nodeType: "view", tag: "span", props: {},
      children: [], parentId: condBlock.id,
    });
    store.addChildToBranch(condBlock.id, 0, "undo-child-1");

    const state2 = usePoiesisStore.getState();
    const afterAdd = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterAdd.branches[0].children).toContain("undo-child-1");

    // Undo should remove the child from the branch
    store.undo();
    const state3 = usePoiesisStore.getState();
    const afterUndo = state3.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterUndo.branches[0].children).not.toContain("undo-child-1");
  });

  it("undo restores removeChildFromBranch", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;

    // Add a child and put it in branch 0
    store.addElement({
      id: "undo-child-2", nodeType: "view", tag: "span", props: {},
      children: [], parentId: condBlock.id,
    });
    store.addChildToBranch(condBlock.id, 0, "undo-child-2");

    // Now remove it
    store.removeChildFromBranch(condBlock.id, 0, "undo-child-2");

    const state2 = usePoiesisStore.getState();
    const afterRemove = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterRemove.branches[0].children).not.toContain("undo-child-2");

    // Undo should restore the child to the branch
    store.undo();
    const state3 = usePoiesisStore.getState();
    const afterUndo = state3.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterUndo.branches[0].children).toContain("undo-child-2");
  });

  it("undo restores removeConditionalBranch", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;

    // Add an @else if branch
    store.addConditionalBranch(condBlock.id, "elif");

    const state2 = usePoiesisStore.getState();
    const beforeRemove = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(beforeRemove.branches).toHaveLength(3);

    // Remove the @else if branch
    store.removeConditionalBranch(condBlock.id, 1);

    const state3 = usePoiesisStore.getState();
    expect(state3.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData).toBeDefined();

    // Undo should restore the @else if branch
    store.undo();
    const state4 = usePoiesisStore.getState();
    const afterUndo = state4.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(afterUndo.branches).toHaveLength(3);
    expect(afterUndo.branches[1].type).toBe("elif");
  });

  it("removeConditionalBlock deletes block and its branch children", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;

    // Add a child element inside the @if branch
    store.addElement({
      id: "child-1", nodeType: "view", tag: "span", props: {},
      children: [], parentId: condBlock.id,
    });
    // Move it to branch 0
    store.addChildToBranch(condBlock.id, 0, "child-1");

    store.removeElement(condBlock.id);

    const state2 = usePoiesisStore.getState();
    expect(state2.elements.find((e) => e.id === condBlock.id)).toBeUndefined();
    // Child should also be deleted (via existing removeElement cascade)
    expect(state2.elements.find((e) => e.id === "child-1")).toBeUndefined();
    // Root should no longer reference condBlock
    const root = state2.elements.find((e) => e.id === "root");
    expect("children" in root! && root!.children).not.toContain(condBlock.id);
  });
});
