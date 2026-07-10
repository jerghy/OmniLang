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
│                         │                                │
│  [Imports: ...]  (可折叠)│                                │
└─────────────────────────┴────────────────────────────────┘
```

### 交互规则

- **左侧面板**：成员列表，按 Signals / Getters / Actions 三个分组 + 底部 Imports 区（可折叠）
  - Signal 条目：内联编辑名称、类型、默认值
  - Getter 条目：名称 + 类型 + 表达式预览行，点击表达式区域 → 弹出 Monaco 编辑器（复用 `CodeEditorModal`）
  - Action 条目：显示完整签名 `name(params): RetType` + async 标记，点击选中 → 中央 CodeFlowCanvas 切换到该 action
- **中央画布**：复用 CodeFlowCanvas（fn / code-block / condition / return 四种节点）
  - 选中 action 时，StorePage 将 action 的 elements/wires 写入 `store.elements` / `store.wires`，FlowCanvas 无需改动
  - 未选中时显示空状态 "Select an action to edit"
  - 选中项高亮标记

### 新建 Store

- 入口：DesignPage 的 TabBar `[+]` 按钮下拉菜单中新增 "New Store" 选项
- 弹出输入框：Store 名称（如 `UserStore`）
- 创建后：自动生成 `src/stores/UserStore.omni`，打开 Tab，初始 StoreDefinition 为空（无 signal/getter/action，仅有 imports 和 store 壳）

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
  expression: string; // 闭包/表达式源码（单行或简单多行）
}

interface StoreActionDef {
  pid: string;
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType?: string;
  isAsync: boolean;
  /** CodeFlow 画布数据 —— 和 fn 完全一致 */
  elements: CodeblockElementData[];
  wires: WireData[];
}

interface StoreDefinition {
  pid: string;           // 如 "s1"
  name: string;
  filePath: string;
  imports: string[];     // import 语句的原始文本行
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

  // Store lifecycle
  loadStoreDef: (def: StoreDefinition) => void;
  removeStoreDef: (name: string) => void;

  // Signal CRUD
  addStoreSignal: (storeName: string, signal: StoreSignalDef) => void;
  updateStoreSignal: (storeName: string, pid: string, updates: Partial<StoreSignalDef>) => void;
  removeStoreSignal: (storeName: string, pid: string) => void;

  // Getter CRUD
  addStoreGetter: (storeName: string, getter: StoreGetterDef) => void;
  updateStoreGetter: (storeName: string, pid: string, updates: Partial<StoreGetterDef>) => void;
  removeStoreGetter: (storeName: string, pid: string) => void;

  // Action CRUD
  addStoreAction: (storeName: string, action: StoreActionDef) => void;
  updateStoreAction: (storeName: string, pid: string, updates: Partial<StoreActionDef>) => void;
  removeStoreAction: (storeName: string, pid: string) => void;

  /** 将 store 成员的 PID 批量注册到 pidIndex */
  registerStorePids: (storeDef: StoreDefinition) => void;
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

Store Tab 的 `tabData` 条目扩展：

```typescript
tabData[tabId]: {
  // ... 现有字段（elements, wires, rootOrder, selectedElementIds, filePath）...
  /** Store 专用: 当前选中的 action PID（切换时保存/恢复） */
  selectedActionPid?: string | null;
}
```

切换逻辑：
- 切换到 Store Tab → `store.elements`/`store.wires` 从 `tabData[tabId]` 恢复
- 在 Store Tab 内切换 action → 当前 action 的 elements/wires 写回 `storeDefs[name].actions[pid]`，加载新 action 的 elements/wires 到 `store.elements`/`store.wires`
- 切出 Store Tab → 保存当前 action 数据到 storeDefs，然后正常 tabData 保存

**FlowCanvas 零改动** —— 它始终读 `store.elements` / `store.wires`，和现有 Tab 切换机制完全一致。

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
        // @pid:s1_n6_c1
        items = [...items, { id: items.len(), text, done: false }];
        // @pid:s1_n6_c2
        lastAction = "added";
        // @pid:s1_n6_c3
        return;
    }

    action remove(id: i32) { // @pid:s1_n7
        // @pid:s1_n7_c1
        items = items.filter(|item| item.id != id);
        // @pid:s1_n7_c2
        lastAction = "removed";
    }

    action toggle(id: i32) { // @pid:s1_n8
        // @pid:s1_n8_c1
        let found = false;
        // @pid:s1_n8_c2
        items = items.map(|item|
            // @pid:s1_n8_c3
            if item.id == id {
                // @pid:s1_n8_c4
                { ...item, done: !item.done }
            // @pid:s1_n8_c5
            } else {
                // @pid:s1_n8_c6
                item
            }
        );
    }
}
```

### 4.2 PID 命名规则

