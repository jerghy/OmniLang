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

**语义化即切换**：首批 14 个语义事件名是唯一合法拼写——旧 DOM 拼写（`@onClick`/`@onTap`/`@onMouseEnter`/`@onMouseLeave` 等）产生**编译期硬错误 + 迁移提示**（`事件名 "@onClick" 已弃用——使用 "@onPress"`，E0006）。其余 40 个 DOM 事件名维持透传（见「事件类型参考」；semantic-universe 域 3 逐事件标注「语义化/透传」）。

### 防抖 / 节流处理器（std/timing）

batch-4 新增：`debounce` / `throttle`（模块登记见 12-standard-library.md「std/timing」）返回**可调用函数**，表达式形态直接作事件处理器——防抖输入搜索、节流刷新按钮：

```omni
<!-- 防抖：输入停顿 300ms 后触发一次 -->
<input bind:value={searchQuery} @onInput={debounce(doSearch, 300)}></input>

<!-- 节流：窗口内至多触发一次（trailing-only） -->
<button @onPress={throttle(doRefresh, 2000)}>Refresh</button>
```

语义：

- `debounce(fn, ms)`：停顿 ms 后执行一次——连续输入只触发最后一次（典型：搜索建议）。
- `throttle(fn, ms)`：窗口内至多执行一次，且取窗口内**最后一次**调用（trailing-only 默认，P6 拍板）——典型：轮询刷新按钮防连点。
- 需 import：`import { debounce, throttle } from "std/timing";`（完整示例见 16-forms-and-async-data.md「防抖 / 节流」）。
- 约束：ms 正整数毫秒、fn 可调用（违规 = 编译期硬错误 V9：`std/timing "fn" 的 ms 参数须为正整数毫秒；fn 须为可调用函数`——消息内 fn 为实际函数名）。
- 跨目标：JS 族（React / OmniView）任意事件形态完整生效；Flutter 完整拦截仅限 void 回调族（`@onPress` 按钮系），`@onInput` 为透传占位（诚实降级）。

## 语义事件（首批 14 个）

| 语义事件 | 语义 | 事件数据（schema） | 旧名 → 硬错误迁移 |
|---------|------|------------------|------------------|
| `@onPress` | 点按完成（press/tap 别名收编） | `x`, `y`（**元素局部坐标**）, `button`, `pressure`, `tiltX`, `tiltY`, `pointerType`, `pointerId`, `buttons`, `alt`, `ctrl`, `shift`, `meta` | `@onClick` / `@click` / `@onTap` / `@tap` → `@onPress` |
| `@onInput` | 控件值变化（输入中） | `value`, `checked` | 文本输入旧 `@onChange` 用点 → `@onInput`（demo/文档指引；change 与 input 是不同事件，均合法，无编译器旧名诊断） |
| `@onChange` | 选值变化（select/checkbox/radio） | `value`, `checked` | — |
| `@onSubmit` | 表单/输入提交 | `values`（`{string: string}`，FormData 收集） | `e.value` 等 DOM 形状字段 → 无效字段硬错误（schema 仅 `values`） |
| `@onKeyDown` | 键按下 | `key`（语义键名枚举，见下）, `code`, `alt`, `ctrl`, `shift`, `meta` | `e.which` 等非语义字段 → 无效字段硬错误 |
| `@onKeyUp` | 键抬起 | 同上 | 同上 |
| `@onPointerDown` / `@onPointerMove` / `@onPointerUp` | 指针原始交互 | 同 `@onPress` | `e.clientX`/`e.clientY`/`e.ctrlKey`/… → 无效字段硬错误 + 迁移提示（`e.x`/`e.y` 元素局部坐标、`e.ctrl`） |
| `@onScroll` | 滚动位置/进度 | `x`（scrollLeft）, `y`（scrollTop）, `progress`（0..1） | `e.deltaY` → 无效字段硬错误（增量语义属 `@onWheel`） |
| `@onFocus` / `@onBlur` | 焦点进出 | `value` | —（名不变，仅 schema 化） |
| `@onEnter` / `@onLeave` | 指针悬停进出（不冒泡） | 同 `@onPress` | `@onMouseEnter` / `@mouseenter` / `@onMouseLeave` / `@mouseleave` → `@onEnter` / `@onLeave` |

