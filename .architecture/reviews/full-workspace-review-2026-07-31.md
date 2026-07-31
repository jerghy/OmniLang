# OmniLang 全工作空间架构评审 — 2026-07-31

**评审范围**: OmniLang monorepo 全工作空间（编译器核心、设计 Token 管线、OmniView/React 代码生成、Poiesis Tauri 编辑器、omni-animation 包、Design-System、文档与 ADR）

**评审团队**: 12 位专家并行评审（Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert, TypeScript/React Specialist, Compiler Architect, Documentation Expert）

**评审日期**: 2026-07-31 | **状态**: Complete

---

## 执行摘要

OmniLang 工作空间的架构核心——编译器管线（lexer→parser→semantic→IR→codegen）——是扎实且经过深思熟虑的。50 份 ADR、550+ Rust 测试、跨 9 个后端的特性门控代码生成，以及成熟的增量缓存架构，构成了一个出色的工程基础。

然而，本次评审发现了**两类反复出现的失效模式**：

1. **边界合约漂移**：管线阶段之间的交接（semantic→lowering→diagnostics）、编译器与嵌入式运行时工件（`include_str!` 打包的动画 dist）、以及 WASM FFI / Tauri IPC 序列化契约——都存在漂移，导致**静默数据丢失**。最严重的发现：`ir::lower()` 产生的诊断信息在**所有**用户可见路径（CLI 构建、WASM 编译、Poiesis 诊断）中被丢弃，验证类错误（E0005/E0006/E0011/@ref）在有可见源码错误的情况下仍以 exit 0 静默通过。

2. **ADR-051 合并回退**：全局评审修复（`a5116408` 热重载清理守卫）在合并冲突解决中被丢失（`4ad49452`），而 ADR-051 提交在之后记录了该修复为"已验证"。`main` 分支上不存在 `__omni_tokenEffectCleanup`——该 ADR 证明了一个不存在的修复。这是一个**治理失效**，而不是代码缺陷。

这两个模式都指向同一个根本原因：**项目已超出其验证边界**。550+ 测试在"直接调用 API"层面是绿色的，但管线入口点（IR lowering 后诊断丢失）、跨组件新鲜度（嵌入式动画 dist、已提交构建产物）、以及 ADR 验证（针对分支状态而非合并后主分支进行验证）——都不在自动化检查范围内。

**整体评估: 需要改进**（编译器核心 Strong，外部边界 Adequate，治理流程 Needs Improvement）

**Pragmatic Score**: 必要性 10/10, 复杂度 4/10, 比率 0.40

---

## 关键发现：CRITICAL（2 项）

### C1 — IR lowering 后诊断在所有用户可见管线路径中被静默丢弃

**来源**: Compiler Architect（经验证重现） | **严重性**: CRITICAL

**位置**:
- `crates/omnilang-core/src/lib.rs:107-111` — `run_pipeline` 在 `ir::lower()` **之前**排空 `semantics.diagnostics`
- `crates/omnilang-core/src/analyse.rs:303-312` — `analyse_multi` Phase 3a，同样模式
- `crates/omnilang-core/src/analyse.rs:464-472` — `analyse_multi_cached`，同样模式
- 生产者: `crates/omnilang-core/src/ir/lower/mod.rs:182-205`（`validate_design`）、`:295-299`（`validate_required_props`）、`:301-305`（`validate_refs`）

**被丢弃的错误类别**:
- E0005/E0006: `class Card { background: token(missing); }` → 编译成功，CSS 包含 `var(--missing)`
- E0011: `<Child />` 缺少必需的 `prop name` → 编译成功，生成代码调用未定义 prop
- `@ref` 绑定验证错误 → 静默通过

**经验证**: `omnic build` 和 `omnic analyse` 对包含 `token(missing)` 的源码返回 `{"diagnostics":[],"success":true}`。生成的 CSS 静默包含 `background: var(--missing)`。

**根因**: 管线交接约定——`ir::lower()` 可以向 `semantics.diagnostics` 追加诊断——但所有三个调用点在 `lower()` 返回**之前**就排空了该字段。现有测试直接调用 `lower()` 并读取作用域内的 `semantics`，因此掩盖了此 bug。

**修复**（S 工作量): 在三处将 `diagnostics.extend(...)` 移到 `ir::lower()` 调用**之后**；在末尾重新计算 `has_errors`；为 E0005/E0006/E0011 添加管线级回归测试。

---

### C2 — 热重载清理修复在合并中丢失；ADR-051 证明了一个不存在的修复

**来源**: Systems Architect（经验证：`main` 分支零匹配） | **严重性**: CRITICAL

