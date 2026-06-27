# Style Panel 全链路重设计 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 端到端重建样式编辑系统，对标 OmniLang CSS 语言模型，使编辑器能生成 ThemeCssVarsDemo / CssVarsDemo / CssUnitsDemo 级别的样式代码。

**Architecture:** 分 6 个 Phase。Phase 1 置换数据模型和命令系统（底层基础）；Phase 2 构建 fude 值编辑器（通用组件）；Phase 3 构建 CSS 样式弹窗（主 UI）；Phase 4 简化右侧面板；Phase 5 适配 codegen；Phase 6 回归验证。

**Tech Stack:** React, TypeScript, Zustand, `@tigerabrodioss/fude`, Vitest

## Global Constraints

- 平台忠实性：编辑器 UI 必须对应 OmniLang 实际编译产物
- SSOT：class 和 variable 定义存在组件级注册表，所有引用同名元素共享同一份定义
- 不可变性：所有状态更新通过 spread 操作，通过 undo 系统执行
- 增量兼容：Phase 1 完成后即可独立验证

---

## File Map

```
editor/client/src/
├── core/
│   ├── types/
│   │   ├── style.ts          [REWRITE] 新类型: StyleValueSegment, CssVariable, StyleClass, StyleProperty, StyleRegistry, ElementStyle
│   │   └── element.ts        [MODIFY]  styles?: StyleEntry[] → style?: ElementStyle
│   ├── state/
│   │   ├── styleRegistry.ts  [CREATE]  原子 store — StyleRegistry 的 canonical owner
│   │   └── styles.ts         [DELETE]  删除 styleOperations
│   └── commands/
│       ├── AddStyleEntry.ts       [DELETE]
│       ├── RemoveStyleEntry.ts    [DELETE]
│       ├── UpdateStyleEntry.ts    [DELETE]
│       ├── ReorderStyleEntry.ts   [DELETE]
│       ├── RenameStyleEntry.ts    [DELETE]
│       ├── SetStyleEntryType.ts   [DELETE]
│       ├── UpdateStyleProperty.ts [DELETE]
│       ├── AddCssVariable.ts      [CREATE]  11 个新命令
│       ├── RemoveCssVariable.ts   [CREATE]
│       ├── UpdateCssVariable.ts   [CREATE]
│       ├── AddStyleClass.ts       [CREATE]
│       ├── RemoveStyleClass.ts    [CREATE]
│       ├── RenameStyleClass.ts    [CREATE]
│       ├── AddClassProperty.ts    [CREATE]
│       ├── RemoveClassProperty.ts [CREATE]
│       ├── UpdateClassProperty.ts [CREATE]
│       ├── SetElementClassRefs.ts [CREATE]
│       ├── SetElementInlineStyles.ts [CREATE]
│       └── __tests__/
│           └── style-commands.test.ts [REWRITE]
├── store/
│   ├── editor.ts             [MODIFY] 删除旧 style 方法，新增 ElementStyle + StyleRegistry pass-through
│   └── __tests__/
│       └── style-store.test.ts [REWRITE]
├── ui/
│   ├── panels/
│   │   ├── StyleEntryEditor.tsx      [DELETE]
│   │   ├── StylePropertyEditor.tsx   [REWRITE] 值编辑器改为 StyleValueEditor
│   │   ├── StyleValueEditor.tsx      [CREATE]  fude 值编辑器封装
│   │   ├── StyleDesignModal.tsx      [CREATE]  CSS 样式弹窗容器
│   │   ├── CssVariableEditor.tsx     [CREATE]  CSS 变量区域
│   │   ├── ClassCard.tsx             [CREATE]  单个 class 卡片
│   │   ├── ClassPreview.tsx          [CREATE]  左侧预览区
│   │   └── PropertyPanel.tsx         [MODIFY]  更新 StyleSection 引用
│   ├── inspector/
│   │   └── sections/
│   │       └── StyleSection.tsx      [REWRITE] 简化为 classRefs + inlineProperties
│   └── shell/
│       └── Toolbar.tsx               [MODIFY]  新增 [🎨 CSS 样式] 按钮
├── services/
│   └── codegen/
│       ├── collect-styles.ts         [MODIFY]  适配新数据模型
│       ├── source-emitter.ts         [MODIFY]  StyleValueSegment 序列化
│       └── pipeline.ts               [MODIFY]  parseStylesFromAST 适配
└── constants/
    └── style-properties.ts           [MODIFY]  删除重复的类型定义
```


---

### Task 1: 重写类型定义 + constants 清理

**Files:**
- Rewrite: `editor/client/src/core/types/style.ts`
- Modify: `editor/client/src/core/types/element.ts`

**Interfaces:**
- Produces: `StyleValueSegment`, `CssVariable`, `StyleClass`, `StyleProperty`, `StyleRegistry`, `ElementStyle`, `variableToClassName`, `classNameToVariable`

- [ ] **Step 1: 重写 `core/types/style.ts`**

用以下内容完全替换 `editor/client/src/core/types/style.ts`：

```typescript
// core/types/style.ts
// Shared style type definitions — redesigned for OmniLang language fidelity.

// ── Value segments (fude model) ──

export type StyleValueSegment =
  | { type: "text"; value: string }
  | { type: "signal"; name: string; label: string }
  | { type: "var-ref"; name: string };

// ── CSS Variable ──

export interface CssVariable {
  id: string;
  name: string;                       // "--primary-color"
  segments: StyleValueSegment[];
}

// ── Style Class ──

export interface StyleClass {
  id: string;
  name: string;                       // "wrapper" | "card.accent" | "btn:hover"
  properties: StyleProperty[];
}

export interface StyleProperty {
  id: string;
  name: string;                       // "padding" | "--radius" | "background"
  segments: StyleValueSegment[];
}

// ── Component-level registry (SSOT) ──

export interface StyleRegistry {
  variables: CssVariable[];
  classes: StyleClass[];
}

// ── Per-element style (references only) ──

export interface ElementStyle {
  classRefs: string[];                // ["wrapper", "card"]
  inlineProperties: StyleProperty[];  // inline props (may contain signal bindings)
}

// ── Variable ↔ Class name conversion (for compiler output) ──

export function variableToClassName(varName: string): string {
  return "_v-" + varName.slice(2);
}

export function classNameToVariable(className: string): string | null {
  if (className.startsWith("_v-")) {
    return "--" + className.slice(3);
  }
  return null;
}
```

- [ ] **Step 2: 更新 `core/types/element.ts`**

替换 ElementData 的 styles 字段：

找到：
```typescript
import type { StyleEntry } from "./style";
// ...
styles?: StyleEntry[];
```

替换为：
```typescript
import type { ElementStyle } from "./style";
// ...
style?: ElementStyle;
```

- [ ] **Step 3: 运行 TypeScript 检查**

```bash
cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -50
```

预期：大量类型错误（因为所有使用旧 `styles` / `StyleEntry` 的地方现在会报错）。记录错误数量和文件列表，为后续任务提供清理清单。

- [ ] **Step 5: 清理 `constants/style-properties.ts`**

删除文件顶部的重复类型定义（`StyleProperty`, `StyleEntry`, `STYLE_PROPERTY_NAMES`, `UnitGroup`）。

替换为：
```typescript
import type { StyleProperty } from "../core/types/style";
```

同时删除 `variableToClassName` 和 `classNameToVariable`（已移至 `core/types/style.ts`）。

- [ ] **Step 6: 删除旧的 style-commands 测试文件**

```bash
rm editor/client/src/core/commands/__tests__/style-commands.test.ts
```

- [ ] **Step 7: 提交**

```bash
git add editor/client/src/core/types/style.ts editor/client/src/core/types/element.ts editor/client/src/constants/style-properties.ts
git rm editor/client/src/core/commands/__tests__/style-commands.test.ts
git commit -m "feat: rewrite style types — StyleValueSegment, StyleRegistry, ElementStyle; clean constants"
```

---

### Task 2: 创建 StyleRegistry 原子 Store

**Files:**
- Create: `editor/client/src/core/state/styleRegistry.ts`
- Modify: `editor/client/src/core/state/styles.ts` → delete

**Interfaces:**
- Consumes: `CssVariable`, `StyleClass`, `StyleProperty`, `StyleRegistry` from `core/types/style`
- Produces: `useStyleRegistryStore` (Zustand store)

- [ ] **Step 1: 创建 `core/state/styleRegistry.ts`**

```typescript
// core/state/styleRegistry.ts
// Atom store — canonical owner of component-level StyleRegistry data.

import { create } from "zustand";
import type { CssVariable, StyleClass, StyleProperty, StyleRegistry } from "../types/style";

interface StyleRegistryState {
  registries: Record<string, StyleRegistry>;

  // ── CSS Variable operations ──
  addVariable: (componentName: string, variable: CssVariable) => void;
  removeVariable: (componentName: string, variableId: string) => void;
  updateVariable: (componentName: string, variableId: string, patch: Partial<CssVariable>) => void;

  // ── Class operations ──
  addClass: (componentName: string, styleClass: StyleClass) => void;
  removeClass: (componentName: string, classId: string) => void;
  renameClass: (componentName: string, classId: string, newName: string) => void;

  // ── Property operations ──
  addProperty: (componentName: string, classId: string, property: StyleProperty) => void;
  removeProperty: (componentName: string, classId: string, propertyId: string) => void;
  updateProperty: (componentName: string, classId: string, propertyId: string, patch: Partial<StyleProperty>) => void;

  // ── Queries ──
  getRegistry: (componentName: string) => StyleRegistry;
}

export const useStyleRegistryStore = create<StyleRegistryState>((set, get) => ({
  registries: {},

  addVariable: (componentName, variable) => set((s) => {
    const reg = s.registries[componentName] ?? { variables: [], classes: [] };
    return {
      registries: {
        ...s.registries,
        [componentName]: { ...reg, variables: [...reg.variables, variable] },
      },
    };
  }),

  removeVariable: (componentName, variableId) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          variables: reg.variables.filter((v) => v.id !== variableId),
        },
      },
    };
  }),

  updateVariable: (componentName, variableId, patch) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          variables: reg.variables.map((v) =>
            v.id === variableId ? { ...v, ...patch } : v
          ),
        },
      },
    };
  }),

  addClass: (componentName, styleClass) => set((s) => {
    const reg = s.registries[componentName] ?? { variables: [], classes: [] };
    return {
      registries: {
        ...s.registries,
        [componentName]: { ...reg, classes: [...reg.classes, styleClass] },
      },
    };
  }),

  removeClass: (componentName, classId) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.filter((c) => c.id !== classId),
        },
      },
    };
  }),

  renameClass: (componentName, classId, newName) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId ? { ...c, name: newName } : c
          ),
        },
      },
    };
  }),

  addProperty: (componentName, classId, property) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId
              ? { ...c, properties: [...c.properties, property] }
              : c
          ),
        },
      },
    };
  }),

  removeProperty: (componentName, classId, propertyId) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId
              ? { ...c, properties: c.properties.filter((p) => p.id !== propertyId) }
              : c
          ),
        },
      },
    };
  }),

  updateProperty: (componentName, classId, propertyId, patch) => set((s) => {
    const reg = s.registries[componentName];
    if (!reg) return {};
    return {
      registries: {
        ...s.registries,
        [componentName]: {
          ...reg,
          classes: reg.classes.map((c) =>
            c.id === classId
              ? {
                  ...c,
                  properties: c.properties.map((p) =>
                    p.id === propertyId ? { ...p, ...patch } : p
                  ),
                }
              : c
          ),
        },
      },
    };
  }),

  getRegistry: (componentName) => {
    return get().registries[componentName] ?? { variables: [], classes: [] };
  },
}));
```

