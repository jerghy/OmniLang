---
name: omni-driver
description: Use when operating the OmniLang visual editor via window.__OMNI__ — from browser DevTools Console (copy-paste scripts), Playwright/Puppeteer E2E tests, AI-driven canvas manipulation, building UI with components and dataflow wiring, diagnosing elements/wires/compile errors, extending existing canvases, or recovering from broken canvas state
---

# OmniDriver — AI 操控 OmniLang 编辑器的浏览器 API

`window.__OMNI__` 是挂载在浏览器全局对象上的编辑器操控 API，采用 Locator → Handle → Chainable 模式。**打开 DevTools Console 就能直接用**，无需任何安装或 import。同时也支持 Playwright/Puppeteer 等自动化工具通过 `page.evaluate()` 调用。

| 接入方式 | 场景 | 示例 |
|---------|------|------|
| **浏览器 Console** | 开发调试、快速原型、AI 辅助操控 | 按 F12 → Console → 粘贴脚本 |
| **Playwright** | E2E 测试、CI/CD 自动化 | `page.evaluate(() => window.__OMNI__.help())` |
| **Puppeteer** | E2E 测试、爬虫集成 | `page.evaluate(() => window.__OMNI__.help())` |

## 心智模型（必读）

**编辑器有两层数据，它们是独立的：**

```
┌─────────────────────────────────────────────────┐
│  Zustand Store (运行时)                           │
│  ← omni.button() / setProp() / connect() 操作     │
│  ← 立即生效，但不会持久化到文件                      │
│  ← getElements() / getWires() 读取                │
├─────────────────────────────────────────────────┤
│  .omni 源文件 (持久化)                              │
│  ← setSourceCode() / getSourceCode() 读写         │
│  ← compile() 解析源文件 → 重建 Store 中的元素       │
│  ← save() / readFile() / writeFile() 文件操作      │
└─────────────────────────────────────────────────┘
```

**关键后果：**
- `compile()` 会**丢弃**程序化创建的元素，从 `.omni` 源文件重建画布
- `getSourceCode()` **不会**反映 `omni.button()` 等程序化操作的结果
- 如果想要"创建 UI → 编译验证"，必须先用 `setSourceCode()` 写入完整的 `.omni` 源码

## When to Use

- **浏览器 DevTools Console** — 打开 F12，直接粘贴脚本操控画布，即时反馈
- 通过 Playwright/Puppeteer 自动化操控编辑器画布
- 批量创建组件和连线
- E2E 测试编辑器功能
- AI agent 需要发现可用组件和事件
- 诊断画布状态：查找孤立节点、缺失事件处理器、编译错误
- **扩展已有画布** — 在现有 UI 上添加新组件和连线
- **修复损坏的画布** — 诊断并修复连线错误、缺失处理器

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
const txt = O.text("Hello", {}, root.id);  // ← 注意：text() 的 props 不可跳过！

// 5. 事件处理 + 数据流（见下方 Dataflow Patterns）
const { handler, wire } = btn.createEventHandler("onPress", {
  source: "action handleSubmit(event) {\n  // logic\n}",
});
const sig = O.codeBlock("signal", "count", { typeAnnotation: "i32" });
handler.connectTo(sig, "result", "set");

// 6. 验证（必须最后一步！）
const report = O.validate();
if (!report.isValid) console.error("Canvas issues:", report);
console.log("Elements:", O.elements.length, "Wires:", O.wires.length);
```

## 📐 Dataflow Patterns（必读！）

**这是 agent 最容易出错的部分。** 以下 3 个模式覆盖 90% 的数据流场景：

### Pattern 1: Button → Handler → Signal（事件驱动更新）

```
┌────────┐  onPress   ┌──────────┐  result   ┌────────┐
│ Button │ ───────→   │ Handler   │ ───────→  │ Signal │
│        │  (event)   │ (action)  │  (data)   │  set   │
└────────┘            └──────────┘           └────────┘
```

```javascript
const btn = O.button("+1", { variant: "primary" }, card.id);
const count = O.codeBlock("signal", "count", {
  typeAnnotation: "i32",
  source: "signal count: i32 = 0",
});

// createEventHandler 自动完成 Button.onPress → Handler.call 的连线
const { handler } = btn.createEventHandler("onPress", {
  handlerName: "increment",
  source: `action increment(event) {
  count.set(count.value + 1)
}`,
});

