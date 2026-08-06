// view 级 @for 解析测试（2026-08-06 补全：parser-view 支持 @for → ForBlockData）。
// 闭环：@for 源码 → 设计树 ForBlockData → buildViewTree → ForViewNode → 发射回 @for。
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { buildViewTree } from "../view-tree-builder";
import type { ForBlockData } from "../../../core/types/element";

const COMP = (body: string) => `import omniview;

component Foo {
    view {
${body}
    }
}`;

function parseFor(source: string): ForBlockData {
  const result = parseOmniSource(source);
  const forEl = result.designElements?.find((e) => e.nodeType === "for") as ForBlockData | undefined;
  if (!forEl) throw new Error("no for block parsed: " + JSON.stringify(result.designElements?.map((e) => e.nodeType)));
  return forEl;
}

describe("parseViewDirective @for", () => {
  it("解析 @for item in items 基本形态（itemName/collection/children）", () => {
    const src = COMP(`        <ul>
            @for item in items {
                <li>{item}</li>
            }
        </ul>`);
    const forEl = parseFor(src);
    expect(forEl.itemName).toBe("item");
    expect(forEl.collection).toBe("items");
    expect(forEl.children.length).toBe(1);
    expect(forEl.emptyChildren).toEqual([]);
    // children 的 parentId 指向 for 块
    const result = parseOmniSource(src);
    const child = result.designElements?.find((e) => e.id === forEl.children[0]);
    expect(child?.parentId).toBe(forEl.id);
  });

  it("解析 store 限定 collection（TodoStore.items）", () => {
    const forEl = parseFor(COMP(`        @for todo in TodoStore.items {
            <p>{todo.text}</p>
        }`));
    expect(forEl.itemName).toBe("todo");
    expect(forEl.collection).toBe("TodoStore.items");
  });

  it("解析 key={expr}", () => {
    const forEl = parseFor(COMP(`        @for item in items key={item.id} {
            <p>{item.name}</p>
        }`));
    expect(forEl.collection).toBe("items");
    expect(forEl.keyExpr).toBe("item.id");
  });

  it("解析 @empty 分支（emptyChildren 归属 for 块）", () => {
    const src = COMP(`        @for item in items {
            <p>{item}</p>
        } @empty {
            <p>No items</p>
        }`);
    const forEl = parseFor(src);
    expect(forEl.emptyChildren.length).toBe(1);
    const result = parseOmniSource(src);
    const emptyChild = result.designElements?.find((e) => e.id === forEl.emptyChildren[0]);
    expect(emptyChild?.parentId).toBe(forEl.id);
  });

  it("buildViewTree → ForViewNode（iterable/itemBinding/emptyBody）", () => {
    const src = COMP(`        @for item in items key={item.id} {
            <p>{item}</p>
        } @empty {
            <p>Empty</p>
        }`);
    const result = parseOmniSource(src);
    const forEl = result.designElements?.find((e) => e.nodeType === "for") as ForBlockData;
    const { roots } = buildViewTree(result.designElements ?? [], "Foo");
    const forNode = roots.find((n) => n.kind === "for") as { iterable: string; itemBinding: string; keyExpr?: string; children: unknown[]; emptyBody?: unknown[] } | undefined;
    expect(forNode).toBeDefined();
    expect(forNode!.iterable).toBe("items");
    expect(forNode!.itemBinding).toBe("item");
    expect(forNode!.keyExpr).toBe("item.id");
    expect(forNode!.children.length).toBe(1);
    expect(forNode!.emptyBody?.length).toBe(1);
    void forEl;
  });

  it("@for 与 @if 混合解析（互不干扰，@if 内可嵌套 @for）", () => {
    const src = COMP(`        @if show {
            @for item in items {
                <p>{item}</p>
            }
        } @else {
            <p>hidden</p>
        }`);
    const result = parseOmniSource(src);
    const forEl = result.designElements?.find((e) => e.nodeType === "for") as ForBlockData | undefined;
    const condEl = result.designElements?.find((e) => e.nodeType === "conditional");
    expect(forEl).toBeDefined();
    expect(condEl).toBeDefined();
    // 嵌套：for 的 parentId 指向 conditional 块
    expect(forEl!.parentId).toBe(condEl!.id);
  });
});
