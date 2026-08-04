# OmniLang 设计文档索引

> **生成日期:** 2026-07-23
> **核实方式:** 通过 git 历史 + 实际代码（`omnilang/src/`、`poiesis/src/`、`test/projects/learn-omniview/`、`omnilang-wasm/`）交叉验证
> **用途:** 本项目所有设计文档的汇总索引，按功能域分类，标注每份文档的状态

---

## 项目事实速览

以下数据由代码和 git 直接验证（非估算）：

| 项目 | 数值 |
|------|------|
| learn-omniview `.omni` 源文件 | **93 个**（`src/components/` 59, `src/pages/` 19, `src/lib/` 7, `src/stores/` 4, 根 4） |
| 编译器 workspace 成员 | **4 个**：`omnic`, `omnilang`, `omnilang-wasm`, `poiesis/src-tauri` |
| 代码生成目标 | **10 个**：Python, JavaScript, TypeScript, Rust, Dart, React, Vue, Dioxus, Flutter, OmniView |
| 各后端独立目录 | **9 个**（TypeScript 无独立目录，可能与 JavaScript 共享） |
| 标准库已实现模块 | **8 个**：array, console, http, json, math, storage, string + tests |
| `omnilang` crate 类型 | `rlib`（纯 Rust 库，非 cdylib） |
| WASM 编译 | 独立 crate `omnilang-wasm`（cdylib + rlib，依赖 omnilang） |
| LSP 服务器 | Python 实现：`test/omnilang/lsp/server.py`（1026 行，11 项 LSP 特性） |
| 编译器关键字 | `theme` 和 `design` 是两个独立的一级关键字（均被 `TokenKind` 枚举和 parser 支持） |
| 设计文档总数 | ~100 份（spec 39 + plan 51 + core design 4 + analysis/report ~8） |

---

## 阅读指南

每份文档标注以下状态之一：

| 状态 | 含义 |
|------|------|
| ✅ 现行 | 文档与当前代码一致，是权威参考 |
| ⚠️ 部分过时 | 核心思路仍有效，但部分细节已偏离当前实现 |
| ❌ 过时 | 已被后续设计或实现取代，仅保留作为历史参考 |
| 🔨 实现中 | 设计已完成，代码实现进行中 |
| 📋 规划中 | 设计已完成，尚未开始实现 |

---

## 一、OmniLang 语言与编译器

### 1.1 编译器核心架构

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: omni-rust-compiler-design](../superpowers/specs/2026-06-15-omni-rust-compiler-design.md) | 2026-06-15 | ⚠️ 部分过时 | Rust 编译器初始设计（lexer→parser→semantic→IR→codegen）。核心管线已实现，但以下偏离: ① WASM 已分离为独立 `omnilang-wasm` crate（非 `omnilang` 内 `wasm.rs`）；② `omnilang` 仅为 `rlib`（非 cdylib）；③ 所有 codegen 后端已迁入 Rust |
| [spec: omni-architecture-redesign](../superpowers/specs/2026-06-16-omni-architecture-redesign.md) | 2026-06-16 | ⚠️ 部分过时 | 架构重设计，统一 Target/Codegen 抽象。当前 `omnilang/src/` 结构基本遵循，但以下偏离: ① CLI 用 Rust（`omnic`）而非 Python 实现；② Python 层降至 `test/omnilang/`（测试 oracle）；③ Rust LSP 从未集成 |
| [spec: omnilang-rust-workspace-refactor](../superpowers/specs/2026-06-28-omnilang-rust-workspace-refactor.md) | 2026-06-28 | ⚠️ 部分过时 | workspace 重构设计（计划 2 成员）。实际为 4 成员（增加了 `omnic` 和 `omnilang-wasm`），test 目录保留（非删除），Python LSP 保留 |
| [spec: r4-file-splitting-design](../superpowers/specs/2026-06-29-r4-file-splitting-design.md) | 2026-06-29 | ✅ 现行 | 大文件拆分方案（codegen/IR/semantic/parser 分批）。当前目录结构紧密对应此设计，如 `codegen/react/` 含 `mod.rs`/`expr.rs`/`view.rs`/`utils.rs`/`motion/`；`semantic/lints/` 含 12 个独立 lint 文件 |
| [plan: consolidate-omniview-compilation](../superpowers/plans/2026-07-12-consolidate-omniview-compilation.md) | 2026-07-12 | ✅ 现行 | 消除 omnic CLI 和 poiesis 间的重复代码 |
| [plan: omnilang-workspace-refactor-plan](../superpowers/plans/2026-06-28-omnilang-workspace-refactor-plan.md) | 2026-06-28 | ⚠️ 历史参考 | workspace 重构实施计划（2 成员版） |
| [plan: omni-rust-compiler](../superpowers/plans/2026-06-15-omni-rust-compiler.md) | 2026-06-15 | ⚠️ 历史参考 | 初始编译器实施计划 |
| [plan: omni-architecture-migration](../superpowers/plans/2026-06-16-omni-architecture-migration.md) | 2026-06-16 | ⚠️ 历史参考 | 架构迁移计划 |
| [plan: batch1-codegen-split](../superpowers/plans/2026-06-29-batch1-codegen-split.md) | 2026-06-29 | ✅ 现行 | codegen 拆分第一批 |
| [plan: batch2-ir-split](../superpowers/plans/2026-06-29-batch2-ir-split.md) | 2026-06-29 | ✅ 现行 | IR 拆分第二批 |
| [plan: batch3-semantic-split](../superpowers/plans/2026-06-29-batch3-semantic-split.md) | 2026-06-29 | ✅ 现行 | semantic 拆分第三批 |
| [plan: batch4-parser-split](../superpowers/plans/2026-06-29-batch4-parser-split.md) | 2026-06-29 | ✅ 现行 | parser 拆分第四批 |
| [plan: batch5-remaining-split](../superpowers/plans/2026-06-29-batch5-remaining-split.md) | 2026-06-29 | ✅ 现行 | 剩余文件拆分第五批 |