**位置**: `omnilang/src/codegen/omniview/design.rs:283-302` — `emit_dynamic_token_effects` 发出不带清理的 `__omni_effect(() => { ... })`

**证据**:
- 提交 `a5116408` 添加了 `window.__omni_tokenEffectCleanup` 清理守卫
- 合并 `4ad49452`（冲突解决）用普通 `__omni_effect(() => {` 替换了清理行
- 提交 `22126492`（ADR-051）记录了验收标准 `[x] Hot-reload effect cleanup guard emitted in generated JS`
- `grep tokenEffectCleanup` 在整个仓库中返回**零匹配**（`main` 分支）

**影响**: 每次编辑器热重载都会重新订阅 effect 闭包，没有清理路径——6 位专家在全局评审中标记的确切 CRITICAL 仍然存在。ADR 证明了一个在合并代码中不存在的修复。

**修复**（S 工作量): 重新应用两行清理：在分配新 effect 返回值之前调用旧的 `window.__omni_tokenEffectCleanup()`；添加回归断言；添加合并后验证步骤（针对 `HEAD` 而非分支状态验证 ADR 验收标准）。

---

## HIGH 发现（12 项，去重后）

### H1 — 非 OmniView 目标静默产生零输出（A2 回归）

**来源**: Implementation Strategist（经验证：编译了 `omnic`）、Systems Architect、Documentation Expert | **严重性**: HIGH

**位置**: `omnilang/src/codegen/mod.rs:92-203`（9 个提前返回）；`omnic/src/commands/build.rs:31-48`（从不打印警告，空输出 exit 0）

设计块守卫（ADR-051 A2）对**所有** 9 个非 OmniView 目标执行 `return CompileOutput { files: vec![], .. }`——不仅是设计 CSS，是整个程序。`omnic build --target react` 在任何含设计块的项目上静默产生零文件。A2 之前的二进制文件为相同命令产生 142 个文件。默认 `omnic build`（自动检测 → react，`api.rs:62-67`）命中此提前返回。

**修复**: 恢复"警告并继续"行为（后端已忽略设计块）；在 `omnic build` 中打印警告；添加 CLI 级测试。S-M 工作量。

---

### H2 — 陈旧动画运行时嵌入每个编译输出

**来源**: Maintainability Expert、TypeScript/React Specialist、Systems Architect | **严重性**: HIGH

**位置**: `omnilang/src/codegen/omniview/utils.rs:194-197, 283-286`（`include_str!` 打包 `packages/omni-animation/dist/`）

**已验证的具体差异**:
1. `dist/` bounceOut 缓动与 bounceIn **相同**（`cubic-bezier(0.215,0.610,0.355,1.000)`），但 `src/` 有经提交 `961025f1` 修复的独立曲线
2. `dist/` 仍包含 hero 代码（13 处引用），在 ADR-032 中已从 `src/` 删除
3. `dist/` 包含旧版模板字面量参数化缓动代码（`${(0.46 + overshoot...`），在源码中已不存在

包自身的测试从 `src/` 导入——因此 `npm test` 通过而发布的工件错误。

**修复**: 重建 dist（`pnpm build`）并提交；添加 CI 新鲜度门控（比较 dist 时间戳/哈希与 src git 日志）；在 `omnilang/build.rs` 或 `scripts/` 中添加重建步骤。

---

### H3 — Poiesis 保存时 WASM 验证门控始终静默通过（死代码）

**来源**: AI Engineer | **严重性**: HIGH

**位置**: `poiesis/src/services/monaco/diagnostics.ts:144-163` — `validateSources` 在包装对象 `{ result, cache_state }` 上检查 `"errors" in result`，而非在 `.result` 内部。始终返回 `[]`。三个保存路径（`useSaveProject.ts:118, 150, 207`）调用此门控。

**影响**: 单个文件保存时，跨文件语义错误（未定义引用、导入失败）从不被捕获。与 ADR-050/051 系统消除的静默降级类别相同。

**修复**: 展开包装：`const inner = result?.result; return (inner?.errors ?? [])`；删除死代码 `Array.isArray` 分支；添加带有模拟 WASM 的 vitest。S 工作量。

---

### H4 — 入口文档陈旧：结构、ADR 数量、示例项目、标准库路径

**来源**: Documentation Expert | **严重性**: HIGH（3 项合并）

