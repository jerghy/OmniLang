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

<!-- 事件简写：直接传参数 -->
<button @onPress={(e) => { handleEvent(e); }}>Log</button>
```

`@onPress` 和 `@onClick` 等价（都映射到 `click`），`@onPress` 是推荐写法。

## 事件数据（Event Data）

编译器分析函数体中访问的字段，自动构建最小事件对象：

```omni
fn handleKey(e) {
    if e.key == "Escape" { close(); }
    if e.ctrl { /* Ctrl 按下 */ }
}

fn handleMouse(e) {
    x = e.x;             // e.clientX
    y = e.y;             // e.clientY
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
| 标准 | `@onClick={handler}` | 显式命名事件 |
| 简写 | `({ fnName })` 或 `{handler}` | 自动包装（部分场景） |
| 内联闭包 | `@onPress={() => { ... }}` | 直接写逻辑 |
| Store action | `@onPress={TodoStore.add("x")}` | 调用 store 方法 |
| 函数调用 | `@onPress={switchTab("home")}` | 带参数调用组件函数 |
