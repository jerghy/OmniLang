# Architectural Review: OmniView — v2 独立验证评审

**日期**：2026-07-30
**评审类型**：全多视角架构评审（11 位专家 + 务实执行者）
**范围**：OmniLang 编译器的多目标 UI 代码生成系统 — 流水线、IR、所有代码生成后端
**方法论**：独立并行专家评审 → 交叉分析 → 综合报告
**上一份评审**：`component-omniview-2026-07-30.md`（今日早些时候完成，11 位专家）

---

## 执行摘要

本次 v2 评审是当天早些时候完成的 12 项修复后对 OmniView 架构的**独立重新评估**。上一份评审中的 12 项立即修复均已在 12 次提交中实现（CSS 净化、URL 净化、ref_names 线程化、body_has_await 检查、typeof 字符串守卫等）。

本次评审发现了**16 个新发现**（上一评审未覆盖或覆盖不足），以及**6 个已确认的遗留问题**需要更新优先级。架构获得了 **Strong (A-)** 的总体评估——管道完整性极佳，测试纪律出色，流程成熟。主要风险集中在三个领域：**IR 枚举缺乏 `#[non_exhaustive]`**（对 10 个后端形成编译时破坏风险）、**React motion 效果使用全局 DOM 查询**（多组件页面中的正确性错误），以及**非 React 后端（Vue/Dioxus）的安全净化缺口**。

---

## 评审团队

| # | 专家 | 严重性分布 | 关键焦点 |
|---|------|-------------------|-----------|
| 1 | 系统架构师 | 2 HIGH, 3 MEDIUM | 双调度路径、FormatExpr 采用、跨目标测试缺口 |
| 2 | 领域专家 | 1 HIGH, 4 MEDIUM, 1 LOW | 片段双重表示、ConstructionKind 死代码、IR 类型保真度 |
| 3 | 安全专家 | 1 HIGH, 3 MEDIUM, 2 LOW | Vue/Dioxus 净化缺口、@for NID 绕过、CSS CR/LF 缺口、数据: URI 过于保守 |
| 4 | 可维护性专家 | 4 MEDIUM, 3 LOW | 表达式/语句重复、47 字段生成器、typescript.rs 死代码 |
| 5 | 性能专家 | 3 MEDIUM, 6 LOW | 11 次视图树遍历/组件、IIFE 膨胀、Vec::contains O(n²) |
| 6 | 实施策略师 | 1 CRITICAL, 2 HIGH, 2 MEDIUM | IR 枚举保护、变更排序、推迟项跟踪流程 |
| 7 | AI 工程师 | 4 MEDIUM, 1 LOW | JS 运行时测试不在 CI、无 E2E 验证、智能体可靠性指标、缺失 AI 技能 |
| 8 | 务实执行者 | 2 MEDIUM, 3 LOW | JsFormatExpr 死抽象、event_object 重复、不活跃后端负载 |
| 9 | Rust 专家 | 4 MEDIUM, 5 LOW | serde_json::Value 类型安全、CSS 黑名单、#[non_exhaustive]、生成器单体 |
| 10 | TS/React 专家 | 2 HIGH, 5 MEDIUM, 2 LOW | 全局 querySelectorAll、内联样式引用名称漏洞、死 motion 代码、ESLint 启发式 |
| 11 | 编译器架构师 | 2 MEDIUM, 3 LOW | 流水线绕过、FormatExpr 不参与、source map、46 字段生成器 |

---

## 跨领域主题

### 主题 1：IR 枚举缺乏 `#[non_exhaustive]` —— 5 位专家标记

**标记人**：实施策略师（CRITICAL）、Rust 专家（MEDIUM M3）、编译器架构师（LOW C4）、务实执行者（LOW — 从其他位置移除）、系统架构师（相关）

`ExprIR`（22 个变体）、`StatementIR`（13 个变体）、`ViewTree`（7 个变体）和 `MutationIntent`（3 个变体）均缺少 `#[non_exhaustive]`。添加任意一个新变体将同步破坏所有 10 个后端。`test-all-backends` CI 作业是非阻塞的，因此故障在合并后才被发现。

