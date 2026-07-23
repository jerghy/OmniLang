# DesignCanvas 组件定义面板 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 DesignCanvas 无限画布内添加可拖动的浮动面板，支持查看/创建/编辑/删除当前组件的 Props 和 Signals

**Architecture:** 面板渲染在 InfiniteCanvas 的 transform 层内（与 Artboard 同级），天然跟随 zoom/pan。面板位置由 canvas-store 管理（画布坐标系），拖拽通过 Header 区域 pointer events 实现。数据操作复用 `componentDefs[compName]` + 新增粒度化 CRUD action。

**Tech Stack:** React 19 + TypeScript + Zustand 5 + CSS Modules

## Global Constraints

- 不可变更新：所有 store 操作使用展开运算符创建新对象
- 类型安全：禁止 `any`，使用 `unknown` + 窄化
- 文件行数：新建文件 <400 行，修改文件 <800 行
- 组件 Props 使用命名 `interface`，不使用 `React.FC`
- 面板文字使用固定 `px` 字号，不随画布缩放而模糊

---

## File Structure

| 文件 | 职责 | 操作 |
|------|------|------|
| `poiesis/src/components/design/ComponentDefPanel.tsx` | 面板主组件 + 子组件 | **新建** |
| `poiesis/src/components/design/ComponentDefPanel.css` | 面板样式 | **新建** |
| `poiesis/src/components/canvas/InfiniteCanvas.tsx` | 集成面板渲染 | **修改** |
| `poiesis/src/components/canvas/canvas-store.ts` | 面板位置/折叠状态 | **修改** |
| `poiesis/src/store/slices/component-lifecycle.ts` | 组件 signal/prop/computed CRUD | **修改** |
| `poiesis/src/store/types.ts` | Action 类型声明 | **修改** |

---

### Task 1: 扩展 Store Types — 声明新 Action 类型

**Files:**
- Modify: `poiesis/src/store/types.ts` (在 ComponentDef 区域后追加)

**Interfaces:**
- Produces: 9 个新 action 类型签名，供 Task 2 实现和 Task 3 消费

在 line 162 (`getComponentDef`) 之后，添加以下 action 类型声明：

- [ ] **Step 1: 添加类型声明**

```typescript
// ── Component signal CRUD ──
addComponentSignal: (compName: string, signal: { pid: string; name: string; type: string; defaultValue?: unknown }) => void;
updateComponentSignal: (compName: string, pid: string, updates: Partial<{ name: string; type: string; defaultValue: unknown }>) => void;
removeComponentSignal: (compName: string, pid: string) => void;

// ── Component prop CRUD ──
addComponentProp: (compName: string, prop: { pid: string; name: string; type: string; defaultValue?: unknown }) => void;
updateComponentProp: (compName: string, pid: string, updates: Partial<{ name: string; type: string; defaultValue: unknown }>) => void;
removeComponentProp: (compName: string, pid: string) => void;

// ── Component computed CRUD ──
addComponentComputed: (compName: string, computed: { pid: string; name: string; returnType: string; expression: string }) => void;
updateComponentComputed: (compName: string, pid: string, updates: Partial<{ name: string; returnType: string; expression: string }>) => void;
removeComponentComputed: (compName: string, pid: string) => void;
```

- [ ] **Step 2: 验证类型**

Run: `cd poiesis && npx tsc --noEmit 2>&1 | head -20`
Expected: No new type errors from the types file (Task 2 will add implementations, so 9 errors about unimplemented members are expected).

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/store/types.ts
git commit -m "feat: add component signal/prop/computed CRUD type declarations"
```

---

### Task 2: 实现 Component Signal/Prop/Computed CRUD Actions

**Files:**
- Modify: `poiesis/src/store/slices/component-lifecycle.ts`

**Interfaces:**
- Consumes: `PoiesisState` 接口中 Task 1 新增的 action 类型
- Produces: 9 个 CRUD action 实现，供 Task 3 (ComponentDefPanel) 调用

- [ ] **Step 1: 实现 component signal CRUD**

在 `createComponentLifecycleSlice` 函数返回对象的末尾（`enterComponentCanvas` 之后），添加：

```typescript
// ── Component signal CRUD ──
addComponentSignal: (compName: string, signal: DefEntry) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const signals = [...(def.signals || []), signal];
  s.setComponentDef(compName, { ...def, signals });
  return {};
}),

