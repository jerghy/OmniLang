# Reference — 高级主题与故障排除

> AI 助理在执行架构操作时需要参考的深入信息和上下文。

## 务实守卫模式

### 配置（`.architecture/config.yml`）

本项目启用务实模式，配置如下：

| 参数 | 值 |
|------|-----|
| `pragmatic_mode.enabled` | `true` |
| `intensity` | `balanced`（推荐） |
| `min_complexity_score` | 5（触发挑战的最低复杂度） |
| `min_necessity_score` | 7（跳过挑战的最低必要性） |
| `max_complexity_ratio` | 1.5（复杂度/必要性 > 1.5 则强烈挑战） |

### 三种强度模式

| 模式 | 行为 | 适用场景 |
|------|------|---------|
| **strict** | 激进挑战，要求任何复杂度都有强理由 | 极简主义项目、原型阶段 |
| **balanced**（本项目） | 深思熟虑地挑战，接受有理由的复杂度 | 大多数项目 |
| **lenient** | 仅对主要复杂度新增提出关注 | 企业项目、合规要求严格 |

### 豁免项（不受务实模式影响）

- `security_critical`：认证、授权、加密、输入验证
- `data_integrity`：数据库事务、数据校验、备份策略
- `compliance_required`：GDPR、HIPAA、审计日志
- `accessibility`：WCAG 合规、屏幕阅读器支持

### 务实执行者分析格式

在 ADR 中使用的分析模板：

```markdown
## Pragmatic Enforcer Analysis

**Necessity Assessment**: X/10 — [当前需求、未来需求、等待成本、证据]

**Complexity Assessment**: X/10 — [新增复杂度、维护负担、学习曲线、依赖]

**Ratio**: X.XX（目标 < 1.5）

**Recommendation**: [✅ 批准 / ⚠️ 批准但简化 / 📋 推迟 / 🔴 建议不要]
```

---

## 子代理可靠性

### 本项目实测数据

基于 `plan-vs-reality-review-2026-07-27` 的 6-agent 审计：

| 指标 | 数值 |
|------|------|
| 实现代理失败率 | **67%**（4/6 声称已完成但未修改文件） |
| 审查代理重大误判率 | **33%**（2/6 声称未完成但实际已完成） |
| 主会话直接编辑可靠性 | **100%** |
| Node.js 脚本自动化率 | **86%**（零虚假报告） |

### 已知失败模式

1. **Grep 表层判断**——agent 根据 grep 结果断言功能存在/不存在，但未阅读实际代码确认调用链
2. **文件存在性假设**——agent 检查文件存在但未验证内容正确性（如 golden 文件 MANIFEST vs 实际文件）
3. **中间状态误判**——agent 分析的是工作进展中的中间提交，而非最终状态（如 P18 lib.rs 拆分）

### 缓解措施

1. **Layer 2 验证**——所有 Layer 1 发现必须经代码级验证（ADR-007）
2. **要求精确引用**——审查 prompt 中要求 `file:line` 格式的证据
3. **主会话交叉验证**——对 CRITICAL/HIGH 发现，主会话用 Read/Grep 直接确认
4. **纯读操作中使用子代理**，实现操作留在主会话

---

## 架构重校准

将审查发现转化为实施计划的过程。

### 本项目当前重校准状态

**Phase 1-4 实施计划**：`docs/superpowers/plans/2026-07-27-architecture-remediation.md`

**推迟项追踪**：`.architecture/deferrals.md`

| 项目 | 状态 | 计划阶段 |
|------|------|---------|
| P35 Crate 拆分 | 立即执行 | Phase 3 Task 3.1 |
| P36 Codegen 并行化 | 立即执行 | Phase 3 Task 3.2 |
| P38b TAURI_INTERNALS | 立即执行（能力收紧 P1 + XSS 链 P2） | Phase 2 Tasks 2.1-2.3 |
| P39 IR 优化 | 立即执行（MVP） | Phase 3 Task 3.3 |
| P34 AI 补全 | 继续搁置 | — |

### 创建重校准计划

```
1. 审查 → 2. 代码验证 → 3. 创建 ADR → 4. 制定计划 → 5. 更新 deferrals.md
```

---

## 故障排除

### 架构审查没有产生有用的发现

- 检查范围是否太窄（宽到全工作区或窄到单个组件）
- 确保 Layer 2 验证被执行（Layer 1 有约 10-20% 错误率）
- 检查务实模式是否过于严格（balanced 是推荐的起点）

### ADR 编号冲突

```bash
ls .architecture/decisions/adrs/ | grep -E "^ADR-[0-9]+" | sed 's/ADR-//' | sed 's/-.*//' | sort -n | tail -1
```

### 子代理反复误判

- 不要重试同一个 agent——换一种 agent 类型或直接在主会话处理
- 子代理在文件检查方面不可靠——使用 Node.js 脚本代替
- 参见上方「子代理可靠性」章节

### Clippy 警告过多

```bash
cargo clippy --fix --lib -p omnilang --allow-dirty --allow-staged  # 自动修复
cargo clippy --lib -p omnilang 2>&1 | tail -5                       # 查看剩余数量
```

