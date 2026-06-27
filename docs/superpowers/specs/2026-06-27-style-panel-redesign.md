# Style Panel 全链路重设计

> 对标 OmniLang CSS 语言模型，端到端重建样式编辑系统。
> 从数据模型 → 命令/undo → store → UI → codegen 全链路。
>
> **目标产出**：编辑器能生成 `ThemeCssVarsDemo.omni`、`CssVarsDemo.omni`、`CssUnitsDemo.omni` 级别的样式代码。

---

## 一、数据模型

### 1.1 值类型（基于 fude Segment 模型）

```typescript
type StyleValueSegment =
  | { type: "text"; value: string }
  | { type: "signal"; name: string; label: string }
  | { type: "var-ref"; name: string };  // "--primary-color"
```

### 1.2 CSS 变量（组件级注册表）

```typescript
interface CssVariable {
  id: string;
  name: string;                       // "--primary-color"
  segments: StyleValueSegment[];
}
```

### 1.3 类定义（组件级注册表）

```typescript
interface StyleClass {
  id: string;
  name: string;                       // "wrapper" | "card.accent" | "btn:hover"
  properties: StyleProperty[];
}

interface StyleProperty {
  id: string;
  name: string;                       // "padding" / "--radius" / "background"
  segments: StyleValueSegment[];
}
```

### 1.4 组件级样式注册表（SSOT）

```typescript
interface StyleRegistry {
  variables: CssVariable[];
  classes: StyleClass[];
}
```

### 1.5 元素上的样式引用（per-element）

```typescript
interface ElementStyle {
  classRefs: string[];                // ["wrapper", "card"]
  inlineProperties: StyleProperty[];  // 内联属性（含信号绑定）
}
```

### 1.6 新旧对比

| 旧 | 新 |
|---|----|
| `StyleEntry.type: "class" \| "variable" \| "inline"` | 去掉 type。class/variable 在 StyleRegistry 层区分 |
| `StyleEntry.name` 承载 class 名 / variable 名 | `StyleClass.name` / `CssVariable.name` |
| 变量只能有 1 个 property | CSS 变量就是 key + segments，无 property 概念 |
| `StyleProperty.value: string` | `StyleProperty.segments: StyleValueSegment[]` |
| element.styles: StyleEntry[] | element.style.classRefs + element.style.inlineProperties |
| 无信号绑定 | signal segment → codegen 生成 `setProperty()` |
| 无 `var()` 感知 | var-ref segment → 编辑器内补全 |

---

## 二、UI 设计

### 2.1 入口

Toolbar 新增按钮：

```
[🎨 CSS 样式]
```

点击打开模态弹窗，宽度 900-1100px，高度自适应内容（max-height 90vh，overflow-y auto）。

### 2.2 CSS 样式弹窗布局

```
┌─────────────────── CSS 样式设计 ──────────────────────────────────┐
│                                                                    │
│  ▼ CSS 变量                                        [+ 添加变量]    │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ --primary:       [fude: #6366f1                ] [×]      │  │
│  │ --theme-bg:      [fude: @ThemeStore.currentBg  ] [×]      │  │
│  │ --border:        [fude: 2px solid @accentColor ] [×]      │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                    │
│  ▼ 类定义                                          [+ 新建 Class]  │
│                                                                    │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ ▶ .wrapper                                                │  │
│  │ ▼ .btn                                      [× 删除]      │  │
│  │ ┌──────────────────┐ ┌──────────────────────────────────┐ │  │
│  │ │  [button ▾]      │ │ padding      [8 px ▾]  [×]      │ │  │
│  │ │                  │ │ background   [@primary      ] [×]│ │  │
│  │ │ ┌──────────────┐ │ │ color        [var(--primary)] [×]│ │  │
│  │ │ │   [ Click ]  │ │ │ font-size    [14 px ▾]  [×]     │ │  │
│  │ │ │              │ │ │                                   │ │  │
│  │ │ └──────────────┘ │ │ [+ 添加属性]                      │ │  │
│  │ └──────────────────┘ └──────────────────────────────────┘  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                    │
│                                                [取消]      [保存]  │
└────────────────────────────────────────────────────────────────────┘
```

#### 折叠规则

- **CSS 变量区域**：点击 `▼/▶ CSS 变量` 收起/展开整块
- **类定义区域**：点击 `▼/▶ 类定义` 收起/展开整块
- **单个 class 卡片**：点击 `▼/▶ .btn` 展开/折叠。折叠态只显示名字，无预览无属性。

