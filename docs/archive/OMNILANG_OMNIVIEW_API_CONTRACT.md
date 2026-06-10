# OmniLang ↔ OmniView API Contract

> **版本:** 1.0 (2026-06-05)  
> **状态:** 生效 — Phase 0.0 委托基础设施编码依据  
> **依赖关系:** OmniView → OmniLang 单向依赖（Dart : Flutter ≈ OmniLang : OmniView）

---

## 1. 合约概述

OmniView 是 OmniLang 上的 UI 领域 DSL。OmniView 不重复实现 OmniLang 已有的能力——表达式解析、类型检查、语义分析全部委托给 OmniLang。

本合约定义了 OmniView 调用 OmniLang 的所有接口、共享的数据类型、以及调用约定。

---

## 2. 解析接口

### 2.1 `parse_inline_expr(expr_text: str) -> Expr`

**用途：** 解析单个 OmniLang 表达式（用于 `{expr}` 文本插值、属性值、`@if` 条件、`@for` 迭代表达式等）。

**调用位置：** 预处理器提取 `{...}` 表达式后，由 `omnilang_bridge.py` 调用。

**输入：** 去除外层 `{` `}` 的表达式文本

**输出：** `omnilang.ast_nodes.Expr` 子类（`Literal | VarRef | BinaryOp | UnaryOp | FuncCall | MethodCall | FieldAccess | ArrayLiteral | MapLiteral | IfExpr | TemplateString | SpreadExpr | OptionalChain | NullCoalesce | ClosureExpr | AwaitExpr | StructLiteral | MatchExpr`）

**现有实现：** `omnilang.parser_helpers.parse_inline_expr` — 已验证可用。

**错误处理：** 解析失败时回退为 `Literal(value=expr_text, kind="String")`。OmniView 应检查返回类型并发出诊断（如 `E0300: invalid expression`）。

### 2.2 `parse_inline_block(block_text: str) -> Block`

**用途：** 解析语句块（用于 fn body、action body、lifecycle hook body）。

**调用位置：** 预处理器提取 `{...}` 语句块后，由 `omnilang_bridge.py` 调用。

**输入：** 去除外层 `{` `}` 的语句文本

**输出：** `omnilang.ast_nodes.Block` 包含 `stmts: list[Stmt]`

**需新建：** 当前 OmniLang 没有此函数。需在 `omnilang.parser_helpers.py` 中添加，实现方式与 `parse_inline_expr` 类似：

```python
def parse_inline_block(block_text: str) -> Block:
    """Parse inline statement block from OmniView delegation."""
    from omnilang.lexer import create_parser
    from omnilang.parser import OmniTransformer
    parser = create_parser()
    source = f"fn __tpl() {{ {block_text} }}"
    tree = parser.parse(source)
    transformer = OmniTransformer()
    prog = transformer.transform(tree)
    if prog.functions and prog.functions[0].body:
        return prog.functions[0].body
    return Block(stmts=[])
```

### 2.3 `parse(source: str, filename: str = "<input>") -> Program`

**用途：** 解析完整的 OmniLang 源文件（保留用于未来 `.omnilang` 独立文件）。

**输出：** `omnilang.ast_nodes.Program`

**现有实现：** `omnilang.parser.parse`

---

## 3. 共享数据类型

### 3.1 表达式类型（OmniView 从 OmniLang 导入）

所有表达式 AST 节点从 `omnilang.ast_nodes` 导入：

| 类型 | OmniLang 模块 | 用途 |
|------|-------------|------|
| `Expr` | `omnilang.ast_nodes` | 所有表达式的基类 |
| `Literal` | `omnilang.ast_nodes` | 字面量（整数/浮点/字符串/布尔） |
| `VarRef` | `omnilang.ast_nodes` | 变量引用 |
| `BinaryOp` | `omnilang.ast_nodes` | 二元运算（算术/比较/逻辑） |
| `UnaryOp` | `omnilang.ast_nodes` | 一元运算（`-` / `!` / `await`） |
| `FuncCall` | `omnilang.ast_nodes` | 函数调用 `fn(args)` |
| `MethodCall` | `omnilang.ast_nodes` | 方法调用 `obj.method(args)` |
| `FieldAccess` | `omnilang.ast_nodes` | ⚠️ OmniLang 无独立 FieldAccess —— 字段访问在 postfix_expr 中通过 MethodCall(args=[]) 表示 |
| `ArrayLiteral` | `omnilang.ast_nodes` | 数组字面量 `[a, b]` |
| `MapLiteral` | `omnilang.ast_nodes` | Map 字面量 `{k: v}` |
| `IfExpr` | `omnilang.ast_nodes` | if 表达式 |
| `TemplateString` | `omnilang.ast_nodes` | 模板字符串 |
| `SpreadExpr` | `omnilang.ast_nodes` | 展开语法 `...expr` |
| `OptionalChain` | `omnilang.ast_nodes` | 可选链 `?.` |
| `NullCoalesce` | `omnilang.ast_nodes` | 空值合并 `??` |
| `ClosureExpr` | `omnilang.ast_nodes` | 闭包/箭头函数 |
| `AwaitExpr` | `omnilang.ast_nodes` | await 表达式 |
| `StructLiteral` | `omnilang.ast_nodes` | 结构体字面量 |
| `MatchExpr` | `omnilang.ast_nodes` | 模式匹配 |

