import { useMemo, useState, useCallback, useEffect, useRef, useId } from "react";
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
import { SyntaxNode } from "./nodes/SyntaxNode";
import { LoopNode } from "./nodes/LoopNode";
import { LifecycleNode } from "./nodes/LifecycleNode";
import { TryNode } from "./nodes/TryNode";
import { GroupBox } from "./nodes/GroupBox";
import { defaultSyntaxProps } from "../../../services/codegen/syntax-nodes";
import { useFlowScope } from "../../../services/codegen/flow-scope-hook";
import { collectLetVarsPerFn, findOwnerFnId } from "../../../services/codegen/fn-topology";
import { CODE_TAGS, TAG_CN, type MenuItem } from "./flowConstants";
import { computeLayeredLayout, buildLayoutInput } from "../../../services/codegen/flow-layout";
import { validateGroupMembers, fnNameOf, GROUP_COLORS, computeGroupBoxes, collectCollapsedMemberIds, buildFlowEdges } from "../../../services/codegen/groups";
import type { GroupBBox } from "../../../services/codegen/groups";
import type { CodeFlowGroup } from "../../../core/types/project-json";
import { NodeSearchMenu } from "./NodeSearchMenu";
import { CodeBlockEditor } from "./CodeBlockEditor";
import type { CodeBlockEditorContext } from "./CodeBlockEditor";
import type { DependencyInfo } from "../../../core/types/dependency-info";
import { FlowInspector } from "./FlowInspector";
import styles from "./FlowCanvas.module.css";
import { useCanvasStore } from "../canvas-store";

// ── 节点 measured 缓存（拖拽防闪烁，2026-08-09）──
// 受控 ReactFlow 下 nodes prop 每次引用变化都会 setNodes → adoptUserNodes：节点对象
// 身份变化时重建内部节点，measured 取自 userNode.measured——我们每帧重建节点对象（拖拽
// 写回必需，身份复用会让 adoptUserNodes checkEquality 命中而内部节点位置不更新 → 拖不动），
// 但只要对象携带 measured，重建后内部节点 measured 保留 → NodeWrapper 不再渲染
// visibility:hidden（节点消失）→ ResizeObserver 不再反复测量 → 无每帧隐藏↔显示循环。
// 修复前实测：拖拽期间组卡 52 次、fn 节点 36 次 visibility 切换 + error015 每帧警告 +
// ResizeObserver loop 错误（~200/拖拽）。
// measured 来源：React Flow 的 ResizeObserver 会发出 dimensions change（updateNodeInternals
// → triggerNodeChanges），handleNodesChange 接收后写入本缓存；节点对象构建时读取。
// 缓存放模块级（组件实例用 useId 键隔离、卸载 effect 清理）：react-compiler 规则禁止
// 渲染期读写 useRef（Cannot access refs during render），模块级 Map 无此限制。
interface NodeMeasured { width: number; height: number; }
const nodeMeasuredCache = new Map<string, Map<string, NodeMeasured>>();
// 模块级读取函数：react-compiler 无法内联分析模块 Map 的别名，直接 `cache.get(id)`
// 会把返回值 taint 成疑似 ref（Cannot access refs during render 误报）；函数调用视为不透明。
function getCachedMeasured(instanceId: string, id: string): NodeMeasured | undefined {
  return nodeMeasuredCache.get(instanceId)?.get(id);
}

const nodeTypes = {
  "fn": FnNode,
  "action": FnNode,
  "code-block": CodeBlockNode,
  "condition": ConditionNode,
  "return": ReturnNode,
  "let": SyntaxNode,
  "assign": SyntaxNode,
  "store-call": SyntaxNode,
  "call": SyntaxNode,
  "throw": SyntaxNode,
  "await": SyntaxNode,
  "while": LoopNode,
  "for-in": LoopNode,
  "try": TryNode,
  "lifecycle": LifecycleNode,
  "group": GroupBox,
};

