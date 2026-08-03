# 11 — 运动与动画（Motion & Animation）

OmniView 提供声明式动画系统：`@motion` 定义补间动画参数，通过 8 个触发器绑定到元素。

**ADR-032 范围界定**：OmniView 动画系统保留 14 项核心功能，移除 19 项（`@timeline`、`states:{}`、Hero、stagger、回调等）。
**ADR-052/053 语义级 DSL**（2026-08-01 同步）：动画语法更新为冒号形式（`over:`/`easing:`）、17 属性语义白名单、8 触发器（kebab-case）；触发器子属性与 `@paused` 已移除。本文档仅覆盖保留功能。**v2（2026-08-03，Task 5.2）**：白名单扩展为 19 项语义集（新增 `borderRadius`/`fontSize`，三目标均已验证并实现）。

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
easing: steps(6, start)                                  // 阶梯（Flutter 目标：error，无原生 steps）
```

- **Named-only**（Back、Elastic）不接收参数——参数形式（`backOut(intensity: 1.7)`、`elasticOut(amplitude: 1.0)`、`spring(tension: 200)`）已随 ADR-032 移除
- `steps()` 以位置参数形式保留（`steps(count, start|end)`，语义级 DSL 新形式）

> 注：`cubic(...)` 与 `delay <expr>`（空格形式）为 **parser 实际接受**的形式。BNF 规范（`docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`）已按 parser 同步（commit 10f2af9c）：`cubic(x1:, y1:, x2:, y2:)` 命名参数形式、`delay <expr>` 空格形式、`direction` 规则含 `alternateReverse`（见 `.architecture/deferrals.md` #25，已关闭）。

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

> 触发器子属性（`@in-view-once`、`@scroll-start`、`@drag-constraints` 等）已按 semantic-level DSL 规范移除（ADR-053）；拖拽范围约束待重新实现，当前无约束时进度为 0（见 `.architecture/deferrals.md` #19）。

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

## 与拖拽触发器配合实现高频拖拽（@drag-*）

`ref.style.*` 已按 ADR-052 删除（DOM 专属 API，用信号 + style 绑定替代）。拖拽驱动的 60fps 位移由 `@drag-*` 触发器 + `@motion` 关键帧实现，最终值用 signal 保存。注意：当前无约束时拖拽进度为 0（范围约束待重新实现，见 `.architecture/deferrals.md` #19）：

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
