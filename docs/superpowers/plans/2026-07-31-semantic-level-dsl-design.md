# OmniLang 语义级 DSL 设计文档

## Context

5 专家架构评审发现：React/Flutter 生产目标与 OmniView 调试运行时之间存在大量 feature gap。根本原因不是"特性缺失"，而是 **OmniLang 当前是 OmniView-first 设计**——Ref API 是 DOM 形状的、动画只测了 OmniView、Design Token 只有 Web CSS 路径。

用户要求：**OmniLang 必须是语义级 DSL**——`.omni` 代码描述"我想做什么"，编译器理解意图后，为每个目标生成对应的惯用代码。React/Flutter 必须输出生产级可维护代码。

核心原则：
1. **官方 API 优先** — React/Flutter 各自用官方/标准 API
2. **热门库其次** — Zustand (React), Riverpod (Flutter)
3. **无法实现的提出来讨论** — 可能删除或降级

---

## 1. Design Philosophy

### 1.1 语义级 DSL 原则

```
.omni 源码 ≠ 指令序列
.omni 源码 = 意图声明

编译器的工作 = 理解意图 → 为目标平台生成惯用代码
```

**之前（OmniView 中心）**：
```
.omni: box.style.transform = "translate(100px, 0)"
     → OmniView: el.style.transform = "translate(100px, 0)"  ✅
     → React: ref.current.style.transform = "..."            勉强
     → Flutter: ??? GlobalKey 拿不到 Widget 属性             ❌ 死路
```

**之后（语义驱动）**：
```
.omni: transform = {x}px, {y}px  @onPointerMove={drag}
     → 语义: "这个元素要跟随拖拽移动"
     → OmniView: pointer events + el.style.transform
     → React: ref + onPointerMove + state-driven style transform
     → Flutter: GestureDetector.onPanUpdate + Transform.translate + setState
```

### 1.2 生成代码原则

```
.omni 源码 = 意图声明
编译器的工作 = 理解意图 → 生成该平台开发者会手写的代码
生成代码 ≠ OmniView 模式的翻译
生成代码 = 该平台上最地道的写法
```

**错误做法**：生成 `setTheme(name)` 包装函数、`__omni_*` 前缀的辅助函数、OmniView 风格的 API 桥接。

**正确做法**：
- `.omni: theme = "dark"` → Flutter 生成 `ref.read(themeModeProvider.notifier).state = ThemeMode.dark`，React 生成 `document.documentElement.setAttribute('data-theme', 'dark')`
- `.omni: ref.focus()` → Flutter 生成 `_focusNode.requestFocus()`，React 生成 `ref.current.focus()`
- `.omni: store.count` → Flutter 生成 `ref.watch(storeProvider.select(s => s.count))`，React 生成 `useStore(s => s.count)`

| 层级 | 范围 | 示例 |
|------|------|------|
| **语义原语** | `.omni` 语法表达的意图 | "播放进入动画"、"绑定元素引用"、"切换主题" |
| **IR 层** | 平台无关的意图建模 | `TriggerIntent::Enter`, `RefDeclIR`, `DesignIR` |
| **Codegen 层** | 每个目标的具体实现 | WAAPI vs AnimationController, useRef vs GlobalKey |

Codegen 层**不要求相同输出，只要求等价行为**。

### 1.3 目标定义

| 目标 | 角色 | 输出 | 约束 |
|------|------|------|------|
| **OmniView** | 调试/开发运行时 | 单文件 HTML + inline runtime | 零外部依赖，iframable |
| **React** | Web 生产 | `.tsx` + `.module.css` + store hooks | TypeScript strict mode，eslint-plugin-react-hooks |
| **Flutter** | 移动端生产 | `.dart` widget + store | `dart analyze` clean，`flutter analyze` clean |

---

## 2. Element Reference System（元素引用）

### 2.1 语义定义

Ref 系统提供**对元素的非响应式访问**，用于以下意图：

| 语义原语 | 意图 | 频率需求 |
|---------|------|---------|
| **聚焦控制** | 让元素获得/失去键盘焦点 | 低频 |
| **滚动控制** | 程序化滚动容器到指定位置 | 低频 |
| **尺寸测量** | 读取元素的布局尺寸/位置 | 中频 |
| **文本选区** | 选中输入框中的文本 | 低频 |
| **表单值** | 读取/写入表单控件值 | 中频 |
| **指针捕获** | 锁定指针事件到此元素 | 高频（Flutter：v1 diagnostic） |

### 2.2 IR 设计（新增，跨平台关键）

当前 IR 中 ref 操作是通过 `MethodCallExpr` 表达的（`box.style.transform = ...`），这意味着语义验证在 codegen 层才做，且无法区分合法/非法操作。需要新增：

```rust
// crates/omnilang-core/src/ir/types.rs — 新增
pub enum RefOpIR {
    Focus,
    Blur,
    ScrollTo { x: Option<ExprIR>, y: Option<ExprIR> },
    ScrollBy { dx: Option<ExprIR>, dy: Option<ExprIR> },
    MeasureWidth,
    MeasureHeight,
    MeasurePosition,
    Select,
    SetSelection { start: ExprIR, end: ExprIR },
    Value,            // 读取表单值
    SetValue { value: ExprIR },  // 设置表单值
    CapturePointer { id: ExprIR },
    ReleasePointer { id: ExprIR },
}

// ExprIR 新增变体（替代对 ref 的 MethodCallExpr）:
RefOpExpr { ref_name: String, op: RefOpIR }
```

**新增语义 Pass** `semantic/validate_ref_ops.rs`：
- 扫描所有 `MethodCallExpr` 和 `FieldAccessExpr` 的目标，检查是否是已声明的 ref
- 合法操作 → 改写为 `RefOpExpr`（平台无关）
- 非法操作（`.style.*`, `.innerHTML`, `.querySelector`, `.getBoundingClientRect()` 等）→ **compile error** + 迁移提示

这比在 codegen 层做验证更早、更可靠——语义层就知道哪些 ref 操作是合法的，codegen 只负责映射。

### 2.3 语法

```omni
component Example {
  ref inputBox: Element        // 声明引用
  ref scrollPane: Element

  fn handleClick() {
    inputBox.focus()           // 聚焦
    inputBox.select()          // 全选文本
    scrollPane.scrollTo(0, 0)  // 滚动到顶部
  }

  <TextField @ref={inputBox} placeholder="输入..." />
  <ScrollView @ref={scrollPane}>
    <LongContent />
  </ScrollView>
}
```

### 2.3 完整 API 表（语义原语 → 目标映射）

| `.omni` API | 语义 | React 实现 | Flutter 实现 | OmniView 实现 |
|-------------|------|-----------|-------------|--------------|
| `ref.focus()` | 聚焦元素 | `ref.current.focus()` | `FocusNode.requestFocus()` | `el.focus()` |
| `ref.blur()` | 失焦 | `ref.current.blur()` | `FocusNode.unfocus()` | `el.blur()` |
| `ref.scrollTo(x, y)` | 滚动到绝对位置 | `ref.current.scrollTo({left: x, top: y, behavior: 'smooth'})` | `scrollController.animateTo(Offset(x, y), ...)` | `el.scrollTo(x, y)` |
| `ref.scrollBy(dx, dy)` | 相对滚动 | `ref.current.scrollBy({left: dx, top: dy, behavior: 'smooth'})` | `scrollController.animateTo(currentOffset + delta)` | `el.scrollBy(dx, dy)` |
| `ref.select()` | 全选文本 | `ref.current.select()` | `textController.selection = TextSelection(baseOffset: 0, extentOffset: text.length)` | `el.select()` |
| `ref.setSelection(start, end)` | 选区 | `ref.current.setSelectionRange(start, end)` | `textController.selection = TextSelection(baseOffset: start, extentOffset: end)` | `el.setSelectionRange(start, end)` |
| `ref.width` | 元素宽度（只读）| `ref.current.offsetWidth` | `renderBox.size.width` | `el.offsetWidth` |
| `ref.height` | 元素高度（只读）| `ref.current.offsetHeight` | `renderBox.size.height` | `el.offsetHeight` |
| `ref.position` | 相对于视口的位置 | `ref.current.getBoundingClientRect()` → `{x, y}` | `renderBox.localToGlobal(Offset.zero)` → `Offset` | `el.getBoundingClientRect()` → `{x, y}` |
| `ref.capturePointer(id)` | 锁定指针 | `ref.current.setPointerCapture(id)` | **diagnostic: 不支持**（需要自定义 RenderObject） | `el.setPointerCapture(id)` |
| `ref.releasePointer(id)` | 释放指针 | `ref.current.releasePointerCapture(id)` | **diagnostic: 不支持** | `el.releasePointerCapture(id)` |

