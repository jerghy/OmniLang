# 02 — 组件（Component）

组件是 OmniView UI 的基本构建块。每个 `component` 封装自己的状态（signals）、计算属性（computed）、函数（fn）、视图（view）和样式（style）。

## 基本语法

```omni
component ComponentName {
    // 成员声明（顺序任意）
    // prop / signal / computed / event / fn / action / getter
    // view { ... }
    // style { ... }
}
```

## 完整组件示例

```omni
import omniview;

component Counter {
    // 1. 属性
    prop initial: i32 = 0;

    // 2. 响应式信号
    signal count: i32 = initial;
    signal step: i32 = 1;

    // 3. 计算属性（简单形式）
    computed doubled: i32 = count * 2;

    // 4. 计算属性（函数形式）
    computed label() -> String {
        if count > 10 {
            return "High";
        }
        return "Normal";
    }

    // 5. 事件声明
    event onCountChange: i32;

    // 6. 普通函数
    fn increment() {
        count = count + step;
        emit onCountChange(count);
    }

    // 7. action 函数
    action reset {
        count = 0;
    }

    // 8. 视图模板
    view {
        <div>
            <p>Count: {count}</p>
            <p>Doubled: {doubled}</p>
            <p>Label: {label()}</p>
            <button @onPress={increment}>+{step}</button>
            <button @onPress={reset}>Reset</button>
        </div>
    }

    // 9. 组件样式
    style {
        .container { max-width: 600px; margin: 0 auto; }
    }
}
```

## 组件成员详解

### prop（属性/参数）

组件接收的外部输入，由父组件传入：

```omni
component Greeting {
    prop name: String = "World";     // 带默认值
    prop age: i32;                    // 必需属性（无默认值）

    view {
        <p>Hello {name}, age {age}</p>
    }
}
```

props 是**不可变的**——组件内部不能修改 prop 的值。

### signal（响应式信号）

组件的响应式状态。当 signal 值改变时，所有依赖它的 UI 自动更新：

```omni
signal count: i32 = 0;
signal name: String = "Alice";
signal isActive: bool = false;
signal items: Array<String> = [];
signal user: User? = null;          // 可选类型
signal price: f64 = 0.0;
```

- 必须提供初始值
- 类型注解是必需的
- 支持所有基础类型（i32, f64, bool, String, Array, struct 等）

### computed（计算属性）

从 signals 或其他数据派生的只读值。当依赖变化时自动重新计算：

**简单形式**（单表达式）：

```omni
computed fullName: String = firstName + " " + lastName;
computed isLong: bool = items.len() > 10;
computed displayLabel: String = user ?? "Guest";
computed grade: String = score >= 90 ? "A" : score >= 80 ? "B" : "C";
```

**函数形式**（含多语句逻辑）：

```omni
computed fullName() -> String {
    if firstName == "" && lastName == "" {
        return "Anonymous";
    }
    if lastName == "" {
        return firstName;
    }
    return firstName + " " + lastName;
}
```

- computed 是**惰性**的（缓存脏标记机制）
- computed 必须是**纯**的（不应有副作用）
- 函数形式的 computed 在 view 中调用：`{fullName()}`

### getter（属性访问器）

类似于 computed，但语法更简洁：

```omni
getter itemCount: i32 = items.len();
getter isEmpty: bool = items.len() == 0;
```

### fn（组件函数）

组件内部的可复用逻辑函数：

```omni
fn formatDate(timestamp: i32) -> String {
    return timestamp.to_string();
}

fn toggle() {
    isOpen = !isOpen;
}
```

- 可以修改 signal 的值
- 可以调用 store 的 action
- 可以被事件处理器引用

### action（组件动作）

组件内部的可调用动作，类似于无返回值的函数：

```omni
action increment {
    count = count + 1;
}

action reset {
    count = 0;
    name = "";
}
```

action 也可以带参数和异步：

```omni
action loadData(url: String) async {
    loading = true;
    let raw = await http_get(url);
    data = raw;
    loading = false;
}
```

### event（事件声明）

声明组件可以向父组件发射的事件：

```omni
event onSelect: String;        // 携带 String 数据
event onClose: bool?;          // 可选事件（带 ?）
event onChange: i32;           // 携带 i32 数据
```

在组件内部触发事件：

```omni
// 两种等价方式
emit onSelect("item-1");      // emit 关键字
onSelect("item-1");            // 函数调用形式
```

父组件接收事件：

```omni
<Child @onSelect={handleSelect} />
```

### ref（元素引用）

声明对 DOM 元素的引用，可以在组件函数中访问：

```omni
component MyComponent {
    ref myInput: HtmlElement;
    ref scrollContainer: HtmlElement;

    fn focusInput() {
        // 通过 ref 名访问 DOM 元素
    }

    view {
        <input ref: myInput bind:value={text} />
        <div ref: scrollContainer class: scroll-area>...</div>
    }
}
```

