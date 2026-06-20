# OmniLang 标准库与前置语言特性 — 实现计划

> **日期**: 2026-06-19
> **状态**: 实施中 (Feature 1 完成; Feature 2 部分完成)
> **依赖设计**: [[2026-06-19-omnilang-stdlib-design]]

## 总览

为实现完整的 HTTP 标准库（struct 构造 + match 错误处理），需要先实现 4 个语言特性，再实现 HTTP stdlib。另外，JSON 和 Storage stdlib 独立于语言特性，可提前实施。

```
已完成:
  await 表达式 ✅ ──────┐
                        ├──→ http_send 可用
待实施:                  │
  Struct Literals (60%)─┤
  Union Constr ─────────┤
  Match 表达式 ─────────┘

独立可实施 (不依赖语言特性):
  std/json ────────── 随时可做
  std/storage ─────── 随时可做
```

---

## §0 当前状态摘要

| Feature | AST | Parser | IR/Lowering | Codegen | Tests | 状态 |
|---------|-----|--------|-------------|---------|-------|------|
| await 表达式 | ✅ | ✅ | ✅ | ✅ 8/8 | ✅ 396 | **完成** |
| Struct Literals | ✅ | ✅ | ✅ | ✅ 9/9 | ❌ | **完成 (缺测试)** |
| Union Construction | ❌ | N/A | ❌ | ❌ | ❌ | **0%** |
| Match 表达式 | ❌ | ❌ | ❌ | ❌ | ❌ | **0%** |
| HTTP Stdlib | ❌ | ❌ | ❌ | ❌ | ❌ | **0%** |
| JSON Stdlib | N/A | N/A | ❌ | ❌ | ❌ | **0%** |
| Storage Stdlib | N/A | N/A | ❌ | ❌ | ❌ | **0%** |

### Feature 2 (Struct Literals) 实际状态

Struct Literals 已全部实施完成——并非之前认为的 60%。验证结果：

- `ast.rs:98-102`: `StructLiteral { type_name, fields, span }` ✅
- `expr.rs:30-36`: post-Pratt `VarRef + LBrace` 检测 ✅
- `expr.rs:58-101`: `parse_struct_literal_suffix()` 完整实现 ✅
- `expr.rs:559`: `span_of()` arm ✅
- `ir/mod.rs:55-58`: `StructExpr { type_name, fields }` ✅
- `ir/mod.rs:1429-1440`: lowering `StructLiteral → StructExpr` ✅
- `semantic/lints.rs:627`: `Expr::StructLiteral` arm ✅
- 9/9 codegen backends 全部处理 `ExprIR::StructExpr`: ✅
  - `javascript.rs:504` / `vue.rs:758` / `omniview.rs:1687` / `dart.rs:455`
  - `python.rs:527` / `dioxus.rs:636` / `flutter.rs:707` / `react.rs:713` / `rust.rs:470`

**唯一缺失**: 无专门测试。Struct Literal 功能已可用，只需补充测试验证。

### Feature 1 (await) 实施细节回顾

**实际改动** (与计划一致):
- `ast.rs`: 新增 `AwaitExpr { operand: Box<Expr>, span: Span }`
- `expr.rs`: `TokenKind::Await` → `parse_expr_bp(0)`，`span_of()` 新增 arm
- `ir/mod.rs`: `ExprIR::AwaitExpr { operand: Box<ExprIR> }` + lowering arm
- `semantic/lints.rs`: `Expr::AwaitExpr` arm（收集操作数中的名字）
- 8× codegen: 6 个前缀 `await <expr>` (JS/Dart/Flutter/React/Vue/OmniView) + 2 个后缀 `<expr>.await` (Rust/Dioxus)
- Python 通过 wildcard `_ => "None"` 自动捕获（静默不报错但不生成 `await`）
- 7 个新测试: 3 parser + 1 OmniView integration + 1 JS + 1 Rust + 1 Dart

---

## § 前置语言特性

### Feature 1: `await` 表达式 ✅ **已完成**

