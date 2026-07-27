# 架构评审实现计划（修订版）

> **来源**: `.architecture/reviews/full-workspace-review-2026-07-26.md`
> **修订依据**: `.architecture/reviews/plan-review-2026-07-26.md`（3 专家评审）
> **创建**: 2026-07-26 | **修订**: 2026-07-26
> **状态**: 计划中
> **总项目数**: 32 条（原 37，移除 1 + 推迟 4）
>
> **术语说明**: 参见 [ADR-005](../decisions/adrs/ADR-005-ts-pipeline-parser-emitter-not-compiler.md)。
> 本文中"TS 解析器"/"TS 管线"均指编辑器往返翻译层（Parser + SourceEmitter），非编译器。

---

## 一、已豁免/已确认项目

| 编号 | 原始建议 | 决策 | 理由 |
|------|---------|------|------|
| P1 | 缩小 FS 权限到 `$APPDATA` | ❌ **豁免** | 编辑器必须全文件系统访问，见 [[poiesis-tauri-security-model]] |
| P15 | 移除 `all-backends`、推迟 6 目标 | ✅ **已确认** | P0=OmniView+React+Flutter，其余 feature-gated 保留，见 [[omnilang-codegen]] |

## 已推迟/移除项目

| 编号 | 项目 | 决策 | 理由 |
|------|------|------|------|
| P34 | AI 代码补全 | ✂️ **移除** | 新功能非债务偿还；3/3 专家一致；Ratio 4.0；另立专项计划 |
| P35 | Crate 拆分评估 | ⏸️ **推迟** | YAGNI — 等编译器 >100K 行且有 5+ 活跃后端时再评估 |
| P36 | 代码生成并行化 | ⏸️ **推迟** | 需 profiling 数据证明 codegen 是瓶颈（当前无数据） |
| P39 | IR 优化传递 | ⏸️ **推迟** | 优化前需 motion 测试达到 95% 分支覆盖率 |
| P38b | Strip TAURI_INTERNALS | ⏸️ **推迟** | 防御性安全——等 iframe 沙箱（P38）先生效再评估必要性 |

---

## 二、依赖关系图（修订版）

```
Sprint 1 — 全部独立并行
  P3  P7  P10  P11  P12  P13  P14  P15  P20  P21  P22  P23  P25  P26  P5a
                                                                        ↑
Sprint 2                                                               Golden 文件收集
  P2 (依赖 P15)  P5b+c (依赖 P5a)  P6  P9  P4a (源映射 lowering)  [基线测量]
                                        ↓
Sprint 3                                P4a 完成
  P4b (非 OmniView 后端源映射)  P8 (增量缓存 WASM)  P27 (WASM 回退)
                                        ↓
Sprint 4                                P8 缓存键依赖 source_map
  P19 (参数化测试框架) → P24 (TypeKind 递归类型) → P17 (TS round-trip 测试)
   ↑                         ↑ 依赖 P15(仅 P0 后端)  ↑ 复用 P5 Golden 文件
   └── 必须在类型重构之前

Sprint 5
  P16 (架构回溯文档)  P18 (拆分 lib.rs)  P28-ir (ir/lower unwrap ~80 处)
  P29 (TS 10% 覆盖率)  P31 (统一 CompileOutput)
                              ↑ 依赖 P19 (测试保护)

Sprint 6
  P28-codegen (codegen unwrap ~85 处)  P30 (WASM 解析器导出+评估)
  P32 (跨目标测试框架)  P33 (IR source_span)  P38 (iframe 沙箱)
  依赖: P5+P17 → P30, P19+P31 → P32, P4 → P33, P2 → P38
```

> 注：P2（调试窗口 capability）向 lib.rs 添加代码，P18（Sprint 5）重构 lib.rs。
> **协调机制**：P2 实现时在 lib.rs 中添加清晰的模块边界注释（`// P18: this belongs in commands/debug.rs`），简化 Sprint 5 的分发工作。

---

## 三、阶段划分

### 阶段 1：快速胜利（Sprint 1，第 1-2 周）

**目标**：消除所有 S 级工作量项，建立安全基线，清理死代码，启动语法漂移缓解。

#### 第 1 周：安全 + 关键 Bug 修复

**Day 1-2：Rust 侧基础修复**

