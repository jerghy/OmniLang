# DesignCanvas 组件定义面板 (ComponentDefPanel)

**日期**: 2026-07-13
**状态**: 设计中

## 问题

当前 Poiesis 中，组件的 signal/prop **定义**（创建、编辑、删除）只能在 CodeFlowCanvas 中完成。DesignCanvas 上的右侧 ContextInspector 只能做"绑定"——把已有 signal 连到元素的 prop 上。用户希望在 DesignCanvas 无限画布内也能直接管理组件的 prop 和 signal 定义。

## 目标

在 DesignCanvas（无限画布）内添加一个可拖动的浮动面板，支持：
1. 查看/创建/编辑/删除当前组件的 **Props**（组件输入参数）
2. 查看/创建/编辑/删除当前组件的 **Signals**（含 Computed）
3. 面板在画布坐标系内，随缩放/平移同步移动，支持拖拽改变位置
4. 数据与 CodeFlowCanvas 中的定义天然同步（操作同一 `componentDefs` store）

## 非目标

- **不是**替代右侧 ContextInspector——右侧面板继续负责"绑定"
- **不是**管理全局 Store 的 signal——只管当前组件的定义
- 不涉及 Global Store、Inline Store 的 signal 管理（那些是 CodeFlowCanvas 的职责）

## 数据模型（现状复用）

### 操作的数据源

```typescript
// poiesis/src/core/types/project-json.ts

interface ComponentSourceDef {
  name: string;
  filePath: string;
  signals: Array<DefEntry>;    // 组件级 signal
  props: Array<DefEntry>;      // 组件输入参数
  computeds: Array<ComputedEntry>; // 组件级 computed
  events: Array<{ pid: string; name: string; payloadType: string }>;
  fns: Array<FnDef>;
}

interface DefEntry {
  pid: string;
  name: string;
  type: string;
  defaultValue?: unknown;
}

interface ComputedEntry {
  pid: string;
  name: string;
  returnType: string;
  expression: string;
}
```

### Store 路径

- 读取：`componentDefs[activeComponentName].signals / .props / .computeds`
- 写入：新增粒度化 CRUD action，或复用 `setComponentDef` 做不可变更新

## 架构

```
InfiniteCanvas
├── CanvasGrid (背景网格)
├── [transformed div — 受 zoom/pan transform 控制]
│   ├── Artboard (现有：设备预览)
│   └── ComponentDefPanel (新增：可拖动的定义卡片)
└── ZoomControls (固定 overlay，不受 transform 影响)
```

`ComponentDefPanel` 放在 transform 层内，和 Artboard 同级，天然跟随画布缩放/平移。

## 组件设计

### ComponentDefPanel

```
┌─────────────────────────────────┐
│ ≡ 组件名                  [－]  │  ← Header（可拖拽手柄 + 折叠按钮）
├─────────────────────────────────┤
│ 📦 Props                    [+] │  ← 折叠时隐藏以下内容
│   ┌─ title : String          ✕ ┐│
│   └─ count : i32    = 0      ✕ ││
├─────────────────────────────────┤
│ ⚡ Signals                  [+] │
│   ┌─ input : String  = ""    ✕ ┐│
│   └─ items : Vec<i32> = []   ✕ ││
│ 📐 Computed                    │
│   ┌─ itemCount : i32          ✕┐│
│   └─ hasItems : bool          ✕ ││
└─────────────────────────────────┘
```

### 子组件

| 子组件 | 职责 |
|--------|------|
| `ComponentDefPanel` | 外层容器、拖拽、折叠、标题栏 |
| `PropsSubPanel` | Props 列表 + 添加按钮 + 行内编辑 |
| `SignalsSubPanel` | Signals 列表 + Computed 列表 + 添加按钮 + 行内编辑 |
| `DefEntryRow` | 单行：名称、类型、默认值（可选）、删除按钮 |
| `AddDefEntryModal` | 弹出小表单：名称、类型、默认值 |

### 交互

| 交互 | 行为 |
|------|------|
| 拖拽 Header | 移动面板位置 |
| 点击折叠按钮 | 折叠/展开面板内容区 |
| 点击 [+] | 弹出 AddDefEntryModal |
| 点击 ✕ | 删除该项 |
| 点击行（名称/类型/默认值） | 切换为行内编辑模式 |
| 画布缩放/平移 | 面板自动跟随（在 transform 内） |

## 状态设计

### 画布级状态（canvas-store.ts 扩展）

