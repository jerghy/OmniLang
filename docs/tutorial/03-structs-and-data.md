# 3. 结构体与数据类型

> **目标**：定义自定义数据类型 —— 结构体、类型别名、可选值和集合操作。

---

## 结构体（Struct）

**结构体**将相关数据组合成一个命名类型：

```omni
struct Point {
    x: f64,
    y: f64,
}

fn main() {
    let p: Point = Point { x: 1.0, y: 2.0 };
    println("ok");
}
```

### 结构体字段

字段需要类型注解。末尾逗号可选：

```omni
struct User {
    name: String,
    age: i32,
    email: String,
}
```

### 创建实例

使用结构体名后跟 `{ field: value, ... }`：

```omni
fn main() {
    let alice: User = User {
        name: "Alice",
        age: 30,
        email: "alice@example.com",
    };
}
```

### 字段访问

用点号访问字段：

```omni
fn print_user(user: User) {
    println(user.name);
    println(user.age.to_string());
    println(user.email);
}
```

### 结构体嵌套

```omni
struct Address {
    street: String,
    city: String,
}

struct Person {
    name: String,
    address: Address,
}

fn main() {
    let addr: Address = Address { street: "人民路 123 号", city: "上海" };
    let person: Person = Person { name: "Alice", address: addr };
    println(person.address.city);    // 上海
}
```

---

## 可见性：`pub`

默认情况下，结构体及其字段对模块是**私有的**。使用 `pub` 让其他文件可以访问：

```omni
pub struct Post {
    title: String,       // 仍然是私有的——只有结构体是公开的
    body: String,
    pub author: String,  // 结构体和这个字段都是公开的
}
```

`pub` 也适用于函数：

```omni
pub fn format_date(timestamp: i64) -> String {
    return "2026-06-17";
}
```

---

## 类型别名

用 `type` 创建更简短或更描述性的类型名称：

```omni
type UserId = i32;
type Username = String;
type Scores = Map<String, i32>;

fn main() {
    let id: UserId = 42;
    let name: Username = "alice";
    let scores: Scores = {"数学": 95, "英语": 88};

    println(id.to_string());
    println(name);
}
```

---

## 带标签联合体（Tagged Union）

> 🚧 **即将推出**：带标签联合体和 `match` 表达式正在开发中。当前你可以使用结构体 + `if` 来判断类型。

---

## 泛型（Generics）

> 🚧 **即将推出**：泛型函数和泛型结构体正在开发中。

---

## 可选值：`null`

OmniLang 使用 `null` 表示缺失的值，类似 Dart。用 `?` 声明可为空类型：

```omni
fn find_user(id: i32) -> String? {
    if id == 1 {
        return "Alice";
    }
    return null;
}
```

> **注意**：可为空语法（`T?`）已被解析器识别。当前空安全强制执行状态请查看[语法规范](../omni-grammar-spec.md)。

### 空值合并：`??`

`??` 运算符在表达式为 `null` 时提供默认值：

```omni
fn main() {
    let name: String? = null;
    let display: String = name ?? "Guest";
    println(display);    // Guest
}
```

---

## 模板字符串

> 🚧 **即将推出**：模板字符串和插值语法正在开发中。当前请使用 `+` 拼接字符串。

---

## 内置集合

| 类型 | 语法 | 可变方法 |
|------|------|---------|
| `Array<T>` | `[1, 2, 3]` | `push()`, `pop()` |
| `Map<K, V>` | `{"k": v}` | `insert()`, `remove()` |

完整 API：

```omni
// 数组
arr.len() -> i32
arr.is_empty() -> bool
arr.get(index: i32) -> T
arr.push(value: T)        // 仅可变数组
arr.pop() -> T            // 仅可变数组

// 映射
map.len() -> i32
map.is_empty() -> bool
map.get(key: K) -> V
map.contains(key: K) -> bool
map.insert(key: K, value: V)    // 仅可变映射
map.remove(key: K)              // 仅可变映射
```

---

## 综合示例

```omni
struct Product {
    name: String,
    price: f64,
    in_stock: bool,
}

type Catalog = Array<Product>;

fn total_value(catalog: Catalog) -> f64 {
    let mut sum: f64 = 0.0;
    for product in catalog {
        if product.in_stock {
            sum = sum + product.price;
        }
    }
    return sum;
}

fn main() {
    let catalog: Catalog = [
        Product { name: "小工具", price: 9.99, in_stock: true },
        Product { name: "小装备", price: 19.99, in_stock: true },
        Product { name: "小玩意儿", price: 29.99, in_stock: false },
    ];

    let value: f64 = total_value(catalog);
    println(value.to_string());    // 29.98
}
```

---

## 下一步

- **[4. 模块与导入](04-modules-and-imports.md)** — 跨文件组织代码
- **[6. 模式匹配](06-pattern-matching.md)** — match 表达式和解构
- **[标准库](../stdlib.md)** — 内置模块：`omni/math`、`omni/string`、`omni/async`
