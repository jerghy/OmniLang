# Poiesis 架构详解

## 一、整体分层

```
┌─────────────────────────────────────────────────┐
│  Pages (ArchitecturePage / DesignPage)           │  ← 顶层路由
├─────────────────────────────────────────────────┤
│  Components (canvas / design / panels / shell)   │  ← UI 组件层
├─────────────────────────────────────────────────┤
│  Hooks / Features / Driver                       │  ← 业务逻辑 + 自动化 API
├─────────────────────────────────────────────────┤
│  Services (codegen / parser / compiler)          │  ← 核心服务层
├─────────────────────────────────────────────────┤
│  Store (Zustand PoiesisState)                    │  ← 统一状态管理
├─────────────────────────────────────────────────┤
│  Core Types (element / design / style / wire)    │  ← 类型定义
└─────────────────────────────────────────────────┘
         ↕ IPC (invoke_command / events)
┌─────────────────────────────────────────────────┐
│  Tauri Rust Backend (lib.rs)                     │
│  ├─ compile_project → omnilang crate             │
│  │   ├─ discovery::discover_omni_sources()       │
│  │   ├─ Target::from_str() 解析目标              │
│  │   └─ compile_multi_internal_cached() 增量编译  │
│  ├─ read_file / write_file (sanitize_path)       │
│  ├─ save_project / open_project                  │
│  └─ sync_store_state / get_store_snapshot        │
├─────────────────────────────────────────────────┤
│  OmniLang WASM (OmniCompiler)                    │  ← 浏览器内编译
│  ├─ compile_to_omniview / compile_to_omniview_full│
│  ├─ compile_multi_to_omniview                    │
│  ├─ resolve_styles / resolve_styles_multi         │
│  └─ wrap_debug_html (v2026-07-12 新增)           │
└─────────────────────────────────────────────────┘
```

## 二、两大页面

### ArchitecturePage（架构页面）

**默认首页。** Obsidian 风格泡泡关系图谱 + 应用完整渲染预览。

```
ArchitecturePage
├─ 画布 (双层，按钮切换)
│   ├─ 上层: 整个应用的完整渲染（含样式、@if 渲染，不含 Store 逻辑）
│   └─ 下层: 泡泡节点图
│       ├─ 整体架构模式 (overall): 只显示全局 Store + 全局代码 + 界面
│       └─ 局部架构模式 (local): 右键组件 → "以此组件为中心"
│           ├─ 🔵 接口层: props / events
│           ├─ 🟢 组合层: 父/子组件
│           ├─ 🔴 依赖层: Store 数据读写
│           └─ ⚪ 内部层: signal / computed / fn / style
└─ 右侧面板: 选中泡泡属性摘要
```

泡泡展开逻辑：三层泡泡体系（界面 / Store / 代码），层层展开直到最底层（含基础 HTML 元素）。双击最底层泡泡 → 进入 DesignPage 对应 `.omni` 文件。

### DesignPage（设计页面）

**标签页编辑器。** 每个 Tab = 一个 `.omni` 文件。

```
DesignPage
├─ TabBar: design.omni | main.omni | MyCard.omni | [+ 添加]
├─ 左侧面板
│   ├─ ComponentPalette: 可拖拽组件列表（div/button/input + 自定义）
│   └─ ComponentTree: 当前文件元素层级树
├─ 中央画布（双模式，visibility 切换，两个组件始终挂载）
│   ├─ 设计模式 (codeFlowVisible=false)
│   │   └─ InfiniteCanvas + ZoomControls (WASM 编译 + 渲染)
│   └─ CodeFlow 模式 (codeFlowVisible=true)
│       ├─ TopLayer display:none 完全隐藏（非 scale 缩小）
│       │    设计预览由 DesignPreviewNode 独立提供，不需双份
│       ├─ CodeFlowCanvas 覆盖层
│       │   ├─ 4 节点: fn / code-block / condition / return（纯控制流连线）
│       │   ├─ GraphCompiler: 保存时 图 → fn body .omni 源码（含 // @pid:xxx）
│       │   ├─ GraphParser: 加载时 .omni 源码 → 图（14 tests, graph-parser.test.ts）
│       │   └─ DesignPreviewNode: 设计缩略图，读 store.previewOutput
│       │       └─ 宽高比从 useCanvasStore.devicePreset 动态计算，同步 DeviceSelector 选择的设备尺寸
└─ 右侧面板 (ContextInspector)
    ├─ 属性: ID / label / 自定义 props
    ├─ 🔗 双向绑定: bind:value prop 选择 + signal 搜索 (仅 input/textarea/select)
    ├─ Class 绑定: 搜索/绑定/解绑 design.omni 中的 class
    ├─ Variant 绑定: 维度选择器 (如 color=[danger] size=[lg])
    ├─ 内联样式: 所有卡片始终可见，属性级自锁——空值灰显(锁)/有值亮起+×清除(token绑定)
    └─ 事件绑定
```

