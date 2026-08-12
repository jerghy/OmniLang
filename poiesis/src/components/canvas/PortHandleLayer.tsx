// 端口手柄层（M3 T5）：醒态元素右缘端口 = ReactFlow 可拖手柄（源/目标双向），拖线即绑定。
// 坐标系（锁定）：本层挂 OmniViewRenderer 容器内（与 ElementPortLayer 同处）——
// 节点 position = collectPortPositions 输出（同容器相对内容坐标）；
// ReactFlow viewport 下行同步 useCanvasStore（设计层 zoom/pan 镜像，pane 事件穿透）。
import { useEffect, useLayoutEffect, useMemo, useState } from "react";
import {
  ReactFlow, ReactFlowProvider, useReactFlow, ConnectionMode,
  type Connection, type Node as RFNode, type Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../store";
import { useCanvasStore } from "./canvas-store";
import { collectPortPositions } from "./ElementPortLayer";
import { PortHandleNode, type PortNodeData } from "./PortHandleNode";
import styles from "./PortHandleLayer.module.css";

const RENDERER_SELECTOR = "[data-renderer='omniview']";
const nodeTypes = { port: PortHandleNode };

/** 元素标签 → 端口槽语义（与 ContentTab slotsFor 对齐） */
function slotKeyForTag(tag: string): string {
  if (["input", "textarea", "select"].includes(tag)) return "value";
  if (tag === "img") return "src";
  if (tag === "a") return "href";
  return "label";
}

function PortHandleLayerInner({ visible }: { visible: boolean }) {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const wires = usePoiesisStore((s) => s.wires);
  const { zoom: canvasZoom, panX, panY } = useCanvasStore();
  const { setViewport } = useReactFlow();
  const [ports, setPorts] = useState<Array<{ elId: string; x: number; y: number; slotKey: string }>>([]);

  // 测量（镜像 ElementPortLayer：immediate + rAF + ResizeObserver + 变更守卫）
  useLayoutEffect(() => {
    if (!(visible && awake)) {
      setPorts([]);
      return;
    }
    const root = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
    if (!root) return; // 根不存在直接退出——无兜底观察 body，避免死监听
    const measure = () => {
      const dots = collectPortPositions(root, canvasZoom);
      // 按元素 tag 补 slotKey；仅保留有槽语义的 view 元素
      const next = dots.flatMap((d) => {
        const el = root.querySelector<HTMLElement>(`[data-omni-el-id="${d.elId}"]`);
        if (!el) return [];
        const tag = el.tagName.toLowerCase();
        return [{ elId: d.elId, x: d.x, y: d.y, slotKey: slotKeyForTag(tag) }];
      });
      setPorts((prev) =>
        prev.length === next.length &&
        prev.every((p, i) => p.elId === next[i].elId && p.x === next[i].x && p.y === next[i].y && p.slotKey === next[i].slotKey)
          ? prev
          : next,
      );
    };
    measure();
    const raf = requestAnimationFrame(measure); // 等 WASM 渲染布局稳定
    const observer = new ResizeObserver(measure);
    observer.observe(root);
    window.addEventListener("resize", measure);
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [visible, awake, canvasZoom]);

  // 视口下行同步（设计层 viewport → ReactFlow；无回写——pane 事件穿透）
  useEffect(() => {
    setViewport({ x: panX, y: panY, zoom: canvasZoom });
  }, [panX, panY, canvasZoom, setViewport]);

  const nodes: RFNode<PortNodeData>[] = useMemo(
    () =>
      ports.map((p) => ({
        id: p.elId,
        type: "port",
        position: { x: p.x, y: p.y },
        draggable: false,
        selectable: false,
        focusable: false,
        data: { elId: p.elId, slotKey: p.slotKey, label: p.elId },
      })),
    [ports],
  );

  // 自动 wire → 边（端口 handle id 编码：`${slot}-src`/`${slot}-tgt`）
  const edges: Edge[] = useMemo(
    () =>
      wires
        .filter((w) => w.fromPort && w.toPort)
        .map((w) => ({
          id: w.id,
          source: w.fromId,
          sourceHandle: `${w.fromPort}-src`,
          target: w.toId,
          targetHandle: `${w.toPort}-tgt`,
          style: { stroke: "var(--ol-color-primary)", strokeWidth: 1.5 },
        })),
    [wires],
  );

  if (!(visible && awake)) return null;

  return (
    <div className={styles.layer} data-testid="port-handle-layer">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView={false}
        nodesDraggable={false}
        nodesConnectable
        nodesFocusable={false}
        elementsSelectable={false}
        connectionMode={ConnectionMode.Loose}
        panOnScroll={false}
        zoomOnScroll={false}
        zoomOnPinch={false}
        zoomOnDoubleClick={false}
        minZoom={0.1}
        maxZoom={5}
        onConnect={(conn: Connection) => {
          const sourceSlot = (conn.sourceHandle ?? "").replace(/-src$/, "");
          const targetSlot = (conn.targetHandle ?? "").replace(/-tgt$/, "");
          if (conn.source && conn.target && sourceSlot && targetSlot) {
            usePoiesisStore.getState().connectElementLink(conn.source, sourceSlot, conn.target, targetSlot);
          }
        }}
      />
    </div>
  );
}

export function PortHandleLayer({ visible }: { visible: boolean }) {
  return (
    <ReactFlowProvider>
      <PortHandleLayerInner visible={visible} />
    </ReactFlowProvider>
  );
}
