# 8. 响应式与状态

> **目标**：理解 OmniView 的响应式系统 —— 信号（signal）、计算属性（computed）、副作用（effect）、双向绑定和生命周期钩子。

OmniView 的响应式系统借鉴了 SolidJS 和 Svelte。当状态变化时，UI 自动更新——无需手动操作 DOM，无需虚拟 DOM diff。

---

## 信号（Signal）

**信号**是一段响应式状态。当信号的值改变时，UI 中所有读取了它的部分会自动重新渲染。

```omni
component Counter {
    signal count: i32 = 0;

    fn increment() {
        count = count + 1;    // 改变 count 会触发重新渲染
    }

    view {
        <div>
            <span>计数：{count}</span>
            <button @onPress={increment}>+1</button>
        </div>
    }
}
```

### 信号的工作原理

1. **声明**用 `signal name: Type = 初始值`
2. **读取**在 `view` 或 `computed` 中——系统自动追踪依赖
3. **写入**（修改）——系统只重新渲染依赖于该信号的 DOM 节点
4. **细粒度更新**——没有虚拟 DOM；只有具体读了这个信号的文本节点和属性会被更新

```omni
component UserProfile {
    signal name: String = "Alice";
    signal age: i32 = 30;

    view {
        <div>
            <h2>{name}</h2>            <!-- name 变化时更新 -->
            <p>年龄：{age}</p>          <!-- age 变化时更新 -->
            <p>{name} 今年 {age} 岁</p> <!-- 任意一个变化时更新 -->
        </div>
    }
}
```

### 信号支持的类型

信号可以保存任何 OmniLang 类型：

```omni
signal count: i32 = 0;                 // 数字
signal name: String = "World";         // 字符串
signal active: bool = false;           // 布尔
signal items: Array<String> = [];      // 数组
signal config: Map<String, i32> = {};  // 映射
```

### 修改数组信号

```omni
component TodoList {
    signal items: Array<String> = [];
    signal input: String = "";

    fn add_item() {
        if input != "" {
            items.push(input);    // push() 触发重新渲染
            input = "";
        }
    }

    fn remove_last() {
        items.pop();              // pop() 触发重新渲染
    }

    view {
        <div>
            <input bind:value={input} placeholder="添加项目..." />
            <button @onPress={add_item}>添加</button>
            <button @onPress={remove_last}>删除最后一个</button>
            <ul>
                @for item in items {
                    <li>{item}</li>
                }
                @empty {
                    <li>暂无项目</li>
                }
            </ul>
        </div>
    }
}
```

---

## 计算属性（Computed）

**计算属性**是一种派生值，当依赖变化时自动重新计算：

```omni
component PriceCalculator {
    signal quantity: i32 = 1;
    signal unitPrice: f64 = 9.99;

    computed total: f64 = quantity * unitPrice;

    view {
        <div>
            <input type="number" bind:value={quantity} />
            <span>× ¥{unitPrice}</span>
            <h3>合计：¥{total}</h3>           <!-- 自动更新 -->
        </div>
    }
}
```

### 多依赖的计算属性

```omni
component Dashboard {
    signal firstName: String = "Alice";
    signal lastName: String = "Smith";
    signal score: i32 = 85;

    computed fullName: String = firstName + " " + lastName;
    computed grade: String = if score >= 60 { "及格" } else { "不及格" };
    computed summary: String = fullName + " — " + grade;

    view {
        <div>
            <h1>{fullName}</h1>
            <p>分数：{score}（{grade}）</p>
            <p>{summary}</p>
        </div>
    }
}
```

计算属性是**惰性**的——只有在被读取且依赖已发生变化时才重新计算。它们也是**缓存**的——在一次渲染中多次读取同一个计算属性会返回缓存值。

---

## 双向绑定：`bind:value`

`bind:value={signal}` 语法在表单元素和信号之间建立双向绑定：

