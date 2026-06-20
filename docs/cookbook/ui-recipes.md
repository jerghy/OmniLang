# UI 食谱

> 即用型 OmniView 模式 —— 待办列表、购物车、仪表板等。

---

## 食谱：待办列表

经典待办应用 —— `bind:value`、信号、事件处理、`@for` + `@empty`。

```omni
import omniview;

component TodoList {
    signal items: Array<String> = [];
    signal input: String = "";

    fn add_item() {
        if input != "" {
            items.push(input);
            input = "";
        }
    }

    view {
        <div>
            <h2>待办列表</h2>

            <div>
                <input bind:value={input} placeholder="添加任务..." />
                <button @onPress={add_item}>添加</button>
            </div>

            <ul>
                @for item in items {
                    <li>
                        <span>{item}</span>
                        <button @onPress={() => { items.pop(); }}>
                            ✕
                        </button>
                    </li>
                }
                @empty {
                    <li>还没有任务——在上面添加一个吧！</li>
                }
            </ul>
        </div>
    }
}

component App {
    view {
        <div>
            <TodoList />
        </div>
    }
}
```

**来源**：基于 `test/projects/view-todo/src/main.omni`

---

## 食谱：带 Store 的购物车

产品列表 + 购物车 Store + 跨组件状态共享。

```omni
import omniview;

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

component CartIcon {
    @store CartStore;

    view {
        <div>
            <span>🛒 {CartStore.count}</span>
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

**来源**：基于 `test/projects/view-ecommerce/src/main.omni`

---

## 食谱：博客信息流

混合逻辑 + 视图 —— 结构体数据、组件组合、`@for` 渲染。

```omni
import omniview;

pub struct Post {
    title: String,
    body: String,
    author: String,
}

pub fn format_date(timestamp: i64) -> String {
    return "2026-06-17";
}

component PostCard {
    prop title: String;
    prop body: String = "";
    prop author: String;

    view {
        <div>
            <h2>{title}</h2>
            <p>{body}</p>
            <span>作者：{author}</span>
        </div>
    }
}

component App {
    signal posts: Array<String> = ["第一篇文章", "第二篇文章"];

    view {
        <div>
            <h1>我的博客</h1>
            @for post in posts {
                <PostCard title={post} author="Admin" body="这是一段正文..." />
            }
            @empty {
                <p>还没有文章。</p>
            }
        </div>
    }
}
```

**来源**：基于 `test/projects/mixed-blog/src/main.omni`

---

## 食谱：实时计数器

信号、计算属性、副作用和步长控制。

```omni
import omniview;

component LiveCounter {
    signal count: i32 = 0;
    signal step: i32 = 1;

    computed doubled: i32 = count * 2;
    computed isEven: bool = count % 2 == 0;

    fn increment() { count = count + step; }
    fn decrement() { count = count - step; }
    fn reset() { count = 0; }

    @effect(count) {
        if count != 0 && count % 10 == 0 {
            println("里程碑：" + count.to_string());
        }
    }

    view {
        <div>
            <h2>计数器</h2>
            <h1>{count}</h1>
            <p>翻倍：{doubled}</p>
            <p>{if isEven { "偶数" } else { "奇数" }}</p>

            <div>
                <button @onPress={decrement}>−</button>
                <span>步长：{step}</span>
                <button @onPress={increment}>+</button>
            </div>

            <div>
                <label>步长值：</label>
                <input type="number" bind:value={step} />
            </div>

            <button @onPress={reset}>重置</button>
        </div>
    }
}
```

---

## 食谱：搜索过滤

带双向绑定的实时过滤列表。

```omni
import omniview;
import { to_lower, contains } from "omni/string";

component SearchList {
    signal query: String = "";
    signal allItems: Array<String> = [
        "苹果", "香蕉", "樱桃",
        "枣子", "接骨木莓", "无花果",
        "葡萄", "蜜瓜",
    ];

    computed filteredItems: Array<String> = {
        if query == "" {
            return allItems;
        }
        let mut result: Array<String> = [];
        for item in allItems {
            if contains(to_lower(item), to_lower(query)) {
                result.push(item);
            }
        }
        return result;
    };

    view {
        <div>
            <h2>搜索</h2>
            <input bind:value={query} placeholder="输入关键字筛选..." />

            <p>共 {filteredItems.len().to_string()} 条结果</p>

            <ul>
                @for item in filteredItems {
                    <li>{item}</li>
                }
                @empty {
                    <li>没有找到匹配 "{query}" 的结果</li>
                }
            </ul>
        </div>
    }
}
```

---

## 食谱：仪表板 KPI 卡片

带主题令牌和计算指标的网格布局。

```omni
import omniview;

theme DashboardTheme {
    colors {
        primary: "#6366f1",
        success: "#22c55e",
        bg: "#f8fafc",
        surface: "#ffffff",
        text: "#1e293b",
        muted: "#94a3b8",
    }
    spacing {
        md: "16px",
        lg: "24px",
    }
}

