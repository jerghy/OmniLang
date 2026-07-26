# OmniView 框架修复计划

*生成: 2026-07-26 | 基于深入代码分析后的修正版本*

## 前一份报告的勘误

深入阅读 parser AST、view parser、IR types、OmniView codegen 后，以下前次报告中的问题被**推翻**：

| 前次错误判断 | 实际情况 | 证据 |
|-------------|---------|------|
| "模板不支持 `{user.getName()}`" | ❌ 支持。view parser 调用 `parse_expr()` 解析 `{...}` 插值，Pratt parser 支持 `MethodCall`/`FieldAccess`/`FuncCall` | `parser/view/mod.rs:110-111, 130-133` |
| "effect 无 cleanup" | ❌ 有。`__omni_effect()` 返回 `() => e._deps.forEach(d => d())` 清理函数 | `codegen/omniview/runtime.rs:81-87` |
| "无 ref 系统" | ❌ 有。`RefDecl` 在 AST 中定义并被 parser 解析 | `parser/ast/view.rs:19,62-66` |
| "无 @unmount 生命周期" | ❌ 有。`LifecycleKind::Unmount` 被完整解析和 codegen | `parser/ast/view.rs:78`, `codegen/omniview/store.rs:338-343` |
| "fn vs action 语义不明确" | ❌ 明确。`fn` 是组件级纯函数，`action` 是 store 级可异步修改状态的方法 | `ir/types.rs:1052 vs 1084`, `parser/ast/view.rs:83-90` |

---

## 经过验证的真实问题

### P0 — 功能缺口

#### 1. OmniView codegen 中 `@effect` 处理器为空

**现状：** Parser 完整支持 `@effect(signals) { body }`，React codegen 完整生成 `useEffect`，但 OmniView codegen 的 `LifecycleKind::Effect` 分支是空的：
```rust
// codegen/omniview/store.rs:345
LifecycleKind::Effect => {}
```

**影响：** `.omni` 源码中的 `@effect` 在 OmniView 预览中不产生任何代码——开发者看到的效果和 React 产出不一致。

**参考方案：**
- Haxe Reflaxe 的"EIE normalization"：在编译时确定 effect 的依赖关系，生成显式的 subscribe/unsubscribe 代码
- Solid.js `createEffect` + `onCleanup`：用 `() => dispose()` 返回模式管理 effect 生命周期

**修复方向：** 在 `store.rs` 中实现 `LifecycleKind::Effect` 分支，利用已有的 `__omni_effect()` 运行时函数（它已经返回 cleanup fn），生成类似如下代码：
```javascript
const _dispose_effect = __omni_effect(() => {
  // effect body
});
// 注册到组件的 unmount 清理列表
```

#### 2. 代码生成后端间存在大量重复逻辑

**现状：** React/Vue/OmniView 三个前端 codegen 各自独立实现了：
- 表达式翻译 (`ExprIR → string`)
- 事件处理代码生成 (`EventHandlerIR → code`)
- CSS 样式规则生成 (`StyleRuleIR → CSS`)
- 条件渲染 / 列表渲染逻辑

**参考方案——Haxe Reflaxe 模式：**
```
┌─────────────────────────────────────┐
│   BaseCompiler (共享基础设施)        │
│   compileClassImpl()    ← 目标实现   │
│   compileExpressionImpl()  ← 目标实现 │
│   compileEnumImpl()     ← 目标实现   │
└─────────────────────────────────────┘
        ↑
        │ 继承
┌───────┴───────┐  ┌───────┴───────┐
│  C++ Compiler  │  │  C# Compiler   │
│  (只需实现3个方法) │  │  (只需实现3个方法) │
└───────────────┘  └───────────────┘
```

**修复方向：** 提取 `UiCodegen` trait，将 UI 目标的共享逻辑集中：
```rust
pub trait UiCodegen: Codegen {
    fn translate_event_handler(&self, h: &EventHandlerIR) -> String;
    fn translate_binding(&self, b: &BindingIR) -> String;
    fn translate_style_rule(&self, s: &StyleRuleIR) -> String;
}
```

