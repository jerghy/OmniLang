# Architectural Review: OmniLang 编译器（语义级 DSL 时代）

## Overview

本评审针对 OmniLang 编译器在**语义级 DSL 时代**的架构：语言已从「OmniView-first 调试运行时 + 平台形状 API」重构为「语义原语 → IR → codegen」三层架构（ADR-052/053/054），当前正处于能力基线执行期（14 维能力矩阵 × 5 批次 × 58 任务，批次 1 反馈/模态进行中）。核心评审问题：**当前架构是否需要重构？** 评审团队同时被明确提示：部分发现可能是有意设计（中文诊断消息、ADR-028 OmniView 调试运行时取舍、语义全集登记纪律、floor 外硬诊断、per-target 诚实降级等），发现均先与 ADR/CONTEXT.md 交叉核对再定性。

> **实证复核注记（2026-08-10）**：评审完成后，主 agent 对全部关键发现逐项亲自验证（读代码 + 构造最小复现实证），结果见 §「发现验证结果」。1 项误报（React ToastProvider——已修复于评审基线之前，两子代理读入过时内容）；其余全部确认。

## Review Details

- **评审对象**: OmniLang 编译器（crates/omnilang-core + omnilang codegen 10 目标）
- **评审时间**: 2026-08-10
- **评审团队**: 11 名成员（compiler-architect / domain-expert / rust-expert / systems-architect / maintainability-expert / performance-specialist / implementation-strategist / pragmatic-enforcer / security-specialist / typescript-react-specialist / ai-engineer）
- **评审方法**: 并行子代理独立评审 → 交叉主题聚合 → 冲突显式化 → 优先级分桶
- **基线**: HEAD 828347f7（批次 1 Task 6 完成，Task 7 收尾中；5 个 .snap.new 待接受）

## 一致性声明（有意设计 vs 缺陷）

所有成员被明确要求：报告缺陷前先核对 ADR/CONTEXT.md。确认的**有意设计**（非缺陷）包括：中文诊断消息政策（ADR-056 决策 1）、OmniView 调试运行时非性能取向（ADR-028 14 项取舍 + ADR-037/038）、语义全集登记先行纪律与脚本双向断言（ADR-074，用户既定规则）、floor 外硬诊断与诚实降级注释（ADR-021 重定义 / ADR-054 原则 3）、透传作为刻意类别（40 个透传事件，ADR-075 取向 B）、`data-*`/`aria-*` 透传静默（ADR-074 有意设计）、机制级条目存在性断言例外（ADR-074:43）、`#[non_exhaustive]` 扩展策略（ADR-040）、validate_form_load 的 lowering 标记丢弃权衡（validate_form_load.rs:1-7 明示）。

---

## Executive Summary

**总体架构健康度**: **良好（Good）**——核心架构不需要重构；需要通过一次「批次 2 前收口」（consolidation commit）修复编排层的结构性弱点与若干真实缺陷。

**关键结论（11 名成员一致）**: 语义级 DSL 三层架构（语义墙 → 意图 IR → 地道 codegen）在批次 1 的 @modal/@skeleton/@errorBoundary/showToast 实证中成立，意图 IR 模式（ModalIntentIR/FormIntentIR/LoadIntentIR）在多目标下运转良好；批次 2-5 的 5 个新域是既有模式的垂直切片（semantic 模块 + 3 个调用点 + 指令解析函数 + 加法式 serde-safe IR 字段），**不需要架构重构**——domain-expert 补充：登记册机器域无关、可 mid-batch 吸收新域（域 10 落地不破 1-9），混合分类法与意图异质性是可文档化/可标准化的成熟成本而非模型错误。但**pass 编排层**存在结构性弱点，且已在产出错误输出（1 个 HIGH 实证缺陷），需要在批次 2 之前修复。

**关键优势**:
- 三层架构（语义原语 → IR → codegen）文档化、单一编排点（run_ir_passes）、floor 门禁而非 codegen 重验证——新能力维是垂直切片，不威胁分解
- 诚实降级纪律系统化：每个已知缺口要么硬诊断、要么警告、要么登记账本；「零诊断静默退化」被明确定义为缺陷类
- 验证网是连贯系统而非脚本堆：855 元组注入语料 + 双向断言 + 豁免账本（陈旧即失败）+ 浏览器套件
- 单一来源转义架构（shared sanitize.rs/json.rs）+ 失败关闭消费者；React 目标 hook 导入门控纪律（每个发射条件镜像到 import 列表）
- 缓存设计正确（内容哈希 + 传递失效），仅尺寸参数错误

**关键关切**（详见优先化清单；标注 ⚙ = 已实证复核确认，❌ = 复核为误报）:
- **HIGH-1 ⚙（唯一「今天产出错误输出」的缺陷）**: 跨文件 required-prop 校验静默跳过——`validate_required_props` 在 per-file 层运行（analyse.rs:102 → ir::lower 内 run_ir_passes），跨文件缺失 required prop 编译零诊断（实证：a.omni import b.omni 缺 title → Compiled 4 file(s) 零诊断；同文件对照 → E0011 正确报错）。违反项目自己的「绝不静默降级」纪律
- **HIGH-2 ❌（误报——已修复）**: React `ToastProvider` 吞视图——子代理声称零参数发射不渲染 {children}；实证复核：提交 6271d5b6（早于评审基线 HEAD 828347f7）已修复，HEAD 中为 `function ToastProvider({ children }: { children: ReactNode })` 且显式渲染 `{children}`（:614）；FIXTURE_STRICT_ALLOWANCE 亦无子代理引用的「TS2559 cap=1 条目」（仅 3 个 TS7006 条目）。两子代理（TS/React + implementation-strategist）读入过时内容。无需行动
- **HIGH-3 ⚙（4 名成员独立确认）**: 程序级 pass 序列三处手写复制（analyse_multi:497-520 / analyse_multi_cached:789-812 / run_pipeline:lib.rs:113-155，注释逐字相同），批次 2-5 每个新程序级 pass 是 3 处编辑；历史上 validate_event_names 曾因漏一处而单文件路径缺失诊断（lib.rs:120-126 自述）
- **MEDIUM 群**（⚙ 均已实证确认）: URL 消毒器 C0 控制字符绕过（`\u{1}javascript:` 当前可达——lexer read_unicode_escape 解码 U+0001 + 两消毒器只剥 \n\r\t + 浏览器剥前导 C0 后执行）；`hidden={x}` 仍是 B2 bug 类活跃实例（parser 15 关键字含 hidden/autoplay vs codegen HTML_BOOLEAN_ATTRS 13 项无 hidden，白名单亦无 → data-hidden fallback）；方法翻译三表漂移（`contains()` React 含字符串 includes 分支 vs OmniView 无——React 注释自认「with the string branch preserved」；deferrals #76/#28/#88 均开）；意图 IR 无 span（FormIntentIR:1026/LoadIntentIR:1046/ModalIntentIR:734 定义均无 span 字段，域 11 @route 将继承组件级 span 退化）；编辑器画布 success=false on warnings（omnilang/src/lib.rs:185/217 vs :464 errors-only）；MAX_CACHE_ENTRIES=50 低于旗舰项目 102 文件（analyse.rs:238/:711，实测 15.6× 回归）

---

## Individual Perspectives（逐成员评审原文）

### Compiler Architect Review（Dr. Lin Wei）

**Perspective**: The three-tier model (semantic intent → IR → codegen, ADR-052) is sound and batch 1 empirically validated it — but the **pass-orchestration layer** built on top of it has structural weaknesses that will compound across batches 2–5 (14-dimension capability matrix, ~5 new intent domains), and one of them is already producing silently-wrong output today.

**Key Observations**:
1. **The pipeline is not one pipeline — it is four hand-replicated orchestrations.** The intra-file IR pass sequence (`run_ir_passes`, 9 passes) is centralized in `analyse.rs:100-185` as claimed by the lower/mod.rs:341-344 comment, but the program-level sequence (optimize → validate_theme → validate_event_names → validate_form_load → validate_feedback_modal → lints) is hand-written **three times**: `analyse_multi` (analyse.rs:497-520), `analyse_multi_cached` (analyse.rs:789-812), and `run_pipeline` (lib.rs:113-155). Phase naming has already degenerated ("3f", then "3f2"). Every batch 2–5 program-level pass must be inserted in three places with three copies of the ordering rationale.
2. **The per-file vs program-level pass split is a manual decision with a proven correctness hole.** All 9 intra-file passes were designed as intra-component (analyse.rs:78-94), but `validate_required_props` is inherently cross-file (usage sites vs declarations) and was left per-file.
3. **Element intent markers vanish during lowering, forcing a diagnostic split-brain that will repeat per domain.** `@form/@load` markers are stripped from the IR view tree (view.rs), so element-anchored checks (V-F1/V-F6) fire inside lowering while the rest (V-F2..F8) run in a program pass — with V-F1's message text duplicated at component.rs:357-362 and 486-493.
4. **ElementIR is becoming a kitchen sink of optional binding fields.** Batch 1 added modal/skeleton/error_boundary bindings; batch 2 adds route, batch 3 breakpoints, batch 5 i18n/lifecycle. Non-P0 targets (vue/dioxus) silently ignore every one of them.
5. **Secondary targets are empirically unreachable from the shipped CLI** (omnic features = 3 P0 backends only; `omnic build -t vue` hard-errors "Backend for 'Vue' is not enabled") — so silent-drop is latent, not live, on shipped paths. The project already tracks this defect class ("静默跳过缺陷修复（🔨 批次 3，诊断先行）", `semantic-universe.md:38`).

**Strengths**:
1. **Intent IR pattern is working.** `ModalIntentIR`/`FormIntentIR`/`LoadIntentIR` cleanly separate "what the user wants" from per-target mechanics; `@modal` compiles to self-drawn overlay + focus trap (omniview), controlled `<dialog>` (react), and `showDialog` with `_modalShown` idempotence (flutter) from one IR node — the model holds under 10+ domains.
2. **Per-file parallel lowering + sequential merge is sound for intra-component checks** (7/9 passes), and the cache design is careful: program passes mutate merged IR but the cache holds pre-mutation IR cloned fresh per run (analyse.rs:766-771); diagnostics replay on cache hits is regression-tested (analyse.rs:845-927).
3. **The lowering/pass split for @form/@load is honestly documented** (validate_form_load.rs:1-7 explains the marker-drop constraint) — an intentional tradeoff, not an oversight.
4. **Hard-diagnostic discipline is real on P0 paths**: `DESIGN_BLOCK_ERROR` gates non-P0 design blocks (codegen/mod.rs:69-72), array-method floor gates fire `E_CODEGEN` (codegen/mod.rs:112), and `in_execution_body` context-flagging (component.rs:59-75) keeps `showToast` scoping precise. The same-file required-prop check fires correctly with a precise span.
5. **Registry-first governance**: the semantic-universe ledger with bidirectional script assertions (`verify-semantic-universe.py`) catches drift at CI time — exactly the right instrument for the batch 2–5 influx.

**Concerns**:
- **1. HIGH — Cross-file required-prop validation silently skipped (empirically confirmed)**. Location: `crates/omnilang-core/src/analyse.rs:102` (`validate_required_props` inside per-file `run_ir_passes`, called from `ir::lower` at `crates/omnilang-core/src/ir/lower/mod.rs:345`) → `crates/omnilang-core/src/semantic/validate_props.rs:18-29` (component_map built from the *per-file* `iter_all_components`). Proof: `b.omni` declaring `component Button { prop title: String; ... }` + `a.omni` with `import b; ... <Button />` compiles with exit 0 and **no diagnostic**; the identical source in one file correctly emits E0011. Fix (S): move the call to a program-level phase after Phase 3b merge.
- **2. MEDIUM — Program-level pass sequence triplicated across three orchestrations**. Location: analyse.rs:497-520, :789-812, lib.rs:113-155. Today order-consistent (verified optimize → theme → event_names → form_load → feedback_modal → lints in all three) — future-change hazard, not current wrong output. Fix (S/M): extract `run_program_passes`.
- **3. MEDIUM — Intent markers dropped at lowering → validation split-brain, repeated per domain**. V-F1 text duplicated at component.rs:357-362 vs :486-493. Fix (S/M): carry `source_span` on intent IR nodes, keep markers on IR view node, move V-F1/V-F6 into domain pass.
- **4. MEDIUM — Intent-binding fields silently ignored on vue/dioxus targets**. Location: vue/template.rs:33-150 (`.modal`/`.skeleton`/`.error_boundary`/`.a11y`/`.drag_binding`/`.motion`/`.ref_binding` ignored; slot fallback dropped at :317-322). Latent today (omnic ships P0 only), but `--all-features` silently produces incomplete output. Fix (M): per-target E_CODEGEN gate keyed on populated intent fields; ship as part of batch 3.
- **5. LOW — Modal heading classification triplicated across three modules**. `MODAL_HEADING_TAGS` + `subtree_has_heading` in omniview/view/modal.rs:32-69, mirrored in react/view.rs and validate_feedback_modal.rs (V13). Fix (L): hoist into shared util.

**Recommendations**: Immediate — move `validate_required_props` to program-level (S, ~1 day); extract `run_program_passes` (S, ~1 day). Short-term — add source_span to FormIntentIR/LoadIntentIR/ModalIntentIR, fold V-F1/V-F6 into domain passes (M, ~1 week); batch-3 E_CODEGEN gates for intent fields on vue/dioxus (M, ~3-4 days). Long-term — consolidate ElementIR per-domain bindings as fields accumulate (L, ~2-3 weeks, only when batch 3 lands).

**Risks**: every new program-level pass (routing validation is inherently cross-file) triplicates before batch 2; cross-file validation hole class likely recurs in routing/i18n domains; silent-drop surface on non-P0 grows per field; split-brain pattern reappears per domain unless intent IRs carry spans.

**Verdict**: **No — the compiler does not need restructuring at the IR/codegen level. The pass-orchestration layer does need restructuring, and it should happen before batch 2**: (a) one declared program-pass sequence called by all three orchestrators, (b) per-pass scope metadata so placement becomes data rather than a correctness-sensitive manual decision, (c) the immediate placement fix for `validate_required_props` — the one finding with demonstrated wrong output today.

### Domain Expert Review

**Perspective**: 评审的「域」不是业务域而是语言本身：OmniLang 的语义全集就是**前端功能域**的域模型。问题是该模型——9 个已登记域 + 4 个预告域（域 10-13）、per-domain 登记纪律、意图 IR 本体——能否承载剩余能力维（导航、响应式、i18n、媒体、交互、长尾）而无需重构。评估结论：登记册机器可扩展；**分类原则**与**意图挂载模式**是先显压力的两处，且都有低成本修正可立即应用。

