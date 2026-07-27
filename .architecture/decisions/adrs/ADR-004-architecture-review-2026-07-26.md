# ADR-004: 2026-07-26 全工作区架构评审结论

## Status

Accepted

## Context

2026-07-26 对 OmniLang 工作区（主要项目：Poiesis 编辑器 + OmniLang 编译器）进行了全视角架构评审。11 位 AI 专家（系统架构师、领域专家、安全专家、可维护性专家、性能专家、实施策略专家、AI 工程师、务实执行者、Rust 专家、TS/React 专家、编译器架构师）并行独立评审，后经交叉分析汇总。

评审报告保存在 `.architecture/reviews/full-workspace-review-2026-07-26.md`，实现计划保存在 `.architecture/reviews/implementation-plan-2026-07-26.md`（经 3 专家评审修订）。

## Decision Drivers

1. **系统性债务识别**：11 专家发现 8 个跨领域主题和 39 条具体建议
2. **用户反馈**：评审后两条关键决策被澄清——Tauri FS 全覆盖为编辑器必要条件、代码生成目标聚焦 3 个
3. **可行性约束**：单人开发团队，需务实优先级排序
4. **长期可持续性**：需要架构治理框架（ADR、检查点、推迟跟踪）

## Decision

本 ADR 记录评审的核心结论和 6 项已确认的架构决策：

### 决策 1：整体评级——Adequate

工作区架构基本健全。编译管线（Lexer→Parser→Semantic→IR→Codegen）是教科书级阶段分离。主要债务集中在：
- 双解析器（TS + Rust）无自动化同步——**#1 风险**
- 9 个代码生成目标仅 1-3 个有实际用户——**#2 风险**
- Tauri 安全配置需加固（CSP、调试窗口隔离）——**#3 风险**

### 决策 2：Tauri FS 权限——豁免

**FS 全覆盖是编辑器必要条件**，不缩小。`C:/**` 和 `D:/**` 范围是设计意图而非安全疏忽。安全防御重点转向纵深防御：XSS、调试窗口隔离、路径遍历加固、供应链审计。详见 [[poiesis-tauri-security-model]]。

### 决策 3：代码生成目标——聚焦 P0（ADR-003）

仅 OmniView + React + Flutter 活跃。其余 6 目标暂停，feature-flag 保留但 CI 移除。详见 [[ADR-003]]。

### 决策 4：实现计划——32 项、6 Sprint、12 周

从评审的 39 项建议中，移除 1 项（P34 AI 补全）、推迟 4 项（P35/P36/P39/P38b），剩余 32 项组织为 6 个双周 Sprint。前 4 周（Sprint 1-2）交付 80% 价值——安全基线 + 测试骨架。详见实现计划。

### 决策 5：ADR 策略——简洁优先

本项目采用"1 篇回溯文档 + 决策时即时写 ADR"策略。关键架构决策（编译器管线、编辑器演进、目标优先级、评审结论）记录为 ADR-001 至 ADR-004。未来重大决策即时创建 ADR——不积压到某个 Sprint。

### 决策 6：编辑器范式确认——v5 Monaco（ADR-002）

v5 CodeBlock Editor（Monaco 文本编辑 + WASM 语义分析）是当前架构。积木编辑器（v3/v4）已废弃。详见 [[ADR-002]]。

## Consequences

### Positive

- **债务具名化**：8 个主题、39 条建议、优先级排序——从"感觉这里不对劲"到可操作的路线图
- **治理框架就位**：ADR 体系、推迟跟踪、Sprint 检查点
- **用户参与决策**：FS 权限和代码生成目标的澄清防止了未来的评审误报

### Negative

- **评审成本**：11 专家并行评审是昂贵的流程，每年 1-2 次为宜
- **计划维护负担**：实现计划需要持续更新进度

### Neutral

- **ADR 体系启动成本**：4 篇 ADR 花了初始创建时间，但后续决策记录将更快

## Implementation Strategy

### 实施路径

实现计划分为 6 个 Sprint，详见 `.architecture/reviews/implementation-plan-2026-07-26.md`。核心交付链：

```
Sprint 1 (安全基线) → Sprint 2 (测试骨架) → Sprint 3 (编译器健康)
    → Sprint 4 (类型+测试) → Sprint 5 (债务清零) → Sprint 6 (战略就绪)
```

### 关键检查点

- Week 3: Golden 文件 CI 双解析器通过？
- Week 5: TS round-trip 测试就绪？
- Week 7: 参数化测试覆盖 P0 后端？
- Week 9: IR source_span 在输出中可见？

### 推迟项跟踪

| 项目 | 推迟原因 | 重新评估条件 |
|------|---------|-------------|
| P34: AI 补全 | 新功能非债务 | 编辑器和编译器核心稳定后 |
| P35: Crate 拆分 | YAGNI | 编译器 >100K 行 + ≥5 活跃后端 |
| P36: 代码生成并行化 | 无 profiling 数据 | codegen >500ms（500 行 .omni） |
| P39: IR 优化传递 | 缺少测试基础 | motion 95% 分支覆盖率 |
| P38b: Strip TAURI_INTERNALS | 防御性过早 | iframe 沙箱生效后 |

## Alternatives Considered

### 不做全工作区评审

**被拒绝**：项目在 3 个月内增长了 ~105K 行代码，多个子系统已出现系统性债务。评审时机正确。

### 仅做局部评审（如单项目）

**被拒绝**：许多发现（如双解析器、IR 类型级联、Tauri 安全）跨越了项目边界。全工作区视角是必要的。

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Necessity Assessment**: 7/10
- 评审本身价值高——具名化了之前模糊感知的架构问题
- 但实现计划中的 37 项原计划过多——经 revision 缩减到 32 项
- 等待成本：推迟评审会导致债务继续积累

**Complexity Assessment**: 4/10
- 评审+计划是文档，不是代码——本身不增加系统复杂度
- 维护 ADR 和实现计划需要持续精力，但回报是防止决策漂移

**Pragmatic Score**:
- 必要性: 7/10
- 复杂度: 4/10
- 比率: 0.57 ✅

**Recommendation**: ✅ 批准评审结论和后续行动计划。注意：实现计划 Sprint 5-6 为条件推进——若 Sprint 1-4 已完成且精力允许才推进。

## Validation

**Acceptance Criteria**:
- [x] 全工作区评审完成（11 专家）
- [x] 实现计划评审完成（3 专家修订）
- [x] 4 篇 ADR 记录核心架构决策
- [ ] 实现计划 Sprint 1 完成（P15 all-backends 移除等）
- [ ] Golden 文件双解析器 CI 通过（Sprint 2）
- [ ] 所有关键链检查点通过（Sprint 1-6）

## References

- [architecture-review-2026-07-26](../reviews/full-workspace-review-2026-07-26.md) — 11 专家全工作区评审
- [plan-review-2026-07-26](../reviews/plan-review-2026-07-26.md) — 实现计划 3 专家评审
- [implementation-plan-2026-07-26](../reviews/implementation-plan-2026-07-26.md) — 修订后实现计划
- [poiesis-tauri-security-model](../../../C:/Users/XHT/.claude/projects/D--space-OmniLang/memory/poiesis-tauri-security-model.md) — Tauri 安全模型
- [architecture-review-2026-07-26](../../../C:/Users/XHT/.claude/projects/D--space-OmniLang/memory/architecture-review-2026-07-26.md) — 评审记忆
- [[ADR-001]] — 编译器管线架构
- [[ADR-002]] — 编辑器范式演进
- [[ADR-003]] — 代码生成目标优先级
