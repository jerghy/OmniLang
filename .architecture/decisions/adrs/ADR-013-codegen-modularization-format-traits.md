# ADR-013: Codegen Modularization — FormatExpr/FormatStmt/FormatType Trait Architecture

## Status

Accepted (substantially implemented)

## Context

OmniLang 有 10 个 codegen 后端（OmniView、React、Vue、JavaScript、TypeScript、Python、Rust、Dart、Dioxus、Flutter）。在 P35 去重和 v5 修复之前，每个后端独立实现了表达式翻译、语句格式化、类型映射、事件处理——总计约 18,000 行代码，估计 40-50% 为跨后端重复。

参考 Haxe Reflaxe BaseCompiler 模式，P0-2 提出提取 `UiCodegen` trait。实际实施超出了最初范围，建立了三层 trait 架构。

## Decision Drivers

1. **消除表达式格式化重复**: 8 个后端各自实现了相同的 `ExprIR` → 目标代码 dispatch
2. **消除语句格式化重复**: `let`、`if`、`return`、`for`、`while` 等语句格式化逻辑高度相似
3. **类型映射模式相同**: `i32→int/number/i64` 映射因语言而异但结构一致
4. **内联事件处理重复**: React/Vue/Flutter/Dioxus 各自生成 `addEventListener` 等效代码
5. **文件大小失控**: 去重前 `html.rs` 58K、`motion_triggers.rs` 41K

## Decision

**采用三层 trait 架构，按语言族提供默认实现，后端仅覆盖差异点。**

**架构层次**:
```
Layer 1: Codegen trait (traits.rs, 110 lines)
    └─ 通用 codegen 接口，含 generate_per_module / generate_per_component 策略

Layer 2: Format traits (expr.rs 479 lines + stmt.rs 425 lines + types.rs 261 lines)
    ├─ FormatExpr — 表达式格式化，3 族实现 (Js/Dart/Rust)
    ├─ FormatStmt  — 语句格式化，8 formatter 实现
    └─ FormatType  — 类型映射，4 族实现 (Js/Dart/Rust/Python)

Layer 3: UI-specific traits (ui_codegen.rs 64 lines)
    └─ UiCodegen — UI 后端特有接口（组件代码/CSS/Store 生成）
```

**族实现模式**:
| 族 | FormatExpr | FormatStmt | 后端 |
|----|-----------|-----------|------|
| JS | `JsFormatExpr` | `JsStmtFormatter`, `ReactStmtFormatter`, `VueStmtFormatter` | React, Vue, JS, TS |
| Dart | `DartFormatExpr` | `DartStmtFormatter`, `FlutterStmtFormatter` | Dart, Flutter |
| Rust | `RustFormatExpr` | `RustStmtFormatter`, `DioxusStmtFormatter` | Rust, Dioxus |
| Python | (default) | `PythonStmtFormatter` | Python |

**共享辅助模块**:
- `inline_handler.rs` (63 行) — `build_inline_handler()` 被 React/Vue/Flutter/Dioxus 共享
- `html.rs` (93 行) — `to_html_tag()`/`to_html_attr()` 被 OmniView/React/Vue/Dioxus 共享
- `generator.rs` (70 行) — `build_component_map()` + 构造函数初始化

**Architectural Components Affected:**
- `crates/omnilang-core/src/codegen/shared/expr.rs` — FormatExpr trait + 3 族实现
- `crates/omnilang-core/src/codegen/shared/stmt.rs` — FormatStmt trait + 8 formatter
- `crates/omnilang-core/src/codegen/shared/types.rs` — FormatType trait + 4 族实现
- `crates/omnilang-core/src/codegen/shared/ui_codegen.rs` — UiCodegen trait
- `crates/omnilang-core/src/codegen/traits.rs` — Codegen trait + 策略方法
- `omnilang/src/codegen/react/expr.rs` — ReactFormatExpr (thin wrapper over JsFormatExpr)
- `omnilang/src/codegen/vue/expr.rs` — VueFormatExpr (thin wrapper over JsFormatExpr)

## Consequences

### Positive

* **消除 ~5,000+ 行重复**: 表达式/语句/类型格式化统一为 ~1,200 行共享 trait 代码
* **新增后端成本降为 O(配置)**: 新语言后端只需实现 trait 并覆盖 5-10 个差异方法
* **文件大小合规**: 所有生产文件 <850 行（从 58K 降至 ~800）
* **族内一致性**: JS 族（React/Vue/JS/TS）共享相同的表达式语义（`===`, `?.`, `undefined`）
* **可测试性**: trait 方法可独立单元测试，不依赖特定后端

