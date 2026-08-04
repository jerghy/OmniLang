// poiesis/src/services/codegen/__tests__/fn-registry.test.ts
// F2：解析侧 fnRegistry 登记（组件维度 fn 元数据，componentDefs.fns 物化来源）。

import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parseFn fnRegistry（F2）", () => {
  it("组件 fn → fnRegistry[组件名] 含 name/params/returnType", () => {
    const src = `
// @pid:fn_a1
component Counter {
    signal count: i32 = 0;
    // @pid:fn_a1
    fn increment(step: i32) -> i32 {
        // @pid:cb_b2
        count = count + step;
        return count;
    }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    const fns = result.fnRegistry?.["Counter"];
    expect(fns).toHaveLength(1);
    expect(fns?.[0]).toMatchObject({ name: "increment", params: [{ name: "step", type: "i32" }], returnType: "i32" });
  });
  it("无 fn 的组件不出现在 fnRegistry", () => {
    const src = `
component Empty {
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    expect(result.fnRegistry).toBeUndefined();
  });
  it("多组件各自登记", () => {
    const src = `
component A {
    fn fa() { }
    view { div { } }
}
component B {
    fn fb() { }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    expect(result.fnRegistry?.["A"]?.map((f) => f.name)).toEqual(["fa"]);
    expect(result.fnRegistry?.["B"]?.map((f) => f.name)).toEqual(["fb"]);
  });
});
