# ADR-062: 语义 DSL v3 修复轮（15 findings 处置 + 8 阶段 fix round + 终验收口）

## Status

Implemented

## Context

v2 对齐评审（`.architecture/reviews/feature-omniview-semantic-syntax-compilability-v2.md`，2026-08-03）给出 15 条 Consolidated Findings（2 CRITICAL + 9 HIGH + 4 MEDIUM 级）：编译级验证网真实但「编译通过 ≠ 行为正确」——React 两标签页运行时卸载（BUG-A/BUG-C）、Flutter `TodoStore.add()` 运行时 TypeError、3 个已端到端验证的注入点、8 个「静默降级」独立实例、门禁度量偏差（React 287 条豁免 / Flutter 1,004 条 warnings 零断言 / OmniView 套件未入 CI）、文档漂移 10+ 处。

用户据此批准「单一总计划」：`docs/superpowers/plans/2026-08-03-semantic-syntax-v3-fixes.md`（8 阶段 68 任务 + 7.5b 展示项目重构子任务 + 终验 8.1），用户决策 D-A/B/C 固化、计划内裁决 D-D..I。2026-08-03 至 08-05 以 subagent-driven development 执行完毕（含 7.5b-K1 事件发射碰撞修复、7.5b-LS 展示项目文件移动）。终验（Task 8.1a）8 门禁 7 绿 1 环境阻断（poiesis 文件锁先例），8 项验收矩阵 PASS×4 / 部分×3 / FAIL×1（DONE_WITH_CONCERNS）；Task 8.1b 修复 2 个 CI 阻断项（D1 spec 路径、D2 clippy）并完成治理收口。

本 ADR 记录该修复轮的 9 项计划决策 + 1 项实现期新裁决（K1），作为 v2 评审闭环的决策记录（评审 Follow-up 承诺的 ADR 决策候选 4 项全部在此裁决）。

## Decision Drivers

* **v2 评审 15 findings 必须逐条处置**：零悬空项（fix-now / defer-with-trigger 二选一），评审结束即闭环
* **门禁是行为正确性的护栏而非终点**：「GATE PASSED」只证明编译；运行时缺陷（TypeError/标签页卸载/注入执行）必须由运行时测试与注入语料抓取
* **修复顺序按「注入 → 行为 → 门禁度量 → 文档」**（评审 Accountability 承诺），门禁先行（Phase 0）避免修复无度量
* **诚实降级 > 静默错误**：无法完整实现时先落诊断（D-G/D-H/2.2 先例），长期项注册触发条件
* **真实文件移动后文档路径必须由门禁守护**（D1 教训：7.5b-S1 移动 11 文件后 spec 路径陈旧，CI docs-gate 接线使 push 必红——门禁在漂移后首跑即抓）

## Decision

**v3 修复轮完成**：15 findings 处置（14 条 fix-now + 1 条部分）、9 项计划决策执行、1 项实现期新裁决、终验阻断项修复。计划决策 D-A..I 与实现期裁决 K1 记录如下：

### D-A：inline store 两步走——React 先硬诊断，IR 完整实现在 IR wave

React 目标先 E_CODEGEN 硬诊断 + 迁移提示（2.2，`86f820be`）；完整实现（inline store → 模块级 Zustand store + 切片订阅，CartStore.addItem 可执行）与 #36/#49 合并为 Phase 5 IR wave（5.2，`a9526faf`）。**终态：硬诊断与完整实现均已落地，非替代关系。**

### D-B：未知事件名 → warning 级（非 error）+ 最近合法事件建议

validate_a11y 先例：未知事件名产生 warning 而非 error（4.2，`556a489e`+`d167215c`）。依据：事件域别名单表已收敛，error 级会阻断 LLM 迭代闭环；warning + 最近合法事件名建议（validate_a11y 先例）在信任与可迭代间取平衡。「No issues found」的静默现状（评审 Trade-off 4 双专家一致不可接受）被消除。

### D-C：单一总计划，8 阶段顺序执行

