# Architecture Review: OmniLang 语义级 DSL v2 重构 — 语法达成度与 React/Flutter 地道性

**Date**: 2026-08-06
**Review Type**: Feature（语义级语法重构终审）
**Reviewers**: Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert, TypeScript & React Specialist, Compiler & Language Architect, Documentation Expert

**评审目标（用户委托）**：语义级语法重构是否真正达到设计要求（语义纯度/地道映射/硬诊断/一名一词/文档虚构归零），能否真正生成地道的 React 与 Flutter 代码；评审报告中须包含对语法本身的评审。

---

## Executive Summary

**总体评估：Adequate（架构层强，行为/台账层有系统性缺口）**

本次评审的结论是**结构成就与行为缺口并存**：v2/v3 重构在架构层面真实兑现了设计意图——10 域语义语法结构健全、BNF v2 无重大歧义、「floor 外硬诊断」在绝大多数路径字面兑现（12 处删除语法实测全部硬错误 + 迁移提示）、IR 语义纯净无平台泄漏、验证网从「假门禁」变为真实执行（docs-gate 实测 247 块 0 失败、注入语料 855 元组、React/Flutter 门禁真实跑工具链）。React 与 Flutter 生成代码在**结构层确实地道**（slice selector 订阅、keyed @for 单次求值、受控 bind:value、`ThemeModeNotifier`/`data-theme` 主题接线、Riverpod 3.0 合规、React.memo/useCallback 卫生）。用户的设计哲学——「语义语法 = 让编译器知道前端意图，编译器为每目标生成最地道代码」——在管线设计与多数映射面上是**真实兑现**的。

但「语法 → 地道代码」的承诺在**行为层与台账层**存在三类系统性缺口，12 位成员共报告 **14 个 HIGH 级发现**：

1. **「宣称 vs 事实」漂移（6 位成员独立报告，最高频交叉主题）**：语义全集台账宣称 hover/toggle「支持（全目标）」实测 web 两目标静默不触发；`theme = "dark"` 文档宣称「自然解析错误/响亮失败」实测静默编译为惰性属性；`bind:checked` 已在三后端实现但权威文档宣称「bind:value 唯一双向绑定」；semantic-universe 域 3 至少 25/54 个事件行过度宣称 Flutter 支持；docs/README.md（新用户第一页）与 docs/stdlib.md（幽灵标准库）不在门禁覆盖内且显著陈旧。**「文档虚构归零」原则的机器强制存在盲区——verify 脚本只做结构断言，不做事实断言，且锚点钉在自声明 Superseded 的 BNF 文件上。**
2. **「exit 0 但生成代码损坏/静默分歧」类（5 位成员）**：数组方法在 ref-op 参数内逃逸重写 → 非编译 Dart 且零诊断；Flutter `scrollTo(x,y)` 静默丢弃 x（设计文档承诺的「双轴差异警告」未实现）；`longpress`/`hover`/`toggle` 在 React/OmniView 发射死处理器零诊断（Flutter 正常）；React 事件处理器内信号「写后读」产生陈旧闭包（5 处实例，与 OmniView 行为分歧）；OmniView keyFn 用 `String()` 塌缩对象 key。**全部是门禁 fixture 覆盖不到的「组合形态」。**
3. **注入防线在运行时/动态面有缺口（2 个独立 HIGH）**：React（指定生产后端）动态 URL 属性无运行时消毒——`javascript:` 协议 XSS 可构造，而 OmniView 预览后端反而有 `__omni_sanitizeUrl`，防御倒挂；Flutter 两个文本发射路径（`flutter/view.rs:2193`、`:332`）绕过共享转义——`$` 插值注入是已修 CRITICAL 类缺陷的回归，注入语料未覆盖多子节点形态。

**Pragmatic Enforcer 裁决**：Necessity 7/10、Complexity 6/10、**Ratio 0.86**（目标 <1.5）——「Approve with simplifications」：10 域设计经语料实证非过度设计（19/19 motion 属性、10/10 RefOp、5/5 a11y 均有 demo 消费者），验证网 ROI 可证明（注入语料两次捕获真实注入）；但 `velocity_max`（零消费者全链路实现 + React slice 特例排除）与双 verify 脚本（120 行逐字重复 + 锚点 Superseded 文档）应简化。相比 v1 评审 0.63 与 v2 对齐评审 0.69，比值上升的增量几乎全部来自这两处。

**Key Findings**:
1. **React 事件处理器内信号写后读 = 陈旧闭包（H1，系统性语义偏差）**：`agreed = !agreed; statusText = if agreed ...` 生成 `setAgreed(...); setStatusText(agreed ? ...)`——读到旧值。5 处实例：checkbox 状态文字与显示矛盾、计数器首次点击报 0、emit 载荷 off-by-one、排序方向与标签相反。React 与 OmniView 同源码行为分歧，浏览器套件结构性不可见。
2. **React 生产后端动态 URL 无运行时消毒（安全 HIGH-1）**：`href={q}` 直发，`javascript:` 协议可执行；OmniView 有 `__omni_sanitizeUrl`、React/Vue/Dioxus 没有——防御倒挂。
3. **语义全集台账与编译器事实大面积脱节**：hover/toggle/longpress「全目标支持」实为 web 静默不触发；25/54 事件行过度宣称 Flutter 支持；`theme="dark"` 文档验证清单勾选的行为不存在；`bind:checked` 实现无文档无跟踪。
4. **Flutter `$` 转义回归（已修 CRITICAL 类缺陷复活）**：多子节点文本形态（`<p>价格 $total 元</p>` 是最常见形状）绕过共享转义，生成 Dart 静默插值/损坏。
5. **「GATE PASSED」= 台账绿，非严格绿**：React 门禁 HEAD 实测 tsc 87 错误 + eslint 4（全豁免覆盖、计数准确），其中 44 个隐式 any 闭包、且 TS2304 豁免掩盖一个点击即崩（H2）。

**Critical Actions**:
1. 闭合注入防线两缺口（React 动态 URL 运行时消毒 + Flutter `$` 转义回归）——可构造 XSS / 供应链注入，S 工作量。
2. 修复 React 行为层三缺陷（H1 陈旧闭包线程化、H2 悬空 setter DCE、H3 数字 bind:value）——「地道代码」承诺在行为层失信。
3. 校准「宣称 vs 事实」台账（semantic-universe/设计文档/README/stdlib + 门禁事实断言）——防漂移机制本身在漂移。

---

## System Overview

**评审对象**：OmniLang 语义级 DSL v2 全量重设计（10 域：Ref 系统/值绑定/@for/@themes 主题/样式 $field/节点/事件/闭包唯一化/数组方法/a11y）+ v3 修复轮（ADR-062，2026-08-05 收口）。

**架构风格**：目标无关意图管线——parser（BNF v2）→ semantic（16 验证文件 + 16 lint）→ 类型化 IR（RefOpIR 10 ops / MotionIR / ElementIR.a11y / DesignIR.theme_signal，全部 `#[non_exhaustive]`）→ 每目标 codegen（React/Flutter/OmniView 为 P0，Vue/Dioxus 为次级）。

**验证网**：react-test-app（React 19 + Zustand v5 strict tsc + eslint-react-hooks，豁免台账 29 条）；learn-omniview Flutter 全量 dart analyze；docs-gate（247 块 0 失败）；浏览器套件（OmniView + React，**已入 CI**——deferral #60 已关闭，omniview-e2e job 在 `.github/workflows/ci.yml:238-272`）；注入语料（23 载荷 × 11 插入点 × 5 后端）；floor_matrix（25 项机器验证）；verify-vocabulary.py / verify-semantic-universe.py / verify-syntax-lifecycle.py。

**治理基线**：ADR-052→054→055→056→062 时代链；deferrals 台账（#36-#83，多数已带证据关闭；#60/BUG-A/unused-import 已在 08-04/08-05 关闭——**评审简报中的「已知未决」陈旧，聚合时已校准**）；Flutter warnings 燃尽 1004→671（ADR-066 诚实下限）。

**约束**：编译器为 Poiesis 编辑器服务（WASM 增量编译、每键诊断）；展示项目 learn-omniview 96 个 `.omni` 文件为唯一生产语料；诊断消息中文政策（ADR-056 决策 1，可逆）。

---

## Individual Member Reviews

### Systems Architect — 系统一致性

**Perspective**: 架构脊柱（目标无关意图管线 + 五门禁验证网 + 文档-代码治理链）对三个 P0 目标真正良好；失败模式集中在系统边缘：两个非 P0 UI 目标、两个文档终止的治理域、demo 语料界定的集成覆盖。

#### Key Observations
- IR 真正目标无关且按意图类型化；「同一语义只有一个名字」对 5 个词汇域中的 4 个做了机械强制（event_names.rs 共享表 + 五后端身份测试）。
- Vue/Dioxus 完全在语义 DSL 治理之外——零 MotionIR/RefOpIR/A11yIR/bind:value 处理，直接违反 ADR-052:94「floor 外特性必须硬诊断，绝不静默丢弃」。
- 设计 token 身份以 `var(--x)` CSS 字符串穿过 IR，Flutter 用字符串解析解码——IR 在类属性层不自描述。
- 验证网是真正互补的历史验证系统（每个门禁捕获过不同缺陷类）；omniview 套件已入 CI（deferral #60 关闭）。
- learn-omniview 真实承担可执行文档角色；behavioral-parity 基线存在且诚实记录 B4 跳过。

#### Strengths
1. **目标分级诚实**：`DESIGN_BLOCK_ERROR`（codegen/mod.rs:38-44）正确模式——非 design 目标响亮失败而非丢弃。
2. **CI 重生成纪律**：每个行为门禁先重生成输出再测试，无陈旧产物可绿门禁。
3. **债治理诚实且机制绑定**：豁免实例上限 + deferral 引用 + 陈旧条目失败。
4. **learn-omniview 真实可执行文档**：96 文件三目标门禁 + docs-gate 单源喂养。
5. **行为等价基线诚实**：Task 7.6 显式记录 B4 跳过。

#### Concerns
1. **HIGH-1: Vue/Dioxus 静默丢弃 motion/refs/a11y/bind:value**（`omnilang/src/codegen/vue/`、`dioxus/` 零处理；契约 ADR-052:94）。用户编译 `@enter`/`bind:value` 到 `--target vue` 静默缺功能；`@ref` 退化为无意义字符串属性。v3「静默降级归零」只对 P0 测量——系统级宣称对 5 个 UI 目标中的 2 个为假，且无门禁能发现（feature-matrix CI 只编译）。修复：编译分发处对 Vue/Dioxus 的 motion/ref/a11y/bind:value 发 E_CODEGEN 错误（镜像 DESIGN_BLOCK_ERROR）或警告通道 + 回归测试 + ADR-052 floor 表更新。S。
2. **MEDIUM-1: 设计 token 身份以 CSS 字符串穿过 IR**（encode `ir/lower/design.rs:146-148`；decode `flutter/design_classes.rs:189,539`）。`ClassIR.properties: FxHashMap<String,String>` 使 IR 的 JSON 往返丢失 token 语义；字面 `var(--foo)` 与 token 引用不可区分。修复：类型化 `DesignValueIR { Literal | TokenRef | Mixed }`。M。
3. **MEDIUM-2: 未声明 `token(name)` 静默接受**（`ir/lower/design.rs:503-513` 测试断言坏行为）。`token(prmiary)` 拼写错误编译干净，React 输出死 `var(--prmiary)` 零诊断——「零诊断静默退化」模式。修复：lowering 时校验 token refs（用已传入的 `tokens` 列表）→ error + suggestion。S。
4. **MEDIUM-3: 5 个治理域中 2 个（motion props、token types）事实链终止于文档**（`verify-vocabulary.py:20-24` 等锚定 BNF production，而强制常量是 `parser/top_level/motion.rs:29-49` 的 `SEMANTIC_ANIM_PROPS`）。三文档可自洽地同步漂移。修复：脚本改锚定 parser 常量（`rust_str_array` 辅助已存在）。S。
5. **MEDIUM-4: 「语料零触发」deferral 触发器被动**：语料增长激活延期缺口（正常），语料收缩（特性悄悄掉出 demo）无检测器。修复：emission_surface.rs 扩展 DSL 特性 × demo 页面矩阵 + CI 断言 10 域各有 fixture。S-M。
6. **LOW-1**: `evals` CI job `continue-on-error: true`——结构性无法失败的门禁。
7. **LOW-2**: 文档化目标分歧（滚动契约/steps() 回退）无断言。

#### Recommendations
即时：Vue/Dioxus floor 诊断（S）；未声明 token → error（S）。短期：脚本锚点迁移（S）；token IR 类型化（M）；特性覆盖矩阵（M）；B4 处置（S）。长期：parity 基线扩展（L）；目标分级模型显式化（L）。

#### Risks if Unaddressed
「语义语法 → 每目标最地道代码」对 2/5 UI 目标静默失效；token 拼写错误继续以死 CSS 零诊断出货；治理三元组与编译器同漂移（「文档虚构」以更微妙形式回归）；语料收缩无声侵蚀集成覆盖。

---

### Domain Expert — 语义域建模准确性

**Perspective**: 从「前端功能意图的表达质量」出发，用 release 版 `omnic` 实测编译器行为（10+ 诊断面），交叉核对语义全集状态列与代码生成事实。

#### Key Observations
- 封闭词汇域机制真实：两个 verify 脚本做实质双向交叉断言且当前通过；a11y 词汇 ARIA 化真实落地（`role:"textField"` 产生带建议的 warning）。
- 「floor 外硬诊断」绝大多数面字面兑现：实测 12 项删除语法全部硬错误 + 迁移提示。
- **但语义全集状态列有两处「宣称支持、实际两目标静默不触发」：hover 与 toggle**；longpress 同类问题被诚实标「部分」，hover/toggle 却标「支持」——同一类别跟踪待遇不一致。
- 权威设计文档与 parser 存在三处确定性漂移：`theme="dark"`（宣称自然解析错误，实测静默编译为惰性属性）、`bind:checked`（已实现但宣称「唯一双向绑定」）、`$Store.field`（承诺的迁移提示未实现）。
- 门禁事实源是自声明 Superseded 的 BNF 文件；v2 设计文档（实际权威）的 BNF 是散文、无机器断言。

#### Strengths
1. **主题域语义模型完整自洽**：`@themes`→`@default-theme`→`@theme` 三件套，跨文件程序级验证（单例性/String 类型/初值成员/@default-theme 与信号初值一致性 D-F）。
2. **「同一语义只有一个名字」在值绑定收敛上基本兑现**：`value={sig}` 即使有 @ref 也硬错误（suggestion + 逐字迁移提示）；`ref.value=x`/`capturePointer(id)` 硬诊断；bind:value 方向一致性契约有 fixture 与浏览器实证。
3. **事件别名单表机制彻底消除历史双写漂移**：event_names.rs 唯一 canonical + 五后端身份测试。
4. **a11y 域语义纯度修复彻底**：aria-* 硬错误 + 逐条迁移提示、role 子集 = WAI-ARIA 规范名 + Flutter 借词黑名单、live 值域硬校验、alt 限 img。
5. **effect deps 验证与 for-key lint 把静默缺陷类前移到编译期**。

