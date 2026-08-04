# CodeFlowCanvas 内联编辑升级设计

> 日期：2026-08-04
> 状态：待实现
> 范围：CodeFlowCanvas 节点编辑体验升级——节点内联字段编辑 + 可搜索节点创建菜单 + 拖出创建 + FlowInspector 降级为预览

## 一、目标与背景

ADR-057 新增的 4 种语法节点（let/assign/store-call/call）目前只能通过右侧 FlowInspector 面板的**纯字符串输入框**编辑（如 `args` 是"逗号分隔字符串"），体验不成熟。本次升级对标 Unity Visual Scripting 的**交互精神**（节点即编辑面、快速创建），将编辑入口从右侧面板移到节点本身。

**关键定性（用户确认）**：本项目是**代码的节点编辑器**（节点=语句，连线=执行顺序），而 Unity VS 是**逻辑的节点编辑器**（节点=操作符/值，连线=数据流）。两者本质不同——**不做数据端口/值连线**。语句是文本表达式（`items.filter(...).len()` 无法数据流化），名字作用域本身就是绑定，数据连线只是名字的可视化替身，成本与风险不对称。

## 二、需求澄清（用户确认）

1. **编辑形态**：节点内联字段编辑（节点即编辑面），对标 Unity VS 交互
2. **覆盖节点**：语法节点（let/assign/store-call/call）+ condition（表达式内联）+ return（表达式内联）；fn 参数列表保留面板；code-block 保持双击 Monaco
3. **创建交互**：① 右键画布→可搜索菜单 ② 从 flow-out 口拖出到空白→搜索菜单→自动连线 ③ 空格键唤出搜索菜单
4. **面板去留**：降级为实时预览（formatSyntaxNode 输出）+ fn 参数编辑保留
5. **实现路线（用户确认）**：自建轻量内联编辑体系，但以**成熟**为标准——先研究开源实现（ComfyUI / xyflow 官方模式），复刻其成熟模式，不追求轻量

## 三、开源研究结论（复刻对象）

### 3.1 ComfyUI（Comfy-Org/ComfyUI_frontend）

**搜索菜单（NodeSearchBoxPopover.vue + v2/）**：
- 键盘导航：`role=combobox`、`aria-autocomplete=list`、Enter 选中 / ↑↓ 导航 / 挂载自动聚焦
- 命中高亮：`highlightQuery()` 对匹配子串高亮（转义后 v-html）
- **拖出创建连线**：`dropped-on-canvas` 事件 → `cancelNextReset` 拦截链接重置 → 打开搜索框（按源端口类型预过滤）→ `addNode` 后 `connectToNode(node, triggerEvent)` 自动接上刚拖出的线
- 状态防护：弹层 300ms 防误关；关闭时 `reset()` 清理半途连线状态；键盘触发时定位画布中心
- 模糊搜索用 Fuse.js——**本项目仅 8 种节点，不引入**（复刻交互结构即可）

**节点内联控件**：BaseWidget 体系 + 具体类型（Text/Combo/Boolean/Select/Textarea）；hover 态背景暗示可编辑、invalid 红色描边、readonly/disabled 支持；widget 值存 store（graphId+nodeId+widgetName 键控），序列化时读取

**弹层纪律**：`useDismissOnCanvasGesture`——画布平移/缩放/框选时自动关闭锚定弹层（防残留错位）

### 3.2 xyflow 官方模式

- 内联控件必须 `class="nodrag"` 防与画布拖拽冲突
- **本地 state 做 UI，提交时同步到 data/props**（避免光标跳动，官方文档明确警告不要把 data 直接当 UI state）
- 自动宽度输入（MindMapNode：`useLayoutEffect` 设置 input width）
- 自定义节点 memo 化

### 3.3 本项目技术基线

- `@xyflow/react ^12.11.0`、React 19、Zustand 5——onConnectStart/onConnectEnd 等 API 全部可用，**无需升级库，升级的是用法**
- 现有 232 tests 全绿基线

## 四、整体架构