- [ ] **Step 2: 删除 `core/state/styles.ts`**

```bash
rm editor/client/src/core/state/styles.ts
```

- [ ] **Step 3: 写 store 单元测试**

创建 `editor/client/src/store/__tests__/style-registry.test.ts`：

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { useStyleRegistryStore } from "../../core/state/styleRegistry";
import { uid } from "../../core/utils/id";

const APP = "App";

describe("useStyleRegistryStore", () => {
  beforeEach(() => {
    useStyleRegistryStore.setState({ registries: {} });
  });

  it("addVariable stores a CSS variable under component key", () => {
    useStyleRegistryStore.getState().addVariable(APP, {
      id: uid("v"),
      name: "--primary",
      segments: [{ type: "text", value: "#6366f1" }],
    });
    const reg = useStyleRegistryStore.getState().getRegistry(APP);
    expect(reg.variables).toHaveLength(1);
    expect(reg.variables[0].name).toBe("--primary");
  });

  it("removeVariable deletes by id", () => {
    const id = uid("v");
    useStyleRegistryStore.getState().addVariable(APP, {
      id, name: "--x", segments: [{ type: "text", value: "1" }],
    });
    useStyleRegistryStore.getState().removeVariable(APP, id);
    const reg = useStyleRegistryStore.getState().getRegistry(APP);
    expect(reg.variables).toHaveLength(0);
  });

  it("updateVariable applies partial patch", () => {
    const id = uid("v");
    useStyleRegistryStore.getState().addVariable(APP, {
      id, name: "--old", segments: [{ type: "text", value: "1" }],
    });
    useStyleRegistryStore.getState().updateVariable(APP, id, { name: "--new" });
    const v = useStyleRegistryStore.getState().getRegistry(APP).variables[0];
    expect(v.name).toBe("--new");
  });

  it("addClass stores a class definition", () => {
    useStyleRegistryStore.getState().addClass(APP, {
      id: uid("c"), name: "card", properties: [],
    });
    const reg = useStyleRegistryStore.getState().getRegistry(APP);
    expect(reg.classes).toHaveLength(1);
    expect(reg.classes[0].name).toBe("card");
  });

  it("renameClass updates the class name", () => {
    const id = uid("c");
    useStyleRegistryStore.getState().addClass(APP, {
      id, name: "old-card", properties: [],
    });
    useStyleRegistryStore.getState().renameClass(APP, id, "new-card");
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].name).toBe("new-card");
  });

  it("addProperty appends to class", () => {
    const cid = uid("c");
    useStyleRegistryStore.getState().addClass(APP, {
      id: cid, name: "btn", properties: [],
    });
    useStyleRegistryStore.getState().addProperty(APP, cid, {
      id: uid("p"), name: "padding", segments: [{ type: "text", value: "8px" }],
    });
    const cls = useStyleRegistryStore.getState().getRegistry(APP).classes[0];
    expect(cls.properties).toHaveLength(1);
  });

  it("removeProperty deletes by property id", () => {
    const cid = uid("c");
    const pid = uid("p");
    useStyleRegistryStore.getState().addClass(APP, {
      id: cid, name: "btn", properties: [
        { id: pid, name: "color", segments: [{ type: "text", value: "red" }] },
      ],
    });
    useStyleRegistryStore.getState().removeProperty(APP, cid, pid);
    const cls = useStyleRegistryStore.getState().getRegistry(APP).classes[0];
    expect(cls.properties).toHaveLength(0);
  });

  it("getRegistry returns empty default for unknown component", () => {
    const reg = useStyleRegistryStore.getState().getRegistry("UnknownComponent");
    expect(reg.variables).toEqual([]);
    expect(reg.classes).toEqual([]);
  });
});
```

- [ ] **Step 4: 运行测试验证**

```bash
cd editor/client && npx vitest run src/store/__tests__/style-registry.test.ts
```

预期：8 个测试全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add editor/client/src/core/state/styleRegistry.ts editor/client/src/store/__tests__/style-registry.test.ts
git rm editor/client/src/core/state/styles.ts
git commit -m "feat: add useStyleRegistryStore atom store, remove styleOperations"
```

---

### Task 3: 更新 Monolith Store（editor.ts）

**Files:**
- Modify: `editor/client/src/store/editor.ts`

**Interfaces:**
- Consumes: `useStyleRegistryStore` from `core/state/styleRegistry`, `StyleProperty`, `ElementStyle` from `core/types/style`
- Produces: monolith store 的 style 相关 getter/methods

- [ ] **Step 1: 在 EditorState interface 中替换旧 style 方法**

在 `editor/client/src/store/editor.ts` 中找到：

```typescript
  // ── Style management ──
  addStyleEntry: (elementId: string, entry: StyleEntry) => void;
  removeStyleEntry: (elementId: string, entryId: string) => void;
  updateStyleEntry: (elementId: string, entryId: string, patch: Partial<StyleEntry>) => void;
  reorderStyleEntries: (elementId: string, entryIds: string[]) => void;
  getStyles: (elementId: string) => StyleEntry[];
```

替换为：

```typescript
  // ── Style management ──
  styleRegistry: StyleRegistry;

  // ── Element style (per-element references) ──
  setElementClassRefs: (elementId: string, classRefs: string[]) => void;
  addElementClassRef: (elementId: string, className: string) => void;
  removeElementClassRef: (elementId: string, className: string) => void;
  setElementInlineStyles: (elementId: string, properties: StyleProperty[]) => void;
  addElementInlineProperty: (elementId: string, property: StyleProperty) => void;
  removeElementInlineProperty: (elementId: string, propertyId: string) => void;
  updateElementInlineProperty: (elementId: string, propertyId: string, patch: Partial<StyleProperty>) => void;
```

同时更新 import（在文件顶部）：

```typescript
import type { StyleProperty, StyleRegistry } from "../core/types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";
```

删除 `import type { StyleEntry } from "../core/types";`。

在 `EditorState` interface 下方的 implementation 区域，所有 `(e as any).style` 已改为使用 `ElementData.style` 类型（Task 1 更新了 ElementData 的 style 字段）。

- [ ] **Step 2: 在 create() 函数中新增实现**

在 `return {` 块中找到现有的 style 方法（`addStyleEntry`, `removeStyleEntry`, `updateStyleEntry`, `reorderStyleEntries`, `getStyles`），全部删除。

替换为：

```typescript
    // ── Style Registry (compute active registry locally) ──
    get styleRegistry(): StyleRegistry {
      const activeDocId = get().activeDocId;
      const componentName = activeDocId?.startsWith("comp:")
        ? activeDocId.replace("comp:", "")
        : "App";
      return useStyleRegistryStore.getState().getRegistry(componentName);
    },

    // ── Element style mutations ──

    setElementClassRefs: (elementId, classRefs) => {
      set((s) => ({
        elements: s.elements.map((e) =>
          e.id === elementId
            ? { ...e, style: { ...e.style, classRefs } }
            : e
        ),
        isDirty: true,
      }));
    },

    addElementClassRef: (elementId, className) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const current = (e as any).style?.classRefs ?? [];
          return { ...e, style: { ...(e as any).style, classRefs: [...current, className] } };
        }),
        isDirty: true,
      }));
    },

    removeElementClassRef: (elementId, className) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const current = (e as any).style?.classRefs ?? [];
          return { ...e, style: { ...(e as any).style, classRefs: current.filter((c: string) => c !== className) } };
        }),
        isDirty: true,
      }));
    },

    setElementInlineStyles: (elementId, properties) => {
      set((s) => ({
        elements: s.elements.map((e) =>
          e.id === elementId
            ? { ...e, style: { ...(e as any).style, inlineProperties: properties } }
            : e
        ),
        isDirty: true,
      }));
    },

    addElementInlineProperty: (elementId, property) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const current = (e as any).style?.inlineProperties ?? [];
          return { ...e, style: { ...(e as any).style, inlineProperties: [...current, property] } };
        }),
        isDirty: true,
      }));
    },

    removeElementInlineProperty: (elementId, propertyId) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const current = (e as any).style?.inlineProperties ?? [];
          return { ...e, style: { ...(e as any).style, inlineProperties: current.filter((p: any) => p.id !== propertyId) } };
        }),
        isDirty: true,
      }));
    },

    updateElementInlineProperty: (elementId, propertyId, patch) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const current = (e as any).style?.inlineProperties ?? [];
          return {
            ...e,
            style: {
              ...(e as any).style,
              inlineProperties: current.map((p: any) =>
                p.id === propertyId ? { ...p, ...patch } : p
              ),
            },
          };
        }),
        isDirty: true,
      }));
    },
```

- [ ] **Step 3: 更新 syncToAtoms() 中的 atom 导出**

