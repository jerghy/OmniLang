# 04 — 函数与控制流

## fn 函数

### 基本定义

```omni
// 带返回值的函数
fn greet(name: String) -> String {
    return "Hello, " + name + "!";
}

// 无返回值
fn update(value: i32) {
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

函数可以修改 signal 的值、调用 store action、被事件处理器引用。

### 提前 return

```omni
fn validate(fields: Array<String>) -> String {
    for field in fields {
        if field == "" { return "empty field"; }
    }
    return "ok";
}
```

## async fn 异步函数

```omni
// 异步函数声明
pub async fn fetchData(url: String) -> ApiResult {
    let raw = await http_get(url);
    return Ok { data: raw };
}

// 组件中的异步逻辑
component DataFetcher {
    signal data: String = "";
    signal loading: bool = false;

    fn loadData() async {
        loading = true;
        let raw = await http_get("https://api.example.com/data");
        data = raw;
        loading = false;
    }

    view {
        <button @onPress={loadData} disabled={loading}>
            {if loading { "Loading..." } else { "Load" }}
        </button>
    }
}
```

`await` 只能在 `async fn` 或 `async action` 中使用。

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

// 表达式形式
let label = if count > 0 { "Positive" } else { "Negative" };
```

### while 循环

```omni
let mut i = 0;
while i < items.len() {
    process(items.get(i));
    i = i + 1;
}
```

### for..in 循环

```omni
// 遍历数组
for item in items {
    process(item);
}

// 提前退出
for item in items {
    if item == target { break; }
    if item == "" { continue; }
}
```

## 变量声明

### let（不可变）

```omni
let name = "Alice";
let count: i32 = 42;
```

`let` 声明的变量默认不可重新赋值。

### let mut（可变）

```omni
let mut counter: i32 = 0;
counter = counter + 1;

let mut results: Array<String> = [];
results.push("new item");
```

> **注意**：Signal 使用 `=` 赋值语法，不需要 `let mut`。`let`/`let mut` 用于函数内部的局部变量。

## 错误处理

### try / catch / finally

```omni
fn safeParse(raw: String) -> String {
    try {
        let parsed = json_parse(raw);
        return json_stringify(parsed);
    } catch e {
        return "Parse error: " + e.to_string();
    } finally {
        // 总是执行
    }
}
```

### throw

```omni
fn validate(age: i32) {
    if age < 0 { throw "Age cannot be negative"; }
    if age > 150 { throw "Age exceeds maximum"; }
}
```

## 输出语句

```omni
print "hello";                        // 不换行
println "Count: " + count.to_string(); // 换行
```

编译为浏览器 `console.log`。

## 常量

```omni
component MyComponent {
    const MAX_ITEMS = 100;
    const API_URL = "https://api.example.com";

    signal limit: i32 = MAX_ITEMS;    // 常量可用于初始值
}
```

常量值在编译时确定，不可在运行时修改。

## 注释

```omni
// 单行注释

/*
   多行块注释
*/

// ═══ 节分隔注释 ═══
```

## 执行模型

1. **同步优先**：默认所有函数同步执行
2. **异步显式标注**：`async fn` + `await` 显式表示异步边界
3. **不可变默认**：`let` 不可重新赋值，需要可变时用 `let mut`
4. **signal 赋值**：在 `fn`、生命周期钩子、store `action` 中修改 signal
5. **signal 是组件状态**：UI 自动响应 signal 变化；局部变量不触发 UI 更新