**状态**: 已于 2026-06-19 实施完成。396 测试通过。

**改动摘要**: ~15 处（1 AST + 1 parser + 1 IR + 1 lowering + 8 codegen + 2 misc）

**未处理边缘情况**:
- Python codegen 通过 wildcard 捕获 `AwaitExpr` 但不生成 `await`——若 Python target 需要 await 支持，需显式添加 match arm
- Semantic lint "await outside async fn" 已推迟——当前无上下文检查

---

### Feature 2: Struct Literals ✅ **完成 (仅缺测试)**

**现状**: 全部实施完成——AST、parser（post-Pratt `LBrace` 检测 + `parse_struct_literal_suffix`）、`span_of`、IR 变体（`StructExpr`）、lowering、semantic lints、**9/9 codegen 后端**全部已实现。唯一缺失是专门测试。

**已有代码位置**:

| 层 | 文件:行 | 内容 |
|---|--------|------|
| AST | `ast.rs:98-102` | `StructLiteral { type_name, fields, span }` |
| Parser | `expr.rs:30-36` | post-Pratt: `PascalCase VarRef + LBrace` → struct literal |
| Parser | `expr.rs:58-101` | `parse_struct_literal_suffix()` — 解析 `{ field: val, ... }` |
| Parser | `expr.rs:559` | `span_of()` arm |
| IR | `ir/mod.rs:55-58` | `StructExpr { type_name, fields: Vec<(String, ExprIR)> }` |
| Lowering | `ir/mod.rs:1429-1440` | `StructLiteral → StructExpr` |
| Lints | `semantic/lints.rs:627` | 收集字段表达式中的名字 |
| JS codegen | `javascript.rs:504` | `{ "field": value, ... }` |
| Vue codegen | `vue.rs:758` | `{ field: value, ... }` |
| OmniView | `omniview.rs:1687` | `{ field: value, ... }` |
| Dart | `dart.rs:455` | `TypeName(field: value, ...)` |
| Flutter | `flutter.rs:707` | `TypeName(field: value, ...)` |
| Python | `python.rs:527` | `{"field": value, ...}` (dict) |
| Rust | `rust.rs:470` | `TypeName { field: value, ... }` |
| Dioxus | `dioxus.rs:636` | `TypeName { field: value, ... }` |
| React/TSX | `react.rs:713` | `{ field: value, ... }` |

**待完成**: 仅测试
- Parser test: `Point { x: 1, y: 2 }` 正确解析
- Codegen tests: JS + Rust + Dart (覆盖三种生成模式: object/dict/nominal)

**预计剩余改动**: ~3 处（2 parser tests + 3 codegen tests）

---

### Feature 3: Union Variant Construction

**现状**: `TaggedUnionDef` + `Variant` 只支持类型定义，不能构造值。StructLiteral parser 已就绪。

**设计决策**: 复用 StructLiteral 的 `PascalCaseIdent { fields }` 语法。Parser 不区分 struct 和 variant——统一 emit `StructLiteral`。Lowering 阶段查询 `Program` 确认是 struct 还是 variant。

#### 3a. 重构 lowering 函数签名

将 `&Program` 参数传入所有 lowering 函数：

```rust
fn lower(program: &Program, ...) -> IRProgram { ... }
fn lower_component(comp: &ComponentDef, program: &Program) -> ComponentIR { ... }
fn lower_expr(expr: &Expr, program: &Program) -> ExprIR { ... }
// ... 全部 lowering 函数 (~8 个签名改动)
```

#### 3b. Type resolution in lowering

`lower_expr` 中 `StructLiteral` 的处理：

```rust
Expr::StructLiteral { type_name, fields, .. } => {
    if is_tagged_union_variant(program, type_name) {
        // Lower as variant construction
        let (union_name, variant_name) = resolve_variant(program, type_name);
        ExprIR::VariantLiteral {
            union_type: union_name,
            variant: variant_name,
            fields: fields.iter().map(|(k,v)| (k.clone(), lower_expr(v, program))).collect(),
        }
    } else {
        // Lower as struct literal
        ExprIR::StructExpr {
            type_name: type_name.clone(),
            fields: fields.iter().map(|(k,v)| (k.clone(), lower_expr(v, program))).collect(),
        }
    }
}
```

