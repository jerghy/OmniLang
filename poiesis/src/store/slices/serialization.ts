// ── Project.json serialization slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { ElementData } from "../../core/types/element";
import type { ProjectJson } from "../../core/types/project-json";
import { buildCodeFlowGraphs } from "../../services/codegen/chain-emitter";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createSerializationSlice(set: SetFn, get: GetFn) {
  return {
    toProjectJson: (): ProjectJson => {
      const s = get();
      // 分组有效性判定用元素 id 全集（collectPersistedPositions 同款扫描范围：
      // 活动 tab elements + 各 tabData 槽），成员存在于任一画布即视为有效。
      const liveElementIds = new Set(
        [
          ...s.elements.map((e) => e.id),
          ...Object.values(s.tabData).flatMap((tab) => (tab ? tab.elements.map((e) => e.id) : [])),
        ],
      );
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
        // 节点位置独立收集：store.elements 是活动 tab 工作副本，其余 tab 画布在
        // tabData 槽（switchTab 时写入）——两处都扫，避免切 tab 后保存丢位置。
        persistedPositions: collectPersistedPositions(s.elements, s.tabData),
        // 分组直写（键=fn 名，与 codeFlowGraphs 对齐）；空数组 fn、空组（成员删光）与
        // 悬空成员组（memberIds 非空但全部失联——removeElement 级联删除不清理组，终审 I-1）不写入。
        // （空组/悬空组无可见 UI 无法手动删——保存时自动清理，spec §8「空框保留」落地为 JSON 剔除）
        codeFlowGroups: Object.fromEntries(
          Object.entries(s.codeFlowGroups)
            .map(([fnName, gs]) => [
              fnName,
              gs.filter(
                (g) => g.memberIds.length > 0 && g.memberIds.some((id) => liveElementIds.has(id)),
              ),
            ] as const)
            .filter(([, gs]) => gs.length > 0),
        ),
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
      // 位置缓存恢复：物化时消费（旧版 project.json 无此字段 → {} 回退行布局）。
      persistedPositions: json.persistedPositions || {},
      // 分组缓存恢复：画布编辑时经 setCodeFlowGroups 写入（旧版 JSON 无字段 → {} 兜底）。
      codeFlowGroups: json.codeFlowGroups || {},
      // componentDefs kept in runtime state, not persisted — loaded from .omni parsing
    })),
  };
}

/**
 * 收集 codeflow 节点位置：元素 id（即 PID，跨 parse 稳定）→ {x,y}。
 * 仅收集 nodeType 为 codeblock 且带 position 的元素；tabData 槽先扫、
 * 活动 tab（elements）后扫（活动 tab 覆盖，最新编辑优先）。
 */
function collectPersistedPositions(
  activeElements: ElementData[],
  tabData: PoiesisState["tabData"],
): Record<string, { x: number; y: number }> {
  const out: Record<string, { x: number; y: number }> = {};
  for (const tab of Object.values(tabData)) {
    if (!tab) continue;
    for (const el of tab.elements) {
      if (el.nodeType === "codeblock" && el.position !== undefined) out[el.id] = { ...el.position };
    }
  }
  for (const el of activeElements) {
    if (el.nodeType === "codeblock" && el.position !== undefined) out[el.id] = { ...el.position };
  }
  return out;
}
