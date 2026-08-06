# 05 — 组件（Component）

组件是 OmniView UI 的基本构建块。每个 `component` 封装状态、计算属性、函数、视图和样式。

## 基本结构

```omni
store TodoStore {
    signal items: Array<String> = [];
}

component ComponentName {
    // 成员（顺序任意）
    prop title: String = "Default";          // 外部属性
    signal count: i32 = 0;                   // 响应式状态
    computed double: i32 = count * 2;        // 派生计算值
    event saved: String;                     // 自定义事件
    fn increment() { count = count + 1; }    // 组件函数
    ref inputBox: Element;                   // DOM 元素引用
    const MAX_ITEMS = 100;                   // 编译时常量

    @store TodoStore;                        // Store 引用
    @mount { print("mounted"); }             // 生命周期：挂载
    @unmount { print("unmounted"); }         // 生命周期：卸载
    @effect(count) { print(count.to_string()); } // 响应式副作用

    view {
        // UI 模板
    }
    style {
        // 组件样式
    }
}
```

## prop（属性）

组件接收的外部输入，由父组件传入。prop 在组件内部**不可修改**：

```omni
component Greeting {
    prop name: String = "World";     // 带默认值
    prop count: i32;                  // 必需（无默认值）

    view {
        <p>Hello {name} × {count}</p>
    }
}

// 父组件使用
component App {
    view {
        <Greeting name="Alice" count={5} />
    }
}
```

## signal（响应式信号）

当 signal 值改变时，所有依赖它的 UI 和 computed 自动更新：

```omni
signal count: i32 = 0;
signal name: String = "Alice";
signal isActive: bool = false;
signal items: Array<String> = [];
signal user: User? = null;
```

**读取**——在 view 和 computed 中直接使用名称：

```omni
view {
    <p>Count: {count}</p>
    <p>Uppercase: {name.to_uppercase()}</p>
}
```

**更新**——使用赋值语法：

```omni
component Counter {
    signal count: i32 = 0;
    signal name: String = "Alice";
    signal items: Array<String> = [];

    fn increment() { count = count + 1; }
    fn reset() { count = 0; name = ""; }
    fn addItem(item: String) { items = [...items, item]; }

    view {
        <p>Count: {count}</p>
    }
}
```

Array 的 `.push()` 和 `.pop()` 自动触发响应式更新。推荐对完整替换使用不可变模式（`[...old, new]`）。

## computed（计算属性）

从 signals 派生的只读值，依赖变化时自动重新计算：

**简单形式**（单表达式）：

```omni
computed fullName: String = firstName + " " + lastName;
computed isLong: bool = items.len() > 10;
computed grade: String = score >= 90 ? "A" : score >= 80 ? "B" : "C";
```

**函数形式**（多语句，在 view 中需 `()` 调用）：

```omni
computed label() -> String {
    if count > 100 { return "Overflow"; }
    if count < 0 { return "Invalid"; }
    return "Normal";
}

view {
    <p>Status: {label()}</p>   // 函数形式需 ()
}
```

- computed 是惰性的——只有依赖变化时才重算
- computed 必须是纯的——不应有副作用

> 在 `store` 中，使用 `getter` 代替 `computed`。详见 [08-Store 状态管理](08-stores.md)。

## event（自定义事件）

声明组件可以向父组件发射的事件：

```omni
component Child {
    event onSelect: String;
    event onClose: bool?;       // 可选事件（父组件可忽略）

    fn trigger() {
        onSelect("item-1");         // 两种等价写法
        emit onSelect("item-1");    // emit 关键字更明确
    }
}

// 父组件接收
component Parent {
    signal closed: bool = false;

    fn handleSelect(value: String) { print(value); }

    view {
        <Child @onSelect={handleSelect} />
        <Child @onClose={() => { closed = true; }} />
    }
}
```

父组件的 `handleSelect` 直接接收事件 payload 作为参数。

## fn（组件函数）

```omni
component TodoInput {
    signal isOpen: bool = false;
    signal input: String = "";
    signal items: Array<String> = [];

    fn formatDate(ts: i32) -> String { return ts.to_string(); }
    fn toggle() { isOpen = !isOpen; }
    fn handleAdd() {
        if input != "" {
            items = [...items, input];
            input = "";
        }
    }

    view {
        <input bind:value={input} />
    }
}
```

- 可以修改 signal 值
- 可以调用 store action
- 在 view 中被事件处理器引用：`@onPress={handleAdd}`

## ref（DOM 元素引用）

`ref` 声明一个对 DOM 元素的引用，用于直接的、高频率的 DOM 操作（ADR-036）。

### 声明与绑定

```omni
component MyComponent {
    ref myInput: Element;          // ref 声明
    ref scrollContainer: Element;  // 多个 ref

    view {
        <!-- 方式 A：冒号简写（原语） -->
        <input ref: myInput bind:value={text} />

        <!-- 方式 B：@ref 事件风格（推荐，与 @onClick 一致性高）-->
        <div @ref={scrollContainer}>...</div>
    }
}
```

两种绑定语法等价，推荐 `@ref={name}`。

### Ref API 表面（跨后端安全子集，语义级 DSL，ADR-052/053）

