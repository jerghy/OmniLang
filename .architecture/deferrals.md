# 推迟项跟踪

> 创建：2026-07-27 | 来源：`.architecture/reviews/implementation-plan-2026-07-26.md`
> 计划评审要求创建此文件以跟踪推迟的 5 个项目。

---

## 已推迟项目

### P35: Crate 拆分评估

**推迟原因**：YAGNI — 编译器当前 ~63K 行，无拆分痛点。计划评审中务实执行者评为 "Ratio 2.0——过早优化"。

**重新评估触发条件**：
- 编译器 >100K 行 且 ≥5 个活跃后端
- 或特定 crate（如 codegen）编译时间 >30s 成为开发瓶颈

**当前状态**：`omnilang` 单一 crate，按模块目录组织（lexer/parser/semantic/ir/codegen），模块边界清晰。

---

### P36: 代码生成并行化

**推迟原因**：无 profiling 数据证明 codegen 是瓶颈。当前解析+语义分析已使用 rayon 并行化。

**重新评估触发条件**：
- codegen 阶段 >500ms（500 行 .omni 文件）
- 或用户报告编辑器编译延迟

**当前状态**：WASM 编译 20 文件项目 0.31s（见 `.architecture/metrics/wasm-baseline.md`）。Rayon 在 parsing/semantic 阶段使用，codegen 单线程。

---

### P39: IR 优化传递

**推迟原因**：优化前需要 motion 测试达到 95% 分支覆盖率。当前 motion 回归测试 13 个，但覆盖率未知（vitest 工具链损坏）。

**重新评估触发条件**：
- P9 motion 回归测试覆盖率 ≥95% 分支
- 且 profiling 显示 IR 优化可带来可量化的运行时收益

**当前状态**：IR lowering 直接翻译 AST→IR，无优化传递。移除未使用信号和内联简单 computed 需可靠的测试安全网。

---

### P38b: Strip TAURI_INTERNALS

**推迟原因**：防御性安全——等 iframe 沙箱（P38）先生效再评估必要性。P38 已实现 `sandbox="allow-scripts"` + postMessage 桥接。

**重新评估触发条件**：
- P38 iframe 沙箱在生产环境运行 30 天无事故
- 且安全审计确认 `__TAURI_INTERNALS__` 暴露构成实际风险

**当前状态**：P38 已完成。`__TAURI_INTERNALS__` 仅在主窗口可用（非 debug webview），debug webview 仅 `core:default` + `core:event:default`。

---

## 已移除项目

### P34: AI 代码补全

**移除原因**：新功能非债务偿还。计划评审中 3/3 专家一致：类别错误——在 Consolidation 阶段引入 feature 传递混乱信号。属于独立的专项计划。

**重新评估**：编辑器和编译器核心稳定后另立专项计划。Ratio 4.0（复杂度远超必要性）。

---

## 跟踪约定

- 每个推迟项在重新评估触发条件满足时重新打开
- 推迟项**不是**废弃项——它们是"现在不做，等时机成熟再做"的决策
- 本文件在每次 Sprint 回顾时更新状态
- 新增推迟项追加到本文件末尾

---

> 来源：`.architecture/reviews/plan-review-2026-07-26.md`（实施策略专家 + 务实执行者 + 系统架构师的 3 专家评审）
