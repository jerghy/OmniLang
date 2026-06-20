---
name: omniview-coding-standards
description: OmniView 编码规范——当编写、审查、生成 .omni 视图代码时使用。覆盖组件/Store/信号/数据流/模板/样式/事件/生命周期、多文件项目结构、命名约定、AI 生成验证检查清单、反模式。
---

# OmniView 编码规范

> 面向 AI 和开发者的 OmniLang/OmniView 编码标准。生成或审查 `.omni` 代码时必须遵守。

---

## 一、项目结构与多文件规范

### 1.1 多文件项目布局（强制）

> ✅ **跨文件导入已完全支持。** 使用 `import { X } from "./path.omni"` 在文件间共享组件、Store、函数和结构体。每个 `.omni` 文件可被 `omnic build` 自动发现并编译。

```
my-app/
├── .omnilang.toml              # 项目配置（必须）
├── src/
│   ├── main.omni               # 入口文件 —— 只放 App 根组件 + import 语句
│   ├── lib.omni                # （可选）纯逻辑工具函数
│   ├── components/             # 所有 UI 组件
│   │   ├── Header.omni         # 一个文件 = 一个组件（文件名 = 组件名）
│   │   ├── TodoList.omni
│   │   └── TodoItem.omni
│   └── stores/                 # 所有 Store
│       ├── TodoStore.omni       # 一个文件 = 一个 Store（文件名 = Store 名）
│       └── AuthStore.omni
└── build/                      # 编译输出（自动生成，不提交 Git）
```

### 1.2 文件规则（强制）

| 规则 | 说明 |
|------|------|
| **一个文件一个组件** | 每个 `.omni` 文件只导出一个 `component` |
| **一个文件一个 Store** | 每个 `.omni` 文件只导出一个 `store` |
| **文件名 = 导出名** | `TodoItem.omni` → `component TodoItem`，`TodoStore.omni` → `store TodoStore` |
| **main.omni 只放 App** | 入口文件只包含 `import` 语句和 `component App`，不写逻辑函数 |
| **工具函数放 lib.omni** | `pub fn helper() { ... }` 放在 `src/lib.omni`，其他文件 `import { helper } from "../lib"` |
| **view 文件第一行** | 每个含 component/store 的文件第一行必须是 `import omniview;` |

### 1.3 `import omniview;` 规则

```omni
// ✅ 正确：文件第一行
import omniview;

// ❌ 错误：不在第一行
// 这是一些注释
import omniview;

// ❌ 错误：纯逻辑文件不能写这个（会被标记为 view 文件）
fn add(a: i32, b: i32) -> i32 { return a + b; }
import omniview;  // 编译错误
```

- 视图文件（含 component/store/theme）**必须**写 `import omniview;`
- 纯逻辑文件（`lib.omni`）**不写** `import omniview;`
- 视图文件不能被纯逻辑文件 import
- 组件和 Store **自动公开**（无需 `pub` 关键字）
- 函数和结构体需要 `pub` 才能跨文件使用

### 1.4 跨文件导入语法

```omni
// 导入组件（推荐：相对路径，不带 .omni 后缀）
import { TodoItem } from "./components/TodoItem";
import { Header } from "./components/Header";

// 导入 Store
import { TodoStore } from "./stores/TodoStore";

// 导入函数和结构体（需要 pub 标记）
import { format_count } from "./lib";

// 多名称导入
import { TodoItem, TodoList } from "./components/TodoList";

// 命名空间导入
import "./components/Sidebar" as Sidebar;

// main.omni 示例
import omniview;
import { TodoList } from "./components/TodoList";
import { Header } from "./components/Header";
import { TodoStore } from "./stores/TodoStore";

component App {
    @store TodoStore;
    view {
        <div>
            <Header />
            <TodoList />
        </div>
    }
}
```

### 1.5 单文件声明顺序（当所有内容在同一文件时）

仅在跨文件导入不可用或项目极小时使用单文件。按以下顺序从上到下声明：

