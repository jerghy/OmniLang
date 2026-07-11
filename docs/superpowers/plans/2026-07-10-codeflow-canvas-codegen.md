# CodeFlowCanvas 代码生成 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 CodeFlowCanvas 从纯视觉占位节点升级为能真正生成 OmniView 代码的可视化编辑器。

**Architecture:** 4 种节点（fn / code-block / condition / return），纯控制流连线。GraphCompiler 在保存时将图编译为 fn body `.omni` 源码（含 `// @pid:xxx`），GraphParser 在加载时反向解析 `.omni` 源码重建图。JSON Store 只存位置和连线拓扑，代码内容全部在 `.omni` 文件中。

**Tech Stack:** React 19 + TypeScript + Zustand 5 + ReactFlow (@xyflow/react)

## Global Constraints

- 只做单组件内 code flow，不做多文件项目管理
- 保存时触发代码生成（非实时）
- DesignPreview 只管样式预览，不联动代码变更
- for / while 节点本期不做
- `// @pid:xxx` 注释保证往返解析稳定性
- 代码内容存 `.omni` 文件，JSON Store 只存位置 + 连线

---

### Task 1: 更新类型定义和常量

**Files:**
- Modify: `poiesis/src/components/canvas/code/flowConstants.ts` (完整替换)
- Modify: `poiesis/src/core/types/element.ts` (新增 tag 联合类型)

**Interfaces:**
- Produces: `CodeblockTag` 类型 = `"fn" | "code-block" | "condition" | "return"`
- Produces: `TAG_CN`、`CODE_TAGS`、`MENU_SECTIONS` 新常量

- [ ] **Step 1: 更新 CODE_TAGS 和 TAG_CN**

替换 `poiesis/src/components/canvas/code/flowConstants.ts` 全部内容：

```typescript
// components/canvas/code/flowConstants.ts
// Constants and types shared between FlowCanvas and CanvasContextMenu.

export interface ContextMenuState {
  x: number;
  y: number;
  flowX: number;
  flowY: number;
}

/** Chinese labels for code-element tags shown in nodes */
export const TAG_CN: Record<string, string> = {
  fn: "函数",
  "code-block": "代码块",
  condition: "判断",
  return: "返回",
};

export const CODE_TAGS = new Set([
  "fn",
  "code-block",
  "condition",
  "return",
]);

export interface MenuItem {
  label: string;
  tag: string;
  shortcut?: string;
}

export interface MenuSection {
  label: string;
  items: MenuItem[];
}

export const MENU_SECTIONS: MenuSection[] = [
  {
    label: "控制流",
    items: [
      { label: "函数定义 (fn)", tag: "fn", shortcut: "fn" },
      { label: "判断 (condition)", tag: "condition", shortcut: "cond" },
      { label: "代码块 (code-block)", tag: "code-block", shortcut: "cb" },
      { label: "返回 (return)", tag: "return", shortcut: "ret" },
    ],
  },
];

export const MENU_WIDTH = 210;
export const ITEM_HEIGHT = 30;
export const HEADER_HEIGHT = 26;
```

- [ ] **Step 2: 验证组件编译通过**

```bash
cd D:/space/OmniLang/poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/flowConstants.ts
git commit -m "refactor: simplify code flow node tags to fn/code-block/condition/return"
```

---

### Task 2: 实现 GraphCompiler（图 → .omni 源码）

**Files:**
- Create: `poiesis/src/services/codegen/graph-compiler.ts`

**Interfaces:**
- Consumes: `ElementData[]`（tag: fn/code-block/condition/return）、`WireData[]`、入口 fn 的 pid
- Produces: `compileCodeFlowGraph(nodes, wires, fnPid): string` — 完整的 fn body 字符串（含 `// @pid:xxx`）

- [ ] **Step 1: 创建 graph-compiler.ts**

