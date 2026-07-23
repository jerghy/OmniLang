# 13 — 标准库参考（Standard Library）

OmniLang 通过 `import { ... } from "std/module"` 语法提供标准库功能。本文档涵盖所有可用模块。

## 使用方式

```omni
// 导入单个函数
import { console_log } from "std/console";
import { array_filter } from "std/array";

// 导入多个函数
import { http_get, http_post } from "std/http";
```

## std/console — 控制台输出

用于调试和日志输出，编译为浏览器 `console.*` 调用。

| 函数 | 说明 | 编译为 |
|------|------|--------|
| `console_log(msg: String)` | 普通日志 | `console.log(msg)` |
| `console_warn(msg: String)` | 警告 | `console.warn(msg)` |
| `console_error(msg: String)` | 错误 | `console.error(msg)` |

```omni
import { console_log, console_warn, console_error } from "std/console";

fn debug() {
    console_log("Debug message");
    console_warn("Warning: low memory");
    console_error("Error: connection failed");
}
```

## std/http — HTTP 请求

异步 HTTP 客户端。

| 函数 | 说明 |
|------|------|
| `http_get(url: String) -> String` | 发送 GET 请求，返回响应体 |
| `http_post(url: String, body: String) -> String` | 发送 POST 请求，返回响应体 |

```omni
import { http_get, http_post } from "std/http";

async fn fetchTodo(id: i32) -> String {
    let raw = await http_get("https://api.example.com/todos/" + id.to_string());
    return raw;
}

async fn submitData(data: String) -> String {
    let result = await http_post("https://api.example.com/submit", data);
    return result;
}
```

## std/json — JSON 处理

| 函数 | 说明 |
|------|------|
| `json_parse(raw: String) -> Map<String, ?>` | 解析 JSON 字符串为 Map |
| `json_stringify(data: Map<String, ?>) -> String` | 序列化 Map 为 JSON 字符串 |

```omni
import { json_parse, json_stringify } from "std/json";

fn process(raw: String) -> String {
    let parsed = json_parse(raw);         // String → Map
    return json_stringify(parsed);        // Map → String
}
```

## std/array — 数组工具

| 函数 | 签名 | 说明 |
|------|------|------|
| `array_filter` | `(arr: Array<T>, fn: (T) -> bool) -> Array<T>` | 过滤数组，返回新数组 |
| `array_map` | `(arr: Array<T>, fn: (T) -> U) -> Array<U>` | 映射数组 |
| `array_find` | `(arr: Array<T>, fn: (T) -> bool) -> T?` | 查找第一个匹配元素 |
| `array_contains` | `(arr: Array<T>, fn: (T) -> bool) -> bool` | 是否包含匹配元素 |
| `array_sort` | `(arr: Array<T>, fn: (T, T) -> i32) -> Array<T>` | 排序数组 |
| `array_length` | `(arr: Array<T>) -> i32` | 数组长度 |
| `array_push` | `(arr: Array<T>, item: T) -> Array<T>` | 添加元素（不可变，返回新数组） |
| `array_remove` | `(arr: Array<T>, index: i32) -> Array<T>` | 移除指定索引元素 |

```omni
import { array_filter, array_sort, array_map } from "std/array";

// 闭包过滤
let active = array_filter(users, |u| u.status == "active");

// 闭包排序
let sorted = array_sort(items, |a, b| a.name);

// 闭包映射
let names = array_map(users, |u| u.name);
```