// 你需要手动连线：Handler.result → Signal.set
handler.connectTo(count, "result", "set");
```

> **🔑 关键规则：一个 action 的 `result` 端口只能连到**一个** `signal.set` 端口。**
> 如果需要更新多个 signal，让 action 源码中直接调用多个 `signal.set()`，而不是用多条 wire。

### Pattern 2: Signal → Computed（派生计算）

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

### Pattern 3: 多 Signal 汇总到一个 Computed

```javascript
// ❌ WRONG: 多个 signal 的 value 都连到一个 computed deps
// （技术上可行但不清晰，computed 收到多路输入后无法区分）

// ✅ CORRECT: 在 computed 源码中直接引用需要依赖的 signal
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

### ❌ Anti-Pattern: 一个 Handler 连多个 Signal

```javascript
// ❌ 不要这样做！一个 action.result 不能分叉连到多个 signal.set！
handler.connectTo(sig1, "result", "set");
handler.connectTo(sig2, "result", "set");  // ← 错误！
handler.connectTo(sig3, "result", "set");  // ← 错误！
// 即使你在 action 源码中更新了多个 signal，也不要把 result 连到多个 set！
// result 端口只能有一条出站 wire。

// ✅ 正确：在 action 源码中直接更新多个 signal，result 只连一个（或不连）
btn.createEventHandler("onPress", {
  source: `action refresh(event) {
  revenue.set(12345)
  orders.set(156)
  lastUpdated.set(now())
}`,
});
handler.connectTo(revenue, "result", "set");  // 只连一个！其他在源码中更新
// 如果不需要 result 传值，甚至可以完全不连 result→set
```

> **🔴 铁律：无论 action 源码中更新了多少个 signal，`result` 端口最多只能有一条出站 wire。** OmniDriver 会在 console.warn 中检测 fan-out 并警告。

## 📖 Recipe Book（复制即用的完整模板）

**先检查 `O.listSnippets()` 是否有内置模板，再查 Recipe Book，都没有才从零构建。**

### Recipe 1: Counter（按钮 + 计数）

```javascript
const card = O.card({ padding: "24px" });
O.heading("Counter", "2", card.id);
const display = O.text("0", {}, card.id);
const btn = O.button("+1", { variant: "primary" }, card.id);
const count = O.codeBlock("signal", "count", {
  typeAnnotation: "i32", source: "signal count: i32 = 0",
});
const { handler } = btn.createEventHandler("onPress", {
  handlerName: "increment",
  source: `action increment(event) { count.set(count.value + 1) }`,
});
handler.connectTo(count, "result", "set");
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
const { handler } = input.createEventHandler("onChange", {
  handlerName: "updateSearch",
  source: `action updateSearch(event: ChangeEvent) { searchQuery.set(event.value) }`,
});
handler.connectTo(query, "result", "set");
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
const { handler } = submitBtn.createEventHandler("onPress", {
  handlerName: "handleSubmit",
  source: `action handleSubmit(event) {
  submitStatus.set("submitting")
  // TODO: call API
  submitStatus.set("success")
}`,
});
handler.connectTo(status, "result", "set");
```

### Recipe 4: Toggle（开关切换）

```javascript
const card = O.card({ padding: "24px" });
const toggleBtn = O.button("OFF", { variant: "outline" }, card.id);
const isOn = O.codeBlock("signal", "isOn", {
  typeAnnotation: "bool", source: "signal isOn: bool = false",
});
const { handler } = toggleBtn.createEventHandler("onPress", {
  handlerName: "toggle",
  source: `action toggle(event) { isOn.set(!isOn.value) }`,
});
handler.connectTo(isOn, "result", "set");
```

> **Snippet 优先：** `O.snippet("login-form")`、`O.snippet("data-card")`、`O.snippet("crud-table")` 开箱即用。用 `O.listSnippets()` 查看全部模板（含 `O.registerSnippet()` 注册的自定义模板）。

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
// card.children() → [btn, txt]

// 事件处理（3 合 1）
const { handler, wire } = btn.createEventHandler("onPress", {
  source: "action handleSubmit(event) {\n  // logic\n}",
});

// 数据流连线 — 参见上方 Dataflow Patterns
const sig = O.codeBlock("signal", "count", { typeAnnotation: "i32", source: "signal count: i32 = 0" });
handler.connectTo(sig, "result", "set");

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