updateComponentSignal: (compName: string, pid: string, updates: Partial<DefEntry>) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const signals = (def.signals || []).map((sig) =>
    sig.pid === pid ? { ...sig, ...updates } : sig
  );
  s.setComponentDef(compName, { ...def, signals });
  return {};
}),

removeComponentSignal: (compName: string, pid: string) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const signals = (def.signals || []).filter((sig) => sig.pid !== pid);
  s.setComponentDef(compName, { ...def, signals });
  return {};
}),
```

- [ ] **Step 2: 实现 component prop CRUD**

```typescript
// ── Component prop CRUD ──
addComponentProp: (compName: string, prop: DefEntry) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const props = [...(def.props || []), prop];
  s.setComponentDef(compName, { ...def, props });
  return {};
}),

updateComponentProp: (compName: string, pid: string, updates: Partial<DefEntry>) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const props = (def.props || []).map((p) =>
    p.pid === pid ? { ...p, ...updates } : p
  );
  s.setComponentDef(compName, { ...def, props });
  return {};
}),

removeComponentProp: (compName: string, pid: string) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const props = (def.props || []).filter((p) => p.pid !== pid);
  s.setComponentDef(compName, { ...def, props });
  return {};
}),
```

- [ ] **Step 3: 实现 component computed CRUD**

```typescript
// ── Component computed CRUD ──
addComponentComputed: (compName: string, computed: ComputedEntry) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const computeds = [...(def.computeds || []), computed];
  s.setComponentDef(compName, { ...def, computeds });
  return {};
}),

updateComponentComputed: (compName: string, pid: string, updates: Partial<ComputedEntry>) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const computeds = (def.computeds || []).map((c) =>
    c.pid === pid ? { ...c, ...updates } : c
  );
  s.setComponentDef(compName, { ...def, computeds });
  return {};
}),

removeComponentComputed: (compName: string, pid: string) => set((s) => {
  const def = s.componentDefs[compName];
  if (!def) return {};
  const computeds = (def.computeds || []).filter((c) => c.pid !== pid);
  s.setComponentDef(compName, { ...def, computeds });
  return {};
}),
```

- [ ] **Step 4: 添加 import**

在文件顶部添加 `DefEntry` 和 `ComputedEntry` 的导入：

```typescript
import type { ComponentDef, DefEntry, ComputedEntry } from "../../core/types/project-json";
```

替换现有的 `ComponentDef` 导入行。

- [ ] **Step 5: 验证编译**

Run: `cd poiesis && npx tsc --noEmit 2>&1 | grep -c "error"`
Expected: 不增加新的 TS 错误（与 Task 1 之前对比）。

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/store/slices/component-lifecycle.ts
git commit -m "feat: implement component signal/prop/computed CRUD actions"
```

---

### Task 3: 扩展 Canvas Store — 面板位置与折叠状态

**Files:**
- Modify: `poiesis/src/components/canvas/canvas-store.ts`

**Interfaces:**
- Produces: `panelPosition`, `isPanelCollapsed`, `setPanelPosition`, `togglePanelCollapsed` 状态和 action

- [ ] **Step 1: 添加状态字段和 action 到接口**

```typescript
export interface CanvasState {
  // ... existing fields ...
  panelPosition: { x: number; y: number };
  isPanelCollapsed: boolean;
  // ... existing actions ...
  setPanelPosition: (x: number, y: number) => void;
  togglePanelCollapsed: () => void;
}
```

- [ ] **Step 2: 添加初始值和 action 实现**

在 `create` 调用中添加：

```typescript
// 初始值（在 zoom: 1.0 等后面）
panelPosition: { x: 0, y: 0 },
isPanelCollapsed: false,

// Actions（在 resetView 后面）
setPanelPosition: (x, y) => set({ panelPosition: { x, y } }),
togglePanelCollapsed: () => set((s) => ({ isPanelCollapsed: !s.isPanelCollapsed })),
```

- [ ] **Step 3: 验证编译**

