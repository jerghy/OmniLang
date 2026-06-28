# OmniView 语法参考

> **版本**：基于 omni-compiler v0.1.0（Rust 编译器）和 learn-omniview 教学项目
> **最后更新**：2026-06-27
>
> 本文档是关于 OmniView 框架语法的权威参考资料。所有语法构造均来自 Rust 编译器源码（`omni-compiler/src/`）的实际实现，代码示例取自 `test/projects/learn-omniview/src/` 中可编译运行的 .omni 文件。

---

## 目录

1. [概述](#1-概述)
2. [项目结构](#2-项目结构)
3. [模块系统](#3-模块系统)
4. [类型系统](#4-类型系统)
5. [函数与控制流](#5-函数与控制流)
6. [组件系统](#6-组件系统)
7. [Store（状态管理）](#7-store状态管理)
8. [View 模板语法](#8-view-模板语法)
9. [样式系统](#9-样式系统)
10. [事件系统](#10-事件系统)
11. [生命周期](#11-生命周期)
12. [动画与运动系统](#12-动画与运动系统)
13. [标准库与内置函数](#13-标准库与内置函数)
14. [附录 A：完整关键词与运算符](#14-附录-a完整关键词与运算符)
15. [附录 B：内置组件目录](#15-附录-b内置组件目录)

---

## 1. 概述

### 1.1 OmniView 是什么

OmniView 是 OmniLang 语言的 UI 框架层。类比 Dart + Flutter 的关系：**OmniLang 是通用编程语言，OmniView 是构建用户界面的框架库**。

OmniView 通过特殊的语法扩展（组件、Store、View 模板、动画等）为声明式 UI 开发提供一等语言支持，同时复用 OmniLang 核心语言的类型系统、表达式和控制流。

### 1.2 `import omniview;`

任何使用 OmniView 特性的 `.omni` 文件都**必须**在文件开头包含：

```omni
import omniview;
```

这个导入充当"模式门控"——编译器根据文件中是否出现 `import omniview;` 来决定是走 UI 代码生成路径还是纯逻辑代码生成路径。

> **纯逻辑文件不需要此导入**。例如，`lib/types.omni`、`lib/stats.omni` 等纯工具库文件不写 `import omniview;`。

### 1.3 文件类型

| 文件类型 | `import omniview` | 可包含的构造 |
|---------|-------------------|------------|
| 纯逻辑文件 | 不需要 | `fn`, `struct`, `type`, `import`, `export` |
| UI 组件文件 | 需要 | 以上全部 + `component`, `store`, `view`, `style`, `@motion` |
| Store 文件 | 需要 | `store` + `signal`, `getter`, `action` |
| 主题文件 | 可选 | `theme` |

---

## 2. 项目结构

### 2.1 项目配置文件 `.omnilang.toml`

每个 OmniView 项目根目录下需要一个 `.omnilang.toml` 文件：

```toml
[project]
name = "learn-omniview"
version = "1.0.0"

[aliases]
"@lib" = "./src/lib"
"@shared" = "./src/shared"

[source]
dir = "src"

[output]
dir = "build"
```

### 2.2 推荐目录布局

```
project/
├── .omnilang.toml          # 项目配置
├── src/
│   ├── main.omni            # 入口 + 根组件 App
│   ├── components/          # 可复用 UI 组件
│   ├── pages/               # 页面级组件
│   ├── stores/              # 全局状态管理
│   ├── shared/              # 主题令牌、常量
│   └── lib/                 # 纯逻辑工具库（类型、函数）
└── build/                   # 编译输出（自动生成）
```

### 2.3 构建与运行

```bash
# 运行开发模式
omnic run

# 构建 OmniView 目标
omnic build --target omniview

# 构建其他目标
omnic build --target react
omnic build --target flutter
```

---

## 3. 模块系统

### 3.1 命名导入

```omni
import { TodoStore } from "./stores/TodoStore";
import { ThemeStore } from "./stores/ThemeStore";
import { http_get } from "std/http";
```

### 3.2 命名空间导入

```omni
import "./lib/stats" as StatsLib;
import "./lib/transform" as TransformLib;

// 使用：StatsLib.average(...)
```

### 3.3 仅类型导入

```omni
import type { StatisticsData } from "./lib/types";
import "./lib/types" as Types;
```

### 3.4 重导出

```omni
export { StatisticsData } from "./types";
export { Percentage } from "./types";
```

### 3.5 `pub` 可见性

```omni
// 公有——可被其他文件导入
pub struct StatisticsData {
    name: String,
    count: i32,
}

pub fn average(values: Array<f64>) -> f64 {
    // ...
}

// 私有——仅当前文件可见（没有 `pub` 前缀）
fn helper() -> i32 { return 0; }
```

---

## 4. 类型系统

### 4.1 原始类型

| 类型 | 含义 | 示例 |
|------|------|------|
| `i8`, `i16`, `i32`, `i64` | 有符号整数 | `42`, `-7` |
| `u8`, `u16`, `u32`, `u64` | 无符号整数 | `255` |
| `f32`, `f64` | 浮点数 | `3.14`, `1.5e10` |
| `bool` | 布尔 | `true`, `false` |
| `String` | 字符串 | `"hello"` |
| `Void` | 空类型 | 用于无返回值函数 |

### 4.2 复合类型

```omni
// 数组
signal items: Array<String> = [];
signal numbers: Array<i32> = [1, 2, 3];

// 映射
signal config: Map<String, i32> = {};
```

### 4.3 可空类型

使用 `?` 后缀表示可空：

```omni
signal name: String? = null;
event onOptional: String?;   // 可选事件
```

### 4.4 结构体

```omni
// 默认：结构等价（structural）
pub struct Point {
    x: i32,
    y: i32,
}

// 名义结构体（nominal）—— 禁止结构等价，类型不透明
struct nominal UserId {
    value: i32,
}
```

### 4.5 标记联合体（Tagged Union / 代数数据类型）

```omni
// 变体可以是空变体，或携带字段
type ValidationResult = Valid { message: String }
                      | Invalid { errors: Array<String> }
                      | Pending

type ApiResult = Ok { data: String }
               | Err { error: String }
               | Loading
```

构造变体：

```omni
fn ok_result(data: String) -> ApiResult {
    return Ok { data: data };
}

fn loading_result() -> ApiResult {
    return Loading;   // 空变体无需字段语法
}
```

模式匹配（穷举）：

```omni
fn unwrap_or_default(result: ApiResult, default: String) -> String {
    match result {
        Ok { data } => { return data; },
        Err => { return default; },
        Loading => { return default; }
    }
}
```

### 4.6 类型别名

```omni
type Score = i32;
pub type Percentage = f64;
```

---

## 5. 函数与控制流

### 5.1 函数声明

```omni
// 无返回值（隐式 Void）
fn handleClick() {
    count = count + 1;
}

// 带参数和返回类型
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// 公有函数
pub fn average(values: Array<f64>) -> f64 {
    // ...
}

// 异步函数
pub async fn fetch_demo(url: String) -> ApiResult {
    let raw = await http_get(url);
    return Ok { data: raw };
}
```

### 5.2 变量声明

```omni
let x = 42;              // 类型推断
let mut counter = 0;     // 可变变量
let name: String = "hi"; // 带类型注解
```

### 5.3 控制流

```omni
// if / else if / else
if score > 90 {
    grade = "A";
} else if score > 80 {
    grade = "B";
} else {
    grade = "C";
}

// while 循环
let mut i = 0;
while i < 10 {
    i = i + 1;
}

// for..in 循环
for item in items {
    println(item);
}

// break / continue
for n in numbers {
    if n < 0 { continue; }
    if n > 100 { break; }
}

// return 提前返回
if input == "" {
    return "empty";
}
```

### 5.4 运算符

| 类别 | 运算符 |
|------|--------|
| 算术 | `+`, `-`, `*`, `/`, `%` |
| 赋值 | `=`, `+=`, `-=`, `*=`, `/=`, `%=` |
| 比较 | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| 逻辑 | `&&`, `||`, `!` |
| 空值 | `??` (空值合并), `?.` (可选链) |

```omni
// 空值合并
let display = name ?? "Guest";

// 可选链
let len = user?.name?.len();
```

### 5.5 闭包

```omni
// 箭头闭包
let add = |a: i32, b: i32| a + b;

// 多行闭包
let compute = |x: i32| {
    let y = x * 2;
    return y + 1;
};
```

### 5.6 异常处理

```omni
fn process() {
    try {
        let result = risky_operation();
        println(result);
    } catch {
        println("something went wrong");
    } finally {
        cleanup();
    }
}
```

---

## 6. 组件系统

### 6.1 组件声明

```omni
import omniview;

component ComponentName {
    // 成员声明...
    view { /* 模板 */ }
    style { /* 样式 */ }
}
```

### 6.2 Props（属性）

```omni
component TodoItem {
    prop text: String;
    prop count: i32 = 0;          // 带默认值
    prop visible: bool = true;
    prop status: String = "active";
}
```

传入 Props：

```omni
<TodoItem text="Buy milk" />
<TodoItem text={item} count={5} />
```

### 6.3 Signals（局部响应式状态）

```omni
signal count: i32 = 0;
signal input: String = "";
signal isActive: bool = false;
signal items: Array<String> = [];
signal user: Map<String, String> = {};
```

Signal 是组件内的响应式状态。修改 signal 会自动触发依赖它的 DOM 更新。

```omni
fn handleAdd() {
    count = count + 1;
    items.push("new item");
}
```

### 6.4 Computed（计算属性）

两种形式：

```omni
// 形式 1：表达式体（单行）
computed doubled: i32 = count * 2;
computed greeting: String = "Hello, " + user.name;
computed hint: String = if input == ""
    { "输入待办事项" }
    else { "已输入 " + input.len().to_string() + " 字符" };

// 形式 2：函数体（带 {...} 块）
computed fullName() -> String {
    return firstName + " " + lastName;
}
```

Computed 自动追踪依赖的信号，仅在依赖变化时重新计算。

### 6.5 函数（fn）

```omni
fn handleClick() {
    count = count + 1;
}

fn switchTab(tab: String) {
    activeTab = tab;
}

fn isActive(tab: String) -> bool {
    return activeTab == tab;
}
```

### 6.6 常量（const）

```omni
const MAX_COUNT = 100;
const ICON_URL = "/assets/icon.png";
const APP_NAME = "OmniView Learn";
```

常量在组件内声明，编译时求值。

### 6.7 Ref（元素引用）

```omni
component StatsDashboard {
    ref container: HtmlElement;

    // ...
    view {
        <div ref: container class: dashboard>
            <!-- ... -->
        </div>
    }
}
```

### 6.8 Store 引用

```omni
component App {
    @store TodoStore;
    @store ThemeStore;
    @store NotificationStore;

    // 在 view 中可以直接访问 store 的信号
    view {
        <p>Total: {TodoStore.totalCount}</p>
    }
}
```

### 6.9 内联 Store

Store 可以直接定义在组件内部（不导出到外部）：

```omni
component InlineStoreDemo {
    store CartStore {
        signal items: Array<String> = [];
        signal totalPrice: f64 = 0.0;

        getter itemCount: i32 = items.len();

        action addItem(name: String, price: f64) {
            items.push(name);
            totalPrice = totalPrice + price;
        }

        action clear() {
            items = [];
            totalPrice = 0.0;
        }
    }

    fn addApple() {
        CartStore.addItem("Apple", 5.0);
    }
}
```

### 6.10 `if_mode` —— 组件级 if 过渡模式

```omni
component ComponentSpecialsDemo {
    if_mode: "wait";   // 等待 @if 的 @leave 动画完成后，再渲染新内容

    // mode 取值：
    //   "sync"  — 默认：新旧内容同时存在，离开动画和进入动画并行
    //   "wait"  — 等待离开动画完成后，才渲染进入内容
}
```

---

## 7. Store（状态管理）

Store 是全局响应式状态容器，可在多个组件之间共享。

### 7.1 Store 声明

```omni
import omniview;

store TodoStore {
    // ── 响应式状态 ──
    signal items: Array<String> = [];
    signal filter: String = "all";
    signal lastAction: String = "";

    // ── 派生值（getter）──
    getter totalCount: i32 = items.len();
    getter hasItems: bool = !items.is_empty();

    // ── 修改状态的动作（action）──
    action add(text: String) {
        if text != "" {
            items.push(text);
            lastAction = "added: " + text;
        }
    }

    action remove() {
        items.pop();
        lastAction = "removed last item";
    }

    action clear() {
        items = [];
        lastAction = "all cleared";
    }
}
```

### 7.2 Getter（派生状态）

```omni
getter totalCount: i32 = array_length(items);
getter filterStatus: String = filter;
getter summary: String = if items.len() == 0
    { "cart empty" }
    else { "total: " + items.len().to_string() + " items" };
```

Getter 是对 Store 中信号的派生计算，自动追踪依赖。

### 7.3 Action（状态修改）

```omni
// 基本 action
action setFilter(mode: String) {
    filter = mode;
}

// 带返回类型的 action
action addTask(title: String, priority: i32, due: String): String {
    if title == "" {
        return "error: title required";
    }
    items.push(title);
    return "ok: " + title;
}

// 异步 action
action fetchReminders(): String async {
    let mut count = 0;
    // ... 异步操作 ...
    return "synced " + count.to_string() + " reminders";
}
```

> **异步 action 语法**：`async` 关键字放在返回类型之后、`{` 之前。

### 7.4 使用 Store

```omni
// 导入
import { TodoStore } from "./stores/TodoStore";

// 注入到组件
component App {
    @store TodoStore;

    // 读取信号/getter（在 view 和逻辑中均可直接访问）
    view {
        <p>Total: {TodoStore.totalCount}</p>
        <p>Filter: {TodoStore.filterStatus}</p>
    }
}

// 调用 action
fn addTask() {
    TodoStore.add("New task");
}

// 调用带返回值的 action
fn handleAdd() {
    let result = TodoStore.addTask("Buy milk", 5, "today");
    println(result);
}
```

---

## 8. View 模板语法

### 8.1 HTML 元素

OmniView 支持完整的 HTML 语义标签：

```omni
view {
    <div>
        <header>
            <nav aria-label="Main navigation">
                <h1>App Title</h1>
            </nav>
        </header>
        <main>
            <section>
                <p>Content goes here</p>
            </section>
        </main>
        <footer>
            <p>Footer text</p>
        </footer>
    </div>
}
```

**自动闭合的 void 元素**（无需写 `</tag>`）：

`area`, `base`, `br`, `col`, `embed`, `hr`, `img`, `input`, `link`, `meta`, `param`, `source`, `track`, `wbr`

### 8.2 组件标签

以 PascalCase（首字母大写）开头的标签被识别为组件引用：

```omni
view {
    <div>
        <Header />
        <TodoPage />
        <TodoItem text={item} />
        <Modal>
            @slot header { <h2>Title</h2> }
        </Modal>
    </div>
}
```

### 8.3 属性绑定

```omni
<!-- 静态字符串 -->
<input placeholder="Enter text" />

<!-- 动态表达式 -->
<input placeholder={hint} />

<!-- 布尔属性（单独写属性名即 true） -->
<input disabled />

<!-- class: 绑定 -->
<div class: container>...</div>
<div class: btn primary>...</div>

<!-- class: 条件映射 -->
<button class: { browserTab: true, tabActive: isActive("home") }>
    Home
</button>

<!-- style: 内联样式 -->
<div style: { background: ThemeStore.currentBg, padding: "16px" }>
    Content
</div>

<!-- ref: 元素引用 -->
<div ref: container>...</div>

<!-- testId: 测试定位 -->
<div testId="demo-container">...</div>

<!-- data-* 自定义属性 -->
<div data-testid="name-field" data-type="text">...</div>
```

### 8.4 文本插值

```omni
<p>Count: {count}</p>
<p>Hello {user ?? "Guest"}</p>
<p>已输入 {input.len()} 字符</p>
```

### 8.5 双向绑定

```omni
<!-- 文本输入绑定 -->
<input bind:value={input} />

<!-- 复选框绑定 -->
<input type="checkbox" bind:checked={agreed} />
```

### 8.6 条件渲染

```omni
@if status == "loading" {
    <p>Loading...</p>
}
@else if status == "error" {
    <p>Something went wrong</p>
}
@else {
    <p>Content loaded</p>
}

<!-- 带过渡模式 -->
@if showPanel mode="sync" {
    <div @enter={fadeEnter} @leave={fadeOut}>
        Panel content
    </div>
}
```

### 8.7 列表渲染

```omni
@for item in items {
    <TodoItem text={item} />
}
@empty {
    <p>No items found</p>
}

<!-- 带 key（用于动画追踪） -->
@for item in items key={item.id} {
    <TodoItem text={item.name} />
}
@empty {
    <p>No items</p>
}
```

### 8.8 插槽（Slot）

**提供插槽的组件**：

```omni
component SlotPanel {
    view {
        <div>
            <div class: header>
                <!-- 命名插槽（无括号） -->
                @slot header
            </div>

            <!-- 匿名插槽 -->
            @slot

            <div class: footer>
                <!-- 命名插槽（带括号） -->
                @slot(footer)

                <!-- 带默认回退内容的插槽 -->
                @slot actions {
                    <button>Default Action</button>
                }
            </div>
        </div>
    }
}

component SlotContainer {
    view {
        <!-- 仅括号形式 -->
        @slot(header)
        @slot(body)
        @slot(footer)
    }
}
```

**消费插槽的组件**：

```omni
<SlotPanel>
    <!-- 填充命名插槽 -->
    @slot header {
        <h2>Custom Header</h2>
    }

    <!-- 填充匿名插槽 -->
    @slot {
        <p>Custom body content</p>
    }

    <!-- 不提供 actions → 使用默认回退内容 -->
</SlotPanel>

<!-- 消费括号形式插槽 -->
<SlotContainer>
    <div slot="header">Header content</div>
    <p slot="body">Body content</p>
    <button slot="footer">Close</button>
</SlotContainer>
```

### 8.9 HTML 注释

```omni
<!-- 这是 HTML 注释，不会渲染到 DOM -->
<div>Visible content</div>
```

---

## 9. 样式系统

### 9.1 Scoped 样式块

在 `style { }` 块中定义的样式自动带有组件作用域：

```omni
component AddTodo {
    view { /* ... */ }

    style {
        .container {
            background: white;
            padding: 20px;
        }
        .title {
            font-size: 18px;
            color: #1f2937;
        }
        .btn-add {
            padding: 12px 20px;
            background: #6366f1;
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 16px;
            cursor: pointer;
        }
        .btn-add:hover {
            background: #4f46e5;
        }
        .btn-add:focus {
            outline: 2px solid #818cf8;
        }
    }
}
```

### 9.2 支持的 CSS 单位

`px`, `rem`, `em`, `vh`, `vw`, `%`, `pt`, `cm`, `mm`, `fr`, `ms`, `s`, `deg`, `rad`, `grad`, `turn`, `vmin`, `vmax`, `svh`, `svw`, `lvh`, `lvw`, `dvh`, `dvw`, `cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, `cqmax`, `ch`, `ex`, `ic`, `lh`, `rlh`

### 9.3 伪类

支持 `:hover`, `:focus`, `:active` 等标准 CSS 伪类：

```omni
style {
    .btn:hover { background: #4f46e5; }
    .input:focus { border-color: #6366f1; }
}
```

### 9.4 CSS 自定义属性（CSS 变量）

**在 style 块中声明和使用**：

```omni
style {
    .wrapper {
        --primary: #6366f1;
        --border-color: #cbd5e1;
        --radius-md: 12px;
    }
    .card {
        background: var(--primary);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
    }
}
```

**在内联样式中使用（静态值）**：

```omni
<div style: {
    --box-bg: "#e0e7ff",
    --box-text: "#3730a3",
    --box-radius: "8px"
}>
    Content
</div>
```

**在内联样式中使用（动态信号绑定）**：

```omni
<div style: {
    --accent: accentColor,             // 信号引用 → 响应式更新
    border: "2px solid " + accentColor
}>
    Current: {accentColor}
</div>
```

### 9.5 组合选择器

```omni
style {
    .card { ... }
    .card.highlighted { ... }     // 复合类选择器
    .card:hover { ... }           // 伪类
}
```

---

## 10. 事件系统

### 10.1 自定义事件声明

```omni
component AddTodo {
    // 基本事件
    event onAdd: String;

    // 可选事件（带 ?）
    event onClose: String?;

    // 无负载事件
    event onAction: Void;
}
```

### 10.2 事件触发

两种等价方式：

```omni
// 方式 1：函数调用形式
onAdd(input);
onDelete(index);

// 方式 2：emit 关键字
emit onAdd("task text");
emit onDelete(3);
```

### 10.3 事件绑定

支持 `@onEventName` 和 `onEventName` 两种写法（`on` + PascalCase 形式可省略 `@`）：

```omni
<!-- @前缀形式 -->
<button @onPress={handleAdd}>Add</button>
<button @onClick={greet("World")}>Greet</button>
<input @onInput={addLog("input")} />

<!-- on前缀形式（无@） -->
<button onPress={handleAdd}>Add</button>
<button onClick={greet("World")}>Greet</button>
```

### 10.4 事件处理器类型

```omni
<!-- 1. 函数引用 -->
<button @onPress={handleClick}>Click</button>

<!-- 2. 带参数的函数调用 -->
<button @onPress={switchTab("home")}>Home</button>

<!-- 3. 内联闭包 -->
<button @onPress={() => { input = ""; }}>Clear</button>

<!-- 4. Store action -->
<button @onPress={TodoStore.add}>Add</button>
```

### 10.5 完整事件列表

**鼠标事件**：`@onClick`, `@onDoubleClick`, `@onContextMenu`, `@onMouseDown`, `@onMouseUp`, `@onMouseMove`, `@onMouseEnter`, `@onMouseLeave`

**键盘事件**：`@onKeyDown`, `@onKeyUp`, `@onKeyPress`

**输入事件**：`@onInput`, `@onChange`, `@onFocus`, `@onBlur`, `@onCopy`, `@onCut`, `@onPaste`

**表单事件**：`@onSubmit`, `@onReset`

**拖拽事件**：`@onDrag`, `@onDragStart`, `@onDragOver`, `@onDragEnd`, `@onDrop`

**触摸事件**：`@onTouchStart`, `@onTouchEnd`, `@onTouchMove`

**滚轮事件**：`@onWheel`

**媒体事件**：`@onLoad`, `@onError`, `@onPlay`, `@onPause`, `@onEnded`

**动画事件**：`@onAnimationStart`, `@onAnimationEnd`, `@onAnimationIteration`, `@onTransitionEnd`

**其他**：`@onScroll`, `@onToggle`

---

## 11. 生命周期

### 11.1 `@mount`

组件挂载到 DOM 后执行一次：

```omni
@mount {
    console_log("Component mounted");
    // 初始化数据、加载资源
    TodoStore.load();
}
```

### 11.2 `@unmount`

组件从 DOM 移除前执行：

```omni
@unmount {
    println("Component unmounting, counter: " + counter.to_string());
    // 清理定时器、取消订阅
}
```

### 11.3 `@effect`

追踪的信号变化时执行（在 DOM 更新之后）：

```omni
// 追踪单个信号
@effect(input) {
    if input.len() > 20 {
        println("输入较长：" + input.len().to_string() + " 字符");
    }
}

// 追踪多个信号
@effect(username, fontSize) {
    // 当 username 或 fontSize 变化时执行
    save_preferences(username, fontSize);
}

// 追踪 Store 中的信号
@effect(TodoStore.items) {
    console_log("Total items: " + TodoStore.totalCount.to_string());
}
```

---

## 12. 动画与运动系统

OmniView 内置了声明式动画系统，灵感来自 GSAP 和 Framer Motion，语法使用 `@motion` 和 `@timeline`。

### 12.1 `@motion` —— 关键帧动画

```omni
@motion fadeInUp {
    from { opacity: 0, y: 20 }
    to   { opacity: 1, y: 0 }
    over 400
    ease easeOut
}
```

**多层关键帧（百分比）**：

```omni
@motion multiStep {
    0%   { opacity: 0, scale: 0.8 }
    50%  { opacity: 0.5, scale: 1.1 }
    100% { opacity: 1, scale: 1 }
    over 800
    ease easeInOut
}
```

### 12.2 `@timeline` —— 序列动画编排

```omni
@timeline pageEnter {
    label intro
    stagger(0.05)

    header {
        from { y: -20, opacity: 0 }
        to   { y: 0, opacity: 1 }
        over 300 ease easeOut
    }

    then body {
        from { opacity: 0 }
        to   { opacity: 1 }
        over 400 delay 200 ease easeOut
    }
}
```

**时间线连接器**：
- `then` — 顺序执行，上一个动画完成后再开始
- `with` — 并行执行，与上一个动画同时开始
- 默认（无连接器）— 等同于 `then`

**时间线位置控制**：

| 值 | 含义 |
|----|------|
| `after` | 在上一个动画完成后开始（默认） |
| `withPrev` | 与上一个动画同时开始 |
| `overlap(N)` | 在上一个动画结束前 N 毫秒开始 |
| `afterPrev(N)` | 在上一个动画完成后延迟 N 毫秒开始 |
| `withOffset(N)` | 从上一个动画开始后 N 毫秒开始 |
| `absolute(N)` | 从时间线开始后 N 毫秒开始 |

### 12.3 元素动画触发器

```omni
<!-- 进入/离开动画 -->
<div @enter={fadeInUp} @leave={fadeOut}>Content</div>

<!-- 悬停动画 -->
<button @hover={pulse} @unhover={fadeBack}>Hover me</button>

<!-- 点击/按压 -->
<button @press={scalePress} @click={handleClick}>Click</button>

<!-- 焦点 -->
<input @focus={highlight} />

<!-- 循环动画 -->
<div @animate={pulse}>Animating</div>

<!-- 滚动进入视口 -->
<div @inView={fadeInUp}>Scroll to reveal</div>

<!-- 滚动驱动动画 -->
<div @scroll={parallax} @scroll-start="top" @scroll-end="bottom" @scroll-scrub={0.5}>
    Parallax content
</div>

<!-- 拖拽动画 -->
<div @dragHorizontal={dragAnim} @drag-constraints="0, 300">
    Drag me
</div>

<!-- 时间线绑定 -->
<div @timeline={pageEnter}>Timeline animated</div>
```

### 12.4 Trigger 子属性

```omni
<!-- @hover/@unhover 方向控制 -->
<div @hover={scaleUp} @hover-reverse="forward">...</div>

<!-- @inView 行为控制 -->
<div @inView={fadeIn} @in-view-once={true}>Animate once</div>
<div @inView={fadeIn} @in-view-repeat={true}>Animate every time</div>
<div @inView={fadeIn} @in-view-threshold={0.5}>50% visible</div>

<!-- @drag 约束和物理 -->
<div @dragHorizontal={dragAnim}
     @drag-constraints="0, 500"
     @drag-elastic={0.3}
     @drag-momentum={0.8}
     @drag-threshold={5}
     @drag-velocity-max={1000}>
    Draggable
</div>

<!-- 暂停和驱动 -->
<div @enter={anim} @paused={isPaused}>
<div @enter={anim} @driven-by={scrollProgress}>
```

### 12.5 States 动画

定义元素在不同交互状态下的动画：

```omni
<div states: {
    idle:    { scale: 1, opacity: 1 }   over: 150,
    hover:   { scale: 1.05, opacity: 1 } over: 150,
    pressed: { scale: 0.95, opacity: 0.8 } over: 100
}>
    Interactive Panel
</div>
```

### 12.6 `on:` 信号驱动动画

将元素的动画进度绑定到一个信号值（0-100）：

```omni
<div on: { progressPct } class: signal-bar>
    Progress: {progressPct}%
</div>
```

### 12.7 Hero 过渡

共享元素过渡动画：

```omni
component MyComponent {
    // 组件级 hero 定义
    @hero cardHero { over: 400, ease: power2Out }

    view {
        <div @hero="hero-demo-1"
             @hero-transition={scaleGlow}>
            Shared element
        </div>
    }
}
```

### 12.8 播放控制

`@motion` 定义中可用的播放控制属性：

| 属性 | 取值 | 说明 |
|------|------|------|
| `loop` | 无（无限）或数字 | 循环播放 |
| `direction` | `normal`, `reverse`, `alternate`, `alternateReverse` | 播放方向 |
| `fill` | `none`, `forwards`, `backwards`, `both` | 填充模式 |
| `delay` | 毫秒数 | 延迟播放 |
| `paused` | 布尔 | 暂停 |
| `repeatDelay` | 毫秒数 | 重复间隔 |
| `repeatRefresh` | 布尔 | 重复时刷新 |
| `timeScale` | 浮点数 | 播放速度 |
| `interrupt` | `stop`, `complete`, `queue`, `replace` | 中断行为 |
| `respectReducedMotion` | 布尔 | 尊重系统减少动画偏好 |
| `delayChildren` | 毫秒数 | 子元素延迟 |
| `staggerChildren` | 毫秒数 | 子元素交错时间 |
| `staggerFrom` | `start`, `center`, `end`, `edges` | 交错起始位置 |
| `when` | `beforeChildren`, `afterChildren`, `withChildren` | 与子元素的关系 |

### 12.9 缓动函数完整目录

**基础缓动**：
- `linear`
- `easeIn`, `easeOut`, `easeInOut`

**Power 曲线**：
- `power1In`, `power1Out`, `power1InOut`
- `power2In`, `power2Out`, `power2InOut`
- `power3In`, `power3Out`, `power3InOut`
- `power4In`, `power4Out`, `power4InOut`

**Bounce**：
- `bounceIn`, `bounceOut`, `bounceInOut`

**Back**（可选 `intensity` 参数）：
```omni
backIn(intensity: 1.7)
backOut(intensity: 1.7)
backInOut(intensity: 1.7)
```

**Elastic**（可选 `amplitude` 和 `period` 参数）：
```omni
elasticIn(amplitude: 1, period: 0.3)
elasticOut(amplitude: 1, period: 0.3)
elasticInOut(amplitude: 1, period: 0.3)
```

**Spring**：
```omni
spring(tension: 100, friction: 10, velocity: 0)
```

**Cubic Bezier**：
```omni
cubic(x1: 0.42, y1: 0, x2: 0.58, y2: 1)
```

**Steps**：
```omni
steps(count: 4, direction: end)
```

### 12.10 Stagger（交错动画）

```omni
@timeline staggerDemo {
    stagger(0.05)

    // 每个子元素延迟 50ms 依次播放
    item {
        from { opacity: 0, y: 20 }
        to   { opacity: 1, y: 0 }
        over 400 ease power2Out
    }
}
```

完整 stagger 语法：
```omni
stagger(motionName, each: 50, from: start)    // 从第一个元素开始
stagger(motionName, each: 50, from: center)   // 从中心开始
stagger(motionName, each: 50, from: end)      // 从最后一个开始
stagger(motionName, each: 50, from: edges)    // 从两端向中心
```

也可以用 `amount` 代替 `each`（总时间分配）：
```omni
stagger(motionName, amount: 300, from: start)
```

### 12.11 回调

```omni
@motion withCallbacks {
    from { opacity: 0 }
    to   { opacity: 1 }
    over 400

    onStart { console_log("animation started"); }
    onUpdate { println("progress: " + progress.to_string()); }
    onComplete { console_log("animation done"); }
    onRepeat { console_log("repeating"); }
    onInterrupt { console_log("interrupted"); }
}
```

---

## 13. 标准库与内置函数

### 13.1 控制台输出

```omni
import { console_log, console_warn, console_error } from "std/console";

console_log("info message");
console_warn("warning message");
console_error("error message");
```

内置函数（无需导入）：
```omni
println("output with newline");
print("output without newline");
```

### 13.2 HTTP 请求

```omni
import { http_get } from "std/http";

let response = await http_get("https://api.example.com/data");
```

### 13.3 JSON

```omni
import { json_parse, json_stringify } from "std/json";

let obj = json_parse(raw_string);
let serialized = json_stringify(data);
```

### 13.4 本地存储

```omni
import { storage_get, storage_set } from "std/storage";

storage_set("key", "value");
let value = storage_get("key");
```

### 13.5 数组操作

```omni
import { array_push, array_pop, array_length, array_is_empty } from "std/array";

let arr = ["a", "b"];
arr = array_push(arr, "c");    // → ["a", "b", "c"]
arr = array_pop(arr);          // → ["a", "b"]
let len = array_length(arr);   // → 2
let empty = array_is_empty(arr); // → false
```

### 13.6 异步工具

```omni
import { async_sleep } from "std/async";

await async_sleep(1000);  // 等待 1 秒
```

### 13.7 内置方法

```omni
// 字符串
"hello".len()           // → 5
"hello".to_upper()      // → "HELLO"
(42).to_string()         // → "42"
```

---

## 14. 附录 A：完整关键词与运算符

### 14.1 编译器识别的所有关键词（64 个）

**OmniLang 核心关键词（48 个）**：

```
fn, const, let, mut, if, else, while, for, in,
return, break, continue, print, println, async, await,
as, throw, try, catch, finally, match, struct, class,
interface, impl, extends, implements, type, nominal, static,
private, pub, override, import, from, export, true, false, null
```

**OmniView 关键词（16 个）**：

```
emit, component, signal, computed, prop, event, ref,
store, view, style, action, getter, theme, bind
```

**`@` 前缀指令词（21 个）**：

```
@store, @if, @else, @for, @empty, @slot, @native,
@fallback, @error, @mount, @unmount, @effect
```

**`@` 前缀动画触发器（25 个）**：

```
@motion, @timeline, @enter, @leave, @hover, @unhover,
@press, @focus, @animate, @in-view, @scroll,
@drag-horizontal (或 @dragHorizontal),
@drag-vertical (或 @dragVertical),
@driven-by, @paused, @hover-reverse,
@in-view-once, @in-view-repeat, @in-view-threshold,
@scroll-start, @scroll-end, @scroll-scrub,
@drag-constraints, @drag-elastic, @drag-scrub,
@drag-momentum, @drag-threshold, @drag-velocity-max,
@hero, @hero-transition, @if-mode
```

### 14.2 运算符（36 个）

| 运算符 | 含义 |
|--------|------|
| `.` | 字段/方法访问 |
| `?.` | 可选链 |
| `??` | 空值合并 |
| `+`, `-`, `*`, `/`, `%` | 算术运算 |
| `+=`, `-=`, `*=`, `/=`, `%=` | 复合赋值 |
| `=`, `==`, `!=`, `<`, `>`, `<=`, `>=` | 赋值与比较 |
| `&&`, `||`, `!` | 逻辑运算 |
| `->` | 返回类型箭头 |
| `=>` | 匹配臂箭头 |
| `...` | 展开 |
| `\|` | 联合类型分隔符 |
| `$` | 预留 |
| `#` | 预留 |
| `:`, `;`, `,` | 分隔符 |
| `(`, `)`, `{`, `}`, `[`, `]` | 分组 |

### 14.3 字面量

```omni
// 整数
42
0xFF       // 十六进制
0o77       // 八进制
0b1010     // 二进制

// 浮点数
3.14
1.5e10     // 科学计数法

// 字符串
"double-quoted"
'single-quoted'
`template string`

// 布尔
true
false

// 空值
null
```

### 14.4 字符串转义序列

```
\"   \\   \n   \t   \r   \u{XXXX}
```

---

## 15. 附录 B：内置组件目录

OmniView 编辑器（Monaco 补全）中注册的内置组件标签：

### 布局

| 组件 | 说明 |
|------|------|
| `Container` | 通用容器 |
| `Flex` | Flex 布局容器 |
| `Grid` | CSS Grid 布局 |
| `Stack` | 堆叠布局 |
| `Spacer` | 弹性间距 |

### 内容

| 组件 | 说明 |
|------|------|
| `Text` | 文本段落 |
| `Heading` | 标题 |
| `Image` | 图片 |
| `Link` | 超链接 |
| `Divider` | 分割线 |

### 表单

| 组件 | 说明 |
|------|------|
| `Button` | 按钮 |
| `Input` | 文本输入框 |
| `TextArea` | 多行文本域 |
| `Checkbox` | 复选框 |
| `Select` | 下拉选择 |
| `Switch` | 开关 |
| `Slider` | 滑块 |
| `Radio` | 单选按钮 |
| `RadioGroup` | 单选按钮组 |

### 数据展示

| 组件 | 说明 |
|------|------|
| `List` | 列表容器 |
| `ListItem` | 列表项 |
| `Table` | 表格 |
| `TableRow` | 表格行 |
| `TableCell` | 表格单元格 |
| `Card` | 卡片 |

### 反馈

| 组件 | 说明 |
|------|------|
| `Spinner` | 加载旋转器 |
| `Progress` | 进度条 |
| `Alert` | 提示信息 |
| `Badge` | 徽标 |
| `Chip` | 标签 |

### 导航与覆盖

| 组件 | 说明 |
|------|------|
| `Tabs` | 标签页 |
| `TabPanel` | 标签面板 |
| `Modal` | 模态对话框 |
| `Drawer` | 抽屉面板 |
| `Tooltip` | 工具提示 |

### 媒体

| 组件 | 说明 |
|------|------|
| `Icon` | 图标 |
| `Avatar` | 头像 |

> **注意**：内置组件是编辑器层面的补全建议，OmniView 编译器在 view 块中将 PascalCase 标识符统一处理为组件调用。使用内置组件无需额外导入。

---

## 参考来源

本文档基于以下权威来源编写：

1. **Rust 编译器源码** (`omni-compiler/src/lexer/`, `parser/`, `semantic/`, `codegen/omniview.rs`)
2. **learn-omniview 教学项目** (`test/projects/learn-omniview/src/`) —— 84 个可编译运行的 .omni 源文件
3. **其他 OmniView 项目** (`test/projects/view-ecommerce/`, `view-todo/`, `mixed-blog/`, `skill-demo/`)
4. **跨框架行为测试** (`tests/frameworks/behavior/`)
5. **OmniView 运行时** (`editor/runtime/src/omniview-runtime.ts`)
