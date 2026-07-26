// poiesis/src/components/canvas/code/blocks/__tests__/BlockSvg.test.tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { BlockSvg, layoutItems } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";

describe("BlockSvg", () => {
  it("renders SVG g with data-block-id", () => {
    const items: LayoutItem[] = [{ kind: "keyword", text: "test" }];
    const { container } = render(
      <svg>
        <BlockSvg colour="#FFAB19" blockId="test-1" items={items} />
      </svg>,
    );
    const g = container.querySelector("[data-block-id='test-1']");
    expect(g).toBeTruthy();
  });

  it("renders rect with correct fill", () => {
    const items: LayoutItem[] = [{ kind: "keyword", text: "x" }];
    const { container } = render(
      <svg>
        <BlockSvg colour="#FFAB19" blockId="test-fill" items={items} />
      </svg>,
    );
    const rect = container.querySelector("rect");
    expect(rect?.getAttribute("fill")).toBe("#FFAB19");
  });
});

describe("layoutItems", () => {
  it("renders keyword text", () => {
    const { elements } = layoutItems([{ kind: "keyword", text: "let" }]);
    expect(elements.length).toBe(1);
  });

  it("renders slot with value", () => {
    const { elements } = layoutItems([
      { kind: "keyword", text: "let" },
      { kind: "slot", slotId: "NAME", value: "count", placeholder: "name" },
    ]);
    expect(elements.length).toBe(2);
  });

  it("renders slot with placeholder when value is empty", () => {
    const { elements } = layoutItems([
      { kind: "slot", slotId: "NAME", value: "", placeholder: "name" },
    ]);
    expect(elements.length).toBe(1);
  });

  it("calculates contentWidth based on items", () => {
    const { elements, contentWidth } = layoutItems([
      { kind: "keyword", text: "let" },
      { kind: "slot", slotId: "NAME", value: "count", placeholder: "name" },
    ]);
    expect(contentWidth).toBeGreaterThan(0);
    expect(elements.length).toBe(2);
  });
});
