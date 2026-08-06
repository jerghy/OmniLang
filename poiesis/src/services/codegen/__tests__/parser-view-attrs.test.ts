// poiesis/src/services/codegen/__tests__/parser-view-attrs.test.ts
// view 元素属性解析（spec D2/D3）：{expr} 花括号保真 + 裸布尔属性 + break 修复。
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import type { ViewElementData } from "../../../core/types/element";

const COMP = (body: string) => `import omniview;

component Foo {
    view {
${body}
    }
}`;

function firstEl(source: string): ViewElementData {
  const result = parseOmniSource(source);
  const el = result.designElements?.find((e) => e.nodeType === "view") as ViewElementData | undefined;
  if (!el) throw new Error("no view element parsed");
  return el;
}

describe("view 元素属性解析：{expr} 动态属性（D2）", () => {
  it("value={count} → props.value === \"{count}\"（花括号保真）", () => {
    const el = firstEl(COMP(`        <input value={count} />`));
    expect(el.props.value).toBe("{count}");
  });

  it("value={inputText.to_string()} 任意表达式保真（原解析失败）", () => {
    const el = firstEl(COMP(`        <input value={inputText.to_string()} />`));
    expect(el.props.value).toBe("{inputText.to_string()}");
  });

  it("嵌套花括号表达式（if 表达式）配平收集", () => {
    const el = firstEl(COMP(`        <input placeholder={if x { "a" } else { "b" }} />`));
    expect(el.props.placeholder).toBe('{if x { "a" } else { "b" }}');
  });

  it("checked={agreed} 布尔属性的表达式形态", () => {
    const el = firstEl(COMP(`        <input type="checkbox" checked={agreed} />`));
    expect(el.props.checked).toBe("{agreed}");
  });

  it("id={x} 保持旧行为（D7：动态 id 范围外）", () => {
    const el = firstEl(COMP(`        <input id={dynId} />`));
    expect(el.id).toBe("dynId");
  });
});

describe("view 元素属性解析：裸布尔属性（D3）", () => {
  it("<input disabled> → props.disabled === true", () => {
    const el = firstEl(COMP(`        <input disabled />`));
    expect(el.props.disabled).toBe(true);
  });

  it("break 修复：<input disabled placeholder=\"x\"> 后续属性不丢", () => {
    const el = firstEl(COMP(`        <input disabled placeholder="x" />`));
    expect(el.props.disabled).toBe(true);
    expect(el.props.placeholder).toBe("x");
  });

  it("多个裸布尔：<button disabled required> 全部解析", () => {
    const el = firstEl(COMP(`        <button disabled required />`));
    expect(el.props.disabled).toBe(true);
    expect(el.props.required).toBe(true);
  });

  it("未知裸属性维持容错（不崩溃、属性循环停止）", () => {
    const el = firstEl(COMP(`        <input unknownattr />`));
    expect(el.tag).toBe("input");
  });
});