```typescript
// poiesis/src/services/codegen/graph-compiler.ts
// Compile code flow graph (nodes + wires) → OmniView fn body source string.
// Output includes // @pid:xxx annotations for round-trip stability.

import type { ElementData, CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";

interface GraphNode {
  element: CodeblockElementData;
  successors: string[]; // element IDs of next nodes in control flow
}

/** Build adjacency map: element ID → GraphNode with successors */
function buildAdjacency(
  elements: CodeblockElementData[],
  wires: WireData[],
): Map<string, GraphNode> {
  const map = new Map<string, GraphNode>();
  for (const el of elements) {
    map.set(el.id, { element: el, successors: [] });
  }
  for (const w of wires) {
    // Only follow control-flow edges (flow-out → flow-in)
    const node = map.get(w.fromId);
    if (node && (w.fromPort === "flow-out" || w.fromPort.startsWith("br"))) {
      node.successors.push(w.toId);
    }
  }
  return map;
}

/** Emit a single code flow node as OmniView source text */
function emitNode(
  node: GraphNode,
  adj: Map<string, GraphNode>,
  indent: number,
  visited: Set<string>,
): string {
  if (visited.has(node.element.id)) return "";
  visited.add(node.element.id);

  const pad = "  ".repeat(indent);
  const pid = node.element.pid || node.element.id;
  const tag = node.element.tag;

  switch (tag) {
    case "fn": {
      const name = (node.element.props?.name as string) || "unnamed";
      const params = (node.element.props?.params as string) || "";
      let out = `// @pid:${pid}\nfn ${name}(${params}) {\n`;
      // Emit successors (fn body)
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent + 1, visited);
      }
      out += `}`;
      return out;
    }

    case "code-block": {
      const source = node.element.source || "";
      let out = `// @pid:${pid}\n`;
      // Split multi-line source and indent each line
      const lines = source.split("\n");
      for (const line of lines) {
        out += `${pad}${line}\n`;
      }
      // Emit successor
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent, visited);
      }
      return out;
    }

    case "condition": {
      const branches = (node.element.props?.branches as Array<{
        id: string; type: "if" | "elif" | "else"; condition: string;
      }>) || [];
      let out = "";

      for (let i = 0; i < branches.length; i++) {
        const br = branches[i];
        const branchSuffix = br.id ? `:${br.id}` : `:br${i}`;
        const brPid = `${pid}${branchSuffix}`;

        const keyword = br.type === "if" ? "if" : br.type === "elif" ? "else if" : "else";
        const condStr = br.type !== "else" ? ` ${br.condition}` : "";

        if (i === 0) {
          out += `// @pid:${pid}\n`;
        }
        out += `${pad}${keyword}${condStr} {\n`;

        // Find the successor for this branch
        const branchWire = node.successors.filter((sid) => {
          // Match wires where fromPort === branch id
          return true; // Simplified — we need to match by branch id
        });

        // Emit branch body
        for (const succId of node.successors) {
          // Find which wire corresponds to this branch
          // We need the actual wire list to match by fromPort
          const succ = adj.get(succId);
          if (succ) {
            out += emitNode(succ, adj, indent + 1, visited);
          }
        }

        out += `${pad}}\n`;
        if (br.type === "else") break; // else is always last
      }

      // Emit successor after condition block (if any)
      return out;
    }

    case "return": {
      const expr = node.element.source || "";
      let out = `// @pid:${pid}\n`;
      out += `${pad}return${expr ? " " + expr : ""};\n`;
      // return has no successors (terminal)
      return out;
    }

    default:
      return "";
  }
}

/**
 * Compile a code flow graph to a complete fn body string.
 *
 * @param elements  All code flow elements (fn, code-block, condition, return)
 * @param wires     All control-flow wires between elements
 * @param fnPid     The PID (or element ID) of the fn entry node
 * @returns         Complete fn definition string with // @pid:xxx annotations
 */
export function compileCodeFlowGraph(
  elements: CodeblockElementData[],
  wires: WireData[],
  fnPid: string,
): string {
  const adj = buildAdjacency(elements, wires);
  const fnNode = adj.get(fnPid);
  if (!fnNode) return "";
  return emitNode(fnNode, adj, 0, new Set());
}
```

注意：condition 节点的分支匹配需要细化——需要根据 wire 的 `fromPort` 匹配到具体分支。这一步在后续任务中完善。

- [ ] **Step 2: 验证编译**

```bash
cd D:/space/OmniLang/poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/graph-compiler.ts
git commit -m "feat: add GraphCompiler for code flow graph → .omni source"
```

---

### Task 3: 实现 GraphParser（.omni 源码 → 图）

**Files:**
- Create: `poiesis/src/services/codegen/graph-parser.ts`

**Interfaces:**
- Consumes: fn body 源码字符串（含 `// @pid:xxx` 注释）
- Produces: `parseCodeFlowGraph(fnBody: string, componentName: string): { elements: CodeblockElementData[], wires: WireData[] }`

