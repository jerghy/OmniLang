# Architecture Review: OmniLang Workspace (v4 — Deep Verification Pass)

> **评审日期**: 2026-07-27
> **评审范围**: v3 广度评审中所有 CRITICAL/HIGH/MEDIUM 发现的对抗性验证
> **评审类型**: 12 位专家并行深度验证（对抗性——每位专家深入阅读代码以确认/反驳/精细化 v3 发现）
> **基准**: [full-workspace-review-2026-07-27-v3.md](./full-workspace-review-2026-07-27-v3.md)

---

## 执行摘要

v3 广度评审后，12 位专家带着"证伪"的使命重新审查了代码——深入阅读实际代码以确认、反驳或精细化每个发现。结果：v3 发现的**精确度约 70%**。若干高调发现被过度陈述（数字夸大、严重性错误归类），但几个最关键的发现被**确认甚至强化**。

**整体评估：v3 发现的核心正确，但噪声比约 30%。** 最高价值的发现（codegen 重复、analyse.rs 行为分歧、theme 数据丢失）经受住了审查。过度陈述的发现（"零文档"、Clippy lint 不存在、组件测试行数 5 倍夸大）降低了 v3 的信噪比。

### 被验证确认的关键事实

| 发现 | 验证结果 | 精确数字 |
|------|---------|---------|
| Codegen 重复（孤儿 crate + fork） | ✅ 确认 | 10,163 行孤儿 + ~3,100 行 fork（非声称的 16,000） |
| `analyse.rs` 优化阶段分歧 | ✅ 确认 | 核心 FFI/WASM 路径缺少 `optimize()` 调用 |
| Theme 数据从不传输到 IR | ✅ **强化** | `themes: Vec::new()`——主题不仅是类型擦除，而是**完全丢弃** |
| Facade 重复定义领域类型 | ✅ 确认 | `SymbolInfo` 和 `CompileResult` 逐字节相同但独立定义 |
| 5 个测试失败被接受 | ✅ 确认 | 2 Rust（类型泛型）+ 3 TS（localStorage 环境 + schema 回归） |
| 5 个未使用的依赖 | ✅ 确认 | `pathdiff` 整个工作区未使用 |
| 零 coverage reporting | ✅ 确认 | CI 中完全不存在 |

### 被验证反驳/过度陈述的发现

| 发现 | 原严重性 | 实际状态 | 关键修正 |
|------|---------|---------|---------|
| "Rust API 零文档" | CRITICAL | ❌ 过度陈述 | 所有公共函数都有文档注释（简短但存在）；12 个模块声明缺注释 |
| "Clippy 标记 3.14" | HIGH | ❌ 不存在 | `approx_constant` 是 allow-by-default——从未启用 |
| "items_after_test_module" | HIGH | ❌ 下游为 LOW | lint 是 allow-by-default；clippy 不标记 |
| "codegen 副本行为分化" | HIGH | ❌ 仅 import 路径 | 差异仅 crate 路径替换，零行为差异 |
| "find_motion 线性扫描" | MEDIUM | ❌ 死代码 | 方法存在但**从未被调用**——不是性能问题 |
| "5 个测试文件 >800 行" | HIGH | ❌ 实际 2 个 | timeline.rs (567)、list.rs (352)、regression/mod.rs (679) 均远低于 800 |
| "114K TSX 行" | — | ❌ 实际 21K | 5 倍夸大 |
| "72 字节每个字面量" | HIGH | ❌ 误导 | 数字/布尔值无堆分配；仅 String/Array/Object 有 |
| "16 个重复文件" | CRITICAL | 🔄 9 个完全相同 | 仅 1 个（generator.rs）有语义分歧 |
| "9 个 feature flags" | MEDIUM | ❌ 实际 4 个 | |
| "62 个 Rete.js 文件" | HIGH | 🔄 实际 71 个 | |
| "getState() 绕过 React" | HIGH | 🔄 上下文错误 | 0 次在 render 路径中；是惯用 Zustand 模式 |
| "88 getState()" | HIGH | 🔄 严重性过度 | 事件处理器/副作用中的 `getState()` 是惯用 Zustand 模式 |
| "Eval 不在 CI 中" | HIGH | 🔄 事实错误 | eval 在 CI 中（`evals` job），只是 `continue-on-error: true` |

---

