# OmniDriver API Reference (v2 — Pure Data-Flow Architecture)

> 完整 API 参考。主 skill 文件：[SKILL.md](SKILL.md)

## OmniDriver 主类

### 状态查询

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `O.getState()` | `EditorSnapshot` | 完整快照：mode, elements, wires, selectedElementIds, projectDir, activeFilePath, compileErrors, previewUrl |
| `O.getElements()` | `ElementData[]` | 所有画布元素（纯数据，无方法） |
| `O.elements` | `ElementData[]` | 同 getElements()（getter 别名） |
| `O.getWires()` | `WireData[]` | 所有连线（纯数据，无方法） |
| `O.wires` | `WireData[]` | 同 getWires()（getter 别名） |
| `O.getMode()` | `"design" \| "code"` | 当前模式 |
| `O.getSelectedElement()` | `ElementHandle \| null` | 当前选中 |
| `O.getPreviewUrl()` | `string \| null` | 预览 URL |
| `O.views` | `ElementHandle[]` | 所有视图元素 (nodeType === "view") |
| `O.codeBlocks` | `ElementHandle[]` | 所有代码块 (nodeType === "codeblock") |

### 元素查找

| 方法 | 参数 | 返回值 |
|------|------|--------|
| `O.element(id)` | `string` | `ElementHandle` — 始终返回 handle，用 `.exists()` 检查 |
| `O.findByTag(tag)` | `string` | `ElementHandle[]` |
| `O.find(query)` | `ElementQuery` | `ElementHandle[]` |
| `O.findByProp(prop, value)` | `string, unknown` | `ElementHandle[]` |

```typescript
interface ElementQuery {
  tag?: string;
  nodeType?: "view" | "codeblock" | "group";
  props?: Record<string, unknown>;
  text?: string;
}
```

### 组件创建 — 参数位置速查

**每个便捷方法的参数位置不同！不要假设它们一致。**

| 方法 | 完整签名 | 调用示例 |
|------|---------|---------|
| `component` | `(tag, props?, parentId?)` | `O.component("Button", {label:"OK"}, cardId)` |
| `button` | `(label, props?, parentId?)` | `O.button("OK", {variant:"primary"}, cardId)` |
| `text` | `(label, props?, parentId?)` | `O.text("Hello", {}, cardId)` ← **props 不可跳过！** |
| `heading` | `(label, level?, parentId?)` | `O.heading("Title", "2", cardId)` ← **level 是字符串！** |
| `input` | `(placeholder?, props?, parentId?)` | `O.input("Email...", {}, cardId)` |
| `card` | `(props?, parentId?)` | `O.card({padding:"16px"}, parentId)` |
| `container` | `(props?, parentId?)` | `O.container({}, cardId)` |
| `flex` | `(direction?, gap?, props?, parentId?)` | `O.flex("row", "8px", {}, cardId)` |
| `image` | `(src, alt?, parentId?)` | `O.image("url", "Photo", cardId)` ← **alt 在第2位！** |
| `list` | `(parentId?)` | `O.list(cardId)` |
| `alert` | `(type?, label?, parentId?)` | `O.alert("info", "Note", cardId)` |

```javascript
// ⚠️ 最容易出错的 3 个方法：

// 1. text() — 第2参数是 props，不能跳过！
O.text("Hello", cardId);        // ❌ cardId 被当作 props！
O.text("Hello", {}, cardId);    // ✅ 正确

// 2. image() — 第2参数是 alt，不能跳过！
O.image("url", cardId);         // ❌ cardId 被当作 alt！
O.image("url", "", cardId);     // ✅ 正确

// 3. heading() — 第2参数 level 是字符串！
O.heading("Title", 2, cardId);  // ❌ level 是数字！
O.heading("Title", "2", cardId);// ✅ 正确
```

### 代码块创建

