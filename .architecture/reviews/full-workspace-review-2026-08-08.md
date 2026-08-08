# Architecture Review: 全工作空间评审（2026-08-08）

**Date**: 2026-08-08
**Review Type**: Component（全工作空间，跨 5 个 Rust crate + poiesis TS/Tauri + learn-omniview + docs + 审计脚本）
**Reviewers**: Systems Architect / Domain Expert / Security Specialist / Maintainability Expert / Performance Specialist / Implementation Strategist / AI Engineer / Pragmatic Enforcer / Rust Expert (Katya Volkov) / TS&React Specialist (Emma Rodriguez) / Compiler Architect (Dr. Lin Wei) / Documentation Expert (Samantha Quill) — 12 人并行评审
**Review Base**: HEAD `9bd3bf74`（f389d3f1 之后），2142 commits，单作者，74 ADR
**Pragmatic Mode**: balanced（ratio target <1.5）

---

## Executive Summary

OmniLang 是罕见的治理密度极高的单作者 monorepo：Rust 编译器（.omni DSL → 9 目标）、Tauri+React 可视化编辑器 poiesis、可执行文档 learn-omniview（102 个 .omni 文件）、以及由 5+ 个 Python 审计门禁 + 74 篇 ADR + deferral 台账构成的防漂移机制。**编译器核心质量优秀**：clippy/fmt/测试全绿（rust 专家实测 cargo check/clippy/test 全部通过）、unsafe 仅存于 2 个 FFI 边界文件、`# [non_exhaustive]`/直接写通道（ADR-070/071/073）/诚实降级（ADR-054）等设计决策经得起审视——12 位专家一致认为这些是合理复杂度而非过度设计。

但评审发现一个**正在发生的高危事件**：2026-08-08 事件意图化 Phase 1（@onClick/@onMouseEnter → 硬错误 @onPress/@onEnter）只完成了编译器侧迁移，**编辑器、文档、词汇登记册三个生态面全部未同步**——8/12 专家独立确认。当前后果：CI 双门禁红（verify-docs / verify-vocabulary）、cheatsheet 教错误语法、poiesis 事件面板主动提供会产生编译错误的旧事件名。这是「文档零失实」与「语义化即切换」治理体系自建立以来最严重的一次半迁移。

另有两项**安全性**发现需要立即决策：Security Specialist 证实 ADR-072 的沙箱移除安全理由（capabilities 限制调试窗口）对自定义 Tauri 命令不成立——调试窗口 eval 的编译产物与 `__TAURI_INTERNALS__` 同 realm，可调用全部 17 个自定义命令，命中即为全盘读写；以及 React（生产后端）缺少动态 URL 绑定运行时净化（OmniView 有 `__omni_sanitizeUrl`，React 无）。

Pragmatic Enforcer 结论：审计门禁机制本身是合理的（本次两个红门禁恰恰证明机制在工作）；但 ~1500 行已证实死面（C FFI 无 cdylib 生产者、Tauri server-compile 三命令零调用、cache_state 恒 None、FastAPI 时代 stub、golden mirror 76/96 漂移）应删除而非维护，门禁合并应暂缓（rule of three 未到）。

**Overall Assessment**: **Adequate**（编译器架构 Strong；生态同步与治理执行层有明确短板）

**Key Findings**:
1. 事件意图化 Phase 1 半迁移：编辑器事件面板/parser-view 归一化/cheatsheet/BNF spec/词汇登记册全部停留在旧名（8/12 专家独立确认）
2. ADR-072 信任边界失效：调试窗口编译产物可调用全部自定义 Tauri 命令（capabilities 不 gate 自定义命令），命中=全盘文件读写（CRITICAL）
3. 增量缓存 MAX_CACHE_ENTRIES=50 < 102 文件旗舰项目 → 实测击键分析 15.6× 退化（0.7ms → 10.9ms），且 LRU 驱逐会销毁失效边（正确性风险）；debug 窗口每 150ms 双编译（其中一个 uncached）
4. 跨文件语义验证缺口：validate_required_props / validate_effect_deps 仅在单文件内运行，imported 组件/effect 跨文件静默漏检
5. 死面堆积：C FFI（无 cdylib 生产者，CI 却上传不可能存在的产物）、Tauri server-compile、cache_state、FastAPI stub、golden mirror（76/96 漂移、零消费者）
6. 诊断通道重载：E0006 承载 44 处/15 域语义错误，suggestion 字段在最高价值的新诊断上为 None，建议列表含 4 个本身是硬错误的名字

