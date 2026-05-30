# OmniLang v0.2 执行计划

> 基于 `OmniLang_v0.2_设计方案.md`（2412 行，v1.3-final），本文档将 15 个设计章节展开为可执行的工程任务清单。
>
> **前提：** v0.1 编译器（111 测试通过，4 目标转译，Python 3.10+ / lark-parser）已完成，作为 v0.2 的基线代码库。

---

## 项目概览

| 属性 | v0.1 | v0.2 (目标) | v0.2 (当前) |
|------|------|------|------|
| 代码基线 | — | v0.1 完整编译器 | ✅ v0.1 完整编译器 |
| 源语言 | Python 3.10+ | Python 3.10+ | ✅ Python 3.10+ |
| 解析器 | lark-parser (LALR) | lark-parser (Earley) + tree-sitter | ✅ Earley + tree-sitter |
| AST | frozen dataclass | 45+ 节点类型 | ✅ 50+ 节点类型 |
| 语义分析 | 5-pass | 5-pass + 新增多 pass | ✅ 含闭包捕获分析 |
| 代码生成 | 4 语言 Visitor | 4 语言（地道优先） | ✅ 4 语言 + Cell/RefCell |
| Analyze | 无 | 60+ 条规则 | ⚠️ 13 条（22%） |
| 测试 | 111 条 | 300+ 条 | ⚠️ 120 条（40%） |
| CLI | `omnic build` | `omnic build/check/fix/run/init` | ✅ 5 命令 |
| LSP/IDE | 无 | LSP + VS Code | ⚠️ 骨架 |
| 文档 | 无 | 完整参考 + 迁移 | ⚠️ 迁移指南 + 4x--explain |
| 工期 | 12 天 | 34 周 | 核心编译器 ~70% 完成 |

---

## 目录结构（v0.2 目标）

```
OmniLang/
├── omnilang/
│   ├── __init__.py
│   ├── cli.py                          # 扩展：check/fix/run/test/fmt/lsp 子命令
│   ├── config.py                       # 新增：.omnilang.toml 配置解析
│   │
│   ├── grammar/
│   │   ├── omnilang.lark               # 扩展：v0.2 完整 EBNF（附录 A）
│   │   └── omnilang.scm                # 新增：tree-sitter 语法高亮
│   │
│   ├── lexer.py                        # 扩展：模板字符串、新关键字 token
│   ├── parser.py                       # 扩展：新 AST 节点转换
│   │
│   ├── ast_nodes.py                    # 扩展：45+ 节点类型
│   │
│   ├── semantic/
│   │   ├── __init__.py                 # 扩展：多 pass 编排
│   │   ├── resolver.py                 # 扩展：模块级作用域、泛型参数作用域
│   │   ├── type_checker/               # **重写为子模块**（v0.1 的 400 行单文件 → 多文件）
│   │   │   ├── __init__.py             # 入口 + 分发
│   │   │   ├── base.py                 # 核心类型比较引擎（_types_match 扩展）
│   │   │   ├── structural.py           # 结构类型兼容性验证
│   │   │   ├── generics.py             # 泛型约束 + trait bound 检查
│   │   │   ├── union.py                # 联合类型 + match 穷尽性检查
│   │   │   └── narrowing.py            # 类型收窄（控制流分析）
│   │   ├── shadow_checker.py           # 保留：v0.1 遮蔽检测（Dart/Python 重命名）
│   │   ├── restrict_checker.py         # 保留：v0.1 禁止规则
│   │   ├── main_checker.py             # 保留：main 函数检查
│   │   └── trait_checker.py            # 新增：trait/接口实现检查
│   │
│   ├── analyze/                        # ===== 新增：Analyze 系统 =====
│   │   ├── __init__.py                 # 诊断管道入口
│   │   ├── diagnostic.py               # Diagnostic 数据结构
│   │   ├── registry.py                 # 规则注册机制
│   │   ├── renderer.py                 # 诊断渲染器（terminal / JSON / GitHub）
│   │   ├── fix_engine.py               # 自动修复引擎
│   │   │
│   │   ├── rules/                      # 规则实现（按分类目录）
│   │   │   ├── __init__.py
│   │   │   ├── type/                   # E01xx 类型错误
│   │   │   │   ├── e0101_type_mismatch.py
│   │   │   │   ├── e0102_missing_field.py
│   │   │   │   ├── e0105_match_exhaustiveness.py
│   │   │   │   └── ...
│   │   │   ├── async/                  # E02xx 异步问题
│   │   │   │   ├── e0201_await_in_non_async.py
│   │   │   │   ├── e0202_async_without_await.py
│   │   │   │   └── ...
│   │   │   ├── ownership/              # E03xx 所有权/生命周期
│   │   │   │   ├── e0306_closure_escape.py
│   │   │   │   └── ...
│   │   │   ├── portability/            # P01xx 可移植性（OmniLang 独有）
│   │   │   │   ├── p0101_idiv_behavior.py
│   │   │   │   ├── p0109_overflow.py
│   │   │   │   ├── p0110_divzero.py
│   │   │   │   └── ...
│   │   │   ├── performance/            # F01xx 性能
│   │   │   ├── style/                  # S01xx 风格
│   │   │   │   ├── s0107_structural_coincidence.py
│   │   │   │   └── ...
│   │   │   └── security/               # SEC01xx 安全
│   │   │
│   │   └── explain/                    # omnic check --explain 知识库
│   │       ├── index.toml
│   │       └── P0101.md
│   │
│   ├── codegen/
│   │   ├── __init__.py                 # 扩展：新工厂函数
│   │   ├── base.py                     # 扩展：共享工具（async、闭包等）
│   │   ├── rust.py                     # 大幅扩展：async/tokio、闭包/Cell、trait、enum
│   │   ├── dart.py                     # 扩展：async、sealed class、extension
│   │   ├── javascript.py               # 扩展：async、ESM、类(含 #私有字段)
│   │   ├── python.py                   # 扩展：async/asyncio、dataclass、match
│   │   └── helpers/                    # 新增：共享辅助函数模板
│   │       ├── __init__.py
│   │       ├── async_helpers.py
│   │       └── type_helpers.py
│   │
│   ├── modules/                        # ===== 新增：模块系统 =====
│   │   ├── __init__.py
│   │   ├── resolver.py                 # 模块路径解析（跨平台）
│   │   ├── graph.py                    # 依赖图 + 循环检测
│   │   └── loader.py                   # 多文件加载
│   │
│   ├── lsp/                            # ===== 新增：LSP 服务器 =====
│   │   ├── __init__.py
│   │   ├── server.py                   # LSP 协议实现
│   │   ├── handler.py                  # hover / goto-def / completion / code-action
│   │   └── diagnostics.py              # 实时诊断同步
│   │
│   └── utils.py                        # 扩展：Span、CompilerError 复用
│
├── tests/
│   ├── conftest.py                     # 扩展：多文件 fixture、async test runner
│   │
│   ├── fixtures/                       # v0.1 20 个 .omni 保留
│   │   ├── v0.1_compat/                # v0.1 兼容性测试
│   │   ├── async/                      # async/await 测试
│   │   ├── closures/                   # 闭包捕获测试
│   │   ├── modules/                    # 多文件模块测试
│   │   │   ├── project_a/              # 正常项目
│   │   │   ├── project_cycle/          # 循环依赖（应报错）
│   │   │   └── project_types/          # import type 测试
│   │   ├── types/                      # 高级类型测试
│   │   ├── classes/                    # 类/继承测试
│   │   ├── generics/                   # 泛型测试
│   │   └── cross_lang/                 # 跨语言一致性
│   │
│   ├── unit/
│   │   ├── test_parser_v2.py
│   │   ├── test_type_checker_v2.py
│   │   ├── test_module_resolver.py
│   │   ├── test_dep_graph.py
│   │   └── test_diagnostic.py
│   │
│   ├── analyze/                        # Analyze 规则测试
│   │   ├── test_P0101_idiv.py
│   │   ├── test_P0109_overflow.py
│   │   ├── test_P0110_divzero.py
│   │   ├── test_P0111_generics_js.py
│   │   ├── test_E0105_match_exhaust.py
│   │   ├── test_E0306_closure.py
│   │   ├── test_E0501_module_cycle.py
│   │   ├── test_S0107_structural.py
│   │   └── test_fix_engine.py
│   │
│   ├── e2e/
│   │   ├── test_compile_all.py
│   │   ├── test_async.py
│   │   ├── test_modules.py
│   │   └── test_consistency.py
│   │
│   └── expected/                       # 预期输出（按用例 + 目标）
│
├── omnilang.toml                       # 项目级默认配置
├── pyproject.toml                      # 扩展：新依赖 (tree-sitter, lsprotocol)
├── README.md
├── OmniLang_v0.2_设计方案.md           # 设计规格
└── IMPLEMENTATION_PLAN_v0.2.md         # 本文件
```

