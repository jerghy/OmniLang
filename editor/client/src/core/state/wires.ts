// core/state/wires.ts
// Wire connection state atom — extracted from store/editor.ts v2.

import { create } from "zustand";
import type { WireData } from "../types/wire";

interface WireState {
  wires: WireData[];
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setWires: (wires: WireData[]) => void;
}

export const useWireStore = create<WireState>((set) => ({
  wires: [],
  addWire: (w) => set((s) => ({ wires: [...s.wires, w] })),
  removeWire: (id) => set((s) => ({ wires: s.wires.filter((w) => w.id !== id) })),
  setWires: (wires) => set({ wires }),
}));
