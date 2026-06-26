# Editor Architecture Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor editor from 25,000-line monolith into 5-layer architecture (core → services → ui → features → app) with zero functional or UI changes. Fix 21 consistency bugs along the way.

**Architecture:** Five dependently-typed layers: `core/` (types, state atoms, commands, events) → `services/` (codegen, compiler, project, runtime, api) → `ui/` (shell, canvas, inspector, panels, editor) → `features/` (design-mode, code-mode, project-explorer, compiler, debug) → `app/` (pure assembly, ~80 lines). Single npm package, no monorepo.

**Tech Stack:** TypeScript, React 18, Zustand, mitt, Vite, Vitest, Playwright (E2E), Python/FastAPI (server)

---

## File Structure Plan

### Files to CREATE

```
editor/client/src/
├── core/
│   ├── types/
│   │   ├── element.ts          # ElementData, PortData
│   │   ├── wire.ts             # WireData
│   │   ├── project.ts          # ProjectConfig
│   │   ├── style.ts            # StyleEntry, StyleProperty, STYLE_PROPERTY_NAMES
│   │   ├── codegraph.ts        # CodeTab, ChainBlock, CodeTabType
│   │   ├── annotation.ts       # AnnotationFile, ViewMarker, Chain, NodeEntry, WireEntry
│   │   └── index.ts
│   ├── constants/
│   │   ├── components.ts       # CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS
│   │   ├── defaults.ts         # getDefaultProps(tag)
│   │   └── index.ts
│   ├── utils/
│   │   ├── id.ts               # uid(prefix)
│   │   ├── omni.ts             # extractComponentName(source, fallback?)
│   │   └── index.ts
│   ├── state/
│   │   ├── elements.ts         # useElementStore
│   │   ├── wires.ts            # useWireStore
│   │   ├── selection.ts        # useSelectionStore
│   │   ├── styles.ts           # useStyleStore
│   │   ├── project.ts          # useProjectStore
│   │   ├── tabs.ts             # useTabStore
│   │   ├── undo.ts             # useUndoStore (migrated)
│   │   ├── componentRegistry.ts # useComponentRegistry (migrated)
│   │   ├── codegraph.ts        # useCodeGraphStore (migrated)
│   │   └── index.ts
│   ├── commands/               # (migrated from src/commands/)
│   ├── events/
│   │   └── bus.ts              # mitt editorBus + EditorEvents type
│   └── index.ts
│
├── services/
│   ├── codegen/                # (migrated from src/omni-gen/)
│   ├── compiler/
│   │   ├── compiler.ts         # CompilerService
│   │   └── index.ts
│   ├── project/
│   │   ├── project-service.ts  # ProjectService
│   │   ├── component-service.ts # component save/load
│   │   └── index.ts
│   ├── runtime/
│   │   ├── run-service.ts
│   │   ├── debug-service.ts
│   │   └── index.ts
│   ├── api/
│   │   ├── client.ts
│   │   ├── compile-api.ts
│   │   ├── project-api.ts
│   │   ├── annotation-api.ts
│   │   └── index.ts
│   └── index.ts
│
├── ui/
│   ├── shell/
│   │   ├── AppShell.tsx
│   │   ├── Toolbar.tsx
│   │   ├── DesignToolbar.tsx
│   │   ├── CodeToolbar.tsx
│   │   ├── StatusBar.tsx
│   │   ├── theme.ts
│   │   └── index.ts
│   ├── canvas/
│   │   ├── design/
│   │   │   └── DesignCanvas.tsx
│   │   ├── code/
│   │   │   └── CodeCanvas.tsx
│   │   └── shared/             # (migrated existing files)
│   ├── inspector/
│   │   └── sections/
│   │       ├── LayoutSection.tsx
│   │       ├── StyleSection.tsx
│   │       ├── EventSection.tsx
│   │       └── DataSection.tsx
│   └── panels/
│       ├── TreeNodeRenderer.tsx
│       └── tree-utils.ts
│
├── features/
│   ├── design-mode/
│   │   ├── DesignModeLayout.tsx
│   │   └── useDesignKeyboard.ts
│   ├── code-mode/
│   │   ├── CodeModeLayout.tsx
│   │   └── useCodeKeyboard.ts
│   ├── project-explorer/
│   │   ├── useProjectInit.ts
│   │   ├── useProjectLoader.ts
│   │   └── useProjectSaver.ts
│   ├── compiler/
│   │   ├── useCompile.ts
│   │   ├── useRun.ts
│   │   └── useAutoCompile.ts
│   └── debug/
│       └── useDebugSession.ts
│
└── app/
    └── App.tsx                 # (rewritten, ~80 lines)
```

### Files to DELETE

```
editor/client/src/canvas/toast.ts    # dead code, no imports
```

### Files to MODIFY (import path updates only)

All 116 source files will have import paths updated as modules move. This is mechanical and task steps specify exact paths.

---

## Phase 1: core/ Foundation

### Task 1.1: Install mitt, create event bus

**Files:**
- Create: `editor/client/src/core/events/bus.ts`

- [ ] **Step 1: Install mitt**

```bash
cd editor/client && npm install mitt
```

- [ ] **Step 2: Create `core/events/bus.ts`**

```typescript
// core/events/bus.ts
import mitt from 'mitt';

export type EditorEvents = {
  'element:added': { elementId: string };
  'element:removed': { elementId: string };
  'element:selected': { elementIds: string[] };
  'wire:added': { wireId: string };
  'wire:removed': { wireId: string };
  'project:loaded': { path: string };
  'project:saved': { path: string };
  'compile:completed': { errors: Array<{ code: string; message: string; line: number }> };
  'mode:changed': { mode: 'design' | 'code' };
  'tab:switched': { docId: string | null };
};

export const editorBus = mitt<EditorEvents>();
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 4: Commit**

```bash
git add editor/client/src/core/events/bus.ts editor/client/package.json editor/client/package-lock.json
git commit -m "feat(core): add mitt event bus with EditorEvents type definitions"
```

---

### Task 1.2: Create core/types/ — migrate all shared types

**Files:**
- Create: `editor/client/src/core/types/element.ts`, `wire.ts`, `project.ts`, `style.ts`, `codegraph.ts`, `annotation.ts`, `index.ts`
- Modify: All files that import from `store/editor`, `codegraph/types`, `annotation/types`, `constants/style-properties`

- [ ] **Step 1: Create `core/types/element.ts`**

Extract ElementData and PortData from `store/editor.ts:9-39`:

```typescript
// core/types/element.ts
export interface PortData {
  name: string;
  direction: "in" | "out";
  portType?: "flow" | "data";
  type: string;
  label?: string;
  configurable?: boolean;
  defaultValue?: unknown;
}

export interface ElementData {
  id: string;
  nodeType: "view" | "codeblock" | "group";
  tag: string;
  props: Record<string, unknown>;
  styles?: import("./style").StyleEntry[];
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
  nodeCategory?: "flow" | "event" | "action" | "data" | "variable" | "code";
  config?: Record<string, unknown>;
  flowOrder?: number;
  componentRef?: string;
}
```

- [ ] **Step 2: Create `core/types/wire.ts`**

```typescript
// core/types/wire.ts
export interface WireData {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  isCollapsed?: boolean;
  collapseGroupId?: string;
}
```

- [ ] **Step 3: Create `core/types/project.ts`**

```typescript
// core/types/project.ts
export interface ProjectConfig {
  target: string;
  autoSave: boolean;
}
```

- [ ] **Step 4: Create `core/types/style.ts`**

Migrate from `constants/style-properties.ts`:

```typescript
// core/types/style.ts
export interface StyleProperty {
  name: string;
  value: string;
}

export interface StyleEntry {
  id: string;
  type: "class" | "variable" | "inline";
  name: string;
  properties: StyleProperty[];
}

export const STYLE_PROPERTY_NAMES = new Set([
  "padding", "margin", "width", "height", "minWidth", "maxWidth",
  "minHeight", "maxHeight", "display", "flexDirection", "justifyContent",
  "alignItems", "gap", "flexWrap", "flex", "position", "top", "left",
  "right", "bottom", "zIndex", "overflow", "opacity", "borderRadius",
  "border", "borderTop", "borderBottom", "borderLeft", "borderRight",
  "backgroundColor", "color", "fontSize", "fontWeight", "fontFamily",
  "textAlign", "lineHeight", "letterSpacing", "textTransform",
  "boxShadow", "transition", "transform", "cursor", "pointerEvents",
  "objectFit", "aspectRatio", "background", "backgroundImage",
  "backgroundSize", "backgroundPosition", "backgroundRepeat",
  "gridTemplateColumns", "gridTemplateRows", "gridGap",
]);

export function variableToClassName(varName: string): string {
  return varName.replace(/^--/, "").replace(/-([a-z])/g, (_, c) => c.toUpperCase());
}

