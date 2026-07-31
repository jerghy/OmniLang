# 语义级 DSL 实现继续——会话启动文档（2026-08-02）

> **用途**：新会话在开始执行剩余任务前，按本文档一次性准备好全部上下文。
> **分支**：`feature/semantic-dsl`（25 commits，未合并 main）
> **进度**：35 tasks 中已完成 25（Phase 0-8.1）；剩余 10（8.2, 9.1-9.5, 10.1）

---

## 1. 文档导航（读什么 / 为什么读 / 什么时候读）

### 1.1 核心执行文档（按重要性排序）

| 文档 | 行数 | 用途 | 何时读 |
|------|------|------|--------|
| `docs/superpowers/plans/2026-07-31-semantic-level-dsl-implementation.md` | ~1550 | **35 tasks 的权威定义**（每个 task 有文件、步骤、commit message） | 执行任何 task 前读对应 section |
| `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md` | ~970 | **BNF 语法权威**（9 语义域），parser 实现者受众 | parser/语法相关任务（9.x 迁移的语法对照） |
| `docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md` | ~1380 | **codegen 平台映射策略**（ref/动画/slot 在各平台的实现方案），codegen 实现者受众 | codegen 相关任务（8.2 验证什么、9.x 为何这样迁移） |
| `.architecture/decisions/adrs/ADR-052-semantic-level-dsl-cross-target-codegen.md` | ~330 | **为什么做这次重构**：三层语义架构 + 24 feature floor 表 | 第一次接触计划时 |
| `.architecture/decisions/adrs/ADR-053-semantic-level-dsl-syntax-formalization-and-implementation.md` | ~330 | **7 项设计决策**（D1-D7：双文档/混合Ref/白名单/原生refs/Slot重构/保留computed/删除零使用）+ 保留vs删除表 | 理解语法决策依据 |
| `.architecture/decisions/adrs/ADR-020-flutter-state-management-riverpod.md` | — | Phase 10 要更新为 Implemented | 10.1 时 |
| `.architecture/decisions/adrs/ADR-021-cross-target-feature-parity.md` | — | Phase 10 要重新定义为 behavior floor + diagnostics | 10.1 时 |
| `.architecture/reviews/cross-target-feature-gap-review-2026-07-31.md` | — | **React/Flutter codegen 原始 6 CRITICAL + 10 HIGH bug 清单**（C2-C8 等） | 理解 Phase 4/5 修复动机（已修完，参考用） |
| `.architecture/reviews/full-workspace-review-2026-07-31.md` | — | 全局评审（含 deferrals 来源） | 可选 |
| `.architecture/deferrals.md` | — | 10.1 要更新（关闭已完成项、新增新项） | 10.1 时 |

### 1.2 记忆文件（Auto Memory，`C:\Users\XHT\.claude\projects\D--space-OmniLang\memory\`）

| 记忆 | 何时读 |
|------|--------|
| `session-2026-08-01-semantic-dsl-implementation.md` | **必读**——本会话（Phase 0-8.1）完整记录：commits、10 个已修复 bug、7 个已知剩余问题、关键实现决策 |
| `next-session-2026-08-02.md` | **必读**——剩余任务清单、迁移清单、执行纪律 |
| `session-2026-07-31-semantic-dsl-syntax-and-plan.md` | 需要理解语法设计过程/失败方案时 |
| `adr-053-semantic-level-dsl-syntax-formalization.md` | ADR-053 摘要 |
| `omnilang-codegen.md` | **⚠️ 顶部有 2026-08-01 更新章节**——React/Flutter/OmniView 当前真实状态；下方历史章节已过时 |
| `omnilang-parser.md` | **⚠️ 顶部有 2026-08-01 更新章节**——新语法状态 + 已知限制 |
| `omnilang-animation.md` | **⚠️ 顶部有 2026-08-01 更新章节**——8 触发器/paused 删除/后端重写 |
| `omnilang-signals.md` / `omnilang-design-tokens.md` | 信号/store/design token 管线背景（store 语法未变） |

