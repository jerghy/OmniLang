import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeblockElementData } from "../../core/types/element";

const codeEl = (id: string, tag: string, name?: string): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {} });

const reset = () => usePoiesisStore.setState({
  elements: [], wires: [], codeFlowGraphs: {}, lifecycleGraphs: {}, persistedPositions: {}, componentDefs: {},
});

describe("materializeFnGraphs（F1 物化）", () => {
  beforeEach(reset);

  it("合并该组件 fn 图：view 元素保留 + 新 codeblock 加入", () => {
    usePoiesisStore.setState({
      elements: [{ id: "view1", nodeType: "view", tag: "div", parentId: null, source: "" } as never],
      wires: [],
      codeFlowGraphs: {
        increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x")], wires: [{ id: "w1", fromId: "fn_a1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }] },
      },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const els = usePoiesisStore.getState().elements;
    expect(els.map((e) => e.id)).toEqual(["view1", "fn_a1", "cb_b2"]);
    expect(usePoiesisStore.getState().wires).toHaveLength(1);
  });

  it("已存在的 pid 不覆盖（保留画布编辑）", () => {
    usePoiesisStore.setState({
      elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "EDITED")],
      wires: [],
      codeFlowGraphs: { increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "original")], wires: [] } },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const el = usePoiesisStore.getState().elements.find((e) => e.id === "cb_b2");
    // brief 逐字断言；元素实为 codeblock（测试构造），转型为已导入的 CodeblockElementData 访问 props（as never 会报 TS2339）
    expect(((el as CodeblockElementData)?.props?.name as string)).toBe("EDITED");
  });

  it("物化元素无 position 时赋确定性行布局（互不重叠）", () => {
    // E2E 缺陷 3 回归：解析侧不生成 position，物化后应赋 y 递增的行布局避免堆叠 (0,0)
    usePoiesisStore.setState({
      elements: [],
      wires: [],
      codeFlowGraphs: {
        increment: {
          // 显式 position: undefined 与缺键同等对待（终审修复 6：undefined 判定走布局分配）
          elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x"), { ...codeEl("cb_b3", "return"), position: undefined }],
          wires: [],
        },
      },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const positions = usePoiesisStore.getState().elements
      .filter((e): e is CodeblockElementData => e.nodeType === "codeblock")
      .map((e) => e.position)
      .filter((p): p is { x: number; y: number } => Boolean(p));
    // 全部物化元素带 position，且 y 互不相同（不重叠）
    expect(positions).toHaveLength(3);
    expect(new Set(positions.map((p) => p.y)).size).toBe(3);
    // 确定性：重复触发不重新分配（幂等，已存在元素不被覆盖）
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    expect(usePoiesisStore.getState().elements
      .filter((e): e is CodeblockElementData => e.nodeType === "codeblock")
      .map((e) => e.position)).toEqual(positions);
  });

  it("跨组件同名 fn 图：fn 元素 id 与 fnRegistry.pid 不匹配时跳过（防污染）", () => {
    // codeFlowGraphs 全局按 fn 名取图，mergeCodeFlowGraphs 跨文件同名后者覆盖（B 的 increment 图）；
    // 组件 A 的 increment pid（fn_a1）与图内 fn 元素 id（fn_b1）不匹配 → 物化 no-op，
    // 避免 B 的 fn 体被静默物化进 A 的画布/保存
    usePoiesisStore.setState({
      elements: [],
      wires: [],
      codeFlowGraphs: {
        increment: { elements: [codeEl("fn_b1", "fn", "increment"), codeEl("cb_b2", "let", "x")], wires: [] },
      },
      componentDefs: { A: { name: "A", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("A");
    expect(usePoiesisStore.getState().elements).toEqual([]);
  });

  it("无 fn 元数据的组件：no-op", () => {
    usePoiesisStore.setState({ componentDefs: { Empty: { name: "Empty", fns: [] } as never } });
    expect(() => usePoiesisStore.getState().materializeFnGraphs("Empty")).not.toThrow();
    expect(usePoiesisStore.getState().elements).toEqual([]);
  });

  it("全部已物化时空合并短路：不重建数组（elements/wires 引用保持）", () => {
    usePoiesisStore.setState({
      elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x")],
      wires: [{ id: "w1", fromId: "fn_a1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }],
      codeFlowGraphs: {
        increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x")], wires: [{ id: "w1", fromId: "fn_a1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }] },
      },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    const beforeEls = usePoiesisStore.getState().elements;
    const beforeWires = usePoiesisStore.getState().wires;
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    // newElements/newWires 均空 → 短路返回，不 set、不换引用（修复前每调用都新建数组）
    expect(usePoiesisStore.getState().elements).toBe(beforeEls);
    expect(usePoiesisStore.getState().wires).toBe(beforeWires);
  });

  it("codeFlowGraphs 无对应 fn：no-op", () => {
    usePoiesisStore.setState({
      codeFlowGraphs: { other: { elements: [codeEl("z1", "let", "z")], wires: [] } },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    expect(usePoiesisStore.getState().elements).toEqual([]);
  });
});

describe("materializeFnGraphs 生命周期合并（G6）", () => {
  beforeEach(reset);

  const lcEntry = (id: string, hookType: string): CodeblockElementData =>
    ({ id, nodeType: "codeblock", tag: "lifecycle", parentId: null, source: "", props: { hookType } });

  it("仅生命周期（无 fn）：入口元素 + body 元素入画布（早退条件含 lifecycles）", () => {
    // 早退守卫 `fns.length === 0 && lifecycles.length === 0` 的 lifecycles 半边：
    // fns 为空但 lifecycles 非空时必须继续合并，不得提前返回。
    usePoiesisStore.setState({
      elements: [],
      wires: [],
      lifecycleGraphs: {
        lc_mount_1: {
          elements: [lcEntry("lc_mount_1", "mount"), codeEl("cb_b2", "call", "initRows")],
          wires: [{ id: "w_lc_1", fromId: "lc_mount_1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }],
        },
      },
      componentDefs: { Counter: { name: "Counter", fns: [], lifecycles: [{ pid: "lc_mount_1", hookType: "mount" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    expect(usePoiesisStore.getState().elements.map((e) => e.id)).toEqual(["lc_mount_1", "cb_b2"]);
    expect(usePoiesisStore.getState().wires.map((w) => w.id)).toEqual(["w_lc_1"]);
  });

  it("生命周期 pid 归属校验：图缺入口元素（id 不匹配）跳过", () => {
    // 与 fn 跨组件防污染同构：registry pid（lc_mount_1）与图内入口元素 id 不匹配 → 跳过，
    // 避免他人生命周期体被静默物化进本组件画布/保存。
    usePoiesisStore.setState({
      elements: [],
      wires: [],
      lifecycleGraphs: {
        lc_mount_1: {
          elements: [lcEntry("lc_other_9", "mount"), codeEl("cb_b2", "call", "initRows")],
          wires: [{ id: "w_lc_1", fromId: "lc_other_9", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }],
        },
      },
      componentDefs: { Counter: { name: "Counter", fns: [], lifecycles: [{ pid: "lc_mount_1", hookType: "mount" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    expect(usePoiesisStore.getState().elements).toEqual([]);
    expect(usePoiesisStore.getState().wires).toEqual([]);
  });

  it("生命周期位置回退：persistedPositions 优先、layoutIdx 行布局兜底", () => {
    usePoiesisStore.setState({
      elements: [],
      wires: [],
      lifecycleGraphs: {
        lc_effect_1: {
          elements: [
            lcEntry("lc_effect_1", "effect"),
            codeEl("cb_b2", "call", "filterTasks"),
            { ...codeEl("cb_b3", "call", "refresh"), position: undefined },
          ],
          wires: [],
        },
      },
      persistedPositions: { cb_b2: { x: 10, y: 99 } },
      componentDefs: { Counter: { name: "Counter", fns: [], lifecycles: [{ pid: "lc_effect_1", hookType: "effect", deps: "searchText" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const els = usePoiesisStore.getState().elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock");
    // 入口无持久化 → 行布局 (0,0)；cb_b2 命中持久化位置且不占行布局序号；cb_b3 → 下一行 (0,60)
    expect(els.find((e) => e.id === "lc_effect_1")?.position).toEqual({ x: 0, y: 0 });
    expect(els.find((e) => e.id === "cb_b2")?.position).toEqual({ x: 10, y: 99 });
    expect(els.find((e) => e.id === "cb_b3")?.position).toEqual({ x: 0, y: 60 });
  });

  it("生命周期 wire 去重：既有同 id 与 fn 图刚并入的同 id 均不重复 push", () => {
    usePoiesisStore.setState({
      elements: [],
      // 既有 wire 与生命周期图 wire 同 id（w_lc_1）——不得重复并入
      wires: [{ id: "w_lc_1", fromId: "lc_mount_1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }],
      codeFlowGraphs: {
        increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x")], wires: [{ id: "w_shared", fromId: "fn_a1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" }] },
      },
      lifecycleGraphs: {
        lc_mount_1: {
          elements: [lcEntry("lc_mount_1", "mount"), codeEl("cb_b2", "call", "initRows")],
          wires: [
            { id: "w_lc_1", fromId: "lc_mount_1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" },
            // 与 fn 图 wire 同 id——fn 循环已 push，生命周期循环须经 newWires.some 去重
            { id: "w_shared", fromId: "lc_mount_1", fromPort: "flow-out", toId: "cb_b2", toPort: "flow-in" },
          ],
        },
      },
      componentDefs: { Counter: { name: "Counter", fns: [{ pid: "fn_a1", name: "increment", params: [], returnType: "void" }], lifecycles: [{ pid: "lc_mount_1", hookType: "mount" }] } as never },
    });
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    const wireIds = usePoiesisStore.getState().wires.map((w) => w.id);
    expect(wireIds.filter((id) => id === "w_lc_1")).toHaveLength(1);
    expect(wireIds.filter((id) => id === "w_shared")).toHaveLength(1);
    expect(usePoiesisStore.getState().wires).toHaveLength(2);
  });

  it("lifecycleGraphs 无对应 pid：no-op（不重建数组）", () => {
    usePoiesisStore.setState({
      elements: [codeEl("view1", "view")],
      wires: [],
      componentDefs: { Counter: { name: "Counter", fns: [], lifecycles: [{ pid: "lc_mount_1", hookType: "mount" }] } as never },
    });
    const beforeEls = usePoiesisStore.getState().elements;
    usePoiesisStore.getState().materializeFnGraphs("Counter");
    expect(usePoiesisStore.getState().elements).toBe(beforeEls);
  });
});