```omni
component SearchBox {
    signal query: String = "";

    view {
        <div>
            <input bind:value={query} placeholder="搜索..." />
            @if query != "" {
                <p>搜索内容：{query}</p>
            } else {
                <p>输入关键字开始搜索</p>
            }
        </div>
    }
}
```

当用户输入时：
1. `query` 自动更新
2. 所有读取 `query` 的 UI 重新渲染
3. 如果程序修改了 `query`，输入框也会同步更新

### 支持的绑定

| 绑定 | 可用元素 |
|------|---------|
| `bind:value` | `<input>`、`<textarea>`、`<select>` |

---

## 副作用：`@effect`

`@effect` 生命周期钩子在指定信号变化后执行副作用：

```omni
component Logger {
    signal count: i32 = 0;

    @effect(count) {
        println("计数变为：" + count.to_string());
    }

    view {
        <div>
            <button @onPress={() => { count = count + 1; }}>
                计数：{count}
            </button>
        </div>
    }
}
```

副作用在 DOM 更新**之后**执行。适用于：
- 日志/埋点
- 持久化到 localStorage
- 触发外部 API 调用
- 与非 OmniView 库同步

---

## 生命周期钩子

### `@mount` — 挂载时执行一次

```omni
component Clock {
    signal time: String = "";

    @mount {
        time = "12:00:00";     // 设置初始值
        println("时钟已挂载！");
    }

    view {
        <div>
            <span>{time}</span>
        </div>
    }
}
```

`@mount` 在组件首次创建并插入 DOM 后执行一次。适用于：
- 获取初始数据
- 设置定时器或订阅
- 初始化第三方库

### `@unmount` — 卸载时执行一次

```omni
component Timer {
    signal elapsed: i32 = 0;

    @mount {
        // 启动定时器（概念示意——实际定时器见 omni/async）
    }

    @unmount {
        println("定时器已移除，清理中...");
        // 清理定时器、订阅等
    }

    view {
        <div>已过时间：{elapsed}秒</div>
    }
}
```

`@unmount` 在组件从 DOM 中移除时执行。用于资源清理。

---

## DOM 引用：`ref`

`ref` 关键字让你获取 DOM 元素的引用：

```omni
component FocusInput {
    ref inputEl: HtmlElement;

    @mount {
        // 挂载后自动聚焦输入框
        inputEl.focus();
    }

    view {
        <div>
            <input ref="inputEl" placeholder="我挂载后会自动聚焦" />
        </div>
    }
}
```

在模板中用 `ref="name"` 赋值，然后在组件函数中通过声明的 ref 变量访问。

---

## 自定义事件

组件可以发出自定义事件，父组件可以监听：

```omni
component Modal {
    event onClose: String;
    signal visible: bool = true;

    fn close() {
        visible = false;
        onClose("模态框已关闭");
    }

    view {
        @if visible {
            <div>
                <p>模态框内容</p>
                <button @onPress={close}>关闭</button>
            </div>
        }
    }
}

component App {
    fn handleClose(reason: String) {
        println("模态框关闭：" + reason);
    }

    view {
        <div>
            <Modal @onClose={handleClose} />
        </div>
    }
}
```

---

## 综合示例：实时计数器

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

## 响应式规则

1. **信号是唯一真相来源**——通过 `computed` 从信号派生一切
2. **计算属性中不要修改信号**——计算属性应该是纯函数
3. **副作用用于副作用**——不要用来更新信号，否则可能导致无限循环
4. **细粒度更新是自动的**——你不需要手动优化；系统精确追踪哪些 DOM 节点依赖哪些信号

---

## 下一步

- **[9. Store 与 Action](09-stores-and-actions.md)** — 多个组件共享的全局状态
- **[10. 样式与主题](10-styling-and-themes.md)** — 作用域 CSS 和设计令牌
- **[7. OmniView 组件](07-omniview-components.md)** — 回到组件基础
