# 9. Store 与 Action

> **目标**：用 Store 管理全局应用状态 —— 共享信号、计算 getter 和 action。

组件内部的信号是局部的。当多个组件需要共享状态时，使用 **Store**（状态存储）。Store 是全局的单例状态容器，任何组件都可以引用。

---

## Store 声明

用 `store` 关键字在文件顶层声明 Store：

```omni
store CartStore {
    signal items: Array<String> = [];
    signal total: f64 = 0.0;

    getter count: i32 = items.len();
}
```

一个 Store 包含：
- **`signal`** —— 响应式状态（和组件信号一样）
- **`getter`** —— 基于 Store 信号的计算/派生值
- **`action`** —— 修改 Store 状态的函数

---

## 在组件中引用 Store

在组件内用 `@store StoreName;` 使 Store 可用：

```omni
component CartIcon {
    @store CartStore;

    view {
        <div>
            <span>{CartStore.count}</span>
            <span>🛒</span>
        </div>
    }
}
```

用 `StoreName.field` 访问 Store 字段——同样的点号语法在组件逻辑和模板中都可以使用。

---

## Store 示例：购物车

一个完整的购物车示例（基于 `test/projects/view-ecommerce`）：

```omni
import omniview;

// ── Store ────────────────────────────────────

store CartStore {
    signal items: Array<String> = [];
    signal total: f64 = 0.0;

    getter count: i32 = items.len();

    action add_item(product: String) {
        items.push(product);
    }

    action clear() {
        items = [];
        total = 0.0;
    }
}

// ── 组件 ─────────────────────────────────────

component CartIcon {
    @store CartStore;

    view {
        <div>
            <span>{CartStore.count}</span>
            <span>购物车</span>
        </div>
    }
}

component ProductList {
    signal products: Array<String> = ["小工具", "小装备", "小玩意儿"];
    @store CartStore;

    fn buy(product: String) {
        CartStore.add_item(product);
    }

    view {
        <div>
            <h2>产品列表</h2>
            <ul>
                @for product in products {
                    <li>
                        <span>{product}</span>
                        <button @onPress={() => { buy(product); }}>
                            加入购物车
                        </button>
                    </li>
                }
                @empty {
                    <li>暂无产品</li>
                }
            </ul>
        </div>
    }
}

component App {
    @store CartStore;

    view {
        <div>
            <header>
                <CartIcon />
            </header>
            <main>
                <ProductList />
            </main>
        </div>
    }
}
```

当用户点击"加入购物车"时：
1. `CartStore.add_item()` 被调用
2. Store 的 `items` 信号更新
3. `CartStore.count` getter 重新计算
4. `CartIcon` 组件用新的数量重新渲染
5. `ProductList` 组件保持不变（它不读取变化了的信号）

---

## Action

Action 是 Store 专用的函数，封装了状态修改。它们让修改逻辑保持井然有序和可测试：

```omni
store UserStore {
    signal name: String = "";
    signal loggedIn: bool = false;
    signal preferences: Map<String, String> = {};

    getter isGuest: bool = !loggedIn;

    action login(username: String) {
        name = username;
        loggedIn = true;
    }

    action logout() {
        name = "";
        loggedIn = false;
        preferences = {};
    }

    action set_preference(key: String, value: String) {
        preferences.insert(key, value);
    }
}
```

### 为什么用 Action？

- **单一职责**——每个 action 只做一件事
- **可测试**——action 是普通函数，可以单元测试
- **可追踪**——所有修改都通过命名 action，调试更轻松
- **防止散落修改**——信号只应在其所属 Store 的 action 内部被修改

---

## 多 Store 组合

一个组件可以引用多个 Store：

```omni
store AuthStore {
    signal user: String = "";
    signal token: String = "";

    getter isLoggedIn: bool = user != "";

    action login(name: String) {
        user = name;
        token = "secret-token";
    }
}

store ThemeStore {
    signal mode: String = "light";    // "light" | "dark"

    getter isDark: bool = mode == "dark";

    action toggle() {
        mode = if mode == "light" { "dark" } else { "light" };
    }
}

component AppShell {
    @store AuthStore;
    @store ThemeStore;

    view {
        <div class: if ThemeStore.isDark { "dark-theme" } else { "light-theme" }>
            <header>
                @if AuthStore.isLoggedIn {
                    <span>欢迎，{AuthStore.user}</span>
                } else {
                    <button>登录</button>
                }
                <button @onPress={ThemeStore.toggle}>
                    {if ThemeStore.isDark { "☀️" } else { "🌙" }}
                </button>
            </header>
            <main>
                <p>这里是内容</p>
            </main>
        </div>
    }
}
```

---

## Getter vs Computed

| 特性 | Store 的 `getter` | 组件的 `computed` |
|------|------------------|-------------------|
| 作用域 | 全局（一个实例） | 组件局部（每个实例） |
| 依赖 | 仅 Store 信号 | 作用域内任意信号 |
| 用途 | 派生的 Store 状态 | 组件专属的派生值 |

```omni
store AppStore {
    signal items: Array<i32> = [1, 2, 3, 4, 5];

    getter count: i32 = items.len();           // Store 级别的派生
    getter sum: i32 = /* 聚合逻辑 */;   // 跨组件复用
}

component Dashboard {
    @store AppStore;
    signal filter: String = "all";

    // 组件级别派生——使用 Store 信号 + 本地信号
    computed filteredCount: i32 = if filter == "all" {
        AppStore.count
    } else {
        /* 过滤后的数量 */
    };

    view { ... }
}
```

---

## Store 最佳实践

1. **保持 Store 专注**——一个领域一个 Store（AuthStore、CartStore、ThemeStore），而不是一个巨大的 AppStore
2. **只通过 action 修改**——永远不要在组件中直接修改 Store 信号
3. **用 getter 处理派生状态**——避免存储可以从其他信号计算出来的值
4. **Store 是单例**——整个应用只有一个实例；每个实例的状态用组件信号
5. **Action 是同步的**——对于异步数据加载，在外部触发异步操作，然后用结果调用 action

---

## 下一步

- **[10. 样式与主题](10-styling-and-themes.md)** — 作用域 CSS 和设计令牌
- **[7. OmniView 组件](07-omniview-components.md)** — 组件基础
- **[8. 响应式与状态](08-reactivity-and-state.md)** — 深入信号和副作用
