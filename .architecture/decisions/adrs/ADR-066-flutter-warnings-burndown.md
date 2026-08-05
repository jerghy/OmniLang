# ADR-066: Flutter warnings 燃尽处置（deferral #81——1004 → 671 诚实下限）

## Status

Implemented

> #81 目标处置：defer-with-trigger（TextStyle 语义路径，见 D4 与 Alternatives）

## Context

deferral #81（`.architecture/deferrals.md:337`）：Flutter 生成产物 warnings 计划目标 <300 未达成。燃尽轨迹：**1004（0.5 基线，881 ECodegen + 123 unused-import）→ 997（3.3）→ 991（3.5）→ 878（4.12）→ 874（8.1a 实测）→ 753（C2a）→ 700（C2b）→ 671（C2d，诚实下限）**。构成质变：08-02 历史家族（Curves 312/重复 motion 225/stdlib 202/String→Color 84）已全部消除；671 的 86% 为 **Task 4.3 design-decoration 解析器的诚实降级类**（#1 class 未声明 46 + #2 使用点 204 + #6 定义点 225），非翻译表缺口。

## Decision Drivers

* 单调下降纪律：门禁 cap 随每次实现更新（879→700→671），下降方向绝不逆转
* v3「诚实降级 > 静默」原则：ECodegen = 有警告的降级，保留而非删除（删除 = 回到静默）
* 诊断可行动性：ECodegen 警告补真实 source span（4.11 已为语义诊断落地，codegen 侧滞后）
* 语料正确性优先：demo 引用未声明 class = 语料问题先修（非解析器缺口——跨文件解析本就工作）
* 合并仅限呈现层：诊断计数合并零生成代码变化（门禁数字不变语义）

## Decision

**D1（语料优先修复，C2a）**：633 条「class 未声明」调查证伪「可归一化变体」假设（14 名 usages 多为 legacy style block 类，真未声明仅 45 名）→ 373 新设计类 + 21 伪态类迁入 design.omni（原值保真）、9 名变体归一、6 名 vestigial 删除、后代选择器 35 条改写保留 → 75 条。**样式意图保真纪律**：fix round 以 git-show 逐类 bit-exact 还原（container-type 恢复 cq 单位容器语义）。

**D2（映射/语义燃尽，C2b）**：event 包装（MouseRegion onEnter/onExit、FocusNode、errorBuilder、onSubmitted）、prop 映射（rows→maxLines、max→maxLength、disabled→enabled:false、data-testid→ValueKey、title→Tooltip）、bind:value onChanged 写回合并、Text 混合内容 → Text.rich、motion 逐轴 Matrix4 + `_OmniStepsCurve`（steps 语义）、inline text → TextStyle → 700。无等价项（touch/wheel/drop/media 族、tabindex/srcset/coords 等）**保持诚实降级**。

**D3（诊断层，C2d）**：ECodegen 警告 **source span 线程化 100%（671/671 真实 file:line，占位文件 "input.omni" 清零——motion 定义 span 补 lowering 1 行）**；#1/#2/#6 按 message（#6 加 file）程序级合并（first-occurrence 锚点保留）。锚点精确性由回归测试锁定（`learn_omniview_flutter_warning_anchors_are_exact`）。

**D4（#81 目标处置）**：671 为诚实下限——诊断合并后残余均为真实降级或语义缺口。**<300 唯一可达路径 = #2/#6 TextStyle/layout 语义实现（−429 → 242）**——**defer-with-trigger**：触发条件 = Flutter 产物文字样式成为实际使用场景（现语料降级无功能破坏，仅视觉）或 motion 域重设计任务。目标更新：1004→671（−33%）为燃尽期正式记录值。

**Architectural Components Affected:**
* `omnilang/src/codegen/flutter/`（view.rs/design_classes.rs/motion.rs/design_tokens.rs/lib.rs 发射与诊断）
* `crates/omnilang-core/src/ir/lower/motion.rs`（定义 span 线程化）
* `test/projects/learn-omniview/src/`（语料修复：design.omni +2897 行、59 文件迁移）

**Interface Changes:**
* ECodegen 警告新增 file:line（JSON 契约含 span 字段——已有，本次填充）
* warnings 门禁基线 700→671

## Consequences

### Positive

* 诊断可行动性质变：671/671 带真实位置（开发者/LLM eval 可直接定位）
* 语料与设计体系对齐：373 类入 design.omni 后 demo 样式真实生效（此前降级静默丢失）
* 静默错误类清零：steps 别名 linear 回退、ref+data-testid 双 key 不可编译、scaleX 自然边界 0.0——3 个 latent 缺陷随 C2b fix round 修复
* 门禁纪律：671 cap 单调下降轨迹可审计

### Negative

* 671 > 计划 <300 目标——目标调整为诚实记录（非掩盖，cap 契约维持）
* design.omni +2897 行膨胀（373 类）——结构负担换取样式语义真实化
* #2/#6 429 条降级警告在 Flutter 产物上意味着文字样式未应用（视觉降级，非功能）

