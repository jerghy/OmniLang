# Architecture Review: OmniLang Workspace (v3 — Third Comprehensive Pass)

> **评审日期**: 2026-07-27
> **评审范围**: 全工作区（OmniLang + Poiesis + omnic + WASM）
> **评审类型**: 12 位专家并行独立复审 → 交叉分析汇总
> **基准**: [full-workspace-review-2026-07-27-v2.md](./full-workspace-review-2026-07-27-v2.md)（上次复审，同日早些时候）
> **变更量**: 自 v2 评审以来 5 commits（P35 去重完善、Task 18/19/21、benchmarks）
> **务实模式**: 已启用（balanced intensity）

---

## 执行摘要

自上次 v2 评审以来，P35 去重 IR 层已成功完成（omnilang facade 现在从 omnilang-core re-export），测试从 90 个编译错误恢复到 424/426 通过，clippy 零错误，Poiesis TypeScript 从 108 错误归零。**整体评估：Adequate → 接近 Strong**，但 P35 去重暴露出一个**系统性未完成模式**：IR 层已去重，但 codegen 基础设施层（~16,000 行 Rust）仍然在两个位置完全重复。

**本次评审最大的发现**：P35 提取是"加法的"而非"替换的"——提取后的 `crates/omnilang-codegen-omniview/` 是一个孤儿 crate（零消费者），而 `omnilang/src/codegen/` 中的所有 codegen 基础设施与 `crates/omnilang-core/src/codegen/` 完全重复。两个位置的 16 个文件已经出现分歧。每次 codegen 修复都需要在两个位置独立应用。

**三大优先事项**：
1. **CRITICAL**: 消除 ~16K 行 codegen 重复（孤儿 crate + forked shared infrastructure）
2. **CRITICAL**: 修复 5 个被接受的测试失败 + 修复 `analyse.rs` 优化阶段分歧
3. **HIGH**: 关闭 CI 治理缺口（coverage、WASM 测试、eval CI、toolchain pin）

---

## 跨领域主题

### 主题 1: P35 去重不完整 — Codegen 基础设施层仍在两个位置重复 (8/12 审查)

**出现于**: Systems Architect, Domain Expert, Pragmatic Enforcer, Rust Expert, Implementation Strategist, Maintainability Expert, Compiler Architect, Performance Specialist

这是本次审查中覆盖面最广、严重性最高的主题。

**具体重复量**：
| 类型 | 行数 | 文件数 | 状态 |
|------|------|--------|------|
| `omnilang/src/codegen/omniview/` vs `crates/omnilang-codegen-omniview/` | ~10,500 / ~10,100 | ~20+ | 已分歧（406 行差异） |
| `omnilang/src/codegen/shared/` vs `crates/omnilang-core/src/codegen/shared/` | ~1,712 / ~1,700 | 8 | 已分歧 |
| standalone codegen files (event, helpers, json, sanitize, stdlib, target, traits, writer) | ~1,230 / ~1,200 | 8 | 已分歧 |
| `analyse.rs` | 567 / 567 | 1 | **行为分歧**（核心缺少 optimize 阶段） |
| `ffi.rs` | ~250 / ~250 | 1 | 249 行差异 |

**关键影响**：
- `crates/omnilang-codegen-omniview/` 是**孤儿 crate**——零消费者链接它，但每次 CI 都编译它（浪费算力）
- `analyse.rs` 的分歧是**行为级别**的：核心路径不运行 `optimize()` 阶段，导致通过核心 API 调用时静默产生未优化的 IR
- 任何 codegen 基础设施修复需要触及两个位置，且 CI 不检测同步状态

### 主题 2: 文件大小系统性超标 (6/12 审查)

**出现于**: Maintainability Expert, Compiler Architect, Performance Specialist, TypeScript Specialist, Rust Expert, Documentation Expert

10 个 Rust 文件超过 800 行（违反 CLAUDE.md 规范），其中 4 个超过 1,200 行：