### 2.4 已删除的 DOM 风格 API

以下 API **不提供**，由信号 + style 绑定 + 动画系统替代：

| 删除的 API | 替代方案 |
|-----------|---------|
| ~~`ref.style.transform`~~ | `style:transform="{x}px, {y}px"` 绑定信号 |
| ~~`ref.style.opacity`~~ | `style:opacity="{v}"` 绑定信号 |
| ~~`ref.style.*`（全部）~~ | `style:*` + 信号 / 动画 |
| ~~`ref.innerHTML`~~ | 安全风险，不暴露 |
| ~~`ref.addEventListener`~~ | `@on*` 事件绑定语法 |
| ~~`ref.querySelector`~~ | 违反组件封装 |
| ~~`ref.classList`~~ | `class:` 绑定语法 |
| ~~`ref.parentElement`~~ | 违反组件封装 |
| ~~`ref.offsetTop/Left`~~ | 用 `ref.position` 替代 |

### 2.5 Ref 绑定

| 场景 | 实现 |
|------|------|
| 原生元素 `@ref={name}` | React: `ref={nameRef}`（`useRef` + JSX ref prop）；Flutter: `key: nameKey`（GlobalKey） |
| 自定义组件 `@ref={name}` | React: `ref` 作为普通 prop 传递（React 19 已废弃 `forwardRef`）；Flutter: 组件最外层 widget 绑 GlobalKey |
| `@for` 内的 `@ref` | React: ref callback 数组 `ref={(el) => { if (el) refs[i] = el; }}`；Flutter: per-item GlobalKey 数组 `Map<int, GlobalKey>` |
| 同一元素多个 ref | ❌ compile error |

**React 19 关键变化**：`forwardRef` 已废弃。`ref` 现在是普通 prop：

```tsx
// React 19 方式（生成的代码）:
function CustomInput({ ref, placeholder, value }: CustomInputProps) {
  return <input ref={ref} placeholder={placeholder} value={value} />;
}

// 消费侧（不变）:
<CustomInput ref={inputRef} placeholder="..." value={text} />
```

不再需要 `React.forwardRef` 包裹或 `displayName` 赋值。

### 2.6 Flutter Ref 实现细节

```dart
// .omni: ref scrollPane: Element
//       <ScrollView @ref={scrollPane}>
//       scrollPane.scrollTo(0, 100)

// 生成的 State 类:
class _CompState extends ConsumerState<Comp> {
  // 每个 ref 生成对应的 GlobalKey + 按需的 Controller
  final GlobalKey _scrollPaneKey = GlobalKey();
  ScrollController? _scrollPaneScrollCtrl;   // 仅当 ref 用到 scrollTo/scrollBy 时生成
  FocusNode? _scrollPaneFocusNode;           // 仅当 ref 用到 focus/blur 时生成

  @override
  void dispose() {
    _scrollPaneScrollCtrl?.dispose();
    _scrollPaneFocusNode?.dispose();
    super.dispose();
  }

  void _initScrollPane() {
    // 从 GlobalKey 获取 widget state 和 controller（在 build 后调用）
    final state = _scrollPaneKey.currentState;
    if (state is ScrollableState) {
      _scrollPaneScrollCtrl = state.widget.controller
          ?? ScrollController();  // 如果没有则创建
    }
  }

  void _scrollPane_scrollTo(double x, double y) {
    _scrollPaneScrollCtrl?.animateTo(
      y,  // Flutter ScrollController 默认垂直滚动
      duration: Duration(milliseconds: 300),
      curve: Curves.easeInOut,
    );
  }

  // 尺寸和位置测量 — 必须用 addPostFrameCallback
  double? _scrollPaneWidth;
  Offset? _scrollPanePosition;

  void _measureScrollPane() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final box = _scrollPaneKey.currentContext
          ?.findRenderObject() as RenderBox?;
      if (box != null) {
        setState(() {
          _scrollPaneWidth = box.size.width;
          _scrollPanePosition = box.localToGlobal(Offset.zero);
        });
      }
    });
  }
}
```

**GlobalKey 生产使用规范**（Flutter 官方最佳实践）：
- **类型化**：`GlobalKey<ScrollableState>` 而非裸 `GlobalKey`
- **null 安全**：`currentContext` 和 `currentState` 始终 null-check
- **测量延迟**：`findRenderObject()` 只能在 layout 后调用 → 必须包裹 `addPostFrameCallback`
- **按需生成**：只为此 ref 实际用到的操作生成对应的 Controller/Node
- **数量控制**：一个组件内 GlobalKey 数应 ≤ 5，而非每个元素一个

---

## 3. Animation System（动画系统）

### 3.1 语义模型

动画的语义是：**"当某个条件触发时，让元素的某些属性随时间变化"**。

```
语义 = 触发器（何时） + 关键帧（怎么变） + 播放控制（变多久/几次）
```

### 3.2 触发器（Triggers）

| 触发器 | 语义 | React 实现 | Flutter 实现 | OmniView 实现 |
|--------|------|-----------|-------------|--------------|
| `@enter` | 元素首次出现时正向播放 | `useEffect([], [])` + WAAPI `Element.animate()` | `initState` + `AnimationController.forward()` | MutationObserver + WAAPI |
| `@hover` | 指针进入正向播放，离开反向 | `onMouseEnter`/`onMouseLeave` + WAAPI play/reverse | `MouseRegion(onEnter/onExit)` + controller forward/reverse | mouseenter/leave + WAAPI |
| `@press` | 按下正向，释放反向 | `onPointerDown`/`onPointerUp` + WAAPI | `GestureDetector(onTapDown/onTapUp)` + controller fwd/rev | pointerdown/up + WAAPI |
| `@focus` | 获得焦点时播放 | `onFocus` + WAAPI | `FocusNode.addListener` + controller | focusin + WAAPI |
| `@in-view` | 滚入视口时播放 | `IntersectionObserver` + WAAPI | `_OmniInViewWidget` 可见性检测器 | IntersectionObserver |
| `@scroll` | 滚动位置驱动动画进度 | scroll event + WAAPI progress scrubbing | `_OmniScrollDrivenWidget` + `ScrollController` | scroll event |
| `@drag-horizontal` | 水平拖拽距离驱动动画 | pointer events + 距离→进度映射 | `_OmniDragWidget` + `GestureDetector.onPanUpdate` | pointer events |
| `@drag-vertical` | 垂直拖拽距离驱动动画 | pointer events + 距离→进度映射 | `_OmniDragWidget` + `GestureDetector.onPanUpdate` | pointer events |

### 3.3 关键帧模型

```omni
motion fadeIn {
  over: 300ms
  easing: ease-out

  keyframe {
    0% { opacity: 0; scale: 0.95; }
    50% { opacity: 0.5; scale: 0.98; }
    100% { opacity: 1; scale: 1; }
  }
}
```

**语义**：定义一组属性在时间点上的值。每个属性独立插值。

| 概念 | React (WAAPI) | Flutter | OmniView |
|------|--------------|---------|----------|
| 多 stop | `KeyframeEffect` offset 数组 | **每个属性独立 `TweenSequence`** | WAAPI offset 数组 |
| 多属性 | WAAPI 原生（CSS 属性列表） | **每个属性独立 `Animatable<T>` + `Animation<T>`** | WAAPI 原生 |
| 数值属性 (opacity) | `opacity: [0, 1]` | `Tween<double>(begin: 0, end: 1)` | `opacity: [0, 1]` |
| 变换属性 (scale) | `transform: scale(0.95)` → `scale(1)` | `Tween<double>(begin: 0.95, end: 1.0)` → `Transform.scale` | CSS transform |
| 尺寸属性 (width) | `width: 0 → 100px` | ⚠️ diagnostic: Flutter 动画不支持 layout 属性 | CSS width |

