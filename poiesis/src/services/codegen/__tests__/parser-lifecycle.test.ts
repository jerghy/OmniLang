import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";

const fixture = `component Main {
  signal searchText: String = "";
  signal filter: String = "";
  @effect(searchText, setFilter) {
    filterTasks(searchText);
  }
  @mount {
    initRows();
  }
  fn handleClick() {
    return;
  }
  view {
    <div id="root"></div>
  }
}
`;

describe("生命周期（@mount/@effect/@unmount）", () => {
  it("解析：lifecycleRegistry 按组件名登记 + 图按 pid 键", () => {
    const r = parseOmniSource(fixture);
    expect(r.lifecycleRegistry?.["Main"]).toBeDefined();
    expect(r.lifecycleRegistry!["Main"].map((l) => l.hookType)).toEqual(["effect", "mount"]);
    expect(r.lifecycleRegistry!["Main"][0].deps).toBe("searchText,setFilter"); // token 无空格拼接
    expect(r.lifecycleRegistry!["Main"][1].deps).toBeUndefined();
    const pid = r.lifecycleRegistry!["Main"][0].pid;
    expect(r.lifecycleGraphs?.[pid]).toBeDefined();
    // fnRegistry 不含生命周期（生命周期不是 fn）
    expect(r.fnRegistry?.["Main"]?.map((f) => f.name)).toEqual(["handleClick"]);
  });

  it("编译-再解析闭环：deps 括号形式 + fixpoint", () => {
    const r = parseOmniSource(fixture);
    const pid = r.lifecycleRegistry!["Main"][0].pid;
    const graph = r.lifecycleGraphs![pid];
    const compiled = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
    expect(compiled).toContain("@effect(searchText,setFilter) {");
    expect(compiled).toContain("filterTasks(searchText)");
    const compiled2 = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
    expect(compiled2).toBe(compiled);
  });

  it("graph-parser 防御路径：@effect 段直接解析（单测，段级）", () => {
    const graph = parseCodeFlowGraph(`@effect(searchText, setFilter) {
  filterTasks(searchText);
}
`);
    expect(graph.elements[0]).toMatchObject({ tag: "lifecycle" });
    expect(graph.elements[0].props).toMatchObject({ hookType: "effect", deps: "searchText, setFilter" });
    expect(graph.wires).toHaveLength(1);
  });
});
