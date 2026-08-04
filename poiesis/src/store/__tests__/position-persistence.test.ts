// ── 节点位置持久化闭环测试（Task 3）──
// 保存侧：toProjectJson 从 elements + tabData 槽收集 codeblock position → persistedPositions
// 加载侧：materializeFnGraphs 物化新元素时命中 persistedPositions → 用持久化位置（行布局兜底）

import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeblockElementData } from "../../core/types/element";

const codeEl = (id: string, tag: string, name?: string, position?: { x: number; y: number }): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {}, position });

const reset = () => usePoiesisStore.setState({
  elements: [], wires: [], codeFlowGraphs: {}, componentDefs: {}, persistedPositions: {}, tabData: {},
});

const counterDef = { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] };

describe("position-persistence（节点位置持久化）", () => {
  beforeEach(reset);

  // ── 保存侧 ──

  it("保存侧：toProjectJson 收集带 position 的 codeblock → persistedPositions（pid → {x,y}），无 position 跳过", () => {
    usePoiesisStore.setState({
      elements: [
        codeEl("fn_a1", "fn", "increment", { x: 120, y: 340 }),
        codeEl("cb_b2", "let", "x"), // 无 position → 跳过
        codeEl("cb_b3", "return", "r", { x: 400, y: 80 }),
      ],
      tabData: {},
    });
    const json = usePoiesisStore.getState().toProjectJson();
    expect(json.persistedPositions).toEqual({
      fn_a1: { x: 120, y: 340 },
      cb_b3: { x: 400, y: 80 },
    });
  });

  it("保存侧：非活动 tab（tabData 槽）的 codeblock position 也被收集（切 tab 后保存不丢）", () => {
    // store.elements 是活动 tab 工作副本；switchTab 把旧 tab 画布存入 tabData。
    // 只收集 elements 会导致切到别的 tab 后保存时，该组件拖拽布局丢失。
    usePoiesisStore.setState({
      elements: [],
      tabData: {
        tabA: {
          elements: [codeEl("fn_a1", "fn", "increment", { x: 120, y: 340 })],
          wires: [], rootOrder: [], selectedElementIds: [],
        },
      },
    });
    const json = usePoiesisStore.getState().toProjectJson();
    expect(json.persistedPositions).toEqual({ fn_a1: { x: 120, y: 340 } });
  });

  // ── 加载侧 ──

  it("加载侧：物化元素命中 persistedPositions → 用持久化 position（不再行布局兜底）", () => {
    usePoiesisStore.setState({
      elements: [],
      wires: [],
      persistedPositions: { fn_a1: { x: 240, y: 160 } },
      codeFlowGraphs: {
        increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x")], wires: [] },
      },
      componentDefs: { Counter: counterDef } as never,
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const fnEl = usePoiesisStore.getState().elements.find((e) => e.id === "fn_a1");
    expect((fnEl as CodeblockElementData).position).toEqual({ x: 240, y: 160 });
    // 未命中的元素仍走确定性行布局（与持久化位置无关，自 0 起）
    const cbEl = usePoiesisStore.getState().elements.find((e) => e.id === "cb_b2");
    expect((cbEl as CodeblockElementData).position).toEqual({ x: 0, y: 0 });
  });

  it("幂等：重复物化不覆盖画布已有 position（画布编辑优先于 persistedPositions）", () => {
    usePoiesisStore.setState({
      elements: [codeEl("fn_a1", "fn", "increment", { x: 500, y: 500 })],
      wires: [],
      persistedPositions: { fn_a1: { x: 240, y: 160 } },
      codeFlowGraphs: { increment: { elements: [codeEl("fn_a1", "fn", "increment")], wires: [] } },
      componentDefs: { Counter: counterDef } as never,
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const fnEl = usePoiesisStore.getState().elements.find((e) => e.id === "fn_a1");
    expect((fnEl as CodeblockElementData).position).toEqual({ x: 500, y: 500 });
  });

  it("无 persistedPositions 数据 → 回退既有确定性行布局（Task 8 行为不回归）", () => {
    usePoiesisStore.setState({
      elements: [],
      wires: [],
      persistedPositions: {},
      codeFlowGraphs: {
        increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x")], wires: [] },
      },
      componentDefs: { Counter: counterDef } as never,
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const positions = usePoiesisStore.getState().elements
      .filter((e): e is CodeblockElementData => e.nodeType === "codeblock")
      .map((e) => e.position);
    expect(positions).toEqual([{ x: 0, y: 0 }, { x: 0, y: 60 }]);
  });

  // ── 闭环：保存 → 加载 → 物化 ──

  it("闭环：toProjectJson → loadFromProjectJson → materializeFnGraphs 布局保持", () => {
    usePoiesisStore.setState({
      elements: [codeEl("fn_a1", "fn", "increment", { x: 120, y: 340 })],
      wires: [], tabData: {}, persistedPositions: {},
    });
    const json = usePoiesisStore.getState().toProjectJson();
    // 模拟 reload：store 重置后 loadFromProjectJson（codeFlowGraphs 置 {}，由后续 parse 重建）
    usePoiesisStore.setState({
      elements: [], wires: [], codeFlowGraphs: {}, persistedPositions: {}, tabData: {},
      componentDefs: { Counter: counterDef } as never,
    });
    usePoiesisStore.getState().loadFromProjectJson(json);
    expect(usePoiesisStore.getState().persistedPositions).toEqual({ fn_a1: { x: 120, y: 340 } });
    // 解析侧图合并进 codeFlowGraphs 后物化 → position 保持
    usePoiesisStore.setState({
      codeFlowGraphs: { increment: { elements: [codeEl("fn_a1", "fn", "increment")], wires: [] } },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const fnEl = usePoiesisStore.getState().elements.find((e) => e.id === "fn_a1");
    expect((fnEl as CodeblockElementData).position).toEqual({ x: 120, y: 340 });
  });
});
