// ── Project.json serialization slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { ProjectJson } from "../../core/types/project-json";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createSerializationSlice(set: SetFn, get: GetFn) {
  return {
    toProjectJson: (): ProjectJson => {
      const s = get();
      return {
        version: 2,
        nodes: s.nodeMeta,
        wires: s.wireMeta,
        architecture: {
          mode: s.architecture.mode,
          expandedBubbles: [...s.architecture.expandedBubbleIds].sort(),
          focusComponent: s.architecture.focusComponentId,
          camera: s.architecture.camera,
        },
        canvasStates: s.canvasStates,
        panels: s.panels,
      };
    },

    loadFromProjectJson: (json: ProjectJson) => set((s) => ({
      nodeMeta: json.nodes || {},
      wireMeta: json.wires || {},
      architecture: {
        ...s.architecture,
        ...(json.architecture ? {
          mode: json.architecture.mode || "overall",
          expandedBubbleIds: json.architecture.expandedBubbles || [],
          focusComponentId: json.architecture.focusComponent || null,
          camera: json.architecture.camera || { x: 0, y: 0, zoom: 1 },
        } : {}),
      },
      canvasStates: json.canvasStates || {},
      panels: json.panels || s.panels,
      // componentDefs kept in runtime state, not persisted — loaded from .omni parsing
    })),
  };
}