## 跨领域主题（验证后）

### 主题 1: P35 去重不完整——真实但过度陈述（8/12 → 经 3/12 验证精细化）

**精确状态**：
- orphan crate `omnilang-codegen-omniview` 存在：✅（10,163 行，零消费者）
- `omnilang/src/codegen/omniview/`（活跃副本）：✅（10,569 行）
- `omnilang/src/codegen/shared/` vs `crates/omnilang-core/src/codegen/shared/`：✅ 但仅 ~3,100 行（声称 5,000）
- 9 个重复文件完全相同：✅
- 语义分歧：**仅 1 个文件**（`shared/generator.rs`——facade 仍使用 Task 21 之前的克隆版本）
- `analyse.rs` 行为分歧：✅（核心缺少 `optimize()` 调用——CRITICAL）
- 其余差异都是机械性的（`pub(crate)` vs `pub`，feature gate 包装，import 路径）

**修正后的风险**：v3 声称的 "16 个不同文件" 实际上只有 1 个有语义分歧。facade 到核心的重连仍需进行，但其影响范围比最初声称的要集中得多。

### 主题 2: 文件大小——被内联测试模块大量膨胀（6/12 → 经 2/12 验证精细化）

**精确状态**：仅 **3 个生产文件**真正超过 800 行：
1. `ir/types.rs`（1,213 行生产代码）
2. `parser/expr/mod.rs`（1,075 行生产代码）
3. `view/element.rs`（1,017 行生产代码）

声称的其余 "大文件" 主要是由内联 `#[cfg(test)] mod tests` 膨胀的——它们各自的生产代码均低于 800 行。

### 主题 3: CI 治理缺口——真实但 eval 已在 CI 中（5/12 → 经 3/12 验证精细化）

| 缺口 | v3 声称 | 实际状态 |
|------|---------|---------|
| Coverage | ❌ 缺失 | ✅ 确认缺失 |
| WASM wasm32 目标 | ❌ 未测试 | ✅ 确认；15 个原生测试存在但 wasm32 目标未构建 |
| Eval CI | ❌ 缺失 | 🔄 **实际已在 CI 中**（`evals` job 存在于 ci.yml 第 107 行），但 `continue-on-error: true` |
| Toolchain pin | ❌ 缺失 | ✅ 确认缺失 |
| cargo-deny | ❌ 缺失 | ✅ 确认缺失 |
| 测试门控 | ❌ 宽松 | ✅ 确认——5 个失败不阻断 CI |

### 主题 4: 测试债务——真实但前端行数被过度陈述（6/12 → 经 3/12 精细化）

- 5 个失败：✅ 确认
- TSX 行：**21,203**（非 114,000）
- 组件测试：✅ 确认（仅 2 个）
- Coverage：✅ 确认缺失

### 主题 5: 性能热点——大部分确认，部分过度陈述（4/12 → 经 2/12 精细化）

- ComponentIR 克隆：✅ 确认（Task 21 未迁移到 OmniView）
- `serde_json::Value` 字面量：🔄 "72 字节每个" 不准确（数字/布尔值无堆分配）
- `all_components()` 14 次：✅ 确认（声称 12+，实际 14）
- Rename map 克隆：✅ 确认
- `format!` 32 次：✅ 确认（精确），但微优化领域

---

## v3→v4 严重性降级摘要

| 发现 | v3 严重性 | v4 严重性 | 原因 |
|------|---------|---------|------|
| Rust API 零文档 | CRITICAL | **HIGH** | 文档存在但简短；"零"是错误的 |
| Clippy 标记 3.14 | HIGH | **LOW** | Lint 未启用；纯粹风格 |
| items_after_test_module | HIGH | **LOW** | Clippy 不标记；仅风格约定 |
| codegen 行为分化 | HIGH | **MEDIUM** | 仅 import 路径差异；零行为差异 |
| "16 个重复文件" | CRITICAL | **HIGH** | 9 个完全相同，仅 1 个语义分歧 |
| "5,000 行 fork" | CRITICAL | **HIGH** | 实际 ~3,100 行 |
| 5 个测试文件 >800 行 | HIGH | **LOW** | 实际 2 个 |
| getState() 88 次 | HIGH | **LOW** | 惯用 Zustand；0 次在 render 路径 |
| bridge (0,eval) | HIGH | **MEDIUM** | 在沙盒 iframe 内（无 Tauri API） |
| Eval 不在 CI 中 | HIGH | **MEDIUM** | 实际已在 CI 中，仅非门控 |
| WASM 仅 2 个测试 | HIGH | **MEDIUM** | 实际 15 个测试；仅 wasm32 目标未构建 |
| find_motion 线性扫描 | MEDIUM | **LOW** | 死代码——从未被调用 |
| 72 字节每个字面量 | HIGH | **MEDIUM** | 基本类型无堆分配 |

