# 02 — 类型与值

OmniLang 是静态类型语言。所有类型在编译时检查，类型不匹配会导致编译错误。

## 基础类型

### 整数

| 类型 | 范围 | 用例 |
|------|------|------|
| `i32` | -2³¹ ~ 2³¹-1 | **默认整数**，计数、索引 |
| `i64` | -2⁶³ ~ 2⁶³-1 | 时间戳、大数值 |
| `i8` / `i16` | 较小范围 | 节省空间 |
| `u8` ~ `u64` | 无符号 | 非负数场景 |

```omni
signal count: i32 = 0;
signal timestamp: i64 = 0;
signal attempts: u8 = 3;
```

### 浮点数

| 类型 | 说明 |
|------|------|
| `f64` | **默认浮点**，双精度 |
| `f32` | 单精度（需显式后缀 `f32`） |

```omni
signal price: f64 = 9.99;
signal ratio: f64 = 0.5;
let pi: f32 = 3.14f32;
```

### 其他基础类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `bool` | 布尔值 | `true`, `false` |
| `String` | UTF-8 字符串 | `"hello"`, `'hello'` |
| `Void` | 无返回值 | 函数返回类型 |

```omni
signal isActive: bool = true;
signal name: String = "Alice";
signal message: String = 'single-quoted works too';
```

### null

`null` 表示空值，只能赋给可选类型（`Type?`）：

```omni
signal user: String? = null;    // ✓ 可选类型可以为 null
signal count: i32 = null;       // ✗ 编译错误！i32 不接受 null
```

## 复合类型

### Array 数组

两种等价写法：

```omni
signal items: Array<String> = [];
signal items: String[] = [];           // 语法糖

signal numbers: Array<i32> = [1, 2, 3];
signal points: Array<Point> = [];
```

### Map 对象

```omni
signal config: Map<String, i32> = { "a": 1, "b": 2 };
signal empty: Map<String, bool> = {};
```

### 可选类型 `Type?`

使用 `?` 后缀表示可为 `null` 的值：

```omni
signal user: User? = null;             // 可为 null
signal name: String? = "Alice";        // 初始有值，但可以赋 null
signal result: Todo? = null;
```

安全访问可选值用 `??` 和 `?.`（见 [03-表达式与运算符](03-expressions-and-operators.md)）。

## struct 结构体

### 定义

```omni
// 结构等价结构体（默认）
pub struct Point {
    x: i32,
    y: i32,
}

pub struct User {
    name: String,
    age: i32,
    active: bool,
}

// 嵌套
pub struct Todo {
    id: i32,
    text: String,
    done: bool,
}
```

### 名义结构体

`struct nominal` 创建类型不兼容的结构体——即使字段相同也不能互换：

```omni
struct nominal UserId { value: i32 }
struct nominal OrderId { value: i32 }

// UserId 和 OrderId 不能互相赋值，编译器会报错
```

### 构造

```omni
let p = Point { x: 10, y: 20 };
let user = User { name: "Alice", age: 30, active: true };

// 作为 signal 初始值
signal current: User = User { name: "Bob", age: 25, active: false };
```

### 字段访问与展开更新

```omni
let name = user.name;                  // 字段访问

// 不可变更新（创建新值）
let updated = { ...user, age: 31 };    // 展开 + 覆盖
```

## Tagged Union（代数数据类型）

用 `type` 定义有标签的联合体，每个变体可有不同的负载字段：

```omni
// 有负载和无负载变体混合
type ApiResult = Ok { data: String }
               | Err { error: String }
               | Loading

type ValidationResult = Valid { message: String }
                      | Invalid { errors: Array<String> }
                      | Pending

// 多字段变体
type Shape = Circle { x: f64, y: f64, radius: f64 }
           | Rectangle { x: f64, y: f64, width: f64, height: f64 }
```

> 注：联合体声明以最后一个变体结束，**不加分号**（单类型别名如
> `type Score = i32;` 仍以 `;` 结尾）。

### 构造与模式匹配

```omni
fn handle(result: ApiResult) -> String {
    match result {
        Ok { data } => { return "Success: " + data; },
        Err { error } => { return "Error: " + error; },
        Loading => { return "Loading..."; }
    }
}

// 构造变体值（局部变量）
fn construct() {
    let ok = Ok { data: "hello" };
    let loading = Loading;    // 无负载变体不需要 {}
}
```

`match` 必须穷尽所有变体，编译器会检查完整性。

## 类型别名

```omni
type Score = i32;
type Percentage = f64;
type UserId = String;

pub type Percentage = f64;     // 导出类型别名
```

## 函数类型

函数类型使用 `fn(Params) -> Return` 语法：

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
```

闭包变量也可以标注函数类型：

```omni
let bonus: fn(i32) -> i32 = |s| s + 5;
let compare: fn(String, String) -> i32 = |a, b| a.len() - b.len();
```

## 类型注解位置

```omni
// 局部变量声明
fn demo() {
    let name: String = "hello";
    let bonus: fn(i32) -> i32 = |s| s + 5;
}

// 函数参数与返回
fn greet(name: String, age: i32) -> String {
    return name + " " + age.to_string();
}

component AnnotationDemo {
    // signal
    signal price: f64 = 0.0;
    signal quantity: f64 = 1.0;

    // computed
    computed total: f64 = price * quantity;

    // Array / Map 泛型
    signal names: Array<String> = [];
    signal mapping: Map<String, i32> = {};

    // 函数类型：fn(Params) -> Return
    fn combine(f: fn(i32, i32) -> bool) {
        let r = f(1, 2);
        if r { print("matched"); }
    }

    view {
        <p>{total}</p>
    }
}
```
