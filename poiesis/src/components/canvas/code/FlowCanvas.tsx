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
import { CodeCardEditor } from "./card-editor/CodeCardEditor";
import { emitOps } from "../../../services/codegen/code-op-emitter";
import { FlowInspector } from "./FlowInspector";
import { NodeContextMenu } from "./NodeContextMenu";
import { FlowCanvasEmptyState } from "./FlowCanvasEmptyState";
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
  const setElementOperations = usePoiesisStore((s) => s.setElementOperations);
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
  const elementsRef = useRef(elements);
  useEffect(() => { selectedNodeIdsRef.current = selectedNodeIds; });
  useEffect(() => { elementsRef.current = elements; });

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
      const elMap = new Map(elementsRef.current.map(el => [el.id, el]));
      for (const id of ids) {
        const el = elMap.get(id);
        if (el && "position" in el) {
          const pos = el.position ?? { x: 0, y: 0 };
          setElementPosition(id, { x: pos.x + dx, y: pos.y + dy });
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [removeElement, setElementPosition, setSelectedNodeIds]); // stable — bind once

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
    () => {
      const selectedSet = new Set(selectedNodeIds);
      return codeElements.map((el) => ({
        id: el.id,
        type: el.tag,
        position: el.position ?? { x: 0, y: 0 },
        selected: selectedSet.has(el.id),
        className: selectedSet.has(el.id) ? "flow-node-selected" : "",
        data: {
          label: TAG_CN[el.tag] || el.tag,
          source: el.source,
          fnName: el.props?.name as string,
          params: el.props?.params as string,
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
      }));
    },
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

  // ── Editing element (for CodeCardEditor) ──
  const editingElement = editingNodeId
    ? elements.find((e) => e.id === editingNodeId && e.nodeType === "codeblock")
    : null;

  return (
    <div ref={flowWrapperRef} style={{ width: "100%", height: "100%", display: "flex" }}>
      <div style={{ flex: 1, height: "100%", minWidth: 0 }}>
      {allNodes.length === 0 ? (
        <FlowCanvasEmptyState
          codeFlowElementId={codeFlowElementId}
          onContextMenu={handlePaneContextMenu}
        />
      ) : (
        <ReactFlow
          nodes={allNodes}
          edges={edges}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ maxZoom: 1 }}
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

      {/* Code Card Editor — double-click code-block node */}
      {editingElement && (
        <CodeCardEditor
          elementId={editingElement.id}
          initialOps={(editingElement as CodeblockElementData).operations || []}
          initialSource={(editingElement as CodeblockElementData).source || ""}
          language="omnilang"
          onSave={(operations) => {
            const source = emitOps(operations);
            setElementOperations(editingNodeId!, operations);
            setElementSource(editingNodeId!, source);
            setEditingNodeId(null);
          }}
          onClose={() => setEditingNodeId(null)}
        />
      )}

      {/* Node right-click menu — delete */}
      <NodeContextMenu
        nodeMenu={nodeMenu}
        menuRef={nodeMenuRef}
        deleteBtnClass={styles.deleteBtn}
        onDelete={(nodeId) => removeElement(nodeId)}
        onClose={() => setNodeMenu(null)}
      />
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
