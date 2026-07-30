# 11 — 运动与动画（Motion & Animation）

OmniView 提供声明式动画系统：`@motion` 定义补间动画参数，通过 7 个触发器绑定到元素。

**ADR-032 范围界定**：OmniView 动画系统保留 14 项核心功能，移除 19 项（`@timeline`、`states:{}`、Hero、stagger、回调等）。本文档仅覆盖保留功能。

## @motion 运动定义

### 基本关键帧

```omni
@motion fadeInUp {
    from { opacity: 0, y: 20 }
    to   { opacity: 1, y: 0 }
    over 400
    ease easeOut
}
```

### 多停靠关键帧（百分比）

```omni
@motion multiStop {
    0%   { opacity: 0, scale: 0.5, background: "#6366f1" }
    25%  { opacity: 0.5, scale: 0.8 }
    50%  { opacity: 1, scale: 1.2, background: "#10b981" }
    75%  { opacity: 1, scale: 1.05 }
    100% { opacity: 1, scale: 1, background: "#f59e0b" }
    over 1200 ease easeInOut
}
```

支持任意百分比停靠点（0%–100%），未用的中间帧由浏览器自动插值。

### 可动画属性（26 个，分 7 组）

| 类别 | 属性 |
|------|------|
| **变换**（7） | `x` `y` `scale` `scaleX` `scaleY` `rotate` `rotateZ` `rotateX` `rotateY` `skewX` `skewY` |
| **滤镜**（5） | `blur` `brightness` `contrast` `saturate` `hueRotate` |
| **颜色**（4） | `backgroundColor` `color` `borderColor` `borderRadius` |
| **尺寸**（4） | `width` `height` `margin` `padding` |
| **排版**（2） | `fontSize` `letterSpacing` |
| **盒模型**（1） | `borderWidth` |
| **定位**（2） | `top` `left` |
| **基础**（1） | `opacity` |

### 播放控制（5 属性）

```omni
@motion pulse {
    to { scale: 1.05 }
    over 800 ease easeInOut

    loop                       // 无限循环
    direction alternate        // normal | reverse | alternate | alternateReverse
    delay 200                  // 延迟 ms（仅在首次播放生效）
    paused                     // 初始暂停（需 @paused={signal} 运行时控制）
    fill forwards              // none | forwards | backwards | both
}
```

| 属性 | 可选值 | 说明 |
|------|--------|------|
| `loop` | — | 无限重复播放 |
| `direction` | `normal` `reverse` `alternate` `alternateReverse` | 播放方向 |
| `delay` | `{N}` | 首次播放延迟 N 毫秒 |
| `paused` | — | 默认暂停，需通过 `@paused={signal}` 恢复 |
| `fill` | `none` `forwards` `backwards` `both` | 动画前后的样式填充模式 |

## 缓动函数（20 种）

### 命名缓动

```omni
// 基础
ease easeIn / easeOut / easeInOut
ease linear

// Power 系列（1 = 二次方, 2 = 三次方, 3 = 四次方, 4 = 五次方）
ease power1In / power1Out / power1InOut
ease power2In / power2Out / power2InOut
ease power3In / power3Out / power3InOut
ease power4In / power4Out / power4InOut

// Back（回弹，named-only，无参数）
ease backIn / backOut / backInOut

// Bounce（弹跳）
ease bounceIn / bounceOut / bounceInOut

// Elastic（弹性，named-only，无参数）
ease elasticIn / elasticOut / elasticInOut
```

### 自定义缓动

```omni
ease cubic(x1: 0.68, y1: -0.55, x2: 0.27, y2: 1.55)  // 三次贝塞尔
```

- **Named-only**（Back、Elastic）不接收参数——参数形式（`backOut(intensity: 1.7)`、`elasticOut(amplitude: 1.0)`、`spring(tension: 200)`、`steps(count: 5)`）已随 ADR-032 移除

## 动画触发器（7 个）

| 触发器 | 触发时机 | 支持子属性 |
|--------|----------|-----------|
| `@enter={motion}` | 元素进入 DOM / 条件变为 true | — |
| `@hover={motion}` | 鼠标悬停 | — |
| `@press={motion}` | 按下（click/touch） | — |
| `@inView={motion}` | 元素滚动到视口内 | `@in-view-once` `@in-view-repeat` `@in-view-threshold` |
| `@scroll={motion}` | 滚动驱动进度动画 | `@scroll-start` `@scroll-end` `@scroll-scrub` |
| `@dragHorizontal={motion}` | 水平拖拽 | `@drag-constraints` `@drag-elastic` `@drag-scrub` `@drag-momentum` `@drag-threshold` `@drag-velocity-max` |
| `@dragVertical={motion}` | 垂直拖拽 | （同上） |

### 示例

