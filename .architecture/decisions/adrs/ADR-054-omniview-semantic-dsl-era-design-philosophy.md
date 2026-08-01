# ADR-054: OmniView 语义 DSL 时代 — 设计理念与实现完成

## Status

Accepted

## Context

ADR-052（三层语义架构 + 24-item feature floor）与 ADR-053（BNF 语法规范化 + 7 项决策）定义了 OmniLang 语义级 DSL 的目标形态。2026-08-01 完成了 35-task 实现计划（Phase 0-10），随后进行了三方覆盖差距分析（功能全集 × 展示项目 × 文档），并完成了 12 个覆盖差距修复任务与 6 个编译器真实 bug 修复。

至此，OmniView 从「OmniView-first 调试运行时 + 平台形状 API」时代进入「语义级 DSL」时代：**语言、编译器、运行时、展示项目、文档五方对齐**。本 ADR 记录这一时代的完整设计理念，并标记被取代的过往设计，作为后续评审与开发的统一参照。

关键背景事实（2026-08-01 验证）：

1. **35/35 实现计划完成**：parser（9 语法域）→ IR（RefOpIR/Slot/Motion 重设计）→ semantic（validate_ref_ops/lower_inline_motions/semantic bindings）→ React（Zustand v5/动画重写/slot D5）→ Flutter（Riverpod 3.0/TweenSequence）→ OmniView（@focus 接线/slot fallback）→ CI（tsc + dart analyze 门禁）→ demo 迁移 → governance。49 commits 合并 main。
2. **覆盖差距分析**（3 探索 agent）发现：27 个孤儿 demo 文件承载的功能在应用中不可见；13 项功能无任何 demo；文档 4 处陈旧（含 15-grammar-view 教授错误语法）。
3. **12 个覆盖差距修复任务**：文档对齐（P0）、demo 修复（P1：Todo 流程/Play All/杂项）、孤儿接线（P2：17 文件）、功能补全（P3：refs 5 项/typed slots/match 广度）。
4. **6 个编译器真实 bug 修复**（浏览器/运行时验证驱动）：`duration:"500ms"` 字符串导致整个应用 bootstrap 崩溃（P0）；match literal arm `(0)._variant === 0` 永不匹配；OmniView ref wrapper（releasePointer 原生 API 名错、scrollTo options 参数 NaN）；view 文本中 throw/try/catch/finally 关键字破坏；UndefinedRef lint 对 inline store 误报；std 模块（async/datetime/encode/regex/math 扩展）缺失注册导致 ReferenceError。

## Decision Drivers

* **语义意图优先于平台形状**：语言表达「做什么」（focus、拖拽、翻译），平台映射（DOM/Flutter/React）是 codegen 层的职责——ADR-052 的核心动因，经实现验证成立
* **跨目标行为对等 + 硬诊断**：ADR-021 重定义——24-item floor 内行为对等，floor 外硬诊断，绝不静默降级
* **消除「静默错误」这一类雷**：本时代所有修复的共同主线——无法实现或已删除的功能必须显式诊断（编译错误/警告/生成代码注释/文档如实记录），而不是悄悄失效
* **运行时正确性由真实执行验证**：字符串断言无法发现 `el.animate` 拒绝字符串 duration 或 `(0)._variant === undefined` 这类运行时问题——浏览器验证（Playwright）与 node 执行测试成为必要手段
* **文档与展示项目是语言的一部分**：语法规范（BNF）、用户文档、展示项目三方必须与编译器事实一致——文档教错误语法与孤儿 demo 一样是「雷」

## Decision

### 设计理念（5 原则）

1. **语义原语优先（Semantic Primitives）**：语言暴露平台无关的语义原语——8 触发器（`@enter/@hover/@press/@focus/@in-view/@scroll/@drag-horizontal/@drag-vertical`）、17 属性动画白名单（opacity/scale*/translate*/rotate*/skew*/backgroundColor/color/width/height + `--*` 透传）、11 个 RefOp（focus/blur/scrollTo/scrollBy/select/setSelectionRange/measure/capturePointer/releasePointer/value）、4 个语义绑定（focus/scroll-x/scroll-y/value）、body 级 `@slot` 声明 + 命名 props 消费。平台映射全部收敛在 codegen 层。

2. **跨目标对等 floor（ADR-021 重定义）**：24-item floor 内的功能在三 P0 后端（OmniView/React/Flutter）产生行为等价的输出；floor 外不支持的功能产生**硬诊断**（编译错误/警告），而非静默降级。已实现的诊断机制：`@hero` 显式错误 + 迁移提示、slot 转发 E0006 警告（含跨文件）、store get/set 塌缩守卫、非字面量 keyframe extent → None + 警告。

3. **诚实降级（Honest Degradation）**：当某功能在特定后端无法表达（如 `@if mode:"wait"` 无离场动画机制、Flutter fade 无过渡），生成代码必须携带说明注释（`// @if mode: "wait" — no leave animation exists; degrades to an instant swap`），文档表格如实记录每后端的支持状态。**「雷」的定义 = 静默错误**——本时代所有修复均以此为标准。