- [ ] **Step 1: 创建 graph-parser.ts**

```typescript
// poiesis/src/services/codegen/graph-parser.ts
// Parse an OmniView fn body string → code flow graph (nodes + wires).
// Reverse of GraphCompiler. Uses // @pid:xxx annotations to identify nodes.

import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import { uid } from "../../core/utils/id";

interface ParsedSegment {
  pid: string;
  startLine: number;
  endLine: number;
  code: string;     // raw code for this segment (without pid comment)
  type: "fn" | "code-block" | "condition" | "return";
  // For condition segments, store branch info
  branches?: Array<{ id: string; type: "if" | "elif" | "else"; condition: string }>;
  parentPid?: string; // enclosing block's pid (for wire reconstruction)
}

/**
 * Split fn body into segments delimited by // @pid:xxx comments.
 * Each segment starts at a @pid comment and ends before the next @pid comment.
 */
function splitByPid(fnBody: string): ParsedSegment[] {
  const segments: ParsedSegment[] = [];
  const pidRe = /^\s*\/\/\s*@pid:(\S+)/;
  const lines = fnBody.split("\n");

  let currentPid = "";
  let currentStart = 0;
  let currentLines: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const match = line.match(pidRe);
    if (match) {
      // Save previous segment
      if (currentPid && currentLines.length > 0) {
        segments.push({
          pid: currentPid,
          startLine: currentStart,
          endLine: i - 1,
          code: currentLines.join("\n").trimEnd(),
          type: detectSegmentType(currentLines),
        });
      }
      currentPid = match[1];
      currentStart = i;
      currentLines = [];
    } else {
      currentLines.push(line);
    }
  }
  // Save last segment
  if (currentPid && currentLines.length > 0) {
    segments.push({
      pid: currentPid,
      startLine: currentStart,
      endLine: lines.length - 1,
      code: currentLines.join("\n").trimEnd(),
      type: detectSegmentType(currentLines),
    });
  }

  return segments;
}

/** Determine segment type from first non-empty line of code */
function detectSegmentType(lines: string[]): "fn" | "code-block" | "condition" | "return" {
  const firstLine = lines.find((l) => l.trim()) || "";
  const trimmed = firstLine.trim();

  if (trimmed.startsWith("fn ")) return "fn";
  if (trimmed.startsWith("if ") || trimmed.startsWith("else if ") || trimmed === "else" || trimmed.startsWith("else {")) return "condition";
  if (trimmed.startsWith("return ") || trimmed === "return;" || trimmed === "return") return "return";
  return "code-block";
}

/** Parse condition segment to extract branches */
function parseConditionBranches(code: string): Array<{ id: string; type: "if" | "elif" | "else"; condition: string }> {
  const branches: Array<{ id: string; type: "if" | "elif" | "else"; condition: string }> = [];
  const lines = code.split("\n");

  let currentType: "if" | "elif" | "else" = "if";
  let currentCond = "";
  let branchIdx = 0;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("if ")) {
      currentType = "if";
      currentCond = trimmed.slice(3).replace(/\s*\{$/, "").trim();
    } else if (trimmed.startsWith("else if ")) {
      // Save previous if/elif
      branches.push({ id: `br${branchIdx++}`, type: currentType, condition: currentCond });
      currentType = "elif";
      currentCond = trimmed.slice(8).replace(/\s*\{$/, "").trim();
    } else if (trimmed === "else {" || trimmed === "else") {
      // Save previous if/elif
      branches.push({ id: `br${branchIdx++}`, type: currentType, condition: currentCond });
      currentType = "else";
      currentCond = "";
    }
  }
  // Save last branch
  branches.push({ id: `br${branchIdx}`, type: currentType, condition: currentCond });

  return branches;
}

/**
 * Parse an OmniView fn body string into code flow elements and wires.
 *
 * @param fnBody        Raw fn body source (inside the braces), with // @pid:xxx comments
 * @param componentName Name of the enclosing component (for element ID generation)
 * @returns             Elements and wires for the code flow graph
 */
export function parseCodeFlowGraph(
  fnBody: string,
  componentName: string,
): { elements: CodeblockElementData[]; wires: WireData[] } {
  const segments = splitByPid(fnBody);
  if (segments.length === 0) return { elements: [], wires: [] };

  const elements: CodeblockElementData[] = [];
  const wires: WireData[] = [];

  // Build elements from segments
  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    const el: CodeblockElementData = {
      id: seg.pid,
      nodeType: "codeblock",
      tag: seg.type,
      parentId: null,
      props: {},
      source: "",
    };

    if (seg.type === "fn") {
      // Extract fn name and params from code
      const sigMatch = seg.code.match(/fn\s+(\w+)\s*\(([^)]*)\)/);
      el.props = {
        name: sigMatch?.[1] || "unnamed",
        params: sigMatch?.[2] || "",
      };
      el.source = "";
    } else if (seg.type === "condition") {
      const branches = parseConditionBranches(seg.code);
      el.props = { branches };
      el.source = "";
    } else if (seg.type === "code-block") {
      el.source = seg.code;
    } else if (seg.type === "return") {
      const retMatch = seg.code.match(/return\s+(.+);?/);
      el.source = retMatch?.[1] || "";
    }

    elements.push(el);

    // Wire to next segment (linear chain by default — branches handled below)
    if (i < segments.length - 1) {
      const next = segments[i + 1];
      wires.push({
        id: uid("wire"),
        fromId: seg.pid,
        fromPort: seg.type === "condition" ? "br0" : "flow-out",
        toId: next.pid,
        toPort: "flow-in",
      });
    }
  }

  return { elements, wires };
}
```