```javascript
O.codeBlock("signal", "name", {
  typeAnnotation: "f64",      // ✅ 有效参数！决定端口类型。支持: i32, f64, bool, String
  source: "signal name: f64 = 3.14",
  parentStore: "",            // 所属 store（可选）
  ports: [],                  // 自定义端口（覆盖默认）
  isFolded: false,            // 是否折叠
});

// typeAnnotation 驱动端口类型：
O.codeBlock("signal", "count", { typeAnnotation: "f64" });
// → 端口类型为 f64 而不是默认的 i32

O.codeBlock("signal", "name", { typeAnnotation: "String" });
// → 端口类型为 String
```

**代码块完整端口参考（22 节点，纯 data 端口）：**

| 类型 | nodeCategory | 输出端口 | 输入端口 | 用途 |
|------|-------------|---------|---------|------|
| `signal` | `"variable"` | `value` (data, out, TYPE) | `set` (data, in, TYPE) | 响应式状态。TYPE 来自 typeAnnotation，默认 i32 |
| `computed` | `"data"` | `value` (data, out, TYPE) | `deps` (data, in, any) | 派生计算。TYPE 来自 typeAnnotation，默认 i32 |
| `store` | `"variable"` | `items` (data, out, any) | — | 数据集合 |
| `customFn` | `"function"` | `return` (data, out, TYPE) | 动态（从 fnParams config 生成） | 自定义函数定义 |
| `return` | `"function"` | — | `value` (data, in, any) | 函数返回点标记 |
| `literal` | `"data"` | `value` (data, out, TYPE) | — | 常量值。config: { value, type } |
| `getVariable` | `"data"` | `value` (data, out, any) | — | 读取变量值。config: { variableName } |
| `setVariable` | `"data"` | — | `value` (data, in, any) | 设置变量。config: { targetSignal } |
| `add` | `"data"` | `result` (data, out, TYPE) | `a`, `b` (data, in, TYPE) | 加法 |
| `sub` | `"data"` | `result` (data, out, TYPE) | `a`, `b` (data, in, TYPE) | 减法 |
| `mul` | `"data"` | `result` (data, out, TYPE) | `a`, `b` (data, in, TYPE) | 乘法 |
| `div` | `"data"` | `result` (data, out, TYPE) | `a`, `b` (data, in, TYPE) | 除法 |
| `compare` | `"data"` | `result` (data, out, bool) | `a`, `b` (data, in, TYPE) | 比较运算。config: { op } |
| `and` | `"data"` | `result` (data, out, bool) | `a`, `b` (data, in, bool) | 逻辑与 |
| `or` | `"data"` | `result` (data, out, bool) | `a`, `b` (data, in, bool) | 逻辑或 |
| `not` | `"data"` | `result` (data, out, bool) | `a` (data, in, bool) | 逻辑非 |
| `concat` | `"data"` | `result` (data, out, String) | `a`, `b` (data, in, String) | 字符串拼接 |
| `branch` | `"flow"` | `true`, `false` (data, out, any) | `condition` (data, in, bool) | 条件分支 |
| `sequence` | `"flow"` | `output0`, `output1`... (data, out, any) | `input` (data, in, any) | 顺序传递。config: { outputs } |
| `forEach` | `"flow"` | `current` (data, out, any), `index` (data, out, i32) | `collection` (data, in, any) | 遍历集合 |
| `updateUIProperty` | `"ui"` | — | `value` (data, in, String) | 更新 UI。config: { targetId, property } |
| `fn` | `"code"` | 源码签名自动生成 | 源码签名自动生成 | 内联代码块 |

```javascript
// ✅ 正确：设置 signal → 连到 set 端口
O.connect(source.id, "result", signal.id, "set");

// ❌ 错误：signal 没有 "value" 输入（value 是输出！）
O.connect(source.id, "result", signal.id, "value");

// ❌ 错误：signal 没有 "input" 端口
source.connectTo(signal, "result", "input");
```

### 连线

