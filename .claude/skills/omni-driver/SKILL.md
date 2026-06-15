---
name: omni-driver
description: Use when operating the OmniLang visual editor via window.__OMNI__ — from browser DevTools Console (copy-paste scripts), Playwright/Puppeteer E2E tests, AI-driven canvas manipulation, building UI with components and dataflow wiring, diagnosing elements/wires/compile errors, extending existing canvases, recovering from broken canvas state, or running/testing compiled apps via /api/run pipeline (compile→Vite→browser→debug breakpoints)
---

# OmniDriver — AI 操控 OmniLang 编辑器的浏览器 API

`window.__OMNI__` 是挂载在浏览器全局对象上的编辑器操控 API，采用 Locator → Handle → Chainable 模式。**打开 DevTools Console 就能直接用**，无需任何安装或 import。同时也支持 Playwright/Puppeteer 等自动化工具通过 `page.evaluate()` 调用。

| 接入方式 | 场景 | 示例 |
|---------|------|------|
| **浏览器 Console** | 开发调试、快速原型、AI 辅助操控 | 按 F12 → Console → 粘贴脚本 |
| **Playwright** | E2E 测试、CI/CD 自动化 | `page.evaluate(() => window.__OMNI__.help())` |
| **Puppeteer** | E2E 测试、爬虫集成 | `page.evaluate(() => window.__OMNI__.help())` |

## 心智模型（必读）

**编辑器有两层数据 + 两套画布，它们是独立的：**

```
┌─────────────────────────────────────────────────┐
│  Zustand Store (运行时)                           │
│  ← O.button() / setProp() / bindEvent() 操作     │
│  ← 立即生效，但不会持久化到文件                     │
│  ← getElements() / getWires() 读取                │
├─────────────────────────────────────────────────┤
│  .omni 源文件 (持久化)                             │
│  ← setSourceCode() / getSourceCode() 读写         │
│  ← compile() 解析源文件 → 重建 Store 中的元素       │
│  ← save() / readFile() / writeFile() 文件操作      │
└─────────────────────────────────────────────────┘

🎨 设计模式画布 (LayoutView)          💻 代码模式画布 (FlowCanvas)
← 组件拖拽，CSS 样式，布局预览        ← 可视化脚本，节点+连线，纯数据流
← OmniDriver 主战场                   ← @xyflow/react v12 渲染
← O.button()/O.card() 等构建 UI       ← 右键搜索添加节点，拖线连接
```

**关键后果：**
- `compile()` 会**丢弃**程序化创建的元素，从 `.omni` 源文件重建画布
- `getSourceCode()` **不会**反映 `O.button()` 等程序化操作的结果
- 如果想要"创建 UI → 编译验证"，必须先用 `setSourceCode()` 写入完整的 `.omni` 源码
- **纯 data-flow 架构**：所有连线都是数据边（无 flow 边），事件在 Design 模式 PropertyPanel 中绑定

### 代码模式架构（v2）

```
代码模式 = 多标签页 (CodeTab) × 纯数据流画布

┌─────────────────────────────────────────────┐
│ Tab: "handleIncrement" [event]  │ "Main" [logic]  │
├─────────────────────────────────────────────┤
│ 每个 Tab 拥有独立的：                          │
│  - CodeGraph (elements + wires)              │
│  - Variables (signals/stores)               │
│  - Undo Stack (per-tab)                     │
├─────────────────────────────────────────────┤
│ 节点类型 (22 种, 6 个 category):              │
│  function: customFn, return                 │
│  variable: signal, store                    │
│  data:     computed, literal, getVariable,  │
│            add/sub/mul/div, compare,        │
│            and/or/not, concat               │
│  flow:     branch, sequence, forEach        │
│  ui:       updateUIProperty                 │
│  code:     fn, setVariable                  │
└─────────────────────────────────────────────┘

事件绑定在 Design 模式完成（非 Code 模式）：
  viewElement.bindEvent("onPress", "handleIncrement", paramBindings)
  → 设置 element.onPress = "handleIncrement"
  → 处理函数 handleIncrement 在 event tab 中定义为 customFn 节点
```

## When to Use

- **浏览器 DevTools Console** — 打开 F12，直接粘贴脚本操控画布，即时反馈
- 通过 Playwright/Puppeteer 自动化操控编辑器画布
- 批量创建组件和连线
- E2E 测试编辑器功能
- AI agent 需要发现可用组件和事件
- 诊断画布状态：查找孤立节点、缺失事件处理器、编译错误
- **扩展已有画布** — 在现有 UI 上添加新组件和连线
- **修复损坏的画布** — 诊断并修复连线错误、缺失处理器
- **管理代码标签页** — 创建/切换/克隆 event 和 logic tab
- **自定义函数** — 创建和验证 customFn 节点及其函数体

## When NOT to Use

- 直接操作 Zustand store — 用 `useEditorStore` 而不是 `window.__OMNI__`
- 解析 .omni 源码 — 用 `omnilang` Python 库
- 只需要文件读写 — 用后端 API (`/api/file/...`) 而不是 OmniDriver
- 服务端编译 — 用 Python 后端的 compile endpoint

## 🚨 Iron Law: Always Call O.help() First

```
O.help() 是唯一的运行时 API 自省入口。
Object.keys(window.__OMNI__) 返回 []（Proxy 拦截）。
跳过 O.help() = 盲目编程 = 必然出错。
即使你读过 OmniDriver.ts 源码，O.help() 返回的 portReference 和 eventUuidTable 是源码中看不全的。
```

```javascript
const O = window.__OMNI__;
O.help();  // ← 每次会话第一件事！返回 portReference、eventUuidTable、criticalRules
```

## Canonical Script Template