| 任务 | 来源 | 文件 | 验证方式 |
|------|------|------|---------|
| P3: `open_project` 规范化失败返回 Err | Security C4 | `poiesis/src-tauri/src/lib.rs:502-506` | `cargo check` + 手动测试无效路径 |
| P10: `dw_log` 使用 `std::env::temp_dir()` | Rust H1 | `poiesis/src-tauri/src/lib.rs:770` | Windows 上验证日志文件生成 |
| P11: 提取诊断转换 helper | Rust H3 | `lib.rs:117-156` + `ffi.rs:120-146` | 4 处调用点编译通过 |
| P14: IPC 命令输入大小限制 | Security C5 | `lib.rs:268,611` | 超大小输入返回错误 |
| P26: `.mcp.json` 使用 `victauri bridge` | Security C9 | `.mcp.json:13` | `get_plugin_info` 连接成功 |

**Day 3-4：前端基础修复**

| 任务 | 来源 | 文件 | 验证方式 |
|------|------|------|---------|
| P7: Monaco 代码分割 + chunkSizeWarningLimit→200 | Perf C1 | `poiesis/vite.config.ts:24-33` | `pnpm build` 检查产出块 |
| P12: 修复 4 处 `as any` | TS/React H | `store/index.ts:94`, `App.tsx:378`, `slices/theme.ts:24`, `store/index.ts:80` | `tsc --noEmit` 零错误 |
| P13: 空 catch 块替换为 logger.error | TS/React H | `DesignPage.tsx:118,129` | ESLint 不再警告空 catch |
| P23: HMR 状态仅存数据（排除函数 + 撤销堆栈） | TS/React M | `poiesis/src/store/index.ts:77-88` | HMR 后 action 函数引用正确 |
| P21: 删除 8 个空 `.claude/agents/*` | AI Eng H | `.claude/agents/` | 目录为空或文件有内容 |
| P22: 清理 brainstorm state + 更新 `.gitignore` | AI Eng M | `.superpowers/brainstorm/` | `git status` 无 stale 文件 |

**Day 5：编译器基础设施 + 语法漂移缓解**

| 任务 | 来源 | 文件 | 验证方式 |
|------|------|------|---------|
| P15: 移除 `all-backends` 默认流 | Pragmatic | `omnilang/Cargo.toml:40-50` | `cargo test -p omnilang` 通过（仅 P0 后端） |
| P20: CI 添加 `cargo audit` + `npm audit` | Security C8 | `.github/workflows/` | CI 绿 |
| P25: `#[non_exhaustive]` 到 `ExprIR` 等枚举 | Impl Strat M | `omnilang/src/ir/types.rs` | 编译——缺失 match 分支现在报错 |
| **P5a**: 收集 20-30 个代表性 `.omni` Golden 文件 | Domain C-1 | `tests/fixtures/golden/` | 覆盖主要语言特性（view/store/signal/motion/@for/@if） |

**Sprint 1 完成标准**：
- [ ] 所有 S 级项目通过
- [ ] `cargo check` + `tsc --noEmit` 零错误
- [ ] CI 包含 cargo audit + npm audit
- [ ] all-backends 从默认流中移除
- [ ] Golden 文件集已收集，覆盖全部主要语言特性
- [ ] **所有现有测试继续通过**（`cargo test` + `pnpm test`）

---

### 阶段 2：测试基础（Sprint 2，第 3-4 周）

**目标**：建立双解析器同步 CI、OmniView 回归测试、修复源映射管线（CRITICAL）。

#### 第 3 周：解析器同步 + 测试骨架

| 任务 | 来源 | 文件 | 依赖 |
|------|------|------|------|
| P5b: TS 侧 Golden 文件解析测试 | Domain C-1 | `poiesis/src/services/codegen/__tests__/golden.test.ts` | P5a |
| P5c: CI 步骤——Rust 解析器也跑同一组 Golden 文件 | Systems C1 | `omnilang/tests/golden_parse.rs` + CI | P5a |
| P9a: 分析 OmniView motion/timeline/@for 代码路径 | Impl Strat C | `omnilang/src/codegen/omniview/motion.rs`, `timeline.rs`, `list_render.rs` | 无 |
| P9b: 编写 motion 回归测试（至少 5 个用例） | Impl Strat C | `omnilang/src/codegen/omniview/tests_regression/motion.rs` | P9a |
| [基线] 测量当前 20 文件 `.omni` 项目的 WASM 编译延迟 | Perf H2 | `.architecture/metrics/wasm-baseline.md` | 无——P8 需要比较基准 |

