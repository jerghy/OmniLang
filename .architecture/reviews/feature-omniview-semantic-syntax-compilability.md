# Architecture Review: OmniView 语义级语法 → React/Flutter 地道代码可编译性

**Date**: 2026-08-02
**Review Type**: Feature
**Reviewers**: compiler-architect (Dr. Lin Wei), typescript-react-specialist (Emma Rodriguez), flutter-reviewer, domain-expert, pragmatic-enforcer, maintainability-expert, rust-expert, performance-specialist, implementation-strategist, systems-architect, security-specialist, ai-engineer, doc-updater

**评审问题**: 当前 OmniView 语义级语法是否能编译为地道 React (TS+Zustand v5) / Flutter (Dart+Riverpod 3.0) 代码？不能的语法该修改/删除。

---

## Executive Summary

语义级 DSL 三层架构（semantic → IR → codegen）本身被**必要且充分**地验证（pragmatic enforcer Ratio 0.63；编译器架构师实测编译；可维护性专家确认依赖方向单向干净）。语法层的语义净化大体达成：无 DOM 形状泄漏的 Ref API（全部编译期拒绝 + 迁移提示）、17 属性白名单解析期强制、Slot fallback 修复入 IR、诚实降级在 motion/slot 域被内化。**「语义 DSL 生成地道代码」的方向是正确的，且多数失败点不在语法层而在 codegen 实现、验证门禁与文档同步。**

然而，对评审问题的直接回答是**「当前不成立，但可修复」**——多名专家用当前 HEAD 编译器实测（`omnic build -t react` 157 文件、`-t flutter` 全量输出）：

- **React 目标输出当前无法编译**：组件局部函数从插值/内联样式路径被遗漏（DataTable.tsx 调 6 个未声明函数，tsc TS2304 ×6）；style 的 `$signal` JSON 通道把 `row.score` 篡改为不存在的 `row_score` 标识符（C1/C2，均产生 ReferenceError）。声称的 tsc 门禁**不存在于实际状态**：react-test-app 锁定 React 18.3 + Zustand 4.5、`strict: false`、无 eslint 配置、仓库内无任何 `tsc --noEmit` 脚本；实测 `tsc --noEmit` 60+ 错误。
- **Flutter 目标输出含静默破坏构造**：`@for` 无条件发射 `ListView.builder`（嵌套/非滚动上下文产生 unbounded viewport 崩溃）且完全忽略 `key_expr`（无 ValueKey）与 `[scroll]` hint——spec 明文规定 Column 默认 + `[scroll]` 才用 builder；Classes/Variants 域整体静默 no-op（`style_classes` 从未被读取，只剩空 `BoxDecoration()` TODO 桩）；prop 值不做类型转换（`fontWeight: 'bold'` 字符串赋给 `FontWeight`、`Image.network(image:)` 错误命名参数、`Icon()` 缺必填参数——三个常见输入直接破坏 dart analyze）；`capturePointer`/`releasePointer` 静默变注释（spec 承诺 error）；语义绑定每次 rebuild 无条件重放（焦点被反复偷走）。
- **一个已端到端验证的 CRITICAL 注入漏洞**：React 后端 `format_class_ref` 对类名不转义——`class: "x',alert(document.cookie),'"` 编译出完全合法的 TSX `styles['x',alert(document.cookie),'']`，esbuild/tsc 均通过、执行时触发任意 JS。供应链威胁模型（项目自身已在 `react/view.rs:227-230` 防御）下等价 first-party RCE。**现有 tsc 门禁无法发现。**
- **主题切换在三目标静默失效**：`theme = "dark"` 语法在编译器中不存在（编译报 "Undeclared variable 'theme'"）——BNF 权威文档却带各目标生成代码示例展示它；demo 路径 `ThemeStore.setMode("dark")` 在 React 生成代码中无任何 data-theme 写入（设计 token 的 `[data-theme="dark"]` CSS 覆盖永不生效）；Flutter 只生成 provider + 引导注释。floor 表声称 ✅ 而行为未接通——正是 ADR-054 宣称已归零的「静默错误」类。
- **验证门禁系统性盲区**：展示项目 Flutter 输出无任何 CI 门禁（含已知不可编译的 match #30 产物）；`compile_never_panics` proptest 是空测试（`catch_unwind` 结果被丢弃，且不含 codegen 后端——2 名专家独立印证）；增量缓存静默丢弃未变更文件诊断（poiesis 编辑器主路径）；React/Vue/Dioxus 隐藏依赖 OmniView 内部 `is_dangerous_url`（per-backend feature 门禁失效 E0432）；CLI 三个命令从不打印 warnings。
- **文档-规范-parser 三方漂移 7+ 处**：spec 教授解析器拒绝的箭头闭包与 `value={sig}` 双向语义、06-templates 教错误插值符 `{name}`、README 教不存在的 `omnic check`、10-design-system 残留「design 仅 OmniView」陈句、spec 三处标注内联动画体「未实现」而实际已实现、spec 状态仍为 Draft、设计文档教授不解析的 `@scroll start/end/scrub` 子属性。

**Overall Assessment**: **Needs Improvement** —— 架构方向正确且必要，但「生成地道可编译代码」的核心承诺在 HEAD 上未兑现（React 不编译、Flutter 含静默破坏、1 个可利用注入、门禁与文档均与宣称不符）。

**Key Findings**:
1. React 目标存在已端到端验证的代码注入漏洞（className 未转义）且现有门禁无法发现
2. React/Flutter 两个生产目标的 @for 渲染都违反自己的契约（React key 恒为 `[object Object]`；Flutter 无条件 ListView.builder + 丢弃 key_expr）
3. 主题切换（floor 声称 ✅ 的旗舰语义）在三目标静默失效
4. 验证门禁系统性盲区：react-test-app 假门禁、展示项目 Flutter 无门禁、proptest 空测试、增量缓存丢诊断

**Critical Actions**:
1. 修复 React className 注入（`format_class_ref` 转义 + lowering 字符集校验双保险）
2. 修复 React C1/C2（函数发射覆盖 + style `$signal` 通道）使生成代码可编译
3. 修复 Flutter `@for`（Column 默认 + `[scroll]` 分派 + `ValueKey`）并补快照测试
4. 主题切换语义收敛（实现 `theme = "dark"` 或从 floor 移除 + 硬错误）
5. 立起真实门禁：react-test-app 升级 React 19 + Zustand 5 + strict tsc + eslint；展示项目纳入 codegen-validate；proptest 加断言

---

## System Overview

- **评审对象**: OmniView 语义级 DSL（`.omni` 视图语言，9 语法域：Ref 11 ops + 4 语义绑定、动画 8 触发器/17 属性白名单、Design Tokens 13 类型/themes/classes/variants/动态、Slots、Stores、@if/@for、Events、Expressions、Styling）
- **架构**: 三层——语义原语（`crates/omnilang-core/src/semantic/`）→ IR（`crates/omnilang-core/src/ir/`）→ codegen（`omnilang/src/codegen/{react,flutter,omniview}`）。P0 目标：React (TS+Zustand v5+React 19) / Flutter (Dart+Riverpod 3.0) / OmniView 调试运行时。非 P0：Vue/Dioxus/Python/JS/TS/Rust/Dart。
- **关键契约**: ADR-021 重定义（floor 内行为对等 + floor 外硬诊断 + 绝不静默降级）；ADR-052（24-item floor + 删除清单）；ADR-053（BNF 9 域规范化 + 单一权威）；ADR-054（5 原则）；ADR-055（@for 渲染与事件委派契约，仅 OmniView）。
- **规模**: ~97k 行 Rust（2 crates）、1,403 tests（+748/639 按 feature）、tsc 12 fixtures + dart analyze 门禁（声称）、learn-omniview 展示项目（92 `.omni` 文件、~19.5k 行、9 标签页）。
- **验证方法**: 13 名专家独立评审 + 多名专家实测编译（`omnic build -t react` 157 文件、`-t flutter`、`tsc --noEmit` 60+ 错误、攻击载荷端到端复现、`cargo check --no-default-features --features codegen-react` E0432、all-backends 634 tests 全绿）。

