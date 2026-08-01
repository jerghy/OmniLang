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

消费者通过**命名属性**传入插槽内容（ADR-053 D5），匿名内容直接作为子节点：

```omni
<Modal header={<h2>Title</h2>} footer={<button @onPress={confirm}>OK</button>}>
    <p>Body content</p>
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
<input @ref={inputBox} value={inputText} placeholder={hintText} />
<button disabled={isLoading}>Submit</button>
<a href={dynamicUrl}>Link</a>
```

`value={signal}` 是语义绑定（双向值绑定），元素必须同时有 `@ref={name}`（ADR-053 D2）；`bind:value` 双向绑定见下文。

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

<!-- @focus 聚焦 -->
<button @focus={focusGlow}>Focus me</button>

<!-- @in-view 滚动入视 -->
<div @in-view={slideIn}>...</div>

<!-- @scroll 滚动驱动 -->
<div @scroll={scrollFade}>...</div>

<!-- @drag-horizontal / @drag-vertical 拖拽触发器 -->
<div @drag-horizontal={dragSlide}>...</div>
<div @drag-vertical={dragSlideY}>...</div>
```

> 注：触发器子属性（`@in-view-once`、`@scroll-start`、`@drag-constraints`、`@drag-elastic` 等）与元素级 `@paused` 已按 semantic-level DSL 规范移除（ADR-053）；运行时暂停/恢复请用条件渲染。

详见 [11-运动与动画](11-motion-animation.md)。

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

**过渡模式**：`mode:` 值（`"fade"` | `"wait"` | `"none"`）在解析期校验，非法值报错。各目标当前支持情况：

| 目标 | `"fade"` | `"wait"` | `"none"` |
|------|----------|----------|----------|
| React | 进入的分支淡入（CSS 淡入，300ms，`.omni-if-fade` 类 + keyframes 生成在组件 `.module.css`；分支包裹在带 `key` 的块级 `div` 中，切换时重新挂载以重放动画） | 无离开动画机制可等待，降级为立即切换（生成代码中注明降级） | 立即切换 |
| Flutter | 无过渡效果 | `AnimatedSwitcher` + `FadeTransition`（等待旧内容过渡完成后插入新内容） | 立即切换 |
| OmniView | 无过渡效果 | 无离开动画机制可等待，降级为立即切换（生成 JS 中注明降级；旧版 `_leave` 查找无任何注册，为死代码，已移除） | 立即切换 |

> **React `"fade"` 布局说明**：分支内容包裹在块级 `<div>` 中（与 Flutter 的 `AnimatedSwitcher` 包装一致）——用于行内上下文时可能改变布局；支持 `prefers-reduced-motion`（生成的 CSS 中禁用动画并直接显示内容）。

```omni
@if isOpen mode: "fade" {
    <div>Content</div>
}
```

`mode: "wait"`（Flutter 目标下）等待旧内容过渡完成后再插入新内容：
```omni
@if showPanel mode: "wait" {
    <div class: panel>
        <p>旧内容过渡完成后新内容才插入</p>
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

组件体的顶部声明插槽（ADR-053 D5，位于 component body，非 view 内）：

```omni
<!-- 组件定义中 -->
@slot              <!-- 匿名插槽 -->
@slot header       <!-- 命名插槽 -->

<!-- 带默认回退内容 -->
@slot header {
    <span>Default Header</span>
}
```

消费者通过命名属性传入（匿名内容直接作为子节点）：
```omni
<Panel header={<div>Custom</div>}>Body content</Panel>
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
| 引用（简写） | `ref: name` | `ref: myInput` |
| 引用（事件风格） | `@ref={name}` | `@ref={myInput}` |
| 测试 ID | `testId="v"` | `testId="btn-save"` |