export function classNameToVariable(className: string): string | null {
  if (!/^[a-z][a-zA-Z0-9]*$/.test(className)) return null;
  return `--${className.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
}
```

- [ ] **Step 5: Create `core/types/codegraph.ts`**

Migrate from `codegraph/types.ts`:

```typescript
// core/types/codegraph.ts
import type { ElementData } from "./element";
import type { WireData } from "./wire";

export type CodeTabType = "logic" | "event";

export interface CodeGraph {
  elements: ElementData[];
  wires: WireData[];
}

export interface CodeTabVariable {
  name: string;
  type: string;
  defaultValue?: unknown;
}

export interface CodeTab {
  id: string;
  type: CodeTabType;
  label: string;
  icon: string;
  graph: CodeGraph;
  variables: CodeTabVariable[];
}

export interface ChainBlock {
  functionName: string;
  nodeLines: string[];
  wireLines: string[];
}
```

- [ ] **Step 6: Create `core/types/annotation.ts`**

Migrate from `annotation/types.ts`:

```typescript
// core/types/annotation.ts
export type ViewRenderMode = "dom" | "canvas";

export interface ViewMarker {
  elementId: string;
  lineNumber: number;
  isFolded: boolean;
  renderMode: ViewRenderMode;
}

export interface NodeEntry {
  id: string;
  tag: string;
  attrs: Record<string, string>;
  posX: number;
  posY: number;
  lineNumber: number;
}

export interface WireEntry {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  isCollapsed: boolean;
  collapseGroupId: string;
  lineNumber: number;
}

export interface Chain {
  name: string;
  startLine: number;
  endLine: number;
  nodes: NodeEntry[];
  wires: WireEntry[];
}

export type CodeTabType_Annotation = "logic" | "event";

export interface AnnotationFile {
  filepath: string;
  codeTabType: CodeTabType_Annotation;
  codeTabLine: number;
  viewMarkers: ViewMarker[];
  chains: Chain[];
  rawSource: string;
}

export interface AnnotatedLoadResponse {
  success: boolean;
  source?: string;
  annotations?: Record<string, unknown>;
  validationErrors?: Array<{ code: string; message: string; line: number }>;
  error?: string;
}

export interface AnnotatedSaveResponse {
  ok: boolean;
  path?: string;
  error?: string;
  validationErrors?: Array<{ code: string; message: string; line: number }>;
}

export interface EditorTomlState {
  active_file?: string;
  active_mode?: string;
  open_files?: string[];
  left_open?: boolean;
  right_width?: number;
  viewports?: Record<string, unknown>;
}
```

- [ ] **Step 7: Create `core/types/index.ts`**

```typescript
export * from "./element";
export * from "./wire";
export * from "./project";
export * from "./style";
export * from "./codegraph";
export * from "./annotation";
```

- [ ] **Step 8: Update all import paths across the codebase**

Replace imports from old locations to new `core/types/`:
- `from "../store/editor"` (ElementData, WireData, PortData, ProjectConfig) → `from "../core/types"`
- `from "../codegraph/types"` → `from "../core/types"`
- `from "../annotation/types"` → `from "../core/types"`
- `from "../constants/style-properties"` → `from "../core/types"`

Run after all replacements:

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

Fix any remaining type import errors before continuing.

- [ ] **Step 9: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 10: Commit**

```bash
git add editor/client/src/core/types/ 
git add -u  # all modified import paths
git commit -m "refactor(core): extract shared types to core/types/ — ElementData, WireData, StyleEntry, CodeTab, AnnotationFile"
```

---

### Task 1.3: Split store/editor.ts → 6 state atoms

**Files:**
- Create: `editor/client/src/core/state/elements.ts`, `wires.ts`, `selection.ts`, `styles.ts`, `project.ts`, `tabs.ts`, `index.ts`
- Modify: `editor/client/src/store/editor.ts` → re-export from new atoms for backward compat

- [ ] **Step 1: Create `core/state/elements.ts`**

Extract element-related state and actions from `store/editor.ts`:

```typescript
// core/state/elements.ts
import { create } from "zustand";
import type { ElementData } from "../types/element";

interface ElementState {
  elements: ElementData[];
  rootOrder: string[];

  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  setElementSource: (id: string, source: string) => void;
  setElementPorts: (id: string, ports: ElementData["ports"]) => void;
  setEventBindings: (id: string, eventName: string, binding: {
    fnName: string;
    paramBindings: Record<string, { sourceType: string; value: string; expectedType: string }>;
  }) => void;
  removeEventBindings: (id: string, eventName: string) => void;
  setBlockValue: (id: string, value: unknown) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  setElementId: (oldId: string, newId: string) => void;
  setElementsAndWires: (elements: ElementData[], wires: import("../types/wire").WireData[]) => void;
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
}

export const useElementStore = create<ElementState>((set, get) => ({
  elements: [],
  rootOrder: [],

  addElement: (el) =>
    set((s) => {
      const normalized = { ...el, props: el.props || {} };
      const elements = [...s.elements, normalized];
      if (normalized.parentId) {
        return {
          elements: elements.map((e) =>
            e.id === normalized.parentId
              ? { ...e, children: [...e.children, normalized.id] }
              : e
          ),
        };
      }
      return {
        elements,
        rootOrder: [...s.rootOrder, normalized.id],
      };
    }),

  removeElement: (id) =>
    set((s) => ({
      elements: (() => {
        const el = s.elements.find((e) => e.id === id);
        let elements = s.elements.filter((e) => e.id !== id);
        if (el?.parentId) {
          elements = elements.map((e) =>
            e.id === el.parentId
              ? { ...e, children: e.children.filter((cid) => cid !== id) }
              : e
          );
        }
        elements = elements.filter((e) => e.parentId !== id);
        return elements;
      })(),
      rootOrder: s.rootOrder.filter((rid) => rid !== id),
    })),

  setProp: (id, prop, value) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e
      ),
    })),

  getProp: (id, prop) => get().elements.find((e) => e.id === id)?.props[prop],

  setElementSource: (id, source) =>
    set((s) => ({
      elements: s.elements.map((e) => (e.id === id ? { ...e, source } : e)),
    })),

  setElementPorts: (id, ports) =>
    set((s) => ({
      elements: s.elements.map((e) => (e.id === id ? { ...e, ports } : e)),
    })),

  setEventBindings: (id, eventName, binding) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        return {
          ...e,
          config: {
            ...e.config,
            _eventBindings: { ...currentBindings, [eventName]: binding },
          },
        };
      }),
    })),

  removeEventBindings: (id, eventName) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        const updated = { ...currentBindings };
        delete updated[eventName];
        return { ...e, config: { ...e.config, _eventBindings: updated } };
      }),
    })),

  setBlockValue: (id, value) =>
    set((s) => {
      const existing = s.elements.find((e) => e.id === id);
      if (existing && JSON.stringify(existing.outputValue) === JSON.stringify(value)) return s;
      return {
        elements: s.elements.map((e) =>
          e.id === id ? { ...e, outputValue: value } : e
        ),
      };
    }),

  setElementPosition: (id, pos) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, position: { ...pos } } : e
      ),
    })),

  setElementId: (oldId, newId) =>
    set((s) => {
      if (s.elements.some((e) => e.id === newId)) return s;
      return {
        elements: s.elements.map((e) => {
          if (e.id === oldId) return { ...e, id: newId };
          if (e.parentId === oldId) return { ...e, parentId: newId };
          return e;
        }),
        rootOrder: s.rootOrder.map((id) => (id === oldId ? newId : id)),
      };
    }),

  setElementsAndWires: (elements, _wires) =>
    set({
      elements,
      rootOrder: elements.filter((e) => !e.parentId).map((e) => e.id),
    }),
  // Note: wires sync + selection clear handled by cross-store coordination in core/state/index.ts

  reorderElement: (elementId, targetSiblingId, position) =>
    set((s) => {
      const el = s.elements.find((e) => e.id === elementId);
      const target = s.elements.find((e) => e.id === targetSiblingId);
      if (!el || !target) return s;
      if (el.parentId === null || target.parentId === null) {
        if (el.parentId !== target.parentId) return s;
        const order = [...s.rootOrder];
        const fromIdx = order.indexOf(elementId);
        const toIdx = order.indexOf(targetSiblingId);
        if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return s;
        order.splice(fromIdx, 1);
        const newToIdx = order.indexOf(targetSiblingId);
        const insertIdx = position === "before" ? newToIdx : newToIdx + 1;
        order.splice(insertIdx, 0, elementId);
        const elements = [...s.elements];
        const [movedEl] = elements.splice(fromIdx, 1);
        elements.splice(insertIdx, 0, movedEl);
        return { rootOrder: order, elements };
      }
      if (el.parentId !== target.parentId) return s;
      const parentId = el.parentId;
      const parent = s.elements.find((e) => e.id === parentId);
      if (!parent) return s;
      const children = [...parent.children];
      const fromIdx = children.indexOf(elementId);
      const toIdx = children.indexOf(targetSiblingId);
      if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return s;
      children.splice(fromIdx, 1);
      const insertIdx = position === "before" ? children.indexOf(targetSiblingId) : children.indexOf(targetSiblingId) + 1;
      children.splice(insertIdx, 0, elementId);
      return {
        elements: s.elements.map((e) =>
          e.id === parentId ? { ...e, children } : e
        ),
      };
    }),

  moveElement: (elementId, newParentId, targetSiblingId, position) =>
    set((s) => {
      const el = s.elements.find((e) => e.id === elementId);
      if (!el) return s;
      const oldParentId = el.parentId;
      let elements = s.elements.map((e) =>
        e.id === elementId ? { ...e, parentId: newParentId } : e
      );
      if (oldParentId) {
        elements = elements.map((e) =>
          e.id === oldParentId
            ? { ...e, children: e.children.filter((cid) => cid !== elementId) }
            : e
        );
      }
      if (newParentId) {
        const newParent = elements.find((e) => e.id === newParentId);
        if (!newParent) return s;
        const newChildren = [...newParent.children];
        if (targetSiblingId && newChildren.includes(targetSiblingId)) {
          const targetIdx = newChildren.indexOf(targetSiblingId);
          const insertIdx = position === "before" ? targetIdx : targetIdx + 1;
          newChildren.splice(insertIdx, 0, elementId);
        } else {
          newChildren.push(elementId);
        }
        elements = elements.map((e) =>
          e.id === newParentId ? { ...e, children: newChildren } : e
        );
      }
      return { elements };
    }),
}));
```

- [ ] **Step 2: Create `core/state/wires.ts`**

```typescript
// core/state/wires.ts
import { create } from "zustand";
import type { WireData } from "../types/wire";

interface WireState {
  wires: WireData[];
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setWires: (wires: WireData[]) => void;
}

export const useWireStore = create<WireState>((set) => ({
  wires: [],
  addWire: (w) => set((s) => ({ wires: [...s.wires, w] })),
  removeWire: (id) => set((s) => ({ wires: s.wires.filter((w) => w.id !== id) })),
  setWires: (wires) => set({ wires }),
}));
```

- [ ] **Step 3: Create `core/state/selection.ts`**

```typescript
// core/state/selection.ts
import { create } from "zustand";

interface SelectionState {
  selectedElementIds: string[];
  selectElement: (id: string | null, multi?: boolean) => void;
  selectAll: (allIds: string[]) => void;
  clearSelection: () => void;
}

export const useSelectionStore = create<SelectionState>((set) => ({
  selectedElementIds: [],

  selectElement: (id, multi = false) =>
    set((s) => {
      if (id === null) return { selectedElementIds: [] };
      if (multi) {
        const exists = s.selectedElementIds.includes(id);
        return {
          selectedElementIds: exists
            ? s.selectedElementIds.filter((x) => x !== id)
            : [...s.selectedElementIds, id],
        };
      }
      return { selectedElementIds: [id] };
    }),

  selectAll: (allIds) => set({ selectedElementIds: allIds }),

  clearSelection: () => set({ selectedElementIds: [] }),
}));
```

- [ ] **Step 4: Create `core/state/styles.ts`**

```typescript
// core/state/styles.ts
import { create } from "zustand";
import type { StyleEntry } from "../types/style";

interface StyleState {
  addStyleEntry: (elementId: string, entry: StyleEntry, updateElements: (fn: (elements: any[]) => any[]) => void) => void;
  removeStyleEntry: (elementId: string, entryId: string, updateElements: (fn: (elements: any[]) => any[]) => void) => void;
  updateStyleEntry: (elementId: string, entryId: string, patch: Partial<StyleEntry>, updateElements: (fn: (elements: any[]) => any[]) => void) => void;
  reorderStyleEntries: (elementId: string, entryIds: string[], updateElements: (fn: (elements: any[]) => any[]) => void) => void;
}

// Style operations need access to elements store for updating element.styles[].
// The updateElements callback bridges this — callers pass useElementStore.setState.
export const styleOperations = {
  addStyleEntry: (elementId: string, entry: StyleEntry, updateElements: (fn: (elements: any[]) => any[]) => void) => {
    updateElements((elements) =>
      elements.map((e: any) =>
        e.id === elementId ? { ...e, styles: [...(e.styles || []), entry] } : e
      )
    );
  },

  removeStyleEntry: (elementId: string, entryId: string, updateElements: (fn: (elements: any[]) => any[]) => void) => {
    updateElements((elements) =>
      elements.map((e: any) =>
        e.id === elementId
          ? { ...e, styles: (e.styles || []).filter((s: StyleEntry) => s.id !== entryId) }
          : e
      )
    );
  },

  updateStyleEntry: (elementId: string, entryId: string, patch: Partial<StyleEntry>, updateElements: (fn: (elements: any[]) => any[]) => void) => {
    updateElements((elements) =>
      elements.map((e: any) =>
        e.id === elementId
          ? {
              ...e,
              styles: (e.styles || []).map((s: StyleEntry) =>
                s.id === entryId ? { ...s, ...patch } : s
              ),
            }
          : e
      )
    );
  },

  reorderStyleEntries: (elementId: string, entryIds: string[], updateElements: (fn: (elements: any[]) => any[]) => void) => {
    updateElements((elements) => {
      const el = elements.find((e: any) => e.id === elementId);
      if (!el?.styles) return elements;
      const byId = new Map(el.styles.map((s: StyleEntry) => [s.id, s]));
      if (!entryIds.every((id) => byId.has(id))) return elements;
      const reordered = entryIds.map((id) => byId.get(id)!);
      return elements.map((e: any) =>
        e.id === elementId ? { ...e, styles: reordered } : e
      );
    });
  },
};
```

- [ ] **Step 5: Create `core/state/project.ts`**

```typescript
// core/state/project.ts
import { create } from "zustand";
import type { ProjectConfig } from "../types/project";

interface ProjectState {
  projectDir: string | null;
  projectPath: string | null;
  projectName: string;
  sourceCode: string;
  isDirty: boolean;
  activeFilePath: string | null;
  activeFileContent: string | null;
  activeMode: "design" | "code";
  config: ProjectConfig;
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;
  files: Array<{ name: string; path: string; type: "file" | "directory" }>;

  setProjectDir: (dir: string) => void;
  loadFiles: (entries: Array<{ name: string; path: string; type: "file" | "directory" }>) => void;
  openFile: (path: string, content: string) => void;
  setActiveFileContent: (content: string) => void;
  setSourceCode: (code: string) => void;
  setCompileErrors: (errors: Array<{ code: string; message: string; line: number }>) => void;
  setPreviewUrl: (url: string | null) => void;
  setActiveMode: (mode: "design" | "code") => void;
  setConfig: (patch: Partial<ProjectConfig>) => void;
  markDirty: () => void;
  markClean: () => void;
  newProject: () => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  projectDir: null,
  projectPath: null,
  projectName: "未命名项目",
  sourceCode: "",
  isDirty: false,
  activeFilePath: null,
  activeFileContent: null,
  activeMode: "design",
  config: { target: "react", autoSave: false },
  compileErrors: [],
  previewUrl: null,
  files: [],

  setProjectDir: (dir) => set({ projectDir: dir }),
  loadFiles: (entries) => set({ files: entries }),
  openFile: (path, content) => set({ activeFilePath: path, activeFileContent: content }),
  setActiveFileContent: (content) => set({ activeFileContent: content }),
  setSourceCode: (code) => set({ sourceCode: code, isDirty: true }),
  setCompileErrors: (errors) => set({ compileErrors: errors }),
  setPreviewUrl: (url) => set({ previewUrl: url }),
  setActiveMode: (mode) => set({ activeMode: mode }),
  setConfig: (patch) => set((s) => ({ config: { ...s.config, ...patch }, isDirty: true })),
  markDirty: () => set({ isDirty: true }),
  markClean: () => set({ isDirty: false }),
  newProject: () =>
    set({
      projectPath: null,
      projectName: "未命名项目",
      sourceCode: "",
      isDirty: false,
    }),
}));
```

- [ ] **Step 6: Create `core/state/tabs.ts`**

```typescript
// core/state/tabs.ts
import { create } from "zustand";
import type { ElementData } from "../types/element";
import type { WireData } from "../types/wire";

interface TabState {
  activeDocId: string | null;
  docBackup: { elements: ElementData[]; wires: WireData[] } | null;
  openTabs: string[];
  tabStates: Record<string, { elements: ElementData[]; wires: WireData[] }>;
  compVersion: number;

  enterComponentCanvas: (componentName: string, elements: ElementData[], wires: WireData[], registrySaveCanvas: (name: string, elements: ElementData[], wires: WireData[]) => void, clearUndo: () => void) => void;
  exitComponentCanvas: (registrySaveCanvas: (name: string, elements: ElementData[], wires: WireData[]) => void, clearUndo: () => void) => void;
  switchTab: (docId: string | null, opts: {
    elements: ElementData[];
    wires: WireData[];
    registrySaveCanvas: (name: string, elements: ElementData[], wires: WireData[]) => void;
    registryDefs: Record<string, { elements: ElementData[]; wires: WireData[] }>;
    clearUndo: () => void;
  }) => { elements: ElementData[]; wires: WireData[]; activeDocId: string | null; };
  closeTab: (docId: string, opts: {
    elements: ElementData[];
    wires: WireData[];
    docBackup: { elements: ElementData[]; wires: WireData[] } | null;
    registrySaveCanvas: (name: string, elements: ElementData[], wires: WireData[]) => void;
    clearUndo: () => void;
  }) => { elements: ElementData[]; wires: WireData[]; activeDocId: string | null; openTabs: string[]; tabStates: Record<string, { elements: ElementData[]; wires: WireData[] }> };
  openFileTab: (path: string, name: string, elements: ElementData[], wires: WireData[]) => void;
}

// Tab state is tightly coupled with elements/wires from other stores.
// This store provides the tab navigation logic; the actual element/wire swapping
// happens in the caller (editor store index).
export const useTabStore = create<TabState>((set, get) => ({
  activeDocId: null,
  docBackup: null,
  openTabs: [],
  tabStates: {},
  compVersion: 0,

  enterComponentCanvas: (componentName, elements, wires, registrySaveCanvas, clearUndo) =>
    set((s) => {
      const backup = { elements, wires };
      const tabKey = `comp:${componentName}`;
      const newOpenTabs = s.openTabs.includes(tabKey) ? s.openTabs : [...s.openTabs, tabKey];
      clearUndo();
      return {
        docBackup: backup,
        activeDocId: tabKey,
        openTabs: newOpenTabs,
        selectedElementIds: [],
      };
    }),

  exitComponentCanvas: (registrySaveCanvas, clearUndo) =>
    set((s) => {
      if (!s.activeDocId || !s.activeDocId.startsWith("comp:")) return s;
      const compName = s.activeDocId.replace("comp:", "");
      clearUndo();
      return {
        docBackup: null,
        activeDocId: null,
        compVersion: s.compVersion + 1,
      };
    }),

  switchTab: (docId, opts) => {
    // Complex tab-switching logic — see store/editor.ts switchTab for full implementation
    // This is extracted 1:1 from the original
    const s = get();
    const newTabStates = { ...s.tabStates };
    const currentKey = s.activeDocId;

    if (currentKey && currentKey.startsWith("file:")) {
      newTabStates[currentKey] = { elements: opts.elements, wires: opts.wires };
      const filePath = currentKey.replace("file:", "");
      const compMatch = filePath.match(/src[/\\]components[/\\](.+)\.omni$/);
      if (compMatch) {
        const compName = compMatch[1];
        if (opts.registryDefs[compName]) {
          opts.registrySaveCanvas(compName, opts.elements, opts.wires);
        }
      }
    } else if (currentKey && currentKey.startsWith("comp:")) {
      const compName = currentKey.replace("comp:", "");
      opts.registrySaveCanvas(compName, opts.elements, opts.wires);
    }

    let newElements: ElementData[];
    let newWires: WireData[];
    let newDocBackup = s.docBackup;

    if (docId === null) {
      newElements = s.docBackup?.elements || [];
      newWires = s.docBackup?.wires || [];
      newDocBackup = null;
    } else if (docId.startsWith("comp:")) {
      const compName = docId.replace("comp:", "");
      const compDef = opts.registryDefs[compName];
      if (!compDef) return s;
      newDocBackup = s.activeDocId === null
        ? { elements: opts.elements, wires: opts.wires }
        : s.docBackup;
      newElements = compDef.elements;
      newWires = compDef.wires;
    } else if (docId.startsWith("file:")) {
      const cached = newTabStates[docId];
      if (cached) {
        newElements = cached.elements;
        newWires = cached.wires;
      } else {
        return s;
      }
      if (s.activeDocId === null) {
        newDocBackup = { elements: opts.elements, wires: opts.wires };
      }
    } else {
      return s;
    }

    opts.clearUndo();
    return {
      activeDocId: docId,
      docBackup: newDocBackup,
      tabStates: newTabStates,
      compVersion: (currentKey?.startsWith("comp:")) ? s.compVersion + 1 : s.compVersion,
    };
  },

  closeTab: (docId, opts) => {
    const s = get();
    const newOpenTabs = s.openTabs.filter((t) => t !== docId);
    const newTabStates = { ...s.tabStates };
    delete newTabStates[docId];

    if (s.activeDocId === docId) {
      if (docId.startsWith("comp:")) {
        const compName = docId.replace("comp:", "");
        opts.registrySaveCanvas(compName, opts.elements, opts.wires);
      } else if (docId.startsWith("file:")) {
        newTabStates[docId] = { elements: opts.elements, wires: opts.wires };
      }
      opts.clearUndo();
      return {
        docBackup: null,
        activeDocId: null,
        openTabs: newOpenTabs,
        tabStates: newTabStates,
        elements: opts.docBackup?.elements || [],
        wires: opts.docBackup?.wires || [],
      };
    }
    return { openTabs: newOpenTabs, tabStates: newTabStates, elements: opts.elements, wires: opts.wires, activeDocId: s.activeDocId };
  },

  openFileTab: (path, name, elements, wires) =>
    set((s) => {
      const tabKey = `file:${path}`;
      if (s.openTabs.includes(tabKey)) return s;
      return {
        openTabs: [...s.openTabs, tabKey],
        tabStates: { ...s.tabStates, [tabKey]: { elements, wires } },
      };
    }),
}));
```

- [ ] **Step 7: Create `core/state/index.ts` — cross-store coordination**

```typescript
// core/state/index.ts
// Cross-store coordination: when an element is removed, also remove its wires.
import { useElementStore } from "./elements";
import { useWireStore } from "./wires";
import { editorBus } from "../events/bus";

export function removeElementWithWires(elementId: string): void {
  const el = useElementStore.getState().elements.find((e) => e.id === elementId);
  if (!el) return;

  // Collect all descendant IDs recursively
  const idsToRemove = new Set<string>();
  const stack = [elementId];
  while (stack.length > 0) {
    const id = stack.pop()!;
    idsToRemove.add(id);
    const child = useElementStore.getState().elements.find((e) => e.id === id);
    if (child?.children) {
      for (const cid of child.children) stack.push(cid);
    }
  }

  // Remove wires connected to any removed element
  const wires = useWireStore.getState().wires;
  const wiresToRemove = wires.filter(
    (w) => idsToRemove.has(w.fromId) || idsToRemove.has(w.toId)
  );

  for (const w of wiresToRemove) {
    useWireStore.getState().removeWire(w.id);
  }

  // Remove elements (bottom-up: children first)
  const ordered = [...idsToRemove];
  for (const id of ordered) {
    useElementStore.getState().removeElement(id);
  }

  editorBus.emit("element:removed", { elementId });
}

export function addElementWithEvent(el: Parameters<typeof useElementStore.getState().addElement>[0]): void {
  useElementStore.getState().addElement(el);
  editorBus.emit("element:added", { elementId: el.id });
}

export function addWireWithEvent(w: Parameters<typeof useWireStore.getState().addWire>[0]): void {
  useWireStore.getState().addWire(w);
  editorBus.emit("wire:added", { wireId: w.id });
}
```

- [ ] **Step 8: Migrate undo and componentRegistry stores**

Copy `store/undo.ts` to `core/state/undo.ts` (1:1, only update imports to use core/types).
Copy `store/componentRegistry.ts` to `core/state/componentRegistry.ts` (1:1, only update imports).
Copy `codegraph/store.ts`, `codegraph/context.ts`, `codegraph/types.ts`, `codegraph/migration.ts` to `core/state/codegraph/` (1:1, update imports).

- [ ] **Step 9: Update `store/editor.ts` — re-export from new atoms for backward compat**

```typescript
// store/editor.ts — backward-compatible re-exports
export { useElementStore as useEditorStore } from "../core/state/elements";
export { useWireStore } from "../core/state/wires";
export { useSelectionStore } from "../core/state/selection";
export { styleOperations } from "../core/state/styles";
export { useProjectStore } from "../core/state/project";
export { useTabStore } from "../core/state/tabs";
export { useUndoStore } from "../core/state/undo";
export { useComponentRegistry } from "../core/state/componentRegistry";
export { useCodeGraphStore } from "../core/state/codegraph/store";
export * from "../core/types/element";
export * from "../core/types/wire";
export * from "../core/types/project";

// Re-export cross-store operations
export { removeElementWithWires, addElementWithEvent, addWireWithEvent } from "../core/state";
```

This keeps all existing imports working while the new atoms exist. Components can be gradually migrated to import from individual atoms.

- [ ] **Step 10: Run TypeScript check and all tests**

```bash
cd editor/client && npx tsc --noEmit --pretty false
cd editor/client && npx vitest run
```

Fix any type errors. All 17 test suites must pass.

- [ ] **Step 11: Commit**

```bash
git add editor/client/src/core/state/
git add editor/client/src/store/editor.ts
git commit -m "refactor(core): split editor store into 6 state atoms with backward-compat re-exports"
```

---

### Task 1.4: Migrate commands/ → core/commands/

**Files:**
- Move: `editor/client/src/commands/*.ts` → `editor/client/src/core/commands/*.ts`
- Modify: All import paths in moved files, all import paths in consumers

- [ ] **Step 1: Move all command files**

```bash
cd editor/client/src
cp commands/*.ts core/commands/
cp -r commands/__tests__ core/commands/__tests__
```

- [ ] **Step 2: Update internal imports in core/commands/**

Each command file that imports from `../store/editor` → `../../store/editor` (will use backward-compat re-exports).
Each command file that imports from `./types` → same, just ensure relative paths are correct after move.

- [ ] **Step 3: Update all consumer imports across codebase**

Search and replace:
- `from "../commands/AddElement"` → `from "../core/commands/AddElement"`
- `from "../commands/RemoveElement"` → `from "../core/commands/RemoveElement"`
- Same pattern for all 17 command modules
- `from "../commands/types"` → `from "../core/commands/types"`

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 4: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 5: Delete old commands/ directory**

```bash
rm -rf editor/client/src/commands/
```

- [ ] **Step 6: Commit**

```bash
git add editor/client/src/core/commands/
git add -u
git commit -m "refactor(core): migrate commands/ to core/commands/"
```

---

### Task 1.5: Create core/constants/ — extract shared constants

**Files:**
- Create: `editor/client/src/core/constants/components.ts`, `defaults.ts`, `index.ts`
- Modify: `editor/client/src/panels/ComponentPalette.tsx:21-37`, `editor/client/src/panels/ComponentTree.tsx:21-48`

- [ ] **Step 1: Create `core/constants/components.ts`**

```typescript
// core/constants/components.ts
export const CATEGORY_LABELS: Record<string, string> = {
  Layout: "布局", Typography: "排版", Actions: "操作", Forms: "表单",
  Data: "数据", Media: "媒体", Feedback: "反馈", Navigation: "导航",
  Custom: "★ 自定义",
};

export const CATEGORY_ORDER = ["Custom", "Layout", "Typography", "Actions", "Forms", "Data", "Media", "Feedback", "Navigation"];

export const COMPONENT_ICONS: Record<string, string> = {
  Container: "📦", Flex: "↔", Grid: "⊞", Card: "🃏",
  Text: "📝", Heading: "📰",
  Button: "🔘", Input: "📋", TextArea: "📄", Select: "📌", Checkbox: "☑",
  List: "📃", ListItem: "📎",
  Image: "🖼",
  Spinner: "⏳", Progress: "📊", Alert: "⚠",
  Link: "🔗", Form: "📨",
};
```

- [ ] **Step 2: Create `core/constants/defaults.ts`**

```typescript
// core/constants/defaults.ts
import { COMPONENT_PROTOCOL } from "../../data/componentProtocol";
import type { ElementData } from "../types/element";

export function getDefaultProps(tag: string): Record<string, unknown> {
  const meta = COMPONENT_PROTOCOL[tag];
  if (!meta) return {};
  const props: Record<string, unknown> = {};
  for (const [key, schema] of Object.entries(meta.props)) {
    if (schema.default !== undefined) {
      props[key] = schema.default;
    }
  }
  // Chinese placeholders for components without defaults
  if (tag === "Input" && !props.placeholder) props.placeholder = "输入...";
  if (tag === "TextArea" && !props.placeholder) props.placeholder = "输入...";
  if (tag === "Select" && !props.placeholder) props.placeholder = "选择...";
  if (tag === "Image" && !props.alt) props.alt = "图片";
  if (tag === "Progress" && props.value === "0") props.value = "50";
  if (tag === "Alert" && !props.label) props.label = "提示信息";
  return props;
}
```

- [ ] **Step 3: Create `core/constants/index.ts`**

```typescript
export * from "./components";
export * from "./defaults";
```

- [ ] **Step 4: Update ComponentPalette.tsx**

Remove the `CATEGORY_LABELS`, `CATEGORY_ORDER`, `COMPONENT_ICONS` constants (lines 15-37). Replace with:

```typescript
import { CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS } from "../core/constants";
```

- [ ] **Step 5: Update ComponentTree.tsx**

Remove the `CATEGORY_LABELS`, `CATEGORY_ORDER`, `COMPONENT_ICONS` constants (lines 21-37). **Keep `TAG_TO_HTML`** (lines 40-48) — it is ComponentTree-specific HTML tag resolution logic, not shared. Replace removed constants with:

```typescript
import { CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS } from "../core/constants";
```

- [ ] **Step 6: Update LayoutView.tsx — use getDefaultProps**

In `LayoutView.tsx:178-205` (`handleDrop`), replace the hardcoded 4 if-else:

```typescript
import { getDefaultProps } from "../core/constants";

// In handleDrop:
const props = getDefaultProps(tag);
```

- [ ] **Step 7: Update OmniDriver.ts — use getDefaultProps**

In `OmniDriver.ts:544-578` (component method), merge defaults:

```typescript
import { getDefaultProps } from "../core/constants";

// In component():
const defaultProps = getDefaultProps(tag);
const cleanProps: Record<string, unknown> = { ...defaultProps };
if (props) {
  for (const [k, v] of Object.entries(props)) {
    if (k !== "children") cleanProps[k] = v;
  }
}
```

- [ ] **Step 8: Update ComponentTree.tsx handleAddChild**

Replace inline `getDefaultProps` (lines 573-589) with import:

```typescript
import { getDefaultProps } from "../core/constants";
// Remove the local getDefaultProps function
```

- [ ] **Step 9: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 10: Commit**

```bash
git add editor/client/src/core/constants/
git add editor/client/src/panels/ComponentPalette.tsx
git add editor/client/src/panels/ComponentTree.tsx
git add editor/client/src/canvas/LayoutView.tsx
git add editor/client/src/driver/OmniDriver.ts
git commit -m "refactor(core): extract shared constants + getDefaultProps; unify component creation defaults across 3 entry points"
```

---

### Task 1.6: Delete dead code `canvas/toast.ts`

- [ ] **Step 1: Verify no imports**

```bash
cd editor/client && grep -r "canvas/toast" src/ --include="*.ts" --include="*.tsx"
```

Expected: no output (confirmed — zero imports).

- [ ] **Step 2: Delete the file**

```bash
rm editor/client/src/canvas/toast.ts
```

- [ ] **Step 3: Verify build**

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 4: Commit**

```bash
git rm editor/client/src/canvas/toast.ts
git commit -m "chore: remove dead code canvas/toast.ts — no imports found"
```

---

### Task 1.7: Create core/utils/ — unify ID generation

**Files:**
- Create: `editor/client/src/core/utils/id.ts`, `omni.ts`, `index.ts`
- Modify: 8 files with custom ID generators

- [ ] **Step 1: Create `core/utils/id.ts`**

```typescript
// core/utils/id.ts
let _counter = 0;

export function uid(prefix = "el"): string {
  const ts = Date.now().toString(36);
  const rand = Math.random().toString(36).slice(2, 8);
  const seq = (_counter++).toString(36);
  return `${prefix}-${ts}-${rand}-${seq}`;
}
```

- [ ] **Step 2: Create `core/utils/omni.ts`**

```typescript
// core/utils/omni.ts
export function extractComponentName(source: string, fallback = "App"): string {
  const match = source.match(/component\s+(\w+)\s*\{/);
  return match ? match[1] : fallback;
}

export function isOmniFile(path: string): boolean {
  return path.endsWith(".omni");
}
```

- [ ] **Step 3: Create `core/utils/index.ts`**

```typescript
export * from "./id";
export * from "./omni";
```

- [ ] **Step 4: Replace custom ID generators**

| File | Replace inline ID code | With |
|------|----------------------|------|
| `OmniDriver.ts:539` | `\`el-${Date.now()}-${Math.random()...}\`` | `uid("el")` |
| `OmniDriver.ts:1432` | inline `newId()` | `uid()` |
| `codegraph/store.ts:117-118` | custom `uid()` | `uid("cb")` |
| `codegraph/types.ts:120` | `\`tab-${Date.now()...}\`` | `uid("tab")` |
| `BlockPalette.tsx:47` | `\`cb-${tag}-${Date.now()}-${_seq++}\`` | `uid(\`cb-${tag}\`)` |
| `pipeline.ts:143-145` | `generateId()` → `\`s_${Date.now()}_${++_idCounter}\`` | `uid("s")` |
| `PropertyPanel.tsx:501` | `\`s_${Date.now()}\`` | `uid("s")` |
| `StatePanel.tsx:363,406` | `\`cb-${kind}-${Date.now()}-...\`` | `uid(\`cb-${kind}\`)` |
| `FunctionLibrary.tsx:201` | `\`cb-fn-${Date.now()}-...\`` | `uid("cb-fn")` |

For each replacement: add `import { uid } from "../core/utils";` at top of file.

- [ ] **Step 5: Replace extractComponentName calls**

In all 6 locations listed in 1.3.6, replace:
```typescript
const compMatch = source.match(/component\s+(\w+)\s*\{/);
const componentName = compMatch ? compMatch[1] : "App";
```
With:
```typescript
import { extractComponentName } from "../core/utils";
const componentName = extractComponentName(source);
```

- [ ] **Step 6: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 7: Commit**

```bash
git add editor/client/src/core/utils/
git add -u
git commit -m "refactor(core): unify ID generation to uid(prefix), extractComponentName to shared util"
```

---

### Task 1.8: Fix Undo bypass in OmniDriver + StatePanel

**Files:**
- Modify: `editor/client/src/driver/OmniDriver.ts`, `editor/client/src/panels/StatePanel.tsx`

- [ ] **Step 1: Fix OmniDriver.ts — use command pattern for element creation**

In `OmniDriver.ts:544-594` (component method), replace `this._store().addElement(el)`:

```typescript
import { AddElement } from "../core/commands/AddElement";
import type { CommandContext } from "../core/commands/types";

// Get the command context (same as App.tsx ctx)
const store = this._store();
const ctx: CommandContext = {
  getElement: (id: string) => store.getState().elements.find((e: any) => e.id === id),
  get elements() { return store.getState().elements; },
  get wires() { return store.getState().wires; },
  addElement: (el: any) => store.getState().addElement(el),
  removeElement: (id: string) => store.getState().removeElement(id),
  setProp: (id: string, p: string, v: unknown) => store.getState().setProp(id, p, v),
  getProp: (id: string, p: string) => store.getState().getProp(id, p) ?? null,
  addWire: (w: any) => store.getState().addWire(w),
  removeWire: (id: string) => store.getState().removeWire(id),
  setElementsAndWires: (elements: any[], wires: any[]) =>
    store.getState().setElementsAndWires(elements, wires),
  reorderElement: (...args: any[]) => (store.getState() as any).reorderElement(...args),
  moveElement: (...args: any[]) => (store.getState() as any).moveElement(...args),
  getRootOrder: () => (store.getState() as any).rootOrder,
};

const cmd = new AddElement(el);
cmd.execute(ctx);
```

Do the same for wire creation in OmniDriver (line 848): replace `store.addWire(...)` with `new AddWire(wire).execute(ctx)`.

- [ ] **Step 2: Fix StatePanel.tsx — use command pattern**

In `StatePanel.tsx:397-401` and `StatePanel.tsx:418-419`, replace:
```typescript
ctx.addElement(el);
```
With:
```typescript
import { AddElement } from "../core/commands/AddElement";
const cmd = new AddElement(el);
cmd.execute(ctx);
```

- [ ] **Step 3: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 4: Commit**

```bash
git add editor/client/src/driver/OmniDriver.ts
git add editor/client/src/panels/StatePanel.tsx
git commit -m "fix(undo): OmniDriver and StatePanel now use command pattern for undoable element/wire creation"
```

---

### Task 1.9: Phase 1 final verification

- [ ] **Step 1: Full TypeScript check**

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 2: Run all tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 3: Build**

```bash
cd editor/client && npm run build
```

- [ ] **Step 4: Visual smoke test**

```bash
cd editor && python -m editor.server.main --dir ../omni-test --port 5173 --no-frontend &
cd editor/client && npx vite --port 5174 &
# Open http://localhost:5174
# Verify: editor loads, can drag components, switch modes, save/reload
```

- [ ] **Step 5: Commit**

```bash
git commit -m "chore: Phase 1 complete — core/ foundation with types, state atoms, constants, utils, commands, events"
```

---

## Phase 2: Services Layer + Pipeline Unification

### Task 2.1: Migrate omni-gen/ → services/codegen/

**Files:**
- Move: `editor/client/src/omni-gen/*.ts` → `editor/client/src/services/codegen/*.ts`
- Move: `editor/client/src/omni-gen/__tests__/*.ts` → `editor/client/src/services/codegen/__tests__/*.ts`

- [ ] **Step 1: Move files**

```bash
cd editor/client/src
mkdir -p services/codegen/__tests__
cp omni-gen/*.ts services/codegen/
cp omni-gen/__tests__/*.ts services/codegen/__tests__/
```

- [ ] **Step 2: Update all internal imports in codegen files**

All imports from `../store/editor` → use `../../../store/editor` (backward-compat re-exports) or directly from `../../core/types`.
All imports from `../codegraph/types` → `../../core/types`.
All imports from `../constants/style-properties` → `../../core/types`.
All imports from `../store/componentRegistry` → `../../core/state/componentRegistry`.

- [ ] **Step 3: Update all consumer imports**

Replace `from "../omni-gen"` → `from "../services/codegen"` across all files.
Replace `from "../omni-gen/pipeline"` → `from "../services/codegen/pipeline"`.
Same for all other omni-gen sub-imports.

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 4: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 5: Delete old omni-gen/ directory**

```bash
rm -rf editor/client/src/omni-gen/
```

- [ ] **Step 6: Commit**

```bash
git add editor/client/src/services/codegen/
git add -u
git rm -r editor/client/src/omni-gen/
git commit -m "refactor(services): migrate omni-gen/ to services/codegen/"
```

---

### Task 2.2: Split pipeline.ts — extract sub-modules

**Files:**
- Create: `editor/client/src/services/codegen/expand-custom-components.ts`, `collect-styles.ts`
- Modify: `editor/client/src/services/codegen/pipeline.ts`

- [ ] **Step 1: Create `expand-custom-components.ts`**

Extract the `expandCustomComponents` function from pipeline.ts (lines 45-111). 1:1 copy, update imports.

- [ ] **Step 2: Create `collect-styles.ts`**

Extract the `collectStylesFromEntries` function + `walkViewNodes` + `applyStyles` from pipeline.ts (lines 628-731). 1:1 copy, update imports.

- [ ] **Step 3: Update `pipeline.ts`**

Import from the new modules:

```typescript
import { expandCustomComponents } from "./expand-custom-components";
import { collectStylesFromEntries } from "./collect-styles";
```

Remove the extracted function bodies. pipeline.ts should now be ~100 lines (pure orchestration).

- [ ] **Step 4: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/services/codegen/
git commit -m "refactor(services): split pipeline.ts — extract expandCustomComponents and collectStyles"
```

---

### Task 2.3: Merge emitCustomComponents into generateOmniSource

**Files:**
- Modify: `editor/client/src/services/codegen/pipeline.ts`, `custom-component-emitter.ts`
- Modify: `editor/client/src/App.tsx` (saveComponentDefinitionsToServer, saveProject)

- [ ] **Step 1: Extend `generateOmniSource` signature**

Add optional `componentMeta` parameter to the options:

```typescript
// In pipeline.ts, add to generateOmniSource options:
componentMeta?: {
  id?: string;
  category?: string;
  description?: string;
  props?: Record<string, unknown>;
  events?: Record<string, unknown>;
  styleProperties?: string[];
  children?: boolean;
}
```

- [ ] **Step 2: Pass componentMeta through to SourceEmitter**

When `componentMeta` is provided, SourceEmitter generates `prop`/`event` DSL declarations + `// @component` metadata comment. When absent (main canvas), behavior is unchanged.

- [ ] **Step 3: Fix saveComponentDefinitionsToServer to pass wires**

In `App.tsx:66-94`:

```typescript
// BEFORE (broken):
components.push({
  name,
  elements: def.elements || [],
  // wires NOT passed!
  meta: { ... },
});

// AFTER (fixed):
components.push({
  name,
  elements: def.elements || [],
  wires: def.wires || [],          // ✅ pass wires
  codeTabs: extractCodeTabsForComponent(name), // ✅ pass associated code tabs
  componentMeta: {                 // ✅ pass full component metadata
    id: def.id,
    props: def.props,
    events: def.events,
    styleProperties: def.styleProperties,
    category: def.category,
    description: def.description,
    children: def.children,
  },
});
```

- [ ] **Step 4: Update save flow — custom components use generateOmniSource**

Replace the `emitCustomComponents` call with `generateOmniSource`:

```typescript
// BEFORE:
const result = emitCustomComponents(components);

// AFTER:
const files: Record<string, string> = {};
for (const comp of components) {
  const result = generateOmniSource({
    designElements: comp.elements,
    designWires: comp.wires || [],
    codeTabs: comp.codeTabs || [],
    componentName: comp.name,
    rootOrder: comp.elements.filter((e) => !e.parentId).map((e) => e.id),
    forSave: true,
    componentMeta: comp.componentMeta,
  });
  if (result.value) {
    files[`src/components/${comp.name}.omni`] = result.value;
  }
}
```

- [ ] **Step 5: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 6: Bump test expectations**

The integration test `integration-generate.test.ts` may need updated fixtures to include the new `componentMeta` field on EmitInput. Update accordingly.

- [ ] **Step 7: Commit**

```bash
git add editor/client/src/services/codegen/
git add editor/client/src/App.tsx
git add editor/client/src/services/codegen/__tests__/
git commit -m "fix(services): merge emitCustomComponents into generateOmniSource — fix wires loss, add chain support for custom components"
```

---

### Task 2.4: Rename @code-tab → @omni-file

**Files:**
- Modify: `services/codegen/source-emitter.ts`, `annotation-data.ts`
- Modify: `services/codegen/omni-parser.ts`
- Modify: `editor/server/annotation/parser.py`
- Modify: `editor/server/annotation/generator.py`

- [ ] **Step 1: Update source-emitter.ts**

```typescript
// BEFORE:
w.emit(`// @code-tab type=logic`);
w.emit(`// @code-tab type=event label=${fn.name}`);

// AFTER:
w.emit(`// @omni-file type=logic`);
w.emit(`// @omni-file type=event label=${fn.name}`);
```

- [ ] **Step 2: Update annotation-data.ts**

```typescript
// BEFORE:
codeTabDirective: `// @code-tab type=${tabType}`,

// AFTER:
codeTabDirective: `// @omni-file type=${tabType}`,
```

- [ ] **Step 3: Update omni-parser.ts — accept both old and new format**

```typescript
// Accept both for backward compat:
if (trimmed.startsWith("// @omni-file type=") || trimmed.startsWith("// @code-tab type=")) {
  const m = trimmed.match(/@(?:omni-file|code-tab)\s+type=(\w+)/);
  // ... rest unchanged
}
```

- [ ] **Step 4: Update annotation/parser.py**

```python
# BEFORE:
_CODE_TAB_RE = re.compile(r"^//\s*@code-tab\s+type=(\w+)\s*$")

# AFTER (accept both):
_OMNI_FILE_RE = re.compile(r"^//\s*@(?:omni-file|code-tab)\s+type=(\w+)\s*$")
```

Update all references from `_CODE_TAB_RE` to `_OMNI_FILE_RE` in parser.py.

- [ ] **Step 5: Update annotation/generator.py**

Replace `@code-tab` with `@omni-file` in generated annotation output.

- [ ] **Step 6: Update annotation/types.py**

Rename `code_tab_type`/`code_tab_line` to `omni_file_type`/`omni_file_line` in AnnotationFile.

- [ ] **Step 7: Run all tests**

```bash
cd editor/client && npx vitest run
cd editor && python -m pytest tests/ -x
```

- [ ] **Step 8: Commit**

```bash
git add editor/client/src/services/codegen/
git add editor/server/annotation/
git commit -m "refactor: rename @code-tab to @omni-file — backward-compat with old format"
```

---

### Task 2.5: Split api/client.ts → domain modules

**Files:**
- Create: `editor/client/src/services/api/compile-api.ts`, `project-api.ts`, `annotation-api.ts`, `index.ts`
- Modify: `editor/client/src/services/api/client.ts` (keep base HTTP/WS, ~100 lines)
- Modify: All files importing from `api/client` → update to use re-exports from `services/api/`

- [ ] **Step 1: Create `services/api/compile-api.ts`**

Extract compile-related methods from `api/client.ts` (lines 14-21, 98-107):

```typescript
// services/api/compile-api.ts
import type { CompileResponse } from "../../core/types";

export interface CompileApiMethods {
  compile(source: string, target?: string, previousFiles?: Record<string, string>, debug?: boolean): Promise<CompileResponse>;
  getRunStatus(): Promise<{ running: boolean; url: string | null }>;
}

export function createCompileApi(post: <T>(url: string, options?: { body?: BodyInit | null }) => Promise<T>, fetch: typeof globalThis.fetch): CompileApiMethods {
  return {
    async compile(source, target = "react", previousFiles?, debug = false) {
      const res = await fetch("/api/compile", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source, target, previousFiles, debug }),
      });
      return res.json();
    },
    async getRunStatus() {
      const res = await fetch("/api/run/status");
      return res.json();
    },
  };
}
```

- [ ] **Step 2: Create `services/api/project-api.ts`**

Extract project/files/welcome methods (lines 23-49, 50-68, 69-96, 248-270):

```typescript
// services/api/project-api.ts
export interface ProjectApiMethods {
  listDir(dirPath?: string): Promise<{ entries: Array<{ name: string; path: string; type: string; ext?: string }>; error?: string }>;
  readFile(filePath: string): Promise<{ content: string; error?: string }>;
  writeFile(filePath: string, content: string): Promise<{ ok: boolean; error?: string }>;
  getProject(): Promise<{ dir: string | null; target: string; mainFile: string | null }>;
  listDirectory(path: string): Promise<{ ok: boolean; current?: string; entries?: Array<{ name: string; isDir: boolean }>; error?: string }>;
  setProjectDir(dir: string): Promise<{ ok: boolean; dir?: string; error?: string }>;
  createProject(dir: string, name: string): Promise<{ ok: boolean; dir?: string; error?: string }>;
}

export function createProjectApi(post: <T>(url: string, options?: { body?: BodyInit | null }) => Promise<T>, fetch: typeof globalThis.fetch): ProjectApiMethods {
  return {
    async listDir(dirPath = ".") {
      const res = await fetch("/api/fs/list", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ dirPath }) });
      return res.json();
    },
    async readFile(filePath) {
      const res = await fetch("/api/fs/read", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ filePath }) });
      return res.json();
    },
    async writeFile(filePath, content) {
      const res = await fetch("/api/fs/write", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ filePath, content }) });
      return res.json();
    },
    async getProject() {
      const res = await fetch("/api/project");
      return res.json();
    },
    async listDirectory(path) { return post("/api/fs/list-directory", { body: JSON.stringify({ path }) }); },
    async setProjectDir(dir) { return post("/api/project/set-dir", { body: JSON.stringify({ dir }) }); },
    async createProject(dir, name) { return post("/api/project/create", { body: JSON.stringify({ dir, name }) }); },
  };
}
```

- [ ] **Step 3: Create `services/api/annotation-api.ts`**

Extract annotation/editor-toml/component-definitions methods (lines 69-96, 223-244):

```typescript
// services/api/annotation-api.ts
export interface AnnotationApiMethods {
  saveComponentDefinitions(definitions: Record<string, unknown>): Promise<{ ok: boolean; error?: string }>;
  loadComponentDefinitions(): Promise<{ definitions: Record<string, unknown>; error?: string }>;
  deleteComponentDefinitions(names: string[]): Promise<{ ok: boolean; error?: string }>;
  annotatedSave(filePath: string, annotations: unknown, source: string): Promise<{ ok: boolean; error?: string }>;
  annotatedLoad(filePath: string): Promise<{ success: boolean; source?: string; annotations?: unknown; error?: string }>;
  saveEditorToml(state: unknown): Promise<{ ok: boolean; error?: string }>;
  loadEditorToml(): Promise<{ state: unknown; error?: string }>;
}

export function createAnnotationApi(post: <T>(url: string, options?: { body?: BodyInit | null }) => Promise<T>, fetch: typeof globalThis.fetch): AnnotationApiMethods {
  return {
    async saveComponentDefinitions(definitions) { return post("/api/component-definitions/save", { body: JSON.stringify({ definitions }) }); },
    async loadComponentDefinitions() { const res = await fetch("/api/component-definitions/load"); return res.json(); },
    async deleteComponentDefinitions(names) { return post("/api/component-definitions/delete", { body: JSON.stringify({ names }) }); },
    async annotatedSave(filePath, annotations, source) { return post("/api/annotated-save", { body: JSON.stringify({ filePath, annotations, source }) }); },
    async annotatedLoad(filePath) { return post("/api/annotated-load", { body: JSON.stringify({ filePath }) }); },
    async saveEditorToml(state) { return post("/api/editor-toml/save", { body: JSON.stringify({ state }) }); },
    async loadEditorToml() { const res = await fetch("/api/editor-toml/load"); return res.json(); },
  };
}
```

- [ ] **Step 4: Create `services/api/index.ts` — composite ApiClient**

```typescript
// services/api/index.ts
import { createCompileApi } from "./compile-api";
import { createProjectApi } from "./project-api";
import { createAnnotationApi } from "./annotation-api";

class ApiClient {
  compileApi = createCompileApi(this.post.bind(this), fetch.bind(globalThis));
  projectApi = createProjectApi(this.post.bind(this), fetch.bind(globalThis));
  annotationApi = createAnnotationApi(this.post.bind(this), fetch.bind(globalThis));

  // Shared HTTP/WS infrastructure
  async post<T = Record<string, unknown>>(url: string, options?: { body?: BodyInit | null }): Promise<T> {
    const res = await fetch(url, {
      method: "POST",
      headers: options?.body != null ? { "Content-Type": "application/json" } : undefined,
      body: options?.body ?? undefined,
    });
    return res.json();
  }
  // ... WebSocket methods (connectWatch, connectDebug, disconnect, etc.) stay in client.ts
}

export const api = new ApiClient();
```

- [ ] **Step 5: Update all consumers**

Replace `api.compile(...)` → `api.compileApi.compile(...)`.
Replace `api.listDir(...)` → `api.projectApi.listDir(...)`.
Replace `api.annotatedLoad(...)` → `api.annotationApi.annotatedLoad(...)`.
Etc.

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 6: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 7: Commit**

```bash
git add editor/client/src/services/api/
git add -u
git commit -m "refactor(services): split api/client into domain modules — compile, project, annotation"
```

---

### Task 2.6: Extract compiler service

**Files:**
- Create: `editor/client/src/services/compiler/compiler.ts`, `index.ts`
- Move: `editor/client/src/omni-compiler-cache.ts` → `editor/client/src/services/compiler/cache.ts`
- Modify: `editor/client/src/App.tsx`, `editor/client/src/canvas/LayoutView.tsx`, `editor/client/src/panels/PropertyPanel.tsx`

- [ ] **Step 1: Create `services/compiler/compiler.ts`**

```typescript
// services/compiler/compiler.ts
import { api } from "../api";
import type { ElementData, WireData, CodeTab, CompileError } from "../../core/types";
import { generateOmniSource } from "../codegen/pipeline";
import { useElementStore } from "../../core/state/elements";
import { useWireStore } from "../../core/state/wires";
import { useCodeGraphStore } from "../../core/state/codegraph";
import { useProjectStore } from "../../core/state/project";
import { extractComponentName } from "../../core/utils";

export interface CompileOptions {
  elements: ElementData[];
  wires: WireData[];
  codeTabs: CodeTab[];
  target: string;
  componentName?: string;
  rootOrder: string[];
}

export interface CompileResult {
  errors: CompileError[];
  generatedSource: string | null;
}

export async function compileOmni(options: CompileOptions): Promise<CompileResult> {
  const componentName = options.componentName
    || extractComponentName(useProjectStore.getState().activeFileContent || "");

  const genResult = generateOmniSource({
    designElements: options.elements,
    designWires: options.wires,
    codeTabs: options.codeTabs,
    componentName,
    rootOrder: options.rootOrder,
  });

  const content = genResult.value || useProjectStore.getState().activeFileContent;
  if (!content) return { errors: [], generatedSource: null };

  const result = await api.compileApi.compile(content, options.target);
  return { errors: result.errors || [], generatedSource: content };
}
```

- [ ] **Step 2: Move omni-compiler-cache.ts**

```bash
mv editor/client/src/omni-compiler-cache.ts editor/client/src/services/compiler/cache.ts
```

Update internal imports in cache.ts. Update all consumers that import from `../omni-compiler-cache` → `../services/compiler/cache`.

- [ ] **Step 3: Unify compilerCache parameter construction**

`LayoutView.tsx:55-88` (`recompile`), `PropertyPanel.tsx:281`, and `App.tsx:561` each construct `compileProject` parameters. After this refactor, all three should use `compileOmni()` from the service, eliminating the duplicated parameter construction.

- [ ] **Step 4: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/services/compiler/
git add -u
git commit -m "refactor(services): extract compiler service, unify compilerCache calls"
```

---

### Task 2.7: Extract project service

**Files:**
- Create: `editor/client/src/services/project/project-service.ts`, `component-service.ts`, `index.ts`
- Modify: `editor/client/src/App.tsx`

- [ ] **Step 1: Create `services/project/project-service.ts`**

1:1 extract `saveProject` and `saveFileEntry` from App.tsx (lines 672-832). Rename to `saveProjectFiles()` and `saveSingleFile()`. All logic unchanged; only parameterize the `setStatus` callback.

- [ ] **Step 2: Create `services/project/component-service.ts`**

1:1 extract `saveComponentDefinitionsToServer` and `loadComponentDefinitionsFromServer` from App.tsx (lines 66-156). Update to use `generateOmniSource` (not `emitCustomComponents`) as per Task 2.3.

- [ ] **Step 3: Update App.tsx**

Replace inline `saveProject` with:
```typescript
import { saveProjectFiles } from "../services/project";
// ...in component:
const handleSave = useCallback(async (customPath?: string) => {
  await saveProjectFiles({ customPath, setStatus, api: api.projectApi });
}, [setStatus]);
```

- [ ] **Step 4: Run tests**

```bash
cd editor/client && npx vitest run
```

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/services/project/
git add editor/client/src/App.tsx
git commit -m "refactor(services): extract project service from App.tsx"
```

---

### Task 2.8: Extract runtime service

**Files:**
- Create: `editor/client/src/services/runtime/run-service.ts`, `debug-service.ts`, `index.ts`
- Modify: `editor/client/src/App.tsx`

- [ ] **Step 1: Create `services/runtime/run-service.ts`**

1:1 extract `handleRun` and `handleStop` from App.tsx (lines 572-667). Wraps `api.compileApi.compile()`, `api.post("/api/run/start")`, `api.post("/api/run/stop")`.

- [ ] **Step 2: Create `services/runtime/debug-service.ts`**

1:1 extract `handleDebugMessage` and debug WebSocket connection logic from App.tsx (lines 1073-1186).

- [ ] **Step 3: Update App.tsx** — replace inline handlers with service calls.

- [ ] **Step 4: Run tests + commit**

```bash
cd editor/client && npx vitest run
git add editor/client/src/services/runtime/ editor/client/src/App.tsx
git commit -m "refactor(services): extract runtime + debug services from App.tsx"
```

---

### Task 2.9: Extract parser service

**Files:**
- Create: `editor/client/src/services/parser/source-parser.ts`, `index.ts`
- Modify: `editor/client/src/App.tsx`
- Move: `editor/client/src/search/SearchIndex.ts` → `editor/client/src/services/parser/search-index.ts`

- [ ] **Step 1: Create `services/parser/source-parser.ts`**

Extract the parse+validate+populate logic from App.tsx (the useEffect at line 1146 that calls `validate()` then `parseOmniSource()`, and the code block parsing in `handleCompile` lines 469-543). Provide as `parseOmniFile(source: string): ParseOmniFileResult`.

- [ ] **Step 2: Move SearchIndex.ts** — `editor/client/src/search/SearchIndex.ts` + `indexLoader.ts` → `services/parser/`. Update imports in `SearchModal.tsx`.

- [ ] **Step 3: Run tests + commit**

---

### Task 2.10: Split Python backend

**Files:**
- Create: `editor/server/routers/compile.py`, `files.py`, `project.py`, `annotations.py`, `components.py`, `connectors.py`, `deploy.py`, `run.py`, `debug.py`
- Create: `editor/server/services/compiler.py`, `project_manager.py`
- Create: `editor/server/models/requests.py`
- Modify: `editor/server/main.py`

- [ ] **Step 1: Create `server/models/requests.py`**

Move all Pydantic models from main.py into a single file: `CompileRequest`, `FsReadRequest`, `FsListRequest`, `FsWriteRequest`, `ListDirectoryRequest`, `SetProjectDirRequest`, `CreateProjectRequest`, `CodeOwnershipRequest`, `ComponentDefinitionsPayload`, `ComponentDeletePayload`, `AnnotatedSaveRequest`, `AnnotatedLoadRequest`, `EditorTomlSaveRequest`, `ConnectorRequest`.

```python
# server/models/requests.py
from pydantic import BaseModel

class CompileRequest(BaseModel):
    source: str
    target: str = "react"
    previousFiles: dict[str, str] | None = None
    debug: bool = False

class FsReadRequest(BaseModel):
    filePath: str

class FsListRequest(BaseModel):
    dirPath: str = "."

class FsWriteRequest(BaseModel):
    filePath: str
    content: str

# ... (all other models, 1:1 copy from main.py)
```

- [ ] **Step 2: Create router files**

Each router file contains routes for one domain. 1:1 extraction from main.py:

| Router file | Routes |
|-------------|--------|
| `routers/compile.py` | `/api/compile`, `/api/parse-view`, `/api/parse-codeblocks`, `/api/wasm/version`, `/api/code-ownership`, `/api/parse-native-ports` |
| `routers/files.py` | `/api/fs/list`, `/api/fs/read`, `/api/fs/write`, `/api/fs/list-directory` |
| `routers/project.py` | `/api/project`, `/api/project/set-dir`, `/api/project/create` |
| `routers/annotations.py` | `/api/annotated-save`, `/api/annotated-load`, `/api/editor-toml/save`, `/api/editor-toml/load` |
| `routers/components.py` | `/api/component-definitions/save`, `/api/component-definitions/load`, `/api/component-definitions/delete` |
| `routers/connectors.py` | `/api/connector/generate` |
| `routers/deploy.py` | `/api/deploy` |
| `routers/run.py` | `/api/run/start`, `/api/run/stop`, `/api/run/status`, `/api/run/write-files` |
| `routers/debug.py` | `/api/debug/ws` |

Each router is a FastAPI `APIRouter`:

```python
# server/routers/compile.py
from fastapi import APIRouter, Request
from editor.server.models.requests import CompileRequest

router = APIRouter()

@router.post("/api/compile")
async def api_compile(req: CompileRequest, request: Request):
    from omnilang.api import compile_omni
    # ... (1:1 copy from main.py)
```

- [ ] **Step 3: Create `server/services/compiler.py` and `project_manager.py`**

Extract business logic from route handlers:
- `services/compiler.py`: compilation orchestration (wasm version check, parse helpers `_serialize_tree`)
- `services/project_manager.py`: path normalization `_normalize_path`, project file scanning

- [ ] **Step 4: Update `server/main.py`** — slim to ~80 lines

```python
# server/main.py (~80 lines)
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

app = FastAPI(title="OmniLang Editor")
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

# Register routers
from editor.server.routers.compile import router as compile_router
from editor.server.routers.files import router as files_router
from editor.server.routers.project import router as project_router
from editor.server.routers.annotations import router as annotations_router
from editor.server.routers.components import router as components_router
from editor.server.routers.connectors import router as connectors_router
from editor.server.routers.deploy import router as deploy_router
from editor.server.routers.run import router as run_router
from editor.server.routers.debug import router as debug_router

app.include_router(compile_router)
app.include_router(files_router)
app.include_router(project_router)
app.include_router(annotations_router)
app.include_router(components_router)
app.include_router(connectors_router)
app.include_router(deploy_router)
app.include_router(run_router)
app.include_router(debug_router)

# ... mount static files, cmd_serve entry point
```

- [ ] **Step 5: Run Python tests**

```bash
cd editor && python -m pytest tests/ -x
```

- [ ] **Step 6: Commit**

```bash
git add editor/server/
git commit -m "refactor(server): split main.py into routers + services + models"
```

### Task 2.11: Phase 2 final verification

- [ ] **Step 1: Full build**

```bash
cd editor/client && npm run build
```

- [ ] **Step 2: All tests**

```bash
cd editor/client && npx vitest run
cd editor && python -m pytest tests/ -x
```

- [ ] **Step 3: E2E smoke test**

Start editor, verify: create custom component with internal wires + event bindings, save, reload — wires and chains should survive.

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: Phase 2 complete — services layer with unified codegen pipeline"
```

---

## Phase 2.5: E2E Safety Net

### Task 2.5.1-2.5.7: Create E2E critical path tests

**Files:**
- Create: `editor/e2e/critical-path.spec.ts`

- [ ] **Step 1: Create E2E spec with 6 scenarios**

```typescript
// editor/e2e/critical-path.spec.ts
import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";

test.describe("Critical Path E2E", () => {

  test("add component via OmniDriver and verify store state", async ({ page }) => {
    await page.goto(EDITOR_URL);
    // Wait for OmniDriver to initialize
    await page.waitForFunction(() => !!(window as any).__OMNI__);
    // Add a Button via OmniDriver
    await page.evaluate(() => {
      const omni = (window as any).__OMNI__;
      omni.button("测试按钮");
    });
    // Verify element exists in store
    const count = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      return store.getState().elements.length;
    });
    expect(count).toBeGreaterThanOrEqual(1);
    // Verify element has correct default props (Chinese label)
    const label = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const btn = store.getState().elements.find((e: any) => e.tag === "Button");
      return btn?.props?.label;
    });
    expect(label).toBe("按钮");
  });

  test("modify property updates element in store", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => !!(window as any).__OMNI__);
    // Add a Text element
    await page.evaluate(() => {
      (window as any).__OMNI__.text("Hello");
    });
    // Modify label via store
    await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const el = store.getState().elements.find((e: any) => e.tag === "Text");
      if (el) store.getState().setProp(el.id, "label", "已更新");
    });
    // Verify prop updated
    const label = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const el = store.getState().elements.find((e: any) => e.tag === "Text");
      return el?.props?.label;
    });
    expect(label).toBe("已更新");
  });

  test("add wire and verify store", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => !!(window as any).__OMNI__);
    // Add elements + wire via store directly
    await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      store.getState().addElement({
        id: "test-btn-1", nodeType: "view", tag: "Button",
        props: { label: "Btn" }, children: [], parentId: null,
      });
      store.getState().addElement({
        id: "test-txt-1", nodeType: "view", tag: "Text",
        props: { label: "Txt" }, children: [], parentId: null,
      });
      store.getState().addWire({
        id: "test-wire-1", fromId: "test-btn-1", fromPort: "out",
        toId: "test-txt-1", toPort: "in",
      });
    });
    // Verify wire exists
    const wireCount = await page.evaluate(() => {
      return (window as any).__ZUSTAND_EDITOR_STORE__.getState().wires.length;
    });
    expect(wireCount).toBeGreaterThanOrEqual(1);
  });

  test("switch mode via store preserves element count", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => !!(window as any).__OMNI__);
    // Add elements in design mode
    await page.evaluate(() => {
      (window as any).__OMNI__.button("ModeTest");
    });
    const beforeCount = await page.evaluate(() =>
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().elements.length
    );
    // Switch to code mode via store
    await page.evaluate(() => {
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().setActiveMode("code");
    });
    await page.waitForTimeout(300);
    // Switch back to design
    await page.evaluate(() => {
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().setActiveMode("design");
    });
    // Verify elements preserved
    const afterCount = await page.evaluate(() =>
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().elements.length
    );
    expect(afterCount).toBe(beforeCount);
  });

  test("save marks project clean", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => !!(window as any).__OMNI__);
    // Add an element to make dirty
    await page.evaluate(() => {
      (window as any).__OMNI__.text("SaveTest");
    });
    // Verify dirty flag
    const isDirtyBefore = await page.evaluate(() =>
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().isDirty
    );
    expect(isDirtyBefore).toBe(true);
    // Trigger save via keyboard
    await page.keyboard.press("Control+s");
    await page.waitForTimeout(500);
    // Verify clean after save
    const isDirtyAfter = await page.evaluate(() =>
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().isDirty
    );
    expect(isDirtyAfter).toBe(false);
  });

  test("reorder elements updates rootOrder", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => !!(window as any).__OMNI__);
    // Add two elements
    const [id1, id2] = await page.evaluate(() => {
      const store = (window as any).__ZUSTAND_EDITOR_STORE__;
      const e1 = { id: "reorder-a", nodeType: "view" as const, tag: "Button", props: { label: "A" }, children: [], parentId: null as string | null };
      const e2 = { id: "reorder-b", nodeType: "view" as const, tag: "Text", props: { label: "B" }, children: [], parentId: null as string | null };
      store.getState().addElement(e1);
      store.getState().addElement(e2);
      return [e1.id, e2.id];
    });
    const orderBefore = await page.evaluate(() =>
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().rootOrder
    );
    // Reorder: move first after second
    await page.evaluate(([a, b]) => {
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().reorderElement(a, b, "after");
    }, [id1, id2]);
    const orderAfter = await page.evaluate(() =>
      (window as any).__ZUSTAND_EDITOR_STORE__.getState().rootOrder
    );
    expect(orderAfter).not.toEqual(orderBefore);
  });
});
```

- [ ] **Step 2: Run E2E tests**

```bash
cd editor/e2e && npx playwright test critical-path.spec.ts
```

- [ ] **Step 3: Commit**

```bash
git add editor/e2e/critical-path.spec.ts
git commit -m "test(e2e): add critical path safety net — 6 scenarios covering save/load/edit flows"
```

---

## Phase 3: UI Reorganization + Features Assembly

### Task 3.1: Create ui/shell/ — extract shared Toolbar

**Files:**
- Create: `editor/client/src/ui/shell/Toolbar.tsx`, `DesignToolbar.tsx`, `CodeToolbar.tsx`, `StatusBar.tsx`, `theme.ts`, `index.ts`
- Modify: `editor/client/src/App.tsx`

- [ ] **Step 1: Extract `theme.ts`**

Move the `S` styles object (App.tsx lines 45-55) and `btnStyle`/`iconBtn` (lines 1640-1641) to `ui/shell/theme.ts`:

```typescript
// ui/shell/theme.ts
import type React from "react";

export const S = {
  app: { display: "flex", height: "100vh", fontFamily: "system-ui, sans-serif" } as React.CSSProperties,
  sidebar: { width: 220, minWidth: 220, flexShrink: 0, borderRight: "1px solid #e5e7eb", overflow: "auto", background: "#f9fafb", display: "flex", flexDirection: "column" } as React.CSSProperties,
  main: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column", position: "relative", zIndex: 1 } as React.CSSProperties,
  toolbar: { height: 36, minHeight: 36, borderBottom: "1px solid #e5e7eb", padding: "0 12px", display: "flex", alignItems: "center", gap: 8, fontSize: 12, position: "relative", zIndex: 5 } as React.CSSProperties,
  canvasArea: { flex: 1, overflow: "auto", position: "relative" } as React.CSSProperties,
  codeArea: { flex: 1, overflow: "hidden", display: "flex" } as React.CSSProperties,
  inspector: { width: 280, minWidth: 280, flexShrink: 0, borderLeft: "1px solid #e5e7eb", display: "flex", flexDirection: "column", background: "#f9fafb" } as React.CSSProperties,
  statusBar: { height: 24, minHeight: 24, borderTop: "1px solid #e5e7eb", padding: "0 12px", display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 11, background: "#f9fafb" } as React.CSSProperties,
  modeBtn: (active: boolean) => ({ padding: "3px 14px", fontSize: 12, border: "none", borderRadius: 4, cursor: "pointer", fontWeight: 600, background: active ? "#4F46E5" : "#F3F4F6", color: active ? "#fff" : "#374151" } as React.CSSProperties),
};

export const btnStyle: React.CSSProperties = { padding: "4px 12px", fontSize: 12, border: "1px solid #d1d5db", borderRadius: 4, background: "#fff", cursor: "pointer" };
export const iconBtn: React.CSSProperties = { padding: "2px 6px", fontSize: 13, border: "1px solid transparent", borderRadius: 4, background: "transparent", cursor: "pointer", lineHeight: 1 };
```

- [ ] **Step 2: Create `Toolbar.tsx`** — shared buttons present in both modes

```typescript
// ui/shell/Toolbar.tsx
import React from "react";
import { S, btnStyle, iconBtn } from "./theme";

interface ToolbarProps {
  mode: "design" | "code";
  isDirty: boolean;
  projectDir: string | null;
  runStatus: { running: boolean; url: string | null };
  runLoading: boolean;
  onModeChange: (mode: "design" | "code") => void;
  onUndo: () => void;
  onRedo: () => void;
  onSave: () => void;
  onRun: () => void;
  onStop: () => void;
  onCompile: () => void;
  children?: React.ReactNode;
}

export function Toolbar({ mode, isDirty, projectDir, runStatus, runLoading, onModeChange, onUndo, onRedo, onSave, onRun, onStop, onCompile, children }: ToolbarProps) {
  return (
    <div style={S.toolbar}>
      <button onClick={() => onModeChange("design")} style={S.modeBtn(mode === "design")}>🎨 设计</button>
      <button onClick={() => onModeChange("code")} style={S.modeBtn(mode === "code")}>💻 代码</button>
      <span style={{ width: 8 }} />
      <button onClick={onUndo} style={iconBtn} title="撤销 (Ctrl+Z)">↩</button>
      <button onClick={onRedo} style={iconBtn} title="重做 (Ctrl+Shift+Z)">↪</button>
      <button onClick={onSave} style={{...iconBtn, color: isDirty ? "#DC2626" : "#6B7280", fontWeight: isDirty ? 700 : 400}} title="保存项目 (Ctrl+S)">💾</button>
      {projectDir && <span style={{ color: "#9ca3af", fontSize: 11 }}>{projectDir.split(/[/\\]/).slice(-2).join("/")}</span>}
      <span style={{ flex: 1 }} />
      {children}
      {runStatus.running ? (
        <>
          <a href={runStatus.url || "#"} target="_blank" rel="noreferrer" style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", textDecoration: "none" }}>🔗 {runStatus.url?.replace("http://", "") || "运行"}</a>
          <button onClick={onStop} style={{...btnStyle, background: "#FEF2F2", borderColor: "#FCA5A5", color: "#DC2626" }}>⏹ 停止</button>
        </>
      ) : (
        <button onClick={onRun} disabled={runLoading} style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", fontWeight: 600 }}>{runLoading ? "⏳ 启动中..." : "▶ 运行"}</button>
      )}
      <button onClick={onCompile} style={btnStyle}>编译 (Ctrl+Enter)</button>
    </div>
  );
}
```

- [ ] **Step 3: Create `DesignToolbar.tsx`** — design-mode-specific buttons

1:1 extract the design-mode-specific toolbar buttons from App.tsx (viewport selector, state preview, token/wire/HUD/focus toggles). Takes same props as Toolbar plus design-specific ones.

- [ ] **Step 4: Create `CodeToolbar.tsx`** — code-mode-specific buttons

1:1 extract the code-mode-specific buttons from App.tsx (target selector, template selector, connector toggle, auto-layout).

- [ ] **Step 5: Create `StatusBar.tsx`**

1:1 extract the status bar JSX from App.tsx (lines 1446-1449, 1550-1555).

- [ ] **Step 6: Update App.tsx** — use new Toolbar components

Replace the duplicated toolbar JSX (both design and code mode sections) with:
```jsx
<Toolbar mode={mode} isDirty={isDirty} ...>
  {mode === "design" ? <DesignToolbar ... /> : <CodeToolbar ... />}
</Toolbar>
```

- [ ] **Step 7: Run E2E + commit**

```bash
cd editor/e2e && npx playwright test critical-path.spec.ts
git add editor/client/src/ui/shell/ editor/client/src/App.tsx
git commit -m "refactor(ui): extract shared Toolbar, DesignToolbar, CodeToolbar, StatusBar to ui/shell/"
```

---

### Task 3.2: Reorganize canvas/ → design/ + code/ + shared/

**Files:**
- Move: canvas files into `ui/canvas/design/`, `ui/canvas/code/`, `ui/canvas/shared/`
- Modify: All consumer import paths

- [ ] **Step 1: Move design canvas files**

```bash
cd editor/client/src
mkdir -p ui/canvas/design ui/canvas/code ui/canvas/shared
git mv canvas/LayoutView.tsx ui/canvas/design/
git mv canvas/CanvasPreview.tsx ui/canvas/design/
git mv canvas/CanvasTabBar.tsx ui/canvas/design/
git mv canvas/ComponentCanvasHeader.tsx ui/canvas/design/
git mv canvas/NodePalette.tsx ui/canvas/design/
```

- [ ] **Step 2: Move code canvas files**

```bash
git mv canvas/FlowCanvas.tsx ui/canvas/code/
git mv canvas/CodeTabBar.tsx ui/canvas/code/
git mv canvas/nodes/BaseNode.tsx ui/canvas/code/
git mv canvas/nodes/CodeBlockNode.tsx ui/canvas/code/
git mv canvas/nodes/index.ts ui/canvas/code/
```

- [ ] **Step 3: Move shared canvas files**

```bash
git mv canvas/FlowEngine.ts ui/canvas/shared/
git mv canvas/connectionValidators.ts ui/canvas/shared/
git mv canvas/layoutUtils.ts ui/canvas/shared/
git mv canvas/elementsToNodes.ts ui/canvas/shared/
git mv canvas/wiresToEdges.ts ui/canvas/shared/
git mv canvas/socketTypes.ts ui/canvas/shared/
git mv canvas/edges/DataEdge.tsx ui/canvas/shared/
git mv canvas/edges/index.ts ui/canvas/shared/
```

- [ ] **Step 4: Move remaining canvas files**

```bash
git mv canvas/DebugPanel.tsx ui/canvas/       # temp; moves to features/debug in Task 3.6
git mv canvas/StateHUD.tsx ui/canvas/
git mv canvas/PerfBadge.tsx ui/canvas/
git mv canvas/PerfMonitor.ts ui/canvas/
git mv canvas/VariablePanel.tsx ui/canvas/
git mv canvas/fnPortGenerator.ts ui/canvas/shared/
git mv canvas/nodeRegistry.ts ui/canvas/shared/
# Move tests
git mv canvas/__tests__/FlowEngine.test.ts ui/canvas/shared/__tests__/
git mv canvas/__tests__/connectionValidators.test.ts ui/canvas/shared/__tests__/
```

- [ ] **Step 5: Update all import paths**

Global search-replace:
- `from "../canvas/LayoutView"` → `from "../ui/canvas/design/LayoutView"`
- `from "../canvas/FlowCanvas"` → `from "../ui/canvas/code/FlowCanvas"`
- `from "../canvas/FlowEngine"` → `from "../ui/canvas/shared/FlowEngine"`
- ... (all other canvas imports)
- Internal imports within canvas files (e.g., nodes importing from sibling) update relative paths

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 6: Run E2E + commit**

```bash
cd editor/e2e && npx playwright test critical-path.spec.ts
git add editor/client/src/ui/canvas/ editor/client/src/App.tsx
git commit -m "refactor(ui): reorganize canvas into design/code/shared subdirectories"
```

---

### Task 3.3: Split PropertyPanel.tsx (HIGH RISK — 1,362 lines)

**Files:**
- Create: `editor/client/src/ui/inspector/sections/LayoutSection.tsx`, `StyleSection.tsx`, `EventSection.tsx`, `DataSection.tsx`
- Modify: `editor/client/src/ui/inspector/PropertyPanel.tsx` (slim to ~300 lines)
- Modify: `editor/client/src/App.tsx` (update import path)

- [ ] **Step 1: Read current PropertyPanel.tsx to identify section boundaries**

PropertyPanel's JSX structure (approximate):
- Lines 1-100: imports, types, helpers
- Lines 100-300: component meta editing (custom component props/events)
- Lines 300-500: layout section (position, size, hierarchy)
- Lines 500-700: style section (StyleEntry list + StylePropertyEditor)
- Lines 700-900: event section (event binding + EventCreationPanel)
- Lines 900-1100: data section (props editing by type)
- Lines 1100-1362: remaining logic

- [ ] **Step 2: Create `sections/LayoutSection.tsx`** (~180 lines)

Extract the layout-related JSX and logic: element position (x, y), dimensions (width, height, min/max), z-index, overflow, parent/child hierarchy display.

```typescript
// ui/inspector/sections/LayoutSection.tsx
import React from "react";
import type { ElementData } from "../../../core/types";
import type { CommandContext } from "../../../core/commands/types";

interface LayoutSectionProps {
  element: ElementData;
  ctx: CommandContext;
}

export function LayoutSection({ element, ctx }: LayoutSectionProps) {
  // 1:1 extract layout-related JSX from PropertyPanel
  // Position fields, size fields, hierarchy info
  return (
    <div>
      {/* ... extracted layout JSX ... */}
    </div>
  );
}
```

- [ ] **Step 3: Create `sections/StyleSection.tsx`** (~120 lines)

Extract style entry list rendering. Delegates to existing `StyleEntryEditor` and `StylePropertyEditor`.

- [ ] **Step 4: Create `sections/EventSection.tsx`** (~150 lines)

Extract event binding UI. Delegates to existing `EventCreationPanel`.

- [ ] **Step 5: Create `sections/DataSection.tsx`** (~150 lines)

Extract component props editing (the data binding / prop value editor).

- [ ] **Step 6: Slim PropertyPanel.tsx to ~300 lines**

The main PropertyPanel becomes a routing shell:
```typescript
// ui/inspector/PropertyPanel.tsx (~300 lines)
import { LayoutSection } from "./sections/LayoutSection";
import { StyleSection } from "./sections/StyleSection";
import { EventSection } from "./sections/EventSection";
import { DataSection } from "./sections/DataSection";