---

## Cross-Cutting Themes

以下主题在 3+ 名专家评审中独立出现（高杠杆发现）：

### 1. @for 在双生产目标均违反自身契约（5 名专家）
- flutter CRITICAL：无条件 `ListView.builder` + 丢弃 `key_expr`/`scroll_hint`（view.rs:1087-1119），零测试覆盖
- perf HIGH-2 + ts/react H1：React `key={String(item)}` → 对象项恒为 `"[object Object]"`，keyed reconciliation 静默塌缩（DataTable 排序演示直接受影响）
- systems MEDIUM-3 + compiler HIGH-3：同源（Flutter 忽略 key_expr 且无诊断）；跨后端参数化矩阵无 @if/@for fixture（MEDIUM-4）
- ADR-055 只覆盖 OmniView 目标——双生产目标的 @for 契约无人守护

### 2. 验证门禁系统性盲区（8 名专家）
- strategy HIGH-1：展示项目 Flutter 输出无 CI 门禁（6-CRITICAL-bug 周期的同源盲区）
- ts/react H3：react-test-app 锁定 React 18.3 + Zustand 4.5、无 eslint、无 tsc 脚本；实测 60+ 错误；Playwright 末次 180/180 failed
- rust H1/H2 + security MEDIUM：增量缓存丢诊断 + proptest 空测试（2 名专家独立印证同一代码）
- systems HIGH-1/HIGH-2：feature 门禁假（is_dangerous_url 依赖 OmniView 模块）+ CLI/编辑器丢 warnings
- compiler MEDIUM-10 + ai MEDIUM-3：fixture 矩阵不含已知缺口（match/theme/@for key）；learn-omniview Flutter 输出含不可编译 Dart 而无门禁
- 结论：**「tsc 门禁 + dart analyze 门禁」的宣称与实际不符**——门禁只覆盖合成 fixture 子集，旗舰项目与真实功能组合无门禁。

### 3. 「静默 no-op / parse-and-ignore」类残留（8 名专家）
- Flutter：Classes/Variants 静默无视觉效果（style_classes 从未被读）、capturePointer 变 TODO 注释（spec 承诺 error）、@if mode:"fade" 静默失效、event 字段提取 10+ 静默桩、design 不可映射值静默跳过（spec §10.3 诊断表零实现）
- `paused` 解析后静默丢弃（strategy MEDIUM-1 + domain LOW-1，双专家）
- `@native`/`theme = "dark"` 文档虚构（pragmatic MEDIUM-1/2）
- python/vue BlockExpr 语句体静默丢弃（rust M3）
- OmniView unhandled StatementIR 仅 warning（maintainability LOW-7）
- 结论：ADR-054 原则 3 的「雷」定义（静默失败）在 codegen 层未贯彻——语义层纪律强、codegen 层纪律弱。

### 4. 文档-规范-parser 三方漂移（7 名专家，10+ 处）
- doc HIGH-1..4：spec 教箭头闭包（parser 只认管道 `|x|`）、06-templates 教 `{name}`（实际 `${name}`）、README 教不存在的 `omnic check`、10-design-system 残留「design 仅 OmniView」
- pragmatic MEDIUM-1/2：`theme = "dark"`、`@native` 文档虚构
- compiler MEDIUM-5：内联动画体实际已实现、spec 三处标「未实现」（反向漂移）
- domain MEDIUM-4/LOW-3：设计文档教不解析的 @scroll/drag 子属性；BNF 遗漏已实现的 variant base/&:: 
- ai MEDIUM-1：spec 所有组件示例省略强制 `view { }` 块（LLM 按权威文档生成 = 系统性不可编译）
- systems MEDIUM-5 + doc LOW-1：spec 状态仍 Draft
- 结论：ADR-053 D1「BNF 单一权威」与 ADR-054 P5「文档对齐」在两个方向上都在失效。

### 5. 主题切换静默失效（3 名专家）
- domain HIGH-1：一个概念（切换主题）、三个名字（`theme`/`mode`/`themeModeProvider`）、三种机制、一种不存在的语法；React 孤儿 useThemeStore 无人接线
- compiler HIGH-2：双路径实测断裂（spec 语法报 Undeclared variable；demo 路径 setMode 后无 data-theme 写入）；`useThemeStore.ts` basename 冲突
- pragmatic MEDIUM-1：BNF 权威文档展示不存在的生成代码
- 结论：floor 表中「主题 | ✅」是静默失实的——被「tsc/dart analyze 只验编译不验行为」的门禁系统性掩盖。

### 6. 绑定解析策略每后端重实现（维护性根因，2 名专家）
- maintainability HIGH-1/HIGH-2：`IdentifierExpr` 只带原始名字，无绑定种类注解；遮蔽修复 242a5e53 只落在 OmniView——React/Flutter 仍带同款潜在 bug（`validate_all(items)` 中 items 是参数却被发射为 `state.items`）；最近 4 个修复 commit 全部触及此类代码
- 结论：一个 bug 类（而非单个实例）被反复修复，因为 IR 缺少绑定解析信息。

### 7. React store 订阅粒度（2 名专家）
- perf HIGH-1 + ts/react H2：组件订阅整个 Zustand store（生成好的 per-field selector hooks 从未被使用），useMemo 依赖 `[TodoStore]` 对象恒新；Flutter 侧 `select()` 精确订阅——同源码两目标性能对等性差距。

---

## Trade-offs and Disagreements

**1. `value={sig}` 语义方向（双向 vs 单向）**
- compiler HIGH-1（实测）：React 单向（`useEffect` 写 DOM，输入不回写信号）、Flutter 双向（listener 回写）——同一 fixture 数据流方向相反；文档三方矛盾（spec §2.3 注释「双向」、§2.4 表「React 生成 value+onChange」、06-templates.md「单向」）
- domain HIGH-2：`value={sig}`（spec 旗舰示例，demo 0 使用，要求 @ref）vs `bind:value`（demo 33 处事实标准）vs 隐式 auto-bind（需 `value={""}` 反模式规避）——同一意图三个表面
- **消解**：两专家一致指向「收敛为一个语义」。推荐：`bind:value` 保留为双向事实标准；`value={sig}` 或降级为 `bind:value` 别名、或明确为单向命令式同步（与 06-templates.md 和 React 实现一致）并统一 Flutter 侧；auto-bind 删除或硬诊断。需用户决策（语法删除需 ADR）。

**2. deferral #17/#22 状态（已修 vs 未修）**
- ai-engineer/flutter-reviewer 引用为「open + 白名单门禁」；systems-architect 实测 `generated_code_validation.rs` 已全量枚举（`flutter_analyze_targets`）、6 个原始 bug 注释明确已修
- **消解**：#17 的 6 个原始 CRITICAL 已修复且门禁已从白名单改为全量枚举 fixture——但门禁仍只覆盖合成 fixture（不含展示项目与 prop 转换/@for/theme/match 形状），flutter-reviewer 发现的 gate-breaking 输入不在快照语料中。两者都真：关闭 #17（附证据）+ 新增门禁缺口为新发现。