### Neutral

* 程序级合并 = 呈现层变化，生成代码零漂移（dart analyze 0 issues 验证）
* 残留 196 条「其他」以诚实单例为主（variant cursor/font stack/多主题告知类）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——Flutter codegen 诊断与语料，React/OmniView 目标零行为变化（verify-react 零豁免漂移验证）

**Affected Components**:
- Flutter codegen 发射/诊断 - 映射表扩展与合并逻辑
- learn-omniview 语料 - design.omni/59 文件（跨目标共享，React 生成物随之更新）
- warnings 门禁测试 - 基线契约更新

**User Impact**: 低——生成 Flutter 产物编译干净（analyze 0 issues）；视觉降级（文字样式）为已知边界

**Risk Mitigation**:
- 每个映射/合并带判别性 fixture（pre-fix 必失败的断言串）
- span 锚点由精确回归测试锁定（防漂移静默）
- 样式意图保真由 git-show bit-exact 对照验证

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**:
- 语料迁移可逐 commit 回退（git revert 粒度清晰）
- 诊断合并/span 线程化独立于映射改动，可单点回退

**Migration Paths**:
- **Forward Migration**: TextStyle 语义任务（触发时立项——design class 文字属性 → TextStyle 组合发射）
- **Rollback Migration**: 各 commit 独立 revert，门禁 cap 同步回退
- **Evolution Path**: 诚实降级类 → 逐个语义实现（TextStyle 优先）

**Options Preserved**: TextStyle 语义、motion translateZ/width/height（#83 长期项）、per-widget 语义增强

### Sequencing & Timing

**Prerequisites**: C1 审计（类构成表）→ C2a 语料 → C2b 映射 → C2d 诊断——已全部完成

**Sequencing Concerns**: TextStyle 语义任务若立项，应排在 #83（motion-in-@for）之前（429 条 vs 4 条影响面）

### Social Cost

**Learning Curve**: Low——无新概念；诊断位置可行动性是减负

**Cognitive Load**: 低——合并语义（message/file 键）单点实现

**Documentation Needs**:
- [x] deferral #81 行状态更新（燃尽记录 + 触发条件）
- [x] 门禁测试注释（671 构成分解）
- [ ] TextStyle 语义任务立项时更新本 ADR 状态（Superseded 或注记）

## Alternatives Considered

* **纯语义/映射修复**（不做合并）：最佳 ~509——不可达 <300，且放弃合并的呈现层收益
* **per-file 诊断合并**：实测零收益（程序级合并已按 message 键饱和，同文件同类早已坍缩）
* **(message, file) 全族键控**：实测重开 217 条跨文件重复（671→888）——拒绝（合并粒度反噬），仅 #6 定义点采用（跨文件同名类各保锚点）
* **TextStyle 语义任务**（#2/#6 文字属性 → TextStyle 发射）：−429 → 242 可达 <300——M-L 工作量，defer-with-trigger（见 D4）
* **删除降级警告**：回到静默——违反 v3 核心原则，拒绝

## Validation

* 门禁轨迹：1004→997→991→878→874→753→700→671（cap 契约同步，单调下降）
* span 覆盖：671/671 真实 file:line、0 占位文件（独立 harness 复现 + 锚点精确测试）
* 生成产物：dart analyze 0 issues、flutter 23 套件全绿、verify-react 零豁免漂移、clippy 0、analyse 0 err
* 判别性：新增 12+ fixtures 均 pre-fix 必失败；C2a 样式还原 git-show bit-exact

## Pragmatic Enforcer Analysis

**TextStyle 语义任务（D4 备选路径）评估**：
- **Necessity (4/10)**：429 条降级为视觉缺口（Flutter 文字样式未应用），无功能破坏、零运行时错误；现语料降级可接受（demo 教学用途）
- **Complexity (7/10)**：design class 属性分类（BoxDecoration vs TextStyle）+ 组合发射 + 继承/覆盖语义——M-L，触及 Flutter 样式体系核心
- **Alternative Analysis**：已评估「接受 671 + 触发式立项」——成本为零，收益保留全部选项
- **Simpler Alternative Proposal**：保持诚实降级 + span 可行动诊断（671 现状）——满足「可诊断、不静默、可演进」三性
- **Recommendation**: Defer（触发条件见 D4）；若立项，先做文字属性映射子集（color/fontSize/fontWeight——覆盖 16 唯一类中的 13 个）
- **Pragmatic Score**: Necessity 4, Complexity 7, Ratio 1.75——超 1.5 阈值，支持 Defer

## References

* deferral #81（.architecture/deferrals.md:337）——燃尽处置记录
* C1 审计：`.superpowers/sdd/2026-08-03-semantic-syntax-v3-fixes/task-leC1-audit.md`（15 类构成表）
* C2a/C2b/C2d 报告与 fix rounds（同目录 task-leC2a/b/d-report.md）
* ADR-062（v3 fix-round 决策——本 ADR 为其 deferral #81 的燃尽处置）
* ADR-052/053（ECodegen 降级语义来源）
