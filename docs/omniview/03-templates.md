# 03 — 模板语法（View Templates）

`view { ... }` 块定义了组件的 UI 结构。模板支持 HTML 元素、组件标签、指令和表达式插值。

## 基本结构

```omni
view {
    <div class: container>
        <h1>Hello World</h1>
        <p>{dynamicContent}</p>
    </div>
}
```

## HTML 元素

OmniView 支持所有标准 HTML 元素，语法与 HTML 一致：

```omni
view {
    <div>
        <!-- 文本内容 -->
        <h1>标题</h1>
        <p>段落文本</p>

        <!-- 自闭和标签 -->
        <br />
        <img src="logo.png" alt="Logo" />
        <input type="text" placeholder="输入..." />

        <!-- 嵌套元素 -->
        <section>
            <article>
                <h2>文章标题</h2>
            </article>
        </section>

        <!-- HTML 注释 -->
        <!-- 这不会出现在渲染输出中 -->
    </div>
}
```

### 空元素（Void Elements）

以下 HTML 空元素会自动自闭和：
`area`, `base`, `br`, `col`, `embed`, `hr`, `img`, `input`, `link`, `meta`, `param`, `source`, `track`, `wbr`

### HTML 语义标签

支持所有 HTML5 语义标签：

| 类别 | 标签 |
|------|------|
| 文本语义 | `b`, `strong`, `i`, `em`, `mark`, `small`, `del`, `ins`, `code`, `kbd`, `samp`, `var`, `sup`, `sub`, `abbr`, `dfn`, `q`, `blockquote`, `cite`, `time`, `address`, `bdi`, `bdo`, `wbr` |
| 表单 | `form`, `input`, `select`, `option`, `optgroup`, `textarea`, `button`, `label`, `fieldset`, `legend`, `datalist`, `output`, `meter`, `progress` |
| 媒体 | `img`, `video`, `audio`, `source`, `canvas`, `picture`, `figure`, `figcaption`, `map`, `area`, `track` |
| 结构 | `header`, `main`, `footer`, `nav`, `section`, `article`, `aside`, `details`, `summary`, `dialog` |
| 列表 | `ul`, `ol`, `li`, `dl`, `dt`, `dd` |
| 表格 | `table`, `thead`, `tbody`, `tfoot`, `tr`, `th`, `td`, `caption`, `colgroup`, `col` |
| 其他 | `div`, `span`, `a`, `p`, `pre`, `hr`, `br`, `ruby`, `rt`, `rp`, `template`, `slot` |

## 组件标签

PascalCase 命名的标签表示**组件引用**——编译时会将子组件的视图内联：

```omni
view {
    <div>
        <Header />
        <Counter initial={10} />
        <TodoList items={todoItems} />
    </div>
}
```

### 向组件传递 Props

```omni
// 静态值
<Button label="Click" />

// 动态值（signal 表达式）
<Button label={buttonText} disabled={isLoading} />

// 多个 props
<UserCard name={user.name} avatar={user.avatarUrl} role="admin" />
```

### 向组件传递插槽内容

```omni
<Modal>
    @slot header {
        <h2>Confirm</h2>
    }
    @slot {
        <p>Are you sure?</p>
    }
    @slot footer {
        <button @onPress={confirm}>Yes</button>
    }
</Modal>
```

## 表达式插值

使用 `{expression}` 在模板中嵌入动态内容：

```omni
<p>Count: {count}</p>
<p>Full name: {firstName + " " + lastName}</p>
<p>Status: {isActive ? "Active" : "Inactive"}</p>
<p>User: {user ?? "Guest"}</p>
<p>Price: ${price}</p>
<p>Total: {items.len()} items</p>
```

插值自动响应——当引用的 signal 值改变时，对应的文本内容自动更新。

## 元素属性

### 静态属性

```omni
<input type="text" placeholder="输入..." />
<img src="logo.png" alt="Logo" width="200" height="100" />
<a href="/about" target="_blank">About</a>
```

### 动态属性

使用 `{expression}` 设置动态属性值：

```omni
<input value={inputText} placeholder={hintText} />
<button disabled={isLoading}>Submit</button>
<a href={dynamicUrl}>Link</a>
```

### 布尔属性

```omni
<input type="checkbox" checked={isChecked} />
<button disabled={isSubmitting}>Save</button>
<details open={isOpen}>
    <summary>展开</summary>
    <p>内容</p>
</details>
```

### class 类名绑定

```omni
<!-- 静态类名 -->
<div class: container>...</div>

<!-- 设计系统类名 -->
<div class: LearnDesign.Card>...</div>

<!-- 条件类名（对象映射） -->
<div class: { active: isActive, disabled: isLocked }>
    ...
</div>

<!-- 混合使用 -->
<button class: { primary: true, large: isWide } @onPress={handleClick}>
    Click
</button>

<!-- 字符串字面量 class（用于语义CSS类名） -->
<div class: "bg-blue-100 border border-blue-300 rounded-lg p-3">
    Tailwind-style utility classes
</div>
```

`class:"string"` 语法适用于引用全局 CSS 框架（如 Tailwind）或不属于 OmniView style 块的语义类名。

### variant 组件变体

引用设计系统的 variant 变体：

```omni
<button variant: LearnDesign.Button(color=primary, size=md)>
    Submit
</button>

<button variant: LearnDesign.Button(color=danger, size=lg)>
    Delete
</button>
```

详见 [08-设计系统](08-design-system.md)。

### style 内联样式

