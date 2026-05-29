## OmniLang v0.1 最终设计方案（原型实现规范）

### 1. 语言定位与核心原则

OmniLang 是一门以 **Rust 语义为基准** 的极简、强类型、命令式语言，可 1:1 转译为 JavaScript、Dart、Python 和 Rust。语言刻意限制表达能力，以换取 **行为绝对一致** 和 **转译结果始终可编译**。

**三条根本原则：**
- **Rust 优先**：所有运行时语义与 Rust 严格对齐，其他语言通过受限模拟达到等效行为。
- **显式即真理**：无隐式类型转换、隐式可空、隐式拷贝；所有操作在源码中明确表达。
- **安全可静态检查**：编译器在前端拒绝所有跨语言不一致的代码，绝不依赖目标后端的宽松语义。

**程序结构要求：**
- 每个 OmniLang 程序必须包含一个无参的入口函数 `fn main() { ... }`，作为程序执行的起点。该函数可省略返回类型（隐式 `void`），允许无值 `return;`，但不能有返回值表达式。

**运行环境假定：**
- **JavaScript** 目标假定 **Node.js** 环境（使用 `process.stdout.write` / `console.log`）。
- **Dart** 目标假定**命令行脚本**环境（使用 `dart:io` 中的 `stdout`）。
- **Python** 目标为标准 CPython 3.x 解释器。
- **Rust** 目标为标准 Rust 工具链编译的可执行程序。

---

### 2. 词法基础

- **关键字**：`let` `mut` `fn` `void` `if` `else` `while` `for` `in` `break` `continue` `return` `true` `false`（预留：`as` `type` `struct` `enum`）
- **标识符**：词法规则为 `[a-zA-Z_][a-zA-Z0-9_]*`，但由语义检查器**拒绝以下划线 `_` 开头的标识符**（保留给编译器生成的辅助函数）。
- **字面量**：
  - 整数：`42`，`0xFF`，`0o77`，`0b1010`（均为 `i32`）
  - 浮点数：`3.14`，`1e9`，`2.0`（均为 `f64`）——**初版禁止浮点除法与取模**（见 §4.1）。
  - 字符串：`"hello"`，转义支持 `\n \t \\ \"`（类型为 `String`）
  - 布尔：`true` `false`
- **注释**：`// 行注释`，`/* 块注释 */`
- **分隔符**：语句以 `;` 结尾；块由 `{ }` 包围。

---

### 3. 类型系统

#### 3.1 内置类型

| 类型 | 描述 | 可复制？ | Rust 参数 | Rust 局部/返回 | Dart | JS | Python |
|------|------|----------|-----------|----------------|------|----|--------|
| `void` | 无返回值（仅用于函数返回） | — | `()` | `()` | `void` | `undefined` | `None` |
| `i32` | 32 位有符号整数 | 是（Copy） | `i32`（值） | `i32` | `int` | `number` | `int` |
| `f64` | 64 位浮点数 | 是（Copy） | `f64`（值） | `f64` | `double` | `number` | `float` |
| `bool` | 布尔值 | 是（Copy） | `bool`（值） | `bool` | `bool` | `boolean` | `bool` |
| `String` | 不可变、拥有所有权 | 否（非Copy） | `&str` | `String` | `String` | `string` | `str` |
| `Array<T>` | 可增长列表 | 否（非Copy） | `&[T]` | `Vec<T>` | `List<T>` | `Array` | `list` |
| `Map<K,V>` | 哈希表 | 否（非Copy） | `&HashMap<K,V>` | `HashMap<K,V>` | `Map<K,V>` | `Map` | `dict` |

**类型限制与说明：**
- `void` **只能用作函数返回类型**，不能用于变量声明、参数类型或容器元素。
- `Copy` 类型在赋值、传参、索引取值时自动复制，原变量保持有效。
- `String` 不可变，但允许克隆。`Array`/`Map` 的内容可变，但绑定不可变（`let`）时不能整体重新赋值。
- **容器元素类型 `T` 和 `V` 只能是 `i32, f64, bool, String`**。**`Map` 的键类型 `K` 限制为 `i32, bool, String`**（**禁止 `f64` 作为键**，因为 Rust `HashMap` 要求 `Eq` 和 `Hash` 而 `f64` 不满足，且浮点键行为跨语言不一致）。`bool` 作为键在 Python 中因 `bool` 是 `int` 子类，与 `i32` 键存在隐式重叠风险，但在当前强类型限制下安全。初版不支持容器嵌套。
- **容器类型初版不提供 `to_string` 方法**。基本类型和 `String` 的 `to_string` 方法照常提供。
- **整数溢出**初版列为已知局限，不规定具体行为。
- **浮点运算**初版仅支持 `+ - *`（且不允许除以零和取模），浮点除法和取模被禁止。**浮点算术运算溢出或产生特殊值（如 NaN、无穷大）在不同运行时行为可能不同**，初版列为已知局限。