```javascript
// 创建连线
O.connect(fromId, fromPort, toId, toPort);
// fromPort 若是事件端口，必须是 UUID (如 "evt-btn-press-001")

// ElementHandle 方式（含端口验证）
el.connectTo(target, "fromPort", "toPort");

// 断开
O.disconnect(wireId);
```

### 模式与编译

```javascript
O.switchMode("design" | "code");

// 编译 — 从 activeFileContent 重建画布，丢弃程序化创建的元素！
const result = await O.compile();
// { success: boolean, errors: Array<{ code, message, line, severity }> }
```

### 等待机制

```javascript
await O.waitForElement("Button", 5000);
await O.waitForWire("evt-btn-press-001", 5000);
await O.waitFor(() => O.getElements().length >= 5, 3000, "timeout");
```

### 撤销/重做

```javascript
O.undo();
O.redo();
```

### 文件操作

```javascript
O.getSourceCode();                               // 读取 activeFileContent
O.setSourceCode("component App { ... }");        // 设置 activeFileContent
O.getActiveFilePath();                           // 当前文件路径
await O.save();                                  // 保存到磁盘
await O.openFile("src/main.omni");               // 打开文件
await O.readFile("src/main.omni");               // 读取文件内容
await O.writeFile("src/main.omni", content);     // 写入文件
```

---

## Event Binding v2（新 API — 替代 createEventHandler）

**v2 架构：事件在 Design 模式绑定，处理函数在 Code 模式 event tab 中定义为 customFn 节点。`createEventHandler` 已废弃。**

```javascript
// bindEvent(element, eventName, fnName, options?)
el.bindEvent("onPress", "handleIncrement");

// 带参数绑定（ParamBinding）
el.bindEvent("onPress", "handleSubmit", {
  params: {
    eventData: { sourceType: "event", value: "event.value", expectedType: "String" },
    formId:    { sourceType: "self", value: "formId", expectedType: "String" },
    defaultTitle: { sourceType: "literal", value: "Untitled", expectedType: "String" },
    appTheme:  { sourceType: "global", value: "app.theme", expectedType: "String" },
  }
});

// 自动发现
el.getAvailableEvents();
// → ["onPress", "onHover", "onFocus", "onLongPress"]

// 查找签名兼容的函数
O.findMatchingFunctions("tab-id", "onPress");
// → [{ fnName: "handleClick", params: [...], returnType: "void" }, ...]
// 传 "*" 搜索所有 tab

// 获取所有事件绑定
O.listAllEventBindings();
// → [{ elementId, tag, eventName, fnName, tabId, paramBindings }, ...]

// 获取单个元素的事件绑定
el.getEventBindings();
// → [{ eventName, fnName, tabId, paramBindings }, ...]

// 解除绑定
el.unbindEvent("onPress");
```

**ParamBinding sourceType 参考：**

| sourceType | 含义 | value 示例 |
|-----------|------|-----------|
| `"self"` | 组件自身的 signal 名 | `"title"`, `"formId"` |
| `"literal"` | 静态字面值 | `"Untitled"`, `"42"`, `"true"` |
| `"event"` | 事件 payload 字段 | `"event.value"`, `"event.target"` |
| `"global"` | 全局 signal | `"app.theme"`, `"app.locale"` |

---

## 🗂️ Code Tab Management（新 API）

**v2 架构：代码模式画布支持多个独立标签页。**

