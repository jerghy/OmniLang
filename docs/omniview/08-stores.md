# 08 — Store 状态管理

Store 是 OmniView 的共享状态容器。封装信号、派生值和修改逻辑，可在多个组件间共享。

## 定义 Store

```omni
import omniview;

struct Todo {
    id: i32,
    text: String,
    done: bool,
}

store TodoStore {
    // 1. 响应式信号
    signal items: Todo[] = [];
    signal filterStatus: String = "all";

    // 2. 计算属性（store 中用 getter）
    getter totalCount: i32 = items.len();
    getter activeCount: i32 = items.count(|item| !item.done);

    // 3. 修改方法（store 中用 action）
    action add(text: String) {
        items = [...items, { id: items.len(), text, done: false }];
    }

    action remove(id: i32) {
        items = items.filter(|item| item.id != id);
    }

    action clear() {
        items = [];
        filterStatus = "all";
    }
}
```

Store 成员：
- **signal** — 响应式状态，与组件 signal 语义一致
- **getter** — 派生计算值（等价于组件的 `computed`），惰性求值
- **action** — 修改 signal 的方法，只能在 action 内部修改 store signal

> Store 不支持 `computed`，用 `getter` 代替。

## 数组方法（filter/map/count）

`.filter(|x| ...)`、`.map(|x| ...)`、`.count(|x| ...)` 是正式的语言方法（无需 import），跨目标映射为各平台惯用写法：

| 方法 | React / JS | Flutter / Dart |
|------|-----------|----------------|
| `.filter(\|x\| cond)` | `items.filter((x) => cond)` | `items.where((x) => cond).toList()` |
| `.map(\|x\| expr)` | `items.map((x) => expr)` | `items.map((x) => expr).toList()` |
| `.count(\|x\| cond)` | `items.filter((x) => cond).length` | `items.where((x) => cond).length` |

`count` 需要一个谓词闭包（`items.count()` 是编译错误）；`array_filter`/`array_map` 等 stdlib 函数保留为底层接口。

## Action 返回类型与异步

```omni
import { http_get } from "std/http";

store NotificationStore {
    signal messages: Array<String> = [];

    // 带返回值
    action add(msg: String): String {
        if msg == "" { return "empty ignored"; }
        messages.push(msg);
        return "added";
    }

    // 异步 + 返回值
    action fetch(): String async {
        let raw = await http_get("/api/notifications");
        messages = [...messages, raw];
        return "fetched";
    }
}
```

## 持久化（storage）

```omni
import { storage_get, storage_set } from "std/storage";

store ThemeStore {
    signal mode: String = "light";

    action load() {
        let saved = storage_get("theme-mode");
        if saved != null { mode = saved; }
    }

    action toggle() {
        mode = if mode == "dark" { "light" } else { "dark" };
        storage_set("theme-mode", mode);
    }
}
```

## 在组件中使用 Store

`@store` 引用后即可访问：

```omni
struct Todo {
    id: i32,
    text: String,
    done: bool,
}

store TodoStore {
    signal items: Todo[] = [];
    signal filterStatus: String = "all";

    getter totalCount: i32 = items.len();

    action add(text: String) { items = [...items, { id: items.len(), text, done: false }]; }
    action clear() { items = []; }
}

component TodoApp {
    @store TodoStore;

    view {
        <p>Total: {TodoStore.totalCount}</p>      <!-- getter -->
        <p>Filter: {TodoStore.filterStatus}</p>    <!-- signal -->

        <!-- action 调用 -->
        <button @onPress={TodoStore.add("New")}>Add</button>
        <button @onPress={TodoStore.clear}>Clear</button>

        <!-- 列表 -->
        @for item in TodoStore.items {
            <li>{item.text}</li>
        }
    }
}
```

访问方式：
- Store signal：`StoreName.signalName`
- Store getter：`StoreName.getterName`
- Store action：`StoreName.actionName(args)`

## 内联 Store

在组件内部定义的局部 store，仅当前组件可见：

```omni
component CartDemo {
    store CartStore {
        signal items: Array<String> = [];
        getter count: i32 = items.len();

        action add(name: String) {
            items.push(name);
        }
    }

    view {
        <p>Count: {count}</p>                    <!-- getter 可直接访问 -->
        <button @onPress={CartStore.add("Item")}> <!-- action 需 StoreName -->
            Add
        </button>
    }
}
```

内联 store 的 getter 和 signal 可以直接在 view 中访问（不需要前缀）。action 仍需 `StoreName.actionName()`。

## Store vs Component

| 特性 | Component | Store |
|------|-----------|-------|
| 作用域 | 组件内部 | 全局 / 组件内联 |
| signal | ✓ | ✓ |
| computed | ✓ | —（用 getter） |
| getter | — | ✓ |
| action | — | ✓ |
| fn | ✓ | — |
| view / style | ✓ | — |
| prop / event | ✓ | — |

## 设计最佳实践

1. **单一职责**：每个 store 管理一个领域（Todos、Theme、Cart）
2. **通过 action 修改状态**：不在组件中直接修改 store signal
3. **不可变更新 Array**：`items = [...items, newItem]` 或 `items.filter(...)`
4. **`.push()` / `.pop()` 自动响应式**：原地操作也会触发更新
5. **Store 间可互相调用**：`NotificationStore.notify("Added: " + text)`
