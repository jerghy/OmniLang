// ── 分组持久化闭环测试（Task 1）──
// 保存侧：toProjectJson 直写 codeFlowGroups（键=fn 名）
// 加载侧：loadFromProjectJson 缓存回 store.codeFlowGroups

import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeFlowGroup } from "../../core/types/project-json";

const group = (id: string, name: string): CodeFlowGroup =>
  ({ id, name, memberIds: ["cb_b2", "cb_b3"], color: "#6366f1", collapsed: false });

const reset = () => usePoiesisStore.setState({
  elements: [], wires: [], codeFlowGraphs: {}, componentDefs: {},
  persistedPositions: {}, tabData: {}, codeFlowGroups: {},
});

describe("groups-persistence（分组持久化）", () => {
  beforeEach(reset);

  it("保存侧：toProjectJson 直写 codeFlowGroups（键=fn 名），空数组不写入", () => {
    usePoiesisStore.setState({ codeFlowGroups: { increment: [group("g1", "初始化")], empty: [] } });
    const json = usePoiesisStore.getState().toProjectJson();
    expect(json.codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("加载侧：loadFromProjectJson 恢复 codeFlowGroups；旧 JSON 无字段 → {} 兜底", () => {
    usePoiesisStore.getState().loadFromProjectJson({
      version: 3, nodes: {}, wires: {}, architecture: { mode: "overall", expandedBubbles: [], focusComponent: null, camera: { x: 0, y: 0, zoom: 1 } },
      canvasStates: {}, panels: { leftWidth: 260, rightWidth: 300 },
      codeFlowGraphs: {}, persistedPositions: {},
      codeFlowGroups: { increment: [group("g1", "初始化")] },
    } as never);
    expect(usePoiesisStore.getState().codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("闭环：setCodeFlowGroups → toProjectJson → loadFromProjectJson → 分组完整保持", () => {
    usePoiesisStore.getState().setCodeFlowGroups("increment", [group("g1", "初始化")]);
    const json = usePoiesisStore.getState().toProjectJson();
    usePoiesisStore.setState({ codeFlowGroups: {} });
    usePoiesisStore.getState().loadFromProjectJson(json);
    expect(usePoiesisStore.getState().codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("setCodeFlowGroups 整体替换：传 [] 清空该 fn 分组", () => {
    usePoiesisStore.getState().setCodeFlowGroups("increment", [group("g1", "初始化")]);
    usePoiesisStore.getState().setCodeFlowGroups("increment", []);
    expect(usePoiesisStore.getState().codeFlowGroups.increment).toEqual([]);
  });
});