> **坐标基准**：`@onPress` / pointer / `@onEnter` / `@onLeave` 的 `x`/`y` = **元素局部坐标**（相对元素左上角，offsetX/offsetY）——非视口坐标（clientX/clientY 已随旧名弃用）；`@onScroll` 的 `x`/`y` = 元素内滚动位置（scrollLeft/scrollTop）。
> **`@press`/`@hover`/`@focus`/`@enter`（不带 `on`）是动画触发器**，不是事件（见 [11-运动与动画](11-motion-animation.md)）；`@drag` / `@dragEnd` 是拖拽意图指令（见「拖拽意图指令」）。

### 语义键名（e.key 枚举）

`@onKeyDown` / `@onKeyUp` 的 `e.key` 比较字面量必须是语义键名——否则 E0006 硬错误 + 最近合法建议：

| 分类 | 键名 |
|------|------|
| 功能键 | `Enter` `Escape` `Backspace` `Delete` `Tab` `CapsLock` `Shift` `Control` `Alt` `Meta` |
| 导航键 | `ArrowUp` `ArrowDown` `ArrowLeft` `ArrowRight` `Home` `End` `PageUp` `PageDown` |
| 空格/字符 | `Space` + 单字母 `A`-`Z` / 单数字 `0`-`9`（大小写敏感） |

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
        x = e.x;             // 元素局部坐标（offsetX）——非视口坐标
        y = e.y;             // 元素局部坐标（offsetY）
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
| `@onPress` | `x`, `y`, `button`, `pressure`, `pointerType`, `pointerId`, `buttons` | 点按（语义事件；`@onClick`/`@onTap` 已弃用） |
| `@onDoubleClick` | `x`, `y` | 双击 |
| `@onContextMenu` | `x`, `y` | 右键菜单 |
| `@onEnter` / `@onLeave` | `x`, `y`（元素局部坐标） | 悬停进出（语义事件；`@onMouseEnter`/`@onMouseLeave` 已弃用） |
| `@onPointerDown` / `@onPointerMove` / `@onPointerUp` | `x`, `y`, `button`, `pressure`, `pointerId`, `buttons` | 指针原始交互（语义事件） |
| `@onMouseMove` | `x`, `y` | 鼠标移动（透传） |

### 输入/键盘

| 事件 | 典型字段 |
|------|---------|
| `@onInput` | `value`, `checked` |
| `@onChange` | `value`, `checked` |
| `@onKeyUp` / `@onKeyDown` | `key`（语义键名枚举）, `code` |
| `@onFocus` / `@onBlur` | `value` |
| `@onSubmit` | `values`（FormData 收集） |

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
| `@onScroll` | `x`, `y`, `progress`（0..1：scrollTop/(scrollHeight−clientHeight)，不可测 0 兜底） |

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
| 标准 | `@onPress={handler}` | 语义事件名（`@onEventName`，推荐） |
| 无 @ 前缀 | `onPress={handler}` | `onEventName` 形式同样被解析 |
| 内联闭包 | `@onPress={() => { ... }}` | 直接写逻辑 |
| Store action | `@onPress={TodoStore.add("x")}` | 调用 store 方法 |
| 函数调用 | `@onPress={switchTab("home")}` | 带参数调用组件函数 |

> 注：`@press`、`@focus` 等（不带 `on`）是**动画触发器**，不是事件——
> 事件绑定一律使用 `@on*` 形式（详见 [11-运动与动画](11-motion-animation.md)）；
> `@drag` / `@dragEnd` 是**拖拽意图指令**（见上「拖拽意图指令」），亦非事件。
