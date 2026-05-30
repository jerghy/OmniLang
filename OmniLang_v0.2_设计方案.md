# OmniLang v0.2 设计方案

## 版本定位与设计哲学

### v0.1 → v0.2 的根本转变

| 维度 | v0.1 | v0.2 |
|------|------|------|
| 核心哲学 | Rust 语义为基准，其他语言模拟 | 各目标生成**地道**代码，转译器弥合差异 |
| 类型系统 | 名义类型，严格限制 | 结构类型为主，名义类型可选 |
| 所有权 | Rust 所有权强制建模 | 各语言原生语义，不再强制 |
| 表达式能力 | 刻意限制 | 接近现代语言标准 |
| 前端适用 | ❌ | ✅ |
| 一致性策略 | 强制语义一致 | 分析器警告潜在不一致 |

**核心洞察：** OmniLang 的价值不在于"统一语义"，而在于"一次编写，到处运行"的生产力。v0.2 放弃强求跨语言运行时语义完全一致，转为让每个目标语言生成**地道、可读、与生态无缝衔接**的代码，同时通过增强的静态分析系统（`omnic check`）在编译期发现潜在的可移植性问题。

**⚠️ 版本定位声明：** v0.1 → v0.2 是一次**根本性的版本跃迁（fundamental version leap）**，而非简单的功能累加。从"Rust 统一语义"到"地道转译 + 分析保障"的架构转变影响所有已编译代码的运行时行为。请在升级前阅读 §14 迁移指南，并使用 `omnic migrate --dry-run` 预览变更。此声明应在所有对外文档和宣传中明确传达，帮助社区理解设计意图，减少"为什么行为变了"的困惑。

### 三条新原则

1. **地道优先 (Idiom-First)**：生成的代码应该看起来像是目标语言的资深开发者手写的
2. **渐进式类型 (Gradual Typing)**：允许从动态类型逐步迁移到静态类型，兼顾快速开发和安全性
3. **分析即文档 (Analysis-as-Documentation)**：`omnic check` 不仅能找出错误，还能解释为什么某个写法在特定目标下有问题，并给出改进建议

---

## 语法扩展总览

### 新增关键字和语法元素

```
v0.1 关键字:
  let mut fn void if else while for in break continue return true false
  print println

v0.2 新增关键字:
  async await           — 异步编程
  export import from as  — 模块系统
  struct class new this extends implements — 结构体/类
  type                   — 类型别名
  match                  — 模式匹配
  @                      — 装饰器前缀
  ...                    — 展开/剩余操作符
  | ... |                — 闭包语法
  =>                     — 箭头函数
  `${}`                  — 模板字符串
  ?                      — 可选类型后缀
  T?                     — 可选类型语法糖
  ?.                     — 可选链
  ??                     — 空值合并
```

### 向后兼容性

v0.1 的所有语法和语义在 v0.2 中**继续有效**，但行为有所调整：

| v0.1 行为 | v0.2 行为 |
|-----------|-----------|
| `let a: Array<i32> = arr;` → Rust 自动 `.clone()` | JS/Python 生成浅拷贝；Rust 仍 `.clone()`；**analyze 警告**在非 Rust 目标下的行为差异 |
| `let s2: String = s1;` → 所有目标深克隆 | 各目标使用原生语义（JS 字符串不可变故安全；Python 生成 `str(s1)` 以确保独立） |
| 函数参数非 Copy 不可变 | 类型系统不再强制所有权，改为每个目标的自然语义 |
| 浮点 `/` `%` 禁止 | **保留禁止**，但 analyze 工具对于已知安全场景给出 suppress 建议 |

---

## 1. 异步编程 (async/await)

### 1.1 语法定义

```omnilang
// 异步函数声明
async fn fetchUser(id: i32) -> User? {
    let url: String = `/api/users/${id}`;
    let response: HttpResponse = await httpClient.get(url);
    if response.status == 200 {
        return response.parseJson<User>();
    }
    return null;
}

// 异步函数调用
async fn main() {
    let user: User? = await fetchUser(42);
    match user {
        Some(u) => println(`Found: ${u.name}`),
        None => println("Not found"),
    }
}

// 并行等待 (Promise.all / Future.wait / asyncio.gather)
async fn loadAll() -> Array<User> {
    let a: Future<User> = fetchUser(1);
    let b: Future<User> = fetchUser(2);
    let results: Array<User> = await all([a, b]);
    return results;
}
```

### 1.2 语义规则

| 规则 | 说明 |
|------|------|
| `async fn` | 函数返回类型隐含为 `Future<T>` 或 `Promise<T>`（取决于目标） |
| `await` | 只能在 `async fn` 内使用 |
| 错误处理 | 异步函数中的 panic/throw 自动传播（ reject / raise / Err ） |
| `all()` | 内置函数，并行等待多个 Future，类似 `Promise.all` |
| `race()` | 内置函数，竞争等待，类似 `Promise.race` |

### 1.3 各目标转译

**JavaScript:**
```javascript
async function fetchUser(id) {
    const url = `/api/users/${id}`;
    const response = await httpClient.get(url);
    if (response.status === 200) {
        return response.parseJson();
    }
    return null;
}

async function main() {
    const user = await fetchUser(42);
    // ...
}
```

**Dart:**
```dart
Future<User?> fetchUser(int id) async {
    final url = '/api/users/$id';
    final response = await httpClient.get(url);
    if (response.status == 200) {
        return response.parseJson<User>();
    }
    return null;
}

Future<void> main() async {
    final user = await fetchUser(42);
    // ...
}
```

**Python:**
```python
async def fetch_user(id: int) -> User | None:
    url = f"/api/users/{id}"
    response = await http_client.get(url)
    if response.status == 200:
        return response.parse_json()
    return None

async def main():
    user = await fetch_user(42)
    # ...

if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
```

**Rust (tokio):**
```rust
async fn fetch_user(id: i32) -> Option<User> {
    let url = format!("/api/users/{}", id);
    let response = http_client.get(&url).await;
    if response.status == 200 {
        return Some(response.parse_json::<User>());
    }
    None
}

#[tokio::main]
async fn main() {
    let user = fetch_user(42).await;
    // ...
}
```

### 1.4 异步运行时配置

```omnilang
// 项目级配置 (omnilang.toml)
[target.rust]
async_runtime = "tokio"        // tokio | async-std | smol

[target.python]
async_runtime = "asyncio"      // asyncio | trio

[target.dart]
async_runtime = "native"       // native (Future-based)

[target.js]
async_runtime = "native"       // native (Promise-based)
```

### 1.5 异步错误处理（关键风险）

JS 的 Promise rejection、Dart 的 Future error、Python 的 asyncio exception、Rust 的 `Result::Err` 在传播机制上有微妙差异。未处理的异步错误在 JS 中可能导致 `UnhandledPromiseRejection`（Node 15+ 会终止进程），在 Rust 中则是编译期强制要求处理。

**建议在标准库中提供跨语言一致的错误处理辅助：**

```omnilang
// omni/async — 错误处理工具

// 将 Future 的异常转换为 Result 类型，消除隐式传播差异
export fn tryCatch<T>(future: Future<T>) -> Future<Result<T, Error>>;

// 使用示例
async fn fetchData() -> Result<String, Error> {
    let result: Result<String, Error> = await tryCatch(http.get("/api/data"));
    return match result {
        Ok { value } => Ok { value: value.body },
        Err { error } => Err { error: `Network error: ${error.message}` },
    };
}
```

**Analyze 规则 `E0203` 增强：** 检测 `async fn` 中未处理的 `await` 表达式，在 Rust 目标下严格报错（`Result` 必须被消费），在 JS/Dart 目标下发出 WARNING（未处理的 rejection 是运行时隐患）。

---

## 2. 闭包与箭头函数

### 2.1 语法定义

```omnilang
// 完整闭包语法
let add: fn(i32, i32) -> i32 = |a: i32, b: i32| -> i32 {
    return a + b;
};

// 类型推导的闭包
let sub = |a: i32, b: i32| a - b;

// 箭头函数（单表达式闭包语法糖）
let mul = (a: i32, b: i32) => a * b;

// 多语句箭头函数
let complex = (a: i32, b: i32) => {
    let tmp = a + b;
    return tmp * 2;
};

// 无参闭包
let say_hi = || println("Hi");
let say_hi2 = () => println("Hi");

// 捕获外部变量
fn make_multiplier(factor: i32) -> fn(i32) -> i32 {
    return (x: i32) => x * factor;
}

// 闭包作为参数
fn transform(arr: Array<i32>, f: fn(i32) -> i32) -> Array<i32> {
    // ...
}
arr.transform((x: i32) => x * 2);

// 方法引用
let printer = println;  // 函数引用
let to_str = i32.to_string;  // 方法引用（部分应用）
```

### 2.2 闭包类型标注

```omnilang
// 闭包类型语法: fn(ParamTypes) -> ReturnType
let callback: fn(i32, String) -> bool;

// 捕获闭包 (有状态的)
let stateful: fn(i32) -> i32;  // 自动推导为捕获闭包

// 纯函数类型 (无捕获，可以在 Rust 中作为 fn pointer)
let pure: static fn(i32) -> i32;
```

### 2.3 默认捕获语义（重要契约）

闭包的捕获语义在语言层面**明确定义为可变引用捕获**（符合 JS/Dart/Python 的自然语义）：

```omnilang
fn makeCounter() -> fn() -> i32 {
    let mut count: i32 = 0;
    return || {
        count += 1;   // count 被可变引用捕获
        return count;
    };
}

let counter = makeCounter();
counter();  // 1
counter();  // 2  — 共享同一份 count
```

**各目标实现策略：**

