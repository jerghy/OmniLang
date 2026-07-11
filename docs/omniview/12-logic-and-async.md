# 12 — 逻辑与异步

OmniLang 提供完整的编程语言控制流、函数系统和异步支持。

## fn 函数

### 函数定义

```omni
// 基本函数
fn greet(name: String) -> String {
    return "Hello, " + name + "!";
}

// 无返回值函数
fn updateState(value: i32) {
    count = value;
}

// 多参数
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// 公共函数（可跨文件导入）
pub fn calculate(x: i32, y: i32) -> i32 {
    return x * y + x;
}
```

### 提前 return

```omni
pub fn validate_not_empty(fields: Array<String>) -> String {
    for field in fields {
        if field == "" {
            return "错误: 发现空字段";  // 提前返回
        }
    }
    return "通过";
}
```

## async fn 异步函数

使用 `async` 关键字声明异步函数，在函数体中使用 `await`：

```omni
// 异步函数
pub async fn fetch_demo(url: String) -> ApiResult {
    if url == "" {
        return Err { error: "url is empty" };
    }
    let raw = await http_get(url);
    return Ok { data: raw };
}

// 组件中的异步 action
component DataFetcher {
    signal data: String = "";
    signal loading: bool = false;
    signal error: String = "";

    action loadData() async {
        loading = true;
        error = "";
        let raw = await http_get("https://api.example.com/data");
        if raw == "" {
            error = "Empty response";
            loading = false;
            return;
        }
        data = raw;
        loading = false;
    }

    view {
        <div>
            <button @onPress={loadData} disabled={loading}>
                {if loading { "Loading..." } else { "Load Data" }}
            </button>
            @if error != "" {
                <p class: error-text>{error}</p>
            }
            <pre>{data}</pre>
        </div>
    }
}
```

`await` 只能在 `async fn` 或 `async action` 中使用。编译器会检查这个约束。

## 控制流

### if / else if / else

```omni
// 语句形式
if status == "loading" {
    showSpinner();
} else if status == "error" {
    showError();
} else {
    showContent();
}

// 表达式形式（在值上下文）
let label = if count > 0 { "Positive" } else { "Negative" };

// 在 computed 中
computed statusLabel: String = if isLoading { "Loading..."
                              } else if isError { "Error!"
                              } else { "Ready" };
```

### while 循环

```omni
let mut i = 0;
while i < items.len() {
    process(items[i]);
    i = i + 1;
}
```

### for..in 循环

```omni
// 遍历数组
for item in items {
    process(item);
}

// 遍历索引（使用 range）
for i in 0..items.len() {
    items[i] = transform(items[i]);
}

// 提前 break
for item in items {
    if item == target {
        break;
    }
}

// 跳过当前迭代
for item in items {
    if item == skipMe {
        continue;
    }
    process(item);
}
```

### break / continue

```omni
for item in items {
    if item == "" {
        continue;  // 跳过空值
    }
    if item == "stop" {
        break;     // 提前终止循环
    }
    process(item);
}
```

## 变量声明

### let（不可变绑定）

```omni
let name = "Alice";
let count: i32 = 42;
let items: Array<String> = [];
```

`let` 声明的变量默认不可重新赋值。

### let mut（可变变量）

```omni
let mut counter: i32 = 0;
counter = counter + 1;

let mut results: Array<String> = [];
results.push("new item");
```

## try / catch / finally

```omni
fn safe_parse(raw: String) -> String {
    try {
        let parsed = json_parse(raw);
        return json_stringify(parsed);
    } catch e {
        return "Parse error: " + e.to_string();
    } finally {
        // 总是执行（无论是否出错）
    }
}
```

## throw

```omni
fn validate(age: i32) {
    if age < 0 {
        throw "Age cannot be negative";
    }
    if age > 150 {
        throw "Age cannot exceed 150";
    }
}
```

## 输出语句