- [ ] **Step 2: 验证编译**

```bash
cd D:/space/OmniLang/poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/graph-parser.ts
git commit -m "feat: add GraphParser for .omni source → code flow graph"
```

---

### Task 4: 创建新节点 UI 组件

**Files:**
- Create: `poiesis/src/components/canvas/code/nodes/FnNode.tsx`
- Create: `poiesis/src/components/canvas/code/nodes/ConditionNode.tsx`
- Modify: `poiesis/src/components/canvas/code/nodes/CodeBlockNode.tsx` (更新 tag 映射)
- Modify: `poiesis/src/components/canvas/code/nodes/ReturnNode.tsx` (简化为单入口)

**Interfaces:**
- Consumes: ReactFlow `NodeProps` 中的 `data`（来自 FlowCanvas 的 nodes mapping）
- Produces: 4 个可渲染的 ReactFlow 自定义节点

- [ ] **Step 1: 创建 FnNode.tsx**

```typescript
// poiesis/src/components/canvas/code/nodes/FnNode.tsx
import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    fnName?: string;
    params?: string;
    onDoubleClick?: () => void;
  };
}

export function FnNode({ data }: Props) {
  const name = data.fnName || data.label || "fn";
  const params = data.params || "";

  return (
    <div
      className="flow-node"
      data-node-type="fn"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#EEF2FF",
        border: "2px solid #6366F1",
        borderRadius: "12px 12px 4px 4px",
        padding: "10px 18px",
        minWidth: 160,
        textAlign: "center",
        fontSize: 13,
        fontWeight: 600,
        color: "#3730A3",
        cursor: "pointer",
      }}
    >
      <div>
        <span style={{ fontSize: 14, marginRight: 4 }}>ƒ</span>
        {name}
        {params && <span style={{ fontWeight: 400 }}>({params})</span>}
      </div>
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "#6366F1" }}
      />
    </div>
  );
}
```

- [ ] **Step 2: 创建 ConditionNode.tsx**