**位置**:
- `README.md:20-30`: 结构映射到 `omnilang/src/{lexer,parser,...}`——P35 crate 拆分后实际代码在 `crates/omnilang-core/src/`；`omnilang/` 现在是一个薄的重新导出 shim
- `README.md:52,86`: "13 ADRs"——实际存在 50 份
- `docs/stdlib.md:5,27,37,47`: 教授 `import { abs } from "omni/math"`——规范路径是 `std/<module>`（`crates/omnilang-core/src/stdlib/mod.rs:3`）
- `QUICKSTART.md:86-97`: 列出 `logic-cli-tool`、`view-todo` 等——`test/projects/` 中不存在；README.md 引用 `TodoPage.omni`、`TimelineMotionDemo.omni`——在重构的 learn-omniview 中不存在
- `README.md:13`: `omnic init my-app`——`init` 子命令不存在；实际命令：`create`

**影响**: 新用户的前三个文档引导到错误的导入、不存在的项目和无效的 CLI 命令。参考文档（15 章 OmniView 手册、BNF 语法）是最新的——但新用户在到达它们之前就失败了。

**修复**: 重新生成结构块和计数；废弃 `docs/stdlib.md` 以支持一致的 `docs/omniview/12-standard-library.md`；将示例重新指向 `test/projects/learn-omniview`；将 `init` 替换为 `create --type view`。M 工作量。

---

### H5 — Poiesis 往返静默破坏主题结构和非字面量 token 值

**来源**: Domain Expert | **严重性**: HIGH

**位置**: `poiesis/src/core/types/design.ts:22-28`（DesignDefinition 无 themes 字段，DesignToken.value 仅为 `string`）；`poiesis/src/services/codegen/parser.ts:1010-1038`（`@themes`/`@default-theme`、`dark "#a5b4fc"` 覆盖被跳过）；`poiesis/src/services/codegen/design-emitter.ts:125-130`（值始终重新以引号形式发出）；`poiesis/src/App.tsx:162-238`（在项目打开时加载 design.omni 到 store 中）

**已验证的数据丢失场景**:
- `token primary: color = BRAND;` → store 得到 `"BRAND"` → 保存为 `token primary: color = "BRAND";`（const 引用变为字面量字符串）
- `@themes light, dark;` + 每个主题覆盖 → 保存时静默删除
- `token primary: color = ThemeStore.primaryColor;` → 保存为 `"ThemeStore.primaryColor"`（动态绑定被破坏）

编辑器的保存时 WASM 验证检查的是**重写后**的输出，该输出编译成功——破坏是静默的。

**修复**: 当解析的文件包含 store 无法表示的构造时，至少拒绝重写 `design.omni` 并显示可见警告；最终添加 `themes`/`themeValues` 到 `DesignDefinition` 和 `TokenValue` 的有区别联合类型。

---

### H6 — `@default-theme` 是文档化的指令，运行时效果为零

**来源**: Domain Expert | **严重性**: HIGH

**位置**: `omnilang/src/codegen/omniview/mod.rs:730-741` — `__omni_theme.init()` 回退到第一个主题，从不读取 `default_theme`；`docs/omniview/10-design-system.md:87` 记录了该功能。

**修复**: 在 `__omni_theme.init()` 中尊重它（回退链：localStorage → prefers-color-scheme → `default_theme` → 第一个主题），或删除该指令及其文档。S 工作量。

---

### H7 — 编辑器 "font" token 类型发出编译器拒绝的 design.omni，阻止保存

**来源**: Domain Expert | **严重性**: HIGH

**位置**: `poiesis/src/core/types/design.ts:10`（编辑器联合中的 `"font"`）；`crates/omnilang-core/src/parser/design.rs:205-223`（编译器的 `parse_token_type` 接受 13 种类型——不包括 `"font"`）。

**影响**: 创建排版 token 发出 `token font-new: font = "Inter";`，编译器拒绝。因为编辑器在写入前验证发出的文件，**每个**后续保存都会失败并出现解析错误。

**修复**: 从编辑器联合中移除 `"font"`，或将 `font` 添加为编译器中的 `fontFamily` 别名。S 工作量。

---

### H8 — 多文件诊断通过非增量路径运行完整代码生成

**来源**: Performance Specialist | **严重性**: HIGH

**位置**: `poiesis/src/services/monaco/diagnostics.ts:81-90` → `omnilang-wasm/src/lib.rs:614-624`（多文件分支调用 `compile_multi_internal`——**非**缓存变体）→ 完整代码生成，丢弃除 `errors` 外的所有内容。

**已测量**: 原生环境对 92 文件语料库约 20-30ms；在 WASM 中估计 **100-300ms**（5-10x 开销），同步主线程阻塞。`diagnostics.ts:2` 的注释声称"快速 parse+semantic+lint（无 codegen）"——实际代码路径相反。

**修复**: 添加一个返回 `{ success, errors, warnings }` 的仅缓存诊断 WASM 导出（`analyse_multi_cached` 已存在并返回此数据）。S 工作量——Rust 端机制已存在；这是管道连接。