**3. CI 门禁是否存在**
- ts/react H3（实测）：react-test-app 无 tsc/eslint、版本错、Playwright 180/180 failed
- systems/strategy/compiler：`generated_code_validation.rs` 的 tsc 12 fixtures + dart analyze 存在
- **消解**：合成 fixture 门禁存在且曾捕获真实 bug（ADR-054:82）；展示项目 harness 门禁不存在且版本陈旧。两者独立——fixture 门禁不能替代展示项目门禁。

**4. `capturePointer(id)` 参数处置**
- domain MEDIUM-1：`id` 是 DOM pointerId（平台形状泄漏），demo 被迫编造 `capturePointer(1)`
- flutter HIGH：实现静默变注释，spec 承诺 error
- **消解**：一致同意——语义层去 id 参数（capturePointer() 捕获全部）或语义化「指针身份」；Flutter 至少发出 spec 承诺的 error 诊断。

---

## Consolidated Findings

### Strengths

1. **三层架构依赖方向干净**（systems 实测 import 扫描）：semantic 只消费 AST/IR、codegen 只消费 IR；历史 P35 反向依赖问题未回归。
2. **RefOpIR 语义 pass 是教科书级意图识别**（compiler）：非法 DOM API 编译期拒绝 + 迁移提示、正确处理 5 类遮蔽、`ref.value = x` 写路径改写；20+ 测试。
3. **Flutter store/ref 层地道**（flutter + compiler）：Riverpod 3.0 `Notifier<T>` + 精确 `select()`、GlobalKey + 按需 controller + 完整 dispose、回调内 watch→read 重写正确。
4. **React motion 子系统最地道**（ts/react）：WAAPI + IntersectionObserver + 完整 cleanup + prefers-reduced-motion 处理。
5. **诚实降级在 motion/slot/设计域内化**（4 名专家）：steps() 警告 + 回退、`mode:"wait"` 降级注释、拖拽 extent 推导 + 警告、slot 类型约束警告——「生成代码注释降级」模式已验证可行。
6. **安全防御纵深在 OmniView 目标扎实**（security）：textContent/classList.toggle 动态路径、escape_html 静态路径、编译期 + 运行期双重 CSS 消毒、`[a-zA-Z0-9_-]` 标识符限制按构造封闭注入面。
7. **可执行文档模型有效**（doc 实测）：learn-omniview `omnic build -t omniview` 干净（exit 0）、15-grammar-view 的「从 parser 提取」参考模型可行、03-expressions 闭包文档是精度范本。
8. **删除纪律正确**（pragmatic）：ADR-052/053 的删除（ref.style.*/paused_override/@paused/Hooks 模式）零使用量执行、无 deprecation shim——24-item floor 映射真实 demo 使用。
9. **诊断基础设施方向正确**（systems）：单一 `Diagnostic` 类型全链路贯穿、非 P0 design block 硬错误门、ts-rs 新鲜度检查。

### Areas for Improvement

1. **React 生成代码可编译性**（CRITICAL）：C1 函数发射遗漏 + C2 style `$signal` 篡改 → 当前输出 ReferenceError。优先级：CRITICAL。
2. **Flutter @for 契约**（CRITICAL）：Column 默认 + `[scroll]` 分派 + `ValueKey` + 快照测试。优先级：CRITICAL。
3. **React className 注入**（CRITICAL）：转义 + 字符集校验 + 回归测试（断言不含载荷模式而非仅可编译）。优先级：CRITICAL。
4. **主题切换语义**（HIGH）：实现 `theme = "dark"` 或 floor 移除 + 硬错误；消除 useThemeStore 命名冲突。优先级：HIGH。
5. **React store 订阅粒度**（HIGH）：接线已生成的 per-field hooks，useMemo 依赖切片。优先级：HIGH。
6. **Flutter 语义绑定 deps 重放**（HIGH）：快照比较仅变化时应用（消灭焦点偷窃）。优先级：HIGH。
7. **Flutter Classes/Variants**（HIGH）：解析 style_classes 或诚实警告；不可映射 design 值加 warnings 通道。优先级：HIGH。
8. **Flutter prop 类型转换**（HIGH）：fontWeight/fontSize 字符串 → 类型值；img/icon 特例；快照测试。优先级：HIGH。
9. **Flutter 诊断纪律**（HIGH）：capturePointer 错误化、fade 降级注释、event 字段桩警告、scrollTo 双轴警告。优先级：HIGH。
10. **验证门禁体系**（HIGH）：react-test-app 升级 + strict tsc + eslint；展示项目纳入 codegen-validate；proptest 断言修复；增量缓存诊断恢复。优先级：HIGH。
11. **绑定解析入 IR**（HIGH/L）：BindingRef 注解消除每后端重实现（维护性根因）。优先级：L（结构性）。
12. **文档三方同步**（HIGH）：10+ 处分歧批量修复 + 文档编译 CI 门禁 + spec 转 Approved。优先级：HIGH（立即项低工作量）。
13. **该删未删语法**（MEDIUM）：`theme = "dark"` 文档虚构删除、`@native` token 删除、`RefOpIR::SetValue` 删除（11→10 ops）、`test-id` 删除、`paused` 硬诊断化、`scroll-x` 补 demo 或删除。优先级：MEDIUM（删除最便宜时刻是现在）。

### Technical Debt

**High Priority**:
- **React 生成代码当前不编译**（C1/C2）：影响——P0 目标核心承诺失效；Resolution——发射遍历补全 + style 通道修复；Effort M；Timeline 立即
- **Flutter 静默破坏构造群**（@for/classes/prop 转换/capturePointer）：Effort M（分项）；Timeline 2-4 周
- **react-test-app 假门禁**（React 18 + Zustand 4 + 无 lint）：Effort M；Timeline 立即
- **WASM FFI 契约漂移**（3 套 Diagnostic 序列化器 + 10 个 json! 信封 + 每调用序列化缓存）：Effort M；Timeline 短期
- **增量缓存丢诊断**（rust H1）：Effort S；Timeline 立即（poiesis 编辑器主路径）

**Medium Priority**:
- **store 名 vs 参数遮蔽**未移植 React/Flutter（maintainability HIGH-1）：Effort S；Timeline 短期
- **Python/Vue BlockExpr 静默丢弃**（rust M3）：Effort S；Timeline 短期
- **消毒器三份拷贝漂移**（hex-escape 绕过）：Effort M；Timeline 短期
- **超大文件**（flutter/design.rs 2458、ir/lower/mod.rs 2252）：Effort M；Timeline 随下一次触碰拆分
- **is_dangerous_url 模块错位**：Effort S；Timeline 短期
- **--all-features 测试漂移**（deferral #22，~14 处机械修复）：Effort S-M；Timeline 随第一笔 IR 变更

**Low Priority**:
- **lib.rs.orig 死文件**：Effort XS；Timeline 立即
- **flush 类**：escape_literal_str 单遍化、fast_hash SipHash 决策、const 传播、_M 表提升、双求值 iterable——Effort S-M；Timeline 长期

### Risks