```omni
<!-- @enter — 入场 -->
<div @enter={fadeInUp}>Content</div>

<!-- @hover — 悬停 -->
<button @hover={pulse} @press={scalePress}>Click</button>

<!-- @if 过渡 + @enter/@leave -->
@if isOpen mode="sync" {
    <div @enter={fadeIn} @leave={fadeOut}>Modal</div>
}

<!-- @inView — 视口触发器 + 子属性 -->
<div @inView={slideIn}
     @in-view-once={true}
     @in-view-threshold={0.5}>
    Scroll to reveal
</div>

<!-- @scroll — 滚动驱动 + 子属性 -->
<div @scroll={scrollProgress}
     @scroll-start="top 80%"
     @scroll-end="bottom 20%"
     @scroll-scrub={0.5}>
    Scroll progress
</div>

<!-- @dragHorizontal — 水平拖拽 + 子属性 -->
<div @dragHorizontal={dragSlide}
     @drag-constraints="left:0 right:300"
     @drag-elastic={0.2}
     @drag-scrub={true}
     @drag-momentum={0.95}
     @drag-threshold={30}
     @drag-velocity-max={2000}>
    Drag me →
</div>

<!-- @dragVertical — 垂直拖拽 -->
<div @dragVertical={dragSlideY}
     @drag-constraints="top:0 bottom:200"
     @drag-elastic={0.3}
     @drag-scrub={true}>
    Drag me ↓
</div>
```

## @paused 运行时控制

通过元素属性 `@paused={signal}` 运行时暂停/恢复动画：

```omni
component PauseDemo {
    signal isPaused: bool = false;

    fn togglePause() { isPaused = !isPaused; }

    view {
        <div @enter={spinLoop} @paused={isPaused}>
            <strong>旋转动画</strong>
        </div>
        <button @onPress={togglePause}>
            {if isPaused { "▶ Resume" } else { "⏸ Pause" }}
        </button>
    }
}
```

- `isPaused = false` → 动画播放
- `isPaused = true`  → 动画暂停（保持当前帧）
- 切换 `isPaused` → 动画从暂停处恢复/立刻暂停

## 拖拽子属性表

| 子属性 | 类型 | 说明 |
|--------|------|------|
| `@drag-constraints` | `String` | 拖拽边界，如 `"left:0 right:300"` `"top:0 bottom:200"` |
| `@drag-elastic` | `f64` | 回弹系数（0=刚性边界, 0.5=中等回弹） |
| `@drag-scrub` | `bool` | 是否实时 scrub 动画进度 |
| `@drag-momentum` | `f64` | 动量系数（0=无惯性, 0.95=高惯性） |
| `@drag-threshold` | `i32` | 拖拽触发阈值（像素） |
| `@drag-velocity-max` | `i32` | 最大速度限制（px/s） |

## 滚动子属性表

| 子属性 | 类型 | 说明 |
|--------|------|------|
| `@scroll-start` | `String` | 触发起点，如 `"top 80%"` |
| `@scroll-end` | `String` | 触发终点，如 `"bottom 20%"` |
| `@scroll-scrub` | `f64` | 进度关联度：0=完全跟随, 0.5=平滑跟随 |

## 视口子属性表

| 子属性 | 类型 | 说明 |
|--------|------|------|
| `@in-view-once` | `bool` | 是否仅播放一次 |
| `@in-view-repeat` | `bool` | 每次滚入视口都重新播放 |
| `@in-view-threshold` | `f64` | 触发阈值（0.0–1.0），表示元素可见比例 |

## 与 Element Reference 配合实现高频拖拽

```omni
component Draggable {
    ref dragBox: Element;
    signal x: f64 = 0;
    signal y: f64 = 0;

    fn onDragUpdate(e) {
        // ref.style 支持 60fps 直接 DOM 操作
        dragBox.style.transform = "translate(" + e.x.to_string() + "px, " + e.y.to_string() + "px)";
        x = e.x;
        y = e.y;
    }

    view {
        <div @ref={dragBox}
             @dragHorizontal={fadeSlideUp}>
            <p>Drag me — x:{x.to_string()} y:{y.to_string()}</p>
        </div>
    }
}
```

同时在 `@dragHorizontal` 触发器中更新 ref.style（高帧率）和 signal（最终值）。

## 架构要点

- 动画运行时基于 GSAP 兼容层（通过 `@omni/animation` 内联，零外部依赖）
- 所有动画使用 WAAPI（`el.animate()`），GPU 加速、独立于主线程
- 编译时通过 `include_str!` 将运行时嵌入输出
- `@enter` 在 signal 变化导致条件渲染切换时自动重播
- `@for key={...}` 列表支持 FLIP 动画（通过 `@motion` 配合 `fill: forwards`）