### 1.2 导入系统

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: omni-import-system-design](../superpowers/specs/2026-06-16-omni-import-system-design.md) | 2026-06-16 | ✅ 现行 | import/export 语法、路径解析、循环依赖检测。已验证：`omnilang/src/path_mapping.rs`、`validate.rs`、`parser/top_level/module.rs` 均已实现 |
| [plan: omni-import-system](../superpowers/plans/2026-06-16-omni-import-system.md) | 2026-06-16 | ✅ 现行 | 导入系统实施计划 |

### 1.3 CLI 工具（`omnic`）

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: omnic-cli-redesign](../superpowers/specs/2026-06-17-omnic-cli-redesign.md) | 2026-06-17 | ⚠️ 部分过时 | CLI 重设计。子命令结构已实现，但设计描述 CLI 为 Python 封装 DLL，实际 `omnic` 用 Rust 重写为独立 workspace crate，直接链接 `omnilang` rlib |
| [plan: omnic-cli-redesign-plan](../superpowers/plans/2026-06-17-omnic-cli-redesign-plan.md) | 2026-06-17 | ⚠️ 历史参考 | CLI 实施计划 |

### 1.4 标准库

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: omnilang-stdlib-design](../superpowers/specs/2026-06-19-omnilang-stdlib-design.md) | 2026-06-19 | ✅ 现行 | 标准库设计。初始计划 3 模块（json/http/storage），实际已实现 8 模块 |
| [spec: omnilang-stdlib-implementation-plan](../superpowers/specs/2026-06-19-omnilang-stdlib-implementation-plan.md) | 2026-06-19 | ✅ 现行 | 实施计划 |
| [spec: stdlib-and-compiler-gaps-design](../superpowers/specs/2026-06-20-stdlib-and-compiler-gaps-design.md) | 2026-06-20 | ⚠️ 部分过时 | 缺口分析。P0 的 4 模块（math/string/array/console）已实现；P1（datetime/crypto）和 P2（fs/timer）未实现。编译器缺口描述的 "null stubs" 已大幅改善 |
| [plan: phase1-stdlib-compiler-gaps](../superpowers/plans/2026-06-20-phase1-stdlib-compiler-gaps.md) | 2026-06-20 | ✅ 现行 | 第一批缺口实施计划 |
| [参考: stdlib](../../stdlib.md) | — | ⚠️ 不完整 | 标准库 API 参考文档。仅覆盖 math/async/string 3 模块，缺少 array/console/http/json/storage 模块文档 |

### 1.5 代码生成

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: react-flutter-codegen-completion-plan](../superpowers/specs/2026-06-29-react-flutter-codegen-completion-plan.md) | 2026-06-29 | ⚠️ 部分过时 | React/Flutter codegen 完善。标记为"等待批准"，后续各后端快照测试已建立 |
| [plan: react-flutter-codegen-completion](../superpowers/plans/2026-06-29-react-flutter-codegen-completion.md) | 2026-06-29 | ⚠️ 历史参考 | 实施计划 |

