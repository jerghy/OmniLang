# Store Editor Design

> Poiesis 可视化 Store 编辑器 —— 2026-07-10

## 一、概述

Store 是 OmniView 的三大第一公民（component / store / design）之一，但 Poiesis 编辑器目前对 Store 的支持是"黑洞"——parser 跳过 store body（只做括号计数），没有可视化编辑 UI，也没有往返闭环。

本设计补齐 Store 的完整可视化编辑链路。

### 范围

| 项目 | 范围 |
|------|------|
| 全局 Store（独立 `.omni` 文件） | **本次做** |
| 内联 Store（组件内部定义） | 暂不做，后续设计 |
| FN 节点参数设置 | 独立改动，直接改 CodeFlowCanvas，组件 fn 和 store action 共享 |

## 二、UI 布局

```
┌──────────────────────────────────────────────────────────┐
│ TabBar: [TodoStore.omni] [main.omni] ...                  │
├─────────────────────────┬────────────────────────────────┤
│  Store Members          │    CodeFlow Canvas             │
│                         │    (选中 Action 时填充)          │
│  📁 Signals  [+Add]     │                                │
│  ┌───────────────────┐  │    ┌─ fn: add ───────────┐    │
│  │ items: Todo[] = []│  │    │  ├─ code-block      │    │
│  │ filterStatus: Str │  │    │  ├─ condition       │    │
│  │ lastAction: Str   │  │    │  └─ return           │    │
│  └───────────────────┘  │    └──────────────────────┘    │
│                         │                                │
│  📁 Getters  [+Add]     │                                │
│  ┌───────────────────┐  │                                │
│  │ activeCount: i32  │  │                                │
│  │   [点击编辑表达式]  │  │                                │
│  └───────────────────┘  │                                │
│                         │                                │
│  📁 Actions  [+Add]     │                                │
│  ┌───────────────────┐  │                                │
│  │ ● add(text:String)│  │  ← 选中项高亮                   │
│  │   remove(id:i32)  │  │                                │
│  │   clear()          │  │                                │
│  └───────────────────┘  │                                │
└─────────────────────────┴────────────────────────────────┘
```

### 交互规则

- **左侧面板**：成员列表，按 Signals / Getters / Actions 三个分组，每组有 `[+Add]` 按钮
  - Signal 条目：内联编辑名称、类型、默认值
  - Getter 条目：名称 + 类型 + 表达式预览行，点击表达式区域 → 弹出 Monaco 编辑器（复用 `CodeEditorModal`）
  - Action 条目：点击选中 → 中央 CodeFlowCanvas 切换到该 action
- **中央画布**：复用 CodeFlowCanvas（fn / code-block / condition / return 四种节点）
  - 选中 action 时加载对应 elements + wires
  - 未选中时显示空状态 "Select an action to edit"
  - 选中的 action 条目高亮标记

## 三、数据模型

### 3.1 类型定义

```typescript
// poiesis/src/core/types/store.ts（新增文件）

interface StoreSignalDef {
  pid: string;
  name: string;
  type: string;
  defaultValue?: unknown;
}

interface StoreGetterDef {
  pid: string;
  name: string;
  returnType: string;
  expression: string; // 闭包/表达式源码
}

interface StoreActionDef {
  pid: string;
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType?: string;
  isAsync: boolean;
}

interface StoreDefinition {
  name: string;
  filePath: string;
  imports: string[];
  signals: StoreSignalDef[];
  getters: StoreGetterDef[];
  actions: StoreActionDef[];
}
```

### 3.2 PoiesisState 扩展

```typescript
// poiesis/src/store/types.ts 新增字段

interface PoiesisState {
  // ... existing ...

  /** Loaded store definitions, keyed by store name */
  storeDefs: Record<string, StoreDefinition>;

  // Store CRUD
  loadStoreDef: (name: string, def: StoreDefinition) => void;
  removeStoreDef: (name: string) => void;
  addStoreSignal: (storeName: string, signal: StoreSignalDef) => void;
  updateStoreSignal: (storeName: string, pid: string, updates: Partial<StoreSignalDef>) => void;
  removeStoreSignal: (storeName: string, pid: string) => void;
  addStoreGetter: (storeName: string, getter: StoreGetterDef) => void;
  updateStoreGetter: (storeName: string, pid: string, updates: Partial<StoreGetterDef>) => void;
  removeStoreGetter: (storeName: string, pid: string) => void;
  addStoreAction: (storeName: string, action: StoreActionDef) => void;
  updateStoreAction: (storeName: string, pid: string, updates: Partial<StoreActionDef>) => void;
  removeStoreAction: (storeName: string, pid: string) => void;
}
```