React/Vue/OmniView 只需实现差异部分，框架自动处理循环/条件/组件组合。

### P1 — 代码健康

#### 3. 大文件需要拆分

| 文件 | 行数 | 目标 | 策略 |
|------|------|------|------|
| `codegen/omniview/html.rs` | ~58K | 8-12 模块 | 按元素类型拆分 (element/div, element/input, element/form...) |
| `codegen/omniview/motion_triggers.rs` | ~41K | 按 trigger 类型 | 拆分 (trigger/hover, trigger/press, trigger/focus...) |
| `codegen/omniview/store.rs` | ~26K | 3-4 模块 | 拆分 (store/signal, store/action, store/lifecycle, store/component) |
| `codegen/omniview/tests.rs` | ~43K | 按功能域 | 拆分到各子模块的 `#[cfg(test)]` 中 |

**参考方案——Rust 模块拆分模式（已验证）：**
```
codegen/omniview/
├── mod.rs              # 公共 API、OmniViewGenerator struct、re-exports
├── component/          # 组件级代码生成
│   ├── mod.rs
│   ├── signal.rs       # signal/computed/action codegen
│   ├── lifecycle.rs    # @mount/@unmount/@effect
│   └── function.rs     # fn 代码生成
├── view/               # 视图模板代码生成
│   ├── mod.rs
│   ├── element.rs      # 元素生成
│   ├── text.rs         # 文本/插值
│   ├── condition.rs    # @if/@else
│   └── list.rs         # @for/@empty
├── style/              # 样式代码生成
│   ├── mod.rs
│   ├── css.rs          # CSS 规则
│   ├── token.rs        # Token 引用
│   └── variant.rs      # Variant 展开
├── motion/             # 动画代码生成
│   ├── mod.rs
│   ├── trigger.rs      # @enter/@leave/@hover/@press 等
│   ├── timeline.rs     # @timeline
│   └── states.rs       # states: { idle/hover/pressed }
├── store/              # Store 代码生成
│   ├── mod.rs
│   ├── signal.rs       # Store signal
│   └── action.rs       # Store action
├── runtime.rs          # 内联 JS 运行时
├── utils.rs            # 工具函数
└── tests/              # 分散在各子模块的 #[cfg(test)]
```

**拆分安全保证：**
1. 父 `mod.rs` 用 `pub use` re-export 所有原公共 API
2. 子模块用 `use super::*;` 访问 OmniViewGenerator 的字段
3. 每一步拆分后运行 `cargo test` + `cargo clippy` 验证

### P2 — 架构增强

#### 4. IR 层增加跨目标优化

**现状：** IR 层主要负责 AST→IR 的翻译，不做优化。Dead code elimination、constant folding 完全依赖各 codegen 后端自行处理或目标语言的编译器。

**参考方案——Svelte 编译时分析：**
Svelte 在编译时做：
- 静态子树检测（不变的部分生成静态字符串）
- 细粒度响应式追踪（哪个变量变了→更新哪个 DOM 节点）
- 编译时 DCE（未使用的 import/component 不出现在输出中）

**修复方向：** 在 `ir/lower/` 中增加优化 pass：
```rust
// ir/lower/optimize.rs
pub fn optimize(ir: &mut IRProgram) {
    remove_unused_signals(ir);      // 删除未被 view/computed/effect 引用的 signal
    inline_trivial_computeds(ir);   // 内联简单 computed（单表达式无副作用）
    hoist_static_elements(ir);      // 将不变的 HTML 标记为静态
}
```

这些优化 pass 对所有 10 个 codegen 后端都有益。

#### 5. 增量编译支持

**现状：** 每次 `compile()` 都重新 lex/parse/semantic/IR/codegen 全流程。在 Poiesis 编辑器中，每次按键都触发完整编译。

**参考方案——Svelte vite-plugin-svelte：**
使用文件 hash 缓存编译结果，只有内容变化时才重新编译。支持 component-level hot reload。

