# Editor Architecture Refactor Design

> 目标：架构清晰 + 易于开发，**功能不变、界面不变**。
> 纯架构重构，不改变任何用户可见行为。

---

## 1. 现状分析

### 1.1 代码量分布

```
editor/client/src:  ~25,000 行 | 116 文件 (TypeScript/React)
editor/server:       ~3,500 行 | 17 文件 (Python/FastAPI)
editor/runtime:      ~1,300 行 | 2 文件 (TypeScript)
editor/e2e:          ~1,300 行 | 8 文件 (Playwright)
```

### 1.2 前端顶层问题

| # | 问题 | 文件 | 行数 | 严重度 |
|---|------|------|------|--------|
| 1 | 上帝组件：模式切换、编译、保存/加载、运行/调试、剪贴板、快捷键、UI 渲染全部耦合 | App.tsx | 1,689 | 🔴 |
| 2 | 巨型 Store：元素/连线/样式/选择/项目/标签/多画布全部在一个 Zustand store | store/editor.ts | 830 | 🔴 |
| 3 | 巨型面板：属性编辑、样式、事件绑定混在一起 | panels/PropertyPanel.tsx | 1,362 | 🔴 |
| 4 | 巨型面板：树渲染、拖拽排序、内联编辑全部在一个组件 | panels/ComponentTree.tsx | 912 | 🔴 |
| 5 | 管线过长：编排、样式收集、组件展开、多文件输出全在一个函数 | omni-gen/pipeline.ts | 731 | 🟠 |
| 6 | 设计/代码双模式各自复制工具栏和布局 | App.tsx (render) | ~240 | 🟠 |
| 7 | omni-gen 直接 import Zustand stores，与 UI 层耦合 | omni-gen/*.ts | - | 🟠 |
| 8 | 19 个面板无统一接口模式 | panels/ | 7,002 | 🟡 |
| 9 | 后端单体：所有 API 路由在一个文件 | server/main.py | 1,096 | 🔴 |
| 10 | 类型定义分散在 4 个不同位置 | - | - | 🟡 |
| 11 | 组件创建默认 props 不统一：LayoutView 拖入用硬编码英文，ComponentTree 右键添加用 COMPONENT_PROTOCOL 中文 | LayoutView.tsx, ComponentTree.tsx | - | 🔴 |
| 12 | CATEGORY_LABELS/ORDER/ICONS 在两个面板中重复维护 | ComponentPalette.tsx, ComponentTree.tsx | ~60 | 🟡 |
| 13 | canvas/toast.ts 死代码：自定义 toast 实现无任何 import | canvas/toast.ts | 44 | 🟡 |
| 14 | 设计/代码模式 Toolbar JSX 各写一份（~120+120 行） | App.tsx | ~240 | 🟡 |
| 15 | componentName 识别正则 `/component\s+(\w+)\s*\{/` 在 6 处复制粘贴 | App.tsx(5), LayoutView.tsx(1) | - | 🟡 |
| 16 | 部分操作绕过 Undo：OmniDriver 和 StatePanel 直接调 store 不走命令模式，创建的元素无法撤销 | OmniDriver.ts, StatePanel.tsx | - | 🟡 |
| 17 | compilerCache.compileProject 在 3 处各自构造参数，componentName 提取方式不一致 | App.tsx, PropertyPanel.tsx, LayoutView.tsx | - | 🟡 |
| 18 | 自定义组件 view 连线静默丢失：saveComponentDefinitionsToServer 只传 elements 不传 wires | App.tsx, custom-component-emitter.ts | - | 🔴 |
| 19 | 主画布和自定义组件使用两套独立保存/加载机制（api.writeFile vs api.saveComponentDefinitions） | App.tsx | - | 🟠 |
| 20 | `@code-tab` 文件头命名错误：.omni 文件同时包含 view + code，但头只描述代码类型 | source-emitter.ts, omni-parser.ts, annotation/parser.py | - | 🟡 |
| 🔗 | 自定义组件不支持代码逻辑（chain）— 待深究后补充 | custom-component-emitter.ts | - | 🔗 |

### 1.3 功能不一致问题详查

审查发现 5 处"同一件事有多个做法"的问题，均需在重构中统一：

#### 1.3.1 组件创建默认 Props 不一致（🔴 严重）

`LayoutView.tsx` 的 `handleDrop`（从组件面板拖入画布）只有 4 个 tag 有硬编码默认值，且是英文：

```typescript
// LayoutView.tsx:185-189 — 仅覆盖 4 种组件
if (tag === "Text") props.label = "Text";
else if (tag === "Heading") props.label = "Heading";
else if (tag === "Button") props.label = "Button";
else if (tag === "Input") props.placeholder = "Input";
```

`ComponentTree.tsx` 的 `getDefaultProps`（右键→添加子组件）读取 `COMPONENT_PROTOCOL` 中所有组件的默认 props，且是中文占位：

```typescript
// ComponentTree.tsx:573-589 — 完整覆盖所有组件
function getDefaultProps(tag: string): Record<string, unknown> {
  const meta = COMPONENT_PROTOCOL[tag];
  // ... 读取 schema.default 并补充中文 placeholder
}
```

**影响：** 同一个 Button，从面板拖出显示 "Button"（英文），右键添加显示 "按钮"（中文）。

**解决方案：** 将 `getDefaultProps` 提取到 `core/component-defaults.ts`，LayoutView、ComponentTree、OmniDriver 三处统一调用。

#### 1.3.2 常量重复维护（🟡）

`CATEGORY_LABELS`、`CATEGORY_ORDER`、`COMPONENT_ICONS` 三份常量在 `ComponentPalette.tsx` 和 `ComponentTree.tsx` 中各维护一份完全相同的副本。新增组件/分类时需改两处。

**解决方案：** 收敛到 `core/constants/components.ts`，两处 import 同一份。

#### 1.3.3 canvas/toast.ts 死代码（🟡）

`canvas/toast.ts` (44 行) 定义了 `showToast`/`getToasts`/`TOAST_STYLES`，但**没有任何文件 import 它**。实际使用的是 `notifications/toast.ts`（基于 sonner 库）。

**解决方案：** 删除 `canvas/toast.ts`。

#### 1.3.4 OmniDriver 不设默认 Props（🟡）

`OmniDriver.ts:544-594` 创建 view 元素时 `props: cleanProps`（调用者传什么用什么），不合并 COMPONENT_PROTOCOL 的默认值。AI/脚本调用方必须自己知道每个组件的必填属性。

**解决方案：** OmniDriver 创建组件时也调用统一的 `getDefaultProps(tag)` 作为 props 基底。

#### 1.3.5 双模式 Toolbar 重复（🟡）

`App.tsx` 中设计模式和代码模式的工具栏 JSX 各写一份（~120 行 × 2），编译、运行、undo/redo 等按钮完全一致但被复制。

**解决方案：** 提取 `<EditorToolbar>` 共享组件到 `ui/shell/Toolbar.tsx`，模式特定按钮通过 children/插槽传入。

#### 1.3.6 componentName 识别正则 6 处复制（🟡）

同一个正则 `/component\s+(\w+)\s*\{/` 在以下位置各自复制：

| 文件 | 行号 | 函数 |
|------|------|------|
| App.tsx | 437 | handleCompile |
| App.tsx | 531 | handleCompile（同一函数内两处！） |
| App.tsx | 625 | handleRun |
| App.tsx | 677 | saveFileEntry |
| App.tsx | 800 | saveProject |
| LayoutView.tsx | 76 | recompile |

**解决方案：** 提取 `extractComponentName(source: string)` 到 `core/` 层工具函数，6 处调用统一入口。

#### 1.3.7 部分操作绕过 Undo 系统（🟡）

当前 Undo/Redo 基于命令模式。但以下路径绕过命令直接操作 store：

| 路径 | 方式 | 可撤销 |
|------|------|--------|
| 面板拖入 | `undoExec(new AddElement(el), ctx)` | ✅ |
| 右键添加 | `undoExec(new AddElement(el), ctx)` | ✅ |
| OmniDriver 添加元素 | `store.addElement(el)` | ❌ |
| OmniDriver 添加连线 | `store.addWire(w)` | ❌ |
| StatePanel 创建 signal | `ctx.addElement(el)`（不经过 Command.execute） | ❌ |
| StatePanel 创建 store | `ctx.addElement(el)` | ❌ |

StatePanel 注释写 "Use Zustand directly to avoid undo for initial creation"，但这导致这些"初始创建"节点在后续按 Ctrl+Z 时会残留。

**解决方案：** OmniDriver 和 StatePanel 的创建操作统一使用 `undoExec(new AddElement(el), ctx)`，与拖入/右键添加行为一致。

#### 1.3.8 ID 生成器分散实现（🟡）

`utils/id.ts:9` 已定义 `uid(prefix)`，但仍有 5+ 处自行实现 ID 生成：

| 位置 | ID 格式 |
|------|---------|
| `utils/id.ts` ✅ 官方 | `{prefix}-{random}` |
| `OmniDriver.ts:539` | `el-{Date.now()}-{random}` |
| `codegraph/store.ts:117` | `{Date.now().toString(36)}-{random}` |
| `BlockPalette.tsx:47` | `cb-{tag}-{Date.now()}-{seq}` |
| `pipeline.ts:143` | `s_{Date.now()}_{counter}` |
| `PropertyPanel.tsx:501` | `s_{Date.now()}` |
| `StatePanel.tsx:363,406` | `cb-{kind}-{Date.now()}-{random}` |
| `FunctionLibrary.tsx:201` | `cb-fn-{Date.now()}-{random}` |

**解决方案：** 统一使用 `utils/id.ts` 的 `uid(prefix)`。对于需要特殊前缀的场景（如 style entry ID `s_xxx`），扩展 `uid` 支持自定义前缀即可。

#### 1.3.9 主画布 vs 自定义组件：两套保存/加载机制（🔴🟠）

主画布和自定义组件使用完全不同的保存路径：

```
主画布保存:
  saveFileEntry() → generateOmniSource(forSave:true) → api.writeFile()
  .omni 源码包含：view + style + @chain 注解 + @edit 标记

自定义组件保存:
  exitComponentCanvas() → saveCanvas(elements, wires) → registry
  saveComponentDefinitionsToServer()
    ↓
  ❌ 只传 elements，wires 被丢弃！
    emitCustomComponents({ name, elements })  ← 参数无 wires
    ↓
  生成的 .omni 源码无连线信息
    ↓
  api.saveComponentDefinitions() → 服务端写入 src/components/Name.omni
```

加载同样两套：
```
主画布加载:  api.annotatedLoad() → parseOmniSource() → elements + wires + chains
自定义组件:  loadComponentDefinitionsFromServer() → parseOmniSource() → elements + wires
             ↑ 加载侧支持 wires，但保存侧从未写入
```

**核心问题：** `emitCustomComponents` 函数签名不接受 wires，保存时静默丢弃。对比格式设计文档（2026-06-21），规格明确要求主画布和自定义组件使用相同的 `.omni` 格式。

**解决方案：** 合并 `emitCustomComponents` 入 `generateOmniSource`。`generateOmniSource` 增加可选的 `componentMeta` 参数（含 props/events 声明），统一生成主画布和自定义组件的 .omni 源码。自定义组件也走 `api.writeFile` 保存。

#### 1.3.10 `@code-tab` 文件头命名不当（🟡）

格式设计文档规定每个 `.omni` 文件以 `// @code-tab type=logic|event` 开头。但这个命名有误导——`.omni` 文件同时包含 view（设计画布）和 code（代码画布），文件头只描述了代码部分。

**解决方案：** 重命名为 `// @omni-file type=logic|event`（或 `// @omni tab=logic`），明确表示这是整个文件的元数据，不是仅代码部分。

解析器同步更新：客户端 `omni-parser.ts:244`、服务端 `annotation/parser.py:22`。

#### 1.3.11 自定义组件代码逻辑（chain）— 待深究 🔗

详见 Chain 系统深度分析。当前 `emitCustomComponents` 传空 chains，自定义组件的代码画布无法持久化。修正方案待 chain 深究完成。

### 1.4 依赖关联项目

| 项目 | 关系 |
|------|------|
| `omni-compiler/` | Rust 编译器：Lexer→Parser→Semantic→IR→Codegen（9 targets），通过 WASM 和 Python DLL 两种方式调用 |
| `omnilang/` | Python 包：编译 API 封装，CLI，LSP，通过 rust_bridge 调用 Rust DLL |
| `vscode-extension/` | VS Code 插件：语法高亮 + 代码片段 |
| `tools/` | 辅助脚本 |
| `omni-test/` | 测试用 OmniLang 项目 |

---

## 2. 目标架构

### 2.1 分层模型

```
app/          ← 纯组装，~80 行，Provider 嵌套
  ↓
features/     ← 用户场景组合（design-mode, code-mode, project-explorer, debug）
  ↓
ui/           ← React 组件（shell, canvas, inspector, panels, editor, preview）
  ↓
services/     ← 纯业务逻辑（compiler, codegen, parser, project, runtime, api）
  ↓
core/         ← 微内核（types, state atoms, commands, events）
```

**依赖铁律：**
- `core/` — 零 UI 依赖，不 import React
- `services/` — 只依赖 `core/`，不 import React 组件
- `ui/` — 依赖 `core/` + `services/`
- `features/` — 依赖 `core/` + `services/` + `ui/`
- `app/` — 唯一的组合根，允许横向 import
- **零循环依赖**

### 2.2 物理结构

```
editor/client/src/
├── core/
│   ├── types/           # 全部共享类型（从各处收敛）
│   ├── constants/       # 共享常量（分类、图标、组件默认 props 工厂）
│   ├── utils/           # 纯工具函数（uid、extractComponentName、isOmniFile）
│   ├── state/           # 按域拆分的 Zustand atoms（含 codegraph）
│   ├── commands/        # 命令模式 (undo/redo)
│   └── events/          # 事件总线 (mitt)
│
├── services/
│   ├── compiler/        # 编译编排 + WASM 桥接
│   ├── codegen/         # omni-gen 管线（从 src/omni-gen 迁移）
│   ├── parser/          # 源码→视觉元素
│   ├── project/         # 保存/加载/项目管理
│   ├── runtime/         # Run/Debug 桥接
│   └── api/             # 按域拆分的 API 客户端
│
├── ui/
│   ├── shell/           # AppShell, Toolbar, StatusBar, ActivityBar
│   ├── canvas/          # design-canvas + code-canvas + shared
│   ├── inspector/       # PropertyPanel (拆分), CodePropertyPanel, StyleEditors
│   ├── panels/          # 所有面板（PanelPlugin 接口统一）
│   ├── editor/          # Monaco 集成
│   ├── preview/         # 预览 iframe
│   └── shared/          # ErrorBoundary, SearchModal, Toast
│
├── features/
│   ├── design-mode/     # 设计模式组装
│   ├── code-mode/       # 代码模式组装
│   ├── project-explorer/# 项目管理
│   ├── compiler/        # 编译/运行 hook
│   └── debug/           # 调试功能
│
└── app/
    ├── App.tsx          # ~80 行，纯组装
    └── main.tsx         # ~14 行，不变

editor/server/
├── main.py              # ~80 行，app 工厂
├── routers/             # 按域拆分的路由
├── services/            # 业务逻辑
├── models/              # Pydantic 模型
└── annotation/          # 注解处理（保持现有结构）
```

---

## 3. core/ 详细设计

### 3.1 类型收敛

所有共享类型统一在 `core/types/`：

| 当前位置 | 迁移目标 |
|----------|----------|
| `store/editor.ts` | `core/types/element.ts` (ElementData, PortData) |
| `store/editor.ts` | `core/types/wire.ts` (WireData) |
| `store/editor.ts` | `core/types/project.ts` (ProjectConfig) |
| `constants/style-properties.ts` | `core/types/style.ts` (StyleEntry, StyleProperty) |
| `codegraph/types.ts` | `core/types/codegraph.ts` (CodeTab, ChainBlock) |
| `annotation/types.ts` | `core/types/annotation.ts` |

### 3.2 State 拆分

当前 `store/editor.ts` (830 行) → 6 个独立 Zustand store：

| Store | 职责 | 估算行数 |
|-------|------|----------|
| `state/elements.ts` | 元素 CRUD、位置、props、reorder/move | ~270 |
| `state/wires.ts` | 连线 CRUD | ~100 |
| `state/selection.ts` | 选中状态、多选、全选 | ~80 |
| `state/styles.ts` | 样式条目 CRUD、重排 | ~150 |
| `state/project.ts` | 项目路径、名称、dirty、sourceCode | ~100 |
| `state/tabs.ts` | 多画布标签、tabStates、enter/exit 组件画布 | ~200 |

迁移到 `core/state/` 的其他 store：
- `store/undo.ts` — undo/redo 栈（93 行，不变）
- `store/componentRegistry.ts` — 自定义组件注册表（219 行）
- `codegraph/store.ts` — 代码图状态（728 行 4 文件 → `core/state/codegraph.ts`，已是合理大小，不拆分）

跨 store 操作（如"删除元素也删除关联连线"）通过 `core/state/index.ts` 的组合函数处理，或订阅事件总线。

### 3.3 事件总线

使用 **mitt**（200 字节，原生 TypeScript 支持）。

```typescript
// core/events/bus.ts
import mitt from 'mitt'

type EditorEvents = {
  'element:added': { elementId: string }
  'element:removed': { elementId: string }
  'element:selected': { elementIds: string[] }
  'wire:added': { wireId: string }
  'wire:removed': { wireId: string }
  'project:loaded': { path: string }
  'project:saved': { path: string }
  'compile:completed': { errors: CompileError[] }
  'mode:changed': { mode: 'design' | 'code' }
  'tab:switched': { docId: string | null }
}

export const editorBus = mitt<EditorEvents>()
```

用途：解耦面板之间的隐式依赖。例如 PropertyPanel 不需要知道 Canvas 的存在 — 它订阅 `element:selected` 事件即可。

### 3.4 命令模式

保持现有 17 个命令文件不变，迁移到 `core/commands/`。`CommandContext` 接口统一在 `core/commands/types.ts`。

### 3.5 共享常量 + 组件默认 Props 工厂

当前分散在三处的常量/函数统一收敛到 `core/constants/`：

```
core/constants/
├── components.ts     # CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS（单一份）
├── defaults.ts       # getDefaultProps(tag) — 组件默认 props 工厂
└── index.ts
```

**`getDefaultProps(tag)` — 统一入口：**

从 ComponentTree.tsx 的 `getDefaultProps` 提取（该实现已是正确的——读 COMPONENT_PROTOCOL 所有默认值 + 中文占位），统一为：

- `LayoutView.handleDrop` — 拖入时调用 `getDefaultProps(tag)` 替代硬编码 4 个 if-else
- `ComponentTree.handleAddChild` — 已有正确实现，改为 import 共享版本
- `OmniDriver.component()` — 调用 `getDefaultProps(tag)` 作为 props 基底，调用者传入的 props 覆盖默认值

这样三个入口创建的组件 props 完全一致。

### 3.6 共享工具函数

当前分散在多处的通用逻辑统一到 `core/utils/`：

```
core/utils/
├── id.ts              # uid(prefix) — 唯一 ID 生成器（从 src/utils/id.ts 迁移并扩展）
├── omni.ts            # extractComponentName(source) — 从 .omni 源码提取组件名
└── index.ts
```

**ID 统一：** 所有 6+ 处自实现 ID 生成替换为 `uid(prefix)` 调用。`uid` 扩展支持自定义前缀参数以覆盖现有场景（`s_`、`cb-`、`tab-` 等）。

**extractComponentName 统一：**
```typescript
// core/utils/omni.ts
export function extractComponentName(source: string, fallback = "App"): string {
  const match = source.match(/component\s+(\w+)\s*\{/);
  return match ? match[1] : fallback;
}
```
App.tsx 5 处 + LayoutView.tsx 1 处全部替换为调用此函数。

**Undo 统一：** OmniDriver 和 StatePanel 的 element/wire 创建操作改为使用 `undoExec(new AddElement(el), ctx)` 或等价的命令包装，确保所有用户可见的元素创建都可撤销。

---

## 4. services/ 详细设计

### 4.1 编译服务

从 App.tsx 的 `handleCompile` (~150 行) 和 `handleRun` (~80 行) 提取：

```typescript
// services/compiler/compiler.ts
export interface CompilerService {
  compile(options: {
    elements: ElementData[]
    wires: WireData[]
    codeTabs: CodeTab[]
    componentName: string
    target: string
    rootOrder: string[]
  }): Promise<CompileResult>

  compileAndRun(options: CompileOptions & { debug?: boolean }): Promise<RunResult>
}
```

### 4.2 codegen 管线拆分

当前 `pipeline.ts` (731 行) → 拆分：

```
services/codegen/
├── pipeline.ts              # 纯编排 (~100 行)
├── expand-custom-components.ts  # (~120 行)
├── collect-styles.ts        # (~150 行)
├── generate-omni-source.ts  # (~80 行)
├── generate-omni-files.ts   # (~100 行)
├── view-tree-builder.ts     # (74 行，不变)
├── wire-analyzer.ts         # (55 行，不变)
├── expression-lowering.ts   # (282 行，不变)
├── store-extractor.ts       # (130 行，不变)
├── chain-emitter.ts         # (263 行，不变)
├── style-resolver.ts        # (121 行，不变)
├── source-emitter.ts        # (276 行，不变)
├── omni-parser.ts           # (673 行，不变)
├── annotation-data.ts       # (64 行，不变)
├── import-collector.ts      # (34 行，不变)
├── token-emitter.ts         # (32 行，不变)
├── custom-component-emitter.ts  # (165 行，不变)
├── validator.ts             # (11 行，不变)
└── index.ts
```

关键改造：管线函数不再 import Zustand stores，通过参数接收所需数据。

### 4.2b 统一保存/加载管线（合并 emitCustomComponents）

当前 `emitCustomComponents` 是独立于 `generateOmniSource` 的简化版管线——不支持 wires、不支持 chain。重构后合并为统一管线：

```typescript
// services/codegen/generate-omni-source.ts
function generateOmniSource(options: {
  designElements: ElementData[]
  designWires: WireData[]        // ✅ 主画布和自定义组件都传入
  codeTabs: CodeTab[]
  componentName: string
  rootOrder: string[]
  forSave?: boolean
  componentMeta?: {              // ← 自定义组件额外参数
    props: Record<string, PropSchema>
    events: Record<string, EventSchema>
    category: string
    description: string
    styleProperties: string[]
    children: boolean
  }
}): StageResult<string>
```

- 自定义组件传入 `designWires` → wires 被正确编码到 `.omni` 源码
- 自定义组件传入 `componentMeta` → 生成 `prop`/`event` 声明
- 不传 `componentMeta` → 行为与当前主画布一致
- 废弃 `emitCustomComponents`，全部走统一管线

### 4.2c `@code-tab` 重命名为 `@omni-file`

| 位置 | 当前 | 改为 |
|------|------|------|
| source-emitter.ts | `// @code-tab type=logic` | `// @omni-file type=logic` |
| annotation-data.ts | `@code-tab` | `@omni-file` |
| omni-parser.ts:244 | 解析 `// @code-tab type=` | 解析 `// @omni-file type=` |
| annotation/parser.py:22 | `_CODE_TAB_RE` | `_OMNI_FILE_RE` |
| 格式设计文档 | `// @code-tab type=logic\|event` | `// @omni-file type=logic\|event` |

向后兼容：解析器同时接受 `@code-tab` 旧格式，新保存统一用 `@omni-file`。

### 4.3 项目管理服务

从 App.tsx 的 `saveProject` (~130 行)、`saveFileEntry` (~60 行)、`loadComponentDefinitionsFromServer` (~50 行)、`handleProjectReady` (~60 行) 提取：

```typescript
// services/project/project-service.ts
export interface ProjectService {
  saveProject(options: SaveOptions): Promise<SaveResult>
  loadProject(path: string): Promise<LoadResult>
  createProject(dir: string, name: string): Promise<CreateResult>
}
```

### 4.4 运行/调试服务

```typescript
// services/runtime/run-service.ts
export interface RunService {
  start(files: Record<string, string>, debug?: boolean): Promise<RunStartResult>
  stop(): Promise<void>
  getStatus(): Promise<RunStatus>
}

// services/runtime/debug-service.ts
export interface DebugService {
  connect(onMessage: DebugHandler, onStatus: StatusHandler): void
  disconnect(): void
  sendCommand(cmd: DebugCommand): void
}
```

### 4.5 API 客户端拆分

当前 `api/client.ts` (273 行) → 按域拆分：

```
services/api/
├── client.ts          # HTTP/WS 基础封装 (~100 行)
├── compile-api.ts     # 编译、parse-view、parse-codeblocks
├── project-api.ts     # 项目管理、文件系统
├── annotation-api.ts  # 注解加载/保存
└── index.ts
```

每个子模块是 `ApiClient` 上的方法分组，保持单一 `api` 实例。

### 4.6 services 层与 core 的合约

services 通过函数参数接收状态数据，不直接 import Zustand stores：

```typescript
// ❌ 当前模式
import { useEditorStore } from '../store/editor'
function compile() {
  const elements = useEditorStore.getState().elements
}

// ✅ 重构后模式
function compile(elements: ElementData[], wires: WireData[], ...): CompileResult
```

调用方（features/ 层的 hooks）负责从 store 读取数据并传入 services。

---

## 5. ui/ 详细设计

### 5.1 面板组织（PanelPlugin 延后）

当前 19 个面板各具差异（有的用 `ctx`，有的直接读 store，有的有独立内部状态）。
本次重构**只做文件迁移和目录重组**，不强制统一接口：

- 所有面板文件迁入 `ui/panels/`
- `ActivityBar.tsx` 保持现有硬编码的面板引用
- PanelPlugin 接口 + 注册表动态渲染 → **标记为后续 PR 的可选优化项**，不作为本次重构目标

后续 PR 中可引入 PanelPlugin（约 30 行接口 + 40 行注册表），收益是新增面板时无需修改 ActivityBar。

### 5.1b 共享 Toolbar 组件（解决双模式重复）

当前设计/代码模式的工具栏在 App.tsx 中各写一份（~240 行重复）。重构后：

```
ui/shell/
├── Toolbar.tsx              # 共享工具栏容器 + 通用按钮（编译、运行、undo/redo、save、目标选择器）
├── DesignToolbar.tsx        # 设计模式特有按钮（viewport、state preview、wire 模式、HUD、焦点顺序）
├── CodeToolbar.tsx          # 代码模式特有按钮（模板、连接器、自动布局）
└── index.ts
```

`DesignModeLayout.tsx` 和 `CodeModeLayout.tsx` 各组合 `<Toolbar>` + 各自的特定按钮。

### 5.2 巨型文件拆分

**PropertyPanel.tsx** (1,362 行 → 拆 4 部分):

```
ui/inspector/
├── PropertyPanel.tsx        # 主面板，~300 行
└── sections/
    ├── LayoutSection.tsx    # 布局属性（位置、尺寸、层级）
    ├── StyleSection.tsx     # 样式属性（→ 委托给 StyleEntryEditor）
    ├── EventSection.tsx     # 事件绑定（→ 委托给 EventCreationPanel）
    └── DataSection.tsx      # 数据绑定/组件 props
```

**ComponentTree.tsx** (912 行 → 拆 3 部分):

```
ui/panels/
├── ComponentTree.tsx        # 主组件，~200 行
├── TreeNodeRenderer.tsx     # 单节点渲染，~200 行
└── tree-utils.ts           # 拖拽、排序、展开/折叠逻辑，~150 行
```

### 5.3 App.tsx 瘦身路径

| 原始内容 | 行数 | 目标位置 |
|----------|------|----------|
| S 样式常量 | 30 | `ui/shell/theme.ts` |
| 设计模式 Toolbar JSX | 120 | `ui/shell/DesignToolbar.tsx` |
| 代码模式 Toolbar JSX | 120 | `ui/shell/CodeToolbar.tsx` |
| 共享 Toolbar 按钮 | 60 | `ui/shell/Toolbar.tsx` |
| 自定义组件保存/加载函数 | 100 | `services/project/component-service.ts` |
| handleCompile | 150 | `services/compiler/` + `features/compiler/useCompile.ts` |
| handleRun / handleStop | 100 | `services/runtime/` + `features/compiler/useRun.ts` |
| saveProject / saveFileEntry | 190 | `services/project/` + `features/project-explorer/` |
| handleProjectReady / openProjectFile | 120 | `features/project-explorer/useProjectLoader.ts` |
| 键盘快捷键 useEffect | 200 | `features/design-mode/useDesignKeyboard.ts` + `features/code-mode/useCodeKeyboard.ts` |
| 调试 WebSocket + handleDebugMessage | 120 | `features/debug/useDebugSession.ts` |
| 设计模式渲染 (JSX) | 120 | `features/design-mode/DesignModeLayout.tsx` |
| 代码模式渲染 (JSX) | 120 | `features/code-mode/CodeModeLayout.tsx` |
| OmniDriver 初始化 | 30 | `app/plugins/omni-driver-plugin.ts` |
| 项目初始化 useEffect | 60 | `features/project-explorer/useProjectInit.ts` |
| 编译 trigger effect | 60 | `features/compiler/useAutoCompile.ts` |
| 各种小 effect/state | 190 | 各自 feature hooks |
| **瘦身后 App.tsx** | **~80** | 纯 Provider 嵌套 + AppShell |

### 5.4 画布组织结构

```
ui/canvas/
├── design/
│   ├── DesignCanvas.tsx          # 设计画布主组件 (~200 行，新增)
│   ├── LayoutView.tsx            # 布局视图 (364→不变)
│   ├── CanvasPreview.tsx         # iframe 预览 (134→不变)
│   ├── CanvasTabBar.tsx          # 标签栏 (275→不变)
│   ├── ComponentCanvasHeader.tsx # (61→不变)
│   └── NodePalette.tsx           # (337→不变)
│
├── code/
│   ├── CodeCanvas.tsx            # 代码画布主组件 (~200 行，新增)
│   ├── FlowCanvas.tsx            # React Flow 画布 (437→不变)
│   ├── CodeTabBar.tsx            # 标签栏 (259→不变)
│   ├── CodeBlockNode.tsx         # 代码块节点
│   └── BaseNode.tsx              # 基础节点
│
└── shared/
    ├── FlowEngine.ts             # (182→不变)
    ├── edges/DataEdge.tsx        # (不变)
    ├── connectionValidators.ts   # (208→不变)
    ├── layoutUtils.ts            # (193→不变)
    ├── elementsToNodes.ts        # (66→不变)
    ├── wiresToEdges.ts           # (54→不变)
    └── socketTypes.ts            # (45→不变)
```

---

## 6. features/ 详细设计

每个 feature 是独立的 hook + Provider + 可选 UI 片段组合。

### 6.1 设计模式 feature

```typescript
// features/design-mode/
// DesignModeLayout.tsx — 设计模式的完整布局 (~120 行)
// useDesignKeyboard.ts — 设计模式快捷键 (~80 行)
//   复制粘贴 (ctrl+c/v/x/d)、方向键微移、ctrl+1/2/3 面板切换、Escape、全选
```

### 6.2 代码模式 feature

```typescript
// features/code-mode/
// CodeModeLayout.tsx — 代码模式的完整布局 (~120 行)
// useCodeKeyboard.ts — 代码模式快捷键 (~80 行)
```

### 6.3 项目管理 feature

```typescript
// features/project-explorer/
// useProjectInit.ts — 初始化加载项目
// useProjectLoader.ts — 加载/打开 .omni 文件
// useProjectSaver.ts — 保存项目（单文件 + 多文件）
// ProjectWelcome.tsx — 欢迎页（从 panels/ 迁移）
```

### 6.4 编译 feature

```typescript
// features/compiler/
// useCompile.ts — handleCompile hook，编排 services
// useRun.ts — handleRun/Stop hook
// useAutoCompile.ts — 文件变更自动编译
```

### 6.5 调试 feature

```typescript
// features/debug/
// useDebugSession.ts — WebSocket 连接 + 消息处理
// DebugOverlay.tsx — 调试面板 UI
```

---

## 7. Python 后端重构

### 7.1 路由拆分

`main.py` (1,096 行 → ~80 行)：

```
server/
├── main.py                   # FastAPI app 创建 + router 注册 (~80 行)
├── routers/
│   ├── compile.py            # /api/compile, /api/parse-view, /api/parse-codeblocks
│   ├── files.py              # /api/fs/list, /api/fs/read, /api/fs/write, /api/fs/list-directory
│   ├── project.py            # /api/project, /api/project/set-dir, /api/project/create
│   ├── annotations.py        # /api/annotated-save, /api/annotated-load, /api/editor-toml/*
│   ├── components.py         # /api/component-definitions/*
│   ├── connectors.py         # /api/connector/generate
│   ├── deploy.py             # /api/deploy
│   ├── run.py                # /api/run/*
│   └── debug.py              # /api/debug/*
├── services/
│   ├── compiler.py           # 编译编排逻辑
│   └── project_manager.py    # 项目管理逻辑
├── models/
│   └── requests.py           # 所有 Pydantic 请求模型集中定义
└── annotation/               # 保持现有，已良好组织
    ├── parser.py             # (279 行)
    ├── generator.py          # (211 行)
    ├── validator.py          # (306 行)
    ├── types.py              # (139 行)
    └── editor_toml.py        # (83 行)
```

---

## 8. 迁移策略

### 原则
- 每个 Phase 结束时：编译通过、所有测试通过、UI 不变
- 每次改动后立即验证，不积压问题
- 仅改 import 路径时，不改逻辑
- 提取逻辑时，1:1 复制，不优化

### Phase 1: 地基 — core/ 建立

| # | 步骤 | 风险 |
|---|------|------|
| 1.1 | 安装 mitt，创建 `core/events/bus.ts` | 低 |
| 1.2 | 创建 `core/types/`，逐个迁移类型定义，更新所有 import 路径 | 低 |
| 1.3 | 拆分 `store/editor.ts` → 6 个 state atoms，保持 API 兼容 | 中 |
| 1.4 | 迁移 `commands/` → `core/commands/`，更新 import | 低 |
| 1.5 | 迁移 `constants/style-properties.ts` → `core/types/style.ts` | 低 |
| 1.6 | 创建 `core/constants/`：提取 CATEGORY_LABELS/ORDER/ICONS + getDefaultProps，统一三处调用 | 低 |
| 1.7 | 删除死代码 `canvas/toast.ts`（确认无 import 后移除） | 低 |
| 1.8 | 创建 `core/utils/id.ts`：扩展 uid 支持自定义前缀，替换 6+ 处自实现 ID 生成 | 低 |
| 1.9 | 创建 `core/utils/omni.ts`：提取 extractComponentName，替换 App.tsx(5) + LayoutView(1) 共 6 处 | 低 |
| 1.10 | 修复 Undo 绕过：OmniDriver + StatePanel 创建操作改为命令模式 | 中 |
| 1.11 | 运行全部测试 + TypeScript 检查，验证通过 | - |

**验收标准：** `npm test` 全绿，`npm run build` 成功，编辑器 UI 不变。

### Phase 2: 服务层提取 — services/

| # | 步骤 | 风险 |
|---|------|------|
| 2.1 | 迁移 `omni-gen/` → `services/codegen/`，拆分 pipeline.ts | 中 |
| 2.2 | **合并 emitCustomComponents 入 generateOmniSource**：统一参数（增加 designWires + componentMeta），废弃独立管线 | 高 |
| 2.3 | **修复自定义组件 wires 丢失**：saveComponentDefinitionsToServer 传入 def.wires | 中 |
| 2.4 | **`@code-tab` → `@omni-file` 重命名**：source-emitter、omni-parser、annotation/parser.py 同步更新，向后兼容旧格式 | 中 |
| 2.5 | 拆分 `api/client.ts` → 按域的子模块 | 低 |
| 2.6 | 提取编译逻辑：App.tsx handleCompile → `services/compiler/` | 中 |
| 2.7 | 提取保存逻辑：App.tsx saveProject → `services/project/` | 中 |
| 2.8 | 提取运行逻辑：App.tsx handleRun/Stop → `services/runtime/` | 中 |
| 2.9 | 提取解析逻辑 → `services/parser/` | 低 |
| 2.10 | Python 后端拆分：routers/ + services/ + models/ | 中 |
| 2.11 | 🔗 自定义组件 chain 支持 — 待深究后插入 | - |
| 2.12 | 运行全部测试，验证通过 | - |

**验收标准：** `npm test` 全绿，`npm run build` 成功，UI 不变，`omnic serve` 正常启动。

### Phase 2.5: 安全网 — E2E 关键路径（新增）

> **为什么需要这个 Phase：** 当前测试覆盖率 ~16%，Phase 3 要动的大文件（PropertyPanel 1,362 行、ComponentTree 912 行、FlowCanvas 437 行等）全部没有单元测试。在动它们之前，必须用 E2E 建立安全网。

| # | 步骤 | 风险 |
|---|------|------|
| 2.5.1 | E2E: 拖入组件 → 验证属性面板显示正确属性 | 低 |
| 2.5.2 | E2E: 修改属性 → 验证画布预览更新 | 低 |
| 2.5.3 | E2E: 连线两个元素 → 验证连线渲染 | 低 |
| 2.5.4 | E2E: 切换设计/代码模式 → 验证布局完整性 | 低 |
| 2.5.5 | E2E: 保存项目 → 重新打开 → 验证画布内容恢复 | 低 |
| 2.5.6 | E2E: 组件树拖拽排序 → 验证顺序持久化 | 低 |
| 2.5.7 | 运行全部 E2E 测试，建立 baseline | - |

**验收标准：** 新增 6 个 E2E spec 全部通过，Phase 3 的改动有自动化兜底。

### Phase 3: UI 重组 + features 组装

| # | 步骤 | 风险 |
|---|------|------|
| 3.1 | 创建 `ui/shell/`，提取共享 Toolbar + DesignToolbar + CodeToolbar + StatusBar | 中 |
| 3.2 | 重组 `ui/canvas/` → design/ + code/ + shared/ | 中 |
| 3.3 | 重组 `ui/inspector/`，拆分 PropertyPanel.tsx（1,362→~300 行 + 4 sections） | 高 |
| 3.4 | 迁移所有面板文件到 `ui/panels/`（仅文件迁移，不改接口） | 中 |
| 3.5 | 迁移 `editor/` + `preview/` + `search/` → `ui/` | 低 |
| 3.6 | 创建 `features/` 层，提取所有 hooks | 高 |
| 3.7 | 瘦身 App.tsx → ~80 行纯组装 | 高 |
| 3.8 | 拆分 ComponentTree.tsx（912→~200 行 + TreeNodeRenderer + tree-utils） | 中 |
| 3.9 | 运行全部测试 + E2E 测试，验证通过 | - |

**验收标准：** `npm test` 全绿，E2E 全部通过，`npm run build` 成功，UI 完全不变。

---

## 9. 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 重构期间破坏现有功能 | 每步后运行全部测试；Phase 间可暂停、可回滚 |
| 测试覆盖率仅 ~16%，关键 UI 文件无单元测试 | Phase 2.5 新增 E2E 关键路径测试作为安全网；Phase 3 每步后跑 E2E |
| Store 拆分导致跨 store 操作遗漏（如 removeElement 同步清理 wires） | `core/state/index.ts` 组合函数集中管理跨 store 逻辑，Phase 1.3 完成后重点验证 |
| import 路径变更遗漏 | 使用 TypeScript 编译器检查所有引用 |
| Zustand store 拆分导致组件订阅失效 | 保持 selector 签名兼容，渐进迁移 |
| E2E 测试依赖内部 DOM 结构 | 使用语义化选择器（role、label），不依赖 className |
| Python 路由移动导致 API 404 | 保持路由路径完全不变，仅改变文件组织 |

---

## 10. 不做什么（明确排除）

- ❌ 不创建 monorepo / 多 npm package
- ❌ 不改变任何 UI 外观
- ❌ 不改变任何用户交互行为
- ❌ 不添加新功能
- ❌ 不修改 API 端点路径
- ❌ 不修改数据格式（.omni、.omni-editor.toml 等）
- ❌ 不修改 omni-compiler、omnilang、vscode-extension
- ❌ 不在这个重构中优化逻辑代码（除 1.3 节列出的不一致修复外，其余逻辑 1:1 复制，优化留给后续 PR）
- ✅ 1.3 节列出的所有不一致项：默认 props 统一、常量去重、死代码删除、OmniDriver 默认 props、Toolbar 提取、正则去重、Undo 统一、ID 生成器统一、wires 丢失修复、管线统一、`@code-tab` 重命名 — 这些是 bug 修复和去重，不改变用户可见界面
