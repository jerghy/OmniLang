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
  it("空参 fn（handleAdd()）→ params 为空数组（无幻影参数）", () => {
    // E2E 缺陷 1 回归：tokenizer 把 () 拆为独立 token 后捕获组为空格串，
    // 直接 split(",") 会产出幻影参数 [{name:"", type:"String"}] 污染保存签名
    const src = `
component Counter {
    fn handleAdd() -> void {
        // @pid:cb_b2
        return;
    }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    expect(result.fnRegistry?.["Counter"]?.[0]).toMatchObject({ name: "handleAdd", params: [], returnType: "void" });
    // fn 元素 props.params 应 trim 为空（不残留空格伪影）
    expect(result.codeFlowGraphs?.handleAdd.elements.find((e) => e.tag === "fn")?.props?.params).toBe("");
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
  it("ParseResult.componentName 暴露组件声明名（main.omni → \"Main\"，App.tsx materializeFns 键对齐）", () => {
    // 回归：materializeFns 用文件名键（"main"）查 fnRegistry（键=声明名 "Main"）恒空 →
    // componentDefs.fns 空 → materializeFnGraphs 提前返回 → 刷新后 fn 不上画布。
    const src = `
component Main {
    fn newFn() -> void {
        newSignal = "demo";
    }
    view { div { } }
}
`;
    const result = parseOmniSource(src);
    expect(result.componentName).toBe("Main");
    expect(result.fnRegistry?.[result.componentName ?? ""]).toHaveLength(1);
  });
});
