# React & Flutter Codegen Completion Plan

**Date**: 2026-06-29
**Status**: 已对标审查 + 分层策略明确，等待批准
**Scope**: `omnilang/src/codegen/react.rs` + `flutter.rs`
**Gold Standard**: `omnilang/src/codegen/omniview.rs`

---

## 核心哲学：两层生成策略

OmniView 之所以有自研 signal 运行时、template clone、委托事件等机制，是因为它跑在裸 JS 环境里，必须从零构建。React 和 Flutter 不需要。

```
Layer 1（首选）: 用平台惯用方式 — 生成像手写代码
Layer 2（降级）: 平台无惯用等价物 — 牺牲可读性，保证功能完整
```

**但这不意味着 Layer 2 就可以随便写。** Layer 2 的"牺牲可读性"是相对于完美的手写代码而言——生成的代码仍然要有清晰的注释、一致的命名、合理的结构。只是它可能引入一个自定义的辅助类、一个不太常见的 pattern、或者一个 workaround。

---

## 1. 每个 Feature 的分层归类

### React

| Feature | 层级 | 策略 |
|---------|------|------|
| 信号/状态 | Layer 1 | `useState` — 天然等价，`const [x, setX] = useState(v)` 就是手写标准 |
| 派生值 | Layer 1 | `useMemo(() => expr, deps)` — 手写标准 |
| 副作用 | Layer 1 | `useEffect(() => {...}, deps)` — 手写标准 |
| 插值 `{expr}` | Layer 1 | JSX `{expr}` — 天然等价 |
| 条件 `@if` | Layer 1 | 三元 `{cond ? <A/> : <B/>}` — 手写标准 |
| `wait` 过渡模式 | Layer 2 | 引入 `AnimatePresence`（framer-motion）— 稍微不如纯 CSS 优雅，但声明式 API 仍然可读 |
| 循环 `@for` | Layer 1 | `.map(item => <Item key={...}/>)` — 手写标准 |
| `@for` per-instance state | Layer 1 | React 天然支持（每个 `.map()` 项的组件实例有独立 state） |
| 双向绑定 | Layer 1 | `value={x} onChange={e => setX(e.target.value)}` — 手写标准 |
| `class:` 绑定 | Layer 1 | `className={`base ${cond ? 'active' : ''}`}` 或 `clsx` — 手写标准 |
| `style:` 绑定 | Layer 1 | `style={{ color: val, fontSize: size }}` — 手写标准 |
| 事件处理 | Layer 1 | `onClick={handler}` JSX 属性 — 手写标准 |
| 组件组合 | Layer 1 | `<Child prop={v} onEvent={cb}/>` — 手写标准 |
| Store | Layer 1 | `useStoreName()` 自定义 hook — 手写标准 |
| Store actions | Layer 1 | Hook 返回对象中包含 action 函数 — 手写标准 |
| 简单动画（@enter, @hover, @press, @focus） | Layer 1 | CSS `@keyframes` + `animation` / `transition` — 手写标准 |
| 复杂动画（@timeline, path, stagger） | Layer 2 | framer-motion 的 `useAnimationControls` + `animate()` 序列。不是每个 React 开发者都用 framer-motion，但它是声明式动画的行业标准 |
| @leave 动画 | Layer 2 | framer-motion `AnimatePresence` + `exit` variant。同样非纯 CSS，但是声明式且可读 |
| @in-view 动画 | Layer 1 | `IntersectionObserver`（已有实现） + CSS animation — 手写标准 |
| @scroll/@drag 动画 | Layer 1 | 已有的 WAAPI 实现是合理的低层 API 使用，加注释说明即可 |
| Slot | Layer 1 | `{props.children}` / 命名 slot 通过 props 传递 — 手写标准 |
| throw/try/catch | Layer 1 | 原生 JS 语法，直接生成 |
| @states 状态机 | Layer 2 | `useState` 追踪当前状态 + `switch` 切换 animation。不如 WAAPI `el.animate()` 紧凑，但在 React 中是合理的 |
| Hero 动画 | Layer 2 | framer-motion `layoutId` 或 CSS `view-transition`。两种都不是纯手写常见模式，但都是声明式的 |
| 编辑器桥接 | Layer 2 | 生成 `window.__OMNI_COMPONENTS__` 注册表 + `postMessage` 桥接。这是工具链基础设施，天生不可见，不需要追求业务代码级别的可读性 |

### Flutter