4. **运行时正确性由真实执行验证（Runtime-Verified Correctness）**：编译器测试体系新增三层：CI 工具链门禁（`tsc --noEmit` 12 fixtures + `dart analyze` 全生成文件，优雅 SKIP）、node 执行测试（js_runtime_tests：match literal arms、std 模块全函数、ref ops）、浏览器冒烟（Playwright：9 标签页渲染、动画运行、scrollTo 实际滚动、指针捕获状态机）。字符串断言仅作为第一层。

5. **文档与展示对齐（Docs & Showcase Parity）**：功能全集 × 展示项目 × 文档三方对齐——27 个孤儿 demo 接线（emit/表单/CSS units/vars/错误处理/std 库/Dashboard）、13 项功能补 demo、文档逐句对照 parser 审计（15-grammar-view 的错误语法教学已修正）、语义绑定专节补写。展示项目（learn-omniview，9 标签页）与语法规范（BNF）共同成为语言的「可执行文档」。

### 实现状态（2026-08-01）

| 域 | 状态 |
|---|---|
| 语法（9 域 BNF） | ✅ 35/35 计划完成，BNF spec 与 parser 逐项一致（12 项分歧已同步） |
| 三 P0 后端 | ✅ React/Flutter/OmniView 全部通过 tsc/dart analyze/浏览器门禁 |
| 展示项目 | ✅ 9 标签页全功能展示（孤儿全接线，仅 Map.set 等 5 项记录 deferral） |
| 文档 | ✅ 16 篇 + README 全部与编译器事实一致 |
| CI | ✅ codegen-validate job（tsc + dart analyze）+ 全量 workspace 测试 |

### 被取代的过往设计（Superseded）

| ADR | 原状态 | 新状态 | 取代理由 |
|---|---|---|---|
| ADR-036 Element Reference System | Accepted | **Superseded**（by ADR-052/053） | 旧 ref 机制（`ref.style.*`/`getBoundingClientRect`/`offsetWidth` 等 DOM 形状 API）被 RefOpIR 11 语义 ops + RefBinding（focus/scroll-x/y/value）+ `style:{...}` 绑定取代；非法 API 现在产生带迁移提示的编译错误 |
| ADR-038 Refined Element Reference System (v2) | Accepted (Phase 2 Deferred) | **Superseded**（by ADR-052/053） | v2 细化的缓存修正/@for 诊断等精神保留，但 ref 的语义形态被 ADR-052 全面重定义为语义 ops；Phase 2 内容不再适用 |
| ADR-032 Animation Design Scoping | Accepted | **Superseded**（by ADR-052/053） | 动画范围界定（33→14 功能）被 17 属性白名单 + 8 触发器（@focus 回归）+ 4 播放属性（paused 删除）+ steps 位置参数取代；其「范围界定方法论」仍作历史参考 |
| ADR-024 React Motion useEffect Lifecycle | Implemented | **Superseded**（by ADR-052） | React 动画重写（native refs + WAAPI、8 触发器、无 DOM 扫描）取代了 useEffect 依赖数组机制；其「避免依赖膨胀」的意图保留在新实现中 |

未标记的过往 ADR（仍有效）：ADR-042（`@ref={name}` 语法保留）、ADR-043/044（ClassMap/主题同步仍适用）、ADR-046（value 语义保留为合法 RefOp）、ADR-047/049/050/051（design token 管线保留）、ADR-020/022/023（Riverpod/缓动表/视图遍历仍有效）、ADR-018/019（事件系统架构保留）、ADR-027/028/029/031（安全/调试/分类框架保留）、ADR-034/035/039/040/041（组件隔离/IR 清理/安全对等/运行时加固保留）、ADR-045（learn-omniview 重构——本时代继续沿用其结构）、ADR-016/017（主题系统/验证升级保留）。

**Architectural Components Affected:**
* 语言语法（parser/AST/BNF spec）
* IR（RefOpIR/Slot/Motion/语义绑定）
* 三 P0 后端 codegen
* 运行时（OmniView inline runtime / omni-animation）
* 测试体系（CI 门禁 + node 执行 + 浏览器）
* 展示项目（learn-omniview）与文档（docs/omniview + BNF spec）

**Interface Changes:**
* 语言面：`over:`/`easing:` 冒号语法、kebab 触发器、17 属性白名单、body 级 `@slot`、命名 props、语义绑定、`mode:` 冒号——均为破坏性变更（旧语法迁移完成，见 Phase 9）
* 诊断面：非法 ref API/`@hero`/slot 转发/arity 错误均产生显式诊断

## Consequences

### Positive