在 `syncToAtoms` 函数中，如果有 `useElementStore.setState` 调用传递了 `styles`，移除它们（elements 的 style 字段仍在 elements 上，不需要单独原子化）。

- [ ] **Step 4: 提交**

```bash
git add editor/client/src/store/editor.ts
git commit -m "refactor: replace old style methods with ElementStyle + StyleRegistry pass-through in monolith store"
```

---

### Task 4: 创建 11 个新命令

**Files:**
- Create: `editor/client/src/core/commands/AddCssVariable.ts`
- Create: `editor/client/src/core/commands/RemoveCssVariable.ts`
- Create: `editor/client/src/core/commands/UpdateCssVariable.ts`
- Create: `editor/client/src/core/commands/AddStyleClass.ts`
- Create: `editor/client/src/core/commands/RemoveStyleClass.ts`
- Create: `editor/client/src/core/commands/RenameStyleClass.ts`
- Create: `editor/client/src/core/commands/AddClassProperty.ts`
- Create: `editor/client/src/core/commands/RemoveClassProperty.ts`
- Create: `editor/client/src/core/commands/UpdateClassProperty.ts`
- Create: `editor/client/src/core/commands/SetElementClassRefs.ts`
- Create: `editor/client/src/core/commands/SetElementInlineStyles.ts`

- [ ] **Step 1: 创建 AddCssVariable.ts**

```typescript
// core/commands/AddCssVariable.ts
import type { Command, CommandContext } from "./types";
import type { CssVariable } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class AddCssVariable implements Command {
  readonly type = "AddCssVariable";
  readonly description: string;

  constructor(
    private componentName: string,
    private variable: CssVariable,
  ) {
    this.description = `添加 CSS 变量 ${this.variable.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addVariable(this.componentName, this.variable);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeVariable(this.componentName, this.variable.id);
  }
}
```

- [ ] **Step 2: 创建 RemoveCssVariable.ts**

```typescript
// core/commands/RemoveCssVariable.ts
import type { Command, CommandContext } from "./types";
import type { CssVariable } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class RemoveCssVariable implements Command {
  readonly type = "RemoveCssVariable";
  readonly description: string;

  constructor(
    private componentName: string,
    private variable: CssVariable,
  ) {
    this.description = `删除 CSS 变量 ${this.variable.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeVariable(this.componentName, this.variable.id);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addVariable(this.componentName, this.variable);
  }
}
```

- [ ] **Step 3: 创建 UpdateCssVariable.ts**

```typescript
// core/commands/UpdateCssVariable.ts
import type { Command, CommandContext } from "./types";
import type { CssVariable } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class UpdateCssVariable implements Command {
  readonly type = "UpdateCssVariable";
  readonly description: string;

  constructor(
    private componentName: string,
    private variableId: string,
    private oldPatch: Partial<CssVariable>,
    private newPatch: Partial<CssVariable>,
  ) {
    this.description = `更新 CSS 变量`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateVariable(this.componentName, this.variableId, this.newPatch);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateVariable(this.componentName, this.variableId, this.oldPatch);
  }
}
```

- [ ] **Step 4: 创建 AddStyleClass.ts**

```typescript
// core/commands/AddStyleClass.ts
import type { Command, CommandContext } from "./types";
import type { StyleClass } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class AddStyleClass implements Command {
  readonly type = "AddStyleClass";
  readonly description: string;

  constructor(
    private componentName: string,
    private styleClass: StyleClass,
  ) {
    this.description = `添加类 ${this.styleClass.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addClass(this.componentName, this.styleClass);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeClass(this.componentName, this.styleClass.id);
  }
}
```

- [ ] **Step 5: 创建 RemoveStyleClass.ts**

```typescript
// core/commands/RemoveStyleClass.ts
import type { Command, CommandContext } from "./types";
import type { StyleClass } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class RemoveStyleClass implements Command {
  readonly type = "RemoveStyleClass";
  readonly description: string;

  constructor(
    private componentName: string,
    private styleClass: StyleClass,
  ) {
    this.description = `删除类 ${this.styleClass.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeClass(this.componentName, this.styleClass.id);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addClass(this.componentName, this.styleClass);
  }
}
```

- [ ] **Step 6: 创建 RenameStyleClass.ts**

```typescript
// core/commands/RenameStyleClass.ts
import type { Command, CommandContext } from "./types";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class RenameStyleClass implements Command {
  readonly type = "RenameStyleClass";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private oldName: string,
    private newName: string,
  ) {
    this.description = `重命名类 ${oldName} → ${newName}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().renameClass(this.componentName, this.classId, this.newName);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().renameClass(this.componentName, this.classId, this.oldName);
  }
}
```

- [ ] **Step 7: 创建 AddClassProperty.ts**

```typescript
// core/commands/AddClassProperty.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class AddClassProperty implements Command {
  readonly type = "AddClassProperty";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private property: StyleProperty,
  ) {
    this.description = `添加属性 ${this.property.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addProperty(this.componentName, this.classId, this.property);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeProperty(this.componentName, this.classId, this.property.id);
  }
}
```

- [ ] **Step 8: 创建 RemoveClassProperty.ts**

```typescript
// core/commands/RemoveClassProperty.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class RemoveClassProperty implements Command {
  readonly type = "RemoveClassProperty";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private property: StyleProperty,
  ) {
    this.description = `删除属性 ${this.property.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeProperty(this.componentName, this.classId, this.property.id);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addProperty(this.componentName, this.classId, this.property);
  }
}
```

- [ ] **Step 9: 创建 UpdateClassProperty.ts**

```typescript
// core/commands/UpdateClassProperty.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useStyleRegistryStore } from "../core/state/styleRegistry";

export class UpdateClassProperty implements Command {
  readonly type = "UpdateClassProperty";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private propertyId: string,
    private oldPatch: Partial<StyleProperty>,
    private newPatch: Partial<StyleProperty>,
  ) {
    this.description = `更新属性`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateProperty(this.componentName, this.classId, this.propertyId, this.newPatch);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateProperty(this.componentName, this.classId, this.propertyId, this.oldPatch);
  }
}
```

- [ ] **Step 10: 创建 SetElementClassRefs.ts**

```typescript
// core/commands/SetElementClassRefs.ts
import type { Command, CommandContext } from "./types";
import { useEditorStore } from "../../store/editor";

export class SetElementClassRefs implements Command {
  readonly type = "SetElementClassRefs";
  readonly description = "设置元素类引用";

  constructor(
    private elementId: string,
    private oldClassRefs: string[],
    private newClassRefs: string[],
  ) {
    this.description = `设置元素类引用`;
  }

  execute(_ctx: CommandContext): void {
    useEditorStore.getState().setElementClassRefs(this.elementId, this.newClassRefs);
  }

  undo(_ctx: CommandContext): void {
    useEditorStore.getState().setElementClassRefs(this.elementId, this.oldClassRefs);
  }
}
```

- [ ] **Step 11: 创建 SetElementInlineStyles.ts**

```typescript
// core/commands/SetElementInlineStyles.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useEditorStore } from "../../store/editor";

export class SetElementInlineStyles implements Command {
  readonly type = "SetElementInlineStyles";
  readonly description = "设置元素内联样式";

  constructor(
    private elementId: string,
    private oldProperties: StyleProperty[],
    private newProperties: StyleProperty[],
  ) {
    this.description = `设置元素内联样式`;
  }

  execute(_ctx: CommandContext): void {
    useEditorStore.getState().setElementInlineStyles(this.elementId, this.newProperties);
  }

  undo(_ctx: CommandContext): void {
    useEditorStore.getState().setElementInlineStyles(this.elementId, this.oldProperties);
  }
}
```

- [ ] **Step 12: 删除 7 个旧命令文件**

```bash
rm editor/client/src/core/commands/AddStyleEntry.ts
rm editor/client/src/core/commands/RemoveStyleEntry.ts
rm editor/client/src/core/commands/UpdateStyleEntry.ts
rm editor/client/src/core/commands/ReorderStyleEntry.ts
rm editor/client/src/core/commands/RenameStyleEntry.ts
rm editor/client/src/core/commands/SetStyleEntryType.ts
rm editor/client/src/core/commands/UpdateStyleProperty.ts
```

- [ ] **Step 13: 重写命令测试**

用以下内容替换 `editor/client/src/core/commands/__tests__/style-commands.test.ts`：

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { useStyleRegistryStore } from "../../state/styleRegistry";
import { useEditorStore } from "../../../store/editor";
import { uid } from "../../utils/id";
import { AddCssVariable } from "../AddCssVariable";
import { RemoveCssVariable } from "../RemoveCssVariable";
import { UpdateCssVariable } from "../UpdateCssVariable";
import { AddStyleClass } from "../AddStyleClass";
import { RemoveStyleClass } from "../RemoveStyleClass";
import { RenameStyleClass } from "../RenameStyleClass";
import { AddClassProperty } from "../AddClassProperty";
import { RemoveClassProperty } from "../RemoveClassProperty";
import { UpdateClassProperty } from "../UpdateClassProperty";
import type { CommandContext } from "../types";

const APP = "App";
const mockCtx: CommandContext = {
  getElement: () => undefined,
  elements: [], wires: [],
  addElement: () => {}, removeElement: () => {},
  setProp: () => {}, getProp: () => undefined,
  addWire: () => {}, removeWire: () => {},
  setElementsAndWires: () => {},
  reorderElement: () => {},
  moveElement: () => {},
};

beforeEach(() => {
  useStyleRegistryStore.setState({ registries: {} });
});

describe("style commands", () => {
  it("AddCssVariable → execute adds, undo removes", () => {
    const v = { id: uid("v"), name: "--primary", segments: [{ type: "text" as const, value: "#333" }] };
    const cmd = new AddCssVariable(APP, v);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(1);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(0);
  });

  it("RemoveCssVariable → execute removes, undo adds", () => {
    const v = { id: uid("v"), name: "--x", segments: [{ type: "text" as const, value: "1" }] };
    useStyleRegistryStore.getState().addVariable(APP, v);
    const cmd = new RemoveCssVariable(APP, v);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(0);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(1);
  });

  it("UpdateCssVariable → execute updates, undo reverts", () => {
    const vid = uid("v");
    useStyleRegistryStore.getState().addVariable(APP, { id: vid, name: "--old", segments: [{ type: "text", value: "1" }] });
    const cmd = new UpdateCssVariable(APP, vid, { name: "--old" }, { name: "--new" });
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables[0].name).toBe("--new");
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables[0].name).toBe("--old");
  });

  it("AddStyleClass → execute adds, undo removes", () => {
    const c = { id: uid("c"), name: "card", properties: [] };
    const cmd = new AddStyleClass(APP, c);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(1);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(0);
  });

  it("RemoveStyleClass → execute removes, undo adds", () => {
    const c = { id: uid("c"), name: "card", properties: [] };
    useStyleRegistryStore.getState().addClass(APP, c);
    const cmd = new RemoveStyleClass(APP, c);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(0);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(1);
  });

  it("RenameStyleClass → execute renames, undo reverts", () => {
    const cid = uid("c");
    useStyleRegistryStore.getState().addClass(APP, { id: cid, name: "old", properties: [] });
    const cmd = new RenameStyleClass(APP, cid, "old", "new");
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].name).toBe("new");
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].name).toBe("old");
  });

  it("AddClassProperty → execute adds, undo removes", () => {
    const cid = uid("c");
    useStyleRegistryStore.getState().addClass(APP, { id: cid, name: "btn", properties: [] });
    const p = { id: uid("p"), name: "padding", segments: [{ type: "text" as const, value: "8px" }] };
    const cmd = new AddClassProperty(APP, cid, p);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(1);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(0);
  });

  it("RemoveClassProperty → execute removes, undo adds", () => {
    const cid = uid("c");
    const p = { id: uid("p"), name: "color", segments: [{ type: "text" as const, value: "red" }] };
    useStyleRegistryStore.getState().addClass(APP, { id: cid, name: "btn", properties: [p] });
    const cmd = new RemoveClassProperty(APP, cid, p);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(0);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(1);
  });

  it("UpdateClassProperty → execute updates, undo reverts", () => {
    const cid = uid("c");
    const pid = uid("p");
    useStyleRegistryStore.getState().addClass(APP, {
      id: cid, name: "btn",
      properties: [{ id: pid, name: "padding", segments: [{ type: "text", value: "8px" }] }],
    });
    const cmd = new UpdateClassProperty(APP, cid, pid, { name: "padding" }, { name: "margin" });
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties[0].name).toBe("margin");
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties[0].name).toBe("padding");
  });
});
```

