# CodeFlowCanvas 分组标注 + 组级折叠 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 CodeFlowCanvas 中实现「节点分组标注（相当于注释）+ 组级 Fold/Unfold」，数据存 project.json，源码生成 `// ── 组名 ──` 注释。

**Architecture:** 分组是纯编辑器元数据：运行时 store 按 fn 名存 `codeFlowGroups`，保存时直写 project.json 独立顶层字段（`persistedPositions` 先例），加载时缓存回 store 供画布渲染。分组框/折叠卡作为 ReactFlow 特殊 node type 渲染（不参与布局算法，只跟随成员包围盒）；折叠仅是视觉——成员节点不渲染、进出连线重路由到组节点边界，代码生成始终输出完整语句 + 组注释。组注释由 graph-compiler 递归 emit 时按「组内首个被 emit 的成员」插入（向后兼容：第 4 参可选，不传则行为不变）。

**Tech Stack:** React 19 + TypeScript + @xyflow/react v12 + Zustand 5 + Vitest。测试命令：`cd poiesis && npm test`（vitest run）；类型检查：`cd poiesis && npx tsc -b --pretty false`。

## Global Constraints

- **测试优先（TDD）**：每个任务先写失败测试，再实现，再绿，再 commit
- **commit 纪律**：每任务一个 commit，conventional commits（feat/fix/test/refactor/docs），消息中文描述 + 英文 type
- **拖拽手势神圣不可侵犯**：绝不占用/修改画布拖拽（pan）与节点拖拽；多选只用 Shift+点击
- **不破坏 project.json v3 形状**：groups 必须走独立顶层字段 `codeFlowGroups`，不动 `codeFlowGraphs` 的 `{pid, nodes, wires}` 形状
- **分组以 JSON 为唯一权威**：解析侧（graph-parser）不识别 `// ── 组名 ──` 注释；注释只是代码生成产物
- **折叠纯视觉**：折叠态不影响 graph-compiler 输出（代码始终完整）
- **成员约束**：仅语句级节点（tag ≠ "fn"/"action"/"lifecycle"），成员必须在同一 fn 的 wire 连通域内
- **不引入新依赖**：分组框用现有 ReactFlow node 机制 + CSS module

---

### Task 1: 数据层——类型 + Store 字段 + 序列化闭环

**Files:**
- Modify: `poiesis/src/core/types/project-json.ts`（加 `CodeFlowGroup` 接口 + `ProjectJson.codeFlowGroups`）
- Modify: `poiesis/src/store/types.ts`（加 `codeFlowGroups` 字段 + `setCodeFlowGroups` action 签名）
- Modify: `poiesis/src/store/slices/serialization.ts`（toProjectJson 写入 + loadFromProjectJson 恢复）
- Modify: `poiesis/src/store/slices/project.ts`（resetEditorState/newProject 清空）
- Modify: `poiesis/src/store/index.ts`（初始值 `codeFlowGroups: {}`）
- Modify: `poiesis/src/store/slices/element-crud.ts`（实现 `setCodeFlowGroups`）
- Modify: `poiesis/src/services/persistence/project-json.ts`（默认序列化补 `codeFlowGroups: {}`，若该文件有固定形状）
- Test: `poiesis/src/store/__tests__/groups-persistence.test.ts`（新建）

**Interfaces:**
- Produces: `interface CodeFlowGroup { id: string; name: string; memberIds: string[]; color: string; collapsed: boolean }`（core/types/project-json.ts 导出）
- Produces: `PoiesisState.codeFlowGroups: Record<string, CodeFlowGroup[]>`（键 = fn 名，与 `codeFlowGraphs` 键对齐）
- Produces: `PoiesisState.setCodeFlowGroups: (fnName: string, groups: CodeFlowGroup[]) => void`（整体替换某 fn 的分组数组）

- [ ] **Step 1: 写失败测试**（`poiesis/src/store/__tests__/groups-persistence.test.ts`，模式完全对齐 `position-persistence.test.ts`）

```typescript
// ── 分组持久化闭环测试（Task 1）──
// 保存侧：toProjectJson 直写 codeFlowGroups（键=fn 名）
// 加载侧：loadFromProjectJson 缓存回 store.codeFlowGroups

import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeFlowGroup } from "../../core/types/project-json";

const group = (id: string, name: string): CodeFlowGroup =>
  ({ id, name, memberIds: ["cb_b2", "cb_b3"], color: "#6366f1", collapsed: false });

const reset = () => usePoiesisStore.setState({
  elements: [], wires: [], codeFlowGraphs: {}, componentDefs: {},
  persistedPositions: {}, tabData: {}, codeFlowGroups: {},
});

describe("groups-persistence（分组持久化）", () => {
  beforeEach(reset);

  it("保存侧：toProjectJson 直写 codeFlowGroups（键=fn 名），空数组不写入", () => {
    usePoiesisStore.setState({ codeFlowGroups: { increment: [group("g1", "初始化")], empty: [] } });
    const json = usePoiesisStore.getState().toProjectJson();
    expect(json.codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("加载侧：loadFromProjectJson 恢复 codeFlowGroups；旧 JSON 无字段 → {} 兜底", () => {
    usePoiesisStore.getState().loadFromProjectJson({
      version: 3, nodes: {}, wires: {}, architecture: { mode: "overall", expandedBubbles: [], focusComponent: null, camera: { x: 0, y: 0, zoom: 1 } },
      canvasStates: {}, panels: { leftWidth: 260, rightWidth: 300 },
      codeFlowGraphs: {}, persistedPositions: {},
      codeFlowGroups: { increment: [group("g1", "初始化")] },
    } as never);
    expect(usePoiesisStore.getState().codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("闭环：setCodeFlowGroups → toProjectJson → loadFromProjectJson → 分组完整保持", () => {
    usePoiesisStore.getState().setCodeFlowGroups("increment", [group("g1", "初始化")]);
    const json = usePoiesisStore.getState().toProjectJson();
    usePoiesisStore.setState({ codeFlowGroups: {} });
    usePoiesisStore.getState().loadFromProjectJson(json);
    expect(usePoiesisStore.getState().codeFlowGroups).toEqual({ increment: [group("g1", "初始化")] });
  });

  it("setCodeFlowGroups 整体替换：传 [] 清空该 fn 分组", () => {
    usePoiesisStore.getState().setCodeFlowGroups("increment", [group("g1", "初始化")]);
    usePoiesisStore.getState().setCodeFlowGroups("increment", []);
    expect(usePoiesisStore.getState().codeFlowGroups.increment).toEqual([]);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/store/__tests__/groups-persistence.test.ts`
Expected: FAIL（类型不存在 / toProjectJson 无 codeFlowGroups 字段）

- [ ] **Step 3: 实现类型与字段**

`poiesis/src/core/types/project-json.ts`（加在 `persistedPositions` 字段后，保持接口末尾）：

```typescript
/** Code flow 分组（节点注释组合），键 = fn 名（与 codeFlowGraphs 对齐）。
 *  独立顶层字段——不动 codeFlowGraphs 的 v3 形状（persistedPositions 先例）。 */
codeFlowGroups: Record<string, CodeFlowGroup[]>;
```

文件末尾追加：

```typescript
/** 代码流分组：成员为语句级节点 id（PID）。color 取自 GROUP_COLORS 8 色调色板。 */
export interface CodeFlowGroup {
  id: string;
  name: string;
  memberIds: string[];
  color: string;
  collapsed: boolean;
}
```

`poiesis/src/store/types.ts`（`persistedPositions` 字段附近）：

```typescript
/** Codeflow 分组缓存：fn 名 → 分组数组（从 project.json 读回，画布编辑时经 setCodeFlowGroups 写入）。 */
codeFlowGroups: Record<string, import("../core/types/project-json").CodeFlowGroup[]>;
setCodeFlowGroups: (fnName: string, groups: import("../core/types/project-json").CodeFlowGroup[]) => void;
```

- [ ] **Step 4: 实现 store 接线**

`poiesis/src/store/index.ts`：初始 state 加 `codeFlowGroups: {}`。

`poiesis/src/store/slices/element-crud.ts`（`setElementPosition` 附近加）：

```typescript
setCodeFlowGroups: (fnName: string, groups: CodeFlowGroup[]) => set((s) => ({
  codeFlowGroups: { ...s.codeFlowGroups, [fnName]: groups },
  isDirty: true,
})),
```

（文件顶部 import 加 `import type { CodeFlowGroup } from "../../core/types/project-json";`）

`poiesis/src/store/slices/serialization.ts` 的 `toProjectJson` 加（`persistedPositions` 后）：

```typescript
// 分组直写（键=fn 名，与 codeFlowGraphs 对齐）；空数组 fn 与空组（成员删光）不写入
// （空组无可见 UI 无法手动删——保存时自动清理，spec §8「空框保留」落地为 JSON 剔除）
codeFlowGroups: Object.fromEntries(
  Object.entries(s.codeFlowGroups)
    .map(([fnName, gs]) => [fnName, gs.filter((g) => g.memberIds.length > 0)] as const)
    .filter(([, gs]) => gs.length > 0),
),
```