```
import omniview;                  ← 第 1 行

pub struct Xxx { ... }           ← 数据结构（如有）
pub fn helper() -> Type { ... }  ← 工具函数（如有）

store XxxStore { ... }           ← Store（可多个）

component LeafA { ... }          ← 叶子组件（被引用的在前）
component LeafB { ... }

component Container { ... }      ← 容器组件

component App { ... }            ← 根组件（最后一个）
```

### 1.6 export 重导出

将其他文件的符号统一导出：

```omni
// src/components.omni —— 统一入口
export { Header } from "./components/Header";
export { TodoList } from "./components/TodoList";
export { TodoItem } from "./components/TodoItem";
```

---

## 二、组件（Component）规范

### 2.1 命名

- 组件名：**PascalCase**（`TodoList`, `ProductCard`, `UserAvatar`）
- 文件名与组件名一致

### 2.2 组件文件格式（强制顺序）

每个组件文件从上到下，严格按此顺序组织：

```
import omniview;                    ← 第 1 行
import { ... } from "...";         ← 导入（按需）

component Xxx {                    ← 组件声明
    // 1. 属性
    prop title: String;
    prop count: i32 = 0;

    // 2. 状态
    signal items: Array<String> = [];
    signal loading: bool = false;

    // 3. 计算属性
    computed total: i32 = items.len();

    // 4. DOM 引用
    ref inputEl: HtmlElement;

    // 5. 自定义事件
    event onSubmit: String;

    // 6. Store 引用
    @store AppStore;

    // 7. 生命周期
    @mount { ... }
    @unmount { ... }
    @effect(signal1) { ... }

    // 8. 组件函数
    fn handleClick() { ... }

    // 9. 模板
    view { ... }

    // 10. 样式
    style { ... }
}
```

### 2.3 完整多文件示例

**src/stores/TodoStore.omni：**
```omni
import omniview;

store TodoStore {
    signal items: Array<String> = [];
    signal filter: String = "all";
    getter activeCount: i32 = items.len();
    action add(text: String) { if text != "" { items.push(text); } }
    action remove(index: i32) { items.pop(); }
    action clearAll() { items = []; }
}
```

**src/components/TodoItem.omni：**
```omni
import omniview;

component TodoItem {
    prop text: String;
    prop index: i32 = 0;
    event onDelete: i32;

    fn handleDelete() { onDelete(index); }

    view {
        <div class: item>
            <span class: text>{text}</span>
            <button class: delete-btn @onPress={handleDelete}>✕</button>
        </div>
    }

    style {
        .item { display: flex; justify-content: space-between; padding: 8px; }
        .text { font-size: 16px; }
        .delete-btn { background: none; border: none; cursor: pointer; color: #e74c3c; }
    }
}
```

**src/components/TodoList.omni：**
```omni
import omniview;
import { TodoItem } from "./TodoItem";
import { TodoStore } from "../stores/TodoStore";

component TodoList {
    @store TodoStore;
    signal input: String = "";

    fn handleAdd() { TodoStore.add(input); input = ""; }
    fn handleDelete(index: i32) { TodoStore.remove(index); }

    view {
        <div class: container>
            <h2>待办列表（{TodoStore.activeCount}）</h2>
            <div class: input-row>
                <input bind:value={input} placeholder="添加新任务..." />
                <button @onPress={handleAdd}>添加</button>
            </div>
            <div class: list>
                @for item in TodoStore.items {
                    <TodoItem text={item} @onDelete={handleDelete} />
                }
                @empty { <p class: empty>暂无任务，添加一个吧！</p> }
            </div>
        </div>
    }

    style {
        .container { max-width: 600px; margin: 0 auto; padding: 24px; }
        .input-row { display: flex; gap: 8px; margin-bottom: 16px; }
        .input-row input { flex: 1; padding: 8px; border: 1px solid #ddd; border-radius: 4px; }
        .input-row button { padding: 8px 16px; background: #6366f1; color: white; border: none; border-radius: 4px; cursor: pointer; }
        .list { display: flex; flex-direction: column; gap: 4px; }
        .empty { color: #999; text-align: center; padding: 24px; }
    }
}
```

