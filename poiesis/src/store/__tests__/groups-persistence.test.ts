// ── 分组持久化闭环测试（Task 1）──
// 保存侧：toProjectJson 直写 codeFlowGroups（键=fn 名）
// 加载侧：loadFromProjectJson 缓存回 store.codeFlowGroups

import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeFlowGroup } from "../../core/types/project-json";

const group = (id: string, name: string): CodeFlowGroup =>
  ({ id, name, memberIds: ["cb_b2", "cb_b3"], color: "#6366f1", collapsed: false });

// group() 引用的成员元素（缺少则被保存侧悬空过滤器剔除）
const MEMBER_ELEMENTS = [
  { id: "cb_b2", parentId: null, nodeType: "codeblock", tag: "fn", props: {}, position: { x: 0, y: 0 } },
  { id: "cb_b3", parentId: null, nodeType: "codeblock", tag: "fn", props: {}, position: { x: 0, y: 0 } },
] as never[];

const reset = () => usePoiesisStore.setState({
  elements: [], wires: [], codeFlowGraphs: {}, componentDefs: {},
  persistedPositions: {}, tabData: {}, codeFlowGroups: {},
});

describe("groups-persistence（分组持久化）", () => {
  beforeEach(reset);

  it("保存侧：toProjectJson 直写 codeFlowGroups（键=fn 名），空数组不写入", () => {
    usePoiesisStore.setState({ elements: MEMBER_ELEMENTS, codeFlowGroups: { increment: [group("g1", "初始化")], empty: [] } });
    const json = usePoiesisStore.getState().toProjectJson();
    expect(json.codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("保存侧：悬空成员组（memberIds 非空但全部失联）被剔除——removeElement 级联删除不清理组的兜底", () => {
    // 成员 cb_b2/cb_b3 在 elements 与 tabData 中均不存在（模拟删除拥有分组的 fn 后）
    usePoiesisStore.setState({ elements: [], codeFlowGroups: { increment: [group("g1", "初始化")] } });
    const json = usePoiesisStore.getState().toProjectJson();
    expect(json.codeFlowGroups).toEqual({});
  });

  it("保存侧：成员仍存在于活动 tab 则保留（部分悬空也保留，只剔全部失联的组）", () => {
    usePoiesisStore.setState({
      elements: [{ id: "cb_b2", parentId: null, nodeType: "codeblock", tag: "fn", props: {}, position: { x: 0, y: 0 } } as never],
      codeFlowGroups: { increment: [group("g1", "初始化")] },
    });
    const json = usePoiesisStore.getState().toProjectJson();
    expect(json.codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("保存侧：成员位于 tabData 槽（非活动 tab）同样视为存在", () => {
    usePoiesisStore.setState({
      elements: [],
      tabData: {
        other: {
          elements: [{ id: "cb_b2", parentId: null, nodeType: "codeblock", tag: "fn", props: {}, position: { x: 0, y: 0 } } as never],
          wires: [], rootOrder: [], selectedElementIds: [],
        },
      },
      codeFlowGroups: { increment: [group("g1", "初始化")] },
    });
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
    usePoiesisStore.setState({ elements: MEMBER_ELEMENTS });
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
