# 2. 语言基础

> **目标**：掌握 OmniLang 的核心构建块 —— 变量、类型、函数、控制流和集合。

本章所有示例都是**纯逻辑**代码，编译到 Python、JavaScript、Rust 和 Dart 后行为一致。跟着一起做：

```bash
omnic create learn-basics --type logic
cd learn-basics
```

---

## 注释

```omni
// 单行注释 —— 从 // 到行尾
/* 块注释
   可以跨越多行 */
```

块注释**不能**嵌套。注释在编译时被移除，对输出不可见。

---

## 变量

用 `let` 声明变量。**默认不可变**——需要使用 `let mut` 才能修改变量的值。

```omni
fn main() {
    // 不可变变量（默认）
    let x: i32 = 10;
    println(x.to_string());

    // 可变变量
    let mut y: f64 = 3.14;
    y = y + 1.0;          // OK — y 是可变的
    println(y.to_string());

    // 编译器可以推断类型时，可以省略类型注解
    let name = "OmniLang";   // 推断为 String
    println(name);
}
```

> **语言特性**：和 Rust 一样，OmniLang 默认不可变。这可以在编译期捕获意外的修改。

### 遮蔽（Shadowing）

你可以声明一个同名的新变量来**遮蔽**之前的变量。允许跨作用域遮蔽，但不允许在同一作用域内。

```omni
fn main() {
    let x: i32 = 5;
    println(x.to_string());  // 5

    {
        let x: i32 = 10;     // 遮蔽外层的 x（不同作用域 — 允许）
        println(x.to_string());  // 10
    }

    println(x.to_string());  // 5 — 外层 x 未改变
}
```

---

## 基本类型

| 类型 | 描述 | 示例 |
|------|------|------|
| `i32` | 32 位有符号整数 | `42`, `-7`, `0` |
| `f64` | 64 位浮点数 | `3.14`, `-0.5`, `1.0` |
| `bool` | 布尔值 | `true`, `false` |
| `String` | UTF-8 字符串 | `"hello"`, `"你好"` |
| `i64` | 64 位有符号整数 | `9999999999` |

```omni
fn main() {
    let age: i32 = 25;
    let price: f64 = 19.99;
    let active: bool = true;
    let name: String = "Alice";
    let big: i64 = 9999999999;

    println(age.to_string());
    println(price.to_string());
    println(active.to_string());
    println(name);
    println(big.to_string());
}
```

### 数字字面量

```omni
let decimal: i32 = 255;       // 255
let hex: i32 = 0xff;          // 255（十六进制）
let octal: i32 = 0o377;       // 255（八进制）
let binary: i32 = 0b11111111; // 255（二进制）
let sci: f64 = 1.5e3;         // 1500.0（科学记数法）
```

---

## 函数

用关键字 `fn` 声明函数。参数需要类型注解。返回值类型在 `->` 后面指定。

```omni
// 带参数和返回类型的函数
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// 无返回值的函数（隐式返回 void）
fn greet(name: String) {
    println("Hello, " + name);
}

// 尾表达式 —— 最后一个不带分号的表达式
// 就是返回值（无需写 return 关键字）
fn make_greeting(name: String) -> String {
    "Hello, " + name       // 没有分号 = 隐式返回
}
```

```omni
fn main() {
    let result: i32 = add(2, 4);
    println(result.to_string());  // 6

    greet("Alice");               // Hello, Alice

    let msg: String = make_greeting("Bob");
    println(msg);                 // Hello, Bob
}
```

> **编码风格**：提前退出用 `return`；最终值推荐尾表达式。语法融合了 Dart 和 Rust 的风格。

### `main()` 函数

每个可执行的 `.omni` 文件都需要一个 `main()` 函数作为入口点：

```omni
fn main() {
    print("Hello, World!");
}
```

---

## print 与 println

两个内置函数用于控制台输出：

| 函数 | 行为 |
|------|------|
| `print(value)` | 不换行输出 |
| `println(value)` | 输出后换行 |

```omni
fn main() {
    println("这行单独一行");
    print("没有换行");
    print(" —— 和前面在同一行");
}
```

`print`/`println` 接受 `String` 类型参数。数字需要用 `.to_string()` 转换。

---

## 运算符

### 算术运算符

| 运算符 | 含义 | 示例 |
|--------|------|------|
| `+` | 加法 | `a + b` |
| `-` | 减法 | `a - b` |
| `*` | 乘法 | `a * b` |
| `/` | 除法（整数除法截断向零） | `a / b` |
| `%` | 取模（取余） | `a % b` |

### 比较运算符

| 运算符 | 含义 |
|--------|------|
| `==` | 等于 |
| `!=` | 不等于 |
| `<` | 小于 |
| `>` | 大于 |
| `<=` | 小于或等于 |
| `>=` | 大于或等于 |

### 逻辑运算符

