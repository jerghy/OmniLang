# 推迟项跟踪

> 创建：2026-07-27 | 最后更新：2026-08-03
> 来源：`.architecture/reviews/implementation-plan-2026-07-26.md`
> **⚠️ 注意：本文件中的推迟项已被 ADR-006 重新分类。详见下方各条目。**

---

## 状态变更摘要（ADR-006，2026-07-27）

原本推迟的 4 个项目（P35/P36/P39/P38b）已被 **ADR-006** 重新分类为**立即执行**。以下记录原始推迟理由（供历史参考）以及当前状态。

---

## 已重新分类为立即执行

### P35: Crate 拆分评估 → 立即执行

**原始推迟原因**：YAGNI — 编译器当前 ~63K 行，无拆分痛点。

**ADR-006 重新分类理由**：深层评审发现 `omnilang/src/` 模块耦合度超出预期——`analyze.rs` 反向依赖 `codegen/`，`ir/` 与 `parser/` 存在循环引用。拆分不是性能优化，是架构卫生措施。

**当前状态**：ADR-009 已制定拆分方案，按依赖 DAG 分阶段执行。

---

### P36: 代码生成并行化 → 立即执行

**原始推迟原因**：无 profiling 数据证明 codegen 是瓶颈。

**ADR-006 重新分类理由**：WASM 基准测试（wasm-baseline.md）显示 20 文件项目 0.31s 的主要耗时在 codegen 而非解析。且 Flutter 后端的 `setState()` 包装逻辑是纯函数式转换，天然适合并行化。

**当前状态**：ADR-009 已设计跨后端并行方案，验证了 Send + Sync 安全性。

---

### P39: IR 优化传递 → 立即执行

**原始推迟原因**：优化前需要 motion 测试达到 95% 分支覆盖率。

**ADR-006 重新分类理由**：P33（source_span 选择性填充）的实现已经暴露了 IR 中可优化模式——空 `source_span` 在 lowering 后仍然遍历整个 IR 树，消除这种浪费不依赖 motion 覆盖率。

**当前状态**：ADR-008（P33）已交付 source_span 选择性填充，IR 优化传递作为后续阶段排入 ADR-010。

---

### P38b: Strip TAURI_INTERNALS → 立即执行

**原始推迟原因**：防御性安全——等 iframe 沙箱（P38）先生效。

**ADR-006 重新分类理由**：P38 iframe 沙箱已交付并验证通过（`sandbox="allow-scripts"` + postMessage 桥接）。延迟 P38b 不再提供安全收益——`__TAURI_INTERNALS__` 暴露是独立于沙箱的攻击面。

**当前状态**：ADR-010 已制定分阶段实施策略（XSS 升级链 + 安全加固 + 选择性性能优化）。

---

## 已移除项目

### P34: AI 代码补全

**移除原因**：新功能非债务偿还。计划评审中 3/3 专家一致：类别错误——在 Consolidation 阶段引入 feature 传递混乱信号。属于独立的专项计划。

**重新评估**：编辑器和编译器核心稳定后另立专项计划。

---

## 当前活跃推迟项

以下推迟项来自 2026-07-30 的 11 专家架构评审（见 `.architecture/reviews/component-omniview-2026-07-30.md`）。

### 来自 ADR-036/038 (Element Reference System)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 1 | `@ref` inside `@for` loop bodies (Phase 2) | User demand for per-item refs | Currently emits compiler warning; unsupported |
| 2 | ~~Flutter `GlobalKey` codegen for `@ref`~~ | ~~Flutter P0 commitment~~ | ✅ 2026-08-01 已关闭（ADR-052/053 Phase 5：`omnilang/src/codegen/flutter/ref.rs` 为每个 `@ref` 生成 `GlobalKey` + 按需 controller） |
| 3 | Custom component ref forwarding (`forwardRef`) | Cross-component ref patterns | Not yet designed |
| 4 | ~~`ref_names` threading through React store codegen (16 call sites)~~ | ~~User reports store + ref combination bugs~~ | ✅ 2026-08-01 已关闭（Phase 4 React 重写：`component/computed.rs`/`effect.rs`/`function.rs` 均接收真实 `ref_names`；剩余 `FxHashSet::default()` 调用点为 state 初始值等无 ref 合法场景） |
| 5 | ~~`element_tag` population from ElementIR tag~~ | ~~Non-div elements with refs~~ | ✅ 2026-08-01 已关闭（被 ADR-052/053 语义 ref 系统取代——ref 类型无关，React `useRef`/Flutter `GlobalKey` 不再需要 tag→type 映射；`element_tags` 仅保留用于 OmniView 编辑器元数据） |

### 来自 ADR-037 (Signal Batching)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 6 | Signal batching with microtask deferral | Profiling gate: real app, measurable frame drops, bottleneck confirmed | Synchronous notification on every set() |

### 来自 ADR-021 (Cross-Target Feature Parity)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 7 | Cross-target regression test matrix | New backend added or paused backend reactivated | ⚠️ 2026-08-01 部分实施：CI `codegen-validate` 对 11 个 React fixture 跑 `tsc --noEmit` + Flutter store 文件跑 `dart analyze`（工具链缺失时 SKIP，见 ci.yml）；全后端矩阵（Vue/Dioxus/JS 等）仍待完成。2026-08-02 T0.3 新增编译级 per-backend `feature-matrix` CI job（`cargo check --no-default-features --features <backend>` × codegen-react/vue/dioxus/flutter/omniview）——fixture 级全后端矩阵仍待完成 |

### 来自 ADR-001 (Pipeline Architecture)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 8 | TypeKind recursive validation for `Named` types | Language adds generic structs with custom type params | Unvalidated type names pass through IR silently |
| 9 | Source map pipeline completion (non-OmniView backends) | React/Flutter debugging needs | target_spans empty for React/Flutter |
| 10 | Codegen trait full adoption (OmniView) | Next major IR refactoring | OmniView bypasses standard compile() path; see also ADR-013 |
| 11 | ~~IR `#[non_exhaustive]` migration~~ | ~~Next ExprIR/StatementIR variant addition~~ | ✅ 已关闭（2026-07-30 会话迁移，ADR-040）：`ir/types.rs` ×4、`ir/design.rs`、`ir/signals.rs` 均带 `#[non_exhaustive]` |

