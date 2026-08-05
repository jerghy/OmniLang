// poiesis/src/components/canvas/code/__tests__/FlowEdge.test.ts
import { describe, it, expect } from "vitest";
import { computeFlowEdgePath, CORRIDOR } from "../FlowEdge";
import { Position } from "@xyflow/react";

describe("computeFlowEdgePath 正交路由", () => {
  it("分支 wire（右侧 → 顶部）：水平出走廊 → 走廊垂直 → 水平进目标", () => {
    // source 在 (100, 50) 右侧 handle，target 在 (300, 100) 顶部 handle
    const d = computeFlowEdgePath({ sourceX: 100, sourceY: 50, targetX: 300, targetY: 100, sourcePosition: Position.Right, targetPosition: Position.Top });
    expect(d).toBe(`M 100 50 H ${100 + CORRIDOR} V 100 H 300`);
  });
  it("flow-out → flow-in 同列：直接垂直直线（不走走廊绕行）", () => {
    const d = computeFlowEdgePath({ sourceX: 100, sourceY: 100, targetX: 100, targetY: 200, sourcePosition: Position.Bottom, targetPosition: Position.Top });
    expect(d).toBe("M 100 100 L 100 200");
  });
  it("近距（source-target 间距 < 2×CORRIDOR）：走廊取中点防越过目标", () => {
    // source 右缘 100、target 顶 130——走廊 min(124, 115) = 115（中点）
    const d = computeFlowEdgePath({ sourceX: 100, sourceY: 50, targetX: 130, targetY: 100, sourcePosition: Position.Right, targetPosition: Position.Top });
    expect(d).toBe("M 100 50 H 115 V 100 H 130");
  });
  it("垂直段在走廊（列间空隙）内——不穿过目标列上方节点（回归：while→body 穿 call 场景）", () => {
    // 布局实测场景：while 右缘 174 → body assign 顶部 349；call 占据 x 260-382
    // 走廊 x = 174 + 24 = 198 < 260（call 左缘）→ 垂直段不穿 call
    const d = computeFlowEdgePath({ sourceX: 174, sourceY: 630, targetX: 349, targetY: 760, sourcePosition: Position.Right, targetPosition: Position.Top });
    expect(d).toBe(`M 174 630 H 198 V 760 H 349`);
  });
});