**Key Observations**:
1. **登记册在一个编号方案下有两个分类原则**。域 1-7 按语法类别命名（可动画属性/数组操作/事件/RefOp/token 类型/值绑定/指令——门禁脚本自身枚举 verify-semantic-universe.py:6-8）；域 8+ 按*特性*命名（表单/数据三态/反馈模态/导航/响应式/数据深化，baseline:132）。特性名域切过语法角色：域 10 的已实现白名单（batch1 spec:261-264）列三个异质事实源——元素解析透传（dialog）、内建函数表（showToast）、元素指令语义化（@modal/@skeleton/@errorBoundary）——归一个域。镜像已文档化的全集域 vs 设计文档域映射问题（CONTEXT.md:146-158），但现在两套分类法在同一登记册内部分相撞。
2. **意图挂载在三个不同 IR 层级、诊断保真度不同**。元素级：ElementIR.drag_binding 与 ElementIR.modal（ir/types.rs:816,822）带元素 span。组件级：ComponentIR.form_intent/load_intent（types.rs:1086,1089）不带——校验 pass 显式文档化「本 pass 无法定位具体元素，只能消费已收集的意图 IR（诊断 span 回退组件级）」（validate_form_load.rs:4-7），V-F1/V-F6 被推入 lowering 因 @form/@load 标记从视图树消失。表达式级：ExprIR::BuiltinToastCall（types.rs:255）——第三种形状，表达式内节点。本体*语义上*连贯（每意图 1:1 镜像 spec）但*结构上*异质。
3. **一个域的校验跨两层、分工随 pass 涌现**。批次 1 的 V1-V14 表部分在 lowering 执行（V4/V11 重复绑定 last-wins——validate_feedback_modal.rs:11-13）、部分在全程序语义 pass（V1/V2/V3/V6/V7/V8，V8 扫 6 个声明点 :36-64）。form/load 同样分裂。每新域必须重新发现其规则归属。
4. **内建语义函数是第三种词汇类别、以螺栓式增长**。showToast（及计划 navigate/back/bp）既非指令也非绑定——新语法类别（CONTEXT.md:64），其保留名强制（V8）需每函数手写扫 6 个声明点（validate_feedback_modal.rs:36-64）。每个未来内建函数复制此机器。
5. **两种状态词汇并存**。能力矩阵用 ✅/🔜/⏳/🏞（决策状态，baseline:20）；登记册用 支持/部分/未支持 + 新 留生态（verify-semantic-universe.py:14，baseline §8）。不同概念（路线图决策 vs 实现状态）经 §8 工作流桥接，但桥是隐式的。

**Strengths**:
1. **「语义表面永不漂移」不变量是承重设计**。verify-semantic-universe.py 把每域钉到编译器 ground truth（SEMANTIC_ANIM_PROPS/ARRAY_METHODS/event_names.rs/RefOpIR/parse_token_type/element.rs/VALIDATE_RULES/存在性锚点 :25-51），「未支持条目留在白名单外」（:17-20）。正确的 DDD 动作：登记册建模*特性空间*、白名单建模*已实现表面*、门禁防两者漂移。
2. **分类法扩展方向正确**。ADR-074 机制级存在性断言豁免（ADR-074:43）与 留生态 状态（baseline §8）正确容纳机制级与裁决出局成员而不强造假精度——两层模型所需。
3. **语义化即切换（ADR-075 原则 8）保持词汇单一来源**。旧名硬错误 + 迁移提示，无双轨。语言表面的有界上下文纪律，消除整类术语漂移 bug。
4. **Per-target 锚点纪律带诚实缺口**。域 10 锚点集按目标指名编译器事实源，公开记录「Flutter 锚点暂缺，Task 5 待落地」（verify-semantic-universe.py:107-108）而非断言不存在的覆盖。门禁 per-domain 增量，故域 10 可 mid-batch 落地不破 1-9。
5. **P3 机制干净分离决策状态与实现状态**（语法只随语义集登记变化；进度只触状态列）——14 维矩阵与登记册以不同速度演进而不互相矛盾。

**Concerns**:
- **MEDIUM — 意图挂载层级不一致，组件级模式会在域 11 以更糟后果重复**。证据：ir/types.rs:816,822（元素级 drag/modal，span 忠实）vs types.rs:1086,1089（组件级 form/load，span 回退文档化 validate_form_load.rs:4-7）vs types.rs:255（表达式级 toast）。form/load 标记 lowering 时从视图树消失，V-F1/V-F6 被逼进 ir/lower/component.rs，诊断退化为组件级 span。**为何要紧**：`@route "/todos/:id"`（域 11 导航）是与 @form/@load 同性质的组件级意图——页面声明非元素属性。域 13 数据深化扩展 @load 分页/轮询/Reset，全部组件级。每个将继承 span 退化 + 两层校验模式。元素级意图免费得精确 span；组件级不得；差异从 spec 的 Validation 表不可见（读起来所有规则平等）。**修复**：域 11 落地前标准化组件级意图模式——lowering 收集 `(intent, source_span)` 对进单一 `ComponentIR.intents` 集合供语义 pass 消费，而非每 pass 重新推导其规则归属。一个 pass 拥有「哪些规则 lowering 时 vs pass 时」；命名约定文档化（如「结构性规则在 lowering、语义规则在 passes」）替代 per-pass 注释。M。
- **MEDIUM — 混合域分类法无文档化放置规则；批次 5 需要 4+ 个未分配域**。证据：verify-semantic-universe.py:6-8（「Nine semantic domains (animatable properties / array operations / ... / directives / form validation / async data three-states)」——语法类别）vs baseline:132（域 10 反馈模态/11 导航/12 响应式/13 数据深化——特性）。门禁脚本自身 docstring 已陈旧（「Nine」而域 10 锚点已加 :107-118）。批次 5「长尾四归组（互相独立，可并行）」（:96-100）——交互/i18n/媒体/生命周期四个特性域一批，无预分配编号。**为何要紧**：放置按批次 spec 临时决定。i18n 落地时 @t/@locale 归域 14 还是加入域 12 响应式（两者都是「环境条件」特性）？tooltip 重评时（feedback spec §6「批次 5 交互归组评估」）归反馈还是交互？无规则可判，唯一可能承载规则的文档（CONTEXT.md:138-159 用词约定）早于特性域。对照表（CONTEXT.md:146-158）需为 13-17 域修订。**修复**：两个便宜动作无需重构：(a) 文档化分类规则——「机制级域按语法类别、功能设计级域按特性；功能域成员可跨语法类别，白名单按编译器事实源分列」（批次 1 spec §5.1 已隐式做）；(b) 每批次登记时更新 verify-semantic-universe.py:6-8 docstring 与 CONTEXT.md 对照表，并在 baseline 预分配批次 5 的域 14-17 使路线图与登记册同步。M。
- **MEDIUM — 内建语义函数以螺栓式保留名机器增长**。证据：V8 扫 6 声明点（validate_feedback_modal.rs:36-64）是 showToast 专属代码。navigate/back（域 11）与 bp（域 12）各需同样扫描，或新全程序 pass/域，或同 pass 复制增长。**为何要紧**：保留名强制是*类别级*不变量（内建语义函数名不得用作声明名），却以*成员级*检查实现。三内建三域 → 三个同样 6 点扫描副本带略微不同消息文本。代码中已可见重复（非仅设计）。**修复**：单一内建函数表（name → signature → 保留名强制）于一模块（如 semantic/builtin_fns.rs），各域 pass 贡献条目；V8 变表的泛型检查。M，可延至域 11（批次 1 先发货）——批次 2 计划中标记为 navigate 的前置。
- **LOW — 两种状态词汇隐式桥**。矩阵 ✅/🔜/⏳/🏞（baseline:20）vs 登记册 支持/部分/未支持/留生态（verify-semantic-universe.py:14、baseline §8）。桥（🔜 → 完成时 支持；🏞 → 留生态）只存在于工作流散文。矩阵读者见 ⏳ 候选查登记册 候选 状态——不存在；反之 留生态 无矩阵符号。每次批次评审重现。**修复**：baseline §8 一行映射四矩阵符号到四登记册状态（注明 未支持 覆盖其余）。S。
- **LOW — Slots 双表示 + 手动同步不变量**。Slot 指令保留在视图树为 ViewTree::Slot *且*复制进 ModalIntentIR.slots 供绑定 IR 消费者；代码注释断言「两处内容一致，无重复渲染风险」（ir/lower/view.rs:948-958）。与 el.drag_binding 同模式（独立字段非 el.events 内——ADR-073 刻意选择，但每新意图加镜像）。每意图绑定加并行通道（元素字段/组件字段/表达式节点/slot 副本）供 codegen 与校验保持一致。今天成立，注释强制。值得加回归测试走视图树断言意图 IR 镜像它，否则域 10-13 加绑定后静默腐坏。L。

**Recommendations**:
- R1 (S)：更新 verify-semantic-universe.py:6-8 docstring「Nine」至当前域数 + 一句两层分类规则；更新 CONTEXT.md 对照表域 10。
- R2 (S)：baseline §8 加矩阵符号 ↔ 登记册状态桥接行。
- R3 (M)：批次 2（导航）计划中，采用标准化组件级意图收集（关切 1）为 @route 的声明前置；内建函数表（关切 3）为 navigate/back 的前置。
- R4 (M，域 13 前)：标准化组件级意图模式——lowering 收集 (intent, span) 于一处；语义 pass 停止重新推导其规则归属；命名约定「结构性规则在 lowering vs 语义规则在 passes」替代 validate_form_load.rs:4-7 与 validate_feedback_modal.rs:11-13 的 per-pass 注释。
- R5 (M)：内建函数表 + 泛型保留名强制（V8 泛化）。
- R6 (L，批次 5 平列表近 ~17 时)：若放置争议（tooltip 反馈 vs 交互；@t 国际化 vs 响应式）在 spec 评审真实出现，CONTEXT.md 加轻量两轴索引（语法类别 × 功能域）查找表——非层级，仅索引。若平列表超 ~20 无争议，保持平列表；它在工作。

**Risks**: 域 11 @route 继承 span 退化诊断（路由声明错误指向整个组件而非 @route 行）+ 两层校验分裂成 per-domain 仪式而非文档化约定；批次 5 四合一以无预分配编号与无放置规则落地（登记册权威——「每条目映射状态、白名单钉编译器真相」——存活，但*分类*问题由先写批次 spec 者回答）；内建保留名扫描重复成消息文本漂移源（V8 文本今日一文件，三域三副本不会保持相同）；陈旧 docstring（「Nine semantic domains」）现是小症状，每批次留下脚本散文会使其文档——当前全模型最佳单一描述——成为新读者最先读、最不可靠之物。

**Verdict**: **无需结构性重构——今天或剩余批次都如此**。核心机器——带编译器真相锚点的 per-domain 登记、支持/部分/未支持/留生态 状态模型、机制级豁免、语义化即切换——域无关且实证可 mid-batch 吸收新域（域 10 落地不破 1-9）。14 维矩阵不需登记册重切；需要的是*桥*被文档化。翻转裁决的条件：(1) 组件级意图（form/load/route/pagination）在域 13 后仍保持 span 退化两层分裂——则意图挂载异质性成主导成本，span 保全收集 pass 是重构非修复；(2) 放置争议常态化（两批次 spec 对同一能力域分歧）——则平列表需两轴索引（R6），是增强非重写；(3) 能力需跨目标*冲突*语义使诚实降级注记无法承载——那是语言设计决策非登记册决策，由既有 ADR 链标记。混合分类法（可文档化）、意图异质性（可标准化）、内建增长（可泛化）都不达「域模型错误」标准——它们是登记册两月内 9→17 域的成熟成本，全部修正都装得进既有域登记流程而非取代它。

### Rust Expert Review（Katya Volkov）

**Perspective**: 评估双 crate 编译器架构（core 管线 vs codegen 门面）能否支撑语义 DSL 时代每批次落地新域校验 pass（5 批次 58 任务）而管线不退化为一堆只增不改的单体。结论：**架构无需重构——只需在批次 2 前一次合并式收口**。

