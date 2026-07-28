# ADR-012: OmniView Codegen as Debug Tool — Independent Implementation

## Status

Accepted

## Context

OmniView 是 OmniLang 的三个 UI codegen 后端之一（另外两个是 React 和 Vue）。经过 P35 去重和 v5 修复，React 和 Vue 后端大量采用了共享的 `FormatExpr`、`FormatStmt`、`FormatType` traits。OmniView 后端仍然使用自己的 `expr_to_js()` (419 行) 和 `emit_statement()` (337 行)，未采纳这些共享 traits。

2026-07-28 会话的深度探索提出了一个去重机会（D1）：让 OmniView 也实现 `FormatExpr`/`FormatStmt`，消除约 750 行重复代码。

## Decision Drivers

1. **OmniView 的使用场景**: 生成自包含的 JS/HTML/CSS，内联信号运行时——用于开发调试，不是生产部署目标
2. **状态式 vs 声明式**: OmniView 使用 `OmniViewGenerator` 结构体，通过 `self.js.push_str()` 直接构建输出——与 React/Vue 的 `CodeWriter` + trait 模式根本不同
3. **统一成本**: 将 OmniView 迁移到 traits 需要重构其信号名解析（`name_renames`、`global_renames`）和延迟 JS 发出模式，风险高于收益
4. **生产后端的共享已充分**: React、Vue、JS、TS、Python、Rust、Dart、Dioxus 共 8 个后端已从 FormatExpr/FormatStmt 受益

## Decision

**OmniView codegen 保持独立实现，不强制采用 `FormatExpr`/`FormatStmt` traits。其作为调试工具的角色决定了它需要不同于生产后端的代码生成模式。**

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/expr.rs` — 保持 `expr_to_js()` 独立实现
- `omnilang/src/codegen/omniview/stmt.rs` — 保持 `emit_statement()` 独立实现
- `crates/omnilang-core/src/codegen/shared/expr.rs` — FormatExpr trait（不扩展到 OmniView）
- `crates/omnilang-core/src/codegen/shared/stmt.rs` — FormatStmt trait（不扩展到 OmniView）

**Interface Changes:** 无

## Consequences

### Positive

* **模式匹配**: OmniView 的状态式生成器模式（`self.js`、`self.css`、`self.mount_js` 缓冲区）匹配其"一次生成完整应用"的需求
* **避免强制重构**: 不需要为了统一而改写信号名解析和延迟 JS 发出逻辑
* **调试速度**: 独立实现可以针对调试场景优化（editor_mode、debug_mode、运行时错误报告）
* **生产后端共享不受影响**: React/Vue/JS/TS/Python/Rust/Dart/Dioxus 继续受益于 traits

### Negative

* **~750 行重复代码**: `expr_to_js()` 和 `emit_statement()` 与 FormatExpr/FormatStmt 功能重叠
* **维护负担**: 新增 ExprIR/StatementIR 变体时需要在 OmniView 中独立更新
* **学习曲线**: 新贡献者需要理解两套并行的表达式格式化系统

### Neutral

* 如果未来 OmniView 演变为生产目标，可以重新评估统一方案

## Implementation Strategy

### Blast Radius

**Impact Scope**: 无——此决策确认现状，不引入变更

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: 此决策仅确认不进行统一。如果未来决定统一，D1 迁移路径仍然存在（创建 `OmniViewFormatExpr` 包装 `JsFormatExpr` + 信号名解析）。

## Alternatives Considered

### 方案 B: 强制 OmniView 采用 FormatExpr/FormatStmt

**Pros:**
* 消除 ~500 行重复代码
* 与 React/Vue 统一的表达式格式化

**Cons:**
* 需要重构信号名解析（`name_renames`、`.value` 后缀）
* 需要适配状态式生成模式（`self.js.push_str` vs `CodeWriter`）
* 调试特定功能（editor_mode、运行时错误注入）需要额外适配层
* 用户反馈：OmniView 是调试工具，不应与生产后端同等对待

### 方案 C: 仅让 OmniView 实现 FormatExpr，stmt 保持独立

**Pros:**
* 折中方案，部分消除重复

**Cons:**
* 增加碎片化——部分使用 trait、部分独立
* 收益减半，复杂度不减

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Overall Decision Complexity Assessment**: 此决策是"有意识地不做事"——确认现状而非引入变更。强制统一会增加系统复杂度（需要适配层、重构状态式生成模式）而收益有限（OmniView 是调试工具，不面向最终用户部署）。

**Necessity Assessment**: 3/10
- **Current need**: 无——OmniView 当前作为调试工具工作正常
- **Future need**: 仅当 OmniView 成为生产目标时才需要统一
- **Cost of waiting**: 零——继续维护独立实现无额外成本
- **Evidence of need**: 无用户报告或 bug 与"OmniView 未使用 FormatExpr"相关

**Complexity Assessment (if we were to unify)**: 6/10
- **Added complexity**: 需要适配层处理信号名解析 + 状态式生成模式
- **Maintenance burden**: 新增一层间接引用
- **Learning curve**: 需要理解 trait + 适配器两层抽象
- **Dependencies introduced**: 新增 OmniView→FormatExpr 依赖

**Recommendation**: ✅ Approve decision (confirm status quo — do not unify)

**Pragmatic Score**:
- **Necessity**: 3/10
- **Complexity (of unifying)**: 6/10
- **Ratio**: 2.0 *(Exceeds target <1.5 — confirming that unification is indeed worse than status quo)*

**Overall Assessment**: 适当的克制——不为统一而统一。调试工具的独立实现是合理的工程权衡，避免了对低收益工作的过度投入。

## Validation

**Acceptance Criteria:**
- [x] OmniView codegen 测试通过（`test_component_with_effect` 等）
- [x] `FormatExpr`/`FormatStmt` traits 被 8 个生产后端正确使用
- [x] 无计划或工作进行 OmniView trait 统一

## References

- [[session-2026-07-28-continuation]] — D1 决策讨论
- [[omnilang-codegen-issues]] — P0-2 去重状态
- `omnilang/src/codegen/omniview/expr.rs` — OmniView 独立表达式格式化
- `crates/omnilang-core/src/codegen/shared/expr.rs` — FormatExpr trait