* 语言表达意图而非平台形状——新功能（如 typed slots、语义绑定）三后端一致实现
* 静默错误类缺陷归零：本时代修复的 6 个编译器 bug 全部是「静默/崩溃」类，修复后均有显式诊断或正确行为
* 文档与展示项目可信：16 篇文档 + 9 标签页与编译器事实一致，可作为语言规范的可执行参照
* CI 门禁在真实工具链上验证生成代码——tsc/dart analyze 已捕获 2 个真实 codegen bug（TS2657 多根、Flutter 6 编译错误）

### Negative

* 破坏性语法变更的历史负担：旧语法（`over 300 ease X`、`@inView`、`@paused`、`ref.style.*`、`slot="name"` 消费）在外部代码中需要迁移；golden 镜像需持续同步
* 过往 ADR 链变长：ADR-024/032/036/038 标记 Superseded 后，历史决策需沿 Superseded 链追溯（本 ADR 提供一站式映射）
* Flutter 目标仍有残余 gap（deferral #28/#30：Map.set 无 codegen 支持、match 变体臂 + 本地类型发射缺失）——docs 已如实记录，非静默

### Neutral

* ADR 文档维护成本随时代演进增长（52→54 篇）
* 展示项目成为「可执行文档」后，demo 修改需要经过与编译器相同的验证纪律

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——本 ADR 是治理/文档决策（记录已完成状态 + 标记过往），不引入新代码变更；其影响在于后续评审与开发以此为设计基线。

**Affected Components**:
- 评审流程（`.architecture/reviews/`）——后续评审以本 ADR 的 5 原则为判定标准
- deferrals 追踪（`.architecture/deferrals.md` #28-#32）——作为本时代的已知残余
- 过往 ADR 阅读者——通过 Superseded 标记导航到当前设计

**User Impact**: 无运行时影响。

**Risk Mitigation**:
- 5 原则均有实现证据（本会话 commits），非空想宣言
- 被标记 ADR 保留原文（仅状态行变更 + Superseded 链接），历史可追溯

### Reversibility

**Reversibility Level**: High——ADR 状态可随时回改；设计原则若被后续实践否定，可经新 ADR 修订。

## Pragmatic Enforcer Analysis

- **Necessity Assessment (2/10)**：本 ADR 不引入新决策，而是记录已完成状态 + 标记过往——治理需求（评审基线统一、历史导航）真实但非紧迫；代价低（文档），等待成本低。
- **Complexity Assessment (1/10)**：纯文档，无代码/架构复杂度增加；5 原则全部来自已实现的实践。
- **Alternative Analysis**：备选方案「不创建 ADR，仅在记忆中记录」——评审与后续会话将失去可引用的设计基线（本会话记忆文件仅个人可读）；「拆成多个 ADR」——5 原则是一个整体设计哲学，拆散会失去关联性。本 ADR 单一文档承载是合理的。
- **Simpler Alternative Proposal**：ADR 内容可更短（仅状态标记），但设计理念记录是用户明确要求（「把设计理念详细记录」）——5 原则的详细记录具有实际复用价值（后续评审判定标准）。
- **Recommendation**: Approve with simplifications——设计理念记录 + 状态标记为核心，实现状态表格可随时代演进更新而非重写。
- **Pragmatic Score**: Necessity 2, Complexity 1, Ratio 0.5（target <1.5 ✅）
- **Overall Assessment**: Appropriate governance documentation——记录已完成的设计基线而非投机性前瞻，符合 YAGNI。

## Alternatives Considered

1. **不创建 ADR，仅更新记忆**——拒绝：评审与后续会话需要仓库内可引用的设计基线；记忆文件仅个人可读。
2. **仅标记过往 ADR，不记录理念**——拒绝：用户明确要求「把设计理念详细记录」；5 原则是后续评审的判定标准。
3. **拆分为多个 ADR（理念/标记/实现状态）**——拒绝：三者是同一时代的一体记录，拆分破坏关联性。
4. **仅更新 ADR-052/053 而非新建**——拒绝：052/053 是设计决策文档，本 ADR 是「实现完成 + 设计哲学固化」的里程碑文档，职责不同。

## Validation

- 5 原则逐一对应本会话实现证据（commits `4bdb5da2` 合并后 38 commits）
- 被标记 ADR 的取代理由逐一对应 ADR-052/053 的删除/重定义条款
- 测试基线：`cargo test --workspace` 27 binaries 全绿；tsc 12 fixtures + dart analyze 全生成文件通过；node 执行测试 4/4；浏览器 9 标签页渲染
- 文档一致性：16 篇 docs + README + BNF spec 与 parser 逐项一致（审计 + 修正 commits）

## References

- ADR-052（三层语义架构 + 24 floor）、ADR-053（BNF 规范化 + 7 决策）
- ADR-021（已重定义为 floor 对等 + 硬诊断）
- `.architecture/deferrals.md` #28-#32（本时代已知残余：Map.set、match Flutter 变体臂/本地类型、裸 `\` lexer、Visibility 过滤、__anon_m_N 碰撞守卫）
- 本会话记录：`memory/session-2026-08-02-semantic-dsl-completion.md`（35/35 + 覆盖差距 + 编译器修复全记录）