Run: `cd poiesis && npx tsc --noEmit 2>&1 | head -5`
Expected: No new type errors.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/canvas-store.ts
git commit -m "feat: add panel position and collapse state to canvas store"
```

---

### Task 4: 创建 ComponentDefPanel 组件

**Files:**
- Create: `poiesis/src/components/design/ComponentDefPanel.tsx`
- Create: `poiesis/src/components/design/ComponentDefPanel.css`

**Interfaces:**
- Consumes: Task 2 的 CRUD actions (`addComponentSignal`, `removeComponentSignal`, etc.), Task 3 的 canvas state (`panelPosition`, `isPanelCollapsed`)
- Produces: 可拖动的浮动面板组件

- [ ] **Step 1: 创建 CSS 样式文件**

```css
/* poiesis/src/components/design/ComponentDefPanel.css */

.component-def-panel {
  position: absolute;
  width: 260px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
  font-size: 12px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  z-index: 10;
  user-select: none;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  background: #f1f5f9;
  border-bottom: 1px solid #e2e8f0;
  cursor: grab;
  font-weight: 600;
  font-size: 12px;
  color: #334155;
}

.panel-header:active {
  cursor: grabbing;
}

.panel-header-title {
  display: flex;
  align-items: center;
  gap: 6px;
}

.panel-collapse-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: #64748b;
  font-size: 14px;
  padding: 0 2px;
  line-height: 1;
}

.panel-collapse-btn:hover {
  color: #334155;
}

.panel-body {
  max-height: 400px;
  overflow-y: auto;
  padding: 0;
}

.panel-section {
  border-bottom: 1px solid #f1f5f9;
}

