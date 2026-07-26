# 11 — 运动与动画（Motion & Animation）

OmniView 提供声明式动画系统：`@motion`（运动定义）和 `@timeline`（时间线编排）。

## @motion 运动定义

### 关键帧动画

```omni
@motion fadeInUp {
    from { opacity: 0, y: 20 }
    to   { opacity: 1, y: 0 }
    over 400
    ease easeOut
}
```

### 多关键帧

```omni
@motion bounce {
    from  { scale: 1 }
    30%   { scale: 1.15 }
    50%   { scale: 0.95 }
    70%   { scale: 1.05 }
    to    { scale: 1 }
    over 600 ease easeInOut
}
```

### 可动画属性

| 类别 | 属性 |
|------|------|
| 变换 | `x` `y` `z` `scale` `scaleX` `scaleY` `rotate` `rotateX` `rotateY` `rotateZ` `skew` `skewX` `skewY` |
| 透明度 | `opacity` |
| 颜色 | `backgroundColor` `color` `borderColor` |
| 尺寸 | `width` `height` |
| 滤镜 | `blur` `brightness` `contrast` `saturate` |
| 其他 | `clipPath` `borderRadius` |

### 播放控制

```omni
@motion pulse {
    to { scale: 1.05 }
    over 800 ease easeInOut

    loop                       // 无限循环
    direction alternate        // normal | reverse | alternate | alternateReverse
    delay 200                  // 延迟 ms
    paused                     // 初始暂停

    // Stagger（子元素逐次）
    delayChildren 30
    staggerChildren 25
    staggerFrom start          // start | center | end | edges

    // 高级
    fill forwards              // none | forwards | backwards | both
    timeScale 1.5
    interrupt replace          // stop | complete | queue | replace
}
```

### 回调

```omni
@motion withCallbacks {
    from { opacity: 0 } to { opacity: 1 }
    over 300 ease easeOut

    onStart { /* 开始时 */ }
    onUpdate { /* 每帧 */ }
    onComplete { /* 完成时 */ }
    onRepeat { /* 重复时 */ }
}
```

## 缓动函数

### 命名缓动

```omni
ease linear
ease easeIn / easeOut / easeInOut

// Power 系列
ease power1In / power1Out / power1InOut
ease power2In / power2Out / power2InOut
ease power3In / power3Out / power3InOut
ease power4In / power4Out / power4InOut

// Bounce
ease bounceIn / bounceOut / bounceInOut
```

运行时通过 `resolveEasing()` 自动转换为 WAAPI 兼容格式。

### 高级缓动

```omni
ease cubic(x1: 0, y1: 0, x2: 1, y2: 1)              // 自定义贝塞尔
ease spring(tension: 200, friction: 15)               // 弹簧
ease backOut(intensity: 1.7)                          // 回退
ease elasticOut(amplitude: 1.0, period: 0.5)          // 弹性
ease steps(count: 5, direction: end)                   // 阶梯
```

## states: 多状态动画

直接在元素上定义状态机：

```omni
<div states: {
    idle:    { scale: 1, opacity: 1 }   over: 150,
    hover:   { scale: 1.05 }            over: 150,
    pressed: { scale: 0.95 }            over: 100
}>
    Interactive
</div>
```

**保留状态名**（自动绑定事件）：`idle`（默认）`hover`（mouseenter）`pressed`（mousedown）

**自定义状态名**（由 signal 驱动）：
```omni
signal loadState: String = "loading";

<div states: {
    idle: { opacity: 1 } over: 300,
    loading: { opacity: 0.5 } over: 300,
    success: { borderColor: "#10b981" } over: 300
}>
```

## 动画触发器

| 触发器 | 触发时机 |
|--------|----------|
| `@enter={motion}` | 元素进入 DOM |
| `@leave={motion}` | 元素离开 DOM |
| `@hover={motion}` / `@unhover={motion}` | 鼠标悬停/离开 |
| `@press={motion}` | 按下（click/touch） |
| `@focus={motion}` | 获得焦点 |
| `@animate={motion}` | 持续循环播放 |
| `@inView={motion}` | 滚动到视口内 |
| `@scroll={timeline}` | 滚动驱动进度动画 |
| `@dragHorizontal={motion}` / `@dragVertical={motion}` | 拖拽 |
| `@timeline={name}` | 时间线绑定 |

### 示例

```omni
<div @enter={fadeInUp}>Content</div>

<button @press={scalePress} @hover={pulse}>Click</button>

@if isOpen mode="sync" {
    <div @enter={fadeIn} @leave={fadeOut}>Modal</div>
}

<!-- 滚动触发 -->
<div @inView={slideIn}
     @in-view-once={false}
     @in-view-threshold={0.3}>
    Scroll to reveal
</div>

<!-- 拖拽 -->
<div @dragHorizontal={slideAnim}
     @drag-constraints="left:0 right:300"
     @drag-elastic={0.5}>
    Drag me
</div>
```

## @timeline 时间线

编排多个动画的顺序和并行关系：

```omni
@timeline mySequence {
    item1 { from { x: -20, opacity: 0 } to { x: 0, opacity: 1 } over 300 ease power2Out }
    then
    item2 { from { y: 20, opacity: 0 } to { y: 0, opacity: 1 } over 300 }
    with
    item3 { from { scale: 0.8 } to { scale: 1 } over 200 ease easeOut }
}
```

### 连接器

| 连接器 | 说明 |
|--------|------|
| `then` | 顺序——上一个完成后才播放 |
| `with` | 并行——与上一个同时播放 |
| `stagger(N)` | 交错——子元素依次延迟 N |

### 项目类型

| 类型 | 语法 | 说明 |
|------|------|------|
| 方向动画 | `name { from/to }` | 关键帧 |
| 绝对设置 | `set { opacity: 0 }` | 立即设置属性 |
| 动画引用 | `refMotion` | 引用 @motion |
| 标签 | `label phase1` | 命名段落 |

### 起始位置

```omni
start: "withPrev"          // 与上一个同时开始
start: "overlap(150)"      // 在上一个结束前 150ms
start: "afterPrev(id)"     // 在指定动画之后
start: "withOffset(50)"    // 延迟 50ms
start: "absolute(0ms)"     // 从绝对位置
```

### 在元素上使用

```omni
<div @timeline={mySequence}>
    <div data-box="item1">Box 1</div>
    <div data-box="item2">Box 2</div>
</div>
```

元素通过 `data-box="name"` 与时间线动画名称关联。

## Hero 共享元素过渡

```omni
<!-- 源组件 -->
<div @hero="avatar">
    <img src={user.avatar} />
</div>

<!-- 目标组件 -->
<div @hero="avatar" @hero-transition={heroAnim}>
    <img src={user.avatar} />
</div>
```

相同 `@hero` 标签建立跨组件过渡关联。

## 架构要点

- 动画运行时基于 `@omni/animation`（~16KB，零外部依赖）
- 所有动画使用 WAAPI（`el.animate()`），GPU 加速
- 编译时通过 `include_str!` 嵌入运行时
- `@for key={...}` 列表支持 FLIP 动画
- `@enter` 在 signal 变化时自动重播
