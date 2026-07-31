# ADR-052: OmniLang 语义级 DSL — 跨目标代码生成架构

## Status

Accepted

## Context

2026-07-31 的 5 专家跨目标特征评审（`cross-target-feature-gap-review-2026-07-31.md`）发现：React 和 Flutter 的 P0 生产目标与 OmniView 调试运行时之间存在系统性 feature gap。根本原因不是"特性缺失"，而是**OmniLang 当前的编译器架构是 OmniView-first 设计**：

- **Ref API 是 DOM 形状的**：`.style.transform`、`.getBoundingClientRect()`、`.capturePointer()` 等 API 直接映射到浏览器 DOM，Flutter 的 `GlobalKey` 无法等价实现
- **动画 codegen 只测了 OmniView**：Flutter 的 `AnimationController`/`TweenSequence` 路径缺少多 stop keyframe、playback properties、多属性 tween
- **Design Token 只有 Web CSS 路径**：React/Flutter 遇到 `design` block 直接返回零文件 + warning（`omnilang/src/codegen/mod.rs:152-203`）
- **Slot 的 `default_content` 在 IR lowering 时被丢弃**（`ir/lower/view.rs:205-208`），所有三个目标都受影响
- **生成的 Flutter Dart 代码不编译**：6 个 CRITICAL bug（`const {` 缺类名、`_runEffect_N()` 未定义、store 读 emit 裸标识符、`color: #ff0000` 不合法 Dart 等）
- **平台惯用方式被忽略**：生成 `setTheme()` 包装函数、`__omni_*` 前缀的辅助函数等 OmniView 风格的抽象，而非目标平台的惯用代码

评审的核心结论：**OmniLang 必须是语义级语言**。`.omni` 源代码描述"我想做什么"（意图），编译器理解意图后，为每个编译目标生成该平台上最地道的代码。

### 设计文档

完整语义级 DSL 设计文档位于 `docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md`。本 ADR 记录支撑该设计的架构决策。

## Decision Drivers

1. **三个 P0 目标的本质不同**：OmniView 是调试运行时（零外部依赖、单文件 HTML），React 是 Web 生产目标（TypeScript + Vite/Rollup 生态），Flutter 是移动端生产目标（Dart + Widget 引擎）。它们的内核完全不同——DOM vs Virtual DOM vs Widget Tree，强制统一输出格式会导致最低公分母设计
2. **生产代码的可维护性**：生成的代码必须能通过该生态的标准 lint 规则（`eslint-plugin-react-hooks`、`dart analyze`），能被该生态的开发者直接阅读和修改，遵循该生态的最佳实践
3. **ADR-021 的语义修正**：当前 ADR-021 要求"所有后端产生语义等价的输出"，但已被 ADR-028（14 项 OmniView 有意设计取舍）事实上打破。需要重新定义为"**语义等价行为 + documented feature floor + hard diagnostics outside it**"
4. **官方 API 优先**：库选择必须遵循：官方/标准 API → 热门社区库 → 无法实现的提出来讨论是否删除
5. **一步到位**：不设 Phase 2 过渡期，直接到最终态。历史经验表明（ADR-020 Riverpod 迁移搁置、@ref Phase 2 延期）过渡期积累技术债务

## Decision

**OmniLang 是一个语义级 DSL**。编译器采用三层架构理解语义并生成目标代码：

### 第一层：语义原语（Semantic Primitives）

`.omni` 源码中每个语言构造都有一个**平台无关的语义定义**。编译器在语义分析阶段（`semantic/`）解析意图，在 IR lowering 阶段（`ir/lower/`）将语法糖展开为标准 IR。

关键语义直觉：

| `.omni` 语法 | 语义（编译器理解的意图） |
|-------------|----------------------|
| `ref.focus()` | "让这个元素获得键盘焦点" |
| `@enter { keyframe { ... } }` | "当元素首次出现在渲染树中时，播放这段动画" |
| `token primaryColor: Color = #6366f1` | "定义一个叫 primaryColor 的颜色设计值" |
| `theme = "dark"` | "切换到 dark 主题" |
| `<slot name="header">` | "这里渲染父组件传入的 header 内容" |
| `@for item in items` | "为列表中每个 item 重复渲染这个元素" |
| `store TodoStore { signal items }` | "定义一个叫 TodoStore 的响应式状态容器" |

### 第二层：IR 作为意图模型

IR 层（`crates/omnilang-core/src/ir/`）不描述"如何渲染"，而描述"渲染什么"。关键设计：

