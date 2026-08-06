// 文件级调试覆盖：ownerKey → 目标文件精确定位，修复跨组件同名冲突。
import { describe, it, expect } from "vitest";
import { applyDebugOverrides, resolveOwnerFile } from "../debug-overrides";

const C = { componentName: "Card", storePaths: { TodoStore: "src/stores/TodoStore.omni" } };

describe("resolveOwnerFile", () => {
  it("组件名 → 组件文件", () => {
    expect(resolveOwnerFile("Card", C)).toBe("src/components/Card.omni");
  });
  it("main → src/main.omni", () => {
    expect(resolveOwnerFile("main", { ...C, componentName: "main" })).toBe("src/main.omni");
  });
  it("独立 store → store 文件", () => {
    expect(resolveOwnerFile("TodoStore", C)).toBe("src/stores/TodoStore.omni");
  });
  it("内联 store（Component.Store）→ 组件文件", () => {
    expect(resolveOwnerFile("Card.CartStore", C)).toBe("src/components/Card.omni");
  });
  it("未知 owner → null", () => {
    expect(resolveOwnerFile("Nope", C)).toBeNull();
  });
});

describe("applyDebugOverrides", () => {
  const sources = {
    "src/components/Card.omni": [
      "component Card {",
      '  prop title: String = "Hello";',
      "  signal count: i32 = 0;",
      "  signal status: String;",
      "}",
    ].join("\n"),
    "src/main.omni": "signal status: String = 'x';",
  };

  it("按文件精确替换 signal 默认值（同名跨文件不冲突）", () => {
    const values = {
      Card: {
        status: { text: '"loading"', valid: true },
      },
    };
    const out = applyDebugOverrides(sources, values, C);
    expect(out["src/components/Card.omni"]).toContain('signal status: String = "loading";');
    expect(out["src/main.omni"]).toBe("signal status: String = 'x';"); // 未动
  });

  it("prop 同样支持覆盖", () => {
    const out = applyDebugOverrides(sources, { Card: { title: { text: '"New"', valid: true } } }, C);
    expect(out["src/components/Card.omni"]).toContain('prop title: String = "New";');
  });

  it("无默认值 signal 补 = 值", () => {
    const out = applyDebugOverrides(sources, { Card: { status: { text: '"ready"', valid: true } } }, C);
    expect(out["src/components/Card.omni"]).toContain('signal status: String = "ready";');
  });

  it("valid=false 不参与覆盖（源码保持）", () => {
    const out = applyDebugOverrides(sources, { Card: { count: { text: "abc", valid: false } } }, C);
    expect(out["src/components/Card.omni"]).toContain("signal count: i32 = 0;");
  });

  it("store 文件内覆盖（qualified 键）", () => {
    const src = { "src/stores/TodoStore.omni": "store TodoStore {\n  signal items: Array<String> = [];\n}" };
    const out = applyDebugOverrides(src, { TodoStore: { items: { text: '["a"]', valid: true } } }, C);
    expect(out["src/stores/TodoStore.omni"]).toContain('signal items: Array<String> = ["a"];');
  });

  it("带空格泛型类型（Map<String, String>）不产生双等号", () => {
    const src = { "src/components/Card.omni": "component Card {\n  signal map: Map<String, String> = {};\n}" };
    const out = applyDebugOverrides(src, { Card: { map: { text: '{ "a": 1 }', valid: true } } }, C);
    expect(out["src/components/Card.omni"]).toContain('signal map: Map<String, String> = { "a": 1 };');
    expect(out["src/components/Card.omni"]).not.toContain("= {} =");
  });

  it("覆盖目标不存在 → 该文件原样返回", () => {
    const out = applyDebugOverrides(sources, { Card: { ghost: { text: '"x"', valid: true } } }, C);
    expect(out["src/components/Card.omni"]).toBe(sources["src/components/Card.omni"]);
  });

  it("返回新对象，不修改输入 sources", () => {
    const values = { Card: { count: { text: "5", valid: true } } };
    const before = { ...sources };
    const out = applyDebugOverrides(sources, values, C);
    expect(out).not.toBe(sources);
    expect(sources).toEqual(before);
  });
});