---

### H9 — React 后端静默丢弃整个设计系统

**来源**: TypeScript/React Specialist（经验证：编译了 learn-omniview 到 React） | **严重性**: HIGH

**位置**: `omnilang/src/codegen/react/` — 没有任何地方处理 design/token/theme。`gen_component_css` 仅发出 `comp.styles`；React 输出中不存在 token `:root`/theme CSS 声明。

**经验证**: 编译 learn-omniview（导入 `LearnDesign`）到 React → `DesignTokenPanel.module.css` 发出 `background: var(--primary)` 但输出中没有任何地方有 `:root` token 声明。CSS 变量未定义。生成的 `styles['Banner']` 引用是 `undefined`。

**修复**: 发出引用的设计类 + token `:root`/theme CSS 到共享的 `design.module.css`；添加 Rust 回归测试。这是后端中特性大小的差距。

---

### H10 — 解析器栈溢出 via 递归守卫绕过（已验证崩溃）

**来源**: Rust Expert（经验证：用 `omnic` 重现崩溃） | **严重性**: HIGH

**位置**: `crates/omnilang-core/src/parser/expr/prefix.rs:119, 132, 40`；`infix.rs:199, 215, 237, 78`

`enter_recursion()` 仅在 `parse_expr` 和 `parse_stmt` 被调用，但 Pratt 解析器有**五个**直接 `parse_expr_bp()` 递归路径从不经过守卫。**经验证**: 300 个连续的 `!` 字符 → `thread '<unknown>' has overflowed its stack`（exit 127）。在 WASM 中（`panic = "abort"`，1MB 栈），这会在页面生命周期内终止编译器的 WASM 实例。

**修复**: 将深度记录移入 `parse_expr_bp` 本身，或通过受守卫的 `parse_expr` 路由五个直接调用。S 工作量。

---

### H11 — React E2E 测试工具无法在 HEAD 运行

**来源**: Implementation Strategist、TypeScript/React Specialist | **严重性**: HIGH

**位置**: `test/projects/learn-omniview/build/react-test-app/` — `src/react-output/` 被 gitignore 且不存在；`src/main.tsx` 导入 `./react-output/components/App`；零文档说明如何生成。

CI e2e 作业编译到错误位置（项目根目录而非 `src/react-output/`），由于 H1 产生空输出，且规范断言的是中文标签页，而应用是英文的。

**修复**: 添加生成脚本，将其连接到 `package.json` 命令，修复输出放置，更新规范期望。M 工作量（修复），S 工作量（禁用）。

---

### H12 — WASM FFI 层违反自身文档化的序列化合约

**来源**: Maintainability Expert | **严重性**: HIGH

**位置**: `omnilang-wasm/src/lib.rs:184-187, 318, 418, 516, 647, 677` — 返回 `serde_json::json!` + `to_value`；合约文档在 `memory/wasm-ffi-serialization-contract.md`。Poiesis 携带三个不一致的 WASM 接口并行声明（`vite-env.d.ts`、`rustCompiler.d.ts`，以及通过 `as unknown as` 的实时代码）。

**修复**: 迁移到 `#[derive(Serialize)]` 结构体；删除两个陈旧声明；根据生成的 `omnilang_wasm.d.ts` 对实时代码进行类型检查。

---

## MEDIUM 发现（16 项）