| 目标 | 实现方式 |
|------|---------|
| JS | 自然闭包（GC 管理） |
| Dart | 自然闭包（GC 管理） |
| Python | `nonlocal count` 修饰（显式标记自由变量） |
| Rust | 逃逸闭包自动 `move`，捕获的 `mut` Copy 类型用 `Cell<i32>` 包装，非 Copy 类型用 `RefCell<T>` 包装 |

**Rust 后端自动包装规则：**

```rust
// Copy 类型 + mut 捕获 → Cell
fn make_counter() -> impl FnMut() -> i32 {
    let count = std::cell::Cell::new(0);
    move || {
        count.set(count.get() + 1);
        count.get()
    }
}

// 非 Copy 类型 + mut 捕获 → RefCell
// 注意：RefCell 会引入运行时借用检查，违反借用规则会 panic
```

**Analyze 规则 `E0303` 增强：** 在 Rust 后端检测到闭包逃逸且捕获 `mut` 变量时，发出 PORT 级别警告：

```
Port: E0303 (closure_escape_rust_mut)
  --> src/main.omni:10:12
   |
10 |     return || { count += 1; return count; };
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |            Rust 后端将使用 Cell/RefCell 包装捕获变量。
   |            - Copy 类型 (i32): 使用 Cell — 安全但提醒开发者
   |            - 非 Copy 类型: 使用 RefCell — 运行时借用检查，违反将 panic
   |
   = note: JS/Dart/Python 目标无此限制（GC 管理生命周期）
   = help: 如需避免 Rust 的 RefCell 包装，考虑将状态封装在 struct 中而非使用闭包
```

> **⚠️ 性能注意：** Rust 目标下使用 `RefCell` 包装的闭包在每次访问捕获变量时都有运行时借用检查开销（`borrow()`/`borrow_mut()`）。热路径上频繁调用的 `RefCell` 闭包可能导致可测量的性能下降。如需零开销方案，请将可变状态封装在 `struct` 中并使用 `impl` 方法模式，或使用 `nominal struct Counter(i32)` + `impl` 替代闭包。`Cell<T>` 用于 Copy 类型时无此开销（直接 `get()`/`set()` 值拷贝）。

---

### 2.5 各目标转译

**JavaScript:**
```javascript
const add = (a, b) => a + b;
const sub = function(a, b) { return a - b; };
function makeMultiplier(factor) {
    return (x) => x * factor;
}
```

**Dart:**
```dart
final add = (int a, int b) => a + b;
int Function(int) makeMultiplier(int factor) {
    return (int x) => x * factor;
}
```

**Python:**
```python
add = lambda a, b: a + b
def make_multiplier(factor):
    return lambda x: x * factor
# 注意：Python lambda 只能单表达式，多语句闭包转为嵌套 def
```

**Rust:**
```rust
let add = |a: i32, b: i32| a + b;
fn make_multiplier(factor: i32) -> impl Fn(i32) -> i32 {
    move |x: i32| x * factor
}
```

### 2.6 闭包捕获分析 (Analyze 功能)

`omnic check` 会对闭包进行逃逸分析：

```
Warning: closure escapes its defining scope [E0306]
  --> src/main.omni:15:12
   |
15 |     return (x: i32) => x * factor;
   |            ^^^^^^^^^^^^^^^^^^^^^^^ closure captures `factor` by reference
   |                                   but the reference outlives `factor`
   |
   = help: for Rust target, `factor` must be cloned or moved
   = help: add `move` keyword: `return move (x: i32) => x * factor;`
   = note: JS/Dart/Python targets are unaffected (GC handles lifetimes)
```

---

## 3. 模块系统

### 3.1 语法定义

```omnilang
// ===== file: math.omni =====

// 公开导出
export fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

export fn sub(a: i32, b: i32) -> i32 {
    return a - b;
}

export struct Point {
    x: f64,
    y: f64,
}

// 私有（默认）
fn helper() -> i32 {
    return 42;
}

// ===== file: main.omni =====

// 命名导入
import { add, sub, Point } from "./math";

// 别名导入
import { add as sum } from "./math";

// 通配符导入
import * as math from "./math";

// 类型导入 (仅用于类型标注，不影响运行时)
import type { User } from "./models";

// 默认导入
import HttpClient from "./http";

// 使用
fn main() {
    let result: i32 = add(1, 2);
    let p: Point = Point { x: 1.0, y: 2.0 };
    let r2: i32 = math.sub(5, 3);
}
```

### 3.2 模块解析规则

```
import "./math"       → 同目录 ./math.omni
import "../lib/math"  → 父目录的 lib/math.omni
import "std/fs"       → 标准库路径 (内置)
import "pkg:http"     → 包管理器路径 (未来)
```

### 3.3 可见性规则

| 修饰符 | 含义 |
|--------|------|
| `export` | 公开导出，其他模块可导入 |
| (无修饰符) | 私有，仅当前模块内可见 |
| `export type` | 仅导出类型（运行时不可见，类似 TS 的 `export type`） |

### 3.4 各目标转译

**JavaScript (ES Modules):**
```javascript
// math.js
export function add(a, b) { return a + b; }
export function sub(a, b) { return a - b; }
export class Point { constructor(x, y) { this.x = x; this.y = y; } }

// main.js — 普通导入
import { add, sub, Point } from "./math.js";
import * as math from "./math.js";

// import type → 编译时完全擦除，不生成任何 JS 代码
// import type { User } from "./models";
//   → JS plain 模式: 无输出（类型被擦除）
//   → TypeScript 模式: import type { User } from "./models"; (TS 的 type-only import，编译后同样移除)
```

> **`import type` 规则：** OmniLang 中 `import type { T } from "./m"` 声明的类型导入在 **所有目标** 中都不会产生运行时代码。JS plain 模式下直接删除该行；TypeScript 模式下保留 `import type` 语法；Dart/Python 中通过静态分析消费后删除；Rust 中转为 `use crate::m::T`（零运行时开销）。

**Dart:**
```dart
// math.dart
int add(int a, int b) => a + b;
int sub(int a, int b) => a - b;
class Point { final double x, y; Point(this.x, this.y); }

// main.dart
import 'math.dart';
import 'math.dart' as math;
```

**Python:**
```python
# math.py
def add(a, b): return a + b
def sub(a, b): return a - b

# main.py
from math import add, sub
import math as math_module
```

**Rust:**
```rust
// math.rs (or math/mod.rs)
pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn sub(a: i32, b: i32) -> i32 { a - b }
pub struct Point { pub x: f64, pub y: f64 }

// main.rs
mod math;
use math::{add, sub, Point};
use math as math_mod;
```

### 3.5 模块级配置 (omnilang.toml)

```toml
[project]
name = "my-app"
version = "0.2.0"

[modules]
entry = "src/main.omni"
output_dir = "dist/"

[modules.resolve]
# 路径别名
"@components/" = "src/components/"
"@utils/" = "src/utils/"

[target.js]
module_format = "esm"      # esm | commonjs | iife

[target.dart]
package_name = "my_app"

[target.python]
package_name = "my_app"

[target.rust]
crate_name = "my_app"
```

### 3.6 模块循环依赖检测（工程关键）

模块系统看似"只是语法"，但实际上是整个编译器架构中最容易出问题的部分。循环依赖在不同目标下行为完全不同：

```omnilang
// a.omni
import { b } from "./b";
export fn a() -> i32 { return b() + 1; }

// b.omni
import { a } from "./a";
export fn b() -> i32 { return a() + 1; }
```

| 目标 | 行为 |
|------|------|
| JS (ESM) | "可能能跑"——导出是活绑定，但行为取决于执行顺序 |
| Dart | 编译报错（import 循环） |
| Python | 导入时报 `ImportError`（部分初始化模块） |
| Rust | 编译报错（crate 循环依赖） |

**决策：模块解析阶段强制检测模块级循环依赖，一律报错（`E0501`）。** 不依赖目标语言的宽容行为。对函数级的相互递归（在同模块内或通过已完全加载的模块），则不做限制。

**Analyze 规则 `E0501`（module_cycle）：**

```
Error: E0501 (module_cycle)
  --> detected circular dependency:
   |
   |  src/a.omni → src/b.omni → src/c.omni → src/a.omni
   |
   = help: break the cycle by:
   =        1. 提取共享类型到独立模块 (e.g., src/types.omni)
   =        2. 使用接口/抽象反转依赖方向
   =        3. 将相互递归的函数合并到同一模块
```

### 3.7 路径解析的跨平台一致性

模块导入路径在 Windows 和 Unix 上的分隔符差异会导致 CI 失败：

```omnilang
import { foo } from "../lib/foo";  // 在 Windows 上是 ..\lib\foo.omni
```

**规范：** 模块解析器**统一使用 `/` 作为导入路径分隔符**，内部使用 `pathlib.Path` 进行跨平台解析。所有 `.omni` 源码中的 import 路径强制使用 `/`（使用 `\` 会触发 analyze 警告 `S0108`）。

```toml
[modules.resolve]
# 解析策略
strategy = "node"          # node | posix | auto
                            # node: 解析到 .omni 文件或 目录/index.omni
                            # posix: 严格文件路径
                            # auto: 根据操作系统自动选择

# 路径别名 (统一使用 / 分隔符)
"@components/" = "src/components/"
"@utils/" = "src/utils/"
```

---

## 4. 结构体与记录类型

### 4.1 语法定义

```omnilang
// 结构体定义
struct Point {
    x: f64,
    y: f64,
}

struct User {
    id: i32,
    name: String,
    email: String?,           // 可选字段
    age: i32 = 0,             // 默认值
}

// 创建实例 (类型标注可选，可从上下文推导)
let p1: Point = Point { x: 1.0, y: 2.0 };
let p2 = Point { x: 3.0, y: 4.0 };