#### Concerns
1. **HIGH-1: hover/toggle 宣称「支持（全目标）」，web 两目标静默不触发**（`semantic-universe.md:154,157` vs `react/utils.rs:233-287` 无臂兜底、`omniview/view/element.rs:379` addEventListener 直通）。实测 checkbox `@onToggle` 零诊断编译：React 输出 `onToggle`（合成事件仅 details 触发）、OmniView `addEventListener("toggle")`（DOM 无此事件）、Flutter `onChanged`（工作）。「零诊断静默退化」正是本项目历史 HIGH 模式；且 `change`+`e.checked` 已有 web 语义等价物，toggle 本质是 Flutter 便捷形状混入 canonical 表。修复：按 longpress 先例改标「部分」+ 后端发射 warning；或 React 映射 `onChange`+`e.target.checked`。S。
2. **HIGH-2: `theme="dark"` 文档宣称「自然解析错误/响亮失败」，实测静默编译为惰性属性**（`2026-08-02-semantic-syntax-v2-design.md:365,374` 验证清单 [x] 勾选不存在）。T4.4 裁决基于错误前提——`theme="dark"` 只是普通 prop 赋值，语言层无法区分虚构语法与合法 prop。击穿原则 ③⑤ 且 docs-gate 未拦截（不覆盖设计文档诊断表）。修复：(a) 已知 DOM 标签 `theme=` 加 warning（自定义组件豁免）；(b) 如实修订文档取消勾选。S。
3. **HIGH-3: `bind:checked` 是已实现的第二个双向绑定面，权威文档宣称不存在**（parser `element.rs:575-620` 接受任意 bind:名；React/OmniView 均实现 onChange 回写；Flutter 警告跳过；v2 设计文档:31 宣称「唯一双向绑定」；semantic-universe 无条目）。三方漂移 + 真实用户面（checkbox 高频意图）无跟踪。修复：正式化（BNF 补 `bind:checked` + universe 条目「部分」+ 登记册注记）或硬诊断；推荐前者。S-M。
4. **MEDIUM-1: a11y label/role 仅接受字符串字面量——「动态可访问名称」是语义黑洞**（`element.rs:246-266` 只收 StringLiteral；aria-* 已删堵死平台 hack）。修复：设计文档显式记录缺口 + a11y 建立状态跟踪。S。
5. **MEDIUM-2: 门禁事实源指向自声明 Superseded 的 BNF 文件**（同 Pragmatic MEDIUM-2 / Systems MEDIUM-3，详见交叉主题）。M。
6. **LOW-1**: `$Store.field` 删除诊断缺少文档承诺的「用 Store.field」提示（实测 `Unexpected token in expression: Dollar`）。S。
7. **LOW-2**: `alt=` 与 `a11y:{alt}` 双表面——有文档记录、Flutter 映射一致，维持现状。
8. **LOW-3**: 多 design 并存时 @theme 初值校验用全集合并——边缘场景。

#### Recommendations
即时：hover/toggle 台账改标 + 后端 warning（S）；theme="dark" 文档修订（S）。短期：bind:checked 正式化（S-M）；a11y 动态名称缺口记录（S）；`$` 诊断提示补全 + 门禁锚点迁移（S）。长期：事件语义全集按目标交付状态维度 + floor_matrix 加事件行（M-L）。

#### Risks if Unaddressed
用户 web 构建无声交付失效交互（hover/toggle 处理器永不被触发）且台账持续宣称「支持」；按文档预期「响亮失败」的迁移用户得到零反馈；「唯一双向绑定」假设下的重构破坏未文档化实现面；门禁从「防漂移机制」退化为「两份文档互相对齐」。

---

### Compiler & Language Architect — 语法设计健全性与多目标一致性

**Perspective**: 评估 v2 设计契约作为*语言*是否健全、「语法→地道代码」映射是否真实可达且实现正确。用 release 版 `omnic.exe` 实测编译两个生产目标。**底线：v2/v3 工作真实且实质改善——架构健全、多数契约诚实实现、验证网真实强制；但发现 3-4 个 HIGH（一个产生 exit 0 + 非编译 Dart）+ 3 MEDIUM + 1 LOW，全部实证。语义纯度原则在可见表面成立；失败在实现缺口与台账漂移，不在语法哲学。**

#### Key Observations
- 10 域设计结构健全：BNF v2 主轴无歧义；删除语法全部硬错误 + 迁移提示。
- 「语法→地道代码」映射多数真实：bind:value→value+onChange（实测）、@for→Column/ListView.builder+ValueKey（实测）、React 主题接线 slice selector + data-theme、Flutter ThemeModeNotifier+MaterialApp(themeMode:)、a11y→aria-*/Semantics、classes/variants→BoxDecoration。
- IR 真正支撑多目标：RefOpIR 10 ops（SetValue 正确删除）、TriggerKind 类型化参数、ListRenderingIntent.key_expr/scroll_hint。
- 验证网真实：floor_matrix 25 行机器验证、三目标门禁、855 元组注入语料。
- **但机器可以用 exit 0 说谎**：实证发现 fixture 矩阵未覆盖的组合（ref-op 内数组方法、web 上 longpress、Flutter 双轴 scrollTo）。

#### Strengths
1. **Ref-op 语义 pass 是教科书级意图识别**：只重写声明 ref 的方法调用、自走子节点、硬错误 DOM 形状、`capturePointer` Flutter floor-out = 编译错误 + 生成 `throw UnsupportedError`——最强诚实降级。
2. **跨文件语义验证正确**：validate_theme 在 IR 合并后运行（@themes 与 @theme 信号可在不同文件）、F1 字面初值成员校验、D-F 双声明一致性（防 React/Flutter 启动主题分叉）；>2 主题 Flutter 折叠是编译警告非静默。
3. **域 8「一种意图一种写法」纪律语法级强制**：管道闭包唯一、`count()` 无谓词逐字报错、`$field`/`$signal` 样式通道三分类消除标识符变形类 bug。
4. **React store slices 真实接线**：per-field selector 默认订阅计划 + motion 通道覆盖。
5. **Flutter codegen 在修复区真正地道**：Riverpod 3.0 `ref.watch` 合规（lazy itemBuilder 提升 build local）、单次求值、const 构造门禁、快照守卫语义绑定（焦点偷窃类修复）。

#### Concerns
1. **HIGH-1: 数组方法在 ref-op 参数内逃逸重写 → 非编译 Dart，exit 0 零诊断**（`validate_array_methods.rs:393` `ExprIR::RefOpExpr => {}` 私有 walker 不下钻；共享 walker `ir/walk.rs::walk_expr:104-141` 是下钻的）。实测 `box.setSelection(DataStore.items.filter(...).len(), 5)` → 生成 `.filter(...)` verbatim → `List.filter` 在 Dart 不存在 → dart analyze 失败但编译器报成功。修复：walker 镜像 walk_ref_op 下钻 + 组合 fixture。S。
2. **HIGH-2: Flutter `scrollTo(x,y)` 静默丢弃 x**（`flutter/ref.rs:1261-1273` 无警告路径）。实测 `box.scrollTo(100, 200)` → `_boxScroll.jumpTo(200)` 零诊断；设计契约明确承诺「双轴差异警告」，上轮 LOW-11 排期未实现——**评审中发现的设计文档承诺唯一完全未兑现项**。修复：双轴时发 `[flutter] warning`。S。
3. **HIGH-3: `longpress` 在 React/OmniView 发射死处理器零诊断**（`react/utils.rs:285` canonical 兜底 `onLongpress`；event_names.rs:35 canonical）。实测 `@longpress` → `onLongpress`（React 19 合成事件/内在属性均不存在 → tsc error 或运行时丢弃）；OmniView 监听永不触发；Flutter 正确映射 `onLongPress`。语义全集把「处理器名透传不触发」当作诊断记录——但**没有编译器诊断**，违反原则 3 诚实降级。修复：validate_event_names 对无目标映射的 canonical 事件发 warning + fixture。S。
4. **HIGH-4: 设计契约文档化的单向 `value={sig}` prop 不可表达**（parser `element.rs:85-125` SEMANTIC_BINDINGS 含 "value" → 裸标识符重分类为 RefBinding；`validate_semantic_bindings.rs:60-76` 无条件 error，提示「普通 value prop 单向」——但该形式恰好被重分类为已删形式，无法写出）。设计文档 D2 与域 1 BNF `value_prop` 直接被 parser 遮蔽，诊断表无此行——未文档化实现偏差 + 误导性提示。修复：仅当元素带 @ref 时 error（v1 双向形状需要 @ref）+ 修正提示 + 诊断表同步。S-M。
5. **MEDIUM-1: semantic-universe 域 3 至少 25/54 事件行宣称「支持 无（全目标）」但 Flutter warn-and-skip**（`semantic-universe.md:149-265` vs `flutter/utils.rs:70-95` 覆盖 ~15 名、`flutter/view.rs:1697-1723` 未映射警告）。实测 `@onScroll` → warning「not supported; skipped」而台账说「支持」。P3 机制本身漂移——权威文件内部就撒谎。修复：改标「部分」+ 实际诊断文本；长期从 codegen 白名单生成支持矩阵。M/L。
6. **MEDIUM-2: `@for item in list [scroll]` 空白敏感且 BNF 未文档化**（`directives.rs:108-119`：仅当有空格间隙时 reinterpret 为 scroll hint）。`items[scroll]`（无空格）与 `items [scroll]`（有空格）语义不同且零诊断。修复：BNF 文档化 + 触发时 note 级诊断。XS-S。
7. **LOW-1**: class-list 解析按位置保留 15 个 HTML 布尔属性关键字（`element.rs:38-61`）——`class: btn disabled` 的 disabled 变布尔属性。确定性但未文档化。XS。

#### Recommendations
即时（全 S）：修 HIGH-1 walker 下钻 + 组合 fixture；HIGH-2 双轴警告；HIGH-3 longpress warning；HIGH-4 value 判别器按 @ref。短期（M）：semantic-universe 25+ 行重分类 + verify 脚本事实交叉检查；`[scroll]` 空白规则文档化；bespoke walker 审计。长期：per-target 支持矩阵生成；fixture 矩阵按*组合*扩展（ref-op×数组×@for×motion）。

#### Risks if Unaddressed
HIGH-1/2/3 共同重造「编译器报成功、输出损坏/静默分歧」类——正是产生 6-CRITICAL 周期与两轮评审的那类，现在在门禁不可见的组合上；HIGH-4 让设计文档旗舰域 1 的 BNF production 被诊断遮蔽；universe 台账漂移掏空 P3 机制——门禁脚本认证虚构。

---

### TypeScript & React Specialist — React 生成代码地道性

**Perspective**: 以 React 工程师读编译器输出的眼光：结构骨架（受控输入、slice selector、keyed 列表、React.memo、useEffect 主题接线）真正地道——但**事件处理器的语句级语义被朴素翻译，产生陈旧闭包读取，静默反转或位移用户可见行为**；「strict tsc 绿」只通过 87 错误豁免台账达成，最大类是无类型 codegen 内部闭包。

#### Key Observations
- 门禁诚实但生成代码非严格绿：HEAD 实测 `tsc --noEmit` 87 错误（TS7006×44、TS2345×11、TS2339×10、TS2554×6、TS2353×6、TS18048×4、TS2322×3、TS2304×2、TS2367×1）+ eslint 4；每类与台账计数逐项吻合——台账准确、上限机制工作。「GATE PASSED」=「所有已知、已分类、已延期的错误在上限内」，非「输出满足 strict tsc/eslint」。
- 系统性陈旧闭包读取：DSL `x = ...; use x` 序列生成 `setX(...)` 后读旧 x；5 处实例跨 4 文件，浏览器套件 17 测试结构性不可见（断言重渲染后显示，不断言顺序语句）。
- 只写信号 DCE bug 遗留悬空 setter → 点击运行时 ReferenceError（TS2304 豁免后编译绿）。
- motion 域地道性退化最重：~70 行命令式拖拽手势 effect 三次内联复制于 833 行组件，无类型辅助闭包（44 TS7006）。
- 状态管理接近手写 Zustand v5——slice selector 全覆盖、无整库订阅；唯一偏差是派生值（`__getters`）物化进 store 状态而非 selector 推导。

#### Strengths
1. **Slice 订阅纪律 = 设计契约**（spec §域 5 手写基线）：`TodoList.tsx:13-17` 五 per-field selector；v2 M2 回归（整库订阅）已修。
2. **bind:value 双向方向真实且地道**：`value={selected}` + `onChange={(e)=>{setSelected(e.target.value)}}` 与 spec 手写基线逐字匹配；主题接线 `useThemeStore.ts:9-11` 与手写基线字节级一致。
3. **Keyed @for 稳定身份 key**：`key={item.id}` + 索引 key 回退时诚实内联注释；iterable 单次求值（IIFE）正确处理。
4. **Memoization 卫生正确**：React.memo 页面、useCallback prop、motion 表提升模块级（exhaustive-deps 91→1 真实）。
5. **豁免台账是模范工程**：per-file 作用域、计数上限门禁、未用豁免检测、deferral 归属；a11y 发射地道。