**Critical Actions**:
- 立即收口 Phase 1：修 3 个红 doc 块 + verify-vocabulary 正则（fn-scoped，同 verify-semantic-universe 先例）+ 登记册域 1 三层模型 + poiesis 事件面板迁移 + DELETED_SYNTAX_PREFIXES 补事件族 + ADR-075
- 立即 gate 自定义 Tauri 命令（AppManifest::commands 或 window.label 守卫）——5 行修复恢复 ADR-072 自以为拥有的安全属性
- 立即修 MAX_CACHE_ENTRIES（≥ sources.len()）与 debug 窗口双编译（resolveStylesMultiCached + sourcesKey 短路）——实测收益最大

---

## System Overview

- **目标**：OmniLang——一个把自研 DSL（.omni）编译到 9 个目标（OmniView 原生 JS+CSS、React 19、Flutter/Dart，其余 feature-gated）的编译器 + 可视化编辑器生态。
- **工作空间构成**：
  - Rust workspace（resolver=2）：`crates/omnilang-core`（权威核心：lexer/parser/semantic/ir/sourcemap/stdlib，~299 .rs）、`omnilang`（薄 facade + 各 codegen 后端）、`omnic`（CLI）、`omnilang-wasm`（cdylib，浏览器内编译）、`poiesis/src-tauri`
  - `poiesis/`：Tauri 2 + React 19 + TS 编辑器（CodeFlowCanvas/ReactFlow、Monaco、Zustand、WASM 编译、debug 窗口直载编译产物 per ADR-072）
  - `test/projects/learn-omniview`：102 个 .omni 展示文件（docs-as-code）
  - `docs/`：omniview 16 篇 + superpowers（semantic-universe.md 域 7 登记、vocabulary-registry.md、specs/plans）
  - `scripts/`：verify-docs / verify-vocabulary / verify-semantic-universe / verify-syntax-lifecycle / verify-demo-coverage / verify-react.sh + exemption ledger
  - `.architecture/`：74 ADR、deferrals.md（87 项）、config（pragmatic balanced、TDD）
- **关键架构决策**：ADR-001 流水线 / ADR-003 目标优先级（3 活跃 + 描述性报错）/ ADR-005 双解析器（TS 编辑器自持 parser，刻意并行）/ ADR-021 跨目标对齐 / ADR-040 `#[non_exhaustive]` / ADR-052-054 语义 DSL 时代 / ADR-055 @for 事件委托 / ADR-070/071/073 直写通道 / ADR-072 debug 窗口直载 / ADR-074 元素指令域。
- **最近活动**：事件意图化 Phase 1（9 TDD 任务 10:02-13:52 全部提交，Task 8 结束）；82 commits 未推送 origin；15 个文件未提交（skill 清理 + 事件测试迁移）。

---

## Cross-Cutting Themes（3+ 专家独立确认）

### T1. 事件意图化 Phase 1 半迁移——编译器正确、生态三面未同步（8/12 专家）
- **Systems Architect**：计划无 TS 管线任务；`parser-view.ts:333-337` 归一化 @click→onClick（正是新硬错误名）；`EventsSection.tsx:20-35` 仍提供 onClick/onMouseEnter。
- **Implementation Strategist**：`componentProtocol.ts:52-53,70-71` 仍注册 onClick/onTap/onMouseEnter/onMouseLeave；onEnter/onLeave 完全缺席面板。
- **TS/React Specialist**：palette→emitter→Rust 语义门端到端断裂，用户绑定「点击」必得编译错误——编辑器主流程已坏。
- **Domain Expert / Documentation Expert / Maintainability**：verify-docs 3 块红（`00-cheatsheet.md:175`、`migration-v1-to-v2.md:185`、BNF spec `:829`）；`DELETED_SYNTAX_PREFIXES` 缺事件族 → fragment 块静默放行。
- **Domain Expert**：verify-vocabulary 正则全文件捕获 legacy_spelling 的 `Some("onClick")` 等 → `whitelist-only: ['onClick','onMouseEnter','onMouseLeave']` 红；登记册域 1 仍在教「click 是正确语义词汇」——与编译现实完全相反。
- **Compiler Architect**：`EVENT_NAME_SUGGESTIONS` 含 4 个本身是硬错误的名字（onClick/onTap/onMouseEnter/onMouseLeave），typo 建议把用户送进硬错误；legacy 硬错误跳过组件实例（`<Child @onClick={f}/>` 放行，KNOWBLIND SPOT 注释）。
- **Documentation Expert**：无 ADR-075 记录这次破坏性语言变更（设计 spec 状态仍「待审阅」）。
- **root cause**：计划 9 任务未覆盖 TS 管线/cheatsheet/登记册；flip-first-migrate-later 排序（Task 2 提交于 10:02，fixtures 10:40，demos 12:34）。