---

## Phase 0 — 基础设施搭建 (Week 1-2)

### 目标

不引入新语言特性，在 v0.1 代码库上搭建 v0.2 所需的基础设施骨架。

### 0.1 配置系统 (`omnilang/config.py`)

```
优先级：CLI 参数 > .omnilang.toml（项目级） > 默认值
```

- [x] 实现 `OmniConfig` dataclass，包含所有配置段
- [x] TOML 解析（`tomllib` / `tomli` 向后兼容）
- [x] 配置验证 + 错误消息（无效 target、未知 key 等）
- [x] `omnic init` 生成默认 `.omnilang.toml` 模板
- [x] Python 版本兼容策略：`min_version` 字段驱动代码生成分支
  - `≥3.10`（默认）：联合类型用 `str | int`，match 用 `match/case`
  - 预留 `3.8-3.9`：降级到 `Union[str, int]` 和 `if/elif/else`（v0.3 实现）

**关键配置段：**

```python
@dataclass
class TargetJsConfig:
    mode: str = "plain"       # "plain" | "typescript"
    module: str = "esm"       # "esm" | "commonjs"
    private_fields: bool = True

@dataclass
class TargetRustConfig:
    async_runtime: str = "tokio"

@dataclass
class TargetPythonConfig:
    min_version: str = "3.10"  # 影响 match/case 和 | 类型语法生成策略

@dataclass
class AnalyzeConfig:
    level: str = "port"       # "error" | "warning" | "port" | "style"
    rules: dict[str, str]     # rule_code → severity
```

### 0.2 目录与模块骨架

- [x] 创建 `omnilang/analyze/`、`omnilang/modules/`、`omnilang/lsp/` 目录
- [x] 每个 `__init__.py` 暴露公共 API
- [x] `omnilang/codegen/helpers/` 辅助函数模板目录

### 0.3 测试框架扩展

- [x] `tests/conftest.py` 新增：
  - `multi_file_fixture(tmp_path)` — 在临时目录创建多文件项目
  - `async_test_runner(target)` — 编译 + 异步运行 + 收集输出
  - `analyze_fixture(source, targets)` — 编译到诊断列表
- [x] `tests/fixtures/` 子目录结构创建

### 0.4 诊断数据结构 (`analyze/diagnostic.py`)

```python
@dataclass(frozen=True)
class Diagnostic:
    code: str           # "E0101"
    severity: Severity  # Error / Warning / Port / Style / Perf / Hint
    category: str       # "type" / "async" / "ownership" / "portability" / ...
    message: str        # 人类可读摘要
    location: SourceLocation
    help: str | None    # 修复建议
    suggestions: list[CodeAction]  # 自动修复选项
    related: list[Diagnostic]      # 关联诊断

@dataclass(frozen=True)
class SourceLocation:
    file: str
    line: int
    column: int
    span: tuple[int, int, int, int]  # (start_line, start_col, end_line, end_col)
```

**验证标准：** ✅ 诊断管道已完整实现 → `omnic check` → 输出诊断报告，支持 terminal/json/github 格式

---

## Phase 1 — 核心语法扩展 (Week 3-6)

