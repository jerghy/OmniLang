// poiesis/src/components/canvas/__tests__/viewport-sync.test.ts
import { describe, it, expect } from "vitest";
import { domToFlowViewport, flowToDomViewport } from "../viewport-sync";

describe("viewport-sync", () => {
  it("DOM 视口 → ReactFlow 视口：语义一致直接映射（屏幕 = 画布坐标 × zoom + pan）", () => {
    expect(domToFlowViewport({ zoom: 1.5, panX: 120, panY: -40 })).toEqual({ x: 120, y: -40, zoom: 1.5 });
  });

  it("ReactFlow 视口 → DOM 视口：逆映射", () => {
    expect(flowToDomViewport({ x: 120, y: -40, zoom: 1.5 })).toEqual({ panX: 120, panY: -40, zoom: 1.5 });
  });

  it("往返一致", () => {
    const dom = { zoom: 0.8, panX: 33, panY: 77 };
    expect(flowToDomViewport(domToFlowViewport(dom))).toEqual(dom);
  });
});
