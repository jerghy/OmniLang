---
name: omni-driver
description: Playwright-like API for AI-driven OmniLang editor automation. Use when programmatically creating components, connecting wires, switching modes, compiling, or testing the editor.
metadata:
  type: reference
  tags: [omnilang, editor, automation, driver, api, playwright, testing]
---

# OmniDriver — AI 操控 OmniLang 编辑器的 Playwright 风格 API

OmniDriver 提供了一套类似 Playwright 的程序化 API，让 AI agent 可以通过 `window.__OMNI__` 完全操控 OmniLang 编辑器：
创建组件、连接数据流、编译源码、切换模式、查询状态等。

**核心设计理念：**
- **Locator 模式** — `omni.element(id)` 返回 `ElementHandle`，链式调用
- **Wait 机制** — `omni.waitForElement("Button")` 等待异步节点出现
- **Handle 模式** — `ElementHandle` / `WireHandle` 封装元素和连线的所有操作
- **Component Protocol** — `omni.listComponents()` 发现可用的组件和事件

## 入口

```javascript
// 浏览器 Console / Playwright page.evaluate()
const omni = window.__OMNI__;

// Node.js 脚本中通过 Playwright 操控
const omni = await page.evaluate(() => window.__OMNI__);
```

## API 参考

### 状态查询

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `omni.getState()` | `EditorSnapshot` | 编辑器完整快照 |
| `omni.getElements()` | `ElementData[]` | 所有画布元素 |
| `omni.getWires()` | `WireData[]` | 所有连线 |
| `omni.getSelectedElement()` | `ElementHandle \| null` | 当前选中元素 |
| `omni.getMode()` | `"design" \| "code"` | 当前编辑器模式 |
| `omni.getPreviewUrl()` | `string \| null` | 预览 URL |
| `omni.views` | `ElementHandle[]` | 所有视图元素 |
| `omni.codeBlocks` | `ElementHandle[]` | 所有代码块 |

### 元素查找 (Locator Pattern)

| 方法 | 参数 | 返回值 |
|------|------|--------|
| `omni.element(id)` | `string` | `ElementHandle \| null` |
| `omni.findByTag(tag)` | `string` | `ElementHandle[]` |
| `omni.find(query)` | `ElementQuery` | `ElementHandle[]` |
| `omni.findByProp(prop, value)` | `string, unknown` | `ElementHandle[]` |

### 组件创建

```javascript
// 通用创建
const btn = omni.component("Button", { label: "提交" });
const txt = omni.component("Text", { label: "Hello" });
const container = omni.component("Container", {}, parentId);

// 便捷方法
const btn   = omni.button("Submit", { variant: "primary" });
const txt   = omni.text("Hello World");
const h1    = omni.heading("Title", "1");
const div   = omni.container({ padding: "16px" });
const input = omni.input("Enter name...");
const card  = omni.card({ padding: "12px" });
const row   = omni.flex("row", "16px");
const img   = omni.image("https://example.com/photo.jpg", "Photo");
const list  = omni.list();
const alert = omni.alert("info", "注意");
```

### 代码块创建

```javascript
// 创建各种类型的代码块
const signal   = omni.codeBlock("signal", "count", { source: "signal count: i32 = 0" });
const computed = omni.codeBlock("computed", "doubled", { source: "computed doubled: i32 { return count * 2 }" });
const action   = omni.codeBlock("action", "handleClick", {
  source: 'action handleClick(event) {\n  // handler logic\n}',
  ports: [
    { name: "call", direction: "in", type: "MouseEvent" },
    { name: "result", direction: "out", type: "any" },
  ],
});
const store    = omni.codeBlock("store", "myStore");
```

### 事件处理

```javascript
// 为组件的某个事件创建处理器并自动连线（等同于点击 🔗 按钮）
const btn = omni.button("Submit");
const { handler, wire } = btn.createEventHandler("onPress", {
  source: `action handleSubmit(event) {
  // 处理点击事件
}`,
});
```

### 连线操作

```javascript
// 手动创建连线
const wire = omni.connect(signalId, "value", computedId, "deps");

// 通过 ElementHandle 连线
signal.connectTo(computed, "value", "deps");

// 断开连线
omni.disconnect(wireId);
wire.disconnect();
```

### ElementHandle API

```javascript
const el = omni.element("some-id");

el.setProp("label", "New Label");   // 设置属性
el.getProp("label");                // 获取属性
el.select();                        // 选中元素（显示属性面板）
el.remove();                        // 删除元素
el.getPorts();                      // 获取端口列表
el.connectTo(target, "out", "in");  // 连接到另一个元素
el.children();                      // 子元素列表
el.parent();                        // 父元素
el.setSource("signal x: i32 = 1");  // 设置代码块源码
el.getSource();                     // 获取代码块源码
await el.waitForLive(5000);         // 等待元素出现在 store 中
```

