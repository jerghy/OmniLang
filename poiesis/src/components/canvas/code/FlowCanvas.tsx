/** @xyflow/react wire canvas — data-flow visual programming with typed ports.
 *  Pure data flow architecture: no flow edges, no event nodes.
 *  Events are bound in Design mode PropertyPanel. */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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

import type { ElementData, WireData } from "../../../core/types";
import { useCodeGraphStore } from "../../../services/codegraph/store";
import { createCodeGraphContext } from "../../../services/codegraph/context";
import { AddWire } from "../../../core/commands/AddWire";
import { RemoveWire } from "../../../core/commands/RemoveWire";
import { AddElement } from "../../../core/commands/AddElement";
import { RemoveElement } from "../../../core/commands/RemoveElement";
import { getNodeConfig, type NodeTypeConfig } from "../shared/nodeRegistry";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { DataEdge } from "../shared/edges/DataEdge";
import type { BaseNodeData } from "./nodes/BaseNode";
import { getAllPorts, validateConnection, generateCustomFnPorts } from "../shared/connectionValidators";
import { NodePalette } from "../design/NodePalette";
import { elementsToNodes } from "../shared/elementsToNodes";
import { wiresToEdges } from "../shared/wiresToEdges";
import { showToast } from "../../../lib/toast";
import { uid } from "../../../lib/utils/id";

// ── Custom node/edge types ──

const nodeTypes = {
  codeblock: CodeBlockNode,
};

const edgeTypes = {
  data: DataEdge,
};

// ── Helpers ──

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

// ── Props ──

interface FlowCanvasProps {
  highlightedWireId?: string | null;
  onNodeDoubleClick?: (nodeId: string, nodeData: ElementData) => void;
  /** Increment to trigger auto-layout from outside. */
  layoutTrigger?: number;
  /** Active node IDs for live execution highlighting. */
  activeNodeIds?: Set<string>;
  /** Active edge IDs for live execution highlighting. */
  activeEdgeIds?: Set<string>;
}

// ── Component ──

