// poiesis/src/components/canvas/code/FlowEdge.tsx
// 正交 wire 路由（2026-08-05 美观优化轮）：替换 xyflow 默认 bezier——
// 分支 wire（source 右侧 → target 顶部）的 bezier 圆弧会扫过目标列上方节点
// 左下角（用户实测 + 画布坐标精确采样：while→body wire 穿过 call）；
// 正交路径把垂直段放在 source 右侧走廊（列间空隙内），水平段在 target 行——
// 不穿过任何节点。flow-out→flow-in 同列垂直保持直线。
import { BaseEdge, type EdgeProps, type Position } from "@xyflow/react";

/** 走廊偏移：source 右侧水平段距离（列间空隙 H_GAP=60，24px 落在空隙内不穿节点） */
export const CORRIDOR = 24;

export interface FlowEdgeInput {
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  sourcePosition: Position;
  targetPosition: Position;
}

/** 正交路径计算（纯函数，可单测） */
export function computeFlowEdgePath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }: FlowEdgeInput): string {
  // flow-out（底部）→ flow-in（顶部）同列：直接垂直直线
  if (sourcePosition === "bottom" && targetPosition === "top" && Math.abs(sourceX - targetX) < CORRIDOR) {
    return `M ${sourceX} ${sourceY} L ${targetX} ${targetY}`;
  }
  // 分支/续接：水平出走廊 → 走廊垂直 → 水平进目标顶部。
  // 走廊 x = source 右侧 CORRIDOR 处，但不超过 source-target 中点（近距时取中点，防走廊越过目标）
  const corridorX = Math.min(sourceX + CORRIDOR, (sourceX + targetX) / 2);
  return `M ${sourceX} ${sourceY} H ${corridorX} V ${targetY} H ${targetX}`;
}

export function FlowEdge({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, style, markerEnd }: EdgeProps) {
  const d = computeFlowEdgePath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition });
  return <BaseEdge id={id} path={d} style={style} markerEnd={markerEnd} />;
}