#### Class 卡片展开后

- **左侧预览区**：tag 下拉选择器（`div` / `button` / `h1` / `input` / `span`）+ 实时渲染预览（仅此一个 class 的效果）
- **右侧属性编辑**：属性列表，每行 property name 搜索选择 + fude 值编辑器 + × 删除

### 2.3 fude 值编辑器

#### Segment 类型

| Segment | UI 外观 | 触发方式 | 数据 |
|---------|--------|---------|------|
| text | 普通文字 | 直接输入 | `{type:"text", value:"2px solid "}` |
| signal | 蓝色 pill / chip | 输入 `@` | `{type:"signal", name:"accentColor", label:"accentColor"}` |
| var-ref | 绿色 pill / chip | 输入 `var(` | `{type:"var-ref", name:"--primary"}` |

#### 触发键与补全

| 输入 | 弹出 |
|------|------|
| `@` | 可用信号列表（store signals、component signals、store getters、signal props） |
| `var(` | 已定义的 CSS 变量列表（`--*`） |

#### 属性值编辑器的适配模式

当前 StylePropertyEditor 的「属性名 + 值编辑器」模式保留，但值编辑器从纯文本升级为 fude：

- 颜色属性：色块 + fude 输入框（默认 text segment）
- 枚举属性：下拉 + fude 输入框
- 数值+单位：数字 + 单位下拉 + fude 输入框
- opacity：滑块 + 数字 + fude 输入框
- 通用：纯 fude 输入框

fude 编辑器作为值输入的基础组件，替代原来的 `<input>` / `<select>`。

### 2.4 右侧元素属性面板（简化）

```
┌─ 元素属性 ───────────────────┐
│ <div> wrapper                │
│                              │
│ 📎 应用的类                   │
│  [×] .wrapper  [×] .card    │
│  [+ 关联 class...]           │
│                              │
│ ⚡ 内联样式                   │
│  --accent: [@accentColor  ]  │
│  padding:  [16px          ]  │
│  [+ 添加]                    │
└──────────────────────────────┘
```

- **应用的类**：从 StyleRegistry.classes 中选择，支持多选和拖拽排序
- **内联样式**：和 class 属性编辑器相同的 fude 输入
- 点击 class 名可打开 CSS 样式弹窗并定位到对应 class

---

## 三、codegen 编译映射

### 3.1 StyleRegistry → componentStyle

```
StyleRegistry.classes
  └→ componentStyle: Record<className, Record<propName, string>>

StyleRegistry.variables
  └→ componentStyle: Record<"_v-xxx", Record<"--xxx", string>>
     通过 variableToClassName() 生成 wrapper class
```

### 3.2 Segment → OmniLang 表达式

```
segments                                       编译输出
──────────────────────────────────────────────────────────────────
[{text,"16px"}]                                "16px"
[{signal,"accentColor"}]                       accentColor
[{text,"2px solid "}, {signal,"accentColor"}]  "2px solid " + accentColor
[{var-ref,"--primary"}]                        var(--primary)
[{text,"calc("}, {var-ref,"--w"}, {text," * 2px)"}]  "calc(" + var(--w) + " * 2px)"
```

**规则**：
- 相邻的不同类型 segment 之间自动插入 `+`
- 纯 text / 纯 signal / 纯 var-ref（单个 segment）不需要 `+`
- signal 在 OmniLang 中直接引用，编译器处理类型转换

### 3.3 ElementStyle → elementClassMap + elementInlineMap

```
element.style.classRefs
  └→ elementClassMap.set(elId, classRefs)

element.style.inlineProperties
  └→ elementInlineMap.set(elId, { propName: compiledValue })
```

### 3.4 CSS 变量信号注入

当 inline property 的 segments 包含 signal 类型时：

```
inline: { name: "--theme-bg", segments: [{type:"signal", name:"ThemeStore.currentBg"}] }
  → style: { --theme-bg: ThemeStore.currentBg }
  → 编译器生成: __omni_effect(() => el.style.setProperty("--theme-bg", ...))
```

### 3.5 影响范围

需修改的文件：

