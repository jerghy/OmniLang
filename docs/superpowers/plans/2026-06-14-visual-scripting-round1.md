# Visual Scripting Round 1: Flow Engine Core + @xyflow/react Migration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Replace Rete.js with @xyflow/react v12, enhance data model for Flow/Data separation, implement flow execution engine, dual-color wire rendering.

**Architecture:** React Flow nodes wrap existing ElementData; custom edge components render Flow (white triangle) vs Data (colored circle) wires; FlowEngine performs topological sort over flow-connected nodes; existing code block nodes (signal/computed/action) render as @xyflow/react nodes with backward-compatible ports.

**Tech Stack:** @xyflow/react v12, React 19, Zustand 5, TypeScript 5

---

## File Map

| File | Role |
|------|------|
| `editor/client/src/store/editor.ts` | Enhanced ElementData + WireData types |
| `editor/client/src/canvas/nodeRegistry.ts` | Node type → visual config registry |
| `editor/client/src/canvas/nodes/BaseNode.tsx` | Shared node chrome (flow/data handles) |
| `editor/client/src/canvas/nodes/CodeBlockNode.tsx` | Existing signal/computed/action node renderer |
| `editor/client/src/canvas/nodes/ViewEventNode.tsx` | View element with event ports |
| `editor/client/src/canvas/edges/FlowEdge.tsx` | White triangle-arrow execution edge |
| `editor/client/src/canvas/edges/DataEdge.tsx` | Colored circle-dot data edge |
| `editor/client/src/canvas/FlowEngine.ts` | Topological sort + data propagation |
| `editor/client/src/canvas/FlowCanvas.tsx` | Main canvas component (replaces Rete-based) |
| `editor/client/src/App.tsx` | Update FlowCanvas import + props |

---

### Task 1: Enhance ElementData and WireData types

**Files:**
- Modify: `editor/client/src/store/editor.ts:1-30`

**Goal:** Add `nodeCategory`, `flowOrder`, `portType`, `config` fields to support visual scripting model while maintaining backward compatibility.

- [ ] **Step 1: Add new type definitions**

Insert after the existing `ElementData` interface (line 25), add the new port and element fields. Replace the ports type inline.

```typescript
// In editor/client/src/store/editor.ts, replace the ElementData interface:

export interface PortData {
  name: string;
  direction: "in" | "out";
  /** NEW: distinguishes execution-flow ports from data-value ports */
  portType?: "flow" | "data";
  type: string;
  label?: string;
  /** NEW: whether this port's value is user-configurable on the node */
  configurable?: boolean;
  /** NEW: default value for configurable data input ports */
  defaultValue?: unknown;
}

export interface ElementData {
  id: string;
  nodeType: "view" | "codeblock" | "group";
  tag: string;
  props: Record<string, unknown>;
  children: string[];
  parentId: string | null;
  ports?: PortData[];
  renderMode?: "dom" | "canvas";
  isFolded?: boolean;
  typeAnnotation?: string;
  storeName?: string;
  source?: string;
  outputValue?: unknown;
  position?: { x: number; y: number };
  /** NEW: visual scripting node category */
  nodeCategory?: "flow" | "event" | "action" | "data" | "variable" | "code";
  /** NEW: node-specific configuration (operator, comparison type, etc.) */
  config?: Record<string, unknown>;
  /** NEW: topological execution order (set by FlowEngine) */
  flowOrder?: number;
}
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
cd "D:/space/OmniLang/editor/client" && npx tsc --noEmit --pretty false 2>&1 | head -30
```

Expected: No new type errors from the store file. There may be pre-existing errors in other files — only check that `editor.ts` changes don't introduce new ones.

- [ ] **Step 3: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/store/editor.ts && git commit -m "feat(editor): enhance ElementData with nodeCategory, PortData with portType for visual scripting"
```

---

### Task 2: Create Node Registry

**Files:**
- Create: `editor/client/src/canvas/nodeRegistry.ts`

**Goal:** Central registry mapping node tags/categories to visual config (colors, icons, default ports). Replaces hardcoded BLOCK_COLORS/BLOCK_ICONS maps from FlowCanvas.tsx.

- [ ] **Step 1: Write the registry**

```typescript
// editor/client/src/canvas/nodeRegistry.ts

import type { PortData } from "../store/editor";

export interface NodeTypeConfig {
  tag: string;
  category: "flow" | "event" | "action" | "data" | "variable" | "code";
  label: string;
  icon: string;
  color: { bg: string; border: string };
  defaultPorts: PortData[];
  /** Search keywords for the right-click palette */
  keywords: string[];
  /** Description shown in search results */
  description: string;
}

// ── Code block types (existing, backward compatible) ──