```typescript
// 新增字段
panelPosition: { x: number; y: number };  // 面板在画布坐标中的位置
isPanelCollapsed: boolean;                // 面板是否折叠
```

- `panelPosition` 使用画布坐标系（非屏幕坐标），所以缩放/平移时自动跟随
- 默认位置：Artboard 右侧偏移约 20px，顶部对齐

### 默认位置计算

```typescript
// 伪代码
const defaultPanelPosition = {
  x: artboardLeft + deviceWidth + 40,  // Artboard 右侧
  y: 0,                                // 与 Artboard 顶部对齐
};
```

## Store Actions 设计

### 需要新增的 action（component-lifecycle.ts 扩展）

```typescript
// 添加组件 signal
addComponentSignal(compName: string, signal: DefEntry): void;

// 更新组件 signal
updateComponentSignal(compName: string, pid: string, updates: Partial<DefEntry>): void;

// 删除组件 signal
removeComponentSignal(compName: string, pid: string): void;

// 添加组件 prop
addComponentProp(compName: string, prop: DefEntry): void;

// 更新组件 prop
updateComponentProp(compName: string, pid: string, updates: Partial<DefEntry>): void;

// 删除组件 prop
removeComponentProp(compName: string, pid: string): void;

// 添加组件 computed
addComponentComputed(compName: string, computed: ComputedEntry): void;

// 更新组件 computed
updateComponentComputed(compName: string, pid: string, updates: Partial<ComputedEntry>): void;

// 删除组件 computed
removeComponentComputed(compName: string, pid: string): void;
```

每个 action：
- 从 `componentDefs[compName]` 读取当前数组
- 创建新的不可变副本
- 通过 `setComponentDef(name, updatedDef)` 写回
- 设置 `isDirty: true`
- 记录 undo（可选，先跳过以降低复杂度）

## 数据同步

面板操作 `componentDefs[compName].signals/props/computeds` 数组，这些数据同时被：
- **CodeFlowCanvas**：通过 `useCodeElements()` / `FlowInspector` 读取
- **ContextInspector（右侧面板）**：通过 `useSignalItems()` 读取，用于 SignalSlashPicker 和 SignalPickerPopover
- **Parser/Emitter**：保存时写入 .omni 文件

所以只要操作的是同一个 `componentDefs` 对象，所有视图自动同步——无需额外同步逻辑。

## 关键文件

| 文件 | 变更 |
|------|------|
| `poiesis/src/components/design/ComponentDefPanel.tsx` | **新建**：面板主组件 |
| `poiesis/src/components/design/ComponentDefPanel.css` | **新建**：面板样式 |
| `poiesis/src/components/canvas/InfiniteCanvas.tsx` | **修改**：在 transform 层内渲染 ComponentDefPanel |
| `poiesis/src/components/canvas/canvas-store.ts` | **修改**：新增 panelPosition、isPanelCollapsed |
| `poiesis/src/store/slices/component-lifecycle.ts` | **修改**：新增 component signal/prop/computed CRUD actions |
| `poiesis/src/store/types.ts` | **修改**：新增 action 类型声明 |
| `poiesis/src/pages/DesignPage.tsx` | **可能修改**：传递 activeComponentName |

## 测试策略

1. **单元测试**：component-lifecycle.ts 新增的 CRUD actions
2. **Victauri 浏览器测试**：
   - 面板默认显示在 Artboard 右侧
   - 添加 prop → 面板列表更新 → 右侧 ContextInspector signal picker 可看到
   - 拖拽面板 → 位置更新 → 缩放/平移后面板跟随
   - 折叠/展开 → 内容区切换
   - 删除 signal → CodeFlowCanvas 同步更新

## 风险

| 风险 | 缓解 |
|------|------|
| 面板在 transform 内可能导致文字过小/过大 | CSS `font-size` 用固定 px，不用相对单位，确保可读 |
| 拖拽可能与画布 pan 手势冲突 | Header 区域 `stopPropagation` 阻止事件冒泡到画布 |
| 组件没有 signals/props 时面板显示空 | 显示 "暂无 Prop" / "暂无 Signal" 空状态提示 |

## 实现顺序

1. 扩展 `component-lifecycle.ts`：添加 signal/prop/computed CRUD actions
2. 扩展 `canvas-store.ts`：添加 panelPosition、isPanelCollapsed
3. 创建 `ComponentDefPanel` 组件
4. 在 `InfiniteCanvas` 的 transform 层内集成
5. Victauri 浏览器测试验证
