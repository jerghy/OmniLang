import { useMemo, useState, useCallback, useEffect, useRef } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useOnSelectionChange,
  type Node as RFNode,
  type OnNodesChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../../store";
import { uid } from "../../../core/utils/id";
import type { ElementData } from "../../../core/types/element";
import { FunctionHeaderNode } from "./nodes/FunctionHeaderNode";
import { SkeletonNode } from "./nodes/SkeletonNode";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { ReturnNode } from "./nodes/ReturnNode";
import { CODE_TAGS, TAG_CN, type ContextMenuState } from "./flowConstants";
import { ContextMenu } from "./CanvasContextMenu";
import styles from "./FlowCanvas.module.css";

const nodeTypes = {
  "function-header": FunctionHeaderNode,
  skeleton: SkeletonNode,
  "code-block": CodeBlockNode,
  return: ReturnNode,
};

function FlowCanvasInner() {
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const addElement = usePoiesisStore((s) => s.addElement);
  const codeFlowElementId = usePoiesisStore(
    (s) => s.designPage.codeFlowElementId,
  );

  const { screenToFlowPosition } = useReactFlow();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [collapsedSections, setCollapsedSections] = useState<Set<number>>(new Set());
  const menuRef = useRef<HTMLDivElement>(null);

  // Derive registered custom functions from elements
  const registeredFns = useMemo(() => {
    const registered = new Set<string>();
    for (const el of elements) {
      if (el.tag === "customFn" && el.config?.fnName) {
        registered.add(el.config.fnName as string);
      } else if (el.tag === "fn" && el.props?.name) {
        registered.add(el.props.name as string);
      }
    }
    return registered;
  }, [elements]);

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
    (tag: string, fnName?: string) => {
      if (!contextMenu) return;
      const id = uid(tag);
      const jitter = Math.random() * 40 - 20;
      const props: Record<string, string> = {};
      const config: Record<string, unknown> = {};

      if (tag === "fn") {
        props.name = "newFn";
      }
      if (tag === "customFn") {
        const name = fnName || "newCall";
        props.name = name;
        config.fnName = name;
      }
      if (tag === "signal") {
        props.name = "newSignal";
        props.typeAnnotation = "string";
      }
      if (tag === "computed") {
        props.name = "newComputed";
        props.typeAnnotation = "string";
      }
      if (tag === "store") {
        props.name = "newStore";
      }
      if (tag === "literal") {
        props.literal = "''";
      }

      const newEl: ElementData = {
        id,
        nodeType: "codeblock" as const,
        tag,
        props,
        children: [],
        parentId: null,
        position: { x: contextMenu.flowX + jitter, y: contextMenu.flowY + jitter },
        ...(Object.keys(config).length > 0 ? { config } : {}),
      };
      addElement(newEl);
      setContextMenu(null);
    },
    [contextMenu, addElement],
  );

  // ── Store actions ──
  const setElementPosition = usePoiesisStore((s) => s.setElementPosition);
  const removeElement = usePoiesisStore((s) => s.removeElement);

  // ── Selected node IDs ──
  const [selectedNodeIds, setSelectedNodeIds] = useState<string[]>([]);
  useOnSelectionChange({
    onChange: ({ nodes: selNodes }) => {
      setSelectedNodeIds(selNodes.map((n) => n.id));
    },
  });

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
        if (el) {
          const pos = el.position || { x: 0, y: 0 };
          setElementPosition(id, { x: pos.x + dx, y: pos.y + dy });
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []); // stable — bind once

  // ── Drag: sync position back to store ──
  const onNodesChange: OnNodesChange = useCallback(
    (changes) => {
      for (const change of changes) {
        if (change.type === "position" && change.position) {
          // Update store when node is dragged to new position
          if (!change.dragging) {
            setElementPosition(change.id, change.position);
          }
        }
      }
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
  const codeElements = useMemo(
    () => elements.filter((e) => CODE_TAGS.has(e.tag)),
    [elements],
  );

  const nodes: RFNode[] = useMemo(
    () =>
      codeElements.map((el) => ({
        id: el.id,
        type:
          el.tag === "fn"
            ? "function-header"
            : el.tag === "if" || el.tag === "else" || el.tag === "for" || el.tag === "while"
              ? "skeleton"
              : el.tag === "return"
                ? "return"
                : "code-block",
        position: el.position || { x: 0, y: 0 },
        data: {
          label: TAG_CN[el.tag] || el.tag,
          source: el.source,
          fnName: el.props?.name as string,
          skeletonType: el.tag,
          condition: el.props?.condition as string,
        },
      })),
    [codeElements],
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

  return (
    <div style={{ width: "100%", height: "100%" }}>
      {nodes.length === 0 ? (
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
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          fitView
          nodesDraggable
          nodesConnectable
          elementsSelectable
          onNodesChange={onNodesChange}
          onPaneContextMenu={handlePaneContextMenu}
          onNodeContextMenu={handleNodeContextMenu}
        />
      )}

      {/* Right-click context menu with collapsible sections and overflow handling */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          menuRef={menuRef}
          collapsedSections={collapsedSections}
          registeredFns={registeredFns}
          onToggleSection={(i) => setCollapsedSections((prev) => {
            const next = new Set(prev);
            if (next.has(i)) next.delete(i); else next.add(i);
            return next;
          })}
          onSelectNode={(tag, fnName) => {
            handleAddNode(tag, fnName);
          }}
          onClose={() => setContextMenu(null)}
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
  );
}

export function FlowCanvas() {
  return (
    <ReactFlowProvider>
      <FlowCanvasInner />
    </ReactFlowProvider>
  );
}