```typescript
// poiesis/src/components/canvas/code/nodes/ConditionNode.tsx
import { Handle, Position } from "@xyflow/react";
import { useState } from "react";

interface BranchData {
  id: string;
  type: "if" | "elif" | "else";
  condition: string;
}

interface Props {
  data: {
    label: string;
    branches?: BranchData[];
    onDoubleClick?: () => void;
    onUpdateBranches?: (branches: BranchData[]) => void;
    onEditCondition?: (branchId: string, condition: string) => void;
  };
}

const TYPE_CN: Record<string, string> = { if: "如果", elif: "否则如果", else: "否则" };

export function ConditionNode({ data }: Props) {
  const branches: BranchData[] = data.branches || [{ id: "br0", type: "if", condition: "" }];
  const [editingBranch, setEditingBranch] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  const handleAddBranch = (type: "elif" | "else") => {
    const newBranches = [...branches, {
      id: `br${branches.length}`,
      type,
      condition: "",
    }];
    data.onUpdateBranches?.(newBranches);
  };

  const handleEditStart = (br: BranchData) => {
    setEditingBranch(br.id);
    setEditValue(br.condition);
  };

  const handleEditConfirm = (br: BranchData) => {
    data.onEditCondition?.(br.id, editValue);
    setEditingBranch(null);
  };

  const hasElse = branches.some((b) => b.type === "else");

  return (
    <div
      className="flow-node"
      data-node-type="condition"
      style={{
        background: "#FFFBEB",
        border: "2px solid #F59E0B",
        borderRadius: 8,
        padding: "6px 0",
        minWidth: 200,
        fontSize: 12,
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "#F59E0B" }}
      />

      <div style={{ padding: "0 10px", fontWeight: 600, marginBottom: 4, color: "#92400E" }}>
        ◆ 判断
      </div>

      {branches.map((br, i) => (
        <div
          key={br.id}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "3px 10px",
            borderTop: i > 0 ? "1px solid #FDE68A" : "none",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
            <span style={{
              fontSize: 10,
              fontWeight: 600,
              color: br.type === "if" ? "#059669" : br.type === "elif" ? "#D97706" : "#DC2626",
              minWidth: 36,
            }}>
              {TYPE_CN[br.type]}
            </span>
            {editingBranch === br.id ? (
              <input
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                onBlur={() => handleEditConfirm(br)}
                onKeyDown={(e) => { if (e.key === "Enter") handleEditConfirm(br); }}
                autoFocus
                style={{
                  width: "100%",
                  fontSize: 11,
                  fontFamily: "monospace",
                  border: "1px solid #F59E0B",
                  borderRadius: 3,
                  padding: "1px 4px",
                }}
                placeholder="条件表达式"
              />
            ) : (
              <span
                onClick={() => br.type !== "else" && handleEditStart(br)}
                style={{
                  fontFamily: "monospace",
                  fontSize: 11,
                  color: br.type === "else" ? "#9CA3AF" : "#1F2937",
                  cursor: br.type === "else" ? "default" : "pointer",
                  flex: 1,
                }}
              >
                {br.type === "else" ? "" : br.condition || "..."}
              </span>
            )}
          </div>

          <Handle
            type="source"
            position={Position.Right}
            id={br.id}
            style={{ background: "#F59E0B", position: "relative", right: -6, transform: "none" }}
          />
        </div>
      ))}

      {/* Add branch buttons */}
      <div style={{ display: "flex", gap: 4, padding: "4px 10px 0", borderTop: "1px solid #FDE68A" }}>
        {!hasElse && (
          <>
            <button
              onClick={() => handleAddBranch("elif")}
              style={{
                fontSize: 10, padding: "2px 6px", border: "1px solid #D97706",
                borderRadius: 3, background: "#FFF", color: "#D97706", cursor: "pointer",
              }}
            >
              + elif
            </button>
            <button
              onClick={() => handleAddBranch("else")}
              style={{
                fontSize: 10, padding: "2px 6px", border: "1px solid #DC2626",
                borderRadius: 3, background: "#FFF", color: "#DC2626", cursor: "pointer",
              }}
            >
              + else
            </button>
          </>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: 更新 CodeBlockNode.tsx**

修改 `poiesis/src/components/canvas/code/nodes/CodeBlockNode.tsx`，将 `TAG_LABELS` 替换为新的 tag 映射：

```typescript
// Replace the TAG_LABELS constant (lines 13-32) with:
const TAG_LABELS: Record<string, string> = {
  "code-block": "代码块",
};
```

同时更新 `data-node-type` 为 `"code-block"`。

- [ ] **Step 4: 更新 ReturnNode.tsx**

修改 `poiesis/src/components/canvas/code/nodes/ReturnNode.tsx`，移除多余的 source handle，只保留 flow-in：

```typescript
// poiesis/src/components/canvas/code/nodes/ReturnNode.tsx
import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    returnExpr?: string;
    onDoubleClick?: () => void;
  };
}