```javascript
// ═══ OmniDriver 脚本模板 — 复制此模板开始 ═══
const O = window.__OMNI__;

// 1. 发现 API（必须第一步！）
O.help();

// 2. 切换到设计模式
O.switchMode("design");
O.clearAll();

// 3. 创建根容器（所有 UI 必须有一个根 Card/Container！）
const root = O.card({ padding: "24px" });

// 4. 创建 UI（用 parentId 嵌套到 root 中）
const btn = O.button("Submit", { variant: "primary" }, root.id);
const txt = O.text("Hello", {}, root.id);

// 5. 数据流 + 事件绑定（纯 data-flow 架构）
const sig = O.codeBlock("signal", "count", { typeAnnotation: "i32" });
// 在设计模式下绑定事件到 code tab 中的 customFn
btn.bindEvent("onPress", "handleIncrement", {
  params: {}  // 此事件无 payload 可传
});

// 6. 切换到 code 模式创建处理函数
O.switchMode("code");
const fnTab = O.createTab("event", "Count Events", "📡");
O.switchTab(fnTab.id);
const incrementFn = O.customFunction("handleIncrement", [], "void");
const ret = O.returnNode(incrementFn.id);

// 7. 验证（必须最后一步！）
const report = O.validate();
if (!report.isValid) console.error("Canvas issues:", report);
console.log("Elements:", O.elements.length, "Wires:", O.wires.length);
```

## 📐 Dataflow Patterns（纯 data-flow 架构）

**v2 架构：没有 flow edge，没有 action 节点。事件通过 bindEvent() 在 design 模式绑定，处理函数在 code 模式 event tab 中定义为 customFn。**

### Pattern 1: Signal → Computed（派生计算）

```
┌────────┐  value   ┌──────────┐  value
│ Signal │ ───────→ │ Computed │ ───────→  (display)
│  i32   │   deps   │   i32    │
└────────┘          └──────────┘
```

```javascript
const price = O.codeBlock("signal", "price", {
  typeAnnotation: "f64",
  source: "signal price: f64 = 9.99",
});
const quantity = O.codeBlock("signal", "quantity", {
  typeAnnotation: "i32",
  source: "signal quantity: i32 = 1",
});
const total = O.codeBlock("computed", "total", {
  typeAnnotation: "f64",
  source: "computed total() -> f64 {\n  price.value * (quantity.value as f64)\n}",
});

// Signal.value → Computed.deps（数据流，不是 set！）
price.connectTo(total, "value", "deps");
quantity.connectTo(total, "value", "deps");
```

### Pattern 2: 多 Signal 汇总到一个 Computed

```javascript
const revenue = O.codeBlock("signal", "revenue", { typeAnnotation: "i32" });
const orders = O.codeBlock("signal", "orders", { typeAnnotation: "i32" });
const totalMetrics = O.codeBlock("computed", "totalMetrics", {
  typeAnnotation: "i32",
  source: "computed totalMetrics() -> i32 {\n  revenue.value + orders.value\n}",
});
// 只需一条线声明依赖关系即可
revenue.connectTo(totalMetrics, "value", "deps");
// computed 源码中直接引用 signal 名
```

### Pattern 3: 事件绑定 + 自定义函数（Event → customFn）

```
Design 模式:
  <Button>  ──bindEvent("onPress", "handleAddToCart", {params:{...}})──→  设置 onPress prop

Code 模式 (event tab):
  ┌──────────────┐  return   ┌────────┐
  │ customFn      │ ───────→ │ return │
  │ handleAddCart │          │  void  │
  └──────────────┘          └────────┘
```

```javascript
// Step 1: Design 模式 — 创建 UI 和 signal
O.switchMode("design");
const card = O.card({ padding: "24px" });
const btn = O.button("Add to Cart", { variant: "primary" }, card.id);
const cartCount = O.codeBlock("signal", "cartCount", { typeAnnotation: "i32" });

// Step 2: Design 模式 — 绑定事件（新 API）
btn.bindEvent("onPress", "handleAddToCart", { params: {} });

// Step 3: Code 模式 — 创建处理函数
O.switchMode("code");
const tab = O.createTab("event", "Cart Events", "🛒");
O.switchTab(tab.id);
const fn = O.customFunction("handleAddToCart", [
  { name: "event", type: "MouseEvent" }
], "void");
const ret = O.returnNode(fn.id);
// 函数体通过 customFn 节点的源码字段实现逻辑
fn.setSource(`action handleAddToCart(event) {
  cartCount.set(cartCount.value + 1)
}`);

// 验证函数签名
const sigReport = O.validateFunction(fn.id);
if (!sigReport.valid) console.error("Function issues:", sigReport.errors);
```

### ❌ Anti-Pattern: 旧架构的 action 节点模式

```javascript
// ❌ v1 旧代码（已废弃）—— createEventHandler 不再创建 action 节点
const { handler } = btn.createEventHandler("onPress", {
  source: "action increment(event) { count.set(count.value + 1) }",
});
handler.connectTo(count, "result", "set");

// ✅ v2 新代码 —— bindEvent + customFn 分离
btn.bindEvent("onPress", "handleIncrement", { params: {} });
// 在 code 模式 event tab 中定义 handleIncrement customFn
```

## Event Binding v2（新 API）

**核心概念：** 事件在 Design 模式绑定，处理函数在 Code 模式 event tab 中定义为 customFn 节点。

```javascript
// bindEvent(element, eventName, fnName, options?)
el.bindEvent("onPress", "handleIncrement");

// 带参数绑定（paramBindings）
el.bindEvent("onPress", "handleSubmit", {
  params: {
    eventData: { sourceType: "event", value: "event.value", expectedType: "String" },
    formId: { sourceType: "self", value: "formId", expectedType: "String" },
    defaultTitle: { sourceType: "literal", value: "Untitled", expectedType: "String" },
  }
});

// 自动发现可用事件
const available = el.getAvailableEvents();
// → ["onPress", "onHover", "onFocus", "onLongPress"]

// 查找签名兼容的函数
const matching = O.findMatchingFunctions("tab-id", "onPress");
// → [{ fnName: "handleClick", params: [...], returnType: "void" }, ...]

// 获取所有事件绑定
const allBindings = O.listAllEventBindings();
// → [{ elementId, tag, eventName, fnName, tabId, paramBindings }, ...]

// 解除绑定
el.unbindEvent("onPress");
```

**ParamBinding sourceType:**
| sourceType | 含义 | value 示例 |
|-----------|------|-----------|
| `"self"` | 组件自身的 signal 名 | `"title"`, `"formId"` |
| `"literal"` | 静态字面值 | `"Untitled"`, `"42"` |
| `"event"` | 事件 payload 字段 | `"event.value"`, `"event.target"` |
| `"global"` | 全局 signal | `"app.theme"`, `"app.locale"` |

