# Omniview 状态管理 — 完整语言参考

> 基于 learn-omniview 教学项目（68+ .omni 源文件）和 omni-compiler 源码分析。
>
> 补充教程：[响应式与状态](tutorial/08-reactivity-and-state.md) · [Store 与 Action](tutorial/09-stores-and-actions.md)

---

## 总览

Omniview 的状态管理是 **Signal → Computed → View** 的纯响应式数据流：

```
用户交互 (点击/输入)
    │
    ▼
@onPress={fn} / bind:value={signal}
    │
    ▼
signal.set()           ◄── 唯一状态变更点（调试在此拦截）
    │
    ├─→ computed 自动标记 dirty → 下次读取时重算
    ├─→ view 中 {signal} 自动更新 DOM
    └─→ @effect(deps) 重新执行
```

---

## 一、`signal` — 响应式状态变量

### 基本声明

```omniview
component X {
    signal count: i32 = 0;
    signal name: String = "hello";
    signal isReady: bool = false;
    signal items: Array<String> = [];
    signal scores: Array<i32> = [95, 87, 72];
    signal config: Map = { "theme": "dark", "lang": "zh" };
}
```

### 完整类型表

| 类型 | 初始值示例 | 说明 |
|------|-----------|------|
| `i32` | `0` `42` `-5` | 32 位整数 |
| `f64` | `3.14` `5999.0` | 浮点数 |
| `bool` | `true` `false` | 布尔值 |
| `String` | `""` `"hello"` | 字符串 |
| `Array<T>` | `[]` `[1, 2, 3]` | 泛型数组 |
| `Map` | `{}` `{"k": "v"}` | 字典 |
| 自定义 struct | `RowItem { id: 1, name: "A" }` | 结构体 |

### 在 view 中读取

```omniview
view {
    <div>
        <p>{count}</p>
        <p>{count * 2}</p>
        <p>{if isReady { "就绪" } else { "等待" }}</p>
        <p>{items.len()}</p>
        <p>{math_abs(counter)}</p>
        <p class: { badge: true, active: status == "Active" }>{status}</p>
    </div>
}
```

### 在 view 中修改

```omniview
view {
    <div>
        <!-- 直接赋值 -->
        <button @onPress={() => { step = 1; }}>步长1</button>

        <!-- 调用 fn -->
        <button @onPress={inc}>+</button>

        <!-- 条件修改 -->
        <button @onPress={() => {
            if sortAsc { sortAsc = false; } else { sortAsc = true; }
        }}>切换排序</button>

        <!-- bind:value 双向绑定 -->
        <input bind:value={searchText} />
    </div>
}
```

### 在 fn 中修改

```omniview
fn inc()    { count = count + step; }
fn reset()  { count = 0; }
fn addFive(){ count += 5; }
fn double() { count *= 2; }
fn modThree() {
    let mut x = counter;
    x %= 3;
    counter = x;
}
```

### 增广赋值操作符

| 操作符 | 示例 | 等价于 |
|--------|------|--------|
| `+=` | `count += 5` | `count = count + 5` |
| `-=` | `count -= 3` | `count = count - 3` |
| `*=` | `count *= 2` | `count = count * 2` |
| `/=` | `count /= 2` | `count = count / 2` |
| `%=` | `count %= 3` | `count = count % 3` |

---

## 二、`computed` — 派生计算

### 形式 A：简单表达式（一行）

```omniview
signal count: i32 = 0;
signal firstName: String = "Zhang";
signal lastName: String = "";
signal sortAsc: bool = true;

computed doubled: i32 = count * 2;
computed label: String = if count % 2 == 0 { "偶数" } else { "奇数" };
computed fullName: String = firstName + " " + lastName;
computed sortLabel: String = if sortAsc { "ASC" } else { "DESC" };
```

### 形式 B：函数式（多行 block）

```omniview
computed fullName() -> String {
    if firstName == "" && lastName == "" {
        return "Anonymous";
    }
    if lastName == "" {
        return firstName;
    }
    return firstName + " " + lastName;
}

computed filteredFrameworks: Array<String> =
    array_filter(demoFrameworks, |f|
        if searchText == "" { true } else { f == searchText }
    );
```

### 关键特性

| 特性 | `computed` | `fn` |
|------|-----------|------|
| 缓存 | ✅ 依赖不变则返回缓存值 | ❌ 每次调用重算 |
| 自动依赖追踪 | ✅ 体内读取的 signal 自动成为依赖 | ❌ 无 |
| 惰性求值 | ✅ 只在被读取时计算 | ❌ 调用即执行 |
| 适用场景 | 派生值渲染 | 复杂逻辑、带副作用的操作 |

