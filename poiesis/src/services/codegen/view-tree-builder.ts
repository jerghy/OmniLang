// poiesis/src/services/codegen/view-tree-builder.ts
import type { ElementData, ViewElementData, ConditionalBlockData } from "../../core/types";
import type { ViewNode, ElementViewNode, IfViewNode } from "./types";

/** Look up any element by ID — view elements from byId cache, others from flat array. */
function findElement(
  cid: string,
  byId: Map<string, ViewElementData>,
  allElements: ElementData[],
): ElementData | undefined {
  return byId.get(cid) ?? allElements.find((e) => e.id === cid);
}

/** Get a stable path component from an element for tree path generation. */
function pathTag(el: ElementData): string {
  if (el.nodeType === "view") return (el as ViewElementData).tag.toLowerCase();
  if (el.nodeType === "conditional") return "if";
  return el.nodeType;
}

/**
 * Build a view tree from flat element array.
 *
 * Custom components are detected by PascalCase tag names (e.g. "MyCard").
 * Built-in OmniLang elements use lowercase HTML tag names (div, p, button, etc.)
 * and are never treated as custom component references.
 */
export function buildViewTree(
  elements: ElementData[],
  compName: string,
  rootOrder?: string[],
): { roots: ViewNode[]; pascalCaseTags: string[] } {
  const byId = new Map<string, ViewElementData>();
  const pascalCaseTags: string[] = [];

  for (const el of elements) {
    if (el.nodeType === "view") {
      if (byId.has(el.id)) {
        const existing = byId.get(el.id)!;
        if (existing.parentId !== null && el.parentId === null) {
          byId.set(el.id, el);
        }
      } else {
        byId.set(el.id, el);
      }
    }
  }

  function buildNode(el: ElementData, currentPath: string): ViewNode {
    // ── Conditional block → IfViewNode ──
    if (el.nodeType === "conditional") {
      const cond = el as ConditionalBlockData;
      const condBranches = cond.branches.map((b) => ({
        condition: b.condition,
        body: b.children
          .map((cid) => findElement(cid, byId, elements))
          .filter((c): c is ElementData => c !== undefined)
          .map((c) => buildNode(c, `${cond.id}.${pathTag(c)}`)),
      }));

      const first = condBranches[0];
      const lastBranch = cond.branches[cond.branches.length - 1];
      const fallback =
        lastBranch.type === "else" && lastBranch.condition === ""
          ? condBranches[condBranches.length - 1].body
          : null;
      const elifBranches = fallback
        ? condBranches.slice(1, -1)
        : condBranches.slice(1);

      const branches = [
        { condition: first.condition, body: first.body },
        ...elifBranches.map((b) => ({ condition: b.condition, body: b.body })),
      ];

      return {
        kind: "if",
        id: cond.id,
        path: currentPath,
        condition: first.condition,
        branches,
        fallback,
        children: condBranches.flatMap((b) => b.body),
      } as IfViewNode;
    }

    // ── View element ──
    const viewEl = el as ViewElementData;

    const isPascal = /^[A-Z]/.test(viewEl.tag);
    if (isPascal && !pascalCaseTags.includes(viewEl.tag)) {
      pascalCaseTags.push(viewEl.tag);
    }

    // Resolve children — use findElement to also pick up conditional blocks
    const childElements = (viewEl.children || [])
      .map((cid) => findElement(cid, byId, elements))
      .filter((c): c is ElementData => c !== undefined);
    const children = childElements.map((c, i) =>
      buildNode(c, `${currentPath}.${pathTag(c)}.${i}`),
    );

    return {
      id: viewEl.id,
      path: currentPath,
      tag: viewEl.tag,
      props: { ...viewEl.props },
      children,
      kind: "element",
      isFolded: viewEl.isFolded,
      renderMode: viewEl.renderMode,
      classRefs: viewEl.classRefs,
      variantRef: viewEl.variantRef,
      componentClasses: viewEl.componentClasses,
      inlineStyle: viewEl.inlineStyle,
    } as ElementViewNode;
  }

  // Use rootOrder for root element ordering; fall back to byParent(null) order
  let rootElements: ElementData[];
  if (rootOrder) {
    rootElements = rootOrder
      .map((id) => elements.find((el) => el.id === id))
      .filter((el): el is ElementData => el !== undefined);
  } else {
    rootElements = elements.filter(
      (el): el is ElementData => el.parentId === null
    );
  }
  const roots = rootElements.map((el) => buildNode(el, compName));

  return { roots, pascalCaseTags };
}
