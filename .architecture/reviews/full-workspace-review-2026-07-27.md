# Architecture Re-Review: OmniLang Workspace (Follow-Up)

> **评审日期**: 2026-07-27
> **评审范围**: 全工作区（OmniLang + Poiesis）
> **评审类型**: 11 位专家并行独立复审 → 交叉分析汇总
> **基准**: [full-workspace-review-2026-07-26.md](./full-workspace-review-2026-07-26.md)
> **变更量**: 59 commits，覆盖 30+/39 个行动项
> **务实模式**: 已启用（balanced intensity）
>
> **📋 架构决策引用**：
> - **FS 权限全覆盖**是编辑器设计意图，非安全疏忽。纵深防御重点在 XSS/CSP/调试窗口隔离。详见 [[ADR-004]] 决策 2 与 [[poiesis-tauri-security-model]]。
> - **推迟项目**（P35/P36/P38b/P39）本次复审升级为**立即执行**——不再推迟。P34 继续搁置。

---

## 执行摘要

自 2026-07-26 架构评审以来的 59 次提交中，OmniLang 工作区在**安全隔离、测试基础设施、编译管线完整性**方面取得了实质性进展。30+ 个行动项得到处理：调试窗口实现沙箱隔离（P2/P38），source_map 管线恢复连接（P4），Tauri 后端从 873 行单体拆分为 10 个聚焦模块（P18），5 篇 ADR 记录核心架构决策（P16），测试基础设施从近乎为零扩展到 ~5,500 行涵盖 golden 文件、往返测试、运动回归测试和跨目标一致性测试。

然而，本次复审揭示了**"部分完成"的系统性模式**：数据结构被定义但未填充（`source_span` 在 32/33 个 IR 结构体上为 `None`），trait 被实现但测试绕过适配器路径，序列化格式丢弃递归类型参数。此外，**4 个推迟项目现在必须完成**（P34 AI 代码补全继续搁置），不能再以 YAGNI 为理由推迟。

**整体评估：Adequate（稳固进步，但部分完成 + 推迟项目需立即处理）**。

**最重要的 4 个优先事项**：
1. 🔴 **`source_span` 在 32/33 个 IR 结构体上为 `None`**——P33 数据结构已定义但从未填充，产生虚假信心
2. 🔴 **TypeKind 序列化丢失 `type_args`**——P24 的递归类型参数在 JSON/WASM 边界静默丢弃
3. 🔴 **完成 P35 Crate 拆分**——omnilang 单一 crate 63K 行，需拆分为 core + 各后端独立 crate
4. 🔴 **完成 P36 代码生成并行化**——codegen 单线程，rayon 仅用于解析/语义分析阶段
5. 🔴 **完成 P39 IR 优化传递**——移除未使用信号、内联简单 computed，需 motion 95% 分支覆盖率

---

## 上次评审行动项完成状态

### ✅ 已完成（23 项）

| 行动项 | 描述 | 验证 |
|--------|------|------|
| P2 | 调试窗口隔离——独立 capability（仅 core:default） | `capabilities/debug-window.json` |
| P3 | `open_project` 规范化失败返回 Err | `project.rs:166-168` |
| P4 | source_map 管线——`IRProgram.source_map` 已填充 | `ir/lower/mod.rs` 5 处插入点 |
| P5 | Golden 文件 CI——51 个 .omni 夹具 | `tests/fixtures/golden/` |
| P7 | Monaco 代码分割——5 个 manualChunks + 200KB 限制 | `vite.config.ts:22-33` |
| P9 | OmniView motion/timeline/@for 回归测试 ~1,749 行 | `tests_regression/` |
| P10 | `dw_log` 使用 `std::env::temp_dir()` | `commands/debug.rs:43` |
| P14 | IPC 输入大小限制——10MB 在 compile + analyse | `commands/compile.rs` + `analyse.rs` |
| P15 | 从默认 CI 移除 `all-backends`；default = 3 个 P0 目标 | `Cargo.toml` + CI |
| P16 | 5 篇 ADR + 回溯文档 | `.architecture/decisions/adrs/` |
| P17 | TS 管线往返测试 ~4,048 行 | `__tests__/` |
| P18 | Tauri lib.rs 拆分——873→111 行，6 个命令模块 | `commands/` |
| P19 | 参数化后端回归测试——9 个文件→1 个宏 | `parameterized.rs` |
| P20 | cargo audit + npm audit 加入 CI | `.github/workflows/ci.yml` |
| P21 | 空 agent 文件已删除 | `.claude/agents/` |
| P22 | Brainstorm 状态文件已清理 + .gitignore | `.superpowers/brainstorm/` |
| P24 | TypeKind::Named 递归 type_args | `ir/types.rs:289-292` |
| P30a | WASM 解析器导出（parse_source） | `omnilang-wasm/src/lib.rs` |
| P30b | WASM 解析器替换评估——决策：保留双解析器 | `parser-unification-evaluation-2026-07-27.md` |
| P31 | OmniView 实现 Codegen trait | `codegen/omniview/mod.rs:146` |
| P32 | 跨目标正确性测试框架 | `tests/cross_target.rs` |
| P33 | source_span 字段添加到 33 个 IR 结构体 | 多个 `ir/` 文件 |
| P38 | 沙箱化 iframe + postMessage 桥接 | `debug-window/index.html` |