**所有权与赋值规则：**
- **Copy 类型**（`i32`, `f64`, `bool`）：赋值按值复制，原变量仍可用。
- **非 Copy 类型**（`String`, `Array`, `Map`）：
  - 允许从另一个同类型变量（包括局部变量、函数参数、循环变量）初始化，语义为**自动深克隆**。因为元素类型被限制为 Copy 或不可变 `String`，浅拷贝即产生完全独立的容器，无别名变异风险。
    - `String`：Rust 生成 `.to_string()`，其他语言直接赋值。
    - `Array`/`Map`：Rust 生成 `.clone()`；Dart 生成 `List.of(x)` / `Map.of(x)`；JS 生成 `_arrClone(x)` / `_mapClone(x)`；Python 生成 `list(x)` / `dict(x)`。
  - 初始值还可以来自字面量、函数调用返回值或 `if` 表达式。
- 函数参数：`String` / `Array` / `Map` 以不可变借用传递，函数体内只能调用只读方法，但可作为所有权源用于初始化（自动深克隆）。
- 变异方法（`push`, `insert`, 索引赋值等）接收非 Copy 实参时，Rust 后端自动克隆，保证原变量可继续使用。克隆仅对**具名变量或引用**进行，对于**临时值表达式**直接移动所有权。
- **字面量内元素自动克隆**：数组或字典字面量包含非 Copy 变量时，Rust 后端自动克隆。

#### 3.2 变量声明（强制类型标注）
唯一例外是 `for-in` 循环变量。
```omnilang
let x: i32 = 10;
let mut y: f64 = 3.14;
let s: String = "hello";
let s2: String = s;                 // 自动克隆
let arr: Array<i32> = [1, 2];
let arr2: Array<i32> = arr;         // 允许，自动克隆
let map: Map<String, i32> = {"a": 1};
let map2: Map<String, i32> = map;   // 允许，自动克隆
```
- 禁止同一作用域内变量遮蔽；嵌套块允许，Dart 和 Python 后端通过安全重命名解决（见 7.2、7.4）。**`for-in` 循环变量同样适用遮蔽重命名规则**。
- 声明必须带初始化表达式，类型必须与标注一致。

---

### 4. 表达式与运算符

#### 4.1 运算符
- 算术：`+ - * / %`。**整数除法 `/` 和整数取模 `%`** 语义均为**向零截断**。除零导致**运行时 panic**（所有目标语言均抛出异常/错误）。
- **浮点 `f64`** 仅支持 `+ - *`，**禁止 `/` 和 `%`**（编译错误）。
- 比较：`== != < > <= >=`，仅同类型。**浮点比较存在跨语言微小差异的可能，初版列为已知局限**。
- 逻辑：`&& || !`，操作数必须 `bool`。
- 字符串拼接：`String + String -> String`。
- 赋值：`= += -= *= /= %=`，仅对 `mut` 变量。`let` 容器允许索引赋值。

**运算符映射表（目标语言代码生成规则）**

| 运算符 | Rust | Dart | JS | Python |
|--------|------|------|----|--------|
| `a / b`（i32） | `a / b` | `a ~/ b` | `_idiv(a, b)` | `_idiv(a, b)` |
| `a % b`（i32） | `a % b` | `a % b` | `_irem(a, b)` | `_irem(a, b)` |
| `a + b`（String） | `format!("{}{}", a, b)` | `a + b` | `a + b` | `a + b` |
| `x /= y` | `x /= y;` | `x ~/= y;` | `x = _idiv(x, y);` | `x = _idiv(x, y);` |
| `x %= y` | `x %= y;` | `x %= y;` | `x = _irem(x, y);` | `x = _irem(x, y);` |
| `s += t`（String） | `s = format!("{}{}", s, t);` | `s += t;` | `s += t;` | `s += t;` |

