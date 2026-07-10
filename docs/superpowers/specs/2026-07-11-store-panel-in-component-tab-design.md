# Store 面板（组件标签页内）设计文档

> 日期：2026-07-11 | 状态：设计中 | 关联：Poiesis DesignPage

## 概述

在组件标签页的右侧面板加一个 `📦 Store` 按钮，点击后从右侧滑出全幅抽屉覆盖标签页内容区，内部复用 StorePage 展示 Store 编辑界面。Store 保存为组件内联 Store（写入组件 .omni 文件内部）。

## 1. 触发按钮

**位置**：右侧面板（ContextInspector）标题栏右侧。

```
┌─────────────────────────────┐
│ 属性面板          [📦 Store]│
├─────────────────────────────┤
│  (ContextInspector 内容)    │
```

**样式**：
- 小号按钮，fontSize: 11，border: 1px solid #e2e8f0
- hover 时高亮（background: #f1f5f9）
- 不依赖元素选中状态，始终可见
- 打开状态时按钮高亮（active 态）

## 2. 抽屉布局与动画

### 打开/关闭

```
关闭状态：                      打开状态：
┌──────────┬──────────┬────┐   ┌──────────┬──────────────────────┐
│ 组件面板  │          │属性│   │ 组件面板  │ ● counter    [✕][📋]│
│          │  画布    │面板│   │          ├──────────┬───────────┤
│ 组件树   │          │   │ → │ (收起)   │ Members  │ FlowCanvas│
│          │          │[S]│   │          │          │           │
└──────────┴──────────┴────┘   └──────────┴──────────┴───────────┘
```

- **动画**：`transform: translateX(100%) → translateX(0)`，duration 250ms，easing `cubic-bezier(0.16, 1, 0.3, 1)`
- **遮罩**：不需要（左面板保留可见）
- **关闭方式**：✕ 按钮 / 再次点击 📦 Store / Esc 键
- **标题栏按钮**：`📋` —— 在标签页中打开（创建正式 Store tab 并关闭抽屉）

### 左上角 Store 切换

标题栏左侧显示小球 `● storeName`，点击弹出下拉浮层：

```
┌──────────────────────────────────────────────────┐
│ ● counter                          [✕ 关闭]      │
│ ┌──────┬───────────────────────────────────────┐ │
│ │      │                                       │ │
│ │ 点●  │  ┌────────────────────┐               │ │
│ │      │  │ ● counter     ✕   │               │ │
│ │      │  │ ○ auth        ✕   │  ← 弹出浮层    │ │
│ │      │  │ ○ settings         │               │ │
│ │      │  ├────────────────────┤               │ │
│ │      │  │ + 新建 Store       │               │ │
│ │      │  └────────────────────┘               │ │
│ └──────┴───────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

- 列出所有 Store（全局 + 内联），当前选中的实心圆 ●，其他空心 ○
- 每个 Store 右侧有 ✕ 删除按钮
- 底部 "+ 新建 Store" 触发新建弹窗

## 3. 面板内容

直接复用 `<StorePage>` 组件，渲染 StoreMembersPanel + FlowCanvas。

**无 Store 时**：显示引导页 + `[+ 新建 Store]` 按钮。

**默认行为**：打开面板时自动选中第一个 Store（优先全局 Store，其次内联 Store）。关闭面板后记住上次选择的 Store。

## 4. 状态管理

在 `DesignPageState` 中新增两个字段：

```typescript
storePanelVisible: boolean;          // 抽屉是否打开
storePanelActiveStore: string | null; // 当前展示的 store name
```

不创建 tab，面板状态独立管理。数据流：

```
📦 Store 按钮点击
  → storePanelVisible = true
  → storePanelActiveStore = 第一个 Store（或上次记住的）
  → 抽屉渲染 <StorePage storeName={storePanelActiveStore} />

Store 切换小球点击
  → 弹出浮层列出所有 storeDefs + inlineStoreDefs
  → 点击某个 Store → storePanelActiveStore = newName

关闭抽屉
  → storePanelVisible = false
  → 不重置 storePanelActiveStore
```

**StorePage 适配**：StorePage 当前需要 `tabId` 来读写 `tabData[tabId].selectedActionPid`。抽屉模式下传虚拟 tabId `"__store_panel__"`。

## 5. 保存与解析：组件内联 Store

### 语法（OmniLang 已定义）

```omni
component TodoApp {
    @store TodoStore;       // 引用全局 Store

    store CartStore {       // 内联 Store
        signal items: Array<String> = [];
        signal totalPrice: f64 = 0.0;

        getter itemCount: i32 = items.len();

        action addItem(name: String) {
            // ...
        }
    }

    view { ... }
}
```

### 数据流

```
保存：Store 面板编辑 → StoreDefinition（内存）
  → SourceEmitter.emitComponent() 新增 emit inline store
  → 写入组件 .omni 文件

解析：组件 .omni 文件 → Parser.parseComponentKeyword()
  新增 "store" case → parseStoreBody()
  → StoreDefinition 回到内存
```

### 具体文件改动

| 层 | 文件 | 改动 |
|----|------|------|
| **Parser** | `parser.ts:parseComponentKeyword()` | 新增 `case "store"` → 调用 `parseStoreBody()` |
| **Emitter** | `source-emitter.ts:emitComponent()` | 在 `@store` refs 之后、`prop` 之前 emit 内联 stores |
| **State** | `store/types.ts` 或 PoiesisState | 新增 `inlineStoreDefs: Record<string, StoreDefinition>`，key 为 `"componentName.storeName"` |
| **StorePage** | `StorePage.tsx` | 支持读写 inlineStoreDefs |
| **Pipeline** | `pipeline.ts` | 内联 Store 跳过独立文件生成 |

### 新建 Store 弹窗

新建时提供选择：

```
┌─────────────────────────────┐
│ 新建 Store                  │
│                             │
│ ○ 内联 Store（保存在当前    │
│   组件 .omni 文件中）       │
│                             │
│ ○ 全局 Store（独立文件      │
│   src/stores/xxx.omni）     │
│                             │
│ Store 名称: [__________]    │
│        [取消]  [创建]       │
└─────────────────────────────┘
```

默认选中"内联 Store"。

## 6. 边界情况

| 场景 | 处理 |
|------|------|
| 无 Store | 显示引导页 + 新建按钮 |
| 删除当前查看的 Store | 自动切换到剩余第一个；全删完显示引导页 |
| 抽屉打开时编辑画布 | 画布被覆盖但状态保留；关闭抽屉后可见 |
| 从抽屉升级到标签页 | 标题栏 📋 按钮 → 创建正式 tab → 关闭抽屉 |
| 内联 Store 删除 | 从 inlineStoreDefs 移除，保存时 emitter 自然不输出 |
| 全局 Store 删除 | 从 storeDefs 移除 + 删除独立 .omni 文件（现有逻辑） |

## 7. 不包含的功能（本次不做）

- 在组件画布上拖拽绑定 store signal 到元素（后续）
- 同时编辑多个 Store 的 split 视图
- Store 之间的依赖图可视化