- [ ] **Step 14: 运行命令测试**

```bash
cd editor/client && npx vitest run src/core/commands/__tests__/style-commands.test.ts
```

预期：9 个测试全部 PASS。

- [ ] **Step 15: 提交**

```bash
git add editor/client/src/core/commands/
git rm editor/client/src/core/commands/AddStyleEntry.ts editor/client/src/core/commands/RemoveStyleEntry.ts editor/client/src/core/commands/UpdateStyleEntry.ts editor/client/src/core/commands/ReorderStyleEntry.ts editor/client/src/core/commands/RenameStyleEntry.ts editor/client/src/core/commands/SetStyleEntryType.ts editor/client/src/core/commands/UpdateStyleProperty.ts
git commit -m "feat: add 11 new style commands, remove 7 old commands, rewrite command tests"
```

---

### Task 5: 安装 fude 并创建 StyleValueEditor

**Files:**
- Modify: `editor/client/package.json` (add dependency)
- Create: `editor/client/src/ui/panels/StyleValueEditor.tsx`

**Interfaces:**
- Consumes: `StyleValueSegment`
- Produces: `<StyleValueEditor segments={segments} onChange={fn} availableSignals={[...]} availableCssVars={[...]} />`

- [ ] **Step 1: 安装 `@tigerabrodioss/fude`**

```bash
cd editor/client && npm install @tigerabrodioss/fude
```

- [ ] **Step 2: 创建 StyleValueEditor 组件**

创建 `editor/client/src/ui/panels/StyleValueEditor.tsx`：

```typescript
// ui/panels/StyleValueEditor.tsx
// Wraps @tigerabrodioss/fude for OmniLang style value editing.
// Supports three segment types: text, signal (@ trigger), var-ref (var( trigger).

import React from "react";
import type { StyleValueSegment } from "../../core/types/style";

interface SignalOption {
  name: string;     // e.g. "ThemeStore.currentBg"
  label: string;    // e.g. "currentBg"
}

interface Props {
  segments: StyleValueSegment[];
  onChange: (segments: StyleValueSegment[]) => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];  // e.g. ["--primary", "--radius-sm"]
  placeholder?: string;
}

// Minimal single-line text input with chip rendering for signals/var-refs.
// Phase 2 is intentionally lightweight — fude library integration can be
// deepened in a follow-up if needed. The important thing is that the
// segment[] ↔ visual chips data flow is correct.

export function StyleValueEditor({
  segments,
  onChange,
  availableSignals = [],
  availableCssVars = [],
  placeholder = "",
}: Props) {
  const [text, setText] = React.useState(segmentsToDisplay(segments));
  const [showSuggestions, setShowSuggestions] = React.useState(false);
  const [suggestionType, setSuggestionType] = React.useState<"signal" | "var-ref" | null>(null);
  const [filterText, setFilterText] = React.useState("");
  const inputRef = React.useRef<HTMLInputElement>(null);

  // Keep text in sync when segments change externally
  React.useEffect(() => {
    setText(segmentsToDisplay(segments));
  }, [segments]);

  const handleInput = (rawValue: string) => {
    setText(rawValue);

    // Detect @ trigger
    const atMatch = rawValue.match(/@(\w*)$/);
    if (atMatch && availableSignals.length > 0) {
      setFilterText(atMatch[1]);
      setSuggestionType("signal");
      setShowSuggestions(true);
      return;
    }

    // Detect var( trigger
    const varMatch = rawValue.match(/var\((\-{0,2}\w*)$/);
    if (varMatch && availableCssVars.length > 0) {
      setFilterText(varMatch[1]);
      setSuggestionType("var-ref");
      setShowSuggestions(true);
      return;
    }

    setShowSuggestions(false);

    // If no special trigger, treat as plain text
    if (!rawValue.includes("@") && !rawValue.includes("var(")) {
      onChange([{ type: "text", value: rawValue }]);
    }
  };

  const insertSignal = (signal: SignalOption) => {
    const beforeText = text.replace(/@\w*$/, "");
    const newSegments: StyleValueSegment[] = [];

    if (beforeText.trim()) {
      newSegments.push({ type: "text", value: beforeText });
    }
    newSegments.push({ type: "signal", name: signal.name, label: signal.label });

    onChange(newSegments);
    setShowSuggestions(false);
    inputRef.current?.focus();
  };

  const insertVarRef = (varName: string) => {
    const beforeText = text.replace(/var\([\-]*\w*$/, "");
    const newSegments: StyleValueSegment[] = [];

    if (beforeText.trim()) {
      newSegments.push({ type: "text", value: beforeText });
    }
    newSegments.push({ type: "var-ref", name: varName });

    onChange(newSegments);
    setShowSuggestions(false);
    inputRef.current?.focus();
  };

  const filteredSignals = availableSignals.filter((s) =>
    s.label.toLowerCase().includes(filterText.toLowerCase()) ||
    s.name.toLowerCase().includes(filterText.toLowerCase())
  );

  const filteredVars = availableCssVars.filter((v) =>
    v.toLowerCase().includes(filterText.toLowerCase())
  );

  // Render chip-based display when segments include signals/var-refs
  const hasChips = segments.some((s) => s.type !== "text");

  return (
    <div style={{ position: "relative", flex: 1 }}>
      {hasChips ? (
        <div
          onClick={() => setText(segmentsToDisplay(segments))}
          style={{
            display: "flex", alignItems: "center", gap: 2, flexWrap: "wrap",
            minHeight: 24, padding: "2px 6px", border: "1px solid #d1d5db",
            borderRadius: 4, background: "#fff", cursor: "text",
            fontSize: 12,
          }}
        >
          {segments.map((seg, i) => {
            if (seg.type === "text") {
              return <span key={i} style={{ color: "#374151" }}>{seg.value}</span>;
            }
            if (seg.type === "signal") {
              return (
                <span key={i} style={{
                  background: "#DBEAFE", color: "#1E40AF", borderRadius: 3,
                  padding: "1px 5px", fontSize: 11, fontWeight: 500,
                  border: "1px solid #BFDBFE",
                }}>
                  @{seg.label}
                </span>
              );
            }
            if (seg.type === "var-ref") {
              return (
                <span key={i} style={{
                  background: "#D1FAE5", color: "#065F46", borderRadius: 3,
                  padding: "1px 5px", fontSize: 11, fontWeight: 500,
                  border: "1px solid #A7F3D0",
                }}>
                  var({seg.name})
                </span>
              );
            }
            return null;
          })}
        </div>
      ) : (
        <input
          ref={inputRef}
          type="text"
          value={text}
          onChange={(e) => handleInput(e.target.value)}
          placeholder={placeholder}
          style={{
            background: "#fff", color: "#374151", border: "1px solid #d1d5db",
            borderRadius: 4, padding: "4px 8px", fontSize: 12, width: "100%",
            boxSizing: "border-box",
          }}
        />
      )}

      {/* Suggestion dropdown */}
      {showSuggestions && (
        <div style={{
          position: "absolute", zIndex: 20, top: "100%", left: 0,
          background: "#fff", border: "1px solid #d1d5db", borderRadius: 4,
          boxShadow: "0 4px 12px rgba(0,0,0,0.1)", maxHeight: 200,
          overflow: "auto", minWidth: 200,
        }}>
          {suggestionType === "signal" && filteredSignals.map((s) => (
            <div
              key={s.name}
              onClick={() => insertSignal(s)}
              style={{
                padding: "6px 10px", cursor: "pointer", fontSize: 12,
                background: "transparent",
              }}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = "#F3F4F6"; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = "transparent"; }}
            >
              <span style={{ color: "#1E40AF", fontWeight: 500 }}>@{s.label}</span>
              <span style={{ color: "#9CA3AF", marginLeft: 8, fontSize: 10 }}>{s.name}</span>
            </div>
          ))}
          {suggestionType === "var-ref" && filteredVars.map((v) => (
            <div
              key={v}
              onClick={() => insertVarRef(v)}
              style={{
                padding: "6px 10px", cursor: "pointer", fontSize: 12,
              }}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = "#F3F4F6"; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = "transparent"; }}
            >
              <span style={{ color: "#065F46", fontWeight: 500 }}>{v}</span>
            </div>
          ))}
          {((suggestionType === "signal" && filteredSignals.length === 0) ||
            (suggestionType === "var-ref" && filteredVars.length === 0)) && (
            <div style={{ padding: "6px 10px", color: "#9CA3AF", fontSize: 11 }}>
              无匹配项
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/** Render segments as a display-only text string (for the raw input view). */
function segmentsToDisplay(segments: StyleValueSegment[]): string {
  return segments.map((s) => {
    if (s.type === "text") return s.value;
    if (s.type === "signal") return `@${s.label}`;
    if (s.type === "var-ref") return `var(${s.name})`;
    return "";
  }).join("");
}
```