### WireHandle API

```javascript
const wire = omni.connect(...);

wire.disconnect();      // 断开连线
wire.source();          // 获取源元素 ElementHandle
wire.target();          // 获取目标元素 ElementHandle
wire.data;              // 获取 WireData 原始数据
```

### 模式切换

```javascript
omni.switchMode("code");    // 切换到代码模式
omni.switchMode("design");  // 切换到设计模式
omni.getMode();             // 获取当前模式
```

### 编译

```javascript
const result = await omni.compile();
// { success: boolean, errors: Array<{code, message, line, severity}> }

if (result.errors.length > 0) {
  console.error("编译错误:", result.errors);
}
```

### 组件发现

```javascript
// AI agent 可以用这个发现所有可用组件
const components = omni.listComponents();
// [{ tag: "Button", category: "Actions", description: "按钮",
//    props: { label: { type: "string", default: "Button", editor: "text" }, ... },
//    events: { onPress: { id: "evt-btn-press-001", description: "点击", payload: "MouseEvent" }, ... },
//    styleProperties: [...], hasChildren: true }, ...]

// 查看特定组件
const btnInfo = omni.getComponentInfo("Button");
```

### Wait 机制

```javascript
// 等待特定组件出现在画布上
const btn = await omni.waitForElement("Button", 5000);

// 等待特定连线出现
const wire = await omni.waitForWire("evt-btn-press-001", 5000);

// 等待自定义条件
await omni.waitFor(() => omni.getElements().length >= 5, 3000, "不够 5 个元素");
```

### 撤销 / 重做

```javascript
omni.undo();   // Ctrl+Z
omni.redo();   // Ctrl+Y
```

### 文件操作

```javascript
const code = omni.getSourceCode();           // 获取当前 .omni 源码
omni.setSourceCode("component App { ... }"); // 设置源码

await omni.save();                           // 保存到磁盘
await omni.openFile("src/main.omni");        // 打开文件
const content = await omni.readFile("src/main.omni"); // 读取文件
await omni.writeFile("src/main.omni", code); // 写入文件
```

### 批量操作

```javascript
omni.batch((driver) => {
  const btn = driver.button("A");
  const txt = driver.text("B");
  driver.connect(btn.id, "evt-btn-press-001", txt.id, "deps");
});
```

### 清理

```javascript
omni.clearAll();  // 删除所有元素和连线
```

## 典型使用场景

### 场景 1: AI 自动创建 UI 并连线

```javascript
// 1. 切换到设计模式
omni.switchMode("design");

// 2. 创建 UI
const container = omni.container({ padding: "20px" });
const heading = omni.heading("用户注册", "1");
const nameInput = omni.input("请输入姓名");
const submitBtn = omni.button("提交", { variant: "primary" });

// 3. 为按钮创建事件处理器
const { handler, wire } = submitBtn.createEventHandler("onPress", {
  source: `action handleSubmit(event) {
  // 处理提交逻辑
  signal result: String = "提交成功"
}`,
});

// 4. 切换到代码模式查看
omni.switchMode("code");

// 5. 编译验证
const result = await omni.compile();
console.log(result.success ? "✅ 编译通过" : `❌ ${result.errors.length} 个错误`);
```

### 场景 2: 通过 Playwright 进行 E2E 测试

```javascript
// test.spec.js
const { test, expect } = require("@playwright/test");

test("AI creates a login form", async ({ page }) => {
  await page.goto("http://localhost:5173");

  const result = await page.evaluate(async () => {
    const omni = window.__OMNI__;

    // 清理画布
    omni.clearAll();

    // 创建表单
    const card = omni.card({ padding: "24px" });
    const heading = omni.heading("登录", "2");
    const emailInput = omni.input("邮箱");
    const passwordInput = omni.input("密码");
    const loginBtn = omni.button("登录", { variant: "primary" });

    // 连线
    const { handler } = loginBtn.createEventHandler("onPress");

    // 切换到代码模式
    omni.switchMode("code");

    return {
      elementCount: omni.getElements().length,
      wireCount: omni.getWires().length,
    };
  });

  expect(result.elementCount).toBeGreaterThanOrEqual(5);
  expect(result.wireCount).toBeGreaterThanOrEqual(1);
});
```

### 场景 3: AI 批量创建数据流