**Technical Risks**:
- **React 注入漏洞潜伏**（Likelihood: High, Impact: Critical）：任何含恶意/被篡改 .omni 组件的项目 = first-party 任意 JS 执行；现有 tsc 门禁不可见。Mitigation：立即修复 + 载荷语料回归。
- **静默错误类回归**（Likelihood: High, Impact: High）：ADR-054 宣称归零的「静默失败」在 codegen 层仍有 10+ 实例（主题/@for/classes/capturePointer/fade）；门禁不覆盖则每轮浏览器测试重新发现。Mitigation：codegen 层诊断纪律 + 展示项目门禁。
- **门禁盲区持续**（Likelihood: High, Impact: High）：6-CRITICAL-bug 周期重演（committed golden 中不可编译代码数周无人发现——ADR-052:173 历史教训正在重演：learn-omniview Flutter 输出当前即不可编译）。
- **漂移复合**（Likelihood: Medium, Impact: Medium）：spec/设计文档/grammar 三份文档持续分歧，LLM 与实现者都基于错误权威工作。

**Business Risks**:
- **核心卖点失信**：「一处编写、三端地道」——React 不编译、Flutter 主题/列表/类静默失效，被发现后连带质疑 floor 表整体可信度。
- **AI 时代语境失效**（ai-engineer）：LLM 按权威文档生成系统性失败（view{}/箭头闭包/`{name}`），「语义 DSL 对 AI 友好」的优势不兑现。

---

## Recommendations

### Immediate (0-2 weeks)

