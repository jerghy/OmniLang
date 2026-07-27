# Architecture Review: OmniLang 工作区全视角评审

> **评审日期**: 2026-07-26
> **评审范围**: 全工作区（主要项目：Poiesis + OmniLang）
> **评审类型**: 11 位专家并行独立评审 → 交叉分析汇总
> **务实模式**: 已启用（balanced intensity）
>
> **🖊 术语修正 (2026-07-27)**：参见 [ADR-005](../../decisions/adrs/ADR-005-ts-pipeline-parser-emitter-not-compiler.md)。
> Poiesis 的 TypeScript 管线（`services/codegen/`）**不是编译器**——它是编辑器的解析器+源码发射器，
> 负责 `.omni` 源文本 ↔ 编辑器数据模型的往返转换。真正产生编译产物的只有 Rust 端 `omnilang`。
> 本文中"双解析器"是准确表述；凡出现"双编译器"措辞均为笔误，应以"编辑器往返管线"理解。

---

## 执行摘要

OmniLang 工作区是一个**架构上健全但存在系统性维护债务**的双语言（Rust 63K 行 + TypeScript 42K 行）编译器项目。编译管线（Lexer → Parser → Semantic → IR → Codegen）具有教科书级阶段分离，增量缓存设计精良，错误处理达到生产质量。然而，**TypeScript 编辑器端维护了一个 10,778 行的独立解析器/源码发射管线**（非编译器，仅做 .omni ↔ 编辑器模型往返），与 Rust 编译器的解析器形成双实现并存，构成了最大的单一风险和重复劳动来源。

在 11 位专家评审中，**8 个跨领域主题**被 3 位以上专家独立识别，表明它们是系统性而非局部性问题。整体评估：**Adequate（足够好，但需重点关注数个关键债务项）**。

**最重要的 3 个优先事项**：
1. ⚠️ 双解析器（TS + Rust）语义漂移——零自动化同步，每次语言变更需双重实现
2. ⚠️ 9 个代码生成目标仅 1-3 个有实际用户——维护负担与价值严重不成比例
3. ⚠️ Tauri 安全配置：文件系统权限覆盖整个 C:/ 和 D:/ 驱动器，调试窗口无沙箱执行编译代码

**如果上述问题不解决**：解析器分歧导致编辑器与编译器不兼容、安全漏洞 XSS→RCE 升级路径、新增 IR 特性需要在 6+ 后端中手动同步。

---

## 工作区架构概览

```
OmniLang/                          ← Cargo workspace (resolver=2) + pnpm workspace
├── omnilang/      62,240 行 Rust  ← 编译器 crate：Lexer→Parser→Semantic→IR→Codegen (9 targets)
├── omnic/              CLI        ← 二进制 crate，包装 omnilang（all-backends）
├── omnilang-wasm/      WASM      ← WASM 编译目标（仅 codegen-omniview）
├── poiesis/        46,592 行 TS   ← 可视化编辑器：React 19 + Tauri 2 + Monaco + XYFlow
│   ├── src/services/codegen/      ← TypeScript 编辑器往返管线 (10,778 行, 30+ 文件): Parser + SourceEmitter, 非编译器
│   ├── src/store/                 ← Zustand 状态管理 (12 slices)
│   ├── src/components/            ← 画布、面板、设计卡片、Shell
│   └── src-tauri/   873 行 Rust   ← Tauri 后端：IPC 命令 + 编译桥接
└── packages/omni-animation/       ← 独立 npm 动画库
```

### 数据流