### 来自 ADR-032 (Animation Scoping) — Parked

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 12 | ~~Dead @focus JS codegen in React backend~~ | ~~React focus animation feature~~ | ✅ 2026-08-01 已关闭（ADR-053 D4：@focus 成为受支持触发器，React codegen 用 `onFocus` + `element.animate()` 原生实现于 `react/motion/mod.rs`；原 mod.rs 死代码区域现为 import 路径辅助函数） |
| 13 | ~~Removed trigger tokens still in lexer~~ | ~~Lexer cleanup pass~~ | ✅ 2026-08-01 已关闭：删除测试 `test_deleted_sub_attribute_tokens_lex_as_ident`（`lexer/mod.rs:1243-1260`）验证 `@paused`/`@in-view-once`/`@drag-constraints` 等子属性 token 已移除、按普通 ident 词法化；`keywords.rs` 无残留映射（仅 `@drag-horizontal`/`@drag-vertical` 触发器 token）。parser 保留 `paused` 播放关键字仅作兼容解析（`PlaybackIR.paused` 已删，运行时不生效）。其余文档陈旧见 #24 |
| 14 | Stale doc comments referencing removed features | Doc audit | ⚠️ 2026-08-01 部分修复：文档侧已全部完成——`06-templates.md` + `11-motion-animation.md`（Task 10.1）+ 其余 4 个文档（commit 10f2af9c，见 #24）；`easing.rs`/`easing.ts` 所有权注释已调和（10f2af9c）。其余代码注释待审计 |

### 来自 ADR-034/035 (v4 Review Structural Fixes)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 15 | OmniViewGenerator decomposition into phase structs | Next major OmniView feature | 47-field struct with coupled phases |
| 16 | Complete React FormatExpr migration (remove direct variant handlers) | Next ExprIR variant addition | 3 variants handled directly, rest delegated to trait

### 来自 ADR-052/053（语义级 DSL，2026-08-01 会话）

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 17 | ~~**[HIGH]** Flutter widget codegen 生成不可编译 Dart（6 处）~~ | ~~`dart analyze` 门禁扩展到全部生成文件~~ | ✅ 2026-08-02 已关闭（Task 0.5 附证据）：6 个原始 CRITICAL 已修复——commit a17e6e0d（多元素根/onPressed/slot 回退 Text(child:)/List 未判空/VoidCallback (e)/Text 内非 String/design 断言 + 门禁扩展至全部生成文件）、commit 5c1db2bc（slot 投影组件导入、bind:value 默认去重、VoidCallback 字段丢弃警告对称化）；门禁已从白名单改为**全量枚举**——`flutter_output_compiles_with_dart_analyze` 对每个 parameterized fixture 的**全部**生成 .dart 文件跑 `dart analyze`（`flutter_analyze_targets`，compilability 评审 systems-architect 实测确认）；展示项目 Flutter 门禁与 match 缺口见 #30 |
| 18 | **[MEDIUM]** AtFocus/AtHero parser 缺口 | 用户使用 `@focus` 文本内容 / AtHero 元素属性 | `is_text_token`（`parser/view/style.rs:564-604`）未含 AtFocus——文本中的字面 `@focus` 静默截断子节点 + hard error；AtHero token 无元素属性解析臂（hard error）。AtHero demo 文件已不在活项目中 |
| 19 | ~~**[MEDIUM]** Drag 触发器无约束时失效~~ | ~~拖拽触发器恢复可用之前~~ | ✅ 2026-08-03 已关闭（Task 5.1 附证据）：`@drag-*` 控制维度恢复（v2 BNF + non_exhaustive enums + 三后端消费 + node 实证——constraints [-100,100] elastic 0.5 → 过冲 -150 约束至 -125 → 0.625 弹性回弹）——无 constraints 时 codegen 从 motion keyframe 范围推导默认 extent（`react/motion/transforms.rs` 无约束分支 emit 推导值 + 注释；仅当无字面 translateX/Y 关键帧时才 warning + Infinity 诚实降级）；`parseConstraint("")` ±Infinity → progress=0 路径不再可达于正常用例 |
| 20 | ~~**[MEDIUM]** Style 读取未计入 lint 信号读取~~ | ~~用户仅通过 `style:` 使用信号~~ | ✅ 2026-08-01 已关闭（见下方状态节，表格行补划）：`semantic/lints/ast_collectors.rs` 新增 `ElementProp::Style` 臂 + 测试（镜像 #27 修复），style-only 信号不再误报 "never used"（React useState 缺口的镜像修复） |
| 21 | **[MEDIUM]** 跨文件 slot 转发检测缺失 | 组件跨文件导入时转发 slot | E0006 slot 转发警告是模块局部的（`module_component_names` 只含当前文件组件）——导入的组件绕过该警告 |
| 22 | ~~**[MEDIUM]** `--all-features` 测试编译失败（历史遗留）~~ | ~~`cargo test --all-features`~~ | ✅ 2026-08-02 已关闭（Task 0.7 实测证据）：`cargo check -p omnilang --features all-backends --tests` 通过（exit 0，仅 17 个既有 unused-variable 警告、无错误）。促成修复：T0.3/T0.3a `is_dangerous_url` 移入 shared + vue/dioxus sibling-backend 解耦（commits de48bcde/36ca42fe/dabdbf79，dioxus feature-matrix 实测 green），旧 IR 字段引用与 `vue/tests.rs` 括号问题随重构清理。注：CI `test-all-backends` job 仍为 `if: false`（禁用），编译级后端口由 T0.3 新增的 per-backend `feature-matrix` job 覆盖（见 #7） |
| 23 | **[MEDIUM]** omni-animation TS 运行时缺口 | 使用 kebab 缓动名或 translateX/Y/Z 关键帧 | `easing.ts` 无 kebab 别名（`power1-in` 降级为 linear）；`keyframes.ts` `TRANSFORM_MAP` 缺 translateX/Y/Z |
| 24 | ~~**[MEDIUM]** 文档陈旧（docs/omniview）~~ | ~~全面文档审计~~ | ✅ 2026-08-01 已关闭（commit 10f2af9c）：`14-grammar-lang.md`、`15-grammar-view.md`、`05-components.md`、`07-events-and-bindings.md` 全部审计并同步 ADR-052/053 语法（kebab 触发器、17 属性白名单、`over:`/`easing:` 冒号 timing、slot D5 体级声明 + 命名属性消费、ref 语义方法表、@if `mode:` 冒号形式等；删除内容均加 "removed per ADR-053" 注记）。`06-templates.md` / `11-motion-animation.md` 已于 Task 10.1 修复 |
| 25 | ~~**[MEDIUM]** BNF 规范与 parser 语法分歧未同步~~ | ~~规范文档更新~~ | ✅ 2026-08-01 已关闭（commit 10f2af9c）：`2026-07-31-semantic-level-dsl-syntax-spec.md` 已按 parser 同步——`cubic(x1:...,y1:...,x2:...,y2:...)` 命名参数形式（替代 `cubic-bezier(num,...)`）、`delay <expr>` 空格形式（替代 `delay ':' expr`）、`direction` 规则补 `alternateReverse`；另修正 `loop`（无 `infinite` 关键字）、`steps()` 位置参数、per-stop `ease`、token_type 小写、computed 空括号等 9 处分歧；触发器子属性与内联动画体标注为未实现（见 #19） |
| 26 | ~~**[MEDIUM]** `@if mode:` 过渡效果跨后端缺失~~ | ~~mode 过渡效果落地前~~ | ✅ 2026-08-01 已关闭（commit 96480059 + 评审修复）：React `"fade"` 实现真实淡入——`.omni-if-fade` CSS 类 + keyframes 生成在组件 `.module.css`（`codegen/react/component/mod.rs`），分支包裹在带按分支 `key` 的 div 中以重放动画（`codegen/react/view.rs`），含 `prefers-reduced-motion` 覆盖；`"wait"` 在 React 与 OmniView 降级为立即切换并在生成代码中注明降级（无 @leave 机制可等待）；OmniView `@if` wait 与 `@for` keyed cleanup 的 `_leave` 死查找全部移除（`editor.rs`、`list.rs`，代码库零 `_leave` 引用）；Flutter `"wait"` 保持 `AnimatedSwitcher`（`codegen/flutter/view.rs`） |
| 27 | ~~**[MEDIUM]** 组件级 `@motion` 关键帧体未计入 lint 信号读取~~ | ~~用户仅在 `@motion` keyframe 属性中使用信号~~ | ✅ 2026-08-01 已关闭（commit 5fe5a52c）：`ast_collectors.rs` 新增 `collect_used_names_in_motion_def`，收集 keyframe 属性值 + `over:` 时长 + `delay` 播放表达式（全部 Expr 字段）；`dead_code.rs` check_ast 遍历 `comp.motions`（+2 测试，镜像 #20 模式）。IR 侧同修：`eliminate_dead_computeds` 现接收 `module.motions` 并走 `collect_idents_from_motion_body`（此前内联触发器体被 `lower_inline_motions` hoist 后 `inline_motion` 清空、module 级 motion 表从未被遍历 → keyframe-only computed 被 DCE 误删，+2 测试含端到端管线） |

