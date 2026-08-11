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
| 28 | **[MEDIUM]** `Map.set` 方法调用无后端实现 + `Map<String, T>` 返回注解未被类型检查器解析 | 用户代码需要 Map 就地变更 / 泛型 Map 返回注解 | ① `map.set(k, v)`：MethodCallExpr 无 `set` 翻译臂，OmniView/React 发射原生 `.set` → 运行时 `TypeError: map.set is not a function`。demo 已改写为字面量构建 `{ "key": value }`（datetime_demo/math_ext/string_ext/error_handling），Map 读取 `.get()` 正常。② `-> Map<String, i32>` 返回注解：`parse_type_string`（semantic/types.rs:73）不解析泛型、回退 `Named`，与字面量推断的 `Map<String, Any>` 不兼容 → E0010；demo 已去掉此类返回注解。修复方向：codegen 增加 `set` 方法臂（Object.assign/spread）+ `parse_type_string` 泛型解析。**2026-08-03 终验复核（T7.3）**：计划「Map.set 随域 8 处理」未落地——T1.2/1.5 覆盖数组方法（filter/map/count/push/pop/sort/splice）与 `contains` map-aware（T1.6），但 `set` 翻译臂三后端均无（react/expr.rs `translate_method_call` 落默认臂，omnilang-core JSON/OmniView/Flutter 方法表同无）；实编译探针 `m.set("key", 2)` 逐字发射进 index.js:317 → 运行期 `TypeError`。保持 defer-with-trigger。**2026-08-05（v3 Task 6.7）**：设计文档 §7 deferrals 行已同步为本状态（未落地 → 移出 v2 实现范围、保持 defer-with-trigger），不再宣称「随域 8 处理」完成 |
| 29 | **[LOW]** encode hex/base64 仅覆盖 Latin1 | 非 ASCII 输入 | JS 臂 `charCodeAt`/`btoa` 对非 Latin1 字符失真（demo 仅用 ASCII——encode_demo 注释已注明）；修复需 TextEncoder/码点分片 |
| 30 | ~~**[HIGH]** match Flutter 变体臂 + 本地类型发射（learn-omniview Flutter 输出不可编译）~~ | ~~展示项目纳入 codegen-validate 门禁后~~ | ✅ **2026-08-02 已关闭（Task 1.4 附 scoped 证据）**：原缺口——match 变体臂无运行时字段支撑（`switch (n._variant)`）+ 本地类型（`Rating` 类）发射缺失——2026-08-02 实测 `MatchBreadthDemo.dart:649-666`：`Undefined class 'Rating'` / `Undefined name 'level'` / `return_of_invalid_type`。修复：① IR 合并漏掉 `module.enums`（`analyse.rs merge_file_ir_into` 只合并 structs/type_aliases，tagged union 被静默丢弃——根因），补 `existing.enums.extend(...)`；② Flutter 新增 `types.rs`：被引用联合发射进合并 `types.dart`（`Rating.Star({ required this.level })` + 公开 `variant` 判别字段——`_` 前缀成员跨库不可见）+ 组件/store 按需 `import './types.dart'`；③ `DartFormatExpr::format_match`：变体臂绑定字段（`final level = _v.level;`，未用绑定 `_` 前缀）、无通配符时追加 `default: throw StateError('Unmatched variant');`（消除 `return_of_invalid_type`）、IIFE 绑定 `final _v = <subject>`；④ Dart 家族补 `to_string` → `.toString()` 方法映射（镜像 React，match 臂体内字符串拼接可运行）。scoped 测试 `match_variant_arms_flutter_compiles`（`generated_code_validation.rs`）：全项目编译 → `MatchBreadthDemo.dart` 专项 `dart analyze` 无 Rating/level/return_of_invalid_type 错误（实测该文件剩余错误仅 Phase 5 类别：Curves 缓动名/重复 motion controller/String→Color）。**门禁 `learn_omniview_flutter_output_passes_dart_analyze` 保持 `#[ignore]`**——全项目仍 RED 于 Phase 5 Flutter 批次缺口（Curves 缓动名 312、重复 motion controller 225、stdlib 方法发射 202、String→Color props 84 等，共 1125 错误/21 类，见 compilability 评审），un-ignore 条件 = Task 5.5/5.6/5.7 完成后全量 `dart analyze` 0 错误（**该条件已于 T5.8a 达成**：learn-omniview Flutter 全量 0 错误 0 警告，门禁 5/5 un-ignore 绿，T7.3 终验复核确认）。② 语义 DSL 时代记录（ADR-054:88,144、compilability 评审 MEDIUM-10、design doc §7、Task 1.4）均以 **#30** 编号本条 |
| 31 | **[LOW]** `math_random_int` 区间退化未防御 | `hi - lo + 1 == 0` | JS `Math.random()*(0)` 恒返回 lo；Rust `rem_euclid(0)` 与 Dart `nextInt(0)` 会 panic/抛错。修复：codegen 夹取最小区间 |
| 32 | **[MEDIUM]** 非 pub 模块函数不被任何后端发射 | pub 函数调用同模块私有辅助函数 | `ir::all_functions()`（ir/types.rs）过滤 `Visibility::Public`，OmniView/React 等后端只发射 pub 函数——被 pub 函数调用的私有辅助函数编译产物中缺失 → 运行时 ReferenceError（async_demo.omni 的 task_a/task_b/slow_task/fast_task 已改 pub 规避）。修复：按调用图保留被引用函数，或后端发射全部模块函数 |
| 33 | **[LOW]** OmniView 输出嵌套 `#root` 结构缺陷 | 编辑器/WASM 桥接层消费 `output.html` 的方式 | 静态 body.html 含 `<div id="root" data-component="App">` 包装，JS `document.getElementById("root").innerHTML` 模板又包一层 → 运行时 2 个嵌套 `#root`（`[data-component="App"]` 匹配 2 元素）。功能无碍（2026-08-02 两轮 + 2026-08-03 终验浏览器全量测试 0 errors），但结构不干净且 `[data-component="App"]` 查询命中 2 元素。修复方向：`emit_dom_and_bindings` 的 innerHTML 模板剥离外层 root 包装（或 body.html 不再含包装）——涉及 WASM/editor 共用输出，需先验证两端消费方式（2026-08-02 发现，ADR-055 记录）。**2026-08-03 终验复核（T7.3）**：结构依旧——`emit_dom_and_bindings`（omniview/view/mod.rs:90）innerHTML 模板仍以 `<div id="root" data-component="App">` 开头（实编译探针 index.js:324），与静态 index.html/body.html 包装重复；T5.5（Flutter @for）/T7.1a（OmniView keyed rebind）未触及此面。保持 defer-with-trigger。**2026-08-05（v3 Task 6.7）**：设计文档 §7 deferrals 行已同步为本状态（未落地 → 移出 v2 实现范围、保持 defer-with-trigger），不再宣称「随域 6/终验处理」完成 |
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
- **新语法进 BNF 前必须走语法生命周期制度**（`docs/superpowers/specs/syntax-lifecycle.md`——提案 → spike → 语料试点 → 收口评审；批准标准：≥1 demo 使用 + 三目标门禁 + 使用基准预估）；收口评审未通过 → defer-with-trigger 入本台账（模板引用见文末 2026-08-05 登记节）

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