**使用场景**：
- 滚动容器控制（`scrollContainer.scrollTop`）
- 输入框聚焦（`myInput.focus()`）
- 画布操作
- 第三方库 DOM 操作

**ref 类型**：`HtmlElement` 是所有 HTML 元素的基础引用类型。

### const（常量）

组件级别的编译时常量，在 `@` 前缀可选：

```omni
component MyComponent {
    const MAX_ITEMS = 100;
    const API_URL = "https://api.example.com";
    const logoUrl = "/assets/logo.png";       // 无 @ 前缀也可

    signal limit: i32 = MAX_ITEMS;             // 常量可用于初始值
}
```

**注意**：
- `const` 定义在组件内部（顶层 const 有限制）
- 常量值在编译时确定，不可在运行时修改
- 可用于 signal 初始值、数组大小等

### @store 引用（内联 Store）

除了引用全局 Store 文件，还可以在组件**内部**定义 inline store：

```omni
component InlineStoreDemo {
    // 内联 store — 仅在此组件内可见
    store CartStore {
        signal items: Array<String> = [];
        signal totalPrice: f64 = 0.0;

        getter itemCount: i32 = items.len();
        getter summary: String = if items.len() == 0 {
            "cart empty"
        } else {
            "total: " + items.len().to_string() + " items"
        };

        action addItem(name: String, price: f64) {
            items.push(name);
            totalPrice = totalPrice + price;
        }

        action clear() {
            items = [];
            totalPrice = 0.0;
        }
    }

    // 在 view 中直接使用 store 的 signal/getter
    // 通过 CartStore.actionName() 调用 action
    fn addApple() {
        CartStore.addItem("Apple", 5.0);
    }

    view {
        <div>
            <p>Items: {itemCount}</p>       <!-- getter 可直接访问 -->
            <p>Total: {totalPrice}</p>      <!-- signal 可直接访问 -->
            <button @onPress={addApple}>Add Apple</button>
            <button @onPress={CartStore.clear}>Clear</button>
        </div>
    }
}
```

内联 store 的 getter 和 signal **可以直接在 view 中访问**（不需要 `StoreName.` 前缀），action 仍需 `StoreName.actionName()`。

## 生命周期钩子

### @mount（挂载时）

组件首次挂载到 DOM 后执行。在事件绑定和 reactive effects 之前注册，但会在 DOM 插入后运行：

```omni
component TodoPage {
    @store TodoStore;
    signal pageLoadCount: i32 = 0;

    @mount {
        pageLoadCount = pageLoadCount + 1;
        TodoStore.fetchReminders();    // 挂载后加载数据
        console_log("Page mounted");
    }
}
```

**常见用途**：
- 初始化第三方库
- 从 storage 加载持久化数据
- 注册全局事件监听（需在 `@unmount` 中清理）
- 启动定时器

### @unmount（卸载时）

组件从 DOM 移除前执行。用于清理资源：

```omni
component StatsDashboard {
    signal counter: i32 = 0;

    @unmount {
        // 清理：取消定时器、移除事件监听等
        println("StatsDashboard unmounting, counter: " + counter.to_string());
    }
}
```

### @effect（响应式副作用）

当依赖的 signals 变化时自动执行。`@effect` 可以追踪组件 signals 或 store signals：

```omni
// 追踪单个 signal
@effect(searchQuery) {
    if searchQuery.len() >= 3 {
        results = performSearch(searchQuery);
    }
}

// 追踪 store signal — 使用 StoreName.field 语法
@effect(TodoStore.items) {
    console_log("Total items: " + TodoStore.totalCount.to_string());
}

// 追踪多个 signals（编译器自动分析依赖）
@effect(input) {
    if input.len() > 20 {
        println("输入较长：" + input.len().to_string() + " 字符");
    }
}
```

**注意事项**：
- `@effect` 在 DOM 更新后异步执行（microtask 队列）
- 追踪 Store signal 时必须使用 `StoreName.field` 语法
- effect 内部可以修改 signals，但要避免无限循环
- effect 回调不能是 async

## view 视图块

组件的 UI 模板，详见 [03-模板语法](03-templates.md)：

```omni
view {
    <div class: container>
        <h1>{title}</h1>
        <p>{content}</p>
        <button @onPress={handleClick}>Click</button>
    </div>
}
```

## style 样式块

组件作用域（scoped）的 CSS 样式，详见 [07-样式系统](07-styling.md)：

```omni
style {
    .container { max-width: 800px; padding: 16px; }
    .title { font-size: 24px; color: #333; }
    button { cursor: pointer; border-radius: 8px; }
}
```

## @store 引用

在组件中引用全局 store，详见 [05-Store 状态管理](05-stores.md)：

```omni
component TodoApp {
    @store TodoStore;       // 引用外部 store
    @store ThemeStore;
}
```