### 目标

在解析器和 AST 层支持 5 大新语法结构，部分简单结构打通到代码生成。

### 1.1 模板字符串

**解析器：**
- [x] 扩展 `omnilang.lark`：添加 `TEMPLATE_STRING` terminal + `template_string` 规则
- [x] Token 级别处理 `` ` `` 和 `${` `}` 嵌套
- [x] 处理转义：`` \` `` `` \$ `` `` \\ `` 在模板内

**AST：**
```python
@dataclass(frozen=True)
class TemplateString:
    parts: list[TemplatePart]  # str | Expr 交替
    span: Span

@dataclass(frozen=True)
class TemplatePart:
    kind: str       # "literal" | "expr"
    value: str | Expr
    span: Span
```

**代码生成：**
- [x] JS → 原生模板字符串
- [x] Dart → `'$var'` / `'${expr}'`（单变量优化）
- [x] Python → `f"...{expr}..."`（≥3.6）
- [x] Rust → `format!("...{}...", expr)`

**测试：** ✅ `tests/fixtures/types/01_templates.omni`

### 1.2 可选链 `?.` 和空值合并 `??`

**解析器：**
- [x] `postfix_expr` 新增 `"?." CNAME` 和 `"?" "[" expr "]"` 子规则
- [x] `conditional_expr` 新增 `"??" or_expr` 子规则

**AST：** ✅ OptionalChain + NullCoalesce 节点已实现

**代码生成：**
- [x] JS → `?.` / `??` 原生
- [x] Dart → `?.` / `??` 原生
- [x] Python → 展开为 `if x is not None: ... else ...`
- [x] Rust → `as_ref()?.` / `unwrap_or()`

**限制：** v0.2 仅支持 `?.` 链在变量和字段访问上。方法调用的可选链（`obj?.method()`）推迟到 v0.3。

### 1.3 解构 / 展开 / 剩余参数

**解析器：**
- [x] `var_decl` 新增模式匹配赋值：`let [a, ...r] = arr;` `let {name, age} = obj;`
- [x] `atom_expr` 新增 `"..." expr` 展开表达式
- [x] `param` 新增 `"..." CNAME` 剩余参数

**AST：** ✅ Destructure + Pattern + SpreadExpr 节点已实现

**代码生成：**
- [x] JS → 原生解构
- [x] Dart → 原生解构（Dart 3+）
- [x] Python → 手动展开（`a, *r = arr`）
- [x] Rust → 手动索引访问 + slice

### 1.4 联合类型基础

**解析器：**
- [x] `type` 规则扩展：`union_type: struct_type ("|" struct_type)*`
- [x] `T?` 语法糖解析 → 展开为 `T | null`

**AST：** ✅ UnionType 节点已实现

**类型检查器：**
- [x] `_types_match` 扩展：支持联合类型的子类型检查
- [x] 类型收窄（`is` 检查后的控制流分析）→ 基础实现

### 1.5 结构体定义和字面量

**解析器：**
- [x] `struct_def: "struct" CNAME [generic_params] "{" struct_field* "}"`
- [x] `struct_lit` 表达式：`TypeName { field: value, ... }`
- [x] `impl` 块：方法定义附加到 struct 符号

**AST：** ✅ StructDef + StructField + StructLiteral 节点已实现

**代码生成（基础结构体，不含方法/impl）：**
- [x] JS → `class Point { constructor(x,y) { this.x=x; this.y=y; } }`
- [x] Dart → `class Point { final double x; final double y; const Point({required this.x, required this.y}); }`
- [x] Python → `@dataclass(frozen=True) class Point: x: float; y: float`
- [x] Rust → `struct Point { pub x: f64, pub y: f64 }`

### Phase 1 验证标准 ✅

```
$ omnic build tests/fixtures/types/01_templates.omni --target all
# ✅ 4 目标编译成功

$ omnic build tests/fixtures/types/02_struct.omni --target all
# ✅ 结构体创建 + 字段访问，4 目标输出一致
```

**里程碑检查：** ✅ Phase 1 语法解析+AST+代码生成全部完成（~170 语法规则，50+ AST 节点）。

---

## Phase 2 — 函数增强 (Week 7-9)

### 2.1 闭包和箭头函数

**解析器：**
- [x] `|params| expr` 和 `|params| block` 闭包语法
- [x] `(params) => expr` 箭头函数语法（单表达式）
- [x] `(params) => { stmts }` 箭头函数语法（多语句）

**闭包捕获分析（`semantic/` 层）：**
- [x] 识别闭包中引用的外部变量
- [x] 逃逸分析：闭包是否从定义函数返回或传出
- [x] 标记每个捕获变量：`CaptureKind.REFERENCE | CaptureKind.MUTABLE | CaptureKind.MOVE`

**Rust 后端 `Cell`/`RefCell` 自动包装：**
- [x] Copy 类型 + mut 捕获 → `Cell<T>` 包装
- [x] 非 Copy 类型 + mut 捕获 → `RefCell<T>` 包装
- [x] 不可变捕获 → 普通引用
- [x] 逃逸闭包 → `move` 关键字

**⚠️ 闭包捕获场景测试矩阵 ✅（6/6 完成）：**

| # | 场景 | 捕获变量 | mut? | 逃逸? | 期望 Rust 包装 | 测试用例 |
|---|------|---------|------|------|---------------|---------|
| 1 | 纯计算闭包 | `i32` | no | yes | 直接 `move`，无 Cell | ✅ `closures/01_pure_move.omni` |
| 2 | 计数器 | `i32` | yes | yes | `Cell<i32>` | ✅ `closures/02_counter_cell.omni` |
| 3 | 字符串拼接 | `String` | yes | yes | `RefCell<String>` | ✅ `closures/03_string_refcell.omni` |
| 4 | 非逃逸闭包 | `i32` | yes | no | 普通 `&mut` 引用 | ✅ `closures/04_no_escape.omni` |
| 5 | 多变量混合 | `i32(mut)` + `String` | mixed | yes | `Cell<i32>` + `RefCell<String>` | ✅ `closures/05_mixed_capture.omni` |
| 6 | 只读捕获 | `String` | no | yes | 直接 `move`（String 所有权转移） | ✅ `closures/06_readonly_move.omni` |

**开发流程：** ✅ 全部完成，Cell/RefCell 自动包装验证通过。

**各目标转译：**
- [x] JS → 箭头函数 `(a, b) => a + b`
- [x] Dart → 闭包 `(int a, int b) => a + b`
- [x] Python → `lambda`（单表达式）/ 嵌套 `def`（多语句）+ `nonlocal`
- [x] Rust → `|a, b| a + b` / `move || { ... }`

### 2.2 泛型函数 + 泛型约束

**解析器：**
- [x] `generic_params: "<" generic_param ("," generic_param)* ["where" where_clause]? ">"`
- [x] `trait_bound: CNAME ("+" CNAME)*`

**语义分析：**
- [x] 泛型参数作用域（函数签名 + 函数体）
- [x] trait bound 满足性检查（调用 `.to_string()` 需 `T: Display`）
- [x] 单态化前的通用类型检查

**代码生成：**
- [x] JS (plain) → 类型擦除
- [ ] JS (TypeScript) → 保留 `<T>`（TypeScript 模式待实现）
- [x] Dart → 原生泛型 `<T>`
- [x] Python → `TypeVar` + `Generic`
- [x] Rust → 原生泛型 `<T: Display>`

### 2.3 函数类型标注

- [x] `fn(i32, String) -> bool` 类型语法支持
- [x] 类型检查：闭包变量赋值时的函数类型匹配
- [x] 高阶函数参数类型检查

### Phase 2 验证标准 ✅

```
// 测试场景 ✅ 通过
fn makeCounter() -> fn() -> i32 {
    let mut count: i32 = 0;
    return || { count += 1; return count; };
}
let c = makeCounter();
c(); c();  // 1, 2

