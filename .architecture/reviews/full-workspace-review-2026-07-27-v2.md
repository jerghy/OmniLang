# Architecture Review: OmniLang Workspace (v2 — Post-Remediation)

> **评审日期**: 2026-07-27
> **评审范围**: 全工作区（OmniLang + Poiesis + omnic + WASM）
> **评审类型**: 12 位专家并行独立复审 → 交叉分析汇总
> **基准**: [full-workspace-review-2026-07-27.md](./full-workspace-review-2026-07-27.md)（上次复审，距今 0 天）
> **变更量**: 自上次复审以来 30 commits（P35/P36/P38b/P39 全部完成）
> **务实模式**: 已启用（balanced intensity）
> **上次评审 4 项推迟**: P35/P36/P38b/P39 全部重新分类为立即执行并已完成

---

## 执行摘要

自上次评审以来的 30 次提交完成了全部 4 个"立即执行"项目——这是显著的交付速度。然而，本次评审揭示了**"已完成但未集成"的系统性模式**，其严重性高于上次评审：

- **P35 crate 提取**: omnilang-core 和 omnilang-codegen-omniview 作为独立 crate 存在，但 **omnilang facade 不依赖它们**——两个位置之间 **57+ 个 IR 类型文件完全相同**，已经出现 `#[non_exhaustive]` 漂移
- **P33 source_span**: 37 个 IR 结构体添加了字段，但测试代码未更新——**90 个测试编译错误**使整个测试套件失效
- **P36 并行 codegen**: 已实现但**零生产调用者**——所有 CLI/WASM 路径使用单目标 API
- **Poiesis 前端**: **108 个 TypeScript 编译错误**、ESLint 配置失效、1 个 CRITICAL 安全漏洞（postMessage 无 origin 验证）

**整体评估: Needs Improvement**（严重发现问题需要立即处理）。上次评审的"部分完成"模式已升级为"已完成但未验证"——交付物存在，但集成、测试和 CI 门控未跟进。

---

## 跨领域主题

### 主题 1: P35 Crate 提取不完整——孤立的提取 + 大规模类型重复

**出现于 6/12 审查**: Systems Architect, Compiler Architect, Maintainability Expert, Implementation Strategist, Pragmatic Enforcer, Rust Expert

**核心问题**: `omnilang`（facade crate）**不依赖** `omnilang-core`。两个位置独立维护 57+ 个完全相同的 IR 类型文件：

```
omnilang/src/ir/types.rs        (~1212 行) ← facade 自有副本
crates/omnilang-core/src/ir/types.rs  (~1212 行) ← 提取后的"规范"副本
```

差异已开始出现:
- 10 个 `#[non_exhaustive]` 注解存在于 facade 但不存在于 core
- P39 `optimize.rs`（561 行）仅存在于 facade，不在 core 的 `analyse.rs` 中
- facade 的 `analyse.rs` 调用 `optimize()`；core 的不调用

**影响**: 任何 IR 类型更改现在必须触及 5+ 个文件跨 2 个 crate。提取后的 crate 编译进工作空间但**无消费者链接它们**——它们增加了 ~11K LOC 却提供零价值。

### 主题 2: 测试套件完全失效（90 个编译错误）

**出现于 12/12 审查**（每位专家都提及）

**根本原因**: P33 向 37 个 IR 结构体添加了 `source_span: Option<Span>` 字段，但未更新测试代码中的结构体初始化器。`cargo check --workspace` 通过（不编译测试），但 `cargo test --workspace` 完全失败：

| 错误类型 | 数量 | 示例 |
|-----------|-------|------|
| E0063 缺少 `source_span` | 73 | `FunctionIR`、`IRProgram`、`VariantIR`、`DesignIR`... |
| E0061 参数数量不匹配 | 1 | `compile_multi_to_target` 5→6 参数 |
| E0308 FxHashMap 类型 | 10 | `javascript.rs` 使用 `HashMap` 而非 `FxHashMap` |
| 其他 | 6 | 未使用变量、不可达匹配臂... |

