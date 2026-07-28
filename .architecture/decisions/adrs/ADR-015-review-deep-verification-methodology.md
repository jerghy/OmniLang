# ADR-015: Architecture Review — Mandatory Deep Verification Before Acting on Findings

## Status

Accepted

## Context

OmniLang 项目经历了多轮架构评审（14 次评审，2026-07-26 至 2026-07-28），积累了 13 个 ADR 和 40+ memory 文件。然而，最近的 v2 广度评审暴露了一个系统性问题：

- **v2 广度评审**（2026-07-28）：14 项 HIGH 发现中有 **3 项（21%）在深度验证中被证明是按设计的误报**
- FS 作用域"过于宽松" → [[project-permissions]] 明确豁免
- 跨目标测试"仅 3/9 后端" → [[ADR-003]] 明确 P0-only 策略
- ArrayLiteral "仅检查第一个元素" → [[ADR-011]] 已知限制

这些误报并非评审质量差——每位专家都正确识别了代码中的真实模式。问题在于：**广度评审的专家无法获取项目 ADR 和 memory 中记录的设计意图上下文**。

之前记录的 33% 子代理误判率与此发现一致：缺乏设计上下文直接导致误报。

同时，经过多轮修复，项目已积累了隐式的质量门控流程（`cargo test --workspace`、`cargo clippy -- -D warnings`、`cargo fmt --check`），但从未正式记录为架构决策。

## Decision Drivers

1. **误报成本**：3 项误报发现消耗了约 30% 的深度验证工作量——这些工作量本可通过事先查阅 ADR 避免
2. **设计知识丢失风险**：随着 ADR 和 memory 积累，新评审者（人或 AI）无法了解历史设计决策的概率增加
3. **质量门控一致性**：经过 14 轮评审和数百次修复，已验证的质量门控流程应固化
4. **可重复性**：评审方法论应可被未来的会话复用，不依赖当前会话的上下文

## Decision

### Part A: 架构评审强制深度验证步骤

**所有架构评审发现（无论严重度）在采取行动之前，必须经过深度验证——与 ADR 和 memory 交叉引用。**

具体步骤：
1. **广度评审**（Layer 1）：专家并行评审，生成发现列表
2. **深度验证**（Layer 2）：逐项交叉引用——ADR（全部）、memory 文件（全部）、git 历史（设计意图）
3. **分类输出**：
   - ✅ **确认真实** — 与所有来源交叉验证，确认是 bug 或设计缺陷
   - ❌ **按设计（BY DESIGN）** — 现有 ADR 或 memory 已记录此设计决策
   - ⚠️ **需要澄清** — 无现有文档，需要向项目维护者确认设计意图
4. **仅对"确认真实"的发现采取修复行动**

**评审提示词模板更新**：未来所有架构评审的专家提示词必须包含：
> "在报告发现之前，先检查 `.architecture/decisions/adrs/` 中的相关 ADR 和项目 memory 文件——某些模式可能是有意为之的设计决策。"

### Part B: 修复验证质量门控

**所有编译器修复提交必须通过以下质量门控：**

| 门控 | 命令 | 通过标准 |
|------|------|----------|
| 测试 | `cargo test --workspace` | 801+ 通过，0 失败 |
| Lint | `cargo clippy --workspace -- -D warnings` | 0 警告 |
| 格式 | `cargo fmt --all -- --check` | 无格式差异 |
| 构建 | `cargo build --workspace` | 编译成功 |

**对于仅文档更改**：跳过测试/lint/构建门控，仅格式检查。

**对于仅 CI 更改**：跳过测试门控（CI 更改不在本地运行），仅格式检查。

**Architectural Components Affected:**
- `.architecture/reviews/` — 所有未来评审文档遵循双层结构（广度 + 深度验证）
- 评审专家提示词模板 — 新增 ADR/内存交叉引用指令
- SDD 修复执行流程 — 强制质量门控检查点

**Interface Changes:**
- 无代码接口变更。纯流程变更。

## Consequences

### Positive

* **消除按设计误报**：21% 的广度评审发现不再消耗修复资源——在行动前被过滤
* **设计知识保留**：每次评审强化 ADR 和 memory 作为权威来源的角色，防止知识丢失
* **质量一致性**：固化质量门控防止回退（如本会话中 TypeScript 侧的 react-doctor 警告是本会话修改之前就存在的）
* **可重复性**：未来的会话可复用相同的双层评审流程，不依赖隐式知识

### Negative

* **评审流程时间增加**：深度验证增加约 30-60 分钟的交叉引用时间（但避免了数小时的错误修复）
* **ADR/内存维护负担**：流程的有效性依赖 ADR 和 memory 的准确性——陈旧文档直接导致验证失败
* **提示词模板膨胀**：每个评审提示词增加一段上下文，增加 token 消耗

### Neutral

* 本 ADR 不改变架构评审或修复执行的核心流程——它只是将已在实际中验证有效的步骤正式化

## Implementation Strategy

### Blast Radius

**Impact Scope**: 仅评审和修复流程。不影响编译器代码、CI 配置或编辑器行为。

**Affected Components**:
- 架构评审技能（`ai-software-architect:architecture-review`）— 提示词模板更新
- SDD 修复执行流程 — 质量门控检查点
- `.architecture/reviews/` — 文件命名约定（`*-breadth-review.md` + `*-deep-verification.md`）

