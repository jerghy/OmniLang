import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { compileCodeFlowGraph } from "../graph-compiler";

// 形态来自 learn-omniview pages/DataFetching.omni（2 个 async action，spec §11 G1 回归文件）
const fixture = `component DataFetching {
  signal data: String = "";
  action loadData() async {
    data = "loading";
    return;
  }
  view {
    <div id="root"></div>
  }
}
`;

describe("组件 action（G1 修复）", () => {
  it("解析：action 进入 fnRegistry（declKind action）与 codeFlowGraphs", () => {
    const r = parseOmniSource(fixture);
    const reg = r.fnRegistry?.["DataFetching"];
    expect(reg).toBeDefined();
    expect(reg!.map((f) => f.name)).toEqual(["loadData"]);
    expect(reg![0]).toMatchObject({ returnType: "", declKind: "action", isAsync: true });
    expect(r.codeFlowGraphs?.["loadData"]).toBeDefined();
  });

  it("编译-再解析闭环：action 双格式发射且 fixpoint", () => {
    const r = parseOmniSource(fixture);
    const graph = r.codeFlowGraphs!["loadData"];
    const compiled = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
    expect(compiled).toContain("action loadData() async {");
    expect(compiled).not.toContain("->");
    // 再解析 → 再编译 → 不变（pid 注释稳定）
    const r2 = parseOmniSource(compiled, "src/components/DataFetching.omni");
    const g2 = r2.codeFlowGraphs!["loadData"];
    const compiled2 = compileCodeFlowGraph(g2.elements, g2.wires, g2.elements[0].id)!;
    expect(compiled2).toBe(compiled);
  });
});