### 3.2 语句类型

| 类型 | OmniLang 模块 | 用途 |
|------|-------------|------|
| `Stmt` | `omnilang.ast_nodes` | 所有语句的基类 |
| `Block` | `omnilang.ast_nodes` | 语句块 |
| `VarDecl` | `omnilang.ast_nodes` | 变量声明 |
| `Destructure` | `omnilang.ast_nodes` | 解构赋值 |
| `Assign` | `omnilang.ast_nodes` | 赋值语句 |
| `IfStmt` | `omnilang.ast_nodes` | if 语句 |
| `WhileStmt` | `omnilang.ast_nodes` | while 循环 |
| `ForInStmt` | `omnilang.ast_nodes` | for-in 循环 |
| `BreakStmt` / `ContinueStmt` | `omnilang.ast_nodes` | 循环控制 |
| `ReturnStmt` | `omnilang.ast_nodes` | return 语句 |
| `PrintStmt` | `omnilang.ast_nodes` | print/println |
| `ExprStmt` | `omnilang.ast_nodes` | 表达式语句 |

### 3.3 类型系统

| 类型 | OmniLang 模块 | 用途 |
|------|-------------|------|
| `TypeAnnotation` | `omnilang.ast_nodes` | 类型标注 |
| `UnionType` | `omnilang.ast_nodes` | 联合类型 |
| `GenericParam` | `omnilang.ast_nodes` | 泛型参数 |

### 3.4 工具类型

| 类型 | OmniLang 模块 | 用途 |
|------|-------------|------|
| `Span` | `omnilang.utils` | 源码位置信息 |

---

## 4. OmniView 自有类型（不从 OmniLang 导入）

OmniView 保留以下自己的 AST 节点（因为它们描述的是 UI 结构，不是语言级概念）：

| 类型 | 模块 | 用途 |
|------|------|------|
| `Program` | `omniview.ast_nodes` | OmniView 程序的顶层容器 |
| `ComponentDef` | `omniview.ast_nodes` | 组件定义 |
| `StoreDef` | `omniview.ast_nodes` | Store 定义 |
| `ThemeDef` | `omniview.ast_nodes` | 主题定义 |
| `ViewBlock` | `omniview.ast_nodes` | 视图块 |
| `Element` | `omniview.ast_nodes` | UI 元素 |
| `*Directive` | `omniview.ast_nodes` | @if/@for/@slot/@native/@fallback |
| `*Decl` | `omniview.ast_nodes` | prop/signal/computed/event/ref/store_ref 声明 |
| `StyleBlock` / `StyleRule` | `omniview.ast_nodes` | 样式定义 |
| `LifecycleHook` | `omniview.ast_nodes` | 生命周期钩子 |
| `TextNode` / `Interpolation` | `omniview.ast_nodes` | 文本/插值 |
| `*Prop` | `omniview.ast_nodes` | 元素属性（static/bind/event/class/style） |
| `FnDef` / `Param` | `omniview.ast_nodes` | 函数定义（body 委托给 OmniLang） |

---

## 5. 语义分析接口

### 5.1 符号解析

OmniView 构建符号表后，委托给 OmniLang 进行表达式级符号解析：

```python
# OmniView 构建 scope
scope = CrossLanguageScope(
    variables={"count": TypeAnnotation(name="i32"), ...},
    functions={"handle_add": FuncSignature(params=[], return_type=...)},
    store_fields={"store.todos": TypeAnnotation(name="Array", type_args=[...])},
)

# OmniView 调用 OmniLang 检查表达式
diagnostics = omnilang.semantic.check_expression(expr, scope)
```

### 5.2 类型检查

```python
# OmniView 调用 OmniLang 推断表达式类型
result_type = omnilang.semantic.infer_type(expr, scope) -> TypeAnnotation
```

### 5.3 依赖收集

