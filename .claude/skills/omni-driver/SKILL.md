---
name: omni-driver
description: Use when programmatically controlling the OmniLang visual editor — creating UI components, wiring dataflow, compiling .omni source, switching design/code modes, or writing E2E tests that automate the editor canvas via window.__OMNI__
---

# OmniDriver — AI 操控 OmniLang 编辑器的 Playwright 风格 API

`window.__OMNI__` 提供了一套 Locator → Handle → Chainable 模式的 API，让 AI agent 程序化操控 OmniLang 编辑器。

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

**两种使用模式：**
1. **程序化操控** — 用 OmniDriver API 操作编辑器 UI（类似 Playwright 操控浏览器），不关心 `.omni` 源文件
2. **源码驱动** — 用 `setSourceCode()` + `compile()` 验证 `.omni` 源码（类似编译器测试）

## When to Use

- 通过 Playwright/浏览器 Console 自动化操控编辑器画布
- 批量创建组件和连线
- E2E 测试编辑器功能
- AI agent 需要发现可用组件和事件
- 诊断画布状态：查找孤立节点、缺失事件处理器、编译错误

## When NOT to Use

- 直接操作 Zustand store — 用 `useEditorStore` 而不是 `window.__OMNI__`
- 解析 .omni 源码 — 用 `omnilang` Python 库
- 只需要文件读写 — 用后端 API (`/api/file/...`) 而不是 OmniDriver
- 服务端编译 — 用 Python 后端的 compile endpoint

## Quick Start

```javascript
const O = window.__OMNI__;

// ── 程序化操控模式 ──
O.switchMode("design");
O.clearAll();

const btn = O.button("Submit", { variant: "primary" });
const txt = O.text("Hello");

// 事件处理（3 合 1：创建 handler 元素 + 设置事件属性 + 连线）
const { handler, wire } = btn.createEventHandler("onPress", {
  source: "action handleSubmit(event) {\n  // logic\n}",
});

// 数据流连线（注意：这是端口数据流，不是父子嵌套！）
const sig = O.codeBlock("signal", "count", { source: "signal count: i32 = 0" });
handler.connectTo(sig, "result", "set");

// ── 源码驱动模式 ──
O.setSourceCode(`component App {
  signal count: i32 = 0
  fn increment() { count += 1 }
  view { <Button onPress={increment}>+1</Button> }
}`);
O.switchMode("code");
const result = await O.compile();
// result.success, result.errors
```

## 核心 API 速查

详细 API 参考见 [api-reference.md](api-reference.md)。

### 创建元素

| 操作 | 代码 | 注意 |
|------|------|------|
| 通用创建（可设 parentId） | `O.component("Button", props, parentId)` | parentId 可选 |
| 便捷方法（均支持 parentId） | `O.button(label, props, parentId)` | 最后一个参数可选 |
| 代码块 | `O.codeBlock("signal", "name", { source, parentStore })` | 预置端口定义 |

### 嵌套子元素

```javascript
// ✅ 所有方法都支持 parentId 参数
const card = O.card({ padding: "16px" });
const btn = O.button("Click", { variant: "primary" }, card.id); // parentId 作为最后一个参数
const h1 = O.heading("Title", "1", card.id);

// ❌ WRONG: children() 是只读的
card.children = [btn]; // 无效!

// ❌ WRONG: connectTo() 是数据流，不是布局嵌套
btn.connectTo(card);  // 创建的是端口连线，不是父子关系!
```

### 元素存在与文本

```javascript
// 检查元素是否存在（推荐方式）
el.exists();    // boolean — 类似于 Playwright locator.count() > 0

// 获取显示文本（自动回退到 label → text → placeholder）
el.text();      // string — 类似于 Playwright locator.textContent()
```

### 查找与等待

| 操作 | 代码 |
|------|------|
| 按 ID | `O.element(id)` → `ElementHandle \| null` |
| 按标签 | `O.findByTag("Button")` → `ElementHandle[]` |
| 按属性 | `O.findByProp("label", "Submit")` → `ElementHandle[]` |
| 通用查询 | `O.find({ tag, nodeType, props, text })` |
| 等待出现 | `await O.waitForElement("Button", timeout)` |
| 等待连线 | `await O.waitForWire("evt-btn-press-001", timeout)` |

### ElementHandle 常用操作

```javascript
const el = O.element("some-id");
el.setProp("label", "New");    // 设置属性
el.getProp("label");           // 读取属性
el.select();                   // 选中（显示属性面板）
el.remove();                   // 删除
el.connectTo(target, "out", "in"); // 数据流连线
el.createEventHandler("onPress", options); // 创建事件处理器
el.getPorts();                 // 获取端口列表
await el.waitForLive(5000);    // 等待元素在 store 中出现
el.data;                       // 原始 ElementData（含 outputValue）
```

**注意：没有 `textContent()` 或 `exists()` 方法。**
- 读取文本 → `el.getProp("label")` 或 `el.getProp("text")`
- 检查是否存在 → `el.data !== undefined`
- 读取运行时值 → `el.data.outputValue`

## 事件连线

事件使用 Component Protocol 中定义的 **UUID 端口 ID**：

| 组件 | 事件 | 端口 ID |
|------|------|---------|
| Button | onPress | `evt-btn-press-001` |
| Button | onHover | `evt-btn-hover-001` |
| Input | onChange | `evt-input-change-001` |
| TextArea | onChange | `evt-textarea-change-001` |
| Select | onChange | `evt-select-change-001` |
| Checkbox | onChange | `evt-checkbox-change-001` |
| Form | onSubmit | `evt-form-submit-001` |