#### 第 4 周：安全加固 + 源映射修复

| 任务 | 来源 | 文件 | 依赖 |
|------|------|------|------|
| P6a: 调研 Vite HMR 与 nonce CSP 兼容性 | Security C2 | 调研文档 | 无 |
| P6b: 实现 nonce-based CSP（若可行）或记录阻塞原因 | Security C2 | `poiesis/src-tauri/tauri.conf.json:22` | P6a |
| P2a: 设计调试窗口最小 capability（仅 `core:default`） | Security C3 | `poiesis/src-tauri/capabilities/debug-window.json` | P15 |
| P2b: 创建独立调试 webview + 绑定新 capability | Security C3 | `poiesis/src-tauri/src/lib.rs` (run 函数) | P2a |
| **P4a**: 实现 lowering 传递——从 AST 跨度填充 `IRProgram.source_map` | Comp Arch C | `omnilang/src/ir/lower/mod.rs:198` | 无（弱依赖 P33 方向） |
| P9c: 编写 timeline + @for 回归测试（至少 5 个用例） | Impl Strat C | `tests_regression/timeline.rs`, `tests_regression/list.rs` | P9a |

> **P2↔P18 协调**：P2 向 lib.rs 添加调试窗口代码时，用 `// P18(Sprint 5): move to commands/debug.rs` 注释标注模块边界。

**Sprint 2 完成标准**：
- [ ] Golden 文件双解析器 CI 测试通过（至少 20 个 fixture 文件）
- [ ] OmniView motion/timeline/@for 至少有 10 个回归测试
- [ ] CSP 已加固或阻塞原因已文档化
- [ ] 调试窗口有独立 capability
- [ ] `IRProgram.source_map` 在 OmniView 路径中非空
- [ ] WASM 编译基线延迟已记录
- [ ] **所有现有测试继续通过**

**Week 3 检查点**：Golden 文件 CI 是否双解析器通过？→ 不放行 P17 除非通过

---

### 阶段 3：编译器基础设施（Sprint 3，第 5-6 周）

**目标**：完成源映射管线、接入增量缓存、添加 WASM 回退路径。

| 任务 | 来源 | 文件 | 依赖 |
|------|------|------|------|
| P4b: 修改非 OmniView 后端的 `compile_to_target_json` 包含源映射 | Comp Arch C | `omnilang/src/lib.rs:356` | P4a |
| P8a: 创建 `analyse_multi_cached` 的 WASM 导出 | Perf H2 | `omnilang-wasm/src/lib.rs` | P4（缓存键需 source_map） |
| P8b: JS 端 `rustCompiler` 单例存储 `IncrementalCache` 实例 | Perf H2 | `poiesis/src/commands/rustCompiler.ts` | P8a |
| P8c: 将 `resolveStylesMulti` 路由到缓存变体 | Perf H2 | `poiesis/src/components/design/useWasmCompile.ts:134` | P8b |
| P27: WASM→Tauri 编译回退路径 | Systems C4 | `poiesis/src/commands/compile-api.ts` | 无 |

> **注**：Sprint 3 有零松弛——若 P4b 或 P8 超出估算，第 6 周末设置检查点决定是否调整 Sprint 4。

**Sprint 3 完成标准**：
- [ ] 所有 P0 后端（OmniView/React/Flutter）输出包含源映射
- [ ] WASM 增量缓存生效（多文件项目延迟降低，与基线对比）
- [ ] WASM 不可用时自动回退到 Tauri 编译路径
- [ ] **所有现有测试继续通过**

---

### 阶段 4：测试保护 + 类型重构（Sprint 4，第 7-8 周）

**目标**：先建立参数化测试框架，再重构 IR 类型系统，最后补齐 TS 管线测试。

