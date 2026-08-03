# Architecture Review v2: OmniView 语义级语法 → React/Flutter 地道代码可编译性（v2 实现后对齐评审）

**Date**: 2026-08-03
**Review Type**: Feature（对齐评审——前次评审 Follow-up 承诺的「Immediate 项完成 + ADR 决策后」重审）
**Reviewers**: compiler-architect (Dr. Lin Wei), typescript-react-specialist (Emma Rodriguez), flutter-reviewer, domain-expert, pragmatic-enforcer, maintainability-expert, rust-expert, performance-specialist, implementation-strategist, systems-architect, security-specialist, ai-engineer, doc-updater（13 名，与前次同阵容）
**Baseline**: HEAD 34e1f0e3（v2 全量实现后：43 任务 × 8 阶段、73 commits、ADR-056）

**评审问题**: v2 重设计（10 语义域、删除一批语法、立起验证网）之后，当前 OmniView 语义级语法是否能编译为地道 React (TS strict + Zustand v5 + React 19) / Flutter (Dart + Riverpod 3.0) 代码？不能的语法该修改/删除。

---

## Executive Summary

**v2 的宣称在 HEAD 上全部实测为真**——这与前次评审（「宣称与事实不符」）形成决定性对比：React 门禁真实运行（166 tsc + 121 eslint 全豁免、零 stale 条目）、Flutter 展示项目 dart analyze 0 错误 0 警告（真实 192s 全量运行）、docs-gate 232 块 0 失败、855 元组注入语料真断言、proptest 真断言 + codegen fuzz 真跑、wasm 测试基建 21/21、删除的语法（value={sig}/paused/@native/typed slots/theme=/auto-bind/SetValue）全部硬诊断 + 迁移提示 + 零语料残留。**三层架构、IR 语义承载、门禁基础设施、删除纪律四个层面 v2 兑现了承诺。**

然而，对评审问题的直接回答仍是**「编译通过 ≠ 行为正确」——React/Flutter 两个 P0 目标的行为层尚未达到 v2 声称的「地道」**：

- **React：2/9 标签页运行时整体卸载**（BUG-A `let mut` → `const`，IR 丢弃 mut 标志；19/36 Playwright 测试 fixme 跳过，UI/Language 两标签页零覆盖）；inline store 生成冻结状态 + `CartStore` 未定义（BUG-C）；match/block 尾表达式静默丢值（DataFetching 成功路径不可达）；实体转义误用于 JS 字符串（界面显示 `&amp;amp;`）。
- **Flutter：旗舰 demo 核心循环 `TodoStore.add()` 运行时 TypeError 崩溃**（struct 字面量 → Dart Map<String,Object> 带引号键 → copyWith cast 失败）——**所有门禁均不可见**（dart analyze 静态、零运行时测试）；`bind:value` 在非 ref 元素上 14 处静默丢弃（add-todo/表单验证/设置全死）；主题选择器不可点击（Container 无 onPressed 映射）、sepia/highContrast 主题不可达；`@in-view`/`@focus` 触发器零警告静默不接线（OmniInViewWidget 从不实例化）；`<textarea>` 渲染为空 Container、`disabled` 静默丢弃。
- **3 个新的已端到端验证注入点**（安全专家实测 exploit）：React motion transform 字符串原样进 JS 代码位（`[alert(1)]` 计算属性执行）、OmniView 未加引号 `onmouseover=alert(1)` 属性逃逸、design token `}`/`</style>` CSS 结构字符逃逸——**855 元组语料的 9 个插入点不含 motion/design 两个发射面**。
- **门禁体系真实但度量偏差**：React「绿」= 287 条已知错误豁免（glob `*` 无法约束新增同类）；Flutter「0/0」= 空 lint 配置（flutter_lints 下 1,662 info）+ 展示项目 1,004 条编译 warnings 零断言；OmniView 44 测试手动运行且可测陈旧产物（#60）；CI 只跑测试子集（all-backends `if: false`、Flutter 模块测试与注入语料 Flutter 行零守护）。
- **「绝无 parse-and-ignore」声明被证伪**：`states:`/`on:` 遗留分支静默吞意图（哨兵无人消费）；未知事件名静默通过（`@onTotallyBogus` → "No issues found"）；Rust/Dioxus block-expr 静默丢弃；缓存诊断 parse 洞（编辑器绿色显示坏代码）。
- **文档漂移在 v2 后复发**（10+ 处）：v2 恢复的 `@scroll`/`@drag` 控制维度被三处文档误述为「已移除/待重新实现」；v1 spec 与 v2 设计文档双权威从未收敛（steps() 行为三处矛盾）；15-grammar 教授 parser 拒绝的 `@themes` 分号形式；fade/wait 跨目标表 Flutter 列与设计文档互相颠倒且各错一格——全部位于 docs-gate（只验 parse）不可达处。

**Overall Assessment**: **Adequate（架构与验证网）＋ Needs Improvement（行为正确性）**——v2 把「声称与事实不符」变成了「声称与事实一致但度量的是编译而非行为」；架构方向、门禁基础设施、删除纪律值得保留并加固；React/Flutter 行为层的运行时缺陷群（1 CRITICAL + 8 HIGH + 3 注入）是「地道代码」承诺的最后一段距离，必须修复后才能宣称语义 DSL 兑现。

**Key Findings**:
1. Flutter 生成代码无任何运行时验证——CRITICAL `TodoStore.add()` TypeError 崩溃直达旗舰 demo 核心循环
2. 3 个新注入点（motion/design/unquoted-handler）端到端可利用，语料插入点覆盖不全
3. React 豁免债本质是运行时 bug 类（BUG-A/BUG-C 在售 demo 上崩溃），19/36 测试 fixme 维持现状
4. 「静默降级归零」声明被 8 个独立实例证伪（states:/on:、未知事件名、bind:value 单向化、in-view/focus、multi-theme 塌缩等）

**Critical Actions**:
1. 修复注入三连（motion 转义 + 语料插入点、unquoted handler、design token CSS 结构字符）
2. 修复 Flutter struct 字面量发射（CRITICAL TypeError）+ 建立生成代码运行时测试（widget test）
3. BUG-A 廉价路径（const→let + un-fixme 19 测试）+ inline store 硬诊断或实现
4. `states:`/`on:` 硬诊断化（恢复「绝无 parse-and-ignore」声明）
5. #60 OmniView 套件入 CI + CI 测试面补齐（all-backends、Flutter 模块测试）

---

## System Overview

- **评审对象**: OmniView 语义级 DSL v2（`.omni` 视图语言，10 语义域：Ref 10 ops、动画 19 属性白名单/8 触发器、Design Tokens/@themes/@theme、Slots（纯净化）、Stores（顶层+内联统一）、@if/@for（keyed 契约）、Events（单表映射）、Expressions（闭包唯一）、Styling（$field 通道）、a11y（新增））
- **架构**: 三层——语义原语（`crates/omnilang-core/src/semantic/`）→ IR（`crates/omnilang-core/src/ir/`，A11yIR/ListRenderingIntent/StoreIR.theme_signal/ArrayMethodIR 等 v2 新增）→ codegen（`omnilang/src/codegen/{react,flutter,omniview}` + 4 非 P0 后端）。P0 目标：React (TS strict+Zustand v5+React 19) / Flutter (Dart+Riverpod 3.0) / OmniView 调试运行时。
- **关键契约**: ADR-054（5 原则，尤其原则 3「floor 外硬诊断，绝无 parse-and-ignore」）；ADR-055（@for 契约）；ADR-056（v2 实现完成 + 6 项实现期决策：中文诊断/虚构语法自然错误/变体编译期合并/注入五连闭环/验证网架构/展示项目重构）
- **规模**: ~100k 行 Rust（2 crates）、1,756+ tests（925 core + 785 all-backends + 注入语料 855 元组）、docs-gate 232 块、learn-omniview 展示项目（93 `.omni` 文件、9,438 行、9 标签页、三目标全绿门禁）
- **验证方法**: 13 名专家独立评审 + 多名专家实测（React 门禁重跑 166/121、Flutter dart analyze 真实 192s 全量、docs-gate 232/0/147、Playwright 17 passed/19 fixme、注入 exploit 端到端复现 ×3、dart run 复现 TypeError、wasm-pack 21/21、PROPTEST_CASES=50、cache-hole 复现、corpus 编译全量）