---

## 2026-08-05 语法生命周期制度登记（v3 Phase 7 设计层制度 Task 7.4——设计 P6）

**制度（非推迟项，模板引用）**：新语法进 BNF 前必须走语法生命周期（`docs/superpowers/specs/syntax-lifecycle.md`）——提案 → spike → 语料试点 → 收口评审 → BNF 批准；批准标准：≥1 demo 使用 + 三目标门禁 + 使用基准预估。`scripts/verify-syntax-lifecycle.py` 强制已批准语法必含「提案 → 试点 → 批准」三段记录 + 试点文件存在性（docs-gate 同批）。本台账凡「新语法/新表面」触发条件一律引用该模板；收口评审未通过 → defer-with-trigger 入本台账。

**动因（设计 P6）**：v2 域 8 新增语言方法 filter/map/count 0/93 语料消费者直接进 BNF（demo 全走 stdlib array_filter/array_map，v3 Task 4.6 prove-or-retire 才迁移 4 处）——删除有纪律（硬诊断 + 迁移提示）、新增没有；本制度把新增纳入与删除对称的纪律：先进语料，再进 BNF。

**补档（一次性豁免）**：v2 现有语法按「已批准」反向登记（源：7.1 登记册 + 7.2 语义全集 + v2 设计文档 §5 逐域验证清单），12 语法组已登记于生命周期文档登记册。

