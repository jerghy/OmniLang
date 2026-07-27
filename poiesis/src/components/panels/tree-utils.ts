// Tree utilities for ComponentTree.

import type { ElementData, ConditionalBlockData } from "../../core/types/element";

export interface TreeNode {
  id: string;
  tag: string;
  htmlTag: string;
  displayLabel: string;
  componentRef?: string;
  children: TreeNode[];
  // Conditional rendering support
  nodeType?: "view" | "conditional" | "conditional-branch";
  condition?: string;           // only for conditional-branch nodes
  branchType?: "if" | "elif" | "else";  // only for conditional-branch nodes
  branchIndex?: number;         // branch index in the branches array
}

export function resolveHtmlTag(tag: string, _props: Record<string, unknown>): string {
  return tag.toLowerCase();
}

export function getDisplayLabel(props: Record<string, unknown>): string {
  if (props.text) return String(props.text).slice(0, 24);
  if (props.label) return String(props.label).slice(0, 24);
  if (props.placeholder) return String(props.placeholder).slice(0, 24);
  return "";
}

/**
 * Build a TreeNode from an ElementData, recursively resolving children via elMap.
 * Handles conditional blocks, view elements, codeblocks, and groups.
 */
export function buildTreeNode(
  el: ElementData,
  elMap: Map<string, ElementData>,
): TreeNode {
  // ── Conditional block ──
  if (el.nodeType === "conditional") {
    const cond = el as ConditionalBlockData;
    return {
      id: cond.id,
      tag: "条件渲染",
      htmlTag: "conditional",
      displayLabel: "",
      nodeType: "conditional",
      children: cond.branches.map((b, i) => ({
        id: `${cond.id}.br${i}`,
        tag: b.type === "else" ? "else" : `(${b.condition || "条件"})`,
        htmlTag: "conditional-branch",
        displayLabel: b.type === "else" ? "" : b.condition,
        nodeType: "conditional-branch" as const,
        condition: b.condition,
        branchType: b.type,
        branchIndex: i,
        children: b.children
          .map((cid) => elMap.get(cid))
          .filter((e): e is ElementData => e != null)
          .map((c) => buildTreeNode(c, elMap)),
      })),
    };
  }

  // ── View / codeblock / group element ──
  const childrenIds =
    (el as { children?: string[] }).children || [];
  const tag = "tag" in el ? el.tag : "div";
  const props = "props" in el ? (el.props as Record<string, unknown>) : {};
  return {
    id: el.id,
    tag,
    htmlTag: resolveHtmlTag(tag, props),
    displayLabel: getDisplayLabel(props),
    componentRef: "componentRef" in el ? el.componentRef : undefined,
    children: childrenIds
      .map((cid) => elMap.get(cid))
      .filter((e): e is ElementData => e != null)
      .map((c) => buildTreeNode(c, elMap)),
  };
}