**辅助函数定义（按需生成）：**

- **JS 辅助函数**（负零归一化注释在 `_idiv` 中）：
  ```javascript
  function _idiv(a, b) {
      if (b === 0) throw new Error("division by zero");
      const r = Math.trunc(a / b);
      return r === 0 ? 0 : r;  // 将 -0 规范化为 0，保证跨语言一致
  }
  function _irem(a, b) {
      if (b === 0) throw new Error("division by zero");
      return a % b;  // JS 的 % 向零截断，配合除零检查
  }
  ```
- **Python 辅助函数**（除零通过 `ZeroDivisionError` 实现 panic）：
  ```python
  def _idiv(a, b):
      # 除零时抛出 ZeroDivisionError，与 panic 语义一致
      q = abs(a) // abs(b)
      return q if (a < 0) == (b < 0) else -q

  def _irem(a, b):
      # 除零时抛出 ZeroDivisionError
      return a - _idiv(a, b) * b
  ```

#### 4.2 字面量
- 数组：`[expr, ...]`，类型由声明确定。
- 字典：`{key: value, ...}`，类型由声明确定。
- 无字符串插值。

#### 4.3 索引
- `arr[index]`，`map[key]`。越界或键缺失 → **panic**（各后端均保证抛出异常/错误）。

#### 4.4 条件表达式
仅单分支 + else：
```omnilang
let result: i32 = if cond { 1 } else { 2 };
```

#### 4.5 内置输出（特殊语句）
`print(s: String)` 和 `println(s: String)` 是语句，不能用作表达式。**参数必须为 `String` 类型**，由语义检查器验证。

---

### 5. 语句与控制流

#### 5.1 变量声明与赋值
见 3.2。Rust 后端对**所有局部容器变量均生成 `let mut`**。

#### 5.2 if 语句
```omnilang
if x > 0 { ... } else if x == 0 { ... } else { ... }
```

#### 5.3 while 循环
```omnilang
while cond { ... }  // 支持 break/continue
```

#### 5.4 for-in 循环
循环变量类型由 `Array<T>` 自动推导为 `T`：
```omnilang
for item in arr { ... }
```
**支持 `break` 和 `continue`**。

**静态约束：**
1. 禁止修改被迭代容器（变异方法、索引赋值）。
2. 循环变量不可变：禁止重新赋值或调用变异方法。允许调用的只读方法：
   - `i32/f64/bool`：`to_string`
   - `String`：`to_string`
   - `Array` / `Map`：`len`, `is_empty`, `get`, `contains`（初版因禁止嵌套，容器分支暂不可达）

循环变量可用于初始化同类型变量（自动深克隆）或参与表达式。Rust 后端：
- 若 `T` 为非 Copy，`item` 是 `&T`，需要所有权时调用 `.to_string()` 或 `.clone()`。
- 若 `T` 为 Copy，`item` 是 `&T`。**在任何读取 `item` 值的上下文中，若期望类型为 `T`（而非 `&T`），Rust 后端自动解引用（加 `*`）**，如 `let x: i32 = *item;`、`*item + 1`。

#### 5.5 函数定义与调用
- **程序必须包含 `fn main() { ... }` 入口函数**。所有函数必须显式标注返回类型（包括 `void`）。`main` 可省略返回类型，隐式 `void`，允许无值 `return;`，但不能有返回值表达式。
- **参数**：Copy 类型值传递，不可变；`String`/`Array`/`Map` 不可变借用，只能调只读方法，但可作为所有权源（自动深克隆）。
- **返回值**：`return expr;` 或末尾无分号表达式。`void` 函数不能有返回值表达式。
- **函数调用**（Rust 后端特别规则）：调用函数时，Copy 类型实参直接传值；非 Copy 类型实参（`String`/`Array`/`Map`）自动加 `&` 传递不可变借用。例如 `greet(s)` 生成 `greet(&s)`；`process(arr)` 生成 `process(&arr)`。其他语言后端直接传变量。
- **初版不支持闭包、匿名函数、嵌套函数。**

---

### 6. 标准库（初版内置）

#### 6.1 只读 vs 变异方法
- **只读方法**：任何变量均可调用。
- **变异方法**：仅局部 `let`/`let mut` 容器可用。

**完整映射矩阵（对象.方法 → 目标代码）**

