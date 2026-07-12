# Component Members Panel — 设计文档

> 在 CodeFlowCanvas 左侧添加面板，设计组件的 prop / signal / computed。
> 参考 StoreMembersPanel 的表单式 UI。

## 一、背景

### 当前状态

- 双击画布元素 → `toggleCodeFlow(elementId)` → `codeFlowVisible=true` → 进入组件代码流编辑模式
- CodeFlowCanvas（FlowCanvas）展示组件所有 codeblock 控制流节点（fn / code-block / condition / return）
- 右侧 FlowInspector 编辑选中 fn/return 节点属性
- **无左侧面板** — prop/signal/computed 声明无法在编辑器中可视化编辑

### 参考实现

- **StorePage**：左 `StoreMembersPanel`（280px，管理 signal/getter/action 声明）+ 中央 `FlowCanvas`（编辑 action fn body）
- **StoreMembersPanel**：三个可折叠 section（Signals / Getters / Actions），内联表单编辑

### 目标

在 codeFlowVisible 状态下，为**组件**提供类似 StoreMembersPanel 的左侧面板，管理组件的 prop / signal / computed 声明。

## 二、数据模型变更

### 2.1 新增 ComputedEntry 类型

文件：`poiesis/src/core/types/project-json.ts`

```typescript
/** 组件级 computed 声明 — 对齐 StoreGetterDef */
export interface ComputedEntry {
  pid: string;
  name: string;
  returnType: string;   // 对齐 StoreGetterDef.returnType
  expression: string;   // 对齐 StoreGetterDef.expression
}
```

### 2.2 扩展 ComponentSourceDef

```typescript
export interface ComponentSourceDef {
  name: string;
  filePath: string;
  signals: Array<DefEntry>;          // 已有
  props: Array<DefEntry>;            // 已有
  computeds: Array<ComputedEntry>;   // 新增
  events: Array<{ pid: string; name: string; payloadType: string }>;
  fns: Array<FnDef>;
}
```

### 2.3 Store 层不变

`componentDefs: Record<string, ComponentDef>` 自动包含新字段，无需额外 Zustand slice。

## 三、新建组件：ComponentMembersPanel

路径：`poiesis/src/components/design/ComponentMembersPanel.tsx`

### 3.1 UI 结构

```
┌─ ComponentMembersPanel ────────────┐
│  ▶ Props                         + │  ← 可折叠
│    name: String = "World"        × │  ← 内联编辑 name/type/default
│    age: i32                      × │  ← 无默认值 = 必需 prop
│                                    │
│  ▶ Signals                       + │
│    count: i32 = 0                × │
│    step: i32 = 1                 × │
│                                    │
│  ▶ Computed                      + │
│    doubled: i32 = count * 2      × │  ← expression 可点击
│    label: String = (click edit)  × │  ← 弹出 CodeEditorModal
└────────────────────────────────────┘
```

### 3.2 交互细节

| 区域 | 操作 | 行为 |
|------|------|------|
| Props section | + 按钮 | 添加新 prop（`pid` 自动生成，name 占位，type=String，无默认值） |
| Props item | name 输入 | 直接编辑 |
| Props item | type 下拉 | 从 OMNI_TYPES 选择 |
| Props item | default 输入 | 留空 = 必需 prop；填值 = 可选 prop |
| Props item | × | 删除 prop |
| Signals section | + 按钮 | 添加新 signal（type=String，default=""） |
| Signals item | name/type/default | 同 Props，但 default 行始终显示 |
| Computed section | + 按钮 | 添加新 computed（returnType=String，expression=""） |
| Computed item | name/returnType | 内联编辑 |
| Computed item | expression span | 点击弹出 CodeEditorModal（和 StoreMembersPanel getter 一致） |
| Computed item | × | 删除 computed |

### 3.3 Props

| prop 属性 | 值 |
|-----------|-----|
| `componentName` | 当前组件名（从 activeTab 获取） |
| `readOnly` | （暂不实现，后续可加） |

读写的核心数据源：`componentDefs[componentName].props / .signals / .computeds`

读写方式：通过 Zustand 的 `setComponentDef(name, updatedDef)` 不可变更新。

### 3.4 样式

完全复用 StoreMembersPanel 的 `S` 内联样式对象（`--ol-*` 设计 token），保持一致外观。

## 四、布局集成

### 4.1 DesignPage 布局变更

当前 codeFlowVisible 时：`gridTemplateColumns: "1fr"`（全部隐藏）
变更后：`gridTemplateColumns: "280px 1fr 240px"`

```
┌─────────────────────────────────────────────────┐
│ TabBar                                          │
├──────────┬──────────────────────┬───────────────┤
│ Props    │                      │ FlowInspector │
│ Signals  │    ReactFlow         │ (fn params,   │
│ Computed │    (fn/code-block/   │  return expr) │
│          │     condition/return)│               │
│ 280px    │    flex: 1           │ 240px         │
└──────────┴──────────────────────┴───────────────┘
```

### 4.2 显示条件

`codeFlowVisible && activeTabType === "component"` 时，左面板从 ComponentPalette+ComponentTree 切换为 ComponentMembersPanel。