### ⚠️ 部分完成 / 存在回归（5 项）

| 行动项 | 描述 | 状态 |
|--------|------|------|
| P6 | CSP 加固——基于 nonce 的生产 CSP | ⚠️ **部分修复**——产品 CSP 中脚本的 'unsafe-inline' 已移除，但仍保留 'unsafe-eval'，无基于 nonce 的方案 |
| P12 | TypeScript `as any` 修复 | ❌ **回退**——4→14（12 处源码），主要集中在新代码中 |
| P13 | 替换空 catch 块 | ⚠️ **部分修复**——DesignPage.tsx 中仍有 2 处未修复 |
| P28 | 替换生产代码 unwrap() | ⚠️ **叙事具有误导性**——165 中的绝大多数在测试代码中。生产代码路径已有效清除。但 164 个测试 unwrap 仍在。 |
| P33 | source_span 传播 | ⚠️ **仅结构定义**——字段存在但在 32/33 个结构体上从未填充。仅 ElementIR 获得实际值。 |

### 🔴 推迟项目升级为立即执行（4 项，原记录在 deferrals.md 中。P34 AI 代码补全继续搁置）

| 行动项 | 描述 | 原推迟原因 | 当前状态 |
|--------|------|-----------|---------|
| P35 | Crate 拆分（omnilang-core + 各后端独立 crate） | "YAGNI，63K 行无痛点" | 编译器已达规模门槛——**立即执行** |
| P36 | 代码生成并行化（rayon 后端分发） | "无 profiling 数据" | 单线程 codegen 在项目增长时成为瓶颈——**立即执行** |
| P38b | Strip TAURI_INTERNALS | "防御性过早" | P38 iframe 沙箱已生效——**立即执行** |
| P39 | IR 优化传递（移除未使用信号、内联简单 computed） | "缺 motion 95% 覆盖率" | motion 测试已有 13 个——**补齐覆盖率后立即执行** |

### 📋 设计决策确认（无需修复）

| 原行动项 | 描述 | ADR |
|---------|------|-----|
| P1 | FS 权限范围 | ADR-004 决策 2：FS 全覆盖为编辑器必要条件。纵深防御转向 XSS/CSP/调试窗口隔离。 |

---

## 跨领域主题分析

以下 8 个主题被 3 位以上专家独立识别：

### 主题 1：source_span 已定义但未填充 ⚠️ 最普遍