辅助函数 `is_tagged_union_variant(program, type_name)` 遍历 `program.tagged_unions` 检查所有 variant 名字。

#### 3c. IR + Codegen

- `ir/mod.rs`: `ExprIR` 新增 `VariantLiteral { union_type: String, variant: String, fields: Vec<(String, ExprIR)> }`
- 9× codegen: `format_expr_ir` 各加 match arm

| Target | `Ok { value: 42 }` 生成 |
|--------|------------------------|
| JS/TS/React/Vue/OmniView | `{ _variant: "Ok", value: 42 }` |
| Python | `{"_variant": "Ok", "value": 42}` |
| Dart | `HttpResult.ok(value: 42)` (sealed class factory) |
| Flutter | `HttpResult.ok(value: 42)` |
| Rust | `HttpResult::Ok { value: 42 }` |
| Dioxus | `HttpResult::Ok { value: 42 }` |

**优先级**: 依赖 Feature 2（Struct Literal codegen 完成 + lowering refactor）

**预计改动**: ~12 处（lowering refactor ~8 签名 + 1 new VariantLiteral + 9 codegen arms - 与 struct literal 共享 parser 逻辑）

---

### Feature 4: `match` 表达式

**现状**: `match` token 存在，parser 不处理

#### 4a. AST

```rust
// ast.rs 新增
MatchExpr {
    matched: Box<Expr>,
    arms: Vec<MatchArm>,
    span: Span,
}

MatchArm {
    pattern: MatchPattern,
    body: Block,  // 单表达式 body 也包裹为 Block
}

MatchPattern {
    kind: MatchPatternKind,
    span: Span,
}

enum MatchPatternKind {
    Variant { name: String, bindings: Vec<String> },  // Ok { value, response }
    Wildcard,  // _
}
```

#### 4b. Parser

`parse_prefix()` 新增 `TokenKind::Match` case:

```
match expr {
    Pattern => expr_or_block,
    Pattern => expr_or_block,
    ...
}
```

`=>` 用 existing `=>` token。body 若以 `{` 开头 → `parse_block()`；否则 → `parse_expr()` 包裹成单语句 Block。

Pattern 解析:
- `_` → Wildcard
- `Ident` → Variant { name, bindings: [] }
- `Ident { ident, ident }` → Variant { name, bindings: [ident, ...] }

#### 4c. Semantic

新增 `semantic/lints.rs` 规则:
- **exhaustiveness**: 所有 variant 都被覆盖 或 存在 `_` arm
- **no-duplicate**: 同一 pattern 不能出现两次
- **binding-count**: `Variant { fields }` 中的 field 数量必须与 variant 定义一致

#### 4d. IR + Codegen

- `ir/mod.rs`: `ExprIR` 新增 `MatchExpr { matched: Box<ExprIR>, arms: Vec<MatchArmIR> }`
- `lower_expr` 新增匹配
- 9× codegen: 各生成 switch/if-else 链

| Target | 生成模式 |
|--------|---------|
| JS/TS/React/Vue/OmniView | `switch (expr._variant) { case "Ok": ... }` |
| Python | `if isinstance(expr, dict) and expr.get("_variant") == "Ok": ...` 或 `match` (3.10+) |
| Dart | `switch (expr) { case HttpResultOk(): ... }` |
| Flutter | Dart switch pattern |
| Rust | `match expr { HttpResult::Ok { value } => ... }` |
| Dioxus | Rust match |

**优先级**: 依赖 Feature 3（需要 VariantLiteral 才能构造被 match 的值）

**预计改动**: ~15 处（AST 4 新类型 + parser 1 + semantic ~50 lines + IR/lowering + 9 codegen）

---

## § 标准库模块