| # | 发现 | 来源 | 位置 |
|---|------|------|------|
| M1 | CSS 消毒器结构绕过（已验证）：`}</style>` 突破、反斜杠字母转义 `\s`→`s`、`@import` 注入 | Security | `sanitize.rs:63-95` |
| M2 | 推测性 `parse_stmts` 存根，4 层，零调用者，存在潜在消费者 bug | Pragmatic Enforcer | `lib.rs:166-188` 等 |
| M3 | ~15MB 已提交生成/缓存产物（占跟踪内容的 46%） | Pragmatic Enforcer | `.playwright-mcp/`、`.git-rewrite/`、`test/omnilang/` |
| M4 | C FFI 层零仓库内消费者（357 行） | Pragmatic Enforcer | `crates/omnilang-core/src/ffi.rs` |
| M5 | 增量缓存上限（50）永久低于 92 文件实际项目 | Performance | `analyse.rs:97` |
| M6 | 无热路径性能门控（单次 8 行基准测试） | Performance | `benches/pipeline_bench.rs:5-25` |
| M7 | 设计 token 诊断使用零源跨度（did-you-mean 包含在内） | AI Engineer、Domain Expert | `resolve.rs:303-312` 等 |
| M8 | 7/9 代码生成后端从未在 CI 中编译或测试 | Maintainability | `.github/workflows/ci.yml` |
| M9 | `try_inline_component`（237 行）手动管理生成器状态，存在静默损坏风险 | Maintainability | `component_html.rs:11-248` |
| M10 | `peek_char` 在非 ASCII 运行上是 O(n²)（中日韩内容二次方） | Rust Expert | `lexer/mod.rs:105` |
| M11 | Clippy 不干净：死代码 + 未门控测试模块（179 个函数在 prod 库中） | Rust Expert | `lexer/mod.rs:742`、`name_resolver.rs:43-81` |
| M12 | 手工打造的诊断序列化器与 serde 实现产生分歧（缺少 `end_line`/`end_col`） | Rust Expert | `ffi.rs:13-27`、`poiesis/src-tauri/src/lib.rs:17-31` |
| M13 | 设计名称跨文件重复静默覆盖，不确定性地 | Compiler Architect | `analyse.rs:231`、`lower/mod.rs:234-238` |
| M14 | 评审框架质量指标是轶事，非系统化（无结构化指标注册表） | AI Engineer | `.architecture/` |
| M15 | Tauri 边界的手工维护合约镜像（诊断/IR 类型三次声明） | Systems Architect | `compile.rs:14-41`、`rustCompiler.ts:20-48` |
| M16 | Did-you-mean 零测试且候选集未过滤（建议无效修复） | AI Engineer | `error.rs:335-366` |

## LOW 发现（16 项）

| # | 发现 | 来源 |
|---|------|------|
| L1 | 设计体解析错误报告 `design` 关键字的行/列，而非错误位置 | Compiler Architect |
| L2 | `TokenValue::Expression` serde 不对称：序列化静默丢弃，反序列化硬错误 | Compiler Architect、Domain Expert |
| L3 | 警告带错误码（E0001 用于 severity:warning） | AI Engineer |
| L4 | `validate_design` 通过字符串内容进行错误分类（脆弱） | Compiler Architect |
| L5 | `@default-theme` 存根但无消费者；文档与实际行为不匹配 | Domain Expert |
| L6 | 重复名称 `VariantIR`（设计变体 vs 标记联合变体）在同一 IR 层 | Domain Expert |
| L7 | `resolve_token_ref` 忽略其 token 映射参数；文档承诺从未发生的查找 | Domain Expert |
| L8 | 仓库杂物：跟踪的机器产物、空文件、路径错误的转储文件 | Maintainability、Pragmatic Enforcer、Systems Architect |
| L9 | 7 个低活跃度后端的 9 倍重复 `DESIGN_BLOCK_WARNING` 守卫 | Pragmatic Enforcer |
| L10 | ADR 粒度漂移（2 行修复获得 200+ 行 ADR） | Pragmatic Enforcer |
| L11 | 误导性的"使用点号表达式语法"诊断产生 `name.name` | Domain Expert |
| L12 | Poiesis CardsEditor 22 个透传 props + 跨 VariantEditor/ClassEditor 的重复辅助函数 | TypeScript/React Specialist |
| L13 | `loadOmniFiles` 在 App.tsx 中是 ~300 行单体，不可测试 | TypeScript/React Specialist |
| L14 | Poiesis token 编辑器仅为静态（E3 对齐风险） | TypeScript/React Specialist |
| L15 | `__omni_forHandlers` 全局注册表从不清理（有上限的泄漏） | TypeScript/React Specialist |
| L16 | Design-System 目录是孤岛，零适配器（三个 token 表示，零同步） | Systems Architect |

---

## 跨领域主题

5 个主题出现在 3+ 位专家的评审中：

### 主题 1: 静默数据丢失发生在合约边界（6 位专家）
**表现**: C1（lowering 后诊断丢弃）、C2（合并中清理修复丢失）、H1（非 OmniView 构建产生零文件）、H3（validateSources 死代码）、H5（Poiesis design.omni 往返破坏）、H12（FFI 合约漂移）

**模式**: 信息在一个上下文中产生（lowering 诊断、主题覆盖、热重载清理），在边界被静默删除，没有信号表明任何事情出错。

**系统修复**: 在合约边界添加"fail-loud"断言——如果诊断在 lowering 后丢失，则使用管道集成测试；如果往返产生不同的 IR，则使用 Poiesis 保存门控断言；如果嵌入式工件与其源码时间戳不匹配，则使用 CI 新鲜度检查。

### 主题 2: 嵌入式工件新鲜度（3 位专家）
**表现**: H2（动画 dist 3 次提交陈旧）、M5（37 文件缓存未命中）、Pragmatic Enforcer 的 M3（15MB 已提交构建产物）

