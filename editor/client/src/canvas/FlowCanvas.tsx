/** @xyflow/react wire canvas — data-flow code blocks with typed ports + wire labels. */

import { useCallback, useEffect, useRef, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Connection,
  type Node,
  type Edge,
  BackgroundVariant,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useEditorStore, type ElementData, type WireData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { AddWire } from "../commands/AddWire";
import { RemoveWire } from "../commands/RemoveWire";
import { AddElement } from "../commands/AddElement";
import { RemoveElement } from "../commands/RemoveElement";
import type { CommandContext } from "../commands/types";
import { getNodeConfig } from "./nodeRegistry";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { ViewEventNode } from "./nodes/ViewEventNode";
import { FlowEdge } from "./edges/FlowEdge";
import { DataEdge } from "./edges/DataEdge";
import { isTypeCompatible, typeMismatchMessage } from "./socketTypes";
import type { BaseNodeData } from "./nodes/BaseNode";

// ── Custom node/edge types (registered once, outside component) ──

const nodeTypes = {
  codeblock: CodeBlockNode,
  viewEvent: ViewEventNode,
};

const edgeTypes = {
  flow: FlowEdge,
  data: DataEdge,
};

// ── Helpers ──

/** Determine edge type from wire data: flow if either port is flow-typed, else data. */
function getEdgeType(
  fromEl: ElementData | undefined,
  toEl: ElementData | undefined,
  wire: WireData,
): "flow" | "data" {
  const fromPorts = fromEl?.ports || [];
  const toPorts = toEl?.ports || [];
  const fromPort = fromPorts.find((p) => p.name === wire.fromPort);
  const toPort = toPorts.find((p) => p.name === wire.toPort);
  if (fromPort?.portType === "flow" || toPort?.portType === "flow") return "flow";
  return "data";
}

/** Get the data type for coloring DataEdges. */
function getEdgeDataType(fromEl: ElementData | undefined, wire: WireData): string {
  const fromPort = fromEl?.ports?.find((p) => p.name === wire.fromPort);
  return fromPort?.type || "any";
}

/** Parse a function parameter string like "a:i32, b:String" into name/type pairs. */
function parseFnString(paramsStr: string): Array<{ name: string; type: string }> {
  if (!paramsStr.trim()) return [];
  const result: Array<{ name: string; type: string }> = [];
  for (const part of paramsStr.split(",")) {
    const [pName, pType] = part.split(":").map((s) => s.trim());
    if (pName && pType) result.push({ name: pName, type: pType });
  }
  return result;
}

// ── Props (same interface as old FlowCanvas for backward compat) ──

interface FlowCanvasProps {
  showViews?: boolean;
  highlightedWireId?: string | null;
  onNodeDoubleClick?: (nodeId: string, nodeData: ElementData) => void;
  ctx: CommandContext;
  /** Increment to trigger auto-layout from outside (Feature 2). */
  layoutTrigger?: number;
}

// ── Default source code for new code blocks ──

const DEFAULT_SOURCE = `// 💻 逻辑代码块
action handleEvent(event) {
    // 在此编写事件处理逻辑

}`;

// ── Component ──

export function FlowCanvas({
  showViews = true,
  highlightedWireId = null,
  onNodeDoubleClick,
  ctx,
  layoutTrigger = 0,
}: FlowCanvasProps) {
  const elements = useEditorStore((s) => s.elements);
  const wires = useEditorStore((s) => s.wires);
  const selectedElementId = useEditorStore((s) => s.selectedElementId);
  const containerRef = useRef<HTMLDivElement>(null);

  // Stable ref for onNodeDoubleClick to avoid useEffect re-run on every render
  const onNodeDoubleClickRef = useRef(onNodeDoubleClick);
  onNodeDoubleClickRef.current = onNodeDoubleClick;

  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  // ── Convert elements -> React Flow nodes ──

  useEffect(() => {
    const showAll = showViews;
    const wiredIds = new Set<string>();
    if (!showAll) {
      for (const w of wires) {
        wiredIds.add(w.fromId);
        wiredIds.add(w.toId);
      }
    }

    const newNodes: Node[] = [];

    for (const el of elements) {
      const isCodeBlock = el.nodeType === "codeblock" && el.tag !== "fn";
      const isVisible = isCodeBlock || showAll || wiredIds.has(el.id);
      if (!isVisible) continue;

      const config = getNodeConfig(el.tag);
      const ports = el.ports?.length ? el.ports : config.defaultPorts;
      const name = String(el.props.name || el.props.label || el.tag);
      const pos = el.position || {
        x: el.nodeType === "codeblock" ? 50 : 400,
        y: 50 + newNodes.length * 120,
      };

      const nodeData: BaseNodeData = {
        elementId: el.id,
        tag: el.tag,
        label: name,
        typeAnnotation: el.typeAnnotation || (el.props.typeAnnotation as string) || undefined,
        outputValue: el.outputValue,
        ports,
        isFolded: el.isFolded,
        onDoubleClick: (elementId) => {
              const cb = onNodeDoubleClickRef.current;
              if (!cb) return;
              const found = useEditorStore.getState().elements.find((e) => e.id === elementId);
              if (found) cb(elementId, found);
            },
      };

      newNodes.push({
        id: el.id,
        type: el.nodeType === "codeblock" ? ("codeblock" as const) : ("viewEvent" as const),
        position: pos,
        data: nodeData as unknown as Record<string, unknown>,
        selected: el.id === selectedElementId,
      });
    }

    setNodes(newNodes);
  }, [elements, wires, showViews, selectedElementId]);

  // ── Convert wires -> React Flow edges ──

  useEffect(() => {
    const newEdges: Edge[] = [];

    for (const w of wires) {
      const fromEl = elements.find((e) => e.id === w.fromId);
      const toEl = elements.find((e) => e.id === w.toId);
      if (!fromEl || !toEl) continue;

      const edgeType = getEdgeType(fromEl, toEl, w);
      const dataType = getEdgeDataType(fromEl, w);

      newEdges.push({
        id: w.id,
        source: w.fromId,
        target: w.toId,
        sourceHandle: w.fromPort,
        targetHandle: w.toPort,
        type: edgeType,
        animated: edgeType === "flow",
        data: { type: dataType },
        selected: w.id === highlightedWireId,
      });
    }

    setEdges(newEdges);
  }, [wires, elements, highlightedWireId]);

  // ── Connection handler: validate types, prevent duplicates, add wire via undo store ──

  const handleConnect = useCallback(
    (connection: Connection) => {
      const fromEl = elements.find((e) => e.id === connection.source);
      const toEl = elements.find((e) => e.id === connection.target);
      if (!fromEl || !toEl) return;
      // Require explicit handles — reject connections without them
      if (!connection.sourceHandle || !connection.targetHandle) return;

      const fromPort = connection.sourceHandle;
      const toPort = connection.targetHandle;

      // Type validation
      const fromPorts = fromEl.ports || getNodeConfig(fromEl.tag).defaultPorts;
      const toPorts = toEl.ports || getNodeConfig(toEl.tag).defaultPorts;
      const sourcePort = fromPorts.find((p) => p.name === fromPort && p.direction === "out");
      const targetPort = toPorts.find((p) => p.name === toPort && p.direction === "in");

      if (sourcePort && targetPort) {
        if (!isTypeCompatible(sourcePort.type, targetPort.type)) {
          alert(typeMismatchMessage(sourcePort.type, targetPort.type));
          return;
        }
        // Port type validation: flow↔data mismatch
        const srcPT = sourcePort.portType || "data";
        const tgtPT = targetPort.portType || "data";
        if (srcPT !== tgtPT) {
          alert(
            `端口类型不兼容：输出端口 "${fromPort}" 是 ${srcPT === "flow" ? "执行流" : "数据"} 类型，` +
            `输入端口 "${toPort}" 是 ${tgtPT === "flow" ? "执行流" : "数据"} 类型。` +
            `\n\n执行流端口（菱形）只能连接执行流端口，数据端口（圆形）只能连接数据端口。`,
          );
          return;
        }
      }

      // Prevent duplicate wires
      const exists = wires.some(
        (w) =>
          w.fromId === fromEl.id &&
          w.fromPort === fromPort &&
          w.toId === toEl.id &&
          w.toPort === toPort,
      );
      if (exists) return;

      const wireData: WireData = {
        id: `w-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        fromId: fromEl.id,
        fromPort,
        toId: toEl.id,
        toPort,
      };

      useUndoStore.getState().execute(new AddWire(wireData), ctx);
    },
    [elements, wires, ctx],
  );

  // ── Edge delete handler — remove wires from store ──

  const handleEdgesDelete = useCallback(
    (edgesToDelete: Edge[]) => {
      for (const edge of edgesToDelete) {
        const wire = useEditorStore.getState().wires.find((w) => w.id === edge.id);
        if (wire) {
          useUndoStore.getState().execute(new RemoveWire(wire.id), ctx);
        }
      }
    },
    [ctx],
  );

  // ── Node delete handler — remove elements + connected wires from store ──

  const handleNodesDelete = useCallback(
    (nodesToDelete: Node[]) => {
      for (const node of nodesToDelete) {
        const currentElements = useEditorStore.getState().elements;
        const el = currentElements.find((e) => e.id === node.id);
        if (!el) continue;

        // Remove connected wires first
        const currentWires = useEditorStore.getState().wires;
        const connectedWires = currentWires.filter(
          (w) => w.fromId === el.id || w.toId === el.id,
        );
        const exec = useUndoStore.getState().execute;
        for (const w of connectedWires) {
          exec(new RemoveWire(w.id), ctx);
        }

        // Remove the element itself
        exec(new RemoveElement(el.id), ctx);
      }
    },
    [ctx],
  );

  // ── Node click -> select in store ──

  const handleNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    useEditorStore.getState().selectElement(node.id);
  }, []);

  // ── Node drag stop -> persist position to store ──

  const handleNodeDragStop = useCallback((_event: React.MouseEvent, node: Node) => {
    useEditorStore.getState().setElementPosition(node.id, {
      x: node.position.x,
      y: node.position.y,
    });
  }, []);

  // ── Pane click -> deselect ──

  const handlePaneClick = useCallback(() => {
    useEditorStore.getState().selectElement(null);
  }, []);

  // ── Node double-click -> open code editor (delegates to onNodeDoubleClick) ──

  const handleNodeDoubleClick = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      const cb = onNodeDoubleClickRef.current;
      if (!cb) return;
      const el = elements.find((e) => e.id === node.id);
      if (el && el.nodeType === "codeblock") {
        cb(el.id, el);
      }
    },
    [elements],
  );

  // ── Context menu (right-click -> add block) ──

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    setContextMenu({ x: e.clientX - rect.left, y: e.clientY - rect.top });
  }, []);

  // ── Drag-and-drop: position existing state nodes / create fn-call nodes ──

  const handleDragOver = useCallback((e: React.DragEvent) => {
    if (
      e.dataTransfer.types.includes("application/omni-state") ||
      e.dataTransfer.types.includes("application/omni-fn")
    ) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    }
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    const stateRaw = e.dataTransfer.getData("application/omni-state");
    const fnRaw = e.dataTransfer.getData("application/omni-fn");
    if (!stateRaw && !fnRaw) return;
    e.preventDefault();
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (stateRaw) {
      try {
        const dropped = JSON.parse(stateRaw) as { id: string };
        useEditorStore.getState().setElementPosition(dropped.id, { x, y });
      } catch { /* ignore invalid payload */ }
    }

    if (fnRaw) {
      try {
        const dropped = JSON.parse(fnRaw) as { fnId: string; fnName: string; params: string; returnType: string };
        const pports = parseFnString(dropped.params);
        const id = `cb-fncall-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
        const ports: Array<{ name: string; direction: "in" | "out"; type: string; label?: string; portType?: "flow" | "data" }> = [];
        for (const p of pports) {
          ports.push({ name: p.name, direction: "in", type: p.type, label: `${p.name}:${p.type}`, portType: "data" });
        }
        ports.push({ name: "result", direction: "out", type: dropped.returnType, label: `返回: ${dropped.returnType}`, portType: "data" });

        const el: ElementData = {
          id, nodeType: "codeblock", tag: "fn-call",
          props: { name: dropped.fnName, fnId: dropped.fnId },
          children: [], parentId: null,
          ports,
          isFolded: false,
          source: `// 📞 调用节点：调用函数库中的 ${dropped.fnName}(${dropped.params}) -> ${dropped.returnType}`,
        };
        useUndoStore.getState().execute(new AddElement(el), ctx);
      } catch { /* ignore invalid payload */ }
    }
  }, [ctx]);

  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  // ── Simple block menu items (Round 2 NodePalette will replace this) ──

  const BLOCK_MENU_ITEMS = [
    { tag: "action", label: "代码块" },
    { tag: "signal", label: "Signal" },
    { tag: "computed", label: "Computed" },
  ];

  const handleAddBlock = (tag: string) => {
    const config = getNodeConfig(tag);
    const id = `cb-${tag}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const ports = config.defaultPorts.map((p) => ({ ...p, direction: p.direction as "in" | "out" }));

    const el: ElementData = {
      id,
      nodeType: "codeblock",
      tag,
      props: { name: config.label },
      children: [],
      parentId: null,
      ports,
      isFolded: false,
      source: DEFAULT_SOURCE,
      nodeCategory: config.category,
    };
    useUndoStore.getState().execute(new AddElement(el), ctx);
    setContextMenu(null);
  };

  return (
    <div
      ref={containerRef}
      style={{ width: "100%", height: "100%", minHeight: 400, position: "relative" }}
      onContextMenu={handleContextMenu}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      tabIndex={0}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={handleConnect}
        onNodeClick={handleNodeClick}
        onNodeDoubleClick={handleNodeDoubleClick}
        onNodeDragStop={handleNodeDragStop}
        onPaneClick={handlePaneClick}
        onEdgesDelete={handleEdgesDelete}
        onNodesDelete={handleNodesDelete}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        deleteKeyCode={["Delete", "Backspace"]}
        multiSelectionKeyCode="Shift"
        snapToGrid
        snapGrid={[16, 16]}
        defaultEdgeOptions={{
          type: "data",
        }}
      >
        <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#E5E7EB" />
        <Controls position="bottom-right" />
        <MiniMap
          position="bottom-left"
          nodeStrokeColor="#9CA3AF"
          nodeColor={(n) => {
            const config = getNodeConfig((n.data as Record<string, unknown>)?.tag as string || "");
            return config.color.bg;
          }}
        />
      </ReactFlow>

      {/* Context menu — simple block adder (Round 2 will add search) */}
      {contextMenu && (
        <div
          style={{
            position: "absolute",
            left: contextMenu.x,
            top: contextMenu.y,
            zIndex: 1000,
            background: "#fff",
            borderRadius: 8,
            boxShadow: "0 4px 16px rgba(0,0,0,0.15)",
            border: "1px solid #E5E7EB",
            minWidth: 200,
            padding: "6px 0",
            fontSize: 12,
          }}
        >
          <div
            style={{
              padding: "4px 12px",
              color: "#9CA3AF",
              fontSize: 10,
              textTransform: "uppercase",
              fontWeight: 600,
            }}
          >
            添加节点
          </div>
          {BLOCK_MENU_ITEMS.map((item) => (
            <div
              key={item.tag}
              onClick={() => handleAddBlock(item.tag)}
              style={{
                padding: "7px 12px",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: 8,
                transition: "background 0.1s",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = "#F3F4F6")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
            >
              <span>{item.label}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