| API | 用途 | 示例 |
|-----|------|------|
| `.focus()` | 聚焦元素 | `myInput.focus()` |
| `.blur()` | 失焦 | `myInput.blur()` |
| `.scrollTo(x, y)` | 滚动到指定位置 | `scrollArea.scrollTo(0, 0)` |
| `.scrollBy(dx, dy)` | 增量滚动 | `scrollArea.scrollBy(0, 100)` |
| `.select()` | 全选文本 | `searchBox.select()` |
| `.setSelection(start, end)` | 选区 | `searchBox.setSelection(0, 3)` |
| `.measure()` | 位置/尺寸查询 | `let rect = searchBox.measure()` |
| `.capturePointer()` | 捕获全部指针 | `el.capturePointer()` |
| `.releasePointer()` | 释放全部指针 | `el.releasePointer()` |

**不暴露**：`innerHTML`、`addEventListener`、`querySelector`、`parentElement`——这些会破坏跨后端抽象。

> 已删除（ADR-052/053）：`.style`（改用 `style: {...}` 绑定 signal）与
> `.getBoundingClientRect()` / `.offsetWidth` / `.offsetHeight`（改用
> `.measure()`，编译器报错并提示替代方案）。高频语义操作优先使用语义绑定
> 属性：`focus={sig}`、`scroll-x={sig}`、`scroll-y={sig}`；表单值双向同步
> 用 `bind:value`（带 `@ref` 的 `value={sig}` v1 双向形状已删除——值绑定
> 收敛 D2；无 `@ref` 的 `value={sig}` 是普通单向 prop，v4 5.4 恢复）。
> （需配合 `@ref={name}`）。

### 高频场景：ref + @drag-* 触发器配合

`ref.style.*` 已按 ADR-052 删除（DOM 专属 API，用信号 + style 绑定替代）。拖拽驱动的 60fps 位移由 `@drag-*` 触发器 + `@motion` 关键帧实现，最终值用 signal 保存。`@drag-*` 支持 `constraints:` / `elastic:` / `threshold:` 控制维度（v2 BNF；`velocity_max` 已移除——零消费者，见 deferral #84）：

```omni
@motion dragSlideX {
    from { translateX: 0 }
    to   { translateX: 100 }
    over: 300ms
}

component Draggable {
    signal x: f64 = 0;

    fn onDragUpdate(e) {
        x = e.x;   // 最终值用 signal 保存
    }

    view {
        <div @ref={dragBox} @drag-horizontal={dragSlideX} @onPress={onDragUpdate}>
            <p>Drag me — x:{x.to_string()}</p>
        </div>
    }
}
```

`@drag-horizontal` / `@drag-vertical` 触发器（kebab-case）驱动 motion 关键帧进度。

### 边界情况

- **`@for` + `@ref`**：在 `@for` 循环内使用 `@ref` 会绑定到循环的最后一个元素（Phase 2 解决语义）
- **自定义组件 ref**：`@ref` 在 `<MyComponent>` 上尚不支持（需要 React `forwardRef` 等价物）

## const（编译时常量）

```omni
const MAX_ITEMS = 100;
const API_URL = "https://api.example.com";

component Limits {
    signal limit: i32 = MAX_ITEMS;

    view {
        <p>Limit: {limit}</p>
    }
}
```

常量值编译时确定，可用于 signal 初始值和列表渲染。

## 生命周期钩子

### @mount

组件挂载到 DOM 后执行：

```omni
@mount {
    pageLoadCount = pageLoadCount + 1;
    ThemeStore.load();
}
```

常见用途：初始化数据、恢复持久化状态、注册全局监听。

### @unmount

组件从 DOM 移除前执行，用于清理资源：

```omni
@unmount {
    println("Component unmounting");
    // 清理定时器、取消订阅等
}
```

### @effect

追踪 signal 变化时自动执行副作用。有两种形式：

**追踪指定 signal**：

```omni
@effect(searchQuery) {
    if searchQuery.len() >= 3 {
        results = performSearch(searchQuery);
    }
}

// 追踪 store signal
@effect(TodoStore.items) {
    console_log("Total: " + TodoStore.totalCount.to_string());
}
```

**无参数**（每次组件更新时执行）：

```omni
@effect {
    effectCount = effectCount + 1;  // 每次更新时递增
}
```

要点：
- effect 在 DOM 更新后异步执行
- 追踪 Store signal 需用 `StoreName.field` 语法
- 避免在 effect 中产生无限循环

## @store 引用

在组件中引用全局 store：

```omni
store TodoStore {
    signal items: Array<String> = [];

    getter totalCount: i32 = items.len();
    action clear() { items = []; }
}

component TodoApp {
    @store TodoStore;

    view {
        <p>Total: {TodoStore.totalCount}</p>
        <button @onPress={TodoStore.clear}>Clear</button>
    }
}
```

详见 [08-Store 状态管理](08-stores.md)。

## 内联 Store

在组件内部定义局部 store，仅当前组件可见：

```omni
component CartDemo {
    store CartStore {
        signal items: Array<String> = [];
        getter count: i32 = items.len();

        action add(item: String) { items.push(item); }
    }

    view {
        <p>Items: {count}</p>                     // getter 可直接访问
        <button @onPress={CartStore.add("Apple")}> // action 需 StoreName 前缀
            Add
        </button>
    }
}
```

## @for key={expr} 身份追踪

```omni
@for item in items key={item.id} {
    <div class: list-item>
        <span>{item.name}</span>
    </div>
} @empty {
    <p>No items yet</p>
}
```

`key={...}` 提供稳定的 DOM 标识，支持列表项的增量更新和 FLIP 动画。

## view 和 style

- `view { ... }` — 组件 UI 模板，详见 [06-模板语法](06-templates.md)
- `style { ... }` — 组件作用域样式，详见 [09-样式系统](09-styling.md)
