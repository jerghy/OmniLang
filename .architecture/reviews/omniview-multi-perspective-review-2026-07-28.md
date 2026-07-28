# OmniView 多视角架构评审报告

**评审日期**: 2026-07-28
**评审类型**: 多专家并行评审（4 位专家）
**评审范围**: OmniView 编译器 + 运行时 + 前端包

## 评审专家

| 专家 | 评审维度 | 关键发现数 |
|------|---------|-----------|
| Compiler & Language Architect | 编译管线、IR 设计、多目标 codegen、类型系统 | 6 |
| Rust Expert | 所有权、错误处理、性能、模块组织 | 7 |
| TypeScript & React Specialist | JS 运行时、动画库、React codegen、事件系统 | 10 |
| Maintainability Expert | 代码规模、命名、文档、测试覆盖、模块耦合 | 11 |

## 总体评估

OmniView 架构方向正确，代码质量良好。编译器管线设计清晰（Lex→Parse→Semantic→IR→Codegen），FormatExpr/FormatStmt trait 族优雅地抽象了 9 个后端。没有发现 CRITICAL 级别的安全或正确性问题。3 项 HIGH 发现集中在 React codegen 动画 bug、巨型函数、和 Dioxus 后端的运行时 panic 风险。

### 严重级别分布

| 级别 | 数量 |
|------|------|
| CRITICAL | 0 |
| HIGH | 3 |
| MEDIUM | 15 |
| LOW | 11 |

---

## HIGH 发现

### H1. React codegen：@enter 动画在每次状态变更时重播 + 事件监听器泄漏

**来源**: TypeScript & React Specialist
**文件**: `omnilang/src/codegen/react/motion/mod.rs:126-135`

生成的 `useEffect` 把所有 signal 名放入依赖数组，导致：
- @enter 动画随每次 `useState` 变更重播（应该仅 mount 时播放）
- `addEventListener` 反复注册从不 `removeEventListener`，造成监听器泄漏

```tsx
// 当前生成的代码（有问题）
useEffect(() => {
  document.querySelectorAll('[data-motion-enter]').forEach(el => {
    animate(el, kf, opts);  // ← 每次 count/isOpen 变化都重播
  });
  document.querySelectorAll('[data-motion-hover]').forEach(el => {
    el.addEventListener('mouseenter', ...);  // ← 泄漏监听器
  });
}, [count, isOpen]);  // ← 应该用 [] 或 ref
```

**修复方向**: @enter 用 `[]` 依赖；交互式 trigger（@hover/@press/@focus）用 `useEffect` cleanup 函数 + ref 避免直接 DOM 查询。

### H2. `gen_component_tsx` 500 行巨型函数

**来源**: Maintainability Expert
**文件**: `omnilang/src/codegen/react/component/mod.rs:19-543`

一个函数处理了：import 决策 → props 接口 → store hook 路径解析 → signal 收集 → computed value 生成 → motion effect 发射 → JSX 返回。所有逻辑耦合在一起，难以独立测试和修改。

**修复方向**: 提取 `collect_component_imports()`、`gen_computed_values()`、`gen_motion_effects()` 三个独立函数，将主函数缩减至 ~200 行的编排逻辑。

### H3. Dioxus 后端 `todo!()` 导致运行时 panic

**来源**: Maintainability Expert
**文件**: `omnilang/src/codegen/dioxus/expr.rs:249`

```rust
_ => format!("todo!(\"field {}\")", field),  // 新增 EventDataKind 变体时编译期崩溃
```

当 IR 新增 `EventDataKind` 变体或引用新字段时，Dioxus codegen 会在编译时 panic（`not yet implemented`）。

**修复方向**: 替换为 `format!("/* UNSUPPORTED: {} */", field)` 或发出正确的诊断错误，而非 panic。

---

## MEDIUM 发现精选

### 跨目标功能不一致

| # | 问题 | 影响后端 | 文件 |
|---|------|---------|------|
| M1 | `class_map_bindings`（动态 class 绑定）静默丢失 | Flutter, Vue, Dioxus | `flutter/view.rs`, `vue/template.rs`, `dioxus/view.rs` |
| M2 | `@for` 列表空状态 `empty_body` 静默丢失 | Vue, Dioxus | `vue/template.rs:203-216`, `dioxus/view.rs:198-215` |
| M3 | Vue 不支持 `<slot>`（但 Vue 框架原生支持） | Vue | `vue/template.rs:239-241` |

**影响**: 相同 `.omni` 源码在 OmniView/React 上正常渲染，在 Flutter/Vue/Dioxus 上静默丢失功能，产生不同的用户可见行为。

### Rust 后端质量

| # | 问题 | 文件 |
|---|------|------|
| M4 | WASM `OmniCompiler` 缺少 `path_mapping`，多文件 import 静默失败 | `omnilang-wasm/src/lib.rs:570` |
| M5 | `ComponentIR` 双重 clone（初始化 clone + 每次内联 clone） | `component_html.rs:18` |
| M6 | `compile_files` 死代码 + HTML 模板三份副本 | `utils.rs:278-320` |

### 前端运行时

