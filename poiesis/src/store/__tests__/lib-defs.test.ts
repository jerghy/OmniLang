import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeblockElementData } from "../../core/types/element";

const codeblock = (id: string, tag: string, extra: Record<string, unknown> = {}): CodeblockElementData => ({
  id, nodeType: "codeblock", tag, parentId: null, props: {}, source: "", ...extra,
});

describe("libDefs slice", () => {
  beforeEach(() => {
    // 重置 store（镜像 materialize-fn-graphs.test.ts 的既有 reset 形态）
    usePoiesisStore.setState({ libDefs: {}, codeFlowGraphs: {}, elements: [], wires: [] });
  });

  it("setLibDef 按路径写入且不覆盖其他路径", () => {
    const s = usePoiesisStore.getState();
    s.setLibDef("src/lib/a.omni", { imports: [], structs: [], types: [], fns: [] });
    s.setLibDef("src/lib/b.omni", { imports: [], structs: [], types: [], fns: [] });
    const state = usePoiesisStore.getState();
    expect(Object.keys(state.libDefs)).toEqual(["src/lib/a.omni", "src/lib/b.omni"]);
  });

  it("materializeLibGraphs 幂等合并 fn 图（pid 归属校验 + 缺 position 行布局）", () => {
    const s = usePoiesisStore.getState();
    const pidA = "fn_fetch_el_aaa";
    s.setLibDef("src/lib/a.omni", {
      imports: [],
      structs: [],
      types: [],
      fns: [{ pid: pidA, name: "fetch", params: [], returnType: "", body: "", pub: false, isAsync: true, declKind: "fn" }],
    });
    usePoiesisStore.setState({
      codeFlowGraphs: {
        fetch: {
          elements: [codeblock(pidA, "fn", { name: "fetch" }), codeblock("st_1", "let", { value: "x = 1" })],
          wires: [{ id: "w0", fromId: pidA, fromPort: "flow-out", toId: "st_1", toPort: "flow-in" }],
        },
      },
    });
    s.materializeLibGraphs("src/lib/a.omni");
    let state = usePoiesisStore.getState();
    expect(state.elements.map((e) => e.id)).toEqual([pidA, "st_1"]);
    expect(state.wires).toHaveLength(1);
    // 幂等：二次调用不重复
    usePoiesisStore.getState().materializeLibGraphs("src/lib/a.omni");
    state = usePoiesisStore.getState();
    expect(state.elements).toHaveLength(2);
    // 缺失 position 的元素获得确定性行布局
    expect((state.elements[0] as { position?: unknown }).position).toBeDefined();
  });

  it("pid 归属校验：fn 入口元素 id ≠ f.pid 的图不物化（防跨文件同名污染）", () => {
    const s = usePoiesisStore.getState();
    s.setLibDef("src/lib/a.omni", {
      imports: [], structs: [], types: [],
      fns: [{ pid: "fn_other_el_zzz", name: "fetch", params: [], returnType: "", body: "", pub: false, isAsync: false, declKind: "fn" }],
    });
    usePoiesisStore.setState({
      codeFlowGraphs: {
        fetch: { elements: [codeblock("fn_fetch_el_aaa", "fn")], wires: [] },
      },
    });
    s.materializeLibGraphs("src/lib/a.omni");
    expect(usePoiesisStore.getState().elements).toHaveLength(0);
  });
});