- **新增 `RefOpIR` 枚举**：`Focus | Blur | ScrollTo | ScrollBy | MeasureWidth | MeasureHeight | MeasurePosition | Select | SetSelection | Value | SetValue | CapturePointer | ReleasePointer`。语义 pass `validate_ref_ops.rs` 将合法 ref 调用改写为 `ExprIR::RefOpExpr`，非法调用（`.style`、`.querySelector` 等）在语义阶段报 compile error
- **Slot IR 修复**：`ViewTree::Slot { name: Option<String>, fallback: Vec<ViewTree> }`（当前 `ViewTree::Slot(Option<String>)` 是叶子节点，lowering 丢弃了 `default_content`）
- **内联动画体 lowering**：`lower_inline_motions` pass 将未命名 keyframe 展开为命名 `MotionIntent`
- **删除 `PlaybackIR.paused` / `MotionBehavior.paused_override`**

### 第三层：Codegen 作为目标翻译

Codegen 层不翻译 OmniView 的实现方式，而是**生成该平台开发者会手写的代码**。关键约束：

- **不生成包装函数**：不生成 `setTheme()`、`__omni_*` 前缀函数、OmniView 风格的 API 桥接
- **直接用目标平台的惯用 API**：
  - React: `useRef`、`useEffect`、`useState`、JSX `ref={}` prop（React 19 已废弃 `forwardRef`）
  - Flutter: `GlobalKey`、`FocusNode`、`ScrollController`、`ThemeExtension`、`AnimationController` + `TweenSequence`
- **库选择（一步到位）**：React → Zustand v5（`create<T>()()` + selector hooks）；Flutter → Riverpod 3.0（`Notifier<T>` + `NotifierProvider`）

### Feature Floor 定义

所有三个 P0 目标必须实现的语义原语（24 项）：

| 类别 | 原语 | 例外 |
|------|------|------|
| Ref | focus/blur, scrollTo/scrollBy, width/height/position, select/setSelection, value/setValue | capturePointer Flutter: diagnostic |
| Animation | 8 triggers, multi-stop multi-property keyframes, playback (loop/count/direction/delay/fill/duration/easing), inline body lowering | steps() easing Flutter: diagnostic |
| Design | 13 token types, static tokens + themes, classes, variants, dynamic tokens | ZIndex Flutter: diagnostic |
| Slots | default + named + fallback | — |
| Control Flow | @if ternary + transition_mode, @for + key_expr + empty_body + [scroll] | — |
| Stores | signals, getters, actions, effects | — |
| Lifecycle | mount, unmount, effect with deps | — |
| Expressions | OptionalChain, BlockExpr IIFE, MatchExpr, Await, Spread, compound assignment | — |

**OmniView-only（明确排除对标）**：源码映射、editor bridges（`editor.rs`、`debug.rs`）、`css_only` 模式、`wrap_debug_html` 无 CSP、inline signal runtime、6-phase JS execution order、`(0, eval)` hot-replace、ElementTree 元数据。

**已删除的语法特性**：`ref.style.*`（用信号+style 绑定替代）、`ref.textContent`（用文本插值）、`paused_override`（用条件绑定）、`PlaybackIR.paused`、React Hooks store 模式（只保留 Zustand v5）、@focus 静态 HTML 中 emit 的 JS 死代码。

### ADR-021 重新定义

旧：*"All codegen backends must produce semantically equivalent output for the same `.omni` source."*

新：*"All P0 backends must produce semantically equivalent **behavior** for the documented **feature floor**. Features outside the floor must produce a hard **diagnostic** — never a silent drop. Never zero output files while claiming success."*

## Consequences

### Positive

- **生成代码 = 地道手写代码**：React 输出通过 `eslint-plugin-react-hooks`，Flutter 输出通过 `dart analyze`。开发者可以打开生成的文件直接理解和修改
- **跨目标行为一致性**：`.omni` 源码的语义在所有平台上产生相同的可观察行为（不是相同的内部实现）
- **IR 层语义验证前置**：`validate_ref_ops`、`lower_inline_motions` 等在语义/IR lowering 阶段就完成验证和规范化，codegen 只负责翻译
- **删除不可跨平台的 API**：`ref.style.*` 等 DOM 专属 API 直接删除，消除未来的维护负担
- **一步到位，零技术债**：不设 Phase 2 过渡期，所有设计决策直接到最终态
- **诊断取代静默失败**：design block 的非 P0 目标从 warning+零文件改为 hard error；非法 ref 操作在语义阶段就报 error

### Negative

- **Flutter 代码生成复杂度显著增加**：`TweenSequence`（多 stop keyframe）、per-property `Animatable<T>`、`ThemeExtension`、`GlobalKey` 按需生成 Controller/Node 都是非平凡的 codegen 逻辑
- **React Zustand v5 getter 模式需要额外生成 selector hooks**：每个 `.omni` getter 需要生成独立的 `useXxx()` hook，而非简单地放在 store 内部
- **React 19 兼容性**：`ref` 作为普通 prop 需要 React 18.3+ / React 19，TS 类型也需要单独声明 `ref` prop
- **IR 变更影响面**：`ViewTree::Slot` 结构变更 + `RefOpIR` 新增需要更新所有后端（但非 P0 后端可以返回 diagnostic）

