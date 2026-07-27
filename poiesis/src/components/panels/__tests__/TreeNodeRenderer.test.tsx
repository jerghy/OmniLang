import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import { TreeNodeView } from "../TreeNodeRenderer";
import type { TreeNode } from "../tree-utils";

/** Minimal query helper — checks container textContent for a substring. */
function hasText(container: HTMLElement, text: string): boolean {
  return container.textContent?.includes(text) ?? false;
}

/** Query helper — returns the first element whose textContent matches. */
function findByText(container: HTMLElement, text: string): Element | null {
  // Walk all children looking for an exact-text leaf match
  const treeWalker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  let node: Text | null;
  while ((node = treeWalker.nextNode() as Text | null)) {
    if (node.textContent?.trim() === text) return node.parentElement;
  }
  return null;
}

// Helper: minimal props for TreeNodeView
function defaultProps(overrides: Partial<Parameters<typeof TreeNodeView>[0]> = {}) {
  return {
    depth: 0,
    selectedIds: new Set<string>(),
    collapsed: new Set<string>(),
    onToggle: vi.fn(),
    onSelect: vi.fn(),
    onContextMenu: vi.fn(),
    treeDropState: null,
    ...overrides,
  };
}

describe("TreeNodeView — conditional branch nodes", () => {
  it("renders 'if' branch with purple label and condition in monospace", () => {
    const node: TreeNode = {
      id: "br0",
      tag: "(x > 0)",
      htmlTag: "conditional-branch",
      displayLabel: "x > 0",
      nodeType: "conditional-branch",
      branchType: "if",
      condition: "x > 0",
      branchIndex: 0,
      children: [],
    };

    const { container } = render(
      <TreeNodeView node={node} {...defaultProps()} />,
    );

    // Should show "if" label
    expect(hasText(container, "if")).toBe(true);
    // Should show condition in monospace
    const condEl = findByText(container, "x > 0");
    expect(condEl).toBeTruthy();
    expect(condEl!.tagName).toBe("SPAN");
  });

  it("renders 'elif' branch with purple label and condition", () => {
    const node: TreeNode = {
      id: "br1",
      tag: "(b > 0)",
      htmlTag: "conditional-branch",
      displayLabel: "b > 0",
      nodeType: "conditional-branch",
      branchType: "elif",
      condition: "b > 0",
      branchIndex: 1,
      children: [],
    };

    const { container } = render(<TreeNodeView node={node} {...defaultProps()} />);
    expect(hasText(container, "elif")).toBe(true);
    expect(hasText(container, "b > 0")).toBe(true);
  });

  it("renders 'else' branch with gray label and no condition", () => {
    const node: TreeNode = {
      id: "br2",
      tag: "else",
      htmlTag: "conditional-branch",
      displayLabel: "",
      nodeType: "conditional-branch",
      branchType: "else",
      condition: "",
      branchIndex: 2,
      children: [],
    };

    const { container } = render(<TreeNodeView node={node} {...defaultProps()} />);
    expect(hasText(container, "else")).toBe(true);
    // "else" branch should NOT show "(空条件)" — condition empty => no text rendered
    expect(hasText(container, "(空条件)")).toBe(false);
  });

  it("shows '(空条件)' placeholder when if/elif condition is empty", () => {
    const node: TreeNode = {
      id: "br0",
      tag: "(条件)",
      htmlTag: "conditional-branch",
      displayLabel: "",
      nodeType: "conditional-branch",
      branchType: "if",
      condition: "",
      branchIndex: 0,
      children: [],
    };

    const { container } = render(<TreeNodeView node={node} {...defaultProps()} />);
    expect(hasText(container, "(空条件)")).toBe(true);
  });

  it("shows expand/collapse toggle when conditional-branch has children", () => {
    const childNode: TreeNode = {
      id: "child",
      tag: "div",
      htmlTag: "div",
      displayLabel: "",
      children: [],
    };

    const node: TreeNode = {
      id: "br0",
      tag: "(ok)",
      htmlTag: "conditional-branch",
      displayLabel: "ok",
      nodeType: "conditional-branch",
      branchType: "if",
      condition: "ok",
      branchIndex: 0,
      children: [childNode],
    };

    const { container } = render(
      <TreeNodeView node={node} {...defaultProps()} />,
    );

    // The toggle arrow (▼) should be present
    expect(hasText(container, "▼")).toBe(true);
    // The child should be rendered
    expect(hasText(container, "div")).toBe(true);
  });

  it("renders conditional block root node with 🔀 icon", () => {
    const node: TreeNode = {
      id: "cond-1",
      tag: "条件渲染",
      htmlTag: "conditional",
      displayLabel: "",
      nodeType: "conditional",
      children: [],
    };

    const { container } = render(
      <TreeNodeView node={node} {...defaultProps()} />,
    );

    // Should render the tag
    expect(hasText(container, "条件渲染")).toBe(true);
    // Should not render branch label (not a conditional-branch)
    expect(hasText(container, "if")).toBe(false);
    expect(hasText(container, "else")).toBe(false);
  });
});