**Flutter 多属性多 stop 实现**：

```dart
// .omni: keyframe { 0% { opacity: 0; scale: 0.95 } 100% { opacity: 1; scale: 1 } }
// 生成代码:
late final AnimationController _fadeInController;
late final Animation<double> _fadeIn_opacity;
late final Animation<double> _fadeIn_scale;

void _initFadeIn(TickerProvider vsync) {
  _fadeInController = AnimationController(
    duration: const Duration(milliseconds: 300),
    vsync: vsync,
  );
  _fadeIn_opacity = Tween<double>(begin: 0.0, end: 1.0).animate(
    CurvedAnimation(parent: _fadeInController, curve: Curves.easeOut),
  );
  _fadeIn_scale = Tween<double>(begin: 0.95, end: 1.0).animate(
    CurvedAnimation(parent: _fadeInController, curve: Curves.easeOut),
  );
}
```

### 3.4 播放控制（Playback）

| 属性 | 语义 | React (WAAPI) | Flutter |
|------|------|--------------|---------|
| `duration` | 单次播放时长 | `options.duration` | `AnimationController(duration: ...)` |
| `loop_count: N` | 重复 N 次 | `options.iterations: N` | `controller.repeat()` 或手动计数 |
| `direction: normal` | 正向播放 | `options.direction: 'normal'` | `controller.forward()` |
| `direction: reverse` | 反向播放 | `options.direction: 'reverse'` | `controller.reverse()` |
| `direction: alternate` | 来回交替 | `options.direction: 'alternate'` | `controller.repeat()` + `TweenSequence` 对称 |
| `delay: Nms` | 延迟开始 | `options.delay: N` | `Future.delayed(delay, () => controller.forward())` |
| `fill: forwards` | 保持最终状态 | `options.fill: 'forwards'` | `Tween.end` 停留在 end 值 |
| `fill: backwards` | 开始前保持初始状态 | `options.fill: 'backwards'` | `Tween.begin` 开始前显示 begin 值 |
| `fill: both` | 前后都保持 | `options.fill: 'both'` | 组合 |
| `paused: true` | 初始暂停 | 不调用 play，等待外部触发 | `controller` 创建后不自动 forward |

### 3.5 缓动函数

| 缓动类型 | React (WAAPI) | Flutter |
|---------|--------------|---------|
| 命名缓动 (29 种) | `options.easing: 'ease-in-out'` | `Curves.easeInOut` ← 共享映射表 `NAMED_EASINGS` |
| `cubic-bezier(x1, y1, x2, y2)` | `options.easing: 'cubic-bezier(...)'` | `Cubic(x1, y1, x2, y2)` |
| `back(in/out/inout)` | JS 实现或 polyfill | `Curves.easeOutBack` 等 |
| `elastic(in/out/inout)` | JS 实现或 polyfill | `Curves.easeOutElastic` 等 |
| `steps(N, start/end)` | `options.easing: 'steps(N, end)'` | ⚠️ diagnostic: Flutter 无原生 steps easing |

### 3.6 滚动驱动动画

```omni
motion parallax {
  @scroll start: 0 end: 1000 scrub: true {
    keyframe { 0% { translateY: 0; } 100% { translateY: -200; } }
  }
}
```

> ✅ **已实现**（Task 5.1，v2 BNF `scroll_params ::= 'start:' expr 'end:' expr 'scrub:' bool`）：
> 控制维度以冒号形式子参数挂在触发器关键字与 `={motion}` 绑定之间——
> `@scroll start: 0 end: 1000 scrub: true ={parallaxScroll}`（内联动画体语法仍按
> ADR-053 不解析，见 BNF spec §3.2）。三后端均消费：React/OmniView 生成
> 数字滚动区间映射（`progress = (scrollY - start) / (end - start)`，缺省回落
> 视口相对字符串配置 `"top 80%"`/`"bottom 20%"`）；Flutter `_OmniScrollDrivenWidget`
> 接收类型化 `start:`/`end:` 构造参数。

**语义**：动画进度由滚动位置线性映射（scrub）。`start`/`end` 定义映射范围。

| 属性 | React | Flutter | OmniView |
|------|-------|---------|----------|
| `start` | 元素顶部到达此视口位置时开始映射 | 同 | 同 |
| `end` | 元素底部到达此视口位置时结束映射 | 同 | 同 |
| `scrub: true` | 进度 = (scrollPos - start) / (end - start) | 同 | 同 |

### 3.7 拖拽驱动动画

```omni
motion swipeCard {
  @drag-horizontal constraints: [-100, 100] elastic: 0.5 velocity_max: 500 {
    keyframe { 0% { rotate: -30deg; } 50% { rotate: 0; } 100% { rotate: 30deg; } }
  }
}
```

> ✅ **已实现**（Task 5.1，v2 BNF `drag_params ::= 'constraints:' '[' expr ',' expr ']'
> 'elastic:' expr 'threshold:' expr ('velocity_max:' expr)?`）：控制维度以冒号形式
> 子参数挂在触发器关键字与 `={motion}` 绑定之间——
> `@drag-horizontal constraints: [-100, 100] elastic: 0.5 velocity_max: 500 ={swipeCard}`。
> 三后端均消费：React/OmniView 生成类型化 `constraints: [min, max]`（运行时
> clamp + 弹性回弹，不再解析字符串配置）；Flutter `_OmniDragWidget` 接收
> `constraintsMin:`/`constraintsMax:` 类型化构造参数。缺省时保持关键帧范围
> 推导（`maxExtent`）与无界警告回退。

| 属性 | 语义 | React | Flutter |
|------|------|-------|---------|
| `constraints: [min, max]` | 拖拽范围限制 | pointer events + clamp | `GestureDetector.onPanUpdate` + clamp |
| `elastic: N` | 超出范围时的弹性系数 | 超出时 progress = clamped + easing | 同 |
| `velocity_max: N` | 拖拽期间速度跟踪上限（px/s 截断） | pointermove 中 `velocity = min(velocityMax, dp/dt*1000)` | `_onDragUpdate` 中同样截断 |
| `threshold: N` | 吸附阈值 | progress 超过 threshold 吸附到最近端点 | 同 |

> ⚠️ 诚实降级（ADR-054）：释放后惯性 decay 在三个目标上均**未实现**——
> `velocity_max` 仅作为拖拽期间速度跟踪器的上限被消费（三目标一致），
> 释放行为统一为 threshold 吸附（play/reverse）。此前的「pointerup +
> velocity calc + decay」描述不实，已修正。

### 3.8 内联动画体 → 命名动画（IR lowering）

内联动画体作为语法糖保留，在 IR lowering 阶段展开：

```rust
// 新增 pass: ir/lower/lower_inline_motions.rs
// 扫描所有 ViewTree，找到 trigger.inline_motion 不为 None 的
// → 合成 MotionIntent { name: "__m_N", body: inline_motion }
// → 追加到 ModuleIR.motions
// → 重写 trigger.motion_ref = Some("__m_N"), trigger.inline_motion = None
```

Lowering 后，**所有三个目标只处理命名 motion**，消除了当前的 Flutter/React inline motion gap。

### 3.9 已删除的动画特性

| 特性 | 原因 | 替代 |
|------|------|------|
| `paused_override` | 无测试覆盖，使用场景极少 | 使用 `paused: true` + 条件播放 |
| `@leave` 触发器 | ADR-032 已删除 | — |
| `timeline` (多元素时间线编排) | ADR-032 已删除 | — |
| `states` (动画状态机) | ADR-032 已删除 | — |
| `@focus` JS 死代码（React） | ADR-032 残留，需要清理 | 重新实现 `@focus` 触发器 |

### 3.10 Flutter 动画限制（需要 diagnostic）