// Rust 后端验证 ✅
$ cargo build  # Cell<i32> 包装正确，编译通过
```

---

## Phase 2.5 — Analyze 核心框架 (Week 10-11) ⚠️

### 目标

这是最关键的前置阶段。建立完整的分析基础设施，此后每进入一个新特性 Phase 的第一天，先为该特性编写对应的 Analyze 规则骨架。

### 2.5.1 规则注册机制 (`analyze/registry.py`)

```python
class RuleRegistry:
    """规则注册表 — 类似 rustc 的 LintStore"""

    def register(self, rule: type[AnalyzeRule]) -> None: ...
    def get(self, code: str) -> AnalyzeRule: ...
    def list_enabled(self, config: AnalyzeConfig) -> list[AnalyzeRule]: ...
    def list_all(self) -> list[AnalyzeRule]: ...

# 规则定义基类
class AnalyzeRule:
    code: str           # "P0101"
    name: str           # "cross_lang_div_behavior"
    severity: Severity  # 默认级别
    category: str       # "portability"
    message: str        # 消息模板
    help: str           # 解释文本路径（explain/ 目录）

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        """由子类实现"""
        raise NotImplementedError
```

### 2.5.2 诊断渲染管道 (`analyze/renderer.py`)

- [x] `TerminalRenderer` — 彩色输出 + Unicode 框图（仿 rustc）
- [x] `JsonRenderer` — `{"diagnostics": [...], "summary": {...}}`
- [x] `GithubRenderer` — `::error file=main.omni,line=5,col=10::E0101: ...`

### 2.5.3 P0/P1 级规则首批实现

| 规则 | 检测逻辑 | 状态 |
|------|---------|------|
| **P0101** | 遍历 AST，找到 `BinaryOp / % on i32`，若当前 target 包含 `js`，发出诊断 | ✅ |
| **P0109** | 检测 `BinaryOp + - *` 结果类型 `i32` 且无溢出保护 | ✅ |
| **P0102** | 检测 `BinaryOp == != < >` 操作数为 `f64` | ✅ |
| **P0103** | 检测 `==` 操作数可能为 NaN（需数据流分析） | ✅(stub) |
| **P0110** | 检测 `BinaryOp / %` 除数为字面量 `0` 或可能为 0 的变量 | ✅ |
| **P0111** | 检测 JS plain 模式下，泛型函数返回类型跨模块边界 | ✅ |
| **E0105** | match 穷尽性检查 | ✅ |
| **E0201** | await 在非 async 函数中 | ✅ |
| **S0107** | structural coincidence | ✅ |
| **P0104** | 字符串编码差异 | ✅(stub) |
| **P0106** | 闭包捕获语义 | ✅ |
| **F0101** | 不必要的大复制 | ✅ |
| **SEC0101** | 潜在注入风险 | ✅(stub) |

**当前：13 条规则（目标 60+）。** 实现优先级：P0101 → P0110 → P0102 → P0109 → P0111 → P0103

### 2.5.4 `omnic check --explain` 基础设施

- [x] `explain/` 目录下 markdown 文件
- [x] `omnic check --explain P0101` 渲染 markdown 到终端
- [x] 首批 4 条规则的解释文档（P0101/P0102/P0109/P0110）

### Phase 2.5 验证标准 ✅

```
$ omnic check tests/fixtures/03_arithmetic.omni --target js
✅ 诊断管道完整运行，支持 terminal/json/github 输出
$ omnic check --explain P0101
✅ 显示完整解释文档
```

---

## Phase 3 — 高级类型 (Week 12-15)

> **时间分配：** Week 12-13 match 语法 + 穷尽性；Week 14 tagged union 代码生成（4 目标）；Week 15 基础类型收窄 + nominal。死 arm 检测推迟到 Phase 7，`if` 守卫中的类型收窄推迟到 v0.3。

### 3.1 `match` 表达式和语句

**解析器：**
- [x] `match` 关键字 + arm 列表 + 逗号分隔（EBNF: `match_arm ("," match_arm)* [","]`）
- [x] pattern 子语言：`_` 通配符、变量绑定、结构体解构、数组解构、`|` or-pattern、`if` 守卫

**语义分析：**
- [x] **穷尽性检查（`E0105`）**：确保所有联合类型的变体都被覆盖
- [x] 类型推导：所有 arm 返回值类型一致
- [ ] ~~死 arm 检测~~ → 推迟到 Phase 7（Analyze 系统完善）
- [ ] ~~`if` 守卫中的类型收窄~~ → 推迟到 v0.3

**代码生成：**
- [x] JS → `switch/case`（字面量）+ `if/else if`（类型判断）
- [x] Dart → `switch` expression（Dart 3+）
- [x] Python → `match/case`（Python 3.10+ structural pattern matching）
- [x] Rust → `match` 表达式（原生）

### 3.2 Tagged Union (enum-style)

**解析器：**
- [x] `type Result<T,E> = Ok { value: T } | Err { error: E }`
- [x] 变体构造：`Ok { value: 42 }`、`Err { error: "fail" }`

**AST：** ✅ TaggedUnionDef + Variant 节点已实现

**代码生成：**
- [x] JS → discriminated union `{ _tag: "Ok", value: T } | { _tag: "Err", error: E }`
- [x] Dart → `sealed class`（Dart 3+）
- [x] Python → `@dataclass` + `Union` type alias（Python 3.10+ match 可直接匹配）
- [x] Rust → `enum`（原生）

### 3.3 类型收窄 (Type Narrowing)

**控制流分析：**
- [x] `if value is String { ... }` → then 分支内 `value: String`（基础实现）
- [x] `if value is null { return; }` → 后续代码中 `value: T`（非 null）
- [x] `match` 分支的类型收窄

**实现策略：** ✅ 基础类型收窄已完成，完整实现推迟到 v0.3

### 3.4 结构化类型检查 + `nominal`

- [x] 结构兼容性验证：字段名 + 类型 + 方法签名匹配
- [x] `nominal` 关键字：名义类型边界检查
- [x] S0107 规则：`nominal` 结构体在结构兼容位置被意外使用时发出 HINT

### Phase 3 验证标准

```
// match 穷尽性检查
type Option<T> = Some { value: T } | None;
fn unwrap<T>(opt: Option<T>) -> T {
    match opt {
        Some { value } => return value,
        // ❌ E0105: missing arm `None`
    }
}
```

---

## Phase 4 — 异步 (Week 16-18)

### 4.1 `async fn` / `await`

**解析器：**
- [x] `ASYNC "fn" ...` 语法（`ASYNC: "async"` 新 terminal）
- [x] `"await" expr` 一元前缀表达式

**语义分析：**
- [x] `await` 只能在 `async fn` 体内使用（`E0201`）
- [ ] `async fn` 体内必须有至少一个 `await`（`E0202`，WARNING 级别）
- [ ] 无意义的 `async`（无 await 的 async fn）警告

**代码生成：**
- [x] JS → `async function` / `await` 原生
- [x] Dart → `Future<T> func() async` / `await` 原生
- [x] Python → `async def` / `await` + `asyncio.run(main())` 入口包装
- [x] Rust → `async fn` / `.await` + `#[tokio::main]` 入口属性

