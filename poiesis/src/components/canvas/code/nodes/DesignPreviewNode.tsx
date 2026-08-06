import { useCallback, useMemo, useRef } from "react";
import type { NodeProps } from "@xyflow/react";
import { useReactFlow } from "@xyflow/react";
import { usePoiesisStore } from "../../../../store";
import { useCanvasStore } from "../../../canvas/canvas-store";
import { OmniViewElementTree } from "../../../design/OmniViewElementTree";
import type { OmniViewOutput } from "../../../design/useWasmCompile";
import styles from "./DesignPreviewNode.module.css";

/** Max bounding box for the preview node inside the flow canvas. */
const MAX_W = 240;
const MAX_H = 360;

interface DesignPreviewNodeData {
  output: OmniViewOutput | null;
  /** 节点当前 flow 坐标（previewPos）——拖拽起点的位移基准，松手提交时叠加增量 */
  position: { x: number; y: number };
  /** 松手提交最终位置（FlowCanvas 侧 setPreviewPos，整个拖拽只此一次 state 写入） */
  onPreviewMove: (pos: { x: number; y: number }) => void;
}

export function DesignPreviewNode({
  data,
}: NodeProps & { data: DesignPreviewNodeData }) {
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const devicePreset = useCanvasStore((s) => s.devicePreset);
  const { screenToFlowPosition } = useReactFlow();
  const rootRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ startFlow: { x: number; y: number } } | null>(null);

  // 拖拽自管：ref + transform 直改，不经 React 状态、不经 ReactFlow 拖拽体系。
  // 背景（2026-08-06 用户实测）：曾用 ReactFlow 原生拖拽（draggable:true）+
  // onNodesChange 回写本地 state——每帧 setPreviewPos 触发 ReactFlow 调和
  // （StoreUpdater 用 nodes prop 复位内部位置）与实时 transform 直改互相打架，
  // 真机拖拽肉眼可见闪烁。解耦后 mousemove 期间零 state 更新、零重渲染：
  // 增量 = 当前 flow 坐标 - 按下时 flow 坐标（screenToFlowPosition 已含 zoom），
  // 直接写内层 div 的 transform；松手清空 transform 并把最终位置提交一次。
  const onMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.button !== 0) return;
      e.preventDefault();
      dragRef.current = {
        startFlow: screenToFlowPosition({ x: e.clientX, y: e.clientY }),
      };
      const handleMouseMove = (ev: MouseEvent) => {
        const drag = dragRef.current;
        if (!drag || !rootRef.current) return;
        const cur = screenToFlowPosition({ x: ev.clientX, y: ev.clientY });
        const dx = cur.x - drag.startFlow.x;
        const dy = cur.y - drag.startFlow.y;
        rootRef.current.style.transform = `translate(${dx}px, ${dy}px)`;
      };
      const handleMouseUp = (ev: MouseEvent) => {
        const drag = dragRef.current;
        dragRef.current = null;
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
        if (!drag || !rootRef.current) return;
        const cur = screenToFlowPosition({ x: ev.clientX, y: ev.clientY });
        const dx = cur.x - drag.startFlow.x;
        const dy = cur.y - drag.startFlow.y;
        rootRef.current.style.transform = "";
        data.onPreviewMove({ x: data.position.x + dx, y: data.position.y + dy });
      };
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    },
    [screenToFlowPosition, data],
  );

  // Derive preview dimensions from device preset, preserving aspect ratio
  const { previewW, previewH, scale } = useMemo(() => {
    const dw = devicePreset.width || 393;
    const dh = devicePreset.height || 852;
    const s = Math.min(MAX_W / dw, MAX_H / dh);
    return { previewW: Math.round(dw * s), previewH: Math.round(dh * s), scale: s };
  }, [devicePreset.width, devicePreset.height]);

  return (
    <div
      ref={rootRef}
      onMouseDown={onMouseDown}
      onDoubleClick={() => toggleCodeFlow(null)}
      className={styles.node}
      style={{ width: previewW, height: previewH }}
    >
      {/* Title bar */}
      <div className={styles.titleBar}>
        预览 (双击退出)
      </div>

      {/* Preview content — scaled down, pointer-events disabled so ReactFlow handles drag */}
      <div className={styles.previewContent}>
        <div
          className={styles.scaleWrapper}
          style={{
            transform: `scale(${scale})`,
            width: devicePreset.width || 393,
            height: devicePreset.height || 852,
          }}
        >
          {data.output ? (
            <OmniViewElementTree
              output={data.output}
              selectedNid={null}
              onSelectNid={() => {}}
              onDoubleClickNid={() => {}}
            />
          ) : (
            <div className={styles.emptyState}>
              编译中...
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