**Key Observations**:
1. **两层 pass 编排存在不对称，迟早咬人**。9 个 per-component IR pass 在 `run_ir_passes`（analyse.rs:100-185）声明一次、从 `ir::lower()` 单一生产点调用（ir/lower/mod.rs:345）——所有管线入口自动继承。但 4 个程序级 merged-IR pass（validate_theme/validate_event_names/validate_form_load/validate_feedback_modal）在**三处**手写追加、注释逐字复制：lib.rs:118-136、analyse.rs:503-520、analyse.rs:795-812——共 12 个调用点。复制注释不是装饰，是漂移风险的烟迹。
2. **IRProgram 构造重复且 Default 是错的**。相同 6 字段 struct literal 出现在 analyse.rs:482-489 与 :754-761；`IRProgram::default()`（ir/types.rs:1145-1152）产出 `version: "0.1"`——但全部生产构造点（ir/lower/mod.rs:333、analyse.rs:483、:755）用 `"1.0"`。任何取 `Default::default()` 的代码（如门面测试 omnilang/src/lib.rs:595-607）静默得到过期版本 IR，而 version 是序列化 WASM 契约的一部分。
3. **per-domain pass 文件模式是正确的结构，批次 2-5 契合**。semantic/ 是 20+ 个 validate_*.rs（validate_ref_ops 1700 行、validate_feedback_modal 794、validate_form_load 635…）+ mod.rs 作为 pass 注册表。批次 1 的 validate_feedback_modal 零摩擦落地。这正是语义 DSL 时代需要的——无需重塑。
4. **unsafe、错误处理、unwrap 纪律堪称模范——clippy 认同**。unsafe 仅限两个 FFI 模块（crates/omnilang-core/src/ffi.rs、omnilang/src/ffi.rs），SAFETY 注释齐全。`cargo clippy -p omnilang-core --all-features` 与 `-p omnilang` 零真实 lint（仅 ts-rs proc-macro 噪音 11/24 条 + 2 个 feature-gated 未用 import）。~5 个生产 `.expect()` 全部是受守卫的不变量。59 个 unwrap 在 ir/lower/mod.rs、58 个在 types_tests.rs，全部 `#[cfg(test)]`。
5. **core/facade 的 codegen 拆分是正确的边界**。core 的 codegen/shared/（expr.rs 2226、stmt.rs、types.rs、easing.rs、writer.rs、json.rs）是 10 个后端共享的目标无关渲染基础设施；后端在 omnilang/src/codegen/* 由 codegen-* feature 门控。把 shared codegen 移出 core 会反转依赖并破坏 core 的 IR-only WASM 角色。**别动它**。

**Strengths**: 类型化诊断设计（DiagnosticKind thiserror 枚举 ~28 变体 + code_str() 兼容层 + 9 字段 DiagnosticJson 信封）；`#[non_exhaustive]` 纪律（ExprIR/StatementIR/ViewTree/RefOpIR + 5 个 motion 枚举）；穷尽匹配 walkers（walk.rs 单个 match 覆盖 ~25 变体——新增 ExprIR 变体未更新 walker 即编译错误）；pass 顺序文档化且承重（两个改写 pass 前置）；增量缓存设计（内容哈希 + 传递 dependents 传播、fast_hash 明示为存储型哈希、缓存 IR 合并无多余 clone）。

**Concerns**:
- **1. MEDIUM（HIGH 轨迹）— 程序级 pass 编排三处复制**。lib.rs:118-136、analyse.rs:503-520、analyse.rs:795-812（4-pass 注释块逐字相同）。lib.rs:120-126 的注释记录了确切失败模式：validate_event_names 曾被**追溯**加入 run_pipeline，因为单文件路径缺失多文件分析已有的诊断——已发生过分歧。修复：提取 `run_program_level_passes(ir, diags)`，3 个调用点调用。5 分钟工作，消除整类漂移。
- **2. LOW — IRProgram::default() 产生过期版本串**。ir/types.rs:1148（"0.1"）vs 生产点 "1.0"。修复：Default 对齐 "1.0"，analyse.rs 两个 literal 改共享构造。S。
- **3. LOW — Feature-gated unused imports**（轻微）。

**Verdict**: 无需重构；一次合并式收口（run_program_level_passes 提取）在批次 2 前完成。

### Systems Architect Review

**Perspective**: OmniLang 是单语言多目标编译器，最关键的边界不是 crate 边界而是**意图契约**：编译器已理解什么（语义层）、决定做什么（IR）、每个目标怎么做（codegen）。评审考察该契约在语言从 10 扩到 ~14 能力维时是否成立。

**Key Observations**:
1. **一个分析核心，两个刻意分离的入口**。单文件路径（run_pipeline，lib.rs:85-169）与多文件路径（analyse_multi，analyse.rs:403-552）共享全部校验阶段但互不调用。对等性靠回归测试而非代码共享——`compile_surfaces_unknown_event_warning` 与 warnings_gateway_tests 簇（omnilang/src/lib.rs:621-695）把两条路径钉在同一诊断契约上。这是正确取舍：共享编排会耦合缓存语义，但测试必须保留——这是「CLI 看到的 = 编辑器看到的」唯一可能静默腐坏的地方。
2. **意图丢失由渐进登记管理，而非静默**。语义化能力以类型化意图落地 IR（RefOpIR 10 ops、EventDataKind + event_data_schema、motion 枚举、ElementIR.a11y）；未语义化表面（54 事件中 40 个）要么目标侧硬诊断（ADR-075 硬切换），要么登记册逐事件标注、脚本双向钉住。两个 codegen floor 门禁（design-block codegen/mod.rs:126-174、array-method shared/expr.rs:623-647）把「错程序零诊断」变成 E_CODEGEN 硬错误。重复知识仅一处：布尔属性集（关切 2）。
3. **跨层信任显式且文档化**。codegen 信任 IR（ADR-052：「codegen 只负责翻译」）；目标级守卫仅两个 floor 门禁。三个网关（CLI/WASM/C FFI）统一 9 字段 DiagnosticJson；文档化折叠与降级（WASM diagnostics() 把 warnings 折进 errors 数组 omnilang-wasm/src/lib.rs:713-718、codegen 警告无 span `-:0:0`）保持严重性且被测试门控。唯一真正分歧的警告语义是编辑器画布（关切 1）。
4. **验证网是连贯系统，不是脚本堆**。verify-semantic-universe.py（双向断言）+ verify-demo-coverage.py + verify-syntax-lifecycle.py + verify-docs.py + verify-react.sh（18 条豁免账本带陈旧强制）+ 注入语料 855 元组 + 浏览器套件（CI omniview-e2e/e2e job）。机制级例外（存在性锚点断言，ADR-074:43）是同一系统的原则性扩展，正被域 10 锚点账本沿用。CI 门禁验证真实产物：ts-rs 绑定新鲜度、WASM 新鲜度（git diff poiesis/src/wasm/）、e2e 重新生成再测。
5. **双 crate 拆分是正确的边界，但有一个死接口**。core = parser/semantic/IR + 语言无关格式化；facade = 设计系统专属 codegen + 编排；重导出缝（shared/mod.rs:17-29）干净。但 C FFI 跨两 crate 拆分且无可复现生产者（关切 3）。

**Strengths**: 三层架构文档化、强制、稳定（run_ir_passes 单一编排点 + 承重 pass 顺序）；诚实降级系统化（每个已知缺口要么硬诊断要么警告要么账本：ADR-074:78 span-less 警告、ADR-075 一步切换、legacy 目标从不收集警告、typescript.rs 停用文档化 ADR-031 D6、debug_assert 层 debug 构建响亮失败）；floor 门禁最小且原则化（恰好两个、P0 零成本提前返回、JSON-scan 完整构造）；诊断消费者对等由测试强制（统一信封、保严重性折叠、CLI stderr 不使构建失败、codegen 警告→结构化 Diagnostic 转换）；缓存是连贯子系统（内容哈希 + LRU + WASM thread-local save_cache_state）。

**Concerns**:
- **1. MEDIUM — 编辑器画布 warnings → success=false；CLI 不会**。lib.rs:185（`success: !has_errors && warnings.is_empty()`）与 :217-220 vs 编译路径 :464（errors-only）。ADR-074 B2 修复后未知属性动态绑定产生警告——同一输入 CLI 构建成功（warnings 非失败性）而画布显示「编译失败」。已登记（ADR-074:77 接受后果 + Poiesis 观察清单）但未裁决。修复：画布 success 只按 errors 门控（镜像 :464），加对等回归测试。S。
- **2. MEDIUM — 布尔属性双源漂移活跃；hidden 仍退化（B2 根因类）**。parser 关键字集 15 个（parser/view/element.rs:38-61，含 hidden/autocomplete/autoplay）vs codegen HTML_BOOLEAN_ATTRS 13 个（omniview/view/element.rs:19-33，含 open 但无 hidden/autoplay）vs 已知属性白名单 :126-136（无 hidden）。`<div hidden={showModal}>` ——最常见可见性切换模式——解析干净后落 `_ =>` fallback 发射静态 data-hidden + warning。B2 后已诊断，但语义退化与 B2 bug 相同：属性不工作，只有警告。域 7 布尔属性断言是存在性（「keywords non-empty」，ADR-074:43）——B2 为之创建的那类漂移未被断言。修复：hidden（+autoplay）入 HTML_BOOLEAN_ATTRS；升级域 7 断言为集合包含。S。
- **3. LOW — C FFI 表面不可验证的死重**。`#[no_mangle]` omni_analyse（core ffi.rs:131-137）与 omni_compile_multi_to_target（omnilang/src/ffi.rs:49-183）；唯一消费者 test/omnilang/rust_bridge.py:18-19 加载 test/omni-compiler/target/*/omni_compiler.dll——工作区不存在该 crate（两个 crate 都是 rlib-only；唯一 cdylib 生产者是 omnilang-wasm 与 poiesis/src-tauri）。新鲜 checkout 上 RustBridge.is_available() 恒 false，Python 测试静默退化。test/omnilang/omni_compiler.dll 是 6 月 26 日过期二进制。修复：二选一——删除 C FFI + Python 桥（S，WASM 覆盖一切消费者），或建 omnilang-ffi cdylib shim + CI 构建（M）。
- **4. LOW — array-method floor 门禁每次编译把整个 IR JSON 字符串化**。program_has_array_methods（shared/expr.rs:607-609）对 Python/Rust/Dioxus 每次编译 serde_json::to_value().to_string() 子串扫描。刻意（完整构造，镜像 DCE 先例）且正确目标限定，但 O(IR) 分配 × 3 目标，已有类型化 walker 可用。可选修复：换类型化遍历。S。

**Recommendations**: Immediate — hidden/autoplay 入册 + 域 7 断言升级（S）；画布 success 语义裁决（errors-only + 对等测试）（S）。Short-term — C FFI 裁决：删或 shim+CI（S/M）；域 10 Flutter 锚点落地时保持锚点账本模式。Long-term — 功能设计级四域落地时 IR 特化跟随 types_motion.rs 先例（每域 IR 子模块而非 ElementIR 加字段）（M）；可选统一警告信封（M，YAGNI 至下一个网关消费者出现）。

**Risks**: hidden 持续是半静默陷阱（最常见动态可见性模式只给 warning）；画布失败性警告侵蚀编辑器信任；C FFI 腐坏（任何 DiagnosticJson 契约变更未验证）。

**Verdict**: **无需重构——四个条件**：(1) codegen 保持纯翻译——第三个 floor 门禁须先经 ADR-052 floor 表评审（今天两个是正确数量）；(2) 登记先行纪律保持；(3) 两个 MEDIUM 项（画布语义、布尔属性）被解决——它们是有意设计与实际行为唯一相撞处；(4) 功能设计级四域落地时 IR 特化跟随 types_motion.rs 先例。补充修正：CLI 位于 omnic/src/commands/（brief 的 omnilang/src/commands/ 不存在）。

### Maintainability Expert Review

**Perspective**: 这是一个治理良好的代码库，风险不是混乱而是**多点同步**——同一逻辑变更（新 pass、新方法翻译）必须在 2-3 处完成，且没有门禁比较它们。治理机器（deferral 账本、豁免账本、双向文档断言脚本）真实且被强制；主要可维护性债是已产生两类缺陷的机械重复，加上膨胀到五层互锁契约的 onboarding 面。

**Key Observations**:
1. **一文件一校验 pass 模式干净扩展**。最新批次 1 pass `validate_feedback_modal.rs`（794 行）完全遵循既有约定：spec 引用 V 编号、V4/V11 的诚实「本层不可验证」注记、与 validate_form_load.rs 相同的 `&IRProgram + &mut Vec<Diagnostic>` 签名。新增域 pass 是对既有 pass 的复制-修改——摩擦低，顺序理由在 run_ir_passes（analyse.rs:60-129）文档化一次。
2. **程序级 pass 序列被三处内联**——run_pipeline（lib.rs:118-136）、analyse_multi（analyse.rs:497-522）、analyse_multi_cached（analyse.rs:789-812），而组件级 pass 已用共享 run_ir_passes 辅助。代码库自己的注释（analyse.rs:62-70）承诺「每个入口看到相同诊断相同顺序」——重复与文档化意图矛盾。批次 1 的 validate_feedback_modal 正确加到三处——纪律有效，但批次 2-5 将加 ~6 个程序级 pass，每个需 3 处同步编辑，且无测试比较三条路径输出。
3. **方法翻译存在于三张未共享表面且有实证漂移**。React translate_method_call（react/expr.rs:2610-2704）、OmniView 方法臂 + stdlib 兜底（omniview/expr.rs:322-382）、共享 Dart 族 format_method_call（shared/expr.rs:851-966）。Deferral #76（React 缺 to_uppercase/to_lowercase/is_empty/abs）仍开——is_empty 发射 `obj.isEmpty`（JS undefined）、abs 运行时抛错零编译诊断。且**行为分歧**：`contains()` 字符串接收者在 React 发射 `"abc".includes("b")`（react/expr.rs:2658）但 OmniView 发射 `Object.prototype.hasOwnProperty.call("abc","b")`（omniview/expr.rs:363）——同源不同真值。
4. **巨型 tests.rs 按任务组织而非快照重**——flutter/tests.rs 249 测试（11946 行）、react/tests.rs 202 个（8991 行），合计仅 6 处 insta 引用。快照层在 tests_regression/（96 .snap）。章节头引用任务（「── Task 3.5 M2: functional updaters ──」），单一模块测试按任务史散落（react ref codegen 在 4333 与 4455；design 在 3680 与 3718）。
5. **豁免燃尽健康验证**：scripts/verify-react.exemptions 24 条活跃条目，每条带计数上限 + deferral 编号 + 陈旧条目门禁——比 brief 声称的 287→29 更好。这是受追踪且有收缩轨迹的债，不是缺陷。

**Strengths**: 共享编排 + 承重顺序是常态（run_ir_passes 集中 9 个组件级 pass 并文档化顺序理由）；治理机器被强制而非装饰（deferral 账本带触发条件与提交证据、豁免账本陈旧即失败、双向 verify 脚本、.snap.new 显示回归机制实时捕获输出变化）；诚实降级而非静默分歧（semantic-universe.md:111 明示方法调用层「不具跨目标保证」，缺口进 deferral #76/#28/#88，E_CODEGEN floor 门禁守卫编译-only 后端）；一致 pass 约定（V 编号规则 ID、白名单常量作登记册锚点、层边界诚实注记）；域内聚模块布局（codegen/{omniview,react,flutter}、semantic/validate_*.rs——新成员一眼定位）。

**Concerns**:
- **MEDIUM-1 — 程序级 pass 序列三处内联**。lib.rs:118-136、analyse.rs:497-522、analyse.rs:789-812。承诺（analyse.rs:62-70 相同诊断相同顺序）靠三份逐字副本的人力纪律维持。修复：提取 `run_program_passes` + 对等测试（同一 fixture 跑 run_pipeline 与 analyse_multi 断言 (kind, message, span) 序列相同）。S，纯提取。
- **MEDIUM-2 — 方法翻译三张未共享表，实证漂移**。react/expr.rs:2610-2704 vs omniview/expr.rs:322-382 vs shared/expr.rs:851-966；分歧 react/expr.rs:2658 vs omniview/expr.rs:363。唯一「用户可见失败零编译诊断」的缺陷类（运行时 TypeError 进生成代码，deferral #76 is_empty/abs）。修复：shared/expr.rs 方法表升为 floor 方法集单一事实源 + per-target 覆盖槽 + 跨目标对等矩阵（cross_target.rs 扩展，string-receiver contains 案例含入）。M。注：semantic-universe.md:111 的「不具跨目标保证」是有意记录——本修复收窄无保证面至真正目标专属方法，而非消除。
- **LOW-3 — validate-pass 签名不一致 + 重复 diag 辅助**。analyse.rs:100-128（validate_props/validate_refs return-then-push vs validate_ref_ops/validate_array_methods 直接 push）；error_diag/warn_diag 在 ~8 个文件重复实现。修复：统一 `&mut Vec<Diagnostic>` + 提取共享 diag_helpers 模块。S。
- **LOW-4 — 巨型按任务组织 tests.rs**。flutter 11946 / react 8991 / omniview 3465 行。修复：现在别拆；批次 2-5 触及某 codegen 区域时把该区域章节按 `── Section ──` 标记切成 tests/ 子目录。M 总 / L 每次触及。

**Recommendations**: Immediate — 提取 run_program_passes + 对等测试（S，批次 2 前，趁同步风险仍 3 处）；统一 pass 签名 + 共享 diag helper（S）。Short-term — 方法表统一 + 跨目标对等矩阵（M，关闭 #76/#28/#88；用户可见收益最高）。Long-term — 增量 tests.rs 切分（M）；CONTRIBUTING 文档映射五层契约（semantic universe → BNF → vocabulary registry → ADR/deferral → exemption ledger）到各 verify 脚本与各捕获的失败模式（S，onboarding 杠杆最高）。

**Risks**: 单文件与多文件编译路径静默诊断分歧（批次 2-5 pass 落在 2/3 处时，无测试比较）；生成 React 代码运行时 TypeError 继续（每新 floor 方法 ~1/3 概率漏 React 表）；跨目标行为分歧（contains 案例）静默侵蚀「语义 DSL」品牌承诺；onboarding 昂贵。

**Verdict**: **无需维护驱动重构——架构一致、受治理、批次 1 最新 pass 零摩擦落入一文件一 pass 模式**。两项针对性合并值得：(a) 三处内联程序级序列 → run_program_passes（S，批次 2 前）；(b) 三张方法翻译表 → 一张共享表 + per-target 覆盖 + 跨目标对等测试（M，关闭唯一零诊断缺陷类）。巨型 tests.rs、豁免账本、deferral 账本、文档对齐机器全部按设计工作，不应预防性触动。

### Performance Specialist Review

**Perspective**: 从编译期（pass 编排、IR 优化、缓存、WASM 入口）与生成代码运行时（信号、直写通道、@for 列表、动画触发器三 P0 目标）两侧评估 ~65k LOC 架构是否需要性能驱动重构——结论是需要有界的参数级与接线级修复，而非重构。