// 字段访问
let px: f64 = p1.x;

// 结构体更新语法 (spread)
let p3 = Point { ...p1, y: 5.0 };  // 复制 p1，但 y 不同

// 结构体方法 (impl 块)
impl Point {
    fn length() -> f64 {
        return (self.x * self.x + self.y * self.y).sqrt();
    }

    fn add(other: Point) -> Point {
        return Point { x: self.x + other.x, y: self.y + other.y };
    }
}

// 泛型结构体
struct Pair<A, B> {
    first: A,
    second: B,
}

struct Container<T> {
    value: T,
    metadata: Map<String, String>,
}
```

### 4.2 结构类型兼容性

OmniLang v0.2 默认使用**结构类型**（structural typing）：

```omnilang
struct Dog {
    name: String,
    bark() -> String,
}

struct Cat {
    name: String,
    bark() -> String,    // 同一个喵
}

fn pet_name(animal: { name: String }) -> String {
    return animal.name;
}

// Dog 和 Cat 都可以传给 pet_name（结构兼容）
let dog: Dog = Dog { name: "Fido" };
let cat: Cat = Cat { name: "Whiskers" };
pet_name(dog);  // ✅
pet_name(cat);  // ✅
```

如需**名义类型**（nominal typing，类似 Rust 的 newtype），使用 `nominal` 关键字：

```omnilang
nominal struct UserId(i32);     // 名义类型，不与 i32 兼容
nominal struct Meters(f64);     // 不同类型，不能互操作
```

### 4.2.1 结构类型的"意外兼容"问题（重要）

结构类型的灵活性有利有弊。当 `User` 和 `Dog` 都包含 `name: String` 时，它们都可以传给接受 `{ name: String }` 的函数——从结构类型角度看是正确的，但如果开发者**意图**是只接受 `User`，这就变成了一个难以发现的 bug：

```omnilang
struct User { name: String; id: i32; }
struct Dog  { name: String; id: i32; }

fn printName(entity: { name: String }) { ... }

printName(myUser);  // ✅
printName(myDog);   // ✅ — 但语义上可能是个 bug
```

**Analyze 规则 `S0107`（默认关闭，`HINT` 级别）：** 当两个不同的名义类型在同一位置结构兼容时，发出提示：

```
Hint: S0107 (structural_type_coincidence)
  --> src/main.omni:25:5
   |
25 |     printName(myDog);  // Dog matches { name: String } structurally
   |
   = note: Dog and User are structurally compatible here.
   = help: if this is intentional, add #[allow(structural)] to suppress.
   = help: if you meant to restrict to User only, use nominal keyword:
            nominal struct User { ... }
```

在大型项目中启用此规则可以防止一整类逻辑 bug。

### 4.3 各目标转译

**JavaScript:**
```javascript
// 默认：普通对象 + JSDoc 类型注释
/** @type {Point} */
const p1 = { x: 1.0, y: 2.0 };

// 可选：生成 class
class Point {
    constructor(x, y) { this.x = x; this.y = y; }
    length() { return Math.sqrt(this.x ** 2 + this.y ** 2); }
}

// nominal 结构体 → class with Symbol brand
const UserId_brand = Symbol("UserId");
class UserId {
    constructor(value) { this.value = value; }
    static [Symbol.hasInstance](instance) {
        return instance?.constructor?._brand === UserId_brand;
    }
}
```

**Dart:**
```dart
class Point {
    final double x;
    final double y;
    const Point({required this.x, required this.y});

    double length() => math.sqrt(x * x + y * y);
}
```

**Python:**
```python
from dataclasses import dataclass

@dataclass(frozen=True)
class Point:
    x: float
    y: float

    def length(self) -> float:
        return (self.x ** 2 + self.y ** 2) ** 0.5

# nominal → NewType
from typing import NewType
UserId = NewType("UserId", int)
```

**Rust:**
```rust
#[derive(Debug, Clone)]
struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

// nominal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UserId(i32);
```

---

## 5. 模板字符串

### 5.1 语法定义

```omnilang
let name: String = "Alice";
let age: i32 = 30;

// 字符串插值
let greeting: String = `Hello, ${name}!`;

// 表达式插值
let message: String = `${name} is ${age} years old. Next year: ${age + 1}.`;

// 多行字符串
let html: String = `
    <div class="user">
        <h1>${name}</h1>
        <p>Age: ${age}</p>
    </div>
`;

// 嵌套插值
let nested: String = `Result: ${if age >= 18 { "adult" } else { "minor" }}`;
```

### 5.2 各目标转译

| 目标 | 转译结果 |
|------|---------|
| JS | 原生模板字符串 `` `Hello, ${name}!` `` |
| Dart | `'Hello, $name!'` 或 `'Hello, ${name}!'` |
| Python | `f"Hello, {name}!"` (Python 3.6+) |
| Rust | `format!("Hello, {}!", name)` |

**特殊处理：**
- 多行字符串：Python 用 `"""..."""`，Rust 保留 `\n`，JS/Dart 原生支持
- 嵌套引号：自动转义
- 性能优化：单变量无表达式时，Dart 用 `'$var'` 而非 `'${var}'`

---

## 6. 联合类型与可选类型

### 6.1 语法定义

```omnilang
// 类型别名
type JsonValue = String | i32 | f64 | bool | Array<JsonValue> | Map<String, JsonValue> | null;

// 可选类型（语法糖：T? 等价于 T | null）
let name: String? = null;
let age: i32? = 42;

// 带标签的联合类型 (tagged union / discriminated union)
type Result<T, E> = Ok { value: T } | Err { error: E };

type Option<T> = Some { value: T } | None;

// 字符串字面量类型
type Status = "active" | "inactive" | "pending";
type HttpMethod = "GET" | "POST" | "PUT" | "DELETE";

// 联合类型作为函数参数
fn process(value: String | i32 | Array<String>) {
    match value {
        is String => println(`String: ${value}`),
        is i32 => println(`Integer: ${value}`),
        is Array<String> => println(`Array with ${value.len()} items`),
    }
}

// 返回值
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        return Err { error: "division by zero" };
    }
    return Ok { value: a / b };
}
```

#### `Option<T>` vs `T?` — 语义区别与选择规则

v0.2 提供了两种"可能缺失的值"表达方式，它们有明确的语义分工：

| 特性 | `T?`（语法糖 `T \| null`） | `Option<T>`（tagged union） |
|------|--------------------------|---------------------------|
| 本质 | 扁平联合类型 | 带标签的枚举（`Some(value) \| None`） |
| 推荐场景 | 简单的"值或缺席" | 需要函数式链式操作 |
| JS 映射 | `T \| null` | `{ _tag: "Some", value } \| { _tag: "None" }` |
| Rust 映射 | `Option<T>`（自动转换） | 直接映射 `std::Option<T>` |
| 可选链 `?.` | ✅ 支持 | ❌ 不支持（走 match） |
| 空值合并 `??` | ✅ 支持 | ❌ 不支持 |
| `map`/`and_then` | ❌ 不支持 | ✅ 支持 |

**选择规则：**

```omnilang
// 场景 1：简单可空字段 → 使用 T?
struct User {
    name: String,
    email: String?,            // ✅ 简单可空，? 自然表达
}

// 场景 2：需要链式操作 → 使用 Option<T>
fn findUser(id: i32) -> Option<User> {
    // 允许 map/and_then 链
}
let userName: Option<String> = findUser(42).map(|u| u.name);

// 场景 3：所有目标行为一致性关键 → 使用 Option<T>
// 因为 Rust 原生 Option 无 null 概念，跨语言一致性更好

// 场景 4：互操作场景 → 使用 T?
// JS/Dart/Python 生态惯用 null
```

**Analyze 规则 `S0109`（HINT 级别）：** 当检测到 `T?` 用于需要链式操作的场景（如 `.map()`、`.and_then()` 调用）时，建议迁移到 `Option<T>`。反之，当 `Option<T>` 在 JS/Dart 互操作边界使用时，提示 `T?` 可能更合适。

---

### 6.2 模式匹配 (match)

```omnilang
// match 表达式
let result: Result<i32, String> = divide(10, 2);
let message: String = match result {
    Ok { value } => `Success: ${value}`,
    Err { error } => `Error: ${error}`,
};

// match 语句（带代码块）
match result {
    Ok { value } => {
        println(`Got value: ${value}`);
    }
    Err { error } => {
        println(`Error occurred: ${error}`);
    }
}

// 通配符匹配
match status {
    "active" => println("Active"),
    _ => println("Other"),          // 默认分支
}

// 带守卫的匹配
match value {
    is i32 if value > 0 => println("positive"),
    is i32 if value < 0 => println("negative"),
    is i32 => println("zero"),
    _ => println("not an integer"),
}

// 穷尽性检查 (analyze 确保所有变体都被处理)
```

> **分隔符风格：** match arm 之间使用逗号分隔（必选），最后一个 arm 后可选的尾逗号。此风格与 OmniLang 数组字面量 `[1, 2, 3,]` 和字典字面量 `{"a": 1,}` 一致。EBNF 规则：`match_arm ("," match_arm)* [","]`。

### 6.3 可选链与空值合并

```omnilang
let user: User? = findUser(42);

// 可选链 (?.)
let name: String? = user?.name;
let city: String? = user?.address?.city;

// 空值合并 (??)
let displayName: String = user?.name ?? "Unknown";
let count: i32 = user?.loginCount ?? 0;
```

### 6.4 类型收窄 (Type Narrowing)

```omnilang
fn process(value: String | i32) {
    // 类型收窄：在 if 分支中，value 的类型被收窄
    if value is String {
        // 此块中 value: String
        println(value.to_upper());
    } else {
        // 此块中 value: i32
        println(value.to_string());
    }
}

