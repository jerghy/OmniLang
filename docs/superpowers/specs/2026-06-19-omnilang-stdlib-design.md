# OmniLang 标准库设计

> **日期**: 2026-06-19
> **状态**: 已确认
> **参照**: [[2026-06-19-learn-omniview-full-demo-design]]

## 概述

为 OmniLang 设计跨 9 编译目标的标准库。导入方式为 `import { fn } from "std/<module>"`。编译器在 codegen 阶段将 std 调用替换为各目标原生实现。不支持目标平台的函数编译时报错。

---

## §1 模块架构

```
std/
├── json        // JSON 解析/序列化 — 全目标支持
├── http        // HTTP 客户端 — 全目标支持，强制 async
├── storage     // KV 存储 — 仅 Web/Flutter 目标
├── math        // (future)
├── datetime    // (future)
├── timer       // (future)
└── crypto      // (future)
```

第一期实现: `std/json` + `std/http` + `std/storage`

---

## §2 API 规格

### std/json — 全目标

```omni
fn json_parse(text: String);
// 返回原生动态类型。JS→any, Dart→dynamic, Rust→serde_json::Value
// 等泛型成熟后改为 json_parse<T>(text: String) -> T

fn json_stringify(value) -> String;
// 接受任何值，返回 JSON 字符串
```

### std/http — 全目标，强制 async

```omni
fn http_get(url: String) -> HttpResponse;
fn http_post(url: String, body: String) -> HttpResponse;
fn http_put(url: String, body: String) -> HttpResponse;
fn http_delete(url: String) -> HttpResponse;
```

调用方必须在 `async fn` / `async action` 内部。codegen 自动生成 `await`。

```omni
// HttpResponse 是编译器内置结构体
struct HttpResponse {
    status: i32,
    body: String,
    headers: Map<String, String>,
}
```

### std/storage — 仅 Web / Flutter

```omni
fn storage_get(key: String) -> String?;
fn storage_set(key: String, value: String);
fn storage_remove(key: String);
fn storage_clear();
```

第一期只支持 String value。复杂数据用 `json_stringify` 序列化后存入。

| Target | 实现 |
|--------|------|
| JS / React / Vue / OmniView | `localStorage` |
| Flutter | `SharedPreferences` |
| Rust / Python / Dart standalone / Dioxus | 编译报错 |

---

## §3 目标映射表

### json_parse / json_stringify

| Target | json_parse | json_stringify |
|--------|-----------|---------------|
| JS/React/Vue/OmniView | `JSON.parse(text)` | `JSON.stringify(value)` |
| Python | `json.loads(text)` | `json.dumps(value)` |
| Rust | `serde_json::from_str(&text).unwrap()` | `serde_json::to_string(&value).unwrap()` |
| Dart/Flutter | `json.decode(text)` | `json.encode(value)` |
| Dioxus | `serde_json::from_str(&text).unwrap()` | `serde_json::to_string(&value).unwrap()` |

### http_get (其余方法同理)

| Target | 实现 |
|--------|------|
| JS/React/Vue/OmniView | `await fetch(url).then(r => r.json())` 包装为 HttpResponse |
| Python | `requests.get(url)` |
| Rust | `reqwest::blocking::get(url)` |
| Dart/Flutter | `await http.get(Uri.parse(url))` |
| Dioxus | `reqwest::blocking::get(url)` |

http 函数只在 async 上下文中可用，codegen 自动加 `await`。同步目标 (Rust blocking) 正常调用。

### storage_get / storage_set

| Target | get | set |
|--------|-----|-----|
| JS/React/Vue/OmniView | `localStorage.getItem(key)` | `localStorage.setItem(key, value)` |
| Flutter | `SharedPreferences.getInstance().getString(key)` | `.setString(key, value)` |
| 其他 | 编译报错 | 编译报错 |

---

## §4 编译器实现架构

### 4.1 Intrinsic 注册表

新增 `omni-compiler/src/stdlib.rs`，包含静态注册表：

```rust
struct StdlibFunction {
    module: &'static str,       // "json" | "http" | "storage"
    name: &'static str,         // "json_parse" | "http_get" | ...
    targets: &'static [Target], // 可用目标列表
    codegen_map: fn(target: Target) -> &'static str, // 目标→实现模板
}
```

### 4.2 Import 解析

Parser 无需改动——`import { json_parse } from "std/json"` 已是合法的 Named import。编译器在 IR lowering 阶段识别 `std/` 前缀路径。

### 4.3 编译时校验

Codegen `generate()` 入口处：
1. 扫描 IR 中所有 std import
2. 查询 Intrinsic 注册表
3. 若当前 target 不在可用列表中 → 编译报错

错误格式：
```
error: std/storage is not available on target 'rust'
  ┌─ app.omni:3:1
  │
3 │ import { storage_get } from "std/storage";
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  │
  = hint: available targets: web, flutter
```

### 4.4 Codegen 映射

每个后端在遇到 std 调用时查注册表，emit 对应原生代码。示例伪代码：

```rust
fn codegen_stdlib_call(func: &str, args: &[String], target: Target) -> String {
    match registry.lookup(func) {
        Some(entry) if entry.targets.contains(&target) => {
            (entry.codegen_map)(target, args)
        }
        Some(_) => compile_error!("std/{} not available on {:?}", func, target),
        None => compile_error!("unknown stdlib function: {}", func),
    }
}
```

---

## §5 使用示例

```omni
import { json_parse, json_stringify } from "std/json";
import { http_get, http_post } from "std/http";
import { storage_get, storage_set } from "std/storage";

component TodoApp {
    signal tasks: Array<String> = [];
    signal loading: bool = false;

    @mount {
        loadTasks();
    }

    action loadTasks() async {
        loading = true;

        // 先读本地缓存 → 秒开
        let cached = storage_get("tasks");
        if cached != null {
            tasks = json_parse(cached);
        }

        // 再从服务器同步
        let resp = await http_get("/api/tasks");
        if resp.status == 200 {
            tasks = json_parse(resp.body);
            storage_set("tasks", resp.body);
        }

        loading = false;
    }

    action addTask(text: String) async {
        let body = json_stringify({ name: text });
        let resp = await http_post("/api/tasks", body);
        if resp.status == 201 {
            tasks.push(text);
            storage_set("tasks", json_stringify(tasks));
        }
    }
}
```

---

## §6 与其他语言对标

| 特性 | OmniLang | Go | Rust | JavaScript | Dart |
|------|----------|----|------|------------|------|
| JSON parse | `json_parse(s)` | `json.Unmarshal` | `serde_json::from_str` | `JSON.parse` | `json.decode` |
| HTTP GET | `http_get(url)` | `http.Get` | `reqwest::get` | `fetch()` | `http.get()` |
| KV Storage | `storage_get(k)` | — | — | `localStorage` | `SharedPrefs` |
| 导入方式 | `import from "std/x"` | `import "pkg"` | `use std::x` | `import from "x"` | `import "pkg"` |
| 不可用平台 | 编译报错 | 编译报错 | 条件编译 | 运行时 error | 编译报错 |

---

## §7 后续扩展

- **泛型 JSON**: `json_parse<T>(text: String) -> T`
- **http_get_json**: `http_get_json<T>(url: String) -> T`
- **Middleware**: 拦截器/超时/重试机制
- **std/math**: `math_abs`, `math_sqrt`, `math_random` 等
- **std/datetime**: `datetime_now`, `datetime_format` 等
- **std/timer**: `timer_set_timeout`, `timer_set_interval` 等