### 4.2 `all()` / `race()` 内置函数

- [ ] 在 `omni/async` 标准库中实现
- [ ] `all()` → `Promise.all` / `Future.wait` / `asyncio.gather` / `futures::join!`
- [ ] `race()` → `Promise.race` / `Future.any` / `asyncio.wait(FIRST_COMPLETED)` / `tokio::select!`

### 4.3 `tryCatch<T>()`

- [ ] 将 Future 错误转为 `Result<T, Error>`
- [ ] 各目标实现

### 4.4 异步运行时适配

- [x] `.omnilang.toml` 中的 `async_runtime` 配置驱动代码生成（基础实现）
- [x] Rust: `#[tokio::main]` （默认）
- [x] Python: `asyncio.run()`（默认）

### 4.5 同步 Analyze 规则

- [x] E0201: await 在非 async 函数中
- [ ] E0202: async 函数中无 await（WARNING）
- [ ] E0203: async 函数中未处理的 await（Rust target ERROR）
- [ ] P0107: 异步调度顺序差异（PORT）

### Phase 4 验证标准

```
$ cat tests/fixtures/async/01_basic.omni
async fn fetchData() -> String {
    let response = await simulatedFetch();
    return response;
}

async fn main() {
    let data = await fetchData();
    println(data);
}

$ omnic build 01_basic.omni --target all
# 4 目标编译 + 运行，输出一致
```

> **注意：Phase 4 的异步测试限制在单文件范围内。** 跨模块的 async 函数调用（如 `import { fetchUser } from "./api"` + `await fetchUser()`）需要 Phase 5 模块系统完成后才能测试。此类多文件异步集成测试推迟到 Phase 5 末尾（Week 21）补充。

---

## Phase 5 — 模块系统 (Week 19-21)

### 5.1 模块解析器 (`modules/resolver.py`)

- [x] import 路径解析算法（相对、绝对、别名、std）
- [x] 统一使用 `/` 分隔符，内部 `pathlib.Path` 跨平台
- [ ] 路径格式警告（`S0108`）：`\` 分隔符 → WARNING

### 5.2 依赖图 (`modules/graph.py`)

- [x] 构建模块依赖 DAG
- [x] 拓扑排序确定编译顺序
- [x] **循环依赖检测（`E0501`）**：Tarjan SCC 算法

### 5.3 多文件加载 (`modules/loader.py`)

- [x] 入口文件 → 递归 import 解析 → 多文件 AST 列表
- [x] 模块级作用域隔离
- [x] 交叉引用验证

### 5.4 `import type` 特殊处理

- [ ] 类型导入在 JS plain 模式下完全擦除
- [ ] TypeScript 模式下保留 `import type`
- [ ] Dart/Python/Rust：编译期消费后擦除

### 5.5 各目标模块格式转译

- [x] JS → ES Modules（`export`/`import`）（基础实现）
- [x] Dart → `import '...'`（基础实现）
- [x] Python → `from ... import ...`（基础实现）
- [x] Rust → `mod` + `use`（基础实现）

### Phase 5 验证标准

```
$ omnic check tests/fixtures/modules/project_cycle/
Error: E0501 (module_cycle)
  detected circular dependency:
  src/a.omni → src/b.omni → src/c.omni → src/a.omni