---

## 2026-08-05 处置同步（v3 Phase 6 文档治理 Task 6.7——设计文档 §7 承诺处置）

**同步**：#28/#33 台账状态不变（open，defer-with-trigger，行内已附 T7.3 终验证据）；v2 设计文档（`docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`）§7 deferrals 行与状态行已按本台账事实改写——原「#28（Map.set）随域 8 处理；#33（嵌套 #root）随域 6/终验处理」在文档标 Implemented 的语境下构成过度宣称（T7.3 终验复核确认两项均未落地），现标注为「未落地 → 移出 v2 实现范围、保持 defer-with-trigger」，两权威源不再互相矛盾。纯文档治理，零代码变更。

---

## 2026-08-05 v3 终验处置（Task 8.1b——v3 fix round 收口）

**来源**：终验矩阵 `task-8.1a-acceptance.md` / `task-8.1a-report.md`（PASS×4 / 部分×3 / FAIL×1）+ 评审处置 `.architecture/reviews/feature-omniview-semantic-syntax-compilability-v2.md`（v3 处置记录节）+ ADR-062。

### 关闭确认（计划承诺逐条核对）

| # | 台账行 | 关闭证据（实测/commit） | 计划承诺核对 |
|---|--------|----------------------|-------------|
| 36 | React `let mut` 发射为 `const`（BUG-A） | **已关闭**——Task 5.1 `db68d812`（LetStmtIR.mut 字段 + 三后端发射 + prefer-const re-enable + TS2588 零残留）；先于 IR 修复的 2.1 廉价路径 `edb19786` 已止损 | 5.1「#36 关闭，D-I 第二步」✅ |
| 37 | React 内联 store 不发射（BUG-C） | **已关闭**——Task 5.2 `a9526faf`（inline store 降级为模块级 Zustand store 发射 + 切片订阅，CartStore.addItem 可执行）；2.2 `86f820be` 先行硬诊断 | 5.2「inline store 入 IR」✅ |
| 49 | React 不发射用户 struct | **已关闭**——Task 5.3 `96c8e297`（StructIR 发射 + React TS 类型——豁免 burn-down ~30 条目） | 5.3「用户 struct 类型发射」✅ |
| 50 | Flutter 非 ref 元素 bind:value 单向 no-op | **已关闭**——Task 3.3 `8c53942d`+`ba93004f`（onChanged 回写 + controller_covers 三层判定 + Checkbox/Switch/Slider 接线）+ 3.6 `5dc4d322`+`3bf6496f`（主题选择器/四态）；widget harness 实测 bind:value 回写 | 3.3/3.6（D2「唯一双向」兑现）✅（brief 台账标注「ArrayMethod floor @ 95699b6d」系错配——#50 与 #55 不同根因，已按实测分开核对） |
| 55 | 数组方法未映射后端 verbatim 发射 | **已关闭**——Task 4.8 `95699b6d`+`206532a7`（out-of-floor 后端 ArrayMethod floor 门 E_CODEGEN——mirror BlockExpr 先例） | 4.8「ArrayMethod floor」✅ |
| 60 | OmniView 44-test 浏览器套件未接入 CI | **已关闭**——Task 0.1 `a0b11b63`（omniview config 接线独立 e2e job；webServer `--directory` 修复；44 passed 1 skipped） | 0.1「#60 入 CI」✅ |