| 任务 | 来源 | 文件 | 依赖 |
|------|------|------|------|
| P19: 参数化后端回归测试（rstest + 共享 runner） | Maint H | `omnilang/src/codegen/tests_regression/` | P15（仅 P0 后端） |
| **P24**: `TypeKind::Named` → 递归 `type_args: Vec<TypeKind>` | Comp Arch M | `omnilang/src/ir/types.rs:271-336` | **P19（测试保护）** |
| P17: TS 管线 round-trip 测试（编辑数据→.omni→解析→比较） | Domain M | `poiesis/src/services/codegen/__tests__/` | P5（Golden 文件可复用） |

> **排序逻辑**：P19（测试框架）→ P24（类型重构，通过 P19 验证 3 个 P0 后端）→ P17（利用已有的 Golden 文件和测试基础设施）。
> 这是 plan-review 发现的关键修正——原计划将 P24 放在 P19 之前，没有测试保护。

**Sprint 4 完成标准**：
- [ ] P0 三个后端的回归测试通过参数化 runner 运行
- [ ] TypeKind 支持递归泛型参数（如 `Array<Optional<String>>`）
- [ ] TS round-trip 测试至少捕获到 1 个解析器差异（证明测试有效性）
- [ ] **所有现有测试继续通过**

**Week 5 检查点**：TS round-trip 测试就绪并捕获到差异？→ 不放行 P30 除非通过

---

### 阶段 5：债务偿还 + 架构对齐（Sprint 5，第 9-10 周）

**目标**：修复错误处理、统一 CompileOutput、模块化 Tauri 后端、正式化架构知识。

| 任务 | 来源 | 文件 | 依赖 |
|------|------|------|------|
| P16: 编写 1 篇架构回溯文档（覆盖 4-5 个关键决策的叙事性记录） | Impl Strat M | `.architecture/retrospective.md` | 无（读记忆文件） |
| P18: 拆分 `lib.rs` → `state.rs` + `commands/` + `path.rs` | Maint M | `poiesis/src-tauri/src/` | P3+P10+P11（Sprint 1 已修 bug） |
| **P28-ir**: 替换 `ir/lower/` 中 unwrap → 诊断产生式（约 80 处） | Maint H | `omnilang/src/ir/lower/` | 无（机械 + 上下文感知） |
| P29: TS 管线 10% 测试覆盖率 | Pragmatic | `poiesis/src/services/codegen/__tests__/` | P17（round-trip 是覆盖率主力） |
| P31: 统一 `CompileOutput`——提升 `source_map`/`warnings` | Comp Arch H | `omnilang/src/codegen/mod.rs` | P19（测试保护） |

> **P28 拆分说明**：ir/lower/ 的 unwrap 约 80 处，是编译器核心路径——优先替换。codegen/ 的 unwrap 约 85 处，在 Sprint 6 处理。

> **P16 简化说明**（根据 plan-review）：1 篇叙事性架构回溯文档替代 5-7 篇模板化 ADR。
> 覆盖：编译器管线设计、CodeCard→Monaco 编辑器演进、9→3 目标决策、Deferred JS 机制、indigo→terracotta 迁移。
> 新增决策（P31 CompileOutput 格式、P18 模块拆分）在决策时即时写简短 ADR。

**Sprint 5 完成标准**：
- [ ] ir/lower/ 中 unwrap 减少至 < 10
- [ ] OmniView 通过标准 `Codegen` trait 对外暴露（不再单独返回 `OmniViewOutput`）
- [ ] `lib.rs` < 200 行（模块分发到 state.rs + commands/ + path.rs）
- [ ] TS 管线测试覆盖 ≥ 10%
- [ ] 架构回溯文档已写入
- [ ] **所有现有测试继续通过**

---

### 阶段 6：战略决策（Sprint 6，第 11-12 周）

**目标**：完成 codegen unwrap 替换、解析器统一评估、IR 源位置、沙箱隔离、跨目标测试框架。

