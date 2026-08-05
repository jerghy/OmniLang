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
  type NodeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../../store";
import { useCodeElements, useWires } from "../../../store/hooks";
import { uid } from "../../../core/utils/id";
import type { CodeblockElementData, ElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";
import type { CodeTab } from "../../../core/types/codegraph";
import { FnNode } from "./nodes/FnNode";
import { ConditionNode } from "./nodes/ConditionNode";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { ReturnNode } from "./nodes/ReturnNode";
import { DesignPreviewNode } from "./nodes/DesignPreviewNode";
import { SyntaxNode } from "./nodes/SyntaxNode";
import { LoopNode } from "./nodes/LoopNode";
import { defaultSyntaxProps } from "../../../services/codegen/syntax-nodes";
import { useFlowScope } from "../../../services/codegen/flow-scope-hook";
import { collectLetVarsPerFn, findOwnerFnId } from "../../../services/codegen/fn-topology";
import { CODE_TAGS, TAG_CN } from "./flowConstants";
import { computeLayeredLayout, estimateNodeSize } from "../../../services/codegen/flow-layout";
import { NodeSearchMenu } from "./NodeSearchMenu";
import { CodeBlockEditor } from "./CodeBlockEditor";
import type { CodeBlockEditorContext } from "./CodeBlockEditor";
import type { DependencyInfo } from "../../../core/types/dependency-info";
import { FlowInspector } from "./FlowInspector";
import { FlowEdge } from "./FlowEdge";
import styles from "./FlowCanvas.module.css";

const nodeTypes = {
  "fn": FnNode,
  "code-block": CodeBlockNode,
  "condition": ConditionNode,
  "return": ReturnNode,
  "design-preview": DesignPreviewNode,
  "let": SyntaxNode,
  "assign": SyntaxNode,
  "store-call": SyntaxNode,
  "call": SyntaxNode,
  "while": LoopNode,
  "for-in": LoopNode,
};

/** 正交 wire 路由（2026-08-05）：分支 wire 走走廊，不扫过目标列上方节点（bezier 圆弧穿节点根除） */
const edgeTypes = { flow: FlowEdge };

function FlowCanvasInner() {
  const elements = useCodeElements();
  // 完整元素（含 view 设计树）：布局写回必须基于全量（useCodeElements 仅 codeblock——
  // 见 handleAutoLayout 注释，Task 12 E2E CRITICAL）。
  const allElements = usePoiesisStore((s) => s.elements);
  const wires = useWires();
  const addElement = usePoiesisStore((s) => s.addElement);
  const addWire = usePoiesisStore((s) => s.addWire);
  const codeFlowElementId = usePoiesisStore(
    (s) => s.designPage.codeFlowElementId,
  );

  // Read preview output from store (set by OmniViewRenderer)
  const previewOutput = usePoiesisStore((s) => s.designPage.previewOutput);


  const { screenToFlowPosition } = useReactFlow();
  const flowWrapperRef = useRef<HTMLDivElement>(null);

  // ── 搜索菜单（替换原 ContextMenu）：右键 / 空格 / 拖出端口创建唤出 ──
  const [menuState, setMenuState] = useState<{ x: number; y: number; flowX: number; flowY: number } | null>(null);
  const pendingDragRef = useRef<{ nodeId: string; handleId: string } | null>(null);
  const menuOpenAtRef = useRef(0); // 300ms 防误关
  // 与 menuState 同步——空格分支在绑定一次的键盘闭包中经 ref 读最新值（刻意模式，勿重构）
  const menuOpenRef = useRef(false);

  const openMenuAt = useCallback(
    (x: number, y: number) => {
      const pos = screenToFlowPosition({ x, y });
      menuOpenAtRef.current = Date.now();
      menuOpenRef.current = true;
      setMenuState({ x, y, flowX: pos.x, flowY: pos.y });
    },
    [screenToFlowPosition],
  );

  // 右键画布
  const handlePaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      if (Date.now() - menuOpenAtRef.current < 300) return; // 防误关（刚打开时忽略）
      pendingDragRef.current = null;
      openMenuAt(event.clientX, event.clientY);
    },
    [openMenuAt],
  );

  // ── 拖出创建状态机：仅 source/flow-out 端口拖出记录，空白释放→菜单→选中自动连线 ──
  const onConnectStart = useCallback(
    (_e: unknown, params: { nodeId: string | null; handleId: string | null; handleType: string | null }) => {
      if (params.handleId === "flow-out" || params.handleType === "source") {
        // 记录实际 handleId：condition 分支口（br0/br1 等）拖出时 fromPort 必须用分支 id，
        // 否则连线 sourceHandle 不匹配（不可见边 + xyflow error008 + codegen 读错端口）
        if (params.nodeId && params.handleId) {
          pendingDragRef.current = { nodeId: params.nodeId, handleId: params.handleId };
        }
      } else {
        pendingDragRef.current = null;
      }
    },
    [],
  );

  const onConnectEnd = useCallback(
    (event: MouseEvent | TouchEvent) => {
      if (pendingDragRef.current && isPaneRelease(event)) {
        openMenuAt(
          event instanceof MouseEvent ? event.clientX : window.innerWidth / 2,
          event instanceof MouseEvent ? event.clientY : window.innerHeight / 2,
        );
        // pendingDragRef 保留：菜单选中时取用
      } else {
        pendingDragRef.current = null;
      }
    },
    [openMenuAt],
  );

  // 菜单选中：若 pendingDrag 存在则自动连线
  const handleMenuSelect = useCallback(
    (tag: string) => {
      if (!menuState) return;
      const id = uid(tag);
      const jitter = Math.random() * 40 - 20;
      let props: Record<string, unknown> = {};
      if (tag === "fn") { props = { name: "newFn", params: "" }; }
      if (tag === "condition") { props = { branches: [{ id: "br0", type: "if", condition: "" }] }; }
      if (["let", "assign", "store-call", "call", "while", "for-in"].includes(tag)) { props = { ...defaultSyntaxProps(tag) }; }
      addElement({ id, nodeType: "codeblock", tag, props, parentId: null, position: { x: menuState.flowX + jitter, y: menuState.flowY + jitter }, source: "" });
      if (pendingDragRef.current) {
        addWire({
          id: uid("wire"),
          fromId: pendingDragRef.current.nodeId,
          fromPort: pendingDragRef.current.handleId ?? "flow-out", // source 拖出恒有 handleId，兜底保留
          toId: id,
          toPort: "flow-in",
        });
        pendingDragRef.current = null;
      }
      menuOpenRef.current = false;
      setMenuState(null);
    },
    [menuState, addElement, addWire],
  );

  // ── Store actions ──
  const setElementPosition = usePoiesisStore((s) => s.setElementPosition);
  const removeElement = usePoiesisStore((s) => s.removeElement);
  const setProp = usePoiesisStore((s) => s.setProp);
  const setElementSource = usePoiesisStore((s) => s.setElementSource);
  const setNodeMeta = usePoiesisStore((s) => s.setNodeMeta);
  const setElementsAndWires = usePoiesisStore((s) => s.setElementsAndWires);
  // ── Context for CodeBlockEditor WASM analysis ──
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const storeReferences = usePoiesisStore((s) => s.storeReferences);
  const activeTab = openTabs.find((t) => t.id === activeTabId);
  const flowScope = useFlowScope(activeTab?.componentName);

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
      // 空格：画布内唤出搜索菜单（非输入焦点）。菜单开关经 menuOpenRef 读最新值，
      // 因为本闭包只绑定一次（刻意模式，勿改依赖）——直接读 menuState 会拿到旧值。
      if (
        e.key === " " &&
        !menuOpenRef.current &&
        !(e.target instanceof HTMLInputElement) &&
        !(e.target instanceof HTMLTextAreaElement) &&
        !(e.target instanceof HTMLSelectElement) &&
        !(e.target instanceof HTMLButtonElement) // 空格不抢按钮焦点（+ 添加参数 / × / mut / emit 等）
      ) {
        e.preventDefault();
        openMenuAt(window.innerWidth / 2, window.innerHeight / 2);
        return;
      }
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

  // ── Drag: controlled 模式必须经 onNodesChange 回写位置 ──
  // xyflow v12 受控流（nodes prop，无 defaultNodes）下，拖拽中的位置更新被内部
  // `hasDefaultNodes` 门控丢弃，仅 onNodesChange 会收到变更——不接线则节点拖拽冻结、
  // 松开后瞬移。这里把 position 变更实时回写 store（setElementPosition 无 undo 条目，
  // 每帧提交不污染撤销栈；末帧 dragging:false 的变更即最终落点）。
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      for (const change of changes) {
        if (change.type === "position" && change.position) {
          setElementPosition(change.id, change.position);
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
  // Filter to code-block elements only. CODE_TAGS may overlap with view-element
  // tags (e.g., "div" means both HTML <div> and the division operator). Without
  // the nodeType guard, deleting a code-flow node would cascade-delete the
  // OmniView root element and all its children.
  const codeElements = useMemo(
    () => elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock" && CODE_TAGS.has(e.tag)),
    [elements],
  );

  // 按 wire 拓扑推断每个 fn 的 let 变量（多 fn 隔离）：语法节点公式框候选数据源
  const letVarsByFn = useMemo(() => collectLetVarsPerFn(codeElements, wires), [codeElements, wires]);

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
          tag: el.tag,
          props: el.props,
          source: el.source,
          fnName: el.props?.name as string,
          params: el.props?.params as string,
          branches: el.props?.branches as Array<{id:string;type:string;condition:string}> | undefined,
          returnExpr: el.source,
          onCommitReturn: (expr: string) => setElementSource(el.id, expr),
          description: el.props?.description as string | undefined,
          // fn 无 source（CodeBlockEditor 打开即空白）：双击仅聚焦选中（FlowInspector 表单编辑参数）。
          // loop 同 fn：props 为权威（LoopNode 画布静态渲染），双击不开 Monaco（与 SyntaxNode 同机制——
          // 禁用点在数据层，SyntaxNode 组件内部无 onDoubleClick 覆盖）。
          onDoubleClick: el.tag === "fn" || el.tag === "while" || el.tag === "for-in" ? undefined : () => setEditingNodeId(el.id),
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
          ...(CODE_TAGS.has(el.tag) && el.tag !== "code-block" ? {
            id: el.id,
            scope: flowScope ?? undefined,
            // 所属 fn 的 let 变量（wire 拓扑反推）：fn 节点自身也携带（不渲染公式框，无害）
            letVars: letVarsByFn.get(findOwnerFnId(el.id, codeElements, wires) ?? "") ?? [],
            onEditProp: (id: string, prop: string, value: unknown) => setProp(id, prop, value),
          } : {}),
        },
      })),
    [codeElements, selectedNodeIds, setEditingNodeId, setProp, flowScope, letVarsByFn],
  );

  const edges = useMemo(
    () =>
      wires.map((w) => ({
        id: w.id,
        type: "flow",
        source: w.fromId,
        target: w.toId,
        sourceHandle: w.fromPort,
        targetHandle: w.toPort,
      })),
    [wires],
  );

  // ── 自动整理布局：computeLayeredLayout 纯函数 → setElementsAndWires 批量写入（一次 undo 条目）──
  // 节点尺寸优先 RFNode.measured（xyflow v12 渲染后提供），无则走 estimateNodeSize 兜底；
  // 全部代码节点重排（含手动拖过的），wires 原样透传。
  // 注意：必须置于 nodes/edges useMemo 之后——依赖数组在渲染期求值，前置引用 codeElements 会 TDZ。
  const handleAutoLayout = useCallback(() => {
    if (codeElements.length === 0) return;
    const nodeById = new Map(nodes.map((nd) => [nd.id, nd]));
    const sizes = new Map<string, { width: number; height: number }>();
    for (const el of codeElements) {
      const n = nodeById.get(el.id);
      if (n?.measured?.width && n?.measured?.height) {
        sizes.set(el.id, { width: n.measured.width, height: n.measured.height });
      }
    }
    const layout = computeLayeredLayout(codeElements, wires, sizes);
    // 必须以「完整 elements」（含 view 设计树）构建写回数组：本组件顶层 elements 是
    // useCodeElements（仅 codeblock），若用它 map 后全量 setElementsAndWires，
    // view 元素会被覆盖删除（组件 view 块整体丢失，保存报 E0006——Task 12 E2E 实测 CRITICAL）。
    const nextElements = allElements.map((e) =>
      e.nodeType === "codeblock" && CODE_TAGS.has(e.tag) && layout[e.id]
        ? { ...e, position: { ...layout[e.id] } } as ElementData
        : e,
    );
    setElementsAndWires(nextElements, wires);
    // 预览组件（design-preview）不参与布局（不在 store elements）：归位到布局包围盒
    // 右下角外，避免盖住布局后的节点（用户实测：固定 (50,50) 挡左上角节点）。
    let maxX = 0;
    let maxY = 0;
    for (const el of codeElements) {
      const p = layout[el.id];
      if (!p) continue;
      const s = sizes.get(el.id) ?? estimateNodeSize(el.tag, el.props);
      maxX = Math.max(maxX, p.x + s.width);
      maxY = Math.max(maxY, p.y + s.height);
    }
    setPreviewPos({ x: maxX + 40, y: maxY + 40 });
  }, [codeElements, allElements, wires, nodes, setElementsAndWires]);

  // Preview node — shows the design canvas inside the flow when in codeflow mode
  // position 为 state：布局按钮点击时把预览组件归位到布局包围盒右下角外（不参与布局——
  // 它不在 store elements 里；否则固定 (50,50) 会盖住布局后的左上角节点，E2E 用户实测）。
  const [previewPos, setPreviewPos] = useState({ x: 50, y: 50 });
  const previewNode: RFNode | null = useMemo(() => {
    if (!codeFlowElementId) return null;
    return {
      id: "design-preview",
      type: "design-preview",
      position: previewPos,
      selectable: false,
      draggable: true,
      data: { output: previewOutput },
    };
  }, [codeFlowElementId, previewOutput, previewPos]);

  const allNodes = useMemo(
    () => (previewNode ? [...nodes, previewNode] : nodes),
    [nodes, previewNode],
  );

  // ── Editing element (for CodeBlockEditor) ──
  const editingElement = editingNodeId
    ? elements.find((e) => e.id === editingNodeId && e.nodeType === "codeblock")
    : null;

  // ── CodeBlockEditor WASM analysis context ──
  const editorContext = useMemo<CodeBlockEditorContext | null>(() => {
    const activeTab = openTabs.find((t) => t.id === activeTabId);
    if (!activeTab) return null;

    const compDef = componentDefs[activeTab.componentName];
    // Extract fn params from the parent fn element
    const fnParams = getFnParams(editingElement, elements);

    // Resolve store names referenced by this component
    const stores = storeReferences[activeTab.componentName] ?? [];

    // Build code tabs from current code-flow elements + wires
    const codeTabs = buildCodeTabs(codeElements, wires);

    return {
      fnParams,
      componentName: activeTab.componentName,
      omniSourcePath: activeTab.filePath,
      signals: compDef?.signals?.map((s) => s.name) ?? [],
      stores,
      // allSources is intentionally empty here — full project source collection
      // requires async file reads (see useSaveProject.ts:collectProjectSources).
      // The CodeBlockEditor uses pipeline.ts generateFullOmniSource() instead,
      // which synthesizes the full .omni source from designElements + designWires.
      allSources: {},
      designElements: elements,
      designWires: wires,
      codeTabs,
    };
  }, [activeTabId, openTabs, componentDefs, storeReferences, editingElement, elements, wires, codeElements]);

  const handleSaveSource = useCallback(
    (source: string, deps: DependencyInfo | null) => {
      if (!editingNodeId) return;
      setElementSource(editingNodeId, source);
      if (deps) {
        setNodeMeta(editingNodeId, {
          pos: { x: 0, y: 0 },
          dependencies: deps,
        });
      }
      setEditingNodeId(null);
    },
    [editingNodeId, setElementSource, setNodeMeta],
  );

  return (
    <div ref={flowWrapperRef} style={{ width: "100%", height: "100%", display: "flex" }}>
      {/* position: relative 作为布局按钮 absolute 定位的锚点（对齐 Controls bottom-right） */}
      <div style={{ flex: 1, height: "100%", minWidth: 0, position: "relative" }}>
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
          <div style={{ fontSize: 11, opacity: 0.7 }}>右键 / 空格 / 拖出端口 → 添加节点</div>
        </div>
      ) : (
        <ReactFlow
          nodes={allNodes}
          edges={edges}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          fitView
          fitViewOptions={{ maxZoom: 1 }}
          nodesDraggable
          nodesConnectable
          nodesFocusable
          elementsSelectable
          panOnScroll
          onNodeClick={(_event, node) => setSelectedNodeIds([node.id])}
          onPaneClick={() => setSelectedNodeIds([])}
          onNodesChange={handleNodesChange}
          onConnect={onConnect}
          onConnectStart={onConnectStart}
          onConnectEnd={onConnectEnd}
          onPaneContextMenu={handlePaneContextMenu}
          onNodeContextMenu={handleNodeContextMenu}
          // spec 6.4：滚轮缩放/平移画布即关闭搜索菜单
          onMoveStart={() => {
            if (menuOpenRef.current) {
              pendingDragRef.current = null;
              menuOpenRef.current = false;
              setMenuState(null);
            }
          }}
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

      {/* 自动整理布局：一键重排全部代码节点（含手动拖过的），一次 undo 条目；空画布禁用 */}
      <button
        type="button"
        onClick={handleAutoLayout}
        disabled={codeElements.length === 0}
        aria-label="自动整理布局"
        title="自动整理布局（自上而下分层）"
        style={{
          position: "absolute",
          right: 14,
          bottom: 86, // Controls（bottom-right）上方
          zIndex: 5,
          padding: "4px 8px",
          fontSize: 12,
          border: "1px solid var(--ol-border)",
          borderRadius: 6,
          background: "var(--ol-surface)",
          cursor: codeElements.length === 0 ? "not-allowed" : "pointer",
          opacity: codeElements.length === 0 ? 0.5 : 1,
        }}
      >
        ⤢ 整理布局
      </button>

      {/* 搜索菜单：右键 / 空格 / 拖出创建唤出 */}
      {menuState && (
        <NodeSearchMenu
          x={menuState.x}
          y={menuState.y}
          onSelect={handleMenuSelect}
          onClose={() => {
            pendingDragRef.current = null;
            menuOpenRef.current = false;
            setMenuState(null);
          }}
        />
      )}

      {/* CodeBlockEditor — double-click code-block node */}
      {editingElement && editorContext && (
        <CodeBlockEditor
          elementId={editingElement.id}
          initialSource={(editingElement as CodeblockElementData).source || ""}
          context={editorContext}
          onSave={handleSaveSource}
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

// ── Helpers ──

/** Extract function parameter names from the parent fn element of a code block. */
function getFnParams(
  editingElement: ElementData | null | undefined,
  allElements: ElementData[],
): string[] {
  if (!editingElement) return [];
  const codeEl = editingElement as CodeblockElementData;
  if (codeEl.tag === "fn") return [];
  // Find the parent fn node by traversing up the parentId chain
  let current: ElementData | null = editingElement;
  while (current) {
    if (current.parentId) {
      const parent = allElements.find((e) => e.id === current!.parentId);
      if (parent) {
        const parentCode = parent as CodeblockElementData;
        if (parentCode.tag === "fn") {
          try {
            return JSON.parse((parentCode.props.params as string) || "[]");
          } catch {
            return [];
          }
        }
      }
      current = parent ?? null;
    } else {
      break;
    }
  }
  return [];
}

/** Build CodeTab[] from the current code-flow elements and wires. */
function buildCodeTabs(
  codeElements: CodeblockElementData[],
  wires: WireData[],
): CodeTab[] {
  if (codeElements.length === 0 && wires.length === 0) return [];
  return [
    {
      id: "main",
      type: "logic",
      label: "Logic",
      icon: "code",
      graph: { elements: codeElements, wires },
      variables: [],
    },
  ];
}

export function FlowCanvas() {
  return (
    <ReactFlowProvider>
      <FlowCanvasInner />
    </ReactFlowProvider>
  );
}

/** 释放目标判定：空白 pane 释放（非 handle/非节点）→ 触发拖出创建。 */
export function isPaneRelease(event: { target: EventTarget | null }): boolean {
  const t = event.target;
  if (!(t instanceof HTMLElement)) return false;
  return !t.closest(".react-flow__handle") && !t.closest(".react-flow__node");
}