- [ ] **Step 3: 验证 TypeScript 编译**

```bash
cd editor/client && npx tsc --noEmit --pretty false 2>&1 | grep -i "StyleValueEditor" | head -10
```

预期：无错误。

- [ ] **Step 4: 提交**

```bash
git add editor/client/package.json editor/client/package-lock.json editor/client/src/ui/panels/StyleValueEditor.tsx
git commit -m "feat: add StyleValueEditor with @ signal and var( CSS variable autocomplete"
```

---

### Task 6: 重构 StylePropertyEditor（集成 fude）

**Files:**
- Rewrite: `editor/client/src/ui/panels/StylePropertyEditor.tsx`

**Interfaces:**
- Consumes: `StyleValueEditor`, `StyleProperty`
- Produces: `<StylePropertyEditor property={prop} onChange={fn} onDelete={fn} availableSignals={[...]} availableCssVars={[...]} />`

- [ ] **Step 1: 重写 StylePropertyEditor.tsx**

```typescript
// ui/panels/StylePropertyEditor.tsx
// Refactored: replaces raw <input>/<select> with StyleValueEditor (fude).

import React, { useState, useEffect, useRef } from "react";
import type { StyleProperty } from "../../core/types/style";
import { StyleValueEditor } from "./StyleValueEditor";
import {
  isColorProperty,
  isCustomProperty,
  getUnitGroup,
  getDropdownOptions,
} from "../../constants/style-properties";

interface SignalOption {
  name: string;
  label: string;
}

interface Props {
  property: StyleProperty;
  onChange: (prop: StyleProperty) => void;
  onDelete: () => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];
}

// ── Property name search-select dropdown (unchanged from original) ──

const PROPERTY_CATEGORIES: { label: string; props: string[] }[] = [
  { label: "Layout", props: ["width","height","min-width","max-width","min-height","max-height","margin","margin-top","margin-right","margin-bottom","margin-left","padding","padding-top","padding-right","padding-bottom","padding-left","display","gap","overflow","overflow-x","overflow-y"] },
  { label: "Typography", props: ["font-size","font-weight","font-family","color","text-align","line-height","letter-spacing","text-decoration","text-transform","white-space","word-break"] },
  { label: "Background & Border", props: ["background-color","background-image","border","border-color","border-radius","box-shadow","opacity"] },
  { label: "Position", props: ["position","top","right","bottom","left","z-index"] },
  { label: "Flex & Grid", props: ["flex-direction","flex-wrap","flex-grow","flex-shrink","flex-basis","justify-content","align-items","align-self","grid-template-columns","grid-template-rows","grid-column","grid-row"] },
  { label: "Transform & Transition", props: ["transform","transition","transition-duration","transition-property","transition-timing-function","animation","animation-name","animation-duration","animation-iteration-count","animation-direction","animation-fill-mode","animation-timing-function","cursor"] },
];

const PropertyNameSelect: React.FC<{
  value: string;
  onChange: (name: string) => void;
  includeCustom?: boolean;
}> = ({ value, onChange, includeCustom }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [searchText, setSearchText] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;
    const close = () => { setIsOpen(false); setSearchText(""); };
    const mouseHandler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) close();
    };
    const keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("mousedown", mouseHandler);
    document.addEventListener("keydown", keyHandler);
    return () => {
      document.removeEventListener("mousedown", mouseHandler);
      document.removeEventListener("keydown", keyHandler);
    };
  }, [isOpen]);

  const filtered = PROPERTY_CATEGORIES
    .map(cat => ({ ...cat, props: cat.props.filter(p => p.toLowerCase().includes(searchText.toLowerCase())) }))
    .filter(cat => cat.props.length > 0);

  const showCustom = includeCustom && value.startsWith("--") && value.toLowerCase().includes(searchText.toLowerCase());

  return (
    <div ref={ref} style={{ position: "relative" }}>
      <div onClick={() => setIsOpen(!isOpen)} style={{ background:"#fff",color:"#374151",border:"1px solid #d1d5db",borderRadius:4,padding:"4px 8px",fontSize:12,cursor:"pointer",width:110,flex:"none" }}>
        {value || "select..."}
      </div>
      {isOpen && (
        <div style={{ position:"absolute",zIndex:10,background:"#fff",border:"1px solid #d1d5db",borderRadius:4,marginTop:2,maxHeight:250,overflow:"auto",width:200,boxShadow:"0 4px 12px rgba(0,0,0,0.1)" }}>
          <input autoFocus placeholder="搜索属性..." value={searchText} onChange={e => setSearchText(e.target.value)}
            style={{ width:"100%",boxSizing:"border-box",padding:"6px 8px",border:"none",borderBottom:"1px solid #e5e7eb",outline:"none",fontSize:12 }} />
          {filtered.map(cat => (
            <div key={cat.label}>
              <div style={{ padding:"4px 8px",fontSize:10,color:"#6B7280",fontWeight:600 }}>{cat.label}</div>
              {cat.props.map(p => (
                <div key={p} onClick={() => { onChange(p); setIsOpen(false); setSearchText(""); }}
                  style={{ padding:"4px 8px",cursor:"pointer",fontSize:12,background:p===value?"#EEF2FF":"transparent",color:p===value?"#4F46E5":"#374151" }}>
                  {p}
                </div>
              ))}
            </div>
          ))}
          {showCustom && (
            <div>
              <div style={{ padding:"4px 8px",fontSize:10,color:"#6B7280",fontWeight:600 }}>Custom</div>
              <div onClick={() => { onChange(value); setIsOpen(false); setSearchText(""); }}
                style={{ padding:"4px 8px",cursor:"pointer",fontSize:12,background:"#EEF2FF",color:"#4F46E5" }}>
                {value}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// ── Delete button ──
const DeleteBtn: React.FC<{ onClick: () => void }> = ({ onClick }) => (
  <button onClick={onClick} style={{ background:"none",border:"none",color:"#DC2626",cursor:"pointer",fontSize:14,padding:"0 4px" }} title="删除">×</button>
);

// ── Main component ──

const ROW = { display: "flex", gap: 6, alignItems: "center", marginBottom: 4 } as React.CSSProperties;

export function StylePropertyEditor({ property, onChange, onDelete, availableSignals, availableCssVars }: Props) {
  const isCustom = isCustomProperty(property.name);
  const color = isColorProperty(property.name);
  const dropdownOpts = getDropdownOptions(property.name);
  const unitGroup = getUnitGroup(property.name);

  // Extract text value from segments for color/unit/dropdown special rendering
  const textValue = property.segments.length === 1 && property.segments[0].type === "text"
    ? property.segments[0].value
    : "";

  // Color properties: color picker + fude editor
  if (color) {
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <input type="color" value={textValue.startsWith("#") ? textValue : "#000000"}
          onChange={(e) => onChange({ ...property, segments: [{ type: "text", value: e.target.value }] })}
          style={{ width: 28, height: 24, border: "none", cursor: "pointer", flex: "none" }} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Dropdown properties
  if (dropdownOpts !== null) {
    if (Array.isArray(dropdownOpts)) {
      return (
        <div style={ROW}>
          <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
          <select value={textValue} onChange={(e) => onChange({ ...property, segments: [{ type: "text", value: e.target.value }] })}
            style={{ background:"#fff",color:"#374151",border:"1px solid #d1d5db",borderRadius:4,padding:"4px 8px",fontSize:12,flex:1 }}>
            {dropdownOpts.map((opt) => (<option key={opt} value={opt}>{opt}</option>))}
          </select>
          <DeleteBtn onClick={onDelete} />
        </div>
      );
    }
    // Free text for transform, font-family, etc.
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Opacity: slider + number + fude
  if (property.name === "opacity") {
    const numVal = parseFloat(textValue);
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <input type="range" min="0" max="1" step="0.01" value={isNaN(numVal) ? 1 : numVal}
          onChange={(e) => onChange({ ...property, segments: [{ type: "text", value: e.target.value }] })}
          style={{ flex: 1 }} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Numeric + unit
  if (unitGroup) {
    return (
      <div style={ROW}>
        <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
        <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
        <DeleteBtn onClick={onDelete} />
      </div>
    );
  }

  // Generic: fude editor
  return (
    <div style={ROW}>
      <PropertyNameSelect value={property.name} onChange={(name) => onChange({ ...property, name })} includeCustom={isCustom} />
      <StyleValueEditor segments={property.segments} onChange={(segments) => onChange({ ...property, segments })} availableSignals={availableSignals} availableCssVars={availableCssVars} />
      <DeleteBtn onClick={onDelete} />
    </div>
  );
}
```

- [ ] **Step 2: 提交**

```bash
git add editor/client/src/ui/panels/StylePropertyEditor.tsx
git commit -m "refactor: integrate StyleValueEditor into StylePropertyEditor"
```

---

### Task 7: 删除 StyleEntryEditor，创建 ClassCard + ClassPreview + CssVariableEditor + StyleDesignModal

**Files:**
- Delete: `editor/client/src/ui/panels/StyleEntryEditor.tsx`
- Create: `editor/client/src/ui/panels/ClassPreview.tsx`
- Create: `editor/client/src/ui/panels/ClassCard.tsx`
- Create: `editor/client/src/ui/panels/CssVariableEditor.tsx`
- Create: `editor/client/src/ui/panels/StyleDesignModal.tsx`

