# 07 — 事件与绑定

## 事件处理器

在元素上使用 `@onEventName` 绑定事件：

```omni
<!-- 引用组件函数 -->
<button @onPress={handleClick}>Click</button>

<!-- 内联闭包 -->
<button @onPress={() => { count = count + 1; }}>+1</button>

<!-- Store action -->
<button @onPress={TodoStore.add("item")}>Add</button>

<!-- 带参数调用函数 -->
<button @onPress={switchTab("home")}>Home</button>

<!-- 内联闭包 -->
<button @onPress={(e) => { handleEvent(e); }}>Log</button>
```

`@onPress` 和 `@onClick` 等价（都映射到 `click`），`@onPress` 是推荐写法。

## 事件数据（Event Data）

编译器分析函数体中访问的字段，自动构建最小事件对象：

```omni
component EventsDemo {
    signal x: f64 = 0;
    signal y: f64 = 0;

    fn close() { print("closed"); }

    fn handleKey(e) {
        if e.key == "Escape" { close(); }
        if e.ctrl { /* Ctrl 按下 */ }
    }

    fn handleMouse(e) {
        x = e.x;             // e.clientX
        y = e.y;             // e.clientY
    }

    view {
        <div @onKeyDown={handleKey} @onMouseMove={handleMouse}>Area</div>
    }
}
```

修饰键字段（适用所有鼠标/键盘/触摸事件）：

| 字段 | 运行时属性 |
|------|-----------|
| `e.ctrl` | `e.ctrlKey` |
| `e.shift` | `e.shiftKey` |
| `e.alt` | `e.altKey` |
| `e.meta` | `e.metaKey` |

## 事件类型参考

### 鼠标/指针

| 事件 | 典型字段 | 说明 |
|------|---------|------|
| `@onPress` / `@onClick` | `x`, `y`, `button` | 点击 |
| `@onDoubleClick` | `x`, `y` | 双击 |
| `@onContextMenu` | `x`, `y` | 右键菜单 |
| `@onMouseEnter` / `@onMouseLeave` | — | 悬停进出 |
| `@onMouseMove` | `x`, `y` | 鼠标移动 |

### 输入/键盘

| 事件 | 典型字段 |
|------|---------|
| `@onInput` | `value` |
| `@onChange` | `value`, `checked` |
| `@onKeyUp` / `@onKeyDown` | `key`, `code` |
| `@onFocus` / `@onBlur` | `value` |
| `@onSubmit` | `values` |

### 剪贴板

| 事件 | 字段 |
|------|------|
| `@onCopy` / `@onCut` / `@onPaste` | `text` |

### 触摸

| 事件 | 字段 |
|------|------|
| `@onTouchStart` / `@onTouchEnd` / `@onTouchMove` | `touches` |

### 拖拽

| 事件 | 字段 |
|------|------|
| `@onDragStart` / `@onDragOver` / `@onDragEnd` / `@onDrop` | `x`, `y`（Drop 另有 `files`, `text`） |

### 拖拽意图指令（@drag / @dragEnd）

`@drag` / `@dragEnd` 是**拖拽意图指令**（ADR-073）——声明「元素可拖拽」，指针捕获由编译器按目标自行选路（React `setPointerCapture` / OmniView 原语组合 / Flutter GestureDetector pan）：

```omni
<div @ref={box} @drag={onDrag} @dragEnd={onDragEnd} />
```

| 指令 | 触发时机 | 事件数据 | 约束 |
|------|---------|---------|------|
| `@drag` | 指针按下 → 捕获 → 持续回调 | `{x, y, dx, dy, pointerId, startX, startY}` | 需同元素 `@ref` |
| `@dragEnd` | 指针抬起/取消 | 同 `@drag` | 可选 |

- 语义：指针在元素上按下 → 捕获拖动（移出元素仍跟随）→ 拖动期间持续回调 `@drag`，抬起回调 `@dragEnd`；`dx`/`dy` 为相对按下点的**累计位移**（数值型）
- 处理器必须是**组件函数名**（禁止内联箭头/调用），0-1 参数（拖拽事件对象）
- `@drag` 与 `@dragEnd` **顺序无关**；`@dragEnd` 不能单独出现，二者不能重复
- `@drag` 要求同元素 `@ref`（语义验证硬错误）；`capturePointer()` / `releasePointer()` 已降级为编译器内部实现细节——源码不显式书写