| 限制 | 原因 | Diagnostic |
|------|------|-----------|
| 动画 `width`/`height` | 动画布局属性会触发布局重计算，性能差 | warning: "layout-property animation not recommended, use Transform.scale instead" |
| `steps()` 缓动 | Flutter 无原生 steps easing | error: "steps easing not supported in Flutter target" |
| `fill: backwards` 独立使用 | Flutter Tween 默认显示 begin | 正确行为恰好等价，无需 diagnostic |

---

## 4. Design Tokens（设计系统）

### 4.1 语义定义

Design block 的语义是：**"定义一组有名字的设计值，可以有主题变体，可以被组件引用"**。

```omni
design MyTheme {
  @themes light, dark              → "我有两个主题"
  @default-theme light             → "默认用 light"

  token primaryColor: Color = #6366f1    → "主色这个值"
  @theme dark { primaryColor = #818cf8 } → "dark 主题覆盖为这个值"

  token spacingSm: Size = 8px            → "小的间距值"

  class Card {                           → "Card 用这些样式"
    background-color: token(primaryColor)
    border-radius: token(spacingSm)
    padding: token(spacingSm)
    &:hover { opacity: 0.9 }
  }

  variant Size {
    sm { padding: 4px; font-size: 12px; }
    md { padding: 8px; font-size: 14px; }
    lg { padding: 16px; font-size: 18px; }
  }
}
```

### 4.2 类型化 Token

| Token 类型 | CSS 值 | React 输出 | Flutter 输出 |
|-----------|--------|-----------|-------------|
| `Color` | `#6366f1` → `var(--color-primary)` | CSS Custom Property | `static const Color primaryColor = Color(0xFF6366F1)` |
| `Size` | `8px` → `var(--spacing-sm)` | CSS Custom Property | `static const double spacingSm = 8.0` |
| `Radius` | `8px` → `var(--radius-md)` | CSS Custom Property | `static const double radiusMd = 8.0` + `BorderRadius.circular(AppTokens.radiusMd)` |
| `FontFamily` | `'Inter'` → CSS | CSS Custom Property | `static const String fontFamily = 'Inter'` |
| `FontSize` | `14px` | CSS Custom Property | `static const double fontSizeSm = 14.0` |
| `FontWeight` | `500` | CSS Custom Property | `static const FontWeight fontWeightMedium = FontWeight.w500` |
| `LineHeight` | `1.5` | CSS Custom Property | `static const double lineHeight = 1.5` |
| `LetterSpacing` | `0.5px` | CSS Custom Property | `static const double letterSpacing = 0.5` |
| `Shadow` | `0 2px 8px rgba(...)` | CSS Custom Property | `static const BoxShadow shadowCard = BoxShadow(...)` |
| `Duration` | `300ms` | CSS Custom Property | `static const Duration durationNormal = Duration(milliseconds: 300)` |
| `Easing` | `cubic-bezier(0.16,1,0.3,1)` | CSS Custom Property | `static const Cubic easeOutExpo = Cubic(0.16, 1, 0.3, 1)` |
| `Opacity` | `0.8` | CSS Custom Property | `static const double opacityDisabled = 0.8` |
| `ZIndex` | `10` | CSS Custom Property | ⚠️ diagnostic: "z-index not applicable in Flutter (use widget order / Stack order)" |

### 4.3 主题切换

```
语义: "切换到另一个主题"

.omni: theme = "dark"
```

**React**：设置 `data-theme` attribute，CSS 变量自动生效。不需要任何 JS 包装函数。

```tsx
// .omni action 中写 theme = "dark"
// 生成代码:
document.documentElement.setAttribute('data-theme', 'dark');
// CSS 中: [data-theme="dark"] { --primary: #818cf8; }  自动切换
```

如果主题状态需要跨组件共享，用 Zustand store 存当前 theme 名：

```tsx
// 生成的 stores/useThemeStore.ts
import { create } from 'zustand';

export const useThemeStore = create<{ theme: string }>()((set) => ({
  theme: 'light',
  setTheme: (name) => {
    document.documentElement.setAttribute('data-theme', name);
    set({ theme: name });
  },
}));
```

**Flutter**：用 `ThemeMode` + `MaterialApp(themeMode: ...)`，Flutter 原生模式。不需要任何包装函数。

```dart
// 生成的入口 main.dart:
final themeModeProvider = StateProvider<ThemeMode>((ref) => ThemeMode.light);

class MyApp extends ConsumerWidget {
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    return MaterialApp(
      themeMode: themeMode,
      theme: ThemeData.light(),
      darkTheme: ThemeData.dark(),
      home: MainPage(),
    );
  }
}

// .omni action 中写 theme = "dark"
// 生成代码:
ref.read(themeModeProvider.notifier).state = ThemeMode.dark;
// MaterialApp 自动重建，ThemeData 切换——Flutter 原生行为
```

**OmniView**：`document.documentElement.dataset.theme = "dark"` — CSS 变量自动切换。

### 4.4 动态 Token（信号驱动的 Token 值）

```omni
token primaryColor: Color = ThemeStore.primaryColor
// 语义: "主色的值来自运行时状态 ThemeStore.primaryColor"
```

**React**：`useEffect` 写 CSS 变量，标准的 React 副作用模式。

```tsx
// 生成的代码（在 App 组件或 theme effect hook 中）:
const primaryColor = useThemeStore(s => s.primaryColor);
useEffect(() => {
  document.documentElement.style.setProperty('--primaryColor', primaryColor);
}, [primaryColor]);
```

**Flutter**：Riverpod 选择器 + ThemeData 重建，纯 Flutter 原生模式。

```dart
// 生成的入口:
final primaryColorProvider = Provider<Color>((ref) {
  return ref.watch(themeStoreProvider.select((s) => s.primaryColor));
});

class MyApp extends ConsumerWidget {
  Widget build(BuildContext context, WidgetRef ref) {
    final primaryColor = ref.watch(primaryColorProvider);
    return MaterialApp(
      theme: ThemeData.light().copyWith(
        colorScheme: ColorScheme.fromSeed(seedColor: primaryColor),
      ),
      // ...
    );
  }
}
```

没有 `__omni_effect` 桥接，没有 `setProperty` 包装——就是 React 的 useEffect / Flutter 的 Provider + ThemeData。

### 4.5 Classes 和 Variants

#### Classes → Flutter 地道实现

```omni
class Card { background-color: token(primaryColor); border-radius: 8px; }

// 组件中: <div class:Design.Card />
```

**React**：全局 `design.css`，组件直接用 `className="Card"`：
```css
/* 生成的 design.css */
.Card { background-color: var(--primaryColor); border-radius: 8px; }
.Card:hover { opacity: 0.9; }
```

**Flutter**：`BoxDecoration` 工厂 + `Container` widget，Flutter 原生模式：
```dart
// 生成的 design.dart
abstract final class AppDesign {
  static BoxDecoration cardDecoration() => BoxDecoration(
    color: const Color(0xFF6366F1),
    borderRadius: BorderRadius.circular(8),
  );
}

// .omni: <div class:Design.Card>
// 生成:
Container(decoration: AppDesign.cardDecoration(), child: ...)
```

#### Variants → Flutter 地道实现

```omni
variant Size { sm { padding: 4px; }, md { padding: 8px; }, lg { padding: 16px; } }

// 组件中: <div variant:Design.Size.sm />
```

**React**：CSS 组合类 `className="Size-sm"`：
```css
.Size-sm { padding: 4px; }
.Size-md { padding: 8px; }
.Size-lg { padding: 16px; }
```

**Flutter**：`ThemeExtension<AppSizeTheme>`，Flutter 3.0+ 官方推荐的自定义主题扩展：
```dart
// 生成的 design.dart
class AppSizeTheme extends ThemeExtension<AppSizeTheme> {
  final EdgeInsets padding;
  const AppSizeTheme({required this.padding});

  static const sm = AppSizeTheme(padding: EdgeInsets.all(4));
  static const md = AppSizeTheme(padding: EdgeInsets.all(8));
  static const lg = AppSizeTheme(padding: EdgeInsets.all(16));

  @override AppSizeTheme copyWith({EdgeInsets? padding}) =>
    AppSizeTheme(padding: padding ?? this.padding);
  @override AppSizeTheme lerp(AppSizeTheme? other, double t) =>
    AppSizeTheme(padding: EdgeInsets.lerp(padding, other?.padding ?? padding, t)!);
}

// 注册到 MaterialApp:
MaterialApp(
  theme: ThemeData(extensions: [AppSizeTheme.md]),
)

// .omni: <div variant:Design.Size.sm>
// 生成代码:
Theme.of(context).extension<AppSizeTheme>()!.padding
```

