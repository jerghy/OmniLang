# 05 — Store 状态管理

Store 是 OmniView 的状态管理容器。它封装共享的响应式状态和修改逻辑，可在多个组件之间共享。

## 全局 Store

### 定义 Store

```omni
import omniview;

store TodoStore {
    // 1. 响应式信号
    signal items: Todo[] = [];
    signal filterStatus: String = "all";
    signal lastAction: String = "none";

    // 2. 计算 getter
    getter totalCount: i32 = items.len();
    getter activeCount: i32 = items.filter(|item| !item.done).len();

    // 3. action 方法（修改信号）
    action add(text: String) {
        items = [...items, { id: items.len(), text, done: false }];
        lastAction = "added";
    }

    action remove(id: i32) {
        items = items.filter(|item| item.id != id);
        lastAction = "removed";
    }

    action toggle(id: i32) {
        items = items.map(|item|
            if item.id == id {
                { ...item, done: !item.done }
            } else {
                item
            }
        );
    }

    action clear() {
        items = [];
        filterStatus = "all";
        lastAction = "cleared";
    }

    action setFilter(status: String) {
        filterStatus = status;
    }
}
```

Store 成员：
- **signal**：响应式状态——与组件 signal 语义一致
- **getter**：派生计算值——类似于 computed，惰性求值。store 中**不支持** `computed`，用 `getter`
- **action**：修改 signal 的方法——只能在 action 内部修改 store signal

### Action 返回类型与异步

Action 可以有返回类型，也可以声明为 async：

```omni
store NotificationStore {
    signal notifications: Array<String> = [];

    // 带返回值的 action
    action add(message: String): String {
        if message == "" {
            return "empty message ignored";
        }
        notifications.push(message);
        return "added";
    }

    // 异步 action + 返回值
    action removeLast(): String async {
        if notifications.len() == 0 {
            return "no notifications";
        }
        notifications.pop();
        return "removed";
    }

    // 多参数 action + 返回值
    action addWithPriority(title: String, body: String, priority: i32): String {
        let full = title + ": " + body;
        notifications.push(full);
        return "added";
    }
}
```

### Store 持久化（storage）

使用 `std/storage` 在 action 中实现持久化：

```omni
import { storage_get, storage_set } from "std/storage";

store ThemeStore {
    signal mode: String = "light";
    signal toggleCount: i32 = 0;

    // 初始化时从 storage 恢复
    action load() {
        let raw = storage_get("theme-mode");
        if raw != null {
            mode = raw;
        }
    }

    // 修改时持久化
    action toggle() {
        if mode == "dark" { mode = "light"; }
        else { mode = "dark"; }
        toggleCount = toggleCount + 1;
        storage_set("theme-mode", mode);
    }
}
```

在组件中调用 load action：
```omni
component App {
    @store ThemeStore;

    @mount {
        ThemeStore.load();    // 挂载时恢复主题
    }
}
```

### 在组件中使用 Store

使用 `@store` 在组件中引用全局 Store：

```omni
component TodoApp {
    @store TodoStore;       // 引用 TodoStore
    @store ThemeStore;      // 引用 ThemeStore

    signal newText: String = "";

    fn handleAdd() {
        TodoStore.add(newText);  // 调用 store action
        newText = "";
    }

    fn handleRemove(id: i32) {
        TodoStore.remove(id);
    }

    view {
        <div>
            <!-- 直接访问 store signal -->
            <p>Total: {TodoStore.totalCount}</p>
            <p>Filter: {TodoStore.filterStatus}</p>

            <!-- 直接访问 store getter -->
            <p>Active: {TodoStore.activeCount}</p>

            <!-- 添加新项 -->
            <input bind:value={newText} />
            <button @onPress={handleAdd}>Add</button>

            <!-- 过滤器 -->
            <button @onPress={TodoStore.setFilter("all")}>All</button>
            <button @onPress={TodoStore.setFilter("active")}>Active</button>

            <!-- 列表 -->
            <ul>
                @for item in TodoStore.items {
                    <li>
                        <span>{item.text}</span>
                        <button @onPress={() => { TodoStore.remove(item.id); }}>
                            ✕
                        </button>
                    </li>
                }
            </ul>
        </div>
    }
}
```

访问方式：
- Store signal：`StoreName.signalName`
- Store getter：`StoreName.getterName`
- Store action：`StoreName.actionName(args)`
- 在事件处理器中可直接传递 store action：`@onPress={StoreName.action}`

### Store 命名约定

编译后，store 字段的 JavaScript 变量使用 `{StoreName}_{fieldName}` 命名：
- `TodoStore.items` → JS 变量 `TodoStore_items`
- `TodoStore.totalCount` → JS 变量 `TodoStore_totalCount`

## 内联 Store

在 component 内部定义**局部 store**——仅在当前组件内可见，不导出到外部：

```omni
component InlineStoreDemo {
    // 内联 store 定义在 component 内部
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

    // 直接使用内联 store 的成员
    view {
        <div>
            <p>Items: {itemCount}</p>
            <p>Summary: {summary}</p>
            <p>Total: {totalPrice}</p>
            <button @onPress={CartStore.addItem("Apple", 5.0)}>Add</button>
        </div>
    }
}
```

内联 store 的 getter 也可以直接在 view 中访问，不需要 store 名称前缀。

## Store 设计最佳实践

### 1. 单一职责

每个 store 管理一个领域的状态：

```
stores/
├── TodoStore.omni      # 待办事项的增删改查
├── ThemeStore.omni     # 主题切换
├── CartStore.omni      # 购物车
└── NotificationStore.omni  # 通知消息
```

### 2. 通过 action 修改状态

所有 signal 修改应在 store 的 action 中进行，不在组件中直接修改 store signal：

```omni
// ✓ 推荐：通过 action 修改
TodoStore.add("new item");

// ✗ 不推荐：直接修改（lint 会报错）
TodoStore.items = [...TodoStore.items, newItem];
```

### 3. 不可变更新

Array 操作应使用不可变模式：

```omni
action add(text: String) {
    items = [...items, newItem];       // 展开 + 新元素
}

action remove(id: i32) {
    items = items.filter(|item| item.id != id);  // filter 返回新数组
}
```

### 4. Store 间通信

一个 store 的 action 可以调用另一个 store：

```omni
store NotificationStore {
    signal messages: Array<String> = [];

    action notify(msg: String) {
        messages = [...messages, msg];
    }
}

store TodoStore {
    action add(text: String) {
        items = [...items, { id: items.len(), text, done: false }];
        NotificationStore.notify("Added: " + text);
    }
}
```

## Store 与 Component 对比

| 特性 | Component | Store |
|------|-----------|-------|
| 作用域 | 组件内部 | 全局 / 局部 |
| signal | ✓ | ✓ |
| getter | ✓ | ✓ |
| computed | ✓ | - |
| action | ✓ (组件内) | ✓ (专用) |
| fn | ✓ | - |
| view / style | ✓ | - |
| prop / event | ✓ | - |
| 导出 | 组件名 | store 名 |