务实执行者提出了一个补充性关注点：`TypeKind` 和 `EventDataKind` **确实**有 `#[non_exhaustive]`，尽管只有内部消费者（workspace 内零个外部 API 消费者），这意味着该属性移除了安全性却不增加 API 稳定性。

**建议**：在 `ExprIR`、`StatementIR`、`ViewTree` 和 `MutationIntent` 上添加 `#[non_exhaustive]`；从 `TypeKind` 和 `EventDataKind` 上移除。在同一提交中完成这两项操作。**工作量**：S（属性 4 行；消费者 catch-all 分支约 20 行）。**影响/成本比**：极高——一个 30 分钟的修复可保护 10 个后端免受未来的编译时破坏。

### 主题 2：React Motion 使用全局 DOM 查询 —— 3 位专家标记

**标记人**：TS/React 专家（HIGH 1）、性能专家（MEDIUM）、实施策略师（HIGH）

在 `omnilang/src/codegen/react/motion/mod.rs:71,93,111` 和 `transforms.rs:21,83,202` 生成的 6 个 `useEffect` 块使用了 `document.querySelectorAll('[data-motion-*]')`。当同一页面渲染两个包含 `@enter` 动画的组件时，实例 A 的效果会附加事件处理程序到实例 B 的元素。在 Strict Mode 下行为是不确定的。

**建议**：将查询范围限定为通过 `useRef` 的组件根元素。用 `rootRef.current.querySelectorAll(...)` 替换所有 `document.querySelectorAll(...)`。**工作量**：M（1–2 天）。这是**正确性错误**，而非性能问题。

### 主题 3：FormatExpr/FormatStmt 特性采用 —— 6 位专家标记

**标记人**：系统架构师（HIGH-2）、可维护性专家（MEDIUM）、编译器架构师（MEDIUM）、实施策略师（HIGH）、性能专家（相关）、务实执行者（相关）

OmniView 维护着一个 417 行的 `expr_to_js()` 和一个 285 行的 `emit_statement()`，它们独立分发 `ExprIR`/`StatementIR` 变体，与共享的 `FormatExpr`/`FormatStmt` 特性并行。新增一个 `ExprIR` 变体需要在 3 处进行更新（共享特性 + OmniView 副本 + React 直接处理程序）。

编译器架构师指出，由于 OmniView 的表达式格式化是有状态的（需要 `signal_names`、`renames`、`for_state_names`），因此纯 `FormatExpr`（不可变 `&self`）永远无法完全替换它——解决方案是一个状态承载包装器。务实执行者指出 `JsFormatExpr`（共享 JS 家族格式化器）作为死抽象具有零消费者——每个 JS 家族后端都编写了自己的实现。

**建议**：创建一个实现 `FormatExpr` 的 `OmniViewFormatExpr`，并包含 OmniView 特定的重写方法。417 行的 `expr_to_js` 将变为一个薄包装器。移除 `JsFormatExpr`（零消费者）。**工作量**：M（特性采用 2–3 天，JsFormatExpr 移除 15 分钟）。**状态**：可延期但杠杆作用高——不会导致用户可见的错误，但每次 IR 变更都重复产生成本。

### 主题 4：安全净化缺口及跨后端对等 —— 2 位专家，高影响

**标记人**：安全专家（HIGH — Vue/Dioxus）、系统架构师（MEDIUM-5 — 跨目标测试）

ADR-039 为 React 修复的 XSS 向量（静态 URL 属性中的 `javascript:` URL）在 Vue（`vue/template.rs:27`）和 Dioxus（`dioxus/view.rs:30`）中仍然开放。这两者都在没有 `is_dangerous_url()` 检查的情况下逐字发出静态 URL 属性。

安全专家还发现了：**@for NID 绕过**（动态 URL 净化引用了原始 NID，当 `@for` 克隆重新分配 NID 时净化器失效）；**Rust CSS 净化器中的 CR/LF/TAB 标准化缺失**（JS 运行时在上次提交中已打补丁，但 Rust 端仍然缺失）；以及**过于保守的 `data:` URI 阻止**（阻止了合法的 `data:image/*` 和内联字体）。

