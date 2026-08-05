// poiesis/src/components/canvas/code/FlowEdge.tsx
// 正交 wire 路由（2026-08-05 美观优化轮）：替换 xyflow 默认 bezier——
// 分支 wire（source 右侧 → target 顶部）的 bezier 圆弧会扫过目标列上方节点
// 左下角（用户实测 + 画布坐标精确采样：while→body wire 穿过 call）；
// 正交路径把垂直段放在 source 右侧走廊（列间空隙内），水平段在 target 行——不穿过任何节点。
//
// 位置来源：运行时 DOM 测量（useStore transform + handle 元素 rect → 画布坐标）——
// 不依赖 ReactFlow 的 handleBounds 缓存（mount 时测量一次，CSS/位置变化不触发重测，
// 实测 handle 归位后 edge 的 sourceX 仍用漂移值 → 走廊越过目标列左缘 → 仍穿节点）。
// ResizeObserver（节点尺寸）+ MutationObserver（节点 transform）跟随一切变化。
import { useEffect, useState } from "react";
import { BaseEdge, Position, useStore, type EdgeProps } from "@xyflow/react";

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

const HANDLE_CENTER = 4; // handle 直径 8px，中心偏移

/** DOM 元素 → 画布坐标（ReactFlow transform [dx, dy, zoom]） */
function domToCanvas(el: Element, vp: DOMRect, transform: [number, number, number]): { x: number; y: number } {
  const r = el.getBoundingClientRect();
  return { x: (r.x - vp.x - transform[0]) / transform[2], y: (r.y - vp.y - transform[1]) / transform[2] };
}

/** EdgeProps 类型缺 sourceHandle/targetHandle（安装版 v12 类型不含），扩展补上 */
interface FlowEdgeProps extends EdgeProps {
  sourceHandle?: string | null;
  targetHandle?: string | null;
}

export function FlowEdge({ id, source, target, sourceHandle, targetHandle, style, markerEnd }: FlowEdgeProps) {
  const transform = useStore((s) => s.transform);
  const [pos, setPos] = useState<FlowEdgeInput | null>(null);

  useEffect(() => {
    const measure = () => {
      const srcEl = document.querySelector(`[data-nodeid="${source}"][data-handleid="${sourceHandle}"]`);
      const tgtEl = document.querySelector(`[data-nodeid="${target}"][data-handleid="${targetHandle}"]`);
      const vp = document.querySelector(".react-flow__viewport")?.getBoundingClientRect();
      if (!srcEl || !tgtEl || !vp) return;
      const s = domToCanvas(srcEl, vp, transform);
      const t = domToCanvas(tgtEl, vp, transform);
      setPos({
        sourceX: s.x + HANDLE_CENTER,
        sourceY: s.y + HANDLE_CENTER,
        targetX: t.x + HANDLE_CENTER,
        targetY: t.y + HANDLE_CENTER,
        sourcePosition: (srcEl.getAttribute("data-handlepos") as Position) ?? Position.Right,
        targetPosition: (tgtEl.getAttribute("data-handlepos") as Position) ?? Position.Top,
      });
    };
    measure();
    // 跟随节点变化：尺寸（ResizeObserver）+ 位置 transform（MutationObserver——CSS 变化/布局按钮后重测）
    const srcNode = document.querySelector(`[data-id="${source}"]`);
    const tgtNode = document.querySelector(`[data-id="${target}"]`);
    const ro = new ResizeObserver(measure);
    const mo = new MutationObserver(measure);
    if (srcNode) { ro.observe(srcNode); mo.observe(srcNode, { attributes: true, attributeFilter: ["style"] }); }
    if (tgtNode) { ro.observe(tgtNode); mo.observe(tgtNode, { attributes: true, attributeFilter: ["style"] }); }
    return () => { ro.disconnect(); mo.disconnect(); };
  }, [source, target, sourceHandle, targetHandle, transform]);

  if (!pos) return null; // 测量前不渲染（防闪烁）
  const d = computeFlowEdgePath(pos);
  return <BaseEdge id={id} path={d} style={style} markerEnd={markerEnd} />;
}