**Risk Mitigation**:
- 本 ADR 记录的是已在 14 次评审中验证有效的流程——不是实验性变更
- 回退方案：如果流程过于繁重，可放宽为"仅 CRITICAL/HIGH 发现需要深度验证"

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: 5 分钟——恢复旧的评审提示词模板，移除质量门控要求。无代码依赖。

**Options Preserved**:
- 可演进为自动化 CI 检查（例如，验证 README ADR 数量与实际文件匹配的脚本）
- 可扩展为"设计决策 freshness check"——标记超过 30 天未验证的 ADR

### Sequencing & Timing

**Prerequisites**: 无。本 ADR 记录现有流程，无需实现。

**Readiness Assessment**: Ready to implement——已在当前会话中验证有效。

### Social Cost

**Learning Curve**: Low
- 团队已隐式遵循此流程——本 ADR 使其显式化
- 新增的"检查 ADR 和 memory"步骤是自然的信息收集行为

**Cognitive Load**: Minimal
- 评审者已经需要阅读代码——阅读 ADR 是同一活动的扩展
- 质量门控是自动化命令，非手动检查

## Implementation

**Phase 1: 文档化（本会话）**
* 创建本 ADR
* 更新 `session-2026-07-28-v2-review-fixes.md` memory —— 引用本 ADR

**Phase 2: 流程集成（下次评审）**
* 更新架构评审技能提示词模板
* 确保所有子代理评审者收到 ADR/内存交叉引用指令

## Alternatives Considered

### 方案 B: 仅对 CRITICAL/HIGH 发现进行深度验证

**Pros:**
* 减少验证工作量
* MEDIUM/LOW 发现不太可能是按设计误报

**Cons:**
* 本会话中 2/3 误报是 HIGH 严重度——此方案无法捕获
* 严重度评级本身可能有误（广度评审的严重度在深度验证中经常被下调）

**→ 被拒绝**：严重度不是设计意图的可靠代理指标。

### 方案 C: 自动化 ADR/内存 freshness 检查

**Pros:**
* 零人工工作量
* 在陈旧文档导致误报之前捕获

**Cons:**
* 需要构建和维护 CI 集成
* 自然语言 ADR 的自动验证是不平凡的

**→ 推迟**：手动交叉引用在 13 个 ADR 的规模下已足够。当 ADR 数量超过 50 时重新评估。

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 本 ADR 并未引入新的工程复杂性——它将已在 14 次评审中验证有效的现有流程正式化。零代码变更。零新依赖。

**Decision Challenge**:

**Proposed Decision**: "所有架构评审发现必须经过深度验证——与 ADR 和 memory 交叉引用——在采取行动之前。"

**Necessity Assessment**: 9/10
- **Current need**: 21% 误报率是可量化的浪费。每次评审在误报上损失约 30% 的验证时间。
- **Future need**: 随着 ADR 和 memory 的增长，没有交叉引用的广度评审误报率将上升。
- **Cost of waiting**: 零——本 ADR 记录现有实践。
- **Evidence of need**: v2 广度评审 3/14 误报；之前记录的 33% 子代理误判率。

**Complexity Assessment**: 1/10
- **Added complexity**: 零代码变更。1 个新 ADR 文件。提示词模板增加一段。
- **Maintenance burden**: 零——记录现有流程。
- **Learning curve**: 零——团队已隐式遵循。
- **Dependencies introduced**: 无。

**Alternative Analysis**:
- 方案 B（仅 CRITICAL/HIGH）被正确拒绝——严重度不是设计意图的代理。
- 方案 C（自动化 freshness 检查）被正确推迟——13 个 ADR 的规模下不值得。

**Simpler Alternative Proposal**: 无——这已经是最简方案（记录现有实践）。

**Recommendation**: ✅ Approve decision

**Justification**: 这不是在引入一个新流程——而是在命名和记录一个已验证有效的现有流程。零成本，高杠杆。唯一风险是如果 ADR/内存未得到维护，流程会衰减——但这是任何依赖文档的流程都面临的失败模式。

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 1/10
- **Ratio**: 0.11 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: 教科书级别的流程正式化——零工程成本，高重复性价值。本 ADR 应作为未来评审技能的参考。

## Validation

**Acceptance Criteria:**
- [x] ADR 创建并合并到 main
- [x] 引用本会话的 21% 误报率作为证据
- [ ] 下次架构评审使用更新后的提示词模板（包含 ADR/内存交叉引用指令）
- [ ] 下次架构评审的误报率 <10%

**Testing Approach:**
- 通过下次架构评审的误报率验证——如果深度验证仍然发现 >10% 按设计误报，则流程需要调整

## References

- [[session-2026-07-28-v2-review-fixes]] — 本会话记录（21% 误报率证据）
- [[session-2026-07-28-breadth-review-fixes]] — v1 广度评审流程
- [[subagent-reliability-issues]] — 33% 子代理误判率
- [[ADR-007]] — 评审方法论（Layer 1/2/3 定义）
- [2026-07-28-breadth-review-v2.md](../reviews/2026-07-28-breadth-review-v2.md) — v2 广度评审
- [2026-07-28-deep-verification-v2.md](../reviews/2026-07-28-deep-verification-v2.md) — v2 深度验证