### Neutral

- **设计文档维护负担**：1,200+ 行的语义级 DSL 设计文档需要随语言演进同步更新
- **Zustand v5 + Riverpod 3.0 同时迁移**：两个库的 API 都与当前 codegen 中使用的版本不同，需要一次性重写

## Implementation Strategy

### Blast Radius

**Impact Scope**：所有三个 P0 代码生成后端 + IR 类型定义 + semantic 验证 pass。非 P0 目标（Vue/Dioxus/Python/JS/TS/Rust/Dart）影响较小——主要是 design block 的 warning→error 升级和 IR 结构变更后的编译修复。

**受影响的组件**：
- `crates/omnilang-core/src/ir/types.rs` — 新增 `RefOpIR`、修改 `ViewTree::Slot`、删除 `PlaybackIR.paused`
- `crates/omnilang-core/src/ir/types_motion.rs` — 新增 `TriggerKind::Focus`、删除 `paused_override`
- `omnilang/src/codegen/mod.rs` — design block gate 移除（P0）+ error 升级（非 P0）
- `omnilang/src/codegen/omniview/design.rs` → `codegen/shared/design.rs` — 提取共享 CSS emitter
- `omnilang/src/codegen/react/` — 全部模块：Zustand v5 重写 store、React 19 ref prop、slot children、design CSS
- `omnilang/src/codegen/flutter/` — 全部模块：Riverpod 3.0 重写 store、Ref handle 生成、TweenSequence 动画、设计 token 映射
- `crates/omnilang-core/src/codegen/shared/expr.rs` — OptionalChain/BlockExpr/Match default override
- 新增 `semantic/validate_ref_ops.rs`
- 新增 `ir/lower/lower_inline_motions.rs`
- `.architecture/deferrals.md` — 更新推迟项列表
- `ADR-020` — 状态更新为 Riverpod 3.0 已实施
- `ADR-021` — 文本重新定义为 behavior floor + diagnostics

**Risk Mitigation**：
- 参数化跨目标测试矩阵扩展（design/slots/refs/key_expr/animation 至少各 1 个 fixture）
- CI 门：`tsc --noEmit`（React 生成输出）+ `dart analyze`（Flutter 生成输出）
- `omnic build --target react|flutter` on learn-omniview smoke test

### Reversibility

**可逆性级别**：Medium。所有变更都是 codegen 层 + IR 类型定义。.omni 语法不变。通过 git revert 可回滚单个模块。但跨模块的 IR 类型变更（`ViewTree::Slot`、`RefOpIR`）一旦落地，回滚需要同时更新所有后端。

**Rollback Feasibility**：单个模块可独立 revert。`ViewTree::Slot` 变更影响最广但结构变更本身就是 bug fix（`default_content` 被丢弃是 bug）。

### Sequencing & Timing

**前置条件**：
- [x] 5 专家跨目标特征评审完成（2026-07-31）
- [x] 完整语义级 DSL 设计文档（`docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md`）
- [ ] IR 类型变更（`RefOpIR`、`ViewTree::Slot`、motion 变更）— 必须先于 codegen
- [ ] `validate_ref_ops` semantic pass — 依赖 `RefOpIR`
- [ ] `lower_inline_motions` IR pass — 依赖 motion 变更

**实施顺序**：
1. IR 层变更（所有 codegen 的依赖）
2. 共享 codegen 提取（`design.rs`、`expr.rs` override）
3. React codegen 重写（Zustand v5 + design CSS + slot + ref + 动画）
4. Flutter codegen 重写（Riverpod 3.0 + 动画 + design + ref + slot）
5. CI 验证门 + 测试扩展
6. 治理更新（ADR-020、ADR-021、deferrals）

## Alternatives Considered

### 方案 A：保持 OmniView-first，为 React/Flutter 打补丁

继续以 OmniView 为实现参考，React/Flutter 逐个补 feature gap。

**被否决**：5 专家评审已证明这条路径导致两个生产后端持续落后。Flutter 的 6 个 CRITICAL bug 在 committed golden snapshot 中存在数周未被发现——因为没有 `dart analyze` 验证门，且 codegen 设计未从 Flutter 角度出发。

### 方案 B：统一 IR 输出，不做目标特化

所有目标输出相同的抽象代码（如 WASM、JS 子集），由运行时库做平台适配。

**被否决**：
- React/Flutter 的差异化正是它们的价值——WAAPI vs AnimationController、CSS vs ThemeData 是经过生态验证的最优解
- 引入运行时库增加依赖和维护负担
- 违反"生成代码 = 地道手写代码"原则

