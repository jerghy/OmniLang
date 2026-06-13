# omnilang-rs 执行计划

> **基于：** Council 四声部审议（Architect / Skeptic / Pragmatist / Critic）  
> **模式：** 阶段驱动——每个 Phase 有明确验收标准  
> **编制日期：** 2026-06-13  
> **状态：** 待批准

---

## §1 基线

### 当前 omnilang-rs 状态

| 指标 | 值 |
|------|-----|
| 总代码量 | ~9300 行 Rust（2 crates） |
| 测试数量 | 67 个 (#\[test\] 函数) |
| 测试通过 | 49 passed, 2 failed (Dart SDK 缺失), 2 ignored (Rust E2E) |
| 测试耗时 | 1.24s |
| 解析器 | 手写递归下降（无共享 grammar 文件） |
| 纯逻辑 codegen | Python ✅ / JS ✅ / Dart ✅ / Rust ✅ |
| 语义分析 pass | 6（resolver / type_checker / shadow / restrict / main_check / capture） |
| 静态分析规则 | ~7 个类别（portability / async / security / style / type / performance / ownership） |
| Module system | Stub（151 行，不可用） |
| LSP | Stub（272 行，不可用） |
| CLI | `build` / `check`（缺 fmt / watch / init / run） |

### Python OmniLang 参考实现

| 指标 | 值 |
|------|-----|
| 测试总数 | 331 |
| 纯逻辑 codegen | Python / JS / Dart / Rust（同 omnilang-rs） |
| 静态分析规则 | 25+ |
| 解析器 | Lark Earley（grammar 驱动） |
| OmniView | ✅（React L2 / Vue L2 / Dioxus L2 / Flutter L1） |

### 关键差距

| 差距 | 严重程度 | 说明 |
|------|---------|------|
| 无交叉验证 | **CRITICAL** | Rust 和 Python 解析器对同一输入可能产生不同 AST/输出 |
| Module system stub | HIGH | 多文件 .omni 项目无法编译 |
| 测试覆盖差距 | HIGH | 67 vs 331 测试，大量边界情况未覆盖 |
| LSP stub | MEDIUM | Rust 重写在 IDE 集成场景的核心价值未兑现 |
| 分析规则不足 | MEDIUM | 7 vs 25+ 规则，静态分析能力弱于 Python 版 |
| CLI 命令不完整 | LOW | 缺 fmt / run / watch / init |

---

## §2 范围决策（Council 共识）

### 做

1. **纯逻辑编译器** —— 四目标 codegen 完整对等 Python 参考实现
2. **交叉验证体系** —— 同一输入→双编译器→diff 输出，确保行为一致
3. **Module system** —— 从 stub 到可用，支持多文件项目
4. **CLI 基线** —— `omnic build / check / run`
5. **补齐分析规则** —— 从 7 类扩展到 15+ 类
6. **LSP 完善** —— v0.2 目标（Rust 在此场景价值最大）

### 不做（永久）

| 项目 | 理由 |
|------|------|
| **OmniView 移植** | Council 三方共识：UI DSL 复杂度翻倍，Python 已有成熟实现，Rust Clone-heavy AST 不适合深层 UI 树 |
| **UI codegen**（React/Vue/Dioxus/Flutter） | 属于 OmniView 范畴 |
| **Formatter**（`omnic fmt`） | Python 版已有，非 Rust 差异化价值 |
| **`omnic init / watch`** | Python CLI 已有，非核心管线 |

### 互补关系（非替代关系）

```
Python OmniLang  ──── 纯逻辑 + OmniView UI ──── 完整平台
Rust omnilang-rs ──── 纯逻辑 only ──────────── 高性能 CLI + LSP + WASM
```

两个版本**共存互补**，不互相替代。Python 版是功能完整的参考实现，Rust 版是高性能纯逻辑工具链。

---

## §3 Phase 计划

### Phase A：交叉验证基础设施（优先级最高）

**目标：** 建立自动化验证流水线，确保 Rust 编译器输出与 Python 参考实现一致。

**任务：**

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| A1 | 创建交叉验证脚本 | `scripts/verify_cross_target.py` | 对每个 fixture 执行 Python 编译→Rust 编译→diff 输出 |
| A2 | 验证 4 targets | `scripts/verify_cross_target.py` | python / js / dart / rust 四个目标各自对照 |
| A3 | 验证 AST 等价性 | `scripts/verify_cross_ast.py` | 解析后对比 AST 节点类型、数量、结构 |
| A4 | 验证语义分析等价性 | `scripts/verify_cross_semantic.py` | type_map / rename_map 对比 |
| A5 | 集成到 CI | `.github/workflows/` 或 `scripts/ci.sh` | 每次提交自动运行交叉验证 |
| A6 | 修复验证中发现的差异 | 按需修改 omnilang-rs 源码 | 逐一消除行为差距 |

**验收标准：**
- [ ] 所有 `tests/fixtures/` 下 ≥80 个 .omni 文件的 4 目标输出与 Python 参考一致
- [ ] 差异报告自动生成，标注具体文件/行号
- [ ] CI 集成：`cargo test && python scripts/verify_cross_target.py`

---

### Phase B：补齐核心管线

**目标：** 纯逻辑编译器功能对等 Python 参考实现。

**任务：**

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| B1 | Module system 实现 | `crates/omnilang-core/src/modules/` | 多文件加载、依赖图解析、循环依赖检测 |
| B2 | 补齐分析规则 | `crates/omnilang-core/src/analyze/rules/` | 从 7 类扩展到 15+ 类（参考 Python 版 25 规则） |
| B3 | 测试迁移 | `crates/omnilang-core/src/*_tests.rs` | 从 Python 331 测试中系统性迁移关键用例 |
| B4 | Rust E2E 测试恢复 | `crates/omnilang-core/src/e2e_tests.rs` | un-ignore Rust E2E，确保 Rust→rustc→运行 全链路通 |
| B5 | 错误消息对齐 | 按需修改 parser / semantic | Rust 编译器的错误消息与 Python 版一致 |
| B6 | 配置文件支持 | `crates/omnilang-core/src/config.rs` | `.omnilang.toml` 完整解析（project / targets / analyze 段） |

**验收标准：**
- [ ] Module system：`import { X } from "./lib"` 可编译多文件项目
- [ ] 分析规则 ≥15 类，每类 ≥3 个测试
- [ ] 测试数量从 67 → ≥150
- [ ] Rust E2E 全部 pass（不再 ignore）
- [ ] `omnic build multi_file_project.omni` 成功输出

---

### Phase C：CLI 完善 + 发版 v0.1

**目标：** 可发布的纯逻辑编译器 CLI。

**任务：**

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| C1 | `omnic run` 命令 | `crates/omnilang-cli/src/main.rs` | 编译→立即执行（自动检测目标运行时） |
| C2 | `--target all` 批量输出 | 同上 | 一次编译输出 4 种语言，对齐 Python CLI |
| C3 | 错误输出美化 | `crates/omnilang-core/src/errors.rs` | 彩色诊断、source context、fix suggestion |
| C4 | 版本号与帮助文本 | `Cargo.toml` + CLI | `omnic --version` / `omnic --help` 完整 |
| C5 | 发布准备 | 项目根 | README、CHANGELOG、安装说明 |
| C6 | 发布到 crates.io | — | `cargo publish` for omnilang-core + omnilang-cli |

**验收标准：**
- [ ] `omnic run hello.omni` 自动编译并执行
- [ ] `omnic build hello.omni --target all` 输出 4 个文件
- [ ] 错误信息有彩色标注、源码上下文、修复建议
- [ ] crates.io 可安装：`cargo install omnilang-cli`

---

### Phase D：LSP 服务器（v0.2）

**目标：** Rust 原生 LSP，亚秒级诊断响应，VS Code 扩展可直接调用。

**任务：**

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| D1 | LSP 协议实现 | `crates/omnilang-core/src/lsp/server.rs` | 完整的 initialize / textDocument/didChange / hover / gotoDefinition / completion |
| D2 | 增量解析 | `crates/omnilang-core/src/parser/` | 仅重新解析变更区域，保持 AST 缓存 |
| D3 | 诊断发布 | `crates/omnilang-core/src/lsp/server.rs` | 实时推送语义错误 + 分析规则警告 |
| D4 | VS Code 扩展适配 | `vscode-extension/` | 支持选择 Rust LSP 后端 |
| D5 | 性能基准 | — | 1000 行 .omni 文件首次诊断 < 200ms，增量 < 50ms |

**验收标准：**
- [ ] LSP 支持 hover / go-to-def / completion / diagnostics
- [ ] 增量更新延迟 < 50ms
- [ ] VS Code 扩展可切换 Python / Rust 后端

---

## §4 里程碑

```
Week 1-2  ██████  Phase A: 交叉验证基础设施
                   → 差异报告，CI 集成
                   
Week 3-4  ██████  Phase B: 补齐核心管线
                   → module system, 分析规则, 150+ tests
                   
Week 5    ██████  Phase C: CLI + v0.1 发版
                   → omnic build/check/run, crates.io
                   
Week 6-7  ██████  Phase D: LSP v0.2 (可选，视 v0.1 反馈)
                   → 原生 LSP, VS Code 适配
```

---

## §5 风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 交叉验证发现大量差异 | 高 | Phase B 范围膨胀 | 按差异类型分批修，优先修 CRITICAL |
| 手写解析器与 Lark grammar 持续分叉 | 中 | 长期维护成本 | Phase A 验证 harness 可捕获，但无法根治。考虑 v0.3 引入 grammar 驱动 |
| 测试迁移工作量大 | 中 | 时间翻倍 | 按优先级挑 150 个最关键用例，剩余逐步补充 |
| Rust E2E 因平台差异失败 | 低 | CI 不稳定 | 使用 Docker 统一环境 |
| 两个版本维护负担 | 中 | 团队注意力分裂 | 明确分工：Python = 功能开发 + OmniView，Rust = 纯逻辑 + 性能 |