### 代码块端口参考（必读）

| 类型 | 输出端口 | 输入端口 | 用途 |
|------|---------|---------|------|
| `signal` | `value` (out) | `set` (in) | 响应式状态变量 |
| `computed` | `value` (out) | `deps` (in) | 派生计算值 |
| `action` | `result` (out) | `call` (in) | 事件处理函数 |
| `store` | `items` (out) | — | 数据集合 |

```javascript
// ✅ 正确：设置 signal → 连到 set 端口
O.connect(handler.id, "result", signal.id, "set");

// ❌ 错误：signal 的输入端口是 "set"，不是 "value"（value 是输出！）
O.connect(handler.id, "result", signal.id, "value");

// ❌ 错误：signal 没有 "input" 端口
handler.connectTo(signal, "result", "input");
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
el.connectTo(target, "out", "in"); // 数据流连线（含端口验证）
el.createEventHandler("onPress", options); // 事件处理器（3合1）
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

## 事件连线

### 事件名 vs 端口 UUID

```javascript
// createEventHandler() — 传事件名（React 风格）
btn.createEventHandler("onPress");   // ✅ "onPress"
btn.createEventHandler("press");     // ❌ DOM 风格，会抛异常
btn.createEventHandler("evt-btn-press-001"); // ❌ UUID，会抛异常

// connect() 手动连线 — 传端口 UUID
O.connect(btn.id, "evt-btn-press-001", action.id, "call"); // ✅ UUID
O.connect(btn.id, "onPress", action.id, "call");            // ❌ 事件名不是端口
```

> **铁律：`createEventHandler(eventName)` 用事件名，`connect(fromPort)` 用 UUID。**

### Handler 源码模板

```javascript
// 模板 A: 更新单个 signal
btn.createEventHandler("onPress", {
  handlerName: "increment",
  source: `action increment(event) {
  count.set(count.value + 1)
}`,
});

// 模板 B: 更新多个 signal（在源码中直接操作，不需要多条 wire）
btn.createEventHandler("onPress", {
  handlerName: "refreshAll",
  source: `action refreshAll(event) {
  revenue.set(fetchRevenue())
  orders.set(fetchOrders())
  lastUpdated.set(now())
}`,
});

// 模板 C: 带参数的事件处理
btn.createEventHandler("onPress", {
  handlerName: "handleSubmit",
  source: `action handleSubmit(event: SubmitEvent) {
  // event 包含表单数据
  status.set("submitting")
}`,
});
```

### 事件端口 UUID 表

| 组件 | 事件 | 端口 ID |
|------|------|---------|
| Button | onPress | `evt-btn-press-001` |
| Button | onHover | `evt-btn-hover-001` |
| Input | onChange | `evt-input-change-001` |
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
```

## 🔍 Context-Aware Extension（扩展已有画布）

**在已有画布上添加功能，不破坏现有结构：**

```javascript
// ═══ 扩展已有画布的模板 ═══
const O = window.__OMNI__;

// Step 1: 扫描现有状态
const state = O.getState();
const cards = O.findByTag("Card");
const existingButtons = O.findByTag("Button");
const existingSignals = O.codeBlocks.filter(cb => cb.data?.tag === "signal");

// Step 2: 找到目标容器
const targetCard = cards[0];
if (!targetCard) {
  console.warn("No Card found — creating one");
  targetCard = O.card({ padding: "24px" });
}

// Step 3: 检查重复 — 不要创建已存在的元素！
const existingNames = new Set(
  state.elements.map(e => (e.props as any)?.name).filter(Boolean)
);

// Step 4: 创建新元素（挂到已有容器下）
if (!O.findByProp("label", "Refresh").length) {
  const btn = O.button("Refresh", { variant: "outline" }, targetCard.id);
  // ... 添加事件处理 ...
}

// Step 5: 验证（确保没有破坏现有结构）
const afterReport = O.validate();
console.log("Canvas healthy:", afterReport.isValid);
```

> **🔑 关键原则：先读后写。** 不要假设画布是空的。始终先扫描 `getState()` 了解现有结构。

## 🩺 Error Recovery Protocol（修复损坏的画布）

**当画布有问题时（编译错误、连线错误、缺失处理器），按以下步骤诊断修复：**

### Step 1: 了解现状

```javascript
const state = O.getState();
console.log("Elements:", state.elements.length, "Wires:", state.wires.length);
```

