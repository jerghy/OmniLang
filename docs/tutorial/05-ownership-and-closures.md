# 5. 所有权与闭包

> **目标**：理解 OmniLang 的所有权模型、编写闭包，以及使用 async/await。

OmniLang 使用借鉴自 Rust 的简化所有权模型。编译器追踪值的所有权，在编译期阻止"使用已移动的值"的错误——但无需显式的生命周期注解。

---

## 移动语义（Move Semantics）

**String** 和 **Array** 类型使用移动语义。当把它们赋值给新变量时，所有权会转移——原变量不能再使用：

```omni
fn main() {
    let s1: String = "hello";
    let s2: String = s1;      // s1 的所有权移动到 s2
    println(s2);               // OK — s2 拥有该值
    // println(s1);            // 错误：s1 已被移动
}
```

数组同理：

```omni
fn main() {
    let a1: Array<i32> = [1, 2, 3];
    let a2: Array<i32> = a1;   // a1 的所有权移动到 a2
    a2.push(4);                // OK
    // a1.len();               // 错误：a1 已被移动
}
```

### 函数获取所有权

将 String 或 Array 传递给函数会转移所有权：

```omni
fn process(s: String) {
    println(s);
}    // s 在此处被释放

fn main() {
    let msg: String = "hello";
    process(msg);           // msg 移动到 process() 中
    // println(msg);        // 错误：msg 已被移动
}
```

### 复制类型（Copy Types）

`i32`、`f64` 和 `bool` 等简单类型是**复制类型**——自动复制，不移动：

```omni
fn main() {
    let x: i32 = 42;
    let y: i32 = x;        // x 被复制（开销很小）
    println(x.to_string()); // OK — x 仍然有效
    println(y.to_string()); // OK
}
```

| 类型类别 | 行为 |
|---------|------|
| `i32`, `f64`, `bool`, `i64` | 复制 — 简单的按位复制 |
| `String`, `Array<T>` | 移动 — 所有权转移 |
| 仅含 Copy 类型的结构体 | 复制 |
| 含 Move 类型的结构体 | 移动 |

---

## 闭包（Closure）

> 🚧 **即将推出**：完整的闭包语法（`|params| body`）正在开发中。当前在组件事件处理器中可以使用内联回调 `() => { ... }`：

```omni
component Counter {
    signal count: i32 = 0;

    view {
        <div>
            <button @onPress={() => { count = count + 1; }}>
                计数：{count}
            </button>
        </div>
    }
}
```

---

## Async / Await ✅

OmniLang 支持 `async fn` 和 `await`，当前可用目标：**Python** 和 **JavaScript**。

### 声明异步函数

```omni
async fn fetch_data() -> String {
    return "数据已接收";
}
```

### 等待结果

```omni
async fn main() {
    let result: String = await fetch_data();
    println(result);               // 数据已接收
}
```

运行：
```bash
omnic run app.omni --target python   # 输出：数据已接收
omnic run app.omni --target js       # 输出：数据已接收
```

> **重要**：`await` 只能在 `async fn` 内部使用。编译目标为 Python 时，`async fn main()` 会自动生成 `asyncio.run(main())`。

### 带参数的异步函数

```omni
async fn greet(name: String) -> String {
    return "你好，" + name;
}

async fn main() {
    let msg: String = await greet("Alice");
    println(msg);
}
```

> 🚧 **即将推出**：`omni/async` 标准库（`sleep`、`spawn`、`all`、`race`、`timeout`）。

---

## 常见陷阱

### 1. 移动后使用

```omni
fn main() {
    let s: String = "hello";
    process(s);              // s 被移动
    println(s);              // 错误：移动后使用
}
```

**修复**：如果需要保留原值，可以重构代码逻辑。

### 2. 循环中的所有权

使用 `for` 遍历时，循环变量获取每个元素的所有权：

```omni
fn main() {
    let arr: Array<String> = ["a", "b"];

    for item in arr {        // 每个元素移动到循环中
        let owned: String = item;
        println(owned);
    }
    // arr 在此处不能使用——元素已被移动
}
```

### 3. for 循环中修改集合

不要在迭代时修改正在遍历的集合——编译器会检测到这种情况：

```omni
fn main() {
    let mut arr: Array<i32> = [1, 2, 3];
    // for item in arr {
    //     arr.push(4);     // 错误：迭代期间修改
    // }
}
```

---

## 总结

| 概念 | 关键规则 |
|------|---------|
| **复制类型** | `i32`, `f64`, `bool` — 可自由复制，无所有权追踪 |
| **移动类型** | `String`, `Array<T>` — 单一所有者，赋值时移动 |
| **闭包** | `\|params\| 函数体` — 捕获作用域变量，由编译器决定捕获方式 |
| **Async** | `async fn` + `await` — 非阻塞操作 |
| **标准库** | `omni/async` — sleep、spawn、all、race、timeout |

---

## 下一步

- **[6. 模式匹配](06-pattern-matching.md)** — match 表达式和带标签联合体解构
- **[3. 结构体与数据类型](03-structs-and-data.md)** — 与所有权配合使用的自定义类型
- **[标准库](../stdlib.md)** — `omni/async` 完整 API 参考