```
src/components/canvas/code/
├── nodes/
│   ├── SyntaxNode.tsx      → 重写为"内联编辑版"（每字段草稿+提交）
│   ├── ConditionNode.tsx   → 加内联表达式编辑
│   └── ReturnNode.tsx      → 加内联表达式编辑
├── fields/（新建）
│   ├── NodeField.tsx       → 共享字段控件（文本/select/chip/参数行），统一草稿提交语义
│   └── NodeField.module.css
├── NodeSearchMenu.tsx（新建）→ 可搜索节点创建菜单（搜索框+列表+高亮+键盘导航）
├── NodeSearchMenu.module.css
├── FlowCanvas.tsx          → 接入空格快捷键/拖出创建/搜索菜单替换右键菜单
└── FlowInspector.tsx       → 降级：语法节点显示 formatSyntaxNode 实时预览 + 编辑提示
```

**核心数据流不变**：所有编辑仍写入 `props` → `formatSyntaxNode` 单一来源 → @pid round-trip 零改动（ADR-057 决策延续）。**草稿只在节点组件局部 state 中，Enter/blur 才提交 setProp**（undo 栈干净）。

**数据补全候选（全部来自现有 store，零新数据源）**：

| 字段 | 候选来源 |
|------|---------|
| store 名 | `storeReferences[componentName]` + `storeDefs`/`inlineStoreDefs` 键 |
| action | `storeDefs[store].actions`（显示签名） |
| fnName | `componentDefs[comp].fns` + store actions + `componentDefs[comp].events`（emit） |
| assign target | signals + props + computeds + fn 参数 + 同 fn let 变量 + store 信号（`Store.signal`） |

## 五、节点内联编辑交互

### 5.1 节点布局（Unity VS 风格：头部 + 字段体 + 流端口）

- **let**：头部（彩色 tag + 中文名 + mutable 徽章）；变量名=内联输入；类型=小 select（可自输入）；初始值=内联输入（可放表达式）
- **assign**：`[目标输入] [=▼] [表达式输入]` 三字段一行
- **store-call**：`[Store▼] . [action▼] ( args 参数行 )`——store/action 是**联动下拉**（选 TodoStore 后 action 只列它的 actions，显示签名 `add(text: String, count: i32)`）；args 是可增删的行式编辑
- **call**：`[函数名输入 带补全▼] ( 参数行 ) [emit 徽章]`
- **condition**：`if ( [表达式输入] )`
- **return**：`return [表达式输入]`
- 所有节点保留 flow-in（上）/ flow-out（下）handle，控制流语义不变

### 5.2 交互纪律

1. **草稿提交**：字段聚焦 → 输入（本地 state，props 不动，光标不跳）→ **Enter 或失焦提交** setProp → **Esc 取消恢复原值**
2. **`class="nodrag"`** 所有内联控件
3. **悬停态**：字段 hover 浅背景 + 可编辑暗示；聚焦高亮边框
4. **参数行**：每参一行（输入框 + × 删除 + 底部「+ 添加参数」）；空的参数行自动折叠
5. **轻量即时校验**（本地，不阻塞输入）：变量名非法标识符/空 target/action 不在所选 store → 字段红色描边 + 节点底部警告条；**校验不阻止输入**（保存时 WASM 门禁仍是最终裁决）
6. **表达式字段 token 级补全**：输入时对当前 token 显示候选列表（fn 参数/let 变量/signals/store 信号），Tab/点击插入——不做完整表达式解析

### 5.3 视觉

- 沿用现有 4 色系分类（变量/赋值、调用等），保持当前卡片圆角/阴影语言
- 节点宽度自动随内容（`useLayoutEffect` 测量）
- 复用 `--ol-*` CSS 变量体系

## 六、节点搜索菜单 + 拖出创建

### 6.1 触发方式（三种，用户确认）

| 触发 | 行为 |
|------|------|
| 右键画布空白 | 打开可搜索菜单（替代现有 CanvasContextMenu） |
| 空格键（画布内无输入焦点时） | 打开菜单，定位画布中心 |
| 从节点 flow-out 口拖出 → 空白处松开 | 打开菜单，选中后自动连线（新节点 flow-in ← 源 flow-out） |

### 6.2 菜单交互

- **过滤**：输入同时匹配中文名 + 英文 tag + 快捷键别名（子串匹配，8 项无需 Fuse.js）；无结果显示「无匹配节点」
- **键盘**：↑↓ 移动选中（自动滚动可见）、Enter 在鼠标位置创建、Esc 关闭
- **分类分组**：复用现有 MENU_SECTIONS（控制流 / 变量与赋值 / 调用）
- **命中高亮**：匹配子串高亮（转义后渲染）

### 6.3 拖出创建状态机（对标 ComfyUI dropped-on-canvas 拦截）