| # | 问题 | 文件 |
|---|------|------|
| M7 | 缓动函数表在 JS runtime + Rust codegen + semantic 三处重复（30+ 个映射值） | `easing.ts`, `keyframes.rs`, `motion.rs` |
| M8 | `__omni_computed` 订阅者累积，每次 run() 创建新 effect 对象不清理旧订阅 | `runtime.rs:56-79` |
| M9 | `@omni/animation` 包零测试覆盖（无 `__tests__/`、无 test script） | `packages/omni-animation/` |
| M10 | Zustand store getter/action 参数类型为 `any`，丢失类型安全 | `react/mod.rs:349,355` |

### 可维护性

| # | 问题 | 文件 |
|---|------|------|
| M11 | `OmniViewGenerator` God object（30+ 字段被 17 个文件访问） | `mod.rs:471-570` |
| M12 | 5 个复合赋值块（Inc/Dec/Mul/Div/Mod）~275 行可简化为 40 行宏 | `react/expr.rs:241-374` |
| M13 | View-tree 扫描函数（has_motion/has_trigger 等）在 omniview/react/flutter 三后端重复 8 次 | 多处 |
| M14 | 3 处函数级 `#[allow(unused_variables)]` 掩盖 feature-gated 参数 | `lib.rs:326,352,375` |
| M15 | `element.rs` 1047 行超出 800 行阈值 | `view/element.rs` |

---

## LOW 发现（11 项）

1. Vue/Dioxus 忽略 `key_expr`（使用硬编码 `:key="index"`）
2. 语义类型检查器对 MethodCall/FieldAccess 回退到 `Any`（设计限制）
3. `build_file_output` 与 `OmniViewCodegen::generate` 的动画 script 标签不一致
4. 每次递归调用分配缩进字符串（可用缓存优化）
5. `write!`/`writeln!` 返回值用 `let _ =` 丢弃（虽正确但噪音多）
6. `emit_store` 分配 signal_names 副本
7. `__omni_bindText` 被生成但从未被调用
8. `build-bundle.js` 用正则剥离 import/export（脆弱，建议换 esbuild）
9. `(window as any).Omni` 类型逃逸
10. FLIP/Hero 动画 style cleanup 在 DOM 提前移除时可能泄漏
11. `keyframe_to_config` 死代码（仅测试使用）
12. 9 个测试文件超 800 行

---

## 建议优先级

### 🔴 立即（下个 session）

1. **修复 H1** — React motion `useEffect` 依赖数组：@enter 用 `[]`，交互式 trigger 用 cleanup
2. **修复 H3** — Dioxus `todo!()` → 优雅降级/诊断错误
3. **修复 M4** — WASM `path_mapping` 一行修复 (`omnilang-wasm/src/lib.rs:570`)

### 🟡 短期（下个 sprint）

4. 补充 Flutter/Vue/Dioxus 的 `class_map_bindings`、`empty_body`、`<slot>` 支持（M1-M3）
5. 拆分 `gen_component_tsx`（H2）：提取 import 收集、computed 生成、motion effect 发射
6. 消除 `ComponentIR` 双重 clone（M5）+ 删除 `compile_files` 死代码（M6）
7. 缓动函数表归一化，单一数据源（M7）
8. 复合赋值宏消除 230 行重复（M12）
9. `@omni/animation` 添加 Vitest 测试（M9）

### 🟢 长期

10. `OmniViewGenerator` 拆分子结构体（JsState, CssState, TreeState）（M11）
11. 提取共享 `view_tree_any_element` 消除 8 个重复 tree-walker（M13）
12. Zustand getter/action 类型安全（M10）
13. `__omni_computed` 重用 effect 对象修复订阅者累积（M8）
14. 拆分 9 个超 800 行的测试文件
15. `#[allow(unused_variables)]` 缩小到参数级别（M14）
16. `element.rs` 拆分为 `events.rs` + `style.rs` 子模块（M15）

---

## 不处理的风险

| 风险 | 来源 | 影响 |
|------|------|------|
| @enter 动画在每次交互时重播 | H1 | 生产组件视觉闪烁、用户体验受损 |
| Dioxus 编译期 panic | H3 | 新增 EventDataKind 时编译器崩溃 |
| WASM 多文件 import 失败 | M4 | Poiesis/在线 playground 多文件编译静默失败 |
| class_map_bindings 跨目标丢失 | M1 | Flutter/Vue/Dioxus 应用中动态样式不生效 |
| 缓动函数三处不同步 | M7 | 新增缓动在一个目标工作、另一个不工作 |
| Zustand getter 类型为 any | M10 | 无 IDE 自动补全、无编译时类型检查 |

## 成功指标

| 指标 | 当前 | 目标 |
|------|------|------|
| 非测试文件超 800 行 | 2 | 0 |
| 函数超 50 行（非测试） | 3+ | 0 |
| `todo!()` in 生产代码 | 1 | 0 |
| 重复 tree-walking 函数 | 8 | 1（共享） |
| 重复复合赋值行数 | ~275 | ~40 |
| 30+ 字段 struct | 1 | 0 |
| 测试文件超 800 行 | 9 | 0 |
| 死函数 | 2 | 0 |
| `#[allow(unused_variables)]`（函数级） | 3 | 0 |
| `@omni/animation` 测试覆盖率 | 0% | ≥80% |