#### Concerns
1. **HIGH H1 — 事件处理器内信号写后读陈旧闭包（系统性语义偏差）**：`FormControlsDemo.tsx:17-19,27-29,76-78`、`EmitChild.tsx:13-16`、`DataTable.tsx:27-30`。`agreed = !agreed; statusText = if agreed {...}` 生成 `setAgreed(a=>!a); setStatusText(agreed ? ...)`——读旧值。用户可见后果：checkbox 勾选后状态行显示「Not agreed」而相邻显示「Checked」；链接计数器首次点击报 0；Emit 载荷 off-by-one（`"emit #0"` 而 DSL 规定 `"emit #1"`——跨组件契约传播）；`toggleSort` 用前置 sortAsc 排序——**列表排序与标签相反**。OmniView 目标（信号运行时）立即读新值——同源码跨目标行为分歧。修复：codegen 事件发射线程化写入值（`const nextAgreed = !agreed; setAgreed(nextAgreed); setStatusText(nextAgreed ? ...)`，复用 mutation_value_expr 机制）。M。
2. **HIGH H2 — 只写信号 DCE 遗留悬空 setter：点击运行时 ReferenceError**（`AdvancedPseudoDemo.tsx:13,16,72,79`，豁免 TS2304×2，deferral #72）：`signal variant` 从未被读 → state hook 被剪枝，但 setDefault/setDark 中的 mutation 未被剪枝——DCE 分析与发射侧剪枝分歧。点击「Dark Variant」抛 `ReferenceError: setVariant is not defined`——未处理交互崩溃，被豁免掩盖。「豁免债 = 运行时 bug 类」模式仍活跃于 HEAD。修复：有可达 mutation 的 signal 保留 state hook（或连 mutation 一起剪枝 + warning）。S。
3. **HIGH H3 — 数字类型 signal 的 bind:value 喂入字符串**（`FormControlsDemo.tsx:148,173`，TS2345×2，deferral #64）：`setAge(e.target.value)` 而 `age: useState<number>`——类型谎言有运行时后果：任何下游算术变字符串拼接（`age+1`→`"301"`）。修复：目标 signal 类型为数字时发射 `Number(e.target.value)`/`valueAsNumber` + fixture。S。
4. **MEDIUM M1 — 「strict tsc 绿」是台账绿，87 个活错误，44 个隐式 any motion 辅助**（`ElementRefDemo.tsx` 16 TS7006+5 TS2339+4 TS18048+3 TS2345 等）：React 工程师读到无类型事件闭包与未守卫的可能 undefined ref 读取（TS18048×4）。给 codegen 内部闭包打类型（`(cfg: string|number[]|null)`）即删最大豁免类。
5. **MEDIUM M2 — 拖拽手势逻辑 3× 内联而非 hook**（`ElementRefDemo.tsx:160-397` 三段 ~75 行近同 useEffect + 模块级可变 `__omni_activePointerId` 共享全局——并发渲染/SSR 不友好）：手写 React 形状是 `useDrag` hook。833 行组件即由此复制造成；任何手势逻辑修复需在三处应用。L。
6. **MEDIUM M3 — 伪 `handler({})` 参数发射（TS2554×6，deferral #62）**：bind:value 与 @onChange 共存时 setter 分支调 0 参 handler 为 `handleSelect({})`；无 setter 分支已有空事件特判——内部不一致。S。
7. **MEDIUM M4 — 派生值物化进 store 状态**（`useTodoStore.ts:23-40` `__getters` 每次 `__set` 重算全部 getters 含无关字段）：手写形状是 selector 推导。非紧急。
8. **MEDIUM M5 — 所有绑定发射 `let`**（deferral #36 过渡修复）：整个输出非 const——手写代码的对立面，且移除编译器自身安全网（真实重赋值 bug 与发射形状不可区分）。应保持关键路径。
9. **MEDIUM M6 — 事件数据经无类型 `e.target.*` duck-typing（TS2339×10，deferral #73）**：手写 React 会 `(e: React.ChangeEvent<HTMLInputElement>)` 或 `e.currentTarget as HTMLInputElement`。发射显式合成事件参数类型可一次折叠两个豁免类。
10. **MEDIUM M7 — `(value: void) => void` 事件 prop 类型 + 死载荷构造**（`SlotBox.tsx:7,44`）：DSL 声明无载荷但生成 prop 类型 `(value: void)=>void` 且调用点构建 11 字段指针事件对象（`e.clientX ?? 0`...）——类型撒谎 + 死分配。
11. **LOW**: IIFE 包裹列表发射（正确但手写会前置计算）；`<React.Fragment key={...}>` 包裹单子（key 应属子元素）；`App.tsx:52-86` 九层三元金字塔（手写是查找映射）；`DataTable.tsx:18-20` setState-in-effect 挂载初始化（手写是 lazy useState）。

#### Recommendations
即时（S）：H2 DCE 一致性；H3 Number()/valueAsNumber + fixture；M3 空事件特判复用。短期（M）：H1 写入值线程化（恢复 DSL 顺序语义）；Playwright 行为断言（H1 类结构性不可见）；M1/M6 类型发射（删 54/87 豁免）；M4 selector 推导。长期（L）：H1 在 IR 层 SSA-lite 重写（Flutter/Dioxus 不继承）；M2 useDrag hook 提取；手写基线对照一致性套件。

#### Risks if Unaddressed
跨目标行为分歧成为常态（写后读同处理器在 React vs OmniView 行为不同——ADR-054 原则 1 已在 HEAD 被 5 处实例违反）；豁免台账成为永久运行时 bug 注册表（H2 证明豁免不只盖风格偏差——当前掩盖用户可见崩溃与类型谎言）；任何 React 工程师读 ElementRefDemo.tsx 会拒绝其为可评审代码——「最地道代码」宣称当前只对 view/state/stores 层站得住。

---

### Rust Expert — 编译器实现（Rust 侧）

**Perspective**: 以编译器构造审查者评估 Rust 侧管线（parser→AST→semantic→IR→codegen）的类型驱动、内存安全、错误响亮性、注入消毒单源覆盖。实证方式：编译 scratch 测试对照 flutter 后端并 diff 生成 Dart。

#### Key Observations
- 管线真正三阶段且端到端类型化：parse 恒返回 Program + Vec<ParseError>，ParseError 带可选类型化 DiagnosticKind，转换恒 Error 严重级——硬诊断响亮，绝无 parse-and-ignore。
- 消毒器是真正单源——**有一处不完整接缝**：编译期 `sanitize_css_value` 与两个运行时 JS 副本从一个定义生成，消除三副本漂移；但 Flutter 的 Dart 字符串转义碎片化：共享 `escape_literal_str` + `$`-replace 与两个手写 `t.replace('\'', "\\'")` 路径共存——**编译实证确认回归**。
- IR 语义纯净无平台泄漏；递归安全是工程化而非假设（深度守卫实测校准 17KB/帧 × 40 层 × 34% 余量）。
- 唯一生产 unsafe 是 2 个 FFI 函数，均带 SAFETY 注释；其余 expect 全在 cfg(test)。

#### Strengths
1. **消毒器架构 + 语料纪律**：hex 转义归一化 + 解码后 CR/LF/TAB 剥离（`java\ascript:` 洞）、`;` 截断、结构性拒绝、is_dangerous_url 四后端纵深防御、tokenizer 状态感知语料 + 控制编译防空洞。
2. **诊断端到端类型化**：DiagnosticKind code_str()/serde 往返、统一 9 字段 DiagnosticJson FFI 信封（显式 null、小写严重级）、LintRuleId 类型化枚举。
3. **parser 健壮性**：实测深度守卫、多错误恢复回归测试、@-关键字文本安全套件锁定 token_text 防 lexer 漂移。
4. **Pass 顺序文档化**（analyse.rs:66-91）：7 个 IR pass 顺序承载、run_ir_passes 单点编排。
5. **unsafe 纪律与 no-panic 基建**：fuzz/property 门禁覆盖消毒器 + 编译 + codegen。

#### Concerns
1. **HIGH — Flutter Dart 字符串转义两条发射路径被绕过（`$` 插值注入/生成 Dart 损坏）**（`flutter/view.rs:2193` 全文本拼接路径、`:332` TextSpan/Text.rich 路径）：两处手写 `t.replace('\'', "\\'")` 而非共享 `escape_literal_str` + `$`-转义（`:222-227`/`:1068-1069` 用对了）。实证：`<p>价格 $total 元</p>` 发射 `Text('价格 $total 元')`——Dart 字符串 `$ident` 插值作用域内变量（有 total signal 时静默错值，否则编译错误）；`${...}` 形文本可执行任意 Dart 表达式。**该文件自身 :218-221 把此向量记录为注入语料发现（Task 3.1）——这些路径后加（a17e6e0d、fd94325a C2b #5）将其回归**。反斜杠案也损坏（`C:\path` → `\p`）。语料漏检：rawtext 点（injection.rs:693-695）单子节点 → 走正确转义的 format_view_tree 路径，多子节点形态无覆盖。修复：两行改用 `escape_literal_str(t, '\'')` + `.replace('$', "\\$")`，或建共享 `escape_dart_string` 统一三种机制（JSON dollar_escape 配置/escape+replace/replace-only 现存三态）；语料加多文本子节点点。S。
2. **MEDIUM — semantic pass 编排在 ir::lower 内创建 ir↔semantic 模块环与不纯 lowering**（`ir/lower/mod.rs:345` 调 `analyse::run_ir_passes`；:342 文档声称「编排已移出 lower」——只移了代码没移调用）：lower 不能脱离 7 个验证 pass 运行；新 IR pass 必须经 IR 模块接线；单文件 run_pipeline 与多文件 analyse_multi 入口分歧风险。修复：lower 纯化，run_ir_passes 移入 2-3 个管线入口 + 断言测试。S。
3. **LOW — 死重复 op 名契约**：`parser/ast/mod.rs:207-231` `BinaryOpKind::as_ir_str` 零调用者，重复 IR 的 serde 名（"lteq"、"??"）——可静默漂移的第二事实源。删除或统一。S。
4. **LOW — 诊断位置用字节偏移/零代替 span**（`semantic/mod.rs:122` `col: declared_at as u32`；collect.rs:206,226-237 变体/导入 declared_at: 0）：重复符号诊断指向错误或 (0,0) 位置；(0,0) 错误在大文件实际不可找。Symbol 存 Span。S-M。

#### Recommendations
即时（S）：修 flutter/view.rs:2193/:332 转义 + `$` 载荷多子节点回归测试 + 语料 rawtext 点加双子变体；删 as_ir_str。短期（S-M）：run_ir_passes 移出 lower + 边界断言；Span 线程化。长期（M-L）：sanitize_css_value 结构性黑名单 → TokenTypeIR 白名单（文档已点名精化目标）；三机制 Dart 转义统一。

#### Risks if Unaddressed
`$` 转义回归是最具体风险：合法带 $ 文本在常见混合内容段落形态产生静默错值/编译失败/（`${...}` 回退）执行任意表达式——语料存在的目的就是堵这类供应链向量，且是已修 CRITICAL 类缺陷的重引入；lowering 不纯使下一个 IR pass 成为接线事故；无类型诊断位置劣化编辑器体验。

---

### Security Specialist — 注入防御体系

**Perspective**: 把注入防御作为闭环系统追踪：从 .omni 源经 lowering 到五后端起落点及运行时，压力测试语料的两个主张——(a) 插入点真的枚举了发射面，(b) 消毒单源无旁路——包括语料静态无法触及的*动态*（运行时数据）路径。

#### Key Observations
- 语料按构造是静态字面量语料：`embed()` 只把载荷放进引号字符串字面量；动态面（`href={signal}`、bind:value→state→属性、@for 项绑定、事件参数）从不被行使。这些面的唯一防线是*发射的运行时辅助函数*，测试套件从不执行它们。
- 运行时 URL 消毒在五个后端中恰好存在于一个：OmniView `__omni_sanitizeUrl`；React/Vue/Dioxus 直发 `href={expr}`，仅编译期 is_dangerous_url 覆盖静态字面量。
- 发射面地图有它要关闭的同一盲点：react/vue/dioxus 的 URL 行只注册静态发射点；动态 URL 面在 3/5 后端未注册。
- 转义顺序正确性真实且逐后端验证（Vue `&`-first 闭合实体重解码向量；OmniView 模板字面量转义 \、backtick、$；escape_html 先 &；Flutter 文本与 a11y label 双路径转义 $、role 降级注释中和 */；主题名标识符文法约束使 `[data-theme="..."]` 选择器注入构造上不可能）。
- ADR-062 K1 未引入新注入面。

#### Strengths
1. **真正单源消毒器且漂移失败已闭**：三副本漂移（React 运行时 sanitizeCss 丢失 hex 归一化等）经 shared/sanitize.rs 生成双运行时副本修复；结构性拒绝不在运行时镜像是有文档的合理分歧（setProperty 限定单声明）。
2. **语料严谨性卓越**：模式缺失断言（剥离字符串/注释后残留，非仅可编译）、逐点逐后端控制编译防空洞、Vue 解码后编码建模、tokenizer 状态感知扫描、`#[cfg(all(...))]` 门控防后端静默跳过、双向防腐发射面地图。
3. **类名表面是代码库最强闭环**：lowering 字符集门 + 发射侧 escape_literal_str + 共享 E0006 handler 突破门全五后端一致。
4. **Design CSS 每个声明发射点均消毒**；运行时动态 token 路径包消毒器。
5. **运行时 sink 构造安全**：textContent/setAttribute/setProperty/classList；React 后端无 dangerouslySetInnerHTML（唯一 innerHTML 是编译期转义的静态模板）。