```javascript
// 在浏览器 Console 中直接执行
const omni = window.__OMNI__;

// 创建 store → signal → computed → view 完整数据流
const store = omni.codeBlock("store", "userStore");

const firstName = omni.codeBlock("signal", "firstName", {
  parentStore: "userStore",
  source: 'signal firstName: String = ""',
});

const lastName = omni.codeBlock("signal", "lastName", {
  parentStore: "userStore",
  source: 'signal lastName: String = ""',
});

const fullName = omni.codeBlock("computed", "fullName", {
  source: `computed fullName: String {
  return firstName + " " + lastName
}`,
});

// 连线 data flow
firstName.connectTo(fullName, "value", "deps");
lastName.connectTo(fullName, "value", "deps");

// 创建 UI 绑定
const card = omni.card();
const text = omni.text("用户: ");
// TODO: bind text to fullName.value

console.log("元素数量:", omni.getElements().length);
console.log("连线数量:", omni.getWires().length);
```

### 场景 4: AI 诊断和修复

```javascript
const omni = window.__OMNI__;

async function diagnoseAndFix() {
  // 1. 检查状态
  const state = omni.getState();
  console.log("当前模式:", state.mode);
  console.log("元素数量:", state.elements.length);
  console.log("连线数量:", state.wires.length);

  // 2. 编译检查
  const result = await omni.compile();
  if (!result.success) {
    console.error("编译错误:");
    result.errors.forEach(e => console.error(`  L${e.line}: ${e.message}`));
  }

  // 3. 查找孤立节点（没有连线的代码块）
  const codeBlocks = omni.codeBlocks;
  const connectedIds = new Set();
  state.wires.forEach(w => {
    connectedIds.add(w.fromId);
    connectedIds.add(w.toId);
  });

  const isolated = codeBlocks.filter(cb => !connectedIds.has(cb.id));
  console.log("孤立节点:", isolated.length);
  isolated.forEach(cb => console.log(`  - ${cb.data?.tag} "${cb.getProp("name")}"`));

  // 4. 检查缺失的事件处理器
  const buttons = omni.findByTag("Button");
  for (const btn of buttons) {
    const meta = omni.getComponentInfo("Button");
    for (const [eventName, eventMeta] of Object.entries(meta.events)) {
      const handlerProp = btn.getProp(eventName);
      if (!handlerProp) {
        console.warn(`按钮 "${btn.getProp("label")}" 缺少 ${eventName} 处理器`);
      }
    }
  }

  return { isolated, errors: result.errors };
}
```

## Playwright 集成

### 在 Playwright 测试中注入 OmniDriver 脚本

```javascript
// playwright.config.js helpers
async function getOmniDriver(page) {
  return page.evaluate(() => {
    if (!window.__OMNI__) throw new Error("OmniDriver not initialized");
    // 返回是否可用即可，实际操作通过 evaluate 进行
    return true;
  });
}

// 封装常用操作
async function omniAction(page, action) {
  return page.evaluate(action);
}

// test helper
test("AI-driven test", async ({ page }) => {
  await page.goto("http://localhost:5173");

  await omniAction(page, () => {
    const omni = window.__OMNI__;
    omni.clearAll();
    omni.switchMode("design");
    omni.button("Test Button");
    omni.switchMode("code");
  });

  const snapshot = await omniAction(page, () => window.__OMNI__.getState());
  expect(snapshot.elements.length).toBe(1);
});
```

## 组件协议参考

OmniDriver 从 `COMPONENT_PROTOCOL`（定义在 `ComponentPalette.tsx`）自动提取组件元数据。
每个组件包含：

| 字段 | 说明 | 示例 |
|------|------|------|
| `tag` | 组件标签名 | `"Button"` |
| `category` | 分类 | `"Actions"`, `"Layout"`, `"Forms"` |
| `props` | 可编辑属性 | `{ label: { type: "string", default: "Button" } }` |
| `events` | 事件列表（含 UUID） | `{ onPress: { id: "evt-btn-press-001", payload: "MouseEvent" } }` |
| `styleProperties` | 样式属性 | `["width", "padding", "color"]` |
| `hasChildren` | 是否支持子元素 | `true` / `false` |

### 事件端口 UUID 对照表

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

## 注意事项

1. **OmniDriver 在 App 组件挂载后初始化** — 确保页面完全加载后再使用
2. **Rete.js 节点创建是异步的** — 使用 `waitForElement` / `waitForLive` 等待节点渲染
3. **端口连线使用 UUID** — 事件端口必须使用 Component Protocol 中定义的 UUID（如 `evt-btn-press-001`），不是事件名称
4. **compile() 会清空画布** — 编译后画布会从源码重新构建，之前程序化创建的元素会丢失
5. **undo/redo 依赖 Command 系统** — 直接调用 store 的操作（非 Command 包装）不会被记录到撤销栈
