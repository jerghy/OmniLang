# 11 — 运动与动画（Motion & Animation）

OmniView 提供声明式动画系统：`@motion` 定义补间动画参数，通过 8 个触发器绑定到元素。

**ADR-032 范围界定**：OmniView 动画系统保留 14 项核心功能，移除 19 项（`@timeline`、`states:{}`、Hero、stagger、回调等）。
**ADR-052/053 语义级 DSL**（2026-08-01 同步）：动画语法更新为冒号形式（`over:`/`easing:`）、17 属性语义白名单、8 触发器（kebab-case）；元素级 `@paused` 已移除，旧破折号形式触发器子属性（`@scroll-start`、`@drag-constraints` 等）已移除（控制维度由 **v2（2026-08-03，Task 5.1）** 以冒号形式恢复，见下文「控制维度子参数」）。本文档仅覆盖保留功能。**v2（2026-08-03，Task 5.2）**：白名单扩展为 19 项语义集（新增 `borderRadius`/`fontSize`，三目标均已验证并实现）。

## @motion 运动定义

### 基本关键帧

```omni
@motion fadeInUp {
    from { opacity: 0, translateY: 20 }
    to   { opacity: 1, translateY: 0 }
    over: 400ms
    easing: ease-out
}
```

### 多停靠关键帧（百分比）

```omni
@motion multiStop {
    0%   { opacity: 0, scale: 0.5, backgroundColor: "#6366f1" }
    25%  { opacity: 0.5, scale: 0.8 }
    50%  { opacity: 1, scale: 1.2, backgroundColor: "#10b981" }
    75%  { opacity: 1, scale: 1.05 }
    100% { opacity: 1, scale: 1, backgroundColor: "#f59e0b" }
    over: 1200ms
    easing: ease-in-out
}
```

支持任意百分比停靠点（0%–100%），未用的中间帧由浏览器自动插值。

### 可动画属性（19 个语义属性白名单，ADR-053 D3 + Task 5.2）

白名单是**语义集**：每个属性的可动画性都已在三个目标（WAAPI / Flutter / OmniView runtime）上验证。v1 的 17 项基础上，v2 新增 `borderRadius`/`fontSize`（Flutter 映射：`BorderRadiusTween` / `TextStyle(fontSize:)`，即 `TextStyle.lerp` 的等价实现）。

| 类别 | 属性 |
|------|------|
| **变换**（12） | `scale` `scaleX` `scaleY` `translateX` `translateY` `translateZ` `rotate` `rotateX` `rotateY` `rotateZ` `skewX` `skewY` |
| **颜色**（2） | `backgroundColor` `color` |
| **尺寸**（2） | `width` `height`（Flutter 目标：warning，布局属性性能差） |
| **圆角/字号**（2，v2） | `borderRadius` `fontSize` |
| **基础**（1） | `opacity` |

> 非白名单属性（`blur`、`brightness`、`borderWidth`、`top`、`left` 等）在解析期报错（ADR-053 D3）；CSS 自定义属性 `--*` 仅对 CSS 目标透传。`borderRadius`/`fontSize` 的取值按长度处理（数字或 `"Npx"` 字符串）。

### 播放控制（4 属性）

```omni
@motion pulse {
    to { scale: 1.05 }
    over: 800ms
    easing: ease-in-out

    loop                       // 无限循环
    direction alternate        // normal | reverse | alternate | alternateReverse
    delay 500                  // 延迟 ms（仅在首次播放生效）
    fill forwards              // none | forwards | backwards | both
}
```

| 属性 | 可选值 | 说明 |
|------|--------|------|
| `loop` | — | 无限重复播放 |
| `direction` | `normal` `reverse` `alternate` `alternateReverse` | 播放方向 |
| `delay` | `{N}` | 首次播放延迟 N 毫秒 |
| `fill` | `none` `forwards` `backwards` `both` | 动画前后的样式填充模式 |

> `paused` 播放关键字与元素属性 `@paused={signal}` 均已删除（ADR-053；v2 起解析期硬诊断「paused 已删除，用 @if 条件渲染替代」）；运行时暂停/恢复请用条件渲染。

## 缓动函数

### 命名缓动（kebab-case）

