# 06 — 事件系统

OmniView 的事件系统覆盖了所有标准 DOM 事件，并提供了 `emit` 关键字用于组件间通信。

## 事件处理器

### 绑定事件处理器

在 HTML 元素上使用 `@onEventName` 属性绑定事件处理器：

```omni
view {
    <!-- 引用组件函数 -->
    <button @onPress={handleClick}>Click</button>

    <!-- 内联闭包 -->
    <button @onPress={() => { count = count + 1; }}>+1</button>

    <!-- 调用 Store action（带参数自动封装） -->
    <button @onPress={TodoStore.add("item")}>Add</button>

    <!-- 带参数调用组件函数 -->
    <button @onPress={switchTab("home")}>Home</button>
}
```

事件名使用 PascalCase（`onPress`），编译时映射到 DOM 事件。

**`@onPress` vs `@onClick`**：两者都映射到 `click` DOM 事件，完全等价。`@onPress` 是 OmniView 推荐的写法（语义更清晰），`@click` 也完全支持。

```omni
<!-- 以下两种写法等价 -->
<button @onPress={handleClick}>Click</button>
<button @onClick={handleClick}>Click</button>
```

### 事件对象（Event Data）

事件处理器可以接收事件对象，编译器**自动提取**所需字段，生成最小事件数据：

```omni
fn handleKeyDown(e) {
    // e 包含你访问的字段
    if e.key == "Escape" { close(); }
    if e.ctrl { /* Ctrl 键被按下 */ }
}

fn handleScroll(e) {
    scrollY = e.y;        // e.target.scrollTop
}

fn handleMouseMove(e) {
    mouseX = e.x;         // e.clientX
    mouseY = e.y;         // e.clientY
}

fn handleWheel(e) {
    delta = e.deltaY;     // e.deltaY
    isCtrl = e.ctrl;      // e.ctrlKey
}
```

编译器分析事件处理器中访问了哪些字段，只为需要的字段生成提取代码（零开销抽象）。

**修饰键字段**（适用所有鼠标/键盘/触摸事件）：
| 字段 | 事件对象属性 |
|------|-------------|
| `e.ctrl` | `e.ctrlKey` |
| `e.shift` | `e.shiftKey` |
| `e.alt` | `e.altKey` |
| `e.meta` | `e.metaKey` (Cmd on Mac, Win on Windows) |

### Component Event（自定义事件）的 Event Data

组件之间通过 `event` 声明传递自定义事件，接收方直接在参数中获取数据：

```omni
// 子组件声明
component Child {
    event onAction: String;
    fn trigger() { onAction("hello"); }
}

// 父组件接收
component Parent {
    fn handleAction(msg: String) {
        // msg = "hello"
    }
    view {
        <Child @onAction={handleAction} />
    }
}
```

自定义事件的数据不通过 `e.detail` 传递——编译器直接将 payload 作为函数参数传入。

### 三种事件处理器类型

**1. 函数引用**（`FnRef`）：引用组件内部函数：

```omni
fn handleClick() {
    count = count + 1;
}

view {
    <button @onPress={handleClick}>Click</button>
}
```

**2. 内联闭包**（`InlineFn`）：直接在模板中编写处理逻辑：

```omni
view {
    <button @onPress={() => {
        count = count + 1;
        log = "clicked";
    }}>Click</button>
}
```

**3. Store Action**：直接调用 store 的 action：

```omni
view {
    <button @onPress={TodoStore.remove(item.id)}>Remove</button>
    <button @onPress={ThemeStore.toggle()}>Toggle Theme</button>
}
```