## 🗂️ Code Tab Management（新 API）

**v2 架构：代码画布支持多个标签页。每个 tab 独立管理 elements/wires/variables/undo。**

```javascript
O.switchMode("code");

// 创建标签页
const logicTab = O.createTab("logic", "Main Logic", "📄");
const eventTab = O.createTab("event", "Button Handlers", "🔘");

// 查询
const allTabs = O.listTabs();
// → [{ id, type, label, icon, variableCount, elementCount }, ...]
const current = O.getActiveTab();
const tab = O.getTab("tab-id");

// 切换与删除
O.switchTab(eventTab.id);
O.removeTab(logicTab.id);  // 不能删除最后一个 tab

// 变量管理（每个 tab 独立 scope）
O.addVariable(eventTab.id, "clickCount", "i32", 0, "按钮点击次数");
O.addVariable(eventTab.id, "lastEvent", "String", "", "最后一次事件类型");
const vars = O.getVariables(eventTab.id);
// → [{ name, type, defaultValue, description }, ...]
O.removeVariable(eventTab.id, "lastEvent");

// 标签页级别画布查询
const elements = O.getTabElements(eventTab.id);
const wires = O.getTabWires(eventTab.id);

// 批量操作
const clone = O.cloneTab(eventTab.id, "Button Handlers Copy");
const exported = O.exportTab(eventTab.id);  // → JSON string
O.importTab(exported);                       // → new tab
```

## 🔧 Custom Function Operations（新 API）

**v2 架构：customFn 节点替代旧 action 节点。通过 return 节点和 BFS 类型验证组成函数体。**

```javascript
O.switchMode("code");

// 创建 customFn 节点
const fn = O.customFunction("calculateTotal", [
  { name: "price", type: "f64" },
  { name: "quantity", type: "i32" },
], "f64");

// fn.data.config → { fnName: "calculateTotal", fnParams: [...], fnReturnType: "f64", description: "" }

// 动态修改参数
fn.addParam("discount", "f64", 0.0);
fn.setReturnType("f64");
fn.setDescription("计算含折扣的总价");

// 创建 return 节点（标记函数返回点，必须连接为函数体末端）
const ret = O.returnNode(fn.id);
// ret.getPorts() → [{ name: "value", direction: "in", portType: "data", type: "any" }]

// 函数体构建（在 customFn 和 return 之间加数据流）
const addNode = O.codeBlock("add", "addPrice", { typeAnnotation: "f64" });
// price 参数 → add.a, quantity 参数 → add.b (通过 literal/connect 等)
addNode.connectTo(ret, "result", "value");

// BFS 返回类型一致性验证（复用 CodePropertyPanel ReturnValidator）
const report = O.validateFunction(fn.id);
// → { valid: boolean, returnType: string, errors: string[], reachableReturnPaths: number }

// 函数发现
const allFns = O.listCustomFunctions();
// → [{ fnName, params, returnType, tabId, description }, ...]
const found = O.findFunctionByName("calculateTotal");
// → { ... } or null
const conflicts = O.checkFunctionNameConflict("handleSubmit");
// → [{ fnName, tabId }] or []

// 获取函数签名摘要
const sig = O.getFunctionSignature(fn.id);
// → { fnName, params: [...], returnType, bodyNodeCount }
```

## 参数位置速查表（必读！）

**每个便捷方法的参数位置不同！不要假设它们一致：**

| 方法 | 签名 | 示例 |
|------|------|------|
| `button` | `(label, props?, parentId?)` | `O.button("OK", {variant:"primary"}, card.id)` |
| `text` | `(label, props?, parentId?)` | `O.text("Hello", {}, card.id)` ← props 不可跳过！ |
| `heading` | `(label, level?, parentId?)` | `O.heading("Title", "2", card.id)` ← level 是字符串 |
| `input` | `(placeholder?, props?, parentId?)` | `O.input("Email...", {}, card.id)` |
| `card` | `(props?, parentId?)` | `O.card({padding:"16px"}, parent.id)` |
| `container` | `(props?, parentId?)` | `O.container({}, card.id)` |
| `flex` | `(direction?, gap?, props?, parentId?)` | `O.flex("row", "8px", {}, card.id)` |
| `image` | `(src, alt?, parentId?)` | `O.image("url", "Photo", card.id)` |
| `list` | `(parentId?)` | `O.list(card.id)` |
| `alert` | `(type?, label?, parentId?)` | `O.alert("info", "Note", card.id)` |

```javascript
// ❌ 常见错误：把 parentId 当 props 传入（text 最常见！）
O.text("Hello", card.id);         // card.id 被当作 props！元素不会嵌套！
O.text("Hello", {}, card.id);     // ✅ 正确：props = {}，parentId = card.id

// ❌ 常见错误：heading level 用数字
O.heading("Title", 2, card.id);   // level 是数字，类型错误
O.heading("Title", "2", card.id); // ✅ 正确：level 是字符串 "2"

// ❌ 常见错误：image 也可以跳过 alt
O.image("url", card.id);          // card.id 被当作 alt！图片不会嵌套！
O.image("url", "", card.id);      // ✅ 正确：alt = ""，parentId = card.id
```

## 快速开始

```javascript
const O = window.__OMNI__;

// ── 程序化操控模式 ──
O.help();                      // ← 第一步！
O.switchMode("design");
O.clearAll();

// 嵌套 UI：parentId 作为最后一个参数
const card = O.card({ padding: "20px" });
const btn = O.button("Submit", { variant: "primary" }, card.id);
const txt = O.text("Hello", {}, card.id);

// 事件绑定（v2 — event prop 方式）
btn.bindEvent("onPress", "handleSubmit");

// 数据流连线
const sig = O.codeBlock("signal", "count", { typeAnnotation: "i32", source: "signal count: i32 = 0" });

// ── Code 模式 — 创建处理函数 ──
O.switchMode("code");
const tab = O.createTab("event", "Submit Handler", "📡");
O.switchTab(tab.id);
const fn = O.customFunction("handleSubmit", [{ name: "event", type: "MouseEvent" }], "void");
fn.setSource(`action handleSubmit(event) {
  count.set(count.value + 1)
}`);
const ret = O.returnNode(fn.id);

// ── 源码驱动模式 ──
O.setSourceCode(`component App {
  signal count: i32 = 0
  fn increment() { count += 1 }
  view { <Button onPress={increment}>+1</Button> }
}`);
O.switchMode("code");
const result = await O.compile();
```