```omni
<div style: { padding: "16px", background: "#f0f0f0", borderRadius: "8px" }>
    Styled div
</div>
```

### bind: 双向绑定

```omni
<input bind:value={name} />
<input type="checkbox" bind:checked={agreed} />
<select bind:value={selectedOption}>
    <option value="a">A</option>
    <option value="b">B</option>
</select>
```

详见 [04-响应式系统](04-reactivity.md)。

### 事件属性

```omni
<button @onPress={handleClick}>Click</button>
<input @onInput={handleInput} @onKeyUp={handleKey} />
```

事件处理器可以带或不带 `@` 前缀，详见 [06-事件系统](06-events.md)。

### 其他特殊属性

| 属性 | 用途 |
|------|------|
| `ref: name` | DOM 元素引用 |
| `testId: "value"` | 测试标识符（→ `data-testid`） |
| `states: { stateName: { prop }, ... }` | 多状态动画 (详见 [09-运动与动画](09-motion.md)) |
| `data-*` | 自定义 data 属性（如 `data-testid`, `data-type`） |
| `aria-*` | 无障碍属性（如 `aria-label`, `role`） |

### `states:` 多状态动画

直接在元素上定义声明式状态机，自动绑定鼠标事件：

```omni
<div states: {
    idle:    { scale: 1, opacity: 1 }   over: 150,
    hover:   { scale: 1.05, opacity: 1 } over: 150,
    pressed: { scale: 0.95, opacity: 0.8 } over: 100
}>
    Interactive Element
</div>
```

保留状态名 `idle`/`hover`/`pressed` 自动绑定 mouseenter/mousedown/mouseup 事件。自定义状态名（如 `loading`/`success`/`error`）由 signal 驱动。

## 模板指令

### @if / @else if / @else（条件渲染）

```omni
@if status == "loading" {
    <div class: spinner>Loading...</div>
} @else if status == "error" {
    <div class: error>Error: {errorMsg}</div>
} @else {
    <div class: content>{data}</div>
}
```

支持的组合模式：
- `@if ... { }`（单独 if）
- `@if ... { } @else { }`（if-else）
- `@if ... { } @else if ... { }`（if-else if）
- `@if ... { } @else if ... { } @else { }`（完整链）
- `@if ... { } @else @if ... { }`（`@else @if` 写法等价于 `@else if`）

**条件渲染模式**：

`mode` 属性控制条件切换时的动画行为：

```omni
<!-- mode="sync"：入场和离场动画同步协调 -->
@if isOpen mode="sync" {
    <div @enter={fadeInUp} @leave={fadeInUp}>
        <h2>Modal Content</h2>
    </div>
}
```

- 不指定 `mode`：条件分支通过 `display: block/none` 切换
- `mode="sync"`：离场动画完成后才渲染新分支（适合模态框/弹窗）
- 未来模式：`mode="out"`（仅离场）、`mode="wait"`（等待离场完成）

### @for / @empty（列表渲染）

```omni
<!-- 基本用法 -->
<ul>
    @for item in items {
        <li>{item}</li>
    }
</ul>

<!-- 带 key 的 @for（推荐，用于高效 DOM 更新） -->
@for item in items key={item.id} {
    <div class: list-item>
        <span>{item.name}</span>
    </div>
} @empty {
    <div class: empty-state>
        <p>No items yet</p>
    </div>
}
```

`@empty` 块在列表为空时显示，是可选的。

`key` 表达式用于稳定的 DOM 标识——编译后在列表更新时基于 key 做增量 DOM 操作，支持入场/离场动画。

### @slot（插槽）

```omni
<!-- 匿名插槽（默认插槽） -->
@slot

<!-- 命名插槽 -->
@slot header
@slot footer

<!-- 带默认回退内容的插槽 -->
@slot header {
    <span>Default Header</span>
}

@slot {
    <p>Default body content</p>
}
```

插槽语义：
- 消费者未提供内容 → 显示默认回退内容
- 消费者提供了内容 → 默认回退被完全替换
- 不带默认值的 `@slot name` → 无内容时渲染空白

父组件使用插槽：

```omni
<SlotPanel>
    @slot header {
        <div>Custom Header</div>
    }
    <!-- 不提供 footer → 使用默认或空白 -->
</SlotPanel>
```

## 元素属性速查

| 属性类别 | 语法 | 示例 |
|----------|------|------|
| 静态属性 | `name="value"` | `type="text"` |
| 动态属性 | `name={expr}` | `value={text}` |
| 布尔属性 | `name={bool}` | `disabled={flag}` |
| 类名 | `class: name` | `class: container` |
| 设计类 | `class: Design.Class` | `class: LearnDesign.Card` |
| 条件类 | `class: {k: expr}` | `class: {a: b, c: d}` |
| 变体 | `variant: D.V(...)` | `variant: D.Button(color=primary)` |
| 内联样式 | `style: {k: v}` | `style: {color: "red"}` |
| 双向绑定 | `bind:prop={s}` | `bind:value={name}` |
| 事件 | `@onEvent={fn}` | `@onPress={handle}` |
| 动画触发器 | `@trigger={motion}` | `@enter={fadeIn}` |
| 引用 | `ref: name` | `ref: myInput` |

## 组件调用语法

PascalCase 标签调用子组件支持两种语法风格：

**HTML 风格**（推荐用于简单调用）：

```omni
<Counter />
<Button label="Click" />
```

**组件调用风格**（推荐用于带子内容的调用）：

```omni
<Button(label: "Submit", disabled: {isLoading})>
    <span>Submit Form</span>
</Button>
```