---

## Cross-Cutting Themes

以下主题在 3+ 名专家评审中独立出现（高杠杆发现）：

### 1. 「门禁 PASSED ≠ 行为正确」——度量偏差是系统性（10 名专家）
- React：287 条已知错误全豁免（166 tsc + 121 eslint），glob `*` 无法约束新增同类错误；fixture 矩阵 strict:false；19/36 Playwright fixme（UI/Language 标签页全跳过）——ts/react C1 + systems HIGH-1 + compiler + strategy HIGH-1
- Flutter：`# minimal` 空 lint 配置（flutter_lints 下 1,662 info）；1,004 条编译 warnings（881 ECodegen + 123 lint）零断言；零运行时测试（CRITICAL TypeError 直达）——flutter + systems + compiler
- CI 只跑测试子集：all-backends `if: false`（唯一全后端组合 job）、Flutter/Vue/Dioxus 模块测试不跑、注入语料 Flutter 行不跑、覆盖率门槛 70% vs 项目 80%——systems HIGH-3
- OmniView 44 测试手动运行、可测陈旧产物（#60 未入 CI）——strategy HIGH-2 + systems HIGH-2 + compiler + pragmatic
- 结论：**编译级验证网已真实且可支撑增量语法变更；运行时级只有 React 侧入 CI（且 fixme 缺口大），Flutter 与 OmniView 侧零守护。**

### 2. 「静默降级/parse-and-ignore」在 v2 后仍存活 8 个独立实例（8 名专家各自发现不同点）
- `let mut` IR 丢弃标志（compiler CRITICAL + pragmatic H1 + ts/react C1）
- inline store React 零发射（ts/react H2 + pragmatic H2 + perf H1）
- `states:`/`on:` 静默吞意图 + 哨兵机制（domain HIGH-1）
- Flutter `bind:value` 非 ref 单向化（compiler HIGH + flutter HIGH，14 处）
- `@in-view`/`@focus` 零警告不接线（flutter HIGH）
- 未知事件名静默通过（ai HIGH，@onTotallyBogus 实测）
- multi-theme Flutter 塌缩（compiler MEDIUM，新发现不在 deferrals）
- Rust/Dioxus block-expr 静默丢弃（rust MEDIUM，绕过 v2 debug_assert 守卫）
- 另含：尾表达式丢值（ts/react H1）、textarea→空 Container、disabled 丢弃（flutter HIGH）、motion-in-@for 双目标各自错误（perf M2）、实体转义误用（ts/react M1）
- 结论：**ADR-054 原则 3 在语义层基本贯彻，但 codegen 层（尤其 Flutter 目标）与 parser 遗留分支仍有多处破口——每个实例都是「编译绿、行为错、无诊断」。**

### 3. 注入面未全闭——语料插入点覆盖与发射面不同步（3 名专家独立实测）
- motion transform 字符串原样进 JS 代码位（security HIGH-1，端到端 alert(1) 执行）
- 未加引号 `onmouseover=` 属性逃逸 escape_html（security HIGH-2，HTML_PATTERNS 只查引号形式）
- design token `}`/`</style>` CSS 结构字符逃逸（security MEDIUM-3，:has() 可实现数据外泄）
- JSON 对象键未转义（rust MEDIUM latent，当前不可达）
- 根因：855 元组语料 9 个插入点缺 motion/design/属性形状变体——**「语料全绿」被错误读作「注入面封闭」**

### 4. 文档-事实漂移在 v2 后复发（6 名专家，10+ 处，全部位于 docs-gate 不可达处）
- v2 恢复的 `@scroll`/`@drag` 控制维度被三处误述为「已移除/待重新实现」（doc HIGH-2，deferral #19 已关闭）
- v1 spec vs v2 设计文档双权威从未收敛（doc HIGH-3 + domain LOW-2：steps() 三处矛盾、setSelectionRange vs setSelection、bind:checked 缺席 BNF、§7 承诺 #28/#33 未落地但标 Implemented）
- 15-grammar 教授 parser 拒绝的 `@themes` 分号形式（doc HIGH-1，同文件 905 行自相矛盾）
- fade/wait 跨目标表 Flutter 列与设计文档互相颠倒且各错一格（doc MEDIUM-4）
- 10-design-system 教授已删除的 mode 魔法名自动同步（domain MEDIUM-4 + doc LOW-9）
- @fallback/@error 死 token 在 grammar 文档列为关键字（domain MEDIUM-3）
- 结论：**docs-gate 只验 parse 真值（232 块 0 失败、147 语义 notes 放行）——「文档与编译器事实一致」只对语法成立，prose/BNF/表格无任何守护。**

### 5. React 豁免债的本质是运行时 bug 类（5 名专家）
- 21 条豁免对应 287 条错误：TS2722（事件处理器 `?.()` → 潜在 TypeError）、exhaustive-deps 93x（stale-closure 类）、TS2322/TS2304（用户 struct 未发射 #49）、TS7006/7034、no-var（消毒器）
- 豁免台账断链：13/18 条根因未注册进 deferrals（strategy MEDIUM-1）；计数漂移 20/21/18、166/161（strategy + systems LOW）
- 「0 runtime error」的 fixme 标记是诚实的，但 19 条 fixme 维持了「两标签页崩溃 + 商店 demo 不可点」的现状（ts/react + compiler）
- 结论：**每个豁免错误类都是待修 bug 或 bug 症状——豁免清单是 burn-down 清单而非终态。**

### 6. 结构性债未动：绑定解析、walker 爆炸、pass 编排、缓存诊断（3 名专家）
- store-action 参数遮蔽仍只修 OmniView（maintainability HIGH-1，React/Flutter 静默生成错误代码，无诊断无测试）
- ~9 套手写 IR 遍历（共享 walker 仅 3 调用方；v2 新增 3 套）——每个新 IR 变体 ~9 处手工同步
- lower() 内 7 个语义 pass（前次 4 个，更纠缠）+ analyse.rs 2 个——新 pass 无规范归宿
- 缓存诊断 parse 洞（rust HIGH：CachedFile.diagnostics 只存 lowering 诊断，parse 错误从不缓存——实测 run1 报错 run2 success=true 错误消失）
- flutter/design.rs 2458→3548（+44%）违反「触碰即拆」触发条件

### 7. 运行时测试缺席是行为层问题的共同根因（5 名专家）
- Flutter 零运行时测试（CRITICAL TypeError、bind:value 死绑定、主题不可点全部由 dart analyze 看不见）
- fixture 断言只测字符串形状：in-view 只断言 helper 类存在不断言实例化（flutter HIGH）；[scroll]+store 读组合无 fixture
- React 19 fixme 套件是唯一运行时证据，且其断言已陈旧（ts/react）
- 结论：**「三目标全绿」需要「编译门禁 + 运行时冒烟」双轨；当前只有编译轨。**

---

## Trade-offs and Disagreements

