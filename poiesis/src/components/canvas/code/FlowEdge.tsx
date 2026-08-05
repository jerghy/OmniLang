// poiesis/src/components/canvas/code/FlowEdge.tsx
// 正交 wire 路由（2026-08-05 美观优化轮）：替换 xyflow 默认 bezier——
// 分支 wire（source 右侧 → target 顶部）的 bezier 圆弧会扫过目标列上方节点
// 左下角（用户实测 + 画布坐标精确采样：while→body wire 穿过 call）；
// 正交路径把垂直段放在 source 右侧走廊（列间空隙内），水平段在 target 行——不穿过任何节点。
// flow-out→flow-in 同列保持垂直直线。
//
// 位置来源：节点 rect DOM 测量（useStore transform 换算画布坐标）+ data.fromPort 判定端口——
// 绕开 ReactFlow handleBounds（mount 时测量，CSS 时序下偏移失真：实测 187.67 vs DOM 100）
// 且 v12 EdgeProps 不传 sourceHandle 给自定义 edge（DOM 版先失败于此）。
// ResizeObserver + MutationObserver 跟随节点尺寸/位置变化。
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

/** EdgeProps 类型缺 sourceHandle/targetHandle（安装版 v12 类型不含），扩展补上 */
interface FlowEdgeProps extends EdgeProps {
  sourceHandle?: string | null;
  targetHandle?: string | null;
  /** FlowCanvas 注入的端口信息（v12 不传 sourceHandle 给自定义 edge，走 data） */
  data?: { fromPort?: string; toPort?: string };
}

/** DOM 元素 → 画布坐标（ReactFlow transform [dx, dy, zoom]） */
function domToCanvas(el: Element, vp: DOMRect, transform: [number, number, number]): { x: number; y: number } {
  const r = el.getBoundingClientRect();
  return { x: (r.x - vp.x - transform[0]) / transform[2], y: (r.y - vp.y - transform[1]) / transform[2] };
}

export function FlowEdge({ id, source, target, data, style, markerEnd }: FlowEdgeProps) {
  const transform = useStore((s) => s.transform);
  const [pos, setPos] = useState<FlowEdgeInput | null>(null);

  useEffect(() => {
    let rafId = 0;
    let ro: ResizeObserver | null = null;
    let mo: MutationObserver | null = null;
    const fromPort = data?.fromPort ?? "flow-out";
    const toPort = data?.toPort ?? "flow-in";
    const measure = () => {
      const srcEl = document.querySelector(`[data-id="${source}"]`);
      const tgtEl = document.querySelector(`[data-id="${target}"]`);
      const vp = document.querySelector(".react-flow__viewport")?.getBoundingClientRect();
      if (!srcEl || !tgtEl || !vp) {
        rafId = requestAnimationFrame(measure); // handle 未就绪（渲染顺序）——重试
        return;
      }
      const sr = srcEl.getBoundingClientRect();
      const tr = tgtEl.getBoundingClientRect();
      const s = domToCanvas(srcEl, vp, transform);
      const t = domToCanvas(tgtEl, vp, transform);
      // handle 位置：source 底部中点（flow-out）/ 右缘中部（分支）；target 顶部中点（flow-in）
      const sX = fromPort === "flow-out" ? s.x + sr.width / 2 : s.x + sr.width;
      const sY = fromPort === "flow-out" ? s.y + sr.height : s.y + sr.height / 2;
      setPos({
        sourceX: sX,
        sourceY: sY,
        targetX: t.x + tr.width / 2,
        targetY: t.y,
        sourcePosition: fromPort === "flow-out" ? Position.Bottom : Position.Right,
        targetPosition: Position.Top,
      });
    };
    measure();
    // 跟随节点变化：尺寸（ResizeObserver）+ 位置（MutationObserver——拖拽/布局按钮后重测）
    const srcNode = document.querySelector(`[data-id="${source}"]`);
    const tgtNode = document.querySelector(`[data-id="${target}"]`);
    ro = new ResizeObserver(measure);
    mo = new MutationObserver(measure);
    if (srcNode) { ro.observe(srcNode); mo.observe(srcNode, { attributes: true, attributeFilter: ["style"] }); }
    if (tgtNode) { ro.observe(tgtNode); mo.observe(tgtNode, { attributes: true, attributeFilter: ["style"] }); }
    return () => { cancelAnimationFrame(rafId); ro?.disconnect(); mo?.disconnect(); };
  }, [source, target, data?.fromPort, data?.toPort, transform]);

  if (!pos) return null; // 测量前不渲染（防闪烁）
  const d = computeFlowEdgePath(pos);
  return <BaseEdge id={id} path={d} style={style} markerEnd={markerEnd} />;
}