### Feature 5: HTTP Stdlib

#### 5a. 类型定义

```omni
struct HttpRequest {
    url: String,
    method: String,
    headers: Map<String, String>,
    query: Map<String, String>,
    body: String?,
    timeout: i32,
}

struct HttpResponse {
    status: i32,
    body: String,
    headers: Map<String, String>,
}

type HttpError = Timeout
               | NetworkError { message: String }
               | HttpStatus { status: i32, body: String };

type HttpResult = Ok { response: HttpResponse }
                | Err { error: HttpError };
```

这些类型是**编译器内置**的——用户 import `std/http` 时自动可用，不需要在自己的 .omni 文件中定义。

#### 5b. API

```omni
// 主函数 — 完整控制
fn http_send(req: HttpRequest) -> HttpResult;

// 便利函数 — http_send 的语法糖
fn http_get(url: String) -> HttpResult;
fn http_get_with(url: String, headers: Map<String, String>, query: Map<String, String>, timeout: i32) -> HttpResult;
fn http_post(url: String, body: String?, headers: Map<String, String>, timeout: i32) -> HttpResult;
fn http_put(url: String, body: String?, headers: Map<String, String>, timeout: i32) -> HttpResult;
fn http_delete(url: String, headers: Map<String, String>, timeout: i32) -> HttpResult;
```

`http_get(url)` 展开为:
```omni
http_send(HttpRequest {
    url: url, method: "GET", headers: {}, query: {}, body: null, timeout: 0
})
```

#### 5c. 编译器实现

**新增文件**: `omni-compiler/src/stdlib.rs`

```rust
struct StdlibModule {
    name: &'static str,
    types: &'static [StdlibType],      // 内置类型定义
    functions: &'static [StdlibFn],     // 内置函数
    available_targets: &'static [Target],
}

struct StdlibFn {
    name: &'static str,
    params: &'static [(&'static str, &'static str)],  // (name, type)
    return_type: &'static str,
    codegen: fn(&CodeWriter, &[String], Target) -> String,  // per-target emit
    is_async: bool,
}
```

**改动文件**:
- `ir/mod.rs`: `IRProgram` 新增 `stdlib_imports: Vec<String>` 字段
- `ir/mod.rs`: `lower()` 识别 `"std/*"` 路径的 import，记录到 stdlib_imports
- `codegen/mod.rs`: `generate()` 入口处校验 target 可用性
- 9× codegen: 遇到 std call 时查 `StdlibModule` 表 emit 对应代码

#### 5d. 目标映射 — http_send

| Target | 实现 |
|--------|------|
| JS/React/Vue/OmniView | `await fetch(req.url, { method: req.method, headers: req.headers, body: req.body, signal: AbortSignal.timeout(req.timeout) })` |
| Python | `requests.request(req.method, req.url, headers=req.headers, params=req.query, data=req.body, timeout=req.timeout/1000)` |
| Rust | `reqwest::blocking::Client::new().request( Method::from_str(&req.method), &req.url).headers(req.headers).query(&req.query).body(req.body).timeout(Duration::from_millis(req.timeout)).send()` |
| Dart/Flutter | `await http.Request(req.method, Uri.parse(req.url)).headers.addAll(req.headers).send()` |

#### 5e. 编译时校验

```
error: std/storage is not available on target 'rust'
  ┌─ app.omni:3:1
  │
3 │ import { storage_get } from "std/storage";
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  │
  = hint: available targets: web (js, react, vue, omniview), flutter
```

**优先级**: 依赖 Feature 1, 2, 3, 4（await + struct + union + match）

**预计改动**: ~15 处（1 新文件 stdlib.rs + IR import 识别 + 9 codegen + semantic + lib.rs 注册）

---

### Feature 6: JSON Stdlib (独立可实施)

**依赖**: 无——`json_parse` / `json_stringify` 是同步函数，不依赖 await/struct/union/match。

**API**:
```omni
fn json_parse(text: String);      // 返回原生动态类型
fn json_stringify(value) -> String;  // 接受任何值，返回 JSON 字符串
```

