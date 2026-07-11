# 11 — 表达式系统

OmniLang 支持丰富的表达式语法，用于计算、条件判断和值操作。

## 字面量（Literals）

```omni
// 整数
42
0xFF         // 十六进制
0o77         // 八进制
0b1010       // 二进制

// 浮点数
3.14
1.5e3        // 科学计数法
1e-2

// 字符串（单引号或双引号，转义序列相同）
"hello world"
'single quoted'
"line 1\nline 2"
"tab\tseparated"
"unicode: \u{263A}"

// 布尔和空
true
false
null
```

## 运算符（按优先级从低到高）

### 赋值运算符（右结合）

```omni
=    +=    -=    *=    /=    %=
```

```omni
count = 10;
price += 5.0;       // 等价于 price = price + 5.0
count -= 1;         // 等价于 count = count - 1
scale *= 2;         // 等价于 scale = scale * 2
```

### 三元运算符 `? :`（右结合）

```omni
let label = count > 10 ? "High" : "Low";

// 在 view 中直接使用
<p>{isActive ? "Active" : "Inactive"}</p>

// 链式三元
computed grade: String = score >= 90 ? "A"
                       : score >= 80 ? "B"
                       : score >= 70 ? "C"
                       : score >= 60 ? "D"
                       : "F";
```

### 逻辑 OR `||`（左结合）

```omni
let displayName = inputName || "Default";
```

### 空值合并 `??`（左结合）

```omni
// 当左边为 null 时使用右边的值
computed displayName: String = user ?? "Guest";
let name = inputValue ?? "Anonymous";
```

### 逻辑 AND `&&`（左结合）

```omni
let isReady = hasData && isConnected;
let canSubmit = formValid && !isSubmitting;
```

### 类型检查 `is`（左结合）

```omni
if value is String {
    // value 被收窄为 String 类型
}
if result is Ok {
    // result 被收窄为 Ok 变体
}
```

### 比较运算符（左结合）

```omni
==    !=    <    >    <=    >=
```

```omni
if count == 0 { ... }
if name != "" { ... }
if age >= 18 { ... }
```

### 算术运算符（左结合）

```omni
+    -    *    /    %
```

```omni
let sum = a + b;
let area = width * height;
let remainder = count % 2;
```

### 后缀运算符

```omni
.field          // 字段访问
?.field         // 可选链字段访问
()              // 函数/方法调用
[index]         // 索引访问
```

## 特殊表达式

### 块表达式（Block Expression）

在值上下文中执行多条语句，最后一行是块的值：

```omni
// 在 computed 中使用块表达式
computed calculation: String = {
    let a2 = a * 2;
    let b2 = b * 3;
    "a×2=" + a2.to_string() + ", b×3=" + b2.to_string()
};

// 用于数值计算
computed total: i32 = {
    let tmp = a + b;
    let doubled = tmp * 2;
    doubled    // 最后一行是块的值（不需要 return）
};

// 在事件处理器中使用
<button @onPress={() => {
    let temp = count;
    count = count + 1;
    log = "Updated from " + temp.to_string();
}}>
```

### if 表达式

`if` 既可以作为语句也可以作为表达式（返回值）：

```omni
// if 作为表达式
let status = if score >= 60 { "pass" } else { "fail" };

// 在 view 中
<p>{if isOpen { "Open" } else { "Closed" }}</p>

// 在 computed 中
computed label: String = if count > 0 { "Positive" } else { "Zero" };
```

### 可选链 `?.`

安全访问可能为 `null` 的值：

```omni
// 如果 user 为 null，整个表达式返回 null（不抛错）
let len = user?.name?.len();

// 配合 ?? 提供默认值
computed nameLen: i32 = user?.len() ?? 0;
```

### 空值合并 `??`

```omni
computed displayName: String = inputName ?? "Anonymous";
let backoff = timeout ?? 3000;
```

### match 表达式

对 tagged union 进行穷尽模式匹配：

```omni
fn format(result: ApiResult) -> String {
    match result {
        Ok { data } => { return "Success: " + data; },
        Err { error } => { return "Error: " + error; },
        Loading => { return "Loading..."; }
    }
}
```

match 的模式包括：
- `VariantName { field1, field2 }`：匹配有负载的变体
- `VariantName`：匹配无负载的变体
- `_`：通配符（匹配任何值）

### 闭包 / Lambda

OmniLang 有两种闭包语法，分别用于不同场景：

| 语法 | 形式 | 使用场景 |
|------|------|----------|
| `\|param\| expr` | Rust 风格管道闭包 | 函数逻辑中（`fn`、`action`、`computed`、`let`） |
| `() => { }` | JS 箭头风格内联闭包 | view 模板的事件处理器 |

#### Rust 风格管道闭包 `|param| expr`

用于函数逻辑上下文，可传给高阶函数、赋值给变量、或在 computed 中使用：

```omni
// 单参数闭包（类型自动推断）
|x| x * 2

// 多参数闭包
|x, y| x + y

// 带类型标注的完整形式
|x: i32| -> i32 { x * 2 }

// 多语句闭包体（用 { } 包裹）
|x| {
    let doubled = x * 2;
    doubled + 1    // 最后一行是返回值
}
```

**赋值给 `let` 变量**（像函数一样调用）：

```omni
fn getBonusTotal() -> i32 {
    let bonus = |s| s + 5;          // 闭包赋值给变量
    let mut total = 0;
    let mut i = 0;
    while i < rows.len() {
        total = total + bonus(rows.get(i).score);  // 通过变量名调用
        i = i + 1;
    }
    return total;
}
```