/** 连线用 ReactFlow 原生默认 edge（bezier 拓扑曲线）——原生渲染与跟随（移动/滑动自动重算），
 *  不自定义 edge（FlowEdge 已删除：DOM 测量开销大）。用户 2026-08-05 实测拍板：放弃
 *  smoothstep 正交折线（此前 ADR-064 决策 2），换回原生拓扑曲线，接受曲线自然交叉
 *  （"让连线有交点"）。主干 flow wire 因 flow-in/out handle 钉节点左缘内侧（node-base.css）
 *  + 主干左缘对齐（flow-layout.ts）而近似垂直直线（两端同 x → 曲线退化为竖直线）；分支
 *  wire（br0/br1 右缘 → 目标左缘）呈 S 形扫过列间空隙，多分支自然交叉。 */

function FlowCanvasInner() {
  const elements = useCodeElements();
  // 完整元素（含 view 设计树）：布局写回必须基于全量（useCodeElements 仅 codeblock——
  // 见 handleAutoLayout 注释，Task 12 E2E CRITICAL）。
  const allElements = usePoiesisStore((s) => s.elements);
  // 分组订阅（Task 4）：组操作（改色/折叠/删除）只经 store 写入，无组件订阅则
  // 视觉不刷新（Zustand 引用比较）；组节点生成与 bbox 测量 effect 均消费此值。
  const codeFlowGroups = usePoiesisStore((s) => s.codeFlowGroups);
  const wires = useWires();
  const addElement = usePoiesisStore((s) => s.addElement);
  const addWire = usePoiesisStore((s) => s.addWire);
  const { screenToFlowPosition, setViewport } = useReactFlow();
  const flowWrapperRef = useRef<HTMLDivElement>(null);

  // ── 视口同步：单一真相 = useCanvasStore（DOM 画布），ReactFlow 跟随；onMove 回写 ──
  const { zoom: canvasZoom, panX, panY } = useCanvasStore();
  useEffect(() => {
    setViewport({ x: panX, y: panY, zoom: canvasZoom });
  }, [canvasZoom, panX, panY, setViewport]);

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
    (item: MenuItem) => {
      if (!menuState) return;
      const tag = item.tag;
      const id = uid(tag);
      const jitter = Math.random() * 40 - 20;
      let props: Record<string, unknown> = {};
      if (tag === "fn") { props = { name: "newFn", params: "" }; }
      if (tag === "condition") { props = { branches: [{ id: "br0", type: "if", condition: "" }] }; }
      if (item.tag === "try") {
        props = { branches: [{ id: "br0", type: "try" }] };
      }
      if (["let", "assign", "store-call", "call", "throw", "await", "while", "for-in"].includes(tag)) { props = { ...defaultSyntaxProps(tag) }; }
      if (tag === "lifecycle") {
        // 单例约束：每组件每种 hookType 仅一个（learn-omniview 实证）——检查 store.elements
        // 中已存在节点（覆盖已保存与画布新建两种来源）
        const hookType = (item.props?.hookType as "mount" | "effect" | "unmount") ?? "mount";
        const hasSame = usePoiesisStore.getState().elements.some(
          (e) => e.nodeType === "codeblock" && e.tag === "lifecycle" && (e.props?.hookType as string) === hookType,
        );
        if (hasSame) return; // 守卫命中：菜单关闭由 NodeSearchMenu select() 统一处理
        props = { hookType, deps: "" };
      }
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
  // 新建组的命名态（Task 4 GroupBox 消费：创建分组后进入改名态）。
  const [editingGroupId, setEditingGroupId] = useState<string | null>(null);

  // 组几何缓存：groupId → 包围盒（展开态实时重算；折叠态沿用最后一次展开值——成员隐藏无 DOM）
  const [groupBoxes, setGroupBoxes] = useState<Map<string, GroupBBox>>(new Map());
  const lastGroupBoxesRef = useRef<Map<string, GroupBBox>>(new Map());
  // 折叠组位置（Task 7 布局落点消费；Task 4 仅 getter——setter 由 Task 7 写入，弃名防 TS6133）
  const [collapsedGroupPositions, setCollapsedGroupPositions] = useState<Record<string, { x: number; y: number }>>({});

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
  // dimensions 变更（React Flow RO 测量结果）写入 measured 缓存：节点对象构建时携带
  //（见 nodes useMemo），adoptUserNodes 重建内部节点时 measured 不丢失 → 拖拽不闪烁。
  const cacheInstanceId = useId();
  useEffect(() => () => {
    nodeMeasuredCache.delete(cacheInstanceId);
  }, [cacheInstanceId]);

  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      for (const change of changes) {
        if (change.type === "dimensions" && change.dimensions) {
          let cache = nodeMeasuredCache.get(cacheInstanceId);
          if (!cache) { cache = new Map(); nodeMeasuredCache.set(cacheInstanceId, cache); }
          cache.set(change.id, { width: change.dimensions.width, height: change.dimensions.height });
        }
      }
      // 组 id 集合（折叠态组卡可拖，spec §4）：组不在 store elements 中，
      // position 写回 collapsedGroupPositions（组节点渲染优先取该落点）；
      // 其余节点照旧回写 store（setElementPosition 无 undo 条目，每帧提交不污染撤销栈）。
      const groupIds = new Set(Object.values(codeFlowGroups ?? {}).flat().map((g) => g.id));
      for (const change of changes) {
        if (change.type === "position" && change.position) {
          if (groupIds.has(change.id)) {
            setCollapsedGroupPositions((prev) => ({ ...prev, [change.id]: change.position! }));
          } else {
            setElementPosition(change.id, change.position);
          }
        }
      }
    },
    [setElementPosition, codeFlowGroups, cacheInstanceId],
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

  // ── 组包围盒测量（Task 4）──
  // 受控模式下 RFNode.measured 恒缺（同布局测量），展开态组框尺寸须 DOM 实测成员节点
  // （rect/zoom 换算画布单位）；折叠态沿用最后一次展开值（lastGroupBoxesRef 缓存——
  // 折叠后成员几何不再变化，卡位置稳定依赖此缓存）。
  // 测量推迟到 requestAnimationFrame（真机 error009）：展开/折叠切换经 store 更新后，
  // ReactFlow 内部节点渲染异步完成（zustand store 同步 + commit 后渲染），useEffect
  // 同步测量拿不到刚恢复的成员节点 DOM → sizes 空 → bbox 永不写入 → 展开态组框被跳过。
  // rAF 后 ReactFlow 渲染完成，测量稳定。cleanup 取消未执行的帧，避免陈旧闭包写回。
  useEffect(() => {
    const allGroups = Object.values(codeFlowGroups ?? {}).flat();
    if (allGroups.length === 0) {
      // 仅在确有缓存/状态要清时复位——空态下每次都 new Map 会给依赖不稳定
      // 的环境（测试 mock 每次渲染返回新 elements 引用）制造无限渲染循环
      if (lastGroupBoxesRef.current.size > 0 || groupBoxes.size > 0) {
        lastGroupBoxesRef.current = new Map();
        setGroupBoxes(new Map());
      }
      return;
    }
    const measure = () => {
      const vpEl = document.querySelector(".react-flow__viewport");
      const vpMatch = vpEl ? getComputedStyle(vpEl).transform.match(/matrix\(([^)]+)\)/) : null;
      const zoom = vpMatch ? Number(vpMatch[1].split(",")[0]) || 1 : 1;
      const sizes = new Map<string, { width: number; height: number }>();
      const activeGroups = allGroups.filter((g) => !g.collapsed);
      for (const el of codeElements) {
        const dom = document.querySelector(`[data-id="${el.id}"]`);
        if (!dom) continue;
        const r = dom.getBoundingClientRect();
        if (r.width > 0 && r.height > 0) sizes.set(el.id, { width: r.width / zoom, height: r.height / zoom });
      }
      const next = computeGroupBoxes(activeGroups, codeElements, sizes);
      if (next.size > 0) {
        lastGroupBoxesRef.current = new Map([...lastGroupBoxesRef.current, ...next]);
        setGroupBoxes(lastGroupBoxesRef.current);
      }
    };
    const rafId = typeof requestAnimationFrame === "function" ? requestAnimationFrame(measure) : 0;
    return () => {
      if (rafId && typeof cancelAnimationFrame === "function") cancelAnimationFrame(rafId);
    };
  }, [elements, codeElements, codeFlowGroups]);

  // 按 wire 拓扑推断每个 fn 的 let 变量（多 fn 隔离）：语法节点公式框候选数据源
  const letVarsByFn = useMemo(() => collectLetVarsPerFn(codeElements, wires), [codeElements, wires]);

  // ── Group ops（Task 4）──
  // 组写入一律走 store.setCodeFlowGroups：patch 用 map-replace（不可变），
  // 订阅 codeFlowGroups 保证改色/折叠/删除视觉即时刷新（Zustand 引用比较）。
  // groupFnName 只负责「创建」时的键定位；patch/remove 按 g.id 全键扫描（终审 I-1：
  // removeElement 级联删除不清理 codeFlowGroups，成员断链后 groupFnName 退化为
  // "unnamed"，若用它读写键将全部命中不存在的键 → 组卡操作静默失效）。
  // 注意：groupFnName 必须位于 handleCreateGroup 之前——deps 数组在渲染期求值，前置引用会触发 TDZ。
  const groupFnName = useCallback(
    (g: CodeFlowGroup) => {
      const ownerFnId = findOwnerFnId(g.memberIds[0] ?? "", codeElements, wires);
      return ownerFnId ? fnNameOf(ownerFnId, codeElements) : "unnamed";
    },
    [codeElements, wires],
  );

  // ── Create group ──
  // 校验（同 fn 连通域、不含容器节点）通过后写入 store.codeFlowGroups，并进入命名态（Task 4 消费）。
  const handleCreateGroup = useCallback(() => {
    const sel = selectedNodeIds;
    if (sel.length < 2) return;
    const v = validateGroupMembers(sel, codeElements, wires);
    if (!v.ownerFnId || v.error) {
      window.alert(v.error ?? "无法创建分组");
      setNodeMenu(null);
      return;
    }
    const group: CodeFlowGroup = {
      id: uid("group"),
      name: "新分组",
      memberIds: v.valid,
      color: GROUP_COLORS[0],
      collapsed: false,
    };
    const fnName = groupFnName(group);
    usePoiesisStore.getState().setCodeFlowGroups(fnName, [...(usePoiesisStore.getState().codeFlowGroups[fnName] ?? []), group]);
    setEditingGroupId(group.id);
    setNodeMenu(null);
  }, [selectedNodeIds, groupFnName]);

  const patchGroup = useCallback(
    (g: CodeFlowGroup, patch: Partial<CodeFlowGroup>) => {
      // 按 g.id 全键扫描定位（终审 I-1）：组可能停留在任意原键（成员断链后
      // groupFnName 不可信）——遍历所有键找含该组 id 的键再操作；找不到（组已被清理）静默跳过。
      const groups = usePoiesisStore.getState().codeFlowGroups;
      for (const fnName of Object.keys(groups)) {
        if (groups[fnName].some((x) => x.id === g.id)) {
          usePoiesisStore.getState().setCodeFlowGroups(
            fnName,
            groups[fnName].map((x) => (x.id === g.id ? { ...x, ...patch } : x)),
          );
          return;
        }
      }
    },
    [],
  );

  const removeGroup = useCallback(
    (g: CodeFlowGroup) => {
      // 同上：全键扫描定位，不依赖 groupFnName。
      const groups = usePoiesisStore.getState().codeFlowGroups;
      for (const fnName of Object.keys(groups)) {
        if (groups[fnName].some((x) => x.id === g.id)) {
          usePoiesisStore.getState().setCodeFlowGroups(
            fnName,
            groups[fnName].filter((x) => x.id !== g.id),
          );
          break;
        }
      }
    },
    [],
  );

  // 删组后折叠态几何缓存同步移除（防缓存残留引用）。原在 removeGroup 内同步清理——
  // react-compiler 规则禁止渲染期传递访问 ref 的回调（Cannot access refs during render），
  // 移入 effect（effect 内访问 ref 合法，语义等价：组消失后下一轮渲染清理）。
  useEffect(() => {
    const liveIds = new Set(Object.values(codeFlowGroups ?? {}).flat().map((x) => x.id));
    for (const id of [...lastGroupBoxesRef.current.keys()]) {
      if (!liveIds.has(id)) {
        const next = new Map(lastGroupBoxesRef.current);
        next.delete(id);
        lastGroupBoxesRef.current = next;
      }
    }
  }, [codeFlowGroups]);

  const nodes: RFNode[] = useMemo(() => {
    // ── 折叠组：成员节点渲染层过滤（Task 5）──
    // 数据防线：只过滤渲染输出，store 元素不动——保存/代码生成永远用全量。
    const collapsedMemberIds = collectCollapsedMemberIds(Object.values(codeFlowGroups ?? {}).flat());
    const visibleElements = codeElements.filter((e) => !collapsedMemberIds.has(e.id));
    const out: RFNode[] = [];
    for (const el of visibleElements) {
      const data = {
        label: TAG_CN[el.tag] || el.tag,
        tag: el.tag,
        props: el.props,
        source: el.source,
        fnName: el.props?.name as string,
        params: el.props?.params as string,
        branches: el.props?.branches as Array<{id:string;type:string;condition?:string;catchVar?:string}> | undefined,
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
        onEditCatchVar: (branchId: string, catchVar: string) => {
          const currentBranches = [...(el.props?.branches as Array<{id:string;type:string;catchVar?:string}> || [])];
          const idx = currentBranches.findIndex((b) => b.id === branchId);
          if (idx >= 0) {
            currentBranches[idx] = { ...currentBranches[idx], catchVar };
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
      };
      // 每帧重建节点对象（拖拽位置写回必需——对象身份复用会让 adoptUserNodes
      // checkEquality 命中且内部节点位置不更新 → 拖不动）；measured 从缓存携带
      //（handleNodesChange 接收 dimensions 变更写入）→ 重建后内部节点 measured 保留
      // → NodeWrapper 不渲染 visibility:hidden → 拖拽无每帧闪烁。
      const node: RFNode = {
        id: el.id,
        type: el.tag,
        position: el.position ?? { x: 0, y: 0 },
        measured: getCachedMeasured(cacheInstanceId, el.id),
        selected: selectedNodeIds.includes(el.id),
        className: selectedNodeIds.includes(el.id) ? "flow-node-selected" : "",
        data,
      };
      out.push(node);
    }
    return out;
    },
    [codeElements, selectedNodeIds, setEditingNodeId, setProp, flowScope, letVarsByFn, codeFlowGroups, wires, cacheInstanceId],
  );

  // ── Group nodes（Task 4）──
  // 展开态 = bbox 尺寸占位节点（GroupBox 内部绝对定位铺满）+ 标题栏交互；折叠态 = 紧凑组卡，
  // 位置优先 Task 7 的 collapsedGroupPositions，其次沿用最后一次展开 bbox（缓存），
  // 再无则退化到成员 position 原点（首次加载即折叠 / 无 DOM 测量环境）。
  // 数组首位渲染：组框在成员节点下层（z 序背景层），ReactFlow 按数组序覆盖。
  // 成员节点过滤在下方 nodes useMemo 完成（Task 5 渲染层过滤，store 元素不动）。
  const groupNodes: RFNode[] = useMemo(() => {
    const out: RFNode[] = [];
    for (const g of Object.values(codeFlowGroups ?? {}).flat()) {
    const bbox = groupBoxes.get(g.id);
    let pos: { x: number; y: number } | undefined;
    if (g.collapsed) {
      pos = collapsedGroupPositions[g.id] ?? (bbox ? { x: bbox.x, y: bbox.y } : undefined);
      // 无缓存位置（首次加载即折叠 / jsdom 无 DOM 可测）：退化到成员 position 原点，
      // 卡落在成员所在处；展开测量后即被 bbox 缓存取代。
      if (!pos) {
        let minX = Infinity, minY = Infinity;
        for (const mid of g.memberIds) {
          const m = codeElements.find((e) => e.id === mid);
          if (m?.position) {
            minX = Math.min(minX, m.position.x);
            minY = Math.min(minY, m.position.y);
          }
        }
        if (Number.isFinite(minX)) pos = { x: minX, y: minY };
      }
    } else if (bbox) {
      pos = { x: bbox.x, y: bbox.y };
    }
    // bbox 缺失（展开态测量未完成，rAF 前）→ pos undefined → 跳过渲染（组框无可靠落点）；
    // style 仅在 bbox 存在时取宽高（守卫：展开态无 bbox 不得解引用）。
    // 组节点同样每帧重建 + measured 携带（拖拽/折叠切换不闪烁，机制同 nodes useMemo）。
    if (!pos) continue;
    const style = g.collapsed ? undefined : bbox ? { width: bbox.width, height: bbox.height } : undefined;
    out.push({
      id: g.id,
      type: "group",
      position: pos,
      style,
      measured: getCachedMeasured(cacheInstanceId, g.id),
      // 折叠态组卡可整体拖动（spec §4：折叠态改拖组卡）；展开态框不可拖（跟随成员，拖成员即可）
      draggable: g.collapsed,
      selectable: false,
      focusable: false,
      className: "codeflow-group",
      data: {
        group: g,
        editing: editingGroupId === g.id,
        onRename: (name: string) => {
          patchGroup(g, { name });
          setEditingGroupId(null);
        },
        onColorChange: (color: string) => patchGroup(g, { color }),
        onToggleCollapsed: () => patchGroup(g, { collapsed: !g.collapsed }),
        onDelete: () => removeGroup(g),
      },
    });
    }
    return out;
  }, [codeFlowGroups, groupBoxes, collapsedGroupPositions, editingGroupId, patchGroup, removeGroup, groupFnName, codeElements, cacheInstanceId]);
  const allNodes = [...groupNodes, ...nodes];

  const edges = useMemo(
    () =>
      buildFlowEdges(wires, Object.values(codeFlowGroups ?? {}).flat()),
    // 原生默认 edge（bezier 拓扑曲线）——显式声明防"注释与实现矛盾"回归（2026-08-05
    // 曾注释声称 smoothstep 但未配置 → 实际 bezier 却无人知晓）；用户拍板要曲线交叉；
    // 折叠组连线由 buildFlowEdges 重路由：组内隐藏、进出线接组边界 flow-in/flow-out。
    [wires, codeFlowGroups],
  );

  // ── 自动整理布局：computeLayeredLayout 纯函数 → setElementsAndWires 批量写入（一次 undo 条目）──
  // 节点尺寸优先 RFNode.measured（xyflow v12 渲染后提供），无则走 estimateNodeSize 兜底；
  // 全部代码节点重排（含手动拖过的），wires 原样透传。
  // 注意：必须置于 nodes/edges useMemo 之后——依赖数组在渲染期求值，前置引用 codeElements 会 TDZ。
  const handleAutoLayout = useCallback(() => {
    if (codeElements.length === 0) return;
    // 数据防线（历史：E2E 实测 CRITICAL——布局后 view 元素丢失、保存 E0006/覆盖
    // .omni view 块）：写回必须基于「完整 elements」（allElements，含 view 设计树）
    // 构建——下方 map 对非 codeblock 元素原样保留（nextElements 全量 = allElements
    // 全量），只替换 codeblock 位置，任何状态下都不丢元素。曾用「view 设计树缺失时
    // 早退」守卫（2d14f9ff），但代码流视图下 store 无 view 元素是合法状态（如从
    // 架构页进入），守卫把布局按钮静默废掉（用户实测按钮无反应）——守卫删除，
    // 数据安全由 allElements 全量 map 保证。
    // 折叠组布局配合（Task 7）：折叠成员剔除、组伪元素 `group:<gid>` 占位参与分层、边界
    // 连线重路由到组伪元素；折叠卡落点写回 collapsedGroupPositions（组节点渲染优先用该落点）。
    // 组数据用组件订阅值 codeFlowGroups（须入 deps 防陈旧闭包）——不能用 getState() 现读：
    // FlowCanvasLayout.test.tsx 的 store mock 仅实现 hook 形式（无 .getState），现读会炸测试。
    const groups = Object.values(codeFlowGroups ?? {}).flat();
    const collapsedMemberIds = collectCollapsedMemberIds(groups);
    const { elements: layoutInput, wires: layoutWires } = buildLayoutInput(codeElements, wires, groups);
    const nodeById = new Map(nodes.map((nd) => [nd.id, nd]));
    const sizes = new Map<string, { width: number; height: number }>();
    // 受控模式下 RFNode.measured 恒缺（xyflow 内部状态不回填受控 nodes）——布局尺寸须
    // DOM 实测（节点已渲染，getBoundingClientRect 真实值；除以 zoom 换算画布单位）：
    // 估算 200 vs 实测 310.67 偏差会让右缘对齐在渲染后错位 → 主干 handle 列错开 → 斜线
    //（用户实测"连线没连上"）。measured 先验（万一非受控填充），DOM 其次，估算兜底。
    const vpEl = document.querySelector(".react-flow__viewport");
    const vpMatch = vpEl ? getComputedStyle(vpEl).transform.match(/matrix\(([^)]+)\)/) : null;
    const zoom = vpMatch ? Number(vpMatch[1].split(",")[0]) || 1 : 1;
    for (const el of codeElements) {
      const n = nodeById.get(el.id);
      let w = n?.measured?.width;
      let h = n?.measured?.height;
      if (!w || !h) {
        const dom = document.querySelector(`[data-id="${el.id}"]`);
        if (dom) {
          const r = dom.getBoundingClientRect();
          w = r.width / zoom;
          h = r.height / zoom;
        }
      }
      if (w && h) sizes.set(el.id, { width: w, height: h });
    }
    const layout = computeLayeredLayout(layoutInput, layoutWires, sizes);
    // 折叠卡落点：组伪元素在分层中的位置（成员原流程位置）；仅折叠组收集——
    // 组节点渲染（GroupNode）优先用 collapsedGroupPositions[g.id] 落点（Task 4 约定）。
    const groupPos: Record<string, { x: number; y: number }> = {};
    for (const g of groups) {
      if (g.collapsed && layout["group:" + g.id]) groupPos[g.id] = layout["group:" + g.id];
    }
    setCollapsedGroupPositions(groupPos);
    // 必须以「完整 elements」（含 view 设计树）构建写回数组：本组件顶层 elements 是
    // useCodeElements（仅 codeblock），若用它 map 后全量 setElementsAndWires，
    // view 元素会被覆盖删除（组件 view 块整体丢失，保存报 E0006——Task 12 E2E 实测 CRITICAL）。
    const nextElements = allElements.map((e) =>
      // !collapsedMemberIds.has(e.id)：折叠成员不写回坐标——其位置被折叠卡落点覆盖，
      // 展开时成员坐标仍保持折叠前的原位（不随布局漂移，展开不跳变）。
      e.nodeType === "codeblock" && CODE_TAGS.has(e.tag) && !collapsedMemberIds.has(e.id) && layout[e.id]
        ? { ...e, position: { ...layout[e.id] } } as ElementData
        : e,
    );
    setElementsAndWires(nextElements, wires);
  }, [codeElements, allElements, wires, nodes, setElementsAndWires, codeFlowGroups]);

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
          <div>右键画布添加节点</div>
          <div style={{ fontSize: 11, opacity: 0.7 }}>右键 / 空格 / 拖出端口 → 添加节点</div>
        </div>
      ) : (
        <ReactFlow
          nodes={allNodes}
          edges={edges}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ maxZoom: 1 }}
          minZoom={0.1}
          maxZoom={5}
          nodesDraggable
          nodesConnectable
          nodesFocusable
          elementsSelectable
          panOnScroll
          onNodeClick={(_event, node) => {
            if (_event.shiftKey) {
              // Shift 点击：数组中 toggle（多选/取消），不打断既有选中
              setSelectedNodeIds((prev) => prev.includes(node.id) ? prev.filter((id) => id !== node.id) : [...prev, node.id]);
            } else {
              setSelectedNodeIds([node.id]);
            }
          }}
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
          onMove={(_e, vp) => {
            const cur = useCanvasStore.getState();
            // setViewport 编程式同步也会触发 onMove——Object.is 守卫防 ping-pong
            // 注意：setZoom 钳制 0.1-5.0 与 ReactFlow zoom 范围须一致，否则钳制后值不等 → 每帧回写
            if (vp.zoom !== cur.zoom || vp.x !== cur.panX || vp.y !== cur.panY) {
              cur.setZoom(vp.zoom);
              cur.setPan(vp.x, vp.y);
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
          {/* 组成分组：右键节点在选中集内且多选 ≥2 时显示（同 fn 校验在 handleCreateGroup 内） */}
          {selectedNodeIds.includes(nodeMenu.nodeId) && selectedNodeIds.length >= 2 && (
            <button
              type="button"
              onClick={() => handleCreateGroup()}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                width: "100%",
                padding: "6px 12px",
                border: "none",
                cursor: "pointer",
                fontSize: 12,
                color: "#6366f1",
                textAlign: "left" as const,
                fontFamily: "inherit",
              }}
            >
              📦 组成分组
            </button>
          )}
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