**建议**：立即 — 将 `is_dangerous_url()` 添加到 Vue 和 Dioxus 属性生成（各 3 行）。添加 `[\r\n\t]` 标准化到 `sanitize_css_value`（1 行）。短期 — 修复 @for NID 绕过，放宽 `data:` 为 MIME 类型感知检查。**工作量**：S（立即），M（短期）。

### 主题 5：领域模型债务 —— 已确认的死字段和类型泄漏

**标记人**：领域专家（1 HIGH, 4 MEDIUM）、Rust 专家（MEDIUM M1）、务实执行者（2 MEDIUM）

领域专家发现了四个具体的 IR 卫生问题：
1. **片段双重表示**（HIGH）：`ViewTree::Fragment(Vec<ViewTree>)` 和 `make_fragment_element` → `ElementIR { tag: "fragment" }` 是同一概念以两种不同的方式建模，导致 Vue/Dioxus/Flutter 的输出不一致
2. **`ConstructionKind`**（MEDIUM）：6 变体枚举，零个生产消费者，所有测试样板文件
3. **`style_classes`/`inline_style` 作为 `serde_json::Value`**（MEDIUM）：基础设施类型泄漏到领域层，迫使运行时模式匹配
4. **`SignalIntent.is_copy`**（MEDIUM）：始终为 `false`，零消费者
5. **`ComputedIntent.deps`**（MEDIUM）：始终为 `vec![]`，已在 ADR-010 中记录但从未实现

**建议**：立即 — 清理 `is_copy`（移除字段）和 `ConstructionKind`（移除未使用的变体）。短期 — 统一片段表示（5 行 IR 降低变更，修正 3 个后端）。长期 — 类型化 `style_classes` 为 `Vec<String>`，`inline_style` 为 `FxHashMap<String, ExprIR>`。

### 主题 6：测试和 CI 缺口 —— 代码覆盖率之外

**标记人**：AI 工程师（3 MEDIUM）、实施策略师（MEDIUM）、系统架构师（MEDIUM-5）、性能专家（相关）

三个正交的测试缺口被识别：
1. **JS 运行时测试（431 行）不在 CI 中**：覆盖信号传播、净化器、去抖——零自动化执行。3 个审查周期内均被标记，从未集成
2. **无浏览器级 E2E 验证**：生成的代码被检查语法正确性，但从未在真实浏览器中执行。运行时行为（信号、事件）被间接验证
3. **跨目标回归矩阵被推迟**：4 个已确认的静默丢弃错误在 3 个后端被发现——通过手动审查，而非自动化测试

**建议**：立即 — 在 `cargo test` 中添加 JS 运行时测试集成（通过 `std::process::Command` shell 到 `node`）。短期 — 为所有 3 个 P0 后端添加参数化跨目标夹具测试。长期 — 用于运行时行为的 Playwright E2E 测试。

---

## 权衡与分歧

### 权衡 1：`#[non_exhaustive]` 添加 vs 移除

**实施策略师** 说在 ExprIR/StatementIR 上添加 `#[non_exhaustive]`（保护 10 个后端免受级联编译时破坏）。**务实执行者** 说从 TypeKind/EventDataKind 上移除 `#[non_exhaustive]`（没有外部消费者，移除安全性却未增加 API 稳定性）。

→ **解决**：不冲突。不同的枚举，不同的消费者配置文件。两者皆可做，且应在同一提交中完成。ExprIR/StatementIR 有 10 个外部消费者和零个 API 稳定性要求——`#[non_exhaustive]` 在此处有意义。TypeKind/EventDataKind 仅有内部消费者和零个外部 API 消费者——`#[non_exhaustive]` 在此处没有意义。

### 权衡 2：FormatExpr 采用方法

**编译器架构师** 说 OmniView 的表达式格式化是有状态的（signal_names、renames），因此纯 FormatExpr 无法完全替换它。**系统架构师** 和 **可维护性专家** 说创建一个带有重写方法的 OmniViewFormatExpr。