没有 `setTheme()`。没有 `expand_variant` 包装。就是 Flutter 开发者会手写的 `ThemeExtension`。

### 4.6 CSS 属性映射（Flutter）

当组件 `style: { ... }` 中直接写 CSS 属性时，Flutter 做语义映射：

**已映射（~25 个）**：
```
padding/margin    → EdgeInsets
width/height      → 尺寸约束
color             → Color
background-color  → BoxDecoration.color
border-radius     → BorderRadius
border            → BoxDecoration.border
box-shadow        → BoxDecoration.boxShadow
font-size/family/weight/height/spacing → TextStyle
text-align        → TextAlign
opacity           → Opacity widget
overflow          → ClipRect
display: flex     → Row/Column
gap               → SizedBox
position: absolute → Stack + Positioned
```

**Diagnostic（无法映射）**：
```
display: grid     → error: "CSS grid not supported in Flutter"
display: inline   → error: "use Row with MainAxisSize.min"
float             → error: "CSS float has no equivalent in Flutter widget layout"
cursor            → warning: "cursor styling has no effect in Flutter"
```

**注**：触发 diagnostic 的属性在组件 `style: { ... }` 中无法用于 Flutter 目标。通过 `class:` 引用的 design class 中的属性同理——如果 class 里包含 Flutter 无法映射的属性，emit warning。

### 4.7 Flutter ThemeExtension（Variant 高级映射）

对于复杂的 variant（不同 variant 改变多个不同属性），使用 Flutter 3.0+ ThemeExtension：

```dart
// 生成的 design.dart
class AppButtonTheme extends ThemeExtension<AppButtonTheme> {
  final EdgeInsets padding;
  final double fontSize;
  final BorderRadius borderRadius;

  const AppButtonTheme({ required this.padding, required this.fontSize, required this.borderRadius });

  // copyWith, lerp, of(context) ...
}

// 生成的 main.dart
MaterialApp(
  theme: ThemeData(
    extensions: [
      AppButtonTheme.sm,  // variant: Size.sm
      AppButtonTheme.md,  // variant: Size.md
      AppButtonTheme.lg,  // variant: Size.lg
    ],
  ),
)
```

这是 Flutter 官方推荐的扩展主题数据方式，零额外依赖。

---

## 5. Component Composition（组件组合 — Slots）

### 5.1 IR 层关键变更

**当前 bug**：`ViewTree::Slot(Option<String>)` 是一个叶子节点，`ir/lower/view.rs:205-208` **丢弃了 `default_content`**（fallback 内容在 IR lowering 时就丢了）。这意味着**所有三个目标**都无法使用 slot fallback。

**修复**：
```rust
// crates/omnilang-core/src/ir/types.rs — 修改:
ViewTree::Slot {
    name: Option<String>,           // None = 默认 slot
    fallback: Vec<ViewTree>,        // fallback 内容（当前被丢弃！）
}
```
Lowering 保留 `default_content`；`inject_slot_children` 不变（消费侧 children → 默认 slot，`slot="name"` → 命名 slot）。

### 5.2 语义定义

```omni
component Modal {
  @slot header { <DefaultHeader /> }   // "可选 header，不给用默认"
  @slot default                         // "这是内容区"
  @slot footer { <DefaultFooter /> }   // "可选 footer"

  <div class="modal">
    <div class="modal-header">{header}</div>
    <div class="modal-body">{children}</div>
    <div class="modal-footer">{footer}</div>
  </div>
}

// 消费侧:
<Modal header={<CustomHeader />}>
  <p>这是内容</p>
  <Button>关闭</Button>
</Modal>
```

### 5.2 实现映射

| 概念 | React | Flutter |
|------|-------|---------|
| 默认 slot | `{children}` (props.children) | `Widget? child` 参数 |
| 命名 slot | `{header}` (命名 prop) | `Widget? header` 参数 |
| Fallback | `{header ?? <DefaultHeader />}` | `header ?? DefaultHeader()` |
| Slot 渲染 | JSX `{header}` 放在模板中 | `header ?? DefaultHeader()` 放在 build 中 |
| 消费侧 children | 已自动透传（`react/view.rs:77-94`） | 需要声明 `children:` / `child:` widget param |

### 5.4 Slot 类型约束

Slot 可以携带类型信息，实现类型安全的 children 传递：

```omni
@slot header: HeaderProps           // header 必须接受 HeaderProps
@slot default { content }: BodyProps // named default slot，注入 content 数据
```

Flutter: `final Widget Function(HeaderProps)? header;` — slot 声明变为 builder 函数类型。
React: `header?: (props: HeaderProps) => ReactNode;` — render prop 模式。

---

## 6. Stores & State（状态管理）

### 6.1 语义定义

```omni
store TodoStore {
  signal items: Todo[] = []
  signal filter: String = "all"

  getter filteredItems: Todo[] = items.filter(item =>
    filter == "all" || item.status == filter
  )

  getter activeCount: Int = items.count(item => item.status == "active")

  action add(text: String) {
    items = [...items, Todo { id: nextId, text, status: "active" }]
  }

  action toggle(id: Int) {
    items = items.map(item =>
      item.id == id ? { ...item, status: item.status == "active" ? "completed" : "active" } : item
    )
  }

  action clearCompleted() {
    items = items.filter(item => item.status != "completed")
  }
}
```

### 6.2 React 实现

**方案：Zustand v5（最新稳定版，唯一方案）**

Zustand v5 是当前生产版本（最新 5.0.13）。核心设计哲学：**不存储派生状态**——getter 不应该放在 store 里，而是作为独立的 selector hook。

```tsx
// 生成的 stores/useTodoStore.ts
import { create } from 'zustand';

interface TodoStoreState {
  items: Todo[];
  filter: string;
}

// ① Raw store — 只存 signals，不含 getters
export const useTodoStore = create<TodoStoreState>()((set) => ({
  items: [],
  filter: 'all',
}));

// ② Getters → 独立 selector hooks（Zustand v5 惯用模式）
// 每个 getter 是一个 hook，订阅它依赖的所有 signals，用 useMemo 派生
export function useFilteredItems() {
  const items = useTodoStore(s => s.items);
  const filter = useTodoStore(s => s.filter);
  return useMemo(
    () => items.filter(item => filter === 'all' || item.status === filter),
    [items, filter]
  );
}

export function useActiveCount() {
  return useTodoStore(s =>
    s.items.filter(item => item.status === 'active').length
  );
}

// ③ Actions — 写在 create() 内部或外部都可以
// 编译器对每个 action 生成一个导出函数
export function useAddTodo() {
  return useCallback((text: string) => {
    useTodoStore.setState((s) => ({
      items: [...s.items, { id: Date.now(), text, status: 'active' }],
    }));
  }, []);
}

// ④ 组件中消费 — 选择器订阅 + 按需引用
export function TodoList() {
  const items = useFilteredItems();       // 只订阅 items + filter
  const toggleItem = useToggleTodo();     // action 引用，不触发重渲染
  // ...
}
```

**为什么不用 store 内 getter**：Zustand v5 的 `get()` 在 store 内部可以读最新值，但组件订阅 `(s) => s.getTotal()` **不会触发重渲染**——因为 `getTotal` 函数引用不变。官方推荐：组件中用 selector 读底层 state，用 `useMemo` 派生。

**为什么 actions 不支持链式调用**（`get().addTodo()`）：Zustand v5 的 actions 写在 `create()` 外更简洁，且天然支持 `useCallback` 稳定引用。codegen 把每个 action 生成为独立导出函数。

### 6.3 Flutter 实现

**方案：Riverpod 3.0（最新稳定版 3.3.2，2025 年 9 月发布）**

