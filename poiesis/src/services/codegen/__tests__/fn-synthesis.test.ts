// poiesis/src/services/codegen/__tests__/fn-synthesis.test.ts
// F1：parseFn 合成 fn 元素（@pid 稳定 id + 首语句 wire）——
// 组件 fn 的 codeFlowGraphs 恒含 fn 入口节点，保存时 pipeline 能找到 fn 编译。

import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parseFn 合成 fn 元素（F1 解析侧）", () => {
  it("codeFlowGraphs 含 fn 元素（id=@pid）并 wire 到首语句", () => {
    const src = `
// @pid:fn_a1
component Counter {
    signal count: i32 = 0;
    // @pid:fn_a1
    fn increment(step: i32) {
        // @pid:cb_b2
        count = count + step;
    }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    const graph = result.codeFlowGraphs?.["increment"];
    const fnEl = graph?.elements.find((e) => e.tag === "fn");
    expect(fnEl?.id).toBe("fn_a1");
    expect(fnEl?.props?.name).toBe("increment");
    expect(graph?.wires.some((w) => w.fromId === "fn_a1" && w.toId === "cb_b2")).toBe(true);
  });
  it("空 body 的 fn：仅 fn 元素，无 wire", () => {
    const src = `
component A {
    fn empty() { }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    const graph = result.codeFlowGraphs?.["empty"];
    expect(graph?.elements).toHaveLength(1);
    expect(graph?.elements[0]?.tag).toBe("fn");
    expect(graph?.wires).toHaveLength(0);
  });
  it("重复解析幂等：二次 parse 不叠加 fn 元素", () => {
    const src = `
// @pid:fn_a1
component Counter {
    // @pid:fn_a1
    fn increment() {
        // @pid:cb_b2
        count = 1;
    }
    view { div { } }
}
`;
    const r1 = parseOmniSource(src);
    const graph = r1.codeFlowGraphs?.["increment"];
    expect(graph?.elements.filter((e) => e.tag === "fn")).toHaveLength(1);
  });
});