---

## 2. 当前状态（2026-08-01 结束时）

### 2.1 完成情况

| Phase | 内容 | Tasks | 状态 |
|-------|------|-------|------|
| 0 | Lexer/Parser（token 重命名、motion 语法、语义绑定、slot/@if/@for） | 0.1-0.4 | ✅ |
| 1 | IR（RefOpIR、Slot struct、Motion IR 变更） | 1.1-1.3 | ✅ |
| 2 | Semantic（validate_ref_ops、lower_inline_motions、validate_semantic_bindings） | 2.1-2.3 | ✅ |
| 3 | Shared codegen（design CSS emitter、expr formatters） | 3.1-3.2 | ✅ |
| 4 | React（Zustand v5、slots、refs、animation 重写、design CSS） | 4.1-4.5 | ✅ |
| 5 | Flutter（Riverpod 3.0、refs、slots、TweenSequence、design tokens） | 5.1-5.5 | ✅ |
| 6 | OmniView（semantic bindings、RefOpExpr、slot fallback、@focus） | 6.1 | ✅ |
| 7 | 非 P0（design gate → hard error） | 7.1 | ✅ |
| 8 | 8.1 parameterized fixtures ✅ / **8.2 CI TS+Dart 验证 ⬜** | 8.1-8.2 | 部分 |
| 9 | **Demo 迁移（9.1-9.5）⬜** | 9.1-9.5 | ⬜ |
| 10 | **Governance（10.1）⬜** | 10.1 | ⬜ |

### 2.2 测试基线（每 task 完成后必须保持）

```bash
cargo test -p omnilang-core            # ~641 tests
cargo test -p omnilang                 # ~256 tests（默认 features）
cargo test -p omnilang --features codegen-flutter   # ~426 tests
cargo test --workspace                 # 25 test binaries（⚠️ 不含 --all-features，那个有 pre-existing 失败）
cargo clippy -p omnilang               # 零新 warnings（基线有 ~16 个 pre-existing）
```

### 2.3 关键 commits（最近 5 个，作为验证基线）

```
7d688da1 test(codegen): parameterized test matrix（8.1，含 2 个编译器 bug 修复）
889a060e fix(omniview): debug runtime for semantic DSL（6.1）
443952e2 fix(non-p0): design block hard error（7.1）
94a14ab9 feat(flutter): design token codegen（5.5）
0c3a03a6 feat(flutter): TweenSequence + semantic animation（5.4）
```

---

## 3. 剩余任务详解

### Task 8.2: CI TypeScript + Dart 验证

**目标**：在 CI 中验证生成的 React/Flutter 代码可编译（不只是字符串断言）。

- **文件**：`.github/workflows/ci.yml`（当前只有 rust + tauri jobs，无 TS/Dart 检查）
- **步骤**（计划原文）：
  1. `tsc --noEmit` on React regression fixture output（编译 8.1 的 parameterized fixtures 到 React，对输出跑 tsc）
  2. `dart analyze` on Flutter regression fixture output
- **需要探索**：
  - `omnilang/tests/fixtures/parameterized/`（8.1 的 5 个 fixtures）
  - React 输出如何组织（`omnilang/src/codegen/react/mod.rs` 的 `build_file_output` / `CompileOutput` 文件列表）
  - 是否有现成的 tsconfig（`omnilang/tests/` 下或 `test/` 下搜索）
  - Flutter 输出验证：参考 5.4/5.5 实现者的做法（他们说用真实 Flutter 项目跑 `dart analyze`）——CI 里可能只需要一个最小 pubspec + analyze
  - `cargo test export_bindings --lib -p omnilang`（ts-rs 绑定新鲜度检查已存在于 CI，不要破坏）
- **风险**：CI 里跑 tsc/dart 需要 node/flutter 环境；建议做成一个 Rust 测试（`#[ignore]` 或 feature-gated）而不是 workflow step，或者在 workflow 加 job。看 8.1 的测试基础设施怎么组织的再决定