## connectTo vs parentId — 决策流程

```
需要建立元素间关系？
├─ 布局嵌套（父子容器关系） → 用 parentId 参数！
│   O.button("OK", {}, card.id)  ← 最后一个参数是 parentId
│   O.card({ children: [btn, txt] })  ← 或 props 中传 children 数组
│
└─ 数据流连线（信号传递） → 用 connectTo() / connect()！
    handler.connectTo(signal, "result", "set")  ← 端口数据流
    O.connect(btn.id, "evt-btn-press-001", handler.id, "call")
```

> **铁律：`connectTo()` 创建的是端口数据流连线，不会影响元素在 UI 树中的父子位置！**

## 核心 API 速查

### 代码块端口参考（纯 data 端口）

| 类型 | nodeCategory | 输出端口 | 输入端口 | 用途 |
|------|-------------|---------|---------|------|
| `signal` | `"variable"` | `value` (data, out) | `set` (data, in) | 响应式状态变量 |
| `computed` | `"data"` | `value` (data, out) | `deps` (data, in) | 派生计算值 |
| `store` | `"variable"` | `items` (data, out) | — | 数据集合 |
| `customFn` | `"function"` | `return` (data, out) | 动态（从 fnParams config 生成） | 自定义函数定义 |
| `return` | `"function"` | — | `value` (data, in) | 函数返回点 |
| `literal` | `"data"` | `value` (data, out) | — | 常量值 |
| `add`/`sub`/`mul`/`div` | `"data"` | `result` (data, out) | `a`, `b` (data, in) | 算术运算 |
| `compare` | `"data"` | `result` (data, out, bool) | `a`, `b` (data, in) | 比较运算 |
| `and`/`or` | `"data"` | `result` (data, out, bool) | `a`, `b` (data, in, bool) | 逻辑运算 |
| `not` | `"data"` | `result` (data, out, bool) | `a` (data, in, bool) | 逻辑非 |
| `concat` | `"data"` | `result` (data, out, String) | `a`, `b` (data, in, String) | 字符串拼接 |
| `branch` | `"flow"` | `true`, `false` (data, out) | `condition` (data, in, bool) | 条件分支 |
| `sequence` | `"flow"` | `output0`, `output1`... (data, out) | `input` (data, in) | 顺序传递 |
| `forEach` | `"flow"` | `current` (data, out), `index` (data, out, i32) | `collection` (data, in) | 遍历集合 |
| `getVariable` | `"data"` | `value` (data, out) | — | 读取变量 |
| `setVariable` | `"data"` | — | `value` (data, in) | 设置变量 |
| `updateUIProperty` | `"ui"` | — | `value` (data, in, String) | 更新 UI 属性 |
| `fn` | `"code"` | 源码自动生成 | 源码自动生成 | 内联代码块 |

```javascript
// ✅ 正确：设置 signal → 连到 set 端口
O.connect(source.id, "result", signal.id, "set");

// ❌ 错误：signal 的输入端口是 "set"，不是 "value"（value 是输出！）
O.connect(source.id, "result", signal.id, "value");

// ❌ 错误：signal 没有 "input" 端口
source.connectTo(signal, "result", "input");
```

### 代码块创建

```javascript
O.codeBlock("signal", "name", {
  typeAnnotation: "f64",     // ✅ 有效参数！决定端口类型，默认 "i32"
  source: "signal name: f64 = 3.14",
  parentStore: "",           // 所属 store（可选）
  ports: [],                 // 自定义端口（覆盖默认）
  isFolded: false,           // 是否折叠
});
// typeAnnotation 支持: "i32", "f64", "bool", "String"，或留空使用默认值
```

### 创建元素

| 操作 | 代码 | 注意 |
|------|------|------|
| 通用创建 | `O.component("Button", props, parentId)` | parentId 可选 |
| 便捷方法 | `O.button(label, props, parentId)` | 最后一个参数可选，见参数速查表 |

### 嵌套子元素

```javascript
// ✅ 方式 1：parentId 参数
const card = O.card({ padding: "16px" });
const btn = O.button("Click", { variant: "primary" }, card.id);

// ✅ 方式 2：children 数组
const card2 = O.card({
  padding: "24px",
  children: [O.button("Submit"), O.text("Welcome", {}, null)]  // null parentId 因为 card 会自动设置
});

// ❌ WRONG: children() 是只读的
card.children = [btn];

// ❌ WRONG: connectTo() 是数据流，不是布局
btn.connectTo(card);  // 这是端口连线，不是父子关系！
```

### 查找与等待

| 操作 | 代码 |
|------|------|
| 按 ID | `O.element(id)` → 始终返回 handle，用 `.exists()` 检查 |
| 按标签 | `O.findByTag("Button")` → `ElementHandle[]` |
| 按属性 | `O.findByProp("label", "Submit")` → `ElementHandle[]` |
| 通用查询 | `O.find({ tag, nodeType, props, text })` |
| 等待出现 | `await O.waitForElement("Button", timeout)` |
| 所有元素数据 | `O.elements` — `ElementData[]`（纯数据，无方法！） |
| 所有连线数据 | `O.wires` — `WireData[]`（纯数据，无方法！） |

> **⚠️ ElementData ≠ ElementHandle：** `getElements()` / `.elements` 返回**纯数据对象**（有 `parentId`、`props`、`children` 字段），**没有** `.parent()`、`.text()`、`.setProp()` 等方法。用 `O.element(id)` 获取 `ElementHandle`。

### ElementHandle 常用操作

```javascript
const el = O.element("some-id");
el.exists();                   // boolean — 元素是否存在
el.text();                     // string — 显示文本（label → text → placeholder）
el.setProp("label", "New");    // 设置属性
el.getProp("label");           // 读取属性
el.select();                   // 选中
el.remove();                   // 删除
el.children();                 // ElementHandle[] — 子元素（只读）
el.parent();                   // ElementHandle | null
el.getPorts();                 // 所有端口（view 自动含事件端口）
el.getEvents();                // 事件元数据 { onPress: { id, description, payload } }
el.getAvailableEvents();       // string[] — 可用事件名列表
el.connectTo(target, "out", "in"); // 数据流连线（含端口验证）
el.bindEvent("onPress", "handleFn", options);  // 事件绑定 v2
el.unbindEvent("onPress");     // 解除事件绑定
el.data;                       // 原始 ElementData
```

