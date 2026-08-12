// poiesis/src/services/codegen/__tests__/source-emitter-attrs.test.ts
// 发射端属性三态输出（spec D6）：{expr} 动态 / boolean 裸 / 静态字符串。
import { describe, test, expect, beforeEach } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { generateOmniSource } from "../pipeline";
import { formatDefaultValue } from "../source-emitter";
import { usePoiesisStore } from "../../../store";
import type { ElementData, CodeTab } from "../../../core/types";

const basicCodeTabs: CodeTab[] = [
  { id: "main", type: "logic", label: "Main", icon: "code", graph: { elements: [], wires: [] }, variables: [] },
];

function emit(elements: ElementData[], componentName = "TestComponent") {
  const result = generateOmniSource({ designElements: elements, designWires: [], codeTabs: basicCodeTabs, componentName });
  const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
  expect(fatalErrors).toHaveLength(0);
  return result.value ?? "";
}

function viewEl(props: Record<string, unknown>, tag = "input"): ElementData {
  return { id: "el-1", nodeType: "view", tag, props, children: [], parentId: null };
}

describe("发射端属性三态（D6）", () => {
  test("{count} 形态 → value={count}", () => {
    const out = emit([viewEl({ type: "text", value: "{count}" })]);
    expect(out).toContain("value={count}");
  });

  test("任意表达式 {fn(x)} 原样输出", () => {
    const out = emit([viewEl({ value: "{inputText.to_string()}" })]);
    expect(out).toContain("value={inputText.to_string()}");
  });

  test("静态字符串 → value=\"Enter name\"（回归）", () => {
    const out = emit([viewEl({ value: "Enter name" })]);
    expect(out).toContain('value="Enter name"');
  });

  test("boolean true → 裸属性 disabled", () => {
    const out = emit([viewEl({ disabled: true })]);
    expect(out).toContain(" disabled");
  });

  test("boolean false → 不发射", () => {
    const out = emit([viewEl({ disabled: false, type: "text" })]);
    expect(out).not.toContain("disabled");
  });

  test("表达式布尔 checked={agreed} 往返", () => {
    const out = emit([viewEl({ type: "checkbox", checked: "{agreed}" })]);
    expect(out).toContain("checked={agreed}");
  });

  test("闭环：<input disabled placeholder=\"x\"> 解析→发射保真", () => {
    const parsed = parseOmniSource(`import omniview;

component Foo {
    view {
        <input disabled placeholder="x" />
    }
}`);
    const out = emit((parsed.designElements ?? []) as ElementData[], "Foo");
    expect(out).toContain("disabled");
    expect(out).toContain('placeholder="x"');
  });

  test("闭环：value={count} 解析→发射保真", () => {
    const parsed = parseOmniSource(`import omniview;

component Foo {
    view {
        <input value={count} />
    }
}`);
    const out = emit((parsed.designElements ?? []) as ElementData[], "Foo");
    expect(out).toContain("value={count}");
  });
});

// A1 实机发现（重开路径）：`signal x: String = ""` 解析 → defaultValue = ""（内容值），
// 发射守卫 `!== ""` 把空串当"无默认值"丢弃 → `signal x: String;` → WASM 编译错误。
// 修复：formatDefaultValue 对 String 空串补引号；守卫只判 undefined。
describe("formatDefaultValue 空串默认值（A1）", () => {
  test("空字符串默认值 → 补引号字面量（区别于无默认值）", () => {
    expect(formatDefaultValue("", "String")).toBe('""');
  });

  test("非空字符串 → 补引号（既有行为）", () => {
    expect(formatDefaultValue("signalcs/", "String")).toBe('"signalcs/"');
  });

  test("已引号字面量原样（parser 内容值 / UI 字面量混源）", () => {
    expect(formatDefaultValue('""', "String")).toBe('""');
    expect(formatDefaultValue('"all"', "String")).toBe('"all"');
  });

  test("undefined/null = 无默认值 → 空串（发射方守卫已拦截）", () => {
    expect(formatDefaultValue(undefined, "String")).toBe("");
    expect(formatDefaultValue(null, "String")).toBe("");
  });

  test("数字/布尔原样（非 String 不补引号）", () => {
    expect(formatDefaultValue(0, "i32")).toBe("0");
    expect(formatDefaultValue(false, "bool")).toBe("false");
  });

  test("parse 端到端：`signal note: String = \"\";` → defaultValue 空串 → 可补引号发射", () => {
    const parsed = parseOmniSource(`import omniview;
component Foo {
    signal note: String = "";
    view { <div/> }
}`);
    const sig = parsed.signals?.find((s) => s.name === "note");
    expect(sig?.defaultValue).toBe("");
    expect(formatDefaultValue(sig?.defaultValue, sig?.type ?? "String")).toBe('""');
  });

  beforeEach(() => {
    usePoiesisStore.setState({ componentDefs: {} });
  });

  test("发射端到端：组件信号空串默认值往返保持（`signal note: String = \"\";` 不丢）", () => {
    // 组件信号来自全局注册表（parser → componentDefs）——直接构造 store 形状
    usePoiesisStore.setState({
      componentDefs: {
        TestComponent: {
          name: "TestComponent",
          filePath: "TestComponent.omni",
          signals: [{ pid: "p1", name: "note", type: "String", defaultValue: "" }],
          props: [],
          computeds: [],
          events: [],
          fns: [],
          lifecycles: [],
        },
      },
    });
    const out = emit([], "TestComponent");
    expect(out).toContain("signal note: String = \"\";");
  });

  test("发射端到端：非 String 空串默认值不产生 `= ;`（跳过默认值发射）", () => {
    usePoiesisStore.setState({
      componentDefs: {
        TestComponent: {
          name: "TestComponent",
          filePath: "TestComponent.omni",
          signals: [{ pid: "p1", name: "count", type: "i32", defaultValue: "" }],
          props: [],
          computeds: [],
          events: [],
          fns: [],
          lifecycles: [],
        },
      },
    });
    const out = emit([], "TestComponent");
    expect(out).toContain("signal count: i32;");
    expect(out).not.toContain("= ;");
  });
});