**1. BUG-A 修复路径（廉价 let vs IR mut 字段）**
- strategy HIGH-1：`react/mod.rs:394` const→let 一行 + un-fixme 4 条测试 = 零 IR 变更立即止损；IR 字段与 #37 合批（~64 构造点一次 churn）
- compiler CRITICAL + pragmatic H1：interim 诊断（S）+ 真修复 LetStmtIR.mut（M-L，~75 构造点）
- **消解**：两方案互补而非冲突——立即执行 strategy 的廉价路径（eslint 无 prefer-const、门禁不破、行为立即正确），interim 诊断与 IR mut 字段在 IR wave（与 #37 合批）中一并落地。二者都要求 un-exempt TS2588。

**2. React 门禁的性质（「真实且诚实」vs「green-by-exemption」）**
- ts/react：门禁真实、自删除、诚实描述——但守的是「已知错误清单」而非「零错误」
- systems HIGH-1 + compiler：green-by-exemption ≈ 「无新错误类」而非「正确输出」，glob `*` 无法约束增长
- **消解**：语义相同、度量口径不同——门禁基建值得保留（stale 双向强制、逐条归因），但必须加「路径 glob 收窄 + 实例计数上限 + fixture strict:true」三件套（systems 方案）把度量从「类别」升级到「实例」。

**3. v2 总体评估（「宣称实测为真」vs「BLOCK」）**
- compiler/rust/doc：v2 声称全部实测为真（门禁、删除、语料、wasm、proptest）
- flutter：**BLOCK**（CRITICAL TypeError + 5 HIGH）；pragmatic：Ratio 0.63→0.69 略恶化
- **消解**：不矛盾——v2 兑现的是「基础设施与语法层」的承诺；「生成地道且行为正确的代码」是 v2 声称的但未兑现的部分。评审结论合并表述：**架构层 Adequate，行为层 Needs Improvement**。

**4. 未知事件名的处置（硬错误 vs warning）**
- ai HIGH：建议 error 级（LLM 信任崩塌点，Flutter 下游分析错误无 .omni 侧提示）
- domain（事件域现状）：事件别名单表已收敛，未知名走 `_ => None` 回退
- **消解**：至少 warning + 最近合法事件名建议（validate_a11y 先例）；error 级留待决策——但「No issues found」的现状不可接受（双专家一致）。

---

## Consolidated Findings

### Strengths（v2 兑现层）

1. **删除纪律执行彻底**（pragmatic 逐项验证 + domain + compiler）：SetValue（11→10 ops）、paused、@native、typed slots、slot="name"、value={sig}、auto-bind、$counter——全部硬诊断 + verbatim 迁移提示 + 回归测试，93 文件语料零残留。「删除最便宜时点」已被正确利用。
2. **注入五连闭环真实**（security 实证）：855 元组 + 45 控制编译、模式缺失断言（非仅可编译）、Vue 实体解码建模、类名双保险（lowering 字符集 + 转义）、消毒器单一来源（React/OmniView 共用 + 回归测试）。class-name 面是代码库中最强的封闭。
3. **React 地道性核心修复落地**（ts/react + perf 实证）：切片订阅（FilterBar 5/5 per-field）、computed/effect deps 含 props 与闭包、@for keyed 契约（key={item.id} + 单次求值 + String(item) 禁令）、functional updaters、coalesced set()、theme wiring 端到端验证（data-theme 断言）。字节级一致（重生成 = 提交产物）。
4. **Flutter 结构性修复落地**（flutter + perf 实证）：@for Column/ValueKey 分派（无条件 ListView.builder 消失）、classes/variants → AppDesign 工厂 + ThemeExtension、Riverpod 3.0 Notifier + 精确 select()（legacy.dart 零残留）、per-element controller + 平衡 dispose、语义绑定快照重放修复（焦点偷窃消灭）、const 注入、设计域诚实 typed（不可映射值 warnings 通道）。
5. **主题语义三目标真实接线**（compiler + domain 实证）：@themes 声明 + @theme 信号标记、程序级配对验证、React data-theme / Flutter ThemeModeNotifier / OmniView __omni_theme——「主题切换静默失效」类彻底关闭（除 multi-theme Flutter 塌缩新发现）。
6. **IR 结构性承载语义**（compiler）：A11yIR、ListRenderingIntent（key_expr/scroll_hint）、StoreIR.theme_signal、ArrayMethodIR——codegen 消费意图而非语法。
7. **Rust 侧质量**（rust 实证）：unsafe 审计干净（全 FFI 边界 + SAFETY 注释）、unwrap/expect 全部有守卫、缓存架构合理（传递失效/LRU/program_valid）、注入语料断言机制仓库最佳（逐点控制组 + 单遍剥离器）、per-call 缓存序列化已移除（显式 save_cache_state）。
8. **豁免/门禁治理成为持久架构**（maintainability + strategy）：豁免逐条归因 + stale 双向强制、fixme 诚实呈现、deferrals #36-#60 全带触发条件、docs-gate「修文档不修编译器」政策、lint 框架真实可扩展点（11 规则 + 教程）。
9. **语料纯净**（domain + pragmatic 实证）：93 文件零旧语法、事件全部 @onXxx 规范形式、ref ops 全部在 10-op 集内。

### Areas for Improvement

1. **注入三连**（CRITICAL，security 端到端验证）：motion transform 字符串转义 + 语料 motion 插入点；unquoted handler 模式 + HTML_PATTERNS 扩展 + escape_html 属性上下文硬化；design token 结构字符中和 + 语料 design 插入点。优先级：CRITICAL。
2. **Flutter struct 字面量发射**（CRITICAL，flutter 端到端复现）：store action 中对象字面量 → 带引号键 Dart Map → copyWith cast TypeError。修复：typed 上下文走 `format_struct_field_named` → 构造调用。优先级：CRITICAL。
3. **React BUG-A/BUG-C**（CRITICAL/HIGH）：const→let 廉价路径 + un-fixme 19 测试；inline store 硬诊断（直到 #37 实现）。优先级：CRITICAL。
4. **`states:`/`on:` 硬诊断化**（HIGH，domain 实测）：替换哨兵机制为 verbatim 迁移提示 + 回归测试。优先级：HIGH（恢复 v2 核心声明）。
5. **Flutter 行为缺口群**（HIGH）：bind:value 非 ref 接线（onChanged/controller）、@in-view/@focus 接线或警告 + fixture 实例化断言、textarea→多行 TextField、disabled→enabled:false、主题选择器（Container→InkWell）+ themeModeProvider 四态 + multi-theme 塌缩警告。优先级：HIGH。
6. **未知事件名验证 pass**（HIGH，ai 实测）：validate_event_names（validate_a11y 先例）+ 最近合法事件建议 + 语料/文档门禁同步。优先级：HIGH。
7. **门禁度量升级**（HIGH）：React 路径 glob 收窄 + 实例计数上限 + fixture strict:true；Flutter warnings 断言基线（1,004 条）；#60 入 CI；all-backends 启用 + Flutter 模块测试 + 注入语料 Flutter 行。优先级：HIGH。
8. **缓存 parse 诊断洞**（HIGH，rust 复现）：CachedFile.diagnostics 补 parse 阶段 + 回归测试。优先级：HIGH（poiesis 编辑器主路径）。
9. **尾表达式 return**（HIGH，ts/react 实测）：fn 尾 match/block 发射 return；DataFetching 成功路径 + 测试。优先级：HIGH。
10. **a11y role 词汇 ARIA 化**（HIGH，domain 实测）：textField→textbox、image→img 内部映射。优先级：HIGH。
11. **文档漂移批量**（HIGH，doc 6 处 + domain 4 处）：@scroll/@drag 恢复表述、双权威收敛、fade/wait 表、steps 统一、@themes 分号、mode 魔法名、grammar 陈旧行、设计文档 BNF（setSelection/bind:checked）、#28/#33 承诺清理。优先级：HIGH（doc HIGH-2 为最高用户伤害项）。
12. **该删未删收尾**（MEDIUM）：@fallback/@error 死 token 删除、ElementProp::TestId 死变体删除、filter/map/count 与 stdlib 双名收口（prove-or-retire）、默认主题双重声明验证、E_PAUSED_REMOVED 补 block 形式。优先级：MEDIUM。
13. **结构性债**（MEDIUM/L）：param 遮蔽移植 React/Flutter、walker 收敛、pass 编排移出 lower()、C-FFI 信封统一、JSON 对象键转义、Rust/Dioxus block-expr、123 unused-import 假阳性（DesignClass 计数）。优先级：MEDIUM（遮蔽移植 S、其余 M）。
14. **性能对等收尾**（MEDIUM）：Flutter [scroll]+computed O(N²)（hoist 一次）、motion-in-@for 诊断或实现、_M 表模块级提升、useCallback/React.memo、滚动相等守卫（设计文档 line 710 失实）。优先级：MEDIUM。
15. **AI 工作流工具**（MEDIUM）：00-cheatsheet、`// [omni] file:line` 源注释 + 再生成契约、LLM 错误 eval 语料、suggestion 字段填充、诊断位置精确化。优先级：MEDIUM。