export function PropertyPanel({ ctx, onNavigateToCode }: PropertyPanelProps) {
  const selectedId = useSelectionStore(s => s.selectedElementIds[0]);
  const element = useElementStore(s => s.elements.find(e => e.id === selectedId));

  if (!element) return <EmptyState />;

  return (
    <div>
      <ElementHeader element={element} ctx={ctx} />
      <LayoutSection element={element} ctx={ctx} />
      <StyleSection element={element} ctx={ctx} />
      <EventSection element={element} ctx={ctx} onNavigateToCode={onNavigateToCode} />
      <DataSection element={element} ctx={ctx} />
    </div>
  );
}
```

- [ ] **Step 7: TypeScript check**

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

Fix any prop drilling issues. The section components must receive the exact same data the original inline JSX received.

- [ ] **Step 8: Run E2E**

```bash
cd editor/e2e && npx playwright test critical-path.spec.ts
```

- [ ] **Step 9: Commit**

```bash
git add editor/client/src/ui/inspector/
git commit -m "refactor(ui): split PropertyPanel into 4 sections — Layout, Style, Event, Data"
```

---

### Task 3.4-3.5: Panel migration + editor/preview/search migration

**Files:**
- Move: `panels/*.tsx` → `ui/panels/` (all 20 files)
- Move: `editor/*.tsx`, `editor/*.ts` → `ui/editor/` (4 files)
- Move: `preview/PreviewFrame.tsx` → `ui/preview/`
- Move: `search/SearchModal.tsx` → `ui/shared/`
- Move: `components/ErrorBoundary.tsx` → `ui/shared/`

- [ ] **Step 1: git mv all files to their new locations**
- [ ] **Step 2: Global import path update across all 100+ consumer files**
- [ ] **Step 3: tsc --noEmit + E2E + commit**

---

### Task 3.6: Create features/ layer — extract all hooks

**Files:**
- Create: `editor/client/src/features/design-mode/DesignModeLayout.tsx`, `useDesignKeyboard.ts`
- Create: `editor/client/src/features/code-mode/CodeModeLayout.tsx`, `useCodeKeyboard.ts`
- Create: `editor/client/src/features/project-explorer/useProjectInit.ts`, `useProjectLoader.ts`, `useProjectSaver.ts`
- Create: `editor/client/src/features/compiler/useCompile.ts`, `useRun.ts`, `useAutoCompile.ts`
- Create: `editor/client/src/features/debug/useDebugSession.ts`, `DebugOverlay.tsx`

- [ ] **Step 1: Create `useDesignKeyboard.ts`** (~80 lines)

1:1 extract keyboard shortcuts specific to design mode from App.tsx useEffect (lines 953-1059). Only the design-mode-specific shortcuts:
- Copy/Cut/Paste/Duplicate (ctrl+c/v/x/d)
- Arrow key nudge
- Select all (ctrl+a)
- Panel toggle shortcuts (ctrl+1/2/3 → design panels)
- Escape to clear selection

```typescript
// features/design-mode/useDesignKeyboard.ts
import { useEffect } from "react";
import type { CommandContext } from "../../core/commands/types";
import { copySelection, pasteFromClipboard, cutSelection, duplicateSelection } from "../../core/commands/CopyPaste";
import { useSelectionStore } from "../../core/state/selection";
import { useElementStore } from "../../core/state/elements";
import { showToast } from "../../notifications/toast";

export function useDesignKeyboard(ctx: CommandContext) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // ... 1:1 copy of design-mode keyboard logic from App.tsx ...
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [ctx]);
}
```

- [ ] **Step 2: Create `useCodeKeyboard.ts`** (~80 lines)

1:1 extract code-mode-specific shortcuts (ctrl+1/2/3 → code panels, compile shortcut).

- [ ] **Step 3: Create `DesignModeLayout.tsx`** (~120 lines)

1:1 extract the design-mode JSX block from App.tsx (lines 1453-1566) into a standalone component. Props: ctx, mode state, all panel visibility state, all toolbar handlers.

- [ ] **Step 4: Create `CodeModeLayout.tsx`** (~120 lines)

1:1 extract the code-mode JSX block from App.tsx (lines 1313-1451).

- [ ] **Step 5: Create project-explorer hooks**

`useProjectInit.ts`: 1:1 extract the initialization useEffect from App.tsx (lines 224-365).
`useProjectLoader.ts`: 1:1 extract `handleProjectReady` + `openProjectFile` (lines 235-949).
`useProjectSaver.ts`: 1:1 extract `saveProject` + `saveFileEntry` (lines 672-832).

- [ ] **Step 6: Create compiler hooks**

`useCompile.ts`: wraps `compileOmni()` from services.
`useRun.ts`: wraps run service start/stop.
`useAutoCompile.ts`: the useEffect that triggers compile on file open.

- [ ] **Step 7: Create debug hooks**

`useDebugSession.ts`: 1:1 extract debug WebSocket connection + handleDebugMessage from App.tsx (lines 1073-1186).
`DebugOverlay.tsx`: 1:1 extract DebugPanel rendering from App.tsx.

- [ ] **Step 8: Run TypeScript check + E2E**

```bash
cd editor/client && npx tsc --noEmit --pretty false
cd editor/e2e && npx playwright test critical-path.spec.ts
```

- [ ] **Step 9: Commit**

---

### Task 3.7: Slim App.tsx to ~80 lines (HIGH RISK)

**Files:**
- Rewrite: `editor/client/src/app/App.tsx`
- Move: `editor/client/src/App.tsx` → backup; new App.tsx at `app/App.tsx`

- [ ] **Step 1: Write the slim App.tsx**

```typescript
// app/App.tsx (~80 lines)
import React, { useState, useMemo } from "react";
import { AppShell } from "../ui/shell/AppShell";
import { Toolbar } from "../ui/shell/Toolbar";
import { DesignToolbar } from "../ui/shell/DesignToolbar";
import { CodeToolbar } from "../ui/shell/CodeToolbar";
import { StatusBar } from "../ui/shell/StatusBar";
import { DesignModeLayout } from "../features/design-mode/DesignModeLayout";
import { CodeModeLayout } from "../features/code-mode/CodeModeLayout";
import { useDesignKeyboard } from "../features/design-mode/useDesignKeyboard";
import { useCodeKeyboard } from "../features/code-mode/useCodeKeyboard";
import { useProjectInit } from "../features/project-explorer/useProjectInit";
import { useCompile } from "../features/compiler/useCompile";
import { useRun } from "../features/compiler/useRun";
import { useDebugSession } from "../features/debug/useDebugSession";
import { useProjectStore } from "../core/state/project";
import { useSelectionStore } from "../core/state/selection";
import type { CommandContext } from "../core/commands/types";

export default function App() {
  const [mode, setMode] = useState<"design" | "code">("design");
  const isDirty = useProjectStore(s => s.isDirty);
  const projectDir = useProjectStore(s => s.projectDir);

  const { ctx, compileErrors, runStatus, runLoading, handleCompile, handleRun, handleStop, handleSave } = useCompile();
  useProjectInit();
  useDesignKeyboard(ctx);
  useCodeKeyboard(ctx, mode, setMode);
  useDebugSession(runStatus.running);

  const toolbarProps = { mode, isDirty, projectDir, runStatus, runLoading,
    onModeChange: setMode, onCompile: handleCompile, onRun: handleRun, onStop: handleStop, onSave: handleSave,
    onUndo: () => {}, onRedo: () => {},  // wired in keyboard hooks
  };

  return (
    <AppShell>
      <Toolbar {...toolbarProps}>
        {mode === "design"
          ? <DesignToolbar /* design-specific props */ />
          : <CodeToolbar /* code-specific props */ />}
      </Toolbar>
      {mode === "design"
        ? <DesignModeLayout ctx={ctx} />
        : <CodeModeLayout ctx={ctx} />}
      <StatusBar />
    </AppShell>
  );
}
```

- [ ] **Step 2: Create `AppShell.tsx`** (~30 lines)

```typescript
// ui/shell/AppShell.tsx
import React from "react";
import { S } from "./theme";