> 等泛型成熟后改为 `json_parse<T>(text: String) -> T`

**编译器实现**: 复用 Feature 5 的 stdlib 注册表（`stdlib.rs`）。JSON 函数注册为 `module: "json"`。

**目标映射**:

| Target | json_parse | json_stringify |
|--------|-----------|---------------|
| JS/React/Vue/OmniView | `JSON.parse(text)` | `JSON.stringify(value)` |
| Python | `json.loads(text)` | `json.dumps(value)` |
| Rust | `serde_json::from_str(&text).unwrap()` | `serde_json::to_string(&value).unwrap()` |
| Dart/Flutter | `json.decode(text)` | `json.encode(value)` |
| Dioxus | `serde_json::from_str(&text).unwrap()` | `serde_json::to_string(&value).unwrap()` |

**Codegen 细节**:
```rust
// javascript.rs / omniview.rs / react.rs / vue.rs — 直接映射
"json_parse" => format!("JSON.parse({})", args[0]),
"json_stringify" => format!("JSON.stringify({})", args[0]),

// dart.rs / flutter.rs — 需要 import 'dart:convert'
"json_parse" => format!("json.decode({})", args[0]),
"json_stringify" => format!("json.encode({})", args[0]),

// python.rs — 需要 import json
"json_parse" => format!("json.loads({})", args[0]),
"json_stringify" => format!("json.dumps({})", args[0]),

// rust.rs / dioxus.rs — 需要 serde_json dep
"json_parse" => format!("serde_json::from_str(&{}).unwrap()", args[0]),
"json_stringify" => format!("serde_json::to_string(&{}).unwrap()", args[0]),
```

**预计改动**: ~12 处（stdlib.rs 注册 + 9 codegen + 2 tests）
**优先级**: 可随时实施，建议在 Feature 2 (struct codegen) 完成后做

---

### Feature 7: Storage Stdlib (独立可实施)

**依赖**: 无——纯同步 KV 操作。仅 Web / Flutter 目标可用。

**API**:
```omni
fn storage_get(key: String) -> String?;
fn storage_set(key: String, value: String);
fn storage_remove(key: String);
fn storage_clear();
```

第一期只支持 String value。复杂数据用 `json_stringify` 序列化后存入。

**目标映射**:

| Target | get | set | remove | clear |
|--------|-----|-----|--------|-------|
| JS/React/Vue/OmniView | `localStorage.getItem(k)` | `localStorage.setItem(k, v)` | `localStorage.removeItem(k)` | `localStorage.clear()` |
| Flutter | `SharedPreferences.getInstance().getString(k)` | `.setString(k, v)` | `.remove(k)` | `.clear()` |
| Rust / Python / Dart standalone / Dioxus | **编译报错** | 编译报错 | 编译报错 | 编译报错 |

**编译时校验**:
```rust
// codegen/mod.rs: generate() 入口
if has_stdlib_import("std/storage") && !target_supports_storage(target) {
    return Err(CompileError::StdlibNotAvailable {
        module: "std/storage",
        target,
        available: vec!["web", "flutter"],
    });
}
```

**预计改动**: ~12 处（stdlib.rs 注册 + target 校验逻辑 + 4 codegen arms × 2 targets + 错误消息）
**优先级**: 可随时实施，与 JSON stdlib 并行

---

## § 实施顺序与依赖

```
当前状态:
  ✅ Phase 0: await 表达式 — 已完成 (396 tests)
  ✅ Phase 0: Struct Literals — 已完成 (仅缺测试)

Phase 1 (可并行):
  ├── Feature 3: Union Construction               ← 依赖 Feature 2 (已完成)
  ├── Feature 6: JSON Stdlib                      ← 独立
  └── Feature 7: Storage Stdlib                   ← 独立

Phase 2:
  └── Feature 4: Match 表达式                     ← 依赖 Feature 2, 3

Phase 3:
  └── Feature 5: HTTP Stdlib                      ← 依赖 Feature 1, 2, 3, 4
```