## 📐 Layout Architecture Patterns

**复杂 UI 需要多层嵌套。** 以下为最常见的布局架构：

### Sidebar + Main

```javascript
const root = O.card({ padding: "0" });
const layout = O.flex("row", "0", {}, root.id);
const sidebar = O.flex("column", "8px", { width: "256px", padding: "24px 0" }, layout.id);
const main = O.flex("column", "24px", { flex: "1", padding: "32px" }, layout.id);
// sidebar ← 放导航  |  main ← 放内容
```

### Header + Content

```javascript
const root = O.flex("column", "0", {});  // 根可以是 Flex！
const header = O.flex("row", "16px", { padding: "16px 24px" }, root.id);
const content = O.card({ padding: "32px" }, root.id);
```

### Card Grid

```javascript
const grid = O.flex("row", "16px", {}, parent.id);
const c1 = O.card({ flex: "1", padding: "24px" }, grid.id);
const c2 = O.card({ flex: "1", padding: "24px" }, grid.id);
const c3 = O.card({ flex: "1", padding: "24px" }, grid.id);
```

### Form Layout

```javascript
const form = O.card({ padding: "32px" });
O.heading("Title", "2", form.id);
O.input("Field 1", {}, form.id);
O.input("Field 2", {}, form.id);
const actions = O.flex("row", "8px", {}, form.id);
O.button("Cancel", { variant: "outline" }, actions.id);
O.button("Submit", { variant: "primary" }, actions.id);
```

> **Card vs Flex vs Container：** `Card` = 视觉分组（边框阴影），`Flex` = 方向排列（纯布局），`Container` = 纯包裹（无样式无方向）。

## 事件绑定 v2 速查

### 事件名 vs 端口 UUID

```javascript
// bindEvent() — 传事件名（React 风格）
btn.bindEvent("onPress", "handleIncrement");
btn.bindEvent("press");           // ❌ DOM 风格，会抛异常
btn.bindEvent("evt-btn-press-001"); // ❌ UUID，会抛异常

// connect() 手动连线 — 传端口 UUID（高级用法，不推荐）
O.connect(btn.id, "evt-btn-press-001", target.id, "somePort"); // ✅ UUID
```

> **铁律：`bindEvent(eventName)` 用事件名，`connect(fromPort)` 用 UUID。**

### 事件端口 UUID 表

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

## 诊断与调试

```javascript
const state = O.getState();

// 查找孤立节点（无连线的代码块）
const connectedIds = new Set(state.wires.flatMap(w => [w.fromId, w.toId]));
const isolated = O.codeBlocks.filter(cb => !connectedIds.has(cb.id));

// 查找缺失事件处理器的组件
for (const el of O.views) {
  const info = O.getComponentInfo(el.tag);
  if (!info) continue;
  for (const [evt, meta] of Object.entries(info.events)) {
    const isWired = state.wires.some(w => w.fromId === el.id && w.fromPort === meta.id);
    if (!isWired) console.warn(`${el.tag} missing ${evt}`);
  }
}

// 组件发现
O.listComponents();           // 所有可用组件及 props/events
O.getComponentInfo("Button"); // 单个组件详情

// 一键健康检查
const report = O.validate();
// { orphanElements, unwiredEvents, typeMismatches, unreachableBlocks, isValid }

// 函数签名验证
const fn = O.customFunction("myFn", [{ name: "x", type: "i32" }], "i32");
const sigReport = O.validateFunction(fn.id);

// 查看所有事件绑定
const bindings = O.listAllEventBindings();

// 代码 tab 信息
O.listTabs();
O.getVariables("tab-id");
```

## 🚀 Run & Test Pipeline — 编译→运行→浏览器实测

**OmniLang 的"运行"是将 `.omni` 编译为可执行代码、启动 Vite dev server、在浏览器中验证的闭环。**

### 🛜 MCP Availability Gate（必须第一步！）

**在走 Run Pipeline 之前，必须先确认 MCP 工具已连接。** 没有 MCP 就不能做浏览器验证，等于只跑了一半。

```
检查清单：
1. mcp__playwright__browser_navigate — 存在？（Browser MCP）
2. mcp__agent-vision__analyze_image — 存在？（Vision MCP，截图对比用）
```

**如果任一 MCP 缺失：** 🛑 **立即停止**。告诉用户：
> "Browser MCP 和/或 Vision MCP 未连接。请运行 `/mcp` 重新连接，然后重新执行 Run Pipeline。"

**没有 MCP 时不要做的事：**
- ❌ 不要用 `curl` 或 Bash 脚本绕过浏览器验证——curl 看不到 DOM 渲染结果
- ❌ 不要写 Node.js Playwright 脚本替代——那些不经过 MCP 协议
- ❌ 不要仅凭"编译成功"就认为 run 没问题——codegen 正确 ≠ runtime 正确
- ❌ 不要用"我手动验证过了"跳过后面的 MCP 步骤

**MCP 不可用时的降级路径：** 只能走到 `/api/run/start` 确认 Vite 启动成功，然后**暂停**，等用户连上 MCP 后再继续浏览器验证。不能声称"测试通过"。

**🔴 铁律：没有 MCP 验证的 Run Pipeline 不是完整的测试。编译成功 ≠ 运行正确。**

### 管线总览

```
.omni 源码
  → /api/compile (target=react|omniview)
  → /api/run/start (scaffold Vite + npm install + npx vite)
  → 浏览器打开 http://localhost:<port>
  → Playwright MCP / DevTools Console 验证
  → /api/run/write-files (热更新，Vite HMR 自动重载)
  → /api/run/stop
```

### /api/run 端点

| 端点 | 方法 | 用途 |
|------|------|------|
| `/api/run/start` | POST | 接收编译文件 → scaffold Vite 项目 → `npm install` → 启动 `npx vite` |
| `/api/run/stop` | POST | 杀掉 Vite 子进程 |
| `/api/run/status` | GET | `{ running: bool, url: string|null }` |
| `/api/run/write-files` | POST | 热更新已编译文件到 `.omni-run/src/`（Vite HMR 自动重载浏览器） |