### Design Tab (design.omni 专用编辑器)

当打开 `design.omni` 时，中央区域切换为设计系统编辑器：

```
DesignTabContent
├─ DesignTabBar: [Tokens] [Classes] [Variants]
├─ TokenEditor: 按类别分组 (colors/spacing/typography) 的 token 列表
├─ ClassEditor: 属性卡片 (Size/Bg/Border/Radius/Spacing) + pseudo states
└─ VariantEditor: 维度管理 + 分支标签 + 属性卡片 + 实时预览
```

## 三、核心数据流

### 3.1 .omni 往返闭环

```
┌──────────┐    parser.ts          ┌──────────┐
│ .omni    │ ──────────────────→   │  Store   │
│ (磁盘)    │                       │ (内存)    │
│          │ ←──────────────────   │          │
└──────────┘    pipeline.ts        └──────────┘
               + emitter.ts

解析方向:
  .omni 文本 → omni-tokenizer → omni-parser → parser.ts 递归下降
  → ElementData[] (元素树) + WireData[] (连线)
  → DesignDefinition (如果是 design.omni)
  → StoreDefinition (如果是 store 文件，`parser-types.ts` + `parseStoreBody()` L503+)
  → ComponentDef (如果是组件文件)

生成方向:
  ElementData[] → view-tree-builder → component-builder
  → source-emitter.ts (组件 .omni)
  DesignDefinition → design-emitter.ts (design .omni)
  StoreDefinition → source-emitter.ts `emitStore()` (store .omni, 含 // @pid:xxx + CodeFlow action body)
  → pipeline.ts 统一输出 → write_file 写磁盘
```

### 3.1.1 Store 数据流

Store 有**两种形态**：全局 Store（独立文件 `src/stores/*.omni`）和内联 Store（写在 `component { }` 内部）。

#### 全局 Store

Store 文件 (*.omni) 的往返闭环和 component/design 一样，但走独立的类型和解析路径：

```
项目启动 (App.tsx L69-97):
  read_file → parseOmniSource() → parseResult.storeDef
  → loadStoreDef(def) → storeDefs[name]
  → registerStorePids(def) → pidIndex

打开 Store Tab (StorePage):
  storeDefs[name] → 渲染 StoreMembersPanel
  → 选中 action → 将 action.elements/wires 写入 store.elements/wires
  → FlowCanvas 渲染（零改动，始终读 store.elements/wires）

编辑 Action (FlowCanvas + FlowInspector):
  右侧 FlowInspector 选中 FN/Return 节点 → setProp 修改元素
  → StorePage useEffect 实时同步 fn name/params → StoreActionDef

保存:
  storeDefs[name] → source-emitter.emitStore() → .omni 含 @pid
  → action body: compileCodeFlowGraph() → 含 @pid 分片
```

**关键点**：`StoreActionDef` 不直接存 `elements/wires` 在 Zustand 顶层——只有当前选中的 action 数据被 swap 到 `store.elements/wires`，FlowCanvas 零改动即渲染。切换 action 时写回 `storeDefs[name].actions[pid]`。

#### 内联 Store（2026-07-11 新增）

在组件标签页内通过右侧面板的 `📦 Store` 按钮打开抽屉，可直接编辑内联 Store——保存在组件 `.omni` 文件内部。状态存储在 `inlineStoreDefs: Record<string, StoreDefinition>`，Key 格式 `"ComponentName.StoreName"`（如 `"main.TestStore"`）。

