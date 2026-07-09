// poiesis/src/services/codegen/view-tree-builder.ts
import type { ElementData, ViewElementData } from "../../core/types";
import type { ViewNode, ElementViewNode } from "./types";

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
    if (el.nodeType !== "view") continue;
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

  function buildNode(el: ViewElementData, currentPath: string): ViewNode {
    // Only PascalCase tags that don't match a built-in entry are custom components.
    const isPascal = /^[A-Z]/.test(el.tag);
    if (isPascal && !pascalCaseTags.includes(el.tag)) {
      pascalCaseTags.push(el.tag);
    }

    // Use el.children order instead of flat-array iteration order
    const childElements = (el.children || [])
      .map((cid) => byId.get(cid))
      .filter((c): c is ViewElementData => c !== undefined);
    const children = childElements.map((c, i) =>
      buildNode(c, `${currentPath}.${c.tag.toLowerCase()}.${i}`),
    );

    return {
      id: el.id,
      path: currentPath,
      tag: el.tag,
      props: { ...el.props },
      children,
      kind: "element",
      isFolded: el.isFolded,
      renderMode: el.renderMode,
      classRefs: el.classRefs,
      variantRef: el.variantRef,
      componentClasses: el.componentClasses,
      inlineStyle: el.inlineStyle,
    } as ElementViewNode;
  }

  // Use rootOrder for root element ordering; fall back to byParent(null) order
  let rootElements: ViewElementData[];
  if (rootOrder) {
    rootElements = rootOrder
      .map((id) => byId.get(id))
      .filter((el): el is ViewElementData => el !== undefined && el.nodeType === "view");
  } else {
    rootElements = elements.filter((el): el is ViewElementData => el.parentId === null && el.nodeType === "view");
  }
  const roots = rootElements.map((el) => buildNode(el, compName));

  return { roots, pascalCaseTags };
}
