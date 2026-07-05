import { useMemo, useState, useCallback, useEffect, useRef } from "react";
import { ReactFlow, ReactFlowProvider, useReactFlow } from "@xyflow/react";
import type { Node as RFNode } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../../store";
import { uid } from "../../../core/utils/id";
import type { ElementData } from "../../../core/types/element";
import { FunctionHeaderNode } from "./nodes/FunctionHeaderNode";
import { SkeletonNode } from "./nodes/SkeletonNode";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { ReturnNode } from "./nodes/ReturnNode";

const nodeTypes = {
  "function-header": FunctionHeaderNode,
  skeleton: SkeletonNode,
  "code-block": CodeBlockNode,
  return: ReturnNode,
};

const CODE_TAGS = new Set([
  // Control flow
  "fn", "if", "else", "for", "while", "return",
  // Variables
  "let", "const", "literal", "getVariable", "setVariable",
  // Arithmetic
  "add", "sub", "mul", "div",
  // Comparison & logic
  "compare", "and", "or", "not",
  // String
  "concat",
  // Reactive
  "signal", "computed", "store",
  // Functions
  "customFn",
]);

interface MenuItem {
  label: string;
  tag: string;
  shortcut?: string;
}

interface MenuSection {
  label: string;
  items: MenuItem[];
}

const MENU_SECTIONS: MenuSection[] = [
  {
    label: "控制流",
    items: [
      { label: "函数定义", tag: "fn", shortcut: "fn" },
      { label: "条件判断 (if)", tag: "if", shortcut: "if" },
      { label: "否则分支 (else)", tag: "else", shortcut: "else" },
      { label: "计数循环 (for)", tag: "for", shortcut: "for" },
      { label: "条件循环 (while)", tag: "while", shortcut: "while" },
      { label: "返回 (return)", tag: "return", shortcut: "ret" },
    ],
  },
  {
    label: "变量",
    items: [
      { label: "变量声明 (let)", tag: "let", shortcut: "let" },
      { label: "常量声明 (const)", tag: "const", shortcut: "const" },
      { label: "读取变量 (get)", tag: "getVariable", shortcut: "get" },
      { label: "赋值 (set)", tag: "setVariable", shortcut: "set" },
    ],
  },
  {
    label: "运算",
    items: [
      { label: "加法 (+)", tag: "add", shortcut: "+" },
      { label: "减法 (−)", tag: "sub", shortcut: "−" },
      { label: "乘法 (×)", tag: "mul", shortcut: "×" },
      { label: "除法 (÷)", tag: "div", shortcut: "÷" },
    ],
  },
  {
    label: "逻辑",
    items: [
      { label: "比较 (==, >, <)", tag: "compare", shortcut: "cmp" },
      { label: "与 (&&)", tag: "and", shortcut: "&&" },
      { label: "或 (||)", tag: "or", shortcut: "||" },
      { label: "非 (!)", tag: "not", shortcut: "!" },
    ],
  },
  {
    label: "值 & 字符串",
    items: [
      { label: "字面量 (literal)", tag: "literal", shortcut: "lit" },
      { label: "字符串拼接 (concat)", tag: "concat", shortcut: "++" },
    ],
  },
  {
    label: "响应式",
    items: [
      { label: "信号 (signal)", tag: "signal", shortcut: "sig" },
      { label: "计算属性 (computed)", tag: "computed", shortcut: "comp" },
      { label: "Store 引用", tag: "store", shortcut: "str" },
    ],
  },
  {
    label: "函数调用",
    items: [
      { label: "自定义函数 (customFn)", tag: "customFn", shortcut: "fn!" },
    ],
  },
];

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
      // Offset each added node slightly so they don't stack exactly
      const jitter = Math.random() * 40 - 20;
      const props: Record<string, string> = {};
      if (tag === "fn" || tag === "customFn") {
        props.name = `new${tag === "fn" ? "Fn" : "Call"}`;
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
      };
      addElement(newEl);
      setContextMenu(null);
    },
    [contextMenu, addElement],
  );

  // Filter to code-relevant elements by tag
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
            : el.tag === "if" ||
                el.tag === "else" ||
                el.tag === "for" ||
                el.tag === "while"
              ? "skeleton"
              : el.tag === "return"
                ? "return"
                : "code-block",
        position: el.position || { x: 0, y: 0 },
        data: {
          label: el.tag,
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
          onPaneContextMenu={handlePaneContextMenu}
        />
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
    </div>
  );
}