**Key Observations**:
1. **缓存节省 parse/semantic/lower，但程序级 merged-IR 阶段每次击键顺序走全项目**。每次击键缓存分析（102 文件 ~10.5ms）由 validate_multi_ref（analyse.rs:629）、全 IR 深克隆（:768）、optimize（:790）、四个程序级校验器（:795/:802/:807/:812）、lints::run_all（:815）主导。codegen 仅占 1.7ms。无 pass 在其输入域不受编辑文件影响时被跳过。
2. **5 个入来域的「线性 pass 墙」是加法非乘法——当前每次击键 ~7 个全 IR 遍历 + 1 个全 IR 克隆**。本批次已确认模式：validate_form_load 与 validate_feedback_modal 作为两个新 merged-IR pass 加入。每域加 1-3 个全 IR 遍历。仍线性、102 文件仍 ~10.5ms，但无 skip-if-unaffected 机制时，500+ 文件趋势 50ms+/击键（WASM worker 线程）。
3. **MAX_CACHE_ENTRIES=50 低于旗舰项目自身文件数（102）**。实测回归 15.6×（逐出路径 10.9ms vs 命中 0.7ms，memory perf-review-2026-08-08）。有界意图（内存上限）是刻意的，但参数在真实项目规模下使缓存失效。修复已在计划（docs/superpowers/plans/2026-07-31-compiler-pipeline-fixes.md:542：500 或 max(50, len)），但 analyse.rs:238 仍读 50。
4. **编译核心验证为线性**。optimize.rs DCE 是单次 collect 进 FxHashSet + retain（optimize.rs:237-305）、不可达码消除 per-unit（:436-456）、merge_file_ir_into 线性（analyse.rs:326）。编译路径无 O(n²) 或超线性。
5. **OmniView 运行时非性能是有意且文档化的**——同步订阅者扇出无批处理（runtime.rs:42-75，ADR-028 D3）、profiling 门控前无批处理（ADR-037）、__omni_el 缓存永不逐出（runtime.rs:188-200，ADR-038 P0-1）、非 keyed @for 全清重克隆（list.rs:149-156）。生产目标承载真正的运行时性能架构，且是真实的（见下）。

**Strengths**: 增量缓存设计正确（内容哈希 + 反向导入边传递失效、per-file 阶段 par_iter 并行、15.6× 回归纯参数问题）；optimize.rs 克制模范（单次线性 collect、保留 V-F8 保留名 optimize.rs:284-293）；生产目标真实运行时性能架构（React per-field store-slice selectors store_slices.rs:1079-1103、保守 React.memo 分析 view.rs:1988-2069 只 memoize 可证明安全实例、Flutter ValueNotifier 字段 + ListenableBuilder(Listenable.merge) 作用域重建 view.rs:3396）；直写通道是编译期数据流非运行时启发式（direct_write.rs 5 个收集器 + direct_write_keys 防双路径竞态）；生成 @for 代码避免明显运行时陷阱（keyed 列表 DOM 复用 + FLIP、per-render _elCache 作用域 list.rs:340、WASM 边界 serde_wasm_bindgen 直接 JsValue 无双重 JSON）。

**Concerns**:
- **HIGH — 缓存路径重跑全项目顺序 pass 无脏度门控**。analyse.rs:629/:768/:790/:795-812/:815。每次击键重新校验优化全部 102 文件 merged IR，编辑只碰一个文件样式也如此。缓存只省 parse/semantic/lower（~22ms 冷部分）；程序级阶段 O(project)/击键零跳过逻辑。5 个入来域加加法全 IR 遍历（本批次已加两个）。102 文件 ~10.5ms；500+ 文件 50ms+/击键。修复：per-pass 脏度追踪——每 merged-IR pass 声明其读取域，编排器在 to_reprocess（排除 dependents）不交集时跳过。analyse_multi_cached Phase 3c-4 内机械改动，无新架构。
- **HIGH — MAX_CACHE_ENTRIES=50 造成实测 15.6× 回归**。analyse.rs:238（const）、:711（逐出循环）。旗舰项目 102 文件，每次编辑逐出最旧条目 → 下击键为那些文件付全量重 parse+semantic+lower。修复：`max(50, sources.len())` 或 500（计划已指定，应用它）。可选构造参数让 poiesis 传项目规模。
- **MEDIUM — WASM 层潜在未缓存多文件编译分支**。omnilang-wasm/src/lib.rs:675（compile_to_omniview 多文件分支 → compile_multi_internal → 未缓存 analyse_multi omnilang/src/lib.rs:297-305）。compile_multi_to_omniview（lib.rs:542）正确走 compile_multi_internal_cached（:578），但对象 API 多文件分支绕过缓存——每调用付全冷编译。修复：多文件分支路由 compile_multi_internal_cached。一行。
- **MEDIUM — 程序级阶段顺序而 per-file 阶段并行**。analyse.rs:649（并行）vs :753-815（顺序 merged 阶段）。顺序阶段是每击键主导成本，而 WASM 单线程 worker 无线程可用——修复不能是朴素并行。修复：脏度门控是真正缓解（减工作而非并行）。:768 全 IR 克隆可改 per-file 指针 join + 编辑文件 copy-on-write——消除 0.7ms 克隆无需重构。
- **LOW — direct_write.rs 每组件多次全视图树遍历**。direct_write.rs:115/:407/:767/:880/:1178 五个独立收集器。O(component view × 5)/组件/编译。当前吸收在 1.7ms codegen 内；仅病态单文件组件（数千节点）会注意。现不修；codegen 预算成问题时融合为单遍历共享 visitor。
- **LOW — 共享 thread-local ANALYSIS_CACHE 跨所有 OmniCompiler 实例**。omnilang-wasm/src/lib.rs:341 单一 static。同线程两项目共享相对路径键缓存。内容哈希防陈旧命中，但 import 图 dependents 边可能跨项目串——正确性边角（最坏多一次重解析，非错误结果）。修复：按项目 id 命名空间或接受最坏成本。

**Recommendations**: Immediate（S）— 应用计划好的缓存尺寸修复（analyse.rs:238）；compile_to_omniview 多文件分支路由 cached（omnilang-wasm/src/lib.rs:675）；验证 poiesis rustCompiler.ts 调试窗用 cached 变体。Short-term（M）— analyse_multi_cached Phase 3c-4 per-pass 脏度门控（每校验器声明输入域；脏集不交集跳过；lints 限制 AST 规则至脏文件）。Long-term（L，仅触发时）— 每击键缓存分析超 ~30ms 或项目超 ~500 文件时：:768 全 IR 克隆改 copy-on-write 模块 join + 考虑 pass 融合（五个 merged-IR 校验器折一遍历）。不并行——WASM 单线程，增量优于并行。

**Risks**: 15.6× 缓存回归随项目增长复合（旗舰项目已过逐出阈值）；加法 pass 墙把每个入来域变成永久每击键成本无跳过机制；潜在未缓存 WASM 分支一个接线失误即复活 150ms 调试窗双编译。

**Verdict**: **无需性能驱动重构**。架构根本健全：全部 pass 验证线性、增量缓存设计正确（仅尺寸参数错）、生产目标 codegen 有真实运行时性能架构。102 文件 10.5ms/击键时重构为时过早。**仅当** (a) 每击键缓存分析超 ~30ms 或 (b) 项目超 ~500 文件——然后应用脏度门控与 copy-on-write 模块 join，是对既有编排器的加法改进，非重写。OmniView 调试运行时非性能按 ADR-028/037/038 正确排除在任何优化职责外。

### Implementation Strategist Review

**Perspective**: 5 批次/58 任务增量程序结构健全：每批次加一个语义 pass 模块 + 一个 parser 指令函数 + 加法式 serde-safe IR 字段，per-batch 全表面执行可持续**因为水平层已存在**（共享 codegen expr 表、per-batch 语义模块、自维护登记册）。真正风险不在架构而在执行纪律：批次 1 当前 mid-flight（Flutter 任务未提交、自身门禁证据未绿），且一个已知 React codegen 缺陷（ToastProvider 吞掉包裹视图）只记录在测试文件注释、无 deferral 账本条目、无排期修复。

**Key Observations**:
1. **B1 是 mid-flight 非关闭**。Task 4（React, 478eea51）与 Task 6（OmniView, 849ac81f）已提交；Task 5（Flutter @modal codegen，~172 行在 flutter/{mod,view,ref}.rs）完整实现但**未提交**，verify 脚本仍标 Flutter 锚点待定（verify-semantic-universe.py:713「Flutter 锚点暂缺：Task 5 Flutter codegen 待落地」，commit 7ecaf627），Task 8（Modal.omni demo 迁移 + NotificationsPanel 删除）未开始。批次自身五层门禁在未提交工作落地前不能绿。
2. **水平层问题在代码中干净解决，不只在计划**。每批次语义工作是一个独立模块（validate_feedback_modal.rs 794 行），恰好接在三个调用点（analyse.rs:520 analyse_multi、analyse.rs:812 analyse_multi_cached、lib.rs:136 run_pipeline）。B3 计划为 validate_breakpoints 指名同一三个调用点——模式已建立且计划正确枚举分派点。parser 扩展点同样每指令一个函数（parser/view/directives.rs:117 parse_modal_directive、:129 skeleton）。这是对「per-batch 全表面执行是否可持续」的结构性回答：是，因为每批次新表面是小的、预先识别的插入点集。
3. **IR 增长构造性向后兼容**。ElementIR（ir/types.rs:772-841）每批次积累 1-3 字段，全部 `#[serde(default)]` + Option + skip_serializing_if，ViewTree `#[non_exhaustive]`。每 IR 加法与已提交 poiesis 绑定与持久化产物 serde/ts-rs 兼容。export_bindings 测试（core lib.rs:249）是新鲜度门禁，B4 Task 3 起每批次强制。
4. **已知 React codegen 缺陷已文档化但未登记**。react/component/mod.rs:583 发射 `function ToastProvider() {`——零参数、从不渲染 {children}——而 JSX 发射器把组件整个返回树包进 <ToastProvider>（jsx.rs:50）。React 19 类型下包点 TS2559；功能上包裹视图**整体被吞**（任何用 toast() 的 React 组件渲染空白）。只记录在 generated_code_validation.rs:279-291（cap=1 allowance），明确为 Task 4 follow-up——但无 deferrals.md 条目、无触发条件、无任何批次计划任务。
5. **B5 计划的可扩展前提过期**。batch5-longtail.md 提议 ENABLED_D14..D17 布尔守卫，前提是脚本硬编码 range(1,10)。实际脚本自维护：verify-semantic-universe.py:419 迭代 sorted(DOMAIN_TITLES)（B1 Task 0 已修）。Wave-2 并行编写导致的计划漂移；执行 agent 必须任务开始时重验计划前提。

**Strengths**: 登记先行 + 门禁先行顺序在代码中验证正确（B1 Task 1 在 parser 变更前落地 vocabulary/universe 条目；universe 脚本两层断言——存在性 + 锚点账本——真实）；跨批次依赖显式追踪（progress.md:41 记录 B1 @skeleton 建于 v0 @load、B4 须扩展至 {name}Loading 分支；B4 计划把该义务带进其 codegen 任务）；校验管线有干净 Phase-5 槽（validate.rs:56-91 1 parse/2 per-file/3 cross-file/4 cycles；B2 路由注册表 Phase 5 无缝契合，V14 重复/冲突 @route 硬错误位置正确）；CI 覆盖累计编译风险（5 后端 feature 矩阵 + test-all-backends job ci.yml:179/362——10 后端 feature 编译面持续验证；Vue/Dioxus semantic floor 是有意、有账本的延期 #87）；门禁豁免纪律诚实（新 TS allowance 带根因注释——ToastProvider 条目 8 行解释缺陷——且是上限非静默）。

**Concerns**:
- **MEDIUM-1 — React ToastProvider 包裹内不渲染任何东西；缺陷未登记**。mod.rs:583 零参数发射、jsx.rs:50 包裹点、仅 generated_code_validation.rs:279-291 文档化。任何用 toast() 的 React 组件整体视图被吞 + demo 语料 TS2559。修复超出剩余 B1 任务范围，无账本条目会被遗忘——B5 @t i18n 与 demo 返工不会发现它。修复：(a) 现在修（children: ReactNode + 渲染 {children}，小、局部、移除 cap 条目）；或 (b) deferrals.md 登记 + 触发条件「任何触碰 React 组件根发射的变更」+ B1 关闭内排期。测试文件内注释对未来批次不可发现。
- **MEDIUM-2 — 批次 1 Task 5 未提交，工作树混合独立工作流**。git status 22 个修改文件 / 917 插入跨越 B1 Flutter codegen + omniview 修复 + 5 个已接受快照 + 3 个新 fixture + generated_code_validation.rs（+149，含 Flutter @modal 测试）**加无关 poiesis CodeFlow 变更**（groups.ts +41「全键扫描定位组」、FlowInspector.tsx +110、omnilang_wasm_bg.wasm 2084457→2194707 字节）。B2 开始门应是「B1 关闭」——Task 5 提交、universe Flutter 锚点绿、Task 8 demo 迁移完成。B1 门禁证据待定时开始 B2 会堆叠未提交 blob 并劣化逐任务审查/bisect。修复：Task 5 独立提交；poiesis groups 变更单独落地；强制 commit-at-task 粒度。
- **LOW-1 — B5 计划 ENABLED_D14..D17 守卫前提与实际脚本矛盾**。batch5-longtail.md vs verify-semantic-universe.py:419 sorted(DOMAIN_TITLES)（B1 Task 0 起自维护）。照本宣科执行浪费 B5 T2 token，最坏引入域启用的第二事实源。修复：B5 执行时重读脚本，去掉守卫或文档化为 belt-and-suspenders。
- **LOW-2 — ElementIR 字段数 B5 后需合并**。ir/types.rs:772-841（20+ 字段，scroll/drag 旧字段已在 791-793 删除）。每批次安全但 5 批次后邀请死字段（B4 多实例 @load 取代 B1 匿名-load @skeleton 耦合时）。修复：B5 收口或后续批次加显式 IR 合并任务——审计消费 codegen 路径被取代的字段。

**Recommendations**: Immediate — B1 Task 5 单提交（Flutter codegen + flutter_modal_generates_show_dialog_with_focus_traversal 测试 + 快照接受 + 3 fixtures）（S）；ToastProvider 缺陷登记 deferrals 或现在修（首选，~5 行）（S）；poiesis groups/FlowInspector/wasm 变更与 B1 blob 分离（S）。Short-term（B2 前）— 按自身门禁契约完整关闭 B1：Task 5 提交 → verify 脚本 Flutter 锚点绿（:713）→ Task 8 demo 迁移（Modal.omni 重写、NotificationsPanel 删除）→ Playwright + dart analyze 绿。B2 开始以此门控（M）；B5 T2 执行时重验 ENABLED 守卫前提（S）。Long-term — 保持 per-batch 插入点模式（语义模块 + 3 调用点 + 指令函数 + IR 加法字段 + ts-rs export_bindings 检查/批次）——不引入批次抽象层，当前模式本身就是水平层，再加层只增间接不减爆炸半径（无/保留）；post-B5 follow-up 加 IR 合并 + 死代码审计任务，含重查 deferred React/Flutter floor 缺口（#76/#81）（M）。