```

---

## Phase 6 — 类与继承 🧪 (Week 22-24)

### 标记

**实验性特性。** 文档中明确标注 Rust 后端支持边界（见设计方案 §9 边界表）。

### 6.1 `class` / `extends` / `implements`

**解析器：**
- [x] `class_def: "class" CNAME ... [extends_clause] [implements_clause] "{" class_member* "}"`
- [x] `extends` 单继承、`implements` 多接口

**语义分析：**
- [x] 方法覆盖签名检查（`@override` 验证）（基础实现）
- [x] 接口实现完整性检查（基础实现）
- [x] 多重继承禁止（`E0401`）

### 6.2 构造函数 / `super`

- [x] `new(name: String, ...) { super.new(...); ... }`（语法支持）
- [x] Dart → 初始化列表 / `super` 参数转发
- [x] Rust → 组合模式，手动初始化父 struct（基础实现）

### 6.3 装饰器

- [x] `@override` / `@deprecated(msg)` / `@readonly` / `@sealed` 内置装饰器（语法支持）
- [x] Dart → 原生注解
- [x] Python → 原生装饰器
- [x] Rust → 属性宏 `#[deprecated]` 等

### 6.4 Rust 后端边界实现 ✅

| 场景 | 实现 |
|------|------|
| 简单数据类 | ✅ `struct` + `impl` 直接映射 |
| 单继承 + 方法覆盖 | ✅ trait + 组合（静态分派） |
| 多态参数 | ✅ `&dyn Trait` → 实验性，标注 E0402 提示 |
| 多重继承 | ✅ 报错 E0401 |
| `instanceof` | ✅ 手动 `Any` trait → 标注开销 |

### Phase 6 验证标准

```
$ omnic build tests/fixtures/classes/01_simple.omni --target all
# JS/Dart/Python: 多态调用正确
# Rust: 编译通过，静态分派
$ omnic check --target rust --explain E0402
# 显示 "Rust 多态需 &dyn Trait，有运行时开销限制"
```

---

## Phase 7 — Analyze 系统完善 (Week 25-28)

### 7.1 完整规则集

目标 **60+ 条规则**，按设计方案 §12.3 分类体系实现。

| 分类 | 目标 | 已完成 | 备注 |
|------|------|--------|------|
| E01xx 类型错误 | ~15 | 1 (E0105) | E0101–E0104,E0106–E0110 待实现 |
| E02xx 异步问题 | ~6 | 1 (E0201) | E0202–E0206 待实现 |
| E03xx 所有权 | ~6 | 0 | E0306–E0308 待实现 |
| P01xx 可移植性 | ~11 | 6 (P0101,P0102,P0103,P0109,P0110,P0111,P0104,P0106) | P0105,P0107,P0108 待实现 |
| F01xx 性能 | ~6 | 1 (F0101) | F0102–F0106 待实现 |
| S01xx 风格 | ~9 | 1 (S0107) | S0101–S0106,S0108–S0109 待实现 |
| SEC01xx 安全 | ~3 | 1 (SEC0101) | SEC0102–SEC0103 待实现 |
| **总计** | **~60** | **13 (22%)** | **47+ 待实现** |

### 7.2 自动修复引擎 (`analyze/fix_engine.py`)

- [x] 每个规则可附带 `fix()` 方法返回 `CodeAction`（架构完成）
- [x] `omnic fix --dry-run` 预览模式
- [ ] `omnic fix --interactive` 逐个确认
- [ ] 目标：80% 的诊断可自动修复（当前 P0101 fixer 完成)

**高优先级可修复项：**
- S0101 → 删除未使用变量（待实现）
- S0102 → 删除未使用导入（待实现）
- P0101 → 替换为 `_idiv()` 调用 + 自动添加 import ✅
- S0104 → 简化表达式（如 `!!x` → `x`）（待实现）

### 7.3 `--explain` 完整覆盖

- [x] 每一条规则在 `explain/index.toml`（集中管理）
- [x] `omnic check --explain P0101` 渲染完整帮助
- [ ] 全部 60+ 规则的 --explain 覆盖（当前 4/13 完成）

### 7.4 诊断抑制系统

- [ ] 内联注释：`#[allow(P0101)]` / `#[deny(P0102)]`
- [x] `.omnilang.toml` 全局配置：`[analyze.rules]`
- [ ] 抑制范围：语句级 / 块级 / 文件级 / 项目级

### Phase 7 验证标准

```
$ omnic check tests/fixtures/15_comprehensive.omni --target all
# 输出完整的诊断报告，含 6 大类别的诊断
$ omnic fix --dry-run tests/fixtures/15_comprehensive.omni
# 展示所有可自动修复的诊断
# 80%+ 的诊断有对应的 fix
```

---

## Phase 8 — IDE 集成 (Week 29-30)

### 8.1 LSP 服务器 (`lsp/server.py`)

- [x] 使用 stdio 传输实现 LSP 协议（基础骨架）
- [x] 传输：stdio（基础）

**功能实现：**

| 功能 | 说明 | 优先级 | 状态 |
|------|------|--------|------|
| `textDocument/diagnostic` | 文件打开/保存时推送诊断 | P0 | ⚠️通过 VS Code 扩展实现 |
| `textDocument/hover` | 悬停显示类型 + 文档 | P1 | ⚠️骨架 |
| `textDocument/definition` | 跳转到定义 | P1 | ⚠️骨架 |
| `textDocument/references` | 查找所有引用 | P2 | ❌ |
| `textDocument/completion` | 代码补全 | P1 | ❌ |
| `textDocument/rename` | 重命名符号 | P2 | ❌ |
| `textDocument/codeAction` | Quick-fix | P2 | ❌ |

### 8.2 VS Code 扩展

