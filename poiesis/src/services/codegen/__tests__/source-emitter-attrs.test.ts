// poiesis/src/services/codegen/__tests__/source-emitter-attrs.test.ts
// 发射端属性三态输出（spec D6）：{expr} 动态 / boolean 裸 / 静态字符串。
import { describe, test, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { generateOmniSource } from "../pipeline";
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
