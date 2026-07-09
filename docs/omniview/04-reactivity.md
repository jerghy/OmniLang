# 04 — 响应式系统

OmniView 的响应式系统基于 **信号（Signal）** 和 **计算属性（Computed）** 构建。当 signal 的值改变时，所有依赖它的 UI 和计算属性自动更新。

## signal（响应式信号）

### 声明信号

```omni
component MyComponent {
    signal count: i32 = 0;
    signal name: String = "Alice";
    signal isActive: bool = true;
    signal items: Array<String> = [];
    signal price: f64 = 9.99;
    signal user: User? = null;
}
```

- 所有 signal 必须在声明时提供初始值
- 类型注解是必需的
- 支持所有基础类型和自定义类型

### 读取信号

在 view 模板和 computed 中，直接使用 signal 名称即可读取：

```omni
view {
    <p>Count: {count}</p>           <!-- 直接读取 -->
    <p>Length: {name.len()}</p>     <!-- 方法调用 -->
    <p>Status: {isActive}</p>       <!-- 布尔值 -->
}
```

### 更新信号

使用**赋值**语法更新信号值：

```omni
// 直接赋值
count = count + 1;
name = "Bob";
isActive = !isActive;
price = price * 1.1;

// 赋值操作符
count += 5;
price -= 2.0;
```

对于 Array 类型，使用展开运算符创建新数组（**不可变模式**）：

```omni
// 追加元素
items = [...items, newItem];

// 移除元素
items = items.filter(|item| item.id != targetId);

// 清空
items = [];
```

## computed（计算属性）

### 简单形式

单个表达式的计算属性，从其他 signals 派生：

```omni
computed fullName: String = firstName + " " + lastName;
computed itemCount: i32 = items.len();
computed isOverBudget: bool = total > budget;
computed displayName: String = user ?? "Anonymous";
```

### 函数形式

含有多语句逻辑的计算属性（需要在 view 中用 `()` 调用）：

```omni
computed statusLabel() -> String {
    if count > 100 {
        return "Overflow";
    }
    if count < 0 {
        return "Invalid";
    }
    return "Normal (" + count.to_string() + ")";
}

// 在 view 中调用
view {
    <p>Status: {statusLabel()}</p>
}
```

### 惰性求值与缓存

- computed 使用**脏标记（dirty-checking）**机制
- 只有依赖的 signal 变化时才重新计算
- 多次读取同一个 computed 不会重复计算

## getter（属性访问器）

getter 是简化的 computed，用于访问派生值：

```omni
getter totalCount: i32 = items.len();
getter isEmpty: bool = items.len() == 0;
getter firstItem: String = if items.len() > 0 { items[0] } else { "" };
```

在 view 中直接使用 `{totalCount}` 读取 getter 的值。

## 数据绑定

### bind: 双向绑定

表单元素的值与 signal 双向同步：

```omni
signal name: String = "";
signal agreed: bool = false;
signal selected: String = "option2";

view {
    <!-- 文本输入 -->
    <input bind:value={name} placeholder="Your name" />

    <!-- 复选框 -->
    <label>
        <input type="checkbox" bind:checked={agreed} />
        I agree
    </label>

    <!-- 下拉选择 -->
    <select bind:value={selected}>
        <option value="option1">Option 1</option>
        <option value="option2">Option 2</option>
        <option value="option3">Option 3</option>
    </select>

    <!-- 显示当前值 -->
    <p>Name: {name}</p>
    <p>Agreed: {agreed}</p>
    <p>Selected: {selected}</p>
}
```

双向绑定的工作方式：
1. 用户输入 → 触发 input/change 事件 → 更新 signal
2. signal 变化 → 更新 DOM 元素的 value/checked 属性

绑定内置防抖和无限循环保护。

### 单行表达式插值

在模板文本中使用 `{expr}` 自动创建单向绑定：

```omni
<p>Count: {count}</p>
<p>Uppercase: {name.to_uppercase()}</p>
<span>Total: ${price * quantity}</span>
```

当任何引用的 signal 改变时，内容自动更新。

## 响应式更新模型

```
用户交互 / 异步操作
    ↓
signal 值改变 (count = 5)
    ↓
依赖追踪系统标记受影响的 computed
    ↓
computed 惰性重算 (doubled = count * 2)
    ↓
DOM 自动更新 (textContent 设置为新值)
```

### 依赖追踪

- 每个 computed 自动追踪它读取了哪些 signal
- 只有依赖的 signal 改变时才触发重新计算
- 不需要手动声明依赖列表

### 批量更新

同一事件处理器中对多个 signal 的修改会批量处理，UI 只更新一次：

```omni
fn resetAll() {
    count = 0;
    name = "";
    isActive = false;
    items = [];
    // UI 一次性更新所有变化
}
```

## 与 Store 的响应式集成

组件内的 `@store` 引用使 store 的 signal 和 getter 可直接使用：

```omni
component TodoApp {
    @store TodoStore;

    view {
        <p>Total: {TodoStore.totalCount}</p>   <!-- store getter -->
        <p>Filter: {TodoStore.filterStatus}</p> <!-- store signal -->
        <ul>
            @for item in TodoStore.items {
                <li>{item.text}</li>
            }
        </ul>
    }
}
```

## @effect 响应式副作用

当指定 signal 变化时执行副作用：

```omni
signal searchQuery: String = "";
signal results: Array<String> = [];

@effect(searchQuery) {
    // searchQuery 改变时自动执行
    if searchQuery.len() >= 3 {
        results = performSearch(searchQuery);
    }
}
```

## 响应式系统注意事项

1. **不可变更新**：对于 Array 和 struct，创建新值而不是修改现有值
2. **computed 纯度**：computed 不应有副作用（lint 规则会检测）
3. **避免循环依赖**：`computed a` 依赖 `signal b`，而 `computed b` 又依赖 `signal a`
4. **signal 赋值在 fn/action 中**：只能在 `fn`、`action`、生命周期钩子中修改 signal
5. **prop 不可变**：组件不能修改自己的 prop 值
