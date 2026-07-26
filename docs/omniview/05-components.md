# 05 — 组件（Component）

组件是 OmniView UI 的基本构建块。每个 `component` 封装状态、计算属性、函数、视图和样式。

## 基本结构

```omni
component ComponentName {
    // 成员（顺序任意）
    prop ...         // 外部属性
    signal ...       // 响应式状态
    computed ...     // 派生计算值
    event ...        // 自定义事件
    fn ...           // 组件函数
    ref ...          // DOM 引用
    const ...        // 编译时常量

    @store ...       // Store 引用
    @mount { ... }   // 生命周期
    @unmount { ... }
    @effect(...) { ... }

    view { ... }     // UI 模板
    style { ... }    // 组件样式
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
<Greeting name="Alice" count={5} />
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
fn increment() { count = count + 1; }
fn reset() { count = 0; name = ""; }
fn addItem(item: String) { items = [...items, item]; }
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
    event onClose: bool?;

    fn trigger() {
        onSelect("item-1");         // 两种等价写法
        emit onSelect("item-1");    // emit 关键字更明确
    }
}

// 父组件接收
<Child @onSelect={handleSelect} />
```

父组件的 `handleSelect` 直接接收事件 payload 作为参数。

## fn（组件函数）

```omni
fn formatDate(ts: i32) -> String { return ts.to_string(); }
fn toggle() { isOpen = !isOpen; }
fn handleAdd() {
    if input != "" {
        items = [...items, input];
        input = "";
    }
}
```

- 可以修改 signal 值
- 可以调用 store action
- 在 view 中被事件处理器引用：`@onPress={handleAdd}`

## ref（DOM 元素引用）

```omni
component MyComponent {
    ref myInput: Element;
    ref scrollContainer: Element;

    fn focus() {
        // 通过 ref 名访问 DOM 元素
    }

    view {
        <input ref: myInput bind:value={text} />
        <div ref: scrollContainer>...</div>
    }
}
```

## const（编译时常量）

```omni
const MAX_ITEMS = 100;
const API_URL = "https://api.example.com";

signal limit: i32 = MAX_ITEMS;
```

常量值编译时确定，可用于 signal 初始值。

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
}
```

### @effect

追踪指定 signal 变化时自动执行：

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

- effect 在 DOM 更新后异步执行
- 追踪 Store signal 需用 `StoreName.field` 语法
- 避免在 effect 中产生无限循环

## @store 引用

在组件中引用全局 store：

```omni
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

## view 和 style

- `view { ... }` — 组件 UI 模板，详见 [06-模板语法](06-templates.md)
- `style { ... }` — 组件作用域样式，详见 [09-样式系统](09-styling.md)
