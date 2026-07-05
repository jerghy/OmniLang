# Poiesis 重设计 —— 统一双向可视化编辑器

> 设计日期：2026-07-01
> 状态：草稿
> 目标：重新设计 poiesis，使其成为一个双向（代码 ↔ 可视化）同步的 OmniLang 编辑器，能够产出 learn-omniview 质量的代码。

---

## 1. 愿景

Poiesis 是一个面向 OmniLang 语言的**统一双向可视化编辑器**。用户在同一个工具中可以：

- 在画布上拖拽排列组件，所见即所得地设计 UI
- 在节点图中连线编排信号、计算属性、函数和事件的依赖关系
- 在 Monaco 编辑器中手写复杂逻辑代码
- 三种视图共享同一个 .omni 源码模型，任一视图的改动实时反映到其他视图
- 最终产出的 .omni 代码质量与手写代码无异

**核心原则：节点图是骨架，代码是肌肉，画布是皮肤。**

---

## 2. 核心编辑模型

### 2.1 双层画布（Z 轴纵深）

整个编辑器是一块全屏无限画布。画布有 Z 轴"表皮/骨骼"两层：

| 层 | 含义 | 显示内容 |
|----|------|---------|
| **上层（界面设计）** | "皮"——组件渲染后的样子 | 全屏画布，所有 .omni 组件渲染为真实 DOM |
| **下层（代码设计）** | "骨"——组件内部逻辑结构 | 节点图（signal、computed、fn、event 等节点 + 连线） |

**交互方式：**

- **默认**：上层全屏。所有组件显示渲染后的 UI 外观，画布可无限滚动/缩放
- **单击组件**：选中组件，右侧属性面板更新为该组件的可编辑属性（样式、事件绑定、props 等）。下层暂不浮现
- **双击组件/元素**：上层截取双击元素所属的组件（向上查找最近的 component 根节点），将该组件的 UI 预览缩小到画布一角，下层浮现——显示该组件的完整节点图。与选中组件**相关的节点高亮**，**不相关的代码节点变灰**。面包屑导航显示 `当前Tab > ComponentName`
- **再次双击**：收回下层，上层恢复全屏（退出"钻进"模式）

```
默认（上层全屏）:
┌──────────────────────────────────┐
│  ┌────────┐  ┌──────────────┐   │
│  │ Header  │  │   Counter     │   │  ← 真实 DOM 渲染
│  │         │  │ count:0       │   │
│  │         │  │ [+] [-]      │   │
│  └────────┘  └──────────────┘   │
│  ┌──────────────────────────┐   │
│  │       TodoList            │   │
│  └──────────────────────────┘   │
│                  3 items        │
└──────────────────────────────────┘

双击 Counter 后（上层截取 + 下层浮现）:
┌──────────────────────────────────┐
│  ┌────────┐                      │
│  │ Header  │  [灰色: 无关]        │
│  └────────┘                      │
│                                  │
│  ┌── Counter ──┐                 │  ← 上层截取缩小
│  │ count:0      │                 │
│  │ [+] [-]      │                 │
│  └──────────────┘                 │
│ ════════════════════════════════ │
│ 🟢 signal:count ────┐            │  ← 下层节点图浮现
│        │              │           │
│ 🔄 computed:doubled  │           │     相关节点高亮
│        │              │           │
│ 📦 fn:inc  fn:dec    │           │
│        │              │           │
│        ▼              ▼           │
│   [view: btn+]  [view: btn-]     │
│ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│
│ signal:items    灰色(无关)       │  ← TodoList 的节点灰掉
│ fn:handleAdd    灰色(无关)       │
└──────────────────────────────────┘

可选手势：触控板双指下滑 = 逐渐掀开下层，双指上滑 = 盖回上层
```

### 2.2 文件粒度：一个 Tab = 一个 .omni 文件

- 编辑器顶部是 Tab 栏，每个 Tab 对应一个 `.omni` 源文件
- 关闭 Tab = 关闭文件
- 双击进入组件内部时，仍在当前 Tab 内（面包屑导航），不新建 Tab
- 面包屑示例：`TodoPage.omni > Counter`，点击面包屑可回退

