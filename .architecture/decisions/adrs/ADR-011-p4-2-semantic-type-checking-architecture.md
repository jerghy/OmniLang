# ADR-011: P4-2 Semantic Type Checking Architecture — Lightweight Bottom-Up Inference

## Status

Accepted

## Context

OmniLang v0.1 需要类型检查来捕获表达式级别的类型错误（如 `"hello" + 42`、`return 1` in void function）。语言当前不支持用户自定义泛型函数或高阶类型，类型系统的消费者仅限于编译时错误诊断。

在 P4-2 设计阶段，评估了三种方案：
- **方案 A**: 结构化 enum + bottom-up inference，不加 unification/泛型（~150 行核心类型 + ~500 行检查逻辑）
- **方案 B**: 完整 Hindley-Milner 类型推理引擎（~1,500+ 行）
- **方案 C**: IR 级字符串匹配增强（在 codegen 阶段做简单的类型名比对）

## Decision Drivers

1. **当前无泛型需求**: v0.1 语言无用户自定义泛型函数，HM 统一引擎当下无消费者
2. **类型系统演进性**: 方案 A 可以日后升级到方案 B，反之则需大量返工
3. **同步成本可控**: 三套类型表示（TypeAnnotation→SemanticType→TypeIR）之间转换代码 <100 行，比改 AST 或改 IR 的风险低
4. **虚假阳性控制**: 方案 C 的字符串匹配导致假阳性泛滥且不可演进
5. **测试覆盖**: 方案 A 可在 2-3 天内实现并达到 737 测试覆盖

## Decision

**选择方案 A：轻量 AST 级类型检查，使用 `SemanticType` enum 独立于 `TypeAnnotation` 和 `TypeIR`，采用 bottom-up 表达式类型推理（`infer_expr_type`）。**

**核心架构**:
```
types.rs (SemanticType enum — 13 variants)
    ↓
validate_types.rs (infer_expr_type + check_* functions)
    ↓
resolve.rs (在 resolve pass 中集成类型检查)
```

**SemanticType 包含 13 个变体**: Int, Float, String, Bool, Void, Null, Named, Array, Map, Optional, Union, Fn, Any

**类型检查规则**:
- 二元运算符: 算术→数值类型，逻辑→Bool，+→String 连接 (LHS 优先)
- 赋值: 声明类型必须兼容初始化类型
- 返回: 返回值必须兼容声明的返回类型
- 兼容性: 同类型、Int→Float 拓宽、`T?` 接受 `T`、Null 兼容 `Optional<T>`、Any 通配
- Any 级联抑制: 防止未解析变量产生级联假阳性

**Architectural Components Affected:**
- `crates/omnilang-core/src/semantic/types.rs` — 新增 SemanticType enum
- `crates/omnilang-core/src/semantic/validate_types.rs` — 类型检查引擎
- `crates/omnilang-core/src/semantic/validate_component.rs` — 组件级验证
- `crates/omnilang-core/src/semantic/resolve.rs` — resolve pass 集成
- `crates/omnilang-core/src/error.rs` — E0010 诊断码

**Interface Changes:**
- 新增 `semantic::analyze()` 公共入口，Poiesis 通过 `omnilang::semantic::analyze(&program)` 调用
- TypeAnnotation→SemanticType 转换: `SemanticType::from_type_annotation()`
- SemanticType 序列化独立于 TypeIR，不破坏 9 个 codegen 后端的序列化契约

## Consequences

### Positive

* **低复杂度**: ~700 行核心代码，737 测试通过，零 clippy
* **独立演进**: SemanticType 不污染 parser AST，不改 codegen 契约
* **可升级**: 日后需要泛型时可将 `infer_expr_type` 替换为 unification 引擎
* **假阳性控制**: Any 类型级联抑制机制防止未解析符号的级联错误

### Negative

