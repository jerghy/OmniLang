// poiesis/src/components/canvas/__tests__/LogicLayer.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { LogicLayer } from "../LogicLayer";

describe("LogicLayer", () => {
  it("可见时透出内容且可交互", () => {
    render(<LogicLayer visible><div data-testid="inner">fn</div></LogicLayer>);
    const layer = screen.getByTestId("logic-layer");
    expect(layer.style.opacity).toBe("1");
    expect(layer.style.pointerEvents).toBe("auto");
    expect(screen.getByTestId("inner")).toBeTruthy();
  });

  it("隐藏时不可交互且不可见（睡态干净画布）", () => {
    render(<LogicLayer visible={false}><div data-testid="inner">fn</div></LogicLayer>);
    const layer = screen.getByTestId("logic-layer");
    expect(layer.style.visibility).toBe("hidden");
    expect(layer.style.pointerEvents).toBe("none");
  });
});