### 在 view 中使用

```omniview
// 简单形式：直接用名
<p>{doubled}</p>
<p>{label}</p>

// 函数式形式：带 () 调用
<p>{fullName()}</p>
```

---

## 三、`store` — 命名空间状态集合

Store 将相关的 signal、getter、action 聚合为一个可全局引用的独立模块。

### 全局 Store（独立文件）

```omniview
// stores/TodoStore.omni
store TodoStore {
    // ── 信号 ──
    signal items: Array<String> = [];
    signal filter: String = "all";
    signal lastAction: String = "";

    // ── getter（派生值，等价于 computed）──
    getter totalCount: i32 = items.len();
    getter hasItems: bool = !array_is_empty(items);
    getter filterStatus: String = filter;

    // ── action（同步、无返回值）──
    action add(text: String) {
        if text != "" {
            items = array_push(items, text);
            lastAction = "added: " + text;
            save();
        }
    }

    // ── action（多参数 + 返回值）──
    action addTask(title: String, priority: i32, due: String): String {
        if title == "" { return "error: title required"; }
        items = array_push(items, title);
        save();
        return "ok: " + title;
    }

    // ── action（async + 返回值）──
    action fetchReminders(): String async {
        let mut count = 0;
        while i < array_length(items) { count = count + 1; }
        lastAction = "synced " + count.to_string() + " items";
        return "synced " + count.to_string() + " reminders";
    }

    action remove() { items = array_pop(items); save(); }
    action clear() { items = []; lastAction = "all cleared"; save(); }
}
```

### action 语法总览

```omniview
store X {
    action name(params) { ... }                    // 同步，无返回值
    action name(params): ReturnType { ... }         // 同步，有返回值
    action name(params): ReturnType async { ... }   // 异步，有返回值
}
```

### 内联 Store（组件内部）

```omniview
component InlineStoreDemo {
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

        action removeLast() {
            if items.len() > 0 { items.pop(); totalPrice = totalPrice - 10.0; }
        }

        action clear() { items = []; totalPrice = 0.0; }
    }

    fn addApple() { CartStore.addItem("Apple", 5.0); }
    fn addBook()  { CartStore.addItem("OmniView Guide", 39.9); }

    view {
        <div>
            <p>Items: {CartStore.itemCount}</p>
            <p>Summary: {CartStore.summary}</p>
            <p>Total: {CartStore.totalPrice}</p>
            <button @onPress={addApple}>Add Apple</button>
            <button @onPress={addBook}>Add Book</button>
        </div>
    }
}
```

### `@store` — 引用全局 Store

```omniview
import { TodoStore } from "../stores/TodoStore";

component App {
    @store TodoStore;     // ← 引入，名称与 import 一致
    @store ThemeStore;

    view {
        <div>
            <p>Items: {TodoStore.totalCount}</p>
            <p>Filter: {TodoStore.filterStatus}</p>
            <p>Last: {TodoStore.lastAction}</p>
            <button @onPress={() => { TodoStore.setFilter("all"); }}>All</button>
            <button @onPress={() => { TodoStore.setFilter("urgent"); }}>Urgent</button>
        </div>
    }
}
```

### Store 成员访问规则

```
TodoStore.items         → 返回 items 信号的值（Proxy 自动解包，等价 items.get()）
TodoStore.totalCount    → 返回 getter 的计算值
TodoStore.add("text")   → 调用 action 函数
TodoStore.items = []    → 等价 items.set([])
```

---

## 四、`getter` — Store 内的派生值

**`getter` 只在 `store { }` 内可用**，等价于组件中的 `computed`。

```omniview
store ThemeStore {
    signal mode: String = "light";
    signal toggleCount: i32 = 0;

    getter currentBg: String = if mode == "dark" { "#111827" } else { "#ffffff" };
    getter currentText: String = if mode == "dark" { "#f9fafb" } else { "#111827" };
    getter currentPrimary: String = if mode == "dark" { "#a5b4fc" } else { "#6366f1" };
    getter modeLabel: String = if mode == "dark" { "Dark" } else { "Light" };
    getter nextLabel: String = if mode == "dark" { "Light" } else { "Dark" };
}
```

### getter vs computed

| 特性 | Store 的 `getter` | 组件的 `computed` |
|------|------------------|-------------------|
| 作用域 | 全局单例 | 组件局部（每个实例独立） |
| 依赖 | 仅本 store 内的 signal | 组件内任何 signal / store |
| 用途 | 派生的 Store 状态，跨组件复用 | 组件专属的派生值 |