* **三套类型表示**: 新增类型需更新三处（TypeAnnotation→SemanticType 转换 + SemanticType 定义本身）
* **已知限制**: MatchExpr/ArrayLiteral 仅检查第一个元素、赋值语句无类型检查、一元运算符无类型检查、比较运算符仅接受数值类型

### Neutral

* 方案 B（完整 HM）的门始终开放——SemanticType 可被 HM 引擎替代而不影响其他模块

## Implementation Strategy

### Blast Radius

**Impact Scope**: 限于 `semantic/` 模块，对外仅暴露 `analyze()` 函数

**Affected Components**:
- `semantic/types.rs` — 新增，149 行
- `semantic/validate_types.rs` — 重写，577 行
- `semantic/resolve.rs` — 类型检查集成，+61 行
- `error.rs` — E0010 诊断码，+3 行

**Risk Mitigation**: 27 个单元测试 + 5 个集成测试覆盖核心检查逻辑

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: 删除 `semantic/types.rs` 和 `semantic/validate_types.rs`，移除 `resolve.rs` 中的 3 处集成点即可回退。无破坏性接口变更。

**Options Preserved**: 完整 HM 引擎、IR 级类型检查、运行时类型检查均保持可行

### Sequencing & Timing

**Prerequisites**: 无——方案 A 是独立的增量

**Readiness Assessment**: ✅ 已实现并合并 main

## Alternatives Considered

### 方案 B: 完整 Hindley-Milner 类型推理

**Pros:**
* 理论上最完备的类型系统
* 支持泛型、类型变量、统一

**Cons:**
* ~1,500+ 行实现，3-5 天工作量
* v0.1 无泛型消费者——过早固化
* 引入 unification 变量增大调试复杂度

### 方案 C: IR 级字符串匹配增强

**Pros:**
* 改动最小（仅 codegen 级别）

**Cons:**
* 假阳性泛滥——字符串匹配无法区分同名不同类型
* 不可演进——每增加一种类型检查需逐个硬编码
* 技术债——parser/semantic/codegen 职责混乱

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Overall Decision Complexity Assessment**: 方案 A 是对当前实际需求的最小可行实现。v0.1 语言无泛型，完整 HM 引擎当下无消费者。选择结构化 enum + bottom-up 推理而非完整 HM 是正确的 YAGNI 决策。

**Necessity Assessment**: 7/10
- **Current need**: 表达式类型错误是真实用户痛点（`"hello" + 42` 等）
- **Future need**: 泛型支持是 v0.2+ 的需求，届时方案 A 可替换
- **Cost of waiting**: 不做类型检查会导致运行时错误，调试体验差
- **Evidence of need**: 27 个测试用例覆盖了真实的类型错误场景

**Complexity Assessment**: 3/10
- **Added complexity**: 新增 1 个 enum + 1 个检查模块，~700 行
- **Maintenance burden**: 三套类型表示同步成本 ~100 行转换代码
- **Learning curve**: 标准 bottom-up 推理，编译器开发者熟悉的模式
- **Dependencies introduced**: 无新外部依赖

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 7/10
- **Complexity**: 3/10
- **Ratio**: 0.43 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: 适当的工程投入——以最小复杂度解决了真实的类型安全问题，保留了未来升级路径，拒绝了两倍以上复杂度的替代方案。

## Validation

**Acceptance Criteria:**
- [x] 27 个类型检查单元测试通过
- [x] 5 个集成测试通过
- [x] 737 总测试零失败
- [x] Clippy 零警告
- [x] Poiesis 编译兼容（`omnilang::semantic::analyze` 接口不变）

## References

- [[session-2026-07-28-p4-completion]] — P4-2 实现完成记录
- [[next-session-2026-07-28]] — 设计决策 D3/D4/D5/D6
- `crates/omnilang-core/src/semantic/types.rs` — SemanticType 实现
- `crates/omnilang-core/src/semantic/validate_types.rs` — 类型检查引擎