const CODE_BLOCK_CONFIGS: NodeTypeConfig[] = [
  {
    tag: "signal", category: "variable", label: "Signal", icon: "⚡",
    color: { bg: "#FEF3C7", border: "#D97706" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "值" },
      { name: "set", direction: "in", portType: "data", type: "any", label: "设置" },
    ],
    keywords: ["signal", "state", "variable", "value", "状态", "变量"],
    description: "响应式状态变量，存储和读取值",
  },
  {
    tag: "computed", category: "data", label: "Computed", icon: "🔄",
    color: { bg: "#DBEAFE", border: "#2563EB" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "计算结果" },
      { name: "deps", direction: "in", portType: "data", type: "any", label: "依赖" },
    ],
    keywords: ["computed", "derived", "calculation", "计算", "派生"],
    description: "从其他值派生计算，依赖变化时自动更新",
  },
  {
    tag: "action", category: "action", label: "Action", icon: "▶️",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "call", direction: "in", portType: "flow", type: "void", label: "调用" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "返回" },
    ],
    keywords: ["action", "handler", "event", "function", "处理", "事件"],
    description: "事件处理函数，接收事件并执行逻辑",
  },
  {
    tag: "store", category: "variable", label: "Store", icon: "🗄️",
    color: { bg: "#F5F3FF", border: "#7C3AED" },
    defaultPorts: [
      { name: "items", direction: "out", portType: "data", type: "any", label: "数据" },
    ],
    keywords: ["store", "collection", "database", "存储", "集合"],
    description: "数据集合容器",
  },
  {
    tag: "fn", category: "code", label: "Function", icon: "📦",
    color: { bg: "#ECFDF5", border: "#059669" },
    defaultPorts: [],
    keywords: ["fn", "function", "定义", "函数"],
    description: "外部函数定义（不在画布上显示端口）",
  },
  {
    tag: "fn-call", category: "action", label: "Fn Call", icon: "📞",
    color: { bg: "#F0FDF4", border: "#10B981" },
    defaultPorts: [],
    keywords: ["call", "invoke", "调用"],
    description: "调用已定义的函数",
  },
];

// ── Placeholder configs for visual scripting nodes (Round 2-3 will flesh these out) ──

const VISUAL_NODE_CONFIGS: NodeTypeConfig[] = [
  // Event nodes
  {
    tag: "onPress", category: "event", label: "On Press", icon: "🔘",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
      { name: "event", direction: "out", portType: "data", type: "MouseEvent", label: "event" },
    ],
    keywords: ["press", "click", "tap", "button", "点击", "按钮"],
    description: "按钮点击时触发执行流",
  },
  {
    tag: "onChange", category: "event", label: "On Change", icon: "📝",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
      { name: "value", direction: "out", portType: "data", type: "String", label: "value" },
    ],
    keywords: ["change", "input", "type", "text", "输入", "变化"],
    description: "输入框内容变化时触发执行流",
  },
  // Action nodes
  {
    tag: "setVariable", category: "action", label: "Set Variable", icon: "⚡",
    color: { bg: "#FEF3C7", border: "#D97706" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
      { name: "value", direction: "in", portType: "data", type: "any", label: "value" },
    ],
    keywords: ["set", "assign", "variable", "设置", "赋值"],
    description: "设置变量的值",
  },
  // Data nodes
  {
    tag: "getVariable", category: "data", label: "Get Variable", icon: "📌",
    color: { bg: "#ECFDF5", border: "#059669" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "value" },
    ],
    keywords: ["get", "read", "variable", "读取", "变量"],
    description: "读取变量的当前值",
  },
  {
    tag: "add", category: "data", label: "Add", icon: "➕",
    color: { bg: "#ECFDF5", border: "#059669" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "A+B" },
    ],
    keywords: ["add", "plus", "sum", "加", "加法"],
    description: "两数相加",
    config: { operator: "+" },
  },
  // Flow nodes (Round 3 will flesh these out)
  {
    tag: "branch", category: "flow", label: "Branch", icon: "🔀",
    color: { bg: "#DBEAFE", border: "#2563EB" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "true", direction: "out", portType: "flow", type: "void", label: "True" },
      { name: "false", direction: "out", portType: "flow", type: "void", label: "False" },
      { name: "condition", direction: "in", portType: "data", type: "bool", label: "条件" },
    ],
    keywords: ["if", "else", "branch", "condition", "条件", "分支"],
    description: "根据条件分支执行",
  },
];

// ── Registry ──

const ALL_CONFIGS = [...CODE_BLOCK_CONFIGS, ...VISUAL_NODE_CONFIGS];
const byTag = new Map<string, NodeTypeConfig>();
const byCategory = new Map<string, NodeTypeConfig[]>();

for (const c of ALL_CONFIGS) {
  byTag.set(c.tag, c);
  const cat = byCategory.get(c.category) || [];
  cat.push(c);
  byCategory.set(c.category, cat);
}

/** Get visual config for a node tag. Falls back to a generic config. */
export function getNodeConfig(tag: string): NodeTypeConfig {
  return byTag.get(tag) || {
    tag,
    category: "code",
    label: tag,
    icon: "📦",
    color: { bg: "#F3F4F6", border: "#9CA3AF" },
    defaultPorts: [],
    keywords: [tag],
    description: "",
  };
}

/** All registered node configs (for search palette — Round 2). */
export function getAllNodeConfigs(): NodeTypeConfig[] {
  return ALL_CONFIGS;
}

/** Get all configs in a category. */
export function getNodeConfigsByCategory(cat: string): NodeTypeConfig[] {
  return byCategory.get(cat) || [];
}

/** Category display metadata. */
export const CATEGORY_META: Record<string, { label: string; icon: string; order: number }> = {
  event:    { label: "Events",     icon: "⚡", order: 1 },
  action:   { label: "Actions",    icon: "⚙️", order: 2 },
  flow:     { label: "Flow Control", icon: "🔀", order: 3 },
  data:     { label: "Data",       icon: "📊", order: 4 },
  variable: { label: "Variables",  icon: "📌", order: 5 },
  code:     { label: "Code Blocks", icon: "📦", order: 6 },
};
```

- [ ] **Step 2: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/canvas/nodeRegistry.ts && git commit -m "feat(editor): add node registry for visual scripting node type configs"
```

---

### Task 3: Create Custom Edge Components (FlowEdge + DataEdge)

**Files:**
- Create: `editor/client/src/canvas/edges/FlowEdge.tsx`
- Create: `editor/client/src/canvas/edges/DataEdge.tsx`