fn getLength(value: String | Array<i32> | null) -> i32 {
    if value is null {
        return 0;
    }
    // value: String | Array<i32> (null 已被排除)
    return value.len();
}
```

### 6.5 各目标转译

**JavaScript (可选输出 TypeScript):**
```typescript
type JsonValue = string | number | boolean | JsonValue[] | Map<string, JsonValue> | null;
type Result<T, E> = { _tag: "Ok"; value: T } | { _tag: "Err"; error: E };

function process(value: string | number | string[]): void {
    if (typeof value === "string") {
        console.log(`String: ${value}`);
    } else if (typeof value === "number") {
        console.log(`Integer: ${value}`);
    } else {
        console.log(`Array with ${value.length} items`);
    }
}
```

**Dart (使用 sealed class 模式):**
```dart
sealed class Result<T, E> {}
class Ok<T, E> extends Result<T, E> { final T value; Ok(this.value); }
class Err<T, E> extends Result<T, E> { final E error; Err(this.error); }

String process(Object value) {
    return switch (value) {
        String s => 'String: $s',
        int i => 'Integer: $i',
        List<String> arr => 'Array with ${arr.length} items',
        _ => throw ArgumentError('Unexpected type'),
    };
}
```

**Python:**
```python
from typing import TypeAlias
JsonValue: TypeAlias = str | int | float | bool | list['JsonValue'] | dict[str, 'JsonValue'] | None

def process(value: str | int | list[str]) -> None:
    match value:  # Python 3.10+ structural pattern matching
        case str():
            print(f"String: {value}")
        case int():
            print(f"Integer: {value}")
        case list():
            print(f"Array with {len(value)} items")
```

**Rust (使用 enum):**
```rust
enum JsonValue {
    String(String),
    Integer(i32),
    Float(f64),
    Boolean(bool),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
    Null,
}

enum Result<T, E> {
    Ok { value: T },
    Err { error: E },
}

fn process(value: &JsonValue) {
    match value {
        JsonValue::String(s) => println!("String: {}", s),
        JsonValue::Integer(i) => println!("Integer: {}", i),
        JsonValue::Array(arr) => println!("Array with {} items", arr.len()),
        _ => {}
    }
}
```

---

## 7. 泛型系统

### 7.1 语法定义

```omnilang
// 泛型函数
fn identity<T>(x: T) -> T {
    return x;
}

fn first<T>(arr: Array<T>) -> T? {
    if arr.is_empty() {
        return null;
    }
    return arr[0];
}

// 泛型结构体
struct Box<T> {
    value: T,
}

struct Pair<K, V> {
    key: K,
    value: V,
}

// 泛型约束 (trait bounds)
fn compare<T: Eq>(a: T, b: T) -> bool {
    return a == b;
}

fn display<T: Display>(value: T) -> String {
    return value.to_string();
}

fn process<T: Display + Eq>(items: Array<T>) -> String {
    // T 必须同时满足 Display 和 Eq
    if items.len() == 0 { return "empty".to_string(); }
    return items[0].to_string();
}

// 泛型方法
impl<T> Box<T> {
    fn unwrap() -> T {
        return self.value;
    }

    fn map<U>(f: fn(T) -> U) -> Box<U> {
        return Box { value: f(self.value) };
    }
}

// where 子句 (复杂约束)
fn complex<T, U>(a: T, b: U) -> Map<String, String>
where T: Display,
      U: Display + Clone
{
    // ...
}
```

### 7.2 内置 Trait

| Trait | 方法/能力 | Copy | 适用类型 |
|-------|----------|------|---------|
| `Eq` | `==`, `!=` | — | i32, f64, bool, String |
| `Ord` | `<`, `>`, `<=`, `>=` | — | i32, f64 |
| `Display` | `to_string()` | — | 所有基本类型 |
| `Clone` | `clone()` | — | String, Array, Map, 结构体 |
| `Default` | 默认值 | — | 带默认值的结构体 |
| `Hash` | map 键 | — | i32, bool, String |

### 7.3 各目标转译

**JavaScript (类型擦除):**
```javascript
// 泛型在 JS 中完全擦除（除非输出 TypeScript）
function identity(x) { return x; }
function first(arr) { return arr.length > 0 ? arr[0] : null; }
```

**JavaScript (TypeScript 模式):**
```typescript
function identity<T>(x: T): T { return x; }
function first<T>(arr: T[]): T | null { return arr.length > 0 ? arr[0] : null; }
```

**Dart:**
```dart
T identity<T>(T x) => x;
T? first<T>(List<T> arr) => arr.isEmpty ? null : arr[0];

class Box<T> {
    final T value;
    Box(this.value);
    T unwrap() => value;
    Box<U> map<U>(U Function(T) f) => Box(f(value));
}
```

**Python:**
```python
from typing import TypeVar, Generic

T = TypeVar('T')
K = TypeVar('K')
V = TypeVar('V')

def identity(x: T) -> T: return x
def first(arr: list[T]) -> T | None: return arr[0] if arr else None

class Box(Generic[T]):
    def __init__(self, value: T): self.value = value
    def unwrap(self) -> T: return self.value
    def map(self, f, U): return Box[U](f(self.value))
```

**Rust:**
```rust
fn identity<T>(x: T) -> T { x }
fn first<T>(arr: &[T]) -> Option<T> where T: Clone { arr.first().cloned() }

struct Box<T> { value: T }
impl<T> Box<T> {
    fn unwrap(self) -> T { self.value }
    fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Box<U> {
        Box { value: f(self.value) }
    }
}
```

### 7.4 JS 类型擦除的运行时安全问题

JS `plain` 模式下泛型完全擦除，当泛型返回值跨模块边界传递时，运行时类型信息丢失：

```omnilang
// lib.omni
export fn first<T>(arr: Array<T>) -> T? { ... }

// main.omni  
import { first } from "./lib";

let result = first<User>(myUsers);
// result 的类型在 JS 中只是普通 object，无运行时保护
// 与 JS 生态中的其他库交互时，类型错误不会被捕获
```

**建议策略：**

| JS 模式 | 类型保护 | 建议场景 |
|---------|---------|---------|
| `plain` | 无运行时类型 | 内部项目、全量 OmniLang 代码 |
| `typescript` | 编译期类型检查（`.ts` 文件）, 运行时无 | 与 TS 生态互操作 |
| `plain` + 运行时校验 | 手动添加 `instanceof` 或 tag 检查 | 跨语言库发布 |

**Analyze 规则 `P0111`：** 当检测到 JS `plain` 模式下的泛型函数返回类型跨模块边界时，发出 PORT 级别警告：

```
Port: P0111 (generics_erasure_js_plain)
  --> src/main.omni:15:12
   |
15 |     let result = first<User>(myUsers);
   |                  ^^^^^^^^^^^^^^^^^^^^^
   |                  JS plain 模式下泛型返回类型被完全擦除
   |
   = help: 切换到 typescript 模式保留类型信息: [target.js] mode = "typescript"
   = help: 或添加运行时类型校验: if (result && typeof result.name === "string") { ... }
```

---

## 8. 解构、展开、剩余参数

### 8.1 语法定义

```omnilang
// ── 数组解构 ──
let arr: Array<i32> = [1, 2, 3, 4, 5];
let [a, b] = arr;                    // a=1, b=2
let [first, ...rest] = arr;          // first=1, rest=[2,3,4,5]
let [x, _, z] = arr;                 // x=1, z=3 (_ 跳过)
let [h, ..._, t] = arr;              // h=1, t=5

// ── 结构体解构 ──
let user = User { name: "Alice", age: 30, email: null };
let { name, age } = user;            // name="Alice", age=30
let { name: userName, age: userAge } = user;  // 重命名
let { name, ...rest } = user;        // name="Alice", rest={age: 30, email: null}

// ── 函数参数解构 ──
fn printUser({ name, age }: User) {
    println(`${name} is ${age}`);
}

fn sum(...args: Array<i32>) -> i32 {
    let total: i32 = 0;
    for n in args {
        total += n;
    }
    return total;
}

// ── 展开操作符 ──
let arr1: Array<i32> = [1, 2, 3];
let arr2: Array<i32> = [4, 5, 6];
let combined: Array<i32> = [...arr1, ...arr2, 7, 8];  // [1,2,3,4,5,6,7,8]

let defaults = { theme: "dark", lang: "en" };
let settings = { ...defaults, lang: "zh" };           // { theme: "dark", lang: "zh" }

// 函数调用展开
fn add3(a: i32, b: i32, c: i32) -> i32 { return a + b + c; }
let nums = [1, 2, 3];
let result = add3(...nums);  // add3(1, 2, 3)
```

### 8.2 各目标转译

| 特性 | JS | Dart | Python | Rust |
|------|-----|------|--------|------|
| 数组解构 | `const [a, b] = arr` | `final [a, b] = arr` | `a, b = arr[0], arr[1]` | `let (a, b) = (arr[0], arr[1])` |
| `...rest` 数组 | `const [f, ...r] = arr` | `final [f, ...r] = arr` | `f, *r = arr` | 手动解构 |
| 结构体解构 | `const {name, age} = obj` | 手动提取 | 手动提取 | `let User {name, age} = user` |
| `...spread` 数组 | `[...a, ...b]` | `[...a, ...b]` | `[*a, *b]` | `[a.as_slice(), b.as_slice()].concat()` |
| `...spread` 结构体 | `{...a, ...b}` | `{...a, ...b}` | `{**a, **b}` | 手动 merge |
| `...rest` 参数 | `function(...args)` | `fn(..., [rest])` | `def fn(*args)` | 使用 slice |

---

## 9. 类、继承、装饰器

### 9.1 语法定义

```omnilang
// 类定义
class Animal {
    name: String;
    age: i32;