> `std/array` 函数的第二参数接受管道闭包 `|param| expr`。闭包可以内联写，也可以先赋值给 `let` 变量再传入。详见 [11-表达式系统](11-expressions.md#闭包--lambda)。

## std/string — 字符串工具

| 函数 | 签名 | 说明 |
|------|------|------|
| `string_length` | `(s: String) -> i32` | 字符串长度 |
| `string_contains` | `(s: String, sub: String) -> bool` | 是否包含子串 |
| `string_starts_with` | `(s: String, prefix: String) -> bool` | 是否以 prefix 开头 |
| `string_ends_with` | `(s: String, suffix: String) -> bool` | 是否以 suffix 结尾 |
| `string_trim` | `(s: String) -> String` | 去除首尾空白 |
| `string_concat` | `(a: String, b: String) -> String` | 连接字符串 |
| `string_split` | `(s: String, delim: String) -> Array<String>` | 按分隔符分割 |
| `string_join` | `(arr: Array<String>, delim: String) -> String` | 用分隔符连接数组 |
| `string_replace` | `(s: String, old: String, new: String) -> String` | 替换子串 |
| `string_substring` | `(s: String, start: i32, end: i32) -> String` | 截取子串 |
| `string_to_upper` | `(s: String) -> String` | 转大写 |
| `string_to_lower` | `(s: String) -> String` | 转小写 |

```omni
import { string_trim, string_split, string_contains } from "std/string";

fn validateEmail(input: String) -> bool {
    let trimmed = string_trim(input);
    return string_contains(trimmed, "@");
}
```

## std/math — 数学函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `math_abs` | `(x: f64) -> f64` | 绝对值 |
| `math_ceil` | `(x: f64) -> i32` | 向上取整 |
| `math_floor` | `(x: f64) -> i32` | 向下取整 |
| `math_round` | `(x: f64) -> i32` | 四舍五入 |
| `math_sqrt` | `(x: f64) -> f64` | 平方根 |
| `math_pow` | `(base: f64, exp: f64) -> f64` | 幂运算 |
| `math_min` | `(a: f64, b: f64) -> f64` | 最小值 |
| `math_max` | `(a: f64, b: f64) -> f64` | 最大值 |
| `math_random` | `() -> f64` | 随机数 `[0, 1)` |

```omni
import { math_random, math_ceil, math_sqrt, math_max } from "std/math";

fn rollDice() -> i32 {
    let raw = math_random();           // 0.0 ~ 1.0
    return math_ceil(raw * 6);         // 1 ~ 6
}

fn safeDiv(a: f64, b: f64) -> f64 {
    return if b == 0.0 { 0.0 } else { a / b };
}
```

## std/storage — 本地存储

基于浏览器 `localStorage` 的键值持久化。

| 函数 | 签名 | 说明 |
|------|------|------|
| `storage_get` | `(key: String) -> String?` | 读取，不存在返回 null |
| `storage_set` | `(key: String, value: String)` | 写入 |

```omni
import { storage_get, storage_set } from "std/storage";

// 配合 json 实现复杂数据持久化
import { json_parse, json_stringify } from "std/json";

fn saveSettings(theme: String, fontSize: i32) {
    let data = json_stringify({ "theme": theme, "fontSize": fontSize.to_string() });
    storage_set("app-settings", data);
}

fn loadSettings() -> String {
    let raw = storage_get("app-settings");
    return raw ?? "{}";
}
```

## 内置函数

以下是可直接调用的内置函数（无需 import）：

| 函数 | 说明 | 可用目标 |
|------|------|----------|
| `datetime_now() -> i64` | 当前时间戳（毫秒） | React |

```omni
fn getTimestamp() -> i64 {
    return datetime_now();
}
```

> **注意**：`datetime_now` 是编译器的内置函数，目前仅 React 目标支持。OmniView 目标请使用 JS interop。

## 内置方法

除了 std 模块，以下方法是直接在类型上可用的：

### String 方法

| 方法 | 说明 | 示例 |
|------|------|------|
| `s.len()` | 长度 | `"hello".len()` → `5` |
| `s.to_string()` | 自身 | `x.to_string()` |
| `s.to_uppercase()` | 转大写 | `s.to_uppercase()` |
| `s.to_lowercase()` | 转小写 | `s.to_lowercase()` |
| `s.get(index)` | 按索引取字符 | `"abc".get(1)` → `"b"` |

### Array 方法

| 方法 | 说明 | 示例 |
|------|------|------|
| `arr.len()` | 长度 | `[1,2,3].len()` → `3` |
| `arr.push(item)` | 追加元素 | `arr.push(4)` |
| `arr.pop()` | 移除最后元素 | `arr.pop()` |
| `arr.get(index)` | 按索引取值 | `arr.get(0)` |
| `arr.is_empty()` | 是否为空 | `arr.is_empty()` |
| `arr.concat(other)` | 连接数组 | `a.concat(b)` |

**注意**：`.push()` 和 `.pop()` 会触发响应式更新（内部 `.set(.slice())`）。

### 数字方法

| 方法 | 说明 | 示例 |
|------|------|------|
| `n.to_string()` | 转字符串 | `42.to_string()` → `"42"` |
| `n.abs()` | 绝对值 | `(-5).abs()` → `5` |