### Technical Debt

**High Priority**:
- **React 豁免 287 条**（21 条豁免，含运行时 bug 类 TS2722/exhaustive-deps）：Effort 分批（M×3+）；Timeline burn-down 里程碑（#36/#37/#49 合批）
- **Flutter 编译 warnings 1,004 条零断言**：Effort S（断言基线）；Timeline 立即
- **Flutter 零运行时测试**：Effort M（widget test harness 前身）；Timeline 短期
- **缓存 parse 诊断洞**：Effort S；Timeline 立即
- **双权威文档**（v1 spec vs v2 设计文档）：Effort M（重写或 Superseded 标注）；Timeline 短期

**Medium Priority**:
- **walker 爆炸**（~9 套遍历，共享 walker 3 调用方）：Effort L；Timeline 随 BindingRef 前置
- **pass 编排**（lower() 内 7 pass）：Effort M；Timeline 短期
- **flutter/design.rs 3548 行**（+44%）+ resolve.rs 1090 行零测试：Effort M；Timeline 随触碰拆分
- **C-FFI 信封**（7+ 处裸字符串）：Effort S；Timeline 短期
- **clippy 非 -D warnings**（~75 条）：Effort M；Timeline 长期
- **codegen_never_panics 只覆盖 3/10 后端**：Effort M；Timeline 长期

**Low Priority**:
- escape_literal_str 6 遍、fast_hash SipHash、零跨度诊断 29 文件、测试残留目录、unused-import 123 条、`@fallback`/`@error`、TestId 死变体

### Risks

**Technical Risks**:
- **Flutter 生成应用行为崩坏无人知**（Likelihood: Certain, Impact: Critical）：旗舰 demo 核心循环 TypeError、全部交互表单 inert、主题域不可达——首次手动测试即推翻「10 域实现」声明；修复前任何 Flutter codegen 变更都是一次 crash 之遥。
- **注入面未全闭**（Likelihood: High, Impact: Critical）：motion 载荷任何 esbuild/Vite 消费者直接执行；body.html 无 CSP 的编辑器/调试预览路径携带可用 XSS；design CSS :has() 外泄。语料被引用为「封闭证据」= 假安全感。
- **静默错误类回归**（Likelihood: High, Impact: High）：8 个实例证明「绝不静默降级」在 codegen/parser 遗留层未闭环；下一轮浏览器终验将重新发现同源问题。
- **React 豁免增长**（Likelihood: High, Impact: Medium）：glob `*` 下新增同类错误静默通过；BUG-A 修复前新增 let-mut demo 扩大崩溃面。
- **文档漂移复发**（Likelihood: Medium, Impact: Medium）：docs-gate 只验 parse，prose 漂移周期性复发而 CI 绿灯——@scroll/@drag 误述即是当前实例。

**Business Risks**:
- **「一处编写、三端地道」行为层失信**：编译全绿但 React 两标签页崩溃、Flutter 核心循环 TypeError——用户首次运行即发现；「GATE PASSED」被门外者读作「编译且运行」。
- **AI 时代定位在语义层重新承压**：未知事件名/方法名静默接受 = LLM 信任崩塌点复现（events 是最高频域之一）；cheatsheet/eval 语料缺席使优势不可测量。
- **安全声誉**：注入语料 855 元组被引用为封闭证明而 3 个插入点缺口存在——供应链威胁模型下 first-party 组件即攻击面。

---

## Recommendations

### Immediate (0-2 周)

1. **注入三连**（security CRITICAL，各 S-M）
   - `react/motion/keyframes.rs:176` transform 字符串 `escape_literal_str(s, '"')`；语料加 motion 插入点（`[alert(1)]` 计算键载荷族）
   - HTML_PATTERNS 加 unquoted handler 模式（`on*=`）+ `</style><script`；escape_html 属性上下文转义 `=`/空白；Vue 同步
   - `sanitize_css_value` 拒绝 `}`/`</style>`；语料加 design 插入点（`}`/`</style>` 载荷）
   - Success Criteria：三 payload 族全后端回归 + 语料插入点覆盖 parity 测试（覆盖地图：枚举全部用户字符串发射面）
2. **Flutter struct 字面量发射**（flutter CRITICAL，M）
   - typed 上下文（StructExpr）走 `format_struct_field_named` → `Todo(id: ..., text: ..., done: false)`；新增 widget test 执行 `TodoStore.add()`
   - Success Criteria：生成 app 挂载冒烟（首个生成代码运行时测试）
3. **BUG-A 廉价路径 + inline store 硬诊断**（strategy + ts/react + pragmatic，S）
   - `react/mod.rs:394` const→let；un-fixme 19 条 Playwright 测试；删 TS2588 豁免（stale 机制强制）
   - inline store 在 React 目标 E_CODEGEN 硬诊断（直到 #37 实现）
   - Success Criteria：Playwright 17→36 全跑、UI/Language 标签页 0 runtime error
4. **`states:`/`on:` 硬诊断化**（domain HIGH-1，S）：verbatim 迁移提示 + 删哨兵 + 回归测试
5. **#60 入 CI**（strategy HIGH-2 + systems HIGH-2，S）：e2e job 加 `omnic build --target omniview` 再生成 + omniview config 套件（独立 job 并行）
6. **CI 测试面补齐**（systems HIGH-3，各一行 YAML）：codegen-validate 改 `cargo test -p omnilang --features codegen-flutter`（lib+integration）；去 `test-all-backends` 的 `if: false`
7. **缓存 parse 诊断洞**（rust HIGH，S）：CachedFile.diagnostics 补 parse 阶段 + parse-error 缓存命中回归测试
8. **文档最高伤害项**（doc HIGH-2 + HIGH-1 + MEDIUM-4/5，S）：@scroll/@drag 恢复表述三处、@themes 分号、fade/wait 表对照 flutter/view.rs、steps 三处统一 warning
9. **未知事件名验证**（ai HIGH，S）：validate_event_names（validate_a11y 先例）+ 最近合法事件建议 + React/Flutter 断言
10. **豁免台账重连**（strategy MEDIUM-1，XS）：task 列→deferral 编号；未登记根因（TS2722/exhaustive-deps/no-var）补入；修正 20/21/18、166/161 漂移

### Short-term (2-8 周)