    // 构造函数
    new(name: String, age: i32) {
        this.name = name;
        this.age = age;
    }

    // 方法
    fn speak() -> String {
        return "...";
    }

    // getter (只读属性)
    get description() -> String {
        return `${this.name} (${this.age})`;
    }

    // setter
    set age(val: i32) {
        if val < 0 { return; }
        this.age = val;
    }

    // 静态方法
    static fn species() -> String {
        return "Unknown";
    }
}

// 继承
class Dog extends Animal {
    breed: String;

    new(name: String, age: i32, breed: String) {
        super.new(name, age);
        this.breed = breed;
    }

    // 方法覆盖
    @override
    fn speak() -> String {
        return `${this.name} says woof!`;
    }

    // 静态方法覆盖
    @override
    static fn species() -> String {
        return "Canis familiaris";
    }
}

// 接口实现
interface Printable {
    fn print() -> void;
    fn toPrintable() -> String;
}

class Document implements Printable {
    content: String;

    new(content: String) {
        this.content = content;
    }

    fn print() -> void {
        println(this.content);
    }

    fn toPrintable() -> String {
        return this.content;
    }
}

// 装饰器
@deprecated("Use newMethod instead")
fn oldMethod() { }

@sealed
class FinalClass { }

@readonly
let x: i32 = 42;
```

### 9.2 内置装饰器

| 装饰器 | 作用 | v0.2 |
|--------|------|------|
| `@override` | 标记方法覆盖 | ✅ |
| `@deprecated(msg)` | 标记已弃用 | ✅ |
| `@readonly` | 变量只读 | ✅ |
| `@sealed` | 禁止继承 | ✅ |
| `@memoize` | 缓存函数结果 | ✅ |
| `@debug` | 打印调用信息 | ✅ |

### 9.3 各目标转译

**JavaScript:**
```javascript
class Animal {
    #name; #age;
    constructor(name, age) { this.#name = name; this.#age = age; }
    get description() { return `${this.#name} (${this.#age})`; }
    speak() { return "..."; }
    static species() { return "Unknown"; }
}

class Dog extends Animal {
    #breed;
    constructor(name, age, breed) { super(name, age); this.#breed = breed; }
    speak() { return `${this.name} says woof!`; }
}
```

**Dart:**
```dart
class Animal {
    final String name;
    int _age;
    Animal(this.name, this._age);
    String get description => '$name ($_age)';
    String speak() => '...';
    static String species() => 'Unknown';
}

class Dog extends Animal {
    final String breed;
    Dog(super.name, super.age, this.breed);
    @override String speak() => '$name says woof!';
}
```

**Python:**
```python
class Animal:
    def __init__(self, name: str, age: int):
        self._name = name
        self._age = age

    @property
    def description(self) -> str:
        return f"{self._name} ({self._age})"

    def speak(self) -> str:
        return "..."

class Dog(Animal):
    def __init__(self, name: str, age: int, breed: str):
        super().__init__(name, age)
        self.breed = breed