→ **解决**：两位都正确。解决方案是一个状态承载包装器，在 10 个需要 OmniView 特定行为的变体上实现带有重写方法的 FormatExpr，并将剩余的 12 个变体委托给共享分发。这不是全有或全无——即使是部分采用也能在每次 IR 变更时减少数百行重复代码。

### 权衡 3：OmniViewGenerator 分解范围

**可维护性专家** 说提取 CSS 和元素树子结构体（M 工作量）。**编译器架构师** 说完整的阶段结构体分解（XL 工作量）。**系统架构师** 说首先按阶段对字段进行文档分组（S 工作量）。

→ **解决**：渐进式方法。立即：按阶段分组字段并添加文档注释（30 分钟）。短期：提取 CSS 状态和元素树状态作为专注的子结构体（2–3 天）。长期：仅当添加下一个主要 OmniView 功能时进行完整的阶段结构体分解，届时分解成本可与功能本身合并。

### 权衡 4：`data:` URI 阻止 —— 安全 vs 功能性

**安全专家** 说当前的完全阻止过于保守——允许 `data:image/*` 和 `data:font/*` 同时阻止 `data:text/html` 和 `data:text/javascript`。这是一个权衡：更细粒度的检查增加了维护复杂性（必须保持 MIME 类型列表最新），但恢复了合法使用案例。

→ **建议**：将 MIME 类型的允许列表范围限定为图像和字体——这是 95% 的合法使用案例，具有零安全风险（基于图像的 XSS 从根本上是不可能的）。将 `data:text/html` 和 `data:application/javascript` 保留在阻止列表中。

---

## 优先级排序建议

### 关键（立即 — 本周）

| # | 发现 | 专家 | 工作量 | 影响 |
|---|------|------|--------|------|
| C1 | 向 `ExprIR`、`StatementIR`、`ViewTree`、`MutationIntent` 添加 `#[non_exhaustive]`；从 `TypeKind`、`EventDataKind` 中移除 | 实施策略师(CRITICAL)、Rust(MEDIUM)、编译器(LOW)、务实执行者(LOW) | S (30 min) | 防止 10 个后端中的级联编译时破坏 |
| C2 | 修复 `view.rs:478` 内联样式引用名称漏洞（传递 `ref_names` 而非 `&FxHashSet::default()`） | TS/React(HIGH 2) | S (1 行) | 完成 ADR-036 Phase 2；内联样式中的 `$signal` 引用静默损坏 |
| C3 | 向 Vue (`template.rs:27`) 和 Dioxus (`view.rs:30`) 添加 `is_dangerous_url()` | 安全(HIGH) | S (各 3 行) | 与 ADR-039 为 React 修复的 XSS 向量相同 |
| C4 | 向 Rust `sanitize_css_value` (`sanitize.rs:65`) 添加 `[\r\n\t]` 标准化 | 安全(MEDIUM) | XS (1 行) | 编译时 CSS 净化中的 CR/LF 绕过 |
| C5 | 在组件边界添加 `debug_assert!(self.element_tree_stack.is_empty())` | 可维护性(MEDIUM) | S (1 行) | 防止一个组件的错误破坏所有后续组件的状态 |
| C6 | 将 JS 运行时测试集成到 `cargo test`（通过 `std::process::Command` shell 到 `node`） | AI 工程师(MEDIUM) | S (30 min) | 431 行测试，零自动化执行，3 个周期被标记 |

### 高优先级（短期 — 1–2 周）