### 新增登记（defer-with-trigger，均带触发条件）

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 78 | **[LOW]** OmniView `stmt_to_js_inline` 缺 `EventEmitStmtIR` 臂——`emit` 出现在 BlockExpr 表达式内（如 computed fn 体 block）退化为 `/* unhandled stmt */` + warnings 条目（K1 修复轮观察 1；**pre-existing**——降级臂自 `09c18415`（2026-08-01）即有，非 K1 引入；K1 的组件化事件解析只覆盖 fn 体语句路径与 CallExpr 形式） | BlockExpr 内使用 `emit` 的语料/demo 出现（现语料零触发）；或 stmt_to_js_inline 发射逻辑改写时 | 生成 JS 注释占位而非事件发射——静默不可用输出（有 warning 无硬诊断）；修复方向：加 EventEmitStmtIR 臂复用 `resolve_event_emit` |
| 79 | **[LOW]** `test-results-omniview.json` 未入 .gitignore（playwright json reporter 产物；相邻 `test-results.json` 已覆盖——K1 修复轮观察 2，每次跑 omniview 套件留下 untracked 文件） | 下次触碰 .gitignore 或 CI e2e 产物清理任务 | untracked 噪音文件；无功能影响 |
| 80 | **[LOW]** golden 镜像 8 份陈旧拷贝（`tests/fixtures/golden/test/`——纯文档快照，无测试读取：golden_parse/cross_target 只读 MANIFEST.md，路径解析到 live 树）。7.5b-S1 后实测：11 个 moved 文件仍滞留镜像 `components/`（10 个字节一致但路径形态错误 + array_ext 字节陈旧），镜像 `lib/` 缺 11 文件；7.5b-LS 报告时点计数 8 份 | 镜像维护任务（`diff -rq` 重同步）或镜像删除/改造 | 文档快照失真（MANIFEST 已声明 live 树为权威）；无测试影响 |
| 81 | **[MEDIUM]** Flutter warnings 874 → 目标 <300 未达（8.1a G8：cap 879 断言内 PASS 但计划目标未兑现）。构成演变：1004（0.5 基线）→ 997（3.3）→ 879（cap）→ 874（C1 审计）→ **700（C2b 语义/映射，d915377a）→ 671（C2d 按唯一名合并，span 线程化 671/671）**——C2d 实测分解：#1 46（唯一名）/ #2 204（唯一使用类）/ #6 225（定义点 1/类）/ #3 30 / #4 23 / #12 cursor 39 / OTHER 值跳过 86 / 其余 18。**按唯一名合并已到诚实下限——<300 仅剩语义燃尽路径（#2+#6 TextStyle/layout 映射，−429 → 242）** | **已裁决（ADR-066 D4，defer-with-trigger）**：671 为诚实下限（诊断合并后残余均真实降级/语义缺口）；<300 唯一路径 #2+#6 TextStyle/layout 语义实现（−429 → 242）defer（Pragmatic Necessity 4/Complexity 7/Ratio 1.75）——触发条件 = Flutter 产物文字样式成为实际使用场景，或 motion 域重设计任务 | 目标更新 1004→671（−33%）为燃尽期正式记录值（门禁 cap 879→700→671 单调下降）；计划 M3 部分——门禁绿、目标按裁决更新 |
| 82 | **[MEDIUM]** 设计层（M7 部分项）剩余机制缺口：生命周期文档语料路径与文件移动的同步机制（D1 暴露：7.5b-S1 移动 11 文件后 spec 路径人工同步漏掉——`verify-syntax-lifecycle.py` 只验路径存在性，文件移动与 spec 更新之间无强制绑定；8.1b 已修路径本身，机制缺口保留） | 下次 learn-omniview/src 结构移动/重构任务；或 lifecycle 门禁脚本升级（路径映射表/自动解析） | 文件移动后 spec 漂移到下一次门禁运行才暴露（CI 级红，非静默） |
| 83 | **[LOW]** motion-in-@for 逐项实例化实现（D-H 长期项）——4.9 `491857d6` 已落编译诊断（WMotion warning，双目标诚实降级）；「@for 内 motion 逐项实例化」（React 共享 ref 只动最后项 / Flutter 全项同播的语义修复）为长期实现项 | 用户对 @for 内 motion 的逐项实例化语义的实际需求（现语料零命中）；或 motion 域重设计任务 | 诊断已消除静默分叉；逐项实现前 @for 内 motion 保持 warning 降级 |

