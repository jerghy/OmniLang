// ── Standalone Architecture Store ──
// Manages the architecture bubble-graph state independently of the monolithic PoiesisStore.
// The monolithic store subscribes to this store for reactive sync.

import { create } from "zustand";
import type { ArchitectureState, BubbleNode, BubbleEdge } from "../core/types/architecture";

// ── Store Interface ──

export interface ArchitectureSlice {
  architecture: ArchitectureState;

  // ── Actions ──
  setArchMode: (mode: "overall" | "local") => void;
  expandBubble: (bubbleId: string) => void;
  collapseBubble: (bubbleId: string) => void;
  focusComponent: (componentId: string | null) => void;
  selectBubbleByPid: (pid: string | null) => void;
  setArchCamera: (camera: { x: number; y: number; zoom: number }) => void;
  setBubblesAndEdges: (bubbles: BubbleNode[], edges: BubbleEdge[]) => void;
  /** Bulk update — used when loading project JSON */
  setArchitectureState: (partial: Partial<ArchitectureState>) => void;
}

// ── Initial State ──

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

// ── Store ──

export const useArchitectureStore = create<ArchitectureSlice>((set) => ({
  architecture: createInitialArchitectureState(),

  // ── Mode ──

  setArchMode: (mode) => set((s) => ({ architecture: { ...s.architecture, mode } })),

  // ── Bubble expand/collapse ──

  expandBubble: (bubbleId) => set((s) => {
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

  collapseBubble: (bubbleId) => set((s) => ({
    architecture: {
      ...s.architecture,
      expandedBubbleIds: s.architecture.expandedBubbleIds.filter(id => id !== bubbleId),
    },
  })),

  // ── Focus ──

  focusComponent: (componentId) => set((s) => ({
    architecture: {
      ...s.architecture,
      mode: componentId ? "local" : "overall",
      focusComponentId: componentId,
    },
  })),

  // ── Selection ──

  selectBubbleByPid: (pid) => set((s) => ({
    architecture: { ...s.architecture, selectedPid: pid },
  })),

  // ── Camera ──

  setArchCamera: (camera) => set((s) => ({
    architecture: { ...s.architecture, camera },
  })),

  // ── Graph data ──

  setBubblesAndEdges: (bubbles, edges) => set((s) => ({
    architecture: { ...s.architecture, bubbles, edges },
  })),

  // ── Bulk update ──

  setArchitectureState: (partial) => set((s) => ({
    architecture: { ...s.architecture, ...partial },
  })),
}));
