# ADR-001: 编译器管线架构

## Status

Accepted

## Context

OmniLang 是一个将 `.omni` 源码编译到 10 个目标 UI 框架（后缩减为 9，见 [[ADR-003]]）的 DSL 编译器。编译管线需要：
- 解析类型化 DSL 语法（view、store、signal、motion、design blocks）
- 执行语义分析（符号解析、类型检查、lint）
- 降低为规范化的中间表示（IR）
- 生成多目标代码（React、Flutter、OmniView、Vue 等）
- 支持增量编译（编辑器 WASM 场景）
- 提供可序列化的诊断信息（跨 FFI/WASM/IPC 边界）

## Decision Drivers

1. **多目标复用**：解析和语义分析逻辑必须在所有 9 个代码生成目标间共享
2. **编辑器集成**：Poiesis 编辑器需要通过 WASM 进行浏览器内编译
3. **可测试性**：每个阶段必须可独立测试，回归测试可自动快照
4. **可演进性**：语言特性变更不应破坏所有目标
5. **错误质量**：诊断信息必须包含准确的源代码位置

## Decision

采用**六阶段分离编译管线**：

```
Lexer → Parser → Semantic Analysis → IR Lowering → Codegen → Output
  │        │           │                  │            │
  │        │           │                  │            └── 4 种输出格式
  │        │           │                  └── 规范化 IR (IRProgram)
  │        │           └── 符号解析 + 类型检查 + Lint
  │        └── 递归下降解析器 + 错误恢复
  └── 手动编写 tokenizer (58 TokenKind)
```

**核心设计决策**：

1. **递归下降解析器**——非 parser generator。手动编码，带 `skip_to`/`at_stmt_restart` 错误恢复
2. **两遍语义分析**——第一遍收集声明（`SymbolCollector`），第二遍解析引用（`Resolver`）
3. **Before-IR 后 After-IR Lint**——Lint 规则在 IR 降低前后均可注册
4. **规范化 IR（Intent IR）**——名-值对结构，支持 `ComponentIR`、`StoreIR`、`ElementIR`、`DesignIR`、`FunctionIR`
5. **IR 从 AST 降低**——`ir/lower/` 模块执行 AST→IR 转换
6. **Codegen trait + FormatExpr 家族**——`Codegen` trait 提供 `generate_per_module`/`generate_per_component` 策略；`FormatExpr` trait 消除跨目标表达式格式化重复
7. **特性门控后端**——`#[cfg(feature = "codegen-react")]` 等，默认全部关闭
8. **增量缓存**——`IncrementalCache` 基于内容哈希 + 导入依赖图传递失效

**Architectural Components Affected**:
- `omnilang/src/lexer/` — Tokenizer（58 TokenKind）
- `omnilang/src/parser/` — 递归下降：`ast/`、`expr/`、`top_level/`、`view/`
- `omnilang/src/semantic/` — 符号表 + `lints/`
- `omnilang/src/ir/` — IR 类型（`types.rs`、`logic.rs`、`design.rs`）+ `lower/`
- `omnilang/src/codegen/` — 9 目标 + `shared/`
- `omnilang/src/analyse.rs` — 多文件并行分析入口
- `omnilang/src/error.rs` — `Diagnostic`/`DiagnosticKind`/`Severity`（thiserror）

## Consequences

### Positive

- **阶段隔离**：每个阶段产生类型化中间产出物，防止无效数据向下游传播
- **多目标效率**：新增目标仅需实现 `Codegen` trait + `FormatExpr`，无需触碰解析器
- **编辑器友好**：增量缓存 + WASM 导出支持按键级重编译
- **高质量错误**：错误恢复允许一次报告多个诊断（不是 fail-fast）
- **可测试**：insta 快照测试每个阶段输出，proptest 验证不 panic

### Negative

