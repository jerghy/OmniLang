import { describe, it, expect } from "vitest";
import { computePanelPlacement } from "../panel-position";

const viewport = { width: 1280, height: 800 };

describe("computePanelPlacement", () => {
  it("右侧放得下 → 右落位，y 贴合元素顶", () => {
    const p = computePanelPlacement({ x: 100, y: 80, width: 200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.side).toBe("right");
    expect(p.x).toBe(100 + 200 + 12);
    expect(p.y).toBe(80);
  });

  it("右侧放不下 → 左落位", () => {
    const p = computePanelPlacement({ x: 1100, y: 80, width: 200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.side).toBe("left");
    expect(p.x).toBe(1100 - 280 - 12);
  });

  it("两侧都放不下 → 元素下方贴合左缘", () => {
    // 计划原用例 x=600/w=1200 时左侧实际放得下（600-280-12=308≥0）——数据 bug；
    // 改用 x=0（元素占满视口左缘起）使两侧都放不下，y 同时体现底部钳制契约（与第 4 条一致）。
    const p = computePanelPlacement({ x: 0, y: 600, width: 1200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.side).toBe("bottom");
    expect(p.x).toBe(0);
    expect(p.y).toBe(800 - 360);
  });

  it("y 溢出视口 → 钳制", () => {
    const p = computePanelPlacement({ x: 100, y: 700, width: 200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.y).toBe(800 - 360);
  });
});
