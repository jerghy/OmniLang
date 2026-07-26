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

## 元素属性

### class 类名绑定

```omni
<!-- 静态类名 -->
<div class: container>...</div>

<!-- 设计系统类名 -->
<div class: LearnDesign.Card>...</div>

<!-- 条件类名（对象映射） -->
<div class: { active: isActive, disabled: isLocked }>...</div>

<!-- Tailwind 等字符串类名 -->
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
<div @enter={fadeInUp} @leave={fadeOut}>...</div>
<button @press={scalePress} @hover={pulse}>Click</button>
<div @inView={slideIn} @scroll={scrollFade}>...</div>
<div @dragHorizontal={dragSlide}>...</div>
```

详见 [11-运动与动画](11-motion-animation.md)。

### states: 多状态动画

```omni
<div states: {
    idle:    { scale: 1, opacity: 1 }   over: 150,
    hover:   { scale: 1.05 }            over: 150,
    pressed: { scale: 0.95 }            over: 100
}>
    Interactive
</div>
```

### ref: DOM 引用

```omni
<input ref: myInput bind:value={text} />
```

### 特殊属性

| 属性 | 编译为 |
|------|--------|
| `testId: "value"` | `data-testid="value"` |
| `data-*` | 原生 data 属性 |
| `aria-*` | 原生 aria 属性 |

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

## 组件调用语法

两种等价风格：

**HTML 风格**（简洁调用）：
```omni
<Counter />
<Button label="Click" />
```

**组件风格**（带子内容时推荐）：
```omni
<Button(label: "Submit", disabled: {isLoading})>
    <span>Submit Form</span>
</Button>
```

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
| 引用 | `ref: name` | `ref: myInput` |