### 方案 C：只保留 React 为 P0，Flutter 降级

**被否决**：ADR-003 已确定 Flutter 为 P0。且本次评审发现的 Flutter 问题全部是 codegen 实现质量问题，非概念不可行。修复这些 bug 比降级更经济。

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**：这不是新增复杂度——这是重新定义已有的复杂度。语义级 DSL 框架替代了"OmniView-first 翻译"框架，不新增代码层，而是改变 codegen 层的代码生成方式。删除的特性（`ref.style.*`、`paused_override`、Hooks 模式）**减少**了整体复杂度。

**Necessity Assessment**: 10/10
- **Current need**: 5 专家评审发现 6 CRITICAL + 10+ HIGH 问题，全部源于 OmniView-first 架构。React/Flutter 生产目标无法编译旗舰 demo 项目
- **Future need**: 任何新增的语言特性都将面临同样的三目标翻译问题，语义级框架一次性解决
- **Cost of waiting**: 每一个延期都意味着 Flutter codegen 继续 emit 不可编译的 Dart、React store 继续 serve 过期数据
- **Evidence of need**: committed golden snapshot 中编码了 `const {`（语法错误）和未定义的 `_runEffect_0()` 调用

**Complexity Assessment**: 4/10
- **Added complexity**: 语义 pass（`validate_ref_ops`）+ IR type 变更 + codegen 重写。但这些替代了现有的临时方案（OmniView 代码中散布的 ref 操作字符串匹配、shared expr 的默认实现）
- **Maintenance burden**: 降低——语义 pass 在编译期发现问题，而非让用户遇到 runtime error
- **Learning curve**: 新贡献者需要理解"语义 DSL"思维模型，而非"OmniView 翻译"模型。但后者从未被文档化过
- **Dependencies introduced**: Riverpod 3.0（替代已废弃的 2.x API）、Zustand v5（替代 v4），都是库版本升级，非新依赖

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 4/10
- **Ratio**: 0.40 ✅ (Target: <1.5)

**Overall Assessment**: 这是纠正已有架构偏斜的必要工程，不是过度设计。删除 DOM 专属 API、废弃 Phase 2 过渡、一步到位到 Zustand v5/Riverpod 3.0——这些决策都是在**减少**复杂度，而非增加。

## Validation

**Acceptance Criteria:**
- [ ] `learn-omniview` 项目可通过 `omnic build --target react` 生成完整的 React 项目（Vite + TSX + CSS）
- [ ] `learn-omniview` 项目可通过 `omnic build --target flutter` 生成 `dart analyze` clean 的 Flutter 项目
- [ ] 参数化测试矩阵覆盖所有 24 项 feature floor 特性
- [ ] `tsc --noEmit` 对生成的 React regression fixture 通过
- [ ] `dart analyze` 对生成的 Flutter regression fixture 通过
- [ ] 所有现有 550+ 测试通过（`cargo test -p omnilang`）
- [ ] `ADR-020` 状态更新为 Riverpod 3.0 已实施
- [ ] `ADR-021` 文本更新为 behavior floor + diagnostics
- [ ] `.architecture/deferrals.md` 更新

**Testing Approach:**
- Insta snapshot 测试覆盖所有 codegen 变更（per-backend + 参数化跨目标矩阵）
- Proptest `compile_never_panics` 验证不 panic
- CI gate: TypeScript/Dart 编译验证防止不可编译代码进入 golden snapshot
- Browser MCP 测试 learn-omniview 在 React target 下的完整页面渲染

## References

- [Cross-Target Feature Gap Review 2026-07-31](../../.architecture/reviews/cross-target-feature-gap-review-2026-07-31.md) — 5 专家评审
- [Semantic-Level DSL Design Document](../../docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md) — 完整设计规范
- [ADR-003](./ADR-003-codegen-target-prioritization.md) — P0 目标优先级
- [ADR-020](./ADR-020-flutter-state-management-riverpod.md) — Flutter Riverpod 选型（状态待更新）
- [ADR-021](./ADR-021-cross-target-feature-parity.md) — 跨目标 parity 执行（文本待更新）
- [ADR-028](./ADR-028-omniview-debug-runtime-design-tradeoffs.md) — OmniView 14 项有意设计取舍
- [ADR-032](./ADR-032-omniview-animation-design-scoping.md) — 动画范围界定 33→14
- [ADR-036](./ADR-036-element-reference-system.md) — Element Reference System
- [ADR-047](./ADR-047-dynamic-variable-design-tokens.md) — 动态 Design Token Phase 1
- [ADR-049](./ADR-049-dynamic-token-variables-phase2-runtime-signals.md) — 动态 Design Token Phase 2
