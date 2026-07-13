// poiesis/src/services/codegen/view-tree-builder.ts
import type { ElementData, ViewElementData, ConditionalBlockData } from "../../core/types";
import type { ViewNode, ElementViewNode, IfViewNode } from "./types";

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
      // Prefer root elements (no parentId) over children with same ID,
      // and keep the first entry, not the last, to avoid parent-child confusion
      if (byId.has(el.id)) {
        const existing = byId.get(el.id)!;
        // Existing is already root-ish; prefer new one only if existing has parent
        if (existing.parentId !== null && el.parentId === null) {
          byId.set(el.id, el);
        }
        // Otherwise keep the existing one
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
          .map((cid) => byId.get(cid))
          .filter((c): c is ViewElementData => c !== undefined)
          .map((c) => buildNode(c, `${cond.id}.${c.tag.toLowerCase()}`)),
      }));

      // Separate else branch as fallback; branches[] only contains if + elif
      const first = condBranches[0];
      const lastBranch = cond.branches[cond.branches.length - 1];
      const fallback =
        lastBranch.type === "else" && lastBranch.condition === ""
          ? condBranches[condBranches.length - 1].body
          : null;
      const elifBranches = fallback
        ? condBranches.slice(1, -1)  // exclude first (if) and last (else -> fallback)
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

    // ── Existing view element handling ──
    const viewEl = el as ViewElementData;

    // Only PascalCase tags that don't match a built-in entry are custom components.
    const isPascal = /^[A-Z]/.test(viewEl.tag);
    if (isPascal && !pascalCaseTags.includes(viewEl.tag)) {
      pascalCaseTags.push(viewEl.tag);
    }

    // Use el.children order instead of flat-array iteration order
    const childElements = (viewEl.children || [])
      .map((cid) => byId.get(cid))
      .filter((c): c is ViewElementData => c !== undefined);
    const children = childElements.map((c, i) =>
      buildNode(c, `${currentPath}.${c.tag.toLowerCase()}.${i}`),
    );

    // Note: When a child element's parentId points to a conditional block,
    // it won't be in `byId` (which only contains view elements). The
    // conditional block handles its children in its own buildNode branch
    // above through the flat `children` array.

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
