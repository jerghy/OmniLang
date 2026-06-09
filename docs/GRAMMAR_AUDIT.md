# Grammar 兼容性审计

> **Phase A0 产出** | **日期：** 2026-06-09  
> **结论：** ✅ 合并可行。3 个差异需要处理，0 个阻塞问题。

---

## 1. 表达式类型覆盖对比

### OmniLang codegen（isinstance dispatch, 18 类型）

`omnilang/codegen/javascript.py` `_gen_expr()`:

| 类型 | 用途 |
|------|------|
| Literal | 字面量（整数/浮点/字符串/布尔） |
| VarRef | 变量引用 |
| BinaryOp | 二元运算（算术/比较/逻辑） |
| UnaryOp | 一元运算（- / ! / await） |
| FuncCall | 函数调用 |
| MethodCall | 方法调用（obj.method） |
| IndexGet | 索引访问（arr[i]） |
| ArrayLiteral | 数组字面量 |
| MapLiteral | Map 字面量 |
| IfExpr | 条件表达式 |
| TemplateString | 模板字符串 |
| NullCoalesce | 空值合并（??） |
| OptionalChain | 可选链（?.） |
| SpreadExpr | 展开语法（...） |
| StructLiteral | 结构体字面量 |
| ClosureExpr | 闭包/箭头函数 |
| MatchExpr | 模式匹配 |
| AwaitExpr | await 表达式 |

### OmniView codegen（dict-key dispatch, 11 类型）

`omniview/codegen/react.py` `_expr_to_js()`:

| 类型 | 用途 |
|------|------|
| LiteralExpr | 字面量 |
| IdentifierExpr | 变量/标识符引用 |
| BinaryOpExpr | 二元运算 |
| UnaryOpExpr | 一元运算 |
| FieldAccessExpr | 字段访问（obj.field） |
| StoreFieldAccessExpr | Store 字段访问（store.getState().field()） |
| ArrayExpr | 数组字面量 |
| SpreadExpr | 展开语法 |
| TernaryIR | 三元条件 |
| FormatCall | 格式化调用（format_date/number） |
| TranslateCall | 翻译调用（t("key")） |

### 差异分析

| 类别 | 类型 | 处理方式 |
|------|------|---------|
| **重叠（不同名称）** | Literal↔LiteralExpr, VarRef↔IdentifierExpr, BinaryOp↔BinaryOpExpr, UnaryOp↔UnaryOpExpr, ArrayLiteral↔ArrayExpr, SpreadExpr↔SpreadExpr | 两条管线各自处理——不合并 |
| **OmniLang 独有** | FuncCall, MethodCall, IndexGet, MapLiteral, IfExpr, TemplateString, NullCoalesce, OptionalChain, ClosureExpr, StructLiteral, MatchExpr, AwaitExpr (12 类型) | OmniView lowering 将这些类型转换为 IR dict 或 IdentifierExpr——codegen 不直接处理 |
| **OmniView 独有** | StoreFieldAccessExpr, FormatCall, TranslateCall, TernaryIR, FieldAccessExpr (5 类型) | 这些都是 UI 特有的语义类型——在 OmniView IR 层定义，不应迁移到 OmniLang AST |

**结论：两条管线机制不同（isinstance vs dict-key），服务不同消费者（纯逻辑 vs UI 模板）。不应合并，也不需要在合并项目中统一。** ✅

---

## 2. Parser Mode 兼容性

| Grammar 文件 | 行数 | Parser Mode |
|-------------|------|-------------|
| `omnilang/grammar/omnilang.lark` (v0.1) | 121 | LALR |
| `omnilang/grammar/omnilang_v0.2.lark` | 248 | Earley（LALR 不兼容——CNAME 歧义） |
| `omniview/grammar.lark` | 237 | Earley（LALR 不兼容——@ 指令和 CNAME 歧义） |

**决策：统一 grammar 使用 Earley 模式。** v0.1 LALR 文件已不再使用，直接删除。

**风险：** Earley 比 LALR 慢约 3-5x，但对 ~500 行 `.omni` 文件仍在 <100ms 内。当前 182 tests 解析时间 <1s。✅

---

## 3. AST 节点对齐

### FieldAccess