### #76 状态复核（保持 open）

- **#76**（React stdlib 方法翻译缺口 `to_uppercase`/`to_lowercase`/`is_empty`/`abs`）：**保持 open 不动**。台账描述复核一致——同族 `splice` 缺口已随 `35dc15ef`（fix(codegen): translate splice on shared table）修复，台账「splice 同族缺口已随 I-1 修复」与事实吻合；#76 本体 4 方法仍落默认臂，defer-with-trigger 不变。

**验证**：本更新仅记录 8.1a 实测事实 + 8.1b 治理收口；零代码变更（D1/D2 修复见终验 commit）。

---

## 2026-08-06 遗留清理轮处置（Task A/B/C——#78/#79/#80/#82 关闭 + #81 裁决登记）

**来源**：遗留轮任务简报/报告（`.superpowers/sdd/2026-08-03-semantic-syntax-v3-fixes/task-leA/leB/leC*`）+ ADR-066（c4baa7cf）。

### 关闭确认（计划承诺逐条核对）

| # | 台账行 | 关闭证据（实测/commit） | 计划承诺核对 |
|---|--------|----------------------|-------------|
| 78 | OmniView `stmt_to_js_inline` 缺 `EventEmitStmtIR` 臂 | **已关闭**——`e0e7df16`（emit 在 BlockExpr 全链路合法调查（parser/semantic/lower 六处证据）；stmt_to_js_inline 加 EventEmitStmtIR 臂复用 `resolve_event_emit`，与 emit_statement 字节同形；2 回归测试断言真实发射串；review Approved） | Task B ✅ |
| 79 | `test-results-omniview.json` 未入 .gitignore | **已关闭**——`3ac45290`（gitignore 追加 omniview json reporter 产物 + flutter 验证残留目录） | Task A G2 ✅ |
| 80 | golden 镜像陈旧拷贝 | **已关闭**——`42d304b0`（`diff -rq` 重同步：11 个 moved 文件 components→lib + 2 新增组件 + 19 份字节漂移修正；96=96 零漂移） | Task A G3 ✅ |
| 82 | lifecycle 文档语料路径与文件移动同步机制缺口 | **已关闭**——`ccd901ed`（方案 a：live 树 canonical 校验 + 移动文件定位提示；`verify-syntax-lifecycle.py` 同批接线，含负向测试） | Task A G4 ✅（机制缺口关闭；触发即重评估） |

### #81 目标裁决登记（ADR-066 D4，行状态已同步）

- **#81**（Flutter warnings <300 未达）：**裁决 = defer-with-trigger**——671 为诚实下限（诊断合并后残余均真实降级/语义缺口；671/671 真实 file:line、0 占位文件）；<300 唯一可达路径 = #2/#6 TextStyle/layout 语义实现（−429 → 242），**defer**（Pragmatic Necessity 4/Complexity 7/Ratio 1.75）；**触发条件 = Flutter 产物文字样式成为实际使用场景，或 motion 域重设计任务**；目标更新 1004→671（−33%）为燃尽期正式记录值（门禁 cap 879→700→671 单调下降，deferrals.md:337 行已同步）。**C2d scoped re-review（c9163084..9391fa13）4/4 ADDRESSED → MERGE-READY**（独立验证：MEDIUM-1 `MotionIntent.source_file` + `merge_file_ir_into` 同款补丁 + flutter 锚点改读真实文件；MEDIUM-2 锚点测试逐条断言与语料行号核对一致且旧行为必失败；LOW-1 #6 族 (message,file) 键（lib.rs:461-465）+ #1/#2 族 message 键（lib.rs:457-460，accepted 裁决记录于 lib.rs:435-439）；LOW-2 死 From impls 零残留——全量 cargo test exit 0）。