```
解析:
  parser.ts parseComponentKeyword() → case "store"
  → _parsingInlineStore=true → pushState(IN_STORE) → parseStoreBody()
  → 存入 ctx.inlineStoreDefs["ComponentName.StoreName"]
  → 通过 parser-finalize.ts → ParseResult.inlineStoreDefs

发射:
  source-emitter.ts emitComponent() 内 → emitInlineStore()
  → 在 @store refs 之后、prop 之前发射 store Name { ... } 块
  → store 名去掉 "ComponentName." 前缀（已嵌套在 component 内）

加载 (3 个路径，漏一个就会丢数据):
  1. App.tsx L69-97: 项目启动批量解析 → parseResult.inlineStoreDefs → loadInlineStoreDef
  2. useProjectInit.ts L199: 加载 main.omni → parseResult.inlineStoreDefs → loadInlineStoreDef
  3. DesignTabBar.tsx L201-214: 打开组件文件 → parseResult.inlineStoreDefs → loadInlineStoreDef

⚠️ 关键陷阱: Parser 用源码中的 component 名（可能是 PascalCase，如 "Main"）
  做 key 前缀，但 Zustand 用 tab 的 componentName（来自文件名，如 "main"）。
  加载时必须 re-key: 提取 store 短名 + 用 componentName 重建 key。
  (见 confusions.md §二十四)

保存:
  useSaveProject.ts: 收集 inlineStoreDefs 中匹配当前 componentName 前缀的 store
  → 作为 inlineStores 参数传入 generateOmniFiles → pipeline → emitComponent
```

#### 组件 prop/signal/computed 数据流（2026-07-12 新增）

组件级声明（prop/signal/computed）的数据源是 `componentDefs[name]`，通过 ComponentMembersPanel 编辑。

```
解析 (.omni → componentDefs) — 3 个路径，漏一个都会丢数据：
  1. App.tsx L238-254: 项目启动批量解析 → ParseResult.props/signals/computeds
     → 对 main: 不设 elements/wires/rootOrder（main 不是自定义组件）
     → 对非 main: 保留 existing 数据优先，无 existing 时从 ParseResult 取
  2. useProjectInit.ts L126-143, L218-235: 加载 main.omni → ParseResult → setComponentDef("main", ...)
  3. component-service.ts L106-119: 自定义组件注册加载 → ParseResult → merged[name]

编辑 (ComponentMembersPanel):
  componentDefs[name].props/signals/computeds → 面板三折叠区
  → CRUD 通过 setComponentDef(name, { ...def, ...updates })

发射 (componentDefs → .omni):
  component-builder.ts buildComponentPart() → getComponentDefinitions()[name]
  → ComponentPart.props/signals/computeds → source-emitter emitComponent()
  → prop <name>: <type> = <default>;
  → signal <name>: <type> = <default>;
  → computed <name>: <type> = <expression>;

⚠️ 保存陷阱: useSaveProject.ts L165-178 会用 elements/wires 覆盖 componentDef，
必须保留已有 props/signals/computeds (L170-173: currentDef.signals || [])。
```

### 3.1.3 bind:value 双向绑定数据流（2026-07-13 新增）

ViewElementData 新增 `bindings?: Record<string, string>` 字段，存储 `{ propName: signalName }` 映射（如 `{ "value": "newTodo" }`）。

```
解析 (.omni → element.bindings):
  parser-view.ts L330-339: 遇到 bind:xxx={yyy} → result.bindings[bindName] = bindValue
  parser-helpers.ts createElement() L121-134: attrs.bindings → element.bindings
  (bind:xxx 不再进入 element.props，与普通 props 分离)

UI (BindingsSection.tsx):
  ContextInspector.tsx → <BindingsSection element={el} signalItems={...} />
  仅 input/textarea/select 显示。Prop 下拉 + SignalPickerPopover (分组搜索)
  → store.setElementBinding(id, propName, signalName | null)
  → element-crud.ts L100-116: 更新 element.bindings，含 undo 支持

发射 (element.bindings → .omni):
  pipeline.ts Phase C (L210-221): 遍历 designElements
  → 合并 element.bindings 到 elBindings Map (导线优先，elements 补充不重叠项)
  → source-emitter.ts L654-661: 读取 elementBindings → emit ` bind:${prop}={${sig}}`
```

⚠️ 关键: 导线绑定优先于 element.bindings —— wire-analyzer 先写入 elBindings，element.bindings 只补充尚未存在的条目。这意味着通过导线连接的 signal→view 覆盖同名 element 绑定。

涉及文件:
- `core/types/element.ts` — ViewElementData.bindings
- `parser-types.ts` — ViewElementAttrs.bindings
- `parser-view.ts` L330-339 — 解析 bind: 语法
- `parser-helpers.ts` L121-134 — createElement 填充 bindings
- `pipeline.ts` L210-221 — Phase C 合并
- `source-emitter.ts` L654-661 — 发射
- `store/slices/element-crud.ts` L100-116 — setElementBinding
- 新增 `inspector/BindingsSection.tsx` — UI

### 3.2 PID 系统

```omni
// @pid:c1            ← 组件级 PID
component Counter {
    // @pid:c1_n1     ← signal PID
    signal count: i32 = 0;
    // @pid:c1_n2     ← view PID
    view { <div>{count}</div> }
}
```

