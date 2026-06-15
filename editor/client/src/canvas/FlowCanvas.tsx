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
import { useCodeGraphStore } from "../codegraph/store";
import { createCodeGraphContext } from "../codegraph/context";
import { AddWire } from "../commands/AddWire";
import { RemoveWire } from "../commands/RemoveWire";
import { AddElement } from "../commands/AddElement";
import { RemoveElement } from "../commands/RemoveElement";
import type { CommandContext, CodeGraphCommandContext } from "../commands/types";
import { getNodeConfig, type NodeTypeConfig } from "./nodeRegistry";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { ViewEventNode } from "./nodes/ViewEventNode";
import { FlowEdge } from "./edges/FlowEdge";
import { DataEdge } from "./edges/DataEdge";
import { isTypeCompatible, typeMismatchMessage } from "./socketTypes";
import { getAllPorts, validateConnection } from "./connectionValidators";
import { validateFlow } from "./FlowEngine";
import { NodePalette } from "./NodePalette";
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
  const fromPorts = fromEl ? getAllPorts(fromEl) : [];
  const toPorts = toEl ? getAllPorts(toEl) : [];
  const fromPort = fromPorts.find((p) => p.name === wire.fromPort);
  const toPort = toPorts.find((p) => p.name === wire.toPort);
  if (fromPort?.portType === "flow" || toPort?.portType === "flow") return "flow";
  return "data";
}