**Risks**: B1 Flutter 锚点待定时 B2 开始 → B2 五层门禁背 B1 债；未提交 Flutter 代码回归会被归错批次；ToastProvider 缺陷未修未登记 → 未来每个用 toast() 的 React demo 发空白视图（cap=1 按包裹点计——每个新 toast 组件加一个点）；计划-现实漂移照本宣科执行 → 浪费 token + verify 工具链双事实源。

**Verdict**: **无需重构**。per-batch 全表面增量模型对该代码库正确：水平层存在（共享 codegen expr 表、3 固定调用点的 per-batch 语义模块、自维护 universe 脚本、加法 serde-safe IR、Phase-5 校验槽），可逆性由 `#[serde(default)]`/`#[non_exhaustive]` 兼容提供而非 feature 旗标（与一步到位 ADR-052 决策 5 一致），跨批次依赖全部账本化。保持该裁决的条件：(1) B1 完整关闭——Task 5 提交、锚点绿、demo 迁移——再开始 B2；(2) 当前工作树恢复 commit-at-task 粒度；(3) ToastProvider 缺陷在 B1 关闭内修复或登记 deferrals 带触发条件；(4) B5 执行在任务开始时重验脚本前提假设。若 B1 滑过 B2 计划开始时间，重排 B2（对 B1 demo 迁移无硬依赖，只依赖已提交的 parser/IR/semantic 表面）而非 mid-B1 开 B2。

### Pragmatic Enforcer Review

**Mode**: Balanced

**Overall Simplicity Assessment**: **7/10 — 范围适当，有具体修剪目标**。核心架构（三层语义墙 + 意图 IR + per-target codegen，ADR-052/054）不过度设计。每层有 2+ 消费者、顺序依赖文档化、两个 IR 改写 pass（validate_ref_ops、validate_array_methods）证明墙是设计非堆积。批次 1 语料驱动（6 个新 fixture：feedback_modal.omni、skeleton.omni、toast.omni、error_boundary.omni、modal_ref.omni、modal_ref_motion.omni）。过度设计风险是**前瞻性**的：per-batch 投机项（media capture、container queries、pagination）会为无真实语料的需求加 IR struct、校验 pass、三目标 codegen。

**Strengths**: 校验墙分阶段非单体（semantic/mod.rs:1-19 文档化三层；analyse.rs:68-99 文档化顺序理由——validate_ref_ops 先于 validate_array_methods 因两者改写 IR；parse 层校验历史上试过被否——折叠校验丢元素级 span、符号表上下文、跨文件检查；每 validate 模块平均 ~450 行含测试——validate_feedback_modal.rs 794 行仅 ~350 生产代码承载 4 特性 12 规则，~29 行/规则）；诚实分层无假覆盖（模块 doc 承认 V4/V11 在语义层不可见、委托 lowering last-wins——架构报告自身盲区而非掩盖）；流程自警（批次 1 spec 自身对延期项引用 YAGNI：无手动关闭按钮、无 toast 设计 token 语法 spec:104-105；§9 六个未决问题诚实开放）；地道 per-target 映射真实（React 原生 dialog.showModal() react/component/mod.rs:1365、Flutter showDialog + FocusTraversalGroup flutter/view.rs:1749-1819——「编译器找地道映射」原则在已发货代码中实证）；登记册 + 断言脚本有真实消费者（verify-semantic-universe.py 460 行 / verify-vocabulary.py 443 行强制 ADR-074 登记先行、docs-gate 层运行、历史上捕获真实漂移——用户声明有意的治理，不可作为 YAGNI 挑战）；合并本能已存在（批次 5 共享 ElementIntentIR 8 元素意图 prop 一 struct——团队在正确时间注意到模式增长的正确本能）。

**Concerns**:
- **C1 — MEDIUM — 程序级 pass 双处登记（cached + fresh 路径）**。analyse.rs:503-520 vs :795-814（同 4 调用点镜像）。每新程序级 pass 两处添加，漏一处则 cached 路径静默跳过诊断——与缓存工作相邻的正确性陷阱。批次 1 已两处加 validate_feedback_modal；批次 2 计划第三登记点（validate_multi_ref V14，Phase 5）。更简替代：提取 fn run_program_level_passes(ir, diags) 两路径调用。行为同一，~10 行 diff。**Necessity 7 / Complexity 4 / Ratio 1.75**（超 1.5 阈值）。建议：⚠️ simplify。
- **C2 — MEDIUM — OmniView 手写焦点陷阱重复原生 <dialog> 能力**。omniview/view/modal.rs:177（style.display 切换 + _dialog.focus()）与 :192（~15 行 keydown Tab 循环焦点陷阱）。React 经 showModal() 免费获得陷阱 + Esc + 初始焦点；Flutter 经 showDialog + FocusTraversalGroup。手写陷阱有真实 a11y 缺口：初始焦点到 <dialog> 自身而非第一个可聚焦子元素；Esc 处理器只在焦点在 dialog 内时工作（原生 showModal 无条件处理 Esc）。~25 行发射 JS 重实现浏览器特性。更简替代：发射 showModal()/close() + cancel 事件监听写信号 false（__omni_subs.set(...)），删除 Tab 循环陷阱。**先验证的注意事项**：showModal() 的 top-layer 放置可能逃出 OmniView 预览容器的 transform/clip——检查 ADR-028 调试运行时取舍是否为此动机；若是，最小修复是仅初始焦点 + Esc-outside 缺口。**Necessity 8 / Complexity 3 / Ratio 2.67**。建议：⚠️ simplify（先调查容器约束；Tab 循环陷阱是可删部分）。
- **C3 — ⏸️ DEFER — 容器查询（批次 3 低优先候选）**。基线 spec §4（低优先候选，范围未定义）；批次 3 计划 T3（style 块 @container 透传）。零语料需求——批次 3 demo 迁移不需要卡片级断点。React 需 container-type + 特性检测；Flutter 无原生容器查询（手动 LayoutBuilder）；OmniView 两者都要。三目标成本 × 无手写先例。更简替代：bp() + @media 透传（批次 3 实际交付物）覆盖真实响应式需求；容器查询骑 @media 透传机制——无需新语法即可开始。**Necessity 3 / Complexity 7 / Ratio 0.43**。建议：❌ skip（无限期延期；透传机制使后期添加便宜）。
- **C4 — ⏸️ DEFER — 媒体捕获 / 相机权限模型（批次 5）**。基线 spec §4 media 行（「权限模型批次 spec 细化」——未定义）；批次 5 计划 T12（canvas ctx 门类 V14 → filePick/capture V15/V17 + 域 16）。路线图最强投机完整性项。相机/麦克风零语料（无 demo、无计划引用的手写先例文件），权限模型与 Tauri capability 系统交互（poiesis 安全模型：命令不按 capability 门控）——语义未定义的安全敏感面。跨目标不对称严重：navigator.mediaDevices vs Flutter permission_handler vs Tauri capabilities。更简替代：拆分批次——@filePick（input type=file / file_picker）便宜可辩护；@capture 整体延期至真实 .omni 组件需要。**Necessity 2 / Complexity 8 / Ratio 0.25**。建议：❌ skip @capture；✅ keep @filePick。Canvas ctx（V14）同桶延期——canvas 绘制 DSL 是巨大表面零语料。
- **C5 — ⏸️ VERIFY — 批次 4 分页/无限滚动**。批次 4 计划报告（13 任务；@load IR Vec-ification 破坏性变更；分页/滚动/轮询）。load_intent → Vec IR 破坏性变更（批次 4 计划风险行）是首个「per-feature IR 设计在范围增长下翻搅」的迹象。分页/无限滚动改变 IR 形状（page/cursor 参数）并加 3 目标列表组件——但计划无手写语料先例（DataFetching 已被标记未决）。debounce/throttle/polling stdlib 半边便宜（小工具函数，一个 IR 节点）。更简替代：先发 debounce/throttle/polling（stdlib 函数，无 IR 破坏变更）；真实 list + load-more demo 出现前不承诺 @load IR 破坏。**Necessity 4 / Complexity 6 / Ratio 0.67**。建议：⏸️ 延期 IR 破坏性变更待语料；✅ 继续 stdlib 工具。
- **C6 — LOW — verify-semantic-universe.py 结构循环硬编码域范围**。批次 5 计划报告行 17（结构循环 range(1,10) 不可扩展；域 14-17 需手写断言函数 + ENABLED 守卫）。脚本域迭代耦合魔法 range；每域超 10 需定制手写断言。**注意**：implementation-strategist 报告 B1 Task 0 已改为 sorted(DOMAIN_TITLES)（verify-semantic-universe.py:419）——若属实 C6 已解决，B5 计划前提过期。更简替代：从 universe doc 标题派生域列表。**Necessity 6 / Complexity 2 / Ratio 3.0**。建议：⚠️ simplify opportunistically（非阻塞）。
- **C7 — LOW — 保留名检查是等待第 4 实例的手写模式**。validate_feedback_modal.rs:36-64（V8：~30 行循环 6 声明点查 showToast）；同模式已存在于 validate_form_load.rs V-F8（form 侧 112、load 侧 150）。三个 ~30 行相同遍历实例。批次 2（导航保留名）与批次 5（@key）会再加。建议：批次 2 加下个保留名时做一个共享 check_reserved_names(component, &["showToast"], msg, diags) helper——不是现在追溯。**Necessity 5 / Complexity 2 / Ratio 2.5**。建议：⚠️ simplify at next instance（批次 2）。
- **C8 — LOW — ElementIR Option 字段积累（watch item，无行动）**。ir/types.rs:810-834——5 个可选意图字段（a11y/drag_binding/modal/skeleton/error_boundary），各 serde-skipped、1 消费者（自身 codegen 路径）。像经典累加器味道，但字段独立——codegen 无交叉污染，各 ~6 行。批次 5 共享 ElementIntentIR 是计划中的合并，且**不明显更好**：8 个无关 Option 一 struct 与 8 字段同成本。别追溯批次 1 的三个字段。建议：保持模式；字段近 ~8 或意图开始交互（@modal 内 @skeleton 需跨字段约束）时再看。**Necessity 4 / Complexity 3 / Ratio 1.33**。建议：✅ implement now（即不变；当前模式是正确的最小形状）。

**检查过且未标记**（验证注记）：~25-pass 墙本身不过度设计（19 validate 模块 + 2-pass 解析 + 程序级检查，31 文件含测试 14805 行；两 pass 改写 IR 证明层必要性；parse 层校验历史被否；折叠校验丢 3 目标 + 编辑器消费的元素 span 与符号上下文；2+ 消费者 → 抽象自重）；登记册 + 门禁脚本是有意治理用户声明硬规则（ADR-074），历史上捕获真实漂移，不按用户提示挑战；批次 2 导航（8 项全进语言）用户裁定（车票 07：全量进语言）+ 语料存在（PageSwitcher 手写模拟）+ 三目标映射地道（React Router v7 / GoRouter / hash）；批次 1 范围（spec:104-105 已对延期项应用 YAGNI；编译器内建焦点陷阱用户裁定；语料全批次最强）；i18n ICU 子集（批次 5）可辩护（真实三目标映射 react-intl / gen_l10n+ARB + 计划精选子集 V9/V10/V11/V16 有界——边界必须在批次 spec 内冻结）；reorder/gestures/@copy/@key（批次 5）便宜且有先例（@drag ADR-073；键盘处理器 1:1 映射）。

**Summary Recommendations**:
1. **现在修（小、高杠杆）**: C1 — 提取 run_program_level_passes 消除双登记陷阱（analyse.rs:503-520 + 795-814）。
2. **现在修（先调查）**: C2 — OmniView 手写焦点陷阱换原生 showModal()/cancel 事件，或至少修初始焦点 + Esc-outside 缺口（omniview/view/modal.rs:177-192）。
3. **现在修剪路线图**: 批次 3/5 范围删除容器查询（C3）与 @capture/canvas（C4）；保留 @filePick 与 stdlib 工具。批次 4 @load IR 破坏性变更（C5）延期至真实分页 demo 存在。
4. **流程卫生，机会主义**: C6（range(1,10) 硬编码——注意可能已被 B1 Task 0 修复）、C7（批次 2 时保留名 helper）。
5. **抵制追溯**: 不提前合并 ElementIR 字段进 ElementIntentIR（C8）；批次 5 计划共享 struct 是中性变更非胜利。

**Deferred Decisions（→ 触发条件）**: 容器查询 → 真实卡片级响应式断点 demo；@capture/相机权限模型 → 真实需相机/麦克风的 .omni 组件，语料出现后才 spec 权限语义；Canvas ctx DSL → 真实绘制组件；@load 分页/无限滚动 IR 破坏 → 真实 list + load-more demo（stdlib debounce/throttle/polling 现在继续）；tooltip → 已延期（批次 5 交互归组评估；React 无规范形状）保持延期；虚拟滚动 → 已延期（后议项 §3.3）保持延期；完整 ICU（plural/gender/select）→ V9/V10/V11/V16 冻结子集外的真实语料需求。

**Verdict**: **否——语义层架构不过度设计；范围适当 + 可修剪的路线图尾部**。已发货机器（语义墙、登记册、意图 IR、批次 1）全部合理：每层多消费者、分阶段文档化、历史验证设计选择、批次 1 语料驱动。登记册/门禁纪律是用户决定保留的有意治理。倾向过度设计的条件：(a) 批次 3-5 无语料推进容器查询、@capture、canvas、分页 IR 破坏；(b) 双登记与焦点陷阱项（C1/C2）经接下来 4 批次原样积累；(c) per-batch validate pass 继续增长而批次 5 合并本能未应用到过程（共享 helper 如保留名检查）与 IR。今天皆不成立；评审价值在于批次 2 开始前命名它们。

### Security Specialist Review

**Perspective**: 从编译器进入语义 DSL 时代的代码注入面、消毒器充分性、以及安全架构是否需要重构的角度评审。对照实际管线（lexer → parser → semantic → codegen → consumers）验证，以批次 1 spec（@modal/showToast/@skeleton/@errorBoundary）与批次 2 路由 spec 为前瞻镜头，交叉核对 ADR-027/028/039/054/056/072 区分有意设计与真实缺口。

**Key Observations**:
1. **架构收敛于健全可验证模式**：语义层 V-rule 验证（硬错误）+ 单一来源发射转义（shared sanitize.rs/json.rs/expr.rs）+ 失败关闭消费者（poiesis compile-api.ts:35、compileForDebugWindow rustCompiler.ts:316、CLI、WASM 信封旗标 omnilang-wasm/src/lib.rs:171）。唯一未验证路径——codegen 在 error IR 上运行（omnilang/src/lib.rs:344，generate() 372-386 不论诊断与否都跑）——在每个消费者端一致门控，端到端验证而非假设。
2. **批次 1 表面发货安全**：Flutter 构造性安全（@skeleton 形状在 flutter/view.rs:1852 被*比较*而非插值；modal 内容 ident-only；@errorBoundary 1880-1916 丢弃 fallback 槽）。React/OmniView @skeleton 形状与 toast kind 是 V10/V6 白名单门控的裸插值——纵深防御缺口，非可利用注入。
3. **一个当前可达的消毒器绕过存在**：`\u{1}javascript:` 同时击败两个 URL 消毒器。lexer 解码 \u{1} 至 U+0001（lexer/mod.rs:446-474），管线无阶段拒绝字符串值中的控制字符，两消毒器只剥 \n\r\t——比浏览器 scheme 解析前应用的 WHATWG「前导 C0 控制与空格」剥离窄。同族绕过已被修两次（CR/LF/TAB：ADR-031、ADR-039 D3）；修复未泛化到完整 C0 类。
4. **注入语料滞后表面增长**：855 元组语料（19 载荷 × 9 插入点 × 5 后端，ADR-056）是 v2 时代快照。批次 1 spec §7（行 315）与批次 2 spec 任务 5（行 427）只加 parse/validation 元组——无注入形状载荷穿过新发射表面（skeleton shape、toast kind、未来 navigate/route 字面量）。
5. **Tauri 侧是有意文档化**：ADR-072 直接加载（调试窗 (0, eval) 编译 JS）是用户决策，爆炸半径有界（event-only capabilities、无命令面；信任边界 = 可信开发环境 ADR-028）。使编译期消毒在调试工作流**承重**——正是 C0 修复在那里重要的原因。