- PID 是**编辑器私有标记**，不影响编译
- 格式：组件 PID = `c1`，子元素 PID = `c1_n1`（组件 PID + `_n` + 序号）
- **作用**：保证往返稳定性——同一元素多次解析/生成保持相同 PID
- `pidIndex: Record<string, PidEntry>` 维护 PID → 文件 + 位置的全局映射
- `verify_pid_roundtrip` Rust 命令验证 PID 一致性

### 3.3 Tab 数据管理

```
用户切换 Tab 时:
  1. 当前 Tab 的 elements/wires/rootOrder/selectedElementIds
     → 写入 tabData[currentTabId]
  2. 目标 Tab 的数据
     → 从 tabData[targetTabId] 恢复到 store 顶层字段
  3. 如果目标 Tab 从未被访问过 (tabData 中无条目)
     → 从 .omni 源文件重新 parser 解析

LRU 淘汰:
  - 最多同时缓存 N 个 Tab 的数据（默认 10）
  - 超限时淘汰最久未访问的
  - 被淘汰的 Tab 再次切换时重新解析
```

### 3.4 编译路径（双路径详解）

**路径 1: 画布预览 (WASM, 编辑器内)**

```
触发: 元素/样式变更（防抖 150ms，useShallow 检测 elements 引用变化）
  Store → useWasmCompile useEffect (useWasmCompile.ts:48-155)
  → generateOmniFiles(elements) → .omni sources (含 style: { } 内联样式)
    ├─ buildViewTree → collectStylesFromEntries → elementInlineMap
    └─ SourceEmitter 发射 style: { key: value } 到 .omni 源
  → rustCompiler.resolveStylesMulti(sources, pathMapping) (rustCompiler.ts:335)
    └─ WASM 端自动调用 derive_path_mapping(&sources) 补全映射，
       TS 端只需传 {} 即可（useWasmCompile.ts L115-117, rustCompiler.ts L320-327）
  → 返回 { design_css, component_css, element_tree, element_classes, nid_map }
  → ★ 注入内联样式 (useWasmCompile.ts:135-157):
      从 elements 的 inlineProperties 构建 Map<elementId, styles>
      → 通过 nid_map 匹配 element_tree 节点 → 写入 node.inlineStyle
  → OmniViewOutput { css, component_css, element_tree(with inlineStyle), ... }
  → <style> 按 scope 分池注入 document.head (useCssScopePool: 哈希比较增量更新)
  → OmniViewElementTree 渲染 → OmniViewElement 合并 node.inlineStyle 到 React style prop
```

**关键**: `resolveStylesMulti` 只处理 design token/class/variant/`style {}` 块，**不处理**元素上的内联 `style: { }`。因此内联样式必须在 WASM 返回后手动注入到 element tree 中（`useWasmCompile.ts` L135-157），否则 canvas 不更新。

**新**: `wrap_debug_html(js, css, body)` 已通过 WASM 导出（`omnilang/src/wasm.rs` L124-129），Poiesis TS 端通过 `rustCompiler.wrapDebugHtml()` 调用（rustCompiler.ts L441-455），替代了 `useCompile.ts` 中手工拼接 HTML 模板的代码。

**路径 2: 项目导出 (Rust 后端)**

```
触发: 用户点"导出"或编译按钮
  Store → save_project (写 project.json + .omni 文件)
  → Tauri compile_project(project_path, target)
    ├─ target 解析使用 Target::from_str()（poiesis/src-tauri/src/lib.rs L190-195）
    └─ 文件发现使用 omnilang::discovery::discover_omni_sources() (lib.rs L226-239)
  → omnilang crate: discovery → lexer → parser → semantic → codegen
  → 输出到目标目录（React/Flutter/Vue/Python...）
```

**备注**: 2026-07-12 整合：Poiesis Tauri 端不再直接使用 `walkdir` 依赖，改为调用 `omnilang::discovery::discover_omni_sources()`（`omnilang/src/discovery.rs`），与 `omnic` CLI 共享同一个文件发现实现。目标解析从手动 `match target.as_str()` 改为标准 `Target::from_str()`。

## 四、Zustand Store 结构