**模式**: `include_str!` 打包已提交的 `dist/`，无人在重构后重建。包自身的测试通过了——它从 `src/` 导入。编译器的测试通过了——它不验证嵌入式内容。

**系统修复**: CI 新鲜度门控：如果 `dist/` 早于其源文件则构建失败；`omnilang/build.rs` 中的 `cargo:rerun-if-changed` 指令。

### 主题 3: 编辑器-编译器鸿沟（4 位专家）
**表现**: H5（Poiesis 不能表示主题/动态 token）、H7（编辑器与编译器的 token 类型枚举不同，14 vs 13）、H8（诊断路径绕过增量缓存）、L14（仅静态 token 编辑器）、H3（验证门控死代码）

**模式**: Poiesis 的 TS 管线是在 Phase 1/2 动态 token 工作**之前**构建的。每个新的编译器 token 特性在编辑器中产生一个差距——有些已记录（E3），有些是意外的（H5 主题破坏，H7 font 类型不匹配）。

**系统修复**: 编译器 IR（`TokenValueIR`、`DesignIR`）与编辑器 `DesignToken` 模型之间的往返测试。当编译器支持编辑器无法表示的构造时，编辑器必须拒绝重写（保留磁盘上的文件）。

### 主题 4: 验证基础设施低于项目规模（3 位专家）
**表现**: C1 测试仅直接调用 `lower()`、H8 仅诊断却运行完整代码生成、M6 仅单行编译器基准、M11 clippy 不干净、H11 React E2E 死亡、M8 7/9 后端在 CI 中从未编译

**模式**: 测试套件验证组件，而非管线。每个阶段独立测试并通过——但端到端路径（pipeline、诊断、跨后端编译）具有在单元测试中不可见的差距。

**系统修复**: 管线集成测试（`run_pipeline` → 断言诊断）、跨后端烟雾测试（所有 9 个后端的 `cargo check`）、基于语料库的性能基准、每个后端的快照新鲜度。

### 主题 5: ADR 治理漂移（2 位专家——但影响是系统性的）
**表现**: C2（ADR-051 证明不存在的修复）、L10（ADR 粒度漂移）、M14（无结构化审查指标）、Pragmatic Enforcer 关于 ADR 开销的观察

**模式**: ADR 作为事后文档编写，而非作为决策记录。当 ADR 的验收标准引用分支状态而非合并后主分支时，ADR 就失去了作为真相来源的可靠性。

**系统修复**: ADR 中合并后验证清单；针对 HEAD 而非分支进行验证；为低于 50 行的修复引入轻量级修复日志，以减少 ADR 膨胀。

---

## 权衡与分歧

### 分歧 1: 非 OmniView 目标——警告 vs 错误 vs 零输出
- **Implementation Strategist + Systems Architect**: A2 修复（返回零文件）是退步——比修复前的行为（组件编译，设计被丢弃）更糟糕
- **ADR-051 的意图**: 当设计块编译到非 OmniView 目标时发出警告——**而非**抑制所有输出
- **实际情况**: 9 个提前返回丢弃**一切**
- **解决**: 当前的零输出行为是一个 bug，而非有意的权衡。恢复"警告并继续"。

### 分歧 2: 静态 HTML 无 fallback 用于动态 token（已验证的有意设计）
- **ADR-049**: 动态 token 无静态 HTML fallback——有意的设计决策
- **评审**: 零位专家将其标记为问题。设计意图是清晰的并已接受。

### 分歧 3: 审查频率与审查价值
- **Pragmatic Enforcer**: 审查过程本身已成为最大的边界成本中心。50 ADR（10,331 行）对应 ~90K 行代码。每修复一个 ADR 是不可持续的。
- **AI Engineer**: 过程**确实**有自我知识——记录的失败率、Layer 2 验证、三桶分类（ADR-029）——但这些指标应被结构化，而非轶事化。
- **解决**: 保留审查过程，但添加轻量级修复日志层；结构化审查指标；合并后验证 ADR 验收标准。

---

## 优先级建议

### 立即（本次 Sprint，~1-2 天）

| # | 操作 | 工作量 | 修复内容 |
|---|------|--------|---------|
| 1 | 修复 lowering 后诊断丢弃（在 `lower()` 之后排空；重新计算 `has_errors`） | S | C1 |
| 2 | 重新应用 `__omni_tokenEffectCleanup` 清理守卫 | S | C2 |
| 3 | 恢复非 OmniView 目标的"警告并继续"；在 `omnic build` 中打印警告 | S-M | H1 |
| 4 | 重建 `packages/omni-animation/dist/` 并提交；添加 CI 新鲜度门控 | S | H2 |
| 5 | 修复 Poiesis `validateSources` 包装展开 + 删除死代码分支 | S | H3 |
| 6 | 修复解析器栈溢出（将递归守卫移入 `parse_expr_bp`） | S | H10 |