**Strengths**: 单一来源转义架构（is_dangerous_url/sanitize_css_value/sanitize_css_helpers_js 共享发射器——runtime.rs:283-292 文档化三副本漂移收敛、per-target 字面量转义 json.rs 含 Dart $ 转义、共享事件 canonical 表——历史跨后端漂移结构性消除）；每个抽样发射点验证转义纪律（React a11y attrs react/view.rs:2322-2341 带显式引号突围威胁模型注释、React 文本节点 :1793 逗号操作符突围文档化 :1780-1782、OmniView innerHTML 模板字面量 view/mod.rs:91-97、属性模板、JSON 对象键 json.rs:146-158）；纵深防御分层（四 web 后端共享编译期静态 URL 检查 react/view.rs:752/omniview/element.rs:183/dioxus/view.rs:35/vue/template.rs:49 + 运行时消毒器 + CSP（OmniView standalone）+ 失败关闭消费者）；注入语料带模式缺失断言（855 元组 + 45 控制编译已捕获 5 个 CRITICAL 类注入——className 突围、实体解码重闭合、verbatim 文本兜底、Dart $ 插值、role 注释 */ 转义 ADR-056 §4——真网非复选框）；构造性安全（属性名与信号名是 parser ident expect_ident——setAttribute('{name}') 注入语法上不可能；语义域 navigate prop 注入面批次 2 名字碰撞门控 V7/V8）。

**Concerns**:
- **MEDIUM-1 — C0 控制字符绕过两个 URL 消毒器（当前可达）**。sanitize.rs:148-159（is_dangerous_url 剥集合仅 \n\r\t :154）；omniview/runtime.rs:169-186（__omni_sanitizeUrl 同 :175）。编译期检查是文档化的供应链防御（sanitize.rs:143-146 指名威胁模型；ADR-027 D4、ADR-039 D1）。组件带 `@href="\u{1}javascript:alert(1)"` 编译**成功**（零诊断——无管线阶段拒绝字符串值控制字符；lexer 解码 \u{1} lexer/mod.rs:446-474；验证 crates/omnilang-core/src 无控制字符校验）、verbatim 发射、浏览器 URL 解析器 scheme 解析前剥 U+0001 → 点击执行。运行时路径等价：app 绑定用户文本到 href（合法表单模式）让终端用户粘贴 \u{1}javascript: 穿过运行时消毒器。CSP 只缓解 OmniView standalone 输出；React 生产输出与 ADR-072 调试窗无 CSP。**具体利用**：.omni 组件 @href="\u{1}javascript:alert(document.cookie)" → 作为组件包发布 → 受害者 app 无警告编译 → 部署 React 输出点击执行。同已修 CR/LF/TAB 绕过类（ADR-031、ADR-039 D3）——修复没覆盖完整 WHATWG 剥集合。修复：两消毒器过滤放宽至全部 C0 控制 + DEL + 空格（`!c.is_control() && !c.is_whitespace()`）（控制字符 URL 非法，整串过滤安全）+ 镜像 sanitize.rs:398-411 的单测 \u{1}/\u{0B}/\u{7F} 前缀。S。
- **MEDIUM-2 — 注入语料扩展仅语法型；新发射表面飞出网外**。批次 1 spec §7（:315）与批次 2 任务 5（:427）只扩展 parse/validation 元组；injection.rs:1201-1223（V6-V10 测试用普通非法值 "bogus"，非引号重闭合/突围载荷）。语料是捕获 5 个 v2 CRITICAL 注入的机制。每新批次加字符串发射表面（skeleton shape、toast kind、批次 2 navigate 路径字面量、@query 默认值、路由表）V-rule 门控但发射未转义且无语料载荷覆盖。下个真实注入恰经此缺口发货。修复：注入形状语料元组（引号重闭合、反斜杠、模板字面量、实体、控制字符载荷）成为每批次 spec §7 强制部分，每新发射表面在 emission_surface.rs 注册带 Cover 枚举。M。
- **LOW-3 — @skeleton 形状裸插值（两后端）**。omniview/view/element.rs:955-966（形状裸进 HTML class 属性 :963）；react/view.rs:489-493（形状裸进 styles["omni-skeleton--{shape}"] 内 JS 字符串字面量 :492）。V10 形状白名单（validate_feedback_modal.rs:180-195）是*完整*防御——合法编译只 text/line/circle/card 达 codegen。V10 被绕过（语义 bug、未来重构）且消费者丢 success 门控时裸插值才可利用。纵深防御：`@skeleton="card\" onclick=\"alert(1)"` 会在 error-IR 输出中突围 class 属性（OmniView）或 JS 字符串（React）。修复：element.rs:963 escape_html(shape)、react/view.rs:492 escape_literal_str(shape, '"')——各一行，保持 V10 门控为主防御。S。
- **LOW-4 — Toast kind 裸透传（V6 门控诚实降级）**。shared/expr.rs:538-540（React kind: "{kind_str}"）；omniview/expr.rs:530-533（__omni_toast(..., "{kind_str}")）。Flutter Unknown → None（expr.rs:563）省略安全。V6 硬错误非白名单 kind；透传是 error-IR 路径的刻意诚实降级（shared/expr.rs:536-537 与 omniview/expr.rs:528-529 注释文档化 ADR-054 原则）。error-IR 输出中 `kind: "x");alert(1);//"` 会注入 JS。运行时侧安全（className 赋值 expr.rs:31 无 HTML 解析）。全部消费者今日丢弃 error-IR 输出。修复：shared/expr.rs:540 与 omniview/expr.rs:533 format! 前 escape_literal_str(raw, '"')。S。

**Recommendations**: Immediate（S 各）— 两 URL 消毒器放宽控制字符剥离（MEDIUM-1）+ 单测 + 语料元组；四处发射点转义 skeleton shape 与 toast kind（LOW-3/LOW-4，4 个一行变更）。Short-term（M）— 批次 1 §7 与批次 2 任务 5 语料清单加入每新表面注入形状载荷；加 codegen 审查清单项（新发射表面 → 登记册条目 + 语料元组）；加固 error-IR 不变量：debug-assert/回归测试断言硬错误诊断时无可消费文件（或反转契约——硬错误存在时跳过 codegen——未转义 error-IR 输出停止存在）。当前设计只在每个未来消费者记得门控时正确。**批次 2 navigate() 设计**：spec 无 URL-scheme 验证（V7/V8 仅名字碰撞规则）。要求 navigate 目标与深链同步（C4）落地前穿过 is_dangerous_url 或 internal-path-only 规则；OmniView hash 路由器绝不写未验证值的 location.href。现在设计便宜，落地后贵。Long-term（L）— 从 emission_surface.rs 登记册生成语料元组（新表面不能落网外）；文档化 React 生产输出 CSP 期望（宿主 app 责任；调试 HTML 已警告 utils.rs:221-226）。

**Risks**: MEDIUM-1 供应链 XSS 进部署 React 输出（ADR-027 D4 为之建立的精确场景）+ 绑定用户文本到 URL 属性的 app 运行时 XSS（类已修两次——每修复便宜、失败模式重复）；MEDIUM-2 下批次落地 v2 className/text/entity 类注入无回归网捕获；批次 2 navigate 无 scheme 验证落地 → 前 ADR-039 React href="javascript:" verbatim 缺口复发（首次是 CRITICAL 类发现）。

**Verdict**: **否——安全架构无需重构**。单一来源转义架构（shared sanitize.rs + json.rs + shared expr.rs）是语义 DSL 时代正确形状——语义层 V-rule 门控 + 失败关闭消费者是健全的 belt-and-suspenders 设计，每批次 1 表面验证安全或 V-门控。重构加风险不闭合任何已识别缺口。三个重访条件：(a) 消费者丢 success 门控；(b) 批次 2+ 表面积累更多无界域（URL、i18n 消息——白名单覆盖不了；**必须发射时转义，不只验证**——navigate 设计决策是首个测试）的 V-门控裸插值；(c) 语料缺口再持续两批次。

### TypeScript & React Specialist Review

**Perspective**: React 目标是真正有纪律的生产编译器——最impressive 的性质是每个 TypeScript 约束（TS2686/TS2779/TS2304/TS2451）被逆向工程为**发射时规则** + 测试门控账本，而非事后抑制。发现的一个功能缺陷（ToastProvider）是带已知 cap 的文档化债；架构本身线性扩展到批次 2-5。

**Key Observations**:
1. **React 目标 IR 驱动非模板驱动**。每发射模块（view.rs、component/state.rs、component/jsx.rs、component/direct_write.rs、component/store_slices.rs、component/imports.rs）读同一意图模型（ComponentIR、ViewTree、store refs）——新批次 2/3/4/5 原语（React Router、useMediaQuery、dnd-kit）是新发射模块 + hook 门控入口，非既有路径扰动。验证：grep react/ 无 router/media-query/dnd 原语——词汇是刻意扩展的闭集。
2. **两个独立 tsc 门禁带并行账本**。generated_code_validation.rs:275-292（Rust harness 于 fixture 语料，FIXTURE_STRICT_ALLOWANCE——count-capped、陈旧条目失败）与 scripts/verify-react.sh（真实 eslint 门禁于已提交 learn-omniview 构建——eslint src --max-warnings 0 + react-hooks 规则、per-file 豁免、未用豁免失败）。两者强制同一纪律：已知错误码带实例上限、无无界容忍。
3. **WASM 桥 OmniView-only，React codegen 无编辑器侧消费者**。omnilang-wasm/src/lib.rs:580 硬编码 Target::OmniView（compile_multi_to_omniview）；poiesis/src/commands/rustCompiler.ts 只暴露 OmniView 方法；compile-api.ts 默认 target="react" 但总是调 compileToOmniView(source, false)——死表面。React 输出只经 CLI（omnic build --target react）+ 已提交 harness 达用户。
4. **编辑器 TS 管线是画布往返非编译器副本**。poiesis/src/services/codegen/pipeline.ts（564 行）是结构投影（画布元素 → .omni 源），3 阶段干净结构 + 经 @pid 注释的图编译器 fn 体注入——刻意与 Rust 编译器分离。架构分离健全；唯一跨层契约是 9 字段诊断信封。
5. **豁免债集中且骑命名 deferral**。TS7006 条目（MotionDrag ×15、MotionCard ×6、MotionScroll ×2——generated_code_validation.rs:279-292）全来自发射回调的 implicit-any 参数，延期至 typed-state 工作（#67/#49）。账本小、命名、count-capped——受管理债非腐坏。

**Strengths**: hook 导入门控纪律（component/mod.rs:296-348——每发射条件镜像进 React import 列表：useState 门控 has_mutable_states || form_intent || load_intent || has_toast、useLayoutEffect 专门门控 modal_refs :340 带注释解释无视图组件为何不能导入、useRef 门控 refs；每条目引用防的 TS2304 轮次——「生成代码必须 import 干净」的正确解法）；Zustand getter 物化是真修复非 workaround（mod.rs:497-580：getters 以 Kahn 拓扑序 __getters helper 计算值发射、__set 每合并重跑 __getters :413-414、{Name}Signals = Pick<{Name}State, ...> 避免重声明 struct 类型——直接修 accessor-freeze bug BUG-C，浏览器验证）；TS 约束工程穷尽且文档化（needs_react_default TS2686 imports.rs:126-128——React.Fragment @for/React.memo 别名/ErrorBoundary class 零 hooks 也正确触发默认导入；! vs ?. TS2779 direct_write.rs；@for 可迭代 IIFE 单求值 view.rs:1661-1669 修每渲染双求值）；平台原生模式零第三方运行时依赖（@drag 原生 Pointer Events + setPointerCapture view.rs:2394+、@modal 受控原生 <dialog> + 遮罩点击关闭 + aria-modal + 合成 omni-modal-title id view.rs:262-335——生成 app 只依赖 react + zustand，使 eslint 门禁可处理、输出真可人工编辑）；硬诊断优于静默丢弃（组件实例 @drag 发警告诊断 view.rs:504-511）；两写通道消毒对等（sanitizeCss 在 D1 直应用路径与 B2 effect 路径同应用 direct_write.rs:134-140/224-226——关闭直写跳过 effect 路径消毒器的经典绕过）。

**Concerns**:
- **HIGH — ToastProvider 功能损坏（文档化债，cap=1）**。jsx.rs:49-51 把整个返回树包进 <ToastProvider>；component/mod.rs:583 发射 `function ToastProvider()` **零参数**从不渲染 {children}——只 toast host div。缺陷文档化于 generated_code_validation.rs:279-291（TS2559 包点，cap=1）带显式注记「the wrapped view would be swallowed entirely」。任何用 showToast 的组件静默渲染只有 toast host——整个 UI 运行时消失而 tsc 恰持一个已知错误。今天无 fixture 练习它（cap=1 因此成立）。修复（S）：`function ToastProvider({ children }: { children: ReactNode })` + 渲染 {children} 与 <div className={styles["omni-toast"]}> host 并排。ReactNode 导入已存在（imports.rs:79-88 slot 机制）——两行发射变更 + 移除 TS2559 账本条目。**在任何批次 2/3 fixture 用 toast 前做**。
- **MEDIUM — WASM 桥无 React 目标导出；编辑器 API 表面误导**。omnilang-wasm/src/lib.rs:580（compile_multi_to_omniview 硬编码 Target::OmniView；全部导出 #[cfg(feature = "codegen-omniview")]）；poiesis/src/commands/compile-api.ts（默认 target="react" 但 body 总是 rustCompiler.compileToOmniView(source, false)）。双面：(1) React 目标——带 tsc/eslint 门禁的生产目标——只被 CLI + 已提交 harness 练习；产品编辑器无法预览 React 输出，生成 React 的运行时行为（toast 除外）从产品不可验证。(2) target="react" 默认是死表面，误导下个开发者认为 React 编译路径存在。修复（M）：想要 React 预览则加 WASM compile_multi_to_target(source, target) 导出复用既有信封 struct（OmniView 调用者不变）；否则删 compile-api.ts 的 target 参数并文档化 OmniView-only 契约。两者皆可——当前态（暗示 React 支持、实际 OmniView-only）最差。
- **MEDIUM — TS7006 implicit-any 残留骑 deferral #67/#49 无过期检查**。generated_code_validation.rs:279-292——MotionDrag ×15、MotionCard ×6、MotionScroll ×2（TS7006）、toast/ToastDemo TS2559 ×1。fixture-path + error-code 上限，但无任何东西在 typed-state deferral 落地时强制重评。implicit-any 在发射回调是唯一静默弱化生成代码类型安全的错误类（对编辑它的终端用户）——账本防增长但不驱动归零。修复（S-M）：#49 落地时加 TS7006 类总数单调递减断言（新总数 ≤ 旧总数，或显式「批次 N 前归零」cap）。
- **LOW — 双豁免账本必须手动同步**。scripts/verify-react.exemptions（learn-omniview 构建 eslint 门禁）vs FIXTURE_STRICT_ALLOWANCE（generated_code_validation.rs:275-292 fixture 语料 tsc 门禁）。allowance 表注释显式指示保持两账本同步。修复（S）：单一共享豁免 JSON 两门禁消费，或断言两账本共享 fixture 路径一致的测试。