component KpiCard {
    prop label: String;
    prop value: String;

    view {
        <div class: card>
            <span class: label>{label}</span>
            <span class: value>{value}</span>
        </div>
    }

    style {
        .card {
            background-color: var(--colors-surface);
            padding: var(--spacing-md);
            border-radius: 12px;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
            display: flex;
            flex-direction: column;
            gap: 4px;
        }
        .label {
            font-size: 14px;
            color: var(--colors-muted);
        }
        .value {
            font-size: 32px;
            font-weight: bold;
            color: var(--colors-text);
        }
    }
}

component Dashboard {
    view {
        <div class: container>
            <h1 class: heading>仪表板</h1>
            <div class: grid>
                <KpiCard label="收入" value="¥12,345" />
                <KpiCard label="用户" value="1,234" />
                <KpiCard label="转化率" value="3.2%" />
                <KpiCard label="流失率" value="1.1%" />
            </div>
        </div>
    }

    style {
        .container {
            padding: var(--spacing-lg);
            background-color: var(--colors-bg);
            min-height: 100vh;
        }
        .heading {
            font-size: 28px;
            color: var(--colors-text);
            margin-bottom: var(--spacing-md);
        }
        .grid {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: var(--spacing-md);
        }
    }
}
```

---

## 食谱：登录表单

带验证、条件渲染和事件处理的表单。

```omni
import omniview;

component LoginForm {
    signal email: String = "";
    signal password: String = "";
    signal error: String = "";
    signal isLoading: bool = false;

    fn handleSubmit() {
        if email == "" {
            error = "请输入邮箱";
            return;
        }
        if password == "" {
            error = "请输入密码";
            return;
        }

        error = "";
        isLoading = true;
        // 模拟登录...
        println("正在登录：" + email);
        isLoading = false;
    }

    view {
        <div>
            <h2>登录</h2>

            @if error != "" {
                <div style: {
                    padding: "12px",
                    background-color: "#fef2f2",
                    color: "#dc2626",
                    border-radius: "6px",
                    margin-bottom: "16px",
                }>
                    {error}
                </div>
            }

            <div>
                <label>邮箱</label>
                <input
                    type="email"
                    bind:value={email}
                    placeholder="you@example.com"
                />
            </div>

            <div>
                <label>密码</label>
                <input
                    type="password"
                    bind:value={password}
                    placeholder="输入密码"
                />
            </div>

            <button
                @onPress={handleSubmit}
                disabled={isLoading}
            >
                {if isLoading { "加载中..." } else { "登录" }}
            </button>
        </div>
    }
}

component App {
    view {
        <div>
            <LoginForm />
        </div>
    }
}
```

---

## 食谱：标签页组件

带条件渲染的动态标签页切换。

```omni
import omniview;

component Tabs {
    signal activeTab: String = "overview";

    view {
        <div>
            <div class: tab-bar>
                <button
                    class: if activeTab == "overview" { "tab-active" } else { "tab" }
                    @onPress={() => { activeTab = "overview"; }}
                >
                    概览
                </button>
                <button
                    class: if activeTab == "settings" { "tab-active" } else { "tab" }
                    @onPress={() => { activeTab = "settings"; }}
                >
                    设置
                </button>
                <button
                    class: if activeTab == "billing" { "tab-active" } else { "tab" }
                    @onPress={() => { activeTab = "billing"; }}
                >
                    账单
                </button>
            </div>

            <div class: content>
                @if activeTab == "overview" {
                    <h3>概览</h3>
                    <p>欢迎来到你的仪表板。</p>
                } else if activeTab == "settings" {
                    <h3>设置</h3>
                    <p>配置你的偏好。</p>
                } else {
                    <h3>账单</h3>
                    <p>管理你的订阅。</p>
                }
            </div>
        </div>
    }

    style {
        .tab-bar {
            display: flex;
            gap: 4px;
            border-bottom: 2px solid #e5e7eb;
            margin-bottom: 16px;
        }
        .tab {
            padding: 8px 16px;
            border: none;
            background: none;
            cursor: pointer;
            color: #6b7280;
        }
        .tab-active {
            padding: 8px 16px;
            border: none;
            border-bottom: 2px solid #6366f1;
            background: none;
            cursor: pointer;
            color: #6366f1;
            font-weight: bold;
        }
        .content {
            padding: 16px;
        }
    }
}
```

---

## 运行任意食谱

```bash
# 创建视图项目
omnic create my-recipe --type view
cd my-recipe

# 将 src/main.omni 替换为食谱代码
# 然后运行：
omnic run src/main.omni
```

---

## 参见

- [逻辑食谱](logic-recipes.md) — CLI 工具、算法、数据处理
- [教程 7：OmniView 组件](../tutorial/07-omniview-components.md) — 组件基础
- [教程 8：响应式与状态](../tutorial/08-reactivity-and-state.md) — 信号和数据绑定
- [教程 9：Store 与 Action](../tutorial/09-stores-and-actions.md) — 全局状态管理