1. **修复 React className 注入**（security CRITICAL）
   - Why：已端到端验证的任意 JS 执行；供应链威胁模型内 first-party RCE
   - How：`format_class_ref` 用 `escape_literal_str(name, '\'')` + lowering 层字符集校验双保险；回归测试断言生成 TSX 不含载荷模式
   - Success Criteria：载荷语料（`'`、`"`、`\`、反引号、`${`、`</`）全后端插入点回归通过
2. **修复 React C1/C2 使生成代码可编译**（ts/react CRITICAL）
   - How：`collect_called_fns_from_view` 补 `Interpolation`/`el.props`/`inline_style`/`bindings` 遍历；`lower_expr_to_json_from_ir` 仅对真实信号/store 引用用 `$signal` 下划线形式
   - Success Criteria：learn-omniview React 输出 `tsc --noEmit` 零错误
3. **修复 Flutter @for**（flutter CRITICAL + compiler HIGH-3 + systems MEDIUM-3）
   - How：`scroll_hint` 分派（false → `Column(children:)`，true → `ListView.builder`）；`key_expr` → `key: ValueKey(...)`；补两种形式的快照测试
   - Success Criteria：@for fixture（keyed + 非滚动 + 嵌套）通过 dart analyze + 快照断言
4. **主题切换语义收敛**（domain HIGH-1 + compiler HIGH-2 + pragmatic MEDIUM-1）
   - 三选一（需用户/ADR 决策）：实现 `theme = "dark"` 语句并三目标接通（React 需同时解决 useThemeStore 接线与命名冲突）；或统一信号名 + React 订阅用户 store；或 floor 降级为「仅 OmniView」+ 硬诊断
5. **立起真实门禁**（ts/react H3 + strategy HIGH-1 + rust H2 + security MEDIUM）
   - react-test-app：升级 React 19 + Zustand 5、strict: true、`*.module.css` ambient、`tsc --noEmit` 脚本、eslint-plugin-react-hooks
   - `generated_code_validation.rs`：learn-omniview main.omni → Flutter → 全输出 dart analyze
   - `property_tests.rs:76-80`：加 `assert!(result.is_ok())` + 增加 codegen 后端 fuzz 目标
6. **恢复增量缓存诊断**（rust H1）+ 移除 resolve_styles warnings 门（rust M2）——各一行级修复，poiesis 编辑器主路径
7. **批量修复文档漂移**（doc HIGH-1..4 + pragmatic MEDIUM-1/2 + compiler MEDIUM-5 + ai MEDIUM-1 + domain MEDIUM-4/LOW-3 + systems MEDIUM-5）
   - spec：箭头闭包改管道、补 `view { }`、`theme = "dark"`/`@native` 删除、内联动画体标注「map 形式已实现」、状态转 Approved + 最后同步 commit
   - 06-templates `${name}`、README CLI 命令与版本、10-design-system 陈旧句、设计文档 §9.1 `@mount` 前缀、§3.6/3.7 未实现标注
8. **该删未删处置**（pragmatic MEDIUM-1/2/3 + strategy MEDIUM-1 + domain LOW-1）
   - 删除：`theme = "dark"` spec 节、`AtNative` token、`RefOpIR::SetValue`（11→10 ops）、`test-id`；`paused` → 解析期 warning/error；每发现指派四选一处置（fix-now / hard-diagnostic / delete / defer-with-trigger），评审结束零悬空项
9. **修复 store 名 vs 参数遮蔽移植**（maintainability HIGH-1，React/Flutter 各 ~10 行）+ 补 3 个缺失回归测试（FnRef 扫描/fn_locals/DCE CallExpr）

### Short-term (2-8 weeks)

10. **React store 订阅粒度**（perf HIGH-1 + ts/react H2）：接线 per-field hooks、useMemo 依赖切片化——最大单项生产性能收益
11. **Flutter 语义绑定 deps 快照**（flutter HIGH + compiler MEDIUM-6）：消灭焦点偷窃与滚动动画重放
12. **Flutter Classes/Variants 解析**（flutter HIGH + compiler MEDIUM-8）：style_classes → `AppDesign.*` 装饰；gen_design_files 加 warnings 通道
13. **Flutter prop 类型转换层**（flutter HIGH）：fontWeight/fontSize/Color/IconData 转换、img/icon 特例、快照测试
14. **Flutter 诊断纪律批量**（compiler HIGH-4/MEDIUM-7/LOW-11 + flutter）：capturePointer error、fade 注释、event 字段桩警告、scrollTo 双轴警告、design §10.3 诊断表实现
15. **React 修复批次**（ts/react M1-M8）：array_sort comparator（Toggle Sort 当前不排序）、setX 函数式更新、Object.assign 清理、set() 合并、effect deps 从 body 分析、computed deps 含 props、fn 参数类型、typed useState
16. **WASM/FFI 契约统一**（rust M1/M4/M5）：单一 DiagnosticJson 序列化器、typed 信封、去每调用缓存序列化、diagnostics/analyse_dependencies 走多文件路径
17. **is_dangerous_url 移入 shared + feature 矩阵 CI job**（systems HIGH-1）
18. **warnings 网关收口**（systems HIGH-2）：OmniViewResult 加 warnings 字段、CLI 打印、lint 提升进 run_pipeline
19. **跨后端 fixture 扩展**（compiler MEDIUM-10 + maintainability）：match / @for key+empty / theme / typed slots / capturePointer / 内联动画体 + @if/@for 参数化 fixture；learn-omniview 纳入门禁（先清 deferral #30 match 变体臂）
20. **`value={sig}` 语义决策与收敛**（compiler HIGH-1 + domain HIGH-2，需用户/ADR 决策）：bind:value 为准，value={sig} 别名或单向化，auto-bind 删除或诊断

### Long-term (2-6 months)

21. **绑定解析入 IR**（maintainability HIGH-2）：IdentifierExpr 加 BindingRef 注解，删除每后端 renames/fn_locals/name_resolver 机制——消灭整类 bug
22. **类型化 slot render-prop 映射或删除**（compiler MEDIUM-9 + pragmatic LOW-4）：React `(data: T) => ReactNode` / Flutter `Widget Function(T)?`，或从语法删除类型约束
23. **Flutter 运行验证体系**（flutter）：widget-test harness（tester.pumpWidget 冒烟 @if/@for/motion/ref）——dart analyze 无法捕获布局异常/Riverpod 断言/焦点行为
24. **消毒器单一事实来源**（security）：Rust runtime.rs 生成 JS helper 字符串，React/OmniView 共用；行为一致性测试
25. **注入载荷语料回归 + 产物静态验证**（security）：per-backend 安全快照；CI 扫描「产物不得含未转义分隔符」
26. **超大文件拆分**（maintainability MEDIUM-6）：flutter/design.rs、ir/lower/mod.rs、resolve.rs——随下一次触碰排期
27. **AI 侧增强**（ai-engineer）：00-cheatsheet（每域一例 + omnic check CI 断言）、生成代码 `// [omni] file:line` 源注释 + 再生成契约文档、生成代码漂移检测
28. **共享滚动驱动 + const 传播 + motion 控制器按元素实例化**（perf MEDIUM-1/3/4 + flutter）
29. **用户向平台映射文档**（doc MEDIUM-5 + ai）：按 06-templates mode 表模式逐特性三后端支持状态表

---

## Success Metrics

1. **React 输出可编译性**: learn-omniview React 输出 `tsc --noEmit --strict` 错误数 60+ → 0（2 周）；eslint-plugin-react-hooks 零告警（1 月）
2. **Flutter 输出可编译性**: learn-omniview Flutter 输出 `dart analyze` 0 错误 0 警告（2 周）；@for keyed/非滚动 fixture 快照断言（2 周）
3. **注入面**: 载荷语料回归全后端通过；`class: "x',alert(1),'"` 类载荷被拒绝（立即）
4. **静默降级归零**: 主题切换三目标行为验证（demo 点主题卡片 → data-theme/themeMode 实际变化）；capturePointer Flutter 产生 error 诊断（2 周）
5. **门禁真实性**: react-test-app 锁定 React 19 + Zustand 5 + strict tsc 全绿（2 周）；codegen-validate 含展示项目 Flutter 全量 dart analyze（1 月）
6. **文档一致性**: spec/设计文档/16 篇 docs 与 parser 全量代码块编译审计通过（`omnic build` 逐块验证，1 月）
7. **测试可靠性**: `compile_never_panics` 真正断言 panic（立即）；增量缓存诊断回归测试（立即）；缺失 3 个回归测试补齐（2 周）
8. **对等性**: 跨后端 fixture 矩阵覆盖 9 域 × 3 目标（match/@for/theme/typed slots/capturePointer 加入，1 月）

---

## Follow-up

**Next Review**: 本批 Immediate 项完成 + ADR 决策（主题语义、value={sig}、capturePointer 参数）后做一次对齐评审。

**Tracking**:
- 每条发现按四选一处置（fix-now / hard-diagnostic / delete / defer-with-trigger），评审结束零悬空项
- 治理同步：deferral #17/#22 关闭（附证据）、#18/#19/#21/#23/#28/#33 补触发条件、spec 转 Approved、删除 lib.rs.orig
- ADR 决策候选项：主题切换语义收敛（HIGH-2）、`value={sig}` 语义统一、capturePointer(id) 参数语义化、typed slots 去留

**Accountability**: 评审文件 + 本会话记忆为追踪基线；下一会话按「门禁先行 → CRITICAL 修复 → 文档同步 → 该删未删」顺序执行。

---

## Related Documentation

**Architectural Decision Records**:
- ADR-052（语义级 DSL 跨目标 codegen）——本评审验证其验收标准未全部兑现（learn-omniview Flutter 输出仍不可编译）
- ADR-053（BNF 规范化 + 单一权威）——「单一权威」模型因 10+ 处 spec/实现分歧失效
- ADR-054（设计理念 5 原则）——原则 3（诚实降级）在 codegen 层未贯彻（10+ 静默 no-op）；原则 5（文档对齐）双向漂移
- ADR-055（@for 渲染契约）——仅覆盖 OmniView；React/Flutter 的 @for 契约（key/scroll）双双违反
- ADR-021（floor + 硬诊断）——「绝不静默降级」在网关层（CLI/编辑器）断链

**Previous Reviews**:
- `cross-target-feature-gap-review-2026-07-31.md` —— 本评审的前身：5 专家发现 OmniView-first 架构问题，催生 ADR-052；本评审验证语义时代后遗留与新增缺口

**Referenced Documents**:
- 语法规范: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`
- 设计文档: `docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md`
- 用户文档: `docs/omniview/*.md`（16 篇）
- deferrals: `.architecture/deferrals.md`（#17-#33）

---

## 附：各专家评审摘要（来源数据）

### Compiler Architect (Dr. Lin Wei) — 编译器/语言架构
**验证**: 通读 semantic/IR/codegen 全链路 + 实测编译 learn-omniview 与 refs.omni fixture。
- HIGH-1: `value={sig}` 语义矛盾 → React 单向、Flutter 双向（同一 fixture 数据流相反）；文档三方矛盾。Fix: 定语义 → 统一两目标 → 同步 spec/06 → cross-target fixture。S
- HIGH-2: 主题切换 React/Flutter 静默失效（实测 `theme = "dark"` → "Undeclared variable"；demo 路径无 data-theme 写入）；useThemeStore.ts basename 冲突。Fix: 实现语句或 floor 移除 + 硬错误。M
- HIGH-3: Flutter `@for` 忽略 key_expr + [scroll]（唯一实现 = 无条件 ListView.builder，view.rs:1087-1119）。Fix: scroll_hint 分派 + ValueKey + KeyedSubtree。M
- HIGH-4: capturePointer/releasePointer Flutter 静默 no-op（spec §10.3 承诺 error）。Fix: ref 扫描补诊断。S
- MEDIUM-5: 内联动画体实际已实现（lower_inline_motions 已接线），spec 三处标「未实现」。Fix: spec 区分 map 形式已实现/完整体未实现。S
- MEDIUM-6: `_syncSemanticBindings` 每次 rebuild 无条件重放（React 有 deps、Flutter 没有）。Fix: 快照比较。M
- MEDIUM-7: `@if mode:"fade"` Flutter 静默无效果（无降级注释）。Fix: AnimatedSwitcher 或降级注释。S
- MEDIUM-8: Flutter design 不可映射值静默跳过；spec §10.3 诊断表（ZIndex/grid/float/cursor）零实现。Fix: warnings 通道。M
- MEDIUM-9: 类型化 slot 三目标分叉且与 spec 不符（React 把约束类型当 prop 类型而非 render prop）。Fix: render-prop 映射或删除类型约束。L
- MEDIUM-10: CI fixture 矩阵不含 match/@for key/theme/typed slots/capturePointer；learn-omniview Flutter 输出不可编译（match #30）而无门禁。Fix: 纳入门禁先清 #30。M
- LOW-11: scrollTo 双轴 Flutter 静默丢 x 轴。LOW-12: steps() spec error vs 实现 warning。
- 建议: 立即 S/M 项 1-3；短期 4-8；长期 typed slot + tag→Flutter widget 契约文档。

### TypeScript & React Specialist (Emma Rodriguez) — React 地道性
**验证**: HEAD 编译器实测 `omnic build -t react`（157 文件，exit 0）+ harness 内 `tsc --noEmit`（60+ 错误）。
- CRITICAL C1: 组件局部函数被发射遍历遗漏（collect_called_fns_from_view 跳过 Interpolation/el.props/inline_style，function.rs:200-249）——DataTable.tsx 调 6 个未声明函数（getSortIcon/getScoreColor/getActiveRows/getGradeForScore/getBonusTotal），TS2304 ×6 + 运行时 ReferenceError。Fix: 遍历补全（S）
- CRITICAL C2: style `$call`/`$signal` JSON 通道把任意 `obj.field` 篡改为 `$signal: "obj_field"`（lower/expr.rs:557-565）→ DataTable.tsx:140 `getScoreColor(row_score)`（row_score 未定义；OmniView 目标同样坏）。Fix: 仅真实信号用下划线形式 + format_style_value_expr 处理字段访问 JSON。S-M
- HIGH H1: @for 无 key → `key={String(item)}` 对象项恒 "[object Object]"（view.rs:785-792，6 处生成产物验证）。Fix: 索引 key + 注释或语义诊断要求 key=。S
- HIGH H2: 整 store 订阅 + 生成好的 selector hooks 死代码（mod.rs:394-403 vs 323-332）；useMemo 依赖 `[TodoStore]` 恒新。Fix: 切片 hooks。M
- HIGH H3: 声称的门禁不存在——react-test-app 锁 React 18.3 + Zustand 4.5、strict: false、无 eslint、仓库无 tsc 脚本；Playwright 末次 180/180 failed。Fix: 真实门禁。M
- MEDIUM M1: array_sort comparator 双调用 → 比较器恒 0（stdlib/array.rs:263）——**Toggle Sort 不排序**。Fix: JS 目标直接发射比较器。S
- MEDIUM M2: `setX(x+n)` 缺函数式更新（stale closure）。M3: 66 处 `Object.assign(e, {})` 噪音。M4: 每 action 多个顺序 set()。M5: effect deps 来自声明而非 body 读取。M6: computed deps 不含 props。M7: 局部函数参数无类型（implicit any）。M8: `useState([])` → never[]（类型被丢弃）。
- LOW: 未用 React 默认导入、平凡值 useMemo、`__el_N`/`_M` 命名泄漏、`_props` 未用参数、React 19 组件 @ref 未测（有语义诊断，无静默破坏）。
- 裁决: 输出 shape-地道（JSX/hooks/受控输入/类型化 props 对）但**不编译、不类型检查、含静默功能 bug**——ADR-052/054 核心断言 HEAD 未兑现。

### Flutter Reviewer — Flutter 地道性
**验证**: 通读 7 个 Flutter codegen 模块 + 快照；零测试覆盖 @for。
- CRITICAL: @for 无条件 ListView.builder（view.rs:1087-1119），忽略 spec Column 默认 + scroll_hint + key_expr → 嵌套/非滚动上下文 unbounded viewport 崩溃；keyed 列表丢状态；零测试覆盖。
- HIGH-1: store 读取 (`ref.watch`) 在 itemBuilder/Builder 闭包内（build 阶段外，Riverpod 3.0 禁止）；迭代表达式每帧 3 次求值；Builder 包装无意义。Fix: 提升为 build 局部变量。 
- HIGH-2: 语义绑定每次 rebuild 后重放（mod.rs:547-554 + ref.rs:616-682）——focus 被偷、scroll animateTo 重播；「与 React deps 对等」的注释虚假。Fix: 快照比较。
- HIGH-3: Classes/Variants 域静默 no-op（style_classes 从未被读；`class:active={cond}` → 空 BoxDecoration() TODO 桩，view.rs:806-843）；AppDesign 工厂与 ThemeExtension 全死代码。Fix: 解析 style_classes 或诚实警告。
- HIGH-4: prop 值不类型转换（tests.rs:4331 断言 `fontWeight: 'bold'` 字符串为正确输出；`Image.network(image: 'x')` 错误命名参数；`Icon()` 缺必填参数）——三输入直接破坏 dart analyze。Fix: per-widget 转换层。
- HIGH-5: capturePointer/releasePointer 静默变 `/* TODO */` 注释（ref.rs:983-990）；表达式位置产生非法 Dart。Fix: error 诊断 + UnsupportedError throw。
- HIGH-6: motion color 动画用 DefaultTextStyle 整体替换继承样式（motion.rs:450-456）——动画期间文本全变默认排版。Fix: copyWith 继承样式。
- MEDIUM: StateProvider+legacy.dart 违反 Riverpod 3.0 声明（design.rs:66-68,850）；无 const 传播；Fragment→Column 静默改变布局语义；event 字段提取 10+ 静默桩（含 debugName 当数据）；`_Omni*` helper 每文件重复 + 字符串型运行时配置 + 60fps 硬编码；store state 缺 ==/hashCode、copyWith 无法清空 nullable；`ref.measure()` 裸 RenderBox cast（TypeError 风险）；steps() 降级与 spec error 不符。
- LOW: MediaQuery.of(context).size 被 flutter analyze 标记、TickerProvider 过度应用、默认 Container 包装、AnimatedSwitcher 硬编码 300ms。
- 裁决: **BLOCK**——CRITICAL @for + 6 个 HIGH 修复前不能声称生产级地道输出或可靠 dart analyze 门禁。

### Domain Expert — 语义准确性
- HIGH-1: 主题切换语义破碎——一个概念三个名字三种机制一种不存在的语法；floor ✅ 失实；「编译干净、点主题不切换」被只验编译的门禁掩盖。Fix: 收敛一个语义全目标接通（或 floor 降级 + 诚实注释）。
- HIGH-2: 表单值绑定三表面（bind:value 33 处事实标准 / value={sig} 0 处 + 要求 @ref / 隐式 auto-bind 需 `value={""}` 反模式规避——FormPage.omni:131-139）。Fix: 收敛 bind:value 为准。
- MEDIUM-1: capturePointer(id) 参数是 DOM 形状（pointerId）；demo 被迫编造 id=1。Fix: 去 id 参数。
- MEDIUM-2: a11y 语义缺失——aria-label 透传（DOM 词汇）；Flutter 上 warning 后丢弃（flutter/view.rs:478-482），alt→semanticLabel 却已映射（边界画在实现便利处）；6 处 demo a11y 属性 Flutter 全失效。Fix: aria-label → semanticLabel 最小映射 + a11y 域语义化。
- MEDIUM-3: 动画白名单边界是「已实现集」非「语义集」——borderRadius/fontSize 三目标均可动画（WAAPI 原生 + TextStyle.lerp + BorderRadiusTween）却被 ADR-053 D3 排除，排除理由事实错误；同表放行 6 个 Flutter 只 warning 的属性。Fix: 补两属性或修正理由文本。
- MEDIUM-4: @scroll/@drag 失去控制维度（start/end/scrub、constraints/elastic/threshold），设计文档仍教授不解析语法。Fix: 文档标注 + 中期重新实现。
- LOW-1: paused 静默接受不生效。LOW-2: 死令牌积累（@native/@fallback/@error/theme 保留 + 遗留 @slot(name)/slot="name"）。LOW-3: BNF 遗漏已实现语法（variant base/&::）。LOW-4: scrollTo 平滑度各目标默认不同无契约。
- 建议: 立即主题收敛 + BNF 补齐 + paused warning + aria-label 映射；短期值绑定收敛 + a11y 语义化 + 白名单决策；长期 @scroll/@drag 维度重实现。

### Pragmatic Enforcer — YAGNI
**数据**: 92 `.omni` 文件使用普查——9 域全有真实消费者（@enter 73、class: 1384 命中、signal 170、computed 44、match 19）。
- 总体: 语义 DSL **必要且充分**（语义前证据：Flutter 6 CRITICAL、stale stores、design 零文件）；非过度工程；~10% 表面积零消费者。
- MEDIUM-1: `theme = "dark"` 文档虚构（spec:509-517 展示不存在的生成代码）。Fix: 删除节 + 文档化实际机制（ThemeStore.mode + data-theme 订阅）。
- MEDIUM-2: `@native` 死 token（lexer 有、parser/codegen 零）。Fix: 删除。
- MEDIUM-3: `RefOpIR::SetValue` 零消费者（11→10 ops 简化 dispatch 表 + 三后端 arm ~150 行）。Fix: 删除。
- LOW-1: test-id 零使用 + spec 缺席。LOW-2: scroll-x 全后端实现零展示（7 行 demo 或删除）。LOW-3: 5/13 token 类型零使用（可接受债）。LOW-4: typed slots 声明侧文档（demo 自认）。
- 评分: Necessity 8/10, Complexity 5/10, Ratio 0.63 ✅, **Approve with deletions**。
- 诚实结论: 语义 DSL 是生成地道 React/Flutter 的必要且充分条件——必要因为 OmniView-first 翻译已被证明失败，充分因为 CI 门禁机器验证生成代码；剩余 YAGNI 债 = 文档虚构 + 零消费者 ops，ADR-054 P5 已授权删除。

### Maintainability Expert — 可维护性
- HIGH-1: store 名 vs fn 参数遮蔽修复只落 1/3 后端（242a5e53 只修 OmniView；React/Flutter 仍 `state.items` 错误发射，无诊断）。Fix: 排除 action 参数名（S）或系统级修复。
- HIGH-2: 绑定解析策略每后端重实现（IdentifierExpr 只带名字；omniview name_resolver + react renames + flutter StoreActionExprFormatter）；最近 4 个修复 commit 全触此类代码。Fix(L): IR 加 BindingRef 注解。
- MEDIUM-3: OmniView 双语句发射器（emit_statement vs stmt_to_js_inline 覆盖分歧 + buffer-swap hack）。MEDIUM-4: 4 个语义 pass 从 ir::lower 调用 + validate_ref_ops 实为改写 pass 命名错误。MEDIUM-5: 3 个 P0 修复（FnRef 扫描/fn_locals/DCE CallExpr）零回归测试。MEDIUM-6: 超大文件（flutter/design.rs 2458、lower/mod.rs 2252、resolve.rs 1080）。LOW-7: OmniView unhandled StatementIR 仅 warning + unreachable_patterns 静音。LOW-8: lib.rs.orig 死文件。
- 亮点: shared easing/event schema/design CSS 前载正确；ComponentScope 纪律（09c18415/a448b623 scope 不泄漏）；参数化矩阵 6 fixture × 3 后端；parser 组织健康。
- 风险: 修复漂移复合（遮蔽 bug 已 2/3 分叉）；新 IR 变体静默带洞上线。

### Rust Expert — Rust 实现质量
- HIGH H1: 增量缓存静默丢弃未变更文件诊断（analyse_multi_cached，analyse.rs:388-622——CachedFile.diagnostics 写入后从不读取；success 可为 true 而错误存在；poiesis 编辑器主路径；零测试）。Fix: 合并缓存诊断 + 回归测试。S
- HIGH H2: `compile_never_panics` 空测试（property_tests.rs:76-80——catch_unwind 结果被丢弃；且 `compile` 不含任何 codegen 后端）。Fix: 断言 + 增加 codegen fuzz 目标。S
- MEDIUM M1: WASM 边界契约漂移（10 个 json! 信封形状不一、3 套并行 Diagnostic 序列化器、ffi.rs:118 硬编码 "Error" 大小写、丢 end_line/end_col）。M2: resolve_styles_multi_cached 以零警告为 success（一处 lint 致整个画布空白）。M3: Python/Vue BlockExpr 语句体静默丢弃（shared 只有 debug_assert）。M4: compile_multi_to_omniview 每次调用序列化整个缓存（编辑热路径）。M5: OmniCompiler::diagnostics/analyse_dependencies 逐文件孤立编译（跨文件 import 永不解析）。
- LOW: escape_literal_str 6 遍替换、fast_hash 声称快速实为 SipHash、format_json_value 对象键未转义、C-FFI NUL 处理、超大文件。
- 风险: 编辑器诊断不可靠（用户追幽灵 bug）；codegen panic 未测（WASM panic=abort = 硬崩溃）；M3 = 项目明确设计对抗的「静默错误码」类。

### Performance Specialist — 生成代码性能
- HIGH-1: React 整 store 订阅（selector hooks 生成未用；useMemo 依赖恒新对象）——Flutter 已做对（select()），对等性差距。Fix: 切片 hooks。M
- HIGH-2: React @for 默认 key `String(item)` → 对象项恒同 key（重建塌缩 + 每渲染一次分配）。Fix: 索引 key + 警告或语义 lint。S-M
- HIGH-3: computed 依赖不深入闭包（getter_expr_contains_ident 无 ClosureExpr 分支）——build/FilterBar.tsx:14-16 实测：filteredFrameworks 依赖 searchText 仅在闭包内 → **输入搜索后结果永不更新**。Fix: 一行。S
- MEDIUM-1: Flutter 每 motion 名共享一个 controller（hover 一个播两个）+ 每组件创建所有模块 motion 的 controller。Fix: 按元素实例 + 使用图剪枝。M
- MEDIUM-2: Flutter @for 无条件 ListView.builder（同 flutter CRITICAL）。MEDIUM-3: 滚动驱动动画无进度相等守卫 + 每元素一个监听器。MEDIUM-4: 无 const 注入（字面子树每 tick 重分配）。
- LOW: React @for empty 双求值 iterable、_M 表每渲染重分配、反转 keyframes 每事件重算、动态 token 文档级样式重算。
- 亮点: Flutter 订阅粒度精确、effect deps 双端收敛良好、motion 不逐帧重渲染内容、controller/listener dispose 完整、IR DCE 真实收益。
- 语法层面: React.memo 不可表达（低紧迫）；Flutter const 可达于编译器推断；ADR-037 批处理按数据门控姿态仍然正确。

### Implementation Strategist — 实施策略
- 总览: 系统正处于**语法删除最便宜状态**（92 文件演示迁移完成、零旧语法残留）——「该删未删」现在是零使用量删除。
- HIGH-1: 展示项目 Flutter 输出无 CI 门禁（generated_code_validation.rs:46-47 只编译 fixture；ci.yml e2e 只到 React）——6-CRITICAL 周期同源盲区。Fix: 门禁扩展（M），**必须先于一切 codegen 修复**。
- HIGH-2: 无机制阻止发现累积成第二次 big-bang——每条发现必须四选一处置（fix-now/hard-diagnostic/delete/defer-with-trigger），结束零悬空。S
- MEDIUM-1: `paused` 兼容解析是 parse-and-ignore 活标本（motion.rs:370 解析后丢弃；spec:209 文档化）——现在删最便宜；同类残留并入同 commit。S
- MEDIUM-2: motion IR 缺 `#[non_exhaustive]`（ir/types_motion.rs 无；types.rs 4 处有）——motion 变更 = 9 后端 match 臂全破。Fix: 触碰时同 commit 补标。
- LOW-1: --all-features 测试失败（#22，~14 处机械修复）。LOW-2: deferral 台账滞后（#17 已修标 open）。
- 建议: 逐发现处置规则 + 展示项目 Flutter 门禁先行 + paused 删除 + 台账同步；短期按类别原子修复（门禁护航每笔）；长期 motion non_exhaustive + 文档。

### Systems Architect — 系统一致性
- HIGH-1: React/Vue/Dioxus 隐藏依赖 OmniView 内部 is_dangerous_url（react/view.rs:16、vue/template.rs:32、dioxus/view.rs:34）——`cargo check --no-default-features --features codegen-react` 实测 E0432；per-backend feature 门禁失效。Fix: 移 shared + feature 矩阵 job。S-M
- HIGH-2: 诊断完整性三网关不一致——run_pipeline 无 lint 阶段（单文件路径漏全部 10 条 lint）；OmniViewResult 无 warnings 字段（安全警告与编译器 bug 标记被丢弃）；CLI 三命令只读 errors；编辑器单文件 vs 多文件模式诊断集不同。Fix: lint 提升共享阶段 + warnings 字段 + CLI 打印。M
- MEDIUM-3: Flutter 静默忽略 @for key_expr（无诊断无注释；React/OmniView 都实现）。MEDIUM-4: 跨后端参数化矩阵无 @if/@for fixture（deferral #7 部分实施）。MEDIUM-5: 治理滞后（#17/#22 代码已修台账 open；spec Draft；mod.rs:34-39 陈旧注释；lib.rs.orig 已入库）。
- LOW-6: compile_to_target_json vs compile_multi_to_target JSON 契约分叉。
- 亮点: 三层依赖方向干净（实测）；单一 Diagnostic 全链路；非 P0 design 硬错误门；codegen-validate 已从白名单改全量枚举（fixture 内）；ts-rs 新鲜度检查；634 all-backends 测试全绿。
- 建议: is_dangerous_url 移位 + CLI warnings + 台账清理（立即）；lint 共享化 + Flutter key_expr + control_flow fixture（短期）；后端能力声明替代 per-arm 手写门（长期）。

### Security Specialist — 安全
- **CRITICAL**: React className 代码注入（已端到端验证）——format_class_ref（react/design.rs:151-157）对非 design 类名生成 `styles['{name}']` 不转义；`class: "x',alert(document.cookie),'"` → 合法 TSX `styles['x',alert(document.cookie),'']`；esbuild/tsc 通过、Node 执行触发；现有 tsc 门禁无法发现。Fix: escape_literal_str(name, '\'') + lowering 字符集校验 + 回归测试（断言不含载荷模式）。CRITICAL
- MEDIUM: `compile_never_panics` 吞 panic 无断言（与 rust H2 独立印证）。MEDIUM: React 运行时 sanitizeCss 与 Rust/OmniView 消毒器漂移（缺 hex-escape 归一化/data: 模式/url() 上下文）——ADR-026 双层设计单点破裂。Fix: 单一事实来源 + 一致性测试。
- LOW: OmniView JS 消毒器缺 CR/LF/TAB 预剥离；CSS 消毒器 data: 子类型封杀不完整；WASM 入口大小限制不一致（parse_code_block/analyse_multi_cached/upsert_source 无上限）；Dioxus style_classes 死代码注入模式（当前不可达）。
- 亮点: CSS 注入纵深防御（编译期 sanitize_css_value + OmniView 运行时 __omni_sanitizeCss + 标识符 [a-zA-Z0-9_-] 按构造封闭）；URL 检查四后端一致复用；parser 递归深度守卫；FFI NUL/UTF-8 防御。

### AI Engineer — AI 时代定位
- MEDIUM-1: spec 示例省略强制 view{} 块（parser 强制，component.rs:154-162）——LLM 按权威文档生成系统性不可编译；报错无提示。Fix: 补示例 + 错误提示 hint。S
- MEDIUM-2: 生成代码无 .omni 源位置引用（仅文件头注释；flutter source_map 恒空）——AI/人改生成代码后被再生成静默覆盖，无契约。Fix: `// [omni] file:line` 注释 + 再生成契约文档。M
- MEDIUM-3: Flutter #17 门禁白名单 → LLM 迭代中「编译错误来自下游而非编译器诊断」时无法区分自己的错与编译器的错（信任崩塌点）。Fix: 全文件 dart analyze。
- LOW-1: 设计文档生命周期示例用废弃语法（mount{} 无 @）。LOW-2: 无单页速查表（LLM few-shot 需「每域一例」紧凑形态）。
- 亮点: 意图语法 + 有界白名单 = LLM 高成功率先验设计；硬诊断 + 迁移提示 = agentic 纠错循环骨架（validate_ref_ops.rs:85-104 的替代建议表 + omnic check 结构化 JSON）；三列式文档 = 高效 LLM 上下文格式；可执行文档模型（5300 行验证过的 few-shot 语料）多数 DSL 项目不具备。
- 建议: 立即 spec 示例修复 + 提示 hint；短期源注释 + cheatsheet + 门禁；长期 tree-sitter 化 BNF + LLM 生成正确性基准。

### Documentation Expert (doc-updater) — 文档工程
**验证**: 逐段对照 parser + 实测 `omnic build -t omniview`（exit 0，构建干净）。
- HIGH-1: spec §6.3/§9.3 教授解析器不接受的箭头闭包（`item => ...`；parser 只认管道 |x|）且与 03-expressions 自身教学冲突；`.count()` 在 OmniView 也运行时 TypeError。Fix: 全规范扫描 => 改写。
- HIGH-2: 06-templates.md:36 教 `{name}`（parser 只认 `${expr}`）。Fix: 一行。
- HIGH-3: README/01 教不存在的 `omnic check`（实际 analyse）+ run 描述错误（无热更新）+ 版本 0.1.0 vs 实际 0.2.0。Fix: 重写命令节。
- HIGH-4: 10-design-system.md:196-197 「design 仅 OmniView 支持 + warning」——React/Flutter 完整实现，语义时代前残留。Fix: 删除/改写。
- MEDIUM-1: 15-grammar-view 内部 4 处自相矛盾（event_decl 示例缺类型、@for BNF 缺 key_expr/scroll_hint、import "omniview" 带引号、`$counter` 不存在的存取语法）。MEDIUM-2: spec §6.2 store_member BNF 滞后（顶层 store 不接受 fn；action 缺返回类型/async）。MEDIUM-3: 08-stores 教 `.filter()`（不在 stdlib；Flutter 目标生成 Dart `items.filter()` 无法编译）。MEDIUM-4: 11-motion「GSAP 兼容层」残留（实际 WAAPI）。MEDIUM-5: 无用户向平台映射文档；README 核心概念表未标注 OmniView 目标。
- LOW-1: spec 状态 Draft。LOW-2: learn-omniview build 目录混入旧目标产物。
- 建议: 立即 6 项小改（合计半天）；短期语法教学审计（fenced 代码块逐条对 parser 验收，仿 #24/#25）+ 用户向平台映射表；长期文档编译 CI 门禁 + spec 机器可读同步状态。

---

## Appendix

### Review Methodology

13 名专家独立评审（compiler-architect / typescript-react-specialist / flutter-reviewer / domain-expert / pragmatic-enforcer / maintainability-expert / rust-expert / performance-specialist / implementation-strategist / systems-architect / security-specialist / ai-engineer / doc-updater），多名专家实测编译验证（React 157 文件 + tsc、Flutter 全量输出、攻击载荷复现、feature 组合 cargo check、all-backends 634 tests）。

**Pragmatic Mode**: Balanced
- Complexity ratio target: <1.5
- Pragmatic Enforcer 评分：Necessity 8/10, Complexity 5/10, Ratio 0.63 ✅

**Review Complete**