### #83 状态复核（保持 open）

- **#83**（motion-in-@for 逐项实例化）：**保持 open 不动**——遗留轮评估维持建议 defer（现语料零命中 + 双目标大实现）；触发条件已记录，最终裁决随汇报给用户。

### #72 关闭登记（v4 Task 2.2——只写信号保留 useState）

- **#72**（mutated-unread 信号 setter 未发射——TS2304×2 / AdvancedPseudoDemo setVariant：信号被 mutate 但从未被 read 时，gen_state_hooks 的「不在 consumed 集即跳过」误杀 hook，mutation 发射引用未声明名，点击即 ReferenceError）：**已关闭**——v4 Task 2.2 commit「fix(codegen-react): 只写信号保留 useState——悬空 setter 崩溃修复 + TS2304 豁免删除」：`gen_state_hooks` skip 条件改为「读与写皆缺失才跳过」+ 写专用形状 `const [, setVariant] = useState(...)`（省略值名无 unused 变量）；pruned 信号（Task 2.11 DCE 契约——值未消费且 mutation 被剪枝）仍跳过，`has_mutable_states` 计入写专用排除 pruned（import 与 hook 发射严格一致）。**验证**：单元 `write_only_signal_keeps_state_hook` + DCE 回归 `test_write_only_signal_is_pruned` 双绿；浏览器 H2「只写信号点击不崩溃」转绿（5/5 全量，H1 四断言回归绿）；tsc 87→85、TS2304×2 归零；豁免条目删除。

### #64 关闭登记（v4 Task 2.3——数字 bind:value Number 强制转换）

- **#64**（TS2345 类中 bind:value string→number 子类——FormControlsDemo `age`/`rangeVal`：数字 signal 的写回裸发射 `setAge(e.target.value)`，字符串喂进 `useState<number>`，类型谎言有运行时后果 `age+1 → "301"`）：**子类关闭**——v4 Task 2.3 commit「fix(codegen-react): 数字 bind:value Number() 强制转换——类型谎言修复（v4 2.3, TS/React H3）」：`view.rs` bind setter 发射处新增 `signal_is_numeric` 判定（`comp.signals` 类型查找——`TypeKind::Int/Float` 折叠名 + i8/u8/u16/u32/u64/isize/usize Named 名集合；Optional 已在 lowering 折叠为 `is_nullable` 天然覆盖，Union 按 `a | b` 名任一成员数字即数字），数字信号发射 `setX(Number(e.target.value))`，checked 恒 `e.target.checked`、字符串等非数字信号零变化。**验证**：单元 `numeric_bind_value_emits_number_coercion` 绿（含 `setQ(e.target.value)` 负向断言）；`cargo test -p omnilang --features codegen-react` 380/380；tsc 85→83、FormControlsDemo TS2345×2 归零、无新增；verify-react.sh GATE PASSED（豁免条目删除）。**注**：#64 其余子类（Option/void/unknown 值、Record 参数字面量）保持 open——豁免条目 5 条不动。

### #62 关闭登记（v4 Task 2.4——bind:value 回调 arity 空事件特判）

- **#62**（bind:value 回调 arity 不匹配——TS2554×6 / FormPage+FormControlsDemo：bind:value 与 @onChange 共存时 setter 分支把空事件对象 `{}` 硬塞给 0 参 handler（`handleSelect({})`），TS 严格模式报「0 参函数被传 1 参」）：**已关闭**——v4 Task 2.4 commit「fix(codegen-react): bind:value setter 分支空事件特判——TS2554×6 删除（v4 2.4, TS/React M3）」：`view.rs` FnRef setter 分支应用与无 setter 分支相同的 `event_obj == "{}"` 特判——事件对象为空时直调 `call()`（零参；`call` 已含 `?.` 可选调用，Task 5.6/#61），非空仍 `call(obj)`，无事件数据仍 `call(e)`（setter 与 handler 共用 e）；InlineFn setter 分支实测已对齐（`Object.assign(e, {})` 空事件特判 v3 期即有），补形状守卫测试锁定。**验证**：单元 `bind_value_handler_with_zero_arity_fn_skips_empty_obj`（RED 先红——`handleChange({})` 形状）+ InlineFn 守卫 `bind_value_inline_handler_skips_empty_event_assign` 双绿；`cargo test -p omnilang --features codegen-react` 382/382（lib）+ cross_target 7/7；tsc 83→77、TS2554×6 归零、无新增；verify-react.sh GATE PASSED（豁免条目删除）。