### T2. 安全：ADR-072 信任边界失效 + React URL 净化缺口（2 专家，1 CRITICAL）
- **Security Specialist**：Tauri 2 的 capabilities 不 gate 自定义命令（`build.rs` 无 `AppManifest::commands`）→ 调试窗口 eval 的编译产物可 invoke 全部 17 命令（read_file/write_file/open_project/project_export…）；drive root 是合法 project root → 全盘读写。ADR-072 的「只有 event 能力」前提不成立。
- **Security Specialist**：React 后端动态 `href={expr}` 无运行时净化（`react/view.rs:400-404`），OmniView 有 `__omni_sanitizeUrl`（`omniview/view/element.rs:161-165`）——生产后端 parity 缺口。

### T3. 性能：增量缓存容量 < 旗舰项目 + debug 双编译（4 专家）
- **Performance Specialist**（实测）：冷编译 21.7ms/45.7ms；缓存命中 0.7ms vs 驱逐 10.9ms = **15.6× 退化**；`MAX_CACHE_ENTRIES=50`（`analyse.rs:238`）< 102 文件项目；`useDebugCompile` 每 150ms 双编译，其中一个 uncached（`rustCompiler.ts:325` resolveStylesMulti）；cached 路径仍整项目重跑 lint/optimize/theme/event 校验（10.5ms vs codegen 1.7ms）。
- **Compiler Architect**：LRU 驱逐销毁反向依赖边 → 被驱逐文件的 importer 用陈旧 IR 合并（正确性）；const 依赖不走 import 边 → 设计 token 变更不失效引用文件。
- **Rust Expert**：`event_data_schema()` 每个 handler 每次调用分配新 Vec+String（lower/lint/各后端 5 处）。

### T4. 死面与无法交付的声明（5 专家）
- C FFI：`omnilang-core/src/ffi.rs` + `omnilang/src/ffi.rs` ~600 行，两个 crate 均 `crate-type=["rlib"]` **无任何 cdylib 生产者**，`ci.yml:46-47` 却上传 `omnilang.dll`/`libomnilang.dylib`（不可能存在）；测试充当代理让死面看起来活着。
- Tauri server-compile 三命令（compile_project/analyse_source/project_export）+ `index.ts` 包装零调用者；`diagnostic_to_json` 标 `#[allow(dead_code)]`。
- `cache_state` 信封恒 None（wasm lib.rs:428,530,595），seed/save 机制不可达。
- FastAPI 时代 stub：`compile-api.ts:163-188`、`index.ts:79-118`。
- Golden mirror：`tests/fixtures/golden/` 96 文件 vs 活树 102，**76/96 字节漂移 + 6 缺失**，零消费者（MANIFEST 明说测试不读镜像）。

### T5. 诊断/LLM 消费面（AI Engineer + Rust Expert + Compiler Architect）
- E0006 重载：44 处/15 域（unknown event warning / deprecated event error / invalid key error 共享一码），consumer 只能 parse 中文 prose。
- 新迁移诊断 `suggestion: None`（`validate_event_names.rs:217-233`）——Monaco `💡 suggestion` 通道死掉；对照 `validate_component.rs:36-46` 有 suggestion。
- `llm_error_corpus.rs:505-531` 只断言 message 不断言 code/severity/suggestion，且缺 deprecated-event 用例（LLM 最可能写出的错误模式未入册）。
- `nearest_key_suggestion` 无距离阈值（`validate_semantic_keys.rs:53-59`）：`"TotallyBogus"` → 建议 "Tab"（12 字符距离），自动修复循环会改变程序语义。
- **正面**：9 字段 DiagnosticJson 信封统一、corpus 有 positive controls 与 shape 守卫、行为钉死测试（suggestion-set 完备性、typo→intended event）。

### T6. 跨文件语义验证缺口（Compiler Architect，HIGH）
- `run_ir_passes` 在单文件内运行（`ir/lower/mod.rs:345`），仅 validate_theme/validate_event_names 被提升到 merged IR（`analyse.rs:505-510`）。
- `validate_required_props`：imported 组件缺必填 prop → component_map 查无 → 无 E0011，OmniView（无类型门禁）静默渲染 undefined props。
- `validate_effect_deps`：@effect 读 ImportedStore.field → 漏报 missing dep——**正是该 pass 要杀的「静默不重触发」bug 的跨文件形态**。