// ── Context menu with collapsible sections and smart positioning ──

const MENU_WIDTH = 210;
const ITEM_HEIGHT = 30; // approximate
const HEADER_HEIGHT = 26;

interface ContextMenuProps {
  x: number;
  y: number;
  menuRef: React.RefObject<HTMLDivElement | null>;
  collapsedSections: Set<number>;
  onToggleSection: (i: number) => void;
  onSelectNode: (tag: string) => void;
  onClose: () => void;
}

function ContextMenu({
  x,
  y,
  menuRef,
  collapsedSections,
  onToggleSection,
  onSelectNode,
  onClose,
}: ContextMenuProps) {
  // Compute total visible items for height estimation
  let visibleCount = 0;
  for (let i = 0; i < MENU_SECTIONS.length; i++) {
    visibleCount++; // header
    if (!collapsedSections.has(i)) {
      visibleCount += MENU_SECTIONS[i].items.length;
    }
  }
  const estimatedHeight = visibleCount * ITEM_HEIGHT + MENU_SECTIONS.length * 4; // +separator gaps
  const maxHeight = Math.min(estimatedHeight, 400);

  const vwW = window.innerWidth;
  const vwH = window.innerHeight;

  // Horizontal: flip left if overflow right
  let left = x;
  if (x + MENU_WIDTH > vwW) {
    left = Math.max(0, x - MENU_WIDTH);
  }
  // Vertical: flip up if overflow bottom (scroll handles remaining overflow)
  let top = y;
  if (y + maxHeight > vwH) {
    top = Math.max(0, y - maxHeight);
  }

  return (
    <div
      ref={menuRef}
      style={{
        position: "fixed",
        top,
        left,
        zIndex: 10000,
        background: "#fff",
        borderRadius: 8,
        boxShadow: "0 4px 20px rgba(0,0,0,0.15)",
        minWidth: MENU_WIDTH,
        maxWidth: MENU_WIDTH,
        maxHeight,
        overflowY: "auto",
        fontSize: 12,
        padding: "2px 0",
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {MENU_SECTIONS.map((section, i) => {
        const collapsed = collapsedSections.has(i);
        return (
          <div key={section.label}>
            {i > 0 && <div style={{ height: 1, background: "#e2e8f0", margin: "2px 0" }} />}
            <button
              onClick={() => onToggleSection(i)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                width: "100%",
                padding: "4px 10px",
                border: "none",
                background: "transparent",
                cursor: "pointer",
                fontSize: 10,
                fontWeight: 600,
                color: "#64748b",
                textTransform: "uppercase" as const,
                letterSpacing: "0.5px",
                fontFamily: "inherit",
              }}
            >
              <span style={{ fontSize: 9, width: 10, textAlign: "center" as const }}>
                {collapsed ? "▶" : "▼"}
              </span>
              <span>{section.label}</span>
              <span style={{ marginLeft: "auto", fontSize: 10, color: "#94a3b8" }}>
                {collapsed ? section.items.length : ""}
              </span>
            </button>
            {!collapsed &&
              section.items.map((item) => (
                <button
                  key={item.tag}
                  onClick={() => { onSelectNode(item.tag); onClose(); }}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    width: "100%",
                    padding: "5px 12px 5px 26px",
                    border: "none",
                    background: "transparent",
                    cursor: "pointer",
                    fontSize: 12,
                    color: "#1e293b",
                    textAlign: "left" as const,
                    fontFamily: "inherit",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "#f1f5f9")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                >
                  <span>{item.label}</span>
                  {item.shortcut && (
                    <span style={{ color: "#94a3b8", fontSize: 10, fontFamily: "monospace" }}>
                      {item.shortcut}
                    </span>
                  )}
                </button>
              ))}
          </div>
        );
      })}
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
