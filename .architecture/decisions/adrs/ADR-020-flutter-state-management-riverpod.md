# ADR-020: Flutter 状态管理选用 Riverpod 3.0

## Status

Accepted

> **⚠️ 2026-07-29 状态**：此 ADR 已被接受但**尚未实施**。所有验收标准仍为空白。Flutter codegen 仍使用 `ChangeNotifier` + `StatefulWidget` + `setState()`。Riverpod 3.0 迁移是已批准的未来方向，但无实现时间线。

## Context

OmniLang 的 Flutter 代码生成目标当前使用 Dart 原生 `ChangeNotifier` 管理 store 状态：
- `gen_store_dart()` 生成 `class XxxStore extends ChangeNotifier`，手动调用 `notifyListeners()`
- 组件状态通过 `StatefulWidget` + `setState()` 管理
- 无依赖注入机制，store 实例需要手动传递
- 与 React 目标的 Zustand/Hooks 模式形成功能不对等

2026-07-28 TypeScript & React Specialist 对状态管理系统做了全面评审，发现 Flutter 目标在 store 抽象层与其他目标存在功能差距。需要为 Flutter 选择一个成熟的状态管理库来对标 React 的 Zustand。

## Decision Drivers

1. **热度与生态**：必须是 Flutter 社区主流选择，pub.dev 下载量 > 1M，有活跃维护
2. **代码生成友好**：库的 API 设计必须适合机器生成，不依赖复杂的 Widget 树上下文
3. **与 OmniLang Store 模型映射**：signal / getter / action 需要自然映射到库的概念
4. **类型安全**：编译期类型检查，减少运行时错误
5. **测试友好**：生成的代码应易于单元测试
6. **ADR-003 约束**：Flutter 是 P0 目标，投入应当充足

## Decision

**选择 Riverpod 3.0 作为 Flutter 目标的状态管理库。**

### 备选方案评估

| 方案 | 下载量 | 代码生成友好度 | 与 Store 模型匹配 | 决定 |
|------|--------|---------------|-------------------|------|
| **Riverpod 3.0** | ~1.9M 🥇 | 极高（全局声明，无 BuildContext） | 优秀 | ✅ 选用 |
| Bloc 9.0 | ~1.5M | 低（需 Event + State 类） | 一般 | ❌ |
| Provider | 稳定 | 中（依赖 Widget 树） | 中 | ❌ 维护模式 |
| GetX | 下降中 | 中 | 低 | ❌ 仓库已消失 |
| Signals 6.0 | 小众 | 高 | 中 | ❌ 生态太小 |

### Riverpod 入选理由

1. **最高下载量** — 1.9M，Flutter 状态管理第一
2. **全局 Provider 声明** — Provider 不依赖 `BuildContext`，代码生成无需推导 Widget 树插入位置
3. **与 OmniLang Store 天然映射**：

| OmniLang 概念 | Riverpod 对应 |
|--------------|--------------|
| `signal name: Type = default` | `StateProvider<Type>` 或 `NotifierProvider` |
| `getter name: Type = expr` | 派生 Provider，`ref.watch` 自动追踪依赖 |
| `action name(params) { ... }` | Notifier class method |
| `store StoreName { ... }` | 一组相关 Provider + Notifier |

4. **`AsyncValue<T>` 内置** — 统一处理异步 action 的 loading/error/data 状态
5. **Riverpod 自身大量使用代码生成**（`@riverpod` + `riverpod_generator`），与 OmniLang 编译哲学一致
6. **测试友好** — `ProviderContainer.overrides` 可一行覆盖任何 provider

### 架构影响

**变更范围**：
- `omnilang/src/codegen/flutter/mod.rs` — `gen_store_dart()` 重写
- `omnilang/src/codegen/flutter/view.rs` — 组件对 store 的引用改为 `ref.watch()`
- `omnilang/src/codegen/flutter/stmt.rs` — mutation 语句改为 Notifier method 调用
- `omnilang/src/codegen/flutter/expr.rs` — store 字段访问改为 `ref.read()`
- `omnilang/src/codegen/flutter/tests.rs` — 所有 store 测试更新
- `omnilang/src/codegen/tests_regression/parameterized.rs` — Flutter store 测试用例更新

**不影响的组件**：
- OmniLang `.omni` 语法 — 无变化
- IR 层 — 无变化
- Parser / Lowering — 无变化
- 其他编译目标 — 无变化

## Consequences

### Positive

- 生成的 Flutter 代码符合 2026 年社区最佳实践
- 获得 `AsyncValue` 内置异步状态管理
- Provider 全局声明消除 Widget 树依赖
- 与 React/Zustand 目标功能对等
- 编译期类型安全

### Negative

- 引入外部依赖 `flutter_riverpod`（生成的代码需要它作为 runtime dependency）
- `ChangeNotifier` 代码全部废弃重写
- 约 400 行 Flutter codegen + 测试需要改造
- 团队需要学习 Riverpod 概念

### Neutral

- ADR-003 已确认 Flutter 为 P0 目标，投入有保障
- 现有 Flutter 测试覆盖会确保不引入回归

## Implementation Strategy

参见 `writing-plans` 生成的执行计划。

### Reversibility

**可逆性级别**：Medium。`ChangeNotifier` 的旧代码可通过 git revert 恢复。但如果在 Riverpod 路径上投入超过 2 周，回滚成本上升。

### Blast Radius

**影响范围**：仅 `omnilang/src/codegen/flutter/` 目录。IR 层无变化。爆炸半径：Low。

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Necessity Assessment**: 8/10
- 当前 `ChangeNotifier` 方案可用但落后，与 React/Zustand 不对等
- 等待成本：低——现有功能不阻塞，但越晚迁移差异越大
- 必要性：Riverpod 的全局 Provider 模型从根本上更适合代码生成

**Complexity Assessment**: 3/10
- 增加依赖：生成的代码需要 `flutter_riverpod`，不需要 OmniLang 编译器本身引入新 crate
- 代码量变化：重写约 400 行，最终代码量可能更少（Riverpod 比 ChangeNotifier 更简洁）
- 风险：仅影响 Flutter 目标，IR 层未动

**Pragmatic Score**:
- 必要性: 8/10
- 复杂度: 3/10
- 比率: 0.375 ✅ **Ratio < 1.5（极佳）**

**Recommendation**: ✅ 批准——用更简洁的 API 替换手写样板代码，符合 YAGNI。

## Validation

**Acceptance Criteria**:
- [ ] Flutter store 代码生成输出有效的 Riverpod Dart 代码
- [ ] 所有现有 Flutter 测试通过（更新后）
- [ ] `cargo test -p omnilang` 全绿
- [ ] 生成的代码可通过 `dart analyze` 静态检查

## References

- [ADR-003](./ADR-003-codegen-target-prioritization.md) — 代码生成目标优先级（Flutter = P0）
- [State Management Review 2026-07-28](../reviews/) — TypeScript & React Specialist 评审
- [Riverpod 官方文档](https://riverpod.dev/)
- [[omnilang-codegen]] — 后端优先级
