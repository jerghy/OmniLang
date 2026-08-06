// poiesis/src/services/codegen/__tests__/parser-view-class.test.ts
// class 双形式解析（spec D5）：class="x" 属性形式 + class: "a b" 字符串形式 → classRefs。
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

describe("class 属性解析（D5）", () => {
  it("class=\"app\" → classRefs 包含 app（不再静默丢弃）", () => {
    const el = firstEl(COMP(`        <div class="app" />`));
    expect(el.style?.classRefs).toContain("app");
  });

  it("class=\"a b\" 空白拆分为多个类名", () => {
    const el = firstEl(COMP(`        <div class="a b" />`));
    expect(el.style?.classRefs).toEqual(expect.arrayContaining(["a", "b"]));
  });

  it("class: \"a b\" 字符串形式 → classRefs（原 STRING 分支丢失）", () => {
    const el = firstEl(COMP(`        <div class: "a b" />`));
    expect(el.style?.classRefs).toEqual(expect.arrayContaining(["a", "b"]));
  });

  it("class: LearnDesign.Card 设计类引用回归（IDENTIFIER 路径不变）", () => {
    const el = firstEl(COMP(`        <div class: LearnDesign.Card />`));
    expect(el.style?.classRefs).toContain("LearnDesign.Card");
  });

  it("class=\"x\" 与后续属性共存", () => {
    const el = firstEl(COMP(`        <div class="app" id="main" />`));
    expect(el.style?.classRefs).toContain("app");
    expect(el.id).toBe("main");
  });
});
