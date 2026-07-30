# 06 — 模板语法（View Templates）

`view { ... }` 块定义组件的 UI 结构。支持 HTML 元素、组件标签、指令和表达式插值。

## HTML 元素

```omni
view {
    <div>
        <h1>标题</h1>
        <p>段落文本</p>
        <br />
        <img src="logo.png" alt="Logo" />
        <input type="text" placeholder="输入..." />
    </div>
}
```

所有标准 HTML5 元素均支持。以下空元素自动自闭和：
`area` `base` `br` `col` `embed` `hr` `img` `input` `link` `meta` `source` `track` `wbr`

## 表达式插值

`{expression}` 在模板中嵌入动态内容，引用的 signal 变化时自动更新：

```omni
<p>Count: {count}</p>
<p>Full: {firstName + " " + lastName}</p>
<p>Status: {isActive ? "On" : "Off"}</p>
<p>User: {user ?? "Guest"}</p>
<p>Items: {items.len()} items</p>
```

也支持模板字符串（反引号）：
```omni
<p>`Hello {name}, count is {count}`</p>
```

## 组件标签

PascalCase 标签引用其他组件——编译时视图会被内联展开：

```omni
<Header />
<Counter initial={10} />
<TodoList items={todoItems} />
```

### 传递 Props

```omni
<!-- 静态 -->
<Button label="Click" />

<!-- 动态（signal 表达式）-->
<Button label={buttonText} disabled={isLoading} />

<!-- 多个 -->
<UserCard name={user.name} role="admin" />
```

### 传递插槽内容

```omni
<Modal>
    @slot header { <h2>Title</h2> }
    @slot { <p>Body content</p> }
    @slot footer { <button @onPress={confirm}>OK</button> }
</Modal>
```

### 事件监听（@onEvent 语法）

```omni
<Counter @onCountChange={handleCount} />
<Child @onAction={(val) => { doSomething(val); }} />
```

## 元素属性

### class 类名绑定

```omni
<!-- 静态类名 -->
<div class: container>...</div>

<!-- 设计系统类名 -->
<div class: LearnDesign.Card>...</div>

<!-- 条件类名（对象映射） -->
<div class: { active: isActive, disabled: isLocked }>...</div>

<!-- 字符串字面量类名 -->
<div class: "bg-blue-100 border rounded-lg">...</div>
```

### variant 组件变体

```omni
<button variant: LearnDesign.Button(color=primary, size=md)>Submit</button>
<button variant: LearnDesign.Button(color=danger, size=lg)>Delete</button>
```

详见 [10-设计系统](10-design-system.md)。

### style 内联样式

```omni
<div style: {
    display: "flex",
    gap: "12px",
    background: isError ? "#fee2e2" : "#f0fdf4"
}>
```

内联样式也支持 CSS 变量——静态值直接 `setProperty()`，信号值响应式同步：

```omni
<div style: {
    --accent: accentColor,       // 信号 → setProperty 响应式
    --radius: "8px"              // 静态 → 直接 setProperty
}>
```

### 动态属性

```omni
<input value={inputText} placeholder={hintText} />
<button disabled={isLoading}>Submit</button>
<a href={dynamicUrl}>Link</a>
```

### 布尔属性

```omni
<input type="checkbox" checked={isChecked} />
<button disabled={isSubmitting}>Save</button>
<details open={isOpen}>...</details>
<input readonly={isReadonly} required={isRequired} />
```

### bind: 双向绑定

```omni
<input bind:value={name} />
<input type="checkbox" bind:checked={agreed} />
<select bind:value={selected}>
    <option value="a">A</option>
    <option value="b">B</option>
</select>
```

详见 [07-事件与绑定](07-events-and-bindings.md)。

### 动画触发器

```omni
<div @enter={fadeInUp}>...</div>
<button @press={scalePress} @hover={pulse}>Click</button>

<!-- @inView + 子属性 -->
<div @inView={slideIn}
     @in-view-once={true}
     @in-view-threshold={0.5}>...</div>

<!-- @scroll + 子属性 -->
<div @scroll={scrollFade}
     @scroll-start="top 80%"
     @scroll-end="bottom 20%"
     @scroll-scrub={0.5}>...</div>

<!-- @dragHorizontal / @dragVertical + 拖拽子属性 -->
<div @dragHorizontal={dragSlide}
     @drag-constraints="left:0 right:300"
     @drag-elastic={0.2}
     @drag-scrub={true}
     @drag-momentum={0.95}>...</div>

<div @dragVertical={dragSlideY}
     @drag-constraints="top:0 bottom:120">...</div>
```

