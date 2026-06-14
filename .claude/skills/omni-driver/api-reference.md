# OmniDriver API Reference

> 完整 API 参考。主 skill 文件：[SKILL.md](SKILL.md)

## OmniDriver 主类

### 状态查询

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `O.getState()` | `EditorSnapshot` | 完整快照：mode, elements, wires, selectedElementId, projectDir, activeFilePath, compileErrors, previewUrl |
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

```javascript
// props 中传 children 数组可实现单行嵌套:
O.component("Card", {
  padding: "16px",
  children: [O.button("A"), O.text("B", {}, null)]  // children 自动设置 parentId
});
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

**代码块端口参考：**

| 类型 | 输出端口 | 输入端口 | 用途 |
|------|---------|---------|------|
| `signal` | `value` (out, TYPE) | `set` (in, TYPE) | 响应式状态。TYPE 来自 typeAnnotation，默认 i32 |
| `computed` | `value` (out, TYPE) | `deps` (in, any) | 派生计算。TYPE 来自 typeAnnotation，默认 i32 |
| `action` | `result` (out, any) | `call` (in, event) | 事件处理函数 |
| `store` | `items` (out, any) | — | 数据集合 |

```javascript
// ✅ 正确：设置 signal → 连到 set 端口
O.connect(action.id, "result", signal.id, "set");

// ❌ 错误：signal 没有 "value" 输入（value 是输出！）
O.connect(action.id, "result", signal.id, "value");

// ❌ 错误：signal 没有 "input" 端口
action.connectTo(signal, "result", "input");
```

### 事件处理

```javascript
// createEventHandler 接受事件名（React 风格，不是 UUID！）
const { handler, wire } = btn.createEventHandler("onPress", {
  source: "action handleSubmit(event) {\n  // logic\n}",
  handlerName: "myHandler",   // 可选：自定义 handler 名称
  parentStore: "",            // 可选：所属 store
  folded: false,              // 可选：是否折叠
});

// ❌ 错误：传了 UUID
btn.createEventHandler("evt-btn-press-001"); // 会抛异常！

// ❌ 错误：传了 DOM 风格事件名
btn.createEventHandler("press"); // 会抛异常！

// 等价于手动操作（手动 connect 才用 UUID）:
// 1. 创建 action 元素
// 2. btn.setProp("onPress", handler.id)
// 3. O.connect(btn.id, "evt-btn-press-001", handler.id, "call")
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

### 组件发现

```javascript
O.listComponents();           // ComponentInfo[] — 所有视图组件元数据
O.getComponentInfo("Button"); // 单个组件的 props, events, styleProperties, hasChildren
```

### 画布诊断

```javascript
// 一键健康检查 — 脚本完成后的必须步骤！
const report = O.validate();
// {
//   orphanElements: string[],         // 无父容器且非根节点的 view 元素
//   unwiredEvents: Array<{            // 有事件定义但无连线的组件
//     elementId, tag, eventName, eventPortId
//   }>,
//   typeMismatches: Array<{           // 类型不匹配的连线
//     wireId, fromPort, fromType, toPort, toType
//   }>,
//   unreachableBlocks: string[],      // 无数据流入的代码块
//   isValid: boolean                  // 是否通过所有检查
// }

// 状态变更追踪 — 比较两个快照
const before = O.getState();
// ... 做一些操作 ...
const change = O.diff(before);
// {
//   added: ElementData[],             // 新增的元素
//   removed: string[],                // 被删除的元素 ID
//   modified: Array<{ id, changes }>, // 被修改的元素
//   addedWires: WireData[],           // 新增的连线
//   removedWires: string[],           // 被删除的连线 ID
// }
```

### 状态快照

```javascript
// 导出完整画布状态为 JSON 字符串
const json = O.exportState();
// → '{"version":"1.0","exportedAt":"...","elements":[...],"wires":[...]}'

// 从 JSON 恢复画布状态（替换所有当前元素和连线）
O.importState(json);

// 使用场景：保存/恢复、状态回滚、测试快照对比
```

### 预设模板

```javascript
// 快速创建常见 UI 模式
O.snippet("login-form");                           // 登录表单
O.snippet("login-form", { title: "Welcome Back" }); // 自定义标题

O.snippet("data-card");                            // 数据卡片
O.snippet("data-card", {
  title: "User Stats",
  description: "Monthly active users: 1,234",
  actionLabel: "View Details",                      // 可选按钮
});

O.snippet("crud-table");                           // CRUD 表格
O.snippet("crud-table", {
  title: "Products",
  columns: ["Name", "Price", "Stock", "Actions"],
});
```

### 其他

```javascript
O.help();       // 🚨 每次会话第一步！返回 portReference、eventUuidTable、criticalRules
O.clearAll();   // 清除 Store 中的 elements + wires + selection（不触及源文件）
O.batch(fn);    // 批量操作：O.batch(O => { O.button("A"); O.button("B"); })
O.removeElement(id); // 删除单个元素
```