**传给 `std/array` 高阶函数**：

```omni
// 过滤 —— 单参数谓词闭包
let active = array_filter(users, |u| u.status == "active");

// 映射 —— 单参数转换闭包
let names = array_map(users, |u| u.name);

// 排序 —— 双参数比较闭包
let sorted = array_sort(items, |a, b| a.name);

// 查找 —— 单参数谓词闭包
let found = array_find(items, |item| item.id == targetId);
```

**在 computed 中使用**：

```omni
signal searchText: String = "";
signal demoItems: Array<String> = ["React", "Vue", "Svelte"];

computed filtered: Array<String> = array_filter(
    demoItems,
    |f| if searchText == "" { true } else { f == searchText }
);
```

**在 store action 中使用**：

```omni
action remove(id: i32) {
    items = array_filter(items, |item| item.id != id);
    lastAction = "removed";
}

action toggle(id: i32) {
    items = array_map(items, |item|
        if item.id == id {
            { ...item, done: !item.done }
        } else {
            item
        }
    );
}
```

**作为函数参数**（高阶函数）：

```omni
fn process(items: Array<String>, filter: fn(String) -> bool) -> Array<String> {
    let mut result: Array<String> = [];
    for item in items {
        if filter(item) {
            result.push(item);
        }
    }
    return result;
}

// 调用时传入闭包
let filtered = process(items, |item| item.len() > 3);
```

#### JS 箭头风格内联闭包 `() => { }`

**仅用于 view 模板中的事件处理器**：

```omni
view {
    // 单行内联
    <button @onPress={() => { count = count + 1; }}>+1</button>

    // 多行内联
    <button @onPress={() => {
        count = count + 1;
        log = "clicked at " + datetime_now().to_string();
    }}>Click</button>

    // 调用 Store action
    <button @onPress={() => { TodoStore.clear(); }}>Clear All</button>

    // 带参数的内联闭包（自定义事件）
    <TodoList @onAdd={(text) => { TodoStore.add(text); }} />
}
```

> **注意**：`() => { }` 语法**只能**在 view 模板的事件属性中使用，不能在 `fn`/`action`/`computed` 中使用。在函数逻辑中请使用 `|param| expr` 管道闭包。

#### 内置 Array 方法中的闭包

除了 `std/array` 模块函数，Array 的内置方法也接受管道闭包：

```omni
// 内置 .filter() 和 .map()
let mapped = arr.map(|x| x * 2);
let filtered = arr.filter(|x| x > 1);

// 与 getter 配合
getter activeCount: i32 = items.filter(|item| !item.done).len();
```

#### 语法区分速查

| | `\|x\| expr` 管道闭包 | `() => { }` 箭头闭包 |
|---|---|---|
| 使用位置 | `fn` 体、`action`、`computed`、`let` | view 模板的事件属性 |
| 典型场景 | 传给 `array_filter/map/sort`、赋值给变量 | `@onPress`、`@onChange` 等事件 |
| 参数 | 有参数（单参省略类型） | 通常无参，自定义事件可有参 |
| 类型标注 | 可选：`\|x: i32\| -> i32 { ... }` | 不支持类型标注 |
| 赋值给变量 | `let f = \|x\| x * 2;` | 不支持 |

### await 表达式

```omni
let response = await http_get(url);
let parsed = await json_parse(raw);
```

`await` 只能在 `async fn` 或 `async action` 中使用。

### 模板字符串

```omni
let msg = `Hello, ${name}! You have ${count} messages.`;
```

### 数组字面量

```omni
let numbers = [1, 2, 3, 4, 5];
let strings = ["a", "b", "c"];
let mixed = [item1, item2];
let empty: Array<i32> = [];
```

### Map 字面量

```omni
let config = { "key": "value", "count": 42 };
let shorthand = { name, age };   // 等价于 { name: name, age: age }
```

### 展开运算符 `...`

```omni
// 数组展开
let extended = [...original, newItem];

// 结构体展开
let updated = { ...user, name: "Bob" };

// 函数调用展开
fn process(...args);
```

## 运算符优先级速查表

| 优先级 | 运算符 | 结合性 |
|--------|--------|--------|
| 最低 | `=` `+=` `-=` `*=` `/=` `%=` | 右 |
| | `? :` | 右 |
| | `\|\|` | 左 |
| | `??` | 左 |
| | `&&` | 左 |
| | `is` | 左 |
| | `==` `!=` `<` `>` `<=` `>=` | 左 |
| | `+` `-` | 左 |
| 最高 | `*` `/` `%` | 左 |
| (后缀) | `.field` `?.field` `()` `[index]` | 左 |

## 表达式使用场景

| 场景 | 可用表达式 |
|------|-----------|
| signal 初始值 | 字面量、数组、map、结构体、三元、`??`、块表达式 |
| computed 值 | 所有表达式（包括块表达式、match、管道闭包 `\|x\| expr`） |
| view 插值 `{...}` | 所有表达式（含 if 表达式、三元、方法调用） |
| 事件处理器参数 | 函数引用 `{fn}`、内联闭包 `{() => {}}`、store action `{Store.action()}` |
| 条件类 `class: {k: expr}` | 返回 bool 的表达式 |
| 条件渲染 `@if expr` | 返回 bool 的表达式 |
| 列表渲染 `@for item in expr` | 返回 Array 的表达式 |