**Recommendations**: Immediate（S）— 修 ToastProvider（mod.rs:583 + jsx.rs:49-51 两行 + 删 TS2559 账本条目 + 重生成 harness）；移除 compile-api.ts 误导性 target="react" 默认（或导出落地后接线）。Short-term（M）— 裁决 React 预览问题：WASM 层加 compile_multi_to_target 或显式文档化 OmniView-only 产品契约；统一两豁免账本为一共享产物。Long-term（S/M）— 批次 2-5：每新原语（Router/useMediaQuery/dnd）是新发射模块跟随既有模式——IR 意图 → 发射模块 → hook 导入门控条目 → 豁免账本条目（若豁免）。无跨切变更。每原语预算约 @modal 模块大小（~75 行 + 测试）；typed-state deferral（#49/#67）落地时加 TS7006 类债单调递减断言。

**Risks**: 未来任一批次 toast 使用 fixture 静默发货空白组件（TS2559 cap 保持 tsc 绿而 UI 被吞——唯一潜在 React 目标*功能*缺陷）；编辑器 OmniView-only 编译路径根深蒂固为「the」编译路径，React 预览后期要求触 C FFI + TS 包装层的 WASM 导出加法；两门禁账本漂移侵蚀「count-capped、stale-detected」使豁免系统可信的性质。

**Verdict**: **无需重构**。React/TS 架构健全，最强部分——hook 导入门控、IR 驱动发射、count-capped 门禁带陈旧条目纪律、平台原生原语——恰是使生成代码过 tsc strict + eslint-plugin-react-hooks 且保持可人工编辑的性质。批次 2-5 线性扩展：每新原语是有界自包含发射模块。条件：(1) 任何 toast 使用 fixture 存在前修 ToastProvider；(2) 刻意裁决编辑器侧 React 预览问题（导出或文档化）；(3) 保持双账本纪律或统一。皆不需架构变更。

### AI Engineer Review

**Perspective**: 评估 AI agents（今日 Claude Code，明日 LLM 作者）与 OmniLang 编译器及其文档语料的交互——作为 grounding 基质、自校正回路、评估基质。编译器自身无 LLM 调用；评审覆盖其周围的 AI 面对表面。

**Key Observations**:
1. **语料是真正的三重基质**：learn-omniview 展示（正例，demo-coverage 门控）+ BNF 语法文档（显式写给「实现 parser 的 AI agents、工具构建者、语言服务作者」14-grammar-lang.md:3）+ llm_error_corpus.rs（30 个负例 fixture 带精确位置钉 + 正控制）。五门禁使编译器为 ground truth，文档可作为 LLM grounding **被信任**——doc-code 漂移问题被机械解决，这是稀缺部分。
2. **诊断质量已达 agent 级**：类型化码（DiagnosticKind error.rs）、结构化 suggestion 字段、迁移提示族（E_PAUSED_REMOVED 等带 verbatim 中文替换）、近邻名建议（编辑距离 ≤3）。迁移指南（migration-v1-to-v2.md）把每个删除语法变成硬诊断 + 提示——「诊断即文档」。omnic analyse 端点（omnic/src/api.rs:80-140，JSON severity/code/message/file/line/col/suggestion）是 agent 需要的精确自校正接口：编译 → 读结构化诊断 → 修 → 重编译。
3. **SDD 循环（spec → plan → per-task subagent → review）是 AI 编排架构**：计划显式写给「agentic workers」带 REQUIRED 子技能与 spec 表钉的 verbatim 诊断消息——使 58 任务委托可信。五门禁是使子代理输出无需人工逐行重读即可验证的反漂移强制。

**Strengths**: 编译器即 ground truth 门禁纪律（verify-docs.py：文档错误必须在 DOC 修，永不编译器）——DSL 项目中最强的 doc-code 对齐机制，使全部文档语料安全喂 LLM；llm_error_corpus.rs 接近完美评估基质（6 类 × 5 fixture，各钉到 diagnostic-exists + 精确行 + 可行动提示 + 干净编译正控制 omnilang/tests/llm_error_corpus.rs:500-580——自校正能力被确定性测试）；双向白名单断言（verify-semantic-universe.py、verify-vocabulary.py）——LLM 会查阅的语义全集表不能从 parser 漂移。