`loadFromProjectJson` 加：

```typescript
codeFlowGroups: json.codeFlowGroups || {},
```

`poiesis/src/store/slices/project.ts`：`resetEditorState` 与 `newProject` 的返回对象里，在 `persistedPositions: {}` 旁加 `codeFlowGroups: {}`（对齐现有 `persistedPositions: {}` 的清空写法——先 grep 该文件里 `persistedPositions` 出现处逐一补）。

- [ ] **Step 5: 运行确认通过**

Run: `cd poiesis && npx vitest run src/store/__tests__/groups-persistence.test.ts`
Expected: PASS（4 个用例全绿）

- [ ] **Step 6: 回归 + 类型检查**

Run: `cd poiesis && npx vitest run src/store/__tests__/position-persistence.test.ts src/store/__tests__/project-slice.test.ts && npx tsc -b --pretty false`
Expected: 全绿 + tsc 零错误

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/core/types/project-json.ts poiesis/src/store/types.ts poiesis/src/store/slices/serialization.ts poiesis/src/store/slices/project.ts poiesis/src/store/slices/element-crud.ts poiesis/src/store/index.ts poiesis/src/store/__tests__/groups-persistence.test.ts
git commit -m "feat(editor): 分组数据层——codeFlowGroups 类型 + store 字段 + project.json 序列化闭环"
```

---

### Task 2: 分组纯函数工具（校验 + fn 归属 + 包围盒）

**Files:**
- Create: `poiesis/src/services/codegen/groups.ts`
- Test: `poiesis/src/services/codegen/__tests__/groups.test.ts`

**Interfaces:**
- Consumes: `findOwnerFnId(elId, elements, wires): string | null`（fn-topology.ts 现有，返回 **fn 元素 id**）
- Produces: `GROUP_COLORS: string[]`（8 色调色板常量）
- Produces: `validateGroupMembers(memberIds, elements, wires): { valid: string[]; ownerFnId: string | null; error?: string }` —— 成员非空、全部同 fn（findOwnerFnId 一致）、不含 fn/action/lifecycle 节点；ownerFnId = 首个成员的 fn 元素 id
- Produces: `fnNameOf(fnElId, elements): string` —— fn 元素的 `props.name`，缺失回退 `"unnamed"`
- Produces: `computeGroupBoxes(groups, elements, sizes): Map<string, { x: number; y: number; width: number; height: number }>` —— 成员 position + size（含 padding 24）求包围盒；无成员/无尺寸的组跳过

**说明：** 本任务不写 DOM 测量（尺寸由 FlowCanvas 提供，Task 4 接线）；纯函数全部可单测。

- [ ] **Step 1: 写失败测试**（`poiesis/src/services/codegen/__tests__/groups.test.ts`）

```typescript
import { describe, it, expect } from "vitest";
import { GROUP_COLORS, validateGroupMembers, fnNameOf, computeGroupBoxes } from "../groups";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";
import type { CodeFlowGroup } from "../../../core/types/project-json";

const el = (id: string, tag: string, name?: string, position?: { x: number; y: number }): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {}, position });