---

## 五、`fn` — 组件的本地函数

### 函数签名变体

```omniview
component Example {
    fn noArgsNoReturn() { count = 0; }
    fn withArgs(a: i32, b: String) { ... }
    fn withReturn() -> String { return "hello"; }
    fn withArgsAndReturn(x: i32) -> bool { return x > 0; }
    fn asyncFn() -> String async { return "done"; }

    // 闭包
    fn getBonusTotal() -> i32 {
        let bonus = |s| s + 5;
        return bonus(score);
    }

    fn getActiveRows() -> Array<RowItem> {
        return array_filter(rows, |r| r.status == "Active");
    }

    fn getGradeForScore(score: i32) -> String {
        if score >= 90 { return "A"; }
        if score >= 80 { return "B"; }
        if score >= 70 { return "C"; }
        return "F";
    }
}
```

### fn 内可做的操作

```omniview
fn complexOps() {
    // 1. 读写本组件 signal
    count = count + 1;

    // 2. 调用 store action
    TodoStore.add("新事项");

    // 3. 读写 store 的 signal/getter
    let total = TodoStore.totalCount;

    // 4. 局部变量
    let mut x = 42;
    let arr: Array<i32> = [10, 20, 30];

    // 5. 控制流
    let mut i = 0;
    while i < len { i = i + 1; }
    if cond { ... } else { ... }

    // 6. 调用其他 fn
    let status = getModStatus();

    // 7. 调用 std 库
    storage_set("key", "value");
    let data = json_parse(raw);
    let sorted = array_sort(rows, |a, b| a.name);
}
```

---

## 六、`event` — 组件自定义事件（子→父通信）

### 声明与触发

```omniview
component TodoItem {
    event onDelete: i32;       // 声明事件，携带 i32 类型数据
    event onToggle: bool;

    fn handleClick() {
        onDelete(index);       // 触发事件
    }
}
```

### 在父组件中接收

```omniview
component TodoList {
    fn handleDelete(index: i32) {
        TodoStore.removeAt(index);
    }

    view {
        <TodoItem @onDelete={handleDelete} />
//                ^^^^^^^^^^^^^^^^^^^^^^^^^ 事件绑定
    }
}
```

### 事件中继（子→当前→父）

```omniview
component TodoList {
    event onDelete: i32;        // 中继给父组件

    fn handleDelete(index: i32) {
        onDelete(index);        // 收到子组件事件，转发给父组件
    }

    view {
        <TodoItem @onDelete={handleDelete} />
    }
}
```

---

## 七、`prop` — 父→子数据

```omniview
component TodoList {
    prop items: Array<String> = [];   // 带默认值
    prop title: String = "默认标题";

    view {
        <h2>{title}</h2>
        @for item in items { ... }
    }
}

// 父组件传值：
<TodoList items={TodoStore.items} title={"待办列表"} />
```

---

## 八、生命周期钩子

```omniview
component LifecycleDemo {
    signal input: String = "";

    // @mount — 组件挂载后执行一次
    @mount {
        console_log("mounted");
        // 初始化数据、订阅事件、启动定时器
    }

    // @unmount — 组件卸载前执行
    @unmount {
        println("unmounting");
        // 清理定时器、取消订阅、释放资源
    }

    // @effect(deps) — 依赖变化时执行（DOM 更新后）
    @effect(input) {
        if input.len() > 20 {
            println("输入较长：" + input.len().to_string() + " 字符");
        }
    }

    view { ... }
}
```

### 各钩子对比

| 钩子 | 执行时机 | 适用场景 |
|------|---------|---------|
| `@mount` | 组件挂载到 DOM 后 | 数据初始化、订阅、定时器 |
| `@unmount` | 组件从 DOM 移除前 | 清理资源、取消订阅 |
| `@effect(deps)` | 指定依赖每次变化后 | 日志、持久化、与非响应式库同步 |

---

## 九、`ref` — DOM 引用

```omniview
component FocusInput {
    ref inputEl: HtmlElement;

    @mount {
        inputEl.focus();
    }

    view {
        <input ref="inputEl" placeholder="我挂载后会自动聚焦" />
    }
}
```

---

## 十、`const` — 编译期常量