**src/lib.omni：**
```omni
// 纯逻辑文件 —— 不需要 import omniview;
pub fn format_count(n: i32) -> String {
    if n == 0 { return "零"; }
    return n.to_string();
}
```

**src/main.omni：**
```omni
import omniview;
import { TodoList } from "./components/TodoList";
import { TodoStore } from "./stores/TodoStore";

component App {
    @store TodoStore;
    view { <div><TodoList /></div> }
}
```

### 2.4 Component Props

- 必填 prop：`prop name: Type;`
- 可选 prop：`prop name: Type = default;`
- prop 类型**必须显式标注**，不推断
- 传递：`<Child propName={value} />`

### 2.5 组件拆分原则

| 场景 | 做法 |
|------|------|
| 组件 > 80 行 | 拆分 |
| 有可复用的 UI 片段 | 拆成独立组件 |
| 有独立交互状态 | 拆成独立组件 |
| 简单展示、1-2 个 prop | 不拆，保持内联 |

- 叶子组件（`TodoItem`）负责**自己的样式和行为**
- 容器组件（`TodoList`）负责**数据获取和状态管理**

---

## 三、信号（Signal）与计算属性（Computed）

### 3.1 命名约定

| 类型 | 命名规则 | 示例 |
|------|---------|------|
| 普通值 | camelCase | `count`, `userName` |
| 布尔 | `is/has/can/should` 前缀 | `isLoading`, `hasError` |
| 数组 | 复数名词 | `items`, `products` |
| 字符串 | 单数名词 | `input`, `title` |

### 3.2 初始值（必须有）

```omni
signal count: i32 = 0;                  // 数字
signal name: String = "";               // 字符串
signal items: Array<String> = [];       // 数组
signal isOpen: bool = false;            // 布尔
signal config: Map<String, i32> = {};   // 映射
```

**禁止无初始值：** `signal x: i32;` ← 编译错误

### 3.3 Computed 规范

- 命名：名词或形容词（`fullName`, `totalPrice`, `isValid`）
- 必须是**纯函数**，不能修改任何 signal
- 依赖自动追踪

```omni
// ✅ 正确
computed fullName: String = firstName + " " + lastName;
computed isPassed: bool = score >= 60;

// ❌ 错误——修改了 signal
computed bad: i32 = { count = count + 1; return count; };
```

### 3.4 修改规则

| 操作 | ✅/❌ |
|------|------|
| 在组件 `fn` 中修改 | ✅ |
| 在 Store `action` 中修改 | ✅ |
| 在 `computed` 中修改 | ❌ |
| 直接修改其他组件 signal | ❌（用 Store 或 event） |
| 在 `@effect` 中修改自身依赖 | ❌（无限循环） |

---

## 四、Store 规范

### 4.1 命名

- Store：PascalCase + `Store` 后缀（`CartStore`, `AuthStore`）
- 文件名与 Store 一致

### 4.2 内部结构顺序

```omni
store XxxStore {
    // 1. 状态
    signal items: Array<String> = [];
    signal filter: String = "all";

    // 2. 派生值
    getter count: i32 = items.len();

    // 3. 修改方法
    action add(item: String) { items.push(item); }
    action remove() { items.pop(); }
    action clear() { items = []; }
}
```

### 4.3 Action 命名

动词开头：`add`, `remove`, `update`, `set`, `toggle`, `clear`, `reset`

```omni
action addItem(text: String) { ... }     // 动词 + 名词
action removeItem(id: i32) { ... }
action toggleMenu() { ... }
```

### 4.4 @store 引用

- 放在组件体前部，紧跟 event 声明之后
- 语法：`@store StoreName;`
- 访问字段：`StoreName.field`（模板 `view` 中可用）
- 调用 action：`StoreName.action(args)`（注意：在组件 `fn` 中调用 Store action 有已知限制，建议通过事件触发）

### 4.5 多 Store 原则

- **一个领域一个 Store**——不做一个巨大的 `AppStore`
- 一个组件可以引用多个 Store

---

