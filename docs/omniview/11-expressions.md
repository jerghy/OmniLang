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

```omni
// 单参数闭包
|x| x * 2

// 多参数闭包
|x, y| x + y

// 用于 Array 方法
items = items.filter(|item| item.active);
let doubled = numbers.map(|n| n * 2);

// 在事件处理器中
<button @onPress={() => { count = count + 1; }}>Click</button>

// 作为函数参数
fn process(items: Array<String>, filter: fn(String) -> bool) -> Array<String> {
    let mut result: Array<String> = [];
    for item in items {
        if filter(item) {
            result.push(item);
        }
    }
    return result;
}
```

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
| computed 值 | 所有表达式（包括块表达式、match、闭包） |
| view 插值 `{...}` | 所有表达式 |
| 事件处理器参数 | 函数引用、内联闭包、store action |
| 条件类 `class: {k: expr}` | 返回 bool 的表达式 |
| 条件渲染 `@if expr` | 返回 bool 的表达式 |
| 列表渲染 `@for item in expr` | 返回 Array 的表达式 |
