import { describe, it, expect, vi, afterEach } from "vitest";
import { collectPortPositions } from "../ElementPortLayer";

describe("collectPortPositions", () => {
  afterEach(() => { vi.restoreAllMocks(); });

  it("按 data-omni-el-id 收集元素右缘中点（容器相对坐标 + 滚动补偿）", () => {
    const container = document.createElement("div");
    const el = document.createElement("div");
    el.dataset.omniElId = "el-1";
    container.appendChild(el);

    vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
      left: 10, right: 110, top: 20, bottom: 60,
      width: 100, height: 40, x: 10, y: 20,
      toJSON: () => ({}),
    } as DOMRect);
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
      left: 0, right: 800, top: 0, bottom: 600,
      width: 800, height: 600, x: 0, y: 0,
      toJSON: () => ({}),
    } as DOMRect);

    expect(collectPortPositions(container)).toEqual([{ elId: "el-1", x: 110, y: 40 }]);
  });

  it("滚动补偿：scrollLeft≠0 且容器有 padding（实机验证修正：层原点=padding 边缘，无需减 padding）", () => {
    const container = document.createElement("div");
    const el = document.createElement("div");
    el.dataset.omniElId = "el-2";
    container.appendChild(el);
    vi.spyOn(container, "scrollLeft", "get").mockReturnValue(30);
    vi.spyOn(container, "scrollTop", "get").mockReturnValue(12);
    vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
      left: 48, right: 148, top: 40, bottom: 80,
      width: 100, height: 40, x: 48, y: 40, toJSON: () => ({}),
    } as DOMRect);
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
      left: 0, right: 800, top: 0, bottom: 600,
      width: 800, height: 600, x: 0, y: 0, toJSON: () => ({}),
    } as DOMRect);
    // 端口坐标 = rect - cRect + scroll：x = 148 - 0 + 30 = 178；y = 60 - 0 + 12 = 72
    expect(collectPortPositions(container)).toEqual([{ elId: "el-2", x: 178, y: 72 }]);
  });

  it("无 data-omni-el-id 的元素被忽略", () => {
    const container = document.createElement("div");
    const plain = document.createElement("div");
    container.appendChild(plain);
    vi.spyOn(plain, "getBoundingClientRect").mockReturnValue({
      left: 0, right: 50, top: 0, bottom: 20, width: 50, height: 20, x: 0, y: 0, toJSON: () => ({}),
    } as DOMRect);
    expect(collectPortPositions(container)).toEqual([]);
  });
});