---

## 2026-08-01 状态更新（语义级 DSL 实施完成）

**已关闭**：#2（Flutter GlobalKey）、#4（ref_names 线程化）、#5（element_tag 被语义 ref 系统取代）、#11（IR `#[non_exhaustive]` 迁移）、#12（@focus 死代码）、#13（lexer 子属性 token 删除——由删除测试 `test_deleted_sub_attribute_tokens_lex_as_ident` 验证）、#20（Style 读取已计入 lint 信号读取——`ast_collectors.rs` 新增 Style 臂 + 测试，并顺带修复同类 ClassMap/内联 motion tween 缺口）、#24（docs/omniview 全部 6 个文档审计完成——Task 10.1 + commit 10f2af9c）、#25（BNF 规范已按 parser 同步——commit 10f2af9c）、#26（`@if mode:` 过渡跨后端——React fade 落地 + wait 诚实降级 + `_leave` 死代码移除，commit 96480059 + 评审修复）、#27（组件级 `@motion` keyframe 体已计入 lint 信号读取——`ast_collectors.rs` 新增 motion 收集器 + `dead_code.rs` 遍历 `comp.motions`；IR 侧 DCE 同步覆盖 `module.motions` keyframe 表达式，commit 5fe5a52c）。原因见上表各条目备注。

**部分实施**：#7（CI codegen-validate 门禁落地：tsc + dart analyze）、#14（文档审计完成，代码注释待审计）。

**新增**：#17-#24（本会话 8 项发现）+ #25/#26（10.1 代码质量评审发现：BNF 规范分歧、`@if mode:` 跨后端缺失）+ #27（#20 代码质量评审发现的 keyframe 体同类缺口，留待后续修复）。其中 #17 为 HIGH（P0 目标生成不可编译代码，门禁尚未覆盖）；#26 涉及 ADR-021 floor 原则的静默行为差异。#27 已于同日关闭（见上）。

**ADR-053 D7 完成项**（无对应条目、已随实施落地）：`paused_override`/`PlaybackIR.paused`/`@paused` 属性删除、React Hooks store 模式删除、demo 迁移（9.1-9.4）与 golden mirror 重同步（9.5）。

---