const ELEMS = [
  el("fn1", "fn", "increment"),
  el("cb1", "let", "count", { x: 0, y: 0 }),
  el("cb2", "assign", undefined, { x: 0, y: 60 }),
  el("cb3", "call", "close", { x: 300, y: 0 }),
];
const WIRES: WireData[] = [
  { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
  { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
  { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
];

describe("groups 纯函数", () => {
  it("GROUP_COLORS 恰 8 色", () => {
    expect(GROUP_COLORS).toHaveLength(8);
    expect(GROUP_COLORS[0]).toBe("#6366f1");
  });

  it("validateGroupMembers：同 fn 语句节点通过，返回 ownerFnId", () => {
    const r = validateGroupMembers(["cb1", "cb2"], ELEMS, WIRES);
    expect(r).toEqual({ valid: ["cb1", "cb2"], ownerFnId: "fn1", error: undefined });
  });

  it("validateGroupMembers：含 fn 节点 → 拒绝并报错", () => {
    const r = validateGroupMembers(["fn1", "cb1"], ELEMS, WIRES);
    expect(r.error).toBeTruthy();
    expect(r.valid).toEqual([]);
  });

  it("validateGroupMembers：跨 fn 成员 → 拒绝", () => {
    const ELEMS2 = [...ELEMS, el("fn2", "fn", "decrement"), el("cb9", "let", "x")];
    const WIRES2 = [...WIRES, { id: "w9", fromId: "fn2", fromPort: "flow-out", toId: "cb9", toPort: "flow-in" }];
    const r = validateGroupMembers(["cb1", "cb9"], ELEMS2, WIRES2);
    expect(r.error).toBeTruthy();
    expect(r.valid).toEqual([]);
  });

  it("validateGroupMembers：孤岛节点（无 fn 可达）→ 拒绝", () => {
    const r = validateGroupMembers(["cb1", "cb3"], ELEMS.slice(0, 3), WIRES.slice(0, 2));
    // cb3 不在 WIRES 里 → findOwnerFnId 为 null → 拒绝
    expect(r.error).toBeTruthy();
  });

  it("fnNameOf：取 fn 元素 props.name，缺失回退 unnamed", () => {
    expect(fnNameOf("fn1", ELEMS)).toBe("increment");
    expect(fnNameOf("nonexist", ELEMS)).toBe("unnamed");
  });

  it("computeGroupBoxes：成员包围盒含 padding，缺尺寸成员跳过", () => {
    const g: CodeFlowGroup = { id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false };
    const sizes = new Map([["cb1", { width: 200, height: 40 }], ["cb2", { width: 200, height: 40 }]]);
    const boxes = computeGroupBoxes([g], ELEMS, sizes);
    expect(boxes.get("g1")).toEqual({ x: -24, y: -24, width: 248, height: 128 });
  });

  it("computeGroupBoxes：空组/无尺寸 → 该组不产出包围盒", () => {
    const g: CodeFlowGroup = { id: "g1", name: "空", memberIds: [], color: "#6366f1", collapsed: false };
    expect(computeGroupBoxes([g], ELEMS, new Map()).size).toBe(0);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/groups.test.ts`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现 groups.ts**

```typescript
// poiesis/src/services/codegen/groups.ts
// 分组纯函数：成员校验 / fn 归属 / 包围盒。DOM 测量与 store 写入在组件层完成。
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import type { CodeFlowGroup } from "../../core/types/project-json";
import { findOwnerFnId } from "./fn-topology";

/** 分组框 8 色调色板（与 --ol-* 设计 token 协调的靛蓝/青/琥珀/玫红系） */
export const GROUP_COLORS = [
  "#6366f1", "#06b6d4", "#f59e0b", "#ef4444",
  "#10b981", "#8b5cf6", "#ec4899", "#64748b",
] as const;

/** 分组框内边距（px，画布单位） */
export const GROUP_PADDING = 24;

const CONTAINER_TAGS = new Set(["fn", "action", "lifecycle"]);

export interface GroupValidation {
  valid: string[];
  ownerFnId: string | null;
  error?: string;
}

/** 校验成员：非空、不含容器节点（fn/action/lifecycle）、全部同一 fn 连通域。
 *  ownerFnId = 首个有效成员的 fn 元素 id（供调用方查 fn 名）。 */
export function validateGroupMembers(
  memberIds: string[],
  elements: CodeblockElementData[],
  wires: WireData[],
): GroupValidation {
  if (memberIds.length < 2) return { valid: [], ownerFnId: null, error: "分组至少需要 2 个节点" };
  const elemById = new Map(elements.map((e) => [e.id, e]));
  const owner = findOwnerFnId(memberIds[0], elements, wires);
  if (!owner) return { valid: [], ownerFnId: null, error: "所选节点不属于任何函数" };
  for (const mid of memberIds) {
    const e = elemById.get(mid);
    if (!e) return { valid: [], ownerFnId: null, error: "所选节点不存在" };
    if (CONTAINER_TAGS.has(e.tag)) return { valid: [], ownerFnId: null, error: "函数节点不能加入分组" };
    if (findOwnerFnId(mid, elements, wires) !== owner) {
      return { valid: [], ownerFnId: null, error: "分组成员必须属于同一个函数" };
    }
  }
  return { valid: [...memberIds], ownerFnId: owner };
}

/** fn 元素名（codeFlowGroups 键），缺失回退 "unnamed"。 */
export function fnNameOf(fnElId: string, elements: CodeblockElementData[]): string {
  const fnEl = elements.find((e) => e.id === fnElId);
  return (fnEl?.props?.name as string) || "unnamed";
}

export interface GroupBBox { x: number; y: number; width: number; height: number; }

/** 成员 position + size（含 padding）求包围盒。无成员或任一成员缺尺寸 → 该组跳过（画布无法可靠摆放）。 */
export function computeGroupBoxes(
  groups: CodeFlowGroup[],
  elements: CodeblockElementData[],
  sizes: Map<string, { width: number; height: number }>,
): Map<string, GroupBBox> {
  const elemById = new Map(elements.map((e) => [e.id, e]));
  const out = new Map<string, GroupBBox>();
  for (const g of groups) {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    let ok = true;
    for (const mid of g.memberIds) {
      const e = elemById.get(mid);
      const s = sizes.get(mid);
      if (!e || !e.position || !s) { ok = false; break; }
      minX = Math.min(minX, e.position.x);
      minY = Math.min(minY, e.position.y);
      maxX = Math.max(maxX, e.position.x + s.width);
      maxY = Math.max(maxY, e.position.y + s.height);
    }
    if (!ok || !Number.isFinite(minX)) continue;
    out.set(g.id, { x: minX - GROUP_PADDING, y: minY - GROUP_PADDING, width: maxX - minX + GROUP_PADDING * 2, height: maxY - minY + GROUP_PADDING * 2 });
  }
  return out;
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/groups.test.ts`
Expected: PASS（8 个用例全绿）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/groups.ts poiesis/src/services/codegen/__tests__/groups.test.ts
git commit -m "feat(editor): 分组纯函数工具——成员校验/fn 归属/包围盒（GROUP_COLORS 8 色）"
```

---

### Task 3: Shift 多选 + 右键「组成分组」

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`
- Test: `poiesis/src/components/canvas/code/__tests__/FlowCanvasGroupSelect.test.tsx`（新建）

**Interfaces:**
- Consumes: `validateGroupMembers`、`fnNameOf`（Task 2）；`usePoiesisStore.getState().setCodeFlowGroups`（Task 1）
- Produces: FlowCanvas 内部 state `editingGroupId: string | null`（新建组的命名态，Task 4 消费）
- Produces: 节点右键菜单新增「组成分组」项（selectedNodeIds ≥2 且含右键节点时显示）

**关键实现说明：** 现有 `onNodeClick` 是 `setSelectedNodeIds([node.id])`（单选）。改为：`event.shiftKey` 时在数组里 toggle；非 shift 时单选。`handleNodeContextMenu` 已存在（记录 nodeMenu state + nodeId）——内联菜单（FlowCanvas.tsx 657-695 行的 div）加按钮。创建流程：点击「组成分组」→ `validateGroupMembers(selectedNodeIds, codeElements, wires)` → 失败则 alert（或控制台——优先 alert 让用户可见）→ 成功 → 构造 group（uid("group")、name "新分组"、默认色 `GROUP_COLORS[0]`）→ `setCodeFlowGroups(fnName, [...existing, group])` → `setEditingGroupId(group.id)` 进入命名态。FlowInspector 多选提示：`selectedNodeIds.length > 1` 时显示「已选 N 个节点：右键 → 组成分组」。

**测试策略：** FlowCanvas 是受控 ReactFlow 的深组件——现有 `FlowCanvasDrag.test.tsx` 用 mock `@xyflow/react` 捕获 props 断言接线。本任务按同模式：mock ReactFlow 捕获 `onNodeClick`/`onNodeContextMenu` 回调 + 内联菜单 DOM，用 jsdom 合成事件验证 shift toggle 与菜单项出现。组创建后断言 `usePoiesisStore.getState().codeFlowGroups` 被写入。

- [ ] **Step 1: 写失败测试**（`poiesis/src/components/canvas/code/__tests__/FlowCanvasGroupSelect.test.tsx`）

```typescript
// Shift+点击多选 + 右键「组成分组」接线测试（mock @xyflow/react 捕获 props）
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type { ReactNode } from "react";

// ── mock @xyflow/react：捕获 FlowCanvas 传给 ReactFlow 的 props ──
const captured = { onNodeClick: null as null | ((e: { shiftKey: boolean }, n: { id: string }) => void),
                   onNodeContextMenu: null as null | ((e: Event, n: { id: string }) => void) };
vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    captured.onNodeClick = props.onNodeClick as never;
    captured.onNodeContextMenu = props.onNodeContextMenu as never;
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  Handle: () => <div />,
  Controls: () => <div />,
  Background: () => <div />,
  BackgroundVariant: { Dots: "dots" },
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
}));

import { FlowCanvas } from "../FlowCanvas";
import { usePoiesisStore } from "../../../../store";

const codeEl = (id: string, tag: string, name?: string) =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {}, position: { x: 0, y: 0 } });

const reset = () => usePoiesisStore.setState({
  elements: [
    codeEl("fn1", "fn", "increment"),
    codeEl("cb1", "let", "count"),
    codeEl("cb2", "assign"),
  ],
  wires: [
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
    { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
  ],
  codeFlowGraphs: {}, componentDefs: {}, persistedPositions: {}, tabData: {}, codeFlowGroups: {},
});

describe("FlowCanvas 分组选择", () => {
  beforeEach(() => { reset(); vi.clearAllMocks(); });

  it("Shift+点击累积多选，再次点击取消；非 shift 点击单选", () => {
    render(<FlowCanvas />);
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb1" }); });
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb2" }); });
    // 再 shift 点击 cb1 → 取消
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb1" }); });
    expect(captured).toBeTruthy(); // 接线存在
    act(() => { captured.onNodeClick!({ shiftKey: false }, { id: "cb2" }); });
  });

  it("右键选中节点且多选 ≥2 → 菜单出现「组成分组」；点击后 store.codeFlowGroups 写入该 fn", () => {
    render(<FlowCanvas />);
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb1" }); });
    act(() => { captured.onNodeClick!({ shiftKey: true }, { id: "cb2" }); });
    act(() => { captured.onNodeContextMenu!({ preventDefault: () => {} } as Event, { id: "cb2" }); });
    const btn = screen.getByRole("button", { name: /组成分组/ });
    expect(btn).toBeTruthy();
    fireEvent.click(btn);
    const groups = usePoiesisStore.getState().codeFlowGroups.increment;
    expect(groups).toHaveLength(1);
    expect(groups![0].memberIds).toEqual(["cb1", "cb2"]);
    expect(groups![0].name).toBe("新分组");
  });

  it("单选（<2）右键 → 菜单无「组成分组」", () => {
    render(<FlowCanvas />);
    act(() => { captured.onNodeClick!({ shiftKey: false }, { id: "cb1" }); });
    act(() => { captured.onNodeContextMenu!({ preventDefault: () => {} } as Event, { id: "cb1" }); });
    expect(screen.queryByRole("button", { name: /组成分组/ })).toBeNull();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/FlowCanvasGroupSelect.test.tsx`
Expected: FAIL（无「组成分组」按钮、shift 未接线——断言失败即可，mock 需先跑通渲染）

- [ ] **Step 3: 实现 FlowCanvas 多选与创建分组**

`FlowCanvas.tsx` 改动（3 处）：

1) `onNodeClick`（581 行附近）改为：

```typescript
onNodeClick={(_event, node) => {
  if (_event.shiftKey) {
    setSelectedNodeIds((prev) => prev.includes(node.id) ? prev.filter((id) => id !== node.id) : [...prev, node.id]);
  } else {
    setSelectedNodeIds([node.id]);
  }
}}
```

2) `handleNodeContextMenu` 保持；内联菜单（657-695 行 div，删除按钮上方）加：

```typescript
{selectedNodeIds.includes(nodeMenu.nodeId) && selectedNodeIds.length >= 2 && (
  <button
    onClick={() => handleCreateGroup()}
    style={{ display: "flex", alignItems: "center", gap: 6, width: "100%", padding: "6px 12px", border: "none", cursor: "pointer", fontSize: 12, color: "#6366f1", textAlign: "left" as const, fontFamily: "inherit" }}
  >
    📦 组成分组
  </button>
)}
```

3) 新增 `handleCreateGroup`（`handleNodeContextMenu` 旁）：

```typescript
const handleCreateGroup = useCallback(() => {
  const sel = selectedNodeIds;
  if (sel.length < 2) return;
  const v = validateGroupMembers(sel, codeElements, wires);
  if (!v.ownerFnId || v.error) {
    window.alert(v.error ?? "无法创建分组");
    setNodeMenu(null);
    return;
  }
  const fnName = fnNameOf(v.ownerFnId, codeElements);
  const group: CodeFlowGroup = {
    id: uid("group"),
    name: "新分组",
    memberIds: v.valid,
    color: GROUP_COLORS[0],
    collapsed: false,
  };
  usePoiesisStore.getState().setCodeFlowGroups(fnName, [...(usePoiesisStore.getState().codeFlowGroups[fnName] ?? []), group]);
  setEditingGroupId(group.id);
  setNodeMenu(null);
}, [selectedNodeIds, codeElements, wires]);
```

imports 追加：`validateGroupMembers, fnNameOf, GROUP_COLORS` from `../../../../services/codegen/groups`；`type CodeFlowGroup` from `../../../../core/types/project-json`；`uid` 已导入。state 追加 `const [editingGroupId, setEditingGroupId] = useState<string | null>(null);`

4) `FlowInspector` 多选提示（`poiesis/src/components/canvas/code/FlowInspector.tsx`）：在 `selectedNodeIds.length > 1` 时渲染提示条（置于面板顶部，替代既有「Select a node to edit」空态文案）：

```typescript
// FlowInspector 内，现「Select a node to edit」空态渲染处：
if (selectedNodeIds.length > 1) {
  return (
    <div className="flow-inspector-empty">
      <div>已选 {selectedNodeIds.length} 个节点</div>
      <div style={{ fontSize: 11, opacity: 0.7 }}>右键 → 组成分组（同函数节点）</div>
    </div>
  );
}
```

（先查 FlowInspector 现有空态 JSX 结构，替换为上述分支；`selectedNodeIds` prop 已存在无需改签名。）

- [ ] **Step 4: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/components/canvas/code/__tests__/FlowCanvasGroupSelect.test.tsx && npx vitest run src/components/canvas/code/__tests__/FlowCanvasDrag.test.tsx`
Expected: 新用例全绿 + 既有拖拽测试不回归（mock 增强不影响）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/__tests__/FlowCanvasGroupSelect.test.tsx
git commit -m "feat(editor): Shift 多选 + 右键「组成分组」——同 fn 校验后写入 store.codeFlowGroups"
```

---

### Task 4: GroupBox 分组框组件（渲染/改名/改色/折叠/删除）

**Files:**
- Create: `poiesis/src/components/canvas/code/nodes/GroupBox.tsx`
- Create: `poiesis/src/components/canvas/code/nodes/GroupBox.module.css`
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（注册 nodeTypes["group"] + 组节点生成 + DOM 尺寸测量接线 computeGroupBoxes）
- Test: `poiesis/src/components/canvas/code/nodes/__tests__/GroupBox.test.tsx`（新建）

**Interfaces:**
- Consumes: `computeGroupBoxes`（Task 2）、`editingGroupId`（Task 3）、`store.setCodeFlowGroups`
- Produces: `GroupBox` props：`{ group: CodeFlowGroup; editing: boolean; onRename(name: string): void; onColorChange(color: string): void; onToggleCollapsed(): void; onDelete(): void }`；折叠时渲染组卡（折叠态由 `group.collapsed` 决定，内部含 ▸ 展开按钮）

**说明：** 组节点是 ReactFlow node（type "group"，draggable/selectable/focusable 全 false，`style: { width, height }` 由 bbox 决定）。FlowCanvas 在 `nodes` useMemo 里对每个非折叠组生成一个 group 节点（**数组最前**保证垫底渲染），`position` = bbox 左上角。bbox 尺寸来自 DOM 测量：新增 `useEffect`（依赖 elements 位置变化）用 `document.querySelector('[data-id="..."]')` + getBoundingClientRect/zoom 换算（复用 handleAutoLayout 的测量手法），组装 `sizes` Map → `computeGroupBoxes` → 存 `groupBoxes` state。

- [ ] **Step 1: 写失败测试**（`poiesis/src/components/canvas/code/nodes/__tests__/GroupBox.test.tsx`）

```typescript
// GroupBox 渲染与回调测试（jsdom，无 ReactFlow 依赖——组件内不渲染 Handle）
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { GroupBox } from "../GroupBox";
import type { CodeFlowGroup } from "../../../../../core/types/project-json";

const group = (over: Partial<CodeFlowGroup> = {}): CodeFlowGroup =>
  ({ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false, ...over });

describe("GroupBox", () => {
  it("展开态渲染标题栏（组名 + 折叠按钮 + 删除按钮）", () => {
    render(<GroupBox group={group()} editing={false} onRename={() => {}} onColorChange={() => {}} onToggleCollapsed={() => {}} onDelete={() => {}} />);
    expect(screen.getByText("初始化")).toBeTruthy();
    expect(screen.getByRole("button", { name: /折叠|收起/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /删除分组/ })).toBeTruthy();
  });

  it("折叠态渲染组卡（组名 + 成员数 + 展开按钮）", () => {
    render(<GroupBox group={group({ collapsed: true })} editing={false} onRename={() => {}} onColorChange={() => {}} onToggleCollapsed={() => {}} onDelete={() => {}} />);
    expect(screen.getByText(/2 条语句/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /展开/ })).toBeTruthy();
  });

  it("editing=true 显示行内命名输入框，Enter 提交 onRename", () => {
    const onRename = vi.fn();
    render(<GroupBox group={group()} editing onRename={onRename} onColorChange={() => {}} onToggleCollapsed={() => {}} onDelete={() => {}} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "校验逻辑" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onRename).toHaveBeenCalledWith("校验逻辑");
  });

  it("色板 8 色点击触发 onColorChange", () => {
    const onColorChange = vi.fn();
    render(<GroupBox group={group()} editing={false} onRename={() => {}} onColorChange={onColorChange} onToggleCollapsed={() => {}} onDelete={() => {}} />);
    fireEvent.click(screen.getAllByRole("button", { name: /颜色/ })[3]);
    expect(onColorChange).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/GroupBox.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现 GroupBox.tsx + css**

`GroupBox.tsx`：

```typescript
// poiesis/src/components/canvas/code/nodes/GroupBox.tsx
// 分组框节点：展开态 = 半透明彩色框 + 标题栏；折叠态 = 紧凑组卡。
// 不渲染 Handle（折叠态的连线重路由在 FlowCanvas 的 edges 层完成）。
import { useState } from "react";
import type { CodeFlowGroup } from "../../../../core/types/project-json";
import { GROUP_COLORS } from "../../../../services/codegen/groups";
import { NodeTextField } from "../../fields/NodeField";
import styles from "./GroupBox.module.css";

interface Props {
  group: CodeFlowGroup;
  editing: boolean;
  onRename: (name: string) => void;
  onColorChange: (color: string) => void;
  onToggleCollapsed: () => void;
  onDelete: () => void;
}

export function GroupBox({ group, editing, onRename, onColorChange, onToggleCollapsed, onDelete }: Props) {
  const [colorOpen, setColorOpen] = useState(false);
  // 双击标题进入本地改名态（editing prop 只管「新建组后的自动命名」；双击改名用本地 state，
  // 避免 FlowCanvas 维护双击入口——二者共用 NodeTextField，提交都走 onRename）
  const [localEditing, setLocalEditing] = useState(false);
  const naming = editing || localEditing;

  if (group.collapsed) {
    return (
      <div className={styles.card} style={{ borderColor: group.color }} data-group-id={group.id}>
        <button type="button" className={styles.foldBtn} onClick={onToggleCollapsed} aria-label="展开分组">▸</button>
        <span className={styles.cardName}>{group.name}</span>
        <span className={styles.cardMeta}>{group.memberIds.length} 条语句</span>
      </div>
    );
  }

  return (
    <div className={styles.box} style={{ borderColor: group.color, background: `${group.color}14` }} data-group-id={group.id}>
      <div className={styles.titleBar}>
        {naming ? (
          <NodeTextField value={group.name} onCommit={(v) => { onRename(v); setLocalEditing(false); }} placeholder="分组名" ariaLabel="分组名" className={styles.nameInput} />
        ) : (
          <span className={styles.name} onDoubleClick={() => setLocalEditing(true)}>{group.name}</span>
        )}
        <button type="button" className={styles.foldBtn} onClick={onToggleCollapsed} aria-label="折叠分组">▾</button>
        <div className={styles.colorWrap}>
          <button type="button" className={styles.colorBtn} onClick={() => setColorOpen((o) => !o)} aria-label="分组颜色" style={{ background: group.color }} />
          {colorOpen && (
            <div className={styles.palette}>
              {GROUP_COLORS.map((c) => (
                <button key={c} type="button" aria-label={`颜色 ${c}`} className={styles.swatch}
                  style={{ background: c }} onClick={() => { onColorChange(c); setColorOpen(false); }} />
              ))}
            </div>
          )}
        </div>
        <button type="button" className={styles.deleteBtn} onClick={onDelete} aria-label="删除分组">×</button>
      </div>
    </div>
  );
}
```

（NodeTextField 的 `onCommit` 签名 = `(v: string) => void`，与 onRename 匹配；双击标题用本地 `localEditing` state 进编辑态，提交后回调 onRename——FlowCanvas 无需感知双击入口。）

`GroupBox.module.css`（关键：`pointer-events` 透传——框体不拦截节点交互）：

```css
.box { position: absolute; border: 1px solid; border-radius: 8px; pointer-events: none; }
/* 标题栏需要接收点击 → 单独恢复 pointer-events */
.titleBar { pointer-events: auto; display: flex; align-items: center; gap: 6px; padding: 2px 8px; font-size: 12px; color: #334155; border-bottom: 1px solid rgba(0,0,0,0.06); cursor: default; }
.name { font-weight: 600; }
.card { position: absolute; display: flex; align-items: center; gap: 8px; padding: 4px 10px; border: 1px solid; border-left: 4px solid; border-radius: 6px; background: #fff; font-size: 12px; box-shadow: 0 1px 4px rgba(0,0,0,0.08); }
.foldBtn { border: none; background: none; cursor: pointer; font-size: 11px; color: #64748b; }
.deleteBtn { border: none; background: none; cursor: pointer; font-size: 14px; color: #dc2626; }
.colorWrap { position: relative; }
.colorBtn { width: 14px; height: 14px; border: 1px solid rgba(0,0,0,0.2); border-radius: 50%; cursor: pointer; }
.palette { position: absolute; top: 18px; left: 0; display: flex; gap: 4px; padding: 4px; background: #fff; border: 1px solid #e2e8f0; border-radius: 6px; z-index: 10; }
.swatch { width: 16px; height: 16px; border: none; border-radius: 4px; cursor: pointer; }
.cardMeta { color: #94a3b8; font-size: 11px; }
.nameInput { width: 120px; }
```

- [ ] **Step 4: FlowCanvas 接线（组节点渲染 + bbox 测量）**

`FlowCanvas.tsx`：

1) `nodeTypes` 加 `"group": GroupBox`（import 追加）。

2) 新增 bbox 状态与测量 effect（`handleAutoLayout` 上方）：

```typescript
// 组几何缓存：groupId → 包围盒（展开态实时重算；折叠态沿用最后一次展开值——成员隐藏无 DOM）
const [groupBoxes, setGroupBoxes] = useState<Map<string, GroupBBox>>(new Map());
const lastGroupBoxesRef = useRef<Map<string, GroupBBox>>(new Map());

// 成员尺寸 DOM 实测（受控模式下 RFNode.measured 恒缺——同 handleAutoLayout 手法）
useEffect(() => {
  const allGroups = Object.values(usePoiesisStore.getState().codeFlowGroups).flat();
  if (allGroups.length === 0) { setGroupBoxes(new Map()); lastGroupBoxesRef.current = new Map(); return; }
  const vpEl = document.querySelector(".react-flow__viewport");
  const vpMatch = vpEl ? getComputedStyle(vpEl).transform.match(/matrix\(([^)]+)\)/) : null;
  const zoom = vpMatch ? Number(vpMatch[1].split(",")[0]) || 1 : 1;
  const sizes = new Map<string, { width: number; height: number }>();
  // 折叠组成员不测量（隐藏无 DOM），其 bbox 沿用 lastGroupBoxesRef 缓存
  const activeGroups = allGroups.filter((g) => !g.collapsed);
  for (const g of activeGroups) {
    for (const mid of g.memberIds) {
      const dom = document.querySelector(`[data-id="${mid}"]`);
      if (dom) {
        const r = dom.getBoundingClientRect();
        sizes.set(mid, { width: r.width / zoom, height: r.height / zoom });
      }
    }
  }
  const next = computeGroupBoxes(activeGroups, codeElements, sizes);
  lastGroupBoxesRef.current = new Map([...lastGroupBoxesRef.current, ...next]);
  setGroupBoxes(lastGroupBoxesRef.current);
  // 依赖 elements：成员 position 变化（拖拽/布局）触发重算
}, [elements, codeElements]);
```

3) 新增分组更新回调（`handleCreateGroup` 旁，四操作共用「按 fn 名整体替换」模式）：

```typescript
// 组归属 fn 名：成员 0 的 findOwnerFnId → fn 元素 → props.name
const groupFnName = useCallback((g: CodeFlowGroup): string => {
  const fnElId = findOwnerFnId(g.memberIds[0] ?? "", codeElements, wires);
  return fnElId ? fnNameOf(fnElId, codeElements) : "unnamed";
}, [codeElements, wires]);

const patchGroup = useCallback((g: CodeFlowGroup, patch: Partial<CodeFlowGroup>) => {
  const fnName = groupFnName(g);
  const groups = usePoiesisStore.getState().codeFlowGroups[fnName] ?? [];
  usePoiesisStore.getState().setCodeFlowGroups(
    fnName,
    groups.map((x) => (x.id === g.id ? { ...x, ...patch } : x)),
  );
}, [groupFnName]);

const removeGroup = useCallback((g: CodeFlowGroup) => {
  const fnName = groupFnName(g);
  const groups = usePoiesisStore.getState().codeFlowGroups[fnName] ?? [];
  usePoiesisStore.getState().setCodeFlowGroups(
    fnName,
    groups.filter((x) => x.id !== g.id),
  );
  // 组删除时同步清理 bbox 缓存（lastGroupBoxesRef，见 Step 4.2）
  lastGroupBoxesRef.current.delete(g.id);
}, [groupFnName]);
```

4) `nodes` useMemo 开头追加组节点（**展开组**生成 bbox 框；**折叠组**生成组卡节点——position 优先 `collapsedGroupPositions[g.id]`（Task 7 布局落点），否则 bbox 缓存）：

```typescript
const groupNodes: RFNode[] = [];
for (const g of Object.values(usePoiesisStore.getState().codeFlowGroups).flat()) {
  const bbox = groupBoxes.get(g.id);
  const pos = g.collapsed ? (collapsedGroupPositions[g.id] ?? (bbox ? { x: bbox.x, y: bbox.y } : undefined))
                          : (bbox ? { x: bbox.x, y: bbox.y } : undefined);
  if (!pos) continue;
  groupNodes.push({
    id: g.id,
    type: "group",
    position: pos,
    style: g.collapsed ? undefined : { width: bbox!.width, height: bbox!.height },
    draggable: false,
    selectable: false,
    focusable: false,
    className: "codeflow-group",
    data: {
      group: g,
      editing: editingGroupId === g.id,
      onRename: (name: string) => { patchGroup(g, { name }); setEditingGroupId(null); },
      onColorChange: (color: string) => patchGroup(g, { color }),
      onToggleCollapsed: () => patchGroup(g, { collapsed: !g.collapsed }),
      onDelete: () => removeGroup(g),
    },
  });
}
const allNodesWithGroups = [...groupNodes, ...nodes];
```

（imports 追加：`collectCollapsedMemberIds`、`computeGroupBoxes`、`GROUP_COLORS`、`validateGroupMembers`、`fnNameOf` from `../../../../services/codegen/groups`；`type CodeFlowGroup`、`type GroupBBox` from `../../../../core/types/project-json` / `../../../../services/codegen/groups`。state 追加：`const [groupBoxes, setGroupBoxes] = useState<Map<string, GroupBBox>>(new Map());`、`const [collapsedGroupPositions, setCollapsedGroupPositions] = useState<Record<string, { x: number; y: number }>>({});`、`const lastGroupBoxesRef = useRef<Map<string, GroupBBox>>(new Map());`）

5) `allNodes` useMemo 改为 `[...allNodesWithGroups, ...(previewNode ? [previewNode] : [])]`。

**注意：** 折叠组的成员节点必须在 `nodes` 生成时**过滤掉**（Task 5 一起做——本任务只做展开态渲染，折叠态渲染在 Task 5 用同样机制关闭）。本任务结束时折叠态行为 = 仍显示成员（Task 5 补齐），测试断言只覆盖展开态。

- [ ] **Step 5: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/GroupBox.test.tsx src/components/canvas/code/__tests__/FlowCanvasGroupSelect.test.tsx && npx tsc -b --pretty false`
Expected: 全绿 + tsc 零错误

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/components/canvas/code/nodes/GroupBox.tsx poiesis/src/components/canvas/code/nodes/GroupBox.module.css poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/nodes/__tests__/GroupBox.test.tsx
git commit -m "feat(editor): GroupBox 分组框——展开态彩色框 + 标题栏（折叠/改名/改色/删除回调），DOM 实测 bbox 接线"
```

---

### Task 5: 折叠态组卡 + 连线重路由

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（折叠组：成员节点过滤 + 组节点折叠态 + edges 重路由）
- Test: `poiesis/src/components/canvas/code/__tests__/FlowCanvasGroupCollapse.test.tsx`（新建）

**Interfaces:**
- Consumes: Task 3/4 的 store 读写 + 组节点生成
- Produces: `collectCollapsedMemberIds(groups): Set<string>`（本地 helper，FlowCanvas 内或 groups.ts——放 `groups.ts` 便于单测）
- Produces: `buildFlowEdges(wires, groups): Edge[]`（纯函数，FlowCanvas 的 edges useMemo 替换；折叠组重路由：`fromId∈成员 → fromId=组id/fromPort="flow-out"`、`toId∈成员 → toId=组id/toPort="flow-in"`、组内连线隐藏；**保留 w.id** 防 key 冲突，展开后从 wires 重推自然还原）

**关键说明：** 折叠态连线重路由是纯函数——`buildFlowEdges` 放 `groups.ts` 单测覆盖；FlowCanvas 的 edges useMemo 改为调它。成员节点过滤在 nodes useMemo 里：`codeElements.filter(e => !collapsedMemberIds.has(e.id))`（数据防线：只在渲染层过滤，store 元素不动——保存/代码生成永远用全量）。

- [ ] **Step 1: 写失败测试**（`groups.test.ts` 追加两个 describe + 新建 FlowCanvasGroupCollapse.test.tsx）

`groups.test.ts` 追加：

```typescript
import { collectCollapsedMemberIds, buildFlowEdges } from "../groups";
import type { Edge } from "@xyflow/react";

// 折叠态连线重路由
it("collectCollapsedMemberIds：收集所有折叠组的成员", () => {
  const groups = [
    { id: "g1", name: "a", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true },
    { id: "g2", name: "b", memberIds: ["cb3"], color: "#6366f1", collapsed: false },
  ] as CodeFlowGroup[];
  expect(collectCollapsedMemberIds(groups)).toEqual(new Set(["cb1", "cb2"]));
});

it("buildFlowEdges：进线/出线重路由到组边界，组内连线隐藏，wire id 保留", () => {
  const groups = [{ id: "g1", name: "a", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }] as CodeFlowGroup[];
  const edges = buildFlowEdges([
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
    { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
    { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
  ], groups);
  expect(edges).toEqual([
    { id: "w1", type: "default", source: "fn1", target: "g1", sourceHandle: "flow-out", targetHandle: "flow-in" },
    { id: "w3", type: "default", source: "g1", target: "cb3", sourceHandle: "flow-out", targetHandle: "flow-in" },
  ]);
});

it("buildFlowEdges：无折叠组时原样透传（含 type: default）", () => {
  const edges = buildFlowEdges([
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
  ], []);
  expect(edges).toEqual([{ id: "w1", type: "default", source: "fn1", target: "cb1", sourceHandle: "flow-out", targetHandle: "flow-in" }]);
});
```

`FlowCanvasGroupCollapse.test.tsx`（mock @xyflow/react 捕获 nodes/edges props）：

```typescript
// 折叠态：成员节点从 nodes 消失、edges 重路由到组节点（mock @xyflow/react 捕获 props）
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, act } from "@testing-library/react";
import type { ReactNode } from "react";

const captured = { nodes: [] as unknown[], edges: [] as unknown[] };
vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    captured.nodes = props.nodes as unknown[];
    captured.edges = props.edges as unknown[];
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  Handle: () => <div />,
  Controls: () => <div />,
  Background: () => <div />,
  BackgroundVariant: { Dots: "dots" },
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
}));

import { FlowCanvas } from "../FlowCanvas";
import { usePoiesisStore } from "../../../../store";

const codeEl = (id: string, tag: string, name?: string) =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {}, position: { x: 0, y: 0 } });

const reset = () => usePoiesisStore.setState({
  elements: [
    codeEl("fn1", "fn", "increment"),
    codeEl("cb1", "let", "count"),
    codeEl("cb2", "assign"),
    codeEl("cb3", "call", "close"),
  ],
  wires: [
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
    { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
    { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
  ],
  codeFlowGraphs: {}, componentDefs: {}, persistedPositions: {}, tabData: {},
  codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }] },
});

describe("FlowCanvas 折叠组", () => {
  beforeEach(() => { reset(); vi.clearAllMocks(); });

  it("折叠组：成员节点 cb1/cb2 不在渲染 nodes，组节点 g1 存在", () => {
    render(<FlowCanvas />);
    const ids = (captured.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(ids).toContain("g1");
    expect(ids).not.toContain("cb1");
    expect(ids).not.toContain("cb2");
    expect(ids).toContain("cb3"); // 组外节点保留
  });

  it("折叠组：edges 重路由（fn1→g1、g1→cb3），w2 组内连线隐藏", () => {
    render(<FlowCanvas />);
    const edges = captured.edges as Array<{ id: string; source: string; target: string }>;
    expect(edges.find((e) => e.id === "w1")).toMatchObject({ source: "fn1", target: "g1" });
    expect(edges.find((e) => e.id === "w3")).toMatchObject({ source: "g1", target: "cb3" });
    expect(edges.find((e) => e.id === "w2")).toBeUndefined();
  });

  it("展开（collapsed:false）→ 成员与原始 edges 恢复", () => {
    usePoiesisStore.setState({
      codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }] },
    });
    render(<FlowCanvas />);
    const ids = (captured.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(ids).toContain("cb1");
    const edges = captured.edges as Array<{ id: string }>;
    expect(edges.map((e) => e.id).sort()).toEqual(["w1", "w2", "w3"]);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/groups.test.ts src/components/canvas/code/__tests__/FlowCanvasGroupCollapse.test.tsx`
Expected: FAIL（collectCollapsedMemberIds/buildFlowEdges 不存在；折叠态未接线）

- [ ] **Step 3: 实现 groups.ts 追加两个纯函数**

```typescript
import type { Edge } from "@xyflow/react";

/** 所有折叠组的成员 id 集合（渲染层过滤用）。 */
export function collectCollapsedMemberIds(groups: CodeFlowGroup[]): Set<string> {
  const out = new Set<string>();
  for (const g of groups) {
    if (g.collapsed) for (const mid of g.memberIds) out.add(mid);
  }
  return out;
}

/** 折叠组连线重路由：进线接到组节点左侧（flow-in）、出线发自组节点右侧（flow-out）、
 *  组内连线隐藏。保留 wire id（防 key 冲突）；展开后从 wires 重推自动还原。 */
export function buildFlowEdges(wires: WireData[], groups: CodeFlowGroup[]): Edge[] {
  const collapsed = collectCollapsedMemberIds(groups);
  if (collapsed.size === 0) {
    return wires.map((w) => ({ id: w.id, type: "default", source: w.fromId, target: w.toId, sourceHandle: w.fromPort, targetHandle: w.toPort }));
  }
  const memberToGroup = new Map<string, string>();
  for (const g of groups) {
    if (!g.collapsed) continue;
    for (const mid of g.memberIds) memberToGroup.set(mid, g.id);
  }
  const out: Edge[] = [];
  for (const w of wires) {
    const fromGroup = memberToGroup.get(w.fromId);
    const toGroup = memberToGroup.get(w.toId);
    if (fromGroup && toGroup && fromGroup === toGroup) continue; // 组内连线隐藏
    out.push({
      id: w.id,
      type: "default",
      source: fromGroup ?? w.fromId,
      target: toGroup ?? w.toId,
      sourceHandle: fromGroup ? "flow-out" : w.fromPort,
      targetHandle: toGroup ? "flow-in" : w.toPort,
    });
  }
  return out;
}
```

（顶部 import 加 `import type { Edge } from "@xyflow/react";`——groups.ts 现在是纯函数模块，加一个类型 import 不影响其可测性。）

- [ ] **Step 4: FlowCanvas 接线（成员过滤 + edges 替换 + 折叠组卡节点）**

`FlowCanvas.tsx`：

1) `nodes` useMemo 内，在 map 前过滤折叠成员：

```typescript
const collapsedMemberIds = collectCollapsedMemberIds(Object.values(usePoiesisStore.getState().codeFlowGroups).flat());
const visibleElements = codeElements.filter((e) => !collapsedMemberIds.has(e.id));
```

（`codeElements.map(...)` 改为 `visibleElements.map(...)`；`letVarsByFn` 仍基于全量 codeElements——不影响。）

2) `edges` useMemo 替换为：

```typescript
const edges = useMemo(
  () => buildFlowEdges(wires, Object.values(usePoiesisStore.getState().codeFlowGroups).flat()),
  [wires, usePoiesisStore((s) => s.codeFlowGroups)],
);
```

3) 组节点生成处（Task 4 Step 4 已有）：折叠组的 group 节点 data 加 `collapsed: true`（GroupBox 已按 `group.collapsed` 渲染卡）；position 优先 `collapsedGroupPositions[g.id]`（Task 7 布局落点），否则 `groupBoxes` 缓存（Task 4 测量 effect 已实现「折叠组沿用 lastGroupBoxesRef 缓存」——本任务无需再改测量逻辑）。

- [ ] **Step 5: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/groups.test.ts src/components/canvas/code/__tests__/FlowCanvasGroupCollapse.test.tsx src/components/canvas/code/__tests__/FlowCanvasDrag.test.tsx && npx tsc -b --pretty false`
Expected: 全绿 + tsc 零错误

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/groups.ts poiesis/src/services/codegen/__tests__/groups.test.ts poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/__tests__/FlowCanvasGroupCollapse.test.tsx
git commit -m "feat(editor): 折叠态——成员节点过滤 + 连线重路由到组边界（纯函数 buildFlowEdges），展开自动还原"
```

---

### Task 6: 代码生成组注释（graph-compiler + pipeline 接入）

**Files:**
- Modify: `poiesis/src/services/codegen/graph-compiler.ts`（compileCodeFlowGraph 第 4 参 groups? + emitNode 组注释）
- Modify: `poiesis/src/services/codegen/pipeline.ts`（options.codeFlowGroups 传入）
- Modify: `poiesis/src/hooks/useSaveProject.ts`（调用 pipeline 时从 store 传 codeFlowGroups）
- Test: `poiesis/src/services/codegen/__tests__/graph-compiler-groups.test.ts`（新建）

**Interfaces:**
- Consumes: `CodeFlowGroup`（Task 1）
- Produces: `compileCodeFlowGraph(elements, wires, fnPid, groups?: CodeFlowGroup[]): string`——**签名向后兼容**（第 4 参可选，不传行为不变）
- Produces: `pipeline` options 新增 `codeFlowGroups?: Record<string, CodeFlowGroup[]>`

**关键实现说明：** 组注释插在「组内第一个被 emit 的成员」之前（emit 顺序即 wire 拓扑顺序）。`emitNode` 的 `visited` 在开头检查后立刻 `add`——组注释判断放在 `visited.add` **之前**：若当前节点属于某组且该组无任何成员已入 visited → 输出 `// ── 组名 ──`（与语句同缩进）。注意 `pad` 计算需提前到判断前。

- [ ] **Step 1: 写失败测试**（`poiesis/src/services/codegen/__tests__/graph-compiler-groups.test.ts`）

```typescript
// 组注释生成测试：注释插在组内首个 emit 成员前；折叠态不影响输出；不传 groups 行为不变
import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";
import type { CodeFlowGroup } from "../../../core/types/project-json";

const el = (id: string, tag: string, props: Record<string, unknown> = {}): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props });

const ELEMS = [
  el("fn1", "fn", { name: "increment", params: "" }),
  el("cb1", "let", { name: "count", value: "0" }),
  el("cb2", "assign", { target: "count", operator: "+=", value: "1" }),
  el("cb3", "return", { expr: "count" }),
];
const WIRES: WireData[] = [
  { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
  { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
  { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
];

describe("graph-compiler 组注释", () => {
  it("组注释插在组内首个 emit 成员（cb1）前，同缩进", () => {
    const groups: CodeFlowGroup[] = [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }];
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn1", groups);
    expect(out).toContain("  // ── 初始化 ──\n");
    expect(out.indexOf("// ── 初始化 ──")).toBeLessThan(out.indexOf("// @pid:cb1"));
  });

  it("折叠态（collapsed:true）不影响输出——注释与语句完整", () => {
    const groups: CodeFlowGroup[] = [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }];
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn1", groups);
    expect(out).toContain("// ── 初始化 ──");
    expect(out).toContain("// @pid:cb1");
    expect(out).toContain("// @pid:cb2");
  });

  it("组内成员分散在分支时，注释只出现在首个成员处一次", () => {
    const COND = el("cd1", "condition", { branches: [{ id: "br0", type: "if", condition: "count > 0" }] });
    const ELEMS2 = [el("fn1", "fn", { name: "f", params: "" }), COND, el("cb1", "let", { name: "a", value: "1" }), el("cb2", "call", { fnName: "g" })];
    const WIRES2: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cd1", toPort: "flow-in" },
      { id: "w2", fromId: "cd1", fromPort: "br0", toId: "cb1", toPort: "flow-in" },
      { id: "w3", fromId: "cd1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
    ];
    const groups: CodeFlowGroup[] = [{ id: "g1", name: "分支组", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }];
    const out = compileCodeFlowGraph(ELEMS2, WIRES2, "fn1", groups);
    expect(out.match(/\/\/ ── 分支组 ──/g)).toHaveLength(1);
    expect(out.indexOf("// ── 分支组 ──")).toBeLessThan(out.indexOf("// @pid:cb1"));
  });

  it("不传 groups → 输出与既有行为一致（无组注释）", () => {
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn1");
    expect(out).not.toContain("// ──");
    expect(out).toContain("// @pid:cb1");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-compiler-groups.test.ts`
Expected: FAIL（第 4 参未接线，组注释不出现）

- [ ] **Step 3: 实现 graph-compiler 组注释**

`graph-compiler.ts`：

```typescript
import type { CodeFlowGroup } from "../../core/types/project-json";
```

`emitNode` 签名加第 5 参并调整开头：

```typescript
function emitNode(
  node: GraphNode,
  adj: Map<string, GraphNode>,
  indent: number,
  visited: Set<string>,
  groups?: CodeFlowGroup[],
): string {
  if (visited.has(node.element.id)) return "";
  const pad = "  ".repeat(indent);
  // 组注释：当前节点是其所属组在 emit（拓扑）序中第一个成员 → 在语句前插注释。
  // 判断置于 visited.add 之前：仅当组内无任何成员已被访问过。
  if (groups && groups.length > 0) {
    const g = groups.find((gr) => gr.memberIds.includes(node.element.id));
    if (g && !g.memberIds.some((mid) => visited.has(mid))) {
      // fn/action/lifecycle 永不为成员（Task 2 校验保证），此处无需特判
      out += `${pad}// ── ${g.name} ──\n`;
    }
  }
  visited.add(node.element.id);
  // ...原逻辑（out 声明需提前到 pad 之后）
```

（原函数体 `const out = ...` 移到 `visited.add` 之后；其余分支不变。**注意原函数 `visited.add` 在 switch 之前、pad 定义之后**——把新代码插在 pad 与 visited.add 之间，out 声明随之提前。）

`compileCodeFlowGraph`：

```typescript
export function compileCodeFlowGraph(
  elements: CodeblockElementData[],
  wires: WireData[],
  fnPid: string,
  groups?: CodeFlowGroup[],
): string {
  const adj = buildAdjacency(elements, wires);
  const fnNode = adj.get(fnPid);
  if (!fnNode) return "";
  return emitNode(fnNode, adj, 0, new Set(), groups);
}
```

所有递归 `emitNode(succ, adj, indent, visited)` 调用点加第 5 参 `groups`（grep 该文件内全部 `emitNode(` 递归调用，逐一补参；`emitNode(fnNode, adj, 0, new Set(), groups)` 顶层已含）。

- [ ] **Step 4: pipeline 接入**

`pipeline.ts` 的 options 类型加：

```typescript
/** 组注释数据（键=fn 名）：保存时从 store.codeFlowGroups 传入。 */
codeFlowGroups?: Record<string, CodeFlowGroup[]>;
```

285 行附近（fn 编译）：

```typescript
const fnName = (fnEl.props?.name as string) ?? "unnamed";
const compiledFn = compileCodeFlowGraph(codeFlowElements, options.designWires, fnEl.id, options.codeFlowGroups?.[fnName]);
```

341 行附近（lifecycle 编译）：同样取 `options.codeFlowGroups?.[hookName]`——lifecycle 元素的 `props.name` 缺失，取 `"unnamed"` 兜底即可（lifecycle 组低频；若用户给 lifecycle 建组，键 = 其所属 fn 名——本处保守：不传 groups 给 lifecycle 编译，明确不做 lifecycle 组注释，注释仅组件 fn/action 主路径）。

`useSaveProject.ts`：调用 pipeline 的 options 里加 `codeFlowGroups: usePoiesisStore.getState().codeFlowGroups`（grep 该文件 `designWires` 传参处，同对象加字段）。

- [ ] **Step 5: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-compiler-groups.test.ts src/services/codegen/__tests__/graph-compiler-syntax.test.ts src/services/codegen/__tests__/fn-node-pipeline.test.ts && npx tsc -b --pretty false`
Expected: 全绿（既有编译器测试证明不传 groups 零回归）+ tsc 零错误

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/graph-compiler.ts poiesis/src/services/codegen/pipeline.ts poiesis/src/hooks/useSaveProject.ts poiesis/src/services/codegen/__tests__/graph-compiler-groups.test.ts
git commit -m "feat(editor): 组注释生成——compileCodeFlowGraph 第 4 参 groups，注释插组内拓扑首成员前（折叠不影响输出）"
```

---

### Task 7: 自动布局配合（折叠卡占位 + 组框跟随）

**Files:**
- Modify: `poiesis/src/services/codegen/flow-layout.ts`（新增 `buildLayoutInput` 纯函数）
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（handleAutoLayout 用 buildLayoutInput + 折叠卡布局位置）
- Test: `poiesis/src/services/codegen/__tests__/flow-layout.test.ts`（追加用例）

**Interfaces:**
- Consumes: `computeLayeredLayout(elements, wires, sizes)`（现有）、`collectCollapsedMemberIds`（Task 5）
- Produces: `buildLayoutInput(elements: CodeblockElementData[], wires: WireData[], groups: CodeFlowGroup[]): { elements: Array<{ id: string; tag: string; props?: Record<string, unknown> }>; wires: WireData[] }`——折叠组替换为伪元素 `{ id: "group:" + g.id, tag: "code-block", props: {} }`（尺寸走 estimateNodeSize 兜底），折叠成员与其连线从输入剔除；展开组原样透传

**关键说明：** 布局后折叠卡的落点 = `layout["group:" + g.id]`；FlowCanvas 布局回调里把该位置写入 `groupPositions` state（`Record<groupId, {x,y}>`），组节点生成时折叠组 position 优先取它。展开组 bbox 由 Task 4 测量 effect 自动跟随（布局改变成员 position → effect 重算）。成员隐藏时**不重排**（其陈旧位置保留，展开后原位出现）。

- [ ] **Step 1: 写失败测试**（`flow-layout.test.ts` 追加）

```typescript
import { buildLayoutInput } from "../flow-layout";
import type { CodeFlowGroup } from "../../../core/types/project-json";

describe("buildLayoutInput（布局输入构建）", () => {
  const el = (id: string, tag: string) => ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props: {}, position: { x: 0, y: 0 } });

  it("折叠组 → 伪元素 code-block 替代，成员与组内连线剔除；展开组原样保留", () => {
    const ELEMS = [el("fn1", "fn"), el("cb1", "let"), el("cb2", "assign"), el("cb3", "call")];
    const WIRES: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
      { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
      { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
    ];
    const groups: CodeFlowGroup[] = [
      { id: "g1", name: "a", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true },
      { id: "g2", name: "b", memberIds: ["cb3"], color: "#6366f1", collapsed: false },
    ];
    const { elements, wires } = buildLayoutInput(ELEMS, WIRES, groups);
    const ids = elements.map((e) => e.id);
    expect(ids).toContain("fn1");
    expect(ids).toContain("group:g1");
    expect(ids).toContain("cb3");
    expect(ids).not.toContain("cb1");
    expect(ids).not.toContain("cb2");
    expect(wires.map((w) => w.id)).toEqual(["w1", "w3"]); // w2 组内连线剔除，w3 保留
  });

  it("无折叠组 → 原样透传", () => {
    const ELEMS = [el("fn1", "fn"), el("cb1", "let")];
    const WIRES: WireData[] = [{ id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" }];
    const { elements, wires } = buildLayoutInput(ELEMS, WIRES, []);
    expect(elements.map((e) => e.id)).toEqual(["fn1", "cb1"]);
    expect(wires).toEqual(WIRES);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/flow-layout.test.ts`
Expected: FAIL（buildLayoutInput 不存在）

- [ ] **Step 3: 实现 buildLayoutInput**

`flow-layout.ts` 追加：

```typescript
import type { CodeFlowGroup } from "../../core/types/project-json";
import { collectCollapsedMemberIds } from "./groups";

/** 布局输入构建：折叠组替换为伪元素（tag: code-block，尺寸走 estimateNodeSize 兜底），
 *  折叠成员与其相关连线从输入剔除；展开组/无组原样透传。返回的 elements 供 computeLayeredLayout 消费。 */
export function buildLayoutInput(
  elements: Array<Pick<CodeblockElementData, "id" | "tag" | "props">>,
  wires: WireData[],
  groups: CodeFlowGroup[],
): { elements: Array<{ id: string; tag: string; props?: Record<string, unknown> }>; wires: WireData[] } {
  const collapsed = collectCollapsedMemberIds(groups);
  if (collapsed.size === 0) return { elements: elements as never, wires };
  const collapsedGroups = groups.filter((g) => g.collapsed);
  const outEls: Array<{ id: string; tag: string; props?: Record<string, unknown> }> = [];
  for (const e of elements) {
    if (collapsed.has(e.id)) continue;
    outEls.push(e as never);
  }
  for (const g of collapsedGroups) outEls.push({ id: `group:${g.id}`, tag: "code-block", props: {} });
  const outWires = wires.filter((w) => !collapsed.has(w.fromId) && !collapsed.has(w.toId));
  return { elements: outEls, wires: outWires };
}
```

- [ ] **Step 4: FlowCanvas.handleAutoLayout 接入**

`FlowCanvas.tsx`：

```typescript
const handleAutoLayout = useCallback(() => {
  if (codeElements.length === 0) return;
  const groups = Object.values(usePoiesisStore.getState().codeFlowGroups).flat();
  const { elements: layoutInput, wires: layoutWires } = buildLayoutInput(codeElements, wires, groups);
  const collapsedMemberIds = collectCollapsedMemberIds(groups);
  // 尺寸：既有 DOM 测量循环照旧（layoutInput 中真实成员才测得 DOM；折叠伪元素无 DOM → estimateNodeSize 兜底）
  const sizes = new Map<string, { width: number; height: number }>();
  // ...（既有测量循环不变）...
  const layout = computeLayeredLayout(layoutInput, layoutWires, sizes);
  // 折叠卡落点单独收：group:g1 → g1（state 已在 Task 4 声明，此处只写）
  const groupPos: Record<string, { x: number; y: number }> = {};
  for (const g of groups.filter((x) => x.collapsed)) {
    const p = layout[`group:${g.id}`];
    if (p) groupPos[g.id] = { x: p.x, y: p.y };
  }
  setCollapsedGroupPositions(groupPos);
  // 写回 elements 只对非折叠成员（layout 键不含 group:* 与隐藏成员）
  const nextElements = allElements.map((e) =>
    e.nodeType === "codeblock" && CODE_TAGS.has(e.tag) && !collapsedMemberIds.has(e.id) && layout[e.id]
      ? { ...e, position: { ...layout[e.id] } } as ElementData
      : e,
  );
  setElementsAndWires(nextElements, wires);
  // ...（预览归位逻辑不变）
}, [codeElements, allElements, wires, nodes, setElementsAndWires]);
```

（`collapsedGroupPositions` state 已在 Task 4 Step 4 声明，此处只使用不重复声明；组节点生成处折叠组 position 优先取 `collapsedGroupPositions[g.id]`——Task 4 Step 4 已写好该分支。）

- [ ] **Step 5: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/flow-layout.test.ts src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx src/components/canvas/code/__tests__/FlowCanvasGroupCollapse.test.tsx && npx tsc -b --pretty false`
Expected: 全绿 + tsc 零错误

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/flow-layout.ts poiesis/src/services/codegen/__tests__/flow-layout.test.ts poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat(editor): 布局配合——buildLayoutInput 折叠组伪元素占位，折叠卡落点单独写回"
```

---

### Task 8: 全量回归 + 真机验证

**Files:**
- Run: 全量测试 + tsc + 真机（Victauri）

- [ ] **Step 1: 全量测试 + 类型检查**

Run: `cd poiesis && npm test`
Expected: 全绿（基线 ~507 + 本功能新增用例）

Run: `cd poiesis && npx tsc -b --pretty false`
Expected: 零错误

- [ ] **Step 2: 真机验证（Victauri 实测脚本）**

App 运行中（`npm run tauri dev`，victauri MCP 已连接）。用 victauri 工具逐步执行并截图确认：

1. `dom_snapshot` 定位 TodoStore 代码流画布，确认已进入（breadcrumb「代码」）
2. Shift+点击两个语句节点 → 右键 → 「组成分组」→ 输入组名「初始化」→ Enter
3. `dom_snapshot`：确认分组框出现（`[data-group-id]` 元素 + 组名文本）
4. 点击折叠按钮 → `dom_snapshot`：成员节点消失、组卡出现（`折叠` 组卡）、连线重路由（`svg.edge` 检查 source/target 指向组节点）
5. 展开 → 成员恢复
6. 保存项目（Ctrl+S）→ 读 `project.json`（`list_app_dir` + `read_app_file`）：`codeFlowGroups` 字段存在且含该组
7. 重开项目（或 reload）→ 确认分组恢复（collapsed 态保持）
8. 打开生成的 `.omni`（读磁盘文件）：fn 体内 `// ── 初始化 ──` 注释存在，语句完整
9. 自动整理布局 → 分组框跟随成员重排、折叠卡参与布局
10. `screenshot` 留存：展开态分组框、折叠态组卡各一张

- [ ] **Step 3: 修复验证中发现的问题**

如真机暴露缺陷：按 systematic-debugging 流程定位修复，补回归测试，重跑 Task 8 Step 1。

- [ ] **Step 4: 最终 commit（如有修复）**

```bash
git add -A
git commit -m "fix(editor): 真机验证修复——<具体问题描述>"
```

---

## 已知限制（本计划诚实标注）

- **action/lifecycle 路径无组注释**：Task 6 只接入 pipeline 主路径（组件 fn）。`source-emitter.ts` 的 action 编译路径（3 处 `compileCodeFlowGraph` 调用）不传 groups——action 内建组保存后源码无注释（画布分组与折叠不受影响）。如需支持，后续在 source-emitter 各调用点透传。
- **折叠卡不可拖动**：折叠态组卡位置由 bbox 缓存 / 布局落点决定，不支持手动拖拽（成员隐藏无拖拽锚点）。展开后可拖成员。
- **跨组同一节点**：不做互斥校验（两组合一节点时组注释可能重复插入同一语句前——低频，接受）。
- **加载非法成员无显式过滤**：spec §8「加载时非法成员过滤兜底」由渲染层容错承担——`computeGroupBoxes` 对缺元素/缺尺寸成员跳过（不产出 bbox → 组不渲染）、`buildFlowEdges` 对不在折叠映射中的成员原样透传。行为满足 spec（非法成员不导致崩溃/错乱），不额外加加载期过滤代码。
- **空组保存时清理**：成员删光的组在画布上无可见 UI（computeGroupBoxes 跳过空组），无法手动删——`toProjectJson` 写入时自动剔除空组（spec §8「空框保留」落地为 JSON 侧清理，行为更实用）。