**Interfaces:**
- Consumes: `StylePropertyEditor`, `StyleValueEditor`, `useStyleRegistryStore`
- Produces: `<StyleDesignModal onClose={fn} componentName={string} />`

- [ ] **Step 1: 删除 StyleEntryEditor**

```bash
rm editor/client/src/ui/panels/StyleEntryEditor.tsx
```

- [ ] **Step 2: 创建 ClassPreview（左侧预览区）**

创建 `editor/client/src/ui/panels/ClassPreview.tsx`：

```typescript
// ui/panels/ClassPreview.tsx
// Renders a single HTML element with the given class styles applied for live preview.

import React, { useMemo } from "react";
import type { StyleClass } from "../../core/types/style";

const PREVIEW_TAGS = ["div", "button", "h1", "h2", "h3", "input", "span", "p"];

interface Props {
  styleClass: StyleClass;
  previewTag: string;
  onTagChange: (tag: string) => void;
}

export function ClassPreview({ styleClass, previewTag, onTagChange }: Props) {
  const cssText = useMemo(() => {
    return styleClass.properties
      .map((p) => {
        const val = segmentsToCssValue(p.segments);
        return `${p.name}: ${val};`;
      })
      .join(" ");
  }, [styleClass]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {/* Tag selector */}
      <select
        value={previewTag}
        onChange={(e) => onTagChange(e.target.value)}
        style={{
          padding: "2px 6px", fontSize: 11, border: "1px solid #d1d5db",
          borderRadius: 4, background: "#fff", cursor: "pointer",
        }}
      >
        {PREVIEW_TAGS.map((t) => (
          <option key={t} value={t}>&lt;{t}&gt;</option>
        ))}
      </select>

      {/* Preview area */}
      <div style={{
        border: "1px dashed #d1d5db", borderRadius: 6,
        padding: 12, minWidth: 120, minHeight: 40,
        background: "#fafafa",
        display: "flex", alignItems: "center", justifyContent: "center",
      }}>
        {previewTag === "button" && (
          <button style={cssText ? parseInlineStyle(cssText) : undefined}>
            {cssText ? "Click" : "Button"}
          </button>
        )}
        {previewTag === "input" && (
          <input placeholder="Input" style={cssText ? parseInlineStyle(cssText) : undefined} />
        )}
        {(previewTag === "div" || previewTag === "span" || previewTag === "p") && (
          React.createElement(previewTag, {
            style: cssText ? parseInlineStyle(cssText) : undefined,
            children: cssText ? "Preview" : previewTag,
          })
        )}
        {(previewTag === "h1" || previewTag === "h2" || previewTag === "h3") && (
          React.createElement(previewTag, {
            style: cssText ? parseInlineStyle(cssText) : undefined,
            children: cssText ? "Heading" : previewTag,
          })
        )}
      </div>
    </div>
  );
}

function segmentsToCssValue(segments: Array<{ type: string; value?: string; name?: string; label?: string }>): string {
  return segments.map((s) => {
    if (s.type === "text") return s.value ?? "";
    if (s.type === "var-ref") return `var(${s.name})`;
    if (s.type === "signal") return `@${s.label}`;
    return "";
  }).join("");
}

function parseInlineStyle(cssText: string): React.CSSProperties {
  const style: Record<string, string> = {};
  cssText.split(";").forEach((decl) => {
    const colonIdx = decl.indexOf(":");
    if (colonIdx > 0) {
      const prop = decl.slice(0, colonIdx).trim();
      const val = decl.slice(colonIdx + 1).trim();
      if (prop && val) {
        const camelProp = prop.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
        style[camelProp] = val;
      }
    }
  });
  return style as React.CSSProperties;
}
```

- [ ] **Step 3: 创建 ClassCard（单个 class 卡片）**

创建 `editor/client/src/ui/panels/ClassCard.tsx`：

```typescript
// ui/panels/ClassCard.tsx
// Expandable card for editing a single StyleClass.

import React, { useState } from "react";
import type { StyleClass, StyleProperty } from "../../core/types/style";
import { ClassPreview } from "./ClassPreview";
import { StylePropertyEditor } from "./StylePropertyEditor";
import { uid } from "../../core/utils/id";

interface SignalOption {
  name: string;
  label: string;
}

interface Props {
  styleClass: StyleClass;
  onChange: (updated: StyleClass) => void;
  onDelete: () => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];
}

export function ClassCard({ styleClass, onChange, onDelete, availableSignals, availableCssVars }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [previewTag, setPreviewTag] = useState("div");

  const handleNameChange = (name: string) => {
    onChange({ ...styleClass, name });
  };

  const handlePropertyChange = (index: number, prop: StyleProperty) => {
    const newProps = [...styleClass.properties];
    newProps[index] = prop;
    onChange({ ...styleClass, properties: newProps });
  };

  const handlePropertyDelete = (index: number) => {
    onChange({
      ...styleClass,
      properties: styleClass.properties.filter((_, i) => i !== index),
    });
  };

  const handleAddProperty = () => {
    onChange({
      ...styleClass,
      properties: [
        ...styleClass.properties,
        { id: uid("p"), name: "padding", segments: [{ type: "text", value: "" }] },
      ],
    });
  };

  if (!expanded) {
    return (
      <div
        onClick={() => setExpanded(true)}
        style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "6px 10px", cursor: "pointer", borderRadius: 4,
          fontSize: 13, color: "#374151",
        }}
        onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }}
        onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
      >
        <span style={{ fontSize: 10, color: "#9CA3AF" }}>▶</span>
        <span style={{ fontWeight: 500 }}>.{styleClass.name || "unnamed"}</span>
      </div>
    );
  }

  return (
    <div style={{
      background: "#fff", border: "1px solid #e5e7eb", borderRadius: 6,
      padding: 10, marginBottom: 6,
    }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <span
          onClick={() => setExpanded(false)}
          style={{ fontSize: 10, color: "#9CA3AF", cursor: "pointer" }}
        >
          ▼
        </span>
        <span style={{ fontSize: 11, color: "#6B7280" }}>.</span>
        <input
          type="text"
          value={styleClass.name}
          onChange={(e) => handleNameChange(e.target.value)}
          placeholder="class-name"
          style={{
            border: "1px solid #d1d5db", borderRadius: 4, padding: "2px 6px",
            fontSize: 12, flex: 1, maxWidth: 200,
          }}
        />
        <button
          onClick={(e) => { e.stopPropagation(); onDelete(); }}
          style={{
            marginLeft: "auto", background: "none", border: "none",
            color: "#DC2626", cursor: "pointer", fontSize: 14, padding: "0 4px",
          }}
        >
          ×
        </button>
      </div>

      {/* Body: preview left + properties right */}
      <div style={{ display: "flex", gap: 12 }}>
        <ClassPreview
          styleClass={styleClass}
          previewTag={previewTag}
          onTagChange={setPreviewTag}
        />
        <div style={{ flex: 1 }}>
          {styleClass.properties.map((prop, i) => (
            <StylePropertyEditor
              key={prop.id}
              property={prop}
              onChange={(p) => handlePropertyChange(i, p)}
              onDelete={() => handlePropertyDelete(i)}
              availableSignals={availableSignals}
              availableCssVars={availableCssVars}
            />
          ))}
          <button
            onClick={handleAddProperty}
            style={{
              background: "none", border: "1px dashed #C7D2FE", borderRadius: 4,
              color: "#4F46E5", cursor: "pointer", fontSize: 11,
              padding: "4px 8px", width: "100%", marginTop: 4,
            }}
          >
            + 添加属性
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 创建 CssVariableEditor（CSS 变量区域）**

创建 `editor/client/src/ui/panels/CssVariableEditor.tsx`：

```typescript
// ui/panels/CssVariableEditor.tsx
// List of CSS variable rows with fude value editors.

import React from "react";
import type { CssVariable, StyleValueSegment } from "../../core/types/style";
import { StyleValueEditor } from "./StyleValueEditor";

interface SignalOption {
  name: string;
  label: string;
}

interface Props {
  variables: CssVariable[];
  onAdd: () => void;
  onRemove: (id: string) => void;
  onChange: (id: string, patch: Partial<CssVariable>) => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];
}

export function CssVariableEditor({
  variables,
  onAdd,
  onRemove,
  onChange,
  availableSignals,
  availableCssVars,
}: Props) {
  return (
    <div style={{
      border: "1px solid #e5e7eb", borderRadius: 6, padding: 8,
      background: "#fafafa",
    }}>
      {variables.map((v) => (
        <div key={v.id} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
          <input
            type="text"
            value={v.name}
            onChange={(e) => onChange(v.id, { name: e.target.value })}
            placeholder="--variable-name"
            style={{
              border: "1px solid #d1d5db", borderRadius: 4, padding: "4px 8px",
              fontSize: 12, width: 160, flex: "none",
            }}
          />
          <span style={{ fontSize: 12, color: "#6B7280", flex: "none" }}>:</span>
          <StyleValueEditor
            segments={v.segments}
            onChange={(segments: StyleValueSegment[]) => onChange(v.id, { segments })}
            availableSignals={availableSignals}
            availableCssVars={availableCssVars}
          />
          <button
            onClick={() => onRemove(v.id)}
            style={{
              background: "none", border: "none", color: "#DC2626",
              cursor: "pointer", fontSize: 14, padding: "0 4px", flex: "none",
            }}
          >
            ×
          </button>
        </div>
      ))}
      <button
        onClick={onAdd}
        style={{
          background: "none", border: "1px dashed #C7D2FE", borderRadius: 4,
          color: "#4F46E5", cursor: "pointer", fontSize: 11,
          padding: "4px 8px", width: "100%",
        }}
      >
        + 添加变量
      </button>
    </div>
  );
}
```

- [ ] **Step 5: 创建 StyleDesignModal（弹窗容器）**

创建 `editor/client/src/ui/panels/StyleDesignModal.tsx`：

```typescript
// ui/panels/StyleDesignModal.tsx
// Modal dialog for editing component-level CSS styles (variables + classes).