> **闭包语法区分**：view 模板中的事件处理器使用 `() => { }`（JS 箭头风格），而 `fn`/`action`/`computed` 中的函数逻辑使用 `|param| expr`（Rust 管道风格）。两种语法**不能互换**——`@onPress={|e| e.value}` 在 view 中无效，`array_filter(items, () => { ... })` 在函数逻辑中也无效。详见 [11-表达式系统](11-expressions.md#闭包--lambda)。

### 事件参数

事件处理器可以接收事件对象作为参数：

```omni
fn handleInput(e) {
    inputText = e.value;
}

fn handleKey(e) {
    if e.key == "Enter" {
        submit();
    }
}

fn handleWheel(e) {
    deltaY = e.deltaY;
}

view {
    <input @onInput={handleInput} @onKeyUp={handleKey} />
    <div @onWheel={handleWheel}>Scroll here</div>
}
```

编译器会分析事件对象中访问了哪些字段，只构建最小的事件数据对象。

## 支持的事件类型

### 鼠标/指针事件

| 事件 | 说明 | 典型事件对象字段 |
|------|------|------------------|
| `@onPress` | 点击（推荐） | `x`, `y`, `button` |
| `@onDoubleClick` | 双击 | `x`, `y` |
| `@onContextMenu` | 右键菜单 | `x`, `y` |
| `@onMouseEnter` | 鼠标进入 | `x`, `y` |
| `@onMouseLeave` | 鼠标离开 | — |

### 输入/键盘事件

| 事件 | 说明 | 典型事件对象字段 |
|------|------|------------------|
| `@onInput` | 输入变化 | `value` |
| `@onChange` | 值改变 | `value`, `checked` |
| `@onKeyUp` | 按键抬起 | `key`, `code` |
| `@onKeyDown` | 按键按下 | `key`, `code` |
| `@onKeyPress` | 按键按压 | `key` |

### 剪贴板事件

| 事件 | 说明 | 事件对象字段 |
|------|------|--------------|
| `@onCopy` | 复制 | `text` |
| `@onCut` | 剪切 | `text` |
| `@onPaste` | 粘贴 | `text` |

### 触摸事件

| 事件 | 说明 | 事件对象字段 |
|------|------|--------------|
| `@onTouchStart` | 触摸开始 | `touches` |
| `@onTouchEnd` | 触摸结束 | `touches` |
| `@onTouchMove` | 触摸移动 | `touches` |

### 拖拽事件

| 事件 | 说明 | 事件对象字段 |
|------|------|--------------|
| `@onDragStart` | 拖拽开始 | `x`, `y` |
| `@onDragOver` | 拖拽悬停 | `x`, `y` |
| `@onDragEnd` | 拖拽结束 | `x`, `y` |
| `@onDrop` | 放置 | `x`, `y`, `files`, `text` |

### 滚轮/滚动事件

| 事件 | 说明 | 事件对象字段 |
|------|------|--------------|
| `@onWheel` | 鼠标滚轮 | `deltaX`, `deltaY`, `deltaZ`, `x`, `y` |
| `@onScroll` | 元素滚动 | `x`, `y` |

### 媒体事件

| 事件 | 说明 |
|------|------|
| `@onLoad` | 资源加载完成 |
| `@onError` | 资源加载失败 |
| `@onPlay` | 媒体开始播放 |
| `@onPause` | 媒体暂停 |
| `@onEnded` | 媒体播放结束 |

### 焦点/表单事件

| 事件 | 说明 | 事件对象字段 |
|------|------|--------------|
| `@onFocus` | 元素获得焦点 | `value` |
| `@onBlur` | 元素失去焦点 | `value` |
| `@onSubmit` | 表单提交 | `values` |
| `@onToggle` | details/summary 切换 | `checked` |
| `@onSelect` | 文本选择 | `start`, `end`, `text` |

### 动画/过渡事件

| 事件 | 说明 | 事件对象字段 |
|------|------|--------------|
| `@onAnimationStart` | 动画开始 | `animationName`, `elapsedTime` |
| `@onAnimationEnd` | 动画结束 | `animationName`, `elapsedTime` |
| `@onAnimationIteration` | 动画迭代 | `animationName`, `elapsedTime` |
| `@onTransitionEnd` | CSS 过渡结束 | `propertyName`, `elapsedTime` |

### 窗口事件

| 事件 | 说明 | 事件对象字段 |
|------|------|--------------|
| `@onResize` | 窗口大小变化 | `width`, `height` |

## 综合示例

```omni
component EventsDemo {
    signal eventLog: Array<String> = [];
    signal dragStatus: String = "idle";
    signal scrollY: i32 = 0;
    signal deltaY: f64 = 0.0;

    fn addLog(msg: String) {
        eventLog = [...eventLog, msg];
    }

    fn handleScroll(e) {
        scrollY = e.y;
    }

    fn handleWheel(e) {
        deltaY = e.deltaY;
    }

    view {
        <div>
            <!-- 点击事件 -->
            <button @onPress={addLog("clicked")}>Click Me</button>

            <!-- 双击 + 右键 -->
            <div @onDoubleClick={addLog("double-click")}
                 @onContextMenu={addLog("context-menu")}>
                双击或右键点击
            </div>

            <!-- 拖拽 -->
            <div draggable="true"
                 @onDragStart={dragStatus = "dragging"}
                 @onDragEnd={dragStatus = "ended"}
                 @onDrop={dragStatus = "dropped"}>
                拖拽状态: {dragStatus}
            </div>

            <!-- 键盘 -->
            <input @onKeyUp={addLog("key: " + e.key)}
                   placeholder="按任意键..." />

            <!-- 滚轮 -->
            <div @onWheel={handleWheel} style: { height: "100px", overflow: "auto" }>
                滚轮 deltaY: {deltaY}
            </div>

            <!-- 滚动 -->
            <div @onScroll={handleScroll} style: { height: "80px", overflow: "auto" }>
                Scroll Y: {scrollY}
            </div>

            <!-- 表单 -->
            <input @onInput={addLog("input: " + e.value)} />
            <input type="checkbox" @onChange={addLog("checked: " + e.checked)} />

            <!-- Toggle -->
            <details @onToggle={addLog("toggled")}>
                <summary>展开/折叠</summary>
                <p>内容</p>
            </details>
        </div>
    }
}
```

## emit 关键字——组件间事件

### 声明事件

在子组件中声明事件：

```omni
component EmitChild {
    event onCustomEvent: String;   // 事件声明（携带 String 数据）

    fn trigger() {
        emit onCustomEvent("hello from child");  // emit 关键字
    }

    fn triggerAlt() {
        onCustomEvent("hello again");  // 函数调用形式（等价）
    }

    view {
        <button @onPress={trigger}>Emit Event</button>
    }
}
```

### 接收事件

在父组件中绑定事件处理器：

```omni
component Parent {
    signal lastMessage: String = "";

    fn handleCustomEvent(val: String) {
        lastMessage = val;
    }

    view {
        <div>
            <p>Last message: {lastMessage}</p>
            <EmitChild @onCustomEvent={handleCustomEvent} />
        </div>
    }
}
```

### emit 与函数调用的等价性

```omni
// 以下两种写法完全等价
emit onEvent("data");   // emit 关键字形式
onEvent("data");        // 函数调用形式
```

`emit` 关键字更明确地表达"这是一个事件发射"的意图。两种形式编译为相同的代码。

## 事件修饰键

事件对象支持修饰键检测：

```omni
fn handleClick(e) {
    if e.ctrl {
        // Ctrl+Click
    }
    if e.shift {
        // Shift+Click
    }
    if e.alt {
        // Alt+Click
    }
    if e.meta {
        // Cmd/Ctrl+Click
    }
}
```

## 事件冒泡

- 所有事件默认冒泡
- 编译后使用 `addEventListener` 绑定
- 在 `@for` 循环内，事件使用**委托**模式——在容器上绑定单一监听器，通过 `data-event-*` 属性分派