```javascript
// 方式 1: createEventHandler() — 推荐，自动处理 UUID
const { handler, wire } = btn.createEventHandler("onPress");

// 方式 2: 手动连线 — 必须使用 UUID，不是事件名
O.connect(btn.id, "evt-btn-press-001", actionBlock.id, "call");
//              ↑ 必须是 UUID               ↑ 端口名
```

## 诊断与调试

```javascript
const state = O.getState();
// state.mode, state.elements, state.wires, state.compileErrors

// 查找孤立节点（无连线的代码块）
const connectedIds = new Set(state.wires.flatMap(w => [w.fromId, w.toId]));
const isolated = O.codeBlocks.filter(cb => !connectedIds.has(cb.id));

// 查找缺失事件处理器的组件
for (const el of O.views) {
  const info = O.getComponentInfo(el.tag);
  if (!info) continue;
  for (const [evt, meta] of Object.entries(info.events)) {
    const isWired = state.wires.some(w => w.fromId === el.id && w.fromPort === meta.id);
    if (!isWired) console.warn(`${el.tag} "${el.getProp("label")}" 缺失 ${evt}`);
  }
}

// 组件发现
O.listComponents();           // 所有可用的视图组件及其 props/events
O.getComponentInfo("Button"); // 单个组件的详细信息
```

## Common Mistakes

这些问题来自真实 AI agent 使用 OmniDriver 时的实际失败：

| 错误行为 | 为什么错 | 正确做法 |
|---------|---------|---------|
| 用 `connectTo()` 做父子嵌套 | `connectTo()` 创建数据流端口连线，不影响布局层级 | 用 `O.component("Button", props, parentId)` |
| 调用 `card.children([btn])` | `children()` 是只读 getter，不是 setter | 创建时传 `parentId` 参数 |
| `compile()` 后程序化创建的元素消失了 | `compile()` 从 `.omni` 源文件重建画布 | 先 `setSourceCode()` 再 `compile()` |
| 用 `"onPress"` 做端口名连线 | 事件端口是 UUID `"evt-btn-press-001"` | 用 `createEventHandler()` 或查 UUID 表 |
| 期望 `getSourceCode()` 返回程序化创建的元素 | `getSourceCode()` 返回 `activeFileContent`，不反映 Store | 用 `getElements()` / `getState()` 读 Store |
| 用 `el.textContent()` 读文本 | OmniDriver 不是 DOM API | 用 `el.text()` |
| 用 `el.exists()` 检查存在 | ✅ 已内置 `el.exists()` 方法 | 直接用 `el.exists()` |
| 便捷方法传 `parentId` | ✅ 已支持！所有便捷方法最后一个参数可选 `parentId` | `O.button("X", {}, cardId)` |
| 以为 `clearAll()` 会清除源文件内容 | `clearAll()` 只清 Store (elements+wires+selection) | 用 `setSourceCode("")` 清源文件 |

## Error Handling

OmniDriver 方法在失败时**不会抛出异常**，而是返回 `null`/`undefined` 或静默 no-op：

- `O.element("bad-id")` → 返回 `ElementHandle`（即使元素不存在），访问 `.data` 为 `undefined`
- `el.setProp("bad-prop", val)` → 静默操作，不报错
- `O.connect(badId, ...)` → 返回 `WireHandle`，但连线可能无效
- **验证操作是否成功：** 操作后检查 `O.getState()` 或 `el.data`

## 便捷方法 → component() 属性映射

便捷方法的参数位置与 `O.component()` 的 prop key 不一定对应：

| 便捷方法 | 等价 component() 调用 |
|---------|----------------------|
| `O.button("Submit", { variant })` | `O.component("Button", { label: "Submit", variant })` |
| `O.heading("Title", "1")` | `O.component("Heading", { text: "Title", level: "1" })` |
| `O.input("placeholder")` | `O.component("Input", { placeholder: "placeholder" })` |
| `O.text("Hello")` | `O.component("Text", { label: "Hello" })` |
| `O.card({ padding })` | `O.component("Card", { padding })` |

> **需要 parentId 时，必须用 `O.component()` 而不是便捷方法。**

## Red Flags — STOP and Check

- **"我先创建元素再 compile 验证"** → compile() 会丢弃程序化元素，必须先 setSourceCode()
- **"我用 connectTo 把子元素连到父容器"** → connectTo 是数据流，不是布局。用 parentId
- **"children() 怎么设置不了子元素"** → children() 是只读的。创建时传 parentId
- **"我用 'onPress' 做端口名怎么连不上"** → 事件端口是 UUID。用 createEventHandler()
- **"getSourceCode() 怎么返回的是旧内容"** → getSourceCode() 读的是文件内容，不是 Store
- **"compile() 后我的元素全没了"** → 正常行为。compile() 从源码重建画布
- **"O.button('X', {}, parentId) 没嵌套进去"** → 便捷方法不支持 parentId。用 O.component()
- **"el.children = [...] 怎么不生效"** → children() 是只读 getter，不是可赋值属性

## 可用组件

19 种视图组件：Container, Flex, Grid, Card, Text, Heading, Button, Input, TextArea, Select, Checkbox, List, ListItem, Image, Spinner, Progress, Alert, Link, Form

```javascript
const info = O.getComponentInfo("Button");
// { tag, category, description, props, events, styleProperties, hasChildren }
```

## 文件操作

```javascript
O.getActiveFilePath();                     // 当前打开的文件路径
O.getSourceCode();                         // 当前 .omni 源码内容
O.setSourceCode("component App { ... }");  // 设置源码
await O.save();                            // 保存到磁盘
await O.openFile("src/main.omni");         // 打开文件
await O.readFile("src/main.omni");         // 读取任意文件
await O.writeFile("src/main.omni", code);  // 写入任意文件
```