```javascript
O.switchMode("code");

// ── CRUD ──
const tab = O.createTab("event", "Cart Events", "🛒");
// type: "event" | "logic"
// label + icon 可选；自动生成唯一 id

O.switchTab(tab.id);          // 切换到指定 tab
O.removeTab(tab.id);          // 删除 tab（需 >=1 个 tab 剩余）

// ── 查询 ──
O.getActiveTab();             // → CodeTab | null（当前活跃 tab）
O.getTab("tab-id");           // → CodeTab | null
O.listTabs();                 // → Array<{ id, type, label, icon, variableCount, elementCount }>

// ── 变量管理（每个 tab 独立 scope）──
O.addVariable(tab.id, "cartCount", "i32", 0, "购物车计数");
O.addVariable(tab.id, "lastUpdated", "String", "", "最后更新时间");
O.getVariables(tab.id);
// → [{ name, type, defaultValue, description }, ...]
O.removeVariable(tab.id, "lastUpdated");

// ── 画布查询（当前 active tab 或指定 tab）──
O.getTabElements(tab.id);     // → ElementData[] — 该 tab 下的所有元素
O.getTabWires(tab.id);        // → WireData[] — 该 tab 下的所有连线

// ── 批量操作 ──
const clone = O.cloneTab(tab.id, "Clone Label");
// 深拷贝 tab 的 elements + wires + variables，分配新 id

const exported = O.exportTab(tab.id);
// → JSON string: { version, exportedAt, tab: { type, label, icon, variables, elements, wires } }

O.importTab(exported);
// 反序列化 JSON → 创建新 tab（分配新 id，避免冲突）
```

---

## 🔧 Custom Function Operations（新 API）

**v2 架构：customFn 节点替代旧 action 节点。通过 return 节点标记函数返回点，BFS 类型验证确保返回类型一致。**

```javascript
O.switchMode("code");

// ── 创建 ──
const fn = O.customFunction("calculateTotal", [
  { name: "price", type: "f64" },
  { name: "quantity", type: "i32" },
], "f64");
// → ElementHandle
// fn.data.config → { fnName: "calculateTotal", fnParams: [...], fnReturnType: "f64", description: "" }
// fn.data.ports → [{ name: "return", direction: "out", ... }, { name: "price", direction: "in", ... }, ...]

// ── 修改 ──
fn.addParam("discount", "f64", 0.0);   // 动态添加参数 + 端口
fn.setReturnType("f64");               // 修改返回类型（同时更新 return 端口类型）
fn.setDescription("计算含折扣总价");     // 设置描述

// ── return 节点 ──
const ret = O.returnNode(fn.id);
// 关联到指定 customFn；return 节点标记函数体末端
// ret.getPorts() → [{ name: "value", direction: "in", portType: "data", type: "any" }]
// 将上游数据流连到 ret.value → 形成函数体

// ── 函数体构建示例 ──
const addNode = O.codeBlock("add", "addTotal", { typeAnnotation: "f64" });
// price input → add.a, quantity input → add.b
addNode.connectTo(ret, "result", "value");

// ── 验证 ──
const report = O.validateFunction(fn.id);
// → { valid: boolean, returnType: string, errors: string[], reachableReturnPaths: number }
// errors 示例: "Return type mismatch: branch true returns i32 but false returns String"
// BFS 遍历 customFn 下游所有 return 节点，检查类型一致性

// ── 发现 ──
O.listCustomFunctions();
// → [{ fnName, params: FnParam[], returnType, tabId, description }, ...]

O.findFunctionByName("calculateTotal");
// → { fnName, params, returnType, tabId, description, nodeId } | null

O.checkFunctionNameConflict("handleSubmit");
// → [{ fnName, tabId }] — 同名函数列表（用于重名检测）
// 返回 length === 0 表示无冲突

// ── 签名摘要 ──
O.getFunctionSignature(fn.id);
// → { fnName: string, params: FnParam[], returnType: string, bodyNodeCount: number }
```

---

### 组件发现

```javascript
O.listComponents();           // ComponentInfo[] — 所有视图组件元数据
O.getComponentInfo("Button"); // 单个组件的 props, events, styleProperties, hasChildren
```

### 画布诊断