```python
# OmniView 调用 OmniLang 收集表达式中的信号引用
deps = omnilang.semantic.collect_dependencies(expr, scope) -> list[str]
```

---

## 6. 诊断接口

### 6.1 诊断类型

OmniView 复用 OmniLang 的诊断框架：

| 类型 | OmniLang 模块 | 用途 |
|------|-------------|------|
| `Diagnostic` | `omnilang.analyze.diagnostic` | 诊断信息 |
| `DiagnosticLevel` | `omnilang.analyze.diagnostic` | ERROR/WARNING/INFO/SUGGESTION |
| `DiagnosticCode` | `omnilang.analyze.diagnostic` | 诊断规则代码 |

### 6.2 错误位置映射

OmniLang 诊断中的位置信息是相对于嵌入表达式的。OmniView 负责映射回 `.omni` 源文件位置：

```python
def map_error_position(
    omni_diagnostic: Diagnostic,
    placeholder_offset: int,  # 嵌入点在 .omni 中的起始偏移
) -> Diagnostic:
    """将 OmniLang 诊断位置映射到 .omni 源位置。"""
```

---

## 7. 预处理器接口

### 7.1 占位符格式

```
__OMNI_EXPR_{id}__  — 表达式占位符（{expr} 文本插值、属性值等）
__OMNI_BLOCK_{id}__ — 语句块占位符（fn body、action body、lifecycle body）
```

### 7.2 预处理器输出

```python
@dataclass
class PreprocessorOutput:
    processed_source: str        # 占位符替换后的源文本
    expressions: dict[str, str]  # placeholder_id → 原始表达式文本
    blocks: dict[str, str]       # placeholder_id → 原始语句块文本
    position_map: dict[str, int] # placeholder_id → .omni 偏移量
```

---

## 8. Codegen 接口

### 8.1 表达式 Codegen

表达式翻译由 `omnilang/codegen/{target}.py` 完成：

| 目标 | OmniLang codegen 模块 | 状态 |
|------|---------------------|------|
| TypeScript/JavaScript | `omnilang/codegen/typescript.py` | 待创建 |
| Dart | `omnilang/codegen/dart.py` | 已有基础 |
| Rust | `omnilang/codegen/rust.py` | 待创建 |

OmniView 的 codegen 负责：
1. 调用 OmniLang 的表达式 codegen 翻译表达式
2. 将翻译结果嵌入到 UI 模板中

```python
# OmniView codegen 调用 OmniLang codegen
js_expr = omnilang.codegen.javascript.translate_expr(expr_ir)
# 然后嵌入到 JSX 中
code = f"<div>{js_expr}</div>"
```

---

## 9. 调用流程图

```
.omni source
    │
    ├── Preprocessor (omniview/preprocessor.py)
    │   ├── 扫描 {  } 区域，平衡括号
    │   ├── 替换表达式为 __OMNI_EXPR_0__
    │   ├── 替换语句块为 __OMNI_BLOCK_0__
    │   └── 输出: PreprocessorOutput
    │
    ├── Lark Parser (omniview/grammar.lark — 仅 UI 结构)
    │   └── 输出: Lark parse tree
    │
    ├── OmniViewTransformer (omniview/parser.py)
    │   ├── 解析 UI 结构 → OmniView AST
    │   ├── 遇到占位符时调用 omnilang_bridge:
    │   │   ├── parse_inline_expr() for __OMNI_EXPR_*
    │   │   └── parse_inline_block() for __OMNI_BLOCK_*
    │   └── 输出: OmniView AST（表达式节点来自 OmniLang）
    │
    ├── Semantic Analyzer (omniview/semantic/analyzer.py)
    │   ├── 构建 CrossLanguageScope
    │   ├── 调用 omnilang.semantic 检查类型/引用
    │   └── 输出: AnalysisResult + diagnostics
    │
    ├── Lowering (omniview/lowering.py)
    │   └── OmniView AST + OmniLang AST → Intent IR
    │
    └── Codegen (omniview/codegen/{target}.py)
        ├── 调用 omnilang/codegen/{target}.py 翻译表达式
        └── 嵌入 UI 模板
```

---

## 10. 版本兼容性

| 合约版本 | OmniLang 版本 | OmniView 版本 | 变更 |
|---------|-------------|-------------|------|
| 1.0 | v0.2 (grammar_v0.2.lark) | v0.3 Phase 0 | 初始合约 |

OmniLang AST 变更时，通过 `omniview/ir_types.py` 中的 `IRVersion` 管理兼容性。OmniView 的 `lowering.py` 负责适配层转换。