### Task 9.1: 迁移动画语法（11 文件）

**目标**：把 learn-omniview + fixtures 里的旧动画语法迁移到新语法。

**文件清单（含当前状态）**：

| 文件 | 状态 | 需要改什么 |
|------|------|-----------|
| `omnilang/tests/fixtures/react/motion_enter.omni` | ✅ 已迁移（0.2） | 无 |
| `omnilang/tests/fixtures/react/motion_hover.omni` | ✅ 已迁移（0.2） | 无 |
| `omnilang/tests/fixtures/react/motion_inview.omni` | ✅ 已迁移（0.2） | 无 |
| `omnilang/tests/fixtures/react/motion_scroll.omni` | ✅ 已迁移（0.2，删了 @scroll-start/end） | 无 |
| `omnilang/tests/fixtures/react/motion_combined.omni` | ✅ 已迁移（0.2） | 无 |
| `test/projects/learn-omniview/src/pages/MotionDemo.omni` | ⚠️ 部分（0.4 改了 @if mode） | motion body 语法 + 触发器 |
| `test/projects/learn-omniview/src/pages/MotionTriggersDemo.omni` | ⬜ 未迁移 | 触发器重命名 + 子属性独立化 + @focus |
| `test/projects/learn-omniview/src/pages/KeyframeAnimationDemo.omni` | ⬜ 未迁移 | keyframe 属性白名单 + @paused 删除 |
| `test/projects/learn-omniview/src/pages/EasingFunctionsDemo.omni` | ⬜ 未迁移 | steps() 位置参数 + easing kebab |
| `test/projects/learn-omniview/src/pages/AnimPropertiesDemo.omni` | ⬜ 未迁移 | 非白名单属性删除（blur/brightness/borderWidth）或改语义属性 |
| `test/projects/learn-omniview/src/pages/HeroTransitionDemo.omni` | ⬜ 未迁移 | @paused 删除 + 语法 |
| `test/projects/learn-omniview/src/pages/SignalPatternsDemo.omni` | ⚠️ 部分（0.4 改了 @if mode） | 如含动画语法则迁移 |
| `test/projects/learn-omniview/src/pages/ElementRefDemo.omni` | ⬜ 9.2 处理 | 其中 @motion 块（dragX300 等）9.1 改 |
| `test/projects/learn-omniview/src/components/CssVarsDemo.omni` | ✅ 已迁移（0.2，--* 透传） | 无 |

**语法映射表（新旧对照）**：

```text
旧                                  新
@motion fadeIn { ... over 400 }     @motion fadeIn { ... over: 400ms }
ease easeOut                        easing: ease-out
from { x: 0, y: 20 }                from { translateX: 0, translateY: 20 }
to { opacity: 1 }                   to { opacity: 1 }
steps(count: 6, direction: end)     steps(6, end)
@inView={motion}                    @in-view={motion}
@dragHorizontal={m}                 @drag-horizontal={m}
@in-view-once={expr}                once={expr}（子属性独立）
@scroll-start="top 80%"             start="top 80%"（子属性独立）
@drag-constraints="..."             constraints="..."（子属性独立）
@paused={expr}                      ❌ 删除（用条件渲染替代）
blur/brightness/borderWidth/fontSize  ❌ 非白名单（用 --* 透传或删除 demo）
```

**验证**：每个文件迁移后 `omnic build --target omniview`（或对应测试）应零错误。

### Task 9.2: 迁移 ref 语法（ElementRefDemo.omni）

| 旧 | 新 |
|----|----|
| `refBox.style.width = "100px"` | `style: { width: "100px" }` 或 style 信号绑定（非法 API → compile error + hint） |
| `refBox.focus()` | `focus={sig}` 语义绑定（高频）或保留 `refBox.focus()`（现在是合法 RefOpExpr） |
| `refBox.blur()` | 同上（blur 是合法 RefOp） |
| `refBox.scrollTo(0, 100)` | `scroll-y={sig}` 语义绑定（高频）或 `refBox.scrollTo(0, 100)`（合法 RefOp） |
| `refBox.getBoundingClientRect()` | `refBox.measure()` |
| `refBox.offsetWidth` | `refBox.measure().width` |
| `refBox.value = "x"` | `refBox.value = "x"`（现在合法：SetValue） |

