// poiesis/src/components/canvas/code/blocks/__tests__/BlockSvg.test.tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { BlockSvg, TextSlot, Keyword } from "../BlockSvg";

describe("BlockSvg", () => {
  it("renders SVG g with data-block-id", () => {
    const { container } = render(
      <svg>
        <BlockSvg colour="#FFAB19" blockId="test-1">
          <Keyword text="test" />
        </BlockSvg>
      </svg>,
    );
    const g = container.querySelector("[data-block-id='test-1']");
    expect(g).toBeTruthy();
  });

  it("renders rect with correct fill", () => {
    const { container } = render(
      <svg>
        <BlockSvg colour="#FFAB19" blockId="test-fill">
          <Keyword text="x" />
        </BlockSvg>
      </svg>,
    );
    const rect = container.querySelector("rect");
    expect(rect?.getAttribute("fill")).toBe("#FFAB19");
  });
});

describe("TextSlot", () => {
  it("shows value when present", () => {
    const { container } = render(
      <svg>
        <text>
          <TextSlot slotId="s1" value="hello" placeholder="name" />
        </text>
      </svg>,
    );
    expect(container.textContent).toContain("hello");
  });

  it("shows placeholder when value is empty", () => {
    const { container } = render(
      <svg>
        <text>
          <TextSlot slotId="s1" value="" placeholder="name" />
        </text>
      </svg>,
    );
    expect(container.textContent).toContain("name");
  });
});