### Step 2: 运行健康检查

```javascript
const report = O.validate();
// 检查每个字段：
// - report.orphanElements → 需要加父容器
// - report.unwiredEvents → 需要 createEventHandler
// - report.typeMismatches → 需要修正连线端口
// - report.unreachableBlocks → 需要添加入站连线
```

### Step 3: 根据诊断结果修复

```javascript
// 问题 A: signal "value" 端口被当作输入 → 改为 "set"
const wrongWires = O.wires.filter(w => w.toPort === "value");
for (const w of wrongWires) {
  const fromEl = O.element(w.fromId);
  const toEl = O.element(w.toId);
  if (toEl.data?.tag === "signal") {
    O.disconnect(w.id);
    fromEl.connectTo(toEl, w.fromPort, "set"); // "value" → "set"
  }
}

// 问题 B: Button 缺少事件处理器
for (const el of O.views) {
  const events = el.getEvents();
  for (const [evtName] of Object.entries(events)) {
    const hasWire = O.wires.some(
      w => w.fromId === el.id && w.fromPort === events[evtName].id
    );
    if (!hasWire) {
      el.createEventHandler(evtName);
    }
  }
}

// 问题 C: 类型不匹配
for (const mm of report.typeMismatches) {
  console.warn(`Type mismatch on wire ${mm.wireId}: ${mm.fromType} → ${mm.toType}`);
  // 修正 codeBlock 的 typeAnnotation 或调整连线
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

## Common Mistakes

| 错误行为 | 为什么错 | 正确做法 |
|---------|---------|---------|
| `O.text("Hi", cardId)` | text() 第2个参数是 props，cardId 被当作 props 对象！ | `O.text("Hi", {}, cardId)` |
| `O.image("url", cardId)` | image() 第2个参数是 alt，cardId 被当作 alt 文本！ | `O.image("url", "", cardId)` |
| `O.heading("T", 2, cardId)` | level 是字符串 `"2"`，不是数字 `2` | `O.heading("T", "2", cardId)` |
| 用 `connectTo()` 做父子嵌套 | `connectTo()` 创建数据流连线，不影响布局 | 用 `parentId` 参数 |
| `compile()` 后程序化元素消失 | `compile()` 从 `.omni` 源文件重建画布 | 先 `setSourceCode()` 再 `compile()` |
| `createEventHandler("press")` | 事件名是 React 风格 `"onPress"` | `createEventHandler("onPress")` |
| `createEventHandler("evt-btn-press-001")` | 不要传 UUID，UUID 只用于 `connect()` | `createEventHandler("onPress")` |
| 连线到 signal 的 `"value"` 或 `"input"` | signal 输入端口是 `"set"` | `connect(id, "result", sigId, "set")` |
| 一个 handler.result 连多个 signal.set | 一个 output 不能驱动多个 input（即使源码中更新了多个 signal） | result 只连一个 signal.set，其他在 action 源码中更新 |
| `O.container()` 当 Card 用 | Container ≠ Card！Card 有边框阴影 | 用 `O.card()` |
| 在 `getElements()` 数据上调用 `.parent()` | `getElements()` 返回纯 `ElementData[]` | 用 `O.element(id)` |
| 期望 `getSourceCode()` 返回程序化元素 | `getSourceCode()` 读的是文件内容 | 用 `getElements()` / `getState()` |
| 认为 `typeAnnotation` 无效 | **有效参数！** 决定 signal/computed 端口类型 | `codeBlock("signal","n",{typeAnnotation:"f64"})` |
| 不调用 `O.help()` 直接编程 | Proxy 隐藏了所有方法，无法枚举 | **第一步必须** `O.help()` |
| 创建元素无 root 容器 | 所有 view 元素应在同一个 Card/Container 下 | 先 `O.card()` 创建根容器 |
| 不调用 `O.validate()` 就结束 | 无法发现连线错误、缺失处理器 | **最后一步必须** `O.validate()` |
| 在已有画布上不扫描就创建 | 可能创建重复元素或破坏现有结构 | 先用 `getState()` 扫描 |
| 不考虑 `O.snippet()` 就从零构建 | 内置模板开箱即用，节省大量代码 | 先 `O.listSnippets()` 查模板 |
| 一次性构建全部 UI 不中间验证 | 问题出现时不知道是哪步引入的 | 每 3-5 个元素调一次 `O.validate()` |
| `createEventHandler` 可用却手动 `connect()` | 手动 connect 需要 UUID，容易出错 | 始终优先用 `createEventHandler(eventName)` |
| 用 `handler1`/`sig1`/`btn` 等无意义命名 | 画布不可读，无法维护 | 用语义化命名：`cartCount`, `handleAddToCart` |

## 🏷️ Naming Convention

**语义化命名让画布可读、可维护、可扩展：**

| 类型 | 命名规则 | ✅ 好例子 | ❌ 坏例子 |
|------|---------|----------|----------|
| signal | `camelCase` 名词 | `searchQuery`, `cartCount`, `userEmail` | `sig1`, `data`, `x` |
| action/handler | `camelCase` 动词 | `handleSearch`, `incrementCounter`, `submitForm` | `handler1`, `fn`, `doIt` |
| computed | 描述性名词 | `filteredItems`, `totalPrice`, `isValid` | `comp1`, `result`, `temp` |
| store | 复数名词 | `products`, `users`, `orders` | `store1`, `list`, `stuff` |
| 元素变量 | 描述用途 | `searchInput`, `submitBtn`, `statsGrid` | `el1`, `btn`, `flex` |

```javascript
// ✅ 好：自文档化
const cartCount = O.codeBlock("signal", "cartCount", { typeAnnotation: "i32" });
submitBtn.createEventHandler("onPress", {
  handlerName: "handleAddToCart",
  source: `action handleAddToCart(event) { cartCount.set(cartCount.value + 1) }`,
});

