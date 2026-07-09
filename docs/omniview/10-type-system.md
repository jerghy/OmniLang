# 10 — 类型系统

OmniLang 的类型系统支持基础类型、结构体、tagged union（代数数据类型）、可选类型和类型别名。

## 基础类型

| 类型 | 说明 | 示例值 |
|------|------|--------|
| `i32` | 32 位有符号整数 | `0`, `42`, `-1` |
| `f64` | 64 位浮点数 | `3.14`, `0.0`, `-2.5` |
| `bool` | 布尔值 | `true`, `false` |
| `String` | UTF-8 字符串 | `"hello"`, `'hello'` |
| `null` | 空值 | `null` |

## 复合类型

### Array 数组

```omni
signal items: Array<String> = [];
signal numbers: Array<i32> = [1, 2, 3];
signal points: Array<Point> = [];
```

`Type[]` 是 `Array<Type>` 的语法糖：

```omni
// 等价写法
signal items: String[] = [];
signal items: Array<String> = [];
```

### Map 对象

```omni
signal config: Map<String, i32> = { "a": 1, "b": 2 };
signal emptyMap: Map<String, bool> = {};
```

### 可选类型（Optional）

使用 `?` 后缀表示可为 `null` 的值：

```omni
signal user: String? = null;       // 可为 null 的字符串
signal result: Todo? = null;       // 可为 null 的结构体
signal name: String? = "Alice";    // 初始有值
```

## struct 结构体

### 定义结构体

```omni
// 结构等价（默认）
pub struct Point {
    x: i32,
    y: i32,
}

pub struct User {
    name: String,
    age: i32,
    active: bool,
}

// 嵌套结构体
pub struct Todo {
    id: i32,
    text: String,
    done: bool,
}
```

### 名义结构体（Nominal Struct）

使用 `nominal` 关键字创建类型不兼容的结构体：

```omni
struct nominal UserId {
    value: i32,
}

struct nominal OrderId {
    value: i32,
}
// UserId 和 OrderId 不能互相赋值，即使字段相同
```

### 构造结构体

```omni
// 结构体字面量
let p = Point { x: 10, y: 20 };
let user = User { name: "Alice", age: 30, active: true };

// 作为 signal 初始值
signal currentUser: User = User { name: "Bob", age: 25, active: false };

// 在数组中
signal items: Todo[] = [
    Todo { id: 0, text: "Buy milk", done: false },
    Todo { id: 1, text: "Walk dog", done: true },
];
```

### 访问结构体字段

```omni
let name = user.name;
let age = user.age;
```

### 展开操作（Spread）

```omni
// 基于现有结构体创建新值（不可变更新）
let updatedUser = { ...user, age: 31 };

// 在数组中
items = items.map(|item|
    if item.id == targetId {
        { ...item, done: !item.done }
    } else {
        item
    }
);
```

## Tagged Union（代数数据类型）

Tagged union 是一种类型安全的联合体，每个变体可有不同的负载字段。

### 定义 Tagged Union

```omni
// 有负载字段的变体
type ApiResult = Ok { data: String } | Err { error: String } | Loading

// 混合有负载和无负载变体
type ValidationResult = Valid { message: String }
                      | Invalid { errors: Array<String> }
                      | Pending

// 多字段变体
type Shape = Circle { x: f64, y: f64, radius: f64 }
           | Rectangle { x: f64, y: f64, width: f64, height: f64 }
           | Empty
```

### 构造 Tagged Union 值

```omni
fn ok_result(data: String) -> ApiResult {
    return Ok { data: data };
}

fn err_result(error: String) -> ApiResult {
    return Err { error: error };
}

fn loading() -> ApiResult {
    return Loading;    // 无负载变体，不需要 {}
}
```

### match 模式匹配

```omni
fn handle_result(result: ApiResult) -> String {
    match result {
        Ok { data } => { return "Success: " + data; },
        Err { error } => { return "Error: " + error; },
        Loading => { return "Loading..."; }
    }
}

fn format_result(result: ValidationResult) -> String {
    match result {
        Valid { message } => {
            return "通过: " + message;
        },
        Invalid { errors } => {
            return "失败: 发现 " + errors.len().to_string() + " 个错误";
        },
        Pending => {
            return "验证中...";
        }
    }
}
```

match 表达式要求**穷尽（exhaustive）**所有变体，编译器会检查完整性。

### 使用变体构造值作为函数返回值

```omni
pub async fn fetch_demo(url: String) -> ApiResult {
    if url == "" {
        return Err { error: "url is empty" };
    }
    let raw = await http_get(url);
    return Ok { data: raw };
}
```

## 类型别名（Type Alias）

```omni
// 基础类型别名
type Score = i32;
type Percentage = f64;
type UserId = String;

// 导出类型别名
pub type Percentage = f64;

// 泛型类型别名
type StringList = Array<String>;
type ResultList = Array<ApiResult>;
```

## 函数类型

函数可以作为参数传递（闭包语法）：

```omni
fn process_items(items: Array<String>, filter: fn(String) -> bool) -> Array<String> {
    let mut result: Array<String> = [];
    for item in items {
        if filter(item) {
            result.push(item);
        }
    }
    return result;
}

// 调用
let filtered = process_items(items, |item| item.len() > 3);
```

## 类型注解语法

```omni
// 变量声明
let name: String = "hello";
let count: i32 = 42;
let maybeUser: User? = null;

// 函数参数
fn greet(name: String, age: i32) -> String { ... }

// 返回类型
fn calculate(x: i32) -> i32 { ... }

// Signal 声明
signal price: f64 = 0.0;
signal data: Array<Todo> = [];

// Computed 声明
computed total: f64 = price * quantity;

// Tagged union
type MyUnion = A { field: Type } | B { field: Type }

// 泛型类型
Array<String>
Map<String, i32>
fn(i32, i32) -> bool
```

## 类型检查

OmniLang 是**静态类型**语言：
- 所有类型在编译时检查
- 类型不匹配会导致编译错误
- 可选类型需要通过 `??` 或 `?.` 安全访问

## 类型系统最佳实践

1. **优先使用 tagged union**：对于多状态返回值，tagged union 比字符串/整数标记更安全
2. **公开类型用 pub**：导出给其他模块使用的 struct 和 type 必须标记 `pub`
3. **区分名义结构体**：对于 ID 类型或不应互换的类型，使用 `nominal struct`
4. **可选类型显式标注**：使用 `Type?` 而不是隐式依赖 `null`
5. **match 穷尽**：总是覆盖 tagged union 的所有变体，编译器会帮助检查
