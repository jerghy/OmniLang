# Editor 代码板块架构设计

> 2026-06-15 | 纯数据流可视化编程系统

## 1. 核心理念

### 1.1 设计原则

```
事件绑定 → 设计界面（PropertyPanel）
数据流   → 代码画布（FlowCanvas）
```

- **纯数据流**：所有连线都是数据依赖（data edge），无 flow 边
- **事件脱离代码图**：onPress/onChange 等事件在设计模式 PropertyPanel 绑定
- **函数为组织单元**：customFn 定义签名，return 标记返回点，fn 内联代码块
- **双 store 独立**：useEditorStore（设计）+ useCodeGraphStore（代码）各自管理

### 1.2 执行模型

```
拓扑排序 (Kahn's algorithm)
    ↓
信号变化 → 依赖图遍历 → 计算节点求值 → UI 更新
```

无事件触发、无流控制边、纯数据驱动。

---

## 2. 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                      App.tsx                             │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │ Design Mode   │  │ Code Mode    │  │ PropertyPanel │ │
│  │ (CanvasPanel) │  │ (FlowCanvas) │  │ (右栏)        │ │
│  └──────┬───────┘  └──────┬───────┘  └───────┬───────┘ │
│         │                 │                   │         │
│  ┌──────┴───────┐  ┌──────┴───────┐  ┌───────┴───────┐ │
│  │useEditorStore│  │useCodeGraph  │  │CodeProperty   │ │
│  │  elements[]  │  │  Store       │  │  Panel        │ │
│  │  wires[]     │  │  tabs[]      │  │  (per-tag)    │ │
│  └──────────────┘  │  activeTabId │  └───────────────┘ │
│                    │  selectElement│                    │
│                    └──────┬───────┘                    │
└───────────────────────────┼────────────────────────────┘
                            │
               ┌────────────┼────────────┐
               ▼            ▼            ▼
        nodeRegistry.ts  connection    CodeTabBar.tsx
        (22 节点配置)    Validators    (event/logic 标签)
```

### 2.1 数据流

```
ElementData[] ──→ elementsToNodes() ──→ React Flow Node[]
WireData[]    ──→ wiresToEdges()   ──→ React Flow Edge[]

用户拖拽端口 ──→ handleConnect() ──→ validateConnection()
                                    ├─ error → showToast() 阻止
                                    └─ pass  → AddWire cmd → store
```

### 2.2 双 Store 设计

| | useEditorStore | useCodeGraphStore |
|---|---|---|
| 模式 | 设计 | 代码 |
| 数据 | elements[], wires[] | tabs[].graph.{elements, wires} |
| 撤销 | 全局单栈 | 每 tab 独立栈 |
| 选择 | selectedElementIds | selectedElementIds |
| 持久化 | .omni.json design 字段 | .omni.json codeTabs 字段 |

---

## 3. 22 节点体系

### 3.1 分类

| 类别 | 节点 | 说明 |
|------|------|------|
| **Functions** | customFn, return | 函数定义 + 返回标记 |
| **Variables** | signal, store | 状态变量 + 数据容器 |
| **Data & Logic** | computed, getVariable, setVariable, add, sub, mul, div, compare, and, or, not, concat, literal | 计算与逻辑 |
| **Flow Control** | branch, sequence, forEach | 分支/顺序/遍历 |
| **UI Actions** | updateUIProperty | UI 属性更新 |
| **Code Blocks** | fn | 内联代码块 |

### 3.2 端口类型系统

```
TYPE_COMPAT:
  any     → [any, i32, f64, bool, String, void, MouseEvent]
  i32     → [i32, f64, any]
  f64     → [f64, i32, any]
  bool    → [bool, any]
  String  → [String, any]
  void    → [void, any]
  MouseEvent → [MouseEvent, any]
```

颜色映射 (`TYPE_COLORS`)：

| 类型 | 颜色 |
|------|------|
| any | #6B7280 (灰) |
| i32 | #2563EB (蓝) |
| f64 | #059669 (绿) |
| bool | #D97706 (橙) |
| String | #DB2777 (粉) |
| void | #9CA3AF (浅灰) |
| MouseEvent | #DC2626 (红) |

### 3.3 节点外观

- 统一背景 `#FAFAFA`，边框 `#E5E7EB`
- customFn 特殊紫色边框 `#7C3AED`
- 端口颜色表示数据类型
- 端口标签始终可见（非 hover tooltip）

---

## 4. 连接验证管线

```
validateConnection(fromEl, fromPort, toEl, toPort, existingWires)
  │
  ├─ 1. PORT_NOT_FOUND    (warning) — 端口不存在
  ├─ 2. TYPE_INCOMPATIBLE (error)   — 类型不兼容 → 阻止
  ├─ 3. DUPLICATE_WIRE    (error)   — 重复连线 → 阻止
  ├─ 4. CYCLE_DETECTED    (warning) — 数据循环
  └─ 5. FAN_OUT_WARNING   (warning) — 单输出多输入
```

- **error** → showToast + 阻止连线
- **warning** → showToast + 允许连线

循环检测使用 BFS 反向遍历 predecessor map，O(V+E)。

---

## 5. 事件绑定系统（设计模式）