### 短期（1-2 个 Sprint，~1 周）

| # | 操作 | 工作量 |
|---|------|--------|
| 7 | 修复入口文档（README 结构/数量、stdlib 路径、示例项目、CLI 命令） | M |
| 8 | 守卫 Poiesis design.omni 往返（拒绝重写不可表示的构造） | M |
| 9 | 修复 `@default-theme` no-op（尊重或删除） | S |
| 10 | 从编辑器联合中移除 `"font"` token 类型 | S |
| 11 | 修复 WASM 诊断以使用仅缓存路径（`analyse_multi_cached`） | S |
| 12 | 修复 CSS 消毒器结构绕过（拒绝 `}` / `</`，完全转义解码） | S |
| 13 | 删除推测性 `parse_stmts` 存根链 + `parseOpsWasm` | S |
| 14 | 将 `MAX_CACHE_ENTRIES` 提升至 500 或按项目大小缩放 | S |
| 15 | 恢复 Clippy 门控：门控测试模块，删除死代码，`clippy --fix` | S |
| 16 | 为设计 token 诊断添加 span（`TokenDef` 上的 `source_span`） | M |

### 长期（待办事项，~1 个月）

| # | 操作 | 工作量 |
|---|------|--------|
| 17 | React 后端设计 CSS 发射（token + 类 + 变体） | M |
| 18 | 修复 React E2E 工具（生成脚本，输出放置，规范更新） | M |
| 19 | 删除 C FFI 或记录外部合约 | S-M |
| 20 | 仓库清洁：已提交产物、遗留 Python 编译器、空文件 | M |
| 21 | 将 WASM FFI 迁移到 `#[derive(Serialize)]` 结构体；删除陈旧的 TS 声明 | M |
| 22 | 重新启用 CI 中的 `cargo check --features all-backends` | S |
| 23 | 添加语料库基准（完整 / 无变化 / 1 文件更改 / 仅诊断） | M |
| 24 | 合并后 ADR 验证步骤 + 轻量级修复日志层 | S |
| 25 | Poiesis E3：编辑器中的动态 token 表达式 | M |
| 26 | 带 `include_str!` 的 `cargo:rerun-if-changed` 的 CI 新鲜度门控 | S |
| 27 | 提取共享 design-editor-utils（`segmentsToString` 等）；减少 CardsEditor props | M |

---

## 如果未解决的风险

1. **静默无效输出交付给用户**：C1 + H1 意味着编辑器显示"无错误"，而 CLI 产生损坏的 CSS（`var(--missing)`）和空构建——信任编译器输出的用户将把损坏的产物部署到生产环境。最严重的项目级风险。

2. **合并回退成为永久性的**：C2 证明 ADR 可以认证不存在的行为。如果不修复，整个 ADR 语料库作为"真相来源"的可信度会下降——未来的审查将重复争论已解决的决策。

3. **令牌生态系统分裂**：H5 + H7 + H9 + L14 + L16 共同创建了四种不兼容的 token 表示。下一个 rebrand 或 token 模式变更需要 4 倍的高风险手动同步。

4. **WASM 编译器崩溃在生产环境中**：H10（解析器栈溢出）在编辑器中只需 300 字节的输入即可终止 WASM 实例。H8（诊断时的完整代码生成）随着项目增长呈线性扩展。它们结合在一起，构成了一个可靠性回归，只有通过浏览器分析才能发现。

5. **跨后端声明侵蚀**：7/9 后端在 CI 中从未编译（M8）。每个 IR 变更都会扩大差距——当下游消费者需要 Vue 或 Flutter 目标时，工作将是一次迁移，而非一次构建。

6. **入口文档腐烂**：H4 意味着新用户在步骤 2 失败（无效导入、不存在的项目、损坏的 CLI 命令）。强大的参考文档从未被触及——但入职是项目增长的第一道门。

---

## 已保留的有意设计（未标记）

以下发现在审查中被识别，但**明确不进行修复**——它们与已有的 ADR 决策一致：

| 发现 | ADR | 设计意图 |
|------|-----|---------|
| 主题单向同步 | ADR-044 L83 | Store 是真相的唯一来源 |
| 二元操作符不在表达式范围内 | ADR-047 L137 | "非完整表达式系统…此乃有意为之" |
| `var(--name)` 优雅降级 | ADR-047 L128 | 宽容优于崩溃 |
| Unknown 豁免 | ADR-050 L159-162 | 拒绝 Unknown 会破坏所有跨文件 const 导入 |
| 效果合并权衡 | ADR-050 L109 | 颗粒度损失可接受（setProperty 无操作） |
| 动态 token 无静态 HTML fallback | ADR-049 | 有意的设计决策 |
| `is_css_value()` 启发式 | ADR-050 | 已文档化，无结构性修复计划 |
| 无批处理/无压缩/内联运行时 | ADR-028 | 14 项有意的调试运行时权衡 |