// ❌ 坏：一个月后没人看得懂
const s1 = O.codeBlock("signal", "s1", { typeAnnotation: "i32" });
btn1.createEventHandler("onPress", {
  handlerName: "h1",
  source: `action h1(event) { s1.set(s1.value + 1) }`,
});
```

## 🔄 Progressive Build-Test Loop

**不要一次性构建全部 UI。** 渐进式构建，每 3-5 个元素验证一次：

```
1. 根容器 + 标题  →  O.validate()  →  修复
2. 添加按钮+事件  →  O.validate()  →  修复
3. 添加数据流连线 →  O.validate()  →  修复
4. 重复直到完成
```

```javascript
const O = window.__OMNI__;
O.help();
O.switchMode("design");
O.clearAll();

// Phase 1: 根容器 + 标题
const root = O.card({ padding: "24px" });
O.heading("My App", "2", root.id);
if (!O.validate().isValid) throw new Error("Phase 1 failed");

// Phase 2: 按钮 + handler + signal
const btn = O.button("Action", { variant: "primary" }, root.id);
const sig = O.codeBlock("signal", "data", { typeAnnotation: "i32" });
const { handler } = btn.createEventHandler("onPress", {
  source: `action handleAction(event) { data.set(42) }`,
});
handler.connectTo(sig, "result", "set");
if (!O.validate().isValid) throw new Error("Phase 2 failed");