## 五、模板（View）规范

### 5.1 标签规范

| 类型 | 写法 | 示例 |
|------|------|------|
| HTML 元素 | 全小写 | `<div>`, `<button>`, `<input />` |
| 组件 | PascalCase | `<TodoItem />`, `<ProductCard />` |
| 自闭合 | 必须带 `/` | `<br />`, `<input />`, `<img />` |
| 配对标签 | 必须闭合 | `<div>...</div>` |

### 5.2 根元素

每个组件 `view` 必须有**唯一根元素**。

```omni
// ✅ 正确
view { <div><h1>A</h1><p>B</p></div> }

// ❌ 错误——多个顶层元素
view { <h1>A</h1><p>B</p> }
```

### 5.3 @if / @else if / @else

```omni
@if condition {
    <div>真</div>
} else if other {
    <div>另一条件</div>
} else {
    <div>默认</div>
}
```

- 必须有 `@else` 兜底
- 嵌套不超过 3 层

### 5.4 @for / @empty（强制配对）

```omni
@for item in items {
    <li>{item}</li>
}
@empty {
    <li>暂无数据</li>
}
```

- **每个 `@for` 必须配 `@empty`**
- 循环体内不能修改被遍历的数组
- 循环变量用单数形式（`@for item in items`）

### 5.5 事件绑定

- 格式：`@eventName={handler}`
- handler 是组件 `fn`：`@onPress={handleClick}`
- 或简单内联：`@onPress={() => { count = count + 1; }}`

### 5.6 属性绑定速查

```omni
<!-- 静态字符串 -->
<h1 class: title>Hello</h1>
<input placeholder="输入" />

<!-- 动态表达式 -->
<PostCard title={post.title} />

<!-- 双向绑定 -->
<input bind:value={input} />

<!-- 条件 class -->
<div class: if isActive { "active" } else { "inactive" }>
```

### 5.7 文本插值

```omni
<span>{count}</span>
<span>{count + 1}</span>
<span>{if cond { "是" } else { "否" }}</span>
```

---

## 六、样式规范

### 6.1 style 块

- 紧跟在 `view { }` 之后
- class 命名：**kebab-case** 语义化

```omni
style {
    .card-title { font-size: 20px; }      // ✅
    .submit-button { padding: 8px; }      // ✅
    .redButton { ... }                     // ❌ 不用 camelCase
    .btn__primary { ... }                  // ❌ 不用 BEM
}
```

### 6.2 内联样式

仅用于**动态值**：

```omni
<div style: {
    background-color: if error { "#e74c3c" } else { "#3498db" },
    padding: "16px",
}>
```

---

## 七、生命周期

### 7.1 @mount —— 挂载后执行一次

```omni
@mount {
    inputEl.focus();     // 聚焦输入框
}
```

用途：获取初始数据、初始化第三方库、聚焦元素

### 7.2 @unmount —— 卸载前执行

```omni
@unmount {
    clearInterval(timer);  // 清理定时器
}
```

### 7.3 @effect —— 依赖变化时执行

```omni
@effect(count) {
    println(count.to_string());  // 日志
}
```

- 在 DOM 更新**之后**触发
- **不能修改自身依赖的信号**

---

## 八、数据流

### 8.1 四种通信方式

| 方向 | 方式 | 语法 |
|------|------|------|
| 父→子 | prop | `<Child prop={value} />` |
| 子→父 | event | `<Child @onEvent={handler} />` |
| 跨组件 | Store | `@store XxxStore;` → `XxxStore.field` |
| 表单 | 双向绑定 | `<input bind:value={signal} />` |

### 8.2 原则

- **单一数据源**：每份数据只有一个 owner
- **最多两层 prop**：不跨层传递
- **Store 做共享**：需要跨组件共享的状态放 Store
- **Event 做通知**：子组件通知父组件用 event

---

## 九、AI 生成与验证

### 9.1 生成代码检查清单