### 双 Target 策略

| Target | 扩展名 | Runtime | Debug | 适用场景 |
|--------|--------|---------|-------|---------|
| `react` | `.tsx` | React 18 + Zustand + omni-runtime.ts | useTrackedSignal 包装（每种状态类型需单独适配） | 生产部署 |
| `omniview` | `.js` | 纯 JS signal runtime (omniview-runtime.ts) | **signal.set() 单一拦截点**——所有状态类型统一 | 开发/Debug |

**OmniView target 优势：**
- 无框架依赖（package.json 不含 react/zustand）
- `signal.set()` 是唯一拦截点——breakpoint/computed/effect 全部经过 DebugBus
- 细粒度 DOM 绑定，无 VDOM diff
- `window.__OMNI_RUNTIME__` 暴露全部内部状态供浏览器 Console 直接操控

### window.__OMNI_RUNTIME__ — OmniView Runtime API

**仅 OmniView target 可用**（React target 使用 `window.__OMNI__` 的 Zustand store）。

```javascript
const R = window.__OMNI_RUNTIME__;
// R.DebugBus    — 断点/暂停/单步状态机
// R.signal()    — 创建响应式信号
// R.computed()  — 派生计算
// R.effect()    — 副作用
// R.store()     — 命名空间信号集合
// R.h()         — 虚拟 DOM 节点
// R.render()    — 挂载 DOM 树
// R.traceExecution() — 执行追踪（FlowCanvas 高亮）
```

## 🔬 Node Registry（22 节点，6 个 category）

```javascript
// 在浏览器 Console 中查看当前注册表：
// O.help().portReference — 动态生成的端口结构

// 6 个 category:
// function — customFn, return
// variable — signal, store
// data     — computed, literal, getVariable, add, sub, mul, div, compare, and, or, not, concat
// flow     — branch, sequence, forEach
// ui       — updateUIProperty
// code     — fn, setVariable
```

## 可用组件

19 种视图组件：Container, Flex, Grid, Card, Text, Heading, Button, Input, TextArea, Select, Checkbox, List, ListItem, Image, Spinner, Progress, Alert, Link, Form

```javascript
const info = O.getComponentInfo("Button");
// { tag, category, description, props, events, styleProperties, hasChildren }
```

## 文件操作

```javascript
O.getActiveFilePath();                     // 当前文件路径
O.getSourceCode();                         // .omni 源码内容
O.setSourceCode("component App { ... }");  // 设置源码
await O.save();                            // 保存到磁盘
await O.openFile("src/main.omni");         // 打开文件
await O.readFile("src/main.omni");         // 读取文件
await O.writeFile("src/main.omni", code);  // 写入文件
```

## 📖 Recipe Book（复制即用的完整模板）

**先检查 `O.listSnippets()` 是否有内置模板，再查 Recipe Book，都没有才从零构建。**

### Recipe 1: Counter（按钮 + 计数）— v2 架构

```javascript
const O = window.__OMNI__;
O.help();
O.switchMode("design");
O.clearAll();

// UI
const card = O.card({ padding: "24px" });
O.heading("Counter", "2", card.id);
const display = O.text("0", {}, card.id);
const btn = O.button("+1", { variant: "primary" }, card.id);

// Signal
const count = O.codeBlock("signal", "count", {
  typeAnnotation: "i32", source: "signal count: i32 = 0",
});

// Event Binding v2
btn.bindEvent("onPress", "handleIncrement");

// Code 模式 — 创建处理函数
O.switchMode("code");
const tab = O.createTab("event", "Counter Events", "🔢");
O.switchTab(tab.id);
const fn = O.customFunction("handleIncrement", [], "void");
fn.setSource(`action handleIncrement(event) {
  count.set(count.value + 1)
}`);
O.returnNode(fn.id);

O.validate();
```

### Recipe 2: Search Filter（搜索过滤）

```javascript
const card = O.card({ padding: "24px" });
O.heading("Search", "2", card.id);
const input = O.input("Search items...", { type: "search" }, card.id);
const resultText = O.text("Type to search...", {}, card.id);

const query = O.codeBlock("signal", "searchQuery", {
  typeAnnotation: "String", source: `signal searchQuery: String = ""`,
});

// 绑定 onChange 事件
input.bindEvent("onChange", "updateSearch", {
  params: {
    searchValue: { sourceType: "event", value: "event.value", expectedType: "String" },
  }
});

// Code 模式 — 处理函数
O.switchMode("code");
const tab = O.createTab("event", "Search", "🔍");
O.switchTab(tab.id);
const fn = O.customFunction("updateSearch", [
  { name: "searchValue", type: "String" },
], "void");
fn.setSource(`action updateSearch(searchValue: String) {
  searchQuery.set(searchValue)
}`);
O.returnNode(fn.id);

// Computed
const filtered = O.codeBlock("computed", "filteredItems", {
  typeAnnotation: "String",
  source: `computed filteredItems() -> String { searchQuery.value }`,
});
query.connectTo(filtered, "value", "deps");
```

### Recipe 3: Form Submit（表单提交）

```javascript
const card = O.card({ padding: "32px" });
O.heading("Contact", "2", card.id);
O.input("Name", {}, card.id);
O.input("Email", { type: "email" }, card.id);
const submitBtn = O.button("Send", { variant: "primary" }, card.id);

const status = O.codeBlock("signal", "submitStatus", {
  typeAnnotation: "String", source: `signal submitStatus: String = "idle"`,
});

// 事件绑定 v2
submitBtn.bindEvent("onPress", "handleSubmit");

// Code 模式
O.switchMode("code");
const tab = O.createTab("event", "Form Handler", "📝");
O.switchTab(tab.id);
const fn = O.customFunction("handleSubmit", [
  { name: "event", type: "SubmitEvent" },
], "void");
fn.setSource(`action handleSubmit(event) {
  submitStatus.set("submitting")
  // TODO: call API
  submitStatus.set("success")
}`);
O.returnNode(fn.id);
```

### Recipe 4: Toggle（开关切换）