> **与 `@onDrag` 的区别**：`@onDrag`（上表「拖拽」行）是 HTML5 标准 `drag` 事件（DragEvent），被动接收浏览器拖放协议事件；`@drag` 是 OmniLang 拖拽意图指令（指针按下 → 捕获 → 跟随回调），二者完全独立。与 `@drag-horizontal` / `@drag-vertical` 也不同——后者是 motion scrubbing 语法糖（手势 → keyframe 进度），前者是元素拖拽意图（移动元素跟随指针）。

### 滚轮/滚动

| 事件 | 字段 |
|------|------|
| `@onWheel` | `deltaX`, `deltaY`, `deltaZ`, `x`, `y` |
| `@onScroll` | `x`, `y` |

### 媒体

| 事件 | 字段 |
|------|------|
| `@onPlay` / `@onPause` / `@onEnded` | — |
| `@onLoad` / `@onError` | — |

### 动画/过渡

| 事件 | 字段 |
|------|------|
| `@onAnimationStart` / `@onAnimationEnd` | `animationName`, `elapsedTime` |
| `@onAnimationIteration` | `animationName`, `elapsedTime` |
| `@onTransitionEnd` | `propertyName`, `elapsedTime` |

### 切换

| 事件 | 字段 |
|------|------|
| `@onToggle` | — |

### 窗口

| 事件 | 字段 |
|------|------|
| `@onResize` | `width`, `height` |

## emit —— 组件间事件

### 声明与发射

```omni
component Child {
    event onCustomEvent: String;
    event onOptionalEvent: bool?;          // 可选事件，父组件可不处理

    fn trigger() {
        emit onCustomEvent("hello from child");
        // 等价：onCustomEvent("hello from child");
    }
}
```

### 接收

```omni
component Parent {
    signal lastMsg: String = "";

    fn handleEvent(val: String) {
        lastMsg = val;
    }

    view {
        <p>Last: {lastMsg}</p>
        <Child @onCustomEvent={handleEvent} />
    }
}
```

自定义事件的数据直接作为函数参数传入，不通过 `e.detail`。

## bind: 双向绑定

表单元素的值与 signal 双向同步：

```omni
signal name: String = "";
signal agreed: bool = false;
signal selected: String = "b";

view {
    <input bind:value={name} placeholder="Name" />
    <input type="checkbox" bind:checked={agreed} />
    <select bind:value={selected}>
        <option value="a">A</option>
        <option value="b">B</option>
    </select>

    <p>Name: {name}</p>
    <p>Checked: {agreed}</p>
    <p>Selected: {selected}</p>
}
```

数据流：
1. 用户输入 → input/change 事件 → 更新 signal
2. signal 变化 → 更新 DOM 的 value/checked

内置防抖和无限循环保护。

## 事件冒泡

- 所有事件默认冒泡，通过 `addEventListener` 绑定
- 在 `@for` 循环内，事件使用委托模式——容器上单一监听器，通过 `data-event-*` 分派

## 事件风格对比

| 风格 | 示例 | 说明 |
|------|------|------|
| 标准 | `@onClick={handler}` | 显式命名事件（`@onEventName`，推荐） |
| 无 @ 前缀 | `onPress={handler}` | `onEventName` 形式同样被解析 |
| 内联闭包 | `@onPress={() => { ... }}` | 直接写逻辑 |
| Store action | `@onPress={TodoStore.add("x")}` | 调用 store 方法 |
| 函数调用 | `@onPress={switchTab("home")}` | 带参数调用组件函数 |

> 注：`@press`、`@focus` 等（不带 `on`）是**动画触发器**，不是事件——
> 事件绑定一律使用 `@on*` 形式（详见 [11-运动与动画](11-motion-animation.md)）；
> `@drag` / `@dragEnd` 是**拖拽意图指令**（见上「拖拽意图指令」），亦非事件。
