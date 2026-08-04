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
`area` `base` `br` `col` `embed` `hr` `img` `input` `link` `meta` `param` `source` `track` `wbr`

## 表达式插值

`{expression}` 在模板中嵌入动态内容，引用的 signal 变化时自动更新：

```omni
<p>Count: {count}</p>
<p>Full: {firstName + " " + lastName}</p>
<p>Status: {isActive ? "On" : "Off"}</p>
<p>User: {user ?? "Guest"}</p>
<p>Items: {items.len()} items</p>
```

也支持模板字符串（反引号，插值符为 `${expr}`——与模板文本的 `{expr}` 不同）：
```omni
<p>`Hello ${name}, count is ${count}`</p>
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
<input value={inputText.to_string()} placeholder={hintText} />
<button disabled={isLoading}>Submit</button>
<a href={dynamicUrl}>Link</a>
```

`value={expr}` 是**普通单向 prop**——显示表达式的计算结果，无任何魔法（值绑定收敛 D2）。双向绑定只有一种形式：`bind:value`（见下文）。注意：`value={sig}` 裸 signal 形式已删除——写它会编译报错「用 bind:value 双向绑定，或普通 value prop 单向」。

### 语义绑定（focus / scroll-x / scroll-y）

语义绑定是编译器识别的元素属性（ADR-053 D2）：把裸 signal 绑定到元素 ref，驱动原生 DOM 副作用。元素必须同时带 `@ref={name}`，否则语义检查报错（E0006）：

| 绑定 | 值语义 | 编译行为 |
|------|--------|----------|
| `focus={sig}` | bool | `el.focus()` / `el.blur()` |
| `scroll-y={sig}` | 数字 | `el.scrollTop = sig` |
| `scroll-x={sig}` | 数字 | `el.scrollLeft = sig` |

```omni
component SearchBox {
    signal isSearchActive: bool = false;
    signal scrollPosition: f64 = 0;

    view {
        <input @ref={searchBox} focus={isSearchActive} />
        <div @ref={scrollPane} scroll-y={scrollPosition}>...</div>
    }
}
```

规则：

- **裸 signal 限定**：值必须是裸 signal 引用（`{sig}`）或限定 store signal（`{Store.sig}`）；其他 `{expr}` 表达式保持为普通属性
- **`@ref` 必需**：缺少 `@ref` 时报错；`focus` 用于不可聚焦标签（如 `div`/`span`）时仅警告，`scroll-x`/`scroll-y` 适用于任意可溢出滚动的元素
- **表单值不用语义绑定**：值双向同步用 `bind:value`（唯一双向形式）；单向显示用普通 `value={expr}` prop

详见 [05-组件](05-components.md) 的 `ref` 章节。

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

> 注：旧破折号形式触发器子属性（`@in-view-once`、`@scroll-start`、`@drag-constraints`、`@drag-elastic` 等）已随 ADR-053 移除；v2（Task 5.1）已恢复 `@scroll`/`@drag-*` 的控制维度为冒号形式子参数（`@scroll start: 0 end: 1000 scrub: true ={motion}`），详见 [11-运动与动画](11-motion-animation.md)。元素级 `@paused` 仍为删除状态（解析期硬诊断）；运行时暂停/恢复请用条件渲染。

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
| `data-testid="value"` | 原生 `data-testid` 属性（旧 `testId:` 简写已删除） |
| `data-*` | 原生 data 属性 |
| `a11y: { label/role/hidden/live/alt }` | 语义可访问性块（**唯一的可访问性表面**：→ aria-* / Flutter Semantics；`alt` 仅对 `<img>` 有意义） |
| `aria-*` | **已删除**——编译报错（硬诊断）。原生 `aria-*` 仅 web 透传、Flutter 丢失可访问名称，请迁移到 `a11y: { ... }` 块 |
| `id="..."` | `id` 原生属性 |
| `href="..."` | `href` 原生属性 |
| `title="..."` | `title` 原生属性 |
| `type="..."` | HTML `type` 属性（input/button） |
| `placeholder="..."` | 占位文本 |
| `src="..."` | `<img>` / `<video>` 等资源路径 |
| `alt="..."` | 普通 DOM 属性（**非** a11y 表面：不进入 `a11y:` 语义层；web 透传，Flutter 经 prop 表映射为 `semanticLabel`）。图片替代文本的语义声明请用 `a11y: { alt }` |

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
| Flutter | `AnimatedSwitcher`（默认过渡；各分支带 `ValueKey`，切换时键变化触发过渡，300ms） | `AnimatedSwitcher` + 显式 `FadeTransition`（transitionBuilder 指定，等待旧内容过渡完成后插入新内容） | 立即切换 |
| OmniView | 无动画，立即切换 | 无离开动画机制可等待，降级为立即切换（生成 JS 中注明降级；旧版 `_leave` 查找无任何注册，为死代码，已移除） | 立即切换 |

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
| 双向绑定（唯一） | `bind:prop={s}` | `bind:value={name}` |
| 单向值 prop | `value={expr}` | `value={age.to_string()}` |
| 语义绑定 | `focus={s}` / `scroll-x={s}` / `scroll-y={s}` | `focus={isActive}` |
| 事件 | `@onEvent={fn}` | `@onPress={handle}` |
| 动画 | `@trigger={m}` | `@enter={fadeIn}` |
| 引用（简写） | `ref: name` | `ref: myInput` |
| 引用（事件风格） | `@ref={name}` | `@ref={myInput}` |
| 测试 ID | `data-testid="v"` | `data-testid="btn-save"` |
| 可访问性 | `a11y: {label: "v"}` | `a11y: { label: "Close", role: "button" }` |