```omniview
component StatsDashboard {
    const MAX_ITEMS = 10;                    // 无类型标记
    const $iconUrl = "/assets/icon.png";      // $ 前缀 = 字符串常量
    const $apiBase = "https://api.example.com";

    view {
        <p>MAX_ITEMS: {MAX_ITEMS}</p>
        <img src={$iconUrl} />
    }
}
```

---

## 十一、条件编译模式：`ref:` / `@if`

```omniview
component Conditional {
    signal removed: bool = false;

    fn removeContainer() { removed = true; }

    view {
        <div>
            @if !removed {                          // 条件渲染
                <button @onPress={removeContainer}>
                    Trigger unmount
                </button>
            }

            <div ref: if_mode_default="hidden">      // CSS display 模式
                ...
            </div>
        </div>
    }
}
```

---

## 十二、完整数据流图谱

```
┌───────────────────────────────────────────────────────────────────┐
│  父子组件通信                                                      │
│                                                                   │
│  父组件                             子组件                          │
│  ┌──────────┐     prop: items →    ┌──────────┐                  │
│  │  App     │ ──────────────────→  │ TodoList │                  │
│  │          │                      │          │                  │
│  │          │  ← event: onDelete   │          │                  │
│  │          │ ←──────────────────  │          │                  │
│  └──────────┘                      └──────────┘                  │
│       │                               │                          │
│       │ @store                         │ @store                   │
│       ▼                               ▼                          │
│  ┌──────────────────────────────────────────────────────┐        │
│  │  Global Store: TodoStore                             │        │
│  │                                                      │        │
│  │  ┌──────────┐    ┌──────────┐    ┌──────────────┐    │        │
│  │  │  signal  │──→ │  getter  │    │   action     │    │        │
│  │  │  items   │    │ totalCount│   │   add()      │    │        │
│  │  │  filter  │    │ hasItems  │    │   remove()   │    │        │
│  │  │ lastAction│   │ filterSt  │    │ fetchRemind()│    │        │
│  │  └──────────┘    └──────────┘    └──────────────┘    │        │
│  └──────────────────────────────────────────────────────┘        │
│                                                                   │
│  响应链：                                                         │
│  用户点击 → @onPress → fn/action → signal.set()                  │
│                                     ├─ computed.dirty = true      │
│                                     ├─ view 中 {signal} 自动更新   │
│                                     └─ @effect(deps) 重跑         │
└───────────────────────────────────────────────────────────────────┘
```

---

## 十三、速查表：什么时候用什么

| 场景 | 用哪个 |
|------|--------|
| 组件内局部可变状态 | `signal` |
| 从 signal 派生的只读值 | `computed`（简单或函数式） |
| 跨组件共享状态 | `store { signal + getter + action }` |
| 组件内部封装的 store | 内联 `store Name { }` |
| 修改状态的操作逻辑 | `fn`（组件内）或 `action`（store 内） |
| 父→子传数据 | `prop` |
| 子→父通知 | `event` + `@onEvent` |
| 初始化/清理 | `@mount` / `@unmount` |
| 依赖跟踪副作用 | `@effect(deps)` |
| DOM 元素引用 | `ref` |
| 编译期常量 | `const` |
| 条件渲染 | `@if` / `ref: if_mode_default` |
| 列表渲染 | `@for ... @empty` |

---

## 十四、与编译器 / 运行时的对应关系

```
.omni 源码              编译器输出 (JS)                     运行时
───────────────────────────────────────────────────────────────
signal count: i32 = 0   →  const Counter_count =               SignalImpl
                            __omni_signal("Counter_count", 0);

computed doubled: i32    →  const Counter_doubled =             ComputedImpl
  = count * 2              __omni_computed("Counter_doubled",
                             () => (Counter_count.value * 2));

store TodoStore { ... }  →  const TodoStore_items = ...        store() Proxy
                            function TodoStore_add(text) {...}

{count} in view          →  __omni_bindText("nid:1")            信号→DOM 绑定
                            .text = Counter_count.value

@onPress={inc}           →  el.addEventListener("click",       DOM 事件绑定
                              Counter_inc)

bind:value={search}      →  input.value = SearchBox_search      双向绑定
                            input.oninput = (e) => {
                              SearchBox_search.value = e.val }
```

---

> 更多细节参见：
> - [响应式与状态](tutorial/08-reactivity-and-state.md)
> - [Store 与 Action](tutorial/09-stores-and-actions.md)
> - [OmniView 组件](tutorial/07-omniview-components.md)
> - 运行时源码：`editor/runtime/src/omniview-runtime.ts`
> - 编译器代码生成：`omni-compiler/src/codegen/omniview.rs`