| # | 发现 | 专家 | 工作量 | 影响 |
|---|------|------|--------|------|
| H1 | 将 React motion 查询范围限定为组件根元素（`useRef` + `querySelector` 在根元素上） | TS/React(HIGH 1)、实施策略师(HIGH)、性能(MEDIUM) | M (1–2 天) | 多组件页面中的正确性错误 |
| H2 | 在 motion 效果发出前添加 `has_hover_or_press_trigger` 门控（消除死 useEffect 块） | TS/React(MEDIUM 3) | S (遵循现有模式) | 每个仅 @enter 的组件产生约 50 行死 JS |
| H3 | 统一片段表示：使 `make_fragment_element` 返回 `ViewTree::Fragment(children)` | 领域专家(HIGH) | M (5 行 IR + 3 个后端) | 修复 Vue/Dioxus/Flutter 输出中的语义错误 |
| H4 | 修复 @for URL 净化 NID 绕过（通过 `for_attr_bindings` 循环添加 `__omni_sanitizeUrl`） | 安全(MEDIUM) | M (5 行) | @for 循环中的动态 URL 从未被净化 |
| H5 | 创建 `OmniViewFormatExpr` 实现 `FormatExpr` 特性 | 系统架构师(HIGH-2)、可维护性(MEDIUM)、实施策略师(HIGH) | M (2–3 天) | 消除 417 行重复；每次 IR 变更减少 3 个位置的更新 |
| H6 | 移除 `JsFormatExpr` 死抽象（零消费者） | 务实执行者(MEDIUM) | S (15 min) | 虚假架构信号；混淆新贡献者 |
| H7 | 将双调度路径统一为单一入口点（通过 `Codegen` 特性线程化 OmniView 配置，或将其文档化） | 系统架构师(HIGH-1)、编译器(MEDIUM) | S–M (半天) | 恢复 OmniView 的 stdlib 验证；消除死代码 |

### 重要（中期 — 2–8 周）

| # | 发现 | 专家 | 工作量 | 影响 |
|---|------|------|--------|------|
| M1 | 清理 IR 死字段：移除 `SignalIntent.is_copy`，将 `ConstructionKind` 减少为 3 个已使用的变体 | 领域专家(MEDIUM) | S (30 min) | 为所有测试移除样板；移除编译器强制执行的领域噪音 |
| M2 | 类型化 `style_classes` → `Vec<String>`，`inline_style` → `FxHashMap<String, ExprIR>` | 领域专家(MEDIUM)、Rust 专家(MEDIUM) | M (半天) | 用编译时安全性替换运行时 JSON 模式匹配 |
| M3 | 在 OmniViewGenerator 中提取 `CssState` 和 `ElementTreeState` 子结构体 | 可维护性(MEDIUM) | M (2–3 天) | 降低 47 字段结构体的认知负载 |
| M4 | 从 `omnilang-core` 中的共享版本参数化 `emit_event_object`（消除 React/Dioxus/Flutter 之间的三重重写） | 务实执行者(MEDIUM) | M (4–6 小时) | 记录在 ADR-019 中的已知发散错误 |
| M5 | 在 `collect_for_events` 中用 `FxHashSet` 替换 `Vec::contains` 去重 | 性能(MEDIUM) | S (30 min) | O(n²) → O(n)，大 @for 循环 |
| M6 | 添加跨目标回归测试矩阵（通过 `compile()` 为所有 3 个 P0 后端参数化夹具测试） | 系统架构师(MEDIUM-5)、AI 工程师(MEDIUM) | S (半天) | 4 个已确认的静默丢弃错误——零自动化防护 |
| M7 | 将 `is_dangerous_url` 从 `omniview/view/element.rs` 移动到 `crates/omnilang-core/src/codegen/sanitize.rs` | 安全(LOW) | S (30 min) | 所有后端的单一净化源 |
| M8 | 添加属性基测试用于净化器（用于 URL/CSS 输入随机化的 proptest） | 安全(LOW) | M (半天) | 净化绕过可能性的防御性覆盖 |
| M9 | 修复 `useEffect` 静默失效的 `collect_all_motion_refs` 中的同义反复 `.any(|_| true)` 和缺失的 Fragment 分支 | TS/React(MEDIUM 4, LOW 9) | S (15 min) | 代码清晰度 + 防御性深度 |

### 锦上添花（长期 — 2–6 个月）

