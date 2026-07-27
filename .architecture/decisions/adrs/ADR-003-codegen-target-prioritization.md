# ADR-003: 代码生成目标优先级（9→3）

## Status

Accepted

## Context

OmniLang 编译器原有 9 个代码生成目标：OmniView、React、Flutter、Vue、Dioxus、Dart、Rust、Python、JavaScript。每个目标都通过特性门控独立编译，通过 `all-backends` 元特性全部开启。

2026-07-26 全工作区架构评审发现：
- 仅 **1-3 个目标有编辑器中实际用例**（OmniView 用于预览、React 是 Web 生产目标、Flutter 是移动端战略目标）
- 其余 6 个目标合计 ~8,400 行代码，仅保证编译通过
- 9 个几乎完全相同的回归测试文件（每个 58 行）增加了维护噪音
- 每个 IR 变更需接触 6+ 代码生成后端——**IR 类型级联问题**
- 务实执行者分析：复杂度/必要性比率 = **2.67**（远超 1.5 阈值）

## Decision Drivers

1. **维护惯性**：IR 变更需要在未使用的后端中进行同步更新
2. **开发者精力**：单人团队无法维持 9 个平行的代码生成器
3. **实际用例**：仅 OmniView（编辑器预览）、React（Web 生产）、Flutter（移动端）有明确需求
4. **测试覆盖**：非活跃目标的测试提供了虚假的安全感但消耗 CI 时间
5. **YAGNI**：Dioxus、Vue、Dart 没有当前或可预见的用户需求

## Decision

**仅 OmniView、React、Flutter 保持活跃开发（P0）。其余 6 目标暂停发展。**

**具体措施**：
- 从 CI 和默认开发工作流中移除 `all-backends` 特性
- 各非活跃目标保留在 feature flag 后可编译——不删除代码
- 新语言特性**仅需在 P0 三个后端实现**
- 非活跃后端新增特性时返回描述性错误（`"unsupported: feature X not yet implemented for target Y"`），不静默失败
- 原 `all-backends` 回归测试从 CI 移除；参数化测试框架（实现计划 P19）仅覆盖 P0 后端
- 推迟项记录在 `.architecture/deferrals.md`

**Architectural Components Affected**:
- `omnilang/Cargo.toml` — `all-backends` 特性保留但不在默认 CI 中使用
- `omnilang/src/codegen/mod.rs` — `compile()` 的 `#[allow(unreachable_patterns)]` 保留但仅活跃后端测试
- `omnilang/src/codegen/tests_regression/` — 非活跃后端测试文件保留但不运行
- CI 配置 — 测试矩阵仅包含 `codegen-omniview`、`codegen-react`、`codegen-flutter`

## Consequences

### Positive

- **IR 变更影响面从 6+ 后端降低到 3**——新特性开发速度提升
- **CI 时间减少**——不编译/测试非活跃后端
- **代码审查负担降低**——PR 不再需要审查非活跃后端中的级联更改
- **专注**——团队精力集中在用户实际接触的目标上

### Negative

- **非活跃目标可能落后**——若未来有用户需求，可能需要追赶工作
- **"通用 DSL"叙事减弱**——OmniLang 从 "编译到 10 个框架" 变为 "编译到 3 个框架（+6 实验性）"
- **Feature flag 架构保留**——即使测试不运行，代码仍然存在并在编译时检查

### Neutral

- **React 从原 P2 列表提升到 P0**——作为 Web 生产目标有明确需求
- **Flutter 保持 P0**——移动端战略目标，但当前使用量低于 OmniView

## Implementation Strategy

### Blast Radius

**影响范围**：仅 CI 配置和回归测试运行器。不删除代码。爆炸半径：Low。

### Reversibility

**可逆性级别**：High——重新启用 `all-backends` CI 是一个单行更改。

**回滚可行性**：5 分钟——恢复 CI 中的 `--features all-backends` 即可。
**选项保留**：所有 6 个非活跃后端的代码保留在仓库中，特性门控保持完整。

### Sequencing & Timing

**前置条件**：
- [x] 架构评审确认决策（2026-07-26）
- [ ] 实现计划 P15：从默认流中移除 all-backends（Sprint 1 Day 5）
- [ ] 实现计划 P19：P0 后端参数化测试框架（Sprint 4）

## Alternatives Considered

### 保留全部 9 目标活跃

**被拒绝**：维护惯性不可持续。务实执行者比率 2.67。单人团队无法维持。

### 删除非活跃目标代码

**被拒绝**：丢掉已工作的代码有不可逆风险。Feature flag 保留允许未来恢复——零维护成本。

### 全部折叠到单一目标

**被拒绝**：违背 OmniLang 作为多目标编译器的核心价值主张。OmniView、React、Flutter 各有独特的输出格式和运行时。

## Pragmatic Enforcer Analysis

**Mode**: Balanced（严谨但非严格）

**Necessity Assessment**: 9/10
- 当前需求：单人团队无法维持 9 个并行后端——这是可验证的瓶颈
- 等待成本：零——非活跃目标没有用户。推迟无成本
- 证据：IR 类型级联每次影响 6+ 后端；9 个重复的 58 行测试文件

**Complexity Assessment**: 2/10
- 增加的复杂度：零——原 feature flag 架构已存在。这是移除负担，而非增加
- 依赖：无新依赖

**Pragmatic Score**:
- 必要性: 9/10
- 复杂度: 2/10
- 比率: 0.22 ✅ **Ratio < 1.5（极佳）**

**Recommendation**: ✅ 批准——简化而非过度设计。非活跃目标的代码保留在 feature flag 后，零风险。

## Validation

**Acceptance Criteria**:
- [ ] `cargo test -p omnilang` 仅测试 P0 三个后端（CI 绿）
- [ ] 非活跃目标在 `cargo test --features all-backends` 下仍可编译
- [ ] CI 矩阵不包含 all-backends
- [ ] P19 参数化测试框架仅覆盖 P0 后端

## References

- [architecture-review-2026-07-26](../reviews/full-workspace-review-2026-07-26.md) — 全工作区评审（主题 2：过度扩展）
- [plan-review-2026-07-26](../reviews/plan-review-2026-07-26.md) — 实现计划评审
- [implementation-plan-2026-07-26](../reviews/implementation-plan-2026-07-26.md) — P15 实施步骤
- [[omnilang-codegen]] — 后端优先级（原有记录）
- [[ADR-001]] — 编译器管线架构
- [[ADR-004]] — 架构评审结论