---

## /api/run 端点 — 编译→运行→测试管线

> 🛜 **前置条件：** 必须先通过 MCP Availability Gate（Browser MCP + Vision MCP 都连接）。详见 [SKILL.md](SKILL.md) 的 Run & Test Pipeline 章节。
>
> 详细流程见 [SKILL.md](SKILL.md) 的 🚀 Run & Test Pipeline 章节

### POST /api/run/start

接收编译后文件，scaffold Vite 项目，启动 dev server。

```javascript
const resp = await fetch("/api/run/start", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    files: { "Counter.js": "export function Counter(mountEl) { ... }" },
    debug: true,   // 注入 __OMNI_DEBUG_WS_URL__，复制 omniview-runtime.ts
  }),
});
const { success, url, port } = await resp.json();
// → { success: true, url: "http://localhost:5173", port: 5173 }
```

**内部流程：**
1. 创建 `.omni-run/` 目录
2. 从文件扩展名检测 target（`.js` → omniview, `.tsx` → react）
3. 生成 `package.json`（OmniView 无 react/zustand 依赖）
4. 生成 `vite.config.ts`（React 含 `@vitejs/plugin-react`）
5. 生成 `index.html` + `src/main.{js|tsx}`
6. `npm install`（如 node_modules 已存在则跳过）
7. 找可用端口，`npx vite --port N --host`

### POST /api/run/stop

```javascript
await fetch("/api/run/stop", { method: "POST" });
// → { success: true }
```

### GET /api/run/status

```javascript
const { running, url } = await fetch("/api/run/status").then(r => r.json());
// → { running: true, url: "http://localhost:5173" }
```

### POST /api/run/write-files

热更新已编译文件。Vite HMR 自动重载浏览器 tab。

```javascript
await fetch("/api/run/write-files", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ files: { "Counter.js": updatedSource } }),
});
// → { success: true, count: 1 }
```

**约束：** 必须在 `/api/run/start` 之后调用，否则返回 error。

---

## window.__OMNI_RUNTIME__ — OmniView Runtime API

**仅 OmniView target（`.js` 输出）可用。** React target 使用 Zustand store（`window.__OMNI__`）。

```javascript
const R = window.__OMNI_RUNTIME__;

// ── Signal System ──
const count = R.signal(0, { id: "Counter.count", name: "count", type: "number" });
count.get();                        // 0
count.set(5);                       // 触发 DebugBus.check() + 订阅者通知
count.set(prev => prev + 1);        // 函数式更新
count.peek();                       // 读取但不注册依赖
count.subscribe(v => console.log(v)); // 返回 CleanupFn

// ── Computed ──
const doubled = R.computed(() => count.get() * 2, { id: "Counter.doubled" });
doubled.get();                      // 自动追踪依赖，惰性求值

// ── Effect ──
const dispose = R.effect(() => {
  console.log("count:", count.get());
  return () => console.log("cleanup");  // 可选清理函数
});

// ── Store ──
const TodoStore = R.store("TodoStore", () => {
  const items = R.signal([], { id: "TodoStore.items" });
  return {
    items,
    add(text) { items.set([...items.get(), text]); },
    get count() { return items.get().length; },
  };
});
TodoStore.items;                   // Proxy 自动 unwrap signal.get()
TodoStore.add("buy milk");         // 方法直接可用
TodoStore.$id;                     // "TodoStore"

// ── DOM: h() + render() ──
const vnode = R.h("div", { class: "container" },
  R.h("button", { onClick: () => count.set(count.get() + 1) }, "+1"),
  R.h("span", {}, count),          // Signal child → auto-bound textContent
);
const cleanup = R.render(vnode, document.getElementById("root"));

// ── Control Flow ──
R.renderConditional(isVisibleSignal, () => R.h("div", {}, "visible"));
R.renderList(itemsSignal, (item, i) => R.h("li", {}, item), item => item.id);

// ── Lifecycle ──
R.onCleanup(() => console.log("unmount"));
R.dispose();                        // 运行所有注册的 cleanup
```

### DebugBus（内嵌在 signal.set 中）

```javascript
const DB = R.DebugBus;

// 断点管理
DB.breakpoints.add("Counter.count");
DB.breakpoints.delete("Counter.count");
DB.hasBreakpoint("Counter.count");  // boolean

// 暂停状态
DB.paused;                          // boolean — 是否有未执行的挂起项
DB.flushing;                        // boolean — 是否正在批量恢复
DB.pendingQueue;                    // [{ signalId, value, apply }]

// 手动控制（无需 WebSocket）
DB.step();                          // 执行一个挂起项
DB.resume();                         // 执行全部恢复
DB.trace("nodeId", "edgeId");       // 发送执行追踪

// WebSocket（自动管理，无需手动操作）
// 连接条件：window.__OMNI_DEBUG_WS_URL__ 已设置
// 协议：JSON { type: "state-change"|"paused"|"resumed"|"runtime-ready"|... }
```

### Debug 断点 WebSocket 协议

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