### 来自 stdlib 接线代码质量评审（2026-08-01 会话）

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 28 | **[MEDIUM]** `Map.set` 方法调用无后端实现 + `Map<String, T>` 返回注解未被类型检查器解析 | 用户代码需要 Map 就地变更 / 泛型 Map 返回注解 | ① `map.set(k, v)`：MethodCallExpr 无 `set` 翻译臂，OmniView/React 发射原生 `.set` → 运行时 `TypeError: map.set is not a function`。demo 已改写为字面量构建 `{ "key": value }`（datetime_demo/math_ext/string_ext/error_handling），Map 读取 `.get()` 正常。② `-> Map<String, i32>` 返回注解：`parse_type_string`（semantic/types.rs:73）不解析泛型、回退 `Named`，与字面量推断的 `Map<String, Any>` 不兼容 → E0010；demo 已去掉此类返回注解。修复方向：codegen 增加 `set` 方法臂（Object.assign/spread）+ `parse_type_string` 泛型解析。**2026-08-03 终验复核（T7.3）**：计划「Map.set 随域 8 处理」未落地——T1.2/1.5 覆盖数组方法（filter/map/count/push/pop/sort/splice）与 `contains` map-aware（T1.6），但 `set` 翻译臂三后端均无（react/expr.rs `translate_method_call` 落默认臂，omnilang-core JSON/OmniView/Flutter 方法表同无）；实编译探针 `m.set("key", 2)` 逐字发射进 index.js:317 → 运行期 `TypeError`。保持 defer-with-trigger |
| 29 | **[LOW]** encode hex/base64 仅覆盖 Latin1 | 非 ASCII 输入 | JS 臂 `charCodeAt`/`btoa` 对非 Latin1 字符失真（demo 仅用 ASCII——encode_demo 注释已注明）；修复需 TextEncoder/码点分片 |
| 30 | ~~**[HIGH]** match Flutter 变体臂 + 本地类型发射（learn-omniview Flutter 输出不可编译）~~ | ~~展示项目纳入 codegen-validate 门禁后~~ | ✅ **2026-08-02 已关闭（Task 1.4 附 scoped 证据）**：原缺口——match 变体臂无运行时字段支撑（`switch (n._variant)`）+ 本地类型（`Rating` 类）发射缺失——2026-08-02 实测 `MatchBreadthDemo.dart:649-666`：`Undefined class 'Rating'` / `Undefined name 'level'` / `return_of_invalid_type`。修复：① IR 合并漏掉 `module.enums`（`analyse.rs merge_file_ir_into` 只合并 structs/type_aliases，tagged union 被静默丢弃——根因），补 `existing.enums.extend(...)`；② Flutter 新增 `types.rs`：被引用联合发射进合并 `types.dart`（`Rating.Star({ required this.level })` + 公开 `variant` 判别字段——`_` 前缀成员跨库不可见）+ 组件/store 按需 `import './types.dart'`；③ `DartFormatExpr::format_match`：变体臂绑定字段（`final level = _v.level;`，未用绑定 `_` 前缀）、无通配符时追加 `default: throw StateError('Unmatched variant');`（消除 `return_of_invalid_type`）、IIFE 绑定 `final _v = <subject>`；④ Dart 家族补 `to_string` → `.toString()` 方法映射（镜像 React，match 臂体内字符串拼接可运行）。scoped 测试 `match_variant_arms_flutter_compiles`（`generated_code_validation.rs`）：全项目编译 → `MatchBreadthDemo.dart` 专项 `dart analyze` 无 Rating/level/return_of_invalid_type 错误（实测该文件剩余错误仅 Phase 5 类别：Curves 缓动名/重复 motion controller/String→Color）。**门禁 `learn_omniview_flutter_output_passes_dart_analyze` 保持 `#[ignore]`**——全项目仍 RED 于 Phase 5 Flutter 批次缺口（Curves 缓动名 312、重复 motion controller 225、stdlib 方法发射 202、String→Color props 84 等，共 1125 错误/21 类，见 compilability 评审），un-ignore 条件 = Task 5.5/5.6/5.7 完成后全量 `dart analyze` 0 错误（**该条件已于 T5.8a 达成**：learn-omniview Flutter 全量 0 错误 0 警告，门禁 5/5 un-ignore 绿，T7.3 终验复核确认）。② 语义 DSL 时代记录（ADR-054:88,144、compilability 评审 MEDIUM-10、design doc §7、Task 1.4）均以 **#30** 编号本条 |
| 31 | **[LOW]** `math_random_int` 区间退化未防御 | `hi - lo + 1 == 0` | JS `Math.random()*(0)` 恒返回 lo；Rust `rem_euclid(0)` 与 Dart `nextInt(0)` 会 panic/抛错。修复：codegen 夹取最小区间 |
| 32 | **[MEDIUM]** 非 pub 模块函数不被任何后端发射 | pub 函数调用同模块私有辅助函数 | `ir::all_functions()`（ir/types.rs）过滤 `Visibility::Public`，OmniView/React 等后端只发射 pub 函数——被 pub 函数调用的私有辅助函数编译产物中缺失 → 运行时 ReferenceError（async_demo.omni 的 task_a/task_b/slow_task/fast_task 已改 pub 规避）。修复：按调用图保留被引用函数，或后端发射全部模块函数 |
| 33 | **[LOW]** OmniView 输出嵌套 `#root` 结构缺陷 | 编辑器/WASM 桥接层消费 `output.html` 的方式 | 静态 body.html 含 `<div id="root" data-component="App">` 包装，JS `document.getElementById("root").innerHTML` 模板又包一层 → 运行时 2 个嵌套 `#root`（`[data-component="App"]` 匹配 2 元素）。功能无碍（2026-08-02 两轮 + 2026-08-03 终验浏览器全量测试 0 errors），但结构不干净且 `[data-component="App"]` 查询命中 2 元素。修复方向：`emit_dom_and_bindings` 的 innerHTML 模板剥离外层 root 包装（或 body.html 不再含包装）——涉及 WASM/editor 共用输出，需先验证两端消费方式（2026-08-02 发现，ADR-055 记录）。**2026-08-03 终验复核（T7.3）**：结构依旧——`emit_dom_and_bindings`（omniview/view/mod.rs:90）innerHTML 模板仍以 `<div id="root" data-component="App">` 开头（实编译探针 index.js:324），与静态 index.html/body.html 包装重复；T5.5（Flutter @for）/T7.1a（OmniView keyed rebind）未触及此面。保持 defer-with-trigger |
| 34 | **[LOW]** `regex_match` 语义跨目标不一致 | 无锚点模式 | JS `RegExp.test` 部分匹配 vs Python `re.fullmatch` 全匹配——无 `^...$` 锚点结果不同（demo 模式均带锚点）。编号说明：原 #30 于 2026-08-02 让位给 match Flutter 变体臂（#30 见上——语义 DSL 时代记录 ADR-054/compilability 评审/design doc/Task 1.4 均以 #30 指 match 缺口），本条内容不变仅重编号。**2026-08-03 终验复核（T7.3）**：行状态一致，保持 open（LOW 语义分歧，demo 模式均带锚点无实际破坏） |
| 35 | **[LOW]** 裸 `\`（字符串外反斜杠）lexer 报错中断流 | 用户源码出现字符串外裸 `\`（如 Windows 路径字面量） | 语义 DSL 时代已知残余（ADR-054:144 记录时代 #31，2026-08-02 补登台账）：`lexer/mod.rs` 回退臂（~L297-316）对字符串外 `\` 产生 `TokenKind::Error("Unexpected character")`，错误 token 中断后续词法流（解析层报 Unexpected token）。现为显式报错（非静默，符合诚实降级原则），修复属全局语义变化（定义裸 `\` 的合理语义或带 span 的错误恢复策略）。字符串内 `\` 转义（`read_unicode_escape` 等）与 Text 内 `\` 显式报错均正常，不受本条影响 |

---

## 2026-08-02 状态更新（语义语法 v2 Phase 0 治理同步）

**已关闭**：#22（`--all-features` 测试编译失败——`cargo check -p omnilang --features all-backends --tests` 实测通过（exit 0）；促成修复：T0.3/T0.3a vue/dioxus 解耦 + `is_dangerous_url` 移入 shared，commits de48bcde/36ca42fe/dabdbf79 + per-backend feature-matrix CI job）。#17 已于 Task 0.5 关闭（见上表，未重复编辑）。**#30 已关闭（Task 1.4，见上表附 scoped 证据）**——match Flutter 变体臂 + 本地类型发射修复（IR 合并补 enums + `types.dart` 联合类发射 + 臂内字段绑定 + 非穷尽 default 兜底 + Dart `to_string` 映射）；门禁 `learn_omniview_flutter_output_passes_dart_analyze` 保持 `#[ignore]`，un-ignore 条件 = Phase 5 Flutter 批次（Task 5.5/5.6/5.7）后全量 `dart analyze` 0 错误。

