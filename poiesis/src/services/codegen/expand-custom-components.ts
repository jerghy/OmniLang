// editor/client/src/services/codegen/expand-custom-components.ts
// Custom component expansion — inline referenced components' element trees
// so the pipeline can process them as a flat element set.

import { useComponentRegistry } from "../../core/state/componentRegistry";
import type { ComponentDefinition } from "../../core/state/componentRegistry";
import type { ElementData } from "../../core/types";

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
  const registry = useComponentRegistry.getState().definitions;

  const hasRefs = elements.some((el) => el.componentRef);
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
      if (!el.componentRef || expandedRefs.has(id)) continue;
      expandedRefs.add(id);

      const def: ComponentDefinition | undefined = registry[el.componentRef];
      if (!def || def.elements.length === 0) continue;

      // Map old definition element IDs -> deterministic IDs based on instance element ID
      const idMap = new Map<string, string>();
      let idx = 0;
      for (const defEl of def.elements) {
        idMap.set(defEl.id, `${el.id}__${idx++}`);
      }

      // Clone definition elements with remapped IDs and parent references
      const rootIds: string[] = [];
      for (const defEl of def.elements) {
        const newId = idMap.get(defEl.id)!;
        const isRoot = defEl.parentId === null;

        const cloned: ElementData = {
          ...defEl,
          id: newId,
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