**影响**: 零回归检测。所有 30 个最近 commits 落在破损的测试之上。CI 未捕获此问题，因为 CI 中缺少 `cargo test --workspace`。

### 主题 3: CI 治理缺口（多维度）

**出现于 6/12 审查**: AI Engineer, Implementation Strategist, Maintainability Expert, Rust Expert, TypeScript Specialist, Security Specialist

CI 流水线缺少多个质量门控:
- `cargo test --workspace` 不在 CI 中 → 90 个破损未被检测
- `cargo fmt --check` 不在 CI 中 → 尾随空白未被捕获
- AI 评估框架（`.claude/evals/`）不在 CI 中 → 基线回归盲点
- ESLint v10 配置不兼容 → lint 不工作
- `tsc --noEmit` 不在 CI 中 → 108 个 TypeScript 错误不可见

### 主题 4: Poiesis 前端技术债务

**出现于 3/12 审查**: TypeScript Specialist, Security Specialist, AI Engineer

108 个 TypeScript 编译错误 + 1 个 CRITICAL 安全漏洞：
- `ArchitecturePreview.tsx` 缺少 postMessage origin 验证 → 任意跨域消息可触发状态变更
- ESLint 配置失效（v10 需要 `eslint.config.js`，但使用的是旧版 `eslintConfig`）
- `inlineStores` 类型化为 `unknown[]` 传入 `StoreDefinition[]` 位置
- 6 处 `console.warn/error` 绕过 Logger 抽象

### 主题 5: P36 并行 Codegen——有复杂度无价值

**出现于 3/12 审查**: Pragmatic Enforcer (CRITICAL), Performance Specialist, Systems Architect

`compile_multi_target` 使用 rayon 并行跨目标编译，但**零生产调用者**使用它——所有 CLI/WASM 路径调用单目标 API。rayon 依赖为所有消费者编译但从未被用户触发。

---

## 专家发现汇总

### 1. Systems Architect — CRITICAL: 提取后的 crate 是孤立的

**发现**: P35 提取了 2 个 crate（omnilang-core、omnilang-codegen-omniview），但 omnilang facade 不依赖其中任何一个。57+ 个文件在两个位置完全相同。提取后的 crate 是**孤立的**——零消费者链接它们。

**关键问题**:
- C1 (CRITICAL): omnilang 不依赖 omnilang-core → IR 类型在 2 个位置重复
- C2 (CRITICAL): 11 个 `#[non_exhaustive]` 注解在 facade 中存在但在 core 中缺失
- H1 (HIGH): 90 个测试编译错误阻塞 CI/CD 治理
- H2 (HIGH): 重复的公共 API 类型（`CompileResult` 在两个 lib.rs 中定义）

### 2. Domain Expert — HIGH: 信号命名不一致 + Motion 双重所有权

**发现**: `ComponentIR` 使用 `states: Vec<StateIntent>` 而 `StoreIR` 使用 `signals: FxHashMap<String, TypeIR>`——相同领域概念（signal）在两个 IR 聚合中名称不同。

**关键问题**:
- C1 (HIGH): ComponentIR.states → 应改为 ComponentIR.signals, StateIntent → SignalIntent
- C2 (HIGH): Motion/timeline 在 ModuleIR 和 ComponentIR 中双重拥有——违反单一真实来源
- C3 (MEDIUM): ConstructionKind 混合 3 个正交关注点（解析来源/结构角色/渲染策略）
- C4 (MEDIUM): TypeKind 词法类型名称 `"String"` vs `"string"` 产生不一致的 IR 表示

### 3. Security Specialist — HIGH: postMessage "*" targetOrigin

**发现**: 注入沙箱 iframe 的桥接脚本在 2 处使用 `"*"` 作为 targetOrigin，将应用运行时状态暴露给任意窗口。

**关键问题**:
- H1 (HIGH): `debug-window/index.html:316,350` 使用 `"*"` targetOrigin——修复: 改用 `e.origin`/`window.parent.origin`
- M1 (MEDIUM): ArchitecturePreview.tsx 缺少 origin/source 验证
- M2 (MEDIUM): 单一 CSP 强制所有 webview 使用 `'unsafe-eval'`——影响主窗口
- M3 (MEDIUM): 文件系统范围过于宽泛（`C:/**`、`$HOME/**`）