```
PropertyPanel [+Add Event]
  → 选择事件类型 (onPress/onChange/onHover...)
  → 选择函数（来自 event-code tab 的 customFn）
  → 参数绑定表 ┌────────────────────────────────┐
               │ 参数名  │ 数据源        │ 值    │
               │ name    │ 📩 event     │ —     │
               │ count   │ 🔤 self      │ count │
               │ message │ 🔢 literal   │ "ok"  │
               │ userId  │ 🌐 global    │ uid   │
               └────────────────────────────────┘
```

4 种数据源：
- **📩 event** — 事件原生数据（MouseEvent.x, y 等）
- **🔤 self** — 组件自身状态
- **🔢 literal** — 字面量常量
- **🌐 global** — 全局状态

---

## 6. CodeTab 系统

### 6.1 标签类型

| 类型 | 用途 | 函数可见范围 |
|------|------|-------------|
| `event` | 事件代码 | 事件绑定下拉中可选 |
| `logic` | 逻辑代码 | 仅在代码画布中使用 |

### 6.2 数据模型

```typescript
interface CodeTab {
  id: string;
  name: string;
  type: "event" | "logic";
  graph: {
    elements: ElementData[];
    wires: WireData[];
  };
}
```

---

## 7. 命令系统 & 撤销

```
AddElement     — undo: RemoveElement
RemoveElement  — undo: AddElement (快照恢复)
AddWire        — undo: RemoveWire
RemoveWire     — undo: AddWire (快照恢复)
SetProp        — undo: 恢复旧值
```

- **设计模式**：全局单栈 (`useUndoStore`)
- **代码模式**：每 tab 独立栈 (`CodeGraphStore.pushUndo(tabId, cmd)`)
- 栈上限 200 条

---

## 8. 迁移策略

### 8.1 废弃节点类型

```
v1 (旧): action, onPress, onChange, fn-call, runCodeBlock
v2 (新): 以上全部移除，仅保留纯数据节点
```

### 8.2 迁移流程

```
加载 .omni.json
  → migrateOldCodeGraph(elements, wires)
    → 过滤 DEPRECATED_TAGS
    → 清理孤儿连线
  → 返回干净的数据图
```

迁移在 `codegraph/migration.ts` 中实现，`App.tsx` 的 `openProjectFile` 自动调用。

---

## 9. 服务端管线

### 9.1 API 端点

| 端点 | 用途 | 返回 blockType |
|------|------|---------------|
| `/api/parse-codeblocks` | 解析 .omni → 代码块 | store, signal, computed, **action** |
| `/api/parse-view` | 解析 .omni → 视图树 | — |
| `/api/compile` | 编译 .omni → 目标代码 | — |
| `/api/run/start` | 启动 Vite 运行服务器 | — |

### 9.2 防御策略

服务端 `parse_code_blocks()` 仍返回 `action` blockType（.omni 语言合法构造），
客户端在 `App.tsx` 两道防线过滤：

```typescript
// App.tsx:226 & App.tsx:332
if (b.blockType === "action" || b.blockType === "onPress" || b.blockType === "onChange") continue;
```

---

## 10. Toast 通知系统

替代 `alert()` 弹窗：

```typescript
showToast("类型不兼容：输出 bool 不能连接到输入 String", "error")
showToast("连接将创建数据循环", "warning")
showToast("节点已创建", "info")
```

- 4 秒自动消失
- 三级样式：error(红) / warning(黄) / info(蓝)
- 发布-订阅模式：`onToastChange(fn)` + 全局队列

---

## 11. 文件清单

### 新建
| 文件 | 职责 |
|------|------|
| `canvas/elementsToNodes.ts` | ElementData[] → React Flow Node[] |
| `canvas/wiresToEdges.ts` | WireData[] → React Flow Edge[] |
| `canvas/toast.ts` | Toast 通知系统 |
| `panels/CodePropertyPanel.tsx` | 代码模式属性面板（per-node-type 编辑器） |
| `codegraph/migration.ts` | 旧格式自动迁移 |

### 修改
| 文件 | 变更 |
|------|------|
| `canvas/nodeRegistry.ts` | 22 节点 + 6 类别 + TYPE_COLORS |
| `canvas/nodes/BaseNode.tsx` | 移除 FlowHandle、可见端口标签 |
| `canvas/FlowCanvas.tsx` | 纯数据流渲染、toast、Tab 快捷键 |
| `canvas/connectionValidators.ts` | 5 种校验规则 |
| `codegraph/store.ts` | updateActiveTab()、per-tab undo |
| `store/editor.ts` | 移除 codeElements/codeWires |
| `panels/PropertyPanel.tsx` | EventBindingRow 事件绑定 UI |
| `panels/BlockPalette.tsx` | 移除废弃 Action 节点 |
| `App.tsx` | CodePropertyPanel、迁移集成、action 过滤 |
| `driver/OmniDriver.ts` | createEventHandler 不再创建节点 |

### 已删除
| 文件 | 原因 |
|------|------|
| `canvas/nodes/ViewEventNode.tsx` | 事件节点已移除 |
| `canvas/edges/FlowEdge.tsx` | 流边已移除 |
| `canvas/visualToSource.ts` | 732 行死代码 |
| `canvas/codeGraphToSource.ts` | 250 行死代码 |

---

## 12. 验证状态

```
TypeScript:    零错误
单元测试:      64/64 通过
E2E 浏览器:    连线创建成功、NodePalette 22 节点正确、零控制台错误
Vite build:    成功
```