**Concerns**:
- **MEDIUM — 最佳 LLM 评估资产锁在 Rust 测试内**。llm_error_corpus.rs:40-51（ErrorCase { name, category, src, expected_line, expects_token, hint }）恰是评估 harness 需要的（负例 fixture + golden 期望），但零机器导出——无 JSONL、无旗标、无 CLI。agent 评估循环或基准无法消费而不重写 fixture。修复：加 `#[ignore]` 测试或小二进制把语料 dump 成 JSONL（src/expected_line/token/hint per case）——~20 行，对既有断言零风险。这是未来 LLM 作者评估的最高杠杆工件，且是唯一非机器可读的。
- **MEDIUM — 混合语言诊断语料**。新校验器中文（validate_form_load.rs ~1410 字符、validate_event_names.rs ~460、validate_effect_deps.rs:35「effect 依赖漏掉：」），旧校验器英文（validate_drag.rs:48「@drag requires @ref on this element」、validate_refs.rs:63、validate_store_access、validate_inline、validate.rs——零中文）。LLM agent 自校正回路看不一致指令语言；更糟，verbatim 消息测试钉住各校验器作者选择的语言——分裂现承重。修复：canonical 消息字段选一语言（中文是既定方向），机械 pass 迁移 ~5 个英文校验器；英文留 suggestion 级替代（如需要）。经既有 verbatim 测试验证。
- **LOW/MEDIUM — span 覆盖不一致**。Diagnostic::error() 从 span 填 end_line/end_col（error.rs），但校验器 make_diag helpers 硬编码 end_line: None, end_col: None（validate_drag.rs:106-109）——每个语义校验器诊断点单行，parser 诊断给范围。高亮错误的 agents（LSP、编辑器）对恰需修复的诊断获降级精度。修复：span 经 make_diag 传播。小、机械。
- **LOW — grounding 文档只含正确语法**。verify-docs.py 任何不编译块即门禁失败，文档语料 100% 正例；负模式（17 个删除 v1 语法长什么样）只存在于迁移指南散文表。错误语料补偿但它不与作者 agent 会 grounding 的文档同位。LLM 在语料上模式匹配；缺「错」例是幻觉旧语法的已知成因（如重生 paused:）。修复：允许围栏 ```omni-error 块类必须产出钉住诊断——扩展既有门禁，保持编译器即真相。
- **LOW — 正则启发 demo 覆盖审计**。verify-demo-coverage.py 按域正则查用法（域 3 数 @on{name}\s*=）——可假正（注释里写的 @onClick 满足审计）。便宜但是「展示项目是完整正例语料」的最弱门禁。修复：接受限制；可选后加编译基计数器（解析 demo 文件数真实绑定）。

**Recommendations**: Immediate（S）— LLM 错误语料 JSONL 导出（一旗标/测试，解锁每未来评估用途）。Short-term（M）— 诊断语言一致性 pass（中文 canonical，按既有 verbatim 消息测试纪律）；semantic 校验器 end_line/end_col 传播。Long-term（M）— agentic 评估循环——LLM 生成 .omni，对照 llm_error_corpus golden 期望评分，门控通过率。基质已存在；这是接线非建设。

**Verdict**: **无需 AI 集成驱动重构——否/次要**。架构已是异常强 AI 基质：doc-code 对齐机械门控、负例钉住且测试、agent 面对诊断接口（结构化 JSON + 迁移提示）完整。缺口是导出/接线（JSONL、评估循环）与一致性（语言、span）——增量非架构。一切所报按设计是外围的：评审确认编译器核心非 AI 形状且不需要是。

---

## Cross-Cutting Themes（交叉主题——3+ 成员独立命名的发现）

1. **程序级 pass 序列多处手写复制（4 名成员：compiler-architect / rust-expert / maintainability-expert / pragmatic-enforcer）**。analyse_multi（analyse.rs:497-520）、analyse_multi_cached（analyse.rs:789-812）、run_pipeline（lib.rs:113-155）三处逐字复制 4-pass 序列；历史上 validate_event_names 曾漏一处致单文件路径诊断缺失（lib.rs:120-126 自述）。**批次 2 前提取 `run_program_passes` 是最高置信修复**（S，~1 天，行为零变化）。systems-architect 提供反对注记：共享编排不得耦合缓存语义——提取只共享序列不共享缓存路径，测试保留，无冲突。

2. **「零诊断静默退化」类在三个真实位置仍活跃（compiler-architect HIGH / systems-architect MEDIUM / security MEDIUM / maintainability MEDIUM）**。跨文件 required-prop 静默跳过（实证编译零诊断）、hidden 布尔属性退化（B2 bug 类活跃实例）、C0 控制字符 URL 消毒绕过（\u{1}javascript: 可达）、方法表漂移（contains() 跨目标行为分歧、#76 运行时 TypeError 零诊断）。项目自身定义「静默错误 = 雷」，这些是该定义下的活跃雷。

3. **~~单点真实功能缺陷：React ToastProvider 吞视图~~（TS/React HIGH + implementation-strategist MEDIUM-1 独立确认——❌ 实证复核为误报）**。两子代理声称零参数发射、不渲染 {children}、cap=1 TS2559 条目——复核确认提交 6271d5b6 已在评审基线前修复（HEAD 中带 children 参数 + 渲染 {children}），FIXTURE_STRICT_ALLOWANCE 无 TS2559 条目。子代理内容失真案例；无行动项。

4. **编排层 vs IR/codegen 层诊断一致**：全部 11 名成员裁决「IR/codegen 不需要重构」；4 名成员独立裁决「pass 编排层需要一次合并式收口（批次 2 前）」。这是本评审的统一定论。

5. **意图挂载层级异质 + lowering 标记丢失（compiler-architect MEDIUM-3 + domain-expert MEDIUM 关切 1——两名核心域专家同一现象的双层观察）**。元素级意图（drag/modal）带精确 span；组件级意图（form/load）span 回退组件级、V-F1/V-F6 被推入 lowering、消息文本两处复制；表达式级（BuiltinToastCall）第三种形状。域 11 @route 与域 13 @load 深化同为组件级——每个继承 span 退化 + 两层校验分裂。**修复收敛为同一动作**：意图 IR 带 source_span + 组件级意图收集进单一 ComponentIR.intents + 结构性规则/lowering vs 语义规则/passes 命名约定。M，批次 2（域 11）前采纳。

6. **验证网是资产但有三处盲区**：注入语料扩展仅语法型（security）、双豁免账本手动同步（TS/React LOW）、verify 脚本计划前提过期（B5 ENABLED 守卫——implementation-strategist LOW-1 与 pragmatic C6 均发现，且脚本可能已被 B1 Task 0 修复）。domain-expert 补充第四处：verify-semantic-universe.py:6-8 docstring 已陈旧（「Nine」），脚本散文是全模型最佳单一描述却最不可靠。

7. **批次 2-5 路线图尾部需要语料门控修剪（pragmatic C3/C4/C5）**：容器查询、@capture/canvas、@load 分页 IR 破坏性变更为无语料投机项；@filePick、stdlib 工具（debounce/throttle/polling）便宜可辩护。i18n ICU 子集可辩护但有界必须冻结。domain-expert 补充：批次 5 四归组（交互/i18n/媒体/生命周期）无预分配域编号、无放置规则——baseline 需预分配域 14-17 + 文档化分类规则（机制级按语法类别 / 功能级按特性）。

## Trade-offs & Disagreements（分歧显式化）

- **pass 序列共享 vs 测试强制对等**：systems-architect 认为两条入口路径「由回归测试而非代码共享」强制对等是正确取舍（共享会耦合缓存语义）；compiler-architect/maintainability/rust/pragmatic 主张提取共享序列。**无实质冲突**：提取只共享 pass 序列（纯函数），缓存路径保持分离，对等测试保留。聚合结论：提取 + 测试并存。
- **OmniView 焦点陷阱手写 vs 原生 showModal()**：pragmatic C2 建议删除 ~15 行手写 Tab 循环换原生 showModal()/cancel 事件；保留 caveat——showModal() top-layer 放置可能逃出预览容器 transform/clip（ADR-028 调试运行时约束），需先验证。**裁决**：先调查容器约束；至少修初始焦点（现在到 <dialog> 自身而非首个可聚焦子元素）与 Esc-outside 缺口。
- **意图标记 lowering 丢弃**：validate_form_load.rs:1-7 文档化为有意权衡（标记从 IR 视图树剥离）；compiler-architect 指出其为每次新域重复的分裂脑模式 + V-F1 消息文本已两处复制。**裁决**：有意但需收敛——意图 IR 节点带 source_span + 消息单一副本。
- **ElementIR 字段积累**：pragmatic C8 与 implementation-strategist LOW-2 一致——现在别合并（批次 5 ElementIntentIR 非明显胜利）；compiler-architect 建议长期按 types_motion.rs 先例做 per-domain IR 子模块。**裁决**：post-B5 合并审计，不追溯。
- **C FFI 去留**：systems-architect LOW-3 二选一（删 or shim+CI）。无成员反对删除（WASM 覆盖全部消费者）；是否保留留给用户拍板。
- **navigate() URL 校验**：security 建议 navigate 目标落地前过 is_dangerous_url；批次 2 spec 现无 scheme 验证。这是「白名单门控裸插值」模式在无界域（URL）的首次测试——**发射时转义而非仅验证**。
- **B5 ENABLED 守卫前提**：批次 5 计划基于 range(1,10) 硬编码前提；implementation-strategist 验证脚本已自维护（sorted(DOMAIN_TITLES) B1 Task 0）——执行时重读脚本，不照本宣科。

## Prioritized Improvements

**Critical（0-2 周，批次 1 收口内）**:
1. **修复跨文件 required-prop 静默跳过**（compiler-architect HIGH-1，⚙ 实证确认）：`validate_required_props` 从 run_ir_passes 移至 Phase 3b merge 后的程序级相位；加两文件回归测试（跨文件 fixture 断言 E0011 必报）。S
2. ~~修复 React ToastProvider 吞视图~~（TS/React HIGH）——**❌ 误报，已修复**（6271d5b6 早于评审基线）。无需行动
3. **提取 run_program_passes**（4 成员，⚙ 实证确认）：三处编排点调用单一序列 + 管线对等测试（(kind, message, span) 序列断言）。S

**Important（2-8 周，批次 2-3 窗口）**:
4. **URL 消毒器 C0 控制字符放宽**（security MEDIUM-1）：两消毒器 `!c.is_control() && !c.is_whitespace()` + 单测 + 语料元组。S
5. **hidden/autoplay 入 HTML_BOOLEAN_ATTRS + 域 7 断言升级为集合包含**（systems-architect MEDIUM-2）。S
6. **方法翻译三表统一 + 跨目标对等矩阵**（maintainability MEDIUM-2）：shared/expr.rs 方法表为 floor 方法集单一事实源 + per-target 覆盖槽 + contains 案例；关闭 #76/#28/#88。M
7. **MAX_CACHE_ENTRIES = max(50, sources.len())**（performance HIGH）：应用已计划修复（analyse.rs:238）。S
8. **per-pass 脏度门控**（performance HIGH）：analyse_multi_cached Phase 3c-4 每 pass 声明输入域、脏集不交集即跳过。M
9. **意图 IR 节点带 source_span + 组件级意图收集进 ComponentIR.intents + V-F1/V-F6 并入域 pass**（compiler-architect MEDIUM-3 + domain-expert R4 收敛）：消除分裂脑模式重复 + 域 11 @route 继承 span 退化；「结构性规则在 lowering、语义规则在 passes」命名约定替代 per-pass 注释。M，批次 2（域 11）前采纳
10. **画布 success 语义裁决：errors-only + 对等测试**（systems-architect MEDIUM-1）。S
11. **WASM compile_to_omniview 多文件分支路由 cached**（performance MEDIUM）。S
12. **批次 2 navigate() 设计含 URL 校验**（security Rec-5）：落地前过 is_dangerous_url 或 internal-path-only；OmniView hash 路由器绝不写未验证 location.href。M（设计内建便宜）
13. **批次 1 §7 / 批次 2 任务 5 注入形状语料元组**（security MEDIUM-2）：每新发射表面带 quote-reclose/backslash/template-literal/entity/control-char 载荷。M
14. **vue/dioxus 意图字段 E_CODEGEN 门禁**（compiler-architect MEDIUM-4）：随批次 3「静默跳过诊断先行」修复扩展。M
15. **内建函数表 + 泛型保留名强制（V8 泛化）**（domain-expert 关切 3）：semantic/builtin_fns.rs 单一表，域 11 navigate/back 前置。M（批次 2 计划内标记）
16. **登记册治理同步**（domain-expert R1/R2/R3）：verify-semantic-universe.py:6-8 docstring 更新 + 两层分类规则一句化；CONTEXT.md 对照表域 10 更新；baseline §8 矩阵符号 ↔ 登记册状态桥接行；批次 5 预分配域 14-17。S

**Nice-to-Have（2-6 月）**:
15. 批次 2-5 前裁决 C FFI 去留（删 S / shim+CI M）；b. 批次 3-5 语料门控修剪（容器查询/@capture/canvas 延期、@load 分页 IR 破坏待 demo）；c. 诊断语言一致性 pass（中文 canonical）；d. semantic 校验器 span 传播；e. llm_error_corpus JSONL 导出；f. 双豁免账本统一；g. B5 执行重验 ENABLED 守卫前提；h. ElementIR post-B5 合并审计；i. tests.rs 增量切分（触及即切）；j. TS7006 单调递减断言（#49 落地时）；k. IRProgram::default() version 对齐 "1.0"；l. OmniView 焦点陷阱调查（showModal 或至少初始焦点 + Esc-outside）；m. reserved-name helper（批次 2 下个实例时）；n. 模态标题检测三处提升共享（L）；o. 生成语料元组自 emission_surface.rs 登记册（L）。

## Technical Debt Assessment

| 区域 | 当前债级 | 趋势 | 影响 | 注记 |
|---|---|---|---|---|
| 程序级 pass 编排 | Medium | 增加（批次 2-5 每新 pass 3 处编辑） | High | 提取 run_program_passes 即清（S） |
| React 豁免账本 | Low | 减少（287→29→24 条活跃） | Low | 受管理、陈旧即失败、TS7006 骑 #67/#49 |
| 方法翻译表 | Medium | 稳定（3 表，已 3 缺陷） | Medium | #76/#28/#88 开；统一 + 对等矩阵关闭 |
| 注入语料 | Low | 稳定（855 元组 v2 快照） | Medium | 新表面无载荷——每批次强制注入形状元组 |
| C FFI | Low（死重） | 腐坏 | Low | 无 cdylib 生产者、6 月过期 DLL——删或 shim |
| tests.rs 巨型文件 | Medium | 增加（每批次增长） | Low | 按触及增量切分，勿预防性 |
| ElementIR 字段积累 | Low | 增加（每批次 1-3 字段） | Low | post-B5 合并审计 |
| 诊断语言混合 | Low | 稳定（新中文旧英文） | Low | 中文 canonical pass（M） |
| 文档五层同步 | Medium | 稳定（门禁强制） | Medium | CONTRIBUTING 映射文档最高杠杆 |

## Architectural Evolution

### Current vs Target Architecture

**当前**: 语义原语 → IR → codegen 三层；语义层 25 pass（per-file 9 + 程序级 4 + lints）；三 P0 目标 + 7 非 P0；验证网（注入语料/门禁/浏览器/登记册断言）；能力基线 5 批次执行中（10 域 → 14 维）。

**目标（批次 5 后）**: 同一三层 + ~14 能力维（导航/响应式/i18n/媒体/交互/生命周期入语言）；编排层单一声明序列（run_program_passes）+ per-pass 域元数据；意图 IR 带 span、per-domain 子模块化（types_motion.rs 先例）；非 P0 目标 E_CODEGEN 门禁；语料门控的路线图尾部。

**差距**: 编排层（提取 S）、跨文件校验放置（S）、意图 IR span（M）、非 P0 门禁（M）、脏度门控（M，触发式）、IR 合并（post-B5）。

### Migration Path

批次 1 收口内：提取 run_program_passes + 对等测试；validate_required_props 移到程序级；ToastProvider 修复或登记；B1 Task 5 提交 + Task 8 完成 + 锚点绿。批次 2 开始门 = 批次 1 关闭。批次 2-3 窗口：消毒器 C0、布尔属性集、方法表统一、navigate URL 校验、注入语料扩展。触发式（500 文件 / 30ms）：脏度门控 → copy-on-write 模块 join。post-B5：IR 合并审计、tests.rs 切分、诊断语言统一。

## 发现验证结果（2026-08-10 实证复核）

评审完成后，主 agent 对全部关键发现逐项亲自验证（读源码 + 构造最小复现实证），避免子代理内容失真。方法：位置核对 → 逻辑链路核对 → 关键项运行时实证。

| # | 发现 | 验证方法 | 结果 |
|---|---|---|---|
| HIGH-1 | 跨文件 required-prop 静默跳过 | **运行时实证**：a.omni import b.omni，`<Button />` 缺 required `title` → `Compiled 4 file(s)` 零诊断；同文件对照 → `required prop 'title' not provided` | ✅ **确认**（缺陷真实，今天可复现） |
| HIGH-2 | ToastProvider 吞视图 | 读 HEAD 源码 + git 历史 + FIXTURE_STRICT_ALLOWANCE | ❌ **误报**——6271d5b6 已修复（早于基线 HEAD）；HEAD 中带 `{ children }: { children: ReactNode }` 并渲染 {children}；无 TS2559 cap 条目 |
| HIGH-3 | 程序级 pass 三处复制 | 读三处调用点（analyse_multi:497-520 / cached:789-812 / run_pipeline:lib.rs:113-155）+ lib.rs:120-126 历史注记 | ✅ **确认**（注释逐字相同） |
| C0 消毒绕过 | 消毒器剥集合 + lexer 转义 + 浏览器行为链路 | 读 sanitize.rs:148-159 + runtime.rs:169-186 + lexer read_unicode_escape（`\u{1}` → U+0001，无控制字符拒绝） | ✅ **确认**（利用链完整） |
| hidden 双源漂移 | parser 关键字集 vs codegen 属性集 | 读 parser/view/element.rs:38-61（15 项含 hidden/autoplay）+ element.rs:19-33（13 项无 hidden）+ 白名单:126-136（无 hidden） | ✅ **确认** |
| contains() 分歧 | 两后端方法翻译 | 读 react/expr.rs:2653-2663（含字符串 includes 分支）vs omniview/expr.rs:359-366（无） | ✅ **确认**（React 注释自认差异） |
| 方法表三处 + #76/#28/#88 | 三表存在性 + deferral 台账 | 读 shared/expr.rs:835 + deferrals.md:264/341/406 | ✅ **确认** |
| MAX_CACHE_ENTRIES=50 | 常量与逐出循环 | 读 analyse.rs:238/:711 | ✅ **确认** |
| IRProgram::default "0.1" | Default 实现 vs 生产构造点 | 读 ir/types.rs:1145-1152 | ✅ **确认** |
| C FFI 无 cdylib 生产者 | crate-type + no_mangle | 读两 crate Cargo.toml（均 rlib-only）+ ffi.rs #[no_mangle] | ✅ **确认**（死接口，6 月过期 DLL 未复验） |
| 画布 success=false on warnings | 三处 success 门控 | 读 omnilang/src/lib.rs:185/:217 vs :464 | ✅ **确认**（errors-only vs warnings 参与） |
| 意图 IR 无 span | 意图 struct 定义 | 读 types.rs:734/1026/1046（Modal/Form/LoadIntentIR 均无 span 字段） | ✅ **确认** |
| vue 静默忽略意图字段 | grep 意图字段引用 | vue/template.rs 零引用 form_intent/load_intent/modal/drag | ✅ **确认**（latent——omnic 只编 P0） |
| WASM 多文件分支未缓存 | 路由核对 | omnilang-wasm:674-675 → compile_multi_internal（未缓存）vs :578 cached | ✅ **确认** |
| universe 脚本 docstring「Nine」陈旧 | 读脚本头部 | verify-semantic-universe.py:4-6 | ✅ **确认** |
| sorted(DOMAIN_TITLES) 自维护 | 读脚本循环 | :419（B5 ENABLED 守卫前提过期确认） | ✅ **确认** |
| Flutter 锚点暂缺 + B1 Task 5 未提交 | git status + 脚本注释 | flutter/{mod,ref,view}.rs 均为 M（未提交）；:711-713 自注 | ✅ **确认** |

**验证方法论注记**：全部「确认」项中，HIGH-1 为运行时实证（fixture 复现），其余为源码级链路核对；「❌ 误报」项（HIGH-2）暴露子代理内容失真的具体机制——两子代理引用了同一不存在行号区域与不存在的 cap 条目（FIXTURE_STRICT_ALLOWANCE 现仅 3 条 TS7006），判定为读到过时/幻觉内容而非恶意误报。其余发现未发现类似失真模式（位置、行号、代码内容均与源码一致）。

## Conclusion

**统一裁决：OmniLang 编译器不需要架构重构。** 语义级 DSL 三层架构经批次 1 实证成立，批次 2-5 的 5 个新域是既有模式（semantic 模块 + 3 调用点 + 指令函数 + 加法 serde-safe IR 字段）的垂直切片；意图 IR 模式、单一来源转义、floor 门禁、验证网全部被 11 名成员确认为正确形状。**需要的是批次 2 前的一次「编排层收口」**：程序级 pass 序列三处复制 → 单一 run_program_passes（4 名成员独立确认 + 主 agent 实证复核，含历史分歧事故证据）；跨文件 required-prop 校验放置错误（⚙ 运行时实证——唯一「今天产出错误输出」的缺陷，跨文件零诊断 vs 同文件 E0011）。**主 agent 实证复核修正 1 项误报**：React ToastProvider 吞视图（TS/React + implementation-strategist 声称）已在评审基线前修复（6271d5b6），从行动清单移除。同时：3 个 HIGH 性能/安全项（缓存尺寸、脏度门控、C0 消毒绕过——⚙ 均确认）与路线图尾部语料门控修剪（容器查询/@capture/canvas/分页 IR 破坏延期）应在批次 2-3 窗口落地。

**保留架构的条件**（systems-architect 四条件 + 各成员裁决并集）：(1) codegen 保持纯翻译，floor 门禁保持最小（2 个）；(2) 登记先行纪律保持（语义全集 → BNF → 白名单）；(3) 两个 MEDIUM 有意设计冲突点（画布警告语义、布尔属性双源）被解决；(4) 功能设计级四域落地时 IR 特化跟随 types_motion.rs 先例；(5) 批次 2-5 尾部按语料门控（无真实语料不加 IR 破坏性变更）。

## Appendices

### A. Review Methodology

11 名成员并行子代理独立评审（各读 ADR/CONTEXT.md 基线 + 指定代码面），全部被提示「发现可能是有意设计，先核对再定性」；TS/React 与 domain-expert 首次因上下文溢出崩溃后以分块阅读纪律重发。聚合：交叉主题（3+ 成员命名）、冲突显式化、严重性分桶。

### B. 主要文件索引（发现定位）

- `crates/omnilang-core/src/analyse.rs`（100-185 run_ir_passes；497-520 / 789-812 程序级三处；238 MAX_CACHE_ENTRIES；629/768/790/795-815 每击键全项目阶段；845-927 缓存诊断重放测试）
- `crates/omnilang-core/src/lib.rs`（85-169 run_pipeline；118-136 程序级序列；185/217-220 画布 success；249 export_bindings）
- `crates/omnilang-core/src/ir/types.rs`（772-841 ElementIR；1145-1152 Default "0.1"）
- `crates/omnilang-core/src/semantic/validate_props.rs:18-29`（per-file component_map）
- `crates/omnilang-core/src/codegen/sanitize.rs:148-159`（is_dangerous_url C0 缺口）
- `omnilang/src/codegen/omniview/view/element.rs`（19-33 布尔属性；126-136 白名单；955-966 skeleton shape）
- `omnilang/src/codegen/react/component/mod.rs:583`（ToastProvider 零参数）、`jsx.rs:49-51`（包点）
- `omnilang/src/codegen/react/expr.rs:2610-2704` / `omniview/expr.rs:322-382` / `shared/expr.rs:851-966`（方法三表）
- `omnilang/src/codegen/omniview/view/modal.rs:177-192`（手写焦点陷阱）
- `omnilang-wasm/src/lib.rs:580/675`（OmniView-only、未缓存分支）
- `scripts/verify-semantic-universe.py`（:419 sorted(DOMAIN_TITLES) 自维护；:713 Flutter 锚点待定）
- `omnilang/tests/generated_code_validation.rs:275-292`（豁免账本 + ToastProvider 注记）
- `omnilang/tests/llm_error_corpus.rs`（AI 评估基质，无 JSONL 导出）

### C. 待用户拍板项

1. **C FFI 去留**（删 vs shim+CI）——systems-architect LOW-3
2. **React 预览**（WASM 加 compile_multi_to_target vs 文档化 OmniView-only 产品契约）——TS/React MEDIUM
3. **批次 3-5 语料门控修剪**（容器查询/@capture/canvas 延期；@load 分页 IR 破坏待 demo；@filePick + stdlib 工具继续）——pragmatic C3/C4/C5
4. **OmniView 焦点陷阱**（showModal() 调查 vs 最小修初始焦点 + Esc-outside）——pragmatic C2
5. **批次 1 收口执行顺序**（Task 5 提交 → 锚点绿 → Task 8 demo 迁移；poiesis 变更分离提交）——implementation-strategist