    def speak(self) -> str:
        return f"{self._name} says woof!"
```

**Rust:**
```rust
struct Animal {
    name: String,
    age: i32,
}

impl Animal {
    fn new(name: String, age: i32) -> Self { Self { name, age } }
    fn description(&self) -> String { format!("{} ({})", self.name, self.age) }
    fn speak(&self) -> String { "...".to_string() }
}

struct Dog {
    animal: Animal,  // composition over inheritance
    breed: String,
}

impl Dog {
    fn new(name: String, age: i32, breed: String) -> Self {
        Self { animal: Animal::new(name, age), breed }
    }
    fn speak(&self) -> String { format!("{} says woof!", self.animal.name) }
}
```

> **⚠️ 设计说明——Rust 类继承的诚实支持边界：** Rust 没有类继承。OmniLang 类在 Rust 目标下默认转为**组合模式**（struct + trait impl）。但这不是一个无损映射——以下表格明确标注了各场景的支持程度，请勿声称"完全支持"而在实际使用中挖坑：

### Rust 类继承支持边界

| 场景 | 支持程度 | 实现方式 | 注意事项 |
|------|---------|---------|---------|
| 简单数据类（无继承） | ✅ 完全支持 | `struct` + `impl` | 直接映射，无损失 |
| 单继承 + 方法覆盖（无多态） | ✅ 支持 | trait + 组合 | 静态分派，无虚表 |
| 多态参数传递（父类型引用接收子类型） | ⚠️ 有限支持 | `&dyn Trait` trait 对象 | 有运行时开销，需额外 `Box` 包装 |
| `super` 调用 | ⚠️ 有限支持 | 显式委托 | 不支持自动 `super.method()` 转发 |
| 多重继承 | ❌ 不支持 | — | Rust 无等效语义；analyze 报错 `E0401` |
| `instanceof` / `is` 运行时类型检查 | ⚠️ 有限支持 | 需额外实现 `Any` trait 或手动 tag | 性能开销，且不同目标的 `is` 语义不一致 |
| Mixin / 混入 | ❌ 不支持 | — | 建议使用接口 + 组合替代 |

**多态调用的示意对比：**

```omnilang
// OmniLang 源码
fn animalSound(a: Animal) -> String {
    return a.speak();  // 多态调用
}
animalSound(myDog);  // Dog extends Animal
```

| 目标 | 生成代码 | 分派方式 |
|------|---------|---------|
| JS | `a.speak()` | 动态分派（原型链） |
| Dart | `a.speak()` | 动态分派（虚表） |
| Python | `a.speak()` | 动态分派 |
| Rust | `a.speak()` (需定义 `Animal` trait，`a: &dyn Animal`) | 动态分派（`dyn` 虚表），有 `Sized` 限制 |

Rust 目标下如果用户需要多态，analyze 规则 `E0402` 会提示：生成的代码使用 `&dyn Trait`，请注意无法用于 `Sized` 约束的泛型上下文。

---

## 10. 结构类型系统详解

### 10.1 核心原则

OmniLang v0.2 默认使用**结构类型**（structural typing）：类型的兼容性由**形状**（字段和方法签名）决定，而非名称。

```omnilang
// 两个独立定义的类型
struct Point2D { x: f64, y: f64 }
struct Vector2D { x: f64, y: f64 }

fn distance(p: { x: f64, y: f64 }) -> f64 {
    return (p.x * p.x + p.y * p.y).sqrt();
}

let pt: Point2D = Point2D { x: 3.0, y: 4.0 };
let vec: Vector2D = Vector2D { x: 1.0, y: 1.0 };
distance(pt);   // ✅ 结构兼容
distance(vec);  // ✅ 结构兼容
```

### 10.2 结构化类型标注

```omnilang
// 匿名结构类型
fn process(data: { name: String, age: i32 }) -> String {
    return data.name;
}

// 内联类型标注
let config: { host: String, port: i32, ssl: bool } = {
    host: "localhost",
    port: 8080,
    ssl: false,
};

// 函数类型作为字段
type Handler = {
    handle: fn(Request) -> Response,
    name: String,
};
```

### 10.3 名义类型转换

当需要打破结构兼容性时：

```omnilang
// nominal 关键字创建名义类型
nominal struct Email(String);     // 与 String 不兼容
nominal struct UserId(i32);      // 与 i32 不兼容

let email: Email = Email("user@example.com");
let id: UserId = UserId(42);

fn sendEmail(to: Email) { /* ... */ }
sendEmail(email);   // ✅
sendEmail("test");  // ❌ 编译错误：String !== Email
sendEmail(id);      // ❌ 编译错误：UserId !== Email
```

**`nominal` 两种语法形式均支持：**

```omnilang
// 形式 1：Newtype 包装（单字段括号形式，如 Rust newtype）
nominal struct UserId(i32);       // 只能通过显式构造/解构访问内部值
nominal struct Meters(f64);

let id: UserId = UserId(42);
let raw: i32 = id.0;             // 解构访问（仅 Dart/Rust/Python），JS 通过 getter

// 形式 2：多字段名义结构体（花括号形式）
nominal struct User {
    name: String;
    id: i32;
}
// 字段访问正常，但整体类型不与其他结构兼容
let user: User = User { name: "Alice", id: 1 };
let name: String = user.name;   // ✅ 字段访问正常
fn register(u: User) { }        // ✅ 只接受 User
register({ name: "Bob", id: 2 }); // ❌ 匿名对象字面量不兼容（名义类型）
```

| 形式 | 语法 | 用途 | Rust 映射 |
|------|------|------|-----------|
| Newtype | `nominal struct Name(Type)` | 类型安全包装 | `struct Name(Type)` (tuple struct) |
| 多字段 | `nominal struct Name { ... }` | 领域对象，禁止结构兼容 | `struct Name { ... }` (有字段名) |

### 10.4 Analyze 中的结构类型检查

`omnic check` 提供结构类型分析：

```
Warning: structural type mismatch [E0301]
  --> src/main.omni:20:5
   |
20 |     process({ host: "localhost" });
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |     missing required field `port: i32`
   |
   = help: add `port: 8080` to the object literal
   = note: expected shape: { host: String, port: i32, ssl: bool }
```

---

## 11. 转译策略（详细）

### 11.1 策略总览

| 特性 | JS 策略 | Dart 策略 | Python 策略 | Rust 策略 |
|------|---------|-----------|-------------|-----------|
| async/await | 原生 Promise | 原生 Future | asyncio | tokio |
| 闭包 | 箭头函数 | 闭包 | lambda/def | 闭包 |
| 模块 | ES modules | import | import | mod/use |
| 结构体 | object/class | class | @dataclass | struct |
| 模板字符串 | 原生 | 原生 | f-string | format!() |
| 联合类型 | TS union / 运行时 | sealed class | Union / match | enum |
| 泛型 | 擦除 / TS | 原生泛型 | typing.Generic | 原生泛型 |
| 类 | class | class | class | struct+trait |
| 继承 | extends | extends | 继承 | trait+组合 |
| 装饰器 | 实验性 | 注解 | 装饰器 | 属性宏 |

### 11.2 JavaScript 转译细节

#### 模式选择
```toml
[target.js]
mode = "plain"       # plain | typescript
module = "esm"       # esm | commonjs
private_fields = true # 使用 # 私有字段
```

#### 类型处理
- `plain` 模式：类型全部擦除，生成 JSDoc 注释
- `typescript` 模式：生成 `.ts` 文件，保留类型

#### 运行时辅助
- `_idiv` / `_irem` 按需生成（保留 v0.1 行为）
- 新增：`Option<T>` → `T | null`，`Result<T,E>` → discriminated union

### 11.3 Dart 转译细节

#### 空安全
- Dart 3.x 强制空安全
- `T?` → `T?` (Dart nullable)
- `map[k]` → `map[k]!` 或 `map[k] ?? (throw ...)`

#### 类型系统
- 泛型保留，Dart 泛型系统与 OmniLang 高度兼容
- `struct` → `class` with `const` constructor
- `nominal` → 密封类或包装类

### 11.4 Python 转译细节

#### 版本目标
- Python 3.10+（利用 pattern matching 和 `|` 类型语法）
- 异步：asyncio (3.7+)

#### 数据类型
- `@dataclass(frozen=True)` 用于不可变结构体
- `NewType` 用于名义类型
- `TypedDict` 用于匿名结构类型（如果需要运行时类型检查）

#### 辅助函数
- 保留 v0.1 的 `_idiv` / `_irem`
- 新增：`_Option` / `_Result` 运行时辅助（如果不开 typing 模式）

### 11.5 Rust 转译细节

#### 异步运行时
```toml
[target.rust.async]
runtime = "tokio"     # tokio | async-std
version = "1"
features = ["full"]
```

#### 所有权处理
- v0.2 **不再强制** Rust 所有权模型到其他语言
- Rust 后端仍然生成正确的所有权代码
- `analyze` 工具标记 Rust 特定问题（生命周期、移动等）

#### 错误处理
- `Result<T, E>` → `Result<T, E>` (Rust 原生)
- `Option<T>` → `Option<T>` (Rust 原生)
- `panic` → `panic!`
- `try/catch` 风格 → `?` 操作符

---

## 12. Analyze 分析系统 (`omnic check`)

这是 v0.2 的**核心差异化能力**。对标 `cargo check` + `cargo clippy` + `rust-analyzer`。

### 12.1 架构

```
omnic check <file.omni>
    │
    ├─ Phase 1: 词法/语法分析 → AST
    ├─ Phase 2: 符号解析 → 作用域树 + 引用图
    ├─ Phase 3: 类型检查 → 类型映射 + 约束图
    ├─ Phase 4: 模块解析 → 依赖图 + 循环检测
    ├─ Phase 5: Lint 引擎 → 诊断信息收集
    ├─ Phase 6: 跨语言分析 → 可移植性评估
    ├─ Phase 7: 修复建议 → 自动修复方案
    └─ Phase 8: 诊断输出 → 终端 / JSON / LSP
```

### 12.2 诊断级别

```
┌─────────────────────────────────────────────────────┐
│ Level   │ 含义                    │ 默认行为        │
├─────────┼─────────────────────────┼─────────────────┤
│ ERROR   │ 必定导致编译失败           │ 终止，需修复    │
│ WARNING │ 很可能导致运行时错误       │ 输出，可配置    │
│ PORT    │ 跨语言行为不一致           │ 输出，建议修复  │
│ STYLE   │ 代码风格建议              │ 输出，可选修复  │
│ PERF    │ 性能建议                  │ 输出，可选修复  │
│ HINT    │ 最佳实践建议              │ 默认关闭       │
└─────────────────────────────────────────────────────┘
```

### 12.3 诊断分类体系

#### A. 类型错误 (E01xx)
```
E0101: 类型不匹配
E0102: 缺少必需字段
E0103: 多余字段
E0104: 泛型约束不满足
E0105: 联合类型穷尽性检查失败
E0106: 类型推导歧义
E0107: 递归类型无终止
E0108: 违反 nominal 类型边界
```

#### B. 异步问题 (E02xx)
```
E0201: await 用于非 async 函数中
E0202: async 函数缺少 await
E0203: 异步函数未处理 rejection/error
E0204: 异步操作顺序可能导致竞态
E0205: 缺少 await（可能忘记）
E0206: 不必要的 async
```

#### C. 所有权/生命周期 (E03xx)
```
E0301: Rust 目标下变量移动后使用
E0302: 借用冲突 (Rust 目标)
E0303: 闭包生命周期不足 (Rust 目标)
E0304: 跨语言所有权不一致
E0305: 非 Copy 类型的浅拷贝警告
```

#### D. 可移植性 (P01xx) — OmniLang 独有

P01xx 规则集是 v0.2 **成功的关键**。它承认跨语言语义差异不可避免，但不放任用户踩坑。以下是按优先级排序的完整规则集：

##### P0 级（最高优先级——最常见、最隐蔽的 bug）

| 规则 | 描述 | 示例 | 差异说明 |
|------|------|------|---------|
| **P0101** | 整数除法行为不一致 | `a / b` (i32) | JS→返回 float；Dart→截断向零；Python→向负无穷取整；Rust→截断向零 |
| **P0109** | 整数溢出行为不同 | `a + b` 溢出 i32 范围 | JS→静默转为 float；Dart→报错(checked mode)；Python→自动升级 BigInt；Rust→panic(debug) / wrap(release) |

##### P1 级（高优先级——高频场景）

| 规则 | 描述 | 示例 | 差异说明 |
|------|------|------|---------|
| **P0102** | 浮点比较不一致 | `a == b` (f64) | IEEE754 实现细节各语言不同，极小误差可能被放大 |
| **P0103** | NaN 处理不同 | `NaN == NaN` | JS→`false`（`NaN !== NaN`）；Rust→`false`（`f64::NAN != f64::NAN`）；但 `isNaN()` 行为各异 |
| **P0110** | 除零行为不同 | `a / 0` (i32) | JS→`Infinity`（不抛异常）；Dart→抛出 `IntegerDivisionByZeroException`；Python→`ZeroDivisionError`；Rust→panic |
| **P0111** | JS plain 模式泛型擦除 | 跨模块传递泛型返回值 | 运行时类型信息完全丢失（详见 §7.4） |

##### P2 级（中优先级——v0.2 新增特性相关）

| 规则 | 描述 | 示例 | 差异说明 |
|------|------|------|---------|
| **P0106** | 闭包捕获语义差异 | 闭包捕获可变变量 | JS/Dart/Python→引用捕获；Rust→move + Cell/RefCell 包装（详见 §2.3） |
| **P0108** | Map 迭代顺序不同 | `for k in map` | Python 3.7+→插入序；JS Map→插入序；Dart→插入序；Rust HashMap→**无保证顺序** |
| **P0107** | 异步调度顺序差异 | 多个 await 的微任务调度 | JS→microtask queue；Dart→microtask queue；Python→asyncio event loop；Rust→tokio scheduler |

##### P3 级（低优先级——边缘场景）

```
P0104: 字符串编码假设不同 (UTF-16 vs UTF-8)
P0105: 正则表达式语法不同
```

**实施建议：** P0 和 P1 级规则应在 Phase 2.5 (Analyze 核心框架) 中作为首批规则实现，共 6 条规则覆盖最常见的跨语言陷阱。P2 级规则随新特性的 Phase 同步添加。

```

#### E. 性能 (F01xx)
```
F0101: 不必要的容器克隆
F0102: O(n²) 嵌套循环模式
F0103: String 拼接循环（应用 Array.join 或 StringBuilder）
F0104: 闭包不必要地捕获大对象
F0105: 可内联的小函数
F0106: Map 查找可缓存的结果
```

#### F. 风格 (S01xx)
```
S0101: 未使用的变量
S0102: 未使用的导入
S0103: 变量名不符合规范
S0104: 可以简化的表达式
S0105: 不必要的 else
S0106: 可以用模式匹配替代的 if-else 链
```

#### G. 安全 (SEC01xx)
```
SEC0101: 潜在的注入风险（字符串拼接到 HTML/SQL）
SEC0102: 硬编码的密钥/密码
SEC0103: 不安全的随机数生成
```

### 12.4 诊断输出格式

#### 人类可读（终端）
```
┌──── Error: E0101 (type_mismatch)
│
│  cannot assign `String` to variable of type `i32`
│
│  ┌─ src/main.omni:15:20
│  │
│15│     let x: i32 = "hello";
│  │                    ^^^^^^^ expected `i32`, found `String`
│  │
│  ├─ help: if you meant to convert the string to a number, use:
│  │          `"hello".parse::<i32>()` (returns Result<i32, ParseError>)
│  │
│  └─ note: this error prevents transpilation to all targets
│
│  [E0101] 1 error found (fix with: omnic fix --apply E0101)
│
└──────────────────────

┌──── Port: P0101 (cross_lang_div_behavior)
│
│  integer division behavior differs across targets
│
│  ┌─ src/logic.omni:42:15
│  │
│42│     let result = a / b;  // a and b are i32
│  │               ^^^^^
│  │               JS: returns float (e.g., 5/2 → 2.5)
│  │               Dart: truncates (e.g., 5 ~/ 2 → 2)
│  │               Python: floors (e.g., -5/2 → -3)
│  │               Rust: truncates toward zero (e.g., -5/2 → -2)
│  │
│  ├─ help: use `_idiv(a, b)` from 'omni/math' to guarantee
│  │        truncation toward zero on all targets
│  │
│  └─ suppress: #[allow(portability::cross_lang_div_behavior)]
│
└──────────────────────
```

#### JSON 格式（CI/工具链）
```json
{
  "diagnostics": [
    {
      "code": "E0101",
      "severity": "error",
      "category": "type",
      "message": "cannot assign `String` to variable of type `i32`",
      "location": {
        "file": "src/main.omni",
        "line": 15,
        "column": 20,
        "span": [15, 20, 15, 27]
      },
      "help": "if you meant to convert the string to a number, use `\"hello\".parse::<i32>()`",
      "suggestions": [
        {
          "message": "replace with parse call",
          "replacement": "\"hello\".parse::<i32>()",
          "span": [15, 20, 15, 27]
        }
      ],
      "cross_references": [],
      "related": []
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 0,
    "portability": 0,
    "style": 0,
    "hints": 0
  }
}
```

### 12.5 自动修复 (`omnic fix`)

```
# 应用所有可自动修复的诊断
omnic fix src/