| 任务 | 来源 | 文件 | 依赖 |
|------|------|------|------|
| **P28-codegen**: 替换 `codegen/` 中 unwrap → 诊断产生式（约 85 处） | Maint H | `omnilang/src/codegen/` 各后端 | 无 |
| P30a: 实现 `omnilang::parser::parse()` 的 WASM 导出 | Comp Arch H | `omnilang-wasm/src/lib.rs` | P5+P17（先有测试套件） |
| P30b: 评估替换可行性——TS 解析器调用方统计 + 迁移成本 | Comp Arch H | 调研文档 | P30a |
| P32: 跨目标正确性测试框架 | Comp Arch M | `omnilang/src/codegen/tests_regression/` | P19+P31 |
| P33: 所有 IR 节点添加 `source_span: Option<Span>` | Comp Arch M | `omnilang/src/ir/types.rs` | P4（源映射思路） |
| P38: 编译输出 iframe 沙箱（`sandbox` 属性） | Security C3 | `poiesis/public/debug-window/` | P2（capability 隔离） |

> **注**：Sprint 6 中的所有 L 级项目（P30 解析器替换、P32 跨目标测试框架）包含**实施 + 评估**；
> 解析器替换是可行性评估（含 WASM 导出原型），非完整迁移。
> 若 P5 或 P17 已延迟，P30 延期到 Phase 2——"保留双解析器"本身是有效的决策结果。

**Sprint 6 完成标准**：
- [ ] codegen/ 中 unwrap 减少至 < 20
- [ ] 解析器统一决策文档：WASM 替换 vs 保留双解析器（含性能和迁移成本数据）
- [ ] 跨目标测试框架可运行（至少 OmniView vs React 的一致性检查）
- [ ] IR 节点带 source_span 字段（`#[serde(default)]` 向后兼容）
- [ ] 调试预览在 iframe 沙箱中执行（`sandbox="allow-scripts"`）
- [ ] **所有现有测试继续通过**

**Week 9 检查点**：IR source_span 在 P0 后端输出中可见？→ P32 依赖此项

---

## 四、里程碑总结

```
Week 1-2  ████████  Sprint 1: 快速胜利 (14 S-items + P5a 收集)
Week 3-4  ████████  Sprint 2: 测试基础 + 源映射修复 (7 项)
Week 5-6  ████████  Sprint 3: 编译器基础设施 (3 项, 4-6 天)
Week 7-8  ████████  Sprint 4: 测试保护 → 类型重构 (3 项, 9-12 天)
Week 9-10 ████████  Sprint 5: 债务偿还 + 架构对齐 (5 项)
Week 11-12████████  Sprint 6: 战略决策 (6 项)
```

| 里程碑 | 截止 | 关键交付物 |
|--------|------|-----------|
| M1: 安全基线 | Week 2 | CSP 加固、调试窗口隔离、路径验证修复、cargo audit CI、Golden 文件集 |
| M2: 测试骨架 | Week 4 | 双解析器 CI 通过、OmniView motion 10+ 回归测试、源映射 OmniView 路径 |
| M3: 编译器健康 | Week 6 | 源映射全覆盖 P0 后端、WASM 增量缓存生效、WASM→Tauri 回退 |
| M4: 类型+测试 | Week 8 | TypeKind 递归类型（测试保护下）、参数化测试框架、TS round-trip 测试 |
| M5: 债务清零 | Week 10 | ir/lower unwrap→诊断、CompileOutput 统一、lib.rs 模块化、架构回溯文档 |
| M6: 战略就绪 | Week 12 | codegen unwrap 完成、解析器统一决策、IR source_span、iframe 沙箱、跨目标测试 |

---

## 五、工作量估算（修订版）

| 阶段 | 项目数 | 估计范围 | 10 天 Sprint 可行性 |
|------|--------|---------|-------------------|
| Sprint 1 | 14 | ~4-6 天 | ✅ 宽裕 |
| Sprint 2 | 7 | ~8-11 天 | ⚠️ 临界——P9 测试编写可能溢出 |
| Sprint 3 | 3 | ~4-6 天 | ✅ 宽裕——含缓冲 |
| Sprint 4 | 3 | ~9-12 天 | ⚠️ 临界——P24 类型重构可能溢出 |
| Sprint 5 | 5 | ~9-12 天 | ⚠️ 临界——P28-ir 可能比预期复杂 |
| Sprint 6 | 6 | ~11-14 天 | ⚠️ 临界——P30a WASM 导出有不确定性 |
| **合计** | **32** | **~56-72 天** | — |