| 文件 | 行数 | 类型 |
|------|------|------|
| `lexer/mod.rs` | 1,721 | 生产 745 + 测试 976 |
| `ir/lower/mod.rs` | 1,591 | 生产 267 + 测试 1,324 |
| `parser/mod.rs` | 1,397 | 生产 + 测试 |
| `ir/types.rs` | 1,216 | 类型定义 |
| OmniView `tests.rs` | 1,376 | 仅测试 |
| `parser/expr/mod.rs` | 1,075 | 生产 + 测试 |
| `view/element.rs` | 1,017 | 仅生产 |
| `VariantEditor.tsx` | 1,601 | TypeScript 组件 |

**模式**: 内联测试模块（`#[cfg(test)] mod tests`）是主要驱动因素——`lexer/mod.rs` 56% 是测试代码，`ir/lower/mod.rs` 83% 是测试代码。

### 主题 3: CI 治理缺口仍然存在 (5/12 审查)

**出现于**: AI Engineer, Implementation Strategist, Security Specialist, Rust Expert, Maintainability Expert

| 缺口 | 上一次状态 | 当前状态 |
|------|-----------|---------|
| `cargo test --workspace` 不在 CI | ❌ 未检测 | ✅ 已在 CI（但容忍 2 个失败） |
| coverage reporting | ❌ 无 | ❌ 仍然无 |
| WASM 编译 CI | ❌ 无 | ❌ 仍然无 |
| eval scripts CI | ❌ 无 | ❌ 仍然无 |
| `rust-toolchain.toml` | ❌ 无 | ❌ 仍然无 |
| `cargo deny` / `cargo audit` | ❌ 无 | ❌ 仍然无 |
| 测试失败阻断 CI | ❌ 宽松 | ❌ 仍然宽松（5 个失败被接受） |

### 主题 4: 测试债务 (6/12 审查)

**出现于**: Implementation Strategist, TypeScript Specialist, Rust Expert, Maintainability Expert, Documentation Expert, AI Engineer

- 5 个被接受的测试失败（2 Rust + 3 TypeScript）正在建立"失败常态化"
- Poiesis 前端仅 2 个 React 组件测试覆盖 114K 行 TSX
- 零 Store 集成测试、零 E2E 测试
- 零 coverage reporting——不知道 624 个 Rust 测试覆盖了多少行
- `omnilang-wasm` 仅 2 个测试（都是类型绑定导出，非运行时编译测试）

### 主题 5: 性能热点 — 不必要的克隆和分配 (4/12 审查)

**出现于**: Performance Specialist, Systems Architect, Compiler Architect, Pragmatic Enforcer

- `ComponentIR` 全量克隆用于 component_map（Task 21 修复了 shared 但 OmniView 后端仍克隆）
- `serde_json::Value` 用于所有字面量表达式——72+ 字节堆分配，每次 clone 深遍历
- `all_components()` 每次调用构建新 HashMap（12+ 调用点）
- `format!` 链创建中间 String 临时变量
- IncrementalCache 克隆整个 IRProgram

---

## 专家发现汇总

### 1. Systems Architect — CRITICAL: 核心-外观双管道分歧

**发现**: `crates/omnilang-core/src/lib.rs` 和 `omnilang/src/lib.rs` 都定义了 `compile()`、`run_pipeline()` 和 `CompileResult`。它们是独立的类型——`omnilang_core::CompileResult` ≠ `omnilang::CompileResult`。

**关键问题**:
- C1 (CRITICAL): 双管道——根据调用路径可能产生不同输出
- H1 (HIGH): IncrementalCache 序列化使跨会话缓存失效（所有程序反序列化为 default）
- H2 (HIGH): 代码生成分发使用特性门控 match（10 个分支 + `#[allow(unreachable_patterns)]`）
- H3 (HIGH): Poiesis 多个 Mutex 存在死锁风险（无锁顺序文档）
- M1-M3 (MEDIUM): 重复类型定义、analyse.rs 管道重复、CLI 错误绕过诊断系统
- L1-L2 (LOW): ts-rs 检查仅 Linux、WASM 输入限制常量重复

**推荐**: 立即从 facade 删除重复的 `compile()` / `run_pipeline()` / `CompileResult`，改为从核心 re-export。添加锁顺序文档。

### 2. Domain Expert — CRITICAL: 领域类型定义重复