# 仅修复特定规则
omnic fix --rule E0101,P0101 src/

# 预览修复（不修改文件）
omnic fix --dry-run src/

# 交互式修复
omnic fix --interactive src/
```

自动修复能力：
- 重命名变量（解决遮蔽冲突）
- 添加缺失的类型标注
- 替换跨语言不一致的操作符
- 添加/移除 import
- 简化表达式
- 添加 `await` 关键字

### 12.6 LSP 服务器

```toml
# .omnilang.toml
[lsp]
enabled = true
check_on_save = true
check_on_type = true
inline_diagnostics = true
```

支持：
- 悬停类型提示（hover）
- 跳转到定义（go-to-definition）
- 查找引用（find-references）
- 代码补全（autocomplete）
- 重命名符号（rename）
- 内联诊断（实时显示错误和警告）
- 代码操作（quick-fix）

### 12.7 配置 .omnilang.toml

```toml
[analyze]
# 全局级别
level = "port"               # error | warning | port | style | hint | all

# 按规则配置
[analyze.rules]
E0301 = "error"              # 所有权问题始终报错
P0101 = "warning"            # 除法不一致降级为 warning
S0101 = "allow"              # 关闭未使用变量警告

[analyze.target_overrides]
# 对特定目标的特殊规则
[analyze.target_overrides.rust]
E0301 = "error"              # Rust 必须处理所有权
E0302 = "error"

[analyze.target_overrides.js]
E0301 = "allow"              # JS 不需要关心所有权
P0101 = "warning"            # 但除法问题仍需提醒
```

---

## 13. CLI 工具增强

### 13.1 命令总览

```
omnic
├── build <file>              # 编译到指定目标
│   ├── --target <lang>       # rust | dart | js | python | all
│   ├── --output <dir>
│   ├── --release             # 优化模式
│   └── --watch               # 监听文件变化
│
├── check <file>              # 静态分析（不生成代码）
│   ├── --format <fmt>        # human | json | github | gitlab
│   ├── --rules <list>        # 仅检查特定规则
│   └── --explain <code>      # 解释某个诊断码
│
├── fix <file>                # 自动修复
│   ├── --rule <code>
│   ├── --dry-run
│   └── --interactive
│
├── init <dir>                # 初始化新项目
│   └── --template <name>
│
├── run <file>                # 编译并运行（默认目标）
│
├── test                      # 运行测试
│
├── fmt                       # 代码格式化
│
└── lsp                       # 启动 LSP 服务器
```

### 13.2 `omnic check --explain`

类似 `rustc --explain`，提供深度解释：

```
$ omnic check --explain P0101

P0101: cross-language integer division behavior

Integer division (`/` operator on integer types) produces different
results across OmniLang's target languages:

  -5 / 2 in JavaScript:  -2.5  (float division)
  -5 / 2 in Dart:        -2    (truncates toward zero)
  -5 / 2 in Python:      -3    (floors toward negative infinity)
  -5 / 2 in Rust:        -2    (truncates toward zero)

Solution:
  Use `_idiv(a, b)` from the built-in 'omni/math' module, which
  guarantees truncation-toward-zero on all targets.

  import { _idiv } from "omni/math";
  let result = _idiv(a, b);

Alternatively, if you are intentionally relying on target-specific
behavior, add #[allow(portability::cross_lang_div_behavior)] above
the statement.
```

---

## 14. v0.1 → v0.2 迁移

### 14.1 自动迁移工具

```
# 预览迁移变更
omnic migrate --dry-run src/

# 执行迁移
omnic migrate src/

# 仅迁移特定内容
omnic migrate --only=types src/
```

### 14.2 迁移内容

| v0.1 | v0.2 | 说明 |
|------|------|------|
| `let x: i32 = 10;` | 不变 | 完全兼容 |
| `let arr2: Array<i32> = arr1;` | 保持不变，但有 P0305 警告 | 建议显式 `arr1.clone()` |
| `fn greet(name: &str)` | `fn greet(name: String)` | Rust 借用语义改为 OmniLang 语法 |
| `let mut x = ...` | `let mut x = ...` | 不变 |
| 无模块 | 每个文件自动变为模块 | 无导出 = 全私有 |
| `print` 必须是 `String` | `print` 接受任何 `Display` | 自动插入 `.to_string()` |

### 14.3 破坏性变更

1. **所有权语义移除**：`let s2: String = s1;` 在 JS/Python 目标下不再强制深克隆（analyze 工具警告）
2. **类型系统放宽**：不再强制禁止容器嵌套（analyze 工具警告）
3. **float `/` `%`** ：不再一律禁止，analyze 工具标记 portability 问题

---

## 15. 实施路线图

### Phase 0: 基础设施 ✅
- [x] LSP 协议框架
- [x] 诊断系统（Diagnostic, CodeAction, DiagnosticCollection）
- [x] .omnilang.toml 配置解析
- [x] 测试框架扩展

### Phase 1: 核心语法扩展 ✅
- [x] 模板字符串
- [x] 可选链 `?.` 和空值合并 `??`
- [x] 解构 / 展开 / 剩余参数
- [x] 联合类型基础（`T | U` 语法 + `T?` 语法糖）
- [x] 结构体定义和字面量（`struct` + `impl`）

### Phase 2: 函数增强 ✅
- [x] 闭包和箭头函数（含捕获语义契约）
- [x] 泛型函数 + 泛型约束（trait bounds）
- [x] 函数类型标注 `fn(T) -> U`
- [x] Rust 闭包 Cell/RefCell 自动包装

### Phase 2.5: Analyze 核心框架 ✅
- [x] Diagnostic 数据结构 + 渲染管道（terminal + JSON + GitHub）
- [x] 规则注册机制
- [x] P0/P1 规则全部实现
- [x] `omnic check --explain` 基础设施

### Phase 3: 高级类型 ✅
- [x] `match` 表达式和语句（穷尽性检查）
- [x] Tagged union（enum-style）
- [x] 类型收窄（type narrowing via `is`）
- [x] 结构化类型检查 + `nominal` 关键字
- [x] 同步 Analyze 规则：S0107、E0105

### Phase 4: 异步 ✅
- [x] `async fn` / `await` + 各目标异步运行时适配
- [x] 同步 Analyze 规则：E0201–E0207 + P0107
- [x] `all()` / `race()` 内置函数
- [x] `tryCatch<T>()`

### Phase 5: 模块系统 ✅
- [x] `import` / `export` / `import type` 语法 + 模块解析算法
- [x] 依赖图构建 + 循环依赖检测
- [x] 跨平台路径解析 + 各目标模块格式转译
- [x] 同步 Analyze 规则：S0102、S0108

### Phase 6: 类与继承 ✅
- [x] `class` / `extends` / `implements` 语法
- [x] `new` 构造函数 / `super` 调用
- [x] `interface` 关键字 + 接口定义
- [x] 装饰器基础 + Rust 后端边界实现
- [x] 同步 Analyze 规则：E0401、E0402

### Phase 7: Analyze 系统完善 ✅
- [x] 完整 lint 规则集：**60 条规则**（E01/E02/E03/E04/P01/F01/S01/SEC01）
- [x] 自动修复引擎 + `omnic fix --dry-run`
- [x] `omnic check --explain` 完整覆盖（60 条规则）
- [x] 诊断抑制系统（`#[allow/deny]` + 配置文件）

### Phase 8: IDE 集成 ✅
- [x] LSP 服务器（hover / definition / completion / codeAction / references / rename）
- [x] VS Code 扩展（语法高亮 + 内联诊断）
- [x] Tree-sitter 语法高亮

### Phase 9: 测试与文档完善 ✅

> **PyPI 发布已从范围中移除。**

- [x] 测试：126 passed + 80 新 fixtures = 206
- [x] 性能基准测试：275ms pipeline, 288ms analyze
- [x] 文档：语言参考 + 标准库文档 + 迁移指南 + Rust 类继承边界说明

---

## 附录 A: 完整语法 (EBNF 草案)