用户决策：不拆多个修复轮；每阶段结束零「open-undispositioned」发现（逐任务 review Approved 记录于 progress ledger）。

### D-D：aria-* 原生属性 → 硬诊断 + 迁移到 a11y 块；alt= 明确为普通 DOM 属性

`aria-*` 前缀硬诊断（每属性一个 Error + a11y 五面迁移提示——web/Flutter 不对称解释）；`alt=` 保留为普通 DOM 属性（透传 + Flutter semanticLabel 映射），文档化边界（4.4，`62c473b2`；a11y 语义表面唯一化）。

### D-E：filter/map/count 语言方法 prove-or-retire → **PROVE 裁决**

迁移 4 处 demo 调用到语言方法 + 三目标门禁验证后：**语言方法保留**（行为等价实证——React 迁移行 parens-only、codegen 恒加括号）；stdlib `array_filter`/`array_map` 走降级路径（deprecated warning 通道）（4.6，`ac7fec51`+`63ed226c`）。语言方法进 BNF 的试点要求（v3 语法生命周期制度）由此满足——0/93 消费者直接进 BNF 的失败模式不可复现。

### D-F：@default-theme ↔ @theme signal initial 双声明 → 一致性验证

mismatch = error（消息含两边值 + file:line:col；per-design 逐一比较——每个含 @themes 的 design 各自挂同一 theme_signal 语义自洽）（4.7，`d12039c3`）。

### D-G：Flutter 多主题（>2）塌缩 → 编译 warning（诚实降级）

>2 主题编译 warning「Flutter 仅支持 light/dark 两态；其余主题不可达」（3.6，`5dc4d322`+`3bf6496f`——同任务实现主题四态完整接线 themeModeProvider 全主题 switch + MaterialApp(theme: ref)，塌缩场景仅剩 >2 自定义主题的诚实降级）。

### D-H：motion-in-@for → 编译诊断（双目标），逐项实现为长期项

`validate_motion_triggers` ForDirective 分支 WMotion warning（每 @for 一条指向 for span、嵌套去重、不扫 @empty）（4.9，`491857d6`）；逐项实例化实现（React 共享 ref 只动最后项 / Flutter 全项同播的语义修复）注册 deferral **#83**。

### D-I：BUG-A 两步走——let 廉价路径 + LetStmtIR.mut 完整落地

立即 `const`→`let` 廉价路径 + un-fixme（2.1，`edb19786`——eslint 无 prefer-const、门禁不破、行为立即正确）；IR `LetStmtIR.mut` 字段在 Phase 5 完整落地（5.1，`db68d812`——is_mut 字段 + lowering 线程化 + 三后端发射 React mut→let/immut→const、Flutter mut→var/immut→final + prefer-const re-enable，~64 构造点更新，TS2588 零残留）。**终态：廉价路径作为首道止损保留，IR 字段为最终修复——评审「廉价路径是否长期保留」裁决为双层共存。**

### K1（实现期新裁决）：事件发射解析组件化——bare 事件名共享表 last-wins 缺陷 → 组件作用域优先 + 全局回退

7.5b-K1（`e604cf4a`）：两个组件声明同名事件（TabBar/AboutPage 同 `onSwitch`）时，bare 事件名共享重命名表 last-wins——后声明组件的前缀发射器覆盖先声明者，TabBar 每个按钮分派到 `AboutPage_onSwitch`（从未挂载的发射器），`App_switchTab` 永不触发，标签切换全失效（33/45 浏览器测试单根因）。**裁决**：`emit`/事件调用解析组件化——`{scope.current_component}_{event}` 优先（权威组件事件表，无 last-wins 覆盖），miss 回退全局重命名表（跨组件 emit 与模块级函数既有行为不变）。两处解析点（`emit` 关键字 EventEmitStmtIR 路径 + CallExpr 调用形式——真实 TabBar 走后者）共享 `component_event_prefixed` 辅助。4 项回归测试（同名单事件各自解析、TabBar 展示项目级形态、CodeFlowMidChain 防污染、onNavigate 同族）。**关联残留**：`stmt_to_js_inline` 无 EventEmitStmtIR 臂（BlockExpr 内 emit 降级注释，pre-existing 于 09c18415）——登记 deferral **#78**。