- **当前状态：** 定义在 `omniview/ast_nodes.py`。用于 postfix_expr 的 DOT CNAME 语法。`lowering.py` 通过 `isinstance(expr, FieldAccess)` 使用。
- **OmniLang 状态：** 不存在独立 `FieldAccess` 类型。OmniLang 将所有 DOT 访问处理为 `MethodCall(args=[])`（v0.2 grammar 已将 method_call 的括号改为可选）。
- **迁移方案：** 保留 `FieldAccess` 在 `omniview/ast_nodes.py` 中，不迁移到 `omnilang/ast_nodes`。**理由：** OmniLang 的 `MethodCall(args=[])` 语义等同于 FieldAccess，且 lowering.py 已经能处理 MethodCall。迁移 FieldAccess 到 OmniLang 会造成概念重复。

**修正（替代之前 ADR 中的迁移方案）：** 不在 omnilang.ast_nodes 中新增 FieldAccess。改为更新 lowering.py，将所有 `isinstance(expr, FieldAccess)` 替换为 OmniLang 的 `MethodCall` 处理（已在 lowering.py 382-401 行实现）。

### TernaryExpr

- **当前状态：** 定义在 `omniview/ast_nodes.py`。用于 C 风格三元 `a ? b : c`。
- **OmniLang 状态：** OmniLang 不支持 `? :` 语法。使用 `if/else` 表达式。AST 类型为 `IfExpr`。
- **迁移方案：** 保留 `TernaryExpr` 在 `omniview/ast_nodes.py` 中，不迁移。**理由：** `? :` 语法是 OmniView 特有的（来自 grammar.lark 121 行），不在 OmniLang 语法中。如果统一语法保留 `? :` 规则，TernaryExpr 留在 omniview 侧是正确归属。如果统一语法移除 `? :` 改用 OmniLang 的 `if/else`，则 TernaryExpr 可以删除。

**决策：** 统一 grammar 保留 `? :` 语法（因为 4 个 examples 使用它）。TernaryExpr 保留在 omniview.ast_nodes。

---

## 4. 受影响的 Import 路径

合并后以下 import 需要更新（14 个文件）：

| # | 文件 | 旧 import | 新 import |
|---|------|----------|----------|
| 1 | `omniview/tests/test_parser.py` | `from omniview.parser import OmniViewTransformer` | `from omnilang.parser import OmniTransformer` |
| 2 | `omniview/tests/test_codegen.py` | 同上 | 同上 |
| 3 | `omniview/tests/test_codegen_vue.py` | 同上 | 同上 |
| 4 | `omniview/tests/test_codegen_dioxus.py` | 同上 | 同上 |
| 5 | `omniview/tests/test_codegen_flutter.py` | 同上 | 同上 |
| 6 | `omniview/tests/test_semantic.py` | 同上 | 同上 |
| 7 | `omniview/tests/test_behavior_fixtures.py` | 同上 | 同上 |
| 8 | `omniview/tests/test_preprocessor.py` | `from omniview.preprocessor import ...` | 不变（preprocessor 保留在 omniview） |
| 9 | `omniview/codegen/react.py` | `from omniview.codegen.base import CodegenBase` | `from omnilang.codegen.base import CodegenBase` |
| 10 | `omniview/codegen/vue.py` | 同上 | 同上 |
| 11 | `omniview/codegen/dioxus.py` | 同上 | 同上 |
| 12 | `omniview/codegen/flutter.py` | 同上 | 同上 |
| 13 | `omniview/lowering.py` | `from omniview.ast_nodes import ..., FieldAccess, TernaryExpr` | 不变（FieldAccess/TernaryExpr 保留在 omniview.ast_nodes） |
| 14 | `omniview/cli.py` | `from omniview.parser import OmniViewTransformer` | `from omnilang.parser import OmniTransformer` |

**更新：** test_preprocessor.py 和 lowering.py 的 import 不需要改变。实际受影响：**12 个文件**（不是 14 个）。

---

## 5. 合并可行性结论

| 检查项 | 结果 |
|--------|------|
| 表达式类型冲突 | ✅ 无——两条管线机制不同，不合并 |
| Parser mode 兼容 | ✅ 统一使用 Earley |
| FieldAccess 迁移 | ✅ 保留在 omniview.ast_nodes（OmniLang MethodCall 已替代） |
| TernaryExpr 迁移 | ✅ 保留在 omniview.ast_nodes（`? :` 语法是 OmniView 特有） |
| Import 路径更新 | ⚠️ 12 个文件需要更新（不是 14——2 个不需要改） |
| v0.1 grammar 遗留 | ✅ 直接删除 |

**总体结论：✅ 合并可行。0 个阻塞问题。3 个差异已记录处理方案。**