### 2.3 节点图 = 逻辑骨架

节点图使用 @xyflow/react（poiesis 已有），所有逻辑结构通过节点 + 连线表达：

**节点类型（按文件类型区分）：**

**Component 文件的节点：**

| 节点类型 | 对应 .omni 语法 | 端口 |
|---------|----------------|------|
| `signal` | `signal count: i32 = 0;` | out: value / in: set |
| `computed` | `computed doubled: i32 = count * 2;` | out: value / in: deps |
| `prop` | `prop items: Array<String> = [];` | out: value |
| `event` | `event onDelete: i32;` | out: emit |
| `fn` | `fn inc() { ... }` | in: params / out: return |
| `store-ref` | `@store TodoStore;` | out: store.getters |
| `view-element` | `<div class:box>...</div>` | in: bind:value / out: events |
| `branch` | `@if condition { ... }` | in: condition / out: true, false |
| `for-each` | `@for item in items { ... }` | in: collection / out: item, index |
| `slot` | `@slot header` | in: content |
| `style-block` | `style { .box { ... } }` | （属性面板编辑） |
| `motion` | `@motion trackedEnter { ... }` | out: trigger（后期） |
| `timeline` | `@timeline flyPath { ... }` | out: trigger（后期） |

**Store 文件的节点：**

| 节点类型 | 对应 .omni 语法 | 端口 |
|---------|----------------|------|
| `signal` | store 内 signal | out: value / in: set |
| `getter` | store 内 getter | out: value |
| `action` | store 内 action | out: call / in: params |

**Lib 文件的节点：**

| 节点类型 | 对应 .omni 语法 | 端口 |
|---------|----------------|------|
| `struct` | `struct Point { x, y }` | out: fields |
| `tagged-union` | `type Result = Ok \| Err` | out: variants |
| `type-alias` | `type Score = i32;` | out: base-type |
| `pub-fn` | `pub fn transform(...)` | in: params / out: return |

### 2.4 Monaco = 代码肌肉

复杂逻辑块通过 Monaco 编辑器直接编写：

- **fn 节点**：双击 → 下层节点图替换为 Monaco（节点图缩小为侧边缩略图），编辑 fn 代码体
- **style 块**：双击 style-block 节点 → Monaco 编辑 CSS
- **复杂表达式**：computed 节点的表达式、branch 节点的条件表达式，在属性面板中写短表达式，复杂逻辑提取为 fn

Monaco 提供：语法高亮、自动补全（基于项目内 signal/computed/fn/type 名称）、实时诊断（调用编译器 WASM）

---

## 3. 代码生成

### 3.1 原理

编辑器不"AI 生成"代码，而是**模板组装**代码。每种节点类型对应一个确定的 .omni 语法模板，连线决定变量名/参数绑定。

```
节点图                          →  生成的 .omni 代码

[signal: count: i32 = 0]        →  signal count: i32 = 0;
        │
[computed: doubled]              →  computed doubled: i32 = count * 2;
        │
[fn: inc] ──→ [view: btn+]      →  fn inc() { count = count + step; }
                                            ...
                                    <button @onPress={inc}>+</button>
```

### 3.2 生成质量保证

- 生成的 .omni 代码结构固定：imports → component/store → signals → computed → getters → actions/fns → view → style
- 缩进、空行、注释头由代码生成器统一管理
- fn 体代码由用户在 Monaco 中编写，保留用户原始格式
- 代码生成器同时是"反向解析器"——打开已有 .omni 文件时，解析回节点图

---

## 4. 项目结构

### 4.1 文件组织（遵循 learn-omniview 模式）

```
project/
├── main.omni                    ← 入口 + App 根组件（路由、全局 import）
├── stores/                      ← 全局状态管理
│   ├── TodoStore.omni
│   ├── ThemeStore.omni
│   └── ...
├── lib/                         ← 共享类型、工具函数
│   ├── types.omni
│   ├── api.omni
│   ├── validation.omni
│   └── ...
├── shared/                      ← 主题令牌、常量
│   ├── theme.omni
│   └── constants.omni
├── pages/                       ← 页面级组件
│   ├── Dashboard.omni
│   ├── TodoPage.omni
│   └── ...
└── components/                  ← 可复用 UI 组件
    ├── Counter.omni
    ├── TodoList.omni
    └── ...
```