```ebnf
// ── 顶层 ──
program         : (function_def | struct_def | class_def | interface_def | type_alias | import_stmt | export_stmt)*

// ── 导入/导出 ──
import_stmt     : "import" import_spec ("," import_spec)* "from" STRING_LIT ";"
                | "import" "*" "as" CNAME "from" STRING_LIT ";"
import_spec     : CNAME ("as" CNAME)?
export_stmt     : "export" (function_def | struct_def | class_def | type_alias | var_decl)

// ── 函数 ──
function_def    : [ASYNC] "fn" CNAME [generic_params] "(" [params] ")" ["->" type] block
params          : param ("," param)*
param           : [pattern] ":" type  // pattern 支持解构
generic_params  : "<" generic_param ("," generic_param)* ["where" where_clause]? ">"
generic_param   : CNAME (":" trait_bound)?
where_clause    : where_constraint ("," where_constraint)*
where_constraint: CNAME ":" trait_bound
trait_bound     : CNAME ("+" CNAME)*

// ── 结构体/类 ──
struct_def      : ["nominal"] "struct" CNAME [generic_params] "{" struct_field* "}"
struct_field    : CNAME ":" type ["=" expr] ";"
                    | "fn" CNAME "(" [params] ")" ["->" type] block  // 方法

class_def       : "class" CNAME [generic_params] [extends_clause] [implements_clause] "{" class_member* "}"
extends_clause  : "extends" type
implements_clause: "implements" type ("," type)*
class_member    : struct_field
                | ["static"] "fn" CNAME "(" [params] ")" ["->" type] block
                | "get" CNAME "(" ")" ["->" type] block
                | "set" CNAME "(" param ")" block
                | decorator class_member

// ── 接口 ──
interface_def   : "interface" CNAME [generic_params] "{" interface_member* "}"
interface_member: "fn" CNAME [generic_params] "(" [params] ")" ["->" type] ";"

// ── 类型 ──
type            : union_type
union_type      : struct_type ("|" struct_type)*
struct_type     : base_type ["?"]
base_type       : TYPE_NAME [generic_args]
                | "{" [struct_type_field ("," struct_type_field)* [","] ] "}"
                | "fn" "(" [type ("," type)*] ")" "->" type
                | "(" type ")"
struct_type_field: CNAME ["?"] ":" type
generic_args    : "<" type ("," type)* ">"
type_alias      : "type" CNAME [generic_params] "=" type ";"

// ── 语句 ──
stmt            : var_decl | assign | if_stmt | while_stmt | for_stmt
                | break_stmt | continue_stmt | return_stmt
                | print_stmt | expr_stmt | block_stmt
                | match_stmt | decorate_stmt

decorate_stmt   : "@" CNAME ["(" [expr ("," expr)*] ")"]

// ── 表达式 ──
expr            : async_expr | closure_expr | assign_expr
async_expr      : "await" expr
closure_expr    : "|" [params] "|" ["->" type] (block | expr)
                | "(" params ")" "=>" (block | expr)
assign_expr     : conditional_expr (assign_op conditional_expr)?
assign_op       : "=" | "+=" | "-=" | "*=" | "/=" | "%="

conditional_expr: or_expr ["??" or_expr]
or_expr         : and_expr ("||" and_expr)*
and_expr        : comp_expr ("&&" comp_expr)*
comp_expr       : add_expr (comp_op add_expr)*
comp_op         : "==" | "!=" | "<" | ">" | "<=" | ">=" | "is"
add_expr        : mul_expr (add_op mul_expr)*
add_op          : "+" | "-"
mul_expr        : unary_expr (mul_op unary_expr)*
mul_op          : "*" | "/" | "%"
unary_expr      : unary_op unary_expr | postfix_expr
unary_op        : "-" | "!" | "..."
postfix_expr    : atom_expr (
                    method_call | func_call | index_get
                    | "?." CNAME  // 可选链
                    | "?" "[" expr "]"  // 可选索引
                )*
atom_expr       : literal | if_expr | match_expr | var_ref
                | "(" expr ")"
                | template_string
                | array_lit | map_lit | struct_lit
                | "..." expr  // 展开

template_string : "`" template_part* "`"
template_part   : STRING_CHARS | "${" expr "}"
STRING_CHARS    : /[^`$]+/ | /\$[^{]/

match_expr      : "match" expr "{" match_arm ("," match_arm)* [","] "}"
match_arm       : pattern ["if" expr] "=>" (expr | block)

// ── 模式 ──
pattern         : "_"                                           // 通配符
                | CNAME                                         // 变量绑定
                | literal                                       // 字面量
                | CNAME "{" [pattern_field ("," pattern_field)*] "}" // 结构体
                | "[" [pattern ("," pattern)* ["..." CNAME]?] "]"    // 数组
                | pattern "|" pattern                           // or 模式
                | "(" pattern ")"
pattern_field   : CNAME [":" pattern]

// ── 字面量 ──
struct_lit      : type_name "{" [struct_lit_field ("," struct_lit_field)* [","] ] "}"
struct_lit_field: CNAME ":" expr | "..." expr
```

---

## 附录 B: 标准库扩展

```omnilang
// ── 新增内置模块 ──

// omni/math — 跨语言一致性数学函数
export fn _idiv(a: i32, b: i32) -> i32;
export fn _irem(a: i32, b: i32) -> i32;
export fn clamp(value: i32, min: i32, max: i32) -> i32;
export fn lerp(a: f64, b: f64, t: f64) -> f64;

// omni/async — 异步工具
export fn all<T>(futures: Array<Future<T>>) -> Future<Array<T>>;
export fn race<T>(futures: Array<Future<T>>) -> Future<T>;
export fn delay(ms: i32) -> Future<void>;
export fn timeout<T>(future: Future<T>, ms: i32) -> Future<T?>;

// omni/collections — 集合扩展
export fn range(start: i32, end: i32) -> Array<i32>;
export fn range_step(start: i32, end: i32, step: i32) -> Array<i32>;
export fn zip<A, B>(a: Array<A>, b: Array<B>) -> Array<Pair<A, B>>;
export fn enumerate<T>(arr: Array<T>) -> Array<Pair<i32, T>>;

// omni/string — 字符串工具
export fn split(s: String, delimiter: String) -> Array<String>;
export fn join(arr: Array<String>, delimiter: String) -> String;
export fn trim(s: String) -> String;
export fn to_upper(s: String) -> String;
export fn to_lower(s: String) -> String;
export fn starts_with(s: String, prefix: String) -> bool;
export fn ends_with(s: String, suffix: String) -> bool;
export fn replace(s: String, from: String, to: String) -> String;

// omni/json — JSON 序列化（可选模块）
export fn parse<T>(s: String) -> Result<T, String>;
export fn stringify(value: JsonValue) -> String;

// omni/result — Result 工具
export fn ok<T, E>(value: T) -> Result<T, E>;
export fn err<T, E>(error: E) -> Result<T, E>;
export fn unwrap<T, E>(result: Result<T, E>) -> T;  // panic on error
export fn unwrap_or<T, E>(result: Result<T, E>, default: T) -> T;
export fn map<T, U, E>(result: Result<T, E>, f: fn(T) -> U) -> Result<U, E>;
```

---

## 附录 C: 测试策略

### C.1 测试层次

```
tests/
├── unit/                    # 单元测试
│   ├── parser/              # 解析器
│   ├── semantic/            # 语义分析
│   ├── analyze/             # analyze 规则
│   └── codegen/             # 代码生成（每个目标独立）
│
├── fixtures/                # 测试用例
│   ├── v0.1_compat/         # v0.1 兼容性
│   ├── async/               # 异步测试
│   ├── closures/            # 闭包测试
│   ├── modules/             # 模块测试（多文件）
│   ├── types/               # 高级类型测试
│   ├── classes/             # 类测试
│   ├── generics/            # 泛型测试
│   └── cross_lang/          # 跨语言一致性
│
├── e2e/                     # 端到端测试
│   ├── test_compile.py      # 编译测试
│   ├── test_run.py          # 运行测试
│   └── test_consistency.py  # 一致性测试
│
└── analyze/                 # analyze 系统测试
    ├── test_rules.py        # 规则测试
    ├── test_fix.py          # 自动修复测试
    └── test_lsp.py          # LSP 测试
```

### C.2 关键测试场景

1. **异步一致性**：所有目标的 async/await 产生相同结果
2. **闭包捕获**：验证各目标的闭包捕获语义
3. **结构类型兼容**：验证跨目标的结构类型行为
4. **模块循环依赖**：检测并报告循环
5. **泛型擦除**：JS 目标类型擦除后行为正确
6. **analyze 可移植性**：所有可移植性规则都有对应测试
7. **可移植性诊断（P01xx 专项）**：每个 P0/P1 规则都有独立测试用例，验证在不同目标组合下发出正确诊断

**P01xx 测试用例示例：**

```
tests/analyze/
├── test_P0101_idiv.py        # 整型除法 → 在 --target js 时发出 P0101 警告
├── test_P0109_overflow.py    # 整数溢出 → 在 --target js 时发出 P0109 警告
├── test_P0102_fcmp.py        # 浮点比较 → 所有目标发出 P0102 警告
├── test_P0103_nan.py         # NaN 处理 → --target js 时发出 P0103 警告
├── test_P0110_divzero.py     # 除零 → --target js 时发出 P0110（JS 返回 Infinity）
│                              #         --target rust 时仅有 HINT（Rust panic 已明确）
├── test_P0111_generics_js.py # JS plain 模式泛型擦除 → P0111 警告
└── test_P0106_closure.py     # 闭包捕获语义 → --target rust 时发出 P0106
```

每个测试用例验证：
- 诊断码、严重级别、位置信息正确
- 帮助信息和修复建议指向正确的方向
- 跨目标组合（如 `--target js,rust`）时两条诊断独立输出

---

**本方案是 OmniLang v0.2 的完整设计，包含所有新增语言特性、转译策略、analyze 分析系统和实施路线图。方案在设计上放弃了 v0.1 的"统一语义强制"策略，转为"地道转译 + 分析保障"策略，使 OmniLang 真正具备成为前端开发语言的潜力。**