- Store PID：`s1`, `s2`, ...（全局递增序号，存储在 `StoreDefinition.pid`）
- Store 成员 PID：`{storePid}_n{序号}`，如 `s1_n1`, `s1_n2`
- Action body 内 CodeFlow 节点 PID：`{actionPid}_c{序号}`，如 `s1_n6_c1`, `s1_n6_c2`

### 4.3 Import 管理

Store 的 import 语句（如 `import { storage_get } from "std/storage";`）存储在 `StoreDefinition.imports: string[]`——每条是一个原始 import 语句文本。

UI 中在左侧面板底部显示为可折叠的 "Imports" 区域，支持添加/删除行。每行点击弹出 Monaco 编辑器或简单的文本输入即可。

## 五、解析器改造

### 5.1 Parser（parser.ts）

`parseStoreBody()` 从"括号计数跳过"改为"实际解析"：

```
IN_STORE 状态下逐 token 解析：
  1. 遇到 signal → 解析名称、类型、默认值表达式 → 产出 StoreSignalDef（含 pid）
  2. 遇到 getter → 解析名称、返回类型、表达式（到 ;） → 产出 StoreGetterDef（含 pid）
  3. 遇到 action → 解析名称、参数、返回类型、async 标记
     → 收集 action body 内的 // @pid:xxx 分片代码 + 源文本
     → 调用 parseCodeFlowGraph(actionBody) 恢复 elements + wires
     → 产出 StoreActionDef（含 pid）
  4. 遇到 import → 收集 import 语句文本行 → 存入 StoreDefinition.imports
  5. 遇到 // @pid:xxx 注释 → 关联到下一个解析出的成员
  6. 遇到 BRACE_CLOSE → 退出 IN_STORE，产出完整 StoreDefinition
```

解析产物通过 parser 的 `ParseResult` 返回，新增字段：
```typescript
interface ParseResult {
  // ... existing ...
  storeDef?: StoreDefinition;  // 如果解析的是 store 文件
}
```

关键点：
- signal 的默认值可能是复合表达式（如 `[]`、`"all"`、`0`），必须保留为原始 tokens 并 emittable
- getter 的表达式处理到 `;` 结束（可能跨行）
- action body 用和 `parseFnBody` 相同的 token 收集逻辑 + `parseCodeFlowGraph`

### 5.2 StoreExtractor（store-extractor.ts）

现有 `store-extractor.ts` 工作于 CodeBlock 元素层。parser.ts 改造后直接在 `IN_STORE` 下产出 `StoreDefinition`，store-extractor 的现有功能逐步废弃。保留文件仅作为 CodeFlow 相关辅助。

## 六、发射器改造

### 6.1 SourceEmitter（source-emitter.ts）

新增/改造 `emitStore` 方法：

```
emitStore(store: StoreDefinition):
  1. 输出 imports（每行一条）
  2. w.emit(`// @pid:${store.pid}`)
  3. w.emit(`store ${store.name} {`)
  4. w.indent()
  5. 遍历 signals:
       emit `signal ${sig.name}: ${sig.type}${defaultClause}; // @pid:${sig.pid}`
  6. 遍历 getters:
       emit `getter ${g.name}: ${g.returnType} = ${g.expression}; // @pid:${g.pid}`
  7. 遍历 actions:
       emit action 签名行 `action ${a.name}(${paramsStr})${retClause}${asyncClause} { // @pid:${a.pid}`
       → 调用 compileCodeFlowGraph(a.elements, a.wires, fnPid) 输出 body
       emit `}`
  8. w.dedent()
  9. w.emit(`}`)
```

### 6.2 Pipeline（pipeline.ts）

`generateOmniFiles()` 中：
- 如果当前文件是 store 类型（`designElements` 来自 store tab），从 `storeDefs[name]` 读取 store 数据，调用 `emitStore`
- 否则走现有组件发射逻辑
- 不需要再从 `designElements` 重新 `extractStores()`

## 七、保存/加载闭环

```
                        ┌───────────────┐
                        │  StoreDef     │
                        │  (storeDefs)  │
                        └──┬────────┬───┘
              emitter.ts   │        │  parser.ts
                           ▼        ▲
                        ┌───────────────┐
                        │  .omni 源码    │
                        │  + @pid注释    │
                        └───────────────┘
