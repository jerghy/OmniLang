# 推迟项跟踪

> 创建：2026-07-27 | 最后更新：2026-07-27
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
| 2 | Flutter `GlobalKey` codegen for `@ref` | Flutter P0 commitment | All Flutter test fixtures pass `ref_binding: None` |
| 3 | Custom component ref forwarding (`forwardRef`) | Cross-component ref patterns | Not yet designed |
| 4 | `ref_names` threading through React store codegen (16 call sites) | User reports store + ref combination bugs | 16 call sites pass `&FxHashSet::default()` |
| 5 | `element_tag` population from ElementIR tag | Non-div elements with refs | Currently uses tag→type mapping (ADR-038 D3) |

### 来自 ADR-037 (Signal Batching)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 6 | Signal batching with microtask deferral | Profiling gate: real app, measurable frame drops, bottleneck confirmed | Synchronous notification on every set() |

### 来自 ADR-021 (Cross-Target Feature Parity)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 7 | Cross-target regression test matrix | New backend added or paused backend reactivated | 4 confirmed silent-drop bugs fixed; no automated guard |

### 来自 ADR-001 (Pipeline Architecture)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 8 | TypeKind recursive validation for `Named` types | Language adds generic structs with custom type params | Unvalidated type names pass through IR silently |
| 9 | Source map pipeline completion (non-OmniView backends) | React/Flutter debugging needs | target_spans empty for React/Flutter |
| 10 | Codegen trait full adoption (OmniView) | Next major IR refactoring | OmniView bypasses standard compile() path; see also ADR-013 |
| 11 | IR `#[non_exhaustive]` migration | Next ExprIR/StatementIR variant addition | Cascading compile errors across 10 backends |

### 来自 ADR-032 (Animation Scoping) — Parked

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 12 | Dead @focus JS codegen in React backend | React focus animation feature | mod.rs:127-144 unused code |
| 13 | Removed trigger tokens still in lexer | Lexer cleanup pass | token.rs, keywords.rs dead tokens |
| 14 | Stale doc comments referencing removed features | Doc audit | 5+ files with outdated animation docs |

### 来自 ADR-034/035 (v4 Review Structural Fixes)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 15 | OmniViewGenerator decomposition into phase structs | Next major OmniView feature | 47-field struct with coupled phases |
| 16 | Complete React FormatExpr migration (remove direct variant handlers) | Next ExprIR variant addition | 3 variants handled directly, rest delegated to trait

---

## 跟踪约定

- 每个推迟项在重新评估触发条件满足时重新打开
- 推迟项**不是**废弃项——它们是"现在不做，等时机成熟再做"的决策
- 本文件在每次 Sprint 回顾时更新状态
- 新增推迟项追加到本文件末尾

---

> 来源：`.architecture/reviews/plan-review-2026-07-26.md`（实施策略专家 + 务实执行者 + 系统架构师的 3 专家评审）
> 更新：ADR-006（2026-07-27）重新分类全部 4 项推迟项