### 1.6 编译诊断与质量保证

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: compiler-issues](../superpowers/specs/2026-06-19-compiler-issues.md) | 2026-06-19 | ⚠️ 部分过时 | 30 个已知编译器问题（13 未支持特性 + 11 parser bug + 6 workaround）。部分已修复，无完整关闭记录 |
| [spec: analyse-guarantees-run-design](../superpowers/specs/2026-06-21-analyse-guarantees-run-design.md) | 2026-06-21 | ⚠️ 部分过时 | `omnic analyse` 守门人设计。26+12 项诊断契约，实现状态不确定 |
| [plan: design-syntax-compiler-fixes](../superpowers/plans/2026-07-08-design-syntax-compiler-fixes.md) | 2026-07-08 | ✅ 现行 | design { token/class/variant } 语法编译管线修复 |
| [plan: code-review-fixes](../superpowers/plans/2026-07-06-code-review-fixes.md) | 2026-07-06 | ⚠️ 历史参考 | 基于 2026-07-06 代码审查报告的修复计划（9 CRITICAL + 19 HIGH + 27 MEDIUM） |

---

## 二、OmniView 框架

### 2.1 语言参考文档（权威现行）

| 文档 | 状态 | 说明 |
|------|------|------|
| [OmniView README](../omniview/README.md) | ✅ 现行 | OmniView 文档索引 |
| [01 项目结构](../omniview/01-project-structure.md) | ✅ 现行 | .omni 项目文件组织 |
| [02 组件](../omniview/02-components.md) | ✅ 现行 | component/props/signals/computed/view/@mount/@effect 等 |
| [03 模板](../omniview/03-templates.md) | ✅ 现行 | 元素、属性、@if/@for/@slot、bind:/variant: 等（`states:` 已删除，见 09-motion） |
| [04 响应式](../omniview/04-reactivity.md) | ✅ 现行 | signal/computed 响应式系统、批量更新、@effect |
| [05 Stores](../omniview/05-stores.md) | ✅ 现行 | 全局 store：signal/getter/action + 内联 store |
| [06 事件](../omniview/06-events.md) | ✅ 现行 | 20 种事件类别、自定义事件、emit |
| [07 样式](../omniview/07-styling.md) | ✅ 现行 | style 块、CSS 变量、作用域样式 |
| [08 设计系统](../omniview/08-design-system.md) | ⚠️ 缺 `theme` 关键字（已废弃） | 仅记录 `design` 关键字（token/class/variant）。旧 `theme` 分组关键字（colors/spacing/typography/radii）已被 v2 的 `@themes`/`@theme` 信号语法取代（v2 设计文档 D1），不再存在于编译器 |
| [09 运动](../omniview/09-motion.md) | ✅ 现行 | @motion、缓动、时间线、触发器、Hero 过渡 |
| [10 类型系统](../omniview/10-type-system.md) | ✅ 现行 | 基础类型、struct、tagged union、可选类型 |
| [11 表达式](../omniview/11-expressions.md) | ✅ 现行 | 运算符、块表达式、match、闭包、await |
| [12 逻辑与异步](../omniview/12-logic-and-async.md) | ✅ 现行 | fn/async fn/await、控制流、try/catch |
| [13 标准库](../omniview/13-standard-library.md) | ✅ 现行 | 7 个 std 模块 API 文档 |

### 2.2 框架特性设计

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: omniview-motion-animation-design](../superpowers/specs/2026-06-20-omniview-motion-animation-design.md) | 2026-06-20 | ✅ 现行 | ~49KB 动画系统设计（关键帧、时间线、缓动、stagger、滚动驱动、Hero）。核心语法已在 09-motion.md 中实现 |
| [plan: omniview-motion-implementation](../superpowers/plans/2026-06-21-omniview-motion-implementation.md) | 2026-06-21 | ✅ 现行 | 动画实现计划 |
| [spec: omniview-event-data-transmission-design](../superpowers/specs/2026-06-29-omniview-event-data-transmission-design.md) | 2026-06-29 | ✅ 现行 | 事件数据传输 v2：20 类 EventDataKind、精确字段提取、@for 委托 |
| [plan: omniview-event-data-transmission](../superpowers/plans/2026-06-29-omniview-event-data-transmission.md) | 2026-06-29 | ✅ 现行 | 事件传输实施计划 |
| [spec: omni-gen-code-generation-design](../superpowers/specs/2026-06-21-omni-gen-code-generation-design.md) | 2026-06-21 | ✅ 现行 | 编辑器画布↔.omni 双向同步（三阶段流水线、PID 系统、StageResult） |
| [plan: omni-gen-implementation](../superpowers/plans/2026-06-21-omni-gen-implementation.md) | 2026-06-21 | ✅ 现行 | omni-gen 实施计划 |
| [spec: omni-animation-library-design](../superpowers/specs/2026-07-23-omni-animation-library-design.md) | 2026-07-23 | 📋 规划中 | 提取共享 `@omni/animation` npm 包（~350 行 TS），统一 OmniView/React 动画管线，删除 ~1434 行 motion-runtime.js |
| [plan: omni-animation-library-plan](../superpowers/plans/2026-07-23-omni-animation-library-plan.md) | 2026-07-23 | 📋 规划中 | 动画库 5 阶段渐进式迁移计划 |