```

| 数据 | 存储位置 | 还原方式 |
|------|---------|---------|
| Store pid | .omni `// @pid:s1` | parser 解析 |
| Store 名称 | .omni `store Name` | parser 直接解析 |
| Signal 名称/类型/默认值 | .omni `signal name: Type = default; // @pid:xxx` | parser 直接解析 |
| Getter 名称/类型/表达式 | .omni `getter name: Type = expr; // @pid:xxx` | parser 直接解析 |
| Action 签名（名称/参数/返回类型/async） | .omni `action name(params): Ret async { // @pid:xxx` | parser 解析签名行 |
| Action body CodeFlow 图 | .omni body 内 `// @pid:xxx` 分片 | `parseCodeFlowGraph(actionBody)` |
| Imports | .omni `import ...` 行 | parser 直接解析 |
| CodeFlow 节点位置 | project.json `nodes` | **已有机制，无需改动** |
| 画布 viewport | project.json `canvasStates` | **已有机制，无需改动** |
| 当前选中 action | tabData `selectedActionPid` | Tab 切换恢复 |

**project.json 不需要新增字段。** Store 的所有语义数据完全从 .omni 源码恢复。

### pidIndex

Store 及其成员的 PID 需要在 load/parse 时注册到全局 `pidIndex`：

```typescript
// store-crud.ts
registerStorePids(storeDef: StoreDefinition) {
  pidIndex[storeDef.pid] = { filePath, type: "store", name: storeDef.name };
  for (const sig of storeDef.signals) pidIndex[sig.pid] = { ... };
  for (const get of storeDef.getters) pidIndex[get.pid] = { ... };
  for (const act of storeDef.actions) pidIndex[act.pid] = { ... };
}
```

这样 ArchitecturePage 的泡泡图可以索引和展示 Store 节点。

## 八、数据流

### 8.1 打开 Store Tab

```
openTab("src/stores/TodoStore.omni", "TodoStore", "store")
  → 检查 tabData[tabId] 是否缓存
  → 未缓存:
      read_file → parser（从 .omni 解析出 StoreDefinition + 选中 action 的 elements/wires）
      → loadStoreDef(def) → storeDefs["TodoStore"]
      → 写入 store.elements/wires（默认选中第一个 action 或空）
      → 注册 PID 到 pidIndex
  → 已缓存:
      从 storeDefs + tabData[tabId] 恢复
      → 恢复 store.elements/wires + selectedActionPid
```

### 8.2 Action 切换

```
用户点击左侧 action "toggle"
  → 保存当前 action 数据:
      store.elements/wires → storeDefs["TodoStore"].actions[currentPid].elements/wires
  → 加载新 action 数据:
      storeDefs["TodoStore"].actions[newPid].elements/wires → store.elements/wires
  → tabData[tabId].selectedActionPid = newPid
  → FlowCanvas 自动重渲染（因为 store.elements 引用变了）
```

### 8.3 Action 数据存储规则

`StoreActionDef.elements` 和 `StoreActionDef.wires` 存储在 `storeDefs[storeName]` 中，与 `PoiesisState.elements` / `PoiesisState.wires` 是独立的。只有当前选中的 action 的数据会被拷贝到顶层 `store.elements` / `store.wires` 供 FlowCanvas 消费。

保存/切换时写回，加载时拷出。这保证了：
1. FlowCanvas 无需改动
2. 每个 action 的数据独立持久化
3. Tab 切换、撤销/重做正常工作

### 8.4 保存触发

- **Ctrl+S** → 触发 save pipeline：从 `storeDefs[name]` + `store.elements/wires`（当前 action）生成 .omni → 写文件
- 切换 Tab 前 → 自动保存当前 action 的 elements/wires 到 storeDefs
- 撤销/重做 → 复用现有 `undoStacks[tabId]`，snapshot 内容为整个 `StoreDefinition`

## 九、Undo/Redo

复用现有 `undoStacks[tabId]` 机制。Store Tab 的 undo snapshot 包含：
- 当前选中 action 的 `elements` + `wires`（用于 FlowCanvas undo）
- Store 成员列表的变更（signal/getter/action 的增删改）

每个 CRUD 操作（add/update/remove signal/getter/action）和 FlowCanvas 内元素变更都 push 一个 undo snapshot。

## 十、组件树

### 新增文件

| 文件 | 职责 |
|------|------|
| `poiesis/src/core/types/store.ts` | StoreSignalDef, StoreGetterDef, StoreActionDef, StoreDefinition |
| `poiesis/src/pages/StorePage.tsx` | Store 页面顶层组件（左侧成员列表 + 中央 CodeFlowCanvas + action 切换逻辑） |
| `poiesis/src/components/store/StoreMembersPanel.tsx` | 成员列表面板（Signals/Getters/Actions 分组 + Imports 区） |
| `poiesis/src/components/store/SignalEditor.tsx` | Signal 内联编辑器（名称/类型/默认值） |
| `poiesis/src/components/store/GetterEditor.tsx` | Getter 编辑器（名称/类型 + Monaco 弹窗触发器） |
| `poiesis/src/components/store/ImportsEditor.tsx` | Import 语句编辑器（文本行列表） |
| `poiesis/src/store/slices/store-crud.ts` | Store CRUD Zustand slice + pidIndex 注册 |