**发现**: 编译器核心领域概念（`CompileResult`, `SymbolInfo`, `PipelineOutput`）在 facade 和 core 中独立定义。

**关键问题**:
- C1 (CRITICAL): facade 重复定义 `CompileResult`, `SymbolInfo`——两个权威来源
- H1 (HIGH): Design import 解析在 IR lowering 而非语义分析中进行
- H2 (HIGH): Theme 领域概念在 AST→IR 之间丢失（存储为 `Vec<serde_json::Value>`）
- M1-M3 (MEDIUM): 信号表示不对称、VariantIR 名称冲突、ConstructionKind 泄漏 codegen 关注点
- L1-L3 (LOW): BindingIR 概念混淆、语义分析不验证组件、TS 命名不一致

**推荐**: 消除 facade 中的重复领域类型。将 design import 解析移至语义分析阶段。定义正确的 `ThemeIR` 类型。

### 3. Security Specialist — 零 CRITICAL ✅ (安全态势显著改善)

**发现**: Poiesis 沙盒隔离模型正确（sandbox iframe + postMessage origin 验证）。核心编译器无 I/O 攻击面。无硬编码机密。

**关键问题**:
- H1 (HIGH): CSP 允许 `'unsafe-eval'`——主窗口 XSS 可获取 Tauri API 访问
- H2 (HIGH): FS scope 过于宽泛（`$HOME/**`, `C:/**`, `D:/**`）
- H3 (HIGH): bridge 脚本使用 `(0,eval)` 进行热替换
- M1-M5 (MEDIUM): save_project 无验证、CLI 文件名验证不完整、FFI unwrap、compile_project 路径未规范化、requireLiteralLeadingDot: false
- L1-L4 (LOW): CLI 任意输出路径、watch 文件描述符、无 CVE 扫描、omni-animation.js 嵌入

**推荐**: 收紧 FS scope。添加 `deny.toml` + 安全 CI。将热替换从 `(0,eval)` 迁移到 `<script>` DOM 附加。

### 4. Maintainability Expert — 零 CRITICAL ✅

**发现**: 代码库质量轨迹积极（P35 去重、clippy 零错误、零 unwrap/panic）。文件大小分布是主要问题。

**关键问题**:
- H1 (HIGH): 10 个 Rust 文件超过 800 行（违反 CLAUDE.md）
- H2 (HIGH): `OmniViewGenerator` 是 46 字段单体，方法分散在 20+ 文件中
- H3 (HIGH): `keyframe_to_config` 死代码（`#[allow(dead_code)]` 抑制，60 行）
- H4 (HIGH): 5 个 Rust 测试文件超 800 行
- M1-M4 (MEDIUM): VariantEditor 1,601 行、parser/expr 1,075 行、29 文件 >500 行、helpers.rs 混杂
- L1-L3 (LOW): ir/types.rs 可拆分、is_false helper、递归深度 guard

**推荐**: 提取内联测试模块。拆分 `ir/types.rs`。分解 `OmniViewGenerator` 为子结构体。

### 5. Performance Specialist — 2 HIGH 性能热点

**发现**: 编译器基础扎实（零拷贝 lexer、并行 pipeline、FxHashMap 全局使用），但存在几个系统性克隆热点。

**关键问题**:
- H1 (HIGH): OmniView codegen 全量克隆 `ComponentIR` 用于 component_map
- H2 (HIGH): `serde_json::Value` 用于每个字面量表达式（72+ 字节堆分配）
- M1-M3 (MEDIUM): `all_components()` 每次构建新 HashMap、rename map 全量克隆、format! 链
- L1-L3 (LOW): `Vec::contains` O(n²)、不必要的 `to_string()`、缓存 ir.clone()

**推荐**: 切换 component_map 为引用。引入 `LiteralIR` 枚举替代 `serde_json::Value`。缓存 `all_components()` 结果。

### 6. Implementation Strategist — CRITICAL: "已完成但未验证"模式延续

**发现**: 工作区处于数月来最佳状态，但 P35 去重留下 16 个重复文件。5 个被接受的测试失败正在建立"失败常态化"。