| OmniLang 方法 | Rust | Dart | JS | Python |
|---------------|------|------|----|--------|
| `i32.to_string()` | `x.to_string()` | `x.toString()` | `String(x)` | `str(x)` |
| `f64.to_string()` | `x.to_string()` | `x.toString()` | `String(x)` | `str(x)` |
| `bool.to_string()` | `x.to_string()` | `x.toString()` | `String(x)` | `str(x)` |
| `String.to_string()` | `x.to_string()` | `x.toString()` | `String(x)` | `str(x)` |
| `arr.len()` | `arr.len() as i32` | `arr.length`（属性） | `_arrlen(arr)` | `len(arr)` |
| `arr.is_empty()` | `arr.is_empty()` | `arr.isEmpty`（属性） | `_arrisempty(arr)` | `len(arr) == 0` |
| `arr.get(i)` | 若 T 为 Copy：`arr[i as usize]`<br>若 T 为 `String`：`arr[i as usize].to_string()` | `arr[i]`（越界抛异常） | `_arrget(arr, i)` | `arr[i]`（越界抛异常） |
| `arr.push(v)` **[变异]** | `arr.push( clone_arg(v) );` | `arr.add(v);` | `arr.push(v);` | `arr.append(v)` |
| `arr.pop()` **[变异]** | `arr.pop().expect("empty")` | `arr.removeLast();` | `_arrpop(arr)` | `arr.pop()` |
| **索引取值** `arr[i]` | 同 `get(i)` 规则 | `arr[i]` | `_arrget(arr, i)` | `arr[i]` |
| **索引赋值** `arr[i] = v` **[变异]** | `arr[i as usize] = clone_arg(v);` | `arr[i] = v;` | `_arrset(arr, i, v);` | `arr[i] = v` |
| `map.len()` | `map.len() as i32` | `map.length`（属性） | `_maplen(map)` | `len(map)` |
| `map.is_empty()` | `map.is_empty()` | `map.isEmpty`（属性） | `_maplen(map) === 0` | `len(map) == 0` |
| `map.get(k)` | 若 V 为 Copy：`map[&k]` 或 `map[ key_literal(k) ]`<br>若 V 为 `String`：`map[&k].to_string()` 等 | `(map[k] ?? (throw Exception('Key not found')))` | `_mapget(map, k)` | `map[k]`（缺失 KeyError） |
| `map.contains(k)` | `map.contains_key( key_arg(k) );` | `map.containsKey(k)` | `map.has(k)` | `k in map` |
| `map.insert(k, v)` **[变异]** | `map.insert( key_clone(k), val_clone(v) );` | `map[k] = v;` | `map.set(k, v);` | `map[k] = v` |
| **索引取值** `map[key]` | 同 `map.get(key)` | 同上 | 同上 | 同上 |
| **索引赋值** `map[key] = v` **[变异]** | `map.insert( key_clone(key), val_clone(v) );` | `map[key] = v;` | `map.set(key, v);` | `map[key] = v` |

**注释：**
- 键的引用 `key_literal(k)` / `key_arg(k)`：若 `k` 为 `String` 字面量，直接使用；其他所有情况（变量、`i32`/`bool`字面量等）加 `&`。
- `clone_arg(v)` 区分变量与临时值，`String` 用 `.to_string()`，`Array`/`Map` 用 `.clone()`（预留）。
- Dart 属性访问（`length`, `isEmpty`, `containsKey`）**不要括号**；JS/Python 的 `len()` 等按函数调用。
- 除零 panic：所有辅助函数必须显式检查除零并抛出错误（JS `_idiv`/`_irem`，Python 依赖 `ZeroDivisionError`）。

**JS 辅助函数完整清单（按需生成）：**
```javascript
function _idiv(a, b) {
    if (b === 0) throw new Error("division by zero");
    const r = Math.trunc(a / b);
    return r === 0 ? 0 : r; // -0 归一化
}
function _irem(a, b) {
    if (b === 0) throw new Error("division by zero");
    return a % b;
}
function _arrlen(arr) { return arr.length; }
function _arrisempty(arr) { return arr.length === 0; }
function _arrget(arr, i) {
    if (i < 0 || i >= arr.length) throw new Error("index out of bounds");
    return arr[i];
}
function _arrset(arr, i, val) {
    if (i < 0 || i >= arr.length) throw new Error("index out of bounds");
    arr[i] = val;
}
function _arrpop(arr) {
    if (arr.length === 0) throw new Error("pop from empty array");
    return arr.pop();
}
function _maplen(map) { return map.size; }
function _mapget(map, key) {
    if (!map.has(key)) throw new Error("Key not found");
    return map.get(key);
}
function _arrClone(arr) { return arr.slice(); }
function _mapClone(map) { return new Map(map); }
```