### 2.3 learn-omniview 项目

> 当前实际有 **93 个 `.omni` 源文件**（`main.omni` 头注释中的 "68+" / "74+" 已过时）

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: learn-omniview-full-demo-design](../superpowers/specs/2026-06-19-learn-omniview-full-demo-design.md) | 2026-06-19 | ⚠️ 过时（已超出） | 目标 50 文件 7 阶段设计。项目已扩展至 91 文件，远超初始范围。作为历史路线图仍有参考价值 |
| [spec: learn-omniview-phase-8-design](../superpowers/specs/2026-06-19-learn-omniview-phase-8-design.md) | 2026-06-19 | ❌ 过时（已实现） | Phase 8 的 4 个文件（api/DataFetching/DataSubmit/UserSettings）已完全实现，代码与设计吻合 |
| [plan: learn-omniview-phase-8](../superpowers/plans/2026-06-19-learn-omniview-phase-8.md) | 2026-06-19 | ❌ 已实施 | Phase 8 实施计划（已完成） |
| [plan: learn-omniview-upgrade](../superpowers/plans/2026-06-20-learn-omniview-upgrade.md) | 2026-06-20 | ✅ 现行 | learn-omniview 升级计划 |
| [plan: learn-omniview-gap-closure](../superpowers/plans/2026-06-20-learn-omniview-gap-closure.md) | 2026-06-20 | ✅ 现行 | 缺口关闭计划 |
| [plan: learn-omniview-supplement](../../.claude/plans/learn-omniview-supplement.md) | 2026-06-21 | ✅ 现行 | learn-omniview 语法补充实施计划 |
| [report: poiesis-learn-omniview-gaps](../superpowers/reports/poiesis-learn-omniview-gaps.md) | 2026-07-23 | ✅ 现行 | **最重要的差距分析**：Poiesis 编辑 learn-omniview 的能力缺口（按 view/store/design/animation/type/lifecycle 分层的 4 阶段路线图） |
| [analysis: omniview-features-not-used](../../.claude/analysis/omniview-features-not-used.md) | 2026-06-20 | ✅ 现行 | 约 136 语言特性中 22 个未被 learn-omniview 使用 |
| [analysis: omniview-maturity-gaps](../../.claude/analysis/omniview-maturity-gaps.md) | 2026-06-20 | ✅ 现行 | OmniView 成熟度评估 v0.7，含 Vue3/Svelte5/React19/Flutter 对比 |

### 2.4 编码规范

| 文档 | 状态 | 说明 |
|------|------|------|
| [omniview-coding-standards SKILL](../../.claude/skills/omniview-coding-standards/SKILL.md) | ✅ 现行 | .omni 代码编写规范 |

---

## 三、Poiesis 可视化编辑器

### 3.1 核心架构（权威参考）

| 文档 | 状态 | 说明 |
|------|------|------|
| [poiesis-dev: architecture](../../.claude/skills/poiesis-dev/references/architecture.md) | ✅ 现行 | **最权威的 Poiesis 架构文档**。覆盖分层、两大页面（ArchitecturePage/DesignPage）、数据流（.omni 往返、PID、Tab 管理）、Zustand Store 结构、Tauri 后端、WASM 编译器双路径 |
| [poiesis-dev: file-map](../../.claude/skills/poiesis-dev/references/file-map.md) | ✅ 现行 | 按功能域的关键文件索引 |
| [poiesis-dev: design-system](../../.claude/skills/poiesis-dev/references/design-system.md) | ✅ 现行 | design { token/class/variant } 系统详解 |
| [poiesis-dev: workflow](../../.claude/skills/poiesis-dev/references/workflow.md) | ✅ 现行 | Poiesis 开发工作流 |
| [poiesis-dev: confusions](../../.claude/skills/poiesis-dev/references/confusions.md) | ✅ 现行 | 已知陷阱（如 PascalCase vs camelCase key 前缀问题） |