### 8.1b 收口裁决（终验阻断项）

- **D1**：syntax-lifecycle spec 陈旧路径修复（`components/array_ext.omni` → `lib/`，全 specs 仅此 1 处 11 文件族引用）——门禁绿。
- **D2**：clippy 3 警告修复——`component_html.rs:7` 未用导入 `FxHashSet`（**provenance 修正：8.1a 称「均 K1 引入」不实**——该导入自 `76ac51f5`（v2 5.8 slot 工作，2026-08-01）引入、使用于后续重构（`5251061a` slot materialize 共享化）后移除，K1 未触碰此文件；`component_html.rs:181` 可折叠 if 自 `8ec10bbde`/`a16ed7051`（2026-07-30）——同样非 K1；仅 `tests_regression/events.rs:18` 未用导入为 K1 创建文件时引入）、`:181` 可折叠 if、`tests_regression/events.rs:18` 未用导入。**额外发现**：`-D warnings` 同目标暴露 floor_matrix.rs 模块文档第 4 个阻断项（doc 行首 `+ ` 被 CommonMark 解析为列表标记 → 连续行未缩进，rustdoc 报错）——一并修复。provenance 结论：D2 中仅 1/3 为 K1 引入，2/3 为 v2 时代存量（预登记 5.8 期吻合——「5.8 曾预登记」属实）。

## Consequences

### Positive

- **v2 评审闭环**：15 findings 14 条 fix-now 完成 + 1 条部分（motion-in-@for 逐项实现 defer #83）；7 Success Metrics 逐项实测达成（8.1a 数字）；ADR 决策候选 4 项全部裁决
- **运行时验证网建立**：Flutter widget test harness（23 文件，TodoStore.add 冒烟）、react 41/1/0 零 runtime error、omniview 44/1/0 入 CI（#60 关闭）
- **注入面按语料闭合**：119 tests（注入语料 29 插入点 + 覆盖地图）断言对生成代码
- **豁免 287 → 29**（React 门禁从「类别」到「实例计数」）；**warnings 1004 → 874**（断言 cap 879，单调下降 4 段连续）
- **静默降级归零**（M4 PASS）：states:/on:/未知事件名/aria-*/block-expr/inline-store/多主题全部产生诊断（122 条诊断类测试）
- **D1 修复证明门禁价值**：spec 路径漂移在 push 前被抓（syntax-lifecycle 接线 CI docs-gate）

### Negative

- **M3 目标未达**：Flutter warnings 874 > 计划目标 <300（monotonic 下降但燃尽缺口按类存续——ECodegen 873 为主，登记 #81，编排层裁决继续燃尽或更新触发条件）
- **K1 残留**：stmt_to_js_inline 缺 EventEmitStmtIR 臂（#78）；test-results-omniview.json 未入 .gitignore（#79）
- **设计层机制缺口**：生命周期文档路径与文件移动无强制同步机制（#82）；golden 镜像 8 份陈旧拷贝（#80）
- **poiesis workspace 测试环境阻断**（D4 先例延续，用户进程运行中）——4 crate 单独全跑 1831 passed

### Neutral

- 7.5b-LS 展示项目重构（11 纯函数文件 components/→lib/）后 spec/镜像路径维护成本显性化（D1/#80/#82 均为其直接产物）
- 诊断语言中文政策延续（v3 决策均以中文 verbatim 消息落地，与 ADR-056 政策一致）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 大但有序——8 阶段顺序执行（0 门禁 → 1 注入 → 2 React → 3 Flutter → 4 静默清零 → 5 IR wave → 6 文档 → 7 设计层制度 → 7.5 重构 → 8 终验），每阶段独立 commit 范围可回退；治理层（review/deferrals/ADR）随 8.1b 单 commit 收口。