**注意**：`getBoundingClientRect`/`offsetWidth`/`offsetHeight`/`style`/`innerHTML` 现在是 **compile error**（validate_ref_ops），迁移后这些必须消失。`focus()`/`blur()`/`scrollTo()`/`value` 是合法 RefOpExpr，可保留。

### Task 9.3: 迁移 slot 语法（4 文件）

| 文件 | 当前 | 目标 |
|------|------|------|
| `components/Modal.omni` | `@slot header` 在 view 内 | `@slot` 声明移到 component body；view 内用 `{header}` 插值 |
| `components/NotificationsPanel.omni` | 同上 + `ref: scrollContainer` 形式 | 同上 + `@ref={scrollContainer}` 统一 |
| `components/SlotDefaultDemo.omni` | 3 种 slot 形式 | 移到 body + 消费侧命名 props |
| `pages/SignalPatternsDemo.omni` | `@slot(header)` 括号形式 | `@slot header` 无括号形式 |

**消费侧**：`slot="name"` 属性 → 命名 props（React: `header={<div/>}`；OmniView 仍支持 slot= 属性——检查）

### Task 9.4: @if mode 语法

- `mode=` → `mode:`（MotionDemo 和 SignalPatternsDemo 已在 0.4 改过 ✅，验证无残留）
- `mode="sync"` → 删除（值验证 fade/wait/none）
- 用 grep 验证：`grep -rn 'mode=' test/projects/learn-omniview/src/`

### Task 9.5: 重新生成 golden fixtures + build output

- 把迁移后的源码同步到 `tests/fixtures/golden/test/projects/learn-omniview/...` 镜像
- **注意**：golden MANIFEST.md 的 "Motion / Animation" section 是空的（探索 agent 发现）——9.5 应补上迁移后的 motion 页面条目
- 重新生成 `build2/` 输出（如果该目录还存在——验证）

### Task 10.1: Governance 更新

- **ADR-020** → status **Implemented**（Riverpod 3.0 已完成）
- **ADR-021** → 重新定义为 "semantically equivalent behavior for documented feature floor + hard diagnostics"
- `.architecture/deferrals.md` → 关闭已完成项（paused_override 删除、@paused、React Hooks 模式等），新增新延期（本会话发现的：is_store_symbol get/set、--all-features 测试、runtime TS 表、文档更新）

---

## 4. 代码探索地图（执行任务前必看）

### 4.1 Task 8.2 前

- `omnilang/src/codegen/tests_regression/parameterized.rs`（8.1 的测试基础设施——宏 + omni_to_compile_output adapter）
- `omnilang/tests/fixtures/parameterized/`（5 个 fixtures）
- `.github/workflows/ci.yml`（现有 jobs 结构；注意已有 `export_bindings` 新鲜度检查）
- 搜索仓库中现有 tsconfig / pubspec（可能没有——需要新建）
- React 输出文件结构：`omnilang/src/codegen/react/mod.rs` 的 `generate()` 返回哪些文件

### 4.2 Phase 9 前

- `test/projects/learn-omniview/src/pages/` 全部动画文件（读当前语法）
- `crates/omnilang-core/src/parser/top_level/motion.rs` + `parser/view/element.rs`（确认当前接受的语法——以代码为准，不是文档）
- `omnilang/tests/fixtures/golden/MANIFEST.md`（了解 golden 机制——**注意它只是 parse-only 一致性测试**，迁移后要更新镜像）
- `omnilang/tests/golden_parse.rs`（golden 测试如何运行）

### 4.3 迁移时验证工具