### 4. Maintainability Expert — CRITICAL: 大规模代码重复

**发现**: 57 个文件在 omnilang/ 和 crates/omnilang-core/ 之间完全相同。17 个更多文件略有不同。大型文件：lexer/mod.rs (1721行)、ir/lower/mod.rs (1591行)。

**关键问题**:
- C1 (CRITICAL): 两个位置之间的代码重复——每个 IR 更改必须手动同步
- C2 (CRITICAL): 90 个测试编译错误阻塞所有测试执行
- H3 (HIGH): 不完整的 P35 crate 提取——创建了 crate 但从未连线
- H4 (HIGH): 关键模块文件过大（6 个文件 >1000 行）
- M6 (MEDIUM): optimize.rs 测试覆盖率仅 2 个测试用例覆盖 561 行

### 5. Performance Specialist — HIGH: 零基准测试 + 热路径 ComponentIR 克隆

**发现**: 工作空间零 Criterion 基准测试。`build_component_map()` 为每次编译克隆所有组件的完整 IR。增量缓存克隆完整 IRProgram 而非共享 Arc。

**关键问题**:
- C1 (HIGH): 无基准测试框架——性能决策是推测性的
- C2 (HIGH): build_component_map() 为每次编译深度克隆 ~15 Vec/Map 字段
- C3 (MEDIUM): 增量缓存命中克隆完整 IRProgram（应使用 Arc）
- C4 (MEDIUM): 顺序 IR merge 在并行流水线中创建串行瓶颈
- C7 (MEDIUM): 无 rayon 线程池配置用于 WASM 上下文

### 6. Implementation Strategist — CRITICAL: CI 不执行测试

**发现**: 主要分支包含 90 个测试编译错误却通过了 CI。CI 不运行 `cargo test --workspace`。P35 提取创建了类型重复，使每个 IR 更改的成本加倍。

**关键问题**:
- C1 (CRITICAL): CI 无法通过测试——仓库与测试不兼容
- C2 (CRITICAL): P35 类型重复使每个 IR 更改的成本加倍
- H1 (HIGH): P39 optimize.rs 在错误的 crate 中（facade 而非 core）
- H2 (HIGH): compile_multi_to_target 签名更改未传播至测试

### 7. AI Engineer — HIGH: 评估未集成 CI + 无智能体遥测

**发现**: `.claude/evals/` 框架成熟但 CI 中无调用。无结构化智能体工作流遥测来追踪子代理可靠性（ADR-007 记录的 33% 误判率）。

**关键问题**:
- H1 (HIGH): 评估工具未集成 CI——基线回归盲点
- H2 (HIGH): 无结构化智能体会话日志——无法追踪 33% 子代理错误率
- M1 (MEDIUM): 3-5 分钟框架修复反馈循环是 AI 效率瓶颈
- M4 (LOW): P34 AI 集成无限期推迟无重新评估触发条件

### 8. Pragmatic Enforcer — CRITICAL: P36 零生产使用 + 9 个 feature flag 中 8 个未使用

**发现**: `compile_multi_target` 零生产调用者——所有 CLI/WASM 路径使用单目标 API。`omnilang-core` 有 9 个 codegen feature flag，但仅 `codegen-omniview` 被使用。`toast.ts` 包装了仅一个调用点的 sonner。

**务实评分**:
| 项目 | 必要性 | 复杂性 | 比率 | 结论 |
|------|---------|--------|-------|--------|
| P36 并行 codegen | 2/10 | 5/10 | 2.5 | 移除或 feature-gate |
| P33 source_span 广度 | 7/10 | 8/10 | 1.14 | 接受但立即修复测试 |
| P39 optimize.rs | 8/10 | 6/10 | 0.75 | 保留——有理由的 |
| 9 个 feature flag 仅 1 个后端 | 2/10 | 3/10 | 1.5 | 精简为 1 个 |
| toast.ts 包装器 | 1/10 | 3/10 | 3.0 | 删除，内联 |

