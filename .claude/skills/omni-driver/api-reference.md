# OmniDriver API Reference

> 完整 API 参考。主 skill 文件：[SKILL.md](SKILL.md)

## OmniDriver 主类

### 状态查询

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `O.getState()` | `EditorSnapshot` | 完整快照：mode, elements, wires, selectedElementId, projectDir, activeFilePath, compileErrors, previewUrl |
| `O.getElements()` | `ElementData[]` | 所有画布元素 |
| `O.getWires()` | `WireData[]` | 所有连线 |
| `O.getMode()` | `"design" \| "code"` | 当前模式 |
| `O.getSelectedElement()` | `ElementHandle \| null` | 当前选中 |
| `O.getPreviewUrl()` | `string \| null` | 预览 URL |
| `O.views` | `ElementHandle[]` | 所有视图元素 (nodeType === "view") |
| `O.codeBlocks` | `ElementHandle[]` | 所有代码块 (nodeType === "codeblock") |

### 元素查找

| 方法 | 参数 | 返回值 |
|------|------|--------|
| `O.element(id)` | `string` | `ElementHandle \| null` |
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

### 组件创建

```javascript
// 通用创建 — 唯一支持 parentId 的方法
O.component("Button", { label: "Submit" }, parentId);
//           tag       props                  parentId (可选)

// 便捷方法 — 不支持 parentId
O.button("Submit", { variant: "primary" });   // label, props
O.text("Hello World");                        // content
O.heading("Title", "1");                      // text, level
O.container({ padding: "16px" });             // props
O.input("Enter name...");                     // placeholder
O.card({ padding: "12px" });                  // props
O.flex("row", "16px");                        // direction, gap
O.image("https://...", "Photo");              // src, alt
O.list();                                     // 无参数
O.alert("info", "注意");                       // type, message
```

### 代码块创建

```javascript
O.codeBlock("signal", "name", {
  parentStore?: string;      // 所属 store
  source?: string;           // 源码
  ports?: PortDescriptor[];  // 额外端口
  typeAnnotation?: string;   // 类型注解
  isFolded?: boolean;        // 是否折叠
});

// 类型预设端口:
// signal:   value(out, TYPE), set(in, TYPE)
// computed: value(out, TYPE), deps(in, any)
// action:   call(in, MouseEvent), result(out, any)
// store:    无预置端口
```

### 事件处理

```javascript
const { handler, wire } = btn.createEventHandler("onPress", {
  source?: string;     // action 源码
  parentStore?: string;
  ports?: PortDescriptor[];
});

// 等价于手动:
// 1. 创建 action 元素（事件名对应 UUID 端口）
// 2. btn.setProp("onPress", handler.id)
// 3. O.connect(btn.id, UUID, handler.id, "call")
```

### 连线

```javascript
// 创建连线
O.connect(fromId, fromPort, toId, toPort);
// fromPort 若是事件端口，必须是 UUID (如 "evt-btn-press-001")

// 断开
O.disconnect(wireId);
```

### 模式与编译

```javascript
O.switchMode("design" | "code");  // 切换模式（派发 omni-mode-change 事件）

// 编译 — 从 activeFileContent 重建画布，丢弃程序化创建的元素
const result = await O.compile();
// { success: boolean, errors: Array<{ code, message, line, severity }> }
```

### 等待机制

```javascript
await O.waitForElement("Button", 5000);                // 按 tag 等待
await O.waitForWire("evt-btn-press-001", 5000);        // 按端口 ID 等待
await O.waitFor(() => O.getElements().length >= 5, 3000, "等待超时");
```

### 撤销/重做

```javascript
O.undo();   // 撤销（依赖 Command 系统）
O.redo();   // 重做
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

### 其他

```javascript
O.clearAll();   // 清除 Store 中的 elements + wires + selection（不触及源文件）
O.batch(fn);    // 批量操作包装
O.removeElement(id); // 删除单个元素
```

---

## ElementHandle API

```javascript
const el = O.element("some-id");

// 属性
el.setProp(prop, value);      // 设置属性
el.getProp(prop);             // 读取属性
el.getPorts();                // 获取端口列表 PortDescriptor[]
el.getSource();               // 获取代码块源文
el.setSource("source code");  // 设置代码块源文
el.data;                      // ElementData 原始数据（含 outputValue）

// 导航
el.select();                  // 选中元素
el.remove();                  // 删除元素
el.children();                // ElementHandle[] — 子元素
el.parent();                  // ElementHandle | null — 父元素

// 连线
el.connectTo(target, "outPort", "inPort");  // 创建数据流连线
el.createEventHandler("onPress", options);  // 创建事件处理器（3合1）

// 等待
await el.waitForLive(timeout);  // 轮询等待元素出现在 store 中
```

**没有的方法（常见错误）：**
- ❌ `el.textContent()` → 用 `el.getProp("label")`
- ❌ `el.exists()` → 用 `el.data !== undefined`
- ❌ `el.isVisible()` → 不存在
- ❌ `el.waitForSelector()` → 用 `O.waitForElement()`
- ❌ `el.count()` → 用 `O.findByTag("tag").length`
- ❌ `el.children = [...]` / `el.children.push(...)` → `children()` 是只读 getter，只能用 `parentId` 建立父子关系

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

## Playwright 集成

```javascript
// test helper
async function omniEval(page, fn) {
  return page.evaluate(fn);
}

test("create and verify form", async ({ page }) => {
  await page.goto("http://localhost:5174");
  await page.waitForFunction(() => window.__OMNI__);

  // 创建 UI
  await omniEval(page, () => {
    const O = window.__OMNI__;
    O.clearAll();
    O.switchMode("design");
    const card = O.card({ padding: "24px" });
    O.component("Heading", { label: "Login" }, card.id);
    O.component("Input", { label: "Email" }, card.id);
    O.component("Button", { label: "Submit" }, card.id);
  });

  // 验证
  const count = await omniEval(page, () => window.__OMNI__.getElements().length);
  expect(count).toBeGreaterThanOrEqual(4);
});
```