import React, { useState } from "react";
import { useStyleRegistryStore } from "../../core/state/styleRegistry";
import { useUndoStore } from "../../store/undo";
import { uid } from "../../core/utils/id";
import { CssVariableEditor } from "./CssVariableEditor";
import { ClassCard } from "./ClassCard";
import { AddCssVariable } from "../../core/commands/AddCssVariable";
import { RemoveCssVariable } from "../../core/commands/RemoveCssVariable";
import { UpdateCssVariable } from "../../core/commands/UpdateCssVariable";
import { AddStyleClass } from "../../core/commands/AddStyleClass";
import { RemoveStyleClass } from "../../core/commands/RemoveStyleClass";
import type { CommandContext } from "../../core/commands/types";
import type { CssVariable, StyleClass } from "../../core/types/style";

interface Props {
  componentName: string;
  ctx: CommandContext;
  onClose: () => void;
}

export function StyleDesignModal({ componentName, ctx, onClose }: Props) {
  const registry = useStyleRegistryStore((s) => s.getRegistry(componentName));
  const undoExec = useUndoStore((s) => s.execute);

  const [varsExpanded, setVarsExpanded] = useState(true);
  const [classesExpanded, setClassesExpanded] = useState(true);

  const handleAddVariable = () => {
    const v: CssVariable = { id: uid("v"), name: "--unnamed", segments: [{ type: "text", value: "" }] };
    undoExec(new AddCssVariable(componentName, v), ctx);
  };

  const handleRemoveVariable = (id: string) => {
    const v = registry.variables.find((x) => x.id === id);
    if (v) undoExec(new RemoveCssVariable(componentName, v), ctx);
  };

  const handleChangeVariable = (id: string, patch: Partial<CssVariable>) => {
    const v = registry.variables.find((x) => x.id === id);
    if (!v) return;
    const oldName = v.name;
    const newName = patch.name ?? oldName;
    const oldSegments = v.segments;
    const newSegments = patch.segments ?? oldSegments;
    if (oldName !== newName || oldSegments !== newSegments) {
      undoExec(new UpdateCssVariable(componentName, id, { name: oldName, segments: oldSegments }, patch), ctx);
    }
  };

  const handleAddClass = () => {
    const c: StyleClass = { id: uid("c"), name: "", properties: [] };
    undoExec(new AddStyleClass(componentName, c), ctx);
  };

  const handleRemoveClass = (id: string) => {
    const c = registry.classes.find((x) => x.id === id);
    if (c) undoExec(new RemoveStyleClass(componentName, c), ctx);
  };

  const handleChangeClass = (updated: StyleClass) => {
    const existing = registry.classes.find((c) => c.id === updated.id);
    if (existing) {
      const store = useStyleRegistryStore.getState();
      store.removeClass(componentName, updated.id);
      store.addClass(componentName, updated);
    }
  };

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={onClose}
        style={{ position: "fixed", inset: 0, zIndex: 9998, background: "rgba(0,0,0,0.3)" }}
      />

      {/* Modal */}
      <div style={{
        position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)",
        zIndex: 9999, background: "#fff", borderRadius: 12,
        boxShadow: "0 8px 40px rgba(0,0,0,0.2)",
        width: 900, maxWidth: "95vw", maxHeight: "90vh",
        overflow: "auto", padding: "24px",
      }}>
        <h2 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600 }}>CSS 样式设计</h2>

        {/* CSS Variables section */}
        <div style={{ marginBottom: 16 }}>
          <div
            onClick={() => setVarsExpanded(!varsExpanded)}
            style={{ display: "flex", alignItems: "center", gap: 4, cursor: "pointer", marginBottom: 8 }}
          >
            <span style={{ fontSize: 11 }}>{varsExpanded ? "▼" : "▶"}</span>
            <span style={{ fontSize: 13, fontWeight: 600, color: "#374151" }}>CSS 变量</span>
            <span style={{ marginLeft: "auto", fontSize: 11, color: "#4F46E5", cursor: "pointer" }}
              onClick={(e) => { e.stopPropagation(); handleAddVariable(); }}>
              + 添加变量
            </span>
          </div>
          {varsExpanded && (
            <CssVariableEditor
              variables={registry.variables}
              onAdd={handleAddVariable}
              onRemove={handleRemoveVariable}
              onChange={handleChangeVariable}
            />
          )}
        </div>

        {/* Class definitions section */}
        <div>
          <div
            onClick={() => setClassesExpanded(!classesExpanded)}
            style={{ display: "flex", alignItems: "center", gap: 4, cursor: "pointer", marginBottom: 8 }}
          >
            <span style={{ fontSize: 11 }}>{classesExpanded ? "▼" : "▶"}</span>
            <span style={{ fontSize: 13, fontWeight: 600, color: "#374151" }}>类定义</span>
            <span style={{ marginLeft: "auto", fontSize: 11, color: "#4F46E5", cursor: "pointer" }}
              onClick={(e) => { e.stopPropagation(); handleAddClass(); }}>
              + 新建 Class
            </span>
          </div>
          {classesExpanded && (
            <div>
              {registry.classes.map((c) => (
                <ClassCard
                  key={c.id}
                  styleClass={c}
                  onChange={handleChangeClass}
                  onDelete={() => handleRemoveClass(c.id)}
                />
              ))}
              {registry.classes.length === 0 && (
                <div style={{ padding: 16, color: "#9CA3AF", fontSize: 12, textAlign: "center" }}>
                  暂无类定义，点击 "+ 新建 Class" 添加
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 20, paddingTop: 16, borderTop: "1px solid #e5e7eb" }}>
          <button onClick={onClose} style={{
            padding: "8px 20px", fontSize: 13, border: "1px solid #d1d5db",
            borderRadius: 6, background: "#fff", cursor: "pointer",
          }}>
            取消
          </button>
          <button onClick={onClose} style={{
            padding: "8px 20px", fontSize: 13, border: "none", borderRadius: 6,
            background: "#4F46E5", color: "#fff", cursor: "pointer", fontWeight: 600,
          }}>
            保存
          </button>
        </div>
      </div>
    </>
  );
}
```

- [ ] **Step 6: 在 Toolbar 中添加 CSS 样式按钮**

在 `editor/client/src/ui/shell/Toolbar.tsx` 的 `return` 中，Save 按钮后面添加：

```tsx
<span style={{ width: 12 }} />
<button onClick={onOpenCssStyles} style={{...btnStyle, color: "#6366F1", fontWeight: 600, fontSize: 12}} title="CSS 样式设计">
  🎨 CSS 样式
</button>
```

在 toolbar 的 props 中新增 `onOpenCssStyles: () => void`。

- [ ] **Step 7: 在 DesignModeLayout 中接入弹窗**

在 `editor/client/src/features/design-mode/DesignModeLayout.tsx` 中：
1. 添加 `const [showStyleModal, setShowStyleModal] = useState(false);`
2. 将 `onOpenCssStyles={() => setShowStyleModal(true)}` 传给 Toolbar
3. 在 return 底部添加：

```tsx
{showStyleModal && (
  <StyleDesignModal
    componentName="App"
    ctx={ctx}
    onClose={() => setShowStyleModal(false)}
  />
)}
```

需要 import `StyleDesignModal`。

- [ ] **Step 8: 提交**

```bash
git add editor/client/src/ui/panels/ClassPreview.tsx editor/client/src/ui/panels/ClassCard.tsx editor/client/src/ui/panels/CssVariableEditor.tsx editor/client/src/ui/panels/StyleDesignModal.tsx
git rm editor/client/src/ui/panels/StyleEntryEditor.tsx
git add editor/client/src/ui/shell/Toolbar.tsx editor/client/src/features/design-mode/DesignModeLayout.tsx
git commit -m "feat: add StyleDesignModal with ClassCard, ClassPreview, CssVariableEditor"
```

---

### Task 8: 重写右侧 StyleSection

**Files:**
- Rewrite: `editor/client/src/ui/inspector/sections/StyleSection.tsx`

- [ ] **Step 1: 重写 StyleSection**

```typescript
// ui/inspector/sections/StyleSection.tsx
// Simplified: shows applied classes (from registry) + inline styles for the selected element.

import React from "react";
import type { ElementData, StyleProperty } from "../../../core/types";
import type { CommandContext } from "../../../core/commands/types";
import { useEditorStore } from "../../../store/editor";
import { useStyleRegistryStore } from "../../../core/state/styleRegistry";
import { useUndoStore } from "../../../store/undo";
import { SetElementClassRefs } from "../../../core/commands/SetElementClassRefs";
import { SetElementInlineStyles } from "../../../core/commands/SetElementInlineStyles";
import { StylePropertyEditor } from "../../panels/StylePropertyEditor";
import { uid } from "../../../core/utils/id";

interface Props {
  element: ElementData;
  ctx: CommandContext;
  onRecompile: () => void;
}