11. **Flutter 行为缺口批次**（flutter HIGH 群 + compiler，M）：bind:value 非 ref 接线、@in-view/@focus（lowering 填充 in_view_once + 实例化断言）、textarea/disabled 映射、Container→InkWell + themeModeProvider 四态、multi-theme 塌缩警告
12. **React 门禁硬化**（systems HIGH-1，M）：路径 glob 收窄 + 实例计数上限 + fixture strict:true——度量从「类别」升级「实例」
13. **IR wave：#36 mut 字段 + #37 inline store + #49 用户 struct 合批**（strategy + compiler，M）：一次 fixture churn、一次 golden 重同步、一次 match 臂审查；#49 紧随同窗
14. **尾表达式 return**（ts/react H1，S）+ 实体转义路径分离（M1，S）+ 事件属性 camelCase/合成事件名映射（M3，S）
15. **a11y 域收尾**（domain，S）：role 词汇 ARIA 化（textbox/img 内部映射）、aria-* 硬诊断 + 迁移提示、@default-theme↔@theme initial 验证、@fallback/@error 删除
16. **文档双权威收敛**（doc HIGH-3，M）：v1 spec 重写或 Superseded 标注 + 修正 §3.6/§10.3/§12/§13；更新 11-motion 引用；docs-gate 加删除语法族语义诊断白名单 + CI 打印 notes 计数
17. **结构性修复批次**（maintainability + rust，S-M）：param 遮蔽移植 React/Flutter + 2 回归测试、C-FFI 信封 DiagnosticJson::synthetic、Rust/Dioxus block-expr 发射或诊断、JSON 对象键转义 + 单测、DesignClass/Variant 计入 used（123 假阳性清零）、filter/map/count prove-or-retire
18. **性能批次**（perf，S-M）：Flutter [scroll] iterable hoist（O(N²)→O(N) + @empty 三求值）、滚动相等守卫 + duration 提升（设计文档 710 行同步）、motion-in-@for 编译诊断（直到逐项实现）、_M 模块级提升、useCallback/React.memo、never-mutated 字面量信号 hoist
19. **Flutter 运行时测试基建**（flutter + compiler，M）：widget test harness（tester.pumpWidget 冒烟 @if/@for/motion/ref/[scroll]+store 读）+ 生成代码挂载测试——修复 CRITICAL 的前提
20. **AI 工作流工具**（ai，S-M）：00-cheatsheet（10 域 × 1 例 + 事件表 + 错误码表，入 docs-gate）、`// [omni] file:line` 源注释 + 再生成契约文档、suggestion 字段填充、LLM 错误 eval 语料（~30 fixtures 入 CI）

### Long-term (2-6 个月)

21. **绑定解析入 IR**（maintainability HIGH-2，L）：BindingRef 注解——前置：walker 收敛 + pass 编排移出 lower()（M ×2）
22. **覆盖地图测试**（security #8，M-L）：枚举 5 后端全部用户字符串发射面，断言每个被语料覆盖/校验/转义
23. **per-target 类型发射**（compiler + ts/react，L）：用户 struct/联合 → React TS 类型 + Dart 类（#49 完整版），豁免减半
24. **flutter_lints 对齐**（flutter，L）：avoid_unnecessary_containers 等 codegen 级修复或逐规则抑制文档化；Scaffold/SafeArea 壳、IndexedStack 标签页、DefaultTextStyle.merge
25. **fuzz 全后端 + 语法感知生成器**（rust，L）：codegen_never_panics 覆盖 10 后端 + 结构化 .omni 生成器达到语句/事件/motion 深路径
26. **CSS 逐类型白名单**（security #7，L）：替代关键字黑名单（镜像 Flutter type-mapper）——消除整类 CSS 注入
27. **a11y label 动态化**（domain LOW-3，M，按需）
28. **超大文件拆分**（maintainability，M）：flutter/design.rs 按域、lower/mod.rs 测试移出、resolve.rs 补测

---

## Success Metrics

1. **注入面**：motion/design payload 族全后端回归通过（立即）；覆盖地图测试枚举全部发射面（1 月）；`[alert(1)]` 类载荷被拒绝（立即）
2. **Flutter 行为**：生成 app widget-test 挂载 + `TodoStore.add()` 执行通过（2 周）；bind:value 表单 E2E 可用、主题四卡片可点、@in-view/@focus 动画播放（1 月）
3. **React 行为**：Playwright 36/36 全跑 0 runtime error（2 周）；UI/Language 标签页回归测试活跃
4. **静默降级归零**：states:/on:/未知事件名/aria-*/block-expr/inline-store/多主题 全部产生诊断（1 月）；「parse-and-ignore 零」声明重新可验证
5. **门禁度量**：React 豁免从「类别」到「实例计数」（2 周）；Flutter warnings 断言基线入库（2 周）；#60 入 CI（2 周）；all-backends 785 tests 在 CI 跑（2 周）
6. **文档一致性**：@scroll/@drag/fade-wait/steps/双权威 修复落地（2 周）；docs-gate 语义诊断白名单（1 月）
7. **编辑器信任**：缓存 parse 洞修复 + 回归（立即）；诊断位置精确化（short-term）

---

## Follow-up

**Next Review**: 本批 Immediate 完成后做 React/Flutter 行为层对齐评审（或纳入下一轮浏览器全量终验）。

**Tracking**:
- 每条发现四选一处置（fix-now / hard-diagnostic / delete / defer-with-trigger），评审结束零悬空项
- 治理同步：豁免台账重连（13 条未注册根因）、deferral 新增（multi-theme Flutter 塌缩、@in-view/@focus 接线、injection 插入点缺口、struct 字面量 TypeError）、#60 关闭
- ADR 决策候选项：未知事件名 error vs warning 级别；filter/map/count 与 stdlib 双名收口方向；aria-* 原生属性去留；BUG-A 廉价路径是否长期保留（IR wave 前）

**Accountability**: 本评审文件 + 会话记忆为追踪基线；按「注入 → 行为 → 门禁度量 → 文档」顺序执行。

---

## Related Documentation

**Architectural Decision Records**:
- ADR-056（v2 实现完成）——本评审验证其宣称：基础设施层全部属实；「行为正确」层未兑现（React 2 标签页崩溃、Flutter CRITICAL TypeError）
- ADR-054（设计理念 5 原则）——原则 3（绝无 parse-and-ignore）在 parser 遗留分支（states:/on:）与 codegen 层（8 实例）未闭环；原则 5（文档对齐）docs-gate 只验 parse
- ADR-055（@for 契约）——React/Flutter 侧已兑现（keyed/Column+[scroll] 分派）；motion-in-@for 新缺口
- ADR-052/053（语义级 DSL / BNF 规范化）——「单一权威」模型被 v1 spec vs v2 设计文档双权威现状再次侵蚀

**Previous Reviews**:
- `feature-omniview-semantic-syntax-compilability.md`（2026-08-02，13 专家）——本评审的对齐基线；其 5 项 CRITICAL（React C1/C2、Flutter @for、className 注入、主题、门禁）已全部修复；本轮新发现为其 Follow-up 承诺的验证结果

**Referenced Documents**:
- 设计契约: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（已 Implemented）
- 语法规范: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（双权威问题）
- 用户文档: `docs/omniview/*.md`（16 篇）
- deferrals: `.architecture/deferrals.md`（#36-#60 + 本轮新增候选）

---

## 附：各专家评审摘要（来源数据）