### 3.2 早期总体设计（❌ 均已过时，仅保留作为设计演进参考）

| 文档 | 日期 | 状态 | 过时原因 |
|------|------|------|------|
| [设计: visual-editor-design](visual-editor-design.md) | 2026-06-13 | ❌ | 基于 Rete.js + Python 后端 + `.omni.json` 主格式。全部已被后续方案取代 |
| [设计: node-editor-interface-design](node-editor-interface-design.md) | 2026-06 | ❌ | Rete.js 节点编辑器接口。Rete.js 已放弃 |
| [设计: execution-plan](execution-plan.md) | 2026-06-12 | ❌ | Python/FastAPI 后端 + `editor/` 目录结构。架构已完全变更为 Tauri+Rust+WASM |
| [spec: visual-scripting-design](../superpowers/specs/2026-06-14-visual-scripting-design.md) | 2026-06-14 | ❌ | 基于 @xyflow/react 的可视化脚本。已被实际编辑器实现取代 |
| [plan: visual-scripting-round1](../superpowers/plans/2026-06-14-visual-scripting-round1.md) | 2026-06-14 | ❌ | 第一轮实施计划 |
| [spec: poiesis-redesign](../superpowers/specs/2026-07-01-poiesis-redesign.md) | 2026-07-01 | ❌ | Poiesis 重设计 v1。已被 v2 取代 |
| [spec: poiesis-redesign-v2](../superpowers/specs/2026-07-02-poiesis-redesign-v2.md) | 2026-07-02 | ⚠️ | v2 为后续各面板设计奠定了基础，但具体细节已分散到各专项文档 |

### 3.3 编辑器数据格式

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: editor-omni-format-redesign](../superpowers/specs/2026-06-21-editor-omni-format-redesign.md) | 2026-06-21 | ⚠️ 部分过时 | 编辑器 .omni 格式设计。`@code-tab`→`@omni-file` 改名已实施，但 `@chain/@node/@wire` 注解体系已被 PID-based graph 方案取代 |
| [spec: annotation-unification-design](../superpowers/specs/2026-07-11-annotation-unification-design.md) | 2026-07-11 | ✅ 现行 | 注解格式统一（`// @pid:xxx` + `// @if` 等 type annotation），`annotation-registry.ts` 集中管理。已验证：代码与设计吻合 |
| [plan: annotation-unification](../superpowers/plans/2026-07-11-annotation-unification.md) | 2026-07-11 | ✅ 现行 | 注解统一实施计划 |

### 3.4 样式系统（设计文档演进链）

> 样式系统经历了多轮迭代，以下按时间排序：

| 文档 | 日期 | 状态 | 被谁取代 |
|------|------|------|------|
| [spec: editor-style-panel-redesign](../superpowers/specs/2026-06-24-editor-style-panel-redesign.md) | 2026-06-24 | ❌ | → style-panel-redesign (06-27) |
| [spec: style-panel-redesign](../superpowers/specs/2026-06-27-style-panel-redesign.md) | 2026-06-27 | ❌ | → style-system-redesign (07-07) |
| [spec: style-system-redesign](../superpowers/specs/2026-07-07-style-system-redesign.md) | 2026-07-07 | ❌ | → poiesis-style-panel-design (07-07，同日) |
| [spec: poiesis-style-panel-design](../superpowers/specs/2026-07-07-poiesis-style-panel-design.md) | 2026-07-07 | ✅ 现行 | design { token/class/variant } 语法，`token(name)` 函数，三名编辑器 |
| [spec: poiesis-class-editor-visual-redesign](../superpowers/specs/2026-07-08-poiesis-class-editor-visual-redesign.md) | 2026-07-08 | ✅ 现行 | Class 编辑器卡片化（5 张专用卡片 + HUD 取色器 + scrub input）。已验证：`ClassEditor.tsx` 中的实现与设计吻合 |
| [spec: poisies-wasm-css-design](../superpowers/specs/2026-07-09-poisies-wasm-css-design.md) | 2026-07-09 | ✅ 现行 | WASM `resolve_styles` 替换 TS `buildInlineStyleMap`，`<style>` 标签注入 |
| [spec: designcanvas-rendering-redesign](../superpowers/specs/2026-07-09-designcanvas-rendering-redesign.md) | 2026-07-09 | ✅ 现行 | 无限画布 + Artboard 设备预设 + CSS scope pool + `useWasmCompile` hook。已验证：`InfiniteCanvas.tsx`、`Artboard.tsx`、`useCssScopePool.ts` 均已实现 |
| [spec: variant-editor-redesign](../superpowers/specs/2026-07-09-variant-editor-redesign.md) | 2026-07-09 | ✅ 现行 | Variant 编辑器复用 Class 编辑器卡片。已验证：`VariantEditor.tsx` 实现与设计吻合 |

