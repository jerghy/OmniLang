import { describe, it, expect } from "vitest";
import { isPaneRelease } from "../FlowCanvas";

describe("isPaneRelease", () => {
  function el(className: string): HTMLElement {
    const div = document.createElement("div");
    div.className = className;
    return div;
  }
  it("释放到空白 pane 返回 true", () => {
    expect(isPaneRelease({ target: el("react-flow__pane") })).toBe(true);
  });
  it("释放到节点上返回 false", () => {
    expect(isPaneRelease({ target: el("react-flow__node") })).toBe(false);
  });
  it("释放到 handle 上返回 false", () => {
    expect(isPaneRelease({ target: el("react-flow__handle") })).toBe(false);
  });
  it("target 非 HTMLElement 返回 false", () => {
    expect(isPaneRelease({ target: null })).toBe(false);
  });
});
