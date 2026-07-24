# 关键文件职责地图

> 按功能域组织的文件索引。找某个功能在哪里 → 看这里。

## 一、类型定义（core/types/）

| 文件 | 定义 | 关键类型 |
|------|------|---------|
| `design.ts` | Design System 类型 | `DesignDefinition`, `DesignToken`, `DesignClass`, `VariantDefinition`, `VariantDimension`, `VariantBranch`, `ElementVariantBinding`, `TokenType` |
| `store.ts` | Store 类型 | `StoreDefinition`, `StoreSignalDef`, `StoreGetterDef`, `StoreActionDef` |
| `element.ts` | 元素数据 | `ElementData` = `ViewElementData` \| `CodeblockElementData` \| `GroupElementData` \| `ConditionalBlockData`。条件块有 `branches[]`（@if/@elif/@else）+ 扁平 `children` |
| `style.ts` | 样式属性 | `StyleProperty`, `StyleValueSegment` |
| `wire.ts` | 连线数据 | `WireData` (from/to 节点 + 端口) |
| `design-page.ts` | 设计页面 | `TabType`, `TabData`, `Breadcrumb`, `DesignPageState` |
| `architecture.ts` | 架构页面 | `BubbleNode`, `BubbleEdge`, `ArchitectureState` |
| `project-json.ts` | 项目元数据 | `ProjectJson`, `ComponentDef`, `NodeMeta`, `WireMeta` |
| `pid.ts` | PID 索引 | `PidEntry`, `PidMap` |
| `annotation.ts` | 注解 | 注释解析类型 |
| `code-op.ts` | ★ CodeOp 操作类型 | `CodeOp` = `DeclareVarOp` \| `SetVarOp` \| `IfBlockOp` \| `WhileLoopOp` \| `ForLoopOp` \| `ReturnExprOp` \| `CallFnOp` \| `AwaitExprOp` \| `BreakStmtOp` \| `ContinueStmtOp` \| `CommentOp` \| `RawCodeOp`。编辑器持久化格式 |
| `editor-node.ts` | ★ EditorNode 编辑器格式 | `EditorNode` = `ContainerNode` \| `BlockNode` \| `InlineNode` \| `InputSlot`。细粒度嵌套树，CodeCard Editor v2 内部格式 |
| `codegraph.ts` | 代码图谱 | CodeGraph 节点/边 |
| `node-flow.ts` | 节点流 | FlowCanvas 类型 |
| `project.ts` | 项目 | 项目配置类型 |
| `index.ts` | 聚合导出 | re-export 所有 types |

## 二、状态管理（store/）

| 文件 | 职责 | 关键内容 |
|------|------|---------|
| `types.ts` | ★ Store 接口定义 | `PoiesisState` (~180 fields/methods) |
| `index.ts` | Store 创建 + hook | `usePoiesisStore()` |
| `hooks.ts` | 派生 hooks | 基于 store 的 computed 选择器 |
| `undo-slice.ts` | 撤销栈数据结构 | `UndoStack`, undo/redo 操作 |
| `undo-apply.ts` | 撤销应用逻辑 | 反向应用各种操作类型 |
| `tab-helpers.ts` | Tab 切换辅助 | Tab 数据保存/恢复 |
| `slices/design-system.ts` | ★ Design System CRUD | Token/Class/Variant 所有 action |
| `slices/store-crud.ts` | ★ Store CRUD | Signal/Getter/Action CRUD + @store 引用追踪 + inlineStoreDefs 并行 CRUD（结构镜像 storeDefs） |
| `slices/project.ts` | 项目管理 | 项目打开/关闭/状态 |
| `slices/element-crud.ts` | 元素 CRUD | addElement/removeElement/setProp/moveElement + addConditionalBlock/addConditionalBranch/updateBranchCondition/addChildToBranch/removeChildFromBranch |
| `slices/navigation.ts` | 页面路由 | navigateTo |
| `slices/architecture.ts` | 架构页面状态 | setArchMode/expandBubble/focusComponent |
| `slices/theme.ts` | 主题 | setTheme/resolvedTheme |
| `slices/metadata.ts` | 编辑器元数据 | setNodeMeta/setCanvasState/setPanelSize |
| `slices/serialization.ts` | 序列化 | toProjectJson/loadFromProjectJson |
| `slices/component-lifecycle.ts` | 自定义组件 | defineComponent/removeComponent/enterComponentCanvas |
| `operations/lru-evict.ts` | LRU 淘汰 | Tab 数据缓存淘汰策略 |
| `operations/tab-lifecycle.ts` | Tab 生命周期 | Tab 打开/关闭/切换 |