**关键问题**:
- C1 (CRITICAL): 16 个文件仍在 core 和 facade 之间重复（codegen 基础设施）
- C2 (CRITICAL): 5 个测试失败被接受为常态（2 Rust + 3 TypeScript）
- H1 (HIGH): 零 coverage reporting——重构风险不可量化
- H2 (HIGH): WASM 路径在 CI 中未测试
- H3 (HIGH): omnic CLI 无条件编译全部 9 个后端
- M1-M3 (MEDIUM): P36 门控但编译、omnilang-core 9 个 feature flag 仅 1 个使用、Tauri 零测试

**推荐**: 立即修复 5 个测试失败。添加 coverage reporting 到 CI。完成 P35 codegen 基础设施去重。

### 7. AI Engineer — CRITICAL: 子代理可靠性被测量但未从根本上解决

**发现**: 项目的 AI 编排框架（12+ 代理、三层审查、evals）是所见最成熟的之一，但评估框架未 CI 门控，子代理 33-67% 的失败率被添加更多层来补偿而非修复根因。

**关键问题**:
- C1 (CRITICAL): 子代理 67% 实现失败、33% 审查误判——用蛮力补偿而非根本修复
- H1 (HIGH): eval 框架未连接到 CI pipeline
- H2 (HIGH): P34 AI 补全延迟缺乏可测量的重新评估标准
- M1-M3 (MEDIUM): test-framework SKILL.md 893 行程序性编码、TS 类型手动与生成重复、Monaco tokenizer 未用 WASM 语义
- L1-L2 (LOW): WASM 单例初始化延迟、tokenizer 仅正则

**推荐**: 分类已知 agent 失败模式并编码为 prompt 约束。将 eval 脚本添加到 CI。为 P34 定义具体可测量触发条件。

### 8. Pragmatic Enforcer — 2 CRITICAL（去重完成但未移除原始文件）

**发现**: P35 去重是"加法的"——提取后从未删除原始文件。`crates/omnilang-codegen-omniview/` 是孤儿 crate（零消费者），5 个依赖已过时。

**关键问题**:
- C1 (CRITICAL): ~10,500 行重复 OmniView codegen + 孤儿提取 crate
- C2 (CRITICAL): ~5,000 行 forked codegen shared infrastructure
- H1 (HIGH): 5 个未使用直接依赖（thiserror, strum, pathdiff, walkdir, log）
- H2 (HIGH): `multi-target` feature flag 是文档化的死代码
- H3 (HIGH): 孤儿 crate 每次 CI 编译浪费算力
- M1-M3 (MEDIUM): 重复 dev-dependencies、OmniViewResult 几乎相同、log crate 未使用

**YAGNI 分析**:
| 元素 | 必要性 | 复杂度 | 比率 | 结论 |
|------|--------|--------|------|------|
| orphan omnilang-codegen-omniview | 1/10 | 6/10 | 6.0 | **删除** |
| forked codegen/shared | 2/10 | 7/10 | 3.5 | **重连** |
| multi-target feature flag | 1/10 | 2/10 | 2.0 | **删除** |

**推荐**: 立即删除孤儿 crate、5 个过时依赖、multi-target feature flag。将 facade codegen/shared 重连到核心。

### 9. Rust Expert (Katya Volkov) — CRITICAL: analyse.rs 行为分歧

**发现**: `crates/omnilang-core/src/analyse.rs` 和 `omnilang/src/analyse.rs` 99% 相同但核心版本**缺少优化阶段**（`optimize()` 调用）。通过核心 API 的调用方静默获取未优化的 IR。

**关键问题**:
- C1 (CRITICAL): analyse.rs 优化阶段在生产代码中分歧
- H1 (HIGH): validate.rs 测试模块顺序违规（items_after_test_module）
- H2 (HIGH): clippy 将 `3.14` 标记为 PI 近似值
- H3 (HIGH): OmniView codegen 副本已分化（不同导入路径）
- M1-M4 (MEDIUM): peek() OnceLock 热路径、ViewTree 大枚举变体、expect 可用于错误、可折叠匹配
- L1-L3 (LOW): 未使用导入、缺少 codegen 基准测试、无 rust-toolchain.toml

