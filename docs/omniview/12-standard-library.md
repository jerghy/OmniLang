# 12 — 标准库参考

OmniLang 通过 `import { ... } from "std/module"` 提供标准库。

## std/console — 控制台

编译为浏览器 `console.*`。

| 函数 | 编译 |
|------|------|
| `console_log(msg: String)` | `console.log(msg)` |
| `console_warn(msg: String)` | `console.warn(msg)` |
| `console_error(msg: String)` | `console.error(msg)` |

## std/http — HTTP 请求

| 函数 | 说明 |
|------|------|
| `http_get(url: String) -> String` | GET 请求，返回响应体 |
| `http_post(url: String, body: String) -> String` | POST 请求 |

```omni
import { http_get } from "std/http";

fn loadData() async {
    let raw = await http_get("https://api.example.com/data");
    data = raw;
}
```

## std/json — JSON

| 函数 | 说明 |
|------|------|
| `json_parse(raw: String) -> Map<String, ?>` | JSON 字符串 → Map |
| `json_stringify(data: Map<String, ?>) -> String` | Map → JSON 字符串 |

## std/array — 数组工具

所有函数返回新数组（不可变）：

| 函数 | 签名 | 说明 |
|------|------|------|
| `array_filter` | `(Array<T>, fn(T)->bool) -> Array<T>` | 过滤 |
| `array_map` | `(Array<T>, fn(T)->U) -> Array<U>` | 映射 |
| `array_find` | `(Array<T>, fn(T)->bool) -> T?` | 查找第一个匹配 |
| `array_contains` | `(Array<T>, fn(T)->bool) -> bool` | 是否包含匹配 |
| `array_sort` | `(Array<T>, fn(T,T)->i32) -> Array<T>` | 排序 |
| `array_length` | `(Array<T>) -> i32` | 长度 |
| `array_push` | `(Array<T>, T) -> Array<T>` | 追加（不可变） |
| `array_remove` | `(Array<T>, i32) -> Array<T>` | 移除索引 |

```omni
import { array_filter, array_sort } from "std/array";

let active = array_filter(users, |u| u.status == "active");
let sorted = array_sort(items, |a, b| a.name);
```

> 闭包参数接受管道闭包 `|param| expr`，详见 [03-表达式与运算符](03-expressions-and-operators.md#闭包--lambda)。

## std/string — 字符串

| 函数 | 说明 |
|------|------|
| `string_length(s) -> i32` | 长度 |
| `string_contains(s, sub) -> bool` | 包含子串 |
| `string_starts_with(s, prefix) -> bool` | 前缀判断 |
| `string_ends_with(s, suffix) -> bool` | 后缀判断 |
| `string_trim(s) -> String` | 去首尾空白 |
| `string_concat(a, b) -> String` | 连接 |
| `string_split(s, delim) -> Array<String>` | 分割 |
| `string_join(arr, delim) -> String` | 连接数组 |
| `string_replace(s, old, new) -> String` | 替换 |
| `string_substring(s, start, end) -> String` | 截取 |
| `string_to_upper(s) -> String` | 转大写 |
| `string_to_lower(s) -> String` | 转小写 |

## std/math — 数学

| 函数 | 签名 |
|------|------|
| `math_abs(x: f64) -> f64` | 绝对值 |
| `math_ceil(x: f64) -> i32` | 向上取整 |
| `math_floor(x: f64) -> i32` | 向下取整 |
| `math_round(x: f64) -> i32` | 四舍五入 |
| `math_sqrt(x: f64) -> f64` | 平方根 |
| `math_pow(base: f64, exp: f64) -> f64` | 幂 |
| `math_min(a: f64, b: f64) -> f64` | 最小值 |
| `math_max(a: f64, b: f64) -> f64` | 最大值 |
| `math_random() -> f64` | 随机数 [0,1) |

```omni
import { math_random, math_ceil } from "std/math";

fn rollDice() -> i32 {
    return math_ceil(math_random() * 6);
}
```

## std/storage — 本地存储

| 函数 | 说明 |
|------|------|
| `storage_get(key: String) -> String?` | 读取，无则 null |
| `storage_set(key: String, value: String)` | 写入 |

```omni
import { storage_get, storage_set } from "std/storage";

fn saveTheme(mode: String) {
    storage_set("theme-mode", mode);
}
```

## std/async — 异步

| 函数 | 说明 |
|------|------|
| `async_sleep(ms: i32) async` | 异步延时（如实现计时器） |
| `async_spawn(fn) async` | 并行执行异步任务 |
| `async_all(tasks) async` | 全部完成 |
| `async_race(tasks) async` | 先到者胜 |
| `async_timeout(fn, ms) async` | 超时兜底 |

```omni
import { async_sleep } from "std/async";

// async fn 内 await 延时；完整演示见 StdLibShowcase（async_demo.omni）
action tick() async {
    await async_sleep(1000);
}
```

## 内置方法

无需 import，直接在类型上调用：

### String

| 方法 | 示例 |
|------|------|
| `.len() -> i32` | `"hi".len()` → `2` |
| `.to_string() -> String` | `x.to_string()` |
| `.to_uppercase() -> String` | `s.to_uppercase()` |
| `.to_lowercase() -> String` | `s.to_lowercase()` |
| `.get(index: i32) -> String` | `"abc".get(1)` → `"b"` |

### Array

| 方法 | 说明 |
|------|------|
| `.len() -> i32` | 长度 |
| `.push(item)` | 追加（触发响应式） |
| `.pop()` | 移除最后（触发响应式） |
| `.get(index) -> T` | 按索引取值 |
| `.is_empty() -> bool` | 是否为空 |
| `.concat(other) -> Array<T>` | 连接 |
| `.filter(\|x\| cond) -> Array<T>` | 过滤（语言方法，见下方跨目标映射） |
| `.map(\|x\| expr) -> Array<U>` | 映射（语言方法，见下方跨目标映射） |
| `.count(\|x\| cond) -> i32` | 满足条件的元素个数（语言方法，展开为 filter + length） |

`.push()` 和 `.pop()` 会触发信号系统的响应式更新。

`filter`/`map`/`count` 是**语言方法**（无需 import，谓词用管道闭包 `|x|`），跨目标映射：

| 方法 | React / JS | Flutter / Dart |
|------|-----------|----------------|
| `.filter(\|x\| cond)` | `filter((x) => cond)` | `where((x) => cond).toList()` |
| `.map(\|x\| expr)` | `map((x) => expr)` | `map((x) => expr).toList()` |
| `.count(\|x\| cond)` | `filter((x) => cond).length` | `where((x) => cond).length` |

`count` 需要一个谓词闭包——`items.count()` 是编译错误（E_ARRAY_METHOD）。`array_filter`/`array_map` 等 stdlib 函数保留为底层接口。

### 数字

| 方法 | 示例 |
|------|------|
| `.to_string() -> String` | `42.to_string()` → `"42"` |
| `.abs() -> f64` | `(-5).abs()` → `5` |