Riverpod 3.0 统一了 API：`StateProvider` 和 `StateNotifierProvider` 已废弃移至 `legacy.dart`。推荐用 `Notifier<T>` + `NotifierProvider`。

```dart
// 生成的 stores/todo_store.dart（Riverpod 3.0）
import 'package:flutter_riverpod/flutter_riverpod.dart';

// ① State class（不可变）
class TodoStoreState {
  final List<Todo> items;
  final String filter;
  const TodoStoreState({this.items = const [], this.filter = 'all'});
  TodoStoreState copyWith({List<Todo>? items, String? filter}) =>
    TodoStoreState(items: items ?? this.items, filter: filter ?? this.filter);
}

// ② Notifier（替代 StateNotifier + StateProvider）
class TodoStoreNotifier extends Notifier<TodoStoreState> {
  @override
  TodoStoreState build() => const TodoStoreState();  // 初始状态

  // Getters — Notifier 内直接读 state，始终是最新值
  List<Todo> get filteredItems => state.items
    .where((item) => state.filter == 'all' || item.status == state.filter)
    .toList();
  int get activeCount => state.items
    .where((item) => item.status == 'active').length;

  // Actions — state = newState 触发更新
  void add(String text) {
    state = state.copyWith(items: [
      ...state.items,
      Todo(id: _nextId++, text: text, status: 'active'),
    ]);
  }
  void toggle(int id) {
    state = state.copyWith(items: state.items.map((item) =>
      item.id == id ? item.copyWith(
        status: item.status == 'active' ? 'completed' : 'active'
      ) : item
    ).toList());
  }
  void clearCompleted() {
    state = state.copyWith(
      items: state.items.where((item) => item.status != 'completed').toList(),
    );
  }
}

// ③ Provider — NotifierProvider<NotifierType, StateType>(Constructor.tearOff)
final todoStoreProvider = NotifierProvider<TodoStoreNotifier, TodoStoreState>(
  TodoStoreNotifier.new,
);

// ④ 组件中使用（ConsumerStatefulWidget）:
class TodoList extends ConsumerStatefulWidget { ... }
class _TodoListState extends ConsumerState<TodoList> {
  Widget build(BuildContext context) {
    final items = ref.watch(todoStoreProvider
      .select((s) => s.items));           // 选择器订阅
    final notifier = ref.read(todoStoreProvider.notifier);
    return ListView(
      children: items.map((item) => TodoItem(
        item: item,
        onToggle: () => notifier.toggle(item.id),
      )).toList(),
    );
  }
}
```

**Riverpod 3.0 关键变化**（相比当前代码中使用的 2.x API）：
- `StateProvider<T>` → **删除**（移至 `legacy.dart`）→ 用 `Notifier<T>` + `NotifierProvider`
- `StateNotifierProvider<Notifier, State>` → `NotifierProvider<Notifier, State>`
- `ref.watch(provider)` → `ref.watch(provider.select((s) => s.field))`（选择器）
- Notifier 内的 `state` 读写不需要 `ref`——直接 `state = newState`、`state.items`
- Async actions → `AsyncNotifier<T>` + `AsyncNotifierProvider`（内置 `AsyncValue`）

### 6.4 Component-local State

```omni
component Counter {
  signal count: Int = 0
  ...
}
```

| 目标 | 实现 |
|------|------|
| React | `const [count, setCount] = useState(0)`（React 原生） |
| Flutter | `int _count = 0;` + `setState(() { _count = value; })`（Flutter StatefulWidget 原生） |
| OmniView | inline signal `__omni_sig('count', 0)` |

Component-local state 不需要任何第三方库。

---

## 7. Control Flow（控制流）

### 7.1 条件渲染 @if / @else-if / @else

```omni
@if isLoading {
  <Spinner />
} @else if error != null {
  <ErrorView message={error} />
} @else {
  <ContentView data={data} />
}
```

| 目标 | 输出 |
|------|------|
| React | `isLoading ? <Spinner /> : error != null ? <ErrorView ... /> : <ContentView ... />` |
| Flutter | `isLoading ? Spinner() : error != null ? ErrorView(...) : ContentView(...)` |
| OmniView | JavaScript 条件语句 |

**`transition_mode` 支持**：
```omni
@if isVisible transition_mode: "wait" {
  <Content />
}
```

| 目标 | transition_mode 实现 |
|------|---------------------|
| React | 包裹 `<AnimatePresence>`-like wrapper 或 CSS transition |
| Flutter | `AnimatedSwitcher(duration: ..., child: isVisible ? Content() : SizedBox.shrink())` |
| OmniView | CSS transition |

### 7.2 列表渲染 @for

```omni
@for item in items {
  <TodoItem data={item} />
}
```

| 目标 | 默认实现 | scroll 模式 |
|------|---------|------------|
| React | `items.map(item => <TodoItem key={key} ... />)` | —（React 用 CSS overflow） |
| Flutter | `Column(children: [...items.map((item) => TodoItem(...))])` | `ListView.builder(itemCount: items.length, itemBuilder: ...)` |
| OmniView | DOM 元素重复 |

**Flutter @for 策略**：
- **默认**：`Column(children: [...for])` — 安全，无 unbounded height 问题
- **长列表**：compiler heuristic：如果 items 来自 store 且可能是无限数据集，或用 `[scroll]` hint → `ListView.builder`
- **嵌套 @for**：内层 @for 在外层 scrollable 内 → 自动用 Column（不嵌套 ListView）

**key_expr**：
```omni
@for item in items key={item.id} { ... }
```

| 目标 | 实现 |
|------|------|
| React | `items.map(item => <TodoItem key={item.id} ... />)` |
| Flutter | `ListView.builder(..., key: ValueKey(item.id))` 或 `Column` 中每个 child 不需要 key |

**默认 key 策略**：
- 如果 `item` 是标量（string/number）→ `key={String(item)}`
- 如果是对象且无 `key_expr` → React: `key={_i}`（用 index），emit warning；Flutter: 无需 key（Column 不需要）

---

## 8. Expressions & Statements

### 8.1 Optional Chaining

```omni
user?.profile?.name
```

| 目标 | 实现 |
|------|------|
| React | `user?.profile?.name` |
| Flutter | `user?.profile?.name`（Dart 支持 `?.`） |
| OmniView | `user?.profile?.name` |

**修复**：在 `ReactFormatExpr` 和 `DartFormatExpr` 中 override `format_optional_chain`，不再使用 shared 的 `{object}.{field}` 默认实现。

### 8.2 Block Expression

```omni
fn compute(x: Int): Int {
  let temp = x * 2         // 块内有语句
  temp + 1                 // 尾表达式是返回值
}
```

当这个函数作为 getter 或 computed 时，BlockExpr 进入表达式位置。

| 目标 | 实现 |
|------|------|
| React | IIFE: `(() => { const temp = x * 2; return temp + 1; })()` |
| Flutter | IIFE: `(() { final temp = x * 2; return temp + 1; })()` |
| OmniView | IIFE: `(function() { const temp = x * 2; return temp + 1; })()` |

**修复**：在 ReactFormatExpr 和 DartFormatExpr 中 override `format_block_expr`，emit IIFE 包裹。

### 8.3 Match Expression

```omni
fn statusText(status: Status): String = match status {
  Status.Active => "活跃"
  Status.Inactive => "未激活"
  _ => "未知"
}
```

| 目标 | 实现 |
|------|------|
| React | `(() => { switch(status) { case Status.Active: return "活跃"; case Status.Inactive: return "未激活"; default: return "未知"; } })()` |
| Flutter | `(() { switch(status) { case Status.active: return '活跃'; case Status.inactive: return '未激活'; default: return '未知'; } })()` |
| OmniView | 现有 IIFE pattern |

**修复**：DartFormatExpr 需要完整的 match 实现（literal arms + bindings destructure + no-match throw），目前 shared/expr.rs 中 Dart 侧的实现被砍掉了。

### 8.4 UnaryOp Await

```omni
let data = await fetchData()
```

| 目标 | 实现 |
|------|------|
| React | `const data = await fetchData()`（生成 async 函数体） |
| Flutter | `final data = await fetchData()`（生成 `Future<T>` async 函数） |