**Python 辅助函数（按需生成）：**
```python
def _idiv(a, b):
    # 除零抛出 ZeroDivisionError，等价 panic
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q

def _irem(a, b):
    # 除零抛出 ZeroDivisionError
    return a - _idiv(a, b) * b
```

#### 6.2 输出语句
- `print(s: String)` / `println(s: String)`：
  - Rust: `print!("{}", s);` / `println!("{}", s);`
  - Dart: `stdout.write(s);` / `stdout.writeln(s);`（**仅当程序包含输出调用时，自动添加 `import 'dart:io';`**）
  - JS: `process.stdout.write(s);` / `console.log(s);`
  - Python: `print(s, end='')` / `print(s)`

---

### 7. 转译映射补充规则

#### 7.1 Rust 后端
- **变量初始化**：`String` 字面量用 `String::from`，容器字面量用 `vec!`/`HashMap::from`；从变量初始化 `String` 用 `.to_string()`，`Array`/`Map` 用 `.clone()`。
- **函数调用**：Copy 类型实参直接传值；非 Copy 类型实参（`String`/`Array`/`Map`）前自动加 `&`。例如 `greet(s)` → `greet(&s)`；`process(arr)` → `process(&arr)`。字面量参数如 `greet("Alice")` 不变。
- **`for-in` 循环**：`for item in &arr`。Copy 类型使用值时加 `*`；非 Copy 类型需要所有权时调用克隆方法。
- **函数定义**：`fn main() { ... }` 无返回类型。
- **字符串拼接**：`format!`；`+=` 映射为 `s = format!("{}{}", s, t);`
- **索引**：数组 `arr[i as usize]`，Map 键按规则加 `&`（字面量特例）。
- **自动导入** `HashMap`（出现 `Map` 时）。

#### 7.2 Dart 后端
- **变量声明**：OmniLang `let` → Dart `final`；`let mut` → Dart `var`。函数参数、循环变量显式加 `final`（例如 `for (final item in arr)`）。
- **变量遮蔽**：嵌套块（包括 `for-in` 循环体）中声明与外部同名的变量时，重命名为 `{原名}_shadow_{唯一ID}`（全局递增，确保唯一）。
- **属性访问**：`arr.length`、`arr.isEmpty`、`map.length`、`map.isEmpty`（去掉括号）。
- **容器克隆**：`List.of(x)` / `Map.of(x)`。
- **除法**：`~/`；取模 `%`；复合赋值对应调整。

#### 7.3 JavaScript 后端
- **变量声明**：OmniLang `let` → JS `const`；`let mut` → JS `let`。
- **整数除零/取模除零**：使用 `_idiv` / `_irem` 辅助函数；复合赋值映射为 `x = _idiv(x, y)` 等。
- **容器克隆**：`_arrClone` / `_mapClone`。
- **程序入口**：在生成代码的末尾（所有函数定义之后）追加 `main();` 语句。
- **环境依赖**：假定 Node.js 运行时，输出使用 `process.stdout.write` / `console.log`。

#### 7.4 Python 后端
- **变量声明**：`let` / `let mut` 均转为普通赋值（Python 无常量语义，依赖前端检查）。
- **变量遮蔽**：Python 缺少块级作用域，**嵌套块（包括 `if`/`while`/`for-in` 循环体）中声明的变量**若与外部同名，自动重命名为 `{原名}_shadow_{唯一ID}`，所有引用同步更新。
- **整数除法/取模**：使用 `_idiv` / `_irem`，复合赋值转为 `x = _idiv(x, y)` 等。
- **容器克隆**：`list(x)` / `dict(x)`。
- **程序入口**：在文件末尾生成：
  ```python
  if __name__ == "__main__":
      main()
  ```
- `is_empty()` → `len(…) == 0`。

---

### 8. 完整示例（含入口调用及实参传递）