```omni
// 基础
easing: ease-out / ease-in / ease-in-out
easing: linear

// Power 系列（1 = 二次方, 2 = 三次方, 3 = 四次方, 4 = 五次方）
easing: power1-out / power1-in / power1-in-out
easing: power2-out / power2-in / power2-in-out
easing: power3-out / power3-in / power3-in-out
easing: power4-out / power4-in / power4-in-out

// Back（回弹，named-only，无参数）
easing: back-out / back-in / back-in-out

// Bounce（弹跳）
easing: bounce-out / bounce-in / bounce-in-out

// Elastic（弹性，named-only，无参数）
easing: elastic-out / elastic-in / elastic-in-out
```

### 自定义缓动

```omni
easing: cubic(x1: 0.68, y1: -0.55, x2: 0.27, y2: 1.55)  // 三次贝塞尔
easing: steps(6, start)                                  // 阶梯（Flutter 目标：warning，回退 Curves.linear）
```

- **Named-only**（Back、Elastic）不接收参数——参数形式（`backOut(intensity: 1.7)`、`elasticOut(amplitude: 1.0)`、`spring(tension: 200)`）已随 ADR-032 移除
- `steps()` 以位置参数形式保留（`steps(count, start|end)`，语义级 DSL 新形式）

> 注：`cubic(...)` 与 `delay <expr>`（空格形式）为 **parser 实际接受**的形式。BNF 权威为 v2 设计文档（`docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`，状态 Implemented；v1 规范 `2026-07-31-semantic-level-dsl-syntax-spec.md` 已 Superseded）：`cubic(x1:, y1:, x2:, y2:)` 命名参数形式、`delay <expr>` 空格形式、`direction` 规则含 `alternateReverse`（见 `.architecture/deferrals.md` #25，已关闭）。

## 动画触发器（8 个）

| 触发器 | 触发时机 |
|--------|----------|
| `@enter={motion}` | 元素进入 DOM / 条件变为 true |
| `@hover={motion}` | 鼠标悬停 |
| `@press={motion}` | 按下（click/touch） |
| `@focus={motion}` | 获得键盘焦点 |
| `@in-view={motion}` | 元素滚动到视口内 |
| `@scroll={motion}` | 滚动驱动进度动画 |
| `@drag-horizontal={motion}` | 水平拖拽 |
| `@drag-vertical={motion}` | 垂直拖拽 |

> **`@drag` 不是触发器**：`@drag` / `@dragEnd` 是独立的**拖拽意图指令**（ADR-073），不使用 motion keyframe——指针按下 → 捕获 → 持续回调处理器，事件数据 `{x, y, dx, dy, pointerId, startX, startY}`，配合动态 `style: { transform }` 绑定实现元素跟随指针（需同元素 `@ref`）。`@drag-horizontal` / `@drag-vertical` 才是 scrubbing 触发器（拖拽手势 → keyframe 进度）。详见 [07-事件与绑定](07-events-and-bindings.md)。

### 控制维度子参数（v2 恢复，Task 5.1）

`@scroll` 与 `@drag-*` 支持**冒号形式控制维度子参数**——挂在触发器关键字与 `={motion}` 绑定之间（v1 语法，v2 Task 5.1 恢复，`.architecture/deferrals.md` #19 已关闭）。旧破折号形式（`@scroll-start`、`@drag-constraints` 等）已在 ADR-053 移除，不得再用。其余 5 个触发器不接受子参数；未知或畸形子参数是**硬解析错误**（不解析忽略）。

```omni
<!-- @scroll — 滚动位置映射区间 + 平滑 -->
<div @scroll start: 0 end: 1000 scrub: true ={scrollProgress}>
    Scroll progress
</div>

<!-- @drag-horizontal — 拖拽范围约束 + 弹性回弹 -->
<div @drag-horizontal constraints: [-100, 100] elastic: 0.5 threshold: 0.3 ={dragSlide}>
    Drag me →
</div>
```