---

## 确认的优先事项（v4 最终版）

### CRITICAL（0-2 天——确认值得立即处理）

| # | 发现 | 验证状态 | 工作量 |
|---|------|---------|--------|
| C1 | **`analyse.rs` 优化阶段分歧**——核心 FFI/WASM 路径缺少 `optimize()` 调用 | ✅ 经 3 位专家确认 | S (30min) |
| C2 | **孤儿 crate `omnilang-codegen-omniview`**——10,163 行，零消费者，浪费 CI | ✅ 确认 | S (删除 + 重建) |
| C3 | **Facade 重复类型**（`CompileResult`、`SymbolInfo`）——逐字节相同但独立定义 | ✅ 确认 | S (1h) |
| C4 | **5 个测试失败**——2 Rust（类型泛型）+ 3 TS（localStorage + schema） | ✅ 确认 | S (1h) |
| C5 | **5 个未使用依赖**（`pathdiff` 完全死亡） | ✅ 确认 | S (5min) |
| C6 | **Theme 数据从不传输到 IR**——`themes: Vec::new()` 总是空的 | ✅ **强化**（比原报告更严重） | M (需要完整实现) |

### HIGH（1-2 周——真实但不如最初声称的严重）

| # | 发现 | 验证状态 | 工作量 |
|---|------|---------|--------|
| H1 | **Codegen shared 基础设施重连**——将 facade 重连到核心 re-export（~3,100 行，9 个文件完全相同） | 🔄 已精细化 | M (2-4h) |
| H2 | **CI 缺口**：添加 coverage + wasm32 构建 + cargo-deny + toolchain pin + eval gate | 🔄 Eval 已在 CI 中 | M |
| H3 | **文件拆分**：仅 3 个生产文件真正超过 800 行 | 🔄 已精细化 | S-M |
| H4 | **ComponentIR 克隆**——Task 21 从未迁移到 OmniView | ✅ 确认 | S |
| H5 | **CSP `unsafe-eval` + FS scope**——危险组合 | ✅ 确认 | L |
| H6 | **删除 `multi-target` 死功能标志** + 删除 4 个未使用的 omnilang-core 功能标志 | ✅ 确认（4 个，非 9 个） | S |

### MEDIUM（本月）

| # | 发现 | 验证状态 |
|---|------|---------|
| M1 | 3 个 HSVPanel `exhaustive-deps` 抑制——**实际运行时错误风险** | ✅ 确认（唯一确认的 hook bug） |
| M2 | 核心公共 API 文档简短——12 个模块声明缺少 `//!` 文档 | 🔄 已精细化 |
| M3 | `LiteralIR` 替代 `serde_json::Value` | 🔄 已精细化 |
| M4 | `TypeKind::Union` 变体保留结构化类型信息 | ✅ 确认 |
| M5 | CONTRIBUTING.md + 形式语法规范 | ✅ 确认缺失 |
| M6 | `applyDebugSignalOverrides` 变异——违反编码规范 | ✅ 确认 |

---

## 方法论说明

v4 深度验证暴露了 AI 辅助广度审查中的系统性模式：

1. **数字通常被 30-60% 过度陈述**（行数、文件数、调用次数）。验证通过精确工具测量始终值得做。
2. **"零 X" 声称应触发怀疑**。若干 "零" 声称是错误的（"零文档"、"零 eval CI"、"零 wasm 测试"）。
3. **"行为分歧" 声称在缺乏 diff 验证时不可靠**。声称的 "codegen 行为分歧" 仅为 import 路径差异。
4. **死代码声称在缺乏调用点验证时不可靠**。`find_motion` 被声称有性能影响但实际是死代码。
5. **严重性过度陈述集中在文件大小和 lint 相关发现上**。在此次验证中，5 个 original HIGH 被降为 LOW/MEDIUM。