```javascript
const report = O.validate();
// {
//   orphanElements: string[],         // 无父容器且非根节点的 view 元素
//   unwiredEvents: Array<{            // 有事件定义但无 bindEvent 的组件
//     elementId, tag, eventName, eventPortId
//   }>,
//   typeMismatches: Array<{           // 类型不匹配的连线
//     wireId, fromPort, fromType, toPort, toType
//   }>,
//   unreachableBlocks: string[],      // 无数据流入的代码块
//   isValid: boolean                  // 是否通过所有检查
// }

// 状态变更追踪
const before = O.getState();
// ... 做一些操作 ...
const change = O.diff(before);
// { added, removed, modified, addedWires, removedWires }
```

### 状态快照

```javascript
const json = O.exportState();     // 导出画布为 JSON
O.importState(json);              // 从 JSON 恢复画布

// 安全回滚
const token = O.saveSnapshot();   // 保存快照
// ... 高风险操作 ...
O.restoreSnapshot(token);         // 恢复
```

### 预设模板

```javascript
O.snippet("login-form");                           // 登录表单
O.snippet("data-card");                            // 数据卡片
O.snippet("crud-table");                           // CRUD 表格

// 自定义 snippet
O.registerSnippet("my-pattern", (opts) => {
  const card = O.card({ padding: "24px" });
  O.heading(opts.title, "2", card.id);
  return card;
});
O.unregisterSnippet("my-pattern");
O.listSnippets();  // { builtin: string[], custom: string[] }
```

### Visual Indicators

```javascript
O.highlight("el-123");            // 高亮单个元素
O.highlightAll("Button");         // 高亮所有 Button
O.clearHighlights();              // 清除所有高亮
```

### 其他

```javascript
O.help();       // 🚨 每次会话第一步！返回 portReference、eventUuidTable、criticalRules
O.clearAll();   // 清除 Store 中的 elements + wires + selection（不触及源文件）
O.batch(fn);    // 批量操作：O.batch(O => { O.button("A"); O.button("B"); })
O.removeElement(id); // 删除单个元素（级联删除 wires）
```

---

## /api/run 端点 — 编译→运行→测试管线

> 🛜 **前置条件：** 必须先通过 MCP Availability Gate（Browser MCP + Vision MCP 都连接）。

### POST /api/run/start

```javascript
const resp = await fetch("/api/run/start", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    files: { "Counter.js": "export function Counter(mountEl) { ... }" },
    debug: true,   // 注入 __OMNI_DEBUG_WS_URL__
  }),
});
const { success, url, port } = await resp.json();
```

### POST /api/run/stop

```javascript
await fetch("/api/run/stop", { method: "POST" });
// → { success: true }
```

### GET /api/run/status

```javascript
const { running, url } = await fetch("/api/run/status").then(r => r.json());
```

### POST /api/run/write-files

```javascript
await fetch("/api/run/write-files", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ files: { "Counter.js": updatedSource } }),
});
// → { success: true, count: 1 }
```

---

## window.__OMNI_RUNTIME__ — OmniView Runtime API

**仅 OmniView target（`.js` 输出）可用。**

```javascript
const R = window.__OMNI_RUNTIME__;

// Signal System
const count = R.signal(0, { id: "Counter.count", name: "count", type: "number" });
count.get();                        // 0
count.set(5);                       // 触发 DebugBus.check() + 订阅者通知
count.set(prev => prev + 1);        // 函数式更新
count.peek();                       // 读取但不注册依赖

// Computed
const doubled = R.computed(() => count.get() * 2, { id: "Counter.doubled" });

// Effect
const dispose = R.effect(() => { console.log("count:", count.get()); });

// Store
const TodoStore = R.store("TodoStore", () => {
  const items = R.signal([], { id: "TodoStore.items" });
  return { items, add(text) { items.set([...items.get(), text]); } };
});

// DOM
const vnode = R.h("div", { class: "container" },
  R.h("button", { onClick: () => count.set(count.get() + 1) }, "+1"),
  R.h("span", {}, count),
);
R.render(vnode, document.getElementById("root"));

// Control Flow
R.renderConditional(isVisibleSignal, () => R.h("div", {}, "visible"));
R.renderList(itemsSignal, (item, i) => R.h("li", {}, item), item => item.id);
```