| # | 发现 | 专家 | 工作量 | 影响 |
|---|------|------|--------|------|
| N1 | 移除 `omnilang/src/codegen/omniview/typescript.rs`（`#[cfg(test)]` 门控的死模块） | 可维护性(LOW)、务实执行者(LOW) | S (删除) | 351 行死代码 |
| N2 | 移除 `omniview/utils.rs` 中的 `map_binary_op` 重复（替换为 `binary_op_to_str(op, true, false)`） | 可维护性(LOW) | S (2 行) | 19 行死代码 |
| N3 | 修复 `is_dangerous_url` 分配（基于迭代器的检查，不分配字符串） | 性能(LOW) | S (重写函数) | 每个 URL 属性一次分配，在热路径中 |
| N4 | 懒分配 `class_name_parts`（`Option<Vec>` 而非空 Vec） | 性能(LOW) | S (20 行重构) | 90%+ 无类属性的元素避免分配 |
| N5 | 通过 `ReactFormatExpr` 特性委托替换 3 个直接 React ExprIR 处理程序 | 系统架构师(MEDIUM-3) | S (数小时) | 完成 React FormatExpr 采用 |
| N6 | 为 React 和 Flutter 代码生成实现 source map 跟踪 | 编译器(LOW) | M 每个后端 | 调试工具；React/Flutter 目前有零 source map 条目 |
| N7 | 创建 `.claude/skills/omnilang-usage/SKILL.md`（AI 智能体代码生成技能） | AI 工程师(LOW) | S (1–2 小时) | 直接提高 AI 代码生成质量 |
| N8 | 为 AI 智能体验证任务添加自动化脚本（确定性检查以取代基于 grep 的智能体判断） | AI 工程师(MEDIUM) | M (半天) | 33% 审查误判率，仅通过流程缓解 |
| N9 | 添加 Playwright E2E 测试用于生成的 JS 运行时行为（信号、净化器、事件；在真实浏览器中） | AI 工程师(MEDIUM) | L (2–3 天) | 唯一缺失的验证层 |
| N10 | 放宽 `data:` 阻止为 MIME 类型感知（允许 `data:image/*`/`data:font/*`） | 安全(MEDIUM) | S (重写检查) | 合法使用案例；零安全风险 |

---

## 如果未解决的风险

| 风险 | 时间线 | 后果 |
|------|---------|------|
| **新增 IR 枚举变体**（无 `#[non_exhaustive]`） | 下一次 IR 功能添加 | 10 个后端同时发生编译时破坏；多小时协调 |
| **Vue/Dioxus XSS**（无 URL 净化） | 已有，未检测到 | 与 ADR-039 修复的 React 供应链攻击向量相同；任何编译到 Vue/Dioxus 的恶意 .omni 组件均可执行任意 JS |
| **多组件动画冲突**（全局 querySelectorAll） | 任何渲染 2+ 相同动画组件的页面 | 非确定性动画故障；在 React Strict Mode 下极易复现 |
| **@for URL 净化静默失效** | 已有，未检测到 | 动态 URL 净化被发出但从未执行；用户相信其工作正常 |
| **IR–代码生成 FormatExpr 漂移** | 3–6 个月内新增约 2 个变体 | 每次新增 ExprIR 变体需更新 3–5 处；遗漏更新的风险增加 |
| **JS 运行时回归已合并** | 下一次运行时变更 | 信号/净化器损坏，通过所有 CI 检查，到达用户 |
| **推迟项跟踪过时** | 1–2 个冲刺周期 | 16 项在最后一次同步后已是最新状态；无流程可防止漂移 |
| **智能体审查疲劳** | 持续 | 33% 的误判率意味着每 3 次专家审查中有 1 次错误；纯流程性缓解无法规模化 |

---

## 成功指标

| 指标 | 当前 | 目标 |
|--------|---------|--------|
| IR 枚举 `#[non_exhaustive]` 覆盖 | 5 中的 2（缺失：ExprIR, StatementIR, ViewTree） | 5 中的 5（所有编译到 2+ 后端的枚举） |
| 跨后端净化对等 | 3 中的 2 个后端（缺失：Vue, Dioxus） | 3 中的 3（所有 HTML 生成后端） |
| JS 运行时测试自动执行 | 0（手动 shell 脚本） | 1（CI 集成） |
| React motion 组件隔离 | 全局 document.querySelectorAll | 组件限定的 rootRef.current.querySelectorAll |
| FormatExpr 采用 | 9 中的 8 个后端；最大的后端仅部分采用 | OmniView 至少有 12/22 个变体委托给共享分发 |
| 领域模型卫生 | 4 个死字段/类型 + 1 个双重表示 | 0 个死字段，1 个统一表示 |
| 推迟项跟踪新鲜度 | 一次性同步；无更新流程 | 每月审查节奏；各条目带"上次审查"日期 |