| 层 | 文件 | 改动 |
|----|------|------|
| 类型 | `core/types/style.ts` | 重构：新类型定义 |
| 类型 | `core/types/element.ts` | ElementData.style 更新 |
| 常量 | `constants/style-properties.ts` | 保留 property 元数据，去掉重复类型定义 |
| Store | **新建** `core/state/styleRegistry.ts` | 原子 store，StyleRegistry 的 canonical owner |
| Store | `core/state/styles.ts` | 删除（styleOperations） |
| Store | `store/editor.ts` | 删除旧 style 方法，新增 ElementStyle + StyleRegistry pass-through |
| 命令 | `core/commands/AddStyleEntry.ts` 等 | 删除旧命令，新增 11 个 StyleRegistry + ElementStyle 命令 |
| UI | `ui/panels/StyleEntryEditor.tsx` | 删除 |
| UI | `ui/panels/StylePropertyEditor.tsx` | 重构：值编辑器改为 fude |
| UI | `ui/inspector/sections/StyleSection.tsx` | 重写：简化为 classRefs + inline |
| UI | `ui/panels/PropertyPanel.tsx` | 更新 StyleSection 引用 |
| UI | `ui/shell/Toolbar.tsx` | 新增 [🎨 CSS 样式] 按钮 |
| UI | **新建** `ui/panels/StyleDesignModal.tsx` | 弹窗容器 |
| UI | **新建** `ui/panels/CssVariableEditor.tsx` | CSS 变量区域 |
| UI | **新建** `ui/panels/ClassCard.tsx` | 单个 class 卡片 |
| UI | **新建** `ui/panels/ClassPreview.tsx` | 左侧预览区 |
| UI | **新建** `ui/panels/StyleValueEditor.tsx` | fude 值编辑器 |
| Codegen | `services/codegen/collect-styles.ts` | 适配新数据模型 |
| Codegen | `services/codegen/source-emitter.ts` | StyleValueSegment 序列化 |
| Pipeline | `services/codegen/pipeline.ts` | parseStylesFromAST 适配 |
| Tests | `store/__tests__/style-store.test.ts` | 重写 |
| Tests | `core/commands/__tests__/style-commands.test.ts` | 重写 |

---

## 四、命令 / Undo 系统

### 4.1 新命令

```
AddCssVariable       — 添加 CSS 变量
RemoveCssVariable    — 删除 CSS 变量
UpdateCssVariable    — 更新 CSS 变量（name / segments）
AddStyleClass        — 添加类定义
RemoveStyleClass     — 删除类定义
RenameStyleClass     — 重命名类
AddClassProperty     — 给类添加属性
RemoveClassProperty  — 删除类属性
UpdateClassProperty  — 更新类属性
SetElementClassRefs  — 设置元素的 class 引用列表
SetElementInlineStyles — 设置元素的内联样式
```

### 4.2 删除的旧命令

```
AddStyleEntry        → AddStyleClass / AddCssVariable
RemoveStyleEntry     → RemoveStyleClass / RemoveCssVariable
UpdateStyleEntry     → UpdateClassProperty / UpdateCssVariable
ReorderStyleEntry    → SetElementClassRefs（classRefs 顺序即渲染顺序）
RenameStyleEntry     → RenameStyleClass
SetStyleEntryType    → 删除（不再有 type 切换）
UpdateStyleProperty  → UpdateClassProperty
```

---

## 五、Store 状态管理

### 5.1 架构原则

遵循现有「monolith + atom delegation」模式：
- `core/state/styleRegistry.ts` — 原子 store，StyleRegistry 的 canonical owner
- `store/editor.ts` — monolith store，提供 pass-through 方法并 syncToAtoms

### 5.2 StyleRegistry Atom Store

```typescript
// core/state/styleRegistry.ts

import { create } from "zustand";
import type { CssVariable, StyleClass, StyleProperty } from "../types/style";

interface StyleRegistry {
  variables: CssVariable[];
  classes: StyleClass[];
}

interface StyleRegistryState {
  /** 每个组件的样式注册表，key = component name */
  registries: Record<string, StyleRegistry>;

  // ── CSS 变量操作 ──
  addVariable: (componentName: string, variable: CssVariable) => void;
  removeVariable: (componentName: string, variableId: string) => void;
  updateVariable: (componentName: string, variableId: string, patch: Partial<CssVariable>) => void;

  // ── 类定义操作 ──
  addClass: (componentName: string, styleClass: StyleClass) => void;
  removeClass: (componentName: string, classId: string) => void;
  renameClass: (componentName: string, classId: string, newName: string) => void;

  // ── 类属性操作 ──
  addProperty: (componentName: string, classId: string, property: StyleProperty) => void;
  removeProperty: (componentName: string, classId: string, propertyId: string) => void;
  updateProperty: (componentName: string, classId: string, propertyId: string, patch: Partial<StyleProperty>) => void;

  // ── 查询 ──
  getRegistry: (componentName: string) => StyleRegistry;
  getActiveRegistry: () => StyleRegistry;
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
    if (!reg) return s;
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
    if (!reg) return s;
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
    if (!reg) return s;
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
    if (!reg) return s;
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
    if (!reg) return s;
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
    if (!reg) return s;
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
    if (!reg) return s;
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

  getActiveRegistry: () => {
    // 从 tabStore 获取 activeDocId → 推导 component name
    const { useTabStore } = require("../store/editor");
    const activeDocId = useTabStore.getState().activeDocId;
    const componentName = activeDocId?.startsWith("comp:")
      ? activeDocId.replace("comp:", "")
      : "App";
    return get().registries[componentName] ?? { variables: [], classes: [] };
  },
}));
```