### CI 未运行某些检查

本项目已知 CI 缺口：
- `cargo clippy` 未在 CI 中运行（Phase 2 Task 2.5 修复）
- `cargo fmt --check` 未在 CI 中运行（同上）
- `cargo audit` 缺少 `--deny warnings`（同上）
- eval 脚本未接入 CI

---

## 已定案决策（禁止重新争论）⚠️

> **本节是未来架构审查的必读前置内容。** 以下决策已通过 ADR 正式记录并经过多轮审查确认。**在后续架构评审中提出这些已定案问题，将被视为未阅读本文档的审查失误。** 如确实需要推翻，必须先提出新的 ADR 替代原有 ADR，而不是在评审中建议"修复"。

### 🔒 不可触碰的设计决策

| # | 决策 | ADR | 禁止建议 |
|---|------|-----|---------|
| 1 | **Tauri FS 全覆盖**（`C:/**` + `D:/**`）是编辑器设计意图 | ADR-004 §2 | ❌ 禁止建议"缩窄 FS 权限到项目目录" |
| 2 | **双解析器架构**是有效领域决策——TS 服务于编辑器领域，Rust 服务于编译器领域 | ADR-005, P30b 评估 | ❌ 禁止建议"用 WASM 替换 TS 解析器"（已评估，选择路径 C） |
| 3 | **P0 代码生成目标仅 3 个**：OmniView + React + Flutter。其余 6 个已暂停 | ADR-003 | ❌ 禁止建议"为 Vue/Dioxus/Dart 等后端添加新特性" |
| 4 | **编辑器范式 v5 Monaco** 是当前架构。v3/v4 积木编辑器已废弃 | ADR-002 | ❌ 禁止建议"恢复积木编辑器"或"评估其他编辑器方案" |
| 5 | **P34 AI 代码补全继续搁置**——编辑器和编译器核心稳定后另立专项计划 | ADR-006 | ❌ 禁止在 Consolidation 阶段建议"添加 AI 补全功能" |

### 🔒 已确认的实施策略

| # | 策略 | ADR | 禁止建议 |
|---|------|-----|---------|
| 6 | **P33 source_span 仅填充 6 站点**（ComponentIR/StoreIR/3×FunctionIR/HeroIntent）。29 个延迟；ModuleIR/IRProgram 物理上无法填充 | ADR-008 | ❌ 禁止建议"填充所有 37 个结构体的 source_span" |
| 7 | **P36 跨后端并行**（`compile_multi_target` + rayon），**不是**跨组件并行 | ADR-009 | ❌ 禁止建议"在单个后端内并行化组件代码生成" |
| 8 | **P38b：修复 XSS 链 + 能力收紧**。`__TAURI_INTERNALS__` **不能完全剥离**（调试窗口需要事件 IPC 通信）；真正有价值的是**能力收紧**（`core:default`→`core:event:allow-emit`+`core:event:allow-listen`）。postMessage/innerHTML 的安全风险被**严重高估**——调试窗口加载的是本地开发者自己写的 .omni 应用，运行在本地 Tauri webview 里，不存在远程攻击面。详见下方「调试窗口安全威胁模型」。 | ADR-010 | ❌ 禁止建议"完全移除 `__TAURI_INTERNALS__`"；❌ 禁止将 postMessage/innerHTML 标记为 CRITICAL |
| 9 | **P39 MVP 是消除未使用计算属性**（~200行），暂不消除信号 | ADR-010 | ❌ 禁止在 motion 95% 覆盖率达成前建议"消除未使用信号" |
| 10 | P35 Crate 拆分为 `omnilang-core` + `omnilang-codegen-omniview` + facade。7 个暂停后端保持不动 | ADR-006, ADR-009 | ❌ 禁止建议"一次拆分所有 9 个后端" |

### 🔒 已确认的术语和分类

| # | 术语 | ADR | 纠正 |
|---|------|-----|------|
| 11 | `poiesis/src/services/codegen/` → **编辑器往返翻译层**（parser + emitter），不是编译器 | ADR-005 | "双编译器"→"双解析器" |
| 12 | P28 unwrap 叙事误导——原始评审报 165 处生产 unwrap，实际几乎所有在 `#[cfg(test)]` 中 | ADR-007（Layer 2 纠正） | 后续评审中不应再建议"修复 165 处 unwrap" |
| 13 | ts-rs **被前端消费**——`CompileResult` + `Diagnostic` 从 `__generated__/` 导入 | ADR-007（Layer 2 纠正） | 后续评审中不应再建议"移除 ts-rs 依赖" |
| 14 | TS golden 测试**已在 CI 中**（`ci.yml:86-87`） | ADR-007（Layer 2 纠正） | 后续评审中不应再建议"将 TS golden 测试加入 CI" |

### 🔒 已确认的流程和标准