```javascript
const card = O.card({ padding: "24px" });
const toggleBtn = O.button("OFF", { variant: "outline" }, card.id);

const isOn = O.codeBlock("signal", "isOn", {
  typeAnnotation: "bool", source: "signal isOn: bool = false",
});

toggleBtn.bindEvent("onPress", "handleToggle");

O.switchMode("code");
const tab = O.createTab("event", "Toggle", "🔘");
O.switchTab(tab.id);
const fn = O.customFunction("handleToggle", [], "void");
fn.setSource(`action toggle(event) { isOn.set(!isOn.value) }`);
O.returnNode(fn.id);
```

> **Snippet 优先：** `O.snippet("login-form")`、`O.snippet("data-card")`、`O.snippet("crud-table")` 开箱即用。用 `O.listSnippets()` 查看全部模板。

## 扩展已有画布

**在已有画布上添加功能，不破坏现有结构：**

```javascript
const O = window.__OMNI__;

// Step 1: 扫描现有状态
const state = O.getState();
const cards = O.findByTag("Card");
const existingSignals = O.codeBlocks.filter(cb => cb.data?.tag === "signal");

// Step 2: 找到目标容器
let targetCard = cards[0];
if (!targetCard) {
  targetCard = O.card({ padding: "24px" });
}

// Step 3: 检查重复 — 不要创建已存在的元素！
const existingNames = new Set(
  state.elements.map(e => (e.props)?.name).filter(Boolean)
);

// Step 4: 创建新元素（挂到已有容器下）
if (!O.findByProp("label", "Refresh").length) {
  const btn = O.button("Refresh", { variant: "outline" }, targetCard.id);
  btn.bindEvent("onPress", "handleRefresh");
}

// Step 5: 扫描代码 tab
O.switchMode("code");
O.listTabs().forEach(t => console.log(t.label, t.elementCount));

// Step 6: 验证
O.validate();
```

> **🔑 关键原则：先读后写。** 不要假设画布是空的。始终先扫描 `getState()` 了解现有结构。

## 🩺 Error Recovery Protocol（修复损坏的画布）

### Step 1: 了解现状

```javascript
const state = O.getState();
console.log("Elements:", state.elements.length, "Wires:", state.wires.length);
// 代码模式：检查 tab 数量
if (O.getMode() === "code") {
  O.listTabs().forEach(t => console.log(`Tab "${t.label}": ${t.elementCount} elements, ${t.variableCount} vars`));
}
```

### Step 2: 运行健康检查

```javascript
const report = O.validate();
// - report.orphanElements → 需要加父容器
// - report.unwiredEvents → 需要 bindEvent
// - report.typeMismatches → 需要修正连线端口
// - report.unreachableBlocks → 需要添加入站连线
```

### Step 3: 根据诊断结果修复

```javascript
// 问题 A: signal "value" 端口被当作输入 → 改为 "set"
const wrongWires = O.wires.filter(w => w.toPort === "value");
for (const w of wrongWires) {
  const toEl = O.element(w.toId);
  if (toEl.data?.tag === "signal") {
    O.disconnect(w.id);
    O.element(w.fromId).connectTo(toEl, w.fromPort, "set");
  }
}

// 问题 B: 缺失事件处理器
for (const el of O.views) {
  const events = el.getAvailableEvents();
  for (const evtName of events) {
    const hasEventProp = el.getProp(evtName);
    if (!hasEventProp) {
      console.warn(`${el.data?.tag} missing event binding for ${evtName}`);
    }
  }
}

// 问题 C: 类型不匹配
for (const mm of report.typeMismatches) {
  console.warn(`Type mismatch on wire ${mm.wireId}: ${mm.fromType} → ${mm.toType}`);
}

// 问题 D: 孤儿 wires
const elementIds = new Set(O.elements.map(e => e.id));
const orphanWires = O.wires.filter(w => !elementIds.has(w.fromId) || !elementIds.has(w.toId));
for (const w of orphanWires) {
  O.disconnect(w.id);
  console.warn(`Cleaned orphan wire ${w.id}`);
}

// 问题 E: 重复连线
const wireKeys = new Set();
const duplicates = [];
for (const w of O.wires) {
  const key = `${w.fromId}::${w.fromPort}::${w.toId}::${w.toPort}`;
  if (wireKeys.has(key)) duplicates.push(w.id);
  else wireKeys.add(key);
}
for (const id of duplicates) O.disconnect(id);

// 问题 F: customFn 缺少 return 节点
for (const fn of O.listCustomFunctions()) {
  const report = O.validateFunction(O.findFunctionByName(fn.fnName)?.id);
  if (!report || !report.valid) {
    console.warn(`Function "${fn.fnName}" has issues:`, report?.errors);
  }
}
```

### Step 4: 重新验证

```javascript
const finalReport = O.validate();
if (!finalReport.isValid) {
  console.error("Remaining issues:", finalReport);
} else {
  console.log("✅ Canvas is healthy!");
}
```

## Self-Check Checklist（脚本完成前必查）

- [ ] `O.help()` 是否在第一行调用？
- [ ] `O.switchMode("design")` 是否在创建元素前调用？
- [ ] 是否有根容器（Card 或 Container）包裹所有 UI 元素？
- [ ] 所有 `O.text()` 调用是否传了 3 个参数 `(label, props, parentId)`？
- [ ] 所有 `O.heading()` 的 level 是否是字符串 `"2"` 而非数字 `2`？
- [ ] `bindEvent()` 是否传的是事件名（`"onPress"`）而非 UUID？
- [ ] 事件处理函数是否在 code 模式 event tab 中定义为 customFn？
- [ ] 所有 customFn 是否有对应的 return 节点？
- [ ] `connectTo()` 连 signal 时输入端口是否是 `"set"` 而非 `"value"`？
- [ ] 所有 view 元素是否有明确的父容器？
- [ ] **最后是否调用了 `O.validate()` 并检查了结果？**
- [ ] `O.listSnippets()` 是否检查过？是否有现成模板可用？
- [ ] signal/computed/customFn 名称是否语义化（`cartCount` 而非 `s1`）？
- [ ] 是否每 3-5 个元素就验证一次（Progressive Build-Test Loop）？
- [ ] `connectTo()` 后是否检查了 console 是否有 error？
- [ ] 删除元素后是否验证了 wires 级联删除？
- [ ] 是否有重复连线？
- [ ] (Run Pipeline) `/api/compile` 是否在 `/api/run/start` 之前调用？
- [ ] (Run Pipeline) `debug:true` 时是否确认了 `__OMNI_DEBUG_WS_URL__` 已注入？
- [ ] (Run Pipeline) 🛜 MCP Availability Gate 是否通过？