**新增**：#35（裸 `\` lexer Error token 中断流——ADR-054 时代残余 #31 补登，见上表）。

**验证**：本更新仅记录已验证事实（本地 cargo check 实测 + 既有行状态核对），未修改其他行。

---

## 2026-08-03 状态更新（语义语法 v2 终验 T7.3 收口）

**已关闭**：#19（Drag 无约束失效——Task 5.1 默认 extent 推导 + warning 降级，见上表附证据）、#20（Style 读取 lint——2026-08-01 已修复，#27 同源，表格行补划）。**#30 收尾**：un-ignore 条件已于 T5.8a 达成（learn-omniview Flutter 0 错误 0 警告，门禁 5/5 un-ignore 绿），行内注记已补。

**复核未关闭**（附终验证据）：#28（`Map.set`——计划「随域 8 处理」未落地，T1.2/1.5 数组方法不含 `set` 臂，实编译探针逐字发射 → 运行期 TypeError；`parse_type_string` 仍无泛型解析）、#33（嵌套 `#root`——innerHTML 模板仍重包 root，`[data-component="App"]` 2 命中；T5.5/T7.1a 未触及）、#34（regex_match 语义分歧，行状态一致）。其余行状态经 T7.3 终验复核无变化。

**新增**（本计划注册的 follow-up，均带触发条件，见下表 #36-#54）：

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 36 | **[HIGH]** React `let mut` 发射为 `const`（TS2588 → 渲染期 `Assignment to constant variable`，整应用卸载；BUG-A） | LetStmtIR 增 mut 标志的 IR 变更任务排期（~75 构造点） | 与 T1.6 deferral-candidate ② 同根。DataTable/ErrorHandlingDemo 两标签页浏览器实测整站崩溃（T7.1 §4.2-A；UI 与 Language 标签页不可用） |
| 37 | **[HIGH]** React 内联 store 不发射（BUG-C：CartStore 对象从未生成 → ReferenceError） | IR 携带内联 store body 的独立任务 | 与 T3.3「内联 store action 未发射」同根（InlineStoreDemo 购物车永不更新，T7.1 §4.2-C） |
| 38 | **[MEDIUM]** React 静态文本 `&` 渲染为字面 `&amp;`（BUG-D） | 用户报告 & 文本显示异常 | JSX 表达式字符串不解码 HTML 实体（SignalPatternsDemo 标题，T7.1 §4.2-D，aria 快照实证） |
| 39 | **[MEDIUM]** React style 对象发射 kebab-case 属性（BUG-E，TS2353 类运行时形态） | 用户使用 kebab 样式名 | 浏览器控制台 `Unsupported style property 'border-radius'`（Design 标签页，T7.1 §4.2-E） |
| 40 | **[LOW]** React 事件处理器小写名（BUG-F：onAnimationend 等 4 条） | 用户使用小写复合事件名 | 需映射表补 camelCase 别名（T7.1 §4.2-F） |
| 41 | **[MEDIUM]** OmniView `focus={sig}` 在隐藏分支内失效（BUG-2） | 用户对 @if 分支内元素做 focus 绑定 | effect 在脚本求值期执行（分支 display:none），分支显示后不重跑（ElementRefDemo 自动聚焦丢失，T7.1 §4.1-BUG-2） |
| 42 | **[LOW]** parser 分歧批次（T7.2 记录 7 项） | BNF spec 对齐任务排期 / 任一分歧致用户困惑 | union `;` 不一致、@themes/@default-theme `;` 不对称、style-entry `;` 要求、class-value 字符串优先规则、fn-type-param E0010、import 诊断误导、view-text 宽容（T7.2 parser-divergences） |
| 43 | **[MEDIUM]** unused-import lint 系统性假阳性 | lint 精度任务排期 | 元素标签/DesignClass/@store 未计为使用 → 123 条虚假警告（T7.0 ①；learn-omniview 构建持续出现） |
| 44 | **[LOW]** 裸 `design` 关键字破坏元素文本 run | 用户写以 design 开头的文本 | 既有 parser 怪癖（T6.1 deferral-candidate + T7.0 ③；demo workaround） |
| 45 | **[LOW]** Flutter CLI 与库路径诊断口径不一致（capturePointer 硬中止 exit 1 vs 门禁过滤） | Flutter CLI 门禁统一任务 | `omnic build --target flutter` CLI smoke 红、零输出；库路径 dart analyze 绿（T7.0 ④/T7.1 §4.3，非回归） |
| 46 | **[LOW]** Map 字面量返回类型不推断（`Map<String, <unknown>>`） | 用户依赖 Map 字面量类型推断 | T7.0 ②；需类型检查器字面量形状传播 |
| 47 | **[LOW]** CSS identity-escape（\j）残留于 Rust 参考实现 | 扩展 normalize_css_escapes 单文件变更任务 | T3.2 deferral-candidate；`\` 后非 hex 字母处理与浏览器行为分歧（\a 洞已修，\j 类残留）。**2026-08-03 最终评审注记**：修复时同步扩展语料——CSS identity-escape（`\j`）用例加入注入载荷集（`injection.rs`）与 `test_sanitizers.js`（现仅 hex 用例），防同类编码绕过回潮 |
| 48 | **[LOW]** plain-Dart 后端裸 enum 发射 | plain-Dart 进入门禁 / 用户使用 field-carrying union | dart/visitor.rs field-carrying 变体仍坏、无门禁（T1.4 deferral-candidate；formatter 侧已修） |
| 49 | **[MEDIUM]** React 不发射用户 struct → `useState<RowItem[]>` 不可达（10 处 never[] 诚实豁免） | React 用户类型发射任务 | T1.6 deferral-candidate ①；typed useState 落空 |
| 50 | **[MEDIUM]** Flutter 非 ref 元素 bind:value 单向 no-op + Checkbox/Switch/Slider onChanged:null | controller 合成任务 | D2「唯一双向」在部分 Flutter 表面未完全兑现（T2.3 deferral-candidate） |
| 51 | **[LOW]** parser 文本 run 空格剥离 | 文本 run 解析重写 | 指令 token 起始的文本 run 丢空格（T7.1 note；`@for key= 身份追踪 Demo` → `@forkey=...`，与 #44 同类） |
| 52 | **[LOW]** OmniView CSP meta `frame-ancestors` 被 Chrome 忽略 | CSP 部署任务 | 应改用 X-Frame-Options 或 HTTP 头（T7.1 note） |
| 53 | **[LOW]** esbuild 锁 build/debug | CI 工具链升级时 | T7.0 ⑤；展示项目构建锁定 esbuild 版本 |
| 54 | **[MEDIUM]** poiesis 编辑器 codegen warnings 不可读（rustCompiler.ts 手维护副本无 warnings 字段） | poiesis 问题面板接线任务 | T0.6 important (tracked)；poiesis 侧，非编译器范围 |
| 55 | **[MEDIUM]** 数组方法未映射后端 verbatim 发射（T1.2 ①：python/rust/dioxus 原样发射 `items.filter(|x| …)`——静默不可用输出、无诊断；与 T1.5 同批 out-of-floor 后端的 BlockExpr E_CODEGEN 硬诊断处理不一致） | 语言方法映射任务覆盖 python/rust/dioxus（Dart 族方法表 `.len()`/`.get(i)` + Python filter 内置陷阱） | ledger T1.2 ①；五个实质性「minor (deferred)」入行之一（2026-08-03 最终评审注册，非 cosmetic） |
| 56 | **[MEDIUM]** plain-prop 路径静默发射非法 Dart（T5.7 ①：`fontWeight="bolder"`/`color="red"` 无 warn 通道） | plain-prop 值转换层扩展（warn 通道 + 已知值表） | ledger T5.7 ①；展示项目不触及 |
| 57 | **[LOW]** OmniView wrapper null 时静默跳过 vs 注释承诺 loud（T2.2 ①，跨后端行为分歧） | 用户对缺失元素执行 ref 操作并依赖报错 | ledger T2.2 ① |
| 58 | **[LOW]** Dioxus 非字面插值分支发射裸 `"{formatted}"`（T3.1 ①：parser 可达，编译失败 fail-closed） | Dioxus 插值发射路径任务 | ledger T3.1 ① |
| 59 | **[MEDIUM]** 裸非信号标识符在 style 中仍发 `$signal`（T1.3 ④） | style 表达式通道判别扩展（非信号标识符 → 直接标识符发射或诊断） | ledger T1.3 ④ |
| 60 | **[MEDIUM]** OmniView 44-test 浏览器套件未接入 CI（e2e job 仅跑默认 React config，`playwright.omniview.config.ts` 为手动运行） | CI e2e 浏览器门禁统一任务（接线 omniview config） | 分支核心声明「OmniView 浏览器 44/0」仅本地终端验证、非 CI 强制（2026-08-03 最终评审 §4 注记） |

---

## 2026-08-03 最终评审修正（final-review fix wave）

**新增**：#55-#59（五个实质性「minor (deferred)」从 ledger 行升格注册——T1.2 ① / T5.7 ① / T2.2 ① / T3.1 ① / T1.3 ④，与 T7.3 报告 §5-4「实质性 follow-up 已全部入行」的声明矛盾，非 cosmetic）+ #60（OmniView 浏览器套件入 CI）。均带触发条件。

**更新**：#47 行补注记——identity-escape（`\j`）修复时同步扩展注入语料与 `test_sanitizers.js` 用例。

**依据**：最终整支评审（`.superpowers/sdd/2026-08-02-semantic-syntax-v2/final-review.md`）must-fix #3 + follow-up 注册；纯台账修正，零代码变更。

---

## 2026-08-03 豁免台账重连（Task 0.6：React gate 豁免 → deferral 编号）

**背景**：T0.4 门禁收窄后 `scripts/verify-react.exemptions` 为 18 个根因类（79 条路径级豁免，166 tsc + 121 eslint），但 13 个根因类未在台账登记——豁免无法追溯到修复任务。本批补登（全部带触发条件），编号与豁免清单第 5 列一一对应；审计脚本 `scripts/check_exemption_ledger.sh` 强制无「无编号」豁免。已有映射：TS2588→#36（let-mut）、TS2304 内联 store→#37（CartStore）、TS2304/TS2322/TS2339 结构体类→#49（用户类型发射）。

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 61 | **[MEDIUM]** React 事件处理器 prop 可选调用未发射（TS2722 → 25 实例/12 条豁免：TabBar/EmitEventChild/SignalPatternsDemo 等 + I2 新增 AboutPage/HomePage——PageSwitcherDemo fixture 的 `event onSwitch` 声明） | Task 5.6（事件处理器 `?.()` 安全调用发射） | 豁免台账 T1.6 事件 prop 调用点；纯类型错误清除，零行为风险 |
| 62 | **[MEDIUM]** bind:value 回调 arity 不匹配（TS2554 → 6 实例/2 条豁免：FormPage/FormControlsDemo） | bind:value 回调签名发射任务（5.x 表单控件批次） | 豁免台账 T1.6 回调 arity |
| 63 | **[MEDIUM]** React ref 访问未判空（TS18048 → 4 实例/1 条豁免：ElementRefDemo） | BindingRef 完整版（计划 P4 标注 5.5 后长期项——触发条件：下次 IR 变更时评估） | 豁免台账 T1.6 ref 访问 |
| 64 | **[MEDIUM]** SetStateAction 类型转换不匹配（TS2345 → 14 实例/8 条豁免：Option/void/unknown/null 值、bind:value string→number 强转、Record 参数字面量） | 表达式类型化任务（值域转换发射） | 豁免台账 T1.6 |
| 65 | **[LOW]** 无重叠比较（TS2367 → 2 实例/2 条豁免：DataFetching fetch typing、ComputedFnFormDemo 字面量比较） | fetch typing + const folding 类型化 | 豁免台账 T1.6 |
| 66 | **[MEDIUM]** CSS 自定义属性（--var）被 React.CSSProperties 拒绝（TS2353 → 14 实例/4 条豁免：DesignTokenPanel/CssVarsDemo 等） | style 通道任务（CSS custom properties 发射；v2 T1.3 残余） | 豁免台账 C2 类（style 通道） |
| 67 | **[MEDIUM]** implicit-any 残留（TS7006 45 + TS7005 2 + TS7034 1 实例/9 条豁免 + 测试 fixture `FIXTURE_STRICT_ALLOWANCE` 30 实例——`generated_code_validation.rs` 注释引用本条） | 随 #49（React 用户类型发射任务）覆盖部分 + 生成物类型化收尾 | 豁免台账 T1.6 implicit-any；fixture 门禁同根因（T0.4 遗留协调点） |
| 68 | **[MEDIUM]** react-hooks/exhaustive-deps 缺失/新数组依赖（91 实例/11 条豁免：EasingFunctionsDemo/KeyframeAnimationDemo 等） | 随 Task 2.6（cleanup 依赖收集 + _M 表提升）+ 5.x | 豁免台账 T1.6 依赖数组发射 |
| 69 | **[LOW]** 生成物 ES5 var 残留（no-var → 10 实例/1 条豁免：useDesignTokens.ts 共享 ES5 sanitizer 单源双输出） | 随 Task 2.11（ES5 var→let 生成物清理批次） | 豁免台账 T3.2 |
| 70 | **[LOW]** 生成未用标识符（@typescript-eslint/no-unused-vars → 17 实例/11 条豁免：_b/_i） | 随 Task 2.11（未用标识符清理批次） | 豁免台账 T1.6 |
| 71 | **[LOW]** 生成注解与正则残留（no-explicit-any 1 + no-control-regex 2 实例/2 条豁免：FnFormatter any 注解、StdLibShowcase 控制字符 regex） | 生成物类型化收尾（随 #67 类清理批次） | 豁免台账 T1.6 |
| 72 | **[MEDIUM]** mutated-unread 信号 setter 未发射（TS2304 → 2 实例/1 条豁免：AdvancedPseudoDemo setVariant——信号被 mutate 但未被 read 时 setter 随 DCE 消失） | 信号 setter 发射正确性任务（DCE 保留被调用 setter） | 豁免台账 T1.6 |
| 73 | **[MEDIUM]** 事件数据与值类型化残留（TS2339 event-data 10 实例/3 条豁免：ElementRefDemo/SlotBox/EventsDemo EventTarget/MouseEvent props；TS2322 Option 值强转 2 实例/2 条豁免：DataSubmit/Modal string vs number；TS2322 事件处理器 nullability 2 实例/2 条豁免：EventsDemo onAnimation* / EmitEventDemo emit param） | 事件数据类型化任务（emit 数据 + 事件 prop 类型发射） | 豁免台账 T3.3/T1.6 非结构体残余 |

---

## 2026-08-04 覆盖地图登记（Task 1.5：emission-surface coverage map）

**新增**：#74（Task 1.5 覆盖地图枚举发现的唯一注入面缺口——只登记不修，修复排期由编排层裁决）。

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 74 | **[LOW]** component `style {}` 块属性值无语料插入点（StyleRuleIR.properties——parser/top_level/component.rs:177-180 → lower/component.rs:184-246；发射路径 React/Vue `generate_css_style_block`（omnilang-core/codegen/helpers.rs:142-157）、OmniView `emit_style`（omniview/view/mod.rs:114-131），均经 `format_css_value`→`sanitize_css_value`） | 语料新增 style-block 插入点任务（injection.rs POINTS + emission_surface.rs 同步登记） | 防御已存在且有专属测试（css_value_rejects_structural_characters 等），缺口在端到端载荷 canary 层——语料 style 点只覆盖 inline style，不覆盖 component style 块；若未来 style 块发射路径被改写绕过 sanitize，语料不会变红（Task 1.5 覆盖地图唯一登记的注入面缺口） |

---

## 2026-08-04 I2 跟进登记（view-root 位置标志的未覆盖节点形态）

**新增**：#75（I2 审查 Minor-2——`in_jsx_children` 标志只接入 Conditional/ListRendering 两臂，Text/Interpolation/Slot 三臂仍无条件发射 JSX-children 形态；只登记不修）。

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 75 | **[LOW]** React 视图根/括号位置中的 Text/Interpolation/Slot 节点仍发射 JSX-children 形态（`{'...'}`、`{expr}`、`{name ?? (fallback)}`——view.rs 的 Text/Interpolation/Slot 三臂无条件容器花括号，I2 的 `in_jsx_children` 标志未接入）。**Minor-2 语义细节**：Interpolation 在 `return ( ... )` 根部的 `{title}` 恰好是合法 TS（对象字面量简写）——不会报语法错误，而是**静默语义错误**（渲染成 `{ title: undefined }` 之类对象），比语法错误更隐蔽；Text 的 `{'...'}` 与 Slot 的 `{name ?? ...}` 在括号位置则是 TS 语法错误（与 I2 修复前同类） | 新增视图根/分支体直接为裸文本、裸插值或裸 slot 的 fixture（现语料零触发）；或该三臂发射逻辑改写时 | 静默错误渲染或 TS 语法错误，均无 fixture 暴露；修复方向：三臂按 `in_jsx_children` 门控，非 children 位置发射 `'text'`/裸表达式/`name ?? (fallback)` 形态 |

---

## 2026-08-04 Task 4.10 审查登记（React 侧 stdlib 方法翻译缺口）

**新增**：#76（Task 4.10 审查 Important-2——React `translate_method_call` 未翻译文档 floor 方法；Task 4.10 方法表按语言 floor 收录后这些名字零诊断 + React 运行时 TypeError。只登记不修，排期由编排层裁决。splice 同族缺口已随 I-1 修复——共享表补 `sublist` 组合臂，不再登记）。

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 76 | **[MEDIUM]** React 后端未翻译文档 floor 方法 `to_uppercase`/`to_lowercase`/`is_empty`/`abs`（`omnilang/src/codegen/react/expr.rs` `translate_method_call` 落默认 `_` 臂 verbatim 发射；OmniView 经共享 stdlib 表、Dart 经 `format_method_call`（crates/omnilang-core/src/codegen/shared/expr.rs:816-818）均已翻译） | 用户按 docs/omniview/12-standard-library.md §内置方法 写 `s.to_uppercase()`/`n.abs()`——Task 4.10 方法表已背书这些名字 → 编译零诊断 | React 运行时 TypeError（`s.to_uppercase is not a function`；`abs` 需 `Math.abs` 语义）；修复方向：`translate_method_call` 补 `to_uppercase`→`toUpperCase`、`to_lowercase`→`toLowerCase`、`is_empty`→`length === 0`、`abs`→`Math.abs(...)` 四臂（镜像 Dart 表） |

---

## 跟踪约定

- 每个推迟项在重新评估触发条件满足时重新打开
- 推迟项**不是**废弃项——它们是"现在不做，等时机成熟再做"的决策
- 本文件在每次 Sprint 回顾时更新状态
- 新增推迟项追加到本文件末尾

---

> 来源：`.architecture/reviews/plan-review-2026-07-26.md`（实施策略专家 + 务实执行者 + 系统架构师的 3 专家评审）
> 更新：ADR-006（2026-07-27）重新分类全部 4 项推迟项

---

## 2026-08-04 CodeFlowCanvas E2E 缺陷修复轮登记（ADR-061）

**新增**：#77（ADR-061 修复轮终审 parked 观察项——均不阻塞，登记留档）。

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 77 | **[LOW]** CodeFlowCanvas 修复轮 parked 项 4 条：(a) `removeElement`-undo 不恢复被删子树后代元素（恢复 wire 但元素缺失→xyflow 不可见边；codeflow 元素全扁平实际不可达，view 树 conditional 场景重载解析重建自愈，不落盘）；(b) 嵌套 condition 为外层分支末句且后接外层 elif 时续接循环可能产生双 wire（既有行为，同对去重已挡）；(c) validate-sources 测试用例 4 用 mockRejectedValue 而绑定同步（应 mockImplementation throw，测试空转未覆盖 catch 分支——一行级）；(d) materializeFnGraphs 的 persistedPos 直接引用共享（无原地修改路径，建议 spread 拷贝——一行级） | (a)(b) 触发即记录，无修复排期；(c)(d) 下次触碰相关文件时顺手修 | (a)(b) 视觉/连线完整性边界；(c) 测试诚信；(d) 不可变惯例一致性 |

> 另注：`task-5-report.md` 误入 git 跟踪（`.superpowers/sdd/.gitignore` 为 `*` 全忽略，仅此一个报告被跟踪）——`git rm --cached` 即可清理，未登记编号。