### 修改文件

| 文件 | 改动 |
|------|------|
| `poiesis/src/store/types.ts` | 新增 storeDefs + CRUD 方法签名 |
| `poiesis/src/store/index.ts` | 注册 store-crud slice |
| `poiesis/src/core/types/design-page.ts` | TabType 新增 `"store"`，tabData 新增 `selectedActionPid` |
| `poiesis/src/services/codegen/parser.ts` | `parseStoreBody()` 改为实际解析；ParseResult 新增 `storeDef` |
| `poiesis/src/services/codegen/source-emitter.ts` | 新增 `emitStore()` 方法 |
| `poiesis/src/services/codegen/pipeline.ts` | `generateOmniFiles()` 处理 store 类型文件 |
| `poiesis/src/services/codegen/types.ts` | StoreDef 对齐新类型（增加 pid, elements, wires） |
| `poiesis/src/pages/DesignPage.tsx` | TabType=store 时路由到 StorePage；TabBar [+] 新增 "New Store" |
| `poiesis/src/store/operations/tab-lifecycle.ts` | Tab 打开/切换时处理 store 类型的数据加载和保存 |
| `poiesis/src/components/design/DesignTabBar.tsx` | 新增 "New Store" 入口（或单独处理） |

## 十一、CodeFlowCanvas FN 节点增强（独立改动）

FN 节点（`FnNode.tsx`）当前无法可视化编辑参数。增加一个 **"Params" 按钮**：

- FN 节点新增 params 按钮（如 `+params` 或齿轮图标）
- 点击弹出参数编辑表单（名称 + 类型，可增删行）
- 修改后更新节点的 `props.params` 字符串
- **效果**：组件的 fn 节点和 store 的 action 节点都受益

此改动独立于 Store 编辑器，直接修改 `FnNode.tsx` 即可，在 Store 实现之前或并行进行都可以。

## 十二、实现阶段

| 阶段 | 内容 | 预估 |
|------|------|------|
| P1 | 类型定义 + Store CRUD slice + Tab 系统适配 + pidIndex 注册 | 基础骨架 |
| P2 | Parser 改造（`parseStoreBody` 实际解析 signal/getter/action + import） | 核心解析 |
| P3 | Emitter 改造（`emitStore` 带 `// @pid:xxx` + action CodeFlow body） | 核心生成 |
| P4 | StorePage UI（成员面板 + Signal/Getter 编辑器 + action CodeFlow 集成） | 前端 |
| P5 | 保存/加载闭环验证 + 往返测试（包括 pidIndex 和 undo/redo） | 质量保证 |
| P6 | CodeFlowCanvas FN 节点参数按钮 | 独立增强 |

## 十三、@store 引用追踪

组件通过 `@store StoreName;` 声明引用。需要维护双向索引以支持：store 重命名联动更新、store 删除时警告引用者。

### 数据模型

```typescript
// PoiesisState
storeReferences: Record<string, string[]>;  // store name → 引用它的 component 名列表
```

注册/注销时机：
- parser 解析 `@store Xxx` → `registerStoreRef(componentName, storeName)` → 加入索引
- 组件被删除或重解析 → `unregisterAllStoreRefs(componentName)` → 清理该组件所有引用

### 操作联动

| 操作 | 行为 |
|------|------|
| Store 重命名 | `storeReferences[oldName]` 获取引用组件列表 → emitter 重新生成这些组件的 .omni（`@store newName`） |
| Store 删除 | `storeReferences[name]` 非空时弹窗警告，列出引用组件 |
| 组件删除 | 遍历 `ComponentDef.storeRefs` → 从 `storeReferences` 中逐一移除 |

## 十四、Store 间引用验证

Action body 中可能调用另一个 store 的方法（如 `NotificationStore.notify("msg")`），需要验证其存在性。

### 验证规则

在 code-block 源码中匹配 `PascalCase.method()` 模式，校验：
- 目标 store 是否已加载 → 不存在则报 ERROR
- 目标 action 是否存在 → 不存在则报 WARNING

### 实现策略

**全量预扫描**：`open_project` 时扫描项目所有 `.omni` 文件，收集 store 名称和 action 签名列表存入 `storeDefs`。这复用现有项目初始化流程（扫描建 `componentDefs` 和泡泡图），parser 即可直接查表验证。

## 十五、开放问题

1. **Getter 表达式中的闭包高亮**：Monaco 编辑器中 OmniLang 语法高亮需覆盖闭包 `|param| body` 语法
2. **Signal 默认值的复合类型**：`Todo[] = []` 中的 `Todo[]` 类型引用需要 parser 正确处理
3. **内联 Store**：后续单独设计，编辑组件可复用全局 Store 的 UI 组件
