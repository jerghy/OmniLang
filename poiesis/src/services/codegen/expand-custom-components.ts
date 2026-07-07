// poiesis/src/services/codegen/expand-custom-components.ts
// Custom component expansion — inline referenced components' element trees
// so the pipeline can process them as a flat element set.

import { getComponentDefinitions } from "../../store";
import type { ElementData } from "../../core/types";

/** Minimal shape of a component definition's element needed for expansion. */
interface DefElement {
  id: string;
  parentId: string | null;
  children: string[];
  tag?: string;
  nodeType?: string;
  componentRef?: string;
  props?: Record<string, unknown>;
  [key: string]: unknown;
}

/** Narrow a component definition to an object with an elements array. */
function getDefElements(def: Record<string, unknown>): DefElement[] | null {
  if (!def || typeof def !== "object") return null;
  const maybe = def as Record<string, unknown>;
  if (!Array.isArray(maybe.elements)) return null;
  return maybe.elements as DefElement[];
}

/**
 * Expand custom component references in the elements array.
 *
 * For each element with `componentRef`, look up the component definition
 * from the registry and clone its internal elements as children of the
 * referencing element. Handles recursive expansion (custom components
 * used inside custom components).
 *
 * Uses deterministic IDs (based on instance element ID) to ensure the
 * generated omniLang source is stable across recompiles, so the compilerCache
 * can hit correctly and the preview iframe doesn't reload unnecessarily.
 */
export function expandCustomComponents(elements: ElementData[]): ElementData[] {
  const registry = getComponentDefinitions();

  // Check for elements that reference custom components (by componentRef OR by tag in registry)
  const hasRefs = elements.some((el) => el.componentRef || registry[el.tag]);
  if (!hasRefs) return elements;

  // Build element map (shallow clone so we can safely update children arrays)
  const elementMap = new Map<string, ElementData>();
  for (const el of elements) {
    elementMap.set(el.id, { ...el });
  }

  const expandedRefs = new Set<string>();

  // Keep expanding until no new componentRefs are found (handles nesting)
  let foundNew = true;
  while (foundNew) {
    foundNew = false;
    for (const [id, el] of [...elementMap]) {
      // Match by componentRef first, then by tag if the tag is in the registry
      const compRef: string | undefined = el.componentRef || (registry?.[el.tag] ? el.tag : undefined);
      if (!compRef || expandedRefs.has(id)) continue;
      expandedRefs.add(id);

      const def = registry?.[compRef];
      const defElements = def ? getDefElements(def) : null;
      if (!defElements || defElements.length === 0) continue;

      // Map old definition element IDs -> deterministic IDs based on instance element ID
      const idMap = new Map<string, string>();
      let idx = 0;
      for (const defEl of defElements) {
        idMap.set(defEl.id, `${el.id}__${idx++}`);
      }

      // Clone definition elements with remapped IDs and parent references
      const rootIds: string[] = [];
      for (const defEl of defElements) {
        const newId = idMap.get(defEl.id)!;
        const isRoot = defEl.parentId === null;

        const cloned: ElementData = {
          ...defEl,
          id: newId,
          nodeType: "view" as const,
          tag: defEl.tag ?? "div",
          props: defEl.props ?? {},
          parentId: isRoot
            ? el.id
            : idMap.get(defEl.parentId ?? "") || defEl.parentId,
          children: defEl.children.map((cid) => idMap.get(cid) || cid),
        };

        elementMap.set(newId, cloned);
        foundNew = true;
        if (isRoot) rootIds.push(newId);
      }

      // Append definition root IDs to the referencing element's children
      const updatedContainer = { ...elementMap.get(id)! };
      let changed = false;
      for (const rootId of rootIds) {
        if (!updatedContainer.children.includes(rootId)) {
          updatedContainer.children = [...updatedContainer.children, rootId];
          changed = true;
        }
      }
      if (changed) elementMap.set(id, updatedContainer);
    }
  }

  return [...elementMap.values()];
}