## 三、代码生成管道（services/codegen/）

| 文件 | 职责 | 方向 |
|------|------|------|
| `pipeline.ts` | ★ 主管道 | Store → .omni 源码（多文件） |
| `pipeline-types.ts` | 管道类型 | GenerateOmniSourceOptions 等 |
| `source-emitter.ts` | 组件/Store .omni 发射 | 生成 component/store 的 .omni 源码。`CodeWriter` 类 + `SourceEmitter` 类 + `emitStoreDefinition()` standalone（Code 面板调用）+ `formatDefaultValue()` 类型感知值格式化 |
| `design-emitter.ts` | ★ design .omni 发射 | DesignDefinition → design.omni 源码 |
| `design-value-parser.ts` | 样式值解析 | .omni 值字符串 → StyleValueSegment[] |
| `parser.ts` | ★ 主解析器 | .omni 源码 → ElementData + DesignDefinition。`parseSignal()`/`parseComputed()` **不再**创建 codeblock 元素——改为 push 到 `ctx.signals`/`ctx.computeds`。`parsePropDeclaration()` 写入 `ctx.props` |
| `parser-types.ts` | 解析器类型 | AST 节点类型 |
| `parser-helpers.ts` | 解析辅助 | 公共解析工具函数 |
| `parser-view.ts` | view 解析 | view 模板块的专门解析。`parseViewBody` + `parseViewElement` + `parseViewDirective`（@if/@else if/@else）。**注意**：body 循环必须处理 `</tagname>` 闭合标签，否则创建虚假元素污染 viewParentStack |
| `parser-finalize.ts` | 解析收尾 | cross-reference 修复、PID 补全 |
| `omni-parser.ts` | Omni 解析入口 | 统一的 parse → program 入口 |
| `omni-tokenizer.ts` | Tokenizer | .omni 文本 → token 流 |
| `view-tree-builder.ts` | 视图树构建 | ElementData[] → ViewNode tree。`findElement()` 从 `byId`（view 元素缓存）+ 扁平数组查找非 view 子节点 |
| `collect-styles.ts` | 样式收集 | 从 elements 收集所有 CSS |
| `component-builder.ts` | 组件构建 | elements + `componentDefs[name]` → ComponentPart。Props/signals/computeds 统一从 `getComponentDefinitions()[name]` 读取，不再从 standaloneSignals/standaloneComputeds 提取 |
| `expression-lowering.ts` | 表达式降级 | 复杂表达式 → 简单形式 |
| `token-emitter.ts` | Token 发射 | 生成 token 声明 |
| `store-extractor.ts` | Store 提取 | 从 elements 提取 Store 定义 |
| `wire-analyzer.ts` | 连线分析 | wires → 信号/数据流图 |
| `annotation-data.ts` | 注解数据 | PID/注解处理 |
| `chain-emitter.ts` | 链式发射 | 链式调用生成 |
| `graph-compiler.ts` | ★ 代码流编译 | CodeFlow 图 (nodes+wires) → fn body .omni 源码（含 // @pid:xxx） |
| `graph-parser.ts` | ★ 代码流解析 | fn body .omni 源码 → CodeFlow 图 (elements+wires)，GraphCompiler 的逆 |
| `expand-custom-components.ts` | 自定义组件展开 | 自定义组件 → 内联元素 |
| `import-collector.ts` | Import 收集 | 收集并去重 import 语句 |
| `validator.ts` | 验证器 | .omni 语义验证 |
| `style-parser.ts` | 样式解析器 | CSS 文本 → StyleProperty[] |
| `types.ts` | 内部类型 | ComponentMeta, StageResult 等 |
| `index.ts` | 公共 API | re-export |

