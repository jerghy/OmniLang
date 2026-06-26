// core/state/styles.ts
// Style operations as pure functions — extracted from store/editor.ts v2.
// Decoupled from elements store; operate via a callback that receives
// the current elements array and returns the updated array.

import type { StyleEntry } from "../types/style";

export const styleOperations = {
  addStyleEntry: (elementId: string, entry: StyleEntry, updateElements: (fn: (elements: unknown[]) => unknown[]) => void) => {
    updateElements((elements) =>
      elements.map((e: Record<string, unknown>) =>
        e.id === elementId
          ? {
              ...e,
              styles: [...((e.styles as StyleEntry[]) || []), entry],
            }
          : e
      )
    );
  },

  removeStyleEntry: (elementId: string, entryId: string, updateElements: (fn: (elements: unknown[]) => unknown[]) => void) => {
    updateElements((elements) =>
      elements.map((e: Record<string, unknown>) =>
        e.id === elementId
          ? {
              ...e,
              styles: ((e.styles as StyleEntry[]) || []).filter(
                (s: StyleEntry) => s.id !== entryId
              ),
            }
          : e
      )
    );
  },

  updateStyleEntry: (elementId: string, entryId: string, patch: Partial<StyleEntry>, updateElements: (fn: (elements: unknown[]) => unknown[]) => void) => {
    updateElements((elements) =>
      elements.map((e: Record<string, unknown>) =>
        e.id === elementId
          ? {
              ...e,
              styles: ((e.styles as StyleEntry[]) || []).map(
                (s: StyleEntry) => (s.id === entryId ? { ...s, ...patch } : s)
              ),
            }
          : e
      )
    );
  },

  reorderStyleEntries: (elementId: string, entryIds: string[], updateElements: (fn: (elements: unknown[]) => unknown[]) => void) => {
    updateElements((elements) => {
      const el = (elements as Record<string, unknown>[]).find((e) => e.id === elementId) as Record<string, unknown> | undefined;
      if (!el?.styles) return elements;
      const byId = new Map(
        (el.styles as StyleEntry[]).map((s: StyleEntry) => [s.id, s])
      );
      if (!entryIds.every((id) => byId.has(id))) return elements;
      const reordered = entryIds.map((id) => byId.get(id)!);
      return (elements as Record<string, unknown>[]).map((e) =>
        e.id === elementId ? { ...e, styles: reordered } : e
      );
    });
  },

  getStyles: (elementId: string, elements: unknown[]) => {
    const el = (elements as Record<string, unknown>[]).find(
      (e) => e.id === elementId
    );
    return (el?.styles as StyleEntry[]) || [];
  },
};