### Compiler Architect (Dr. Lin Wei) — 编译器/语言架构
**验证**: 重跑全部门禁（React GATE PASSED 166/121 全豁免零 stale、Flutter dart analyze 真实 192s 全量 0/0、docs-gate 232/0）+ 通读 IR。
- CRITICAL: BUG-A IR 根因——`LetStmtIR` 无 mut 标志（ir/types.rs:539），React 恒 `const`（DataTable.tsx:65 崩溃）、Flutter 恒 `var`（永失 final）——一个 IR 缺口两种方向相反的错误行为零诊断。Fix: interim 诊断（S）+ mut 字段（M-L）。
- HIGH: Flutter `bind:value` 非 ref 元素静默单向化（view.rs:1436-1461，onChanged: null → 控件永久禁用）——设计 D2「禁止任何目标单向化」未兑现（#50）。
- MEDIUM: multi-theme（>2）Flutter 静默塌缩 light/dark（design.rs:1340），第三主题生成死代码——新发现不在 deferrals。MEDIUM: ArrayMethod python/rust/dioxus verbatim 发射（#55）——BlockExpr 有 E_CODEGEN 先例应镜像。MEDIUM: unused-import 假阳性 123（#43）。
- LOW: 设计文档 §7 承诺 #28/#33 未落地但标 Implemented；`@default-theme` 缺席设计文档 BNF（#42）；a11y `alt` 非 image 元素 / `hidden` 可聚焦容器无验证。
- 亮点: 删除语法诊断 verbatim 全验证（setValue→bind:value 提示）；bind:value 方向契约 fixture 断言；validate_theme 程序级配对；主题三目标接线真实；注入加固结构性闭环（855 元组含失败编译断言）。
- 裁决: **v2 声称全部实测为真**；剩余缺口 = 文档化债务（#36/#37/#50/#55/#60）+ 2 个新发现（multi-theme 塌缩、mutability-IR-gap 框架化）。

### TypeScript & React Specialist (Emma Rodriguez) — React 地道性
**验证**: HEAD 重跑门禁 + Playwright（17 passed / 19 fixme / 0 failed）+ 重生成产物字节一致 + react-dom/server 实证。
- CRITICAL C1: BUG-A `let mut`→const 渲染期崩溃（DataTable.tsx:60-69 getBonusTotal 渲染中调用 → 整 app 卸载；ErrorHandlingDemo/StatsDashboard/ValidationInput 同族）；19 fixme 覆盖 UI/Language 全标签页。
- HIGH H1: match/block 尾表达式静默丢值（DataFetching.tsx:6-11 error_message → undefined → 成功路径不可达；TS2367 豁免掩盖）。HIGH H2: inline store action 引用未定义 CartStore（InlineStoreDemo.tsx:16-32，TS2304 豁免；store 状态冻结）。
- MEDIUM M1: 实体转义误用于 JS 字符串字面量（view.rs:1007 → 界面显示 '&amp;amp;'）。M2: App 根 useDesignTokens 整 store 订阅复活 + per-field hooks 死代码（useDesignTokens.ts:91）。M3: kebab style 属性 + onAnimationend/onTransitionend 非合成事件名（运行时警告 + 事件永不触发）。M4: unmount cleanup 闭包 stale state（Timer final count 恒 0；exhaustive-deps 93x 非全假阳性）。
- LOW L1-L4: 模块级副作用、17 未用标识符、ES5 var 体、豁免计数陈旧。
- 亮点: 切片订阅/计算 deps/@for keyed 单次求值/受控输入真实地道；theme wiring 端到端断言；motion cleanup 正确。
- 裁决: **「GATE PASSED」守的是已知错误清单**——门禁诚实但 287 条诊断仍在，其中若干是行为 bug 被类型错误掩盖；修复路径 S-M 且豁免文件已正确命名根因，差距在执行。

### Flutter Reviewer — Flutter 地道性
**验证**: HEAD 全量编译 83 文件 + 重跑 CI 门禁（PASSES）+ dart analyze 双配置 + warnings dump（1,004 条）+ 独立 Dart 程序复现 TypeError。
- CRITICAL: store action 对象字面量 → Dart Map<String,Object>（键带引号 `'id'`）→ copyWith cast `List<Todo>` 失败——`TodoStore.add()` 运行时 TypeError（dart run 复现 CAST FAILED）；dart analyze 静态不可见、fixture 只断言字符串。Fix: typed 上下文走构造调用 + widget test。
- HIGH-1: `bind:value` 非 ref 全失效（14 警告 dropped：add-todo/表单验证/URL 编辑/设置全死）。HIGH-2: 主题选择器不可点（Container 无 onPressed 映射）+ sepia/highContrast 不可达（main.dart:19 两态）+ 死代码。HIGH-3: `@in-view`/`@focus` 零警告不接线（lowering 硬编码 None；OmniInViewWidget 从不实例化；fixture 只断言 helper 类存在）。HIGH-4: `<textarea>` → 空 Container。HIGH-5: `disabled` 静默丢弃。
- MEDIUM: ref.watch 在 .where 谓词/effect 内（Riverpod 违规，[scroll]+store 组合未测）；motion color/fontSize 仍 DefaultTextStyle 整体替换（非 merge）；标签页切换卸载页面状态（无 IndexedStack）；无 Scaffold/SafeArea 壳。
- LOW: `# minimal` 空 lint 配置（flutter_lints 1,662 info）；Image.network 相对路径运行时异常；fontFamily CSS 栈。
- 亮点: Riverpod 3.0 Notifier 地道（copyWith sentinel/==/hashCode/typed select）；动画生命周期正确；语义绑定快照重放真实修复；设计域诚实 typed；@for Column/ValueKey 分派落地。
- 裁决: **BLOCK**——CRITICAL TypeError + 5 HIGH 修复前不能声称 Flutter 行为正确；「0/0 门禁」只证明空 lint 配置下静态通过。

### Domain Expert — 语义准确性
**验证**: 设计契约/规范/语义 pass/生成产物/93 文件语料交叉 + HEAD omnic 实测编译。
- HIGH-1: `states:`/`on:` 静默丢弃（element.rs:196-231 解析后吞 body + "__states_removed" 哨兵无人消费；on: 状态误报「箭头仅用于事件内联」；设计文档域 7 BNF 还把 on: 文档化为事件表面）——「绝无 parse-and-ignore」声明被证伪。
- HIGH-2: a11y role 词汇 Flutter 旗标形状（textField/image 非 WAI-ARIA；web 输出 role="textField" 非法、合法 role="searchbox" 反被拒）。
- MEDIUM-1: a11y 意图 2-3 表面（aria-* 原生属性 Flutter warning+丢语义；alt= DOM 形状）语料自相矛盾。MEDIUM-2: 默认主题双声明（@default-theme + @theme initial）无交叉验证、目标优先级分叉。MEDIUM-3: @fallback/@error 死 token（lexed + 文档列为关键字 + 渲染为字面文本）。MEDIUM-4: 10-design-system:116 教已删除的 mode 魔法名同步。
- LOW: DESIGN_DOCS_INDEX 陈旧；spec/设计文档漂移（setSelectionRange vs setSelection；bind:checked 缺席 BNF）；a11y label 仅静态；paused: block 形式缺迁移提示。
- 亮点: 值绑定/主题/a11y 三域收敛大体成立（verbatim 诊断 + 语料零残留 + 程序级验证）；事件单表别名系统消除双表类 bug。
- 裁决: 收敛成立但「零静默路径」声明需要 states:/on: + @fallback/@error + aria-* 三处补刀。

### Pragmatic Enforcer — YAGNI
**数据**: 93 文件使用普查逐项验证 v2 删除声明。
- 验证: 9/9 删除干净（SetValue/paused/@native/typed slots/slot="name"/value={sig}/capturePointer(id)/$counter——全部硬诊断 + 回归 + 零残留）。
- HIGH H1: `let mut` 是 IR 边界 parse-and-ignore（parser 有 mut 标志、LetStmtIR 无字段、React 恒 const 崩溃）——v2 自身原则的违反者，8+ 语料文件使用。H2: inline store React 零发射（文档化表面静默 no-op）。
- MEDIUM M1: filter/map/count 语言方法 0/93 消费者 + 与 stdlib 双名并存（域 8「一种意图一种写法」自违）。M2: unused-import 假阳性代码级确认（DesignClass/Variant 不计入 used）。
- LOW L1: TestId 死 AST 变体（data-testid 转换永不发生）。L2: deferral #18 台账陈旧（代码已修 221de33a）。L3: scroll-x/borderRadius/fontSize 零语料消费者（对称镜像，保留）。
- 评分: Necessity 8/10, Complexity 5.5/10, **Ratio 0.69（vs 前次 0.63）**——劣化恰为豁免内 H1/H2 + 未证明的 array-method 表面。
- 裁决: **Approve with simplifications**——v2 删除清扫真实彻底勿重开；剩余动作小而外科：硬诊断两 HIGH、删 TestId、关陈旧行、prove-or-retire array 方法。

