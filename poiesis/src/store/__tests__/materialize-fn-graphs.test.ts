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
    // brief 逐字断言；ElementData 联合类型（GroupElementData 无 props）下用 as never 保证 tsc 零错误
    expect(((el as never)?.props?.name as string)).toBe("EDITED");
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