**推荐**: 将优化阶段统一到核心（或文档化其缺失为 API 契约）。添加 rust-toolchain.toml。修复所有 clippy -D warnings 违规。

### 10. TypeScript & React Specialist (Emma Rodriguez) — 3 HIGH 前端质量问题

**发现**: Poiesis 前端架构合理（Zustand 切片、不可变更新、HMR 状态恢复），但存在严重的类型安全缺口和接近零的测试覆盖。

**关键问题**:
- H1 (HIGH): `@typescript-eslint/no-explicit-any` 完全关闭——15 处 `as any`、47 处 `as unknown`
- H2 (HIGH): 7 个文件中 10 处 React hook 依赖抑制，未注明理由
- H3 (HIGH): 21 个文件中 88 处 `store.getState()` 绕过 React 订阅
- M1-M5 (MEDIUM): 组件测试几乎为零、applyDebugSignalOverrides 原地修改、noUnusedParameters: false、内联样式分散、emoji 缺 ARIA 标签
- L1-L3 (LOW): 重复 Store CRUD 模式、CodeWriter 不处理缩进错误、Monaco 双生命周期

**推荐**: 将 `no-explicit-any` 改为 warn。为核心交互路径添加 RTL 组件测试。将所有 getState() 转为订阅模式。

### 11. Compiler & Language Architect (Dr. Lin Wei) — 2 HIGH 编译器架构问题

**发现**: IR 设计在正确的抽象级别，trait 系统设计良好。但缺少类型检查器——IR 类型是装饰性的而非强制性的。

**关键问题**:
- H1 (HIGH): **无类型检查器**——IR 类型未被验证（`String` 参数可接收 `Int` 表达式）
- H2 (HIGH): Union 类型 lowering 丢失结构信息（`String | Int` → 扁平字符串 `"string | int"`）
- M1-M3 (MEDIUM): `project_uses_motion()` O(N²)、`find_motion` 线性扫描、无模块级作用域
- L1-L3 (LOW): lowering 可变传递语义、lexer 1,721 行、JSON codegen 未用 CodeWriter

**推荐**: 实现最小可行类型检查器（Hindley-Milner + 5 基本类型）。添加 `TypeKind::Union` 变体。为 motion/timeline 添加名称索引。

### 12. Documentation Expert (Samantha Quill) — 1 CRITICAL 文档严重不足

**发现**: ADR 和架构文档基础设施成熟，但面向开发者的 API 文档几乎为零。62 个废弃 Rete.js 文件仍在 docs 中。

**关键问题**:
- C1 (CRITICAL): Rust 公共 API 零文档——`compile()` 仅一行 doc comment
- H1 (HIGH): 无形式语法规范（EBNF 等）
- H2 (HIGH): 62 个废弃 Rete.js 文档文件仍保留
- H3 (HIGH): `theme` 关键字无用户文档（尽管是编译器一级关键字）
- M1-M4 (MEDIUM): stdlib 仅覆盖 3/8 模块、无贡献指南、doc comments 只有一行、无迁移指南
- L1-L3 (LOW): Ai agent 文档对人类不可见、docs 目录长期尾巴问题、双语碎片化

**推荐**: 为核心公共 API 添加模块级和函数级文档。删除 Rete.js 文档目录。生成形式语法规范。编写 CONTRIBUTING.md。

---

## 权衡与分歧

### 权衡 1: Systems Architect vs Security Specialist — FS scope 收紧

**Systems Architect** 认为 Poiesis 的 per-field Mutex 模式存在死锁风险，建议合并为单一 `Mutex<ProjectStateInner>`。

**Security Specialist** 认为 FS scope（`$HOME/**`, `C:/**`, `D:/**`）过于宽泛，建议收紧到 `$APPDATA` 和 `$HOME/Documents/OmniLang/`。

**权衡**: 两者不冲突——可以同时实施。Mutex 合并减少死锁面，FS scope 收紧减少攻击面。但 FS scope 收紧可能影响用户体验（用户可能希望将项目存储在任何位置）。建议：默认收紧 scope，但允许通过 Tauri 文件对话框在 scope 外选择项目（一次性授权模式）。