| # | 标准 | ADR | 禁止建议 |
|---|------|-----|---------|
| 15 | **三层审查方法论**：Layer 1 广度 → Layer 2 深度/代码验证 → Layer 3 实施计划 | ADR-007 | ❌ 禁止跳过 Layer 2 直接产出实施计划 |
| 16 | **子代理不可靠**——实现代理 67% 失败率，审查代理 33% 重大误判。Layer 2 必须主会话交叉验证 | ADR-007 | ❌ 禁止仅依赖单层子代理审查 |
| 17 | **UTF-8 注释和文档优先**（中文） | 项目约定 | ❌ 禁止将文档或注释批量翻译为英文 |

### 🔒 已确认不再争论的技术事实

| # | 事实 | 来源 |
|---|------|------|
| 18 | `ComputedIntent.deps` 始终为 `vec![]`——依赖追踪从未实现，这是 bug 不是设计决策 | 深度复审 Finding #7 |
| 19 | `OmniViewGenerator` 有 **45** 个字段（不是 55）——已逐字段计数 | 深度复审纠正 |
| 20 | Clippy 警告 **41** 个（不是 39）——`cargo clippy --lib` 实测 | 深度复审纠正 |
| 21 | source_map 覆盖 bug：`codegen/mod.rs:117` 用 `=` 覆盖逐元素源映射，应为 `extend` | 深度复审 Finding #1 |
| 22 | TypeKind 序列化丢失 `type_args`——注释明确说 "Not serialized"，`Array<String>` 序列化后变为 `Array` | 深度复审 Finding #3 |
| 23 | 测试基准：Rust `656 passed / 0 failed`（lib），TS `20 files / 144 tests` | 上次会话 |

---

---

## 调试窗口安全威胁模型

> **作用**：防止未来安全审查再次将 postMessage/innerHTML 标记为 CRITICAL。

### 架构

```
┌─ 主窗口（Tauri main webview）──────────────────┐
│  __TAURI_INTERNALS__ 可用（完整 IPC）            │
│                                                  │
│  ┌─ 调试窗口（debug webview）──────────────────┐ │
│  │  __TAURI_INTERNALS__ 可用（仅事件 IPC）     │ │
│  │  debug-window capability: emit + listen     │ │
│  │                                             │ │
│  │  ┌─ iframe #omni-preview ────────────────┐ │ │
│  │  │  sandbox="allow-scripts"               │ │ │
│  │  │  __TAURI_INTERNALS__ ❌ 不可访问       │ │ │
│  │  │  加载开发者自己的 .omni 编译产物       │ │ │
│  │  └────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

### 威胁模型评估

| 因素 | 现实 | 安全审查的误判 |
|------|------|---------------|
| **加载的内容** | 开发者**自己写的** `.omni` 应用，不是用户输入 | 被错误当作"不可信第三方内容" |
| **运行环境** | 本地 Tauri webview，无网络暴露 | 被按 Web 公网服务标准要求 |
| **攻击前提** | 需要同一台机器上的另一个恶意进程或已被入侵的窗口 | 攻击前提本身就意味着系统已被攻破 |
| **沙箱状态** | `sandbox="allow-scripts"` 已阻止 iframe 内代码访问 Tauri API | 沙箱已正确实现隔离目标 |

### 各项修复的务实优先级

| 原评级 | 修复项 | 实际优先级 | 理由 |
|--------|--------|-----------|------|
| P0 CRITICAL | innerHTML→textContent | **P2**（顺手改） | 自己写的应用不会 XSS 自己，但改用 textContent 也不费事 |
| P0 CRITICAL | postMessage origin 验证 | **P3**（可有可无） | 本地 webview，没有跨域攻击面。`e.source !== omniPreview.contentWindow` 已经足够 |
| P1 HIGH | postMessage `'*'` → 特定 origin | **P3** | 同上 |
| P1 HIGH | capability: `core:default` → 2 原子权限 | **P1**（该修） | 唯一真正有价值的修复——调试窗口不需要 menu/tray/window/app 等 7 个权限包 |

### 安全审查时的正确结论

1. **沙箱架构是正确的**——iframe `sandbox="allow-scripts"` 已阻止用户编译代码访问 Tauri API
2. **`__TAURI_INTERNALS__` 不能剥离**——调试窗口需要事件 IPC 与主窗口通信（转发编译结果、热替换、元素点击）
3. **唯一值得修的**：收紧 debug-window capability，从 9 个权限包减少到 2 个原子权限
4. **postMessage/innerHTML 是纵深防御的最外层**——在本地桌面应用场景下，不应该列为 CRITICAL 或 HIGH

---

## 项目特定知识

### 编译器管线架构

```
Lexer → Parser → Semantic → IR → Codegen（9 目标，P0 仅 3 个）
```

- **TS 管线**（`poiesis/src/services/codegen/`）是编辑器往返翻译层，**不是编译器**（ADR-005）
- **真正的编译器**只有 Rust 端 `omnilang/`（ADR-001）

### 代码生成目标优先级（ADR-003）

- **P0 活跃**：OmniView + React + Flutter
- **暂停**：Vue, Dioxus, Python, JavaScript, Rust, Dart（feature-gated 保留）