### 9. Rust Expert (Katya) — HIGH: 12 处 `_ => unreachable!()` 吞噬新 enum variant

**发现**: 12 处生产代码中的 catch-all 匹配臂主动颠覆 Rust 的穷举性保证。`cargo fmt --check` 因尾随空白失败。14 个 clippy 错误。

**关键问题**:
- H1 (HIGH): 90 个测试编译错误——需要结构性修复（`#[derive(Default)]` + `..Default::default()`）
- H2 (HIGH): 14 个 clippy 错误在 `-D warnings` 下
- H3 (HIGH): 12 处不可达模式在生产代码中——新 enum variant 将静默 panic
- H4 (HIGH): `cargo fmt --check` 因 store.rs 中的尾随空白失败
- M1-M4 (MEDIUM): 死代码、WASM RefCell 双重获取、分离的 derive 属性、FxHashMap 不匹配

### 10. TypeScript/React Specialist (Emma) — CRITICAL: postMessage 无 origin 验证

**发现**: `ArchitecturePreview.tsx:16-23` 接受来自任意 origin 的 postMessage 且无验证——任意跨域 iframe 可触发 Poiesis 状态变更。

**关键问题**:
- C1 (CRITICAL): postMessage 监听器无 origin 验证
- H1 (HIGH): 108 个 TypeScript 编译错误——零错误基线未建立
- H2 (HIGH): `applyDebugSignalOverrides` 原地修改 `sources` 参数
- H3 (HIGH): 测试基础设施破损（缺少 @types/node，环境不匹配）
- H4 (HIGH): Store 类型接口是单一 270 行类型，包含 ~100 个方法
- M5 (MEDIUM): ESLint v10 配置不兼容——lint 不工作

### 11. Compiler Architect (Dr. Lin) — CRITICAL: IR 类型在两个位置完全重复

**发现**: 每个 IR 类型定义在 omnilang 和 omnilang-core 之间完全相同。`#[non_exhaustive]` 不匹配使外部 crate 的模式匹配不安全。TypeKind 规范化缺口。

**关键问题**:
- C1 (CRITICAL): IR 类型定义跨 2 个 crate 完全重复
- C2 (CRITICAL): `#[non_exhaustive]` 不匹配——core 中缺少使 omnilang-codegen-omniview 的模式匹配不安全
- H1 (HIGH): TypeKind 规范化缺口——`"String"` vs `"string"` 产生不同的 IR 表示
- H2 (HIGH): P39 优化无法区分不同作用域的标识符
- H3 (HIGH): Lowering 期间的内联验证错误产生无源位置的诊断

### 12. Documentation Expert (Samantha) — HIGH: stdlib API 文档仅覆盖 3/8 个模块

**发现**: 用户文档缺口：stdlib 仅 3/8 模块已文档化。71 个废弃 Rete.js 文件仍存留在 docs/ 中。README 和 QUICKSTART 指向不同的示例项目。

**关键问题**:
- H1 (HIGH): stdlib.md 仅覆盖 3/8 个模块（缺少 json、http、array、console、storage）
- H2 (HIGH): 71 个废弃 Rete.js 文件污染 docs/ 命名空间
- H3 (HIGH): README.md 和 QUICKSTART.md 在示例项目上存在分歧
- H4 (HIGH): ADR-005 标题内部写为 "ADR 1" 而非 "ADR-005"

---

## 权衡与分歧

### 权衡 1: source_span 广度 vs 实际价值

**Performance Specialist**: P33 向所有 37 个结构体添加了 `source_span: Option<Span>` 但仅填充了 6 个——增加了维护负担，且无消费者价值。

**Compiler Architect**: 添加 source_span 是所有结构体正确的架构决策——它使 API 一致，并按需填充（ADR-008 选择性填充策略）。

**Pragmatic Enforcer**: 必要性 7/10 但复杂性 8/10。该决策是正确的，但执行因未更新测试而不完整。