| 运算符 | 含义 |
|--------|------|
| `&&` | 逻辑与 |
| `\|\|` | 逻辑或 |
| `!` | 逻辑非 |

### 复合赋值

| 运算符 | 含义 |
|--------|------|
| `=` | 赋值 |
| `+=` | 加后赋值 |
| `-=` | 减后赋值 |
| `*=` | 乘后赋值 |
| `/=` | 除后赋值 |
| `%=` | 取模后赋值 |

### 字符串拼接

用 `+` 连接字符串：

```omni
fn main() {
    let first = "Hello";
    let second = "World";
    let combined = first + ", " + second;
    println(combined);  // Hello, World
}
```

---

## 控制流

### if / else if / else

```omni
fn main() {
    let x: i32 = 10;

    if x > 5 {
        println("大");
    } else if x == 0 {
        println("零");
    } else {
        println("小");
    }
}
```

### if 作为表达式

`if` 可以当作表达式使用（类似三目运算符）：

```omni
fn main() {
    let score: i32 = 85;
    let grade: String = if score >= 60 { "及格" } else { "不及格" };
    println(grade);
}
```

### while 循环

```omni
fn main() {
    let mut i: i32 = 0;
    while i < 3 {
        print(i.to_string());
        i = i + 1;
    }
    println("完成");
    // 输出: 012完成
}
```

### for-in 循环

遍历数组和集合：

```omni
fn main() {
    let items = [10, 20, 30];

    for item in items {
        println(item.to_string());
    }
    // 输出: 10, 20, 30（每个独占一行）
}
```

### break 和 continue

```omni
fn main() {
    let mut i: i32 = 0;
    while i < 10 {
        i = i + 1;
        if i == 3 {
            continue;   // 跳过 3
        }
        if i == 7 {
            break;      // 在 7 处停止
        }
        print(i.to_string());
    }
    // 输出: 12456
}
```

> `break` 和 `continue` 只能在循环内使用。在 `while` 或 `for` 之外使用会导致编译错误。

---

## 数组（Array）

数组是固定类型的同构集合，用 `Array<T>` 声明：

```omni
fn main() {
    // 创建数组
    let arr: Array<i32> = [1, 2, 3];

    // 数组方法
    println(arr.len().to_string());      // 3
    println(arr.is_empty().to_string()); // false
    println(arr.get(0).to_string());     // 1
    println(arr.get(2).to_string());     // 3
}
```

### 可变数组

使用 `let mut` 创建可变数组，再调用 `push()` 和 `pop()`：

```omni
fn main() {
    let mut arr: Array<i32> = [1, 2, 3];

    arr.push(4);
    arr.push(5);
    println(arr.len().to_string());      // 5
    println(arr.get(4).to_string());     // 5

    let last = arr.pop();                // 移除并返回 5
    println(last.to_string());
    println(arr.len().to_string());      // 4
}
```

> **所有权说明**：数组（和字符串一样）使用**移动语义**。把数组赋值给新变量会转移所有权——原变量不能再使用。详见[教程 5：所有权与闭包](05-ownership-and-closures.md)。

---

## 映射（Map）

映射是键值对集合，用 `Map<K, V>` 声明：

```omni
fn main() {
    let map: Map<String, String> = {"name": "Alice", "role": "admin"};

    println(map.len().to_string());         // 2
    println(map.contains("name").to_string()); // true
    println(map.get("name"));               // Alice
    println(map["role"]);                   // admin
}
```

### 可变映射

```omni
fn main() {
    let mut scores: Map<String, i32> = {};

    scores.insert("Alice", 95);
    scores.insert("Bob", 87);

    println(scores["Alice"].to_string());  // 95
    println(scores.len().to_string());     // 2
}
```

---

## 类型转换

使用 `.to_string()` 将数字和布尔值转换为字符串：

```omni
fn main() {
    let n: i32 = 42;
    let s: String = n.to_string();     // "42"

    let f: f64 = 3.14;
    let s2: String = f.to_string();    // "3.14"

    let b: bool = true;
    let s3: String = b.to_string();    // "true"
}
```

---

## 综合示例

一个使用本章所有知识的完整示例：

```omni
fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() {
    let limit: i32 = 10;
    let mut i: i32 = 0;

    println("斐波那契数列：");
    while i < limit {
        let value = fib(i);
        println(value.to_string());
        i = i + 1;
    }
}
```

运行：

```bash
omnic run src/main.omni
```

---

## 下一步

你已经掌握了足够编写基础程序的 OmniLang 知识。继续阅读：

- **[3. 结构体与数据类型](03-structs-and-data.md)** — 自定义类型、泛型和带标签联合体
- **[7. OmniView 组件](07-omniview-components.md)** — 用组件和模板构建 UI
- **[Omni 语法规范](../omni-grammar-spec.md)** — 完整形式化语言参考