### 3.5 画布

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: canvas-runtime-preview-design](../superpowers/specs/2026-06-23-canvas-runtime-preview-design.md) | 2026-06-23 | ❌ 过时 | **iframe 方案已被放弃！** 当前使用 React 组件树渲染（`OmniViewElementTree` 从 WASM `element_tree` 输出渲染），不是 iframe 沙箱。postMessage 协议、工厂函数热替换等均未实现 |

### 3.6 CodeFlow 画布与代码编辑

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: codeflow-canvas-codegen](../superpowers/specs/2026-07-10-codeflow-canvas-codegen.md) | 2026-07-10 | ⚠️ 部分过时 | 4 节点（fn/code-block/condition/return）分类已实现，但 source-in-annotation 方案已被 CodeOp 方案取代 |
| [spec: codecard-editor-design](../superpowers/specs/2026-07-23-codecard-editor-design.md) | 2026-07-23 | 🔨 实现中 | **最新设计**：12 种 CodeOp 卡片类型 + Lexical 框架。数据模型已实现，编辑器 UI 仍在开发中 |
| [spec: codecard-editor-lexical-arch](../superpowers/specs/2026-07-23-codecard-editor-lexical-arch.md) | 2026-07-23 | 📋 规划中 | Lexical 基座架构设计。方案：单一 Lexical 实例 + 自定义 ElementNode（非 DecoratorNode）渲染卡片，`SlotTextNode` 标记槽位，光标自然流动无需跳入跳出。补全系统按光标上下文 + 前缀过滤（`l`→`let`/`let mut`，`w`→`while`）。卡片空槽位 Backspace→退化为文本。双模式保留 Monaco fallback。现有 `cards/` 目录下的独立 React 卡片组件将删除，由 CardElementNode + 子节点取代 |
| [plan: codecard-editor-plan](../superpowers/plans/2026-07-23-codecard-editor-plan.md) | 2026-07-23 | 🔨 实现中 | 16 任务 × 4 阶段实施计划 |
| [plan: codecard-editor-lexical](../superpowers/plans/2026-07-23-codecard-editor-lexical.md) | 2026-07-23 | 📋 规划中 | Lexical 基座重写实现计划（仅含目标和约束，**任务列表待补充**）。单一 Lexical 实例 + ElementNode 卡片 + 前缀过滤补全 + 双模式（保留 Monaco fallback）。现有 `cards/` 目录将被删除，现有测试必须继续通过 |
| [plan: codecard-editor-handoff](../superpowers/plans/2026-07-23-codecard-editor-handoff.md) | 2026-07-23 | 🔨 实现中 | 子代理驱动执行的交接文档 |

### 3.7 Store 编辑器

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: store-editor-design](../superpowers/specs/2026-07-10-store-editor-design.md) | 2026-07-10 | ✅ 现行 | Store 编辑器（Signals/Getters/Actions + CodeFlowCanvas）。已验证：`StorePage.tsx`、`StoreMembersPanel.tsx`、parser/emitter 均已实现 |
| [spec: store-panel-in-component-tab-design](../superpowers/specs/2026-07-11-store-panel-in-component-tab-design.md) | 2026-07-11 | ✅ 现行 | 组件标签页内嵌 Store 面板（StoreDrawer + 虚拟 tab ID）。已验证：`StoreDrawer.tsx` 实现吻合 |