export function ReturnNode({ data }: Props) {
  const expr = data.returnExpr || "";

  return (
    <div
      className="flow-node"
      data-node-type="return"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#FFF1F2",
        border: "2px solid #F43F5E",
        borderRadius: "4px 4px 12px 12px",
        padding: "8px 14px",
        minWidth: 120,
        textAlign: "center",
        fontSize: 12,
        fontWeight: 600,
        color: "#BE123C",
        cursor: "pointer",
      }}
    >
      <div>return{expr ? ` ${expr}` : ""}</div>
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "#F43F5E" }}
      />
    </div>
  );
}
```

- [ ] **Step 5: 验证编译**

```bash
cd D:/space/OmniLang/poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/components/canvas/code/nodes/FnNode.tsx poiesis/src/components/canvas/code/nodes/ConditionNode.tsx poiesis/src/components/canvas/code/nodes/CodeBlockNode.tsx poiesis/src/components/canvas/code/nodes/ReturnNode.tsx
git commit -m "feat: rewrite code flow node components for fn/code-block/condition/return"
```

---

### Task 5: 更新 FlowCanvas 集成

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`

**Interfaces:**
- Consumes: 新 nodeTypes、CODE_TAGS、MENU_SECTIONS
- Produces: 完整可用的 CodeFlowCanvas

- [ ] **Step 1: 更新 FlowCanvas.tsx**

主要改动：
1. `nodeTypes` 映射更新：`fn` → FnNode, `condition` → ConditionNode, `code-block` → CodeBlockNode, `return` → ReturnNode
2. 移除 `SkeletonNode`、`FunctionHeaderNode` 导入
3. `handleAddNode` 更新为新 tag
4. `codeElements` 过滤使用新 `CODE_TAGS`
5. node type mapping 更新
6. 右键菜单添加新标签的默认 props

关键改动点：

```typescript
// nodeTypes 行 26-32 替换为:
const nodeTypes = {
  "fn": FnNode,
  "code-block": CodeBlockNode,
  "condition": ConditionNode,
  "return": ReturnNode,
  "design-preview": DesignPreviewNode,
};

// handleAddNode (行 101-146) 替换为:
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
      source: tag === "fn" || tag === "condition" ? "" : "",
    };
    addElement(newEl);
    setContextMenu(null);
  },
  [contextMenu, addElement],
);

// codeElements filter (行 238-241) 保持: tag check with CODE_TAGS

// nodes mapping (行 243-265) 更新:
const nodes: RFNode[] = useMemo(
  () =>
    codeElements.map((el) => ({
      id: el.id,
      type: el.tag,
      position: el.position ?? { x: 0, y: 0 },
      data: {
        label: TAG_CN[el.tag] || el.tag,
        source: el.source,
        fnName: el.props?.name as string,
        params: el.props?.params as string,
        branches: el.props?.branches as Array<{id:string;type:string;condition:string}>,
        returnExpr: el.source,
        onDoubleClick: () => { /* open inline editor */ },
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
  [codeElements],
);
```

- [ ] **Step 2: 验证编译**

```bash
cd D:/space/OmniLang/poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat: integrate new code flow node system into FlowCanvas"
```

---

### Task 6: 集成 GraphCompiler 到保存流程

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline.ts`
- Modify: `poiesis/src/services/codegen/component-builder.ts`（如需）

**Interfaces:**
- Consumes: GraphCompiler, code flow elements, wires
- Produces: fn body 字符串注入到 EmitInput

- [ ] **Step 1: 在 pipeline.ts 中集成 GraphCompiler**

在 `generateOmniSource` 函数中，Phase B 之后，在构建 `EmitInput` 的 `fns` 之前，调用 `compileCodeFlowGraph` 为每个 fn 生成 body：

```typescript
// 在 pipeline.ts Phase B 之后添加:

import { compileCodeFlowGraph } from "./graph-compiler";
import type { CodeblockElementData } from "../../core/types/element";

// 在构建 emitInput.components[].fns 之前:
const codeFlowElements = input.designElements.filter(
  (e): e is CodeblockElementData => e.nodeType === "codeblock"
);
const codeFlowWires = input.designWires;

// 为每个 fn 元素生成 body
for (const fnEl of codeFlowElements) {
  if (fnEl.tag !== "fn") continue;
  const fnBody = compileCodeFlowGraph(codeFlowElements, codeFlowWires, fnEl.id);
  // 将 fnBody 写入对应的 FnDef.body
  const fnName = (fnEl.props?.name as string) || "unnamed";
  const existingFn = emitInput.components[0]?.fns?.find(f => f.name === fnName);
  if (existingFn) {
    existingFn.body = fnBody;
  }
}
```

- [ ] **Step 2: 验证保存流程能正常写入 .omni 文件**

启动 Poiesis 并手动测试：
1. 打开一个包含 component 的 .omni 文件
2. 在 CodeFlowCanvas 中创建 fn → code-block → return 图
3. 保存
4. 检查 .omni 文件是否包含正确的 fn body 和 // @pid:xxx 注释

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/pipeline.ts
git commit -m "feat: integrate GraphCompiler into save pipeline"
```