### 4.2 脚手架

- 创建项目时自动生成 `main.omni` + 目录结构 + `.omnilang.toml`
- "新建 Component" → 自动在 `components/` 下创建文件，写入组件骨架代码，打开 Tab
- "新建 Store" → 自动在 `stores/` 下创建文件，写入 store 骨架代码
- "新建 Lib 模块" → 自动在 `lib/` 下创建文件

### 4.3 Import 管理（全自动）

- **拖入组件到画布**：自动在文件顶部添加 `import { ComponentName } from "./path/to/ComponentName";`
- **连接组件到 Store**：自动添加 `@store StoreName;` 和对应 import
- **在 fn 代码中使用 std 函数**：Monaco 自动补全触发时，自动添加 `import { fn } from "std/module";`
- **删除最后一个引用**：某个 import 不再被任何节点/连线引用时，自动移除
- Monaco 中可以手动编辑 import 语句，手动编辑的 import 同样被编辑器识别和保留

---

## 5. 编辑器布局 & Tauri 多窗口

### 5.1 默认布局

```
┌──────────────────────────────────────────────────┐
│  工具栏（编译/运行/撤销/重做/缩放/模式切换）       │
├──────────┬────────────────────────┬──────────────┤
│ 文件树    │                        │  属性面板     │
│ (可折叠)  │    画布区域             │  (根据选中    │
│          │    (双层画布)           │   元素变化)   │
│ 📁 stores │                        │              │
│ 📁 lib   │                        │  选中: Counter │
│ 📁 pages │                        │  ──────────── │
│ 📁 comps │                        │  props: ...   │
│          │                        │  class: ...   │
│          │                        │  events: ...  │
│          │                        │  style: ...   │
├──────────┴────────────────────────┴──────────────┤
│  状态栏（编译状态 / 错误数 / 项目名 / 脏标记）      │
└──────────────────────────────────────────────────┘
```

### 5.2 Tauri 多窗口能力

所有面板都是**可分离（detachable）**的：

- 文件树 → 拖出成独立窗口
- 属性面板 → 拖出成独立窗口
- Tab → 拖出成独立窗口（多个 .omni 文件在不同屏幕上编辑）
- Monaco 编辑器 → 拖出成独立窗口（编辑 fn 代码体时）

实现方式：Tauri 的 `WebviewWindow` API，每个 detachable 面板对应一个 webview。通过 Tauri 事件系统（emit/listen）跨窗口通信，共享同一个 Zustand store（主窗口持有，子窗口通过 IPC 读写）。

---

## 6. 数据模型

### 6.1 统一数据存储

整个编辑器共享**一个** Zustand store（重构自当前的 useEditorStore / useCodeGraphStore 双 store）：

```typescript
interface EditorState {
  // 项目
  projectDir: string | null;
  projectName: string;
  activeFilePath: string | null;
  
  // 当前打开的文件（Tab）
  openTabs: TabData[];
  activeTabId: string;
  
  // 当前 Tab 的数据
  elements: ElementData[];    // 节点 + 视图元素的统一定义
  wires: WireData[];          // 连线
  rootOrder: string[];        // 顶层元素排列顺序
  
  // 全局
  imports: ImportEntry[];     // 自动管理的 import 列表
  stores: StoreRef[];         // 项目内可用的 Store 列表
  types: TypeDefinition[];    // 项目内定义的类型（struct/union/alias）
  
  // 编辑器状态
  selectedElementIds: string[];
  drillTarget: string | null; // 当前"钻进"的组件 ID（null = 上层全屏）
  
  // 编译器
  compileErrors: CompileError[];
  wasmReady: boolean;
}
```

### 6.2 ElementData（统一元素定义）