```
┌────────────────── 编辑器往返层（非编译器）──────────────────┐
│                                                            │
│  编辑器 UI ─→ TS Parser/Emitter ─→ .omni 源文本            │
│    ▲              │                   │                    │
│    └── 解析读取 ──┘   └── 写入发射 ───┘                    │
│   (17 TokenKind,      (SourceEmitter,                      │
│    ViewNode 树)        DesignEmitter)                      │
└────────────────────────────────────────────────────────────┘
                           │
                           ▼ .omni 源文本
┌────────────────── 编译器层（唯一编译器）────────────────────┐
│                                                            │
│  WASM 编译器 ─→ omnilang 管线 ─→ 预览输出 (OmniView)      │
│  (1.2MB)         Lexer→Parser→Semantic→IR→Codegen          │
│                                                            │
│  Tauri 后端 ─→ omnilang::compile() ─→ 9 目标输出           │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

> **关键区分**：TS 管线是**编辑器往返翻译层**，不产生编译产物。仅 Rust `omnilang` 是编译器。

### 关键指标

| 指标 | 值 |
|------|-----|
| Rust 总行数 | 63,342 |
| TypeScript/React 总行数 | 42,463 |
| Rust 代码生成目标 | 9 个（feature-gated） |
| TypeScript 测试覆盖率 | ~0.4%（~170 测试/42K 行） |
| Rust 生产代码 unwrap() | 165 处 |
| unsafe 代码块 | 9 处（全部在 FFI，均有 SAFETY 注释） |
| ADR 记录 | 0 篇 |
| 现有架构评审 | 1 篇（initial-system-analysis） |

---

## 跨领域主题分析

以下 8 个主题被 3 位以上专家独立识别，代表系统性的而非局部性的架构关注点：

### 主题 1：双解析器/TypeScript 代码生成重复 ⚠️ 最关键

**被 7/11 专家识别**：Systems Architect (C1-CRITICAL)、Domain Expert (C-1-CRITICAL)、Maintainability Expert (HIGH)、Implementation Strategist (HIGH)、Pragmatic Enforcer (#2)、TypeScript/React Specialist (store耦合)、Compiler Architect (HIGH)

**问题本质**：Poiesis 编辑器的 `services/codegen/` 目录（30+ 文件、10,778 行）包含一个完整的 `.omni` 语言解析器和源码发射器，与 Rust 编译器（`omnilang/src/lexer/` + `parser/`，约 4,700 行）独立维护。每个语法变更必须在两个代码库中实现。零自动化同步验证。TypeScript 解析器仅支持 17 种 TokenKind（Rust 有 58 种）。TypeScript 管道的视图树构建（ViewNode）在传给 Rust 编译器前被序列化为文本再重新解析——丢弃已计算的丰富结构。

**建议**（按时间线）：
- **立即**：添加 Golden 文件解析器一致性测试套件——20-30 个 .omni 文件同时被两个解析器解析，验证结构等价性
- **短期**：文档化 TS 解析器的语言覆盖矩阵，将 `expression-lowering.ts` 逻辑移入 WASM
- **长期**：评估用 WASM 包装的 Rust 解析器替换 TS 解析器的可行性

### 主题 2：9 个代码生成目标过度扩展

**被 5/11 专家识别**：Pragmatic Enforcer (Ratio 2.67)、Domain Expert (M-1)、Maintainability Expert (HIGH 测试重复)、Performance Specialist (H1)、Compiler Architect (OmniView 特殊化)

**问题本质**：9 个后端中仅 OmniView（+ JavaScript/Python）有编辑器中实际用例。Dioxus、Vue、Dart、Flutter 的代码生成器合计约 8,400 行，通过 `all-backends` 元特性编译但从未被主动调用。每个 IR 变更需要接触 6+ 代码生成后端。9 个几乎完全相同的回归测试文件（每个 58 行）增加了维护噪音。

**建议**：
- **短期**：从 CI 和默认开发工作流中移除 `all-backends` 特性；保留各目标在 feature flag 后可编译但按需启用
- **中期**：参数化回归测试套件（共享 runner + rstest），减少 9 个文件为 1 个

### 主题 3：测试覆盖不足与测试缺口

**被 6/11 专家识别**：Systems Architect (C3/C6)、Maintainability Expert、Implementation Strategist (CRITICAL-OmniView motion)、Pragmatic Enforcer (#3)、Compiler Architect (MEDIUM 跨目标)、Performance Specialist (缓存未接入)

**关键缺口**：
1. **OmniView motion/timeline/@for 代码生成** (~2,200 行) 零回归测试——最复杂的子系统
2. **TypeScript 管道** 仅 ~170 测试覆盖 42,463 行（约 0.4%）
3. **增量编译缓存** 未接入 WASM 编辑器路径——每次按键重新解析所有文件
4. **无跨目标正确性测试**——同一 .omni 源在所有 9 个后端上的一致性未验证

### 主题 4：Tauri 安全与平台问题

**被 4/11 专家识别**：Security Specialist (C1-CRITICAL 到 C4-HIGH)、Rust Expert (H1)、Systems Architect (C4)、AI Engineer (C5)

**关键发现**：
- **文件系统范围** `C:/**` 和 `D:/**` → 任何 XSS 即可获得完整文件系统读写（Security C1-CRITICAL）
- **CSP 使用 `'unsafe-inline'`** → 禁用主要 XSS 防御（Security C2-HIGH）
- **调试窗口无沙箱** → 编译代码在 Tauri webview 进程中通过 `innerHTML` + `eval()` 执行（Security C3-HIGH）
- **`open_project` 规范化失败静默回退** → TOCTOU 风险（Security C4-HIGH）
- **`dw_log` 硬编码 POSIX 路径** `/tmp/poiesis-debug.log`，在 Windows 上不可用（Rust H1）
- **无 `cargo audit` 或依赖 CVE 扫描**（Security C8-MEDIUM）

### 主题 5：架构框架未充分利用

**被 4/11 专家识别**：AI Engineer (CRITICAL-零 ADR)、Implementation Strategist (MEDIUM)、Domain Expert、Systems Architect

**问题**：`.architecture/` 框架已完全配置（260 行 config.yml、320 行 principles.md、10 个成员角色），但 ADR 目录为空，8 个 agent 文件为 0 字节，模板仍含 `[PROJECT_NAME]` 占位符。关键决策（Block Editor v4→v5、9 目标架构、Deferred JS 交换机制、indigo→terracotta 迁移）未以 ADR 形式记录。

### 主题 6：代码质量与类型安全

**被 4/11 专家识别**：TypeScript/React Specialist (HIGH 4 处 as any)、Rust Expert (M2/M4)、Maintainability Expert (HIGH 165 unwrap)、Compiler Architect (MEDIUM)

- TypeScript：4 处 `as any` 绕过严格类型检查、服务直接导入 store、空 catch 块
- Rust：165 个生产代码 `unwrap()` 调用（主要在 `ir/lower/`）、诊断转换 4 处重复
- IR 层：`TypeKind::Named { name: String }` 将类型存储为不透明字符串，不做结构验证

### 主题 7：OmniView 代码生成特殊化

**被 3+ 专家识别**：Compiler Architect (HIGH)、Domain Expert (M-2)、Implementation Strategist (HIGH-UiCodegen 未填充)

- OmniView 绕过 `Codegen` trait，返回独立的 `OmniViewOutput`（11 个额外字段）
- `OmniViewGenerator` 结构体 50+ 字段——待分解
- `UiCodegen` trait 已提取但代码未移入共享方法（注释说"跟进重构"）
- 源映射管线已断开：`IRProgram.source_map` 永远为空

### 主题 8：性能优化机会

**被 3+ 专家识别**：Performance Specialist (C1/H1/H2/H3)、Rust Expert (H2-IR 克隆)、Pragmatic Enforcer

- Monaco Editor 缺少代码分割 → 16MB 压缩 JS 混入 vendor 块
- WASM 1.2MB——增量缓存未接入编辑器路径
- IR 缓存合并阶段深拷贝所有未变更文件（O(n) 内存压力）
- 代码生成单线程（rayon 仅在解析/语义分析阶段使用）

---

## 专家分歧与权衡

| 分歧点 | 甲方观点 | 乙方观点 | 建议 |
|--------|---------|---------|------|
| **9 个代码生成目标** | Pragmatic Enforcer: 推迟 6 个 (Ratio 2.67) | Domain Expert: 保留结构，提升编辑器可见性 | **推迟非活跃目标**——保留 OmniView + JS + Python + Rust 活跃，其余保持 feature-gated 但从 CI 移除 |
| **TypeScript 解析器** | Compiler Architect: WASM 替换 | Implementation Strategist: 先加同步测试再迁移 | **渐进式**——先加同步 CI 测试，再逐步迁移到 WASM |
| **HMR 状态持久化** | TS/React Specialist: 仅存序列化数据 | Performance Specialist: 限制快照键 | **采用两者**——只存数据（不要函数），排除撤销堆栈 |
| **测试优先序** | Implementation Strategist: OmniView motion 回归测试 | Pragmatic Enforcer: TS 管线覆盖率 | **并行推进**——两者都是关键缺口 |

---

## 优先级建议

### 🔴 关键（0-2 周）

| # | 行动 | 工作量 | 相关文件 |
|---|------|--------|---------|
| P1 | 缩小 FS 权限范围到 `$APPDATA/...` + 项目目录 | S (5 min) | `poiesis/src-tauri/capabilities/default.json:11-16` |
| P2 | 为调试窗口创建受限 Tauri capability（仅 `core:default`） | S (30 min) | `poiesis/src-tauri/capabilities/` |
| P3 | 修复 `open_project` 规范化失败时返回 Err 而非静默回退 | S (5 min) | `poiesis/src-tauri/src/lib.rs:502-506` |
| P4 | 修复源映射管线——填充 `IRProgram.source_map` | M (2-3 天) | `omnilang/src/ir/lower/mod.rs:198` |
| P5 | 添加 Golden 文件双解析器同步 CI 测试 | M (1-2 天) | `poiesis/src/services/codegen/` + CI |
| P6 | 生成基于 nonce 的 CSP 替换 `'unsafe-inline'` | M (1-2 天) | `poiesis/src-tauri/tauri.conf.json:22` |

### 🟠 高优先级立即行动（本周）

| # | 行动 | 工作量 | 相关文件 |
|---|------|--------|---------|
| P7 | 为 Monaco 添加代码分割 + 降低 chunkSizeWarningLimit | S (10 min) | `poiesis/vite.config.ts:24-33` |
| P8 | 将增量编译缓存接入 WASM 编辑器路径 | M (2-3 天) | `omnilang/src/analyse.rs` + WASM + `useWasmCompile.ts` |
| P9 | 添加 OmniView motion/timeline/@for 回归测试 | M (2-3 天) | `omnilang/src/codegen/omniview/tests_regression/` |
| P10 | 修复 `dw_log` 路径——使用 `std::env::temp_dir()` | S (5 min) | `poiesis/src-tauri/src/lib.rs:770` |
| P11 | 提取诊断转换 helper 消除 4 处重复 | S (30 min) | `poiesis/src-tauri/src/lib.rs` + `omnilang/src/ffi.rs` |
| P12 | 修复 4 处 TypeScript `as any` | S (30 min) | `store/index.ts:94`、`App.tsx:378`、`slices/theme.ts:24`、`store/index.ts:80` |
| P13 | 替换空 catch 块为至少记录错误日志 | S (5 min) | `DesignPage.tsx:118,129` |
| P14 | 为 IPC 命令添加输入大小限制 | S (15 min) | `poiesis/src-tauri/src/lib.rs:268,611` |

### 🟡 短期（2-8 周）

| # | 行动 | 工作量 | 相关文件 |
|---|------|--------|---------|
| P15 | 从 CI/默认开发流中移除 `all-backends`；推迟 6 个非活跃目标 | S (30 min) | `omnilang/Cargo.toml:40-50`、CI 配置 |
| P16 | 创建 5-7 篇回溯性 ADR（编译器管线、编辑器演进、设计迁移等） | S (2-3 小时) | `.architecture/decisions/adrs/` |
| P17 | 添加 TypeScript 管线 round-trip 测试（编辑数据→.omni→解析→比较） | M (1-2 天) | `poiesis/src/services/codegen/__tests__/` |
| P18 | 拆分 Tauri lib.rs 单体（873 行→state.rs + commands/ + path.rs） | S (1-2 小时) | `poiesis/src-tauri/src/` |
| P19 | 参数化后端回归测试（共享 runner + rstest） | M (2-3 天) | `omnilang/src/codegen/tests_regression/` |
| P20 | 添加 `cargo audit` + `npm audit` 到 CI | S (30 min) | `.github/workflows/` |
| P21 | 删除 8 个空的 `.claude/agents/*` 文件 | S (1 min) | `.claude/agents/` |
| P22 | 清理 `.superpowers/brainstorm/` 过期状态文件 + 更新 `.gitignore` | S (10 min) | `.superpowers/brainstorm/` |
| P23 | 修复 HMR 状态序列化——仅存数据、排除函数和撤销堆栈 | M (2-3 小时) | `poiesis/src/store/index.ts:77-88` |
| P24 | 将 `TypeKind::Named` 替换为递归类型表示 | M (3-5 天) | `omnilang/src/ir/types.rs:271-336` |
| P25 | 添加 `#[non_exhaustive]` 到 `ExprIR` 等常用枚举 | S (30 min) | `omnilang/src/ir/types.rs` |
| P26 | 修复 `.mcp.json` 使用 `victauri bridge` 而非硬编码 URL | S (5 min) | `.mcp.json:13` |
| P27 | 添加 WASM→Tauri 编译回退路径 | S (1-2 小时) | `poiesis/src/commands/compile-api.ts` |
| P28 | 替换 165 处生产代码 `unwrap()` 为诊断产生式替代 | M (2-3 天) | `omnilang/src/ir/lower/` 等 |
| P29 | 为 TypeScript 管线添加 10% 最低测试覆盖率 | M (2-3 天) | `poiesis/src/services/codegen/__tests__/` |

### 🟢 长期（2-6 个月）

| # | 行动 | 工作量 |
|---|------|--------|
| P30 | 评估用 WASM 包装 Rust 解析器替换 TS 解析器 | L (2-4 周) |
| P31 | 停止 OmniView 特殊化——统一 `CompileOutput` | M (3-5 天) |
| P32 | 跨目标正确性测试框架 | L (2-3 周) |
| P33 | 为所有 IR 节点添加 `source_span` | M (3-5 天) |
| P34 | 在 Poiesis 中添加 AI 代码补全功能（`useAiCompletion` hook） | M (2-3 天) |
| P35 | 将 `omnilang` 拆分为 `omnilang-core` + 每个后端的独立 crate | L (2-4 周) |
| P36 | 代码生成并行化（rayon 后端分发） | M (3-5 天) |
| P37 | Zustand store 领域子存储拆分 | L (3-5 天) |
| P38 | 添加编译后输出运行时 iframe 沙箱用于调试预览 | M (2-3 天) |
| P39 | 实现 IR 优化传递（移除未使用信号、内联简单 computed） | L (1-2 周) |

---

## 若不解决的风险

| 风险 | 可能性 | 影响 | 相关主题 |
|------|--------|------|---------|
| **语法漂移**——TS 与 Rust 解析器分歧，编辑器产生编译器无法解析的 `.omni` 文件 | 高（每次语言变更） | **关键**——编辑器基本功能损坏 | 主题 1 |
| **安全升级路径**——XSS→RCE 通过过宽 FS 权限 + 弱 CSP + 无沙箱调试 | 中 | **关键**——完整文件系统接管 | 主题 4 |
| **OmniView 退化**——motion/animation 代码生成在无测试下静默损坏 | 中 | **高**——编辑器预览显示错误 | 主题 3 |
| **维护惯性**——IR 变更需接触 6+ 未使用后端，拖慢编译器演进 | 中（持续累积） | **高**——新特性开发速度降低 | 主题 2 |
| **测试债务**——TypeScript 0.4% 覆盖率下任何重构都是"信任我"提交 | 中 | **中**——重构恐惧症、bug 被用户发现 | 主题 3 |
| **知识丢失**——无 ADR 记录的架构决策在团队变化时被遗忘 | 中 | **中**——重复失败的设计决策 | 主题 5 |
| **WASM 性能退化**——增量缓存未接入，1.2MB 二进制随特性增长 | 中 | **中**——编辑器延迟增加 | 主题 8 |

---

## 各专家评审原文

以下为 11 位专家的完整独立评审报告。汇总部分基于这些原文进行交叉分析和优先级排序，但不替代原文——原文是最权威的原始数据。

### 1. 系统架构师（Systems Architect）

**领域**：分布式系统、服务架构、可扩展性模式、系统分解

**评估**：双编译拓扑结构（WASM + Tauri）合理但未充分利用。TypeScript 代码生成层（8,070 行）是完整的语法重新实现。Cargo workspace 边界清晰，但 pnpm workspace 未正式集成 `omni-animation` 包。Tauri IPC 边界缺少类型合约（ts-rs 生成未消费）。

**关键发现**：CRITICAL—重复语法定义；HIGH—ts-rs 类型绑定未接入；MEDIUM—缺少 WASM→Tauri 回退路径。

### 2. 领域专家（Domain Expert）

**领域**：DDD、限界上下文、通用语言、业务逻辑建模

**评估**：OmniLang 和 Poiesis 是两个核心领域，但文档呈现为一个。Rust IR 和 TypeScript 视图模型之间无单一真相来源。九种代码生成目标作为不同的子领域结构良好，但仅 1 个在编辑器中可见。

**关键发现**：CRITICAL—双解析器无正式同步；HIGH—TS 管线视图树在 Rust 代码生成中未被使用；MEDIUM—样式解析两个路径。

### 3. 安全专家（Security Specialist）

**领域**：威胁建模、安全模式、数据保护、漏洞识别

**评估**：Tauri 2 基于能力的权限模型已应用但范围过宽（C:/**、D:/**）。编译器管道使用两个执行上下文具有不同安全属性。路径遍历保护存在但有静默回退薄弱点。调试窗口是沙箱外编译代码执行环境。

**关键发现**：HIGH—FS 范围过宽、CSP 'unsafe-inline'、调试窗口沙箱外执行、open_project 规范化失败静默回退；MEDIUM—无限 IPC 输入大小、TOCTOU 导出竞争、缺少 cargo audit。

### 4. 可维护性专家（Maintainability Expert）

**领域**：代码质量、重构、技术债务、复杂度分析

**评估**：编译管线阶段分离干净。双解析器架构是最大的单一可维护性风险。165 个生产 unwrap() 调用代表潜在 panic 风险。9 个几乎完全相同的后端测试套件。重构势头良好（element_to_html 839→110 行等）。

**关键发现**：HIGH—双解析器 10,778 行、165 个 unwrap、9 个重复测试套件；MEDIUM—Tauri 后端单体（873 行）、大文件（parser.ts 1,454 行）、诊断严重性映射不一致。

### 5. 性能专家（Performance Specialist）

**领域**：性能优化、资源利用、瓶颈识别

**评估**：1.2MB WASM 二进制在优化配置下可接受。Monaco Editor 缺少代码分割。增量编译缓存未接入 WASM 编辑器路径。代码生成单线程。Zustand HMR 快照整个 store。

**关键发现**：CRITICAL—Monaco 缺少代码分割 + chunkSizeWarningLimit 500；HIGH—WASM 编译所有后端、缺少增量缓存、代码生成单线程；MEDIUM—Zustand 选择器无 useShallow、IR 降级大量 clone。

### 6. 实施策略专家（Implementation Strategist）

**领域**：变更排序、爆炸半径分析、可逆性设计、迁移规划

**评估**：编辑器在 2 周内经历了 3 次完整范式转换（CodeCard→Blockly→SVG→Monaco）。重构提交粒度完美（每次提取一个方法）。OmniView motion/timeline/@for 代码生成零回归测试。IR 类型级联问题影响 6+ 后端。

**关键发现**：CRITICAL—OmniView 无回归测试；HIGH—TypeScript 解析器分歧、UiCodegen trait 未填充代码；MEDIUM—零 ADR、IR 类型级联。

### 7. AI 工程师（AI Engineer）

**领域**：AI/ML 集成、Agent 系统、AI 评估框架

**评估**：AI 辅助开发基础设施成熟（记忆系统、技能、钩子、架构师框架），但产品层面零 AI 功能。记忆系统 20 文件组织精良但缺少自动化检查。架构框架深度配置但从未使用（0 篇 ADR）。8 个 agent 文件为空（脚手架失败）。

**关键发现**：CRITICAL—零 ADR（目录为空）；HIGH—8 个空 agent 文件、Poiesis 零 AI 功能；MEDIUM—过期 brainstorm 状态文件、96 条目权限列表。

### 8. 务实执行者（Pragmatic Enforcer / YAGNI Guardian）

**领域**：YAGNI 原则、复杂度分析、最小可行方案

**评估**：对所有主要架构关注点进行了 10 项必要性与复杂度分析。9 个代码生成目标比率 2.67（复杂度远超必要性）——建议推迟 6 个。WASM/Tauri 双路径比率 0.33（设计良好）。XYFlow 比率 0.22（非常合适）。

**关键建议**：推迟 Dioxus/Vue/Dart/Flutter/React 目标、简化 TypeScript 解析器、TypeScript 端达到最低测试覆盖率。

### 9. Rust 专家（Katya Volkov）

**领域**：内存安全、零成本抽象、编译器设计、WASM 编译

**评估**：编译管线架构成熟。零生产 unsafe（FFI 除外），均有 SAFETY 注释。错误处理达到生产质量（Diagnostic/DiagnosticKind/Severity + thiserror）。Rayon 并行保守且正确。Codegen trait + FormatExpr 模式正确但采纳不完整。

**关键发现**：HIGH—dw_log 硬编码 POSIX 路径在 Windows 上不可用、IR 缓存合并深拷贝、诊断转换 4 处重复；MEDIUM—Symbol line:0 硬编码、非字符串 API 值静默转换为空字符串。

### 10. TypeScript & React 专家（Emma Rodriguez）

**领域**：React 组件架构、TypeScript 类型系统、前端状态管理

**评估**：代码库在管道架构方面展现成熟工程纪律，但存在务实捷径——as any 类型转换、store 作为服务后门、空 catch 块——侵蚀了严格 tsconfig 旨在提供的类型安全。Zustand 切片组合结构良好，但整体的 PoiesisState 接口是单点耦合。

**关键发现**：HIGH—4 处 as any 绕过类型安全、服务直接导入 store、空 catch 块；MEDIUM—缺失 ARIA 属性、HMR 存储函数对象在 window 上、tab ID 不匹配。

### 11. 编译器 & 语言架构师（Dr. Lin Wei）

**领域**：编程语言设计、多目标代码生成、IR 设计、解析器架构

**评估**：管线阶段分离一致但 OmniView 路径分支。IR 层结构化但类型系统基本（TypeKind 为不透明字符串）。错误恢复设计得当（emit_error/skip_to/at_stmt_restart）。源映射基础设施存在但管线断开。

**关键发现**：CRITICAL—源映射管线断开（IRProgram.source_map 永远为空）；HIGH—两个独立解析器实现、OmniView 绕过 Codegen trait；MEDIUM—TypeKind 基于字符串无结构验证、OmniViewGenerator 50+ 字段、解析器仅报告第一个错误。

---

## 跟进计划

1. **评审日**：团队成员审阅本报告，标注不同意的发现或调整优先级
2. **ADR 冲刺**：为回溯性决策创建 5-7 篇 ADR，验证模板
3. **关键修复冲刺**：P1-P6（0-2 周）——安全 + 解析器同步 + 源映射
4. **高优修复冲刺**：P7-P14（本周）——性能 + 测试 + 代码质量
5. **债务偿还跟踪**：在 `.architecture/deferrals.md` 中记录推迟的目标和决策
6. **30 天复查**：重新评分关键发现，追踪进展

---

## 参与专家

| 专家 | 角色 | 关键贡献 |
|------|------|---------|
| Systems Architect | 系统架构师 | 双编译拓扑分析、IPC 边界评估 |
| Domain Expert | 领域专家 | 双解析器领域模型、通用语言评估 |
| Security Specialist | 安全专家 | Tauri FS 权限、CSP、沙箱审计 |
| Maintainability Expert | 可维护性专家 | 165 unwrap 计数、测试重复度分析 |
| Performance Specialist | 性能专家 | Monaco 代码分割、增量缓存缺口 |
| Implementation Strategist | 实施策略专家 | 编辑器范式转换历史、IR 级联问题 |
| AI Engineer | AI 工程师 | Agent 基础设施、记忆系统、AI 功能缺口 |
| Pragmatic Enforcer | 务实执行者 | 10 项复杂度/必要性比率分析 |
| Katya Volkov | Rust 专家 | FFI 安全、错误处理、IR 克隆分析 |
| Emma Rodriguez | TS/React 专家 | as any 类型安全、store 耦合、ARIA 审计 |
| Dr. Lin Wei | 编译器架构师 | 源映射管线、IR 类型系统、OmniView 特殊化 |

---

> 🤖 本评审由 AI Software Architect 框架通过 11 位专家并行评审 + 交叉分析生成。
> 评审范围：全工作区。务实模式：已启用（balanced intensity）。