## Red Flags — STOP and Check

- **跳过了 O.help()** → 你不知道有哪些 API，必然会出错
- **没有根容器** → 所有元素散落在画布根级，布局混乱
- **"我先创建元素再 compile 验证"** → compile() 会丢弃程序化元素
- **"我用 connectTo 把子元素连到父容器"** → connectTo 是数据流，不是布局
- **"O.text('Hi', cardId) 怎么没嵌套进去"** → text() 第2个参数是 props
- **"O.image('url', cardId) 图片没显示在卡片里"** → image() 第2个参数是 alt
- **"bindEvent('press') 报错了"** → 事件名是 React 风格 `"onPress"`
- **"createEventHandler('evt-btn-press-001') 报错了"** → 已废弃，用 bindEvent
- **"handler.connectTo(signal, 'result', 'value') 连不上"** → signal 输入端口是 `"set"`
- **"O.heading('Title', 2) 显示不对"** → level 是字符串 `"2"`
- **"O.container() 没有卡片样式"** → Container ≠ Card
- **"el.parent() 报 undefined"** → 你在 `getElements()` 的纯数据上调用方法
- **"画布上有之前的内容，我直接 clearAll 了"** → 如果需要保留，先 `getState()` 扫描
- **"我先从零构建，不用 snippet"** → 检查 `O.listSnippets()` — 可能有现成模板
- **"我把所有元素创建完再一起验证"** → 等全部完成再验证，出问题找不到根因
- **"createEventHandler 还能用吗"** → 🚫 已废弃！使用 `bindEvent()` + `customFunction()`
- **"action 节点去哪了"** → v2 纯 data-flow 架构中已移除，用 customFn + return 取代
- **"flow 端口还存在吗"** → 纯 data-flow 架构中所有连线都是 data 端口，没有 flow 边
- **"MCP 没连接，我用 curl 验证就行"** → STOP。curl 不能验证 DOM 渲染
- **"Vite 启动了但页面空白"** → 检查编译 target 和 `index.html` 路径
- **"Debug 断点不触发"** → 确认 `window.__OMNI_DEBUG_WS_URL__` 已设置
- **"el.remove() 后 wires 还在"** → 已修复！自动级联删除

## Naming Convention

| 类型 | 命名规则 | ✅ 好例子 | ❌ 坏例子 |
|------|---------|----------|----------|
| signal | `camelCase` 名词 | `searchQuery`, `cartCount`, `userEmail` | `sig1`, `data`, `x` |
| customFn | `camelCase` 动词 | `handleSearch`, `incrementCounter`, `submitForm` | `handler1`, `fn`, `doIt` |
| computed | 描述性名词 | `filteredItems`, `totalPrice`, `isValid` | `comp1`, `result`, `temp` |
| store | 复数名词 | `products`, `users`, `orders` | `store1`, `list`, `stuff` |
| tab | 描述用途 | `Cart Events`, `Search Logic` | `tab1`, `events` |
| 元素变量 | 描述用途 | `searchInput`, `submitBtn`, `statsGrid` | `el1`, `btn`, `flex` |

## Common Mistakes

| 错误行为 | 为什么错 | 正确做法 |
|---------|---------|---------|
| `O.text("Hi", cardId)` | text() 第2个参数是 props，cardId 被当作 props 对象！ | `O.text("Hi", {}, cardId)` |
| `O.image("url", cardId)` | image() 第2个参数是 alt，cardId 被当作 alt 文本！ | `O.image("url", "", cardId)` |
| `O.heading("T", 2, cardId)` | level 是字符串 `"2"`，不是数字 `2` | `O.heading("T", "2", cardId)` |
| 用 `connectTo()` 做父子嵌套 | `connectTo()` 创建数据流连线，不影响布局 | 用 `parentId` 参数 |
| `compile()` 后程序化元素消失 | `compile()` 从 `.omni` 源文件重建画布 | 先 `setSourceCode()` 再 `compile()` |
| `bindEvent("press")` | 事件名是 React 风格 `"onPress"` | `bindEvent("onPress")` |
| 连线到 signal 的 `"value"` 或 `"input"` | signal 输入端口是 `"set"` | `connect(id, "result", sigId, "set")` |
| `O.container()` 当 Card 用 | Container ≠ Card！Card 有边框阴影 | 用 `O.card()` |
| 在 `getElements()` 数据上调用 `.parent()` | `getElements()` 返回纯 `ElementData[]` | 用 `O.element(id)` |
| 期望 `getSourceCode()` 返回程序化元素 | `getSourceCode()` 读的是文件内容 | 用 `getElements()` / `getState()` |
| `createEventHandler` 在 v2 中使用 | 🚫 已废弃！不再创建 action 节点 | 用 `bindEvent()` + `customFunction()` |
| 在 design 模式找 flow 连线 | v2 纯 data-flow 架构无 flow 边 | 所有连线都是 data 边 |
| 不调用 `O.help()` 直接编程 | Proxy 隐藏了所有方法，无法枚举 | **第一步必须** `O.help()` |
| 创建元素无 root 容器 | 所有 view 元素应在同一个 Card/Container 下 | 先 `O.card()` 创建根容器 |
| 不调用 `O.validate()` 就结束 | 无法发现连线错误、缺失处理器 | **最后一步必须** `O.validate()` |
| 在已有画布上不扫描就创建 | 可能创建重复元素或破坏现有结构 | 先用 `getState()` 扫描 |
| 不考虑 `O.snippet()` 就从零构建 | 内置模板开箱即用 | 先 `O.listSnippets()` 查模板 |
| 一次性构建全部 UI 不中间验证 | 问题出现时不知道是哪步引入的 | 每 3-5 个元素调一次 `O.validate()` |
| customFn 缺少 return 节点 | 函数体必须有 return 标记返回点 | 创建 customFn 后立即 `O.returnNode(fn.id)` |
| 忘记切换 mode 就创建 tab | tab 操作只在 code 模式有效 | 先 `O.switchMode("code")` |