```
onConnectStart → 记录 { 源节点 id, 端口 flow-out }
onConnectEnd → 检测释放点落在画布空白（event.target 非 handle）
  ├─ 是 → 拦截半途连线（不产生无效边）→ 打开菜单，记 pending 源
  │        → 选中节点：addElement(flow 坐标) + addWire(源 flow-out → 新节点 flow-in)
  │        → 菜单关闭：清空 pending
  └─ 否 → 正常 port→port 连线走现有 onConnect
```

### 6.4 弹层纪律

- 挂载到画布容器；菜单外点击 / Esc / 再次右键即关闭
- 打开时 300ms 防误关（ComfyUI 模式）
- 画布平移/缩放/框选时关闭（useDismissOnCanvasGesture 模式——本项目用 ReactFlow viewport onMoveStart 或等价实现）

### 6.5 清理

现有 `CanvasContextMenu.tsx` 被完全替代后删除（避免死代码）。

## 七、FlowInspector 降级

```
┌──────────────────────────┐
│ 变量声明 (let)            │  ← 标题：中文名 + tag
├──────────────────────────┤
│ ⚡ 在节点上直接编辑        │  ← 编辑提示
│                          │
│ 生成代码（实时预览）       │
│ ┌──────────────────────┐ │
│ │ let mut x: i32 = 0;  │ │  ← formatSyntaxNode 实时输出（等宽、语法色）
│ └──────────────────────┘ │
│ ⚠ 校验提示（如有）        │  ← 与节点上的警告同步
└──────────────────────────┘
```

- 删除 5 个旧表单组件（LetInspector/AssignInspector/StoreCallInspector/CallInspector/ReturnInspector）
- **fn 参数编辑保留不动**（params 多行结构化，留在面板合理）
- 预览即"单一来源"的直观展示：`formatSyntaxNode(tag, props)` 与节点、GraphCompiler 三处同源

## 八、错误处理

- **本地校验**：只提示不阻塞（非法标识符/空 target/action 不存在等），保存时 WASM 门禁最终裁决
- **拖出创建**：半途连线拦截失败时兜底为普通连线（不产生无效边）；pending 源在菜单关闭时清空，防悬空引用
- **补全候选缺失**：数据源不存在（如 storeDefs 无该 store）时静默降级为自由文本输入
- **提交失败**：setProp 无失败路径（纯状态更新）；草稿与 props 失同步时以 props 为准重渲染

## 九、测试策略

| 层 | 内容 |
|----|------|
| 单元 | NodeField 提交语义（Enter/blur 提交、Esc 取消、草稿不触发 setProp）；args 行增删；搜索过滤（中/英/别名匹配 + 高亮定位）；空格/键盘冲突守卫 |
| 组件 | SyntaxNode 内联编辑（渲染输入 → 提交 → props 更新）；NodeSearchMenu（过滤/键盘/无结果）；FlowInspector 预览渲染 |
| 回归 | 现有 232 tests 全绿（graph-parser/round-trip 零改动）；FlowInspector.test.tsx 旧断言更新为预览行为 |
| E2E (Victauri) | 空格→菜单→搜索过滤→Enter 创建；拖出创建自动连线；内联改 props → 保存 → 重开类型保持；`.omni` 含 @pid |

## 十、风险清单

1. **拖出创建与正常连线的区分**：`onConnectEnd` 检测"释放到空白"（event.target 非 handle）才拦截；正常 port→port 走现有 onConnect
2. **空格键冲突**：输入焦点内不触发（复用现有 `e.target instanceof HTMLInputElement` 守卫）；确认 xyflow 无空格默认绑定
3. **校验规则与 WASM 门禁关系**：本地轻量校验只提示不阻塞
4. **性能**：内联控件全部 `nodrag` + 节点 memo（现有 SyntaxNode 已 memo）

## 十一、非目标（明确不做）

- ❌ 数据端口/值连线（本质定性：代码节点编辑器，语句非表达式树）
- ❌ for/while 循环节点（2026-07-10 spec 后续迭代）
- ❌ signal/computed 声明节点（左侧面板已有）
- ❌ 遗留 operations/SlotValue 体系（CodeCard v3 废弃）
- ❌ Fuse.js 等模糊搜索库（8 项节点无需）
- ❌ 节点内 Monaco 表达式编辑器（token 级补全已覆盖需求）