### 3.3 Tab 系统

```typescript
// TabType 新增 "store"
type TabType = "component" | "design" | "store";

// 打开 Store 文件时
openTab("src/stores/TodoStore.omni", "TodoStore", "store");
```

### 3.4 Tab 数据缓存

Store 的 CodeFlow 画布数据（action 的 elements + wires）也走 `tabData` 缓存机制，和现有 Tab 切换逻辑一致。切换 Tab 时，当前 action 的 elements/wires 保存到 `tabData[tabId]`。

## 四、.omni 源码格式

### 4.1 PID 注释格式

每个 store 成员尾部附带 `// @pid:xxx` 注释，action body 内节点使用和 CodeFlow fn 完全一致的格式：

```omni
import omniview;
import { storage_get, storage_set } from "std/storage";
// @pid:s1
store TodoStore {
    signal items: Todo[] = []; // @pid:s1_n1
    signal filterStatus: String = "all"; // @pid:s1_n2
    signal lastAction: String = "none"; // @pid:s1_n3

    getter totalCount: i32 = items.len(); // @pid:s1_n4
    getter activeCount: i32 = items.filter(|item| !item.done).len(); // @pid:s1_n5

    action add(text: String) { // @pid:s1_n6
        // @pid:s1_n7
        items = [...items, { id: items.len(), text, done: false }];
        // @pid:s1_n8
        lastAction = "added";
        // @pid:s1_n9
        return;
    }

    action remove(id: i32) { // @pid:s1_n10
        // @pid:s1_n11
        items = items.filter(|item| item.id != id);
        // @pid:s1_n12
        lastAction = "removed";
    }

    action toggle(id: i32) { // @pid:s1_n13
        // @pid:s1_n14
        let found = false;
        // @pid:s1_n15
        items = items.map(|item|
            // @pid:s1_n16
            if item.id == id {
                // @pid:s1_n17
                { ...item, done: !item.done }
            // @pid:s1_n18
            } else {
                // @pid:s1_n19
                item
            }
        );
    }

    action clear() { // @pid:s1_n20
        // @pid:s1_n21
        items = [];
        // @pid:s1_n22
        filterStatus = "all";
        // @pid:s1_n23
        lastAction = "cleared";
    }

    action setFilter(status: String) { // @pid:s1_n24
        // @pid:s1_n25
        filterStatus = status;
    }
}
```

### 4.2 PID 命名规则

- Store PID：`s1`, `s2`, ...（全局递增序号）
- Store 成员 PID：`{storePid}_n{序号}`，如 `s1_n1`, `s1_n2`
- Action body 内 CodeFlow 节点 PID：`{actionPid}_c{序号}`，如 `s1_n6_c1`, `s1_n6_c2`

### 4.3 Signal 特殊语法：`items.length` vs `items.len()`

源码中 `items.length` 是 OmniLang 内置属性语法。解析时将其视为表达式，不做特殊处理。默认值 `[]` 也需要正确解析。

## 五、解析器改造

### 5.1 Parser（parser.ts）

`parseStoreBody()` 从"括号计数跳过"改为"实际解析"：

```
IN_STORE 状态下逐 token 解析：
  1. 遇到 signal → 解析名称、类型、默认值表达式 → 产出 StoreSignalDef
  2. 遇到 getter → 解析名称、返回类型、表达式 → 产出 StoreGetterDef
  3. 遇到 action → 解析名称、参数、返回类型、async 标记
     → 收集 action body 内的 // @pid:xxx 分片代码
     → 调用 parseCodeFlowGraph(actionBody) 恢复 elements + wires
     → 产出 StoreActionDef
  4. 遇到 import / COMMENT_LINE → 正常处理
  5. 遇到 BRACE_CLOSE → 退出 IN_STORE
```

关键点：
- signal 的默认值可能是复合表达式（如 `Todo[] = []`、`String = "all"`）
- getter 的表达式可能是多行（如含闭包和链式调用），需要正确处理到结尾的 `;`
- action body 用和 `parseFnBody` 相同的 token 收集逻辑 + `parseCodeFlowGraph`