详见 [11-运动与动画](11-motion-animation.md)。

### @paused 运行时控制

```omni
<div @enter={spinLoop} @paused={isPaused}>...</div>
```

signal 为 `true` 时暂停动画，`false` 时恢复。详见 [11-运动与动画](11-motion-animation.md)。

### ref: / @ref={} DOM 引用

两种等价的元素引用绑定语法：

```omni
<!-- 冒号简写 -->
<input ref: myInput bind:value={text} />

<!-- @ref 事件风格（推荐，与 @onClick 等一致性更高）-->
<input @ref={myInput} bind:value={text} />
<div @ref={scrollContainer}>...</div>
```

详见 [05-组件](05-components.md) 的 `ref` 章节。

### 特殊属性

| 属性 | 编译为 |
|------|--------|
| `testId: "value"` 或 `testId="value"` | `data-testid="value"` |
| `data-*` | 原生 data 属性 |
| `aria-*` | 原生 aria 属性 |
| `id="..."` | `id` 原生属性 |
| `href="..."` | `href` 原生属性 |
| `title="..."` | `title` 原生属性 |
| `type="..."` | HTML `type` 属性（input/button） |
| `placeholder="..."` | 占位文本 |
| `src="..."` | `<img>` / `<video>` 等资源路径 |
| `alt="..."` | 图片替代文本 |

## 模板指令

### @if / @else if / @else

```omni
@if status == "loading" {
    <div>Loading...</div>
} @else if status == "error" {
    <div>Error: {errorMsg}</div>
} @else {
    <div>{data}</div>
}
```

**过渡模式**：`mode="sync"` 使入场/离场动画同步：

```omni
@if isOpen mode="sync" {
    <div @enter={fadeIn} @leave={fadeOut}>Content</div>
}
```

`mode="wait"` 等待旧内容的离场动画完成后再插入新内容：
```omni
@if showPanel mode="wait" {
    <div class: panel>
        <p>旧内容的离开动画完成后新内容才插入</p>
    </div>
}
```

### @for / @empty

```omni
@for item in items key={item.id} {
    <div class: list-item>
        <span>{item.name}</span>
    </div>
} @empty {
    <p>No items yet</p>
}
```

- `key={...}` 提供稳定的 DOM 标识，支持增量更新和 FLIP 动画
- `@empty` 在列表为空时显示，可选

### @slot

```omni
<!-- 组件定义中 -->
@slot              <!-- 匿名插槽 -->
@slot header       <!-- 命名插槽 -->

<!-- 带默认回退内容 -->
@slot header {
    <span>Default Header</span>
}

<!-- 消费者使用时 -->
<Panel>
    @slot header { <div>Custom</div> }
</Panel>
```

## emit 关键字

在组件内部触发自定义事件：

```omni
component Child {
    event onAction: String;

    fn trigger() {
        emit onAction("hello from child");
    }
}
```

详见 [07-事件与绑定](07-events-and-bindings.md)。

## 属性速查表

| 类别 | 语法 | 示例 |
|------|------|------|
| 类名 | `class: name` | `class: container` |
| 设计类 | `class: Design.X` | `class: LearnDesign.Card` |
| 条件类 | `class: {k: v}` | `class: {active: flag}` |
| 变体 | `variant: D.V(...)` | `variant: D.Button(color=primary)` |
| 内联样式 | `style: {k: v}` | `style: {color: "red"}` |
| 双向绑定 | `bind:prop={s}` | `bind:value={name}` |
| 事件 | `@onEvent={fn}` | `@onPress={handle}` |
| 动画 | `@trigger={m}` | `@enter={fadeIn}` |
| 动画运行时控制 | `@paused={s}` | `@paused={isPaused}` |
| 引用（简写） | `ref: name` | `ref: myInput` |
| 引用（事件风格） | `@ref={name}` | `@ref={myInput}` |
| 测试 ID | `testId="v"` | `testId="btn-save"` |
