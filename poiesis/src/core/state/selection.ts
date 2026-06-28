// core/state/selection.ts
// Selection state atom — extracted from store/editor.ts v2.

import { create } from "zustand";

interface SelectionState {
  selectedElementIds: string[];
  selectElement: (id: string | null, multi?: boolean) => void;
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
}

export const useSelectionStore = create<SelectionState>((set) => ({
  selectedElementIds: [],
  selectElement: (id, multi = false) =>
    set((s) => {
      if (id === null) return { selectedElementIds: [] };
      if (multi) {
        const exists = s.selectedElementIds.includes(id);
        return {
          selectedElementIds: exists
            ? s.selectedElementIds.filter((x) => x !== id)
            : [...s.selectedElementIds, id],
        };
      }
      return { selectedElementIds: [id] };
    }),
  selectAll: (ids) => set({ selectedElementIds: ids }),
  clearSelection: () => set({ selectedElementIds: [] }),
}));