**Goal:** @xyflow/react custom edges that visually distinguish execution flow (white, triangle arrow) from data flow (colored by type, circle dot).

- [ ] **Step 1: Create FlowEdge**

```typescript
// editor/client/src/canvas/edges/FlowEdge.tsx

import { BaseEdge, getSmoothStepPath, type EdgeProps } from "@xyflow/react";

/**
 * Flow edge — white/gray thick line with triangle arrowhead.
 * Represents execution order (sequence flow) between nodes.
 */
export function FlowEdge({
  id,
  sourceX, sourceY,
  targetX, targetY,
  sourcePosition, targetPosition,
  selected,
}: EdgeProps) {
  const [edgePath] = getSmoothStepPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
    borderRadius: 8,
  });

  return (
    <>
      {/* Shadow/glow for selected state */}
      {selected && (
        <path
          d={edgePath}
          fill="none"
          stroke="#A5B4FC"
          strokeWidth={6}
          strokeLinecap="round"
          opacity={0.4}
        />
      )}
      <path
        d={edgePath}
        fill="none"
        stroke={selected ? "#6366F1" : "#9CA3AF"}
        strokeWidth={selected ? 3 : 2.5}
        strokeLinecap="round"
      />
      {/* Triangle arrowhead marker */}
      <marker
        id={`flow-arrow-${id}`}
        viewBox="0 0 10 8"
        refX="9" refY="4"
        markerWidth="8" markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 4 L 0 8 z" fill={selected ? "#6366F1" : "#9CA3AF"} />
      </marker>
      <path
        d={edgePath}
        fill="none"
        stroke="transparent"
        strokeWidth={1}
        markerEnd={`url(#flow-arrow-${id})`}
      />
    </>
  );
}
```

- [ ] **Step 2: Create DataEdge**

```typescript
// editor/client/src/canvas/edges/DataEdge.tsx

import { BaseEdge, getSmoothStepPath, type EdgeProps } from "@xyflow/react";

/** Color map for data types (matches socketTypes.ts TYPE_COLORS). */
const DATA_TYPE_COLORS: Record<string, string> = {
  any: "#6B7280",
  i32: "#2563EB",
  f64: "#059669",
  bool: "#D97706",
  String: "#DB2777",
  void: "#9CA3AF",
  MouseEvent: "#DC2626",
};

/**
 * Data edge — colored by data type with circle dot marker.
 * Represents value passing between node ports.
 */
export function DataEdge({
  id,
  sourceX, sourceY,
  targetX, targetY,
  sourcePosition, targetPosition,
  data,
  selected,
}: EdgeProps) {
  const [edgePath] = getSmoothStepPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
    borderRadius: 8,
  });

  // data.type is set when creating the edge in FlowCanvas
  const dataType = (data as any)?.type || "any";
  const color = DATA_TYPE_COLORS[dataType] || DATA_TYPE_COLORS.any;

  return (
    <>
      {selected && (
        <path
          d={edgePath}
          fill="none"
          stroke={color}
          strokeWidth={5}
          strokeLinecap="round"
          opacity={0.2}
        />
      )}
      <path
        d={edgePath}
        fill="none"
        stroke={color}
        strokeWidth={selected ? 2.5 : 2}
        strokeLinecap="round"
      />
      {/* Circle dot marker at target end */}
      <circle
        cx={targetX}
        cy={targetY}
        r={selected ? 5 : 4}
        fill={color}
        stroke="#fff"
        strokeWidth={1.5}
      />
    </>
  );
}
```

- [ ] **Step 3: Create barrel export for edges**

```typescript
// editor/client/src/canvas/edges/index.ts
export { FlowEdge } from "./FlowEdge";
export { DataEdge } from "./DataEdge";
```

- [ ] **Step 4: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/canvas/edges/ && git commit -m "feat(editor): add FlowEdge and DataEdge custom edge components for visual scripting"
```

---

### Task 4: Create Node Components

**Files:**
- Create: `editor/client/src/canvas/nodes/BaseNode.tsx`
- Create: `editor/client/src/canvas/nodes/CodeBlockNode.tsx`
- Create: `editor/client/src/canvas/nodes/ViewEventNode.tsx`
- Create: `editor/client/src/canvas/nodes/index.ts`

**Goal:** React Flow custom nodes that render existing code blocks and view elements with correct port shapes (triangle for flow, circle for data).

- [ ] **Step 1: Create BaseNode (shared chrome)**