### 5.2 StoreExtractor（store-extractor.ts）

现有 `store-extractor.ts` 工作于 CodeBlock 元素层（CodeFlow 画布节点），用于从元素中提取 StoreDef。

改造方向：
- 保留现有逻辑作为"从画布元素提取"路径
- 新增独立的 `parseStoreSource(source: string): StoreDefinition` 函数——直接从 .omni 源码解析 Store，用于 Tab 打开时初始化

实际上，parser.ts 改造后直接在 `IN_STORE` 状态下完成解析，产出 `StoreDefinition`，store-extractor 的现有逻辑可以逐步废弃或仅保留 CodeFlow 相关部分。

## 六、发射器改造

### 6.1 SourceEmitter（source-emitter.ts）

`emitStore` 方法改造为输出带 `// @pid:xxx` 注释的格式：

```
emitStore(store: StoreDefinition):
  1. 输出 store 头部 import
  2. w.emit(`// @pid:${store.pid}`)
  3. w.emit(`store ${store.name} {`)
  4. 遍历 signals: 每个 emit `signal name: type = default; // @pid:xxx`
  5. 遍历 getters: 每个 emit `getter name: type = expr; // @pid:xxx`
  6. 遍历 actions:
     - 输出 action 签名行 `action name(params): returnType async { // @pid:xxx`
     - 调用 compileCodeFlowGraph(action.elements, action.wires, action.fnPid) 输出 body
     - 输出 `}`
  7. w.emit(`}`)
```

### 6.2 Pipeline（pipeline.ts）

`generateOmniFiles()` 中生成 `src/stores/<Name>.omni` 的逻辑改造：
- 从 `storeDefs` 读取而非从 designElements 重新 extract
- 调用 SourceEmitter 的新 `emitStore` 方法

## 七、保存/加载闭环

```
                        ┌─────────────┐
                        │  StoreDef   │
                        │  (内存)      │
                        └──┬──────┬───┘
              emitter.ts   │      │  parser.ts
                           ▼      ▲
                        ┌─────────────┐
                        │  .omni 源码  │
                        │  + @pid注释  │
                        └─────────────┘
```

| 数据 | 存储位置 | 还原方式 |
|------|---------|---------|
| Store 名称 | .omni `store Name` | parser 直接解析 |
| Signal 名称/类型/默认值 | .omni `signal name: Type = default` | parser 直接解析 |
| Getter 名称/类型/表达式 | .omni `getter name: Type = expr` | parser 直接解析 |
| Action 名称/参数/返回类型/async | .omni `action name(params): Ret async` | parser 直接解析 |
| Action body CodeFlow 图 | .omni body 内 `// @pid:xxx` 分片 | `parseCodeFlowGraph(actionBody)` |
| Imports | .omni `import ...` | parser 直接解析 |
| CodeFlow 节点位置 | project.json `nodes` | **已有机制，无需改动** |
| 画布 viewport | project.json `canvasStates` | **已有机制，无需改动** |

**project.json 不需要新增字段。** Store 的所有语义数据完全从 .omni 源码恢复。

## 八、数据流（StorePage 运行时）

### 8.1 打开 Store Tab

```
openTab("src/stores/TodoStore.omni", "TodoStore", "store")
  → 检查 tabData[tabId] 是否缓存
  → 未缓存: read_file → parser → StoreDefinition → storeDefs["TodoStore"]
  → 已缓存: 从 tabData[tabId] 恢复（含当前选中 action 的 CodeFlow 状态）
```

### 8.2 选中 Action 编辑

```
用户点击左侧 action "add"
  → StorePage 从 StoreDefinition.actions 中找到 "add"
  → 获取 action.elements + action.wires
  → 传给 CodeFlowCanvas
  → CodeFlowCanvas 渲染 (pan/zoom 来自 canvasStates)

用户编辑完后切换 action 或 Tab:
  → 当前 CodeFlowCanvas 的 elements/wires 写回 StoreDefinition.actions[actionName]
  → 切换到新 action 或保存到 tabData
```

### 8.3 tabData 扩展

Store Tab 的 `tabData` 条目需要额外字段：