| Feature | 层级 | 策略 |
|---------|------|------|
| 状态字段 | Layer 1 | 类字段 + `setState(() => _x = v)` — 手写标准 |
| 派生值 | Layer 1 | Dart getter `String get fullName => ...` — 手写标准 |
| 插值 `{expr}` | Layer 1 | `Text('$expr')` 或 `Text(expr.toString())` — 手写标准 |
| 条件 `@if` | Layer 1 | `ListenableBuilder(listenable: signal, builder: (_, __) => cond ? a : b)` — Standard Flutter reactive pattern |
| `wait` 过渡模式 | Layer 2 | `AnimatedSwitcher` + 自定义 `transitionBuilder`。不如简单三元可读但不罕见 |
| 循环 `@for` | Layer 1 | `ListView.builder(itemCount: n, itemBuilder: ...)` — 手写标准 |
| `@for` per-instance state | Layer 1 | 每个 item 用独立 `StatefulWidget` — 手写标准 |
| 双向绑定 | Layer 1 | `TextEditingController` + `addListener` 双向同步 — 手写标准 |
| `class:` 绑定 | Layer 1 | 条件 `decoration` / `AnimatedContainer` — 手写标准 |
| `style:` 绑定 | Layer 2 | 映射 CSS 属性到 Flutter widget 参数（padding→EdgeInsets, color→Color 等）。映射表不可避免地有点长，但每个映射都是直观的 |
| 事件处理 | Layer 1 | `GestureDetector(onTap: () => ...)` — 手写标准 |
| 组件组合 | Layer 1 | `ChildWidget(param: val, onEvent: cb)` — 手写标准 |
| Store | Layer 1 | `ChangeNotifier` + 构造函数注入 — 手写标准 |
| Store actions | Layer 1 | `ChangeNotifier` 上的方法，调 `notifyListeners()` — 手写标准 |
| 简单动画（@enter, @hover, @press, @focus） | Layer 1 | `AnimationController` + `Tween` — 手写标准 |
| 复杂动画（@timeline, stagger） | Layer 2 | 自定义 `TimelinePlayer` 类编排 `AnimationController` 序列。多个 controller 协同在 Flutter 中本身就复杂，不是生成代码的问题 |
| @leave 动画 | Layer 2 | dispose 时播放反向动画。正常 Flutter 较少这么做（更常用 `AnimatedSwitcher`），但功能正确 |
| @in-view/@scroll/@drag 动画 | Layer 2 | 已有的 `_OmniInViewWidget`/`_OmniScrollDrivenWidget`/`_OmniDragWidget` 辅助类。这些类的存在就已经是 Layer 2（手写不会有这些命名），但它们封装了非平凡的 IntersectionObserver/scroll/gesture 逻辑 |
| throw/try/catch | Layer 1 | 原生 Dart 语法，直接生成 |
| @states 状态机 | Layer 2 | `AnimatedBuilder` + switch 切换 tween。不如 WAAPI 紧凑，但可以工作 |
| Hero 动画 | Layer 1 | Flutter 原生 `Hero` widget — 手写标准 |
| Slot | Layer 2 | Widget placeholder 通过构造函数传入。Flutter 没有原生 slot 概念，这是最干净的等价实现 |
| 编辑器桥接 | Layer 2 | 组件注册表 + 平台 channel。工具链基础设施 |

---

## 2. Layer 2 实现规范

Layer 2 不是"随便写"。必须满足：

1. **在生成代码中写注释解释为什么这样写**（参考 OmniView 已有的 `/// ── Component: Name ──` 风格）
2. **辅助类/函数命名统一前缀**：React 用 `__omni_` 或抽出为 `omni-helpers.ts`；Flutter 已有的 `_OmniInViewWidget` 等延续 `_Omni` 前缀
3. **辅助代码集中放在文件尾部或单独的 helper 文件**，不混入业务逻辑区
4. **依赖声明**：如果引入了 framer-motion，在输出顶部添加 `// Requires: framer-motion` 注释

---

## 3. 实施计划

### Phase 1: Layer 1 渲染修复（P0）— 3-4 天

**目标**：生成的代码看起来像手写、编译通过、渲染正确

- React `Interpolation` → `{expr}` JSX 表达式
- Flutter `Interpolation` → `Text(expr.toString())`
- Flutter `@if` → `ListenableBuilder` 包裹三元
- Flutter `@for` → `ListView.builder`
- 两个目标：双向绑定（React: `value`+`onChange`，Flutter: `TextEditingController`）
- 两个目标：throw/try/catch/BlockStmt

### Phase 2: Store & 事件 (P1) — 2-3 天

- React: hook 中补 actions、修复 getter 依赖数组、store_action 事件
- Flutter: `ChangeNotifier` 中补 actions、`Provider.of`/构造函数接线

### Phase 3: class:/style: & @for 完善 (P1) — 2-3 天

- React: `className` 模板、`style` 对象、@for key + empty
- Flutter: 条件 decoration、CSS→Flutter 映射表、@for key + empty

### Phase 4: 组件组合 (P1) — 3-4 天

- React: `component_map` 查询 → `<Child prop={v} />` JSX 引用
- Flutter: `component_map` 查询 → `ChildWidget(param: v)` 构造函数调用
- Props 接线、Events 接线

### Phase 5: 动画 (P2) — 3-4 天

- React Layer 1: @enter/@hover/@press/@focus → CSS animation/transition
- React Layer 2: @timeline/@leave/path/stagger → framer-motion
- React: 用 motion_ref 替换硬编码关键帧
- Flutter Layer 1: 补全 @enter/@hover/@press/@focus → AnimationController
- Flutter Layer 2: @timeline → TimelinePlayer 类、@leave → dispose 反向播放
- Flutter: 补全 Animate/Leave/Timeline 触发器

### Phase 6: 收尾 (P2-P3) — 2-3 天

- Slot、@states、Hero、prop_defaults、EventEmitStmtIR
- sanitize_js_ident 全局应用
- SourceMap 注释
- 测试覆盖

---

## 4. 预估

| Phase | 天数 |
|-------|------|
| Phase 1: Layer 1 渲染修复 | 3-4 |
| Phase 2: Store & 事件 | 2-3 |
| Phase 3: class:/style: & @for 完善 | 2-3 |
| Phase 4: 组件组合 | 3-4 |
| Phase 5: 动画（Layer 1 + Layer 2） | 3-4 |
| Phase 6: 收尾 | 2-3 |
| **总计** | **15-21 天** |

---

_审查完成于 2026-06-29。两次 code-reviewer 对标审查 + 分层策略明确。_