### 8.5 复合赋值

```omni
count += 1
name ??= "default"
```

| 目标 | React | Flutter |
|------|-------|---------|
| `+=` | `count += 1` | `count += 1` |
| `??=` | `name ??= "default"` | `name ??= 'default'` |
| `-=`, `*=`, `/=` | 对应 JS/Dart 操作符 |

### 8.6 Spread 表达式

```omni
let combined = [...list1, ...list2]
```

| 目标 | 实现 |
|------|------|
| React | `const combined = [...list1, ...list2]` |
| Flutter | `final combined = [...list1, ...list2]`（Dart 支持 spread `...`） |

### 8.7 Struct 和 Map 字面量

```omni
let point = Point { x: 10, y: 20 }
let config = { key: "value", count: 5 }
```

| 目标 | StructExpr | MapExpr |
|------|-----------|---------|
| React | `{ x: 10, y: 20 }`（需要生成 TS `interface Point`） | `{ key: "value", count: 5 }` |
| Flutter | `Point(x: 10, y: 20)`（需要生成 Dart `class Point`） | `{ 'key': 'value', 'count': 5 }` |

---

## 9. Component Lifecycle

### 9.1 语义定义

```omni
component Timer {
  signal elapsed: Float = 0

  mount {                         // "当组件首次挂载时"
    let timer = setInterval(() => {
      elapsed += 0.1
    }, 100)
  }

  unmount {                       // "当组件被卸载前"
    clearInterval(timer)           // timer 来自 mount 的闭包
  }

  effect {                        // "当 deps 变化时执行"
    saveToLocal(elapsed)           // 依赖自动追踪
  }

  <Text>已运行 {elapsed}s</Text>
}
```

### 9.2 映射

| 生命周期 | React | Flutter | OmniView |
|---------|-------|---------|----------|
| `mount` | `useEffect(() => { ... }, [])` | `initState()` | `__omni_mount(() => { ... })` |
| `unmount` | `useEffect(() => { return () => { ... }; }, [])` | `dispose()` | `__omni_unmount(() => { ... })` |
| `effect` | `useEffect(() => { ... }, [deps])` | 生成方法 + initState 调用 + didUpdateWidget(deps 比较) 重调 | `__omni_effect(() => { ... })` |
| async effect | `useEffect(() => { (async () => { await ... })(); }, [deps])` | `Future.microtask(() async { await ... })` + dispose 取消 | async IIFE |

**Flutter effect 修复**：
- 当前：`_runEffect_N()` 是未定义方法
- 修复：生成 `void _runEffect_N() { ... }` 方法，在 `initState` 调用，在 `didUpdateWidget` 中对 deps 做 equality check 后重调

---

## 10. Cross-Target Feature Floor（跨目标兼容性矩阵）

### 10.1 P0 Feature Floor（所有三个目标必须实现）

| 类别 | 特性 | 说明 |
|------|------|------|
| **组件** | Component, signal, prop, store binding | ✅ |
| **元素** | 原生元素渲染、属性、事件 | ✅ 各目标用自己平台的原生元素 |
| **绑定** | `bind:value`, style, class, variant | ✅ 双向绑定在 Flutter 需 read+onChanged |
| **控制流** | @if/@else, @for, key_expr, empty_body | ✅ @for 在 Flutter 默认 Column |
| **插槽** | `<slot>` default + named + fallback | ✅ |
| **引用** | @ref 声明、绑定、focus/blur/scroll/measure | ✅ capturePointer Flutter diagnostic |
| **状态** | signal, computed, action, effect | ✅ |
| **生命周期** | mount, unmount, effect with deps | ✅ |
| **动画** | 8 triggers, keyframes, playback, easing | ✅ Flutter steps 除外 |
| **设计 Token** | DesignIR → CSS/ThemeData, token 引用 | ✅ ZIndex Flutter diagnostic |
| **动态 Token** | TokenValueIR::Dynamic | ✅ 各目标独立实现 |
| **主题** | @themes, @default-theme, theme 切换 | ✅ |
| **Classes** | class: Design.Xxx | ✅ |
| **Variants** | variant: Design.Xxx.Yyy | ✅ React CSS 组合，Flutter ThemeExtension |
| **表达式** | OptionalChain, BlockExpr IIFE, Match, Await, Spread | ✅ |
| **类型** | LiteralExpr, Identifier, FieldAccess, BinaryOp, UnaryOp, Ternary, Call, Array | ✅ |
| **a11y** | `a11y: { label, role, hidden, live, alt }` | ✅ Task 4.5 已实现：React aria-*；Flutter `Semantics`/`ExcludeSemantics`/`semanticLabel`/`liveRegion`（role 子集 + warning）；OmniView aria-* 透传 |

### 10.2 Diagnostic Floor（出 diagnostic 但不阻止编译）

| 特性 | 目标 | Diagnostic |
|------|------|-----------|
| `capturePointer` / `releasePointer` | Flutter | error: "pointer capture not supported in Flutter" |
| `steps()` easing | Flutter | error: "steps easing not supported in Flutter" |
| `z-index` token | Flutter | warning: "z-index not applicable in Flutter widget layout" |
| CSS `display: grid` | Flutter | error: "CSS grid not supported" |
| CSS `float` | Flutter | error: "CSS float not supported" |
| CSS `cursor` | Flutter | warning: "cursor ignored in Flutter" |
| 动画 width/height 属性 | Flutter | warning: "animating layout properties degrades performance" |
| @for without key_expr (object items) | React | warning: "add key= for object items to avoid duplicate keys" |
| @ref on custom component | Both | warning: "ref forwarding on custom components not yet supported (Phase 2)" |

### 10.3 OmniView-Only（明确排除对标）

| 特性 | 理由 |
|------|------|
| 源码映射（Source Maps） | 调试工具，生产不需要 |
| Editor bridges（`editor.rs`, `debug.rs`） | Poiesis 专属 |
| `css_only` 模式 | Poiesis Canvas 专属 |
| `wrap_debug_html` 无 CSP | 调试模式 |
| ElementTree 元数据 | Poiesis 组件树重建 |
| `nid_map` / `element_classes` / `element_tags` | Poiesis 编辑器桥 |
| Inline signal runtime（~200行） | React/Flutter 用 Zustand/Riverpod |
| 6-phase JS execution order | OmniView 独有的 deferred_js 缓冲模式 |
| `debug_assert!` phase guards | 编译期安全检查，release 跳过 |
| `(0, eval)` hot-replace | 编辑器热更新 |

---

## 11. Deleted / Deprecated Features

| 特性 | 操作 | 原因 |
|------|------|------|
| React Hooks store 模式 | **删除** | Zustand v4 唯一方案；`get()` 解决链式 getter；选择器避免全局重渲染 |
| `ref.style.*` 全部 | **删除** | DOM 专属，信号+style 绑定替代 |
| `ref.innerHTML` | **删除** | 安全风险 |
| `ref.addEventListener` | **删除** | `@on*` 替代 |
| `ref.querySelector` | **删除** | 违反封装 |
| `ref.classList` | **删除** | `class:` 替代 |
| `ref.parentElement` | **删除** | 违反封装 |
| `ref.textContent` | **删除** | 内容应通过信号/文本插值流动 |
| `paused_override` | **删除** | 无测试覆盖，条件绑定可替代 |
| `PlaybackIR.paused` | **删除** | 同上 |
| `@leave` 触发器 | 已删除（ADR-032） | — |
| `timeline` | 已删除（ADR-032） | — |
| `states` (动画状态机) | 已删除（ADR-032） | — |
| 非 P0 目标 design block | **error 替代 warning** | 生产语义不应静默降级 |

---

## 12. Governance & Quality Gates

### 12.1 CI 验证

每个 PR 必须：
- `cargo test -p omnilang` — 所有 Rust 测试
- `tsc --noEmit` 对生成的 React 回归 fixture 输出
- `dart analyze` 对生成的 Flutter 回归 fixture 输出

### 12.2 参数化测试矩阵

