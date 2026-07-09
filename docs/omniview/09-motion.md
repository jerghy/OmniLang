# 09 — 运动与动画（Motion & Animation）

OmniView 提供声明式的动画系统，包括 `@motion`（运动定义）和 `@timeline`（时间线编排）。

## @motion 运动定义

### 关键帧动画

最基本的动画形式——从起始状态过渡到结束状态：

```omni
@motion fadeInUp {
    from { opacity: 0, y: 20 }
    to   { opacity: 1, y: 0 }
    over 400
    ease easeOut
}
```

### 多关键帧动画

支持百分比关键帧：

```omni
@motion bounce {
    from  { scale: 1 }
    30%   { scale: 1.15 }
    50%   { scale: 0.95 }
    70%   { scale: 1.05 }
    to    { scale: 1 }
    over 600
    ease easeInOut
}
```

### 可动画属性

| 类别 | 属性 |
|------|------|
| 变换 | `x`, `y`, `z`, `scale`, `scaleX`, `scaleY`, `rotate`, `rotateX`, `rotateY`, `rotateZ`, `skew`, `skewX`, `skewY` |
| 透明度 | `opacity` |
| 颜色 | `backgroundColor`, `color`, `borderColor` |
| 尺寸 | `width`, `height` |
| 路径 | `path`, `pathRotate`（沿 SVG 路径运动） |
| 滤镜 | `blur`, `brightness`, `contrast`, `saturate` |
| 其他 | `clipPath`, `borderRadius` |

### 播放控制属性

```omni
@motion pulse {
    to { scale: 1.05 }
    over 800
    ease easeInOut

    // 播放控制
    loop                    // 无限循环
    // loop: 5              // 循环 5 次
    direction alternate     // normal | reverse | alternate | alternateReverse
    delay 200               // 延迟 200ms 开始
    paused                  // 初始暂停

    // Stagger（子元素逐次动画）
    delayChildren 30
    staggerChildren 25
    staggerFrom start       // start | center | end | edges

    // 高级
    fill forwards           // none | forwards | backwards | both
    repeatDelay 100
    timeScale 1.5
    interrupt replace       // stop | complete | queue | replace
    respectReducedMotion    // 尊重系统减少动效设置
}
```

### 回调

```omni
@motion withCallbacks {
    from { opacity: 0 }
    to   { opacity: 1 }
    over 300
    ease easeOut

    onStart {
        // 动画开始时执行
    }
    onUpdate {
        // 每帧执行
    }
    onComplete {
        // 动画完成时执行
    }
    onRepeat {
        // 每次重复时执行
    }
    onInterrupt {
        // 被中断时执行
    }
}
```

## 缓动函数（Easing Functions）

### 命名缓动

```omni
ease linear
ease easeIn
ease easeOut
ease easeInOut

// Power 系列
ease power1In      ease power1Out      ease power1InOut
ease power2In      ease power2Out      ease power2InOut
ease power3In      ease power3Out      ease power3InOut
ease power4In      ease power4Out      ease power4InOut

// Bounce 弹性
ease bounceIn      ease bounceOut      ease bounceInOut
```

### 高级缓动

**Cubic Bezier**：
```omni
ease cubic(x1: 0, y1: 0, x2: 1, y2: 1)
```

**Spring 弹簧**：
```omni
ease spring(tension: 200, friction: 15, velocity: 0)
```

**Back（回退）**：
```omni
ease backIn(intensity: 1.7)
ease backOut(intensity: 1.7)
ease backInOut(intensity: 1.7)
```

**Elastic（弹性）**：
```omni
ease elasticIn(amplitude: 1.0, period: 0.5)
ease elasticOut(amplitude: 1.0, period: 0.5)
ease elasticInOut(amplitude: 1.0, period: 0.5)
```

**Steps（阶梯）**：
```omni
ease steps(count: 5, direction: end)
```

## 动画触发器

在 HTML 元素上使用 `@trigger` 属性将动画绑定到 DOM 生命周期事件：

### 入场/离场动画

```omni
<div @enter={fadeInUp} @leave={fadeOut}>
    Conditional content
</div>
```

### 交互动画

| 触发器 | 触发时机 |
|--------|----------|
| `@enter={motion}` | 元素进入 DOM |
| `@leave={motion}` | 元素离开 DOM |
| `@hover={motion}` | 鼠标悬停 |
| `@unhover={motion}` | 鼠标离开 |
| `@press={motion}` | 按下（click/touch） |
| `@focus={motion}` | 获得焦点 |
| `@animate={motion}` | 持续播放 |
| `@inView={motion}` | 滚动到视口内 |
| `@scroll={timeline}` | 滚动驱动的进度动画 |
| `@dragHorizontal={motion}` | 水平拖拽 |
| `@dragVertical={motion}` | 垂直拖拽 |
| `@timeline={timelineName}` | 时间线绑定 |

### 触发器综合示例

