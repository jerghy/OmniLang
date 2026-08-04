// ── Project.json serialization slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { ProjectJson } from "../../core/types/project-json";
import { buildCodeFlowGraphs } from "../../services/codegen/chain-emitter";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createSerializationSlice(set: SetFn, get: GetFn) {
  return {
    toProjectJson: (): ProjectJson => {
      const s = get();
      return {
        version: 3,
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
        // 运行时 store.codeFlowGraphs 为解析侧形状（fn 名 → {elements, wires}），
        // 保存时经 buildCodeFlowGraphs 转换为 v3 持久化形状（{pid, nodes, wires}）。
        codeFlowGraphs: buildCodeFlowGraphs(s.codeFlowGraphs ?? {}),
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
      // v3 持久化形状（pid/nodes/wires）不含 CodeblockElementData 的 props/source，
      // 无法无损重建解析侧图——运行时图由 App.tsx loadOmniFiles 物化重建。
      codeFlowGraphs: {},
      // componentDefs kept in runtime state, not persisted — loaded from .omni parsing
    })),
  };
}