- [ ] `import omniview;` 在第一行
- [ ] 所有标签闭合（自闭合带 `/`）
- [ ] 每个 `@for` 有对应的 `@empty`
- [ ] 组件有唯一根元素
- [ ] 每个 signal 有显式初始值
- [ ] `class:` 用冒号（不是 `class=`）
- [ ] 不使用 match 表达式（用 if/else 替代）
- [ ] 不使用泛型 `<T>`、带标签联合体 `|`、闭包 `|x|`
- [ ] 不使用标准库 import（`omni/math` 等）
- [ ] view 模板内不含 `<!-- -->` HTML 注释（用 view 外的 `//` 注释）
- [ ] 内联回调不含类型标注（`(text) =>` 而非 `(text: String) =>`）
- [ ] CSS hex 颜色避免数字开头含 `e` 的（如 `#92400e`，改用命名颜色）
- [ ] 代码是完整可编译的 `.omni` 文件

### 9.2 验证命令

```bash
omnic build file.omni --target omniview    # 验证 UI 编译
omnic run file.omni --target python        # 验证逻辑输出
omnic analyse                              # 静态分析
```

### 9.3 模板内限制

| 限制 | 说明 | 替代方案 |
|------|------|---------|
| HTML 注释 `<!-- -->` | view 模板内不支持 HTML 注释 | 用 `//` 注释放在 view 外 |
| 内联回调类型标注 | `@onAdd={(text: String) => {}}` 不支持 | 去掉类型：`@onAdd={(text) => {}}` |
| CSS `#` 数字+e | hex 颜色如 `#92400e`（数字开头含 `e`）会被误解析 | 用命名颜色或避免 `[0-9]+e` 模式 |
| 模板内 `...` | `...` 被解析为展开运算符 | 用其他字符替代 |

### 9.4 不支持语法

| 需求 | ❌ 不支持 | ✅ 替代 |
|------|----------|--------|
| 模式匹配 | `match x { ... }` | `if/else if/else` |
| 泛型 | `fn name<T>(...)` | 暂不支持 |
| 联合类型 | `type X = A \| B` | 暂不支持 |
| 闭包 | `\|x\| { ... }` | 定义 `fn` |
| 模板字符串 | `` `hello ${x}` `` | `"hello " + x` |
| 标准库 | `import { ... } from "omni/math"` | 暂不支持 |
| 空值合并 | `x ?? default` | 暂不支持 |

### 9.5 当前可用（已验证）

- 变量/类型/函数/控制流
- `println`/`print`
- 结构体
- 数组（`Array<T>`）：`len()`, `is_empty()`, `get(i)`, `push()`, `pop()`
- 映射（`Map<K,V>`）：`len()`, `contains()`, `get()` `insert()`
- 块作用域 `{ }`
- `if` 作为表达式：`let x = if cond { a } else { b };`
- `while`, `for-in` 循环
- `async fn` / `await`（Python + JS）
- OmniView 全功能（component/store/signal/computed/@if/@for/@empty/bind:value/@onPress/style/theme）

---

## 十、反模式

| # | ❌ 错误 | ✅ 正确 |
|---|---------|---------|
| 1 | `import omniview;` 不在第一行 | 放在文件第一行 |
| 2 | `<br>` 不闭合 | `<br />` |
| 3 | `@for` 没有 `@empty` | 每个 @for 配 @empty |
| 4 | 多个顶层元素 | 用一个根元素包裹 |
| 5 | `class="btn"` | `class: btn` |
| 6 | signal 无初始值 | `signal x: i32 = 0;` |
| 7 | computed 中修改 signal | computed 保持纯函数 |
| 8 | 直接修改其他组件 signal | 通过 Store action |
| 9 | @effect 中修改自身依赖 | effect 只做副作用 |
| 10 | Store 信号在 action 外改 | 所有修改放 action |
| 11 | 一个大杂烩 AppStore | 按领域拆分 Store |
| 12 | 使用 match 表达式 | 用 if/else |
| 13 | `|x| { ... }` 闭包 | 定义独立的 `fn` |
| 14 | @for 中修改遍历的数组 | 先收集再统一改 |
| 15 | prop 无类型标注 | `prop name: String;` |