**最快路径到可工作的 HTTP 请求**:
```
Feature 3 Union Construction → Feature 4 Match → Feature 5 HTTP
  (struct literal 已完成，JSON stdlib 可并行做)
```

**独立可做的低风险项**:
```
Feature 6 JSON Stdlib (1-2h) — 无需任何语言特性
Feature 7 Storage Stdlib (1h) — 无需任何语言特性
Feature 2 补充测试 (30min) — 验证已完成的 struct literal 功能
```

---

## § 测试策略

### 每个 Feature 的测试要求

| Feature | Parser 测试 | Codegen 测试 | 集成测试 | 语义测试 |
|---------|-----------|-------------|---------|---------|
| Struct Literals | `Point { x: 1, y: 2 }` + 嵌套 + 尾逗号 | JS/Rust/Dart 各 1 | — | — |
| Union Construction | 复用 struct parser | JS/Rust 各 1 (variant lit) | — | variant 名不存在时报错 |
| Match | 3 cases + wildcard + 嵌套 | 至少 JS + Rust | — | exhaustiveness + duplicate |
| HTTP Stdlib | — | 每个 target 1 个 | `http_get` 编译输出验证 | 不可用 target 报错 |
| JSON Stdlib | — | 每个 target 1 个 | `json_parse`/`json_stringify` 输出验证 | — |
| Storage Stdlib | — | Web 2 target 各 1 个 | `storage_get`/`storage_set` 输出验证 | 不可用 target 报错 |

### 回归测试

每次改动后运行:
```bash
cd omni-compiler && cargo test   # 当前 396 tests，改动后必须 ≥396
```

Rust 穷尽性检查会自动标记所有需要更新的 match——**编译不过 = 有后端没更新**。

---

## § 自审检查

### Placeholder scan
- ✅ 所有 AST/IR 类型定义完整
- ✅ 所有 codegen 映射表明确了每个 target 的生成方式
- ❌ 无 TODO/TBD

### Internal consistency
- ✅ Struct Literal infix `{` handler 不与 MapLiteral prefix `{` 冲突：prefix `{` → MapLiteral，infix `{` after PascalCase ident → StructLiteral（parser 已实现此逻辑）
- ✅ Variant construction 复用 StructLiteral parser —— lowering 阶段分辨
- ✅ Match exhaustiveness 在 semantic pass 检查 —— 不在 parser 检查
- ✅ HTTP HttpRequest.type_name 在 lowering 阶段能从 `stdlib.rs` 注册表查到
- ✅ `http_get` → `http_send` 展开是通过编译器内在函数，不是运行时调用
- ✅ JSON/Storage stdlib 独立于语言特性——可提前实施降低风险

### 实际进展 vs 计划

| 项目 | 计划状态 | 实际状态 |
|------|---------|---------|
| Feature 1: await | 未开始 | ✅ 完成 (396 tests) |
| Feature 2: struct lit | 未开始 | ✅ 完成 (仅缺测试，9/9 codegen 已实现) |
| Feature 6: JSON | 未列入 | 新增——独立可实施 |
| Feature 7: Storage | 未列入 | 新增——独立可实施 |

**关键发现**: Feature 2 的 codegen 并非 "0/9 完成"——在初始分析时未被发现，因为 grep 的是 `StructLiteral`（AST 名称）而非 `StructExpr`（IR 名称）。实际上所有 9 个后端早已实现 `ExprIR::StructExpr` 的处理。此发现将整体时间线缩短了 50%。