> 注：估算范围反映了 plan-review 发现的 1.5-2x 低估修正。临界 Sprint 需在阶段中期灵活调整优先级。
>
> 所有 Sprint 6 的 L 级项目（P30 解析器替换评估、P32 测试框架）包含实施——非纯评估。
> 若 P5 或 P17 已延迟，P30 延期到 Phase 2，产出"保留双解析器"的明确决策文档。

---

## 六、风险跟踪（修订版）

| 风险 | 影响项目 | 缓解措施 |
|------|---------|---------|
| nonce CSP 与 Vite HMR 不兼容 | P6 | 记录阻塞原因，延期到生产构建配置 |
| TypeKind 递归类型改动面大 | P24 | 先 P15 移除 all-backends + P19 参数化测试保护 |
| 源映射管线改动面大 | P4 | 渐进——先 OmniView（Sprint 2），再扩展到其他后端（Sprint 3） |
| WASM 增量缓存在大项目中正确性未验证 | P8 | Sprint 2 先记录基线延迟，Sprint 3 实现后对比验证 |
| TS 解析器到 WASM 迁移性能不达标 | P30 | Sprint 2 记录基线，Sprint 6 评估时对比 |
| Sprint 4 P24 超出估算 | P17, P31 | 最小可行范围——P0 三个后端优先，Rust/Python 后端可选 |
| Sprint 5 P28-ir 比预期复杂 | P29, P31 | ir/lower 优先——核心编译器路径。若溢出，P29 可降为 8% |
| 计划在第 7 周被放弃 | 全部 Phase 2 | 前 4 周交付 80% 价值——即使中止，代码库已显著改善 |

---

## 七、每周站会检查点（修订版）

- **Week 1**: P3+P10+P11+P12——安全关键 Bug 必须全部修复
- **Week 2**: Golden 文件集是否覆盖主要语言特性？（≥20 个 fixture 文件）
- **Week 3**: ⚡ **关键链检查点**——Golden 文件 CI 是否双解析器通过？不放行 P17 除非通过
- **Week 4**: OmniView motion 测试是否捕获到至少 1 个真实 bug？
- **Week 5**: ⚡ **关键链检查点**——TS round-trip 测试就绪并捕获到差异？不放行 P30 除非通过
- **Week 6**: WASM 增量缓存在 20+ 文件项目中延迟降低多少？vs Sprint 2 基线
- **Week 7**: ⚡ **关键链检查点**——参数化测试框架是否覆盖 3 个 P0 后端？不放行 P31 除非通过
- **Week 8**: TypeKind 重构后 P0 后端快照测试全部通过？
- **Week 9**: ⚡ **关键链检查点**——IR source_span 在 P0 后端输出中可见？P32 依赖此项
- **Week 10**: ir/lower unwrap 减少至 < 10？CompileOutput 统一后快照测试通过？
- **Week 11**: P30a WASM 解析器导出原型可用？性能对比基线？
- **Week 12**: 解析器统一决策——WASM 替换 vs 保留双解析器（含数据）

---

## 八、推迟项跟踪

以下项目从本计划推迟，记录在案供未来 Phase 2 参考：

| 项目 | 推迟原因 | 重新评估触发条件 |
|------|---------|----------------|
| P34: AI 代码补全 | 新功能非债务；另立专项计划 | 编辑器和编译器核心稳定后 |
| P35: Crate 拆分 | YAGNI——无当前痛点 | 编译器 >100K 行 且 ≥5 个活跃后端 |
| P36: 代码生成并行化 | 无 profiling 数据证明瓶颈 | codegen >500ms（500 行 .omni 文件） |
| P39: IR 优化传递 | 优化前需 motion 95% 分支覆盖 | P9 回归测试覆盖率达到 95% |
| P38b: Strip TAURI_INTERNALS | 防御性安全——等 iframe 沙箱先生效 | P38 iframe 沙箱生效后评估 |

---

> 本计划基于 `.architecture/reviews/full-workspace-review-2026-07-26.md` 中 39 条建议，
> 经 `.architecture/reviews/plan-review-2026-07-26.md` 3 专家评审修订。
> 已移除 P34（AI 补全）；推迟 P35、P36、P39、P38b。
> 关键修正：P4a 提前、P5a 提前、P24 移到 P19 之后、P28 拆分、P23 提前、P16 简化。
>
> 每阶段结束后更新进度到本文件。