| 子参数 | 语义 | React 实现 | Flutter 实现 |
|--------|------|-----------|-------------|
| `@scroll start:` / `end:` | 数字滚动映射区间 | `progress = (scrollY - N) / (M - N)` | `_OmniScrollDrivenWidget(start: N, end: M)`（类型化 `double?` 字段） |
| `@scroll scrub: true` | 平滑系数（布尔 true = 1s） | `scrub > 0` 指数平滑 | `_OmniScrollDrivenWidget(scrub:)` 指数平滑 |
| `@drag-* constraints: [min, max]` | 拖拽范围限制 + 弹性回弹 | 运行时 clamp + `(1 - elastic)` 回弹 | `_OmniDragWidget(constraintsMin: min, constraintsMax: max)` |
| `@drag-* elastic:` / `threshold:` | 弹性系数 / 吸附阈值 | 生成数值常量 | 类型化构造参数 |

> 缺省行为：`@scroll` 无 `start`/`end` 时回落视口相对字符串配置（`"top 80%"` → `"bottom 20%"`）；`@drag-*` 无 `constraints` 时回落关键帧范围推导（`maxExtent`），无可用范围时生成代码中带警告注释。诚实降级（ADR-054）：释放后惯性 decay 在三个目标上均**未实现**——释放行为统一为 threshold 吸附（play/reverse）。`velocity_max` 已于 v4 移除（零消费者——拖拽速度跟踪器无消费方，见 deferral #84）。

### 示例

```omni
<!-- @enter — 入场 -->
<div @enter={fadeInUp}>Content</div>

<!-- @hover — 悬停 -->
<button @hover={pulse} @press={scalePress}>Click</button>

<!-- @focus — 聚焦 -->
<button @focus={focusGlow}>Focus me</button>

<!-- @if 过渡（mode: 各目标支持情况见 06-模板语法 → 过渡模式） -->
@if isOpen mode: "fade" {
    <div @enter={fadeIn}>Modal</div>
}

<!-- @in-view — 视口触发器 -->
<div @in-view={slideIn}>
    Scroll to reveal
</div>

<!-- @scroll — 滚动驱动 -->
<div @scroll={scrollProgress}>
    Scroll progress
</div>

<!-- @drag-horizontal — 水平拖拽 -->
<div @drag-horizontal={dragSlide}>
    Drag me →
</div>

<!-- @drag-vertical — 垂直拖拽 -->
<div @drag-vertical={dragSlideY}>
    Drag me ↓
</div>
```

## 元素跟随（@drag 意图）vs 拖拽 scrubbing（@drag-*）

**元素跟随指针**（60fps 高频拖拽）用 **`@drag` 意图指令 + 动态 `style: { transform }` 绑定**（ADR-073）——`@drag` 处理器把累计位移写入 signal，`style` 动态条目经**直写通道**落到目标（React `useRef` 直接 DOM 写 / Flutter `ValueNotifier` + `ListenableBuilder` / OmniView `el.style.setProperty`），不经过框架 diff/reconciliation（见 [09-样式系统](09-styling.md)）：

```omni
component Draggable {
    ref dragBox: Element;
    signal x: f64 = 0;
    signal y: f64 = 0;

    fn onDrag(e) {
        x = e.dx;
        y = e.dy;
    }

    view {
        <div @ref={dragBox} @drag={onDrag}
             style: { transform: "translate(" + x + "px, " + y + "px)" }>
            <p>Drag me</p>
        </div>
    }
}
```

**进度控制**（手势 → keyframe 进度，scrubbing）用 `@drag-horizontal` / `@drag-vertical` 触发器 + `@motion` 关键帧；拖拽进度范围可用 `constraints:` 子参数显式约束（v2 Task 5.1 恢复），缺省回落关键帧范围推导（`maxExtent`），无可用范围时生成代码中带警告注释（拖拽无位移）：

```omni
@motion dragSlideX {
    from { translateX: 0 }
    to   { translateX: 100 }
    over: 300ms
}

component Draggable {
    view {
        <div @drag-horizontal={dragSlideX}>
            <p>Drag me — 拖拽距离驱动动画进度</p>
        </div>
    }
}
```

## 架构要点

- 动画运行时内联进编译产物（零外部依赖）
- 所有动画使用 WAAPI（`el.animate()`），GPU 加速、独立于主线程
- 编译时通过 `include_str!` 将运行时嵌入输出
- `@enter` 在 signal 变化导致条件渲染切换时自动重播
- `@for key={...}` 列表支持 FLIP 动画（通过 `@motion` 配合 `fill: forwards`）