---

## 2026-08-06（v4 修复轮收口）

- **#43 unused-import 假阳性 123 条**：已修复（e454f907，08-04——123→6 全真实）——台账补关（此前缺关闭注记，v4 review implementation-strategist MEDIUM）
- **#61 TS2722 可选调用**：已修复（Task 5.6/deferral #61——`?.` 可选调用发射，react view.rs，d88161de）——豁免清单 REMOVED 注记已删
- **#69 ES5-var**：已修复（2.11 var→let 批次）——豁免 REMOVED 注记已删
- **#70 unused-identifiers**：已修复（2.11 标识符清理批次）——豁免 REMOVED 注记已删
- **#62 TS2554**：已关闭（v4 Task 2.4——bind:value setter 分支空事件特判，e34f010a）
- **#64 TS2345**：已关闭（数字 bind:value 子类——v4 Task 2.3 Number() 强制转换，c5a41da1；**其余子类保持 open——豁免条目 5 条不动**）
- **#72 TS2304**：已关闭（v4 Task 2.2——只写信号保留 useState，ba0d2474）
- **#42 parser 分歧**：触发重写（原「任一分歧致用户困惑」不可证伪）→「任一分歧被用户或评审报告」；7 项分歧批次折叠为单一对齐任务（对齐排期随 v4 台账节）
- **新增 deferral**：
  - **#84 velocity_max**（v4 Task 5.8 移除）——触发：「demo 或用户需要 fling 速度物理」；git 历史保留完整实现；syntax-lifecycle 门禁为自然重入路径
  - **#85 a11y 动态可访问名称**（domain MEDIUM-1）——label/role 当前限字符串字面量；触发：「真实应用需要 i18n 动态 label」
  - **#86 React 动态 URL 运行时消毒**（security HIGH-1，用户 2026-08-06 范围裁定 defer）——React 原生阻断 javascript:/vbscript:（ReactDOM isJavaScriptProtocol），主向量已关闭；残余面 = data: URL + 非 React 目标。**触发：「真实用户输入流进 URL 绑定」或「Vue/Dioxus 生产使用」（届时按 Task 1.1/1.3 正文执行，含 sanitizeUrl helper + node 行为 harness）**。ADR-056 语料计数更新仍按 Task 0.1 执行
  - **#87 Vue/Dioxus 语义 floor 诊断**（systems HIGH-1，用户范围裁定 defer）——非目标后端；ADR-052:94 契约违约已记录（本行）。**触发：「vue/dioxus 真实生产使用」**——届时按 Task 4.1 正文执行（E_CODEGEN 门 + floor_matrix 行）；触发前不投资

---

## 2026-08-06 learn-omniview 语义展示补全轮（#88-#90 注册）

**来源**：learn-omniview 展示项目语义覆盖补全轮（MapOperationsDemo 等新 demo）——Map 方法调用层边界实证（semantic-universe.md:137「未翻译操作在后端 verbatim」+ ArrayMethod 教训——floor 门 E_CODEGEN 于 v3 Task 4.8 补齐）。三缺口均零诊断（语法面合法、语义面运行期爆炸）——诚实降级原则要求登记触发条件，触发前不投资。

