# 7. OmniView 组件

> **目标**：用 OmniView 组件构建 UI —— 模板、属性、条件渲染、列表遍历和组件组合。

OmniView 是 OmniLang 的 UI 框架，借鉴了 Flutter 的组件模型。在文件顶部添加 `import omniview;` 即可激活 UI 层。

跟着一起做：

```bash
omnic create learn-ui --type view
cd learn-ui
```

---

## `import omniview` 大门

`import omniview;` 这行代码将 `.omni` 文件从纯逻辑转变为支持 UI：

```omni
// 纯逻辑文件 — 没有 import omniview
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

```omni
// 支持 UI 的文件 — 有 import omniview
import omniview;

component App {
    view {
        <div>
            <h1>你好，OmniView！</h1>
        </div>
    }
}
```

> **规则**：带 `import omniview` 的视图文件不能被纯逻辑文件导入。这保证了清晰的逻辑↔UI 边界。

---

## 组件声明

用 `component` 关键字声明组件。组件必须包含一个 `view { ... }` 块来定义 HTML 模板：

```omni
import omniview;

component Greeting {
    view {
        <div>
            <h1>欢迎！</h1>
            <p>这是我的第一个组件。</p>
        </div>
    }
}
```

### `view` 块

`view` 块使用类 HTML 语法，但有一些重要区别：

- **所有标签必须闭合** —— `<br />` 而不是 `<br>`，`<img />` 而不是 `<img>`
- **文本插值**用 `{表达式}` —— 不是 `{{}}` 或 `${}`
- **组件名使用 PascalCase（大驼峰）** —— `<TodoList />` 而不是 `<todo-list />`
- **标准 HTML 标签用小写** —— `<div>`、`<button>`、`<input>`

```omni
component Profile {
    view {
        <div>
            <h2>用户资料</h2>
            <img src="avatar.png" alt="头像" />
            <p>姓名：Alice</p>
            <input type="text" placeholder="输入姓名" />
            <br />
            <button>保存</button>
        </div>
    }
}
```

---

## 属性（Props）

组件通过 **prop**（属性）接收外部数据。在组件体内用 `prop` 关键字声明：

```omni
component PostCard {
    prop title: String;
    prop body: String = "";       // 可选属性，带默认值
    prop author: String;

    view {
        <div>
            <h2>{title}</h2>
            <p>{body}</p>
            <span>作者：{author}</span>
        </div>
    }
}
```

### 传递属性

属性以类 HTML 的方式传递：

```omni
component App {
    view {
        <div>
            <PostCard
                title="你好世界"
                author="Alice"
                body="这是我的第一篇博客。"
            />
            <PostCard title="另一篇文章" author="Bob" />
        </div>
    }
}
```

---

## 文本插值

元素内容中任何 `{ }` 包裹的 OmniLang 表达式都会被求值并作为文本插入：

```omni
component Counter {
    signal count: i32 = 0;

    view {
        <div>
            <span>计数：{count}</span>
            <span>下一个：{count + 1}</span>
        </div>
    }
}
```

---

## 条件渲染：`@if` / `@else if` / `@else`

`@if` 指令根据条件渲染元素：

```omni
component StatusBadge {
    signal status: String = "active";

    view {
        <div>
            @if status == "active" {
                <span class: badge-active>活跃</span>
            } else if status == "pending" {
                <span class: badge-pending>待处理</span>
            } else {
                <span class: badge-inactive">停用</span>
            }
        </div>
    }
}
```

`@if` 指令可以包裹任意数量的元素：

```omni
component Dashboard {
    signal loggedIn: bool = false;

    view {
        <div>
            @if loggedIn {
                <h2>欢迎回来！</h2>
                <button>退出登录</button>
            } else {
                <p>请登录后继续。</p>
                <button>登录</button>
            }
        </div>
    }
}
```

---

## 列表渲染：`@for` / `@empty`

`@for` 指令遍历数组。可选的 `@empty` 块在数组为空时显示：

```omni
component ProductList {
    signal products: Array<String> = ["小工具", "小装备", "小玩意儿"];

    view {
        <div>
            <h2>产品列表</h2>
            <ul>
                @for product in products {
                    <li>{product}</li>
                }
                @empty {
                    <li>暂无产品</li>
                }
            </ul>
        </div>
    }
}
```

`@for` 可以遍历任何类型的数组——字符串、数字、结构体甚至组件：

```omni
component BlogFeed {
    signal posts: Array<String> = ["第一篇文章", "第二篇文章"];

    view {
        <div>
            @for post in posts {
                <PostCard title={post} author="匿名" />
            }
            @empty {
                <p>还没有文章。</p>
            }
        </div>
    }
}
```

---

## 组件组合

组件可以嵌套在其他组件内部。使用 PascalCase 组件名作为标签：

```omni
component CartIcon {
    signal count: i32 = 0;

    view {
        <div>
            <span>{count}</span>
            <span>🛒 购物车</span>
        </div>
    }
}

component Header {
    view {
        <header>
            <h1>我的商店</h1>
            <CartIcon />
        </header>
    }
}

component App {
    view {
        <div>
            <Header />
            <main>
                <p>欢迎光临！</p>
            </main>
        </div>
    }
}
```

---

## 事件绑定

组件通过事件绑定响应用户交互。支持两种语法：

| 语法 | 示例 | 说明 |
|------|------|------|
| `@eventName={handler}` | `@onPress={handleClick}` | 推荐写法 |
| `onEvent={handler}` | `onPress={handleClick}` | 不带 `@` 前缀 |

```omni
component Clicker {
    signal count: i32 = 0;

    fn increment() {
        count = count + 1;
    }

    view {
        <div>
            <span>点击了 {count} 次</span>
            <button @onPress={increment}>点我</button>
        </div>
    }
}
```

### 内联事件处理器

简单场景可以直接定义行内事件处理器：

```omni
component SimpleCounter {
    signal count: i32 = 0;

    view {
        <div>
            <button @onPress={() => { count = count + 1; }}>
                +1
            </button>
            <span>{count}</span>
        </div>
    }
}
```

---

## 综合示例：待办列表

一个整合本章所有知识的完整待办列表组件（基于 `test/projects/view-todo`）：

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
                    <li>{item}</li>
                }
                @empty {
                    <li>还没有任务</li>
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

> `bind:value={input}` 和 `signal` 将在[教程 8：响应式与状态](08-reactivity-and-state.md)中详细解释。

---

## 组件完整参考

一个组件可以包含以下所有成员：

```omni
component Example {
    // 外部属性
    prop title: String;
    prop count: i32 = 0;

    // 响应式状态（见教程 8）
    signal name: String = "";
    computed display: String = title + ": " + name;

    // DOM 引用（见教程 8）
    ref inputEl: HtmlElement;

    // 自定义事件
    event onSubmit: String;

    // Store 引用（见教程 9）
    @store AppStore;

    // 生命周期钩子（见教程 8）
    @mount { /* 挂载时执行 */ }
    @unmount { /* 卸载时执行 */ }

    // 组件函数
    fn helper() -> String { return "帮助"; }

    // 模板
    view {
        <div>{title}</div>
    }

    // 作用域样式（见教程 10）
    style {
        .title { font-size: 16px; }
    }
}
```

---

## 下一步

- **[8. 响应式与状态](08-reactivity-and-state.md)** — 信号、计算属性、双向绑定和生命周期
- **[9. Store 与 Action](09-stores-and-actions.md)** — 全局状态管理
- **[10. 样式与主题](10-styling-and-themes.md)** — CSS 和设计令牌