```typescript
tabData[tabId]: {
  // ... 现有字段 ...
  /** Store 专用: 当前选中的 action 名称 */
  selectedActionName?: string | null;
}
```

不需要在 tabData 中冗余存储整个 StoreDefinition —— action 的 elements/wires 直接在 `storeDefs[storeName]` 中就地更新，切换 Tab 时完整保存到 tabData 缓存。

## 九、组件树

### 新增文件

| 文件 | 职责 |
|------|------|
| `poiesis/src/core/types/store.ts` | StoreSignalDef, StoreGetterDef, StoreActionDef, StoreDefinition |
| `poiesis/src/pages/StorePage.tsx` | Store 页面顶层组件（左侧成员列表 + 中央 CodeFlowCanvas） |
| `poiesis/src/components/store/StoreMembersPanel.tsx` | 成员列表面板（Signals/Getters/Actions 分组） |
| `poiesis/src/components/store/SignalEditor.tsx` | Signal 行内/表单编辑器 |
| `poiesis/src/components/store/GetterEditor.tsx` | Getter 行内编辑器 + Monaco 弹窗触发器 |
| `poiesis/src/components/store/ActionEditor.tsx` | Action 编辑入口（选中 → 加载 CodeFlowCanvas） |
| `poiesis/src/store/slices/store-crud.ts` | Store CRUD Zustand slice |

### 修改文件

| 文件 | 改动 |
|------|------|
| `poiesis/src/store/types.ts` | 新增 storeDefs 字段 + CRUD 方法签名 |
| `poiesis/src/store/index.ts` | 注册 store-crud slice |
| `poiesis/src/core/types/design-page.ts` | TabType 新增 `"store"` |
| `poiesis/src/services/codegen/parser.ts` | `parseStoreBody()` 改为实际解析 |
| `poiesis/src/services/codegen/source-emitter.ts` | `emitStore()` 输出 `// @pid:xxx` + action CodeFlow body |
| `poiesis/src/services/codegen/pipeline.ts` | `generateOmniFiles()` 从 storeDefs 读 store |
| `poiesis/src/services/codegen/store-extractor.ts` | 新增 `parseStoreSource()` 或标记废弃 |
| `poiesis/src/services/codegen/types.ts` | StoreDef 对齐新类型（增加 pid 等） |
| `poiesis/src/pages/DesignPage.tsx` | 根据 TabType 路由到 StorePage 或现有 DesignTabContent |
| `poiesis/src/store/operations/tab-lifecycle.ts` | Tab 打开/切换时处理 store 类型 |

## 九、CodeFlowCanvas FN 节点增强（独立改动）

FN 节点（`FnNode.tsx`）当前无法可视化编辑参数。增加一个 **"Params" 按钮**：

- FN 节点新增 params 按钮（如 `+params` 或齿轮图标）
- 点击弹出参数编辑表单（名称 + 类型，可增删行）
- 修改后更新节点的 `props.params` 字符串
- **效果**：组件的 fn 节点和 store 的 action 节点都受益

此改动独立于 Store 编辑器，直接修改 `FnNode.tsx` 即可。

## 十、实现阶段

| 阶段 | 内容 | 预估 |
|------|------|------|
| P1 | 类型定义 + Store CRUD slice + Tab 系统适配 | 基础骨架 |
| P2 | Parser 改造（`parseStoreBody` 实际解析） | 核心解析 |
| P3 | Emitter 改造（带 `// @pid:xxx` 输出 + action CodeFlow） | 核心生成 |
| P4 | StorePage UI（成员面板 + Signal/Getter 编辑器 + Action CodeFlow 集成） | 前端 |
| P5 | CodeFlowCanvas FN 节点参数按钮 | 独立增强 |
| P6 | 保存/加载闭环验证 + 往返测试 | 质量保证 |

## 十一、开放问题

1. **Getter 表达式中的闭包高亮**：Monaco 编辑器中 OmniLang 语法高亮需覆盖闭包 `|param| body` 语法
2. **Signal 默认值的复合类型**：`Todo[] = []` 中的 `Todo[]` 类型引用需要 parser 正确处理
3. **Store 间引用**：`NotificationStore.notify(...)` —— parser 需要识别 `OtherStore.method()` 调用模式
4. **内联 Store**：后续单独设计，编辑组件可复用全局 Store 的 UI 组件