#### Concerns
1. **HIGH-1 — React/Vue/Dioxus 动态 URL 属性值未消毒（运行时 XSS）**（`react/view.rs:319-323` `_` 臂直发 `attr={expr}`、:335 bindings；vue/template.rs:57-60,67；dioxus/view.rs:51-64；对照 omniview/view/element.rs:160-164；覆盖地图 emission_surface.rs:497-532 只注册静态）。**React 是显式指定的生产后端**（omniview/utils.rs:146-148「Use the React production backend instead」）。组件绑定动态值（`<input bind:value={q}/>` + `<a href={q}/>` 或 prop `link: String` 下传）编译为裸 `href={q}`——用户粘贴 `javascript:alert(document.cookie)` 并点击即执行。OmniView 的 `__omni_sanitizeUrl` 证明动态 URL 值在本项目自身威胁模型中——另外三后端只是缺机制。语料结构性不可见（载荷在编译期不存在）。修复：shared/sanitize.rs 扩展 `sanitize_url_helper_js`（镜像 __omni_sanitizeUrl：trim→lowercase→剥离 CR/LF/TAB→阻断 javascript:/data:/vbscript:→'#blocked'），React 输出每模块发射一个共享 helper，六个 URL 属性（href/src/srcset/action/formaction/poster）发射点包裹；Vue/Dioxus 同镜像；注册动态路径行。S-M。
2. **MEDIUM-2 — 发射的运行时消毒器从未被执行**（`omniview/tests_regression/sanitizers.rs:13-96`、shared/sanitize.rs:157-231 断言生成 JS 的子串存在而非行为）：`__omni_sanitizeUrl`/`__omni_sanitizeCss`/React sanitizeCss 是所有动态值的唯一防线，且是手维护 JS 模板镜像 Rust 逻辑——正是已漂移过一次的安排（React 副本丢失 hex 归一化）。改模板保持关键字但换解码顺序的编辑会通过全部测试并打开活旁路。修复：node/vitest 执行发射的 helper 对 23 载荷断言*行为*；语料加动态绑定模式（载荷入 signal → 绑到 href/src/attr/style，断言发射的绑定代码经运行时消毒器）。M。
3. **LOW-3 — 陈旧注释与消毒器行为矛盾**（injection.rs:83-89 声称 alert( 不在阻止列表；sanitize.rs:118-123 实为 task 1.4 添加阻止）：行为安全，注释误导未来安全工作。
4. **LOW-4 — ADR-056 语料计数陈旧**（855 元组 vs 实际 23 载荷 × 11 点，design-token 点在 Vue/Dioxus 跳过）。
5. **NOTE-5 — 发射面地图无 @for/ref/theme-values 行**：今日是共享代码防御（非活洞），风险在未来的 for 模板专用发射路径分歧不触发地图。

#### Recommendations
即时：React 运行时 URL 消毒器（S-M，唯一可构造 XSS 于生产后端）；Vue/Dioxus 镜像（M）。短期：JS harness 执行运行时消毒器行为断言（M）；语料动态绑定模式（M）；修陈旧注释与 ADR 计数（S）。长期：JS 运行时消毒器从 Rust 逻辑生成（单事实源）或一致性测试（L）；发射面地图扩展 @for/ref/theme-values + 新发射路径必须带地图行（L）。

#### Risks if Unaddressed
React 生产输出对任何数据驱动 URL 绑定携带活 XSS 向量，而预览后端（非生产路径）是唯一受防的——不对称倒挂部署指引；运行时消毒器漂移静默复发（子串测试放过行为可绕过的漂移副本）；语料对动态层的「全绿被读作封闭」失败模式在运行时边界再现。

---

### Performance Specialist — 生成代码运行时与编译管线性能

**Perspective**: 三后端 @for keyed 生成、主题切换传播、motion 热路径是否产生地道（合成器友好/keyed 高效/按需重渲染）生成代码，及语义验证网与 lint 在编辑器主路径（poiesis/WASM 增量编译）上的编译时成本。

#### Key Observations
- 三后端 @for 生成已是「性能地道」基线：React `.map+key`+单次求值；Flutter `[scroll]` 惰性 ListView.builder + iterable 提升消除 O(N²)；OmniView 键控克隆复用 + FLIP + 一次性委托。v2 修复的 keyFn 硬编码/重排不重渲染已落地。
- 主题切换传播路径最优形态：React/OmniView 写 data-theme + CSS 自定义属性零 DOM 重建；Flutter 纯函数 ThemeData builder 仅切换时重建。
- motion 全走合成器友好路径：React WAAPI el.animate()（合成器线程）+ 相等性守卫；Flutter AnimatedBuilder child 预构建 + 每轴合成单一 Matrix4。
- 增量编译是「半增量」：缓存跳过 lex/parse/semantic/lower，但全量 validate_multi_ref、lint、merged-IR 克隆重建、optimize、全部文件全量 codegen 每次按键都跑——而 codegen 恰是 0.31s/20 文件基准主要耗时。
- 性能债处置状态优于简报：__omni_el 缓存被 isConnected 检查 + miss 删除约束（ADR-038 P0-1 已缓解）；信号批处理按 ADR-037 门控挂起；**基准套件不覆盖任何热路径**。

#### Strengths
1. **React @for 单次求值 + 显式 key 策略**（IIFE 提升杜绝双重求值；无 key 时显式索引 key + 注释「NEVER String(item)——对象项折叠为 '[object Object]'」）。
2. **Flutter [scroll] 惰性列表 + 迭代器提升**：计算型 iterable 提升为 build local，itemCount/itemBuilder/@empty 三处共享；无 hint 时 Column 文档流而非无条件 ListView.builder（历史 CRITICAL 已修）。
3. **React 重渲染粒度**：per-field Zustand slice selector；prop fn useCallback + React.memo；computed→useMemo/useCallback；motion 上下文 store 引用进 slice 通道——只改字段才重渲染。
4. **OmniView 键控列表 DOM 纪律**：克隆 nid 后缀稳定、DocumentFragment 批量插入、事件委托只挂一次、FLIP 只跑 keyed 路径、消失 key 定向清理。
5. **编译管线并行化与缓存正确性**：per-file rayon、IncrementalCache 传递失效 + LRU 50、缓存命中保留诊断、接入 poiesis 主路径与 WASM。

#### Concerns
1. **HIGH — OmniView keyFn 用 `String()` 强制转换 key，对象 key 全折叠**（`list.rs:166-173` `const keyFn = ({item_var}) => { try { return String(<key_js>); } ...`）：key 表达式为对象本身（`key={item}`）时全项 `String` 为 `"[object Object]"` → `_forKeys` Map 塌缩到同一克隆 → 列表只渲染一个克隆，键控复用/FLIP 机制形同虚设。`for-key` lint（`semantic/lints/for_key.rs:94-105`）只在 key_expr **缺失**时警告对象项——用户按 lint 建议补 `key={item}` 恰好踩中塌缩，静默通过验证。React 端明确注释「NEVER String(item)」并退化为索引 key；Flutter ValueKey 用对象恒等不塌缩——**同源码三端行为分歧，仅 OmniView 静默损坏**。修复：编译期 for_key.rs 用已有 infer_item_kind 类型信息对对象类型 key 表达式报错/警告（要求 .id 等字段访问）；运行时兜底 typeof key === 'object' 时 JSON.stringify 或退化 _idx。S。
2. **MEDIUM — 复用克隆绑定查找退化为逐绑定 subtree querySelector**（`list.rs:359-361,400-402,424-426,466,701`）：`_unid = "{nid}_" + _idx` 用**当前索引**，复用克隆携带**创建时索引**后缀——前插/删除致索引位移时 `_elCache.get(_unid)` 全 miss，每绑定走 `clone.querySelector('[data-nid^="base_"]')` 子树查找——O(绑定数×子树深度) 每次渲染。key 最有价值场景（项位移）退化最重。修复：克隆创建时同一次遍历把 nid→el 映射挂克隆本体（`clone.__omniEls`）。S-M。
3. **MEDIUM — 非 keyed @for 每次效果重跑全列表 DOM 拆除重建**（`list.rs:148-155`）：无 key 路径每次信号变化全量 removeChild + 重 clone + 子树重命名 + 全部绑定重跑——O(N) 抖动，输入焦点/滚动位置丢失。React 有索引 key 兜底，Flutter 依赖 Dart 元素复用；OmniView 无兜底，for-key lint 只覆盖对象项。修复：发射索引 key 键控路径或 lint 警告非键控长列表。S-M。
4. **MEDIUM — 缓存路径仍全量跑验证/lint/IR 重建/全部文件 codegen**（`analyse.rs:591,728-733,752,757/764,767`；`omnilang/src/lib.rs:344-391`）：依赖图（dependents 反向边）已存在，只用于分析失效不用于 codegen 增量。poiesis 每键延迟下限随规模线性增长（20 文件 ≈ 数百 ms 级）。修复：lint+单文件校验门控到 changed+dependents（M）；增量 codegen 只重发射受影响文件（OmniView 已有 per-component delta 机制可推广）（M-L）。
5. **LOW — 基准套件不覆盖热路径**（`benches/pipeline_bench.rs` 单文件、仅 analyse_multi、无 codegen 无缓存）：0.31s/20 文件无人再测量；缓存命中对比、三后端 codegen、多文件语义网均无基准。修复：3 个 criterion bench。S。
6. **LOW（信息性）— ADR-037 信号批处理按门控挂起**：触发前第一步是 P1-3 每属性 style effect 合并，未做。非缺陷。

#### Recommendations
即时：keyFn 对象 key 塌缩（编译期 lint + 运行时兜底）（S）；克隆本体挂 nid→el 映射（S-M）；补基准（S）。短期：非键控索引 key 兜底（S-M）；增量 codegen（M-L）；lint 门控 changed+dependents（M）。长期：IR 优化 passes 扩展（M）；长列表自动惰性化评估（M）。

#### Risks if Unaddressed
key 塌缩：用户按 lint 建议补 key 的「正确」动作在 OmniView 生成静默损坏列表——v2 主推 keyed 能力在最常用法上失信，跨后端分歧难归因；querySelector 回退：键控列表恰在项位移时退化；半增量：20 文件以上每键数百 ms 且无上限；基准缺口：全部无回归保护。

---

### Maintainability Expert — 可维护性与技术债

**Perspective**: 债务治理（台账、单调门禁上限、ADR 裁决）异常强，包裹机械上良好结构的编译器——剩余风险不是蔓延或巨石，而是三类小型漂移危害：重复的管线编排、被误述的共享抽象层、双语诊断表面。全部低成本可修，当前无一致病 bug。

#### Key Observations
- 简报已知债快照陈旧且向好：deferral #36（let-mut→const）已关闭（`ir/types.rs:539-546` is_mut 落地）；#37/#49/#50/#55/#60 已关闭（`deferrals.md:323-328`）；仍开 #43（unused-import 假阳性）、#81（ADR-066 裁决 671 诚实下限）。
- 共享/特化 codegen 边界真实且大体良好，但有三形状：(a) 组合/委托（FlutterFormatExpr 包 DartFormatExpr；dioxus 复用 RustFormatExpr）；(b) trait+override（react ~17 override、vue 4）；(c) **完全分离**（omniview 自实现 expr_to_js 与 stmt.rs，零共享 FormatExpr/FormatStmt）。Rust 穷尽性迫使新 ExprIR 变体双世界处理，缓解但未消除双实现成本。
- 文档漂移机械化而非仅巡逻：四门禁脚本交叉钉住 BNF/登记册/全集/生命周期对编译器事实源；唯一残余缺口 #82 有文档与触发。
- 中文诊断是唯一系统性英文代码库分歧，但耦合在测试非生产控制流：生产按 DiagnosticKind 匹配；~12 测试文件断言中文逐字文本。ADR-056「可逆（中）」准确，但逆转路径触及每个测试断言 + 设计文档诊断表。
- 提交量仍高（11 天 914 commits）但架构在收敛非扩张：v3 关闭 7/8 个 v2 时代 HIGH/MEDIUM deferral；新条目全带触发。

#### Strengths
1. **run_ir_passes 单点编排**（analyse.rs:92-157）：7 个 IR 验证 pass 移出 ir::lower，顺序理由文档化承载——修复过 lib.rs vs ffi.rs 重复的先例。
2. **Lint 框架 1 文件 1 行扩展路径**（LintRule trait + all_rules() 注册表 + 两阶段 AST/IR check）。
3. **单调 per-instance 上限门禁**：verify-react.exemptions 29 条 per-file 计数上限 + deferral 链接 + 陈旧失败；Flutter warnings 879→700→671 单调；check_exemption_ledger.sh 强制 deferral 链接——编译器代码库中见过的最强豁免治理。
4. **表驱动验证**：validate_ref_ops 数据表而非散乱条件。
5. **大文件大而因子化非巨石**：flutter/view.rs 3089 行 = ~40 小函数；文件大小跟踪真实表面积。

#### Concerns
1. **MEDIUM-1 — 合并后管线阶段序列三处复制粘贴**（`crates/omnilang-core/src/lib.rs:114-140` run_pipeline、`analyse.rs:470-502` analyse_multi、`analyse.rs:752-774` analyse_multi_cached，后者还重实现整个管线形状 ~264 行）：`optimize → validate_theme → validate_event_names → lints::run_all → warning 分区` 三处 verbatim；新增程序级 pass 需三处同步编辑。现实失败模式：pass 只进两处，单文件（WASM/编辑器/poiesis 每键）与多文件（CLI/CI）分歧——「编辑器说 OK/构建失败」类；代码库已为此类修过一次（run_ir_passes 提取），程序级阶段被留下。修复：提取 finish_pipeline 辅助三处调用。S。
2. **MEDIUM-2 — `JsFormatExpr` 是死生产代码且文档注释误述共享层**（`shared/expr.rs:568-570` + 文档 8-15 声称「JS (React, Vue, JavaScript) 家族」）：grep 实证零生产消费者，仅自文件测试；React/Vue/JavaScript 各从头实现 FormatExpr。死而测试通过是最坏形态——文档使新后端贡献者误用或扩展它；「JS 家族默认」实为 vue 形状而 react 忽略。修复：删除 + 修正模块文档（S）或委托化（M）。
3. **MEDIUM-3 — Omniview 是完全独立格式化家族，边界未文档化**（`omniview/expr.rs` expr_to_js ~380+ 行、`stmt.rs` 434，零 FormatExpr 引用）：每个新 ExprIR 变体双世界实现（穷尽性把遗漏变编译错误——好；语义工作双倍——现实）。部分分歧有正当性（运行时注入耦合 __omni_subs/__omni_forGetState/editor_mode）；但共享层无文档说明服务对象。修复：boundary note（S）。
4. **MEDIUM-4 — 双语代码库：中文诊断 + 英文其他**（error.rs:734 + ~226 中文串跨 40 文件；测试断言 12 文件）：生产控制流正确按 DiagnosticKind 匹配（可逆性机械成立），但逆转路径触及 ~12 测试文件 + 设计文档诊断表；非中文维护者今天无法有意义评审诊断措辞。修复：测试约定「断言 kind + 稳定消息 ID，不断言中文逐字」+ error.rs 头部注释使政策在作者现场可发现。S。
5. **LOW-1 — Flutter 表达式格式化器在错误文件**（FlutterFormatExpr 在 `flutter/ref.rs:1107`，expr.rs 委托）：内聚瑕疵，S 纯移动。
6. **LOW-2 — 豁免台账 deferral 链接非 CI 强制**（`scripts/check_exemption_ledger.sh` 存在但 verify-react.sh 与 ci.yml 均未调用）：第 5 列 deferral 链接（豁免可追溯到修复任务）只手动审计；T0.4 历史（13 个未追踪根因类）证明可腐。修复：CI 一行调用。S。

#### Recommendations
即时（S）：finish_pipeline 提取（MEDIUM-1）；check_exemption_ledger.sh 入 CI（LOW-2）；JsFormatExpr 删除或文档修正（MEDIUM-2）。短期（S-M）：FlutterFormatExpr 移动（LOW-1）；测试约定（MEDIUM-4）；omniview 边界文档（MEDIUM-3）。长期（M-L）：ADR-066 D4 触发时 TextStyle 语义路径（671→<300 唯一路线）；#43 假阳性（671 诚实故事依赖噪音低）；三 whitelist 脚本合并。

#### Risks if Unaddressed
管线漂移（MEDIUM-1）是唯一有合理错误输出终态而非仅摩擦的关切——程序级 pass 进两处三处之一，编辑器与 CLI 行为静默分歧，正是代码库已为 IR pass 修过一次的 bug 类；误导共享层（MEDIUM-2/3）使新后端作者建立死扩展或复利双世界成本；语言政策侵蚀（MEDIUM-4）使逆转路径每 sprint 更贵；台账腐（LOW-2）豁免无链接静默复现（T0.4 事故）。

---

### Implementation Strategist — 变更管理与演进就绪度

**Perspective**: 评估 10 域 v1→v2→v3 的落地（排序、爆炸半径、可逆性）与治理机制（deferrals、门禁、迁移提示）是否保持语言表面可演进而不需再大爆炸。先行结论：**变更本身以异常有纪律的排序与可逆提交结构落地；残余风险不在迁移机制而在治理层自身的信息新鲜度——台账与评审简报落后代码数天，用户报告类 deferral 触发可能永不到期。**

#### Key Observations
- 共享简报每个「已知未决」项都陈旧：ADR-062:77 与 deferrals.md:319-328 记录 #60 关闭（CI job omniview-e2e 存在，ci.yml:238-272，commit a0b11b63）、BUG-A 关闭（D-I，db68d812）、unused-import 修复（e454f907）。简报从 ADR-056 Negative 节编译而来，而该节只带头部注记（line 7）未改写正文——读者得到 08-03 事实。
- 迁移治理真实非口号：每删除语法族有硬诊断 + 迁移提示（实测 9 处）；docs-gate 用 DELETED_SYNTAX_CODES/PREFIXES 使教学已删语法 fail CI——文档块用 value={sig} 推上就红。
- Demo 迁移完成零残留：96 `.omni` 文件 grep 八删除族零命中。
- 排序正确且全程树绿：v3 8 阶段按评审自身问责顺序——门禁先（Phase 0 先测量）、注入（Phase 1 先于行为使修复不增攻击面）、行为（2/3）、文档（6）；每阶段独立可回滚。
- 语言表面现已有效锁定（锁日 08-04 而非「2 周」）：最后删除 275cc524/bc4b81be（08-04）；之后 parser-touching 提交全为 chore。

#### Strengths
1. **硬诊断优于弃用**对零外部用户语言正确（ADR-056 决策 2）：无兼容层可腐；每消费者得到响亮可行动消息。弃用期 + 兼容垫会加倍爆炸半径而无真实用户。
2. **验证网是真防回归架构非勾选框**：8 门禁含双目标 regen-then-test e2e、Flutter widget harness、单调 warnings 上限（唯一一次经批准上升有文档出处）、双向陈旧豁免强制。
3. **每次变更迁移成本有界且提交时强制**：「demo 迁移随任务；文档同步随任务同 commit」使任何单变更回滚构造上完整。
4. **deferral 行多数带可执行触发**：#63（「下次 IR 变更时评估」——金标准）、#45/#48/#52-54/#58/#74/#75/#76/#83 具体；#60/#43/#61/#69/#70 关闭可溯源真 commits。
5. **诚实目标调整**：#81 <300 → 671 诚实下限经 ADR-066 带 Pragmatic 计分（N4/C7/R1.75）+ 具体触发（Flutter 文字样式真实使用/motion 重设计）；v3 计划验收矩阵收口时更新而非留 FAIL 未裁决。

#### Concerns
1. **MEDIUM — deferral 台账自身违反自己的陈旧纪律**（`deferrals.md:189` #43 unused-import 已修 e454f907 但无关闭注记；:226/234/235 #61 TS2722/#69 ES5-var/#70 unused-identifiers 已从 exemptions 移除带 REMOVED 注记但台账无关闭标注）：08-05 关闭表只覆盖 6 个计划承诺行，漏掉同波关闭的 4 个豁免类行。未来维护者读 #43 会花精力重新诊断已修缺陷；台账失去单一事实源权威——项目反复制度化的失败模式（exemptions 台账断链曾是 v2 评审发现）。修复：按 08-05 节风格补 #43/#61/#69/#70 关闭行带 commit 证据；可选一行脚本（grep exemptions REMOVED 注记 vs 台账开行）镜像 check_exemption_ledger.sh。S。
2. **MEDIUM — 评审简报新鲜度：v2 时代事实传播进并行评审**（ADR-056 正文未改写只注记）：我烧掉 1/3 验证预算重确认已关闭事实；并行专家可能对已修项提交发现产生噪音。修复：改写（非注记）ADR-056 Negative 项为划线/移入 resolved 附录；评审启动规定「简报引用最新处置记录日期」。S。
3. **MEDIUM — 用户报告触发制造事实性永不到期 deferral**（#38/#39/#40/#41 四个浏览器实证缺陷 + 已知修复，触发「用户报告/用户使用」；#44/#51/#57/#76/#83 用户驱动）：96 demo + poiesis 为唯一已知消费者时「用户报告」可能永不发生。诚实 YAGNI 姿态（LOW/MEDIUM、诊断已响亮降级）但触发器无时钟：无定期扫。开单随时间静默累积「bug 形」行与「故意设计限制」不可区分。修复：轻量定期重审（每 ADR 修复轮扫用户驱动触发行；BUG-* 触发器改挂同发射路径任务）。S。
4. **LOW — #42 触发软**（「任一分歧致用户困惑」不可证伪）；#74/#75/#77「只登记不修」。
5. **LOW — 删除波对外部语料影响设计上未测量**：硬诊断政策对已知语料正确，但仓库外任何 v1 时代 .omni 文件升级即破且无工具枚举/修复表面。成为第一天语言有外部作者时的真实采用成本。修复：QUICKSTART 一行声明「无向后兼容、仅迁移提示」契约。XS。

#### Recommendations
即时（S）：补台账关闭行 #43/#61/#69/#70；改写 ADR-056 Negative 项（或移 resolved 附录）+ 简报引用处置日期；重写 #42 软触发。短期（S-M）：dormant 旗标约定（用户报告触发行标「dormant——下次修复轮重评」，BUG-D/E/F/2 触发器挂发射路径任务）；考虑 verify-deferrals.py 镜像 check_exemption_ledger.sh（最高杠杆治理卫生守卫）。长期（M）：新维护者入职包一页（ADR-054→056→062 + lifecycle spec + 8 门禁图 + 台账约定 + 中文政策可逆与记录位置）；若外部用户出现，重访 no-compat 立场（v1→v2 迁移工具）。

#### Risks if Unaddressed
台账权威衰减（#43 式修复后仍开行教未来维护者不信任台账——豁免台账重连工作的另一半成为不可信方）；陈旧简报级联（每轮评审从 ADR-056 时代文本重推导事实，烧编排预算）；已知缺陷累积（BUG-D/E/F/2 挂在可能永不触发的事件上——「静默退化」类被 deferral 系统反讽地保存）。

---

### AI Engineer — AI/Agent 工具化基座

**Perspective**: 把编译器当 LLM/agent 工作流构建块评估：语义 DSL 作为 AI 代码生成目标、诊断网作为 AI 反馈通道、WASM 增量管线作为交互基座。**当前代码库零 LLM 集成**（P34 AI 补全已删），本评审评估基座就绪度与 AI 工具首触接缝。

#### Key Observations
- 语义 DSL 是真强 AI 生成目标：意图级语法使 LLM 生成 .omni 永不需要 React/Flutter 惯用知识；00-cheatsheet 显式「面向 LLM few-shot」10 域最小可编译示例全 docs-gate 校验；「floor 外硬诊断」正是 AI 迭代环需要的确定性反馈。
- 诊断信封在边界已 LLM 消费级：DiagnosticJson 统一 9 字段（小写严重级、显式 null、稳定类型化码、per-diagnostic suggestion、文件/行/列/结束位置），FFI/WASM/Tauri 三路径契约测试强制。
- WASM 增量管线交互工具就绪：analyse_multi_cached 内容哈希跳过 + 传递依赖失效，回归测试保护「poiesis 每键」路径；OmniCompiler 有状态 WASM 类逐会话缓存。
- **但编辑器诊断热路径绕过廉价表面**：多文件 Monaco 路径跑全量 codegen 取诊断（`poiesis/src/services/monaco/diagnostics.ts:81-89` compile_to_omniview；:144-175 validateSources）——WASM 层导出的诊断专用 analyse_multi_cached 从未被前端用于此目的；且丢 lint warnings（runDiagnostics 只读 errors）。
- 结构化 suggestion 字段填充不一致：部分验证器放修复进结构化字段，部分内联进消息。

#### Strengths
1. **删除纪律使语言表面对 AI 自文档化**：每已删形式硬错误带迁移路径——LLM 生成陈旧语法从诊断本身得到当前表面。
2. **文档不可能漂移出编译器**——doc 语料是可信 few-shot 素材（罕见且对 prompt 工程高价值）。
3. **最近邻建议在正确位置**：未知事件名建议最近合法拼写、未定义名编辑距离建议、effect deps 点名缺失 store 字段。
4. **语义全集治理回答 AI 安全问「什么支持？」**——每语义集条目带实现状态列与确切诊断行为，agent 可被告知「只生成支持条目；未支持硬错误」自机器可验证表。
5. **warnings 三网关（CLI/单文件/编辑器）闭合**——WASM diagnostics() 显式折叠 warnings 进编辑器 marker 流。

#### Concerns
1. **MEDIUM-1 — 结构化 suggestion 字段填充不一致——修复提示埋进消息文本**（好模式：validate_ref_ops.rs:139-153 迁移提示在 suggestion、消息可读；坏模式：validate_event_names.rs:92-95 修复内联 + suggestion:None、validate_a11y.rs:196、validate_array_methods.rs:98、lint_to_diag 硬编码 None）：suggestion 是 agent 可消费的「应用此修复」信号；今天 AI 工具必须从自由文本中文消息解析修复提示（对中文模型可用，对其他脆弱，消息改写即破）。修复：政策 + 一测试「携带修复提示的诊断必须填 suggestion；消息只留描述」；扫 ~4 验证器 + lint_to_diag。S。
2. **MEDIUM-2 — 多文件诊断路径跑全量 codegen——AI 迭代环每次验证付 codegen 代价**（diagnostics.ts:81-89/:152；CodeEditorModal.tsx:97 500ms debounce；useSaveProject.ts:237）：且丢 warnings（多文件路径不折叠 warnings，markers 缺失）。修复：切到导出的 analyse_multi_cached + 合并 errors+warnings（诊断专用路径已存在未用）。S。
3. **MEDIUM-3 — analyse_dependencies 在缓存管线之上重跑语义分析**（omnilang-wasm/src/lib.rs:735-776 调 analyse_multi_cached 后每文件再 semantic::analyze 提取符号/依赖；CodeBlockEditor.tsx:138 每次 debounced 编辑）：热路径付两次分析，成本随文件数线性（未变文件也重分析）。修复：缓存管线返回符号/依赖，WASM 变薄适配器。M。
4. **LOW-4 — Monaco markers 永不因代码变干净而清除**（CodeBlockEditor.tsx:129-133 applyDiagnostics 仅 diags.length>0 时调用）：AI 修好错误后陈旧红色 marker 持续——反馈环缺陷。修复：空数组也调用（清 owner markers）。S。
5. **LOW-5 — 中文诊断对异构 AI 工具链脆弱**：对中文优先模型是优势；英文提示 agent 需消息内嵌 hint（MEDIUM-1 强制）变脆弱；稳定英文 code 字段是唯一语言中立锚。无行动；保持 codes 稳定。
6. **LOW-6 — 缓存序列化往返刻意冷**：沙盒 iframe/每任务新页面 AI 上下文冷缓存首验付全量分析——在 AI 工具契约中记为预算假设。

#### Recommendations
即时（S）：suggestion 政策标准化 + 一契约测试（AI 消费最高杠杆单变更）；多文件编辑器路径切 analyse_multi_cached + 合并 warnings；空结果清 markers。短期（M）：消除 analyse_dependencies 双分析；从现事实源生成机器可读语言/诊断注册表（error codes + canonical 消息 + suggestion 例 + universe 状态列 → 单 JSON，脚本 + 门禁）——AI 工具的结构化适配器，运行时零成本。长期（L）：在基座上重开 AI 代码生成计划（P34 后继）：delta「更新单文件」WASM API（现每次调用发全量 sources map）、每次验证延迟预算（缓存命中 vs 冷）、结构化代码动作（span + replacement 作第 11 信封字段）。

#### Risks if Unaddressed
首个 AI 集成在时间压力下改装 suggestion 政策（~15 验证器 + lint + 测试 + docs 断言散落，消息文本嵌入测试与文档——现在 S 扫除，之后昂贵数倍）；迭代速度封顶 agent 环（codegen-in-diagnostics + 双分析使每迭代延迟随项目线性增长，削弱优秀增量缓存设计）；语言赌注（若 AI 工具被迫建在消息文本解析上，中文政策从「可逆」变事实 API）；陈旧反馈破坏信任（LOW-4 使 agent 的「我修好了」验证显示错误绿/红）。

---

### Documentation Expert — 文档体系

**Perspective**: 文档体系的**强制力设计（门禁）世界级，入口与生命周期管理（索引页、废弃、迁移）存在结构性缺口**：编译器每个字节被 247 个代码块钉死，而 docs/README.md（所有新用户第一页）与 docs/stdlib.md（幽灵标准库文档）不在任何门禁覆盖内，且已在三轮评审被标记后依然存活。**文档与编译器的一致性由门禁保证；文档与用户之间的导航一致性没有保证者。**

#### Key Observations
- docs-gate 承诺真实且超基线：实测 247 omni blocks 0 failed、173 semantic notes；complete/fragment 双轨、删除语法族拦截、仅 1 条显式豁免——豁免纪律干净。
- cheatsheet「可直接复制为独立文件编译」承诺逐字成立：10 域示例全 complete 块；删除语法表 verbatim 诊断消息与编译器源码逐字节匹配。
- 四层文档分工基本清晰；唯一模糊带：**被 Superseded 的 BNF spec 仍被门禁检查且含「修正已证伪断言」的补丁——僵尸文档靠打补丁续命**。
- **新用户旅程断裂点不在内容层，在入口层**：docs/README.md 同时声称 13 章（实际 16 文件）、指向不存在的示例项目、把语法规范和迁移指南标「待补充」（语法规范其实存在）、列 10 份 ADR（实际 65 份）、推荐错误路径的标准库文档。
- 破坏性变更通信走「诊断即文档」路线（每条删除语法 verbatim 硬诊断 + 迁移提示）；但**无端到端迁移指南**（README 自己承认「待补充」）。

#### Strengths
1. **删除语法治理 = 行业级实践**：cheatsheet §3 16 项删除语法 × verbatim 诊断 × 替代方案，门禁以「前缀族」判定文档是否教授已删语法；LLM error corpus 钉死诊断可行动性。
2. **词汇登记册 + 语义全集双机制**：各由独立脚本双向断言——「文档-规范-编译器三方一致」的最强工程化表达。
3. **语法参考（14/15）质量高**：混合 BNF + 消歧散文 + 代码例、token 表、完整 AST 类型、删除项均带 ADR 引用、附录 C 完整示例经门禁 complete 校验。
4. **教学路径结构良好**：cheatsheet → 01 入门 → 02-13 逐域 → 14/15 工具开发者；README 导航表把「面向谁」显式分层。
5. **用户文档内嵌迁移提示**：05/06/11 各章对删除语法持续有「已删除 → 替代」注记。

#### Concerns
1. **HIGH-1 — 文档入口 docs/README.md 显著陈旧且不在门禁覆盖内**（`:11` 13 章实为 16 文件、`:35` 错误标准库路径、`:36-37` 语法规范标「待补充」实际存在、`:43` 10 份 ADR 实为 65、`:51` 指向 5 个不存在示例项目）：这是仓库 docs 第一页；verify-docs.py:302-305 只扫 docs/omniview + 3 spec——入口页每处陈旧不被门禁捕获。新用户按入口走全部踩空。修复：重写 README（S）+ 纳入轻量链接/断言（断言引用相对路径存在 + 章节数匹配）。S。
2. **HIGH-2 — docs/stdlib.md 是僵尸标准库文档**（`:5-46` 教授 `omni/math`，编译器事实 `stdlib/mod.rs:3` 是 `std/<module>`；全仓 grep 零 `omni/` 前缀——复制即编译失败）：已活过三轮评审标记（07-27/07-28/07-31）仍未处置；不在 docs-gate 覆盖；与 12-standard-library.md（正确 `std/*` 中文）双文档竞争且入口指向错的。修复：弃用横幅 + 修正/删除 + DESIGN_DOCS_INDEX.md:292 引用修正 + 纳入门禁或移除引用。S。
3. **MEDIUM-3 — 无端到端迁移指南（v1→v2 破坏性变更用户通信缺口）**（README:37 承认「待补充」）：删除信息已齐备但无「我是 v1 用户，我的代码哪里会报错、怎么改」单文档；硬诊断兜底但每次失败都是用户成本。修复：以 cheatsheet §3 为骨架 + v1→v2 对照 + 诊断码索引生成 migration-v1-to-v2.md 接入导航纳入门禁。M。
4. **MEDIUM-4 — 173 条语义 note 无预算机制**（06-templates 31/05-components 25/15-grammar 38/13-project-structure 13；verify-docs.py:404 打印无阈值）：门禁对 fragment 语义错误降级 note 正确（包装上下文假阳性），但用户无法区分可复制块与片段——「note 不失败」静默累积。修复：per-file note 预算（如 ≤5）超限失败 + per-file 汇总输出。M。
5. **LOW-5 — 语法表面小漂移**（README:43「std/* 全部 7 个模块」实为 11 注册/8 覆盖；12-std:18-20 尾置 async 在 14-grammar BNF 未收录，实测两位置均解析）。
6. **LOW-6 — 文档双语政策未成文**（14/15 英文工具开发者 vs 16 篇中文用户 vs BNF 中文）：新贡献者无法判断写作语言。修复：docs/README 加「文档语言政策」一节。S。

#### Recommendations
即时（S，1-2 天）：重写 docs/README.md；stdlib.md 弃用横幅 + 修正/删除；修模块计数 + async 位置注记。短期（M，1 周）：migration-v1-to-v2.md；note 预算；语言政策成文。长期（L）：「跨目标行为」用户章节（semantic-universe 状态列翻译成用户语言——用户「语法→地道代码」关切期望管理）；docs-gate 索引完整性断言。

#### Risks if Unaddressed
首印象伤害：新用户/LLM 从入口出发得到不存在示例项目/错误 stdlib 路径/缺失语法规范指针——「文档 = 可执行承诺」在入口即失效且门禁全绿（不覆盖入口）；幽灵文档永久化（stdlib.md 活过三轮标记，同类僵尸会继续在门禁盲区繁殖）；迁移知识蒸发（v1 教程/LLM few-shot 重造已删语法）；note 静默膨胀（可复制性逐年下降）。

---

### Pragmatic Enforcer — YAGNI / 复杂度守卫

**Perspective**: 以实际消费语料（learn-omniview 192 个 `.omni` 文件）与每个门禁脚本的地面真相溯源为证据计数——v2 重构**整体未过度设计**：demo 语料证明 19/19 motion 属性、10/10 RefOps、5/5 a11y 原语真实使用，证据率优于多数生产语言。过度建设在边缘：一个零消费者语法表面迫使 React slice codegen 特例、治理元层双脚本重复提取机制且把地面真相钉在 **Superseded** 文档、生命周期门禁强制纸面痕迹而非其声称保护的用法结果。

#### Key Observations
- 10 域语料验证非投机：motion 白名单 19/19 用（borderRadius 2 用、8 个 skew/rotate/translate 变体全用）、RefOp 10/10、a11y 5/5、bind:value 32 用（确认 D2 收敛正确）、@themes 4 个真实主题。
- 三个表面超前于需要：`velocity_max`（可选拖拽参数）**零 demo 消费者**却跨 parser AST/指令/IR lowering/IR optimize/三后端全链路实现，**外加 React slice 订阅特例排除**（store_slices.rs:859,861,945「deliberately NOT collected」）；`count()` 数组方法零 demo 消费者；18/54 canonical 事件未用。
- 验证网 ROI 可证明——除文档钉层：855 元组语料（1254 行 injection.rs）最终回归捕获 2 个真实 Flutter 注入缺口；docs-gate 捕获 D1 spec 路径漂移；浏览器套件捕获 7 bug 含 2 P0。但 verify-vocabulary.py（308 行）与 verify-semantic-universe.py（348 行）**5 个辅助函数逐字重复（~120 行）**且都钉 BNF 地面真相到 **Superseded** v1 spec（头部 1-7 行自声明），而声明「唯一权威」的 v2 设计文档完全无机器钉。
- BUG-A 经济正确处理、标题未：75 点 LetStmtIR.mut 修复是地道的正确修复非可避免债；廉价 let 临时方案迟一阶段到达使旗舰 demo 两个标签页卸载约一天，诚实 fixme 标记。可避免成本是装饰性：「41/1/0 零 runtime error」头条里的「1」是应用卸载 bug。
- 治理层在 YAGNI 临界点：4 门禁脚本 + 2 登记册（327+351 行）+ 豁免台账（156 行）+ floor_matrix（590 行）+ lifecycle 文档（129 行）全为强制「文档匹配编译器白名单」；每个都有真实失败故事，但下一个提议门禁应要求合并前评审。

#### Strengths
1. **Demo 语料优先设计纪律**：19/19 motion 属性与 10/10 RefOps 真实使用是最强 YAGNI 证据；v1→v2 删除（SetValue/value={sig}/auto-bind/paused/typed slots）全删真实冗余（一意图三表面 → 一）。
2. **注入语料是代码库最高 ROI 验证投资**：855 元组 × 模式缺失断言（非仅编译）两次捕获供应链 RCE 类。1254 行成本，价值 5 个 CRITICAL 注入闭环。勿动。
3. **floor_matrix.rs（590 行）是「对等无法证伪」的正确答案**：手写 floor 宣称 → 每项编译级测试映射（25 项）机器验证。
4. **诚实降级 + deferral 制度有效**：#81（671 warnings，R1.75 deferral）、#83（motion-in-@for）、#28/#33 全正确触发；4 主题 demo 本身在 Flutter 上触发 >2 主题警告而非静默折叠。
5. **@effect deps 验证（686 行 2 demo 消费者）是正确的「语义纯度税」**：把静默 bug 类（漏 deps → 永不触发）变编译错误；验证了替代（从 body 推导 deps）更复杂难以健全。

#### Concerns
1. **MEDIUM-1 — velocity_max：零消费者全表面实现 + codegen 特例**（BNF:287；设计:191；lower/view.rs:918-944；optimize.rs:150-157；React 排除 store_slices.rs:859-945；Flutter motion_widgets.rs；OmniView motion_triggers.rs）：三个过早泛化标准全中（0 消费者；从 v1 设计主动恢复非从需要；可测复杂度 11+ 触点）。React「deliberately NOT collected」slice 规则是潜伏陷阱：下一个动 store_slices.rs 的人会问为何一个拖拽参数被排除，移除排除即发射未用局部。修复：从 BNF/parser/IR/optimize/三后端移除 + deferral 注册触发「demo 或用户需要 fling 速度物理」；git 历史保留完整实现；syntax-lifecycle 门禁是自然重入路径。S-M。
2. **MEDIUM-2 — 双门禁脚本重复提取机制且钉 Superseded 文档**（verify-vocabulary.py vs verify-semantic-universe.py 5 函数逐字重复；都硬编码 BNF_SPEC_FILE = Superseded v1 spec）：门禁把「历史基线」文档当机器检查事实源而「唯一权威」v2 无强制——分层颠倒；白名单变更（第 20 个可动画属性）要碰 **5 文件**（v1 spec + v2 设计 + 登记册 + 全集 + 编译器）；任何人删除 Superseded v1 spec（其头部自邀）两 CI 门禁以费解提取失败崩。修复：(a) 合并两脚本或提取共享 spec_whitelists.py（省 ~120 行）；(b) BNF 地面真相重指向 parser 代码（事件/RefOp 已如此——event_names.rs、RefOpIR 枚举）；semantic_anim_prop/token_type production 有 parser 常量可提取；(c) v1 spec 头加「machine-pinned——勿删」。S。
3. **LOW-1 — count() 零 demo 消费者**（validate_array_methods.rs 1122 行；设计:666；ADR-062 D-E prove）：D-E prove 轮迁移 filter/map 调用点但 count 以封闭集成员过 prove 未达 lifecycle 自身 ≥1-demo 标准。成本小（方法表行 + 测试）——**保留**但记录 grandfather；deprecated_array_stdlib 迁移提示（526 行 lint）当前 demo 语料零活触发。
4. **LOW-2 — syntax-lifecycle 门禁强制纸面非用法**（verify-syntax-lifecycle.py:79 FILE_TOKEN_RE 只断言 pilot 单元格引用存在文件，无法断言 ≥1 demo 消费者）：count() 零 demo 使用通过门禁。边际过程剧场——作为纸面诚实可接受，但营销应说「文档诚实」非「用法强制」。
5. **LOW-3 — 中文诊断表面大**（570 唯一中文串 semantic/*.rs + 293 codegen；与英文诊断混汤——validate_motion.rs 英文而 validate_ref_ops.rs 中文）：863+ 串英文优先工具切换成本 800+ 重写。非当前问题；切换成本记台账政策即可。

**明确清白的项**（验证过）：855 元组注入语料（2 真实 Flutter 缺口——仓库最强网）；docs-gate（D1 证明价值、omnic analyse 廉价）；豁免台账 29 条计数上限（新实例失败、构造诚实）；BUG-A 双步 D-I 处置（勿重审）；#81 deferral（<300 路径 TextStyle 无真实用户，触发正确）；18 个未用 canonical 事件（保留封闭集——边际成本表行——但停止为未用事件建 per-event 字段提取桩——那是事件域成本集中处）。

#### Pragmatic Enforcer Analysis（per adr-template）
**Proposed Decision**: 「10 域语义 DSL v2 重构 + 验证网 + 双登记册治理如实现」
- **Necessity: 7/10** — 13 专家评审驱动（不可编译 React 输出、活注入 RCE、系统性门禁盲区）；每域有 demo 消费者；v1→v2 证据链（0.63→0.69）成立。
- **Complexity: 6/10** — 17,421 行语义层（24 文件、15 validate 模块、15 lints）+ 4 CI 门禁脚本 + 2 登记册 + floor 矩阵 + 豁免台账——语言唯一生产语料是 92 文件 demo 项目。每域成本有据；元层是比值退化处。
- **Ratio: 0.86**（目标 <1.5）— 总体适当，但比先前 0.63/0.69 上升。增量几乎全部是 MEDIUM-1（velocity_max）与 MEDIUM-2（重复门禁机制）。
- **Recommendation**: ⚠️ **Approve with simplifications** — 保留域与网；移除/推迟 velocity_max；合并门禁层。

#### Recommendations
即时：移除 velocity_max（BNF/IR/三后端/React slice 排除）+ deferral 注册（S-M ~11 文件机械，store_slices.rs 排除移除是奖励）；两 verify 脚本 BNF 地面真相重指向 parser 常量（S）+ v1 spec 头 machine-pinned 注记。短期：合并两脚本共享提取（省 ~120 行，一事实源）；停止未用事件字段提取桩（按需）；一页门禁清单（每脚本防什么失败 + 证明 commit——下一门禁须答「先合并？」）。长期：第 6 白名单或第 3 登记册提议时治理层合并为单一语言治理模块（L 触发门控非现在）；中文政策仅外部贡献者加入时重访，863 串切换成本记台账。

#### Risks if Unaddressed
MEDIUM-1 不动：store_slices.rs 排除是站立陷阱，编译器拥有零训练的 fling 物理语义——首个真实消费者撞未测试行为；MEDIUM-2 不动：每白名单变更 5 触点两门禁重复逻辑，登记册与全集漂移产生费解双面门禁失败，Superseded 作事实源是未来清理会打破的不可见耦合；治理面增长：每门禁提仪式-信号比，lifecycle 已偏过程剧场——4 脚本 + 台账 + floor 矩阵后，下一机制应是合并非添加。

---

## Collaborative Discussion

### Cross-Cutting Themes（3+ 成员独立报告的高杠杆发现）

**主题 1 —「宣称 vs 事实」漂移：防漂移机制本身在漂移（6 位成员）**
Domain（hover/toggle/theme="dark"/bind:checked 三 HIGH）、Compiler（longpress 语义全集「部分」行无诊断、MEDIUM-1 universe 25/54 行过度宣称、HIGH-4 设计契约被 parser 遮蔽）、Systems（MEDIUM-3 治理锚点钉文档非代码）、Pragmatic（MEDIUM-2 verify 脚本钉 Superseded 文档）、Implementation Strategist（ADR-056 正文陈旧未改写、台账 #43/#61/#69/#70 缺关闭行）、Documentation（README/stdlib 入口陈旧）、Security（LOW-4 ADR-056 语料计数陈旧）。**共识：verify 脚本只做结构断言（白名单 ⊆ 语义集、状态词汇表），不做事实断言（状态列 ≠ codegen 实际交付）——「语义全集/设计文档/README」三类权威文件都在机器盲区。** 修复模式统一：重指向 parser 常量 + 事实级交叉检查 + 权威文件入 docs-gate。

**主题 2 —「exit 0 但输出损坏/静默分歧」类：门禁 fixture 的组合盲区（5 位成员）**
Compiler HIGH-1（ref-op×数组方法 walker 缺口→非编译 Dart）、Compiler HIGH-3/Domain HIGH-1（longpress/hover/toggle web 死处理器零诊断）、TS/React H1（陈旧闭包浏览器套件结构性不可见）、Perf HIGH（keyFn 塌缩 lint 建议恰好触发）、Security（语料静态-only，动态面无覆盖）。**共识：所有 HIGH 都是单域 fixture 矩阵覆盖不到的组合形态——门禁只捕获 fixture 行使的路径。** 修复模式：组合 fixture 矩阵 + 浏览器行为断言 + 语料动态绑定模式。

**主题 3 — 验证网结构性盲区（4 位成员）**
TS/React（浏览器套件断言重渲染后显示，不断言顺序语句→H1 不可见）、Security（运行时消毒器从不执行、动态面无断言）、Performance（基准不覆盖热路径）、Compiler（fixture 矩阵单域）。**共识：验证网在「运行时行为/动态数据/性能」三维有系统盲区——与静态编译绿不矛盾。**

**主题 4 — 台账/豁免信任（4 位成员）**
TS/React（87 活错误台账绿 + H2 崩溃被 TS2304 掩盖）、Implementation Strategist（#43/#61/#69/#70 关闭缺台账行）、Maintainability（check_exemption_ledger.sh 不在 CI）、Systems（evals continue-on-error）。**共识：台账机制本身强（计数上限/双向陈旧强制），但台账与 CI 的最后一环与关闭纪律有洞——「豁免债 = 运行时 bug 类」至少有 1 个活实例（H2）。**

**主题 5 — 跨目标行为分歧（5 位成员）**
TS/React H1（React 陈旧闭包 vs OmniView 新值）、Domain HIGH-1（hover/toggle Flutter 工作 web 死）、Compiler HIGH-3（longpress）、Perf HIGH（keyFn 塌缩仅 OmniView）、Systems HIGH-1（Vue/Dioxos 全族静默丢）。**共识：同源码跨目标行为不等价是当前最大「地道性」违约——ADR-054 原则 1 的行为层。**

**主题 6 — 治理层维护成本/复杂度（3 位成员）**
Pragmatic（velocity_max 全链路 + slice 特例；双脚本重复 120 行）、Maintainability（finish_pipeline 三处复制；JsFormatExpr 死代码；omniview 双世界）、Systems（token 字符串往返）。**共识：治理元层进入维护成本现实区——下一步是合并/简化而非添加。**

### Trade-offs and Disagreements

**主题：Vue/Dioxus 的处置**
- **Systems Architect**：必须立即给 Vue/Dioxus 加 floor 诊断（E_CODEGEN 或警告通道）——ADR-052 契约已违约，「静默降级归零」宣称过度。
- **Pragmatic Enforcer**：不应投资非 P0 目标；当前半状态（4 特性族静默丢、design 硬错误）是真实用户面最小成本。
- **调和**：两方实际收敛于同一廉价修复——每特性族一条 E_CODEGEN 诊断（S 工作量、镜像 DESIGN_BLOCK_ERROR）既是 Systems 的契约遵守也是 Pragmatic 的最小投资；分歧仅在「全支持 vs floor 诊断」，裁决取 floor 诊断 + ADR-052 floor 表显式化（Systems 长期建议 8 采纳）。

**主题：豁免台账语义**
- **TS/React**：「GATE PASSED」应被理解为台账绿（87 活错误），不是 strict 绿；豁免债有运行时 bug 实例（H2）。
- **系统自身（ADR-056）**：宣称「React strict tsc 0 错误」。
- **调和**：不是成员间分歧，是 ADR 宣称 vs 台账现实的分歧——裁决：修订 ADR 措辞为「台账绿（计数上限内）」；台账机制本身被所有成员评为模范（不修机制，修表述）。

**主题：中文诊断政策**
- **Pragmatic（LOW-3）**：863+ 串双语混汤，切换成本 800+ 重写。
- **AI Engineer（LOW-5）**：对中文优先模型是优势，英文模型脆弱——保持 codes 稳定。
- **Maintainability（MEDIUM-4）**：生产控制流按 kind 匹配（可逆性机械成立）；测试逐字断言中文是逆转路径主要成本。
- **Documentation（LOW-6）**：双语政策未成文。
- **共识**：四成员一致——**保留政策**（用户群体中文优先、ADR-056 决策 1），加约定（测试断言 kind + 稳定 ID 不断言逐字中文、政策在 error.rs 与文档成文、切换成本记台账）；无人主张现在逆转。

**主题：velocity_max 处置**
- **Pragmatic**：移除（零消费者 + 特例陷阱）。
- **Compiler/其余成员**：无反对意见（无人使用或为其辩护）。
- **裁决**：采纳移除 + deferral 触发注册；git 历史保留。

**主题：count() 处置**
- **Pragmatic**：保留但 grandfather 记录（成本小、封闭集成员、D-E prove 已裁决）。
- **其余**：无反对。
- **裁决**：保留 + grandfather 注记；deprecated_array_stdlib 迁移提示零活触发保持登记。

### Priorities Established

**Critical（0-2 周）**:
1. 注入防线补漏（Security HIGH-1 React 动态 URL XSS + Rust HIGH Flutter `$` 转义回归）——2 个独立可构造注入面，S 工作量。
2. React 行为层三缺陷（TS/React H1 陈旧闭包、H2 悬空 setter 崩溃、H3 数字 bind:value）——「地道代码」行为层失信 + 1 个用户可见崩溃，S-M。
3. 「exit 0 输出损坏」类（Compiler HIGH-1 walker 下钻、HIGH-2 scrollTo 双轴警告、HIGH-3 longpress 警告 + Domain HIGH-1 hover/toggle 警告）——同族：目标映射诚实性，全 S。
4. 「宣称 vs 事实」台账校准（Domain HIGH-2 theme="dark"、HIGH-3 bind:checked、Compiler HIGH-4 value={sig}、semantic-universe 25+ 行重分类、docs/README + stdlib.md）——治理权威恢复，S-M。
5. 跨目标静默降级（Systems HIGH-1 Vue/Dioxus floor 诊断 + Perf HIGH keyFn 塌缩）——契约遵守 + 列表静默损坏，S。

**Important（2-8 周）**:
- verify 脚本事实断言升级 + 锚点重指向 parser 常量（Pragmatic MEDIUM-2 / Systems MEDIUM-3 / Domain MEDIUM-2 三成员同见）。
- 运行时消毒器执行验证 + 语料动态绑定模式（Security MEDIUM-2）。
- 浏览器行为断言补 H1 类（TS/React 建议 5）+ 组合 fixture 矩阵（Compiler 建议 9）。
- 台账关闭纪律（#43/#61/#69/#70 补行 + check_exemption_ledger.sh 入 CI + ADR-056 正文改写 + verify-deferrals.py）。
- suggestion 政策标准化（AI Engineer MEDIUM-1）+ 编辑器诊断路径切 analyse_multi_cached（MEDIUM-2/3）。
- 管线编排收敛（Maintainability MEDIUM-1 finish_pipeline + Rust MEDIUM run_ir_passes 纯化——同一病灶两面）+ JsFormatExpr 处置（MEDIUM-2）。
- 未声明 token → error + token IR 类型化（Systems MEDIUM-1/2）。
- a11y 动态名称缺口记录（Domain MEDIUM-1）。
- 增量 codegen / lint 门控 changed+dependents（Performance MEDIUM-4 + AI Engineer 关注同路径）。
- 类型发射删豁免类（TS/React M1/M6——54/87 豁免）。
- migration-v1-to-v2.md + docs note 预算（Documentation MEDIUM-3/4）。

**Nice-to-Have（2-6 个月）**:
- velocity_max 移除（Pragmatic MEDIUM-1）；count() grandfather 注记。
- useDrag hook 提取（TS/React M2）；getters selector 推导（M4）。
- 基准套件热路径覆盖（Performance LOW）。
- parity 基线扩展含文档化分歧负断言（Systems LOW-2）。
- 「跨目标行为」用户章节（Documentation 长期 7）。
- 语言/诊断注册表 JSON 生成（AI Engineer 建议 4）。
- AI 代码生成计划重开（P34 后继，L）。
- 治理层合并单模块（Pragmatic 长期 6，触发门控）。
- IR 优化 passes 扩展（Performance 长期）。

---

## Consolidated Findings

### Strengths

1. **架构层真实兑现设计意图**：10 域语义语法结构健全（BNF v2 主轴无歧义）、IR 语义纯净无平台泄漏（RefOpIR/MotionIR/ElementIR.a11y/DesignIR.theme_signal 全 `#[non_exhaustive]` 强制后端编译断链）、「floor 外硬诊断」12 项删除语法实测全部硬错误 + 迁移提示——语义纯度原则在可见表面成立。
2. **验证网从「假门禁」变为真实执行系统**：docs-gate 实测 247 块 0 失败（超原 232 基线）、注入语料 855 元组捕获 2 个真实 Flutter 缺口、React/Flutter 门禁跑真实工具链、浏览器套件已入 CI（#60 关闭）、每门禁捕获过不同缺陷类（互补性实证）。
3. **React 结构层地道**：slice selector 全覆盖、bind:value 与手写基线逐字匹配、keyed @for 单次求值、React.memo/useCallback 卫生、exhaustive-deps 91→1 真实、豁免台账是模范工程（计数上限/双向陈旧强制/deferral 链接）。
4. **Flutter 修复区地道**：Riverpod 3.0 ref.watch 合规、ThemeModeNotifier + MaterialApp(themeMode:)、BoxDecoration 工厂、快照守卫语义绑定（焦点偷窃类修复）、`[scroll]` 惰性列表 + iterable 提升。
5. **主题域语义模型完整自洽**：@themes→@default-theme→@theme 三件套跨文件验证 + D-F 双声明一致性（防启动主题分叉）。
6. **注入消毒器单源 + 语料纪律**：三副本漂移已闭、escape 顺序逐后端验证、tokenizer 状态感知语料 + 控制编译防空洞、类名表面全后端统一 E0006 门。
7. **变更治理有纪律**：硬诊断优于弃用（零用户语言正确）、迁移随任务同 commit（回滚构造完整）、deferral 多数带可执行触发、warnings 单调上限诚实（ADR-066 671 下限）、v3 8 阶段排序正确。

### Areas for Improvement

1. **「宣称 vs 事实」一致性**（HIGH）：
   - **Current state**: semantic-universe/设计文档/README 含 6+ 处与编译器事实矛盾的宣称（hover/toggle 全目标支持、theme="dark" 自然报错、bind:value 唯一、25/54 事件行、README 13 章/不存在示例/错误 stdlib）。
   - **Desired state**: 权威文件与编译器事实一致；verify 脚本做事实断言；所有权威文件入门禁覆盖。
   - **Gap**: 门禁只做结构断言；锚点钉 Superseded 文档；README/stdlib 不在覆盖。
   - **Priority**: High（3 HIGH 发现直接源于此）。
2. **行为层语义偏差**（HIGH）：
   - **Current state**: React 事件处理器内信号写后读陈旧闭包（5 实例）；只写信号悬空 setter 崩溃（H2）；数字 bind:value 类型谎言（H3）；OmniView keyFn 对象塌缩。
   - **Desired state**: 同源码跨目标行为等价（ADR-054 原则 1）；生成代码类型真实。
   - **Gap**: 浏览器套件结构性不可见（断言显示不断言顺序语句）；lint 建议恰好触发塌缩。
   - **Priority**: High。
3. **运行时/动态注入面**（HIGH）：
   - **Current state**: React/Vue/Dioxus 动态 URL 无运行时消毒（XSS 可构造）；Flutter 两文本路径绕过共享转义（$ 注入回归）；运行时消毒器从不执行。
   - **Desired state**: 动态面与静态面同等封闭；运行时 helper 行为验证。
   - **Gap**: 语料按构造静态；发射面地图只注册静态点；OmniView 有 `__omni_sanitizeUrl` 而生产后端没有。
   - **Priority**: High。
4. **非 P0 目标契约**（HIGH）：
   - **Current state**: Vue/Dioxus 对 4 个语义族静默丢弃（ADR-052 契约违约）；design 却硬错误——半状态。
   - **Desired state**: 每特性族 floor 诊断或显式目标分级。
   - **Gap**: 无门禁能发现（feature-matrix CI 只编译）。
   - **Priority**: High。
5. **豁免/台账最终链路**（MEDIUM-HIGH）：
   - **Current state**: 87 个 React 活错误（44 隐式 any）全豁免；H2 崩溃被 TS2304 掩盖；check_exemption_ledger.sh 不在 CI；4 个已关 deferral 缺台账行。
   - **Desired state**: 豁免债最小化（类型发射删 54/87）；台账与 CI 闭环；「豁免=运行时 bug」零实例。
   - **Gap**: 台账机制强但表述（「GATE PASSED」）与最后一环有洞。
   - **Priority**: Medium-High。
6. **验证网行为/动态/性能三维盲区**（MEDIUM）：
   - **Current state**: 浏览器套件断言行径显示；语料静态-only；基准不覆盖热路径；fixture 单域。
   - **Desired state**: 组合 fixture + 行为断言 + 动态绑定语料 + 热路径基准。
   - **Gap**: 全部 HIGH 都是组合/动态/顺序形态。
   - **Priority**: Medium（但建议 2/3/5 的修复依赖它）。
7. **治理元层维护成本**（MEDIUM）：
   - **Current state**: finish_pipeline 三处复制、双 verify 脚本重复 120 行、JsFormatExpr 死代码误导、omniview 双世界未文档化、velocity_max 特例陷阱、token 字符串往返。
   - **Desired state**: 单点编排、共享提取、文档化边界、死代码删除。
   - **Gap**: 治理层在 YAGNI 临界点（Ratio 0.86，增量来自元层）。
   - **Priority**: Medium。
8. **文档入口与迁移**（MEDIUM-HIGH）：
   - **Current state**: docs/README.md 入口陈旧（13 章/不存在示例/待补充标注/10 ADR）；docs/stdlib.md 幽灵文档教授 `omni/*`；无迁移指南；173 notes 无预算。
   - **Desired state**: 入口真实、stdlib 废弃、迁移指南存在、note 预算。
   - **Gap**: 入口不在门禁覆盖——「文档=可执行承诺」在首页失效。
   - **Priority**: Medium-High（High 由 2 个 HIGH 发现构成）。
9. **AI 工具化接缝**（MEDIUM）：
   - **Current state**: suggestion 字段不一致、编辑器诊断路径跑全量 codegen 且丢 warnings、analyse_dependencies 双分析、markers 不清理。
   - **Desired state**: 结构化建议统一、廉价诊断路径、单次分析、反馈清理。
   - **Gap**: 语义 DSL 是强 AI 基座，但接缝未标准化。
   - **Priority**: Medium（对 AI 集成是 High）。
10. **编译管线半增量**（MEDIUM）：
    - **Current state**: 缓存省 lex/parse/semantic/lower 但全量 validate/lint/IR 重建/codegen 每键跑；延迟随规模线性。
    - **Desired state**: lint + 校验门控 changed+dependents；增量 codegen。
    - **Gap**: dependents 图存在但只用于分析失效。
    - **Priority**: Medium（编辑器体验）。

### Technical Debt

**High Priority**:
- **React 87 错误豁免台账**（TS7006×44 隐式 any）：Impact——生成代码非严格可评审 + H2 崩溃被掩盖；Resolution——类型发射（codegen 内部闭包 + 合成事件参数）删 54/87；Effort M；Timeline 2-4 周。
- **Flutter warnings 671（ADR-066 诚实下限）**：Impact——86% 诚实降级故事依赖噪音低；Resolution——#43 假阳性修复 + TextStyle 语义路径（触发门控）；Effort M；Timeline 触发后。
- **semantic-universe 台账事实漂移（25+ 行）**：Impact——P3 机制认证虚构；Resolution——重分类 + 事实断言；Effort M；Timeline 2-4 周。
- **半增量编译**：Impact——每键延迟随规模线性；Resolution——changed+dependents 门控 + 增量 codegen；Effort M-L；Timeline 8 周+。

**Medium Priority**:
- 三处管线编排复制（finish_pipeline 提取，S）；JsFormatExpr 死代码（S）；omniview 边界文档（S）；Dart 转义三机制统一（S-M）；token IR 字符串往返（M）；a11y 动态名称缺口（S）；velocity_max 全链路 + slice 特例（S-M）；check_exemption_ledger.sh 入 CI（S）；ADR-056 正文改写 + 台账补行（S）；verify 脚本合并（S）。

**Low Priority**:
- BinaryOpKind::as_ir_str 死契约（S）；Symbol.declared_at 字节偏移→Span（S-M）；evals continue-on-error（S）；#42 软触发改写（XS）；README 计数修正（S）；stdlib.md 弃用（S）。

### Risks

**Technical Risks**:
- **React 动态 URL XSS**（Likelihood: High, Impact: High）——生产后端数据驱动 href 可执行 `javascript:`；Mitigation：sanitize_url_helper_js 发射（S）；Owner：codegen-react。
- **Flutter `$` 注入回归**（Likelihood: Medium, Impact: High）——混合内容段落（最常见文本形状）生成 Dart 静默插值/任意表达式；Mitigation：共享 escape_dart_string + 语料多子节点点（S）；Owner：codegen-flutter。
- **跨目标行为分歧常态化**（Likelihood: High, Impact: Medium-High）——H1 陈旧闭包使 React vs OmniView 同源码不同行为；Mitigation：写后读线程化（M）+ 浏览器行为断言（M）；Owner：codegen-react。
- **「exit 0 输出损坏」组合盲区**（Likelihood: Medium, Impact: High）——ref-op×数组、longpress、scrollTo 双轴；Mitigation：组合 fixture 矩阵（M）；Owner：测试基建。
- **台账信任衰减**（Likelihood: Medium, Impact: Medium）——#43 式修复后仍开行 + 结构断言认证虚构；Mitigation：关闭纪律 + 事实断言（S-M）；Owner：治理。
- **运行时消毒器漂移复发**（Likelihood: Medium, Impact: High）——子串测试放过行为可绕过副本；Mitigation：JS harness 行为断言（M）；Owner：安全。

**Business Risks**:
- **「最地道代码」宣称过度**（Likelihood: High, Impact: Medium）——行为层 5 处实例违约 + 2/5 目标静默丢；用户/市场叙事需修正为「P0 结构层地道，行为层修复中」；Mitigation：修复 + 文档诚实。
- **外部语料升级无迁移工具**（Likelihood: Low, Impact: Medium）——语言获外部作者日即采用成本；Mitigation：QUICKSTART 契约声明 + 未来迁移工具。

**Operational Risks**:
- **编辑器延迟随项目规模线性**（Likelihood: High, Impact: Medium）——20 文件数百 ms/键；Mitigation：增量 codegen（M-L）。
- **新维护者双语/台账学习成本**（Likelihood: High, Impact: Low-Medium）——中文诊断 + 4 脚本 + 台账约定；Mitigation：入职包一页 + 测试约定。

---

## Recommendations

### Immediate（0-2 周）

1. **闭合注入防线两缺口**（Security HIGH-1 + Rust HIGH）
   - **Why**: 一个可构造 XSS（生产后端动态 URL）+ 一个已修 CRITICAL 类缺陷回归（Flutter `$` 转义）。
   - **How**: shared/sanitize.rs 加 `sanitize_url_helper_js` → React 输出发射 + 六 URL 属性包裹（S-M）；`flutter/view.rs:2193/:332` 改共享转义 + 语料 rawtext 加多子节点变体（S）。
   - **Owner**: codegen-react / codegen-flutter。
   - **Success Criteria**: `javascript:` 载荷经动态 href 在 React 输出被阻断（JS harness 行为断言）；`<p>价格 $total 元</p>` 生成 Dart 转义正确 + 语料回归绿。
   - **Estimated Effort**: S-M。

2. **修复 React 行为层三缺陷**（TS/React H1/H2/H3）
   - **Why**: 「地道代码」行为层失信——陈旧闭包 5 实例、点击崩溃 1 实例、数字类型谎言。
   - **How**: H1 事件发射线程化写入值（`nextAgreed` 模式，复用 mutation_value_expr，M）；H2 DCE 一致性（有可达 mutation 的 signal 保留 state hook，S）；H3 数字目标发射 `Number()`/`valueAsNumber` + fixture（S）。
   - **Owner**: codegen-react。
   - **Success Criteria**: FormControlsDemo 勾选后状态行与显示一致；emit 载荷无 off-by-one；排序方向与标签一致；点击无 ReferenceError；`age+1` 不再 "301"。
   - **Estimated Effort**: M。

3. **「exit 0 输出损坏」族修复**（Compiler HIGH-1/2/3 + Domain HIGH-1）
   - **Why**: 合法源码在 P0 目标静默产生非编译/死处理器输出且零诊断。
   - **How**: validate_array_methods walker 下钻 RefOp 子节点 + 组合 fixture（S）；flutter/ref.rs 双轴 scrollTo `[flutter] warning`（S）；validate_event_names 对无目标映射 canonical（longpress/hover/toggle）发 warning + fixture（S）。
   - **Owner**: semantic / codegen-flutter。
   - **Success Criteria**: 三场景均产生诊断且生成代码可编译；组合 fixture 入矩阵。
   - **Estimated Effort**: S。

4. **「宣称 vs 事实」台账校准**（Domain HIGH-2/3 + Compiler HIGH-4 + Documentation HIGH-1/2）
   - **Why**: 治理权威文件与编译器事实矛盾，防漂移机制认证虚构。
   - **How**: semantic-universe hover/toggle/longpress/25+ 事件行改标实际状态（S-M）；theme="dark" 文档修订或 `theme=` warning（S）；bind:checked 正式化（BNF+universe+登记册，S-M）；value={sig} 判别器按 @ref + 提示修正（S-M）；重写 docs/README.md + stdlib.md 弃用（S）。
   - **Owner**: 治理 + parser + 文档。
   - **Success Criteria**: 所有权威文件与实测编译器行为一致；verify 脚本加事实断言样例。
   - **Estimated Effort**: M。

5. **跨目标静默降级收口**（Systems HIGH-1 + Perf HIGH）
   - **Why**: 2/5 UI 目标违约 ADR-052；OmniView 键控列表静默塌缩。
   - **How**: Vue/Dioxus motion/ref/a11y/bind:value → E_CODEGEN 或警告通道（镜像 DESIGN_BLOCK_ERROR）+ ADR-052 floor 表更新（S）；for_key lint 对对象类型 key 表达式报错/警告 + 运行时 typeof 兜底（S）。
   - **Owner**: codegen dispatch / semantic lint。
   - **Success Criteria**: `--target vue` 编译 motion 源产生诊断；`key={item}`（对象项）在 OmniView 不再塌缩。
   - **Estimated Effort**: S。

### Short-term（2-8 周）

1. **门禁事实断言升级**：verify 脚本锚点重指向 parser 常量（SEMANTIC_ANIM_PROPS/token-type 数组/event_names.rs/RefOpIR）+ universe 行 ↔ codegen 白名单交叉检查 + 权威文件入 docs-gate 覆盖（S-M，三成员同见）。
2. **运行时消毒器执行验证 + 语料动态绑定模式**（Security MEDIUM-2，M）：node/vitest 执行发射 helper 对 23 载荷行为断言；语料加 payload-in-signal 动态模式。
3. **浏览器行为断言补 H1 类 + 组合 fixture 矩阵**（TS/React 建议 5 + Compiler 建议 9，M）：checkbox 状态文字、emit 载荷值、排序方向断言；ref-op×数组×@for×motion 组合参数化。
4. **台账关闭纪律**（Implementation Strategist MEDIUMs，S）：#43/#61/#69/#70 补关闭行；ADR-056 Negative 节改写（非注记）；#42 触发重写；check_exemption_ledger.sh 入 CI；verify-deferrals.py 可行性评估。
5. **suggestion 政策标准化 + 编辑器诊断路径切 analyse_multi_cached + markers 清理**（AI Engineer MEDIUM-1/2 + LOW-4，S）：政策 + 契约测试；合并 errors+warnings；空数组清 markers。
6. **管线编排收敛**（Maintainability MEDIUM-1 + Rust MEDIUM，S）：finish_pipeline 提取三处调用；run_ir_passes 移出 ir::lower（边界断言测试）；JsFormatExpr 删除或文档修正。
7. **未声明 token → error + token IR 类型化评估**（Systems MEDIUM-1/2，S-M）。
8. **类型发射删豁免类**（TS/React M1/M6，M）：codegen 内部闭包参数类型 + 合成事件参数类型——TS7006+TS2339 ≈ 54/87 豁免删除。
9. **a11y 动态名称缺口记录 + 文档语言政策成文**（Domain MEDIUM-1 + Documentation LOW-6，S）。
10. **lint + 单文件校验门控 changed+dependents**（Performance MEDIUM-4，M）+ 增量 codegen 评估（M-L）。

### Long-term（2-6 个月）

1. **组合 fixture + 行为 parity 扩展**：per-domain parity oracle 覆盖 10 域 + 文档化分歧负断言（L）。
2. **增量 codegen**：dependents 图驱动只重发射受影响文件（OmniView delta 机制推广）（L）。
3. **useDrag hook 提取 + 事件载荷类型化**（TS/React M2/M7，L）。
4. **per-target 支持矩阵生成**（从 codegen 白名单生成 universe，杀死「台账说全目标、codegen 跳过」类）（L）。
5. **语言/诊断注册表 JSON + AI 集成计划重开（P34 后继）**（AI Engineer 建议 4/5，L）。
6. **治理层合并**（单语言治理模块；第 6 白名单或第 3 登记册触发）（L，触发门控）。
7. **velocity_max 移除**（Pragmatic MEDIUM-1，S-M——可提前做，此处列长期因非阻塞）。
8. **新维护者入职包**（Implementation Strategist 长期 6，M）。

---

## Success Metrics

1. **「宣称 vs 事实」漂移**：Current——6+ 处已知矛盾（hover/toggle、theme="dark"、bind:checked、25 事件行、README、stdlib）；Target——0 已知矛盾 + verify 事实断言覆盖全部 5 词汇域；Timeline——4 周；Measurement——重跑本评审的 10+ 实测点全绿。
2. **React 行为语义**：Current——5 处陈旧闭包实例 + 1 崩溃 + 1 类型谎言；Target——0；Timeline——2-4 周；Measurement——浏览器行为断言（状态文字/载荷/排序方向）+ 点击无 ReferenceError。
3. **注入面**：Current——1 可构造 XSS（动态 URL）+ 1 转义回归（$）；Target——0 + 语料含动态绑定模式 + 运行时 helper 行为验证入 CI；Timeline——4 周；Measurement——JS harness 23 载荷行为断言绿。
4. **React 豁免**：Current——87 错误（44 隐式 any）；Target——≤33（类型发射删 54）；Timeline——8 周；Measurement——tsc --noEmit 计数。
5. **exit-0 损坏类**：Current——3 实证（ref-op×数组、scrollTo x、longpress/hover/toggle）；Target——0 + 组合 fixture 矩阵覆盖；Timeline——4 周；Measurement——组合 fixture 编译 + dart analyze/tsc 绿。
6. **跨目标静默降级**：Current——Vue/Dioxus 4 特性族静默丢 + OmniView key 塌缩；Target——全部 floor 诊断 + 塌缩修复；Timeline——4 周；Measurement——vue/dioxus 编译 motion 源产生诊断；keyFn 对象项测试绿。
7. **治理复杂度（Pragmatic Ratio）**：Current——0.86；Target——≤0.75（velocity_max 移除 + 脚本合并后）；Timeline——8 周；Measurement——重评 Ratio。

---

## Follow-up

**Next Review**: 语义 DSL v3 行为修复轮收口后（预计 2026-08 下旬），或本评审 Critical 项处置完成时。

**Tracking**:
- Critical 5 项 → ADR-067（行为/注入修复轮）计划任务化，遵循 v3 先例（门禁先 → 注入 → 行为 → 文档 → 终验）。
- 台账校准 → 治理轮（verify 事实断言 + 关闭纪律 + 文档入口）。
- 长期项 → deferrals 注册（全带触发条件）。

**Recalibration**:
修复轮收口后运行：
```
"Start architecture recalibration for semantic-dsl-v2"
```

**Accountability**:
- 修复轮由用户驱动的 SDD 流程执行（历史惯例：subagent-driven + 逐任务审查）。
- 本评审 14 个 HIGH + 23 个 MEDIUM 的处置追踪按 v3 先例（findings 表 + 四选一裁决：fix-now / hard-diagnostic / delete / defer-with-trigger）。

---

## Related Documentation

**Architectural Decision Records**:
- ADR-052（跨目标 codegen 契约）——Vue/Dioxus floor 诊断的契约依据（当前被违约）。
- ADR-053/054（语法形式化 + 语义 DSL 时代设计理念）——本评审的判定标准（语义纯度/地道映射/硬诊断）。
- ADR-055（@for 契约）——keyed 列表三目标映射。
- ADR-056（v2 实现）——正文 Negative 节陈旧需改写；注入语料计数需更新。
- ADR-062（v3 修复轮）——本评审的基线（#60/BUG-A 等已关闭）。
- ADR-066（Flutter warnings 燃尽）——671 诚实下限 + TextStyle defer-with-trigger。

**Previous Reviews**:
- `feature-omniview-semantic-syntax-compilability.md`（2026-07-31，13 专家）——本评审的动因与基线；其 findings 处置追踪见 `.architecture/reviews/feature-omniview-semantic-syntax-compilability-v2.md`。
- `feature-omniview-semantic-syntax-compilability-v2.md`（2026-08-03，13 专家 v2 对齐）——0.63→0.69；「宣称实测全真但行为层未兑现」——本评审确认行为层缺口仍在（H1 陈旧闭包等为实证新增）。

**Referenced Documents**:
- `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（902 行，10 域设计契约）——theme="dark" 验证清单勾选不存在、bind:checked 未文档化、value_prop 被遮蔽。
- `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（1325 行 BNF）——自声明 Superseded 但仍是门禁锚点。
- `docs/superpowers/specs/vocabulary-registry.md` + `semantic-universe.md`——hover/toggle 过度宣称、25+ 事件行漂移。
- `scripts/verify-vocabulary.py` / `verify-semantic-universe.py`——5 函数重复、锚点 Superseded。
- `docs/omniview/` 16 篇 + `docs/README.md` + `docs/stdlib.md`——入口陈旧/幽灵文档。
- `omnilang/src/codegen/{react,flutter,omniview}/`——H1/H2/H3、keyFn 塌缩、$ 转义回归位置。
- `crates/omnilang-core/src/`（parser/semantic/ir/analyse）——walker 缺口、管线编排、Span 缺口。

---

## Appendix

### Review Methodology

本评审由 AI Software Architect 框架执行，12 位成员独立并行评审后聚合：

- **Compiler & Language Architect**: 语言设计健全性、语义鸿沟、多目标 codegen 一致性（omnic 实测编译）。
- **Domain Expert**: 语义域建模准确性、词汇封闭纪律（omnic 实测 10+ 诊断面）。
- **TypeScript & React Specialist**: React 生成代码地道性（重跑 tsc/eslint 门禁 + 逐组件审查生成代码）。
- **Rust Expert**: 编译器实现质量、注入消毒单源（scratch 编译 diff Dart）。
- **Systems Architect**: 四层管线自洽性、多目标治理、门禁互补性。
- **Security Specialist**: 注入防线闭环（追踪每条字符串发射面静态+动态）。
- **Performance Specialist**: 生成代码热路径 + 编译管线性能。
- **Maintainability Expert**: 技术债、复杂度分布、演进性。
- **Implementation Strategist**: 变更管理、deferral 质量、可逆性。
- **Documentation Expert**: 文档体系入口/参考/迁移（docs-gate 实测 247 块）。
- **AI Engineer**: AI/agent 工具化基座就绪度。
- **Pragmatic Enforcer**: YAGNI/复杂度守卫（语料消费者计数 + 门禁 ROI 溯源）。

**Pragmatic Mode**: Balanced
- Complexity ratio target: <1.5（实测 0.86）
- 全部建议经 YAGNI 透镜评估（velocity_max/count()/门禁合并裁决）

**实证方式**: 6 位成员用 release 版 `omnic`/真实工具链实测（编译 scratch 源、重跑 tsc/dart analyze/docs-gate、grep 全仓计数），非仅读码。

**校准说明**: 共享简报中的「已知未决」项（#60 浏览器套件未入 CI / BUG-A / unused-import 123）经多位成员实证已在 08-04/08-05 关闭（omniview-e2e CI job、db68d812、e454f907）——本评审所有发现均已按关闭事实校准；ADR-056 正文（非注记）陈旧问题本身成为发现之一（见 Implementation Strategist MEDIUM）。

### Glossary

- **P0 目标**: React/Flutter/OmniView（语义 DSL 全域支持）。
- **次级目标**: Vue/Dioxus（域外特性当前静默丢弃——Systems HIGH-1）。
- **语义全集（semantic-universe）**: 语义集 vs 已实现集分离机制（设计原则 7）。
- **词汇登记册（vocabulary-registry）**: 5 个封闭词汇域 + 借词黑名单（设计原则 6）。
- **floor 外硬诊断**: 设计原则 3——删除/未支持语法必须响亮失败 + 迁移提示。
- **台账绿**: React 门禁「GATE PASSED」= 87 个已知错误全在豁免计数上限内（非 strict 绿）。
- **组合盲区**: 单域 fixture 矩阵覆盖不到的跨域组合形态（ref-op×数组、事件×目标、写后读顺序）。

---

**Review Complete**