---

## 成功指标与跟进计划

**合并前门控**（在合并本评审的任何修复之前）:
- [ ] `cargo test --workspace` — 554 omnilang-core + 161 omnilang + poiesis（零失败）
- [ ] `cargo clippy --workspace -- -D warnings`（恢复零警告门控）
- [ ] `cargo fmt --check --all`
- [ ] 针对 E0005/E0006/E0011 的管线集成测试通过（防止 C1 再次发生）
- [ ] `__omni_tokenEffectCleanup` 回归测试存在于 `design_codegen_test.rs` 中

**短期跟进**:
- [ ] 重新构建并提交 `packages/omni-animation/dist/`
- [ ] 添加 CI 新鲜度门控：如果 dist 早于 src，则构建失败
- [ ] 修复所有 4 个入口文档（README、QUICKSTART、stdlib 路径、示例项目）
- [ ] 展开 Poiesis `validateSources` 包装 + 添加 vitest
- [ ] 添加 ADR 合并后验证清单

**长期跟进**:
- [ ] 结构化审查指标注册表（`review-metrics.md`）
- [ ] 轻量级修复日志层（低于 ADR 的修复）
- [ ] 语料库性能基准
- [ ] 跨后端编译在 CI 中（`--features all-backends`）
- [ ] Poiesis E3：编辑器中的动态 token 表达

---

## 方法论说明

12 位专家并行审查了工作空间。每位专家都给出了：简要说明（扫描仓库结构、近期提交、ADR 和已推迟项），其专业领域内的具体关注点，以及关于**不要**标记什么为问题的明确指示（ADR 对齐的有意设计、已知已推迟项）。所有 CRITICAL/HIGH 发现均由审查者独立进行经验证（针对实际二进制文件编译、在浏览器中运行，或对照合并历史进行 git 验证）。所有交叉领域主题在 3+ 位专家趋同后被确认。

**每位专家的逐节完整评审**见下方附录。

---

## 附录 A: 每位专家的完整评审

*注：以下 12 节均为此轮评审中每位专家的完整、逐字输出。*

### A.1 Systems Architect
> 见上方的完整 Systems Architect 评审报告（CRITICAL 1 + HIGH 2, 3 + MEDIUM 4, 5 + LOW 6, 7）。

### A.2 Domain Expert
> 见上方的完整 Domain Expert 评审报告（HIGH 1, 2, 3 + MEDIUM 4, 5 + LOW 6, 7, 8 + 设计说明）。

### A.3 Security Specialist
> 见上方的完整 Security Specialist 评审报告（MEDIUM C1-C4 + 建议）。

### A.4 Maintainability Expert
> 见上方的完整 Maintainability Expert 评审报告（HIGH 1 + MEDIUM 1-4 + LOW 1）。

### A.5 Performance Specialist
> 见上方的完整 Performance Specialist 评审报告（HIGH 1 + MEDIUM 1-3 + LOW 1-3）。

### A.6 Implementation Strategist
> 见上方的完整 Implementation Strategist 评审报告（HIGH 1, 2 + MEDIUM 3, 4 + LOW 5, 6）。

### A.7 AI Engineer
> 见上方的完整 AI Engineer 评审报告（HIGH-1 + MEDIUM-1..3 + LOW-1）。

### A.8 Pragmatic Enforcer
> 见上方的完整 Pragmatic Enforcer 评审报告（MEDIUM 1-3 + LOW 1-2 + 观察）。

### A.9 Rust Expert
> 见上方的完整 Rust Expert 评审报告（HIGH 1 + MEDIUM 1-5 + LOW 1-2）。

### A.10 TypeScript/React Specialist
> 见上方的完整 TypeScript/React Specialist 评审报告（HIGH 1, 2 + MEDIUM 3-5 + LOW 6-8）。

### A.11 Compiler Architect
> 见上方的完整 Compiler Architect 评审报告（CRITICAL 1 + MEDIUM 2 + LOW 3, 4 + 设计说明 5）。

### A.12 Documentation Expert
> 见上方的完整 Documentation Expert 评审报告（HIGH 1-3 + MEDIUM 1-4 + LOW 1-3）。

---

**评审结束** | 聚合者: Claude | 日期: 2026-07-31
