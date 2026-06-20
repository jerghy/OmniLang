# 6. 模式匹配

> 🚧 **本章内容即将推出**

`match` 表达式、带标签联合体（tagged union）和泛型正在开发中。

当前你可以使用 `if`/`else if`/`else` 来实现类似的条件逻辑：

```omni
fn main() {
    let x: i32 = 2;
    let msg: String = if x == 1 {
        "一"
    } else if x == 2 {
        "二"
    } else {
        "其他"
    };
    println(msg);    // 二
}
```

以及用结构体 + `if` 做类型判断：

```omni
struct Circle { radius: f64 }
struct Rectangle { width: f64, height: f64 }

fn area_circle(c: Circle) -> f64 {
    return 3.14159 * c.radius * c.radius;
}
```

---

> 🚧 以下是规划中的 `match` 语法，尚未实现：

## 基本 Match（规划中）

```omni
fn main() {
    let x: i32 = 2;
    let msg: String = match x {
        1 => "一",
        2 => "二",
        _ => "其他",          // 通配符 — 匹配任意值
    };
    println(msg);              // 二
}
```

每个 `match` 分支在 `=>` 左侧有一个**模式**，右侧有一个**结果表达式**。分支按顺序求值——第一个匹配的分支胜出。

---

## 通配符 `_`

下划线 `_` 匹配任意值，用作兜底：

```omni
fn describe(n: i32) -> String {
    return match n {
        0 => "零",
        1 => "一",
        2 => "二",
        _ => "很多",           // 捕获 3, 4, 5, ...
    };
}
```

> **没有 `_` 时**：如果没有分支匹配且没有通配符，编译器会报告非穷尽 match 错误。

---

## 匹配多个值

用 `|` 在一个分支中匹配多个值：

```omni
fn is_vowel(c: String) -> bool {
    return match c {
        "a" | "e" | "i" | "o" | "u" => true,
        _ => false,
    };
}
```

---

## 解构结构体

Match 可以解构结构体以提取字段：

```omni
struct Point {
    x: f64,
    y: f64,
}

fn quadrant(p: Point) -> String {
    return match p {
        Point { x, y } if x > 0.0 && y > 0.0 => "第一象限",
        Point { x, y } if x < 0.0 && y > 0.0 => "第二象限",
        Point { x, y } if x < 0.0 && y < 0.0 => "第三象限",
        Point { x, y } if x > 0.0 && y < 0.0 => "第四象限",
        _ => "在坐标轴上",
    };
}
```

模式后面的 `if` 子句是**守卫（guard）**——只有在守卫条件为真时该分支才匹配。

---

## 解构带标签联合体

这是 `match` 真正大放异彩的地方。带标签联合体（见[教程 3](03-structs-and-data.md)）的每个变体都可以分别匹配：

```omni
type Shape = Circle { radius: f64 }
           | Rectangle { width: f64, height: f64 }
           | Triangle { base: f64, height: f64 };

fn area(shape: Shape) -> f64 {
    return match shape {
        Circle { radius } => 3.14159 * radius * radius,
        Rectangle { width, height } => width * height,
        Triangle { base, height } => 0.5 * base * height,
    };
}

fn main() {
    let c: Shape = Circle { radius: 5.0 };
    let r: Shape = Rectangle { width: 4.0, height: 6.0 };

    println(area(c).to_string());    // ~78.5
    println(area(r).to_string());    // 24.0
}
```

### 单元变体

没有数据的变体按名称匹配：

```omni
type Status = Active | Inactive | Banned;

fn can_post(status: Status) -> bool {
    return match status {
        Active => true,
        Inactive => true,
        Banned => false,
    };
}
```

---

## 嵌套模式

模式可以嵌套以进行深层解构：

```omni
struct Address {
    street: String,
    city: String,
}

struct Person {
    name: String,
    address: Address,
}

fn is_from(person: Person, city: String) -> bool {
    return match person {
        Person { name: _, address: Address { city: c, .. } } => c == city,
    };
}
```

- `name: _` —— 忽略 name 字段
- `Address { city: c, .. }` —— 提取 `city`，用 `..` 忽略 Address 的其他字段

---

## 带守卫的 Match

守卫在模式结构之外添加条件：

```omni
fn classify(n: i32) -> String {
    return match n {
        n if n < 0 => "负数",
        0 => "零",
        n if n % 2 == 0 => "正偶数",
        _ => "正奇数",
    };
}
```

---

## 穷尽性检查

编译器会验证所有可能的值都被覆盖：

```omni
type Color = Red | Green | Blue;

fn hex(color: Color) -> String {
    return match color {
        Red => "#ff0000",
        Green => "#00ff00",
        // Blue => "#0000ff",    // 如果注释掉：错误 — 非穷尽匹配
        _ => "#000000",          // 通配符满足穷尽性
    };
}
```

> 始终包含通配符 `_` 分支，除非你明确希望编译器强制处理每个变体。

---

## Match vs If

| 使用 `match` 的时机 | 使用 `if` 的时机 |
|-------------------|-----------------|
| 解构带标签联合体 | 简单的布尔条件 |
| 多个明确分支 | 二分支逻辑 |
| 需要穷尽性检查 | 条件之间没有关联 |

---

## 综合示例：表达式求值器

```omni
type Expr = Num { value: i32 }
          | Add { left: Expr, right: Expr }
          | Mul { left: Expr, right: Expr };

fn evaluate(expr: Expr) -> i32 {
    return match expr {
        Num { value } => value,
        Add { left, right } => evaluate(left) + evaluate(right),
        Mul { left, right } => evaluate(left) * evaluate(right),
    };
}

fn main() {
    // 表示: (2 + 3) * 4
    let expr: Expr = Mul {
        left: Add {
            left: Num { value: 2 },
            right: Num { value: 3 },
        },
        right: Num { value: 4 },
    };

    println(evaluate(expr).to_string());    // 20
}
```

---

## 下一步

- **[3. 结构体与数据类型](03-structs-and-data.md)** — 带标签联合体和类型别名
- **[5. 所有权与闭包](05-ownership-and-closures.md)** — 所有权与 match 解构的交互
- **[Omni 语法规范](../omni-grammar-spec.md)** — match 语法的形式化参考
