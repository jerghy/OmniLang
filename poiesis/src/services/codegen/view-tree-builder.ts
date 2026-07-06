// poiesis/src/omni-gen/view-tree-builder.ts
import type { ElementData } from "../../core/types";
import type { ViewNode } from "./types";

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
  const byId = new Map<string, ElementData>();
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

  function buildNode(el: ElementData, currentPath: string): ViewNode {
    // Only PascalCase tags that don't match a built-in entry are custom components.
    const isPascal = /^[A-Z]/.test(el.tag);
    if (isPascal && !pascalCaseTags.includes(el.tag)) {
      pascalCaseTags.push(el.tag);
    }

    // Use el.children order instead of flat-array iteration order
    const childElements = (el.children || [])
      .map((cid) => byId.get(cid))
      .filter((c): c is ElementData => c !== undefined);
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
    };
  }

  // Use rootOrder for root element ordering; fall back to byParent(null) order
  let rootElements: ElementData[];
  if (rootOrder) {
    rootElements = rootOrder
      .map((id) => byId.get(id))
      .filter((el): el is ElementData => el !== undefined);
  } else {
    rootElements = elements.filter((el) => el.parentId === null && el.nodeType === "view");
  }
  const roots = rootElements.map((el) => buildNode(el, compName));

  return { roots, pascalCaseTags };
}