## Dataflow Patterns

> 详细说明见 [SKILL.md](SKILL.md) 的 📐 Dataflow Patterns 章节

### Pattern 1: Button → Handler → Signal

```javascript
const btn = O.button("+1", {}, card.id);
const count = O.codeBlock("signal", "count", { typeAnnotation: "i32" });
const { handler } = btn.createEventHandler("onPress");
handler.connectTo(count, "result", "set");  // ← 注意：连到 "set"，不是 "value"！
```

### Pattern 2: Signal → Computed

```javascript
const price = O.codeBlock("signal", "price", { typeAnnotation: "f64" });
const total = O.codeBlock("computed", "total", { typeAnnotation: "f64" });
price.connectTo(total, "value", "deps");  // ← 数据流：value → deps
```

### ❌ Anti-Pattern

```javascript
// 一个 handler.result 不能分叉连到多个 signal.set！
handler.connectTo(sig1, "result", "set");
handler.connectTo(sig2, "result", "set");  // ← 错误！
// 正确：在 action 源码中操作多个 signal
```

---

## ElementHandle API

```javascript
const el = O.element("some-id");

// 存在与文本
el.exists();                  // boolean — 元素是否在 store 中存在
el.text();                    // string — 显示文本（label → text → placeholder → ""）

// 属性
el.setProp(prop, value);      // 设置属性
el.getProp(prop);             // 读取属性
el.getPorts();                // 所有端口（view 元素自动含 COMPONENT_PROTOCOL 事件端口）
el.getEvents();               // 事件元数据 { onPress: { id, description, payload } }
el.getSource();               // 代码块源文
el.setSource("source code");  // 设置代码块源文
el.data;                      // ElementData 原始数据（含 outputValue）

// 导航
el.select();                  // 选中元素
el.remove();                  // 删除元素
el.children();                // ElementHandle[] — 子元素（只读）
el.parent();                  // ElementHandle | null — 父元素

// 连线
el.connectTo(target, "outPort", "inPort");  // 数据流连线（含端口验证）
el.createEventHandler("onPress", options);  // 事件处理器（3合1）

// 等待
await el.waitForLive(timeout);  // 轮询等待元素出现在 store 中
```

**注意：**
- `getPorts()` 对 view 元素自动包含事件端口
- `getEvents()` 直接返回事件元数据，比 `O.getComponentInfo(tag).events` 更直接
- `O.element(id)` 始终返回 `ElementHandle`（不会返回 null），用 `.exists()` 判断

**没有的方法（常见错误）：**
- ❌ `el.textContent()` → 用 `el.text()`
- ❌ `el.innerText()` → 用 `el.text()`
- ❌ `el.children = [...]` → `children()` 是只读 getter
- ❌ `el.waitForSelector()` → 用 `O.waitForElement()`

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

> **⚠️ ElementData ≠ ElementHandle：** `getElements()` / `.elements` 返回纯数据 `ElementData[]`（没有 `.parent()`、`.text()` 等方法）。用 `O.element(id)` 获取 `ElementHandle`。`getWires()` / `.wires` 返回纯 `WireData[]`。

```typescript
interface EditorSnapshot {
  mode: "design" | "code";
  elements: ElementData[];
  wires: WireData[];
  selectedElementId: string | null;
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
  children: string[];       // 子元素 ID 列表
  parentId?: string;
  source?: string;          // 代码块源码
  ports?: PortDescriptor[];
  isFolded?: boolean;
  outputValue?: unknown;    // 运行时求值结果
}

interface PortDescriptor {
  name: string;
  direction: "in" | "out";
  type: string;
  label?: string;
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
| TextArea | onChange | `evt-textarea-change-001` |
| Select | onChange | `evt-select-change-001` |
| Checkbox | onChange | `evt-checkbox-change-001` |
| Form | onSubmit | `evt-form-submit-001` |

---

## 浏览器 Console & 自动化集成

### 浏览器 DevTools Console

打开 `http://localhost:5174`，按 F12 → Console，直接粘贴脚本执行。无需安装、无需 import，`window.__OMNI__` 已在全局作用域中。

```javascript
// 复制到浏览器 Console 直接运行
const O = window.__OMNI__;
O.help();
O.switchMode("design");
O.clearAll();

const card = O.card({ padding: "24px" });
O.heading("Quick Test", "2", card.id);
O.button("Click", { variant: "primary" }, card.id);

console.log("Elements:", O.elements.length, "Valid:", O.validate().isValid);
```

> **序列化约束：** Console 中可以直接使用 Handle 对象（如 `el.text()`、`el.parent()`），无需序列化。只有通过 `page.evaluate()` 调用时才有限制。

### Playwright

> **序列化约束：** `page.evaluate()` 只能返回可序列化的值。不要返回 `ElementHandle` 或 `WireHandle`。返回 `.text()`、`.exists()`、`.data` 或普通对象。

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
    O.button("Submit", { variant: "primary" }, card.id);
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