/** Get the data type for coloring DataEdges. */
function getEdgeDataType(fromEl: ElementData | undefined, wire: WireData): string {
  const fromPorts = fromEl ? getAllPorts(fromEl) : [];
  const fromPort = fromPorts.find((p) => p.name === wire.fromPort);
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
  /** Phase 2: Active node IDs for live execution highlighting. */
  activeNodeIds?: Set<string>;
  /** Phase 2: Active edge IDs for live execution highlighting. */
  activeEdgeIds?: Set<string>;
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
  activeNodeIds,
  activeEdgeIds,
}: FlowCanvasProps) {
  const elements = useCodeGraphStore((s) => s.activeGraph.elements);
  const wires = useCodeGraphStore((s) => s.activeGraph.wires);
  const selectedElementIds = useCodeGraphStore((s) => s.selectedElementIds);
  const activeTabId = useCodeGraphStore((s) => s.activeTabId);
  const pushUndo = useCodeGraphStore((s) => s.pushUndo);
  const codeCtx = createCodeGraphContext();
  const containerRef = useRef<HTMLDivElement>(null);

  // Stable ref for onNodeDoubleClick to avoid useEffect re-run on every render
  const onNodeDoubleClickRef = useRef(onNodeDoubleClick);
  onNodeDoubleClickRef.current = onNodeDoubleClick;

  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  // ── Flow validation state (design-time analysis) ──

  const [flowIssues, setFlowIssues] = useState<{
    cycles: string[][];
    orphanFlowIns: string[];
    unreachable: string[];
  }>({ cycles: [], orphanFlowIns: [], unreachable: [] });

  useEffect(() => {
    const result = validateFlow(elements, wires);
    // Find unreachable codeblock nodes (not event sources, not in any flow chain)
    const visited = new Set<string>();
    const eventNodes = elements.filter(
      (e) => e.nodeCategory === "event" || e.tag === "onPress" || e.tag === "onChange" || e.tag === "onHover" || e.tag === "onMount",
    );
    // BFS from event nodes through wires
    const queue = eventNodes.map((e) => e.id);
    while (queue.length > 0) {
      const id = queue.shift()!;
      if (visited.has(id)) continue;
      visited.add(id);
      const outs = wires.filter((w) => w.fromId === id);
      for (const w of outs) {
        if (!visited.has(w.toId)) queue.push(w.toId);
      }
    }
    // Unreachable = codeblock nodes not visited that SHOULD be connected to an event source.
    // Signal/computed/store can legitimately exist standalone as state holders — don't flag them.
    // Only flag action/flow nodes that are meant to be triggered by events.
    const unreachable = elements
      .filter((e) => {
        if (visited.has(e.id)) return false;
        if (e.nodeType !== "codeblock") return false;
        if (e.nodeCategory === "event") return false;
        // Variable/data nodes (signal, computed, store) are standalone — not unreachable
        if (e.nodeCategory === "variable" || e.nodeCategory === "data") return false;
        return true;
      })
      .map((e) => e.id);

    setFlowIssues({
      cycles: result.cycles,
      orphanFlowIns: result.orphanFlowIns,
      unreachable,
    });
  }, [elements, wires]);

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
      const ports = getAllPorts(el);
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
      };

      // Apply validation-driven styling
      const isInCycle = flowIssues.cycles.some((c) => c.includes(el.id));
      const isUnreachable = flowIssues.unreachable.includes(el.id);
      const isOrphanFlowIn = flowIssues.orphanFlowIns.includes(el.id);
      const isExecuting = activeNodeIds?.has(el.id);
      const validationStyle: React.CSSProperties = {};
      if (isExecuting) {
        validationStyle.border = "2px solid #10B981";
        validationStyle.boxShadow = "0 0 12px rgba(16, 185, 129, 0.6)";
        validationStyle.transition = "box-shadow 0.3s ease, border 0.3s ease";
      } else if (isInCycle) {
        validationStyle.border = "2px solid #EF4444";
        validationStyle.boxShadow = "0 0 8px rgba(239,68,68,0.4)";
      } else if (isOrphanFlowIn) {
        validationStyle.border = "2px solid #F59E0B";
        validationStyle.boxShadow = "0 0 6px rgba(245,158,11,0.3)";
      } else if (isUnreachable) {
        validationStyle.opacity = 0.4;
        validationStyle.border = "2px dashed #F97316";
      }

      newNodes.push({
        id: el.id,
        type: el.nodeType === "codeblock" ? ("codeblock" as const) : ("viewEvent" as const),
        position: pos,
        data: nodeData as unknown as Record<string, unknown>,
        selected: selectedElementIds.includes(el.id),
        style: validationStyle,
      });
    }

    setNodes(newNodes);
  }, [elements, wires, showViews, selectedElementIds, flowIssues, activeNodeIds]);

  // ── Convert wires -> React Flow edges ──

  useEffect(() => {
    const newEdges: Edge[] = [];

    for (const w of wires) {
      const fromEl = elements.find((e) => e.id === w.fromId);
      const toEl = elements.find((e) => e.id === w.toId);
      if (!fromEl || !toEl) continue;

      const edgeType = getEdgeType(fromEl, toEl, w);
      const dataType = getEdgeDataType(fromEl, w);

      const isActiveEdge = activeEdgeIds?.has(w.id);
      newEdges.push({
        id: w.id,
        source: w.fromId,
        target: w.toId,
        sourceHandle: w.fromPort,
        targetHandle: w.toPort,
        type: edgeType,
        animated: edgeType === "flow" || isActiveEdge,
        data: { type: dataType },
        selected: w.id === highlightedWireId,
        style: isActiveEdge ? {
          stroke: "#10B981",
          strokeWidth: 2.5,
          filter: "drop-shadow(0 0 4px rgba(16,185,129,0.5))",
        } : undefined,
      });
    }

    setEdges(newEdges);
  }, [wires, elements, highlightedWireId, activeEdgeIds]);

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

      // Use shared validation (same as OmniDriver.connect())
      const fromPorts = getAllPorts(fromEl);
      const toPorts = getAllPorts(toEl);
      const issues = validateConnection(
        fromEl, fromPort, toEl, toPort, wires, fromPorts, toPorts,
      );

      // Block on errors
      const errors = issues.filter((i) => i.severity === "error");
      if (errors.length > 0) {
        alert(errors[0].message);
        return;
      }

      // Log warnings
      for (const w of issues) {
        console.warn(`[FlowCanvas] ${w.code}: ${w.message}`);
      }

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
    useEditorStore.getState().selectElement(node.id, _event.shiftKey);
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
      const el = useEditorStore.getState().elements.find((e) => e.id === node.id);
      if (el && el.nodeType === "codeblock") {
        cb(el.id, el);
      }
    },
    [], // stable — uses store.getState() for fresh data
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

  // ── NodePalette: select a node type from the search popup ──

  const handlePaletteSelect = useCallback(
    (config: NodeTypeConfig) => {
      const id = `node-${config.tag}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const ports = config.defaultPorts.map((p) => ({ ...p, direction: p.direction as "in" | "out" }));

      const el: ElementData = {
        id,
        nodeType: "codeblock",
        tag: config.tag,
        props: { name: config.label },
        children: [],
        parentId: null,
        ports,
        isFolded: false,
        source: config.tag === "action" ? DEFAULT_SOURCE : "",
        nodeCategory: config.category,
      };
      useUndoStore.getState().execute(new AddElement(el), ctx);
      setContextMenu(null);
    },
    [ctx],
  );

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

      {/* NodePalette — Unity VS style search popup */}
      {contextMenu && (
        <NodePalette
          position={contextMenu}
          onSelect={handlePaletteSelect}
          onClose={() => setContextMenu(null)}
        />
      )}

      {/* Flow validation issues panel */}
      {(flowIssues.cycles.length > 0 || flowIssues.orphanFlowIns.length > 0 || flowIssues.unreachable.length > 0) && (
        <div style={{
          position: "absolute", bottom: 12, left: 12, right: 12,
          background: "#FFF7ED", border: "1px solid #FED7AA",
          borderRadius: 8, padding: "8px 12px",
          fontSize: 11, fontFamily: "system-ui", zIndex: 10,
          maxHeight: 120, overflow: "auto",
          boxShadow: "0 2px 8px rgba(0,0,0,0.08)",
        }}>
          <div style={{ fontWeight: 600, color: "#C2410C", marginBottom: 4 }}>
            ⚠ Flow Validation Issues
          </div>
          {flowIssues.cycles.length > 0 && (
            <div style={{ color: "#991B1B", marginBottom: 2 }}>
              🔴 {flowIssues.cycles.length} cycle(s) detected — execution loops not allowed
            </div>
          )}
          {flowIssues.orphanFlowIns.length > 0 && (
            <div style={{ color: "#92400E", marginBottom: 2 }}>
              🟡 {flowIssues.orphanFlowIns.length} action(s) with unconnected flow input
            </div>
          )}
          {flowIssues.unreachable.length > 0 && (
            <div style={{ color: "#9A3412" }}>
              🟠 {flowIssues.unreachable.length} node(s) unreachable from any event source
            </div>
          )}
        </div>
      )}
    </div>
  );
}