```typescript
interface PoiesisState {
  // ── 项目 ──
  projectDir, projectName, isDirty
  activeFilePath, activeFileContent

  // ── 页面路由 ──
  currentPage: "architecture" | "design"
  mode: "design" | "code"
  compileTarget: string

  // ── 架构页面 ──
  architecture: ArchitectureState
    // mode: "overall" | "local"
    // expandedBubbles, focusComponentId
    // bubbles, edges (BubbleNode[], BubbleEdge[])
    // camera: { x, y, zoom }

  // ── 设计页面 ──
  designPage: DesignPageState
    // tabs: TabData[] (当前打开的标签页)
    // activeTabId
    // codeFlowVisible / codeFlowElementId / codeFlowFocusPoint
    // previewOutput: OmniViewOutput | null  ← CodeFlow 模式中 OmniViewRenderer 写入，FlowCanvas 读取
    // breadcrumbs: Breadcrumb[]

  // ── 当前 Tab 画布数据 ──
  elements: ElementData[]       // 元素树
  wires: WireData[]             // 连线
  rootOrder: string[]           // 根元素顺序
  selectedElementIds: string[]

  // ── Tab 缓存 ──
  tabData: Record<string, {
    elements, wires, rootOrder, selectedElementIds,
    filePath, codeFlowVisible, codeFlowElementId
  }>

  // ── 设计系统 ──
  activeDesign: DesignDefinition  // tokens + classes + variants
  elementVariants: Record<string, ElementVariantBinding>

  // ── 组件定义 ──
  componentDefs: Record<string, ComponentDef>

  // ── PID ──
  pidIndex: Record<string, PidEntry>

  // ── 编辑器元数据 ──
  nodeMeta, wireMeta            // 节点/连线在画布上的位置
  canvasStates                  // 每个文件的 zoom/pan
  panels: { leftWidth, rightWidth }

  // ── 撤销/重做 ──
  undoStacks: Record<string, UndoStack>  // 按 tabId 分栈

  // ── 编译 ──
  compileErrors, previewUrl
}
```

## 五、Tauri 后端架构

### Rust State

```rust
struct ProjectState {
    project_dir: Mutex<Option<String>>,          // 当前项目路径
    file_list_cache: Mutex<Option<CachedFileList>>, // mtime-based 缓存
    compile_cache: Mutex<IncrementalCache>,       // 增量编译缓存
    store_snapshot: Mutex<String>,                // 前端同步的 Store JSON
    pid_index: Mutex<String>,                     // PID 索引 JSON
}
```

### 命令分类

| 类别 | 命令 | 说明 |
|------|------|------|
| 编译 | `compile_project(path, target)` | 项目目录 .omni → 目标语言 |
| 诊断 | `analyse_source(source, file_path)` | 词法+语法+语义检查 |
| 导出 | `project_export(path, out_dir, target)` | 编译 + 写文件 |
| 文件 | `read_file` / `write_file` | 读写项目内文件（sanitize_path 防路径遍历） |
| 项目 | `open_project` / `save_project` / `close_project` | 项目生命周期 |
| 状态 | `sync_store_state` / `get_store_snapshot` / `get_pid_index` | 前后端状态同步 |
| 验证 | `verify_pid_roundtrip` | PID 注释一致性校验 |

### 路径遍历保护

`sanitize_path(base, user_path)`:
1. 拒绝空路径
2. 拒绝绝对路径
3. join base → canonicalize → 验证结果在 base 内
4. 文件不存在时 canonicalize 父目录 + append 文件名

## 六、WASM 编译器接口

```typescript
class OmniCompiler {
    constructor();                                           // 新建实例（含增量缓存）
    upsert_source(path: string, source: string): void;      // 添加/更新源文件 → 失效缓存
    remove_source(path: string): void;                       // 移除源文件
    compile_to_omniview(editor_mode: boolean): OmniViewOutput; // ★ 主编译入口
    diagnostics(): DiagnosticResult;                         // 仅诊断，不编译
    version(): string;
}

// 独立函数（创建临时编译器实例，无状态）
compile_multi_to_omniview(sources: Record<string, string>, editor_mode: boolean): OmniViewOutput;
compile_omniview_component(source: string, component_name: string, editor_mode: boolean): any;
compile_to_ir(source: string): IRJson;
```

`editor_mode: true` 时编译器注入 `data-nid` 属性 + 调试包装器，编辑器据此做元素选中/高亮。

## 七、多窗口通信

```
主窗口 (DesignPage)
  ├─ Zustand store (主副本)
  ├─ 子窗口通过 IPC emit/listen 通信
  └─ Tauri WebviewWindow API 创建/销毁子窗口

可分离的面板 (全部可实现，当前未全部启用):
  ├─ 文件树 → 独立窗口
  ├─ 属性面板 → 独立窗口
  ├─ Tab → 独立窗口（多屏编辑）
  └─ Monaco → 独立窗口
```