**决议**: 保留 source_span 在所有结构体上（一致 API 是正确的），但通过修复测试 + 记录仅 6/37 已填充来清除破损。剩余 31 个需在消费者出现时填充（遵循 ADR-008 策略）。

### 权衡 2: P35 Crate 提取——先创建后连线 vs 一步完成

**Systems Architect**: 两步方法（先提取，后连线）造成了当前孤立的 crate 局面——应一步完成。

**Implementation Strategist**: 两步方法是合理的变更管理（每个步骤可独立验证），但第二步（连线）从未发生。

**决议**: 步骤 1 已完成（提取）。步骤 2（连线 facade 依赖 core）是缺失的关键步骤——必须在任何后续 IR 更改之前完成。

### 权衡 3: `_ => unreachable!()` 匹配臂 vs 穷举性匹配

**Rust Expert**: 12 处 `_ => unreachable!()` 主动破坏 Rust 的穷举性保证——应移除，让编译器在添加新 enum variant 时强制更新。

**Compiler Architect**: 部分同意——但指出某些 catch-all 是故意的（防御性代码），因为 IR enum 跨越 crate 边界序列化，可能来自外部输入。

**决议**: 对**内部** enum（ExprIR、StatementIR、ViewTree 等）移除 catch-all——让编译器强制穷举性。仅对序列化边界枚举保留防御性 catch-all，并添加 `#[non_exhaustive]`。

---

## 优先级建议

### Critical（0-2 周——阻塞所有其他工作）

| # | 优先级 | 描述 | 发现者 | 工作量 |
|---|--------|------|--------|--------|
| P1 | **CRITICAL** | 修复 90 个测试编译错误——添加 `source_span: None`、修复 FxHashMap、修复函数签名 | 全部 12 位 | M（1-2h） |
| P2 | **CRITICAL** | 添加 `cargo test --workspace` 至 CI——阻止未来破损合并 | AI Engineer + Impl Strategist | S（10 行 YAML） |
| P3 | **CRITICAL** | 修复 postMessage origin 验证——`ArchitecturePreview.tsx` + debug-window bridge `"*"` targetOrigin | Security + TS Specialist | S（5 分钟） |
| P4 | **CRITICAL** | 完成 P35 提取：使 omnilang 依赖 omnilang-core，移除重复 IR 文件 | Systems + Compiler Architect | L（1-2 天） |
| P5 | **CRITICAL** | 对齐 `#[non_exhaustive]`：从 facade 复制 10 个注解至 omnilang-core | Compiler Architect | S（10 分钟） |

### Important（2-8 周——应在下个里程碑前处理）

| # | 优先级 | 描述 | 发现者 | 工作量 |
|---|--------|------|--------|--------|
| P6 | **HIGH** | 修复 14 个 clippy 错误 + 12 处 `_ => unreachable!()` 模式 | Rust Expert | M（2h） |
| P7 | **HIGH** | 修复 `cargo fmt --check`——移除 store.rs 中的尾随空白 | Rust Expert | S（1 分钟） |
| P8 | **HIGH** | 建立 108 个 TS 错误的零错误基线 | TS Specialist | M（3-4h） |
| P9 | **HIGH** | 修复 ESLint v10 配置（eslint.config.js 替代 eslintConfig） | TS Specialist | M（1h） |
| P10 | **HIGH** | 修复 6 个直接 `console.*` 调用以使用 Logger | TS Specialist | S（15 分钟） |
| P11 | **HIGH** | 添加 Criterion 基准测试（lexer、pipeline、codegen） | Perf Specialist | M（半天） |
| P12 | **HIGH** | 修复 TypeKind 规范化：`String` → `TypeKind::String` | Compiler Architect | S（1 行） |
| P13 | **HIGH** | 将评估框架集成至 CI | AI Engineer | S（10 行 YAML） |
| P14 | **HIGH** | 移除或 feature-gate `compile_multi_target` + rayon 依赖 | Pragmatic Enforcer | S（20 分钟） |
| P15 | **HIGH** | 移除 8 个未使用的 codegen feature flag | Pragmatic Enforcer | S（5 分钟） |
| P16 | **HIGH** | 补齐 stdlib.md（json、http、array、console、storage） | Doc Expert | M（1-2 天） |
| P17 | **HIGH** | 将 P39 optimize.rs 移至 omnilang-core | Compiler Architect | S（1h） |
| P18 | **HIGH** | 重命名 `ComponentIR.states` → `signals`, `StateIntent` → `SignalIntent` | Domain Expert | M（部分天） |
| P19 | **HIGH** | 从 ComponentIR 移除 motions/timelines（仅保留在 ModuleIR） | Domain Expert | M（部分天） |
| P20 | **HIGH** | 替换 `build_component_map()` 克隆为引用传递 | Perf Specialist | M（2-3h） |

