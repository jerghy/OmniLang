// ── Architecture slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { ArchitectureState, BubbleNode, BubbleEdge } from "../../core/types/architecture";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createInitialArchitectureState(): ArchitectureState {
  return {
    mode: "overall",
    bubbles: [],
    edges: [],
    focusComponentId: null,
    expandedBubbleIds: [],
    selectedPid: null,
    camera: { x: 0, y: 0, zoom: 1 },
  };
}

export function createArchitectureSlice(set: SetFn, _get?: GetFn) {
  return {
    setArchMode: (mode: "overall" | "local") => set((s) => ({ architecture: { ...s.architecture, mode } })),

    expandBubble: (bubbleId: string) => set((s) => {
      const linkedIds = s.architecture.bubbles
        .filter(b => b.id === bubbleId)
        .flatMap(b => {
          const linked = s.architecture.bubbles.filter(
            o => o.depth === b.depth && o.type !== b.type && o.parentId === b.parentId,
          );
          return [b.id, ...linked.map(l => l.id)];
        });
      const unique = [...new Set([...s.architecture.expandedBubbleIds, ...linkedIds])];
      return { architecture: { ...s.architecture, expandedBubbleIds: unique } };
    }),

    collapseBubble: (bubbleId: string) => set((s) => ({
      architecture: { ...s.architecture, expandedBubbleIds: s.architecture.expandedBubbleIds.filter(id => id !== bubbleId) },
    })),

    focusComponent: (componentId: string | null) => set((s) => ({
      architecture: { ...s.architecture, mode: componentId ? "local" : "overall", focusComponentId: componentId },
    })),

    selectBubbleByPid: (pid: string | null) => set((s) => ({
      architecture: { ...s.architecture, selectedPid: pid },
    })),

    setArchCamera: (camera: { x: number; y: number; zoom: number }) => set((s) => ({
      architecture: { ...s.architecture, camera },
    })),

    setBubblesAndEdges: (bubbles: BubbleNode[], edges: BubbleEdge[]) => set((s) => ({
      architecture: { ...s.architecture, bubbles, edges },
    })),
  };
}
