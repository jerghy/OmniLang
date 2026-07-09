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

声明对 DOM 元素的引用：

```omni
ref myInput: HtmlInputElement;
ref myDiv: HtmlDivElement;

view {
    <input ref: myInput bind:value={text} />
    <div ref: myDiv>...</div>
}
```

### const（常量）

组件级别的常量：

```omni
const MAX_ITEMS = 100;
const $API_URL = "https://api.example.com";
```

## 生命周期钩子

### @mount（挂载时）

组件首次挂载到 DOM 后执行：

```omni
component Timer {
    signal elapsed: i32 = 0;

    @mount {
        // 组件挂载后启动定时器
        // 可以在这里初始化第三方库
    }
}
```

### @unmount（卸载时）

组件从 DOM 移除前执行：

```omni
@unmount {
    // 清理资源：取消定时器、移除事件监听等
}
```

### @effect（响应式副作用）

当依赖的 signals 变化时自动执行：

```omni
signal searchQuery: String = "";
signal results: Array<String> = [];

@effect(searchQuery) {
    // 每当 searchQuery 改变时执行
    results = performSearch(searchQuery);
}
```

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