### 权衡 2: Performance Specialist vs Compiler Architect — `LiteralIR` vs 类型检查器

**Performance Specialist** 建议用 `LiteralIR` 枚举替换 `serde_json::Value` 以减少分配。

**Compiler Architect** 建议优先实现类型检查器（最小可行 Hindley-Milner）。

**权衡**: 两者都是有效的中等优先级改进，但服务于不同目标。`LiteralIR` 减少运行时分配和 clone 成本（影响所有后端性能）。类型检查器增加编译时正确性保证（防止静默错误输出）。**建议**: `LiteralIR` 排在前面（更小范围、更确定的收益），类型检查器紧随其后（更大范围、正确性关键）。

### 权衡 3: Maintainability Expert vs Pragmatic Enforcer — 文件拆分 vs 孤儿 crate 删除

**Maintainability Expert** 建议拆分大文件（`lexer/mod.rs` → 4-5 子模块）。

**Pragmatic Enforcer** 建议优先删除孤儿 crate 和重复代码。

**权衡**: 两者都正确但解决不同问题。孤儿 crate 删除消除 ~16K 行死代码和 CI 浪费——这是一个**正确性/浪费**问题。文件拆分改善**可维护性/可读性**。**建议**: 孤儿 crate 删除优先（CRITICAL——错误风险 + CI 浪费），文件拆分作为短期跟进（HIGH——可维护性）。

### 权衡 4: AI Engineer vs Implementation Strategist — 子代理可靠性

**AI Engineer** 认为 67% 的实现代理失败率需要根本修复（prompt 工程 + 脚本替代）。

**Implementation Strategist** 关注测试失败常态化和 CI 缺口。

**权衡**: 不冲突。AI Engineer 关注开发流程质量，Implementation Strategist 关注代码产品质量。两者都需要解决。**建议**: 测试修复优先（S 工作量，立即修复），代理可靠性改进作为短期跟进（M 工作量）。

---

## 优先级排序

### CRITICAL（0-2 天——阻塞进一步开发）

| # | 问题 | 来源 | 工作量 | 具体行动 |
|---|------|------|--------|---------|
| C1 | **~16K 行 codegen 重复**（孤儿 crate + forked shared） | 8/12 审查 | M (4-6h) | 删除 `crates/omnilang-codegen-omniview/`；删除 `omnilang/src/codegen/shared/` 并 re-export 核心版本；删除孤儿 standalone codegen 文件 |
| C2 | **`analyse.rs` 优化阶段分歧**（核心缺少 `optimize()` 调用） | Rust Expert C1 | S (30min) | 将 Phase 3c optimize 调用添加到 `crates/omnilang-core/src/analyse.rs` |
| C3 | **5 个被接受的测试失败**（2 Rust + 3 TS） | Implementation Strategist C2 | S (1h) | 修复 `"HashMap"`→`"FxHashMap"`；更新 golden/roundtrip 测试期望值 |
| C4 | **facade 重复定义领域类型** | Domain Expert C1, Systems Architect C1 | S (1h) | 删除 facade 中的 `CompileResult`/`SymbolInfo`/`compile()`/`run_pipeline()`，改为 `pub use omnilang_core::*` |
| C5 | **5 个过时依赖** | Pragmatic Enforcer H1 | S (5min) | `cargo remove thiserror strum pathdiff walkdir log -p omnilang` |

### HIGH（2-7 天——当前 sprint 完成）

| # | 问题 | 来源 | 工作量 |
|---|------|------|--------|
| H1 | 关闭 CI 缺口（coverage + WASM test + eval CI + toolchain pin + cargo-deny） | 5/12 审查 | M |
| H2 | 拆分 10 个 >800 行文件（提取内联测试、拆分子模块） | Maintainability Expert | M |
| H3 | 切换 component_map 为引用 / 引入 `LiteralIR` | Performance Specialist | M |
| H4 | 删除 `multi-target` feature flag（文档化死代码） | Pragmatic Enforcer H2 | S |
| H5 | 删除 62 个废弃 Rete.js 文档 | Documentation Expert H2 | S |
| H6 | 为核心公共 API 添加文档（compile, CompileResult, SymbolInfo） | Documentation Expert C1 | S |
| H7 | 将 `no-explicit-any` 改为 warn，修复 `applyDebugSignalOverrides` 变异 | TypeScript Specialist H1, M1 | M |
| H8 | CSP 收紧 / FS scope 收紧 / `(0,eval)` 替换 | Security Specialist H1-H3 | L |
| H9 | 删除 `omnilang-core` 中 8 个未使用的 feature flags | Implementation Strategist M2 | S |

