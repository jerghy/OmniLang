# 03 — 表达式与运算符

## 字面量

```omni
// 整数
42
0xFF         // 十六进制
0b1010       // 二进制

// 浮点数
3.14
1.5e3        // 科学计数法

// 字符串
"hello"
'single quoted'
"line 1\nline 2"

// 布尔和空
true
false
null
```

## 运算符（按优先级从低到高）

| 优先级 | 运算符 | 结合性 | 示例 |
|--------|--------|--------|------|
| 最低 | `=` `+=` `-=` `*=` `/=` `%=` | 右 | `count += 1` |
| | `? :` | 右 | `a > 0 ? "yes" : "no"` |
| | `\|\|` | 左 | `a \|\| b` |
| | `??` | 左 | `a ?? "default"` |
| | `&&` | 左 | `a && b` |
| | `is` | 左 | `value is String` |
| | `==` `!=` `<` `>` `<=` `>=` | 左 | `a == b` |
| | `+` `-` | 左 | `a + b` |
| | `*` `/` `%` | 左 | `a * b` |
| 最高 | `.field` `?.field` `[idx]` `()` | 左 | `obj.field` |

### 赋值运算符

```omni
count = 10;
price += 5.0;          // price = price + 5.0
count -= 1;            // count = count - 1
scale *= 2;            // scale = scale * 2
```

### 三元运算符

```omni
let label = count > 10 ? "High" : "Low";

// 链式
computed grade: String = score >= 90 ? "A"
                       : score >= 80 ? "B"
                       : score >= 70 ? "C"
                       : "F";
```

### 空值合并 `??`

左边为 `null` 时使用右边的默认值：

```omni
let name = input ?? "Anonymous";
computed display: String = user ?? "Guest";
let timeout = configValue ?? 3000;
```

### 类型检查 `is`

```omni
if value is String {
    // value 类型收窄为 String
}
if result is Ok {
    // result 类型收窄为 Ok 变体
}
```

### 可选链 `?.`

安全访问可能为 `null` 的值——如果中间任何一环为 `null`，整个表达式返回 `null`：

```omni
let len = user?.name?.len();           // user 或 name 为 null → 返回 null
let count = obj?.items?.len() ?? 0;    // 配合 ?? 提供默认值
```

### 展开运算符 `...`

```omni
// 数组展开
let extended = [...original, newItem];

// 结构体展开（不可变更新）
let updated = { ...user, name: "Bob" };

// 在 store action 中
action toggle(id: i32) {
    items = items.map(|item|
        if item.id == id { { ...item, done: !item.done } }
        else { item }
    );
}
```

## 特殊表达式

### 块表达式

在值上下文中执行多条语句，最后一行是块的值（不需要 `return`）：

```omni
computed result: i32 = {
    let tmp = a + b;
    let doubled = tmp * 2;
    doubled    // 最后一行是块的值
};

// 在事件处理器中
<button @onPress={() => {
    let old = count;
    count = count + 1;
    log = "from " + old.to_string();
}}>

// 字符串拼接结果
computed label: String = {
    let a2 = a * 2;
    "result: " + a2.to_string()
};
```

### if 表达式

`if` 既可以作为语句也可以作为返回值：

```omni
// 在值上下文中
let status = if score >= 60 { "pass" } else { "fail" };

// 在 view 插值中
<p>{if isOpen { "Open" } else { "Closed" }}</p>

// 在 computed 中
computed label: String = if count > 0 { "Positive" } else { "Zero" };
```

### match 表达式

对 tagged union 进行穷尽模式匹配：

```omni
match result {
    Ok { data } => { return "Success: " + data; },
    Err { error } => { return "Error: " + error; },
    Loading => { return "Loading..."; }
}
```

模式类型：
- `VariantName { field1, field2 }` — 匹配有负载变体，解构字段
- `VariantName` — 匹配无负载变体
- `_` — 通配符（匹配任何值）

### 模板字符串

```omni
let msg = `Hello, ${name}! You have ${count} messages.`;
```

## 闭包 / Lambda

OmniLang 有两种闭包语法，用于不同场景：

| 语法 | 使用场景 | 示例 |
|------|----------|------|
| `\|param\| expr` | `fn`/`action`/`computed` 中的函数逻辑 | `\|x\| x * 2` |
| `() => { }` | view 模板中的事件处理器 | `() => { count += 1 }` |

### 管道闭包 `|param| expr`

用于函数逻辑上下文——传给高阶函数、赋值给变量：

```omni
// 单参数（类型自动推断）
|x| x * 2

// 多参数
|x, y| x + y

// 多语句体
|x| {
    let doubled = x * 2;
    doubled + 1
}

// 赋值给 let 变量
let bonus = |s| s + 5;
let result = bonus(10);   // 15

// 传给 std/array 函数
let active = array_filter(users, |u| u.status == "active");
let names = array_map(users, |u| u.name);
let sorted = array_sort(items, |a, b| a.name);

// 在 store action 中
action remove(id: i32) {
    items = array_filter(items, |item| item.id != id);
}
```

### 箭头闭包 `() => { }`

**仅用于 view 模板中的事件处理器**：

```omni
// 单行
<button @onPress={() => { count = count + 1; }}>+1</button>

// 多行
<button @onPress={() => {
    count = count + 1;
    log = "clicked";
}}>Click</button>

// 带参数（自定义事件）
<TodoList @onAdd={(text) => { TodoStore.add(text); }} />
```

> `|param| expr` 和 `() => { }` **不能互换**。在 `fn` 中使用 `() => {}` 或 view 中使用 `|x| x` 都会导致编译错误。

## 运算符使用场景

| 场景 | 可用表达式 |
|------|-----------|
| signal 初始值 | 字面量、数组、map、结构体、三元、`??` |
| computed | 所有表达式（块表达式、match、管道闭包） |
| view 插值 `{...}` | 所有表达式（if 表达式、三元、方法调用） |
| 事件处理器 | 函数引用 `{fn}`、箭头闭包 `{() => {}}`、store action `{S.action()}` |
| 条件类 | 返回 bool 的表达式：`class: { active: isActive }` |
| `@if` 条件 | 返回 bool 的表达式 |
| `@for` 源 | 返回 Array 的表达式 |