### T7. 仓库卫生（4 专家）
- 根目录 45 个已跟踪垃圾文件（零字节孤儿 `codegen,`/`Intent`/`IR`/`JSON`/`lexer`/`parser`/`semantic`、全角冒号文件名、Gitee/GitHub stub、storybook 样板、tests/results 产物）+ 139 个未跟踪 PNG。
- `test/` vs `tests/` 双根（158+105 文件，golden 路径 `tests/fixtures/golden/test/...` 两种拼写并存）。
- `Cargo.lock` gitignore（.gitignore:4）且 CI 无 `--locked`；npm + pnpm 双 lockfile 并存。
- flutter/tests.rs 11,038 行 / react/tests.rs 7,989 行，超过既有 tests_regression 拆分模式阈值。

### T8. 文档准确性系统性问题（Documentation Expert + Systems Architect）
- QUICKSTART 教 `--target python/js/rust`（默认构建必失败）、示例项目不存在（`test/projects/view-todo` 等 5 个全缺）、browser-open 与 stdlib 链接失实。
- README：「13 ADRs」（实际 74）、`omnic init`（不存在，只有 create）、示例文件不存在。
- gate 二进制新鲜度无检查：checked-in `target/debug/omnic.exe`（08-07 17:26）早于事件迁移 → 本地 verify-docs 静默全绿。**这解释了红门禁为何能带着三个错误块「绿」着上库。**

---

## Trade-offs / Disagreements

1. **Golden mirror 处置**：Pragmatic Enforcer 与 Maintainability 一致反对保留；分歧在自动化——Pragmatic 明确「不要加 CI diff gate（重建被裁的机制）」，Maintainability 给出「删除或接线二选一」。**共识：删除字节镜像，MANIFEST 改写为活树索引。**
2. **E0006 码复用**：Phase-1 计划（plan:120）刻意复用（b）；AI Engineer / Rust / Compiler Architect 一致指出后果（eval set 路由、门禁 message 匹配脆弱、自动修复无法分类）。**折中：事件弃用族新增专用码（E_EVENT_DEPRECATED），E0006 保留为兼容码（serde 旧串映射，error.rs:201-237 已有先例）。**
3. **legacy 硬错误是否覆盖组件实例**：Compiler Architect 主张覆盖（无需 DOM 表、canonical 比较即可，当前是保守过度豁免）；计划未定义。**需决策**：建议覆盖 + 保留 unknown-name warning 豁免（那部分才真正需要 tag 解析）。
4. **verify 脚本合并**：Pragmatic 判定三重实现（verify-vocabulary 345 行 + verify-semantic-universe 614 行 + verify-syntax-lifecycle 253 行）为「一个机制实现三次」，但 ratio 1.5 恰在阈值 → **Defer**（触发条件：第三个 anchor 正则 bug 或合计 >2000 行）。
5. **E0006 触达的文档门禁前缀匹配**：Domain/Maintainability 建议把「事件名 」前缀加入 DELETED_SYNTAX_PREFIXES——全体一致，无分歧。
6. **ADR 数量（74）**：Pragmatic 判定为工作记忆非仪式（ratio 0），Documentation Expert 认可 060-074 可独立阅读；分歧点是 fix-round ADR（061/063/064/065/067）未来可主题合并——不急。

---

## 成员评审摘要（verbatim 详见各子评审输出）

### Systems Architect — 系统一致性
- 优势：四层 Rust 拆分清晰；增量缓存三端共享单实现（omnilang/src/lib.rs:322）；CI 验证文化远超单作者常态；feature-flag 尊重 ADR-003。
- 主要关切：事件迁移无 TS 管线任务（HIGH）；C FFI 无法交付（MEDIUM-HIGH）；CLI 宣称 10 目标实发 3 个 + 硬编码版本 0.2.0（MEDIUM）；Cargo.lock/双包管理器（MEDIUM）；双 WASM 缓存面 + cache_state 死管道（MEDIUM）；Tauri server-compile 未用双路径（MEDIUM-LOW）。

### Domain Expert — 域模型与登记册
- 优势：7 域语义模型机制真实（verify-semantic-universe 0 问题）；三见证（spec/编译器/展示）事件分类四方一致；诚实降级纪律。
- 主要关切：verify-vocabulary 门禁红（HIGH，提取正则过宽）；verify-docs 3 块红（HIGH）；登记册域 1 与事件模型矛盾（MEDIUM，教 @click）；建议表含硬错误名（LOW/MEDIUM）；custom event 名与 legacy 名冲突未定义（LOW）；Phase 2 intent 域与机制域层模型未决（MEDIUM）。