### MEDIUM（1-4 周）

| # | 问题 | 来源 | 工作量 |
|---|------|------|--------|
| M1 | 引入 `TypeKind::Union` 变体（保留结构类型信息） | Compiler Architect H2 | M |
| M2 | ThemeIR 正确定义类型（替代 `Vec<serde_json::Value>`） | Domain Expert H2 | M |
| M3 | Design import 解析从 lowering 移至语义分析 | Domain Expert H1 | M |
| M4 | 为核心交互路径添加 RTL 组件测试 | TypeScript Specialist M1 | L |
| M5 | 定义 P34 具体重新评估触发条件 | AI Engineer H2 | S |
| M6 | 拆分 VariantEditor.tsx (1,601行) | Maintainability Expert M1 | M |
| M7 | 替换 rename-map 全量克隆为 undo stack | Performance Specialist M2 | M |
| M8 | 统一 StoreIR 和 ComponentIR 信号表示 | Domain Expert M1 | M |
| M9 | 创建 CONTRIBUTING.md | Documentation Expert M2 | M |
| M10 | 完成 stdlib.md 缺少的 5 个模块文档 | Documentation Expert M1 | M |

### LOW（1-3 个月）

| # | 问题 | 来源 | 工作量 |
|---|------|------|--------|
| L1 | 实现最小可行类型检查器 | Compiler Architect H1 | L |
| L2 | 为 motion/timeline 名称添加索引 | Compiler Architect M2 | S |
| L3 | 生成形式语法规范 (EBNF) | Documentation Expert H1 | L |
| L4 | 按目标 gate CLI 后端（替代 all-backends） | Implementation Strategist H3 | L |
| L5 | Arena-allocate IR 类型 | Performance Specialist L2 | L |
| L6 | 88 处 getState() → 订阅模式迁移 | TypeScript Specialist H3 | L |

---

## 如果不处理的风险

| 风险 | 可能性 | 影响 | 不处理的后果 |
|------|--------|------|-------------|
| Codegen 基础设施漂移（C1） | 高（每次 codegen 变更） | 严重 | 同一 bug 在两个位置表现不同；修复仅在一个位置应用 |
| analyse.rs 行为分歧（C2） | 中（增量编辑路径） | 严重 | Poiesis 编辑器呈现过时的计算属性值 |
| 测试失败常态化（C3） | 高（已发生） | 高 | 测试套件失去信号价值；更多失败被接受 |
| 管道分歧（C4） | 中（6-12 个月） | 严重 | 同一源文件通过不同 API 产生不同输出 |
| 孤儿 crate 持续消耗 CI 算力（C5） | 100%（每次 CI） | 低 | 每次提交浪费数分钟 CPU 时间 |
| FS scope 过宽 + CSP eval（H8） | 低（需先有 XSS） | 严重 | XSS → 完全桌面控制 |
| 零 API 文档（H6） | 高（已存在） | 中 | 阻碍外部贡献、WASM 消费者、工具构建者 |

---

## 成功指标与跟进计划

### 本轮评审目标（v3 → v4 之间）

- [ ] 零重复 codegen 文件（从 16 → 0）
- [ ] 零孤儿 crate
- [ ] 零测试失败（从 5 → 0）
- [ ] 零过时依赖
- [ ] coverage reporting 在 CI 中运行
- [ ] 核心公共 API 有 doc comments

### 跟进

1. 本次评审保存至 `.architecture/reviews/full-workspace-review-2026-07-27-v3.md`
2. 对需要记录的优先事项使用 `create-adr`
3. 对需要深入研究的具体关注点使用 `specialist-review`
4. 下一次全面评审：在 CRITICAL 项完成后