## 四、UI 组件

### 4.1 画布系统（components/canvas/）

| 文件 | 职责 |
|------|------|
| `InfiniteCanvas.tsx` | ★ 画布顶层：pan/zoom 状态 + 事件分发 |
| `CanvasGrid.tsx` | 无限网格背景 |
| `Artboard.tsx` | 设备尺寸画板容器 |
| `ArtboardContent.tsx` | 画板内容：CSS scope 注入 + OmniViewElementTree |
| `ArtboardHeader.tsx` | 设备选择下拉 + 尺寸显示 |
| `ZoomControls.tsx` | 缩放按钮 + 百分比 |
| `canvas-store.ts` | Canvas 专用 Zustand slice |
| `code/FlowCanvas.tsx` | ★ 代码流画布（ReactFlow），4 节点：fn/code-block/condition/return |
| `code/nodes/FnNode.tsx` | fn 节点：函数入口，单出口 flow-out |
| `code/nodes/ConditionNode.tsx` | condition 节点：if/elif/else 组合判断，每分支独立出口 |
| `code/nodes/CodeBlockNode.tsx` | code-block 节点：自由 OmniView 代码文本，flow-in + flow-out |
| `code/nodes/ReturnNode.tsx` | return 节点：返回语句，单入口 flow-in（终端） |
| `code/nodes/DesignPreviewNode.tsx` | ★ CodeFlow 中的设计缩略图节点 |
| `code/FlowInspector.tsx` | ★ CodeFlow 右侧面板：选中 FN 节点编辑 name+params，选中 Return 编辑表达式 |
| `code/CanvasContextMenu.tsx` | 画布右键菜单 |
| `code/CodeEditorModal.tsx` | ★ 代码块代码编辑器（Monaco Editor 80vw×80vh 模态面板）。集成 `services/monaco/` 提供 OmniLang 语法高亮 + 代码片段 + 类型注入 + WASM 诊断。Props：`language`/`storeDefs`/`omniSourcePath`/`omniSource`（diagnostics 用） |
| `code/CodeCardEditor.tsx` | ★★ CodeCard v2 入口：Lexical 编辑器容器（5 种自定义节点 + 4 插件 + Monaco 双模式） |
| `code/card-editor/` | ❌ 已删除（2026-07-24，被 Lexical v2 架构取代） |
| `code/lexical/` | ★★ CodeCard v2 Lexical 层（16 files） |
| `code/lexical/nodes/CardNode.ts` | 内联卡片：DecoratorNode 只读卡片（mut/operator/type/var_ref/literal/dot/separator/comment），`<span>` 渲染 |
| `code/lexical/nodes/ContainerNode.ts` | 容器卡片：ElementNode 包裹子卡（let/set/return/await/call/array/map），浅色背景 + 左侧色条 |
| `code/lexical/nodes/BlockNode.ts` | 块结构卡片：ElementNode 三行封闭色条（if/while/for），含 `__branches` 元数据支持 elif/else |
| `code/lexical/nodes/BodyLineNode.ts` | 代码行节点：ElementNode 块内缩进行，含 `__branchIndex` 标记分支归属 |
| `code/lexical/nodes/InputSlotNode.ts` | 输入槽：TextNode 子类 `\` 唤起的内联输入框，`data-slot-kind` 区分 name/type/expression/condition/fn_name |
| `code/lexical/nodes/index.ts` | 节点 barrel 导出 |
| `code/lexical/transforms/convertToNodes.ts` | ★ CodeOp[] → EditorNode[]：加载方向转换器 |
| `code/lexical/transforms/convertToOps.ts` | ★ EditorNode[] → CodeOp[]：保存方向转换器 |
| `code/lexical/plugins/CodeOpSyncPlugin.ts` | ★ 双向同步：initialOps → EditorNode[] → Lexical 树（加载）；Lexical 树 → EditorNode[] → CodeOp[] → onSync（变更） |
| `code/lexical/plugins/AutocompletePlugin.tsx` | 补全插件：打字检测 + 上下文感知补全 + `\` 输入框创建 + 卡片插入 |
| `code/lexical/plugins/KeyboardPlugin.ts` | 键盘导航：← → 卡片间移动、↑ ↓ body line 跳转、Delete 层级删除、Tab 运算符循环、Enter body line 换行 |
| `code/lexical/autocomplete/autocomplete-rules.ts` | 补全规则：$parentKind 感知的上下文过滤（let→mut/:/type，set→operator，if/while→comparison，dot→methods） |
| `code/lexical/autocomplete/AutocompleteMenu.tsx` | 浮动补全菜单：window.getSelection 定位，↑↓ Enter/Escape 导航 |

### 快速事实 — CodeCard Editor v2

| 事实 | 说明 |
|------|------|
| 模型 | 细粒度 EditorNode[] 嵌套树：Container（容器）/ Block（块结构）/ Inline（内联卡）/ InputSlot（输入框） |
| 基座 | Lexical 单一 contentEditable 实例，5 种自定义节点 |
| 补全 | 打字前缀过滤 + 上下文感知（parentKind/precedingCardKind），菜单定位跟随光标 |
| 视觉 | 统一圆角色块 + 语义色：紫(let)/蓝(set)/橙(mut)/灰(operator)/青(type/call)/绿(var_ref/literal)/红(return) |
| 容器 | 浅色半透明背景包裹子卡（宽度=内容宽），右侧空白为 ▌ 整组删除区 |
| 块结构 | 三行封闭色条：┏ 头行 / ┃ 代码行（Enter 扩展）/ ┗ 收口行（细条） |
| 键盘 | ← → 卡片间移动、↑ ↓ body line 跳转、Delete 层级删除、Tab 切换 set 运算符、Enter body line 换行、\ 唤起输入框 |
| 双模式 | "文本模式"按钮 → CodeEditorModal (Monaco) 作为 fallback |
| 数据流 | CodeOp[] → convertToNodes → EditorNode[] → Lexical 树 → 编辑 → EditorNode[] → convertToOps → CodeOp[] → emitOps → source |
| `shared/nodeRegistry.ts` | 节点类型注册表 |
| `shared/connectionValidators.ts` | 连线验证 |
| `shared/layoutUtils.ts` | 自动布局工具 |
| `shared/socketTypes.ts` | 端口类型定义 |

### 4.2 Design 编辑器（components/design/）

| 文件 | 职责 |
|------|------|
| `DesignTabBar.tsx` | Tokens / Classes / Variants 标签切换 |
| `StoreMembersPanel.tsx` | Store 编辑器的左侧成员列表（Signals/Getters/Actions 分组 + 折叠 + 内联 CRUD） |
| `StorePage.tsx` | Store 页面顶层组件：左侧 StoreMembersPanel + 中央 CodeFlowCanvas（action 切换逻辑） |
| `DesignTabContent.tsx` | 根据 active tab 分发到 TokenEditor/ClassEditor/VariantEditor |
| `TokenEditor.tsx` | Token 列表：创建、编辑、删除、分组 |
| `ClassEditor.tsx` | Class 编辑器：属性卡片 + pseudo states |
| `VariantEditor.tsx` | ★ Variant 编辑器：维度管理 + 分支标签 + 属性卡片 + 预览 |
| `ClassPreview.tsx` | Class/Variant 实时预览（支持多元素类型切换） |
| `CodeDrawer.tsx` | 生成代码抽屉（可切换目标语言） |
| `OmniViewElement.tsx` | 单元素 → React.createElement |
| `OmniViewElementTree.tsx` | 元素树递归渲染 |
| `OmniViewRenderer.tsx` | ★ 设计画布渲染入口（始终挂载，output 同步到 store.previewOutput） |
| `StoreDrawer.tsx` | ★ Store 面板抽屉（组件标签页内）：右侧滑出全幅面板，内嵌 StorePage。Store 切换下拉 + 新建弹窗（内联/全局选择） |
| `CodeFlowCanvas.tsx` | 代码流画布入口（→ re-export FlowCanvas） |
| `ComponentMembersPanel.tsx` | ★ CodeFlow 左侧面板：设计组件 prop/signal/computed 声明。三折叠区 + 内联 CRUD，读写 `componentDefs[name]`。Computed expression 弹出 CodeEditorModal |
| `DesignPreviewNode.tsx` | CodeFlow 中的设计缩略图（ReactFlow 节点，无 handle，读 store.previewOutput） |
| `FileTreePanel.tsx` | 项目文件树 |
| `BreadcrumbBar.tsx` | 面包屑导航 |
| `CollapsibleGroup.tsx` | 可折叠属性分组 |
| `PropertyRow.tsx` | 通用属性行（label + input） |
| `ScrubInput.tsx` | 拖拽调值输入框 |
| `ColorHUD.tsx` | HUD 取色器 |
| `HSVPanel.tsx` | HSV 颜色面板 |
| `TokenSlashPicker.tsx` | / 触发的 token 选择面板 |
| `FocusOverlay.tsx` | 聚焦覆盖层 |
| `CodeFlowCanvas.tsx` | fn 节点流画布 |
| `IconButtonGroup.tsx` | 图标按钮组（枚举选择） |
| `NewFileModal.tsx` | 新建文件弹窗 |
| `render-utils.ts` | 渲染工具函数 |
| `useCssScopePool.ts` | ★ CSS scope 分池增量更新 |
| `useWasmCompile.ts` | ★ 防抖 WASM 编译 |
| `cards/SizeCard.tsx` | Width/Height 卡片 |
| `cards/BackgroundCard.tsx` | Background + Opacity 卡片 |
| `cards/BorderCard.tsx` | Border (width/style/color) 卡片 |
| `cards/RadiusCard.tsx` | BorderRadius 卡片 |
| `cards/SpacingCard.tsx` | Padding/Margin 卡片 |
| `cards/BoxModelPreview.tsx` | 盒模型可视化 |
| `cards/LockIcon.tsx` | 锁定/解锁图标 |

### 4.3 面板（components/panels/）

| 文件 | 职责 |
|------|------|
| `ComponentPalette.tsx` | 可拖拽组件列表（内置 + 自定义） |
| `ComponentTree.tsx` | 当前文件元素层级树 |
| `ContextInspector.tsx` | ★ 元素右侧属性面板 |
| `CssStylePanel.tsx` | CSS 变量 + Class 编辑面板（design.omni） |
| `inspector/InlineStylePanel.tsx` | ★ 画布元素内联样式面板（替代旧 StylesSection）。属性级自锁：空值灰显(opacity:0.4)=锁定，有值亮起+×清除=解锁。始终展示所有卡片（无 section 级 🔒/🔓 开关） |
| `inspector/TypographyBasics.tsx` | 轻量文字卡片（字号 scrub + 预设 + 字重 select + 颜色 picker），仅文本元素显示 |
| `TreeNodeRenderer.tsx` | 树节点递归渲染 |
| `tree-utils.ts` | 树操作工具函数 |
| `inspector/PropsSection.tsx` | ★ Props 编辑区。内含 `LabelEditor` 组件：表达式 `{name}` 以紫色芯片展示+删除，文本以内联 `<input>` 直接编辑，纯文本时退化为普通输入框 |
| `inspector/EventsSection.tsx` | 事件绑定区 |
| `inspector/BindingsSection.tsx` | 双向绑定区 (bind:value)。仅 input/textarea/select 显示。Prop 下拉 + Signal 搜索弹出选择器。写入 `element.bindings`，走 pipeline → emitter 输出 `bind:value={sig}` |
| `inspector/ConditionalBlockEditor.tsx` | ★ 条件块全分支编辑器：列出所有 @if/@elif/@else 分支、InlineConditionEditor、子元素列表、[+ 添加 @else if] |
| `inspector/BranchConditionEditor.tsx` | 单分支编辑器：条件表达式 + 子元素 + 删除（elif only） |
| `inspector/InlineConditionEditor.tsx` | 内联条件编辑器（click to edit），复刻 Computed 编辑体验：靛蓝背景+文字、monospace、单行省略 |
| `inspector/StylesSection.tsx` | 样式编辑区 |
| `inspector/ClassesSection.tsx` | Class 绑定区 |
| `inspector/VariantBindingSection.tsx` | Variant 绑定区 |
| `inspector/ColorPicker.tsx` | 颜色选择器 |
| `css-panel/FormulaValueInput.tsx` | 公式值输入 |

### 4.4 架构页面（components/architecture/）

| 文件 | 职责 |
|------|------|
| `ArchitectureGraph.tsx` | 泡泡关系图 |
| `ArchitecturePreview.tsx` | 架构预览 |
| `ArchitectureToolbar.tsx` | 工具栏（模式切换/缩放） |
| `nodes/BubbleNode.tsx` | 泡泡节点渲染 |
| `nodes/ChildBubbleNode.tsx` | 子泡泡节点 |
| `edges/BubbleEdge.tsx` | 泡泡连线 |
| `context-menu/BubbleContextMenu.tsx` | 泡泡右键菜单 |

### 4.5 Shell（components/shell/）

| 文件 | 职责 |
|------|------|
| `Toolbar.tsx` | 顶部工具栏 |
| `StatusBar.tsx` | 底部状态栏 |
| `theme.ts` | 编辑器主题 tokens |

## 五、服务层（services/）

### 5.1 编译器（services/compiler/）

| 文件 | 职责 |
|------|------|
| `compiler.ts` | WASM 编译器封装（compile_to_omniview, compileMultiToOmniView, resolveStyles, resolveStylesMulti, wrapDebugHtml, OmniCompiler stateful） |
| `cache.ts` | 编译缓存管理 |

### 5.2 解析器（services/parser/）

| 文件 | 职责 |
|------|------|
| `source-parser.ts` | .omni 源码 → 结构化数据 |
| `omni-regex-parser.ts` | 正则辅助解析 |
| `index-loader.ts` | 索引加载 |
| `search-index.ts` | 搜索索引 |

### 5.3 持久化（services/persistence/）

| 文件 | 职责 |
|------|------|
| `save-manager.ts` | 保存流程（生成 .omni → 写文件 → 写 project.json） |
| `load-manager.ts` | 加载流程（读 project.json → 读 .omni → 解析） |
| `project-json.ts` | project.json 读写 |
| `index.ts` | 公共 API |

### 5.4 Monaco 语言服务（services/monaco/）

**2026-07-10 新增。** 为 CodeEditorModal 提供 OmniLang 语法高亮、代码片段、类型定义注入、WASM 诊断集成。

| 文件 | 职责 |
|------|------|
| `index.ts` | 统一导出入口 |
| `omnilang-language.ts` | Monarch tokenizer 注册 `omnilang` 语言：70+ 关键字、50+ 注解（`@pid`/`@dim`/`@if` 等）、13 种设计 token 类型、双字符串/模板字符串、行+块注释 |
| `omnilang-types.ts` | 静态 `.d.ts`（11 个 std 模块 + 基本类型 + 实例方法）+ `generateStoreTypes(storeDefs)` 动态生成 Store 类型 → `addExtraLib()` 注入 TS IntelliSense |
| `omnilang-snippets.ts` | `CompletionItemProvider` — 16 个代码逻辑片段（if/for/while/let/match/try/import/lambda/arrow 等），注册在 `typescript` 语言上 |
| `diagnostics.ts` | WASM `OmniCompiler.diagnostics()` 接入：`getOmniCompiler()` 实例管理 + `wasmDiagnosticToMarker()` 映射 + `applyDiagnostics()` → `setModelMarkers()` |

### 5.5 其他服务

| 文件 | 职责 |
|------|------|
| `services/project/project-service.ts` | 项目创建/验证 |
| `services/project/component-service.ts` | 组件注册/发现 |
| `services/pid/index.ts` | PID 索引管理 |
| `services/pid/pid-index.ts` | PID 索引数据结构 |
| `services/codegraph/store.ts` | 代码图谱 store |
| `services/codegraph/types.ts` | 代码图谱类型 |
| `services/architecture/bubble-data.ts` | 泡泡数据处理 |
| `services/runtime/debug-service.ts` | 运行时调试 |
| `services/runtime/run-service.ts` | 运行时执行 |

## 六、Driver（driver/）

| 文件 | 职责 |
|------|------|
| `OmniDriver.ts` | ★ 主编排器：协调 Canvas/Code/File/State 各 Driver |
| `CanvasDriver.ts` | 画布操作 API（添加元素、移动、缩放） |
| `CodeDriver.ts` | 代码操作 API |
| `ConnectionDriver.ts` | 连线操作 API |
| `ElementHandle.ts` | 元素句柄 |
| `FileDriver.ts` | 文件操作 API |
| `StateDriver.ts` | 状态操作 API |
| `WireHandle.ts` | 连线句柄 |
| `SnippetService.ts` | 代码片段服务 |
| `VisualService.ts` | 视觉服务 |
| `store.ts` | Driver 专用 store |
| `types.ts` | Driver 类型 |

## 七、Tauri 后端（src-tauri/）

| 文件 | 职责 |
|------|------|
| `src/lib.rs` | ★ 全部 Tauri 命令 + ProjectState + sanitize_path |
| `src/main.rs` | 入口（调用 lib::run()） |
| `tauri.conf.json` | Tauri 配置（identifier: com.omnilang.poiesis） |
| `Cargo.toml` | Rust 依赖 |
| `.mcp.json` | Victauri bridge 配置 |

## 八、数据文件（data/）

| 文件 | 职责 |
|------|------|
| `componentProtocol.ts` | ★ 内置组件协议：标签名/属性/事件/推荐 variant/推荐 class |

## 九、Hooks（hooks/）

| 文件 | 职责 |
|------|------|
| `useSaveProject.ts` | 保存项目 hook |
| `useDragHandlers.ts` | 拖拽处理 hook |
| `useOmniViewRender.ts` | OmniView 渲染 hook |
| `useLiveCodePreview.ts` | 实时代码预览 hook |
| `useBreadcrumb.ts` | 面包屑导航 hook |
| `useBubbleLayout.ts` | 泡泡布局 hook |
| `useBubbleExpand.ts` | 泡泡展开 hook |
| `useZAxisCanvas.ts` | Z 轴画布 hook |
| `useDebugBridge.ts` | 调试桥接 hook |

## 十、Features（features/）

| 文件 | 职责 |
|------|------|
| `compiler/useAutoCompile.ts` | 自动编译（防抖 + 变更检测） |
| `compiler/useCompile.ts` | 手动编译 |
| `compiler/useRun.ts` | 运行 |
| `debug/useDebugSession.ts` | 调试会话 |
| `useProjectInit.ts` | 项目初始化 |

## 十一、命令封装（commands/）

| 文件 | 职责 |
|------|------|
| `compile-api.ts` | compile_project 封装 |
| `project-api.ts` | open/save/close 封装 |
| `annotation-api.ts` | analyse_source 封装 |
| `rustCompiler.ts` | WASM 编译器封装（OmniCompiler 类实例） |
| `types.ts` | 命令类型 |
| `index.ts` | api 统一导出 |