### Security Specialist — 安全
- **C1 CRITICAL**：调试窗口编译产物可 invoke 全部自定义 Tauri 命令（build.rs 无 AppManifest::commands；capabilities 只管插件命令）→ 全盘读写。修复：AppManifest::commands 限定 main window，5 行。
- H1 HIGH：React 动态 URL 绑定无运行时净化（parity 缺口 vs __omni_sanitizeUrl）。
- M1/L1-L4：drive root 放大 C1；compile_project 任意路径；devCsp unsafe-eval；victauri auth_disabled 仅 debug。
- 优势：三层净化（编译期 URL 块表 + 运行时 sanitizer + CSP）、WASM 契约干净（10MB 上限/E_BOUNDARY/struct 信封）、sanitize_path 规范、前端零 dangerouslySetInnerHTML。

### Maintainability Expert — 可维护性
- 优势：deferral 台账纪律（398 行 87 项，触发条件+证据）是本领域最强模式；事件名单一事实源 + 测试镜像交叉核对；门禁豁免↔deferral 重连（#61-#73）。
- 主要关切：doc 门禁结构性盲区（HIGH，DELETED_SYNTAX_PREFIXES 缺事件族）；45 个已跟踪根垃圾文件（MEDIUM）；golden mirror 三次漂移（MEDIUM，机制缺口）；test/ vs tests/（MEDIUM）；Cargo.lock 决策（LOW）；tools/*.js codemod 无主（LOW）；flutter/react tests.rs 超阈值（LOW）。

### Performance Specialist — 性能（全实测）
- C-1 HIGH：MAX_CACHE_ENTRIES=50 < 102 文件 → 击键分析 15.6× 退化（10.9 vs 0.7ms）。
- C-2 HIGH：debug 窗口每 150ms 双编译，resolveStylesMulti uncached（~22ms 冷）+ 无变更短路。
- C-3 MEDIUM：cached 路径仍整项目重跑 validate/lint/optimize（10.5ms vs codegen 1.7ms），收益封顶 ~56%。
- C-5 LOW-MEDIUM：OmniCompiler 多文件分支绕过缓存（潜在发散）。
- 优势：编译器本身快（冷编译 21.7ms）；生成的运行时精简（index.js 11.9KB）；直写通道符合 ADR-070/071/073；WASM 边界机制良好。C-8/C-9 判定为 intentional-per-ADR。

### Implementation Strategist — 变更排序
- 优势：spec→plan→TDD→gate 结构保持；硬切换可逆（单 commit）；爆炸半径前置量化；@onChange 判别保留真实语义区别。
- 主要关切：编辑器事件面板未迁移（MEDIUM-HIGH）；flip-then-migrate 造成 38min 红测试 + 2.5h 红 demo（MEDIUM）；cheatsheet 教死语法（MEDIUM）；Task 8 违反「豁免不新增」约束（MEDIUM-LOW，有基线证据 + deferral #73）；82 commits 未推送（LOW）；@onInput 判别无回归保护（LOW，intentional）。

### AI Engineer — LLM 消费面
- 优势：9 字段信封统一 + 边界 shape 测试；corpus 非空设计（positive controls、行锚定、6×5 shape 守卫）；suggestion 机制有行为钉死测试；机器可查知识库接入 CI。
- 主要关切：新诊断 suggestion=None（HIGH）；corpus 只钉 message 不钉 schema + 缺 deprecated-event 用例（HIGH）；E0006 重载（MEDIUM-HIGH，intentional-but-costly）；nearest_key_suggestion 无阈值（MEDIUM）；Tauri analyse_source 平行诊断面覆盖更弱且零调用（MEDIUM）；README「13 ADRs」过时（LOW）。

### Pragmatic Enforcer — YAGNI
- **核心判断**：门禁红是因为在工作（verify-vocabulary 提取正则 bug + docs 3 块 = 真漂移被真捕获）；黄金镜像 79% 漂移是假文档；~1500 行死面应删；直接写通道为 justified complexity（实测前置）；74 ADR ratio 0 合理；门禁合并 ratio 1.5 → Defer。
- 逐项 Pragmatic 评分：R1 修双红门禁 0.33 ✅ 立即；R2 C FFI 删 0.5 ✅ 立即（触发：有 Python 绑定目标才生产）；R3 golden mirror 删 0.14 ✅ 立即；R4 死面清理 0.5 ✅；R5 lockfile 0.17 ✅；R6 脚本合并 1.5 ⚠️ Defer（触发：第三个 anchor bug 或 >2000 行）；R7 ADR 0 ✅ 维持。

### Rust Expert（Katya Volkov）— 全实测通过
- **验证**：cargo check / clippy -D warnings / fmt / test 全绿。unsafe 仅 2 个 FFI 边界文件。unwrap/expect 审计依旧成立（4 处全部 by-construction 可证）。
- C1 MEDIUM：event_data_schema 每次调用分配（→ &'static slice）。
- C2 MEDIUM：3 处 guard+unwrap（flutter/view.rs:2218,1341、dioxus/view.rs:93）→ if let。
- C3 MEDIUM：IR_EVENT_NAMES 手写镜像单向保证（→ 从 map_event_data_kind 机械派生）。
- C4-C9 LOW：warnings lock poison 升级 panic、validate_semantic_keys 反序操作数旁路、prefix-score 重复、E0006 三语义共享、过时测试注释、overflow-checks 未开。
- **总体：Approve，无 CRITICAL/HIGH。**

### TS&React Specialist（Emma Rodriguez）— 编辑器层
- **C1 CRITICAL**：palette 提供编译器硬拒绝的事件名；点击/悬停绑定是编辑器主流程，已端到端损坏。
- H2：parser-view.ts:333-339 归一化 @click→onClick（读侧制造硬错误源）。
- H3：双 hook 每 150ms 双全量编译（perf 佐证）。
- H4：debug 窗口 CSS-only 变更与编译失败静默陈旧（diff 只比 component JS hash；`if(!output) return` 吞错）。
- M5-M8：source 生成编排重复 50 行、HandlerModal 无 dialog 语义/焦点陷阱、事件改名双 undo 记录、事件名三处 truth + onTap 不可达 + Button 丢通用事件。
- L9-L12：WASM 边界 any、Storybook 实际 1 个 story 且双配置、toggle 无 aria 状态、loadOmniFiles 370 行单体。
- 优势：store 组合纪律、useSyncExternalStore 使用模型正确、87 个测试文件、a11y 基础优于平均。

### Compiler Architect（Dr. Lin Wei）— 语言工程
- C1 HIGH：nearest_event_suggestion 返回硬错误名（契约破坏）。
- C2 MEDIUM：legacy 硬错误跳过组件实例（计划原则 8「不留旧残留」不统一）。
- **C3 HIGH**：required props / effect deps 仅单文件校验（跨文件静默错码类）。
- **C4 HIGH-adj**：LRU 驱逐毁失效边（陈旧 IR 合并）+ const 依赖不失效。
- C5 MEDIUM：source map 管线惰性（所有 target_spans 空 Vec，逆向映射空 omni_file）。
- C6-C9 LOW：诊断锚在元素 span 非 token；docs 教被拒名；IR 镜像列表；重复名合并 warning+非确定 last-wins。
- intentional 确认：type_ann 未填充（deferred by design，Flutter type_annotate.rs 工作绕）；E0006 桶；ADR-040 移除 EventDataKind non_exhaustive 后 claim 过时（低危）。

### Documentation Expert（Samantha Quill）— 文档体系
- **C-1 CRITICAL**：入口文档教硬错误语法（cheatsheet 自称「可直接复制为独立文件编译」，§1.7 现不能编译）。
- C-2 HIGH：门禁无二进制新鲜度检查 → 陈旧 omnic 静默全绿（本次 3 块错语法「绿」着过本地）。
- C-3 HIGH：DELETED_SYNTAX_PREFIXES 缺事件族 → fragment 块放行。
- C-4 HIGH：QUICKSTART 教 feature-gated 目标 + 不存在示例项目。
- C-5 MEDIUM：README 多处失实（13 ADRs/omnic init/示例文件）。
- C-6 MEDIUM：事件意图化无 ADR-075（决策链在「待审阅」spec 处断头）。
- C-7 MEDIUM：capturePointer 迁移指导与 ADR-073 冲突（两旗舰文档互斥）。
- C-8 LOW：Diátaxis how-to 象限空（learn-omniview 是替代载体但缺人读索引）。
- 优势：verify-docs 契约（「fix the DOC, never the compiler」）操作化 ADR-054 P5；semantic-universe 是真正知识库设计；ADR-054 Superseded 表是全库最佳导航；07-events-and-bindings.md 是质量标杆。

---

## 优先级与建议

### Critical（0-2 周）

1. **事件意图化 Phase 1 收口（T1/T5/T8）** — Effort S-M
   - 修 3 个红 doc 块：`00-cheatsheet.md:175-189`（@onClick→@onPress）、`migration-v1-to-v2.md:185-234`、BNF spec `:829-893`；同步 05-components.md:202 / 06-templates.md:226 注释。
   - 修 `verify-vocabulary.py` 提取正则（fn-scoped，镜像 verify-semantic-universe.py:120-129 先例）+ 登记册域 1 重写为三层模型（语义名 onPress / legacy 名 hard-rejected / canonical 实现名 click）。
   - `verify-docs.py` DELETED_SYNTAX_PREFIXES 增加事件弃用族前缀（如「事件名 」）。
   - poiesis 事件面板迁移：`componentProtocol.ts` + `EventsSection.tsx` 换 14 语义名，加 onEnter/onLeave；`parser-view.ts:333-339` 归一化到语义名；emitter 侧做旧名→语义名映射（一处）。
   - `EVENT_NAME_SUGGESTIONS` 过滤 4 个硬错误名 + 新增 `every_suggestion_is_legal` 测试。
   - 写 ADR-075（14/40 切分、legacy→hard-error 契约、不留旧残留原则）。
   - 收口验证：重建 omnic 后跑全部门禁 + cargo test + verify-react + Playwright，结果表入 ADR。
   - **Pragmatic 0.33 ✅ 立即**。

2. **安全 C1：gate 自定义 Tauri 命令** — Effort S
   - `poiesis/src-tauri/build.rs` 用 `AppManifest::commands(&[...])` 限定 main window，或每个敏感命令加 `window.label() == "main"` 守卫。恢复 ADR-072 自以为拥有的属性，5 行变更。
   - 同步修正 ADR-072 + poiesis-tauri-security-model 记录真实爆炸半径（未修时=全盘）。

3. **安全 H1：React 动态 URL 运行时净化** — Effort S
   - 发射共享 sanitizeUrl runtime helper（同 sanitizeCss 模式，react/design.rs:483），包裹动态 href/src/action/formaction/poster；注入回归测试 `href={signal}` + `javascript:` 值。

4. **性能 C-1/C-2：缓存容量 + debug 双编译** — Effort S（实测收益最大）
   - MAX_CACHE_ENTRIES ≥ sources.len()（如 max(50, len) 或 1000）。
   - `compileForDebugWindow` 换 resolveStylesMultiCached + sourcesKey JSON 短路（同 useWasmCompile.ts:142-147）。

### Important（2-8 周）

5. **缓存失效边正确性（C4a/b）**：独立于条目驱逐的 reverse-import index；const 变更使引用文件失效（程序级 const-version stamp）。Effort S-M。
6. **跨文件语义提升（C3）**：validate_required_props / validate_effect_deps 移入 merged IR 阶段（同 validate_event_names 模式）——OmniView 目标无下游门禁，这是静默错码类。Effort S。
7. **死面清理一轮（T4）**：C FFI 删除（或加 cdylib crate + CI 构建，二选一）+ ci.yml:46-47 幻影产物路径；Tauri server-compile 三命令 + index.ts 包装 + diagnostic_to_json 删除；cache_state 信封 + seed/save API + TS _cacheState 拆除；compile-api.ts/index.ts FastAPI stub 删除。Effort S。
8. **golden mirror 删除**：`tests/fixtures/golden/` 子树删除，MANIFEST.md 改写为活树索引；不加自动化 gate。Effort S。**Pragmatic 0.14 ✅。**
9. **仓库卫生**：git rm ~40 个根垃圾文件；PNG 归档或 gitignore；test/ vs tests/ 更名（test-support/）；tools/*.js 归档；删除双 pnpm-lock.yaml；提交 Cargo.lock + CI --locked。Effort S。
10. **debug 窗口静默陈旧（TS H4）**：CSS/HTML hash 纳入 diff（变更时整页推送）；编译错误显式 error payload。Effort S-M。
11. **诊断通道（T5）**：事件/键诊断填 suggestion + end_line/end_col；nearest_key_suggestion 加距离阈值（≤3 同 suggest_similar_name）；llm_error_corpus 补 deprecated-event 用例 + 断言 code/severity；事件弃用族新诊断码。Effort S-M。
12. **legacy 检查覆盖组件实例（C2）**：canonical 比较不需要 DOM 表，是保守过度豁免；保留 unknown-name warning 豁免。Effort S。**需决策。**
13. **CLI/QUICKSTART/README 失实（T8）**：ALL_TARGETS 按 feature 过滤 + env!("CARGO_PKG_VERSION")；QUICKSTART 目标表标注 feature-gated + 示例项目换 learn-omniview；README 命令/计数/示例修正。Effort S。
14. **Rust hygiene（Katya 项）**：3 处 guard+unwrap → if let；warnings lock unwrap_or_else；event_data_schema → &'static；IR_EVENT_NAMES 从 map_event_data_kind 派生；validate_semantic_keys 反序操作数；prefix-score 去重；过时注释。Effort S。
15. **flip-then-migrate 规则入模板（Implementation Strategist）**：先迁消费者后翻开关单 commit；Phase 2 四个域直接复用。Effort S。

### Nice-to-Have（2-6 月）

16. source map 管线激活（OmniView 起，target_spans + omni_file）；type_ann 核心修复 + Flutter workaround 拆除（跨目标回归门槛）；E0006 子码体系；analyze_multi_cached fuzz 覆盖；Storybook 双配置合并 + 面板 stories；Diátaxis how-to 象限（任务→demo→章节索引）；flutter/react tests.rs 拆分 tests_regression；单来源 TS 事件表（从 event_names.rs 生成）；corpus 发布为 (bad source, diagnostics, gold fix) 三元组 LLM eval 集；HandlerModal dialog 语义 + 焦点陷阱；undo 原子改名；重复组件名合并改硬错误。

---

## Risks If Unaddressed

| 风险 | 可能性/影响 | 缓解 |
|---|---|---|
| 编辑器成为硬错误工厂（palette 教 onClick，@onClick 编译失败，debug 预览冻结） | 高/高 | Critical #1（面板迁移 + emitter 映射） |
| 调试窗口任意代码 → 全盘文件读写（ADR-072 边界失效） | 中（需 codegen bug 或 window 可达）/极高 | Critical #2（AppManifest::commands） |
| 生产 React 应用 javascript: URL XSS（动态绑定） | 中/高 | Critical #3（sanitizeUrl） |
| 红门禁常态化 → 豁免滑坡（「豁免不新增」已被 Task 8 破例） | 高/中 | Critical #1 + 收口验证表入 ADR |
| >50 文件项目击键卡顿 + 陈旧 IR 合并（LRU 毁边） | 高/中高 | Critical #4 + Important #5 |
| 跨文件 effect 缺依赖/缺必填 prop 静默错码（OmniView 无门禁） | 中/高 | Important #6 |
| 82 commits 未推送 = 单点磁盘故障全失 | 中/极高 | 推送观察 CI（现状即缓解手段，建议日更） |
| QUICKSTART 把新用户带进死路（第三条命令即失败） | 高/中 | Important #13 |

---

## Success Metrics & Follow-up

1. **门禁状态**：verify-docs / verify-vocabulary / verify-semantic-universe / verify-syntax-lifecycle 全绿（CI 0 failed）→ 目标：本收口后 + 保持 4 周（CI 可见性：每日 push）。
2. **编辑器事件**：poiesis 事件面板 100% 语义名，onEnter/onLeave 可选，round-trip 测试（parse→emit→omnic analyse）入 CI → 收口时。
3. **安全**：AppManifest::commands 生效（调试窗口 invoke 自定义命令被拒的回归测试）→ 收口时。
4. **性能**：learn-omniview 击键分析 10.9ms → ≤1ms（缓存适配后实测）；debug 窗口每 tick 1 次编译 → 收口时。
5. **死面**：~1500 行已证死面删除（grep 验证 importers=0）→ 2 周内。
6. **仓库卫生**：根垃圾文件 45 → 0；lockfile 单一化 → 2 周内。
7. **ADR 连续性**：ADR-075 落库（事件意图化决策链不中断）→ 收口时。

**Next Review**: Phase 1 收口后 / Phase 2（意图级域）启动前，建议 component-review 粒度复检事件域。

**Tracking**: 本评审发现已与 ADR-074/deferral 台账风格一致；Critical 项建议成立收口计划（docs/superpowers/plans/）逐项 TDD 执行；Pragmatic Enforcer 判定「R1-R5 立即、R6 门禁合并 Defer」已纳入优先级。

---

## 成员评审全文

> 12 位专家的完整 verbatim 评审（含全部 file:line 证据与 Pragmatic 逐项评分）由本次评审工作区保留；上文各节为忠实摘要。若需任一成员的完整原文，可单独提取（会话内已完整生成）。

---

**Review Complete**