**OmniLang 源码：**
```omnilang
fn greet(name: String) -> String {
    "Hello, " + name
}

fn main() {
    let s: String = "Alice";
    let msg: String = greet(s);   // 实参为局部 String 变量
    println(msg);

    let arr1: Array<i32> = [1, 2, 3];
    let arr2: Array<i32> = arr1;   // 自动克隆
    arr2.push(4);
    println(arr1.len().to_string());
}
```

**转译 Rust：**
```rust
use std::collections::HashMap;

fn greet(name: &str) -> String {
    format!("{}{}", "Hello, ", name)
}

fn main() {
    let s: String = String::from("Alice");
    let msg: String = greet(&s);   // 实参加 &
    println!("{}", msg);

    let mut arr1: Vec<i32> = vec![1, 2, 3];
    let mut arr2: Vec<i32> = arr1.clone();
    arr2.push(4);
    println!("{}", (arr1.len() as i32).to_string());
}
```

**转译 Python：**
```python
def _idiv(a, b):
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q

def _irem(a, b):
    return a - _idiv(a, b) * b

def greet(name):
    return "Hello, " + name

def main():
    s = "Alice"
    msg = greet(s)
    print(msg)

    arr1 = [1, 2, 3]
    arr2 = list(arr1)   # 克隆
    arr2.append(4)
    print(str(len(arr1)))

if __name__ == "__main__":
    main()
```

**转译 JavaScript：**
```javascript
function _idiv(a, b) {
    if (b === 0) throw new Error("division by zero");
    const r = Math.trunc(a / b);
    return r === 0 ? 0 : r;
}
function _irem(a, b) {
    if (b === 0) throw new Error("division by zero");
    return a % b;
}
function _arrlen(arr) { return arr.length; }

function greet(name) {
    return "Hello, " + name;
}

function main() {
    const s = "Alice";
    const msg = greet(s);
    console.log(msg);

    const arr1 = [1, 2, 3];
    const arr2 = arr1.slice();   // 克隆
    arr2.push(4);
    console.log(String(_arrlen(arr1)));
}

main();
```

**转译 Dart：**
```dart
import 'dart:io';

String greet(final String name) => 'Hello, ' + name;

void main() {
  final s = 'Alice';
  final msg = greet(s);
  stdout.writeln(msg);

  final arr1 = <int>[1, 2, 3];
  var arr2 = List.of(arr1);   // 克隆
  arr2.add(4);
  stdout.writeln(arr1.length.toString());
}
```

---

### 9. 编译器架构（Python 实现）

1. **词法/语法**：`lark-parser` + EBNF。
2. **AST**：Python dataclass 节点。
3. **语义检查器**（需按顺序执行的 pass）：
   - 强制类型标注（`for-in` 变量除外）。
   - 禁止同级作用域变量遮蔽；记录所有嵌套遮蔽以供重命名。
   - 禁止 `_` 开头的用户标识符。
   - 容器元素类型和 `Map` 键类型检查（禁止 `f64` 键）。
   - 禁止浮点除法与取模。
   - 函数参数只读：不允许对参数重新赋值或调用变异方法。
   - `for-in` 体内禁止修改迭代容器和循环变量；只允许规定的只读方法。
   - `main` 函数规则：必须存在，无参，可省略返回类型，不能有返回值表达式。
   - `print`/`println`：仅能作为独立语句出现，且参数类型必须为 `String`。
4. **代码生成器**：四个 Visitor，严格按映射表生成，处理解引用、克隆、重命名、除零辅助函数、Dart 条件导入、Python/JS 入口调用、Rust 实参加 `&` 等。
5. **CLI**：`omnic build <file.omni> --target rust|dart|js|python`

---

### 10. 已知局限与规划

- 无自定义结构体、枚举、模式匹配、错误处理、异步。
- 浮点运算仅支持 `+ - *`，无除法/取模。浮点算术溢出、特殊值可能存在跨语言差异；浮点比较也存在微小差异。
- 整数溢出行为未定义（后续版本引入检查）。
- 容器仅能存储基本类型和 `String`，不支持嵌套；`Map` 键不允许 `f64`，`bool` 键在 Python 中因与 `int` 重叠而存在潜在冲突，但目前类型隔离下安全。
- 所有限制均为确保 v0.1 转译结果的 **绝对一致与可编译**，后续版本逐步扩展。

---

**本规范已补全所有缺失规则：强制 main 入口、函数调用实参引用传递、print 参数类型检查、Dart 异常具体类型等，所有细节自洽，可直接作为原型实现唯一权威依据。**