export function AppShell({ children }: { children: React.ReactNode }) {
  return <div style={S.app}>{children}</div>;
}
```

- [ ] **Step 3: Update `main.tsx`** — change import path

```typescript
// main.tsx
import App from "./app/App";  // was: "./App"
```

- [ ] **Step 4: Run TypeScript check** — fix any missing prop drilling

```bash
cd editor/client && npx tsc --noEmit --pretty false
```

- [ ] **Step 5: Run E2E full suite**

```bash
cd editor/e2e && npx playwright test
```

- [ ] **Step 6: Visual verification**

Start editor. Manually verify all functionality from the 1,689-line App.tsx still works: design mode, code mode, mode switch, compile, run, save, load, keyboard shortcuts, undo/redo, OmniDriver API.

- [ ] **Step 7: Commit**

```bash
git add editor/client/src/app/App.tsx editor/client/src/ui/shell/AppShell.tsx editor/client/src/main.tsx
git commit -m "refactor(app): slim App.tsx from 1689 to ~80 lines — pure assembly"
```

---

### Task 3.8: Split ComponentTree.tsx

**Files:**
- Create: `editor/client/src/ui/panels/TreeNodeRenderer.tsx`, `tree-utils.ts`
- Modify: `editor/client/src/ui/panels/ComponentTree.tsx`

- [ ] **Step 1: Extract `TreeNodeView` → `TreeNodeRenderer.tsx`** (~200 lines)

1:1 extract the recursive TreeNodeView component (lines 326-497) and its interface. Update imports.

- [ ] **Step 2: Extract tree utilities → `tree-utils.ts`** (~150 lines)

Extract helper functions: `resolveHtmlTag`, `getDisplayLabel`, `buildTreeData` (useMemo logic from lines 112-139), `getDefaultProps`.

- [ ] **Step 3: Slim ComponentTree.tsx** to ~200 lines (main tree + context menu logic)

- [ ] **Step 4: TypeScript check + E2E + commit**

```bash
cd editor/client && npx tsc --noEmit --pretty false
cd editor/e2e && npx playwright test critical-path.spec.ts
git add editor/client/src/ui/panels/
git commit -m "refactor(ui): split ComponentTree into TreeNodeRenderer + tree-utils"
```

---

### Task 3.9: Phase 3 final verification

- [ ] **Step 1: Full build**

```bash
cd editor/client && npm run build
```

- [ ] **Step 2: All tests + E2E**

```bash
cd editor/client && npx vitest run
cd editor/e2e && npx playwright test
```

- [ ] **Step 3: Full visual verification**

Start editor. Verify:
- Design mode: palette drag, canvas preview, property panel, tree, save/reload
- Code mode: node palette, flow canvas, code tabs, compile
- Custom components: create, edit, add events with chains, save, reload — all survive
- Mode switch, keyboard shortcuts, undo/redo

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: Phase 3 complete — UI reorganized, App.tsx slimmed to ~80 lines"
```