### Maintainability Expert — 可维护性
**验证**: 925 core + 314 codegen 测试全绿 + 历史评审声明逐条对 HEAD 复验。
- HIGH-1: store-action 参数遮蔽仍只在 OmniView 修复（react/mod.rs:256-264 + expr.rs:913-995 + flutter/expr.rs:752-758 纯按名重写无作用域意识；场景「param items vs signal items」静默生成 state.items；无诊断无测试）。
- MEDIUM: ~9 套手写 IR 遍历（共享 ir/walk.rs 仅 3 调用方；v2 新增 3 套 validate_* 家庭；每个新 ExprIR 变体 ~9 处手工同步，靠警觉维持）。MEDIUM: lower() 内 7 pass（前次 4 个，更纠缠；semantic↔ir 双向耦合；semantic/mod.rs:1 header 陈旧）。MEDIUM: 超大文件（flutter/design.rs 3548 +44%、mod.rs 2507、lower/mod.rs 2297、validate_ref_ops.rs 1514、resolve.rs 1090 **零测试**）。
- LOW: 29 文件零跨度诊断（validate_store_access/validate_component 在 AST 层有 span 可用却伪造 0:0）；测试残留目录；OmniView 双语句发射器收缩但存留。
- 亮点: 豁免治理 exemplar（无孤儿 FIXME、逐条归因、stale 双向强制）；lint 框架真实扩展点（11 规则 + 教程）；React 后端 9 文件解构；map_event 双写合并；v2 新行为全测试守护。
- 裁决: **v2 修了症状（诊断/治理/测试）未动结构根因（绑定解析/walker/orchestration）**——两个 HIGH-1/HIGH-2 结构债仍全活。

### Rust Expert (Katya Volkov) — Rust 实现质量
**验证**: cargo test 双 crate（925+31 / 785+44）+ PROPTEST_CASES=50 + wasm-pack 21/21 + 注入语料 all-backends + cache-hole 复现 + clippy。
- HIGH: 缓存 parse 诊断洞（analyse.rs:531 只存 Phase-3a 诊断、:457-459 parse 诊断从不缓存——实测 run1 E0001 run2 success=true 错误消失；现有回归只测 lowering 错误所以漏网）——poiesis 编辑器主路径「绿屏显示坏代码」。
- MEDIUM: Rust/Dioxus format_block_expr 静默丢语句（shared/expr.rs:1063 覆写 trait 默认 debug_assert 守卫——v2 对 Python/Vue 的修复被绕过）。MEDIUM: C-FFI 7+ 处裸字符串信封（ffi.rs:49-174）违反统一 DiagnosticJson。MEDIUM(latent): format_json_value 对象键未转义（json.rs:125，当前 parser 不可达、未测试——未来 IR 路由即活）。
- LOW: escape_literal_str 6 遍、fast_hash SipHash、codegen_never_panics 3/10 后端、clippy ~75 警告。
- 亮点: v2 声称（proptest 真断言 + codegen fuzz 真跑 + wasm 基建 20/20 + 855 元组含逐点控制组 + 缓存架构传递失效/LRU/program_valid + 每调用序列化移除）全部实证为真；unsafe/unwrap 审计干净。
- 裁决: **v2 声称大部分真实；最深缺陷恰在前次点名的子系统**——缓存修复不完整（parse 洞），修复后仍会骗编辑器。

### Performance Specialist — 生成代码性能
**验证**: 五处声称修复直接在 codegen 源 + 生成语料双验证。
- HIGH H1: inline store React 冻结+崩溃（state.rs:68-70 非变异信号→const + store_slices.rs:963 无切片可能的注释——「语义静默降级」活标本；Flutter 已修同构造）。H2: Flutter [scroll]+computed O(N²)（itemCount + itemBuilder 双求值 N+1 次/构建；React O(N)；@empty 三求值）——对等性差距在 DSL 旗舰模式上。
- MEDIUM M1: 滚动/拖拽相等守卫缺失（设计文档 710 行声称已交付——失实）+ 每事件 getTiming()。M2: motion-in-@for 双目标各自错误（Flutter 全 N 项同播、React 共享 ref 只动最后项——语义层显式支持 @enter+key=）。M3: 非变异字面量信号每渲染重分配（FilterBar demoFrameworks 打败自身 memo）。M4: fn props 每渲染重建 + 零 React.memo（TodoList 级联）。M5: motion 值 store ref 强制整 store 订阅。
- LOW: _M 表每渲染、死 selector hooks 导出、fn-vs-computed 文档、StatsDashboard const-reassignment（门禁豁免盲区实证）。
- 亮点: 切片订阅/effect deps 编译期强制/@for keyed 单次求值/Flutter controller 生命周期/const 注入保守正确——五处声称中四处真实（M1 失实）。
- 裁决: **性能修复大体真实；三个静默退化 + 一个声称失实**；两目标对等性在切片层收敛、在 [scroll]/motion-in-@for 层未收敛。

### Implementation Strategist — 实施策略
**数据**: 豁免清单/台账/CI 逐项核对 + BUG-A 修复路径实证。
- HIGH-1: BUG-A 存在零 IR 变更廉价路径（react/mod.rs:394 const→let；eslint 无 prefer-const 门禁不破；un-fixme spec.ts:79/93/157/266 即回归测试）——正确性修复不应等 ~64 构造点的 IR churn。
- HIGH-2: #60（OmniView 44 测试入 CI）是下一批修复排头兵——其余 OmniView 修复（#41）与任何语法变更的回归安全前置；committed golden + 无再生成纪律 = 6-CRITICAL 周期同源结构。
- MEDIUM-1: 豁免台账断链——13/18 条根因（TS2722/exhaustive-deps/no-var 等）未注册进 deferrals；计数三处漂移（20/21/18、166/161）。MEDIUM-2: 门禁对豁免类错误数量失明（verify-react.sh:77-85 只验 used/unused）。
- LOW-1: #49（用户 struct 发射）与 IR wave golden 冲突——开发并行提交合批。
- 裁决: **编译级验证网足以支撑增量语法变更；运行时级 OmniView 在 #60 前不支撑**——「删除最便宜时点」已过且被正确利用，当前廉价窗口转移到豁免 burn-down（每修一根因 = 删一条豁免 + un-fixme 一组测试）。

### Systems Architect — 系统一致性
**验证**: per-backend feature 矩阵实测（全部通过）+ 每个诊断网关追踪 + ADR-056 逐项对证。
- HIGH-1: React 门禁 green-by-exemption（~280 实例；豁免 glob 全 `*`——新实例静默通过；fixture 矩阵 strict:false——implicit-any 类只能被展示项目抓到）。HIGH-2: OmniView——旗舰运行时验证网最弱（44 测试手动 + 可测陈旧产物；无任何真实工具链门禁证明生成 JS 可在浏览器执行；编辑器自身编译路径最不设防）。HIGH-3: CI 只跑测试子集（all-backends `if: false`、Flutter/Vue/Dioxus 模块测试不跑、注入语料 Flutter 行不跑、evals continue-on-error、覆盖率 70% vs 80%）。
- MEDIUM-1: warnings 通道两形状（Vec<String> vs Vec<Diagnostic>；单文件 errors 数组 vs 多文件 warnings 数组；codegen warnings 零位置）。MEDIUM-2: C-FFI 边界违反自身「one shape」契约（7+ 处裸字符串）。
- LOW: ADR-056 数字漂移（21 vs 19 条、166+121 vs 161+118）；陈旧注释（「只失败于 ERROR」vs 0/0 门禁）；resolve_styles 成功语义差异；feature stub 空实现。
- 亮点: 诊断管道收敛真实（统一 9 字段 DiagnosticJson + severity 小写强制 + 边界形状测试）；feature-gate 隔离结构性修复（sanitizer 共享 + cfg-gated 模块 + 「Backend not enabled」响亮回退）；豁免机制自删除；Flutter 门禁真严格；治理台账纪律 exceptional（#36-#60 全带触发、T7.3 复核）。
- 裁决: **「三目标全绿」只在各自定义的 green 下为真**（Flutter 严格 / React 豁免 / OmniView 手动）——头部声明与证据分离，ADR-056 自己在 Negative 节也承认。

