# 关键文件职责地图

> 按功能域组织的文件索引。找某个功能在哪里 → 看这里。

## 一、类型定义（core/types/）

| 文件 | 定义 | 关键类型 |
|------|------|---------|
| `design.ts` | Design System 类型 | `DesignDefinition`, `DesignToken`, `DesignClass`, `VariantDefinition`, `VariantDimension`, `VariantBranch`, `ElementVariantBinding`, `TokenType` |
| `element.ts` | 元素数据 | `ElementData` (id, tag, props, children, parentId, position, source, style, cssClasses, componentRef) |
| `style.ts` | 样式属性 | `StyleProperty`, `StyleValueSegment` |
| `wire.ts` | 连线数据 | `WireData` (from/to 节点 + 端口) |
| `design-page.ts` | 设计页面 | `TabType`, `TabData`, `Breadcrumb`, `DesignPageState` |
| `architecture.ts` | 架构页面 | `BubbleNode`, `BubbleEdge`, `ArchitectureState` |
| `project-json.ts` | 项目元数据 | `ProjectJson`, `ComponentDef`, `NodeMeta`, `WireMeta` |
| `pid.ts` | PID 索引 | `PidEntry`, `PidMap` |
| `annotation.ts` | 注解 | 注释解析类型 |
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
| `slices/project.ts` | 项目管理 | 项目打开/关闭/状态 |
| `slices/element-crud.ts` | 元素 CRUD | addElement/removeElement/setProp/moveElement |
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
| `source-emitter.ts` | 组件 .omni 发射 | 生成 component/store 的 .omni 源码 |
| `design-emitter.ts` | ★ design .omni 发射 | DesignDefinition → design.omni 源码 |
| `design-value-parser.ts` | 样式值解析 | .omni 值字符串 → StyleValueSegment[] |
| `parser.ts` | ★ 主解析器 | .omni 源码 → ElementData + DesignDefinition |
| `parser-types.ts` | 解析器类型 | AST 节点类型 |
| `parser-helpers.ts` | 解析辅助 | 公共解析工具函数 |
| `parser-view.ts` | view 解析 | view 模板块的专门解析 |
| `parser-finalize.ts` | 解析收尾 | cross-reference 修复、PID 补全 |
| `omni-parser.ts` | Omni 解析入口 | 统一的 parse → program 入口 |
| `omni-tokenizer.ts` | Tokenizer | .omni 文本 → token 流 |
| `view-tree-builder.ts` | 视图树构建 | ElementData → ViewNode tree |
| `collect-styles.ts` | 样式收集 | 从 elements 收集所有 CSS |
| `component-builder.ts` | 组件构建 | elements → ComponentDef |
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
| `code/CanvasContextMenu.tsx` | 画布右键菜单 |
| `shared/nodeRegistry.ts` | 节点类型注册表 |
| `shared/connectionValidators.ts` | 连线验证 |
| `shared/layoutUtils.ts` | 自动布局工具 |
| `shared/socketTypes.ts` | 端口类型定义 |

### 4.2 Design 编辑器（components/design/）

| 文件 | 职责 |
|------|------|
| `DesignTabBar.tsx` | Tokens / Classes / Variants 标签切换 |
| `DesignTabContent.tsx` | 根据 active tab 分发到 TokenEditor/ClassEditor/VariantEditor |
| `TokenEditor.tsx` | Token 列表：创建、编辑、删除、分组 |
| `ClassEditor.tsx` | Class 编辑器：属性卡片 + pseudo states |
| `VariantEditor.tsx` | ★ Variant 编辑器：维度管理 + 分支标签 + 属性卡片 + 预览 |
| `ClassPreview.tsx` | Class/Variant 实时预览（支持多元素类型切换） |
| `CodeDrawer.tsx` | 生成代码抽屉（可切换目标语言） |
| `OmniViewElement.tsx` | 单元素 → React.createElement |
| `OmniViewElementTree.tsx` | 元素树递归渲染 |
| `OmniViewRenderer.tsx` | ★ 设计画布渲染入口（始终挂载，output 同步到 store.previewOutput） |
| `CodeFlowCanvas.tsx` | 代码流画布入口（→ re-export FlowCanvas） |
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
| `inspector/InlineStylePanel.tsx` | ★ 画布元素内联样式面板（替代旧 StylesSection）。🔒/🔓 分区锁机制，复用 SizeCard/SpacingCard 等卡片 |
| `inspector/TypographyBasics.tsx` | 轻量文字卡片（字号 scrub + 预设 + 字重 select + 颜色 picker），仅文本元素显示 |
| `TreeNodeRenderer.tsx` | 树节点递归渲染 |
| `tree-utils.ts` | 树操作工具函数 |
| `inspector/PropsSection.tsx` | Props 编辑区 |
| `inspector/EventsSection.tsx` | 事件绑定区 |
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
| `compiler.ts` | WASM 编译器封装（compile_to_omniview） |
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

### 5.4 其他服务

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