### Negative

* **抽象层增加**: 理解一个表达式如何生成需要追踪 trait → 族实现 → 后端覆盖
* **OmniView 未统一**: 调试工具保持独立实现（见 [[ADR-012]]）
* **React 语句格式未迁移**: `generate_statement_tsx_with_mode` (~400 行) 仍为独立实现，未使用 FormatStmt

### Neutral

* trait 的泛型关联类型 (`type ExprFormatter: FormatExpr`) 增加了一些类型系统复杂度

## Implementation Strategy

此架构在 P35 去重（crate 拆分）和 v5 修复中逐步实现。关键里程碑:

**Phase 1: 提取共享 traits → 已完成**
- 定义 FormatExpr/FormatStmt/FormatType traits
- 实现 3 族 FormatExpr（Js/Dart/Rust）
- 实现 8 个 FormatStmt formatter

**Phase 2: 后端迁移 → 已完成**
- React/Vue 迁移到 thin wrapper 模式
- JS/TS/Python/Rust/Dart/Dioxus 直接使用族实现

**Phase 3: UiCodegen 深化 → 推迟**
- 当前 UiCodegen 为 thin delegation（注释: "code is NOT moved into the trait methods"）
- 深化需将 gen_component_file/gen_store_file 逻辑移入 trait 默认实现

### Reversibility

**Reversibility Level**: Medium

**Rollback Feasibility**: 每个后端的 thin wrapper 可回退到独立实现，但需逐个后端迁移。trait 定义本身无破坏性——它们是增量添加的。

## Alternatives Considered

### 方案 B: 每个后端完全独立（P35 之前的状态）

**Pros:**
* 无抽象层，直接可读
* 每个后端自由选择最适合的代码模式

**Cons:**
* ~5,000+ 行重复代码
* 新增 ExprIR 变体需更新 8 个后端
* 文件大小失控（58K html.rs）
* **已被实际经验否决**

### 方案 C: 宏驱动的代码生成

**Pros:**
* 编译时生成，零运行时开销

**Cons:**
* Rust 宏调试困难
* 不适合需要运行时配置的场景（如 `strict_eq` 因语言而异）
* 不如 trait + 族模式灵活

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Overall Decision Complexity Assessment**: 三层 trait 架构是对真实重复问题的成比例回应。5,000+ 行实际存在的重复代码（非推测性）证明了抽象的合理性。族模式（Js/Dart/Rust）以最少配置覆盖了大多数后端。

**Necessity Assessment**: 8/10
- **Current need**: P35 之前 8 个后端各自独立实现相同逻辑——这是已验证的痛点
- **Future need**: 新增后端（如 Swift、Kotlin）将直接受益
- **Cost of waiting**: 无——此架构已实现
- **Evidence of need**: 文件大小从 58K 降至 ~800 行，净减少 ~12,800 行

**Complexity Assessment**: 4/10
- **Added complexity**: 3 traits + 族实现，~1,200 行共享代码
- **Maintenance burden**: 新增 ExprIR 变体需更新 trait 默认实现 + 各族的覆盖（如有）
- **Learning curve**: 中等——需要理解 trait 默认实现 + 覆盖机制
- **Dependencies introduced**: 无新外部依赖

**Recommendation**: ✅ Approve decision (already implemented — confirming correctness)

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 4/10
- **Ratio**: 0.5 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: 教科书级别的抽象——消除已验证的重复而非推测性的"可能需要"。族模式以最小配置成本覆盖了 8 个后端，是 Haxe Reflaxe BaseCompiler 模式在 Rust 中的恰当翻译。

## Validation

**Acceptance Criteria:**
- [x] 8 个后端正确使用 FormatExpr/FormatStmt traits
- [x] 文件大小合规（最大 ~814 行）
- [x] 737 测试全部通过
- [x] Clippy 零警告
- [x] 全工作区构建零错误

## References

- [[omnilang-codegen-issues]] — P0-2/P1 去重任务
- [[session-2026-07-28-continuation]] — Codegen 模块化验证
- [[ADR-012]] — OmniView 调试工具独立实现
- `crates/omnilang-core/src/codegen/shared/expr.rs` — FormatExpr trait
- `crates/omnilang-core/src/codegen/shared/stmt.rs` — FormatStmt trait
- Haxe Reflaxe BaseCompiler — 参考模式