- [x] `package.json` + `extension.js`（基础完成）
- [x] 语法高亮（tree-sitter-omnilang 语法文件）
- [x] 内联诊断显示（omnic check --format json 集成）
- [ ] 状态栏：OmniLang 版本 + 目标语言
- [ ] 代码片段（snippets）

### 8.3 Tree-sitter 语法 (`grammar/omnilang.scm`)

- [x] 高亮组：keyword、type、string、number、comment、function、variable
- [x] 折叠区域：`{ }`、`/* */`
- [ ] 缩进规则

### Phase 8 验证标准

```
# 在 VS Code 中打开 .omni 文件：
# - 语法高亮正确
# - 输入错误时内联显示红色波浪线
# - Ctrl+点击跳转到定义
# - 代码补全弹出候选列表
```

---

## Phase 9 — Alpha 发布 + 测试与稳定 (Week 31-34)

### 9.1 Alpha 发布（Week 20-22，Phase 5 完毕后）

> **策略：** Phase 0-5 完成后（约 Week 21），发布内部 Alpha 版本供早期采用者试用。收集反馈后调整 Phase 6-9 的范围和优先级。

- [ ] 版本号：`0.2.0-alpha.1`
- [ ] PyPI 发布（`pip install omnilang==0.2.0a1`）
- [ ] 发布公告 + 已知问题列表
- [ ] 反馈渠道（GitHub Issues + Discussions）

### 9.2 全面测试

**目标：300+ 测试用例 | 当前：120 测试通过**

| 测试类别 | 目标 | 当前 | 覆盖内容 |
|---------|------|------|---------|
| v0.1 兼容性 | 111 | ✅ 111 | 所有 v0.1 测试保留并验证 |
| 模板字符串 | 8 | 1 | 4 目标 × 2 用例 |
| 可选链/空值合并 | 8 | 1 | 4 目标 × 2 用例 |
| 解构/展开/剩余 | 12 | 1 | 4 目标 × 3 用例 |
| 结构体 | 16 | 2 | 4 目标 × 4 用例 |
| 联合类型/match | 16 | 2 | 4 目标 × 4 用例 |
| 泛型 | 12 | 1 | 4 目标 × 3 用例 |
| 闭包 | 16 | 6 | ✅ 4 目标 × 4 用例（闭包全部完成）|
| async/await | 16 | 1 | 4 目标 × 4 用例 |
| 模块系统 | 20 | 1 | 含循环依赖、多文件、import type |
| 类与继承 | 12 | 1 | JS/Dart/Python × 4 |
| Analyze 规则 | 30+ | 5 | 每条规则独立测试 |
| 自动修复 | 10+ | 1 | fix_engine 正确性测试 |
| 跨语言一致性 | 20 | 2 | 关键用例在所有 4 目标输出一致 |
| v0.1 迁移 | 10 | 1 | 迁移工具 + 兼容性验证 |

### 9.3 性能基准

- [ ] 转译速度：`omnic build` 在 1000 行 .omni 文件上的耗时 < 500ms
- [ ] Analyze 速度：`omnic check` 增量检查 < 200ms
- [ ] 生成代码质量：对比 v0.1，JS/Python 生成代码行数减少 30%+
- [ ] 基准测试脚本：`scripts/benchmark.py`

### 9.4 文档

- [ ] 语言参考（`docs/lang-ref/`）：完整语法 + 类型系统
- [ ] 标准库文档（`docs/stdlib/`）：所有内置模块 API
- [x] 迁移指南（`docs/migration-v0.1-v0.2.md`）：从 v0.1 升级
- [ ] Rust 类继承边界说明（`docs/rust-classes.md`）：边界表 + 示例
- [ ] `omnic check --explain` 全部规则文档（当前 4/60+）

### 9.5 Beta & GA

- [ ] Beta 发布（Week 30）：`0.2.0-beta.1`，API 稳定
- [ ] GA 发布（Week 34）：`0.2.0`，全量发布

---

## 风险矩阵

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|---------|
| Rust 闭包 Cell/RefCell 包装导致运行时 panic | 中 | 高 | Phase 2 末尾专项测试（Rust 闭包 16+ 用例），analyze 规则 E0306 警告逃逸 |
| match 穷尽性检查实现复杂度过高 | 中 | 中 | Phase 3 中优先穷尽性检查，推迟"死 arm 检测"到 v0.3 |
| 模块循环依赖不够准确的误报/漏报 | 低 | 中 | Phase 5 使用标准拓扑排序 + Tarjan SCC 算法 |
| TypeScript 模式下 JS 类型信息与源语言不一致 | 中 | 低 | Phase 1/2 每新增类型特性时同步测试 TS 模式输出 |
| Rust 类继承多态（`&dyn Trait`）性能问题 | 高 | 低 | 已标注实验性 + 文档诚实标注边界，用户自行评估 |
| 34 周工期不可控 | 中 | 高 | Alpha 发布（Week 20-22）作为中期里程碑，Phase 6-9 范围可根据反馈调整 |
| v0.1 兼容性破坏过多 | 低 | 高 | Phase 0 建立 v0.1 兼容性测试套件（111 条），Phase 1-6 每次合并前运行 |
| LSP 协议实现工作量大 | 中 | 低 | 使用 `lsprotocol` 库减少协议层开发，优先实现 P0 功能（诊断 + hover） |

---

## 依赖关系图

```
Phase 0 (基础设施)
    │
    ▼
Phase 1 (核心语法扩展) ─────────────────────┐
    │                                        │
    ▼                                        │
Phase 2 (函数增强)                            │
    │                                        │
    ├──────────► Phase 2.5 (Analyze 核心) ◄──┘
    │                 │
    ▼                 ▼
Phase 3 (高级类型) ──► Analyze 规则同步
    │
    ▼
Phase 4 (异步) ──────► Analyze 规则同步
    │
    ▼
Phase 5 (模块系统) ──► Analyze 规则同步
    │                       │
    ▼                       ▼
Phase 6 (类/继承 🧪)    Phase 7 (Analyze 完善)
    │                       │
    ▼                       ▼
Phase 8 (IDE 集成) ◄────────┘
    │
    ▼
Phase 9 (Alpha/测试/稳定)
```