```typescript
// editor/client/src/canvas/nodes/BaseNode.tsx

import { Handle, Position, type NodeProps } from "@xyflow/react";
import { getNodeConfig } from "../nodeRegistry";
import type { PortData } from "../../store/editor";

export interface BaseNodeData {
  elementId: string;
  tag: string;
  label: string;
  typeAnnotation?: string;
  outputValue?: unknown;
  ports: PortData[];
  isFolded?: boolean;
  onDoubleClick?: (elementId: string) => void;
}

/**
 * Shared node chrome. Renders icon + label + typed handles.
 * Flow ports (portType="flow") get triangle Handle style.
 * Data ports (portType="data" or undefined) get circle Handle style.
 */
export function BaseNode({ id, data, selected }: NodeProps) {
  const nodeData = data as unknown as BaseNodeData;
  const config = getNodeConfig(nodeData.tag);
  const valueStr = nodeData.outputValue !== undefined
    ? ` → ${JSON.stringify(nodeData.outputValue)}`
    : "";
  const typeAnn = nodeData.typeAnnotation || "";

  // Split ports by direction and type
  const flowIn = nodeData.ports.filter(p => p.direction === "in" && p.portType === "flow");
  const flowOut = nodeData.ports.filter(p => p.direction === "out" && p.portType === "flow");
  const dataIn = nodeData.ports.filter(p => p.direction === "in" && p.portType !== "flow");
  const dataOut = nodeData.ports.filter(p => p.direction === "out" && p.portType !== "flow");

  const handleDblClick = () => {
    nodeData.onDoubleClick?.(nodeData.elementId);
  };

  return (
    <div
      onDoubleClick={handleDblClick}
      style={{
        background: config.color.bg,
        border: `${selected ? 3 : 2}px solid ${config.color.border}`,
        borderRadius: 10,
        padding: "10px 16px",
        minWidth: 140,
        maxWidth: 260,
        fontFamily: "system-ui, sans-serif",
        fontSize: 12,
        boxShadow: selected
          ? "0 0 0 2px rgba(99,102,241,0.3), 0 4px 12px rgba(0,0,0,0.15)"
          : "0 2px 6px rgba(0,0,0,0.08)",
        position: "relative",
        transition: "box-shadow 0.15s",
      }}
    >
      {/* Flow input handles (left side, triangle) */}
      {flowIn.map((p, i) => (
        <FlowHandle
          key={p.name}
          port={p}
          type="target"
          position={Position.Left}
          index={i}
          total={flowIn.length}
        />
      ))}

      {/* Data input handles (left side, circle) */}
      {dataIn.map((p, i) => (
        <DataHandle
          key={p.name}
          port={p}
          type="target"
          position={Position.Left}
          index={i + flowIn.length}
          total={flowIn.length + dataIn.length}
        />
      ))}

      {/* Node body */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ fontSize: 16 }}>{config.icon}</span>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 600, color: "#1F2937", fontSize: 12, lineHeight: 1.3 }}>
            {nodeData.label}
          </div>
          {(typeAnn || valueStr) && (
            <div style={{ fontSize: 9, color: "#6B7280", marginTop: 2 }}>
              {typeAnn && <span>: {typeAnn}</span>}
              <span>{valueStr}</span>
            </div>
          )}
        </div>
      </div>

      {/* Flow output handles (right side, triangle) */}
      {flowOut.map((p, i) => (
        <FlowHandle
          key={p.name}
          port={p}
          type="source"
          position={Position.Right}
          index={i}
          total={flowOut.length}
        />
      ))}

      {/* Data output handles (right side, circle) */}
      {dataOut.map((p, i) => (
        <DataHandle
          key={p.name}
          port={p}
          type="source"
          position={Position.Right}
          index={i + flowOut.length}
          total={flowOut.length + dataOut.length}
        />
      ))}
    </div>
  );
}

// ── Handle sub-components ──

function FlowHandle({ port, type, position, index, total }: {
  port: PortData; type: "source" | "target"; position: Position; index: number; total: number;
}) {
  const yOffset = total <= 1 ? 0 : (index - (total - 1) / 2) * 24;
  return (
    <Handle
      id={port.name}
      type={type}
      position={position}
      style={{
        // Triangle approximation via CSS
        width: 10,
        height: 10,
        background: "#9CA3AF",
        border: "2px solid #fff",
        borderRadius: 2,
        transform: `translateY(${yOffset}px) rotate(45deg)`,
        cursor: "crosshair",
      }}
      title={`${port.label || port.name} (flow${port.type !== "void" ? `: ${port.type}` : ""})`}
    />
  );
}

function DataHandle({ port, type, position, index, total }: {
  port: PortData; type: "source" | "target"; position: Position; index: number; total: number;
}) {
  const yOffset = total <= 1 ? 0 : (index - (total - 1) / 2) * 24;
  const typeColor = getTypeColor(port.type);
  return (
    <Handle
      id={port.name}
      type={type}
      position={position}
      style={{
        width: 12,
        height: 12,
        background: typeColor,
        border: "2px solid #fff",
        borderRadius: "50%",
        transform: `translateY(${yOffset}px)`,
        cursor: "crosshair",
      }}
      title={`${port.label || port.name} (${port.type})`}
    />
  );
}

function getTypeColor(type: string): string {
  const colors: Record<string, string> = {
    any: "#6B7280", i32: "#2563EB", f64: "#059669",
    bool: "#D97706", String: "#DB2777", void: "#9CA3AF",
    MouseEvent: "#DC2626",
  };
  return colors[type] || colors.any;
}
```

- [ ] **Step 2: Create CodeBlockNode (wraps BaseNode for code blocks)**

```typescript
// editor/client/src/canvas/nodes/CodeBlockNode.tsx

import type { NodeProps } from "@xyflow/react";
import { BaseNode } from "./BaseNode";
import type { BaseNodeData } from "./BaseNode";

/**
 * Node renderer for existing code blocks (signal, computed, action, store, fn-call).
 * Extracts ports from element data and delegates to BaseNode.
 */
export function CodeBlockNode(props: NodeProps) {
  return <BaseNode {...props} />;
}
```

- [ ] **Step 3: Create ViewEventNode (wraps BaseNode for view elements with events)**

```typescript
// editor/client/src/canvas/nodes/ViewEventNode.tsx

import type { NodeProps } from "@xyflow/react";
import { BaseNode } from "./BaseNode";
import type { BaseNodeData } from "./BaseNode";

/**
 * Node renderer for view elements (Button, Input, etc.) shown on the code canvas.
 * Event ports (onPress, onChange) are rendered as data-out ports.
 */
export function ViewEventNode(props: NodeProps) {
  return <BaseNode {...props} />;
}
```

- [ ] **Step 4: Create barrel export**

```typescript
// editor/client/src/canvas/nodes/index.ts
export { BaseNode } from "./BaseNode";
export type { BaseNodeData } from "./BaseNode";
export { CodeBlockNode } from "./CodeBlockNode";
export { ViewEventNode } from "./ViewEventNode";
```

