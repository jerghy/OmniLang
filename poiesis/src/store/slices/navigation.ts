// ── Navigation slice for Poiesis store ──

import type { PoiesisState } from "../types";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createNavigationSlice(set: SetFn, _get?: GetFn) {
  return {
    navigateTo: (page: "architecture" | "design") => set({ currentPage: page }),
  };
}