```typescript
interface ElementData {
  id: string;
  nodeType: "view" | "codeblock";  // view = UI 元素, codeblock = 逻辑节点
  tag: string;                      // signal | computed | fn | div | button | ...
  props: Record<string, unknown>;
  children: string[];               // 子元素 ID 列表
  parentId: string | null;
  ports: PortData[];
  position: { x: number; y: number };
  source: string;                   // fn 节点的代码体
  style: Record<string, string>;    // 内联样式
  cssClasses: string[];             // class 列表
  componentRef: string | null;      // 引用的外部组件名
  isFolded: boolean;
}
```

### 6.3 WireData（连线）

```typescript
interface WireData {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  // 连线类型由端口类型决定：data = 数据流, event = 事件绑定, bind = 双向绑定
}
```

---

## 7. 技术栈

| 层 | 技术 | 说明 |
|----|------|------|
| 桌面框架 | Tauri v2 | 多窗口、文件系统、原生能力 |
| 前端框架 | React 19 + TypeScript | UI 层 |
| 状态管理 | Zustand | 统一 store |
| 节点图 | @xyflow/react（已有） | 逻辑节点编辑 |
| 代码编辑器 | Monaco Editor（已有） | fn 代码体编辑 |
| 编译器 | omni WASM（已有） | .omni → HTML/CSS/JS 编译 |
| 画布渲染 | iframe + srcdoc（已有） | 组件实时预览 |
| 拖拽 | @dnd-kit（已有） | 组件面板 → 画布 |
| 撤销/重做 | Command 模式（已有） | 所有操作可撤销 |
| 样式 | Tailwind CSS 或自定义 | 编辑器 UI 样式 |

---

## 8. 与现有 Poiesis 的差异

| 维度 | 当前 Poiesis | 新 Poiesis |
|------|-------------|-----------|
| 模式 | Design / Code 两个独立模式 | 统一画布，双层 Z 轴 |
| 数据存储 | 两个独立 store | 一个统一 store |
| 代码同步 | 编译器单向桥接 | 模板组装，双向实时 |
| 文件粒度 | 一个组件多 tab（main tab + 子 tab） | 一个 tab = 一个 .omni 文件 |
| Store 编辑 | 无专门的 Store 编辑 | Store 节点系统（signal/getter/action） |
| Import 管理 | 手动 | 全自动 |
| 项目结构 | 无脚手架 | 半自动模板 |
| 动画编辑 | 无 | 后期扩展（motion 时间轴轨道编辑器） |

---

## 9. 待定 & 后期扩展

以下设计维度已在本轮讨论中涉及，但具体方案不在当前范围内：

### 9.1 动画编辑器（后期扩展）

- Motion 和 Timeline 节点可展开为时间轴轨道编辑器
- 关键帧可视化编辑（类似 After Effects）
- 缓动曲线选择器
- 节点图已预留 motion/timeline 节点类型，不影响当前架构

### 9.2 样式编辑器

- CSS 属性面板（调色板、数值滑块、渐变编辑器）
- 设计 Token 系统（shared/theme.omni）
- 类选择器管理（新建/删除/重命名 CSS 类）

### 9.3 类型系统编辑器

- struct 字段表格编辑
- tagged union 变体编辑
- 类型验证（连线时检查端口类型兼容性）

### 9.4 测试生成

- 从组件逻辑自动生成 Playwright 测试骨架
- 可视化定义测试场景

---

## 10. 设计决策记录

| 决策 | 选择 | 理由 |
|------|------|------|
| 画布方案 | 双层 Z 轴（表皮/骨骼） | 无模式切换，空间关联直观，双击 = "钻进/钻出" |
| 代码编辑 | Monaco 嵌入（非弹窗） | 弹窗遮挡 UI 预览；嵌入下层保持对照 |
| 数据模型 | 统一 store | 消除双 store 同步问题，所有视图共享同一数据源 |
| 节点图库 | 继续使用 @xyflow/react | 已有实现，生态最大，TypeScript 支持好 |
| 代码生成 | 模板组装（非 AI） | 确定性输出，质量和手写代码等价 |
| Import 管理 | 全自动 | 减少手动劳动，消除 import 遗漏/冗余 |
| 项目结构 | 半自动脚手架 | 引导最佳实践，但保留灵活性 |