### DebugBus

```javascript
const DB = R.DebugBus;
DB.breakpoints.add("Counter.count");
DB.breakpoints.delete("Counter.count");
DB.hasBreakpoint("Counter.count");
DB.paused;
DB.pendingQueue;
DB.step();
DB.resume();
DB.trace("nodeId", "edgeId");
```

### Debug WebSocket Protocol

| 方向 | 消息 type | 触发时机 |
|------|----------|---------|
| App→Editor | `state-change` | 每次 `signal.set()` |
| App→Editor | `paused` | 断点命中，暂停 |
| App→Editor | `resumed` | 全部恢复 |
| App→Editor | `runtime-ready` | WS 连接建立 |
| App→Editor | `execution-trace` | 代码执行追踪 |
| Editor→App | `set-breakpoint` | DebugPanel 添加断点 |
| Editor→App | `remove-breakpoint` | DebugPanel 移除断点 |
| Editor→App | `step` | 单步执行 |
| Editor→App | `resume` | 恢复执行 |

---

## ElementHandle API

```javascript
const el = O.element("some-id");

// 存在与文本
el.exists();                  // boolean
el.text();                    // string（label → text → placeholder → ""）

// 属性
el.setProp(prop, value);      // 设置属性
el.getProp(prop);             // 读取属性
el.getPorts();                // 所有端口
el.getEvents();               // 事件元数据 { onPress: { id, description, payload } }
el.getSource();               // 代码块源文
el.setSource("source code");  // 设置代码块源文
el.data;                      // ElementData 原始数据

// 导航
el.select();                  // 选中元素
el.remove();                  // 删除元素（级联删除 wires）
el.children();                // ElementHandle[] — 子元素（只读）
el.parent();                  // ElementHandle | null

// 连线
el.connectTo(target, "outPort", "inPort");  // 数据流连线（含端口验证）

// 事件绑定 v2
el.bindEvent("onPress", "handlerFn", { params: {...} });
el.unbindEvent("onPress");
el.getAvailableEvents();      // → string[]
el.getEventBindings();        // → [{ eventName, fnName, tabId, paramBindings }, ...]

// 等待
await el.waitForLive(timeout);  // 轮询等待元素出现在 store 中
```

**没有的方法：**
- ❌ `el.textContent()` → 用 `el.text()`
- ❌ `el.innerText()` → 用 `el.text()`
- ❌ `el.children = [...]` → `children()` 是只读 getter
- ❌ `el.createEventHandler()` → 🚫 **已废弃！** 用 `el.bindEvent()` + `O.customFunction()`

---

## WireHandle API

```javascript
const wire = O.connect(fromId, fromPort, toId, toPort);

wire.disconnect();          // 断开连线
wire.source();              // ElementHandle — 源元素
wire.target();              // ElementHandle — 目标元素
wire.data;                  // WireData { id, fromId, fromPort, toId, toPort }
```

---

## 类型定义

> **⚠️ ElementData ≠ ElementHandle：** `getElements()` / `.elements` 返回纯数据，用 `O.element(id)` 获取 Handle。