**被 8/11 专家识别**：Compiler Architect (H1-CRITICAL)、Maintainability Expert (HIGH-1)、Rust Expert (Critical #2)、Systems Architect、Domain Expert、Implementation Strategist、Pragmatic Enforcer (Concern B)、Performance Specialist

**问题**：P33 向 33 个 IR 结构体添加了 `source_span: Option<Span>`，但所有降级函数中仅有 **1 个**（`ElementIR`，位于 `view.rs:599`）实际填充了它。ComponentIR、StoreIR、ModuleIR、IRProgram、FunctionIR 以及所有 motion/timeline/design 结构体的 32 个生产构造站点全部设为 `None`——即使 AST span 随手可用。这造成了维护陷阱：未来开发者会认为 `source_span` 携带位置数据，但实际上并不携带。更丰富的 `IRProgram.source_map` 机制（P4 已修复）提供了逐节点源映射，使结构体级别的 `source_span` 字段变得冗余。

**共识建议**：
- **立即**：填充 4 个核心结构体（ComponentIR、StoreIR、ModuleIR、IRProgram）
- **短期**：确定 `source_span` 和 `source_map` 之间的职责划分——它们目前重叠但服务于不同目的
- **长期**：若有实际消费者需要结构体级别的 span，再为剩余的面向表达式的结构体填充

### 主题 2：TypeKind 序列化在 JSON/WASM 边界丢失 type_args

**被 6/11 专家识别**：Rust Expert (Critical #1)、Compiler Architect (M1)、Maintainability Expert (MEDIUM-2)、Domain Expert (MEDIUM)、Systems Architect、Implementation Strategist

**问题**：P24 使 `TypeKind::Named` 支持递归（`type_args: Vec<TypeKind>`），且降级正确填充了类型参数（如 `Array<String>` 变为 `Named { name: "Array", type_args: [Named { name: "String" }] }`）。但自定义的 `Serialize`/`Deserialize`（`types.rs:336-348`）将变体扁平化为一个纯字符串（`"Array"`），在序列化时丢弃了所有 `type_args`。任何跨 JSON/WASM 边界的往返操作都会失去泛型类型信息——使得 P24 在编辑器路径上对消费者毫无改进。

**共识修复**：扩展自定义序列化以发出结构化 JSON（如 `{"name":"Array","args":["String"]}`），同时保持对旧版扁平字符串格式的反向兼容。

### 主题 3：`as any` 从 4 处回退到 14 处

**被 6/11 专家识别**：TS/React Specialist (H1-HIGH)、Maintainability Expert (MEDIUM-1)、Implementation Strategist (C1)、Systems Architect (H2)、Pragmatic Enforcer (Concern E)、Domain Expert (LOW)

**问题**：原始 4 处 `as any`（位于 store/index.ts、App.tsx、slices/theme.ts）已通过 `WindowWithPoiesis` 类型化接口修复。但在开发过程中出现了 12 处新实例，集中在三个反模式中：
1. **DOM 属性 hack**（PropsSection.tsx 有 5 处）——在 DOM 节点上附加 `__labelSlashInsert` 回调
2. **WASM bindgen 解包**（rustCompiler.ts 有 1 处，useWasmCompile.ts 有 1 处）
3. **私有属性附加**（graph-parser.ts 有 2 处 `_operations`，graph-compiler.ts 有 1 处）

根源：`.eslintrc` 中缺少 `@typescript-eslint/no-explicit-any` lint 规则。无自动化门控。

### 主题 4：P31/P33 的"部分完成"模式

**被 5/11 专家识别**：Compiler Architect、Maintainability Expert、Systems Architect、Domain Expert、Pragmatic Enforcer

**模式**：P31 添加了 `OmniViewCodegen: Codegen` trait 实现，但内部 `generate()` 仍返回特殊的 `OmniViewOutput`（11 个额外字段），回归测试绕过 trait 路径使用一个 `omni_to_compile_output` 适配器。P33 定义了数据结构但从未填充。P24 添加了递归类型参数但在序列化中丢失它们。在每种情况下，**结构就位但功能未完全贯穿**——造成了比完全没有解决方案更危险的"看起来正确"的错觉。

### 主题 5：P28 unwrap 叙事具有误导性——实际情况好于报告

**被 4/11 专家识别**：Implementation Strategist、Pragmatic Enforcer、Rust Expert、Maintainability Expert

**纠正**：原始评审报告了 165 处生产代码 `unwrap()`。实际审计显示几乎所有都在 `#[cfg(test)]` 块中。生产代码路径中仅存在约 2-3 处真正的 unwrap/expect——位于 `ir/lower/mod.rs:226` 的单处 `expect()` 是一个编译器不变性断言（"模块必须存在"），而它在 Rust 中是惯用且合理的。P28-ir 正确地修复了这唯一的真正生产 unwrap。剩余的 164 处测试 unwrap 不是紧急风险。

### 主题 6：P38 postMessage 安全——沙箱正确，但消息传递存在缺口

**被 3/11 专家识别**：TS/React Specialist (H3-HIGH、H4-HIGH)、Security Specialist (H1)、Systems Architect

**问题**：沙箱化 iframe（`sandbox="allow-scripts"`，无 `allow-same-origin`）在架构上是正确的，但：
- `postMessage` 在两个位置使用 `'*'` 作为 targetOrigin——任何窗口都可接收消息
- 桥接脚本通过字符串拼接 + `eval()` 注入——如果编译后的输出包含用户数据，则构成 XSS 向量
- 调试窗口 HTML 在 `public/` 和 `src-tauri/` 之间重复——维护陷阱

### 主题 7：CI 集成缺口——测试存在但未自动化

**被 4/11 专家识别**：AI Engineer (C1)、Pragmatic Enforcer (P5c)、Domain Expert、Rust Expert

**缺口**：
1. TS golden 文件解析测试**不在 CI 中运行**——仅 `npm run build`，不运行 `vitest`（Pragmatic Enforcer：这是评审周期中最大的单一剩余风险）
2. Eval 脚本未集成到 CI——手动离线工具
3. Rust 方面有 39 个 Clippy 警告，且 `-D warnings` 会阻止 CI
4. Cargo audit 运行但未配置为在发现 CVE 时阻止 CI（缺少 `--deny warnings`）

### 主题 8：CodegenCache 死代码及其他未使用的抽象

**被 3/11 专家识别**：Performance Specialist (H1)、Rust Expert、Systems Architect

**实例**：
- `codegen/shared/cache.rs`（135 行）——CodegenCache 从未导入或连接
- `codegen/shared/ui_codegen.rs`——`build_component_output`、`build_store_output` 从未调用
- `ir/lower/expr.rs:543`——`lower_expr_to_json` 死函数
- `ts-rs` 依赖——`#[derive(TS)]` 注解存在但 TypeScript 前端从未消费生成的类型
- `packages/omni-animation` 未包含在 pnpm workspace 中——通过 `include_str!` 嵌入但在 monorepo 工具中不可见

---

## 专家分歧与权衡

| 分歧点 | 甲方观点 | 乙方观点 | 建议 |
|--------|---------|---------|------|
| **P30a WASM 解析器导出** | Pragmatic Enforcer: 过度工程（为未被选择的路径构建基础设施，Ratio 1.67） | Domain Expert: 低风险基础设施，可能有未来用途 | **保留但标记为"实验性"**——添加文档注释说明其未被支持的状态；若 6 个月内无调用者则弃用 |
| **P33 source_span 范围** | Compiler Architect: 从 32 个未填充的结构体中移除字段 | Maintainability Expert: 填充所有 32 个结构体 | **折中方案**：填充 4 个核心结构体（ComponentIR、StoreIR、ModuleIR、IRProgram）；延迟 28 个面向表达式的结构体直到有实际消费者 |
| **P28 严重性** | 原始评审: 165 处生产 unwrap（HIGH） | Implementation Strategist: 几乎全部在测试代码中；生产代码路径已清除 | **原始评审的计数具有误导性**。P28 在消除生产 unwrap 方面是成功的。应基于实际生产影响（最小）重新评估严重性。 |
| **Golden 测试范围** | Domain Expert: 应验证语义等价性，而非仅验证解析成功 | Pragmatic Enforcer: 解析成功 + 往返测试已足够作为漂移检测 | **当前方法合理**——解析成功可捕获语法漂移；语义等价性测试是一项独立的、工作量更大的特性 |
| **OmniView 输出统一** | Systems Architect: P31 是"统一性的假象" | Compiler Architect: 务实适配器——WASM 路径需要额外的编辑器特定字段 | **记录设计权衡**——OmniViewOutput 为编辑器预览携带合法的领域特定字段；统一性假象比完全没有 trait 更危险 |

---

## 优先级建议

### 🔴 关键——立即执行（本周内）

#### A 组：部分完成项补齐

| # | 行动 | 工作量 | 文件 |
|---|------|--------|------|
| A1 | **填充 source_span** 在 ComponentIR、StoreIR、ModuleIR、IRProgram 上 | **S** (30 min) | `ir/lower/component.rs:289,634`、`ir/lower/mod.rs:206,264` |
| A2 | **修复 TypeKind 序列化**以保留 type_args——在 JSON 中发出结构化 `{name, args}` 格式，反向兼容旧格式 | **M** (2-3 hrs) | `ir/types.rs:336-348` |
| A3 | **将 TS golden 测试接入 CI**——添加 `npx vitest run golden.test.ts` | **S** (30 min) | `.github/workflows/ci.yml` |
| A4 | **从生产 CSP 中移除 `'unsafe-eval'`**（或记录为何 Monaco 需要它） | **S** (5 min) | `tauri.conf.json:22` |
| A5 | **将 postMessage targetOrigin 固定为 `window.location.origin`** | **S** (5 min) | `debug-window/index.html:176,285` |
| A6 | **修复 39 个 Clippy 警告**——运行 `cargo clippy --fix`，手动处理剩余 | **M** (2-3 hrs) | 多个 `omnilang/src/` 文件 |

#### B 组：推迟项目立即启动

| # | 行动 | 工作量 | 描述 |
|---|------|--------|------|
| B1 | **P35: Crate 拆分**——omnilang-core + 各后端独立 crate | **L** (2-4 周) | 将 63K 行单一 crate 拆分为 `omnilang-core`（lexer/parser/semantic/ir）+ `omnilang-codegen-omniview` + `omnilang-codegen-react` + `omnilang-codegen-flutter`。6 个暂停后端保留 feature-gated 在各自 crate 中。 |
| B2 | **P36: 代码生成并行化**——rayon 后端分发 | **M** (3-5 天) | 在 `codegen::compile()` 中使用 `rayon::scope` 并行生成各后端输出。当前 parsing/semantic 已并行，codegen 仍单线程。 |
| B3 | **P38b: Strip TAURI_INTERNALS** | **S** (1-2 hrs) | P38 iframe 沙箱已生效 1 天。剥离 `__TAURI_INTERNALS__` 暴露以加固调试窗口隔离。 |
| B4 | **P39: IR 优化传递**——移除未使用信号、内联简单 computed | **L** (1-2 周) | 先补齐 motion 回归测试到 95% 分支覆盖率，再实现优化传递。当前 motion 测试 13 个。 |

### 🟠 高优先级（2-4 周）

| # | 行动 | 工作量 | 文件 |
|---|------|--------|------|
| H1 | **消除 3 处 `graph-parser.ts`/`graph-compiler.ts` 中的 `as any`**——向 SegData 添加 `operations` 字段 | **S** (15 min) | `graph-parser.ts:492,508`、`graph-compiler.ts:118` |
| H2 | **消除 5 处 PropsSection `as any`**——用 `WeakMap<HTMLElement, callback>` 替换 DOM hack | **M** (30 min) | `PropsSection.tsx:164,176,180,365,396` |
| H3 | **添加 `@typescript-eslint/no-explicit-any: "warn"`** 到 .eslintrc | **S** (5 min) | `.eslintrc` |
| H4 | **连接 CodegenCache** 到 OmniView 代码生成路径或删除它 | **M** (2-3 hrs) | `codegen/shared/cache.rs` + `codegen/omniview/mod.rs` |
| H5 | **替换调试窗口桥接脚本注入**——使用 `URL.createObjectURL(new Blob(...))` 代替字符串拼接 + srcdoc | **S** (30 min) | `debug-window/index.html:275-334` |
| H6 | **修复 cargo audit CI**——添加 `--deny warnings` 以在发现 CVE 时阻止合并 | **S** (5 min) | `.github/workflows/ci.yml:99` |
| H7 | **记录 OmniViewOutput vs CompileOutput 设计权衡**——在 `mod.rs:39` 处添加注释 | **S** (5 min) | `codegen/omniview/mod.rs:39` |
| H8 | **修复空 catch 块**——DesignPage.tsx 中 2 处，debug-window 中 2 处 | **S** (5 min) | `DesignPage.tsx:117,128` |

### 🟡 短期（4-8 周）

| # | 行动 | 工作量 |
|---|------|--------|
| S1 | **解耦服务层与 store**——4 个服务文件直接导入 Zustand store；应通过参数接收 | **M** (1-2 hrs) |
| S2 | **消重调试窗口 HTML**——从单个来源构建 `public/` 和 `src-tauri/` 副本 | **S** (30 min) |
| S3 | **填充 `agent_docs/`**——创建 README.md、workflows.md、reference.md 以解析 `templates/AGENTS.md` 中的断开引用 | **M** (2-3 hrs) |
| S4 | **将 eval 脚本集成到 CI**——添加 `bash .claude/evals/eval-run-omnilang.sh` 作业 | **S** (30 min) |
| S5 | **修复 WASM 缓存序列化开销**——仅当缓存已更改时才序列化 | **S** (30 min) |
| S6 | **创建 `poiesis/src/services/codegen/README.md`**——执行 ADR-005 文档待办事项 | **S** (15 min) |
| S7 | **连接或移除 ts-rs**——生成前端消费的 `.d.ts` 文件，或移除依赖 | **M** (1 day) |
| S8 | **向 `packages/omni-animation` 添加 pnpm workspace 集成** | **S** (5 min) |
| S9 | **添加 `#[non_exhaustive]`** 到 ElementIR、ComponentIR、StoreIR | **S** (10 min) |
| S10 | **在 postMessage 接收处理程序中添加 origin 验证** | **S** (10 min) |
| S11 | **重新评估 CVE 审计追踪**——确认 8 个关键问题的状态，更新计数 | **M** (2-4 hrs) |
| S12 | **替换 `omni_to_compile_output` 适配器**——使回归测试使用 `OmniViewCodegen.generate()` | **S** (30 min) |

### 🟢 长期（2-6 个月）

| # | 行动 | 工作量 |
|---|------|--------|
| L1 | 为面向表达式的 IR 结构体填充 source_span（若有消费者） | **M** |
| L2 | 将 OmniViewGenerator 从 55 个字段拆分为关注点清晰独立的组件 | **L** |
| L3 | 实现语义等价性测试——验证相同 IR 在后端之间产生等效的运行时行为 | **L** |
| L4 | 为 Poiesis 添加结构化可观测性（编译延迟、WASM 回退率、错误频率） | **M** |

---

## 若不解决的风险

| 风险 | 可能性 | 影响 | 相关主题 |
|------|--------|------|---------|
| **语法漂移仍不可见**——TS 解析器分歧，CI 保持绿色（TS golden 测试不在 CI 中） | 中（每次语言变更） | **高**——编辑器损坏 | A3 |
| **source_span 产生虚假信心**——开发者认为源位置可用，但实际不可用，导致静默调试失败 | 中 | **中**——浪费调试时间 | 主题 1, A1 |
| **泛型类型损坏**——`Array<String>` 在 WASM 边界变为 `Array` 后生成不正确代码 | 中（首次传递泛型类型的序列化发生时） | **高**——不正确的编译输出 | 主题 2, A2 |
| **`as any` 持续增长**——无 lint 规则阻止；12→20+ 在未来 sprint 中是可能的 | 高 | **中**——运行时 TypeError | 主题 3, H1-H3 |
| **39 个 Clippy 警告阻止 CI**——日常开发受阻；压倒性的质量信号 | 当前（CI 已损坏） | **中**——开发者体验 | A6 |
| **调试窗口 postMessage 欺骗**——`'*'` 源允许任何窗口接收消息 | 低（需要第二个已入侵的窗口） | **中**——沙箱逃逸 | 主题 6, A5 |
| **推迟项目无限期延迟**——P35/P36/P39 继续推迟导致架构腐烂和性能退化 | 高（若不立即启动） | **高**——编译器演进受阻 | B1-B4 |

---

## 各专家评审原文

以下为 11 位专家的完整独立评审报告。汇总部分基于这些原文进行交叉分析和优先级排序，但不替代原文——原文是最权威的原始数据。

### 1. 安全专家（Security Specialist）

**领域**：威胁建模、安全模式、数据保护、漏洞识别

**总体裁决**：13 项安全相关优先级中，9 项已修复。P1（FS 权限）已由 ADR-004 决策 2 确认为设计意图。生产 CSP 已移除脚本的 `'unsafe-inline'`，但 `'unsafe-eval'` 仍然存在。调试窗口隔离（P2）和 IPC 大小限制（P14）是正确实施的亮点。

**新发现**：`compile_source` 无速率限制——任何 webview 上下文均可调用。CI cargo audit 未配置 `--deny warnings`（M2）。

**关键发现**：CRITICAL—CSP 'unsafe-eval'（C2）；HIGH—debug-window eval 注入（H1）、compile_source 无速率限制（H2）；MEDIUM—cargo audit 策略缺失（M2）、词法分析器 DoS（M3）。

### 2. 实施策略专家（Implementation Strategist）

**领域**：变更排序、爆炸半径分析、可逆性设计

**总体裁决**：Sprint 排序正确尊重了依赖链。三个"声称完成"的项目仍未完成（P13、P22、P5c）。P33 的 3 阶段实施（结构定义→测试修复→核心结构体补充）是教科书式的变更管理，适用于一次有 38 个文件受影响的大范围爆炸半径变更。

**对先前评审的纠正**：
1. P18 完全完成（111 行 lib.rs + 6 个命令模块）——计划评审分析的是中间状态
2. P33 完全完成（37/37 个结构体）——计划评审遗漏了提交 625bc2a 中的最终 4 个
3. deferrals.md 存在且包含所有 5 项推迟的合理理由

**关键发现**：MEDIUM—as any 系统性模式（C1）、DesignPage.tsx 中仍有空 catch 块（C2）、lower/mod.rs:226 处的 expect() 仍是 panic（C3 降级为 LOW）。

### 3. AI 工程师（AI Engineer）

**领域**：AI/ML 集成、Agent 系统、AI 评估框架

**总体裁决**：自 2026-07-26 以来显著改善（5 篇 ADR、deferrals.md、运营后分析文化），但改善的是**架构治理**而非**AI 实施**。代码库中仍无 AI 功能、无可观测性仪表盘、无评估 CI 集成。P34（AI 补全）继续搁置，待编辑器和编译器核心稳定后另立专项计划。

**关键发现**：HIGH—评估脚本未集成到 CI（C1）；MEDIUM—`agent_docs/` 为空，AGENTS.md 引用断开（C2）、Poiesis 无 AI 功能且应立即启动设计（C3 升级）；LOW—CVE 审计追踪腐烂（C4）、无结构化 AI 可观测性（C5）。

### 4. 性能专家（Performance Specialist）

**领域**：性能优化、资源利用、瓶颈识别

**总体裁决**：P7 Monaco 代码分割实施正确且有效。P8 WASM 增量缓存正确连接。最大的性能缺口是**代码生成级缓存**——`CodegenCache`（cache.rs 中 135 行）是死代码，从未连接到 OmniView 代码生成路径。P36（codegen 并行化）应立即执行，不再推迟。

**关键发现**：HIGH—CodegenCache 死代码（H1）、每次 WASM 编译调用都进行完整 IncrementalCache 序列化（H2）；MEDIUM—IR 深克隆所有未更改文件（M1）、useWasmCompile.ts 中的 JSON.stringify 去重（M2）、HMR 序列化静默数据丢失（M3）。

### 5. 编译器 & 语言架构师（Dr. Lin Wei）

**领域**：编程语言设计、多目标代码生成、IR 设计

**总体裁决**：编译器管道在架构上是合理的，但存在系统性的"部分完成"缺陷。P4 恢复了 source_map 连接，P24 为 TypeKind 添加了递归，P31 统一了 Codegen trait——但三者都需要后续工作才能充分发挥作用。P35 crate 拆分和 P39 IR 优化应立即执行。`OmniViewGenerator` 从 50+ 增长至 55 个字段。

**关键发现**：HIGH—32/33 个 IR 结构体上的 source_span 为 None（H1）；MEDIUM—TypeKind 序列化丢失 type_args（M1）、OmniViewGenerator 55 个字段膨胀（M2）、参数化测试验证一致性而非等价性（M3）；LOW—omni_to_compile_output 适配器绕过 Codegen trait 路径（L1）。

### 6. 可维护性专家（Maintainability Expert）

**领域**：代码质量、重构、技术债务

**总体裁决**：P18（Tauri 拆分）、P19（参数化测试）和 P31（Codegen trait）是执行良好的重构。关键可维护性风险集中在三个缺口：32/33 个 IR 结构体上的 source_span 未被填充（维护陷阱）、Codegen trait 路径未被回归测试覆盖、as any 从 4 增长到 14。

**关键发现**：HIGH—source_span 未填充（H1）、OmniView Codegen trait 路径未经测试（H2）；MEDIUM—as any 侵蚀（M1）、TypeKind 序列化（M2）。

### 7. 系统架构师（Systems Architect）

**领域**：分布式系统、服务架构、可扩展性模式、系统分解

**总体裁决**：系统的双编译拓扑（WASM + Tauri）现在具有更好的 IPC 契约、调试窗口隔离和跨目标测试。然而，关键集成点在 monorepo 工具中不可见：`packages/omni-animation` 不包含在 pnpm workspace 中（通过 `include_str!` 的盲集成点），而 ts-rs 类型被生成但从未被 TypeScript 前端消费。P35 crate 拆分应优先执行以改善系统分解。

**关键发现**：HIGH—pnpm workspace 省略 omni-animation（H1）、as any 回退（H2）、CSP 'unsafe-eval'（H3）；MEDIUM—OmniView Codegen trait 是假象（M1）、all-backends 特性文档缺口（M2）、Tauri ProjectState Mutex 瓶颈（M3）、ts-rs 未被消费（M4）。

### 8. 务实执行者（Pragmatic Enforcer / YAGNI Guardian）

**领域**：YAGNI 原则、复杂度分析、最小可行方案

**总体裁决**：**整体务实评分：B+**（从 B 提升）。P15（9→3 目标，Ratio 0.22）是 YAGNI 的教科书范例。P30a（为未被选择路径构建的 WASM 基础设施，Ratio 1.67）和 ADR-004（评审的评审）是过度工程。P33 处于最危险状态：数据模型成本已支付（29/33 个结构体），但 4 个核心结构体缺失，管线仍未完成。P28 叙事"165→164 unwrap"具有高度误导性——生产 unwrap 已被有效消除。

**对推迟项目的务实评估**：用户要求立即完成推迟项目。从务实角度：P35 crate 拆分降低了面向 IR 变更的爆炸半径（目前每次 IR 变更触达 9 个后端）。P36 并行化在项目 >30 文件时收益显著。P39 优化传递依赖于先补齐 motion 测试覆盖率。P38b 是低成本的纵深防御加固。P34 AI 补全继续搁置。

### 9. Rust 专家（Katya Volkov）

**领域**：内存安全、零成本抽象、编译器设计、WASM 编译

**总体裁决**：核心架构改进显著（Codegen trait 统一、源映射文档、测试基础设施），但 P33 实现仅 33% 完成（结构已定义但未传播），CI 因 39 个 Clippy 错误而损坏。TypeKind 序列化错误是在生成代码中导致静默数据损坏的最关键新发现。P35 crate 拆分将使各后端可独立编译和测试。

**关键发现**：CRITICAL—TypeKind 序列化丢失 type_args（#1）、source_span 始终为 None（#2）；HIGH—39 个 Clippy 错误阻止 CI（#3）、`lower()` 中的生产 expect()（#4）、手动 strip_prefix 可能 panic（#5）；MEDIUM—测试中未使用的变量（#6）、静默 store_mode 回退（#7）、重复的 MAX_INPUT_SIZE（#8）；LOW—ElementIR 缺少 `#[non_exhaustive]`（#9）。

### 10. TypeScript & React 专家（Emma Rodriguez）

**领域**：React 组件架构、TypeScript 类型系统、前端状态管理

**总体裁决**：测试套件（144 个测试，全部通过）质量高，golden 文件解析覆盖 51 个生产 .omni 夹具。但出现了三个安全架构问题：postMessage `'*'` 源（H3）、通过字符串拼接 + eval 注入桥接脚本（H4）以及服务直接导入 Zustand store（H2）。`as any` 从 4 增长至 12（源码）集中在三个文件中。

**关键发现**：HIGH—as any 回退（H1）、服务层直接导入 store（H2）、postMessage `'*'` 源（H3）、桥接脚本注入向量（H4）；MEDIUM—PropsSection DOM hack 5 处（M1）、可修复的 as any 被忽视（M2）、调试窗口 HTML 重复（M3）、轮转测试子集选择脆弱（M4）。

### 11. 领域专家（Domain Expert）

**领域**：DDD、限界上下文、通用语言、业务逻辑建模

**总体裁决**：双解析器架构是一个有效的领域决策——TS 解析器服务于编辑器领域（可视化布局、PID 稳定性、代码流图），而 Rust 解析器服务于编译器领域（符号解析、类型检查、代码生成）。它们共享语法但服务于不同的限界上下文。Golden 测试仅验证"解析成功"而非"语义等价"，但这是务实的第一步。ADR-005 术语尚未传播到代码注释/目录。

**关键发现**：MEDIUM—Golden 测试验证解析而非语义等价性、OmniView 输出统一是假象、TypeKind 序列化、通用语言未推送到代码；LOW—as any 回退、跨目标测试扩展性。

---

## 跟进计划

1. **A 组关键修复（本周）**：A1-A6——source_span + TypeKind 序列化 + CI 缺口 + CSP + postMessage + Clippy
2. **B 组推迟项目立即启动（本周开始）**：B1-B4——crate 拆分 + codegen 并行化 + TAURI_INTERNALS + IR 优化
3. **高优修复（2-4 周）**：H1-H8——as any 消除 + CodegenCache + postMessage 加固 + 空 catch 块
4. **短期债务（4-8 周）**：S1-S12——服务层解耦 + 文档 + 工具集成 + 可观测性
5. **30 天复查**：验证 A 组和 B 组进度；重新评分关键发现；审查 as any 趋势
6. **deferrals.md 更新**：清空推迟列表，所有项目已升级为立即执行

---

## 参与专家

| 专家 | 角色 | 关键贡献 |
|------|------|---------|
| Security Specialist | 安全专家 | 确认 FS 权限为设计决策；CSP unsafe-eval 仍存在；compile_source 无速率限制 |
| Implementation Strategist | 实施策略专家 | 纠正先前评审中关于 P18/P33/deferrals 的 3 处事实错误；揭示 P28 unwrap 叙事误导 |
| AI Engineer | AI 工程师 | 发现 agent_docs/ 为空；CI 评估集成缺口；CVE 审计追踪腐烂；P34 应立即启动设计 |
| Performance Specialist | 性能专家 | 发现 CodegenCache 是死代码；每次按键进行 WASM 缓存 JSON 序列化；P36 应立即执行 |
| Dr. Lin Wei | 编译器架构师 | 揭示"部分完成"模式；32/33 source_span 为 None；OmniViewGenerator 55 个字段；P35+P39 应立即执行 |
| Maintainability Expert | 可维护性专家 | as any 分类；Codegen trait 路径未经测试；TypeKind 序列化丢失 |
| Systems Architect | 系统架构师 | pnpm workspace 缺口；ts-rs 死依赖；OmniView trait 假象；P35 优先执行 |
| Pragmatic Enforcer | 务实执行者 | 比率分析验证 P15；P30a/ADR-004 过度工程；P33 处于最危险中间状态 |
| Katya Volkov | Rust 专家 | TypeKind 序列化关键 bug；39 个 Clippy 错误；source_span 未传播 |
| Emma Rodriguez | TS/React 专家 | postMessage `'*'` 源；桥接脚本注入向量；服务-存储耦合 |
| Domain Expert | 领域专家 | 双解析器领域有效性；术语传播缺口；语义等价性测试缺口 |

---

> 🤖 本评审由 AI Software Architect 框架通过 11 位专家并行复审 + 交叉分析生成。
> 评审范围：全工作区。务实模式：已启用（balanced intensity）。
> 基线：[full-workspace-review-2026-07-26.md](./full-workspace-review-2026-07-26.md)
> 架构决策引用：[[ADR-004]] 决策 2（FS 权限设计意图）、[[ADR-003]]（代码生成目标优先级）