- **#88 Map.insert 方法调用层未翻译臂**——`m.insert(k, v)`：react/expr.rs `translate_method_call` 与 Dart 族 `format_method_call` 均落默认臂逐字发射——JS `m.insert(...)` 运行时 TypeError + tsc TS2339；Dart `Map` 无 `insert` → analyze 错误；Python/Rust 同名异义（dict 无 insert / HashMap 无 insert）→ 无跨目标保证。**同族 #28（Map.set）**——修复方向：三后端方法表补翻译臂（`Object.assign`/spread 系）+ `parse_type_string` 泛型，或按 ArrayMethod 教训补 E_CODEGEN floor 门。**触发：「真实应用需要 Map 就地变更」**；当前惯用法 = 完整字面量重新赋值（MapOperationsDemo 已展示，demo 头部注释与 skill §9.5 同步）。*实测（2026-08-06 probe，omnic 0.2.0）：React 发射 `m.insert("c", "3")` verbatim（无 tsc 拦截——类型检查器对方法调用层无方法表）；Dart 发射 `m.insert('c', '3')` verbatim（Dart `Map` 无此方法 → analyze error）。两 target 均编译静默、运行即错*
- **#89 @for 迭代 Map**——`@for item in m`：JS 族发射 `for (const item of <对象>)`，Map 字面量编译为普通对象、不可迭代 → 运行时 TypeError「not iterable」；Flutter 侧 Map 迭代产物为 MapEntry（与元素类型错配）。**触发：「真实应用需要 Map 遍历展示」**（届时设计键值对/值迭代的跨目标语义 + floor 门）；当前可读 Map 用 contains/get/len 组合（MapOperationsDemo 展示）。*实测（2026-08-06 probe）：React 发射 `__omni_iterable.length === 0` 守卫 + `__omni_iterable.map(...)`——普通对象 `.length` 为 undefined、`.map` 不存在 → 运行 TypeError；类型检查器不拦截（Map 类型在 view 迭代无校验）*
- **#90 Map spread**——`[...m]`：普通对象不可迭代 → 运行时 TypeError；`{...m}` 对象上下文展开产物为普通对象（非 Map，键序/形状无跨目标保证）。**触发：「真实应用需要 Map 合并/复制」**（届时补 spread 翻译臂或禁止并诊断）。*实测（2026-08-06 probe）：类型检查器先行拦截——`let arr: Array<String> = [...m]` 报 E0010「cannot assign 'Array<Map<String, String>>' to 'Array<String>'」（spread 的 Map 操作数被推导为元素类型）——比运行时 TypeError 更早，静态层面已堵*

## 2026-08-11 批次 2（导航路由）Task 10 收口轮（#91 注册）

**来源**：Task 10 demo 迁移 + Playwright 门禁轮（PageSwitcherDemo/NavigationDemo 迁移、main.omni 接线）——verify-react.sh 门禁残余 6 tsc + 2 eslint 非豁免，经 **HEAD 基线实证**（`git stash` 我的全部改动 → HEAD codegen 重建到临时目录 → `diff` 产物逐字节一致 → `stash pop` 恢复）判定为预存在错误（并行会话遗留孤儿生成物），非本批次引入。

- **#91 ElementRefDemo @ref style 属性 number 直写 + SlotBox PointerEvent 无映射字段 `?? undefined` 兜底**——React 目标两个预存在类型错误簇：(a) ElementRefDemo.tsx TS2322×4：`ref.current.style.borderRadius = boxRadiusNum` 等 number 值直写 `CSSStyleDeclaration` string 属性（@ref 直写通道对 style 属性未做 number→string 转换）；(b) SlotBox.tsx TS2871×2 + no-constant-binary-expression×2：codegen 的 PointerEvent 字段映射对 clientX/clientY 无映射时输出 `undefined ?? undefined` 空值兜底（`e.nativeEvent.offsetX ?? 0` 系 @onClose 事件参数映射的兜底分支）。**触发：「React @ref 直写 style 数字属性真实使用」或「PointerEvent 事件参数真实消费」**（届时补 number→string 转换臂 + 无映射字段省略而非 `?? undefined`）；当前展示项目不消费该路径（守卫演示在 @route /admin/console，元素引用/槽位事件为历史 demo 未接线）。
