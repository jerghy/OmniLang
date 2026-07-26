// poiesis/src/components/canvas/code/blocks/__tests__/SelectionOverlay.test.tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { SelectionOverlay } from "../SelectionOverlay";
import type { SelectionTarget } from "../BlockModel";

describe("SelectionOverlay", () => {
  let container: HTMLElement;

  beforeEach(() => {
    // Clean up any injected <style> elements between tests
    document.querySelectorAll("style").forEach((s) => s.remove());
  });

  it("hides overlay when target is null", () => {
    const { container } = render(
      <SelectionOverlay target={null} getElement={() => null} />,
    );
    const div = container.querySelector("div");
    expect(div!.style.display).toBe("none");
  });

  it("hides overlay when getElement returns null", () => {
    const target: SelectionTarget = {
      type: "block",
      blockIndex: 0,
      blockId: "b-1",
    };
    const { container } = render(
      <SelectionOverlay target={target} getElement={() => null} />,
    );
    const div = container.querySelector("div");
    expect(div!.style.display).toBe("none");
  });

  it("positions overlay based on getElement bounding rect", () => {
    const target: SelectionTarget = {
      type: "block",
      blockIndex: 0,
      blockId: "b-1",
    };
    const fakeEl = {
      getBoundingClientRect: () => ({
        x: 100,
        y: 200,
        width: 300,
        height: 40,
        top: 200,
        right: 400,
        bottom: 240,
        left: 100,
      }),
    } as HTMLElement;

    const { container } = render(
      <SelectionOverlay target={target} getElement={() => fakeEl} />,
    );
    const div = container.querySelector("div");
    expect(div!.style.display).toBe("block");
    expect(div!.style.left).toBe("100px");
    expect(div!.style.top).toBe("200px");
    expect(div!.style.width).toBe("300px");
    expect(div!.style.height).toBe("40px");
  });

  it("applies block style for block target type", () => {
    const target: SelectionTarget = {
      type: "block",
      blockIndex: 0,
      blockId: "b-1",
    };
    const fakeEl = {
      getBoundingClientRect: () => ({
        x: 0,
        y: 0,
        width: 100,
        height: 32,
        top: 0,
        right: 100,
        bottom: 32,
        left: 0,
      }),
    } as HTMLElement;

    const { container } = render(
      <SelectionOverlay target={target} getElement={() => fakeEl} />,
    );
    const div = container.querySelector("div");
    expect(div!.style.outline).toBe("2px solid #cc6b4a");
    expect(div!.style.outlineOffset).toBe("2px");
    expect(div!.style.borderRadius).toBe("4px");
    expect(div!.style.animation).toBe("none");
  });

  it("applies slot style for slot target type", () => {
    const target: SelectionTarget = {
      type: "slot",
      blockIndex: 0,
      blockId: "b-1",
      slotName: "NAME",
    };
    const fakeEl = {
      getBoundingClientRect: () => ({
        x: 0,
        y: 0,
        width: 100,
        height: 32,
        top: 0,
        right: 100,
        bottom: 32,
        left: 0,
      }),
    } as HTMLElement;

    const { container } = render(
      <SelectionOverlay target={target} getElement={() => fakeEl} />,
    );
    const div = container.querySelector("div");
    expect(div!.style.border).toBe("2px dashed rgb(204, 107, 74)");
    expect(div!.style.borderRadius).toBe("4px");
    expect(div!.style.animation).toBe("none");
    expect(div!.style.outline).toBe("none");
  });

  it("applies gap style for gap target type with blink animation", () => {
    const target: SelectionTarget = {
      type: "gap",
      position: 0.5,
    };
    const fakeEl = {
      getBoundingClientRect: () => ({
        x: 0,
        y: 0,
        width: 100,
        height: 4,
        top: 0,
        right: 100,
        bottom: 4,
        left: 0,
      }),
    } as HTMLElement;

    const { container } = render(
      <SelectionOverlay target={target} getElement={() => fakeEl} />,
    );
    const div = container.querySelector("div");
    expect(div!.style.borderTop).toBe("2px solid rgb(204, 107, 74)");
    expect(div!.style.animation).toBe("sel-gap-blink 1s step-end infinite");
    expect(div!.style.outline).toBe("none");
  });

  it("updates position when target changes", () => {
    const targetA: SelectionTarget = {
      type: "block",
      blockIndex: 0,
      blockId: "b-1",
    };
    const targetB: SelectionTarget = {
      type: "block",
      blockIndex: 1,
      blockId: "b-2",
    };

    const rectA = {
      x: 10,
      y: 20,
      width: 100,
      height: 32,
      top: 20,
      right: 110,
      bottom: 52,
      left: 10,
    };
    const rectB = {
      x: 50,
      y: 80,
      width: 200,
      height: 32,
      top: 80,
      right: 250,
      bottom: 112,
      left: 50,
    };

    const getElement = vi.fn((t: SelectionTarget) => {
      const rect = t.blockId === "b-1" ? rectA : rectB;
      return { getBoundingClientRect: () => rect } as HTMLElement;
    });

    const { container, rerender } = render(
      <SelectionOverlay target={targetA} getElement={getElement} />,
    );

    let div = container.querySelector("div");
    expect(div!.style.left).toBe("10px");

    rerender(<SelectionOverlay target={targetB} getElement={getElement} />);

    div = container.querySelector("div");
    expect(div!.style.left).toBe("50px");
    expect(div!.style.top).toBe("80px");
    expect(div!.style.width).toBe("200px");
  });

  it("hides when switching from a valid target to null", () => {
    const target: SelectionTarget = {
      type: "block",
      blockIndex: 0,
      blockId: "b-1",
    };
    const fakeEl = {
      getBoundingClientRect: () => ({
        x: 0,
        y: 0,
        width: 100,
        height: 32,
        top: 0,
        right: 100,
        bottom: 32,
        left: 0,
      }),
    } as HTMLElement;

    const { container, rerender } = render(
      <SelectionOverlay target={target} getElement={() => fakeEl} />,
    );

    let div = container.querySelector("div");
    expect(div!.style.display).toBe("block");

    rerender(<SelectionOverlay target={null} getElement={() => fakeEl} />);

    div = container.querySelector("div");
    expect(div!.style.display).toBe("none");
  });

  it("injects blink keyframes style element", () => {
    const { container } = render(
      <SelectionOverlay target={null} getElement={() => null} />,
    );
    const styleEl = container.querySelector("style");
    expect(styleEl).toBeTruthy();
    expect(styleEl!.textContent).toContain("@keyframes sel-gap-blink");
  });

  it("sets pointerEvents to none and zIndex to 1000", () => {
    const { container } = render(
      <SelectionOverlay target={null} getElement={() => null} />,
    );
    const div = container.querySelector("div");
    expect(div!.style.pointerEvents).toBe("none");
    expect(div!.style.zIndex).toBe("1000");
  });
});