### 未解决的边缘情况
1. **变量名与 struct 名冲突**: `let HttpRequest = ...; HttpRequest { ... }` 会被 parser 当作 struct literal。目前按约定 (type PascalCase, var camelCase) 解决，后续 semantic pass 可加 lint。
2. **Match on non-union type**: parser 接受 `match 42 { _ => ... }`。语义 pass 应警告 `_` 是唯一的合法模式。
3. **Recursive lowering refactor**: 将 `&Program` 传入所有 lowering 函数需要改动 ~8 个函数签名。影响范围可控（仅 `ir/mod.rs` 内部）。
4. **Python codegen AwaitExpr**: 当前通过 wildcard 静默捕获——Python target 使用 await 时需显式添加 arm。
5. **StructExpr 穷尽性**: 已验证——所有 9 个后端都有显式 `ExprIR::StructExpr` arm，无需处理 wildcard 回退。

### Scope check
- ✅ 聚焦于 5 个语言特性 + 3 个 stdlib 模块，无蔓延
- ✅ `std/storage` 设计在 [[2026-06-19-omnilang-stdlib-design]] 中已完成
- ✅ JSON stdlib 作为 HTTP 的前置依赖独立实施——降低 Phase 4 风险

---

## § 改动文件清单

| 特性 | 文件 | 改动类型 | 状态 |
|------|------|---------|------|
| await | `ast.rs` | +1 Expr variant | ✅ |
| await | `expr.rs` | +1 prefix case, +1 span_of arm | ✅ |
| await | `ir/mod.rs` | +1 ExprIR variant, +1 lowering arm | ✅ |
| await | `semantic/lints.rs` | +1 match arm | ✅ |
| await | 8× `codegen/*.rs` | +1 match arm each | ✅ |
| await | `tests/parser_tests.rs` | +3 tests | ✅ |
| await | `tests/codegen_tests.rs` | +1 test | ✅ |
| await | JS/Rust/Dart codegen tests | +3 tests | ✅ |
| struct literal | `expr.rs` | parse_struct_literal_suffix, post-Pratt check | ✅ |
| struct literal | `ast.rs` | StructLiteral variant | ✅ |
| struct literal | `ir/mod.rs` | StructExpr variant + lowering | ✅ |
| struct literal | `semantic/lints.rs` | +1 match arm | ✅ |
| struct literal | 9× `codegen/*.rs` | +1 match arm each | ✅ |
| struct literal | `tests/parser_tests.rs` | +2 tests | ❌ |
| struct literal | `tests/codegen_tests.rs` | +2 tests | ❌ |
| union construction | `ir/mod.rs` | +1 ExprIR variant, refactor lowering fns to accept &Program | ❌ |
| union construction | 9× `codegen/*.rs` | +1 match arm each | ❌ |
| match expression | `ast.rs` | +4 new types (MatchExpr, MatchArm, MatchPattern, MatchPatternKind) | ❌ |
| match expression | `expr.rs` | +1 prefix case, +pattern parser | ❌ |
| match expression | `ir/mod.rs` | +1 ExprIR variant, +1 lowering arm | ❌ |
| match expression | `semantic/lints.rs` | +3 lint rules (exhaustiveness, no-duplicate, binding-count) | ❌ |
| match expression | 9× `codegen/*.rs` | +1 match arm each | ❌ |
| match expression | `tests/parser_tests.rs` | +3 tests | ❌ |
| http stdlib | **新** `stdlib.rs` | ~150 lines, 注册表 + codegen helpers | ❌ |
| http stdlib | `ir/mod.rs` | std import 识别 | ❌ |
| http stdlib | `codegen/mod.rs` | 入口 target 校验 | ❌ |
| http stdlib | 9× `codegen/*.rs` | std call emit | ❌ |
| http stdlib | `lib.rs` | 注册 stdlib 模块 | ❌ |
| http stdlib | `semantic/lints.rs` | 支持 stdlib 类型解析 | ❌ |
| json stdlib | `stdlib.rs` | +2 函数注册 | ❌ |
| json stdlib | 9× `codegen/*.rs` | std call emit for json_parse/stringify | ❌ |
| storage stdlib | `stdlib.rs` | +4 函数注册 + target 白名单 | ❌ |
| storage stdlib | `codegen/mod.rs` | target 可用性校验 | ❌ |
| storage stdlib | 2× `codegen/*.rs` (JS+Flutter) | std call emit | ❌ |