具体：修改 `showSidePanels` 逻辑，拆分为两种状态：
- `showDesignPanels`（默认左面板：ComponentPalette + ComponentTree）→ `!codeFlowVisible && !storePanelVisible && tabType === "component"`
- `showCodeFlowPanels`（代码流左面板：ComponentMembersPanel）→ `codeFlowVisible && tabType === "component"`

### 4.3 FlowCanvas 内部调整

当前 FlowCanvas 内部是 `flex: 1` + FlowInspector 240px。布局层面不变——左面板在 DesignPage 层级添加，FlowCanvas 保持内部两栏。

## 五、保存管道变更

### 5.1 component-builder.ts

`buildComponentPart()` 当前从两个来源获取组件声明：
- Props/Events → `getComponentDefinitions()[name]`（注册表）
- Signals/Computeds → `standaloneSignals/standaloneComputeds`（从 codeblock 元素提取）

变更后，**统一从 `componentDefs[name]` 读取**：

```typescript
// 变更前
const registryProps = getComponentDefinitions()[name].props;
componentSignals[s.name] = { type: s.type, ... }; // from standaloneSignals
componentComputeds[c.name] = { type: c.type, ... }; // from standaloneComputeds

// 变更后
const def = getComponentDefinitions()[name] as ComponentDef | undefined;
const componentProps = (def?.props || []).map(p => ({ name, type, defaultValue }));
// signals/computeds 也从 def 读取
```

### 5.2 source-emitter.ts

`emitComponent()` 已经支持发射 prop/signal/computed（line 284-299），无需变更。

### 5.3 store-extractor.ts

signal/computed 不再从 codeblock 元素提取（它们不再是 codeblock 元素），standaloneSignals/standaloneComputeds 可以移除或降低为备用路径。

## 六、解析管道变更

### 6.1 保留 computed body expression

当前 `parseComputed()`（`parser.ts` line 474-491）用 `skipUntilSemicolon()` 丢弃了 expression 内容。

需要改为：收集 `=` 到 `;` 之间的 token 文本作为 expression。

### 6.2 ParseResult 新增 computeds

`parser-types.ts` 的 `ParseContext` 新增 `computeds` 字段：

```typescript
computeds: Array<{ name: string; type: string; expression: string }>;
```

`parser-finalize.ts` 的 `buildParseResult()` 传出。

### 6.3 移除 signal/computed codeblock 元素

Parser 的 `parseSignal()` 和 `parseComputed()` **不再创建 codeblock 元素**。改为：
- signal → 存入 `ctx.signals`（已有字段）+ 存入 `componentDefs[name].signals`
- computed → 存入 `ctx.computeds`（新增字段）+ 存入 `componentDefs[name].computeds`

### 6.4 加载时写入 ComponentDef

`component-service.ts` 的 `loadComponentDefinitionsFromServer()` 已有合并 props 的逻辑，扩展为也包括 signals 和 computeds。

`useProjectInit.ts` 中解析结果也需同步写入 componentDefs。

## 七、实现步骤概览

| # | 步骤 | 涉及文件 |
|---|------|---------|
| 1 | 新增 `ComputedEntry` 类型，扩展 `ComponentSourceDef` | `core/types/project-json.ts` |
| 2 | 新建 `ComponentMembersPanel` 组件 | `components/design/ComponentMembersPanel.tsx` |
| 3 | 修改 DesignPage 布局（codeFlowVisible 时三栏） | `pages/DesignPage.tsx` |
| 4 | 修改 `buildComponentPart()` 从 componentDefs 读取 | `services/codegen/component-builder.ts` |
| 5 | 修改 `parseComputed()` 保留 body expression | `services/codegen/parser.ts` |
| 6 | Parser 不再为 signal/computed 创建 codeblock 元素 | `services/codegen/parser.ts` |
| 7 | ParseContext 和 ParseResult 新增 computeds | `services/codegen/parser-types.ts`, `parser-finalize.ts` |
| 8 | 加载时写入 ComponentDef.computeds | `services/project/component-service.ts`, `features/useProjectInit.ts` |
| 9 | 编写往返测试 | `services/codegen/__tests__/` |

## 八、设计决策记录

| 决策 | 选择 | 原因 |
|------|------|------|
| 面板触发方式 | 绑定 `codeFlowVisible` | 进入组件代码流编辑时自然显示，无需额外 UI |
| 数据源 | 单一 `componentDefs[name]` | 和 storeDefs 模式一致，避免多数据源同步问题 |
| Computed 模型 | 对齐 StoreGetterDef | 复用编辑模式（CodeEditorModal），降低认知负担 |
| signal/computed 与 FlowCanvas 关系 | 完全分离 | FlowCanvas 专做控制流，面板专做接口声明 |
| Props 可选/必需 | 默认值留空 = 必需 | 符合 OmniView 语法语义，无需额外字段 |
| 布局 | 三栏 280px / 1fr / 240px | 和 StorePage 一致 |
| 解析变更 | signal/computed 不再创建 codeblock 元素 | 消除冗余数据，数据从 componentDefs 走 |