`tests_regression/parameterized.rs` 必须覆盖所有 P0 feature floor 特性，每个特性至少一个 fixture：
- 组件 + props + signals
- store 读 + action 调用
- 条件渲染 + 列表渲染 + key_expr + empty_body
- slot（default + named + fallback）
- ref（focus + scroll + measure）
- 动画（enter + hover + press + 多 stop keyframe + playback properties）
- design tokens + themes + classes + variants
- OptionalChain + BlockExpr + MatchExpr + AwaitExpr

### 12.3 ADR 维护

- **ADR-020**：更新状态为 ✅ Implemented（Riverpod 2.x），标注 pin 版本决策
- **ADR-021**：更新为 "semantically equivalent behavior for documented feature floor + hard diagnostics"
- **新增 ADR-052**：Cross-Target Feature Floor Definition（本设计文档的执行决策）

---

## 13. Migration Notes（当前实现 → 新设计）

### 13.1 编译器结构变更

| 变更 | 文件 | 说明 |
|------|------|------|
| 移除 design block 编译门 | `omnilang/src/codegen/mod.rs:57-65, 152-203` | React/Flutter 不再返回零文件；非 P0 目标改为 error |
| 提取共享 design CSS emitter | `omniview/design.rs` → `codegen/shared/design.rs` | `emit_design_css`, `expand_variant`, `emit_dynamic_token_effects` 抽取为共享模块 |
| 新增共享 expr override | `crates/omnilang-core/src/codegen/shared/expr.rs` + `react/expr.rs` + `flutter/expr.rs` | `format_optional_chain`, `format_block_expr` override；`DartFormatExpr::format_match` 完整实现 |
| 新增语义 pass | `semantic/validate_ref_ops.rs` | `MethodCallExpr` → `RefOpExpr` 改写 + 非法 API 报错 |
| 新增 IR lowering pass | `ir/lower/lower_inline_motions.rs` | 内联动画体 → 命名 motion |
| 修改 Slot IR | `ir/types.rs` + `ir/lower/view.rs` | `ViewTree::Slot { name, fallback }` 保留 fallback |
| 删除 paused/paused_override | `ir/types_motion.rs` | 删除 `PlaybackIR.paused` 和 `MotionBehavior.paused_override` |
| 新增 TriggerKind::Focus | `ir/types_motion.rs` | 重新加入 @focus 触发器 |

### 13.2 React codegen 修复（按优先级）

1. **Store 层 4 个 bug**（Section 6）：
   - H4: `gen_store_hook` getter 依赖扫描 → 传递闭包（getter 名加入 deps 扫描）
   - H2/H3: Zustand rewrite → IR 层标识符限定（`count` → `state.count`），不在格式化文本上做 replace
   - H5: 总是 emit `const state = get();` 当 action body 引用任何 signal

2. **Read collection 修复**：`collect_view_read_signals_inner` 补充扫描 `class_map_bindings` values + inline style `$signal` refs

3. **List key 修复**：`String(item)` 仅对标量类型；对象类型用 index + warning

4. **BlockExpr/OptionalChain/Match 修复**（Section 8）：override shared 默认实现

5. **Slots**：`{props.children}` 默认 slot + 命名 props

6. **Motion**：@enter 从 `document.body` scope 到组件 root ref；移除 per-component MutationObserver

### 13.3 Flutter codegen 修复（按优先级）

1. **编译错误修复**（C2-C6）：
   - C2: `@effect` → 生成真实 `_runEffect_N()` 方法，initState 调用 + didUpdateWidget deps 比较后重调
   - C3: `const CompName({` 构造函数修复
   - C4: store 读 → `ref.watch(storeProvider.select(s => s.field))` + store-alias→provider-name 映射表
   - C5: CSS hex `#ff0000` → `Color(0xFFFF0000)` 转换函数
   - C6: 属性映射白名单 + 非法映射 → diagnostic

2. **Slot + 组合修复**（H1/H7）：`Widget? child` + 命名 `Widget?` params + constructor 声明

3. **动画修复**（H8）：
   - 每个属性独立 `Animatable<T>` + `TweenSequence`（多 stop）
   - playback properties（loop/count/direction/delay/fill）完整实现
   - duration 解析修复（`"1.5s"` → 1500, `"300ms"` → 300）
   - 只为此组件用到的 motion 创建 controller（不用全局）

4. **@for 修复**（H9）：默认 `Column(children: [...for])` + `key: ValueKey(expr)` + `[scroll]` hint → ListView

### 13.4 CI 验证门

| 门 | 内容 |
|---|------|
| `cargo test -p omnilang` | 所有现有 + 新增测试 |
| `tsc --noEmit` | 对生成的 React fixture 输出运行 TypeScript 编译器 |
| `dart analyze` | 对生成的 Flutter fixture 输出运行 Dart 静态分析 |
| 参数化矩阵扩展 | design / slots / refs / key_expr / animation 各至少 1 个 fixture |

### 13.5 治理更新

- **ADR-020**：更新为 ✅ Implemented（Riverpod 3.0 `Notifier` + `NotifierProvider`），标注 `StateProvider` 废弃
- **ADR-021**：文本更新为 "semantically equivalent behavior for documented feature floor + hard diagnostics outside it"
- **新增 ADR-052**：本设计文档的执行决策（Cross-Target Feature Floor Definition）
- **删除 Hooks 模式**：React store 只保留 Zustand v5，`ReactStoreMode` enum 删除
- **React 19**：`forwardRef` 废弃，ref 作为普通 prop 传递

---

## 14. Open Questions（全部已决）

所有设计决策已根据"一步到位 + 语义 DSL"原则确定：

| # | 问题 | 决策 | 理由 |
|---|------|------|------|
| Q1 | React store 方案 | **Zustand v4 唯一方案**，删除 Hooks 模式 | `get()` 天然解决链式 getter 问题；选择器订阅避免全局重渲染 |
| Q2 | Flutter store 方案 | **Riverpod 3.0**（Notifier + NotifierProvider + AsyncValue） | 一步到位；`StateProvider`（2.x）已废弃 |
| Q3 | Flutter 动态 token | **直接实现**：`ValueNotifier` + `ValueListenableBuilder` | 一步到位 |
| Q4 | `@for [scroll]` 语法 | **`@for item in items [scroll]`** | 简洁，不引入新关键字，语义清晰 |
| Q5 | `transition_mode` | **v1 支持**: `fade`（CSS transition / AnimatedSwitcher）、`wait`（AnimatedSwitcher）、`none`（即时不带动画）。默认时长 200ms | 最通用的三个模式，跨平台可映射 |
| Q6 | 多个 design block | **React**: `[data-design~="name"]` scope selector + `<div data-design="name">` 应用根包裹。**Flutter**: 每个 design block 独立 `AppDesign` class | 复用 OmniView 已有的 scope 方案 |
| Q7 | 非 P0 目标 design block | **硬 error** | 生产语义不应静默降级；"no files + warning" 是最差的失败模式 |
| Q8 | `@focus` 触发器 | **重新加入** Floor | Focus 动画在生产 UI 中是常见需求；React `focusin`/Flutter `FocusNode`/OmniView `focus` 全部原生支持 |
| Q9 | `ref.capturePointer` Flutter | **v1 diagnostic** | `GestureDetector` 默认覆盖拖拽手势场景（capturePointer 的主要用例）；如真有需求后续实现 |
| Q10 | Flutter `steps()` 缓动 | **diagnostic** | Flutter 无原生 steps easing；如用户需要可自定义 `Curve` 子类 |
| Q11 | `ref.value`（表单值） | **保留** | 跨平台表单值读写是真正需求（React: `ref.current.value`, Flutter: `TextEditingController.text`） |
| Q12 | Flutter theme 切换 | **生成 `setTheme(name)` 全局函数** | 从 `.omni` action 可调用；和 React 的 `useTheme()` 对等 |
| Q13 | CSS 属性映射范围 | **~25 个常用属性**映射为 Flutter Widget 参数，其余 **diagnostic** | 具体列表见 Section 4.6；不是"全部 CSS" |
| Q14 | `@for` 默认 key 策略 | **标量**（string/number）→ `String(item)`；**对象** → index + compiler warning（推荐加 `key=`） | 避免 `"[object Object]"` 重复 key |

无遗留设计问题。