.panel-section:last-child {
  border-bottom: none;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  font-weight: 600;
  font-size: 11px;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.section-add-btn {
  background: none;
  border: none;
  color: #7C3AED;
  cursor: pointer;
  font-size: 16px;
  padding: 0 2px;
  line-height: 1;
  font-weight: 700;
}

.section-add-btn:hover {
  color: #6D28D9;
}

.entry-row {
  display: flex;
  align-items: center;
  padding: 4px 10px;
  gap: 6px;
  font-size: 12px;
  color: #1e293b;
  border-top: 1px solid #f8fafc;
}

.entry-row:hover {
  background: #f8fafc;
}

.entry-name {
  font-weight: 600;
  color: #1e293b;
  min-width: 0;
  flex-shrink: 0;
}

.entry-type {
  color: #64748b;
  font-size: 11px;
  flex-shrink: 0;
}

.entry-default {
  color: #94a3b8;
  font-size: 11px;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.entry-delete-btn {
  background: none;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  font-size: 12px;
  padding: 0 2px;
  line-height: 1;
  flex-shrink: 0;
  visibility: hidden;
}

.entry-row:hover .entry-delete-btn {
  visibility: visible;
}

.entry-delete-btn:hover {
  color: #ef4444;
}

.empty-state {
  padding: 10px;
  text-align: center;
  color: #94a3b8;
  font-size: 11px;
  font-style: italic;
}

/* Inline editing */
.entry-row input {
  font-size: 12px;
  border: 1px solid #cbd5e1;
  border-radius: 3px;
  padding: 1px 4px;
  outline: none;
  min-width: 0;
}

.entry-row input:focus {
  border-color: #7C3AED;
}

.entry-name-input {
  width: 60px;
  font-weight: 600;
}

.entry-type-input {
  width: 50px;
  color: #64748b;
}

.entry-default-input {
  width: 60px;
}

/* Add form */
.add-form {
  display: flex;
  gap: 4px;
  padding: 4px 10px;
  align-items: center;
}

.add-form input {
  font-size: 12px;
  border: 1px solid #cbd5e1;
  border-radius: 3px;
  padding: 2px 4px;
  outline: none;
  min-width: 0;
}

.add-form input:focus {
  border-color: #7C3AED;
}

.add-form-confirm {
  background: #7C3AED;
  color: #fff;
  border: none;
  border-radius: 3px;
  padding: 2px 6px;
  font-size: 11px;
  cursor: pointer;
}

.add-form-cancel {
  background: none;
  border: none;
  color: #94a3b8;
  font-size: 11px;
  cursor: pointer;
}
```

- [ ] **Step 2: 创建 ComponentDefPanel.tsx**

```tsx
import React, { useState, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { useCanvasStore } from "../canvas/canvas-store";
import type { DefEntry, ComputedEntry } from "../../core/types/project-json";
import "./ComponentDefPanel.css";

/** 生成简单 PID（组件内唯一） */
function generatePid(): string {
  return `pid-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function ComponentDefPanel() {
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const addComponentSignal = usePoiesisStore((s) => s.addComponentSignal);
  const removeComponentSignal = usePoiesisStore((s) => s.removeComponentSignal);
  const updateComponentSignal = usePoiesisStore((s) => s.updateComponentSignal);
  const addComponentProp = usePoiesisStore((s) => s.addComponentProp);
  const removeComponentProp = usePoiesisStore((s) => s.removeComponentProp);
  const updateComponentProp = usePoiesisStore((s) => s.updateComponentProp);
  const addComponentComputed = usePoiesisStore((s) => s.addComponentComputed);
  const removeComponentComputed = usePoiesisStore((s) => s.removeComponentComputed);
  const updateComponentComputed = usePoiesisStore((s) => s.updateComponentComputed);

  const panelPosition = useCanvasStore((s) => s.panelPosition);
  const isCollapsed = useCanvasStore((s) => s.isPanelCollapsed);
  const setPanelPosition = useCanvasStore((s) => s.setPanelPosition);
  const toggleCollapsed = useCanvasStore((s) => s.togglePanelCollapsed);

  // 拖拽状态
  const dragRef = useRef<{ startX: number; startY: number; posX: number; posY: number } | null>(null);
  const [isAdding, setIsAdding] = useState<"prop" | "signal" | "computed" | null>(null);
  const [editingPid, setEditingPid] = useState<string | null>(null);

  // 获取当前组件名
  const activeTab = openTabs.find((t) => t.id === activeTabId);
  const componentName = activeTab?.componentName || "main";
  const def = componentDefs[componentName];
  const signals = def?.signals || [];
  const props = def?.props || [];
  const computeds = def?.computeds || [];

  // 拖拽处理
  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        posX: panelPosition.x,
        posY: panelPosition.y,
      };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      e.stopPropagation();
    },
    [panelPosition],
  );

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragRef.current) return;
    const dx = e.clientX - dragRef.current.startX;
    const dy = e.clientY - dragRef.current.startY;
    setPanelPosition(dragRef.current.posX + dx, dragRef.current.posY + dy);
  }, [setPanelPosition]);

  const handlePointerUp = useCallback(() => {
    dragRef.current = null;
  }, []);

  // 添加项
  const handleAdd = (kind: "prop" | "signal" | "computed", name: string, type: string, extra?: string) => {
    if (!name.trim()) return;
    const pid = generatePid();
    if (kind === "prop") {
      addComponentProp(componentName, { pid, name, type });
    } else if (kind === "signal") {
      addComponentSignal(componentName, { pid, name, type });
    } else {
      addComponentComputed(componentName, { pid, name, returnType: type, expression: extra || "" });
    }
    setIsAdding(null);
  };

  // 行内编辑 — 双击名称开始编辑
  const handleDoubleClick = (pid: string) => {
    setEditingPid(pid);
  };

  const handleEditSave = (kind: "prop" | "signal" | "computed", pid: string, field: string, value: string) => {
    if (kind === "computed") {
      const updates: Partial<ComputedEntry> = {};
      if (field === "name") updates.name = value;
      if (field === "returnType") updates.returnType = value;
      if (field === "expression") updates.expression = value;
      updateComponentComputed(componentName, pid, updates);
    } else {
      const updates: Partial<DefEntry> = {};
      if (field === "name") updates.name = value;
      if (field === "type") updates.type = value;
      if (field === "defaultValue") updates.defaultValue = value;
      if (kind === "prop") {
        updateComponentProp(componentName, pid, updates);
      } else {
        updateComponentSignal(componentName, pid, updates);
      }
    }
    setEditingPid(null);
  };

  // 删除项
  const handleDelete = (kind: "prop" | "signal" | "computed", pid: string) => {
    if (kind === "prop") removeComponentProp(componentName, pid);
    else if (kind === "signal") removeComponentSignal(componentName, pid);
    else removeComponentComputed(componentName, pid);
  };

  return (
    <div
      className="component-def-panel"
      style={{
        left: panelPosition.x,
        top: panelPosition.y,
      }}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {/* Header */}
      <div
        className="panel-header"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <span className="panel-header-title">📋 {componentName}</span>
        <button className="panel-collapse-btn" onClick={toggleCollapsed}>
          {isCollapsed ? "□" : "－"}
        </button>
      </div>

      {!isCollapsed && (
        <div className="panel-body">
          {/* Props 区块 */}
          <div className="panel-section">
            <div className="section-header">
              <span>📦 Props</span>
              <button className="section-add-btn" onClick={() => setIsAdding(isAdding === "prop" ? null : "prop")}>+</button>
            </div>
            {isAdding === "prop" && (
              <AddForm kind="prop" onAdd={handleAdd} onCancel={() => setIsAdding(null)} />
            )}
            {props.length === 0 && !isAdding && (
              <div className="empty-state">暂无 Props</div>
            )}
            {props.map((p) => (
              <DefEntryRow
                key={p.pid}
                entry={p}
                kind="prop"
                isEditing={editingPid === p.pid}
                onDoubleClick={() => handleDoubleClick(p.pid)}
                onEditSave={(field, value) => handleEditSave("prop", p.pid, field, value)}
                onDelete={() => handleDelete("prop", p.pid)}
              />
            ))}
          </div>

          {/* Signals 区块 */}
          <div className="panel-section">
            <div className="section-header">
              <span>⚡ Signals</span>
              <button className="section-add-btn" onClick={() => setIsAdding(isAdding === "signal" ? null : "signal")}>+</button>
            </div>
            {isAdding === "signal" && (
              <AddForm kind="signal" onAdd={handleAdd} onCancel={() => setIsAdding(null)} />
            )}
            {signals.length === 0 && !isAdding && (
              <div className="empty-state">暂无 Signals</div>
            )}
            {signals.map((s) => (
              <DefEntryRow
                key={s.pid}
                entry={s}
                kind="signal"
                isEditing={editingPid === s.pid}
                onDoubleClick={() => handleDoubleClick(s.pid)}
                onEditSave={(field, value) => handleEditSave("signal", s.pid, field, value)}
                onDelete={() => handleDelete("signal", s.pid)}
              />
            ))}
          </div>

          {/* Computed 区块 */}
          <div className="panel-section">
            <div className="section-header">
              <span>📐 Computed</span>
              <button className="section-add-btn" onClick={() => setIsAdding(isAdding === "computed" ? null : "computed")}>+</button>
            </div>
            {isAdding === "computed" && (
              <AddForm kind="computed" onAdd={handleAdd} onCancel={() => setIsAdding(null)} />
            )}
            {computeds.length === 0 && !isAdding && (
              <div className="empty-state">暂无 Computed</div>
            )}
            {computeds.map((c) => (
              <ComputedEntryRow
                key={c.pid}
                entry={c}
                isEditing={editingPid === c.pid}
                onDoubleClick={() => handleDoubleClick(c.pid)}
                onEditSave={(field, value) => handleEditSave("computed", c.pid, field, value)}
                onDelete={() => handleDelete("computed", c.pid)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ── 子组件 ──

interface DefEntryRowProps {
  entry: DefEntry;
  kind: "prop" | "signal";
  isEditing: boolean;
  onDoubleClick: () => void;
  onEditSave: (field: string, value: string) => void;
  onDelete: () => void;
}

function DefEntryRow({ entry, kind, isEditing, onDoubleClick, onEditSave, onDelete }: DefEntryRowProps) {
  if (isEditing) {
    return (
      <div className="entry-row">
        <input
          className="entry-name-input"
          defaultValue={entry.name}
          autoFocus
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span>:</span>
        <input
          className="entry-type-input"
          defaultValue={entry.type}
          onBlur={(e) => onEditSave("type", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("type", e.currentTarget.value); }}
        />
        {kind === "signal" && (
          <>
            <span>=</span>
            <input
              className="entry-default-input"
              defaultValue={String(entry.defaultValue ?? "")}
              placeholder="默认值"
              onBlur={(e) => onEditSave("defaultValue", e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") onEditSave("defaultValue", e.currentTarget.value); }}
            />
          </>
        )}
      </div>
    );
  }

  return (
    <div className="entry-row" onDoubleClick={onDoubleClick}>
      <span className="entry-name">{entry.name}</span>
      <span>:</span>
      <span className="entry-type">{entry.type}</span>
      {kind === "signal" && entry.defaultValue !== undefined && (
        <>
          <span>=</span>
          <span className="entry-default">{String(entry.defaultValue)}</span>
        </>
      )}
      <button className="entry-delete-btn" onClick={onDelete}>✕</button>
    </div>
  );
}

interface ComputedEntryRowProps {
  entry: ComputedEntry;
  isEditing: boolean;
  onDoubleClick: () => void;
  onEditSave: (field: string, value: string) => void;
  onDelete: () => void;
}

function ComputedEntryRow({ entry, isEditing, onDoubleClick, onEditSave, onDelete }: ComputedEntryRowProps) {
  if (isEditing) {
    return (
      <div className="entry-row">
        <input
          className="entry-name-input"
          defaultValue={entry.name}
          autoFocus
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span>:</span>
        <input
          className="entry-type-input"
          defaultValue={entry.returnType}
          onBlur={(e) => onEditSave("returnType", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("returnType", e.currentTarget.value); }}
        />
      </div>
    );
  }

  return (
    <div className="entry-row" onDoubleClick={onDoubleClick}>
      <span className="entry-name">{entry.name}</span>
      <span>:</span>
      <span className="entry-type">{entry.returnType}</span>
      <button className="entry-delete-btn" onClick={onDelete}>✕</button>
    </div>
  );
}

// ── 添加表单 ──

interface AddFormProps {
  kind: "prop" | "signal" | "computed";
  onAdd: (kind: "prop" | "signal" | "computed", name: string, type: string, extra?: string) => void;
  onCancel: () => void;
}

function AddForm({ kind, onAdd, onCancel }: AddFormProps) {
  const nameRef = useRef<HTMLInputElement>(null);
  const typeRef = useRef<HTMLInputElement>(null);
  const extraRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    nameRef.current?.focus();
  }, []);

  const handleConfirm = () => {
    const name = nameRef.current?.value || "";
    const type = typeRef.current?.value || "String";
    const extra = extraRef.current?.value || "";
    onAdd(kind, name, type, extra);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleConfirm();
    if (e.key === "Escape") onCancel();
  };

  return (
    <div className="add-form" onKeyDown={handleKeyDown}>
      <input ref={nameRef} placeholder="名称" className="entry-name-input" />
      <span>:</span>
      <input ref={typeRef} placeholder="类型" className="entry-type-input" defaultValue="String" />
      {kind === "signal" && (
        <>
          <span>=</span>
          <input ref={extraRef} placeholder="默认值" className="entry-default-input" />
        </>
      )}
      {kind === "computed" && (
        <>
          <span>=</span>
          <input ref={extraRef} placeholder="表达式" className="entry-default-input" />
        </>
      )}
      <button className="add-form-confirm" onClick={handleConfirm}>✓</button>
      <button className="add-form-cancel" onClick={onCancel}>✕</button>
    </div>
  );
}
```

- [ ] **Step 2: 验证编译**

Run: `cd poiesis && npx tsc --noEmit 2>&1 | grep "ComponentDefPanel" | head -10`
Expected: No TS errors.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/ComponentDefPanel.tsx poiesis/src/components/design/ComponentDefPanel.css
git commit -m "feat: create ComponentDefPanel with prop/signal/computed management"
```

---

### Task 5: 在 InfiniteCanvas 中集成面板

**Files:**
- Modify: `poiesis/src/components/canvas/InfiniteCanvas.tsx`

**Interfaces:**
- Consumes: Task 4 的 `ComponentDefPanel` 组件
- Produces: 面板渲染在 canvas transform 层内，随 zoom/pan 移动

- [ ] **Step 1: 导入 ComponentDefPanel**

在文件顶部 import 区添加：

```typescript
import { ComponentDefPanel } from "../design/ComponentDefPanel";
```

- [ ] **Step 2: 渲染面板到 transform 层内**

在 `<Artboard>{children}</Artboard>` 之后添加：

```tsx
<Artboard>{children}</Artboard>
<ComponentDefPanel />
```

两行代码，放在 `</div>` (transform 层) 关闭前。

- [ ] **Step 3: 验证编译**

Run: `cd poiesis && npx tsc --noEmit 2>&1 | grep -E "error|InfiniteCanvas" | head -10`
Expected: No new errors.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/InfiniteCanvas.tsx
git commit -m "feat: integrate ComponentDefPanel into InfiniteCanvas transform layer"
```

---

### Task 6: Victory 浏览器验证

使用 Victauri MCP 进行浏览器验证。

- [ ] **Step 1: 启动 Poiesis**

Run: `cd poiesis && pnpm tauri dev`
Expected: Poiesis 启动成功，Victauri MCP 连接就绪。

- [ ] **Step 2: 验证默认面板位置**

使用 `mcp__victauri__screenshot` 截图，确认面板出现在画布内。
使用 `mcp__victauri__dom_snapshot` 检查 `.component-def-panel` 元素存在。

- [ ] **Step 3: 验证拖拽**

```typescript
// 获取面板初始位置
const pos1 = await eval_js("document.querySelector('.component-def-panel')?.getBoundingClientRect()")
// 模拟 Header 拖拽
// 重新获取位置
const pos2 = await eval_js("document.querySelector('.component-def-panel')?.getBoundingClientRect()")
// 验证 pos1 !== pos2
```

- [ ] **Step 4: 验证折叠/展开**

```typescript
// 点击折叠按钮
await mcp__victauri__find_elements({ text: "－" }) → 点击
// 验证 .panel-body 不可见
await mcp__victauri__assert_semantic({ expression: "document.querySelector('.panel-body')", condition: "falsy" })
// 再次点击展开
await mcp__victauri__find_elements({ text: "□" }) → 点击
// 验证 .panel-body 可见
```

- [ ] **Step 5: 验证添加/删除 Signal**

```typescript
// 点击 Signals 的 [+] 按钮
// 填写名称 "testSignal"、类型 "i32"、默认值 "0"
// 点击 ✓ 确认
await mcp__victauri__wait_for({ condition: "text", value: "testSignal" })
// 验证行出现
await mcp__victauri__assert_semantic({ expression: "document.querySelector('.entry-name')?.textContent", condition: "contains", expected: "testSignal" })
// 点击 ✕ 删除
// 验证行消失
```

- [ ] **Step 6: 验证缩放时面板跟随**

```typescript
// 记录面板位置 → Ctrl+滚轮缩放 → 验证面板相对 Artboard 位置不变
await mcp__victauri__eval_js("document.querySelector('.component-def-panel')?.getBoundingClientRect()")
```

- [ ] **Step 7: Commit (如有调整)**

```bash
git add -A
git commit -m "test: verify ComponentDefPanel via Victauri browser testing"
```

---

### Task 7: 设置面板默认位置

**Files:**
- Modify: `poiesis/src/components/canvas/InfiniteCanvas.tsx`

**说明:** 面板默认放在 Artboard 右侧。需要在 InfiniteCanvas 中计算默认位置。

- [ ] **Step 1: 添加默认位置计算**

在 InfiniteCanvas 组件中添加 `useEffect`，在设备预设变更时重新计算面板默认位置：

```typescript
const devicePreset = useCanvasStore((s) => s.devicePreset);
const panelPosition = useCanvasStore((s) => s.panelPosition);
const setPanelPosition = useCanvasStore((s) => s.setPanelPosition);

// 面板默认位置：Artboard 右侧
useEffect(() => {
  // 只在初始加载时设置（panelPosition 为 {0,0} 时）
  if (panelPosition.x === 0 && panelPosition.y === 0) {
    const artboardWidth = devicePreset.width || 393;
    setPanelPosition(artboardWidth + 40, 0);
  }
}, []); // 仅在 mount 时运行一次
```

- [ ] **Step 2: 验证编译**

Run: `cd poiesis && npx tsc --noEmit 2>&1 | head -5`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/InfiniteCanvas.tsx
git commit -m "feat: set ComponentDefPanel default position next to Artboard"
```