```omni
component MotionDemo {
    signal isOpen: bool = false;
    signal count: i32 = 0;

    fn toggle() { isOpen = !isOpen; }

    view {
        <div>
            <!-- 入场动画（h1 首次渲染时执行 fadeInUp） -->
            <h1 @enter={fadeInUp}>Motion Demo</h1>

            <!-- 按钮：按下 + 悬停组合 -->
            <button @press={scalePress} @hover={pulse} @onPress={toggle}>
                Toggle
            </button>

            <!-- 条件渲染 + 入场/离场动画 -->
            @if isOpen mode="sync" {
                <div @enter={fadeInUp} @leave={fadeInUp}>
                    <p>Modal Content</p>
                </div>
            }

            <!-- 多状态动画 -->
            <div states: {
                idle:    { scale: 1, opacity: 1 }   over: 150,
                hover:   { scale: 1.05, opacity: 1 } over: 150,
                pressed: { scale: 0.95, opacity: 0.8 } over: 100
            }>
                Interactive Element
            </div>
        </div>
    }
}
```

## @timeline 时间线

时间线用于编排多个动画的**顺序**和**并行**关系。

### 基本语法

```omni
@timeline mySequence {
    item1 { from { x: -20, opacity: 0 } to { x: 0, opacity: 1 } over 300 ease power2Out }
    then
    item2 { from { y: 20, opacity: 0 } to { y: 0, opacity: 1 } over 300 ease power2Out }
    with
    item3 { from { scale: 0.8 } to { scale: 1 } over 200 ease easeOut }
    then
    stagger(0.05)
    item4 { from { opacity: 0 } to { opacity: 1 } over 200 }
}
```

### 连接器

| 连接器 | 说明 |
|--------|------|
| `then` | 顺序——上一个动画完成后才播放下一个 |
| `with` | 并行——与上一个动画同时播放 |
| `stagger(N)` | 交错——子元素依次延迟 N 播放 |

### 时间线项目类型

| 类型 | 语法 | 说明 |
|------|------|------|
| 方向动画 | `name { from/to }` | 关键帧动画 |
| 绝对设置 | `set { opacity: 0 }` | 立即设置属性 |
| 动画引用 | `refMotion` | 引用已定义的 @motion |
| 标签 | `label phase1` | 命名时间线段落 |
| 起始位置 | `start: "after"` | 调整播放起始点 |

### 起始位置修饰符

```omni
start: "after"              // 上一个完成之后
start: "withPrev"           // 与上一个一起开始
start: "overlap(150)"       // 在上一个结束前 150ms 开始
start: "afterPrev(id)"      // 在指定动画之后
start: "withOffset(50)"     // 延迟 50ms 开始
start: "absolute(0ms)"      // 从时间线绝对位置开始
```

### 在元素上使用时间线

```omni
<div @timeline={mySequence}>
    <div data-box="item1">Box 1</div>
    <div data-box="item2">Box 2</div>
    <div data-box="item3">Box 3</div>
    <div data-box="item4">Box 4</div>
</div>
```

时间线动画元素通过 `data-box="name"` 属性与时间线中的动画名称关联。

## 滚动驱动动画

### @inView（滚动进入视口时触发）

```omni
<div @inView={fadeInUp}
     @in-view-once={false}          // 每次进入都触发（true = 仅一次）
     @in-view-threshold={0.3}>     // 可见 30% 时触发
    Scroll-triggered content
</div>
```

### @scroll（滚动驱动的进度动画）

```omni
<div @scroll={scrollTimeline}
     @scroll-start="top 80%"
     @scroll-end="bottom 20%"
     @scroll-scrub={0.5}>
    Progressively revealed content
</div>
```

## 拖拽动画

```omni
<div @dragHorizontal={dragAnim}
     @dragVertical={dragAnim}
     @drag-constraints="{ left: 0, right: 300, top: 0, bottom: 200 }"
     @drag-elastic={0.5}
     @drag-momentum={true}
     @drag-threshold={10}>
    Drag me
</div>
```

## Hero 共享元素过渡

跨组件/页面的共享元素平滑过渡：

```omni
<!-- 在源组件中 -->
<div @hero="profile-avatar">
    <img src={user.avatar} />
</div>

<!-- 在目标组件中 -->
<div @hero="profile-avatar" @hero-transition={heroAnim}>
    <img src={user.avatar} />
</div>
```

相同的 `@hero` 标签名使两个元素建立过渡关联。

## 动画注意事项

1. **Motion 运行时按需加载**：仅当项目使用动画功能时，~46KB 的 motion 运行时库才被嵌入
2. **@enter 重播**：当组件的 signal 变化时，`@enter` 动画会自动重播
3. **性能**：动画属性应优先使用 `transform` 和 `opacity`（GPU 加速）
4. **Stagger**：`staggerFrom` 控制子元素交错的起始方向（`start`、`end`、`center`、`edges`）
5. **@for 列表动画**：在 `@for key={...}` 中使用 key 可实现列表项的入场/离场动画
