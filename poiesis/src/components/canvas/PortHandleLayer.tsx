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

/**
 * 从连接解析源/目标槽（纯函数）：优先测量节点（确定性）。
 * C1 修复：源/目标 Handle 同坐标叠加、source 后渲染压上层，端口中心 elementFromPoint
 * 恒命中 source handle → targetHandle 字符串携带对侧（-src）后缀，剥除后成垃圾槽；
 * 节点查询不受 handle-id 影响，是唯一可靠来源。handle-id 后缀剥除仅作节点未命中兜底。
 */
/** M3 T6 亮线判定（纯函数）：端点元素被选中（任一端）或悬停（任一端）→ 相关边亮起 */
export function edgeIsActive(
  wire: { fromId: string; toId: string },
  selectedIds: string[],
  hoveredId: string | null,
): boolean {
  return (
    selectedIds.includes(wire.fromId) ||
    selectedIds.includes(wire.toId) ||
    hoveredId === wire.fromId ||
    hoveredId === wire.toId
  );
}

export function resolveSlots(
  conn: { source: string | null; target: string | null; sourceHandle?: string | null; targetHandle?: string | null },
  nodes: Array<{ id: string; data: { slotKey: string } }>,
): { sourceSlot: string | null; targetSlot: string | null } {
  const strip = (h?: string | null) => (h ?? "").replace(/-src$/, "").replace(/-tgt$/, "");
  const sourceSlot = nodes.find((n) => n.id === conn.source)?.data.slotKey ?? (strip(conn.sourceHandle) || null);
  const targetSlot = nodes.find((n) => n.id === conn.target)?.data.slotKey ?? (strip(conn.targetHandle) || null);
  return { sourceSlot, targetSlot };
}

function PortHandleLayerInner({ visible }: { visible: boolean }) {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const wires = usePoiesisStore((s) => s.wires);
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
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

  // M3 T6 亮线·元素悬停（复审替换）：文档级事件委托——层 pointer-events:none 穿透，
  // mouseover 落在渲染元素上。原生 .react-flow__edge:hover 永不触发（边 inactive，
  // elementsSelectable=false → pointer-events:none），故悬停语义挂在「元素」而非「边」。
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  useEffect(() => {
    if (!(visible && awake)) return;
    const readId = (node: EventTarget | null): string | null =>
      node instanceof Element ? node.closest("[data-omni-el-id]")?.getAttribute("data-omni-el-id") ?? null : null;
    const onMouseOver = (e: MouseEvent) => setHoveredId(readId(e.target));
    const onMouseOut = (e: MouseEvent) => setHoveredId(readId(e.relatedTarget)); // 移入新元素即接管；离开窗口置空
    document.addEventListener("mouseover", onMouseOver);
    document.addEventListener("mouseout", onMouseOut);
    return () => {
      document.removeEventListener("mouseover", onMouseOver);
      document.removeEventListener("mouseout", onMouseOut);
    };
  }, [visible, awake]);

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
  // M3 T6 亮线：端点元素被选中（任一端）或悬停（任一端）→ 边加 port-edge-active 类 + 主色加深加粗
  const edges: Edge[] = useMemo(
    () =>
      wires
        .filter((w) => w.fromPort && w.toPort)
        .map((w) => {
          const isRelated = edgeIsActive(w, selectedIds, hoveredId);
          return {
            id: w.id,
            source: w.fromId,
            sourceHandle: `${w.fromPort}-src`,
            target: w.toId,
            targetHandle: `${w.toPort}-tgt`,
            className: isRelated ? "port-edge-active" : undefined,
            style: {
              stroke: isRelated ? "var(--ol-color-primary-active)" : "var(--ol-color-primary)",
              strokeWidth: isRelated ? 2.5 : 1.5,
            },
          };
        }),
    [wires, selectedIds, hoveredId],
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
          // 槽解析以测量节点为准（C1 修复：handle-id 后缀不可靠——端口中心恒命中
          // source handle，targetHandle 携带 -src 后缀；节点查询不受 handle 叠加影响）。
          const { sourceSlot, targetSlot } = resolveSlots(conn, nodes);
          if (!conn.source || !conn.target || !sourceSlot || !targetSlot) return;
          if (conn.source === conn.target) return; // 自环拒绝（Loose 模式可回环自身端口）
          usePoiesisStore.getState().connectElementLink(conn.source, sourceSlot, conn.target, targetSlot);
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