### 3.8 面板系统

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: component-members-panel-design](../superpowers/specs/2026-07-12-component-members-panel-design.md) | 2026-07-12 | ✅ 现行 | prop/signal/computed 三折叠面板。已验证：`ComponentMembersPanel.tsx` 已实现 |
| [spec: event-binding-panel-design](../superpowers/specs/2026-07-12-event-binding-panel-design.md) | 2026-07-12 | ✅ 现行 | 事件绑定面板（HandlerModal 双标签页：绑定函数/写表达式）。已验证：`HandlerModal.tsx`、`FunctionBindingTab.tsx`、`ExpressionEditor.tsx` 均已实现 |
| [spec: conditional-rendering-design](../superpowers/specs/2026-07-13-conditional-rendering-design.md) | 2026-07-13 | ✅ 现行 | `@if/@else if/@else` 编辑器支持（`ConditionalBlockData` + branches 数组）。已验证：含 round-trip、CRUD、view-tree 测试 |
| [spec: canvas-component-def-panel-design](../superpowers/specs/2026-07-13-canvas-component-def-panel-design.md) | 2026-07-13 | ✅ 现行 | 画布上浮动组件定义面板。已验证：`ComponentDefPanel.tsx` 实现吻合 |
| [plan: poiesis-omniview-design-alignment](../superpowers/plans/2026-07-09-poiesis-omniview-design-alignment.md) | 2026-07-09 | ✅ 现行 | Poiesis 与 OmniView 设计对齐 |

### 3.9 调试窗口

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: poiesis-debug-window-design](../superpowers/specs/2026-07-11-poiesis-debug-window-design.md) | 2026-07-11 | ✅ 现行 | Tauri 调试 Webview（双向高亮 + 信号检查 + 150ms 热更新）。已验证：`useDebugWindow.ts`、Tauri event bus 均已实现 |

---

## 四、工具与基础设施

### 4.1 测试框架

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: test-framework-skill-design](../superpowers/specs/2026-06-13-test-framework-skill-design.md) | 2026-06-13 | ✅ 现行 | 7 阶段闭环测试技能（构建→转译→截图对比→测试逻辑→修复框架代码） |
| [SKILL: test-framework](../../.claude/skills/test-framework/SKILL.md) | — | ✅ 现行 | 测试框架 AI 技能实现 |
| [SKILL: testing-omniview-react](../../.claude/skills/testing-omniview-react/SKILL.md) | — | ✅ 现行 | React 转译测试技能（omnic→React→tsc+vite→Playwright） |

### 4.2 VS Code 扩展

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: vscode-extension-design](../superpowers/specs/2026-06-16-vscode-extension-design.md) | 2026-06-16 | ⚠️ 部分过时 | LSP-first 扩展架构。Python LSP 存在于 `test/omnilang/lsp/server.py`（1027 行，11 项特性），但设计中的 Rust `omni_analyze` FFI 未作为独立模块实现 |

### 4.3 OmniDriver

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [plan: omnidriver-v2](../superpowers/plans/2026-06-15-omnidriver-v2.md) | 2026-06-15 | ⚠️ 部分过时 | OmniDriver v2 API。当前 Poiesis 功能可能超出此设计范围 |

### 4.4 Python 框架合并

| 文档 | 日期 | 状态 | 说明 |
|------|------|------|------|
| [spec: omnilang-python-framework-merge](../superpowers/specs/2026-06-16-omnilang-python-framework-merge.md) | 2026-06-16 | ❌ 已实施 | Python 目录合并。Rust 编译器已取代 Python 管道，Python 代码保留在 `test/omnilang/` 作为测试 oracle |

---

## 五、其他参考文档和目录

### 5.1 项目级文档

| 文档 | 类型 | 说明 |
|------|------|------|
| [README.md](../../README.md) | 项目概述 | 项目简介、10 编译目标成熟度矩阵、目录结构 |
| [QUICKSTART.md](../../QUICKSTART.md) | 快速入门 | 3 命令上手 |
| [docs/README.md](../../README.md) | 文档索引 | 5 轨道教程结构（部分章节标为"待补充"） |
| [docs/stdlib.md](../../stdlib.md) | API 参考 | ⚠️ 不完整：仅覆盖 math/async/string，缺少 array/console/http/json/storage |

### 5.2 AI 技能参考

| 文档 | 说明 |
|------|------|
| [poiesis-dev SKILL](../../.claude/skills/poiesis-dev/SKILL.md) | Poiesis 开发技能（含 5 篇 references） |
| [omniview-coding-standards SKILL](../../.claude/skills/omniview-coding-standards/SKILL.md) | .omni 编码规范 |
| [test-framework SKILL](../../.claude/skills/test-framework/SKILL.md) | 测试框架技能 |
| [poiesis/CLAUDE.md](../../poiesis/CLAUDE.md) | Poiesis Victauri 集成配置 |

### 5.3 评估与分析