---

### Task 3.10: Final cleanup verification

- [ ] **Step 1: Check for stale import paths**

```bash
cd editor/client
grep -r "from.*\.\./store/editor" src/ --include="*.ts" --include="*.tsx" | grep -v "core/state"
grep -r "from.*\.\./omni-gen" src/ --include="*.ts" --include="*.tsx"
grep -r "from.*\.\./canvas/toast" src/ --include="*.ts" --include="*.tsx"
grep -r "from.*\.\./commands/" src/ --include="*.ts" --include="*.tsx" | grep -v "core/commands"
```
Expected: no output on all four checks.

- [ ] **Step 2: Remove backup files if any**

```bash
rm -f editor/client/src/App.tsx.bak 2>/dev/null
rm -rf editor/client/src/store/editor.ts.bak 2>/dev/null
```

- [ ] **Step 3: Final TypeScript check**

```bash
cd editor/client && npx tsc --noEmit --pretty false
```
Expected: zero errors.

- [ ] **Step 4: Final production build**

```bash
cd editor/client && npm run build
```
Expected: successful build with no warnings.

- [ ] **Step 5: Final full test suite**

```bash
cd editor/client && npx vitest run
cd editor/e2e && npx playwright test
```
Expected: all tests pass (unit + E2E).

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "chore: final cleanup — all imports migrated, zero stale paths, full test suite passing"
```

---

## Risk Mitigation Checklist

- [ ] Each task commit is independently revertable
- [ ] `npx tsc --noEmit` runs clean after every task
- [ ] `npx vitest run` passes after every task
- [ ] Phase 2.5 E2E tests established before Phase 3 begins
- [ ] All E2E tests pass after Phase 3
- [ ] `npm run build` produces working output after each Phase
- [ ] Editor UI visually identical to pre-refactor state