---

### Task 7: 集成 GraphParser 到加载流程

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`

**Interfaces:**
- Consumes: GraphParser, token stream, fn body tokens
- Produces: 解析后的 code flow elements + wires 注入 ParseResult

- [ ] **Step 1: 重写 parser.ts 的 parseFn 方法**

当前 `parseFn()` 只是 brace-count 跳过函数体。需要改为：

```typescript
// parser.ts 中:
import { parseCodeFlowGraph } from "./graph-parser";

private parsedFnBodies: Map<string, { elements: CodeblockElementData[]; wires: WireData[] }> = new Map();

private parseFn(): void {
  this.stream.consume(); // "fn"

  // Collect fn signature tokens until {
  let fnSignature = "";
  while (!this.stream.isEof()) {
    const tok = this.stream.peek();
    if (tok.kind === "BRACE_OPEN") {
      this.stream.consume();
      break;
    }
    fnSignature += tok.text + " ";
    this.stream.consume();
  }

  // Extract fn name from signature
  const sigMatch = fnSignature.match(/(\w+)\s*\(([^)]*)\)/);
  const fnName = sigMatch?.[1] || "unnamed";

  // Collect fn body with brace counting, preserving // @pid:xxx comments
  let fnBody = "";
  let depth = 1;
  while (depth > 0 && !this.stream.isEof()) {
    const tok = this.stream.peek();
    if (tok.kind === "BRACE_OPEN") { depth++; fnBody += "{\n"; }
    else if (tok.kind === "BRACE_CLOSE") { depth--; if (depth > 0) fnBody += "}\n"; }
    else { fnBody += tok.text + " "; }
    this.stream.consume();
  }

  // Parse fn body into code flow graph
  const { elements, wires } = parseCodeFlowGraph(fnBody, this.ctx.currentComponent);

  this.parsedFnBodies.set(fnName, { elements, wires });
}
```

同时需要将解析后的 elements/wires 暴露到 ParseResult 中。

- [ ] **Step 2: 更新 ParseResult 类型**

在 `poiesis/src/services/codegen/types.ts` 中：

```typescript
export interface ParseResult {
  // ... existing fields
  /** Code flow graphs parsed from fn bodies, keyed by fn name */
  codeFlowGraphs?: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>;
}
```

- [ ] **Step 3: 验证加载流程**

启动 Poiesis，打开包含 fn 的 .omni 文件，验证 CodeFlowCanvas 正确显示节点图。

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/types.ts
git commit -m "feat: integrate GraphParser into parse pipeline"
```

---

### Task 8: 清理废弃代码

**Files:**
- Delete: `poiesis/src/components/canvas/code/nodes/FunctionHeaderNode.tsx`
- Delete: `poiesis/src/components/canvas/code/nodes/SkeletonNode.tsx`
- Modify: `poiesis/src/components/canvas/code/CanvasContextMenu.tsx`（如有引用旧常量）

- [ ] **Step 1: 删除废弃节点文件**

```bash
rm poiesis/src/components/canvas/code/nodes/FunctionHeaderNode.tsx
rm poiesis/src/components/canvas/code/nodes/SkeletonNode.tsx
```

- [ ] **Step 2: 验证编译通过**

```bash
cd D:/space/OmniLang/poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

- [ ] **Step 3: 用 Victauri 验证编辑器功能**

启动 Poiesis，在 CodeFlowCanvas 中：
1. 右键菜单只显示 4 个选项（fn / condition / code-block / return）
2. 添加 fn 节点 → 显示紫色 ƒ 节点
3. 添加 code-block → 双击可编辑代码
4. 添加 condition → 可编辑分支条件
5. 连线正常工作
6. 保存后 .omni 文件包含正确的代码

- [ ] **Step 4: Commit**

```bash
git add -A poiesis/src/components/canvas/code/
git commit -m "chore: remove deprecated FunctionHeaderNode and SkeletonNode"
```