**修复方向：**
1. WASM 侧：为 `compile()` 增加文件级缓存（基于 source hash）
2. Poiesis 侧：用 `requestAnimationFrame` 做 debounce，300ms 内的连续输入合并为一次编译
3. 最小实现：先做文件级缓存（整个 .omni 文件），后续可做 per-component 缓存

### P3 — 探索性改进

#### 6. 泛型组件（语言级功能）

**现状：** 组件不支持类型参数，无法写 `component ListView<T>`。

**参考方案——Flutter/SwiftUI 泛型组件：**
```dart
// Flutter
class ListView<T> extends StatelessWidget {
  final List<T> items;
  final Widget Function(T) itemBuilder;
}
```
```swift
// SwiftUI  
struct ListView<T: Identifiable>: View {
    let items: [T]
    let rowContent: (T) -> any View
}
```

**修复方向：** 
1. Parser: 支持 `component Name<T, U> { ... }` 语法
2. IR: 添加 `ComponentIR.type_params: Vec<TypeParam>`
3. Codegen: 各目标按自身泛型语法生成（React: TS generics, Flutter: Dart generics, OmniView: 运行时类型擦除）

#### 7. 细粒度响应式（OmniView 运行时优化）

**现状：** 当前 signal 变化→所有订阅的 effect 重新运行→整个组件的 DOM 更新。这是"组件级"响应式粒度。

**参考方案——Marko 6 Fine-Grained Bundling：**
Marko 编译器在编译时分析每个 `let` 变量→哪些 DOM 节点依赖它→只生成更新那些特定节点的代码。

**修复方向：** 在 OmniView codegen 中：
1. 为每个 `{signal}` 插值生成唯一的 text-node-id
2. signal setter 中，除了通知 effect，直接更新对应的 DOM text node
3. 跳过整个组件的 diff/re-render

---

## 优先级排序

| # | 任务 | 优先级 | 预计工作量 | 依赖 |
|---|------|--------|-----------|------|
| 1 | OmniView `@effect` codegen 实现 | P0 | 1-2 天 | 无 |
| 2 | 提取 UiCodegen trait | P0 | 3-5 天 | 无 |
| 3 | 拆分 `html.rs` (58K→8 模块) | P1 | 2-3 天 | #2 后做更好 |
| 4 | 拆分 `motion_triggers.rs` (41K→5 模块) | P1 | 2-3 天 | #2 后做更好 |
| 5 | 拆分 `store.rs` (26K→4 模块) | P1 | 1-2 天 | 无 |
| 6 | IR 层优化 pass | P2 | 3-5 天 | #2 后做 |
| 7 | WASM 增量编译（文件缓存） | P2 | 2-3 天 | 无 |
| 8 | 泛型组件 | P3 | 1-2 周 | 需要语言设计讨论 |
| 9 | 细粒度响应式 | P3 | 1-2 周 | #2 #6 后做 |

## 执行建议

1. **先 P0 后 P1**：修复 effect codegen 和提取 trait 是阻断性问题，必须优先
2. **#2 在 #3/#4 之前**：提取 trait 后再拆分文件，避免重复工作
3. **每个任务独立 PR**：一个 PR 只做一件事，便于 review 和回滚
4. **CI gate**：每个 PR 后运行全量 `cargo test --all-features` + `cargo clippy`

## 参考来源

1. Haxe Reflaxe: BaseCompiler 三方法模式 + Smart DCE + EIE Normalization (github.com/SomeRanDev/reflaxe)
2. Rust 模块拆分模式: SRP + pub use re-export + cargo-check 安全验证
3. Svelte 5: 编译时依赖分析 + Runes 信号系统 (svelte.dev)
4. Marko 6: Fine-grained bundling + 表达式级响应式 (markojs.com)
5. OmniLang 源码: parser/ast/view.rs, parser/view/mod.rs, ir/types.rs, codegen/omniview/runtime.rs, codegen/omniview/store.rs
