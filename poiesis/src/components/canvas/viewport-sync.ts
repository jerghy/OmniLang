// DOM 画布（CSS transform）↔ ReactFlow viewport 的坐标映射。
// 两引擎语义一致：屏幕坐标 = 画布坐标 × zoom + pan（pan 为屏幕像素）。
// DOM 侧 transform 为 `scale(zoom) translate(panX/zoom, panY/zoom)`（pan 已被 /zoom 补偿，
// 组合效果 = 先平移再缩放），ReactFlow 侧 x/y/zoom 同义——故直接映射，无换算。
// 若实机出现偏移，修这里两侧的换算（单测锁定契约），不要改消费方。
import type { Viewport } from "@xyflow/react";

export interface DomViewport {
  zoom: number;
  panX: number;
  panY: number;
}

export function domToFlowViewport(v: DomViewport): Viewport {
  return { x: v.panX, y: v.panY, zoom: v.zoom };
}

export function flowToDomViewport(v: Viewport): DomViewport {
  return { panX: v.x, panY: v.y, zoom: v.zoom };
}