**并行化机会：**
- Phase 2.5 + Phase 3 的类型检查部分可并行（不同团队/模块）
- Phase 4 (异步) 和 Phase 5 (模块) 的代码生成部分独立，可并行
- Phase 7 (Analyze 完善) 可与 Phase 6 (类) 和 Phase 8 (IDE) 并行

---

## 每日/每周检查清单

### 每 Phase 开始前

- [ ] 阅读设计方案对应章节
- [ ] 确认该 Phase 的依赖已就绪（见依赖图）
- [ ] 编写该 Phase 的 Analyze 规则骨架

### 每 Phase 进行中

- [ ] 每天至少运行一次现有测试套件
- [ ] 新代码添加类型标注（Python typing）
- [ ] 每个新 AST 节点/函数添加 docstring

### 每 Phase 结束时

- [ ] 所有新测试通过
- [ ] v0.1 兼容性测试（111 条）无回归
- [ ] 代码格式化（`black` + `isort`）
- [ ] 更新 CHANGELOG.md
- [ ] 评审该 Phase 产生的技术债务

---

## 开放问题与决策记录

以下问题在计划制定时被识别，每个都有明确的决策和理由：

| # | 问题 | 决策 | 理由 |
|---|------|------|------|
| 1 | lark LALR 能否处理 v0.2 完整 EBNF？ | Phase 1.1 **第一天**用 `earley` 模式加载完整 EBNF 验证；如果 LALR 有冲突，立即切换到 Earley | 模板字符串的 `` ` `` 和 `${` 嵌套是最可能出冲突的地方；Earley 解析 1000 行文件应 < 200ms，性能损失可接受 |
| 2 | 新增 25+ AST 节点后 frozen dataclass 内存？ | 暂不做 premature optimization；Phase 3 前做一次 AST 内存基准测试；如类型收窄需大量节点替换，将 `frozen` 改为 `False` 或使用 `copy.replace()` | v0.1 的 frozen 设计在 20 节点时无问题，45 节点时需验证 |
| 3 | `T?` 语法糖展开时机？ | **Parser 阶段展开**：`T?` → `T \| null` | AST 中只存在 `UnionType`，类型检查器无需处理 `OptionalType` 特殊节点；诊断消息中措辞区分"可选类型"和"联合类型" |
| 4 | TypeScript 模式生成 `.ts` 还是 `.js` + `.d.ts`？ | 生成 **`.ts`**（含完整类型标注） | 用户可直接用 `tsc` 编译；`.d.ts` 手动维护成本高，推迟到 v0.3 作为"库发布模式" |
| 5 | Rust 后端 `use` 语句去重？ | 在 `base.py` 中增加 `ImportCollector` 工具类，所有后端共享 | 按 `(module, item)` 去重；Phase 5 模块系统实现时自然需要 |
| 6 | LSP 与 CLI 共享代码边界？ | LSP 通过 Python API 调用核心逻辑（**不通过子进程**） | 暴露 `omnilang.compile()` / `omnilang.check()` / `omnilang.fix()` 作为公共函数；lark grammar 缓存到模块级变量避免重复加载 |
| 7 | `omnic test` 是内置还是集成现有工具？ | **v0.2 不做** `omnic test`，推迟到 v0.3 | 跨语言测试运行器工作量巨大（需管理 4 语言运行时），v0.2 仍通过 pytest 管理测试，`omnic run` 用于单文件快速执行 |

---

## 启动检查清单（"三个第一天"原则）

| 时间点 | 动作 | 目的 |
|--------|------|------|
| **Phase 1.1 第一天** | 用 lark Earley 模式加载完整 v0.2 EBNF（含模板字符串嵌套），确认无解析冲突 | 在语法层投入 4 周前，先验证解析可行性 |
| **Phase 2 第一天** | 写 Rust 闭包捕获场景矩阵的**第一个测试用例**（`closures/02_counter_cell.omni`），先写测试再写代码 | 闭包是 Rust 后端最容易出 bug 的地方，TDD 最有效 |
| **Phase 2.5 第一天** | 实现 **P0101 规则**并跑通整个诊断管道（解析→AST→规则检查→终端渲染→JSON 输出） | 验证 Analyze 架构可行，后续 50+ 条规则只需填充检查逻辑 |

---

## 附录：v0.1 基线关键文件清单

以下 v0.1 文件在 v0.2 中**需要修改**（非新增）：

| 文件 | 修改幅度 | 说明 |
|------|---------|------|
| `omnilang/ast_nodes.py` | **大改** | 20+ → 45+ 节点类型 |
| `omnilang/grammar/omnilang.lark` | **大改** | 47 → ~120 规则 |
| `omnilang/parser.py` | **大改** | 所有新节点 transformer |
| `omnilang/semantic/resolver.py` | **中改** | 模块级/泛型参数作用域 |
| `omnilang/semantic/type_checker/` | **重写为子模块** | 拆分为 base/structural/generics/union/narrowing 六个文件 |
| `omnilang/codegen/rust.py` | **大改** | async/tokio, 闭包/Cell, trait, enum |
| `omnilang/codegen/dart.py` | **中改** | async, sealed, extension, 泛型 |
| `omnilang/codegen/javascript.py` | **中改** | async, ESM, class, import type |
| `omnilang/codegen/python.py` | **中改** | async/asyncio, dataclass, match |
| `omnilang/codegen/base.py` | **小改** | 通用工具函数添加 |
| `omnilang/cli.py` | **中改** | check/fix/run/fmt/lsp 子命令 |
| `omnilang/utils.py` | **小改** | Span/Diagnostic 扩展 |
| `tests/conftest.py` | **中改** | 新 fixtures |
| `pyproject.toml` | **小改** | 新依赖 |

---

**本执行计划与 `OmniLang_v0.2_设计方案.md`（v1.3-final）严格对齐，涵盖从基础设施到 GA 发布的完整工程链路。**