| 文档 | 日期 | 说明 |
|------|------|------|
| [omniview-compiler-audit](../../.claude/evals/omniview-compiler-audit-20250618.md) | 2026-06-18 | OmniView 编译器审计 |
| [omnilang-compiler](../../.claude/evals/omnilang-compiler.md) | — | OmniLang 编译器评估 |
| [editor-client](../../.claude/evals/editor-client.md) | — | 编辑器客户端评估 |
| [editor-client-src-20250627](../../.claude/evals/editor-client-src-20250627.md) | 2026-06-27 | 编辑器源码评估 |
| [code-review-report](../../CODE_REVIEW_REPORT_2026-07-06.md) | 2026-07-06 | 代码审查报告 |
| [anim-gap-closure-fixes](../superpowers/fixes/2026-06-22-animation-gap-closure-fixes.md) | 2026-06-22 | 动画缺口修复（已应用） |

### 5.4 非设计文档目录

| 目录 | 说明 |
|------|------|
| `.superpowers/sdd/` | SDD 任务简报和报告（~300 文件），按批次组织。开发执行产物，非设计文档 |
| `.superpowers/brainstorm/` | ~20 篇头脑风暴会话产物（画布布局、节点分类、动画研究等） |
| `docs/rete-docs/` | Rete.js 第三方 API 文档（62 文件）。**Rete.js 已放弃**，仅保留作为历史参考 |
| `test/projects/learn-omniview/build/` | playwright-report + test-results（不纳入设计文档） |

---

## 六、已发现的文档缺口

| 缺口 | 严重程度 | 说明 |
|------|------|------|
| **`theme` 关键字无文档** | 高 | 已随 v2 语法迭代关闭：旧 `theme` 分组关键字被 `@themes` 声明 + `@theme` 信号标记取代（v2 设计文档 D1），demo 已迁移，无 `shared/theme.omni` |
| **WASM 分离架构无设计文档** | 中 | `omnilang-wasm` crate 作为独立的 WASM 绑定层（cdylib + rlib），此架构决策缺乏设计文档 |
| **omnic Rust CLI 重写无设计文档** | 中 | `omnic` 从设计中的 Python 封装改为 Rust 独立 workspace crate，此决策缺乏专门的设计文档 |
| **标准库 API 文档不完整** | 中 | `docs/stdlib.md` 仅覆盖 3 个模块（math/async/string），实际 stdlib 有 8 个模块（缺少 array/console/http/json/storage 文档） |
| **Rust LSP 从未实现** | 低 | 多份设计文档提及 Rust `lsp.rs` 模块，但从未集成。LSP 由 Python 实现 |
| **`omni-animation` 库无 API 参考** | 低 | `packages/omni-animation/` 已创建并有设计 spec，但缺少独立的 API 参考文档 |
| **learn-omniview 头注释数量过时** | 低 | `main.omni` 头注释写"68+"和"74+"，实际为 91 个 .omni 文件 |

---

## 七、快速导航

| 你想了解... | 去看... |
|-------------|--------|
| 当前 Poiesis 架构 | [poiesis-dev: architecture](../../.claude/skills/poiesis-dev/references/architecture.md) |
| OmniView 语言怎么写 | [docs/omniview/](../omniview/) 系列（13 章） |
| Poiesis 当前能力缺口 | [poiesis-learn-omniview-gaps](../superpowers/reports/poiesis-learn-omniview-gaps.md) |
| OmniLang 编译器架构 | [omni-rust-compiler-design](../superpowers/specs/2026-06-15-omni-rust-compiler-design.md) + [r4-file-splitting-design](../superpowers/specs/2026-06-29-r4-file-splitting-design.md) |
| 正在开发的功能 | [codecard-editor-design](../superpowers/specs/2026-07-23-codecard-editor-design.md) + [omni-animation-library-design](../superpowers/specs/2026-07-23-omni-animation-library-design.md) |
| 样式系统设计演进 | §3.4 中的取代链：editor-style-panel → style-panel → style-system → poiesis-style-panel |
| 某项功能的历史开发过程 | `.superpowers/sdd/` 中对应批次的任务简报和报告 |
| 早期设计参考 | §3.2 中的过时文档（标记了过时原因） |

---

> **维护说明：**
> - 新增设计文档时，在此索引中添加条目并标注状态
> - 功能被重新设计时，将旧文档标记为"过时"并注明被哪个新文档取代，保留设计演进历史
> - 索引中的"项目事实速览"应在重大架构变更时更新
> - 文档缺口（§6）应在补充文档后从此列表中移除