**Affected Components**:
- 安全：注入语料/转义/编译期拒绝门（Phase 1）
- codegen 三后端：React（2.x/5.x）、Flutter（3.x/5.x）、OmniView（4.x + K1）
- IR：LetStmtIR.mut、ModuleIR.stores、StructIR 发射（Phase 5）
- 验证网：CI jobs（#60/all-backends/feature-matrix）、warnings 基线断言、widget harness
- 文档/制度：docs-gate 语义白名单、语法生命周期、词汇登记册、语义全集

**User Impact**: 三目标展示项目行为层修复（React 标签页可点、Flutter TodoStore.add 可执行、OmniView 事件解析正确）；用户区（poiesis/CodeFlowMidChain.omni）零触碰。

**Risk Mitigation**: 每任务 RED→GREEN + review Approved（68+ 任务全录）；终验 8 门禁 + 8 项矩阵独立复核；D1/D2 阻断项在 8.1b 修复后全量复验。

### Reversibility

**Reversibility Level**: 高——各阶段独立 commit 可回退；语法删除（@fallback/@error）为 v2 既有纪律延续（硬诊断已内建）；D-E prove 保留语言方法为可逆偏好（stdlib 降级路径未删）。

**Commitments Made**: 门禁-行为双轨验证是语义 DSL 时代的稳定性基础；组件化事件解析（K1）是事件域解析的契约（组件优先 + 全局回退）。

### Sequencing & Timing

**Prerequisites**: 门禁先行（Phase 0）——修复无度量不开始；注入（Phase 1）先于行为（Phase 2/3）——修复不能引入新注入面。

**Readiness Assessment**: 已完成——终验 8 门禁 7 绿 1 环境阻断；8.1b 后 syntax-lifecycle/clippy 全绿；工作树干净。

### Social Cost

**Learning Curve**: 低——决策沿 v2 既有模式（硬诊断/诚实降级/注册触发条件），无新抽象。

**Documentation Needs**: 本 ADR + review v3 处置记录 + deferrals 2026-08-05 节为闭环证据链；语法生命周期文档为 BNF 新增的唯一入口。

## Alternatives Considered

1. **不修复 D2 的 floor_matrix.rs 文档项，仅修 3 条预登记警告**——拒绝：`-D warnings` 下 rustdoc 项同为 CI 阻断（8.1a 因 lib 目标先失败未达该目标；修 3 条后第 4 条必现），一并修复才满足「clippy -D warnings = 0」验收。
2. **K1 事件解析修法 b（只修 EventEmitStmtIR 路径）**——拒绝（K1 报告实测）：真实 TabBar 走 CallExpr 形式（lowering 不重写为 EventEmitStmtIR），单修 stmt.rs 后 index.js 仍调用 `AboutPage_onSwitch`；必须双解析点 + 共享辅助。
3. **D-E retire（删除语言方法保留 stdlib）**——拒绝：三目标行为等价实证（React 迁移行 parens-only）后语言方法为地道形态；stdlib 走降级警告通道而非删除，兼顾既有语料。
4. **D-B error 级（ai 专家原建议）**——拒绝：warning + 建议在 LLM 迭代闭环与信任之间更优；error 级保留为未来触发条件（若 warning 被证明不足以阻止误用）。
5. **motion-in-@for 直接实现**——拒绝：逐项实例化跨双目标语义成本高且语料零需求；诊断先行（D-H）符合 YAGNI，长期项注册 #83。

## Validation

