// ── Metadata slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { PidEntry } from "../../core/types/pid";
import type { NodeMeta, WireMeta } from "../../core/types/project-json";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createMetadataSlice(set: SetFn, _get?: GetFn) {
  return {
    // ── PID index ──
    setPidIndex: (index: Record<string, PidEntry>) => set({ pidIndex: index }),
    updatePidEntry: (pid: string, entry: PidEntry) => set((s) => ({
      pidIndex: { ...s.pidIndex, [pid]: entry },
    })),

    // ── Node/wire metadata ──
    setNodeMeta: (pid: string, meta: NodeMeta) => set((s) => ({
      nodeMeta: { ...s.nodeMeta, [pid]: meta },
    })),
    setWireMeta: (wireId: string, meta: WireMeta) => set((s) => ({
      wireMeta: { ...s.wireMeta, [wireId]: meta },
    })),
    removeNodeMeta: (pid: string) => set((s) => {
      const { [pid]: _, ...rest } = s.nodeMeta;
      return { nodeMeta: rest };
    }),
    removeWireMeta: (wireId: string) => set((s) => {
      const { [wireId]: _, ...rest } = s.wireMeta;
      return { wireMeta: rest };
    }),
    setCanvasState: (filePath: string, state: { zoom: number; panX: number; panY: number }) => set((s) => ({
      canvasStates: { ...s.canvasStates, [filePath]: state },
    })),
    setPanelSize: (side: "left" | "right", width: number) => set((s) => ({
      panels: { ...s.panels, [side === "left" ? "leftWidth" : "rightWidth"]: width },
    })),
  };
}