export function StyleSection({ element: el, ctx, onRecompile }: Props) {
  const undoExec = useUndoStore((s) => s.execute);
  const registry = useStyleRegistryStore((s) => s.getRegistry("App"));
  const classRefs = (el as any).style?.classRefs ?? [];
  const inlineProperties: StyleProperty[] = (el as any).style?.inlineProperties ?? [];

  const handleAddClassRef = (className: string) => {
    const newRefs = [...classRefs, className];
    undoExec(new SetElementClassRefs(el.id, classRefs, newRefs), ctx);
    onRecompile();
  };

  const handleRemoveClassRef = (className: string) => {
    const newRefs = classRefs.filter((c: string) => c !== className);
    undoExec(new SetElementClassRefs(el.id, classRefs, newRefs), ctx);
    onRecompile();
  };

  const handleAddInline = () => {
    const newProps = [...inlineProperties, { id: uid("p"), name: "padding", segments: [{ type: "text" as const, value: "" }] }];
    undoExec(new SetElementInlineStyles(el.id, inlineProperties, newProps), ctx);
    onRecompile();
  };

  const handleRemoveInline = (index: number) => {
    const newProps = inlineProperties.filter((_, i) => i !== index);
    undoExec(new SetElementInlineStyles(el.id, inlineProperties, newProps), ctx);
    onRecompile();
  };

  const handleChangeInline = (index: number, prop: StyleProperty) => {
    const newProps = [...inlineProperties];
    newProps[index] = prop;
    undoExec(new SetElementInlineStyles(el.id, inlineProperties, newProps), ctx);
    onRecompile();
  };

  // Get unapplied classes (in registry but not on element)
  const unappliedClasses = registry.classes.filter((c) => !classRefs.includes(c.name));

  return (
    <>
      <div style={SECTION_HEADER}>样式</div>

      {/* Applied classes */}
      <div style={{ marginBottom: 8 }}>
        <div style={{ fontSize: 11, color: "#6B7280", marginBottom: 4 }}>📎 应用的类</div>
        {classRefs.length === 0 && (
          <div style={{ fontSize: 11, color: "#9CA3AF", padding: "4px 0" }}>无</div>
        )}
        {classRefs.map((cn: string) => (
          <div key={cn} style={{ display: "flex", alignItems: "center", fontSize: 12, padding: "2px 0" }}>
            <span style={{ color: "#374151", flex: 1 }}>.{cn}</span>
            <button
              onClick={() => handleRemoveClassRef(cn)}
              style={{ background: "none", border: "none", color: "#DC2626", cursor: "pointer", fontSize: 14 }}
            >
              ×
            </button>
          </div>
        ))}
        {unappliedClasses.length > 0 && (
          <select
            value=""
            onChange={(e) => { if (e.target.value) handleAddClassRef(e.target.value); }}
            style={{
              width: "100%", marginTop: 4, padding: "2px 6px", fontSize: 11,
              border: "1px solid #d1d5db", borderRadius: 4,
            }}
          >
            <option value="">+ 关联 class...</option>
            {unappliedClasses.map((c) => (
              <option key={c.id} value={c.name}>.{c.name}</option>
            ))}
          </select>
        )}
      </div>

      {/* Inline styles */}
      <div>
        <div style={{ fontSize: 11, color: "#6B7280", marginBottom: 4 }}>⚡ 内联样式</div>
        {inlineProperties.map((prop, i) => (
          <StylePropertyEditor
            key={prop.id}
            property={prop}
            onChange={(p) => handleChangeInline(i, p)}
            onDelete={() => handleRemoveInline(i)}
          />
        ))}
        <button
          onClick={handleAddInline}
          style={{
            background: "none", border: "1px dashed #C7D2FE", borderRadius: 4,
            color: "#4F46E5", cursor: "pointer", fontSize: 11,
            padding: "4px 8px", width: "100%", marginTop: 4,
          }}
        >
          + 添加
        </button>
      </div>
    </>
  );
}

const SECTION_HEADER: React.CSSProperties = {
  fontSize: 11, color: "#6b7280", textTransform: "uppercase",
  margin: "12px 0 4px",
};
```

- [ ] **Step 2: 更新 PropertyPanel.tsx 中的 StyleSection 调用**

`StyleSection` 的 props 不变（`element`, `ctx`, `onRecompile`），但 import 从 classes/inline 改为新的 `ElementStyle`。确保传入的 `element` 仍然是 `ElementData` 类型。

- [ ] **Step 3: 提交**

```bash
git add editor/client/src/ui/inspector/sections/StyleSection.tsx editor/client/src/ui/panels/PropertyPanel.tsx
git commit -m "refactor: rewrite StyleSection as classRefs + inlineProperties, update PropertyPanel"
```

---

### Task 9: 适配 Codegen

**Files:**
- Modify: `editor/client/src/services/codegen/collect-styles.ts`
- Modify: `editor/client/src/services/codegen/source-emitter.ts`
- Modify: `editor/client/src/services/codegen/pipeline.ts`
- Modify: `editor/client/src/services/codegen/types.ts`

- [ ] **Step 1: 适配 collect-styles.ts**

重写 `collectStylesFromEntries` 以处理新数据模型：

```typescript
// collect-styles.ts — adapted version

export function collectStylesFromEntries(
  elements: Array<{ id: string; style?: ElementStyle }>,
  styleRegistry: StyleRegistry,
  roots?: ViewNode[],
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
} {
  const componentStyle: Record<string, Record<string, string>> = {};
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

  // Phase 1: Build componentStyle from StyleRegistry
  for (const cls of styleRegistry.classes) {
    const rules: Record<string, string> = {};
    for (const prop of cls.properties) {
      rules[prop.name] = compileSegments(prop.segments);
    }
    componentStyle[cls.name] = { ...(componentStyle[cls.name] ?? {}), ...rules };
  }

  for (const v of styleRegistry.variables) {
    const className = variableToClassName(v.name);
    const rules = { [v.name]: compileSegments(v.segments) };
    componentStyle[className] = { ...(componentStyle[className] ?? {}), ...rules };
  }

  // Phase 2: Element classRefs → elementClassMap
  for (const el of elements) {
    const classRefs = el.style?.classRefs ?? [];
    if (classRefs.length > 0) {
      elementClassMap.set(el.id, [...new Set(classRefs)]);
    }
    const inlineProps = el.style?.inlineProperties ?? [];
    if (inlineProps.length > 0) {
      const inlineMap: Record<string, string> = {};
      for (const prop of inlineProps) {
        inlineMap[prop.name] = compileSegments(prop.segments);
      }
      elementInlineMap.set(el.id, inlineMap);
    }
  }

  return { componentStyle, elementClassMap, elementInlineMap };
}

function compileSegments(segments: StyleValueSegment[]): string {
  if (segments.length === 0) return "";
  if (segments.length === 1) {
    const s = segments[0];
    if (s.type === "text") return `"${s.value}"`;
    if (s.type === "signal") return s.name;
    if (s.type === "var-ref") return `var(${s.name})`;
  }
  return segments.map((s, i) => {
    if (s.type === "text") {
      // Only wrap in quotes if it's standalone or between other segments
      return `"${s.value}"`;
    }
    if (s.type === "signal") return s.name;
    if (s.type === "var-ref") return `var(${s.name})`;
    return "";
  }).join(" + ");
}
```

- [ ] **Step 2: 适配 pipeline.ts — 更新 generateOmniSource 调用 collectStylesFromEntries**

找到 `generateOmniSource` 函数中调用 `collectStylesFromEntries` 的地方（约第 348 行），改为传入 `styleRegistry`：

```typescript
// Before:
const { componentStyle, elementClassMap, elementInlineMap } = ...
  ? { componentStyle: {}, elementClassMap: new Map(), elementInlineMap: new Map() }
  : collectStylesFromEntries(expandedElements, roots);

// After:
const activeRegistry = useStyleRegistryStore.getState().getRegistry(options.componentName);
const { componentStyle, elementClassMap, elementInlineMap } = ...
  ? { componentStyle: {}, elementClassMap: new Map(), elementInlineMap: new Map() }
  : collectStylesFromEntries(expandedElements, activeRegistry, roots);
```

添加 import:
```typescript
import { useStyleRegistryStore } from "../../core/state/styleRegistry";
```

- [ ] **Step 3: 适配 parseStylesFromAST — 重建 StyleRegistry + ElementStyle**

`parseStylesFromAST` 现在需要两件事：
1. 从 AST `style { }` block 构建 StyleRegistry（写入 `useStyleRegistryStore`）
2. 从 view node 的 `class:` / `style:` 构建 ElementStyle

```typescript
// 在 parseStylesFromAST 中替换旧的 StyleEntry[] 构建逻辑：
// 旧：el.styles = [StyleEntry, ...] 
// 新：
const classRefs: string[] = [];
const inlineProperties: StyleProperty[] = [];

// Parse class: attribute → classRefs
if (classAttr && typeof classAttr === "string") {
  classRefs.push(...classAttr.split(/\s+/));
}

// Parse inline_style → inlineProperties
if (inlineStyle && typeof inlineStyle === "object") {
  for (const [name, value] of Object.entries(inlineStyle)) {
    inlineProperties.push({
      id: generateId(),
      name,
      segments: [{ type: "text" as const, value: String(value) }],
    });
  }
}

el.style = { classRefs, inlineProperties };

// Parse style { } block → StyleRegistry
for (const rule of styleBlock) {
  const className: string = rule.class_name;
  const varName = classNameToVariable(className);
  const properties: StyleProperty[] = Object.entries(rule.properties).map(
    ([name, valueExpr]: [string, any]) => ({
      id: generateId(),
      name,
      segments: [{ type: "text" as const, value: exprToString(valueExpr) }],
    })
  );
  if (varName) {
    store.addVariable(componentName, { id: generateId(), name: varName, segments: properties[0]?.segments ?? [] });
  } else {
    store.addClass(componentName, { id: generateId(), name: className, properties });
  }
}
```

- [ ] **Step 3b: 适配 source-emitter.ts — inline 中区分信号和文本**

在 `emitViewNode` 中（约第 228 行），inline styles 的输出逻辑改为：

```typescript
const inlineStyles = this.elementInlineMap?.get(node.id);
if (inlineStyles && Object.keys(inlineStyles).length > 0) {
  const styleStr = Object.entries(inlineStyles)
    .filter(([, v]) => v !== "" && v !== undefined && v !== null)
    .map(([k, v]) => {
      // v is already the compiled Segments string from collectStylesFromEntries.
      // Signals appear bare, text appears quoted, expressions use +.
      return `${k}: ${v}`;
    })
    .join(", ");
  parts.push(` style: { ${styleStr} }`);
}
```

`compileSegments` 的返回值已经处理了引号（纯 text segment → `"val"`，signal → `name`），source-emitter 直接使用即可。

- [ ] **Step 4: 提交**

```bash
git add editor/client/src/services/codegen/
git commit -m "feat: adapt codegen for new StyleRegistry + StyleValueSegment model"
```

---

### Task 10: 端到端回归测试

**Files:**
- Create/Update: codegen integration tests

- [ ] **Step 1: 运行所有 style 相关测试**

```bash
cd editor/client && npx vitest run src/store/__tests__/style-registry.test.ts src/core/commands/__tests__/style-commands.test.ts
```

预期：所有测试 PASS。

- [ ] **Step 2: 运行 TypeScript 编译检查**

```bash
cd editor/client && npx tsc --noEmit 2>&1 | tail -20
```

预期：0 errors。

- [ ] **Step 3: 启动开发服务器验证 UI**

```bash
cd editor && python server.py &
cd editor/client && npm run dev
```

打开浏览器验证：
1. Toolbar 中出现 [🎨 CSS 样式] 按钮
2. 点击打开样式弹窗
3. 添加 CSS 变量、class、属性
4. 右侧面板选择元素可关联 class 和添加内联样式

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "chore: final integration fixes and test verification"
```
