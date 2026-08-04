import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeblockElementData } from "../../core/types/element";

const codeEl = (id: string, tag: string, name?: string): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {} });

const reset = () => usePoiesisStore.setState({
  elements: [], wires: [], codeFlowGraphs: {}, componentDefs: {},
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
        increment: { elements: [codeEl("fn_a1", "fn", "increment"), codeEl("cb_b2", "let", "x"), codeEl("cb_b3", "return")], wires: [] },
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

  it("无 fn 元数据的组件：no-op", () => {
    usePoiesisStore.setState({ componentDefs: { Empty: { name: "Empty", fns: [] } as never } });
    expect(() => usePoiesisStore.getState().materializeFnGraphs("Empty")).not.toThrow();
    expect(usePoiesisStore.getState().elements).toEqual([]);
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