// Phase 3: 显示元素
O.text("Ready", {}, root.id);
const finalReport = O.validate();
if (!finalReport.isValid) console.error("Issues:", finalReport);
else console.log("✅", O.elements.length, "elements,", O.wires.length, "wires");
```

> **🔑 关键：问题出现时你知道是最近 3-5 步引入的，而不是在 50 个元素的画布上大海捞针。**

## Self-Check Checklist（脚本完成前必查）

- [ ] `O.help()` 是否在第一行调用？
- [ ] `O.switchMode("design")` 是否在创建元素前调用？
- [ ] 是否有根容器（Card 或 Container）包裹所有 UI 元素？
- [ ] 所有 `O.text()` 调用是否传了 3 个参数 `(label, props, parentId)`？
- [ ] 所有 `O.heading()` 的 level 是否是字符串 `"2"` 而非数字 `2`？
- [ ] `createEventHandler()` 是否传的是事件名（`"onPress"`）而非 UUID？
- [ ] `connectTo()` 连 signal 时输入端口是否是 `"set"` 而非 `"value"`？
- [ ] 每个 action 的 `result` 端口是否最多只连一个 signal 的 `set`？
- [ ] 所有 view 元素是否有明确的父容器？
- [ ] **最后是否调用了 `O.validate()` 并检查了结果？**
- [ ] `O.listSnippets()` 是否检查过？是否有现成模板可用？
- [ ] signal/action/computed 名称是否语义化（`cartCount` 而非 `s1`）？
- [ ] 是否每 3-5 个元素就验证一次（Progressive Build-Test Loop）？

## Red Flags — STOP and Check

- **跳过了 O.help()** → 你不知道有哪些 API，必然会出错
- **没有根容器** → 所有元素散落在画布根级，布局混乱
- **"我先创建元素再 compile 验证"** → compile() 会丢弃程序化元素
- **"我用 connectTo 把子元素连到父容器"** → connectTo 是数据流，不是布局
- **"O.text('Hi', cardId) 怎么没嵌套进去"** → text() 第2个参数是 props，需要 `O.text('Hi', {}, cardId)`
- **"O.image('url', cardId) 图片没显示在卡片里"** → image() 第2个参数是 alt，需要 `O.image('url', '', cardId)`
- **"createEventHandler('press') 报错了"** → 事件名是 React 风格 `"onPress"`
- **"createEventHandler('evt-btn-press-001') 报错了"** → createEventHandler 要事件名，不是 UUID
- **"handler.connectTo(signal, 'result', 'value') 连不上"** → signal 输入端口是 `"set"`
- **"O.heading('Title', 2) 显示不对"** → level 是字符串 `"2"`
- **"O.container() 没有卡片样式"** → Container ≠ Card
- **"el.parent() 报 undefined"** → 你在 `getElements()` 的纯数据上调用方法
- **"typeAnnotation 参数被忽略了"** → 它是有效参数，决定端口类型
- **"我把 handler 连到了 3 个 signal 的 set"** → 一个 action result 不能分叉连多个 set。即使源码更新了多个 signal，result 端口也只能有一条出站 wire
- **"result 连多个信号是技术可行的"** → OmniDriver 会 warn。一个 result 端口=一条 wire。多 signal 更新在源码中完成，不需要多条 wire
- **"画布上有之前的内容，我直接 clearAll 了"** → 如果需要保留，先 `getState()` 扫描
- **"我先从零构建，不用 snippet"** → 检查 `O.listSnippets()` — 可能有现成模板
- **"我把所有元素创建完再一起验证"** → 等全部完成再验证，出问题找不到根因。每 3-5 步验证一次
- **"我用 `O.connect(id, 'evt-btn-press-001', ...)` 手动连线"** → `createEventHandler(eventName)` 更安全，自动处理 UUID

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

## Error Handling

OmniDriver 方法在失败时**不会抛出异常**，而是返回 `null`/`undefined` 或静默 no-op：

- `O.element("bad-id")` → 返回 `ElementHandle`（即使元素不存在）
- `el.setProp("bad-prop", val)` → 静默操作
- `O.connect(badId, ...)` → 返回 `WireHandle`，但连线可能无效
- **验证操作：** 操作后始终检查 `O.getState()` 或 `el.data`

## 浏览器 Console & 自动化集成

### 方式 1: 浏览器 DevTools Console（最直接）

**打开 `http://localhost:5174`，按 F12 → Console，直接粘贴脚本：**

```javascript
// 复制这段代码到浏览器 Console，回车即执行
const O = window.__OMNI__;
O.help();                      // 第一步：发现 API
O.switchMode("design");
O.clearAll();

// 创建 UI
const card = O.card({ padding: "24px" });
O.heading("Hello Omni", "2", card.id);
O.button("Click Me", { variant: "primary" }, card.id);

// 验证
const report = O.validate();
console.log("✅ Valid:", report.isValid, "| Elements:", O.elements.length);
```

> **Console 技巧：** 多行粘贴用 Shift+Enter 换行，或用 `{ }` 包裹代码块。浏览器 Console 有完整的自动补全——输入 `O.` 然后等 1 秒，所有方法都会出现。

### 方式 2: Playwright E2E 测试

> **序列化约束：** `page.evaluate()` 只能返回可序列化的值。返回 `.text()`、`.exists()`、`.data`，不要返回 Handle 对象。

```javascript
// 辅助函数
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
    O.button("Submit", { variant: "primary" }, card.id);
  });

  const count = await omniEval(page, () => window.__OMNI__.getElements().length);
  expect(count).toBeGreaterThanOrEqual(4);
});
```

### 方式 3: Puppeteer

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
  const card = O.card({ padding: "24px" });
  O.button("Test", { variant: "primary" }, card.id);
});
```
