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

### 1.3 依赖关联项目

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
│   ├── state/           # 按域拆分的 Zustand atoms
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

单独保留的 store：
- `store/undo.ts` — undo/redo 栈（93 行，不变）
- `store/componentRegistry.ts` — 自定义组件注册表（219 行，迁移到 `core/state/`）

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

### 5.1 PanelPlugin 接口

所有面板实现统一接口：

```typescript
// ui/panels/PanelPlugin.ts
interface PanelPlugin {
  id: string
  title: string
  icon: string
  category: 'design' | 'code' | 'both'
  defaultPosition: 'left' | 'right' | 'bottom'
  component: React.ComponentType<{ ctx: CommandContext; isVisible: boolean }>
}
```

面板注册表 `ui/panels/registry.ts`：

```typescript
export const panelRegistry: PanelPlugin[] = [
  { id: 'components', title: '组件', icon: '🧩', category: 'design', ... },
  { id: 'tree', title: '组件树', icon: '🌳', category: 'both', ... },
  { id: 'files', title: '文件', icon: '📁', category: 'code', ... },
  // ... 所有面板
]
```

ActivityBar 从注册表动态渲染，新增面板只需注册，不碰 ActivityBar 代码。

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
| 1.6 | 运行全部测试 + TypeScript 检查，验证通过 | - |

**验收标准：** `npm test` 全绿，`npm run build` 成功，编辑器 UI 不变。

### Phase 2: 服务层提取 — services/

| # | 步骤 | 风险 |
|---|------|------|
| 2.1 | 迁移 `omni-gen/` → `services/codegen/`，拆分 pipeline.ts | 中 |
| 2.2 | 拆分 `api/client.ts` → 按域的子模块 | 低 |
| 2.3 | 提取编译逻辑：App.tsx handleCompile → `services/compiler/` | 中 |
| 2.4 | 提取保存逻辑：App.tsx saveProject → `services/project/` | 中 |
| 2.5 | 提取运行逻辑：App.tsx handleRun/Stop → `services/runtime/` | 中 |
| 2.6 | 提取解析逻辑 → `services/parser/` | 低 |
| 2.7 | Python 后端拆分：routers/ + services/ + models/ | 中 |
| 2.8 | 运行全部测试，验证通过 | - |

**验收标准：** `npm test` 全绿，`npm run build` 成功，UI 不变，`omnic serve` 正常启动。

### Phase 3: UI 重组 + features 组装

| # | 步骤 | 风险 |
|---|------|------|
| 3.1 | 创建 `ui/shell/`，提取 AppShell, Toolbar（design+code）, StatusBar | 中 |
| 3.2 | 重组 `ui/canvas/` → design/ + code/ + shared/ | 中 |
| 3.3 | 重组 `ui/inspector/`，拆分 PropertyPanel.tsx | 高 |
| 3.4 | 重组 `ui/panels/`，引入 PanelPlugin 接口 + 注册表 | 中 |
| 3.5 | 迁移 `editor/` + `preview/` + `search/` → `ui/` | 低 |
| 3.6 | 创建 `features/` 层，提取所有 hooks | 高 |
| 3.7 | 瘦身 App.tsx → ~80 行纯组装 | 高 |
| 3.8 | 拆分 ComponentTree.tsx | 中 |
| 3.9 | 运行全部测试 + E2E 测试，验证通过 | - |

**验收标准：** `npm test` 全绿，E2E 全部通过，`npm run build` 成功，UI 完全不变。

---

## 9. 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 重构期间破坏现有功能 | 每步后运行全部测试；Phase 间可暂停、可回滚 |
| import 路径变更遗漏 | 使用 TypeScript 编译器检查所有引用 |
| Zustand store 拆分导致订阅失效 | 保持 selector 兼容，渐进迁移 |
| E2E 测试依赖内部 DOM 结构 | E2E 使用语义化选择器，不依赖 className |
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
- ❌ 不在这个重构中优化逻辑代码（逻辑 1:1 复制，优化留给后续 PR）