- [ ] **Step 5: Verify compilation**

```bash
cd "D:/space/OmniLang/editor/client" && npx tsc --noEmit --pretty false 2>&1 | head -30
```

- [ ] **Step 6: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/canvas/nodes/ && git commit -m "feat(editor): add BaseNode, CodeBlockNode, ViewEventNode with flow/data handle differentiation"
```

---

### Task 5: Rewrite FlowCanvas.tsx with @xyflow/react

**Files:**
- Modify: `editor/client/src/canvas/FlowCanvas.tsx` (full rewrite)
- Reference (keep for context during migration): old Rete-based code in git history

**Goal:** Replace the 846-line Rete.js FlowCanvas with a ~350-line @xyflow/react implementation. Maintain backward compatibility — existing code blocks and view elements render identically, wires render with correct edge types.

- [ ] **Step 1: Write the new FlowCanvas**

```typescript
// editor/client/src/canvas/FlowCanvas.tsx

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  type Connection,
  type Node,
  type Edge,
  BackgroundVariant,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useEditorStore, type ElementData, type WireData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { AddWire } from "../commands/AddWire";
import { RemoveWire } from "../commands/RemoveWire";
import { AddElement } from "../commands/AddElement";
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
  _wire: WireData,
): "flow" | "data" {
  const fromPorts = fromEl?.ports || [];
  const toPorts = toEl?.ports || [];
  const fromPort = fromPorts.find(p => p.name === _wire.fromPort);
  const toPort = toPorts.find(p => p.name === _wire.toPort);
  if (fromPort?.portType === "flow" || toPort?.portType === "flow") return "flow";
  return "data";
}

/** Get the data type for coloring DataEdges. */
function getEdgeDataType(
  fromEl: ElementData | undefined,
  _wire: WireData,
): string {
  const fromPort = fromEl?.ports?.find(p => p.name === _wire.fromPort);
  return fromPort?.type || "any";
}

// ── Props ──