### 5.3 Monolith Store 集成

在 `store/editor.ts` 中：

```typescript
// 导入
import { useStyleRegistryStore } from "../core/state/styleRegistry";

// EditorState interface 新增：
interface EditorState {
  // ... existing ...

  // ── Style Registry (pass-through to atom store) ──
  styleRegistry: StyleRegistry;
  // 以上为 getter，通过 active component name 计算

  // ── Element style (simplified) ──
  setElementClassRefs: (elementId: string, classRefs: string[]) => void;
  addElementClassRef: (elementId: string, className: string) => void;
  removeElementClassRef: (elementId: string, className: string) => void;
  setElementInlineStyles: (elementId: string, properties: StyleProperty[]) => void;
  addElementInlineProperty: (elementId: string, property: StyleProperty) => void;
  removeElementInlineProperty: (elementId: string, propertyId: string) => void;
  updateElementInlineProperty: (elementId: string, propertyId: string, patch: Partial<StyleProperty>) => void;
}

// create 函数内：
return {
  // ... existing ...

  // ── Style Registry getter ──
  get styleRegistry(): StyleRegistry {
    return useStyleRegistryStore.getState().getActiveRegistry();
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
    syncToAtoms();
  },

  addElementClassRef: (elementId, className) => {
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? {
              ...e,
              style: {
                ...e.style,
                classRefs: [...(e.style?.classRefs ?? []), className],
              },
            }
          : e
      ),
      isDirty: true,
    }));
    syncToAtoms();
  },

  removeElementClassRef: (elementId, className) => {
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? {
              ...e,
              style: {
                ...e.style,
                classRefs: (e.style?.classRefs ?? []).filter((c) => c !== className),
              },
            }
          : e
      ),
      isDirty: true,
    }));
    syncToAtoms();
  },

  setElementInlineStyles: (elementId, properties) => {
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? { ...e, style: { ...e.style, inlineProperties: properties } }
          : e
      ),
      isDirty: true,
    }));
    syncToAtoms();
  },

  addElementInlineProperty: (elementId, property) => {
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? {
              ...e,
              style: {
                ...e.style,
                inlineProperties: [
                  ...(e.style?.inlineProperties ?? []),
                  property,
                ],
              },
            }
          : e
      ),
      isDirty: true,
    }));
    syncToAtoms();
  },

  removeElementInlineProperty: (elementId, propertyId) => {
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? {
              ...e,
              style: {
                ...e.style,
                inlineProperties: (e.style?.inlineProperties ?? []).filter(
                  (p) => p.id !== propertyId
                ),
              },
            }
          : e
      ),
      isDirty: true,
    }));
    syncToAtoms();
  },

  updateElementInlineProperty: (elementId, propertyId, patch) => {
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? {
              ...e,
              style: {
                ...e.style,
                inlineProperties: (e.style?.inlineProperties ?? []).map((p) =>
                  p.id === propertyId ? { ...p, ...patch } : p
                ),
              },
            }
          : e
      ),
      isDirty: true,
    }));
    syncToAtoms();
  },
};
```

### 5.4 ElementData 类型更新

```typescript
// core/types/element.ts

import type { StyleProperty } from "./style";

interface ElementData {
  // ... existing fields ...
  style?: {
    classRefs: string[];
    inlineProperties: StyleProperty[];
  };
}
```

### 5.5 数据流图

