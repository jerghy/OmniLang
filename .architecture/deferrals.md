# 推迟项跟踪

> 创建：2026-07-27 | 最后更新：2026-08-01
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
| 7 | Cross-target regression test matrix | New backend added or paused backend reactivated | ⚠️ 2026-08-01 部分实施：CI `codegen-validate` 对 11 个 React fixture 跑 `tsc --noEmit` + Flutter store 文件跑 `dart analyze`（工具链缺失时 SKIP，见 ci.yml）；全后端矩阵（Vue/Dioxus/JS 等）仍待完成 |

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
| 14 | Stale doc comments referencing removed features | Doc audit | ⚠️ 2026-08-01 部分修复：`docs/omniview/06-templates.md` + `11-motion-animation.md` 已同步 ADR-052/053 语法（Task 10.1）；代码注释与其余文档待审计（见 #24） |

### 来自 ADR-034/035 (v4 Review Structural Fixes)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 15 | OmniViewGenerator decomposition into phase structs | Next major OmniView feature | 47-field struct with coupled phases |
| 16 | Complete React FormatExpr migration (remove direct variant handlers) | Next ExprIR variant addition | 3 variants handled directly, rest delegated to trait

### 来自 ADR-052/053（语义级 DSL，2026-08-01 会话）

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 17 | **[HIGH]** Flutter widget codegen 生成不可编译 Dart（6 处） | `dart analyze` 门禁扩展到全部生成文件 | ① 多元素根逗号拼接（Fragment 场景）② `ElevatedButton` 缺必需 `onPressed` ③ slot 回退 `Text(child: ...)` 非法命名参数 + `List<Widget>` 未判空 ④ `onPressed: (e)` 与 VoidCallback 不匹配 ⑤ `ref.watch(...select(...))` 在 `Text()` 中 int vs String ⑥ design 断言失败。现有 Flutter 测试断言了错误模式（8+ 处），修复需同步更新测试；`generated_code_validation` 目前仅白名单 store 文件 |
| 18 | **[MEDIUM]** AtFocus/AtHero parser 缺口 | 用户使用 `@focus` 文本内容 / AtHero 元素属性 | `is_text_token`（`parser/view/style.rs:564-604`）未含 AtFocus——文本中的字面 `@focus` 静默截断子节点 + hard error；AtHero token 无元素属性解析臂（hard error）。AtHero demo 文件已不在活项目中 |
| 19 | **[MEDIUM]** Drag 触发器无约束时失效 | 拖拽触发器恢复可用之前 | `parseConstraint("")` 返回 ±Infinity → `maxExtent=Infinity` → `progress=0` → 拖拽盒不位移（`react/motion/transforms.rs:276,301`）。旧 `@drag-constraints` 已按 ADR-053 移除；codegen 需从 keyframe 范围推导默认 extent，或演示文案软化（ElementRefDemo 已注明"当前拖动无位移"） |
| 20 | **[MEDIUM]** Style 读取未计入 lint 信号读取 | 用户仅通过 `style:` 使用信号 | `semantic/lints/ast_collectors.rs` 死代码 lint 不把 `ElementProp::Style` 计入信号读取——style-only 信号被误报 "never used"（React useState 已修复缺口的镜像） |
| 21 | **[MEDIUM]** 跨文件 slot 转发检测缺失 | 组件跨文件导入时转发 slot | E0006 slot 转发警告是模块局部的（`module_component_names` 只含当前文件组件）——导入的组件绕过该警告 |
| 22 | **[MEDIUM]** `--all-features` 测试编译失败（历史遗留） | `cargo test --all-features` | javascript/rust/vue/dioxus 测试文件使用旧 IR 字段（`SignalRefIR.owner`、`MutationIntent.field_object`）；`vue/tests.rs` 缺括号；dioxus feature 接线缺失 |
| 23 | **[MEDIUM]** omni-animation TS 运行时缺口 | 使用 kebab 缓动名或 translateX/Y/Z 关键帧 | `easing.ts` 无 kebab 别名（`power1-in` 降级为 linear）；`keyframes.ts` `TRANSFORM_MAP` 缺 translateX/Y/Z |
| 24 | **[MEDIUM]** 文档陈旧（docs/omniview） | 全面文档审计 | `06-templates.md` / `11-motion-animation.md` 已修复（Task 10.1，2026-08-01）；其余文档（`14-grammar-lang.md`、`15-grammar-view.md`、`05-components.md`、`07-events-and-bindings.md` 等）仍教授 ADR-052/053 前语法 |
| 25 | **[MEDIUM]** BNF 规范与 parser 语法分歧未同步 | 规范文档更新 | `2026-07-31-semantic-level-dsl-syntax-spec.md` 写 `cubic-bezier(num,...)`、`delay ':' expr`，且 `direction` 规则缺 `alternateReverse`；parser 实际接受 `cubic(x1:...,y1:...,x2:...,y2:...)`（`parser/expr/motion.rs:93`）、`delay <expr>` 空格形式（`parser/top_level/motion.rs:366`）、`alternateReverse`（demo 已用）——两份权威文档教授不同语法，需以 parser 为准同步规范 |
| 26 | **[MEDIUM]** `@if mode:` 过渡效果跨后端缺失 | mode 过渡效果落地前 | `transition_mode`（ADR-052 feature floor）仅 Flutter 的 `"wait"` 生成 `AnimatedSwitcher`（`codegen/flutter/view.rs:848`，`"fade"`/`"none"` 无效果）；React codegen 完全不读取（静默 no-op）；OmniView wait 路径查找 `__omni_animations.get(oldNid + "_leave")`（`codegen/omniview/editor.rs:127`）但无任何 `_leave` 注册（@leave 不存在）——静默行为差异违反 ADR-021 floor 原则，需实现或出诊断 |

---

## 2026-08-01 状态更新（语义级 DSL 实施完成）

**已关闭**：#2（Flutter GlobalKey）、#4（ref_names 线程化）、#5（element_tag 被语义 ref 系统取代）、#11（IR `#[non_exhaustive]` 迁移）、#12（@focus 死代码）、#13（lexer 子属性 token 删除——由删除测试 `test_deleted_sub_attribute_tokens_lex_as_ident` 验证）。原因见上表各条目备注。

**部分实施**：#7（CI codegen-validate 门禁落地：tsc + dart analyze）、#14（2 个文档已修复）。

**新增**：#17-#24（本会话 8 项发现）+ #25/#26（10.1 代码质量评审发现：BNF 规范分歧、`@if mode:` 跨后端缺失）。其中 #17 为 HIGH（P0 目标生成不可编译代码，门禁尚未覆盖）；#26 涉及 ADR-021 floor 原则的静默行为差异。

**ADR-053 D7 完成项**（无对应条目、已随实施落地）：`paused_override`/`PlaybackIR.paused`/`@paused` 属性删除、React Hooks store 模式删除、demo 迁移（9.1-9.4）与 golden mirror 重同步（9.5）。

---

## 跟踪约定

- 每个推迟项在重新评估触发条件满足时重新打开
- 推迟项**不是**废弃项——它们是"现在不做，等时机成熟再做"的决策
- 本文件在每次 Sprint 回顾时更新状态
- 新增推迟项追加到本文件末尾

---

> 来源：`.architecture/reviews/plan-review-2026-07-26.md`（实施策略专家 + 务实执行者 + 系统架构师的 3 专家评审）
> 更新：ADR-006（2026-07-27）重新分类全部 4 项推迟项
