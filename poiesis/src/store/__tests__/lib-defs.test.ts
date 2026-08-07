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

  // ── MUST-FIX 1 回归（最终 review）：ROOT action 重载 → 物化 → 保存对齐不删除 ──
  it("ROOT action 补登 libDef.fns 并物化上画布，保存对齐不删除声明", () => {
    const s = usePoiesisStore.getState();
    const actPid = "fn_fetchData_el_act1";
    s.setLibDef("src/lib/a.omni", { imports: [], structs: [], types: [], fns: [] });
    usePoiesisStore.setState({
      codeFlowGraphs: {
        fetchData: {
          elements: [
            codeblock(actPid, "action", { props: { name: "fetchData", params: "x: String, y: Int", isAsync: true } }),
            codeblock("cb_1", "let", { value: "r = x" }),
          ],
          wires: [{ id: "w0", fromId: actPid, fromPort: "flow-out", toId: "cb_1", toPort: "flow-in" }],
        },
      },
    });
    // 重载路径：materializeLibGraphs 把 ROOT action 图补登进 def.fns 并物化元素
    s.materializeLibGraphs("src/lib/a.omni");
    let state = usePoiesisStore.getState();
    expect(state.elements.map((e) => e.id)).toEqual([actPid, "cb_1"]);
    const fns = state.libDefs["src/lib/a.omni"].fns;
    expect(fns).toHaveLength(1);
    expect(fns[0]).toMatchObject({
      pid: actPid, name: "fetchData", declKind: "action", isAsync: true, pub: false, returnType: "",
      params: [{ name: "x", type: "String" }, { name: "y", type: "Int" }],
    });
    // 保存对齐链尾（useSaveProject 同款过滤）：action 声明必须存活
    const workingPids = new Set(
      state.elements.filter((e) => e.nodeType === "codeblock" && (e.tag === "fn" || e.tag === "action")).map((e) => e.id),
    );
    const aligned = fns.filter((f) => !!f.pid && workingPids.has(f.pid));
    expect(aligned.map((f) => f.pid)).toEqual([actPid]);
    // 幂等：二次物化不重复补登
    usePoiesisStore.getState().materializeLibGraphs("src/lib/a.omni");
    state = usePoiesisStore.getState();
    expect(state.libDefs["src/lib/a.omni"].fns).toHaveLength(1);
    expect(state.elements).toHaveLength(2);
  });

  it("组件 action 不补登 lib（pid 被 componentDefs.fns 认领则跳过）", () => {
    const s = usePoiesisStore.getState();
    const compPid = "fn_loadData_el_comp1";
    usePoiesisStore.setState({ componentDefs: {} });
    usePoiesisStore.setState({
      componentDefs: {
        DataFetching: { fns: [{ pid: compPid, name: "loadData", params: [], returnType: "" }] },
      } as never,
    });
    s.setLibDef("src/lib/a.omni", { imports: [], structs: [], types: [], fns: [] });
    usePoiesisStore.setState({
      codeFlowGraphs: {
        loadData: {
          elements: [
            codeblock(compPid, "action", { props: { name: "loadData", isAsync: true } }),
            codeblock("cb_2", "let", { value: "x = 1" }),
          ],
          wires: [],
        },
      },
    });
    s.materializeLibGraphs("src/lib/a.omni");
    const state = usePoiesisStore.getState();
    expect(state.libDefs["src/lib/a.omni"].fns).toHaveLength(0);
    expect(state.elements).toHaveLength(0);
  });
});
