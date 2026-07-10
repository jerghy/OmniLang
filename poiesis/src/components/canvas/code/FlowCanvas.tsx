import { useMemo, useState, useCallback, useEffect, useRef } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  Controls,
  Background,
  BackgroundVariant,
  type Node as RFNode,
  type Connection,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../../store";
import { useCodeElements, useWires } from "../../../store/hooks";
import { uid } from "../../../core/utils/id";
import type { CodeblockElementData } from "../../../core/types/element";
import { FnNode } from "./nodes/FnNode";
import { ConditionNode } from "./nodes/ConditionNode";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { ReturnNode } from "./nodes/ReturnNode";
import { DesignPreviewNode } from "./nodes/DesignPreviewNode";
import { CODE_TAGS, TAG_CN, type ContextMenuState } from "./flowConstants";
import { ContextMenu } from "./CanvasContextMenu";
import { CodeEditorModal } from "./CodeEditorModal";
import { FlowInspector } from "./FlowInspector";
import styles from "./FlowCanvas.module.css";

const nodeTypes = {
  "fn": FnNode,
  "code-block": CodeBlockNode,
  "condition": ConditionNode,
  "return": ReturnNode,
  "design-preview": DesignPreviewNode,
};

function FlowCanvasInner() {
  const elements = useCodeElements();
  const wires = useWires();
  const addElement = usePoiesisStore((s) => s.addElement);
  const codeFlowElementId = usePoiesisStore(
    (s) => s.designPage.codeFlowElementId,
  );

  // Read preview output from store (set by OmniViewRenderer)
  const previewOutput = usePoiesisStore((s) => s.designPage.previewOutput);

  const { screenToFlowPosition } = useReactFlow();
  const flowWrapperRef = useRef<HTMLDivElement>(null);

  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [collapsedSections, setCollapsedSections] = useState<Set<number>>(new Set());
  const menuRef = useRef<HTMLDivElement>(null);

  // Reset collapsed sections when menu opens
  useEffect(() => {
    if (contextMenu) setCollapsedSections(new Set());
  }, [contextMenu]);

  // Close menu on click outside
  useEffect(() => {
    if (!contextMenu) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [contextMenu]);

  // Close menu on Escape
  useEffect(() => {
    if (!contextMenu) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setContextMenu(null);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [contextMenu]);

  const handlePaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      const pos = screenToFlowPosition({ x: event.clientX, y: event.clientY });
      setContextMenu({ x: event.clientX, y: event.clientY, flowX: pos.x, flowY: pos.y });
    },
    [screenToFlowPosition],
  );

  const handleAddNode = useCallback(
    (tag: string) => {
      if (!contextMenu) return;
      const id = uid(tag);
      const jitter = Math.random() * 40 - 20;
      const props: Record<string, unknown> = {};

      if (tag === "fn") {
        props.name = "newFn";
        props.params = "";
      }
      if (tag === "condition") {
        props.branches = [{ id: "br0", type: "if", condition: "" }];
      }

      const newEl: CodeblockElementData = {
        id,
        nodeType: "codeblock",
        tag,
        props,
        parentId: null,
        position: { x: contextMenu.flowX + jitter, y: contextMenu.flowY + jitter },
        source: "",
      };
      addElement(newEl);
      setContextMenu(null);
    },
    [contextMenu, addElement],
  );

  // ── Store actions ──
  const setElementPosition = usePoiesisStore((s) => s.setElementPosition);
  const removeElement = usePoiesisStore((s) => s.removeElement);
  const setProp = usePoiesisStore((s) => s.setProp);
  const setElementSource = usePoiesisStore((s) => s.setElementSource);
  const addWire = usePoiesisStore((s) => s.addWire);

  // ── Connect handler ──
  const onConnect = useCallback(
    (connection: Connection) => {
      addWire({
        id: uid("wire"),
        fromId: connection.source,
        fromPort: connection.sourceHandle || "flow-out",
        toId: connection.target,
        toPort: connection.targetHandle || "flow-in",
      });
    },
    [addWire],
  );

  // ── Selected node IDs (manual tracking — xyflow v12 internal selection resets on element updates) ──
  const [selectedNodeIds, setSelectedNodeIds] = useState<string[]>([]);
  const [editingNodeId, setEditingNodeId] = useState<string | null>(null);

  // ── Node context menu (right-click on a node) ──
  const [nodeMenu, setNodeMenu] = useState<{ x: number; y: number; nodeId: string } | null>(null);
  const nodeMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!nodeMenu) return;
    const handler = (e: MouseEvent) => {
      if (nodeMenuRef.current && !nodeMenuRef.current.contains(e.target as Node)) {
        setNodeMenu(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [nodeMenu]);

  // ── Keyboard: arrow keys to move selected nodes, Delete to remove ──
  const MOVE_STEP = 10;
  const selectedNodeIdsRef = useRef(selectedNodeIds);
  selectedNodeIdsRef.current = selectedNodeIds;
  const elementsRef = useRef(elements);
  elementsRef.current = elements;

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const ids = selectedNodeIdsRef.current;
      if (ids.length === 0) return;
      // Skip if user is typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        for (const id of ids) removeElement(id);
        setSelectedNodeIds([]);
        return;
      }
      let dx = 0, dy = 0;
      if (e.key === "ArrowUp") dy = -MOVE_STEP;
      else if (e.key === "ArrowDown") dy = MOVE_STEP;
      else if (e.key === "ArrowLeft") dx = -MOVE_STEP;
      else if (e.key === "ArrowRight") dx = MOVE_STEP;
      else return;
      e.preventDefault();
      for (const id of ids) {
        const el = elementsRef.current.find((x) => x.id === id);
        if (el && "position" in el) {
          const pos = el.position ?? { x: 0, y: 0 };
          setElementPosition(id, { x: pos.x + dx, y: pos.y + dy });
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []); // stable — bind once

  // ── Drag: commit position only on drag end ──
  // During drag, ReactFlow manages node positions internally (uncontrolled for position).
  // We only commit to the Zustand store when the user finishes dragging,
  // avoiding excessive store updates on every pixel of movement.
  const onNodeDragStop = useCallback(
    (_event: MouseEvent | TouchEvent, node: RFNode) => {
      setElementPosition(node.id, node.position);
    },
    [setElementPosition],
  );

  // ── Node context menu handler ──
  const handleNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: RFNode) => {
      event.preventDefault();
      setNodeMenu({ x: event.clientX, y: event.clientY, nodeId: node.id });
    },
    [],
  );

  // ── Build nodes/edges ──
  // Filter to code-block elements only. CODE_TAGS may overlap with view-element
  // tags (e.g., "div" means both HTML <div> and the division operator). Without
  // the nodeType guard, deleting a code-flow node would cascade-delete the
  // OmniView root element and all its children.
  const codeElements = useMemo(
    () => elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock" && CODE_TAGS.has(e.tag)),
    [elements],
  );

  const nodes: RFNode[] = useMemo(
    () =>
      codeElements.map((el) => ({
        id: el.id,
        type: el.tag,
        position: el.position ?? { x: 0, y: 0 },
        selected: selectedNodeIds.includes(el.id),
        className: selectedNodeIds.includes(el.id) ? "flow-node-selected" : "",
        data: {
          label: TAG_CN[el.tag] || el.tag,
          source: el.source,
          fnName: el.props?.name as string,
          params: el.props?.params as string,
          onParamsChange: (newParams: string) => {
            setProp(el.id, "params", newParams);
          },
          branches: el.props?.branches as Array<{id:string;type:string;condition:string}> | undefined,
          returnExpr: el.source,
          description: el.props?.description as string | undefined,
          onDoubleClick: () => setEditingNodeId(el.id),
          onUpdateBranches: (branches: unknown) => {
            setProp(el.id, "branches", branches);
          },
          onEditCondition: (branchId: string, condition: string) => {
            const currentBranches = [...(el.props?.branches as Array<{id:string;type:string;condition:string}> || [])];
            const idx = currentBranches.findIndex(b => b.id === branchId);
            if (idx >= 0) {
              currentBranches[idx] = { ...currentBranches[idx], condition };
              setProp(el.id, "branches", currentBranches);
            }
          },
        },
      })),
    [codeElements, selectedNodeIds, setEditingNodeId, setProp],
  );

  const edges = useMemo(
    () =>
      wires.map((w) => ({
        id: w.id,
        source: w.fromId,
        target: w.toId,
        sourceHandle: w.fromPort,
        targetHandle: w.toPort,
      })),
    [wires],
  );

  // Preview node — shows the design canvas inside the flow when in codeflow mode
  const previewNode: RFNode | null = useMemo(() => {
    if (!codeFlowElementId) return null;
    return {
      id: "design-preview",
      type: "design-preview",
      position: { x: 50, y: 50 },
      selectable: false,
      draggable: true,
      data: { output: previewOutput },
    };
  }, [codeFlowElementId, previewOutput]);

  const allNodes = useMemo(
    () => (previewNode ? [...nodes, previewNode] : nodes),
    [nodes, previewNode],
  );

  // ── Editing element (for CodeEditorModal) ──
  const editingElement = editingNodeId
    ? elements.find((e) => e.id === editingNodeId && e.nodeType === "codeblock")
    : null;

  const handleSaveCode = useCallback(
    (source: string, description: string) => {
      if (!editingNodeId) return;
      setElementSource(editingNodeId, source);
      setProp(editingNodeId, "description", description);
      setEditingNodeId(null);
    },
    [editingNodeId, setElementSource, setProp],
  );

  return (
    <div ref={flowWrapperRef} style={{ width: "100%", height: "100%", display: "flex" }}>
      <div style={{ flex: 1, height: "100%", minWidth: 0 }}>
      {allNodes.length === 0 ? (
        <div
          onContextMenu={handlePaneContextMenu}
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            height: "100%",
            color: "#94a3b8",
            fontSize: 13,
            gap: 6,
          }}
        >
          <div>
            {codeFlowElementId
              ? "双击组件元素进入代码节点流编辑"
              : "右键画布添加节点，或从组件标签页双击进入"}
          </div>
          <div style={{ fontSize: 11, opacity: 0.7 }}>右键 → 添加节点</div>
        </div>
      ) : (
        <ReactFlow
          nodes={allNodes}
          edges={edges}
          nodeTypes={nodeTypes}
          fitView
          nodesDraggable
          nodesConnectable
          nodesFocusable
          elementsSelectable
          panOnScroll
          onNodeClick={(_event, node) => setSelectedNodeIds([node.id])}
          onPaneClick={() => setSelectedNodeIds([])}
          onNodeDragStop={onNodeDragStop}
          onConnect={onConnect}
          onPaneContextMenu={handlePaneContextMenu}
          onNodeContextMenu={handleNodeContextMenu}
        >
          <Background variant={BackgroundVariant.Dots} gap={16} size={1} color="#e2e8f0" />
          <Controls
            showZoom
            showFitView
            showInteractive={false}
            position="bottom-right"
          />
        </ReactFlow>
      )}

      {/* Right-click context menu with collapsible sections and overflow handling */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          menuRef={menuRef}
          collapsedSections={collapsedSections}
          onToggleSection={(i) => setCollapsedSections((prev) => {
            const next = new Set(prev);
            if (next.has(i)) next.delete(i); else next.add(i);
            return next;
          })}
          onSelectNode={(tag) => {
            handleAddNode(tag);
          }}
          onClose={() => setContextMenu(null)}
        />
      )}

      {/* Code Editor Modal — double-click code-block node */}
      {editingElement && (
        <CodeEditorModal
          elementId={editingElement.id}
          initialSource={(editingElement as CodeblockElementData).source}
          initialDescription={((editingElement as CodeblockElementData).props?.description as string) || ""}
          onSave={handleSaveCode}
          onClose={() => setEditingNodeId(null)}
        />
      )}

      {/* Node right-click menu — delete */}
      {nodeMenu && (
        <div
          ref={nodeMenuRef}
          style={{
            position: "fixed",
            top: nodeMenu.y,
            left: nodeMenu.x,
            zIndex: 10001,
            background: "#fff",
            borderRadius: 6,
            boxShadow: "0 2px 10px rgba(0,0,0,0.15)",
            padding: "2px 0",
            minWidth: 120,
            fontSize: 12,
          }}
          onContextMenu={(e) => e.preventDefault()}
        >
          <button
            onClick={() => { removeElement(nodeMenu.nodeId); setNodeMenu(null); }}
            className={styles.deleteBtn}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              width: "100%",
              padding: "6px 12px",
              border: "none",
              cursor: "pointer",
              fontSize: 12,
              color: "#dc2626",
              textAlign: "left" as const,
              fontFamily: "inherit",
            }}
          >
            🗑 删除节点
          </button>
        </div>
      )}
      </div>
      {/* Right-side inspector: edits selected FN or Return node */}
      <FlowInspector selectedNodeIds={selectedNodeIds} />
    </div>
  );
}

export function FlowCanvas() {
  return (
    <ReactFlowProvider>
      <FlowCanvasInner />
    </ReactFlowProvider>
  );
}