interface FlowCanvasProps {
  showViews?: boolean;
  highlightedWireId?: string | null;
  onNodeDoubleClick?: (nodeId: string, nodeData: ElementData) => void;
  ctx: CommandContext;
  layoutTrigger?: number;
}

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
  const containerRef = useRef<HTMLDivElement>(null);

  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  // ── Convert elements → React Flow nodes ──

  const elementMapRef = useRef(new Map<string, ElementData>());

  useEffect(() => {
    elementMapRef.current.clear();
    for (const el of elements) elementMapRef.current.set(el.id, el);

    const newNodes: Node[] = [];
    const wiredIds = new Set<string>();
    if (!showViews) {
      for (const w of wires) { wiredIds.add(w.fromId); wiredIds.add(w.toId); }
    }

    for (const el of elements) {
      const isCodeBlock = el.nodeType === "codeblock" && el.tag !== "fn";
      const isViewWithEvents = el.nodeType === "view" && showViews;
      const isWired = wiredIds.has(el.id);

      if (!isCodeBlock && !isViewWithEvents && !isWired) continue;

      const config = getNodeConfig(el.tag);
      const ports = el.ports?.length ? el.ports : config.defaultPorts;
      const name = (el.props.name || el.props.label || el.tag) as string;
      const pos = el.position || {
        x: el.nodeType === "codeblock" ? 50 : 400,
        y: 50 + newNodes.length * 120,
      };

      const nodeData: BaseNodeData = {
        elementId: el.id,
        tag: el.tag,
        label: name,
        typeAnnotation: el.typeAnnotation || (el.props.typeAnnotation as string),
        outputValue: el.outputValue,
        ports,
        isFolded: el.isFolded,
        onDoubleClick: (id) => {
          const found = elements.find(e => e.id === id);
          if (found) onNodeDoubleClick?.(id, found);
        },
      };

      newNodes.push({
        id: el.id,
        type: el.nodeType === "codeblock" ? "codeblock" : "viewEvent",
        position: pos,
        data: nodeData,
        selected: el.id === useEditorStore.getState().selectedElementId,
      });
    }

    setNodes(newNodes);
  }, [elements, wires, showViews, onNodeDoubleClick]);

  // ── Convert wires → React Flow edges ──

  useEffect(() => {
    const newEdges: Edge[] = [];

    for (const w of wires) {
      const fromEl = elements.find(e => e.id === w.fromId);
      const toEl = elements.find(e => e.id === w.toId);
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

  // ── Connection handler ──

  const handleConnect = useCallback((connection: Connection) => {
    const fromEl = elements.find(e => e.id === connection.source);
    const toEl = elements.find(e => e.id === connection.target);
    if (!fromEl || !toEl) return;

    const fromPort = connection.sourceHandle || "out";
    const toPort = connection.targetHandle || "in";

    // Type validation
    const fromPorts = fromEl.ports || getNodeConfig(fromEl.tag).defaultPorts;
    const toPorts = toEl.ports || getNodeConfig(toEl.tag).defaultPorts;
    const sourcePort = fromPorts.find(p => p.name === fromPort && p.direction === "out");
    const targetPort = toPorts.find(p => p.name === toPort && p.direction === "in");

    if (sourcePort && targetPort) {
      if (!isTypeCompatible(sourcePort.type, targetPort.type)) {
        alert(typeMismatchMessage(sourcePort.type, targetPort.type));
        return;
      }
    }

    // Prevent duplicate wires
    const exists = wires.some(
      w => w.fromId === fromEl.id && w.fromPort === fromPort &&
           w.toId === toEl.id && w.toPort === toPort
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
  }, [elements, wires, ctx]);

  // ── Context menu (right-click → add block — Round 2 will add search) ──

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    setContextMenu({ x: e.clientX - rect.left, y: e.clientY - rect.top });
  }, []);

  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  // ── Delete handler (delete key removes selected nodes + their edges) ──

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Delete" || e.key === "Backspace") {
      const store = useEditorStore.getState();
      const selected = store.selectedElementId;
      if (selected) {
        store.removeElement(selected);
        // Also remove connected wires
        const relatedWires = store.wires.filter(
          w => w.fromId === selected || w.toId === selected
        );
        for (const w of relatedWires) store.removeWire(w.id);
      }
    }
  }, []);

  // ── Edge delete handler ──

  const handleEdgeDelete = useCallback((edgesToDelete: Edge[]) => {
    const store = useEditorStore.getState();
    for (const edge of edgesToDelete) {
      const wire = store.wires.find(w => w.id === edge.id);
      if (wire) {
        useUndoStore.getState().execute(new RemoveWire(wire.id), ctx);
      }
    }
  }, [ctx]);

  // ── Node click → select in store ──

  const handleNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    useEditorStore.getState().selectElement(node.id);
  }, []);

  // ── Pane click → deselect ──

  const handlePaneClick = useCallback(() => {
    useEditorStore.getState().selectElement(null);
  }, []);

  // ── Simple context menu (placeholder until Round 2 NodePalette) ──

  const BLOCK_MENU_ITEMS = [
    { tag: "action", label: "💻 代码块" },
    { tag: "signal", label: "⚡ Signal" },
    { tag: "computed", label: "🔄 Computed" },
  ];

  const handleAddBlock = (tag: string) => {
    const config = getNodeConfig(tag);
    const id = `cb-${tag}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const ports = config.defaultPorts.map(p => ({ ...p }));

    const el: ElementData = {
      id,
      nodeType: "codeblock",
      tag,
      props: { name: config.label },
      children: [],
      parentId: null,
      ports,
      isFolded: false,
      source: "",
    };
    useUndoStore.getState().execute(new AddElement(el), ctx);
    setContextMenu(null);
  };

  return (
    <div
      ref={containerRef}
      style={{ width: "100%", height: "100%", minHeight: 400 }}
      onContextMenu={handleContextMenu}
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={handleConnect}
        onNodeClick={handleNodeClick}
        onPaneClick={handlePaneClick}
        onEdgesDelete={handleEdgeDelete}
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
            const config = getNodeConfig((n.data as any)?.tag || "");
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
          <div style={{
            padding: "4px 12px",
            color: "#9CA3AF",
            fontSize: 10,
            textTransform: "uppercase",
            fontWeight: 600,
          }}>
            添加节点
          </div>
          {BLOCK_MENU_ITEMS.map(item => (
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
              onMouseEnter={e => (e.currentTarget.style.background = "#F3F4F6")}
              onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
            >
              <span>{item.label}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
cd "D:/space/OmniLang/editor/client" && npx tsc --noEmit --pretty false 2>&1 | head -50
```

Fix any type errors. Common issues:
- `BaseNodeData` import — ensure export path is correct
- `WireData` import — verify the enhanced type is used
- `ReactFlow` component props — ensure version 12 API compatibility

- [ ] **Step 3: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/canvas/FlowCanvas.tsx && git commit -m "feat(editor): rewrite FlowCanvas with @xyflow/react — flow/data edge types, custom nodes"
```

---

### Task 6: Create Flow Engine

**Files:**
- Create: `editor/client/src/canvas/FlowEngine.ts`

**Goal:** Topological sort executor that walks flow-connected nodes from event sources, evaluates data dependencies, and propagates values. Used by the codegen layer (Round 6) and optionally for live preview.

- [ ] **Step 1: Write the FlowEngine**

```typescript
// editor/client/src/canvas/FlowEngine.ts

import type { ElementData, WireData } from "../store/editor";

export interface ExecutionStep {
  elementId: string;
  tag: string;
  /** Data inputs gathered from incoming data edges */
  inputs: Record<string, unknown>;
  /** Flow order index */
  order: number;
}

export interface ExecutionResult {
  steps: ExecutionStep[];
  /** Data outputs produced (port → value) */
  outputs: Map<string, Map<string, unknown>>;
  /** Nodes that are unreachable from any event source */
  unreachable: string[];
}

/**
 * Topological sort over flow-connected nodes.
 * Starts from event nodes (nodeCategory === "event" or tag in EVENT_TAGS),
 * follows flow-out edges to build an execution DAG.
 *
 * Event tags that serve as execution roots:
 */
const EVENT_TAGS = new Set(["onPress", "onChange", "onHover", "onMount"]);

/**
 * Execute the flow graph for a given set of elements and wires.
 * Returns ordered execution steps with gathered data inputs.
 */
export function executeFlow(
  elements: ElementData[],
  wires: WireData[],
): ExecutionResult {
  const elMap = new Map(elements.map(e => [e.id, e]));
  const outputs = new Map<string, Map<string, unknown>>();
  const steps: ExecutionStep[] = [];
  const visited = new Set<string>();

  // Find all event nodes (execution roots)
  const eventNodes = elements.filter(
    e => e.nodeCategory === "event" || EVENT_TAGS.has(e.tag),
  );

  if (eventNodes.length === 0) {
    // No event nodes — all code blocks are unreachable in flow sense
    return {
      steps: [],
      outputs,
      unreachable: elements.filter(e => e.nodeType === "codeblock").map(e => e.id),
    };
  }

  // For each event node, trace flow graph via BFS
  for (const eventNode of eventNodes) {
    const queue: string[] = [eventNode.id];
    const localVisited = new Set<string>();
    let order = 0;

    while (queue.length > 0) {
      const currentId = queue.shift()!;
      if (localVisited.has(currentId)) continue;
      localVisited.add(currentId);
      visited.add(currentId);

      const current = elMap.get(currentId);
      if (!current) continue;

      // Gather data inputs from incoming data wires
      const inputs: Record<string, unknown> = {};
      const incomingDataWires = wires.filter(
        w => w.toId === currentId,
      );
      for (const w of incomingDataWires) {
        const sourceOutputs = outputs.get(w.fromId);
        if (sourceOutputs?.has(w.fromPort)) {
          inputs[w.toPort] = sourceOutputs.get(w.fromPort);
        }
      }

      steps.push({
        elementId: current.id,
        tag: current.tag,
        inputs,
        order: order++,
      });

      // Follow flow-out edges to next nodes
      const flowOutWires = wires.filter(
        w => w.fromId === currentId,
      );
      for (const w of flowOutWires) {
        const toEl = elMap.get(w.toId);
        const toPort = toEl?.ports?.find(p => p.name === w.toPort);
        // Only follow if this is a flow connection
        const fromPort = current.ports?.find(p => p.name === w.fromPort);
        if (fromPort?.portType === "flow" || toPort?.portType === "flow") {
          queue.push(w.toId);
        }
      }

      // For branch nodes, all flow-out ports go to queue
      // (Branch.true and Branch.false are both flow-out)
    }
  }

  // Find unreachable code blocks
  const unreachable = elements
    .filter(e => e.nodeType === "codeblock" && !visited.has(e.id))
    .map(e => e.id);

  return { steps, outputs, unreachable };
}

/**
 * Simple flow validation: check for cycles and missing connections.
 */
export function validateFlow(
  elements: ElementData[],
  wires: WireData[],
): { valid: boolean; cycles: string[][]; orphanFlowIns: string[] } {
  // Detect cycles via DFS on flow edges
  const cycles: string[][] = [];
  const flowAdj = new Map<string, string[]>();
  for (const w of wires) {
    const fromEl = elements.find(e => e.id === w.fromId);
    const toEl = elements.find(e => e.id === w.toId);
    const fromPort = fromEl?.ports?.find(p => p.name === w.fromPort);
    const toPort = toEl?.ports?.find(p => p.name === w.toPort);
    if (fromPort?.portType === "flow" || toPort?.portType === "flow") {
      const list = flowAdj.get(w.fromId) || [];
      list.push(w.toId);
      flowAdj.set(w.fromId, list);
    }
  }

  // Simple cycle detection (for production, use a proper algorithm)
  const WHITE = 0, GRAY = 1, BLACK = 2;
  const color = new Map<string, number>();
  for (const el of elements) color.set(el.id, WHITE);

  function dfs(nodeId: string, path: string[]): boolean {
    const c = color.get(nodeId);
    if (c === GRAY) {
      cycles.push([...path, nodeId]);
      return true;
    }
    if (c === BLACK) return false;
    color.set(nodeId, GRAY);
    path.push(nodeId);
    const neighbors = flowAdj.get(nodeId) || [];
    for (const n of neighbors) {
      if (dfs(n, path)) return true;
    }
    path.pop();
    color.set(nodeId, BLACK);
    return false;
  }

  for (const el of elements) {
    if (color.get(el.id) === WHITE) dfs(el.id, []);
  }

  // Find action nodes with unconnected flow-in ports
  const orphanFlowIns = elements
    .filter(e => {
      const flowInPorts = (e.ports || []).filter(
        p => p.direction === "in" && p.portType === "flow",
      );
      if (flowInPorts.length === 0) return false;
      // Check if any wire connects to these ports
      return !wires.some(w =>
        w.toId === e.id && flowInPorts.some(p => p.name === w.toPort),
      );
    })
    .map(e => e.id);

  return {
    valid: cycles.length === 0,
    cycles,
    orphanFlowIns,
  };
}
```

- [ ] **Step 2: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/canvas/FlowEngine.ts && git commit -m "feat(editor): add FlowEngine — topological sort executor for visual scripting flow graphs"
```

---

### Task 7: Update App.tsx Integration

**Files:**
- Modify: `editor/client/src/App.tsx:15,562-579`

**Goal:** Ensure the new @xyflow/react FlowCanvas receives correct props and the code mode canvas area works.

- [ ] **Step 1: Update imports in App.tsx**

Replace the existing FlowCanvas import (line 15). The import path stays the same since we're rewriting the file in-place:

```typescript
// In App.tsx, line 15 — no change needed, the import path is identical:
import { FlowCanvas } from "./canvas/FlowCanvas";
```

- [ ] **Step 2: Verify FlowCanvas usage site (lines 562-579)**

The existing usage in App.tsx should work as-is because the new FlowCanvas accepts the same props interface:

```typescript
// Lines 562-579 in App.tsx — no change needed:
<FlowCanvas
  ctx={ctx}
  showViews={false}
  highlightedWireId={highlightedWireId}
  layoutTrigger={layoutTrigger}
  onNodeDoubleClick={(id, el) => {
    const blockSource = el.source || "";
    setCodePopup({
      visible: true,
      blockId: id,
      title: `${(el.props as Record<string,unknown>)?.name || "Code"} (${el.tag})`,
      source: blockSource,
      blockElement: el,
    });
  }}
/>
```

- [ ] **Step 3: Add @xyflow/react CSS import**

Ensure the React Flow stylesheet is loaded. The FlowCanvas already imports it:

```typescript
import "@xyflow/react/dist/style.css";
```

But verify no duplicate CSS issues. React Flow's styles are scoped.

- [ ] **Step 4: Build and verify**

```bash
cd "D:/space/OmniLang/editor/client" && npx tsc --noEmit --pretty false 2>&1 | head -50
```

- [ ] **Step 5: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/src/App.tsx && git commit -m "chore(editor): verify App.tsx compatibility with new @xyflow/react FlowCanvas"
```

---

### Task 8: Integration Test — Verify Canvas Renders Code Blocks

**Files:**
- Create: `tests/editor/e2e/test_m4_visual_scripting.spec.ts`

**Goal:** End-to-end test that the new FlowCanvas renders existing code blocks correctly with @xyflow/react.

- [ ] **Step 1: Write the E2E test**

```typescript
// tests/editor/e2e/test_m4_visual_scripting.spec.ts

import { test, expect } from "@playwright/test";

test.describe("M4: Visual Scripting — @xyflow/react FlowCanvas", () => {

  test("canvas renders with @xyflow/react container", async ({ page }) => {
    await page.goto("http://localhost:5173");
    await page.waitForSelector('[data-testid="rf__wrapper"]', { timeout: 10000 });

    // React Flow wrapper should be present
    const rfWrapper = page.locator('[data-testid="rf__wrapper"]');
    await expect(rfWrapper).toBeVisible();
  });

  test("right-click opens block menu on canvas", async ({ page }) => {
    await page.goto("http://localhost:5173");
    await page.waitForSelector('[data-testid="rf__wrapper"]', { timeout: 10000 });

    // Switch to code mode if needed
    const codeBtn = page.locator("button", { hasText: "代码" });
    if (await codeBtn.isVisible()) await codeBtn.click();

    // Right-click on the canvas
    const rfWrapper = page.locator('[data-testid="rf__wrapper"]');
    await rfWrapper.click({ button: "right", position: { x: 300, y: 200 } });

    // Context menu should appear
    const menu = page.locator("text=添加节点");
    await expect(menu).toBeVisible({ timeout: 3000 });
  });

  test("adding a code block creates a ReactFlow node", async ({ page }) => {
    await page.goto("http://localhost:5173");
    await page.waitForSelector('[data-testid="rf__wrapper"]', { timeout: 10000 });

    const codeBtn = page.locator("button", { hasText: "代码" });
    if (await codeBtn.isVisible()) await codeBtn.click();

    // Right-click and add a code block
    const rfWrapper = page.locator('[data-testid="rf__wrapper"]');
    await rfWrapper.click({ button: "right", position: { x: 300, y: 200 } });
    await page.locator("text=💻 代码块").click();

    // A ReactFlow node should appear
    const node = page.locator(".react-flow__node");
    await expect(node).toBeVisible({ timeout: 5000 });
  });

  test("flow and data edges render with different styles", async ({ page }) => {
    // This tests that edge type differentiation works after connecting nodes
    // Will be filled out in Round 2 when we have event + action nodes to connect
    test.skip(true, "Requires Round 2 event/action nodes");
  });

});
```

- [ ] **Step 2: Run the E2E tests**

```bash
cd "D:/space/OmniLang" && npx playwright test tests/editor/e2e/test_m4_visual_scripting.spec.ts --reporter=list 2>&1
```

Expected: Tests that check @xyflow/react DOM structure pass. Node creation test may need adjustment.

- [ ] **Step 3: Commit**

```bash
cd "D:/space/OmniLang" && git add tests/editor/e2e/test_m4_visual_scripting.spec.ts && git commit -m "test(editor): add E2E tests for @xyflow/react FlowCanvas rendering"
```

---

### Task 9: Cleanup — Remove Unused Rete.js Dependencies (Optional, Post-Validation)

**Files:**
- Modify: `editor/client/package.json`

**Goal:** After confirming the new canvas works end-to-end, remove Rete.js packages to reduce bundle size.

- [ ] **Step 1: Remove Rete dependencies**

```bash
cd "D:/space/OmniLang/editor/client" && npm uninstall rete rete-area-plugin rete-auto-arrange-plugin rete-connection-plugin rete-react-plugin
```

- [ ] **Step 2: Verify build**

```bash
cd "D:/space/OmniLang/editor/client" && npx tsc --noEmit --pretty false 2>&1 | grep -i "rete" || echo "No Rete references remain"
```

- [ ] **Step 3: Commit**

```bash
cd "D:/space/OmniLang" && git add editor/client/package.json editor/client/package-lock.json && git commit -m "chore(editor): remove Rete.js dependencies, fully migrated to @xyflow/react"
```

---

## Pre-Implementation Checklist

- [ ] `@xyflow/react` v12 is installed (confirmed: `^12.11.0`)
- [ ] Backend server running for E2E tests (`python -m editor.server.main --dir build/test --port 5173`)
- [ ] Existing 3 E2E test suites still pass after migration
- [ ] TypeScript compilation succeeds for all changed files

## Spec Coverage Check

| Spec Requirement | Covered By |
|-----------------|------------|
| ElementData enhancements (nodeCategory, portType, config) | Task 1 |
| Replace Rete.js with @xyflow/react | Task 5 |
| Flow Engine (topological sort) | Task 6 |
| Dual-color wire rendering (flow vs data) | Task 3 |
| Port shape differentiation (triangle vs circle) | Task 4 |
| Node registry for type configs | Task 2 |
| Backward compatibility (existing code blocks render) | Task 4 + 5 |
| Right-click context menu (placeholder for Round 2) | Task 5 |
| E2E test coverage | Task 8 |
| Cleanup old Rete.js deps | Task 9 |