```typescript
interface EditorSnapshot {
  mode: "design" | "code";
  elements: ElementData[];
  wires: WireData[];
  selectedElementIds: string[];
  projectDir: string | null;
  activeFilePath: string | null;
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;
}

interface ElementData {
  id: string;
  tag: string;
  nodeType: "view" | "codeblock" | "group";
  props: Record<string, unknown>;
  children: string[];
  parentId?: string;
  source?: string;
  ports?: PortDescriptor[];
  isFolded?: boolean;
  outputValue?: unknown;
  nodeCategory?: "function" | "variable" | "data" | "flow" | "ui" | "code";
  config?: Record<string, unknown>;
}

interface PortDescriptor {
  name: string;
  direction: "in" | "out";
  portType: "data" | "flow";  // v2: 绝大多数为 "data"
  type: string;               // i32, f64, bool, String, any, void
  label?: string;
  configurable?: boolean;
  defaultValue?: unknown;
}

interface WireData {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
}

interface CompileResult {
  success: boolean;
  errors: Array<{
    code: string;
    message: string;
    line: number;
    severity: string;
  }>;
}

interface ComponentInfo {
  tag: string;
  category: string;
  description: string;
  props: Record<string, { type: string; default?: unknown; values?: string[]; editor: string }>;
  events: Record<string, { id: string; description: string; payload: string }>;
  styleProperties: string[];
  hasChildren: boolean;
}

interface FnParam {
  name: string;
  type: string;
  default?: unknown;
}

interface CustomFunctionMeta {
  fnName: string;
  fnParams: FnParam[];
  fnReturnType: string;
  description: string;
}

interface CodeTab {
  id: string;
  type: "event" | "logic";
  label: string;
  icon: string;
  graph: { elements: ElementData[]; wires: WireData[] };
  variables: Array<{ name: string; type: string; defaultValue?: unknown; description?: string }>;
}

interface ParamBinding {
  sourceType: "self" | "literal" | "event" | "global";
  value: string;
  expectedType: string;
}

interface EventBindingRecord {
  elementId: string;
  tag: string;
  eventName: string;
  fnName: string;
  tabId: string;
  paramBindings: Record<string, ParamBinding>;
}
```

---

## 事件端口 UUID 完整表

| 组件 | 事件 | 端口 ID |
|------|------|---------|
| Button | onPress | `evt-btn-press-001` |
| Button | onHover | `evt-btn-hover-001` |
| Button | onFocus | `evt-btn-focus-001` |
| Button | onLongPress | `evt-btn-longpress-001` |
| Input | onChange | `evt-input-change-001` |
| Input | onFocus | `evt-input-focus-001` |
| Input | onBlur | `evt-input-blur-001` |
| TextArea | onChange | `evt-textarea-change-001` |
| Select | onChange | `evt-select-change-001` |
| Checkbox | onChange | `evt-checkbox-change-001` |
| Form | onSubmit | `evt-form-submit-001` |

---

## 浏览器 Console & 自动化集成

### 浏览器 DevTools Console

打开 `http://localhost:5174`，按 F12 → Console，直接粘贴脚本执行。

```javascript
const O = window.__OMNI__;
O.help();
O.switchMode("design");
O.clearAll();

const card = O.card({ padding: "24px" });
O.heading("Quick Test", "2", card.id);
O.button("Click", { variant: "primary" }, card.id);

console.log("Elements:", O.elements.length, "Valid:", O.validate().isValid);
```

### Playwright

> **序列化约束：** `page.evaluate()` 只能返回可序列化的值。不要返回 `ElementHandle` 或 `WireHandle`。

```javascript
async function omniEval(page, fn) {
  return page.evaluate(fn);
}

test("create and verify form", async ({ page }) => {
  await page.goto("http://localhost:5174");
  await page.waitForFunction(() => window.__OMNI__);

  await omniEval(page, () => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode("design");
    O.clearAll();
    const card = O.card({ padding: "24px" });
    O.heading("Login", "2", card.id);
    O.input("Email", {}, card.id);
    const btn = O.button("Submit", { variant: "primary" }, card.id);
    btn.bindEvent("onPress", "handleLogin");
  });

  const count = await omniEval(page, () => window.__OMNI__.getElements().length);
  expect(count).toBeGreaterThanOrEqual(4);
});
```

### Puppeteer

```javascript
const puppeteer = require('puppeteer');
const browser = await puppeteer.launch();
const page = await browser.newPage();
await page.goto('http://localhost:5174');
await page.waitForFunction(() => window.__OMNI__);

await page.evaluate(() => {
  const O = window.__OMNI__;
  O.help();
  O.switchMode("design");
  O.clearAll();
  O.card({ padding: "24px" });
});
```