- **IR 类型级联**：添加 `ExprIR` 变体需触碰 6+ 后端 + 7 语义文件（已知问题，`#[non_exhaustive]` 迁移中）
- **TypeKind 过于简单**：`Named { name: String }` 存储类型为不透明字符串，无递归验证（见 [[ADR-003]] Sprint 4）
- **源映射管线未完全连接**：`IRProgram.source_map` 当前为空，仅 OmniView 后端构建源映射（修正安排在 Sprint 2-3）
- **Codegen trait 采纳不完整**：`UiCodegen` trait 存在但代码未移入共享方法；OmniView 绕过标准 `compile()` 路径

### Neutral

- **OmniView 返回特殊输出类型**（`OmniViewOutput` 11 个额外字段）——这是务实选择，但统一到 `CompileOutput` 列入计划（Sprint 5）

## Implementation Strategy

### Blast Radius

**影响范围**：修改 IR 类型影响 6+ 代码生成后端 + 7 语义文件。修改解析器语法影响 TS 编辑器解析器（见 [[ADR-002]]）。
**缓解**：`#[non_exhaustive]` 枚举 + insta 快照 + 参数化测试框架（计划中）。

### Reversibility

**可逆性级别**：High（管线阶段可独立修改，不破坏下游）

各阶段有明确的输入/输出契约。修改降低逻辑不影响解析；修改语义分析不影响 lexer。

### Sequencing & Timing

**系统就绪**：已生产就绪。待修复：源映射管线（Sprint 2-3）、TypeKind 递归类型（Sprint 4）、CompileOutput 统一（Sprint 5）。

## Alternatives Considered

### 方案 A：Parser Generator（LALRPOP / Pest）

**否决原因**：错误恢复质量、自定义错误消息、IDE 集成（增量重解析）不如手写递归下降。

### 方案 B：单遍编译（无 IR）

**否决原因**：无法支持 9 个代码生成目标——每个目标需要独立的语义→语法转换。

### 方案 C：AST→Codegen 直接（无 IR 中间层）

**否决原因**：AST 保留了语言特有结构（语法糖、设计块展开），IR 规范化表示对多目标代码生成至关重要。

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Necessity Assessment**: 8/10
- 当前需求：多目标编译要求阶段分离；已有 3 个活跃后端
- 未来需求：规划中的新目标（Swift？）将从 IR 降低中受益
- 等待成本：重构缺少 IR 的编译器将非常昂贵，结构已经产生价值

**Complexity Assessment**: 6/10
- 增加的复杂度：6 阶段 vs 可能的 4 阶段（解析→语义→代码生成）
- 维护负担：IR 类型级联是已知痛点；源映射复杂性被推迟
- 学习曲线：新贡献者必须理解 AST→IR→Codegen 转换
- 依赖：IR 层引入序列化/反序列化开销

**Alternative Analysis**: 方案 B/C 已考虑并否决。设计是务实的最小值——不包含优化传递（计划中但推迟了 P39）。

**Pragmatic Score**:
- 必要性: 8/10
- 复杂度: 6/10
- 比率: 0.75 ✅（在 balanced 目标 <1.5 之内）

**Recommendation**: ✅ 批准设计——管线架构对多目标编译器是适当的工程选型。复杂性来自问题领域本身，而非过度设计。待处理简化：统一 CompileOutput、连接源映射、完成 UiCodegen trait 代码迁移。

## Validation

**Acceptance Criteria**:
- [x] 所有 9 个目标通过 insta 快照测试
- [x] proptest `compile_never_panics` 通过（1000 随机输入）
- [x] 增量缓存正确性：文件哈希变更触发传递失效
- [x] 多文件并行分析（rayon）产生确定性输出
- [ ] IRProgram.source_map 非空（Sprint 2-3）
- [ ] 所有 Codegen trait 方法迁移到 UiCodegen 共享实现（Sprint 5）

## References

- [architecture-review-2026-07-26](../reviews/full-workspace-review-2026-07-26.md) — 全工作区评审
- [[omnilang-codegen]] — Codegen 模式与陷阱
- [[omniview-compiler-internals]] — OmniView 编译器 21 模块
- [[ADR-002]] — 编辑器范式演进
- [[ADR-003]] — 代码生成目标优先级