```omni
print "hello";          // 不换行输出
println "hello";        // 换行输出
println 42;
println "Count: " + count.to_string();
```

## 注释

```omni
// 单行注释

/*
   多行块注释
   （不支持嵌套）
*/

// ═══ 分隔线注释（常用风格） ═══
```

## 标准库（std）

OmniLang 通过 `import { ... } from "std/module"` 提供标准库功能。

### std/console（控制台）

```omni
import { console_log, console_warn, console_error } from "std/console";

fn debug() {
    console_log("Debug message");
    console_warn("Warning message");
    console_error("Error message");
}
```

### std/http（HTTP 请求）

```omni
import { http_get } from "std/http";

action fetchData() async {
    let raw = await http_get("https://api.example.com/data");
    if raw != "" {
        data = raw;
    }
}
```

### std/json（JSON 处理）

```omni
import { json_parse, json_stringify } from "std/json";

fn process(raw: String) -> String {
    let parsed = json_parse(raw);
    return json_stringify(parsed);
}
```

### std/array（数组工具）

```omni
import { array_filter } from "std/array";

action removeItem(id: i32) {
    items = array_filter(items, |item| item.id != id);
}
```

## 内置方法

### String 方法

```omni
let s = "hello";
let len = s.len();            // 长度
let upper = s.to_uppercase(); // 转大写
let lower = s.to_lowercase(); // 转小写
let text = count.to_string(); // 数字转字符串
```

### Array 方法

```omni
let arr = [1, 2, 3];
let len = arr.len();          // 长度
arr.push(4);                  // 追加
arr.pop();                    // 移除最后
let mapped = arr.map(|x| x * 2);           // 映射（管道闭包）
let filtered = arr.filter(|x| x > 1);      // 过滤（管道闭包）
let newArr = arr.concat([4, 5]);           // 连接
```

管道闭包也可以赋值给 `let` 变量后复用：

```omni
let isPositive = |x| x > 0;
let pos = arr.filter(isPositive);   // 复用同一个闭包
```

> 闭包语法详见 [11-表达式系统](11-expressions.md#闭包--lambda)。

### i32 / f64 方法

```omni
let s = count.to_string();    // 转字符串
let absVal = num.abs();       // 绝对值
```

## 常量

```omni
// 组件内常量
const MAX_ITEMS = 100;
const $API_URL = "https://api.example.com";

// 在信号初始值中使用
signal limit: i32 = MAX_ITEMS;
```

## 完整逻辑示例

```omni
// lib/validation.omni —— 纯逻辑文件
type ValidationResult = Valid { message: String }
                      | Invalid { errors: Array<String> }
                      | Pending;

pub fn validate_not_empty(fields: Array<String>) -> String {
    for field in fields {
        if field == "" {
            return "错误: 发现空字段";
        }
    }
    return "通过";
}

pub fn validate_length(values: Array<String>, min_len: i32) -> String {
    for value in values {
        if value.len() < min_len {
            return "错误: 长度不足";
        }
    }
    return "通过";
}

pub fn validate_all(items: Array<String>, min_len: i32) -> Array<String> {
    let mut errors: Array<String> = [];
    for item in items {
        if item == "" {
            errors.push("发现空字符串");
        }
        if item.len() < min_len {
            errors.push("长度不足: " + item);
        }
    }
    return errors;
}

pub fn format_result(result: ValidationResult) -> String {
    match result {
        Valid { message } => { return "通过: " + message; },
        Invalid { errors } => { return "失败: " + errors.len().to_string() + " 个错误"; },
        Pending => { return "验证中..."; }
    }
}
```

## 执行模型

1. **同步优先**：默认情况下所有函数都是同步的
2. **async 显式标出**：异步边界通过 `async fn` 和 `await` 显式表示
3. **不可变默认**：变量默认不可变（`let`），需要可变时使用 `let mut`
4. **提前 return**：支持在任何位置提前返回
5. **穷尽 match**：match 表达式必须处理所有 tagged union 变体