export function FlowCanvas({
  highlightedWireId = null,
  onNodeDoubleClick,
  activeNodeIds,
  activeEdgeIds,
}: FlowCanvasProps) {
  const tabs = useCodeGraphStore((s) => s.tabs);
  const activeTabId = useCodeGraphStore((s) => s.activeTabId);
  const selectedElementIds = useCodeGraphStore((s) => s.selectedElementIds);

  const activeTab = useMemo(() => tabs.find((t) => t.id === activeTabId) || tabs[0], [tabs, activeTabId]);
  const elements = useMemo(() => activeTab?.graph.elements || [], [activeTab]);
  const wires = useMemo(() => activeTab?.graph.wires || [], [activeTab]);

  const containerRef = useRef<HTMLDivElement>(null);

  const onNodeDoubleClickRef = useRef(onNodeDoubleClick);
  // eslint-disable-next-line react-hooks/refs -- keep ref in sync with latest callback prop
  onNodeDoubleClickRef.current = onNodeDoubleClick;

  const [nodes, setNodes, onNodesChange] = useNodesState<Node<BaseNodeData>>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  // ── Convert elements → React Flow nodes ──

  useEffect(() => {
    const newNodes = elementsToNodes({
      elements,
      wires,
      selectedElementIds,
      activeNodeIds,
    });
    setNodes(newNodes);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- setNodes is a useState setter (stable)
  }, [elements, wires, selectedElementIds, activeNodeIds]);

  // ── Convert wires → React Flow edges ──

  useEffect(() => {
    const newEdges = wiresToEdges({
      wires,
      elements,
      highlightedWireId,
      activeEdgeIds,
    });
    setEdges(newEdges);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- setEdges is a useState setter (stable)
  }, [wires, elements, highlightedWireId, activeEdgeIds]);

  // ── Connection handler ──

  const handleConnect = useCallback(
    (connection: Connection) => {
      const fromEl = elements.find((e) => e.id === connection.source);
      const toEl = elements.find((e) => e.id === connection.target);
      if (!fromEl || !toEl) return;
      if (!connection.sourceHandle || !connection.targetHandle) return;

      const fromPort = connection.sourceHandle;
      const toPort = connection.targetHandle;

      const fromPorts = getAllPorts(fromEl);
      const toPorts = getAllPorts(toEl);
      const issues = validateConnection(
        fromEl, fromPort, toEl, toPort, wires, fromPorts, toPorts,
      );

      // Block on errors — use toast instead of alert()
      const errors = issues.filter((i) => i.severity === "error");
      if (errors.length > 0) {
        showToast(errors[0].message, "error");
        return;
      }

      // Log warnings
      for (const issue of issues) {
        if (issue.severity === "warning") {
          showToast(issue.message, "info");
        }
      }

      const wireData: WireData = {
        id: uid("w-"),
        fromId: fromEl.id,
        fromPort,
        toId: toEl.id,
        toPort,
      };

      const tabId = useCodeGraphStore.getState().activeTabId;
      const cCtx = createCodeGraphContext();
      const cmd = new AddWire(wireData);
      useCodeGraphStore.getState().pushUndo(tabId, cmd);
      cmd.execute(cCtx);
    },
    [elements, wires],
  );

  // ── Edge delete handler ──

  const handleEdgesDelete = useCallback(
    (edgesToDelete: Array<{ id: string }>) => {
      const cCtx = createCodeGraphContext();
      for (const edge of edgesToDelete) {
        const wire = cCtx.wires.find((w) => w.id === edge.id);
        if (wire) {
          const cmd = new RemoveWire(wire.id);
          useCodeGraphStore.getState().pushUndo(useCodeGraphStore.getState().activeTabId, cmd);
          cmd.execute(cCtx);
        }
      }
    },
    [],
  );

  // ── Node delete handler ──

  const handleNodesDelete = useCallback(
    (nodesToDelete: Array<{ id: string }>) => {
      const cCtx = createCodeGraphContext();
      const tabId = useCodeGraphStore.getState().activeTabId;
      for (const node of nodesToDelete) {
        const el = cCtx.elements.find((e) => e.id === node.id);
        if (!el) continue;

        const connectedWires = cCtx.wires.filter(
          (w) => w.fromId === el.id || w.toId === el.id,
        );
        for (const w of connectedWires) {
          const cmd = new RemoveWire(w.id);
          useCodeGraphStore.getState().pushUndo(tabId, cmd);
          cmd.execute(cCtx);
        }

        const cmd = new RemoveElement(el.id);
        useCodeGraphStore.getState().pushUndo(tabId, cmd);
        cmd.execute(cCtx);
      }
    },
    [],
  );

  // ── Node click → select in store ──

  const handleNodeClick = useCallback((_event: React.MouseEvent, node: { id: string }) => {
    useCodeGraphStore.getState().selectElement(node.id, _event.shiftKey);
  }, []);

  // ── Node drag stop → persist position ──

  const handleNodeDragStop = useCallback((_event: MouseEvent | TouchEvent, node: Node<BaseNodeData>) => {
    useCodeGraphStore.getState().setElementPosition(node.id, {
      x: node.position.x,
      y: node.position.y,
    });
  }, []);

  // ── Pane click → deselect ──

  const handlePaneClick = useCallback(() => {
    useCodeGraphStore.getState().selectElement(null);
  }, []);

  // ── Node double-click → open code editor ──

  const handleNodeDoubleClick = useCallback(
    (_event: React.MouseEvent, node: { id: string }) => {
      const cb = onNodeDoubleClickRef.current;
      if (!cb) return;
      const storeState = useCodeGraphStore.getState();
      const actTab = storeState.tabs.find((t) => t.id === storeState.activeTabId) || storeState.tabs[0];
      const el = actTab?.graph.elements.find((e) => e.id === node.id);
      if (el && el.nodeType === "codeblock" && el.tag === "fn") {
        cb(el.id, el);
      }
    },
    [],
  );

  // ── Context menu (right-click → add block) ──

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    setContextMenu({ x: e.clientX - rect.left, y: e.clientY - rect.top });
  }, []);

  // ── Keyboard shortcut: Tab to open NodePalette ──

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Tab" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        // Only if the canvas area is focused
        const active = document.activeElement;
        if (active?.closest("[data-flow-canvas]")) {
          e.preventDefault();
          const rect = containerRef.current?.getBoundingClientRect();
          if (rect) {
            setContextMenu({ x: rect.width / 2 - 160, y: rect.height / 2 - 200 });
          }
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // ── Drag-and-drop from library ──

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
        useCodeGraphStore.getState().setElementPosition(dropped.id, { x, y });
      } catch { /* ignore */ }
    }

    if (fnRaw) {
      try {
        const dropped = JSON.parse(fnRaw) as { fnId: string; fnName: string; params: string; returnType: string };
        const pports = parseFnString(dropped.params);
        const id = uid("fn-");
        const ports: Array<{ name: string; direction: "in" | "out"; type: string; label?: string; portType?: "flow" | "data" }> = [];
        for (const p of pports) {
          ports.push({ name: p.name, direction: "in", type: p.type, label: `${p.name}:${p.type}`, portType: "data" });
        }
        ports.push({ name: "result", direction: "out", type: dropped.returnType, label: `返回: ${dropped.returnType}`, portType: "data" });

        const el: ElementData = {
          id, nodeType: "codeblock", tag: "fn",
          props: { name: dropped.fnName },
          children: [], parentId: null,
          ports,
          isFolded: false,
          source: `// 📦 内联代码块：${dropped.fnName}(${dropped.params}) -> ${dropped.returnType}`,
        };
        const tabId = useCodeGraphStore.getState().activeTabId;
        const cCtx = createCodeGraphContext();
        const cmd = new AddElement(el);
        useCodeGraphStore.getState().pushUndo(tabId, cmd);
        cmd.execute(cCtx);
      } catch { /* ignore */ }
    }
  }, []);

  // ── Close context menu on outside click ──

  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  // ── NodePalette: select a node type from the search popup ──

  const handlePaletteSelect = useCallback(
    (nodeConfig: NodeTypeConfig) => {
      const id = uid(`node-${nodeConfig.tag}-`);
      const defaultPorts = nodeConfig.defaultPorts.map((p) => ({ ...p, direction: p.direction as "in" | "out" }));

      const el: ElementData = {
        id,
        nodeType: "codeblock",
        tag: nodeConfig.tag,
        props: { name: nodeConfig.label },
        children: [],
        parentId: null,
        ports: defaultPorts,
        isFolded: false,
        source: "",
        nodeCategory: nodeConfig.category,
        config: nodeConfig.config ? { ...nodeConfig.config } : undefined,
      };

      // customFn ports are generated from fnParams/fnReturnType config
      if (nodeConfig.tag === "customFn") {
        el.ports = generateCustomFnPorts(el);
      }

      const tabId = useCodeGraphStore.getState().activeTabId;
      const cCtx = createCodeGraphContext();
      const cmd = new AddElement(el);
      useCodeGraphStore.getState().pushUndo(tabId, cmd);
      cmd.execute(cCtx);
      setContextMenu(null);
    },
    [],
  );

  return (
    <div
      ref={containerRef}
      style={{ width: "100%", height: "100%", minHeight: 0, overflow: "hidden", position: "relative" }}
      onContextMenu={handleContextMenu}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      tabIndex={0}
      data-flow-canvas=""
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

      {/* NodePalette */}
      {contextMenu && (
        <NodePalette
          position={contextMenu}
          onSelect={handlePaletteSelect}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