- 终验 8 门禁（8.1a 实测）：G1 Rust 1831 passed/0 failed（4 crate，poiesis 环境阻断先例）/ G2 verify-react PASS（豁免 29 条 0 stale）/ G3 react **41/1/0** / G4 omniview 5199 **44/1/0** / G5 flutter **23/23** / G6 docs-gate **247 blocks 0 failed** / G7 analyse 0 err/1 warn（用户区）/ G8 warnings 断言 cap 879 实测 **874**
- 8 项验收矩阵：PASS×4（M1 注入 119/0、M2 React 行为、M4 静默清零、M8 治理）/ 部分×3（M3 warnings <300 未达、M5/M7 lifecycle 脚本红——8.1b D1 修复后绿）/ FAIL×1（M6 clippy——8.1b D2 修复后 0）
- 8.1b 复验：`verify-syntax-lifecycle.py` 0 problems / `verify-docs.py --check` 247 blocks 0 failed / `cargo clippy --workspace --all-targets -- -D warnings` 0 错误 / `cargo test -p omnilang --features codegen-flutter` 全绿
- 处置闭环：review v3 处置记录（15 findings + 7 metrics 实测状态）/ deferrals #36/#37/#49/#50/#55/#60 关闭确认 + #78-#83 新登记 / #76 保持 open 复核

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 修复轮由 v2 评审 15 findings 驱动，全部沿既有模式（硬诊断/诚实降级/门禁-行为双轨/触发条件注册）展开；唯一新抽象为事件解析组件化（K1，共享辅助函数 + 双解析点）。

**Proposed Decision**: "v3 修复轮 8 阶段完成——15 findings 14 条 fix-now + 1 条 defer；D-A..I + K1 裁决记录；终验阻断项 D1/D2 修复"

**Necessity Assessment**: 10/10
- **Current need**: CI 阻断（D1 push 必红、D2 clippy -D warnings）+ 评审闭环（15 findings 零悬空承诺）
- **Future need**: ADR-062 是后续评审的判定基线（豁免 29 条/warnings 874/44+41 套件的度量口径）
- **Cost of waiting**: D1/D2 不修则 CI 永久红、v3 无法合并宣称完成
- **Evidence of need**: 8.1a 实测（FAIL×1/部分×3 + D1/D2 漂移清单）

**Complexity Assessment**: 3/10
- **Added complexity**: 零新依赖；D1 一行路径修复、D2 三处删除/折叠 + 1 处文档换行
- **Maintenance burden**: 门禁守护 spec 路径（D1 已修复即证明机制）；deferral 触发条件为后续燃尽的唯一依据
- **Learning curve**: 低
- **Dependencies introduced**: 零

**Alternative Analysis**: 不记录 ADR（备选 0）会让 D-A..I 裁决与 K1 新裁决失去唯一参照（评审 Follow-up 明确要求）；K1 修法 b 与 D-E retire 均被实测否决。

**Simpler Alternative Proposal**: ADR 可仅记录 K1 与 8.1b 收口——但 D-A..I 是评审 4 项 ADR 决策候选的正式裁决，合并记录成本低于逐项检索。

**Recommendation**: Approve

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 3/10
- **Ratio**: 0.3 *(Target: <1.5)*

**Overall Assessment**: 适当的收口治理——阻断项最小修复（D1 1 行 + D2 4 处）+ 评审闭环 + 决策记录；无过度设计。

## References

- 评审：`.architecture/reviews/feature-omniview-semantic-syntax-compilability-v2.md`（v3 处置记录节）
- 计划：`docs/superpowers/plans/2026-08-03-semantic-syntax-v3-fixes.md`（8 阶段 68 任务）+ `.superpowers/sdd/2026-08-03-semantic-syntax-v3-fixes/progress.md`（逐任务 commit 证据）
- 终验：`.superpowers/sdd/2026-08-03-semantic-syntax-v3-fixes/task-8.1a-acceptance.md` / `task-8.1a-report.md` / `task-8.1b-report.md`
- 关键 commits：`a0b11b63`（#60）、`db68d812`（D-I IR）、`a9526faf`（D-A IR）、`96c8e297`（#49）、`95699b6d`（#55）、`e604cf4a`（K1）、`f7b6f211`（7.5b-S1 文件移动）
- 上游：ADR-056（v2 实现完成——本修复轮动因）、ADR-054/055（设计理念与 @for 契约）
- 遗留登记：`.architecture/deferrals.md`（#36-#60 关闭确认 + #78-#83 新登记；#76 保持 open）