### Nice-to-Have（2-6 个月——长期改进）

| # | 描述 | 发现者 | 工作量 |
|---|------|--------|--------|
| P21 | 添加结构化智能体会话日志记录 | AI Engineer | M |
| P22 | 拆分大型文件（lexer/mod.rs 1721 行、lower/mod.rs 1591 行） | Maint Expert | L |
| P23 | 用 `CSSStyleSheet.adoptedStyleSheets` 替换动态 `<style>` 注入 | Security | M |
| P24 | 将 71 个 Rete.js 文件移至 `docs/archive/` | Doc Expert | S |
| P25 | 为 P34 AI 补全添加具体的重新评估触发条件 | AI Engineer | S |
| P26 | 减少 CodeWriter 分配（`Vec<String>` → 单 String 缓冲区） | Perf Specialist | S |
| P27 | 向 optimize.rs 添加基于属性的测试（覆盖所有 20+ ExprIR 变体） | Maint Expert | M |
| P28 | 对齐 README.md 和 QUICKSTART.md 示例项目 | Doc Expert | M |

---

## 若不解决的风险

| 风险 | 触发条件 | 影响 |
|------|---------|------|
| IR 类型漂移导致静默错误 | 新 enum variant 添加至 facade 但未同步至 core | omnilang-codegen-omniview 遗漏新 variant，运行时 panic |
| 零回归检测 | 任意合并落在 90 个破损测试之上 | 所有未来更改未经测试——bug 静默积累 |
| postMessage 欺骗 | 恶意页面或扩展在同一设备上打开 | Poiesis 架构状态可被任意消息操控 |
| CI 盲点 | 缺少 `cargo test --workspace` 在 CI 中 | 破损从未被捕获——测试套件衰亡为无用 |
| 类型身份碎片化 | `String` 和 `string` 产生不同的 IR enum variant | 类型检查不可靠，跨模块比较产生错误结果 |
| 性能退化不可见 | 无基准测试框架 | 优化或回归是推测性的，无法衡量进度 |

---

## 成功指标

本次评审后的目标状态：
- `cargo test --workspace` → 零编译错误，所有测试通过
- `tsc --noEmit` (Poiesis) → 零错误
- `cargo fmt --check --workspace` → 通过
- `cargo clippy --workspace -- -D warnings` → 通过
- CI 在所有 PR 上执行以上全部
- `omnilang` 依赖 `omnilang-core`（移除重复文件）
- 已应用 postMessage origin 修复
- 评估框架在 CI 中运行

## 后续步骤

1. 🔴 **立即**: 修复 90 个测试编译错误（P1）——解锁整个测试套件
2. 🔴 **立即**: 修复 postMessage origin 验证（P3）——消除运行时安全漏洞
3. 🔴 **本 sprint**: 将 CI 测试添加至 CI（P2）——防止破损重新进入
4. 🟡 **本 sprint**: 完成 P35 提取（P4）——消除 57+ 文件双重维护
5. 🟡 **下个 sprint**: 修复 P6-P20（HIGH 优先级项目）
6. 📋 **记录决策**: 为 P4 (P35 完成) 和 P6 (unreachable 模式移除) 创建 ADR

---

> **12 位专家**: Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert (Katya), TypeScript/React Specialist (Emma), Compiler Architect (Dr. Lin), Documentation Expert (Samantha)