---

## 后续步骤

1. **立即实施**：完成 C1–C6 的 6 个关键项目（合计工作量 < 2 小时）——这些是具有高安全/正确性影响或极高影响/成本比的单行或少量修复
2. **冲刺规划**：在下个冲刺中安排 H1–H7 的 7 个高优先级项目（合计工作量约 6–8 天）。Motion 隔离（H1）有望提供最大的用户可见改进
3. **ADR 创建**：为 C1（IR 枚举 `#[non_exhaustive]` 策略）和 H7（双调度路径解决）编写 ADR——这些是需要记录以供未来维护者参考的架构决策
4. **推迟项更新**：将所有 16 个 N 级项目添加到具有触发条件和"上次审查"日期的 `deferrals.md` 中
5. **每后端专项审查**：分别对 Vue、Dioxus 和 Flutter 进行专项安全审查，因为本次审查中的安全检查发现了跨后端缺口，表明每个后端可能存在额外的独特漏洞

---

## 附录 A：各评审视角

各专家的完整评审报告可通过代理输出文件获取（`C:\Users\XHT\AppData\Local\Temp\claude\D--space-OmniLang\...\tasks\*.output`）。以下是以发现为维度的摘要矩阵：

| 发现 ID | 发现 | 严重性 | 位置 | 工作量 |
|----------|---------|----------|----------|--------|
| SYS-H1 | 双调度路径绕过 stdlib 验证 | HIGH | `lib.rs:398-418`, `mod.rs:81` | S–M |
| SYS-H2 | OmniView expr_to_js 重复 FormatExpr | HIGH | `omniview/expr.rs:1-417` | M |
| SYS-M3 | React 3 个 ExprIR 变体绕过 FormatExpr | MEDIUM | `react/expr.rs:431-461` | S |
| SYS-M4 | 47 字段 OmniViewGenerator 单体 | MEDIUM | `omniview/mod.rs:441-536` | M |
| SYS-M5 | 跨目标回归测试缺口 | MEDIUM | `deferrals.md` 第 7 项 | S |
| DOM-H1 | 片段双重表示（ViewTree::Fragment vs 合成元素） | HIGH | `ir/types.rs:676`, `ir/lower/view.rs:910` | M |
| DOM-M2 | ConstructionKind 6 变体，0 消费者 | MEDIUM | `ir/types.rs:602-610` | S |
| DOM-M3 | style_classes/inline_style 作为 serde_json::Value | MEDIUM | `ir/types.rs:622-623` | M |
| DOM-M4 | SignalIntent.is_copy = 始终为 false, 0 消费者 | MEDIUM | `ir/signals.rs:16` | XS |
| DOM-M5 | ComputedIntent.deps = 始终为空, 0 消费者 | MEDIUM | `ir/signals.rs:95` | XS |
| SEC-H1 | Vue/Dioxus 代码生成中缺少 URL 净化 | HIGH | `vue/template.rs:27`, `dioxus/view.rs:30` | S |
| SEC-M2 | @for URL 净化 NID 绕过 | MEDIUM | `list.rs:238-259`, `element.rs:180-184` | M |
| SEC-M3 | Rust CSS 净化器中的 CR/LF/TAB 标准化缺口 | MEDIUM | `sanitize.rs:65` | XS |
| SEC-M4 | data: URI 完全阻止过于保守 | MEDIUM | `element.rs:30`, `runtime.rs:159` | S |
| MNT-M1 | expr_to_js 重复 FormatExpr 分发 | MEDIUM | `omniview/expr.rs:1-417` | L |
| MNT-M2 | emit_statement 重复 FormatStmt 分发 | MEDIUM | `omniview/stmt.rs:1-285` | L |
| MNT-M3 | 47 字段生成器混合不相关的状态 | MEDIUM | `omniview/mod.rs:441-536` | M |
| MNT-M4 | element_tree_stack 平衡不变式未强制执行 | MEDIUM | `view/element.rs:42-76` | S |
| PER-M1 | 每组件 11 次独立视图树遍历（React） | MEDIUM | `react/component/mod.rs` gen_component_tsx | L |
| PER-M2 | 每个运动触发器的 IIFE 样板 JS 膨胀 | MEDIUM | `motion_triggers.rs` | M |
| PER-M3 | collect_for_events O(n²) Vec::contains 去重 | MEDIUM | `utils.rs:60` | S |
| IMP-C1 | IR 枚举缺少 #[non_exhaustive] — 级联破坏 | CRITICAL | `ir/types.rs:135,521,666` | S |
| IMP-H2 | React motion 全局 querySelectorAll — 正确性错误 | HIGH | `react/motion/mod.rs:71,93,111` | M |
| IMP-H3 | OmniView expr_to_js 复制 — 杠杆作用最高的重构 | HIGH | `omniview/expr.rs:1-417` | M |
| AI-M1 | 431 行 JS 运行时测试不在 CI 中 | MEDIUM | `tests/js_runtime/run_tests.sh` | S |
| AI-M2 | 无浏览器级 E2E 验证 | MEDIUM | 系统性缺口 | M–L |
| AI-M3 | 跨目标回归矩阵推迟 | MEDIUM | `deferrals.md` 第 7 项，ADR-021 | M |
| AI-M4 | 智能体可靠性（67%/33%）仅通过流程缓解 | MEDIUM | `agent_docs/reference.md:54-77` | M |
| PE-M1 | JsFormatExpr 有零消费者 — 死抽象 | MEDIUM | `shared/expr.rs:307-332` | S |
| PE-M2 | emit_event_object 三重重写，记录在案的分歧 | MEDIUM | `react/utils.rs`, `dioxus/expr.rs`, `flutter/motion_widgets.rs` | M |
| RUS-M1 | IR 类型中的 serde_json::Value 削弱了类型安全性 | MEDIUM | `ir/types.rs:137,621-624,766` | M |
| RUS-M2 | 基于黑名单的 CSS 净化本质上脆弱 | MEDIUM | `sanitize.rs:63-90` | M |
| RUS-M3 | IR 枚举缺少 #[non_exhaustive] | MEDIUM | `ir/types.rs:134,520,416,665` | S |
| RUS-M4 | generate() 是单一的 200 行函数 | MEDIUM | `omniview/mod.rs:179-413` | S |
| RXN-H1 | 全局 querySelectorAll 在 motion 效果中 — 跨实例干扰 | HIGH | `react/motion/mod.rs:71,93,111` | M |
| RXN-H2 | 内联样式引用名称翻译被跳过（空集） | HIGH | `react/view.rs:478` | S |
| RXN-M3 | @hover/@press 效果块即使未使用也无条件发出 | MEDIUM | `react/motion/mod.rs:87-144` | S |
| RXN-M4 | view_has_motion_or_effects 中的同义反复 .any(|_| true) | MEDIUM | `react/component/effect.rs:315` | S |
| RXN-M5 | 每次表达式调用都会克隆 FxHashSet<String> | MEDIUM | `react/expr.rs:431-461` | M |
| RXN-M6 | 硬编码 @omni/animation import，无可配置路径 | MEDIUM | `react/component/imports.rs:119` | S–M |
| RXN-M7 | 多个 useEffect 钩子增加开销（约 5 个独立效果） | MEDIUM | `react/motion/mod.rs`, `transforms.rs` | M |
| CMP-M1 | OmniView WASM/FFI 绕过 compile() 流水线检查 | MEDIUM | `lib.rs:81-178`, `codegen/mod.rs:63-118` | S |
| CMP-M2 | OmniView 是唯一不参与共享特性的后端 | MEDIUM | `omniview/expr.rs:11-417` | M |