```
┌─────────────────────────────────────────────────────────────┐
│                      useEditorStore (monolith)                │
│                                                              │
│  elements[i].style.classRefs          ← per-element 引用    │
│  elements[i].style.inlineProperties   ← per-element 内联    │
│                                                              │
│  styleRegistry (getter)               ← 代理到 atom store   │
├─────────────────────────────────────────────────────────────┤
│              useStyleRegistryStore (atom)                     │
│                                                              │
│  registries["App"].variables[]        ← 组件级 CSS 变量     │
│  registries["App"].classes[]          ← 组件级类定义        │
│  registries["MyCard"].variables[]     ← 自定义组件样式      │
│  registries["MyCard"].classes[]                                │
└─────────────────────────────────────────────────────────────┘

CSS 样式弹窗:
  useStyleRegistryStore → getActiveRegistry()
     → registries[activeComponentName]

元素属性面板:
  useEditorStore → elements[i].style
     → classRefs + inlineProperties

fude @ 触发:
  读取可用信号列表 ← useEditorStore.elements + wire bindings + stores

Codegen:
  collectStylesFromEntries()
     → 读取 useStyleRegistryStore.registries[componentName]
     → 读取 elements[].style
     → 拼装 componentStyle + elementClassMap + elementInlineMap
```

### 5.6 组件切换时的 StyleRegistry 行为

| 操作 | 行为 |
|------|------|
| 切换到 `comp:MyCard` tab | `getActiveRegistry()` 返回 `registries["MyCard"]` |
| 打开 CSS 样式弹窗 | 读取 active registry，展示当前组件的 class/variable |
| 在 `comp:MyCard` 中添加 `.card` | 写入 `registries["MyCard"].classes` |
| 切换回主画布 | CSS 弹窗展示 `registries["App"]`（主画布对应 "App"） |
| 关闭自定义组件 tab | 保留其 registry（不自动删除，允许 re-open 后恢复） |
| 保存项目 | 所有 registries 序列化到 .omni 文件的 `style { }` block |

### 5.7 删除的旧 store 方法

```
addStyleEntry()          → 拆分到 useStyleRegistryStore + element style 方法
removeStyleEntry()       → 同上
updateStyleEntry()       → 同上
reorderStyleEntries()    → setElementClassRefs (classRefs 顺序即顺序)
getStyles()              → element.style 直接读取
```

同时删除 `core/state/styles.ts` 中的 `styleOperations` 对象。

---

## 六、实现阶段

### Phase 1: 数据模型 + Store + 命令
1. 定义新类型（`core/types/style.ts` 重构，`core/types/element.ts` 更新）
2. 新建 `core/state/styleRegistry.ts`（原子 store）
3. 重构 `store/editor.ts`（删除旧 style 方法，新增 ElementStyle + StyleRegistry pass-through）
4. 删除 `core/state/styles.ts`（`styleOperations`）
5. 实现 11 个新命令（undo/redo）
6. 删除 7 个旧命令
7. 重写相关测试

### Phase 2: fude 值编辑器
1. 安装 `@tigerabrodioss/fude`
2. 实现 `StyleValueEditor`（封装 fude，配置 @ 触发信号列表、var( 触发变量列表）
3. 重构 `StylePropertyEditor`（值输入框替换为 fude）

### Phase 3: CSS 样式弹窗
1. UI 库安装和配置
2. 新建 `StyleDesignModal`
3. 新建 `CssVariableEditor`（变量列表）
4. 新建 `ClassCard`（卡片：预览 + 属性）
5. 新建 `ClassPreview`（tag 选择 + iframe 实时渲染）
6. Toolbar 按钮集成

### Phase 4: 右侧面板简化
1. 重写 `StyleSection` → 简化为 classRefs + inlineProperties
2. 更新 `PropertyPanel`

### Phase 5: Codegen 适配
1. 适配 `collect-styles.ts`（读取新数据模型）
2. 实现 `Segment` → OmniLang 表达式编译
3. 适配 `parseStylesFromAST`（.omni → 新数据模型）
4. 回归测试

### Phase 6: 测试与回归
1. 重写 store 测试
2. 重写命令测试
3. 端到端 codegen 测试（ThemeCssVarsDemo, CssVarsDemo, CssUnitsDemo 三个用例）
4. 构建验证

---

## 七、设计约束

1. **平台忠实性**：编辑器 UI 必须对应 OmniLang 实际编译产物。不创造编译器不支持的模式。
2. **SSOT**：class 和 variable 定义存在组件级注册表。所有引用同名的元素共享同一份定义。
3. **不可变性**：所有状态更新通过不可变操作（spread），通过 undo 系统执行。
4. **增量兼容**：Phase 1 完成后即可独立验证，后续 phase 可并行或顺序推进。