```bash
# 单文件编译验证（每个迁移文件后跑）
cargo run -p omnic -- build --target omniview test/projects/learn-omniview/src/main.omni 2>&1
# 或直接看 analyse 诊断
cargo run -p omnic -- analyse test/projects/learn-omniview/src/main.omni 2>&1
```

**注意**：learn-omniview 是完整项目（main.omni 入口 + 多个页面），单页迁移后整个项目仍可能因其他页面报错——用 analyse 看错误清单，逐页清零。

---

## 5. 已知风险与规避

| # | 风险 | 规避 |
|---|------|------|
| 1 | **[MEDIUM] `is_store_symbol` 未覆盖 lower_expr 的 get/set 特例**（`crates/omnilang-core/src/ir/lower/expr.rs:232-238`）：store action 名为 `get`/`set` 时 `@onPress={CounterStore.set(5)}` 静默输出 `set;` | 8.2 或 9.x 前修复（加同样守卫 + fixture）。改动小，2-3 行 |
| 2 | **--all-features 测试编译失败**（pre-existing）：javascript/rust/vue/dioxus 测试文件用旧 IR 字段（`SignalRefIR.owner`、`MutationIntent.field_object`） | 不阻塞默认流程；如 Phase 10 要全量验证需修（~14 处，机械） |
| 3 | **vue/tests.rs 缺右括号** + dioxus feature-wiring 问题（pre-existing） | 同上，非阻塞 |
| 4 | **CRLF 行尾**：仓库部分文件 CRLF，`cargo fmt` 会破坏行尾产生大量 churn | 改完文件后 revert 无关 fmt churn；只 commit 本 task 文件 |
| 5 | **工作树遗留未提交文件**：`.architecture/decisions/adrs/ADR-020/021`、learn-omniview 若干 `.omni`、untracked docs（前会话遗留） | **不要动、不要 commit**——它们是其他工作 |
| 6 | **learn-omniview 在迁移完成前编译失败**（预期） | Phase 9 迁移完成前，任何依赖它的测试都可能红；golden 测试只覆盖 MANIFEST 列出的文件 |
| 7 | **runtime TS 缺口**（非 P0）：`packages/omni-animation/src/easing.ts` 无 kebab 别名（power1-in 降级 linear）；`keyframes.ts` TRANSFORM_MAP 无 translateX/Y/Z | 不在计划内，记录到 deferrals（10.1） |
| 8 | **文档过时**：`docs/omniview/06-templates.md:128` 教 `value={inputText}`（现需 @ref）、`11-motion-animation.md` 教 mode="sync" | 10.1 或 9.x 顺手更新 |

---

## 6. 执行流程模板（沿用本会话纪律）

```markdown
## 每个 task 的循环

1. 读实现计划对应 section（全文给 subagent，不让它读文件）
2. 派发实现 subagent（general-purpose, sonnet）——prompt 含：task 全文 + 上下文 + 本文档相关章节
3. 实现完成后派发 spec reviewer（haiku）——独立验证，不信实现报告
4. 通过后派发 code-quality reviewer（code-reviewer agent）——检查代码质量
5. 有问题 → 通知原实现 agent 修复（SendMessage 恢复上下文）→ 重新审查
6. 全部通过 → 标记 task 完成 → 下一个 task

## 每个 task 的验证命令

cargo build -p omnilang && cargo test -p omnilang && cargo clippy -p omnilang
# codegen 任务加：
cargo test -p omnilang --features codegen-flutter
# demo 迁移任务加：
cargo run -p omnic -- analyse <项目入口>  # 错误清零

## commit 规范

git commit -m "<type>: <description>"   # 只 commit 本 task 文件
git add -A 之后检查 git status——排除工作树遗留文件
```

## 7. 完成所有任务后的收尾

1. 全量验证：`cargo test --workspace` + `cargo clippy --workspace`
2. 更新记忆：session 记录 + next-session（或标记完成）
3. 用 `superpowers:finishing-a-development-branch` 决定合并/PR 策略（分支 feature/semantic-dsl → main，25+ commits）
4. 考虑把本启动文档归档或更新