### Security Specialist — 安全
**验证**: 全部发射路径通读 + 3 个 exploit 端到端编译实证 + 语料机制核对。
- 五连闭环真实: 855 元组模式缺失断言（非仅可编译）、逐点控制组防空过、Vue 实体解码建模、class 点硬失败 + 字符集消息、CI all-backends 运行；消毒器单一来源成立。
- HIGH-1: **React motion transform 字符串原样进 JS 代码位**（keyframes.rs:173-177 无转义——`translateX: "10px\", [alert(1)] : \""` 编译通过、esbuild 解析、_M 构造时 alert(1) 执行；语料无 motion 插入点）。
- HIGH-2: **unquoted `onmouseover=alert(1)` 属性逃逸**（escape_html 只转义引号形式；HTML_PATTERNS 只查带引号形态——body.html 独立产物无 CSP；wrap_debug_html 编辑器预览路径明确无 CSP）。
- MEDIUM-3: **design token `}`/`</style>` CSS 结构字符逃逸**（sanitize_css_value 关键字黑名单保留结构字符——`:has()` 属性选择器可数据外泄；`</style><script>` 在无 CSP 预览中成脚本执行；语料无 design 插入点）。
- LOW-4: 语料 .html 扫描跳过 EXEC_PATTERNS 与 `</style>` 重开。
- 修复: 转义 + 插入点 + 覆盖地图测试（枚举全部 write!/format! 用户字符串发射面）。
- 裁决: **语料机制真实但插入点覆盖与发射面不同步**——「855 元组全绿」被错误读作「注入面封闭」；motion/design 两个面是活口。

### AI Engineer — AI 时代定位
**验证**: 4 个探针（错误事件名/错误方法名/删除语法/位置精度）+ docs-gate 本地重跑 + 语料 warnings dump。
- HIGH: 未知事件名静默通过（@onTotallyBogus → "No issues found" → React 逐字发射 → 运行时 no-op；事件域 20 类型是最高频域之一）——「信任崩塌点」类复现；Flutter 侧只有下游 dart analyze 错误无 .omni 提示，LLM 无法区分自己的错与编译器 bug。
- MEDIUM: 诊断位置退化（语义 pass 锚定组件起始行、lints 0:0——JSON 契约承诺 file:line:col 但多数交付占位）；docs-gate 只证 parse 真值（147 语义 notes 放行；suggestion 字段全 null——迁移提示埋在 message 里结构化消费者读不到）；无 00-cheatsheet；旗舰语料 123 warnings（46× LearnDesign——LLM 行为范本带噪音）；再生成契约未实现（React 头注释无 file:line 源标记）。
- LOW: 具体类型方法名静默（toUpperCase 拼错 → Any 类型 → 运行时 TypeError 才见）；Windows 无意义「skipping inaccessible entry」警告污染每次输出。
- 亮点: parse 层闭环真实（docs-gate 政策「修文档不修编译器」、删除语法文档化为已删除 + 精确硬诊断消息）；机器可行动诊断 schema 稳定（9 字段 + 精确 parse/解析错误位置）；确定性有界语法 = 高首次成功率先验；93 文件可执行语料构建干净。
- 裁决: **parse 层闭环已交付；语义层闭环是 AI 时代定位生死线**——事件名静默接受是当前唯一真实信任破坏点，其余是契约质量与工作流工具缺口。

### Documentation Expert (doc-updater) — 文档工程
**验证**: docs-gate 本地重跑（232/0/147）+ 逐 parser/codegen 事实核对 + 权威关系审计。
- HIGH-1: 15-grammar-view.md:1161 教授 @themes 分号形式（parser 只收花括号 + 硬错误迁移提示；同文件 905 行 BNF 正确——文档自相矛盾；表格单元格 docs-gate 不可达）。
- HIGH-2: v2 恢复的 @scroll/@drag 控制维度被 11-motion:125/167、06:207、15:1091-1095 三处误述为「已移除/待重新实现」（deferral #19 已关闭；parser 真实接受 start:/end:/scrub:/constraints:/elastic:）——v2 最显著功能恢复被文档埋没；grammar 文档 whitelist 17 vs parser 事实 19。
- HIGH-3: v1 spec 与 v2 设计文档双权威未收敛（v1 header 仍「Approved 与 parser 同步 2026-08-02」、steps()→error 与实现 warning 矛盾、:1295 内联动画体「未实现」实际已实现、§12 权威关系失效；设计文档 75 行「按域重写」计划未执行）。
- MEDIUM-4: 06-templates fade/wait Flutter 列错误（「无过渡效果」vs AnimatedSwitcher 真实现；设计文档域 6 wait 行也错——两权威源互相颠倒）。MEDIUM-5: steps() 三处矛盾。MEDIUM-6: 02-types 教授未实现 nominal 强制。MEDIUM-7: docs-gate 盲区（语义诊断放行/行为零验证/覆盖无完整性检查/147 notes CI 不可见）。MEDIUM-8: 设计文档域 1 BNF（setSelectionRange/measureWidth vs parser setSelection/measure()）。
- LOW-9/10: 10-design-system mode 魔法名（与 433 行正确 @theme 形式自相矛盾）；grammar 陈旧行（store_inline fn 成员、type_constraint、RefBinding value、timeline_def、void 元素表互不一致）。
- 亮点: 设计-验证协议真实执行（§4.2 可独立复核）；v1 spec 补丁质量高；逐域诊断表模板是面向编译器实现者的正确信息层级；fragment 四层包装器启发式聪明；16 篇用户文档主体健康（06/03/05/08/10 正确教 v2 语义）。
- 裁决: **门禁真实但度量错误的性质**——「文档与编译器事实一致」只对语法成立；漂移全在 prose/BNF/表格，且@scroll/@drag 误述直接伤害用户。

---

## Appendix

### Review Methodology

13 名专家独立评审（compiler-architect / typescript-react-specialist / flutter-reviewer / domain-expert / pragmatic-enforcer / maintainability-expert / rust-expert / performance-specialist / implementation-strategist / systems-architect / security-specialist / ai-engineer / doc-updater），多名专家实测验证：React 门禁重跑（166/121）、Flutter dart analyze 真实全量（192s 0/0 + flutter_lints 1,662 info 对照）、docs-gate（232/0/147）、Playwright（17/19/0）、注入 exploit 端到端 ×3（motion/attr/design）、dart run 复现 TypeError、wasm-pack 21/21、PROPTEST_CASES=50、cache-hole 复现、参数化 fixture 矩阵、corpus 全量编译、feature 组合 cargo check。

**Pragmatic Mode**: Balanced
- Pragmatic Enforcer 评分：Necessity 8/10, Complexity 5.5/10, **Ratio 0.69**（前次 0.63——劣化恰为豁免内两个 parse-and-ignore HIGH + 未证明的 array-method 表面）
- 整体 Recommendation: ⚠️ Approve with simplifications——v2 删除清扫勿重开；剩余动作小而外科

**Review Complete**
