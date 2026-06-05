# OmniView Phase 0 — 冻结任务书

**基于：** OmniLang_v0.3_设计方案.md v3.0-final
**IR 版本：** 0.3
**冻结日期：** 2026-06-05

> 本文档从 OmniLang_v0.3_设计方案.md v3.0-final 提取，Phase 0 期间冻结不可修改。任何调整通过 IRVersion 次版本号迭代。
>
> 这是一个完整的、自包含的 Phase 0 实施任务书。Phase 0 工程师应仅以此文档为工作依据，无需阅读完整设计文档。

---

## 目录

1. [设计哲学](#一设计哲学)
2. [OmniView 组件模型](#二omniview-组件模型)
3. [抽象元素系统](#三抽象元素系统)
4. [@指令系统](#四指令系统)
5. [响应式状态系统](#五响应式状态系统)
6. [Store 状态管理](#六store-状态管理)
7. [语义样式系统](#七语义样式系统)
8. [跨框架编译策略](#八跨框架编译策略)
9. [IR 规范与跨框架预埋（Phase 0 子集）](#十ir-规范与跨框架预埋phase-0-子集)
10. [转译约束与逃逸机制](#十三转译约束与逃逸机制)
11. [Phase 0 精确范围与任务清单](#十四phase-0-精确范围与任务清单)
12. [TodoMVC 完整示例](#十六todomvc-完整示例)
13. [测试策略](#十七测试策略)
14. [调试与开发体验](#十八调试与开发体验)
15. [已知未决事项（P0 阻断）](#二十已知未决事项v30-final)

---

## 一、设计哲学

### 为什么不使用 JSX

| JSX 问题 | 根因 | OmniView 解法 |
|----------|------|--------------|
| 表达式→指令逆编译 | `{cond && <A/>}` 是短路求值，`{items.map(...)}` 是数组方法，Vue Template 不支持 | `@if`/`@for` 指令：1:1 映射到 v-if/v-for/.map() |
| HTML 元素耦合 | `<div>`/`<span>` 绑定 Web 语义 | 55+ 抽象元素，每个框架精确定义渲染方式 |
| 样式绑定 | className/style={{}} 是 Web 专属 | 语义样式属性，编译器按框架翻译 |
| 事件模型 | onClick/onChange 是 DOM 事件名 | 语义事件名，编译器按框架翻译 |

### 三条原则

1. **语义优先**：描述 UI 意图而非实现。`<Button onPress={fn}>` 而非 `<button onClick={fn}>`
2. **受限即自由**：不提供任意 JS 表达式。在受限空间内做精确跨框架映射
3. **渐进式可逃逸**：90% OmniView 抽象 + 10% `@native(target)` 框架原生代码

### 设计张力：视图受限 vs 逻辑表现力

OmniView 的两层设计——视图层受限（`@if`/`@for` 禁用任意表达式）、逻辑层图灵完备（`fn`/`struct`/`match`）——形成了"上松下紧"的结构。这类似 SQL 与存储过程的关系：`SELECT` 简洁受限，但复杂查询逻辑可以封装在存储过程中调用。

**当视图逻辑超出 `@if`/`@for` 的表达能力时，推荐的最佳实践是将其提取到 `fn` 方法中**：

```
// ❌ 试图在视图内表达复杂分支（被编译器拒绝）
// @if items.filter(...).sort_by(...).take(3).len() > 0 { ... }

// ✅ 将复杂逻辑提升到 computed / fn
computed featured_items: Array<Item> = items
    .filter(fn(i) { i.score > threshold })
    .sort_by(fn(a, b) { b.score - a.score })
    .take(3);

fn has_featured() -> bool { featured_items.len() > 0 }

view {
    @if has_featured() {
        @for item in featured_items {
            <ListItem key={item.id}>{item.name}</ListItem>
        }
    }
}
```

`fn` 返回纯数据结构（`Array<ItemSection>`、`Option<ViewConfig>`、`bool`）供 `@指令` 消费。**编译器绝不会在 `@指令` 内部控制流位置增加表达式解析能力——这是必须坚守的长期设计红线。** 我们通过提供更强大的 `computed` 和 `fn`，将复杂性从受限的视图层推向表现力丰富的逻辑层，以此解决张力，而非打破视图层的安全性。

### 安全表达式子集：属性绑定与文本插值

上述设计在**控制流**（`@if`/`@for` 的条件和迭代源）上坚守了红线。但审查指出：当仅涉及**属性值**和**文本插值**时，完全禁止表达式会导致过度碎片化——原本一行内联的条件样式被迫拆为独立的 `computed`，降低内聚性。

因此，OmniView 在 Phase 0 即支持**安全表达式子集**，限定在以下位置：

| 位置 | 允许的表达式 | 示例 | 禁止 |
|------|------------|------|------|
| 属性值 | 三元、比较、逻辑、属性访问、字面量 | `variant={is_primary ? "primary" : "text"}` | 函数调用、方法调用、副作用 |
| 文本插值 | 属性访问、三元、比较 | `{items.len()} items left` | 函数调用（除 `.len()`）、循环 |
| `class_map()` | 布尔表达式键值对 | `class_map({active: is_active, disabled: !enabled})` | 方法调用 |
| `style={{}}` 内联 | 三元、属性访问 | `style={{color: is_error ? "red" : "black"}}` | 复杂计算 |

**安全表达式的约束（编译器强制）：**
- ❌ 无副作用：禁止赋值（`=`）、`++`/`--`、`event` 触发
- ❌ 无函数调用：禁止调用 `fn`、方法、Store action（除 `.len()` 等内置零参数方法）
- ❌ 无复杂逻辑：禁止 `match`、`@if`、`@for`、闭包
- ✅ 允许：三元 `a ? b : c`、比较 `== != < > <= >=`、逻辑 `&& || !`、属性访问 `x.y`、字面量

**编译策略：** 安全表达式直接翻译为目标框架的等价表达式——React 的三元就是 JSX 的三元，Vue 的 `:class` 就是 `v-bind:class`。这不会产生"指令逆编译"问题，因为属性值表达式不改变控制流结构。

**实例对比：**
```
// ❌ 旧方案：属性级简单逻辑也被迫提升
computed btn_variant: String = if is_primary { "primary" } else { "text" };
view { <Button variant={btn_variant}>...</Button> }

// ✅ 新方案：安全表达式可直接内联
view { <Button variant={is_primary ? "primary" : "text"}>...</Button> }
```

这个调整**不改变 @指令的设计红线**——`@if` 的条件仍然只能是单个 signal/computed/fn 调用，`@for` 的迭代源仍然只能是标识符。表达式子集仅解放了属性值和文本插值这两个局部位置。

**事件处理器与安全表达式的区分：** 事件属性（`onPress`, `onChange`, `onSubmit` 等）的值是**事件处理器**，而非安全表达式。它们接受内联闭包 `fn(params) { ... }` 或对组件 `fn` 方法的引用。内联闭包内部是完整的方法体（图灵完备），可以包含状态修改、Store action 调用、条件逻辑等任意代码，**不受安全表达式子集限制**。安全表达式仅适用于以下位置：

| 位置 | 类型 | 约束 |
|------|------|------|
| `prop={value}` | 安全表达式 | 三元/比较/属性访问/字面量，无副作用，无函数调用 |
| `{text_content}` | 安全表达式 | 同上 |
| `onEvent={fn(params) { ... }}` | **事件处理器** | 内部是完整方法体，无表达式限制 |
| `onEvent={method_ref}` | **事件处理器** | 引用组件 `fn` 方法或 Store action |
| `@if condition` | 标识符/fn 调用 | 仅允许 signal/computed 名或返回 bool 的 fn 调用 |
| `@for iterable` | 标识符 | 仅允许 signal/computed/Store getter 名 |

### Vue 编译策略确认

Vue 目标仅使用 `<template>` + 指令模式，不使用 JSX。`@if`→`v-if`、`@for`→`v-for`、`@slot`→`<slot>` 全部精确 1:1 映射，不需要 JSX→Template 逆编译引擎。

### OmniView 定位再澄清：设计系统描述语言，而非通用前端语言

经过多轮审查，OmniView 的定位需要进一步收敛。React、Vue、Flutter、Dioxus 等框架本身在快速演进（RSC、Vapor mode、Impeller），任何抽象层都**会持续追赶框架特性，永远滞后**。试图用一套语言覆盖所有框架的全部应用场景，将陷入"每个目标都能编译，但没有一个目标能用于生产"的困境。

因此，OmniView 的诚实定位是：

> **OmniView 是设计系统描述语言——专注于描述组件库的 API、交互和样式令牌，生成各框架的薄 wrapper。实际应用逻辑（路由、数据获取、复杂状态流）仍由各框架原生编写。**

具体而言，OmniView 的**最佳适用范围**：

| 场景 | 适合 OmniView？ | 原因 |
|------|----------------|------|
| 跨平台设计系统 / 组件库 | ✅ **核心场景** | 按钮、输入框、卡片等基础组件的 API 在各框架中高度一致 |
| 设计令牌（颜色/间距/字体） | ✅ **核心场景** | 语义样式天然跨框架 |
| 简单表单 / 列表页面 | ✅ 适合 | 交互单一，框架差异可控 |
| 复杂交互（拖拽/手势/动画） | ❌ 不适合 | 框架差异巨大，抽象泄漏严重 |
| 框架特性深度使用（RSC/Hero/并发） | ❌ 不适合 | 超越抽象能力，应使用 `@native` 或原生编写 |
| 路由 / 数据获取 / SSR | ❌ 永久不在范围内 | 见持久边界声明 |

**这一定位的关键含义：**
1. OmniView 不试图让开发者"忘记底层框架"——相反，它要求开发者**了解每个目标的特性**，并在可移植性和地道性之间做**知情选择**
2. 跨框架编译的承诺从"100% 覆盖"收缩为"最大化可移植部分"
3. `@native` 不是设计缺陷的补丁，而是**一等公民**——它是平台相关逻辑的正式承载者

### 地道转译的哲学基础：从"翻译器"到"优化编译器"

一个常见的批评是：跨框架抽象天然是"最小公分母"——IR 只能表达各框架的共同子集，因此生成的代码不可能是地道的。

这个批评的漏洞在于它假设了 **codegen 只是做简单的 1:1 语法翻译**。真正的可能性在于：**让 codegen 具备"理解目标框架习惯"的能力，在翻译时进行语义等价的模式替换，将高层抽象展开为符合目标框架惯用法的代码。**

**类比：** GCC/LLVM 不会因为 IR 是框架无关的，就生成"所有 CPU 的共同子集"代码。相反，每个后端（x86/ARM/RISC-V）从同一个 IR 生成针对该架构优化的指令序列。OmniView 的 codegen 应该做同样的事——**React/Vue/Dioxus/Flutter 就是我们的"指令集架构"**。

**核心公式：**
```
地道转译 = 语义保持 + 意图识别 + 惯用法展开
```

1. **语义保持**（最低要求）：`<Button onPress={fn}>` 在所有框架中都触发 fn。意图 IR 保证这一点
2. **意图识别**（模式匹配）：`@for item in items { <ListItem key={item.id}> }` 不仅是"循环渲染"，它表达的是"渲染动态列表"。Flutter 代码应该用 `ListView.builder` 而非 `Column(children: [...].map(...))`
3. **惯用法展开**（各框架独立）：同一个 `ListRenderingIntent`，React→`.map()`，Vue→`v-for`，Dioxus→`.iter().map()`，Flutter→`ListView.builder`

**这意味着 IR 不应该存储"语法结构"（ForLoop），而应该存储"渲染意图"（ListRenderingIntent { strategy: Dynamic }）。** 详见意图 IR 设计。

---

## 二、OmniView 组件模型

### 2.1 组件定义

```
component ComponentName {
    prop name: Type = default;          // Props（输入）
    signal name: Type = initial;        // 响应式状态
    computed name: Type = expression;   // 计算属性
    event name: PayloadType;            // 必须事件
    event name: PayloadType?;           // 可选事件（? 后缀）
    ref name: Ref<HtmlElement>;         // 跨框架引用（P2+）
    @store StoreName;                   // Store 连接
    
    @effect(signal_name) { ... }       // 副作用
    @mount { ... }                      // 挂载后
    @unmount { ... }                    // 卸载前
    
    fn name(params) -> ReturnType { ... }  // 方法
    view { <Element>...</Element> }         // 视图
    style { .class { property: value; } }  // 样式
}
```

### 2.2 Event 系统

```
event on_search: String;     // 必须传递（默认）
event on_focus: void?;       // 可选传递（? 后缀）

// 触发
event on_search(query);      // 触发必须事件
event on_focus();            // 触发可选事件（未传递时安全空操作）
```

编译规则：必须事件未传递 → 编译错误（P0214）。可选事件未传递 → 各框架通过 `?.()` / 条件检查实现 no-op。

选择 `event` 而非 `emit`：避免与 Node.js `EventEmitter.emit()` 冲突，与 Vue `defineEmits` / Flutter `ValueChanged` 语义一致。

### 2.3 Ref 系统（P2 实现，P0 IR 预埋）

`Ref<T>` 是跨框架泛型类型。`.current` 是统一访问接口：

| 框架 | 展开 | `.current` 类型 |
|------|------|----------------|
| React | `useRef<T>(null)` + `ref={refVar}` | `T \| null` |
| Vue | `ref<T \| null>(null)` + `ref="name"` | `T \| null` |
| Dioxus | `NodeRef<T>` | `T \| None` |
| Flutter | `GlobalKey<State<...>>` | `T?` |

### 2.4 关键字安全

所有新增关键字为**上下文关键字**（仅在 `component`/`store` 内有效）。`@if`/`@for`/`@slot`/`@mount` 等通过 `@` 前缀消除歧义。选择 `event` 而非 `emit` 避免 Node.js 冲突。`omnic check --compat v0.3` 自动检测冲突。

---

## 三、抽象元素系统

OmniView 使用语义抽象元素，不暴露 HTML 标签。每个框架精确定义如何渲染。

### Phase 0 核心元素（20 个）

```
布局：Container, Flex, Grid, Stack, Spacer
内容：Text, Heading, Image, Link, Divider
交互：Button, Input, TextArea, Checkbox, Select
列表：List, ListItem
反馈：Spinner, Progress, Alert
复合：Card, Form
```

### 部分映射示例

| OmniView | React | Vue | Dioxus | Flutter |
|----------|-------|-----|--------|---------|
| `<Container>` | `<div>` | `<div>` | `div {}` | `Container()` |
| `<Flex direction="row">` | flex CSS | flex CSS | flex CSS | `Row()` |
| `<Text>` | `<span>` | `<span>` | `span {}` | `Text()` |
| `<Button onPress={fn}>` | `<button onClick={fn}>` | `<button @click="fn">` | `button { onclick: fn }` | `ElevatedButton(onPressed: fn)` |
| `<Input bind:value={v}>` | value+onChange | `v-model` | oninput | `TextField(controller:)` |
| `<Checkbox bind:checked={c}>` | checked+onChange | `v-model` | checkbox input | `Checkbox(value:)` |
| `<List>` | `<ul>` | `<ul>` | `ul {}` | `ListView()` |

完整的 55+ 元素映射表见附录。

### 元素分级

- **核心原语**：所有目标框架完美映射。Phase 0 的 20 个元素全部属于此级
- **平台增强**：仅部分框架支持，引入时标注 `#[compat(react, vue)]`，文档化限制。`<Form>` 属于此级：仅做语义容器，不提供验证抽象。Flutter 的 `Form` widget 强依赖 `TextFormField` 等组件，当前映射不包含验证逻辑；跨框架表单验证延至 P3 `<FormField>` 复合元素

### 语义等价 ≠ 行为等价

抽象元素映射表描述了各框架的**渲染目标**，但这不意味着同一元素在所有框架上**行为完全一致**。以下关键差异需要开发者知情：

| 元素 | 语义等价（保证） | 行为差异（不保证） |
|------|----------------|-------------------|
| `<Input>` | 文本输入控件 | React 受控组件 vs Vue `v-model` 修饰符 vs Flutter `TextEditingController` 生命周期 vs Dioxus 事件模型。键盘类型、自动聚焦、光标控制、输入法组合等**各框架行为不同** |
| `<List>` | 可滚动列表容器 | Flutter `ListView.builder` 是虚拟化的，Web `<ul>` 不是。同一份 OmniView 代码在 Web 上可能因长列表而性能差 |
| `<Select>` | 下拉选择控件 | Web 原生 `<select>` vs Flutter `DropdownButton`（需要 `DropdownMenuItem` 子组件），交互模式完全不同 |
| `<Modal>` (P3) | 覆盖层弹窗 | 焦点管理、背景锁定、键盘 ESC 关闭、`<body>` 滚动锁定等**深度依赖框架机制**（React Portal、Vue Teleport、Flutter Overlay） |
| `<Image>` | 图片显示 | 加载策略（lazy loading）、占位符、错误回退、缓存行为各框架完全不同 |

**设计原则：**
- **保证语义等价**：`<Input>` 在所有框架上都是"文本输入控件"，事件和值绑定语义一致
- **不保证行为等价**：输入法的光标行为、键盘事件冒泡、焦点切换顺序等由底层框架决定
- **框架特定能力通过 `@native` 访问**：需要 `inputMode="numeric"` 或 `autofocus` 等框架特有属性时，使用 `@native` 块

这一定位诚实承认：OmniView 不是"框架无关的万能 UI 运行时"，而是"描述 UI 意图、由各框架尽力渲染"的编译层。开发者使用每个元素时，需要了解该元素在目标框架上的**行为边界**。

---

## 四、@指令系统

### @if / @else if / @else

```
@if state == "loading" {
    <Spinner />
} @else if state == "error" {
    <Alert variant="error" />
} @else {
    <Content />
}
```

| OmniView | React | Vue | Dioxus | Flutter |
|----------|-------|-----|--------|---------|
| `@if cond { ... }` | `{cond ? (...) : null}` | `v-if="cond"` | `if cond { rsx!(...) }` | `if (cond) ...` |
| `@else { ... }` | `: (...)` | `v-else` | `else { rsx!(...) }` | `else ...` |

**`@if` 分支内 `bind:` 元素的 key 策略：** 当 `@if` 分支中包含 `bind:value`/`bind:checked` 等双向绑定元素时，框架可能需要处理元素重建与状态保持（React reconciliation、Vue `v-if`+`v-model`）。编译器对此类场景发出 **S 级提示**："`@if` 分支内含 `bind:` 元素，建议显式提供 `key` 属性以确保状态正确保持"。开发者应在此类元素上主动添加 `key`。

### @for / @empty

```
@for item in items {
    <ListItem key={item.id}>{item.name}</ListItem>
}
@empty {
    <Text>No items</Text>
}
```

| OmniView | React | Vue | Dioxus | Flutter |
|----------|-------|-----|--------|---------|
| `@for` | `.map()` | `v-for` | `.iter().map()` | `.map().toList()` |

`key` 属性在 `@for` 中是**强制要求**（P0201 规则）。

**循环变量不可变性（编译规则）：** `@for` 循环变量在视图中被视为**不可变**。编译器自动为循环变量创建块级 `const`（React）或按需插入 `.clone()`（Dioxus），确保闭包捕获的值不会因循环复用而错乱。此规则适用于 `@for` 体内所有事件处理器和 `@effect` 引用。

### @slot / @slot:name

```
// 定义侧
<Container class="card">
    @slot                          // 默认插槽
    @slot name="footer"            // 具名插槽
</Container>

// 使用侧
<Card>
    <Text>Default content</Text>
    @slot:footer { <Button>OK</Button> }
</Card>
```

| OmniView | React | Vue | Dioxus | Flutter |
|----------|-------|-----|--------|---------|
| `@slot` | `{children}` | `<slot />` | `{children}` | `child:`/`children:` |
| `@slot name="x"` | `{x}` (named prop) | `<slot name="x">` | `{x}` (named field) | 命名参数 |

### bind:prop

双向绑定语法糖，仅用于跨平台值绑定：

```
<Input bind:value={email} />
// 等价于：<Input value={email} onChange={fn(val) { email = val; }} />
```

**约束（P0214 规则）：** `bind:` 只能绑定组件自身的 `signal` 或 Store 的 `signal`。绑定到 `prop` 或 `prop.field` → 编译错误。

### @native / @fallback

```
@native(react) {
    <Suspense fallback={<Spinner/>}><LazyComponent/></Suspense>
}
@fallback {
    <Container><LazyComponent/></Container>
}
```

@native 块内容**不经过 OmniView 解析**，直接透传到目标输出。P0 无校验，P1 加入词法大括号匹配。@fallback 为未覆盖的目标提供默认实现。

**@native 严格规则（P0 实现，防范"跨框架幻觉"）：**

| 规则 | 严重度 | 说明 |
|------|--------|------|
| `@native(target)` 在非 target 编译时 | **ERROR** | 默认阻止编译。例如 `@native(react) { useEffect(...) }` 编译到 Vue 时必须报错，而非静默跳过 |
| 使用 `#[allow(unportable)]` 抑制 | — | 开发者显式标注后方可降级为 WARNING，表示知情接受不可移植 |
| 跨框架模块中禁止 `@native` | **ERROR** (P2+) | 标记为 `#[portable]` 的模块中，任何 `@native` 块都是编译错误 |
| `@fallback` 为必填 | **WARNING** (P1+) | 有 `@native` 就必须有对应的 `@fallback`，P0 仅提示 |

这确保：使用 `@native(react)` 的组件**从第一天起就不是跨框架组件**——它是 React 组件，用 `.omni` 语法书写。

---

## 五、响应式状态系统

### signal

```
signal count: i32 = 0;
signal items: Array<Todo> = [];

count = 5;                    // 直接赋值
items = [...items, new_item]; // 不可变更新
```

| 框架 | 编译 | 读取 | 写入 |
|------|------|------|------|
| React | `useState(0)` | `count` | `setCount(5)` |
| Vue | `ref(0)` | `count.value` | `count.value = 5` |
| Dioxus | `use_signal(\|\| 0)` | `count()` | `count.set(5)` |
| Flutter | `late T x` | `x` | `setState(() => x = 5)` |

### computed

```
computed full_name: String = `${first} ${last}`;
computed active_count: i32 = todos.filter(fn(t) { !t.done }).len();
```

依赖追踪分三级：
- Layer 1：内联表达式 → 精确收集
- Layer 2：函数调用 → 参数推断
- Layer 3：无法分析 → `#[deps(a, b)]` 显式标注

**computed 链式依赖扁平化：** 当 `computed B` 依赖 `computed A`，而 `computed A` 又依赖底层 `signal` 时，编译器将依赖链**扁平化展开**，最终收集为所有底层 signal 的并集：

```
signal count: i32 = 0;
computed doubled: i32 = count * 2;           // 依赖: [count]
computed quadrupled: i32 = doubled * 2;       // 依赖: [doubled] → 扁平化 → [count]
// 最终 quadrupled 的依赖数组为 [count]，不包含中间 computed
```

此设计确保依赖链再长也不会产生"中间 computed 触发两次重算"的性能问题——每一层 computed 的值缓存由其所在框架的 `useMemo`/`computed` 机制保证，而依赖订阅始终落在最底层 signal 上。

### Signal 行为契约

OmniView 的 `signal` 语法（`count += 1`）在不同框架展开为截然不同的运行时机制（`useState`、`ref`、`use_signal`、`setState`）。为避免开发者对跨框架行为产生错误假设，定义以下**语义契约**——所有目标的 codegen 必须保证这些行为成立：

| 契约 | 保证内容 | 实现策略 |
|------|---------|---------|
| **同步批处理** | 同一个同步块（fn 方法体、事件处理器）内的多次 signal 修改，在代码块结束后**统一反映**到 UI，不产生中间渲染 | React：依赖 React 18+ 自动批处理；Vue：`nextTick` 队列（原生支持）；Dioxus：`use_signal` 自动合并；Flutter：`setState` 单帧合并 |
| **不可变读取** | 在同步块内读取 signal，读到的是**块开始时的值**，不受块内其他修改影响 | 所有目标：codegen 在块开头生成 `const _snapshot = signal` 快照 |
| **异步后同步** | `async action` 中 `await` 之后的代码块，读取 signal 得到的是**最新值**（非快照） | React/Zustand：`set` 的 state 参数是当前值；其他框架：await 后重新读取 signal |
| **computed 惰性求值** | `computed` 仅在依赖 signal 变化且**被读取时**才重新计算 | 所有目标：依赖框架自身机制（`useMemo`/`computed`/`use_memo`） |

**超纲行为标记：** 以下行为**不在 OmniView 契约范围内**，开发者如需使用，应通过 `@native` 显式调用：
- React `useTransition` / `useDeferredValue`（并发渲染）
- Vue `nextTick` 的精确时机控制
- Flutter `WidgetsBinding.instance.addPostFrameCallback`（帧后回调）
- Dioxus `schedule_update_any`（手动调度更新）

**codegen 履约责任：** 各框架的 codegen 负责插入必要逻辑以确保上述契约成立（如 React 的 Class Component 需要手动批处理，但 P0 使用 Function Component + React 18+，批处理自动成立）。若某目标框架无法满足某条契约，必须在文档中标注为 **⚠️ 行为例外**，并提供替代方案。

### 生命周期

```
@mount { ... }                          // 组件挂载
@unmount { ... }                        // 组件卸载
@effect(count) { localStorage.set(...); } // 依赖变化
```

| OmniView | React | Vue | Dioxus | Flutter |
|----------|-------|-----|--------|---------|
| `@mount` | `useEffect(fn, [])` | `onMounted` | mount effect | `initState` |
| `@unmount` | cleanup fn | `onUnmounted` | drop | `dispose` |
| `@effect(x)` | `useEffect(fn, [x])` | `watch(x, fn)` | `use_effect` | `didUpdateWidget` |

---

## 六、Store 状态管理

### 6.1 Store 定义

```
store TodoStore {
    signal todos: Array<Todo> = [];
    signal filter: String = "all";

    getter filtered_todos: Array<Todo> = match filter {
        "all" => todos,
        "active" => todos.filter(fn(t) { !t.done }),
        "done" => todos.filter(fn(t) { t.done }),
    };

    action set_filter(f: String) { filter = f; }
    action add(text: String) {
        if text.trim().len() > 0 {
            todos = [...todos, Todo { id: next_id, text, done: false }];
            next_id += 1;
        }
    }
    async action fetch() { ... }
}
```

只有 `signal`、`getter`、`action` 三种成员。`action` 是唯一修改状态的途径。无 `mutation` 层。

**action 参数语法：**
```
action set_filter(f: String) { filter = f; }        // 带参数的 action
action clear_completed() { todos = ...; }            // 无参 action
action add(text: String) { ... }                     // 带参数的 action
async action fetch(id: i32) -> Data { ... }          // 异步 action（参数 + 返回值）
```

### 6.2 在组件中使用

```
component TodoApp {
    @store TodoStore;
    
    view {
```

**Store action 作为事件处理器：** 当 `onSubmit={store.add}` 将 Store action 直接作为事件处理器传递时，`EventHandlerIR` 的 `kind=STORE_ACTION`、`ref="add"`（不含 `store.` 前缀）。Codegen 根据目标框架展开：

| 目标 | 展开方式 |
|------|---------|
| Zustand | `const add = useStore(s => s.add); ... onSubmit={(text) => add(text)}` 或 `onSubmit={useStore.getState().add}` |
| Pinia | `@submit="store.add"`（Pinia action 可直接作为 `@event` 的值） |
| Dioxus | `let add = store.add(); ... onsubmit: move \|text\| add(text)` |
| Riverpod | `onSubmit: (text) => ref.read(todoStoreProvider.notifier).add(text)` |

组件内 `store.xxx` 的读取分析与 getter 精确订阅使用相同的依赖展开算法（见"精确订阅"）。

```
component TodoApp {
    @store TodoStore;
    
    view {
        // 读取：编译器自动生成精确选择器
        <Text>{store.active_count} items</Text>
        // 写入：直接调用 action
        <Button onPress={fn() { store.add("new todo"); }} />
    }
}
```

**精确订阅：** 组件只读取 `store.todos`，则生成 `useStore(s => s.todos)`。编译器做读取分析，仅订阅用到的字段。

**Getter 访问消解：** 源码 `store.xxx` 在不同目标展开方式不同：
- Zustand → `useStore(s => s.xxx())`（函数调用）
- Pinia → `store.xxx`（computed ref 自动解包）
- 编译器在 codegen 层根据 target 决定是否插入 `()`

**Getter 依赖展开：** `status_text → active_count → todos`，递归追踪到最底层 signal。选择器订阅底层 signal 而非中间 getter。

**async action 内的状态一致性：** `async action` 中的多次赋值（如 `set_loading(true)` → `set_data(...)` → `set_loading(false)`）会被编译为多次状态更新调用。在 Zustand（React 18+）下这些调用会被 React 自动批处理合并为单次渲染，但跨框架不保证原子性。若需原子更新，应在 action 内使用批量更新 API（Zustand 的 `set(state => ...)` 合并写法），编译器在 Dioxus/Flutter 目标下逐框架处理批处理语义。

### 6.3 Store 编译目标

```
omnic build --target react --store-target zustand     # Phase 0（默认）
omnic build --target react --store-target redux       # Phase 3
omnic build --target vue --store-target pinia         # Phase 1
omnic build --target flutter --store-target riverpod  # Phase 4
omnic build --target dioxus --store-target signals    # Phase 2
```

### 6.4 Store 适用范围声明

OmniLang Store 模型（`signal`/`getter`/`action`）是 Zustand、Pinia、Riverpod 的**共同子集**，这意味着它刻意放弃了各库的差异化能力：

| 能力 | Zustand | Pinia | Riverpod | OmniLang Store |
|------|---------|-------|----------|---------------|
| 中间件 / 插件 | ✅ `persist`/`devtools`/`immer` | ✅ `pinia-plugin-persistedstate` | ✅ `ProviderObserver` | ❌ 不映射 |
| 异步缓存 / 去重 | ✅ 手动实现 | ✅ `async setup` | ✅ `AsyncNotifier` + `ref` | ❌ 不映射 |
| Family / 参数化 Provider | — | — | ✅ `family` 修饰符 | ❌ 不映射 |
| 热重载状态保持 | ✅ Vite HMR | ✅ Vite HMR | ✅ Riverpod 内置 | ❌ 依赖框架 |
| 简单全局状态 | ✅ | ✅ | ✅ | ✅ |

**明确定位：OmniLang Store 不是状态管理银弹，而是跨框架共享基础状态的轻量方案。** 它适用于：
- ✅ 全局 UI 状态（当前 filter、主题模式、侧栏展开）
- ✅ 简单 CRUD 数据（todo 列表、表单数据缓存）
- ✅ 跨组件共享的 signal（多个组件需要读写同一状态）

它**不适用**于：
- ❌ 复杂异步流（缓存策略、请求去重、乐观更新）——使用目标框架的状态管理库
- ❌ 需要中间件的场景（持久化、DevTools 集成）——使用 `@native` 直接书写目标 store
- ❌ Riverpod 的 `family` 模式（参数化 provider）——静态 store 模型无法表达

**若开发者的需求超出 OmniLang Store 的范围，应直接使用 `@native` 书写目标框架的 store。**

---

## 七、语义样式系统

### 7.1 语义属性（Phase 0）

```
style {
    .counter {
        layout: column;         // → display: flex; flex-direction: column
        gap: 16px;              // → gap: 16px
        padding: 24px;          // → padding: 24px
        border_radius: 8px;     // → border-radius: 8px
        shadow: sm;             // → box-shadow: var(--shadow-sm)
    }
}
```

语义属性→CSS 映射是编译器内置查找表（~50 条），不需要外部预处理器。

### 7.2 class 边界规则

| 写法 | 校验 | 用途 | 示例 |
|------|------|------|------|
| `style.xxx` | **编译时** | OmniView 定义的类 | `class={style.base}` |
| `class_map({...})` | **编译时** | 条件类名 | `class={class_map({base: true, active: is_active})}` |
| `"字符串"` | **不校验** | Tailwind / 第三方 | `class="flex gap-4 p-4"` |
| 混用 | **S0110 INFO** | 提醒统一风格 | — |

### 7.3 主题

```
theme {
    colors { primary: #1890ff; text_secondary: #8c8c8c; surface: #fff; }
    spacing { sm: 8px; md: 16px; lg: 24px; }
    radius { md: 8px; }
}
```

编译为 `:root { --color-primary: #1890ff; }`（CSS Variables）。组件中 `theme.colors.xxx` → 编译时替换为 `var(--xxx)` 字符串，不做 JS 端运行时解析。

### 7.4 CSS 预处理

Phase 0 仅支持纯 CSS。SCSS/Less/Stylus 延至 Phase 3，通过桥接用户本地的预处理工具链实现。

### 7.5 样式跨框架诚实边界

语义属性（~50 条查找表）只能覆盖最基础的 CSS。现实中的设计稿大量使用渐变、阴影、滤镜、网格、滚动行为、伪元素等，这些**无法通过语义属性表达**。方案允许通过字符串 class 绕过，但这带来了诚实的边界问题：

**字符串 class 的跨框架命运：**

| class 类型 | React/Vue (Web) | Dioxus (Web) | Flutter |
|-----------|----------------|-------------|---------|
| `style.xxx`（OmniView 定义） | CSS Modules 类名 | CSS 类名 | 退化常量 + 语义属性→Widget |
| `class_map({...})` | 条件 CSS Modules | 条件 CSS 类名 | 退化常量 + 语义属性→Widget |
| 字符串（如 Tailwind） | **直接透传** | **直接透传**（同为 Web） | **❌ 死代码**——无法渲染 |
| `style={{}}` 内联 | 内联 style 对象 | 内联 style 字符串 | 语义属性→Widget |

**编译规则（P0 实现）：**

| 规则 | 严重度 | 说明 |
|------|--------|------|
| 字符串 class 编译到非 Web 目标 | **ERROR** | `class="flex gap-4"` 编译到 Flutter/Dioxus(非Web) 时**强制报错**，而非静默丢弃。开发者必须用语义属性或 `#[allow(css_string)]` 抑制 |
| 混用 `style.xxx` + 字符串 | S0110 INFO | 提醒统一风格（已有规则） |

**设计立场：** 不强求在 Flutter/Dioxus 中复现 Web CSS。样式跨框架仅限于**语义属性 + theme 变量**。字符串 class 是 **Web 专属能力**，编译到非 Web 目标时显式报错——这是诚实的设计，而非假装通用。开发者如需在 Flutter 上实现复杂视觉效果，应使用 `@native` 直接书写 Flutter 样式。

---

## 八、跨框架编译策略

### 编译流程：三阶段管道

```
.omni → Parser → AST → Semantic Analysis → 意图 IR → Codegen（三阶段）→ 目标代码
                                                         ├─ 模式匹配
                                                         ├─ 惯用法选择
                                                         └─ 代码展开
```

**关键转变：** Codegen 不是简单的 1:1 模板翻译，而是一个**多目标优化编译器**。每个目标框架的 codegen 从同一个意图 IR 出发，经过模式匹配→惯用法选择→代码展开三个阶段，生成该框架的地道代码。

### 三阶段详解

**阶段 1：模式匹配（Pattern Matching）**

遍历意图 IR 树，识别可优化的高层模式：

| IR 意图 | 识别的模式 | Flutter 惯用法 |
|---------|-----------|---------------|
| `ListRenderingIntent(strategy: Dynamic)` | 动态列表渲染 | `ListView.builder` |
| `ListRenderingIntent(strategy: Static)` | 静态列表渲染 | `Column(children: [...])` |
| `BindIntent(source, TextInput, Bidirectional)` | 文本双向绑定 | `TextEditingController` + 完整生命周期 |
| `ConditionalIntent` 空 else 分支 | 单分支条件 | `SizedBox.shrink()` |
| `StyleIntent(flex: 1)` | 弹性空间分配 | `Expanded(flex: 1)` |
| `SlotIntent(name: "default")` | 默认插槽 | `child:` 参数 |

**阶段 2：惯用法选择（Idiomatic Selection）**

根据目标框架的特性，从候选惯用法中选出最地道的实现：

```
ListRenderingIntent(strategy: Dynamic) →
  React:   items.map(item => <ListItem key={item.id} .../>)
  Vue:     v-for="item in items" :key="item.id"
  Dioxus:  items.iter().map(|item| rsx!(ListItem { key: "{item.id}", ... }))
  Flutter: ListView.builder(itemCount: items.length, itemBuilder: (ctx, i) { ... })
```

同一个 `BindIntent`：
```
React:   <input value={email} onChange={e => setEmail(e.target.value)} />
Vue:     <input v-model="email" />
Dioxus:  input { value: "{email}", oninput: move |e| { email.set(e.value()) } }
Flutter: 生成 StatefulWidget + TextEditingController + initState/didUpdateWidget/dispose
```

**阶段 3：代码展开（Code Expansion）**

将选定的惯用法展开为具体的目标代码。展开由**惯用法规则库**驱动，每个目标框架维护一套规则。

### 地道性层级（Idiomaticity Tiers）

地道性是一个光谱，而非二元状态：

| 层级 | 名称 | 标准 | 示例（Flutter signal 修改） |
|------|------|------|---------------------------|
| **L2** | **地道生成** | 代码看起来像该框架资深开发者写的。利用框架特有惯用法 | `setState(() => _count += 1)` |
| **L1** | **规范生成** | 遵循框架基本模式，可读、可维护，但可能不是最佳实践 | `setState(() { _count = _count + 1; })` — 啰嗦但正确 |
| **L0** | **可运行** | 编译通过、功能正确。不追求代码风格 | 全局变量 + 手动 `build` 调用 — 能跑但不应出现在生产中 |

**目标分配：**
- 核心 20 元素 + signal/computed/@effect/@if/@for/@slot/bind → 全部追求 **L2**
- 平台增强元素（VirtualList、动画）→ 通过框架辅助库提供 **L2**
- 罕见模式无对应惯用法 → 降级到 **L1**（绝不低于 L1）
- **L1 自动化验证：** L1 的 7 条标准将在 Phase 1 转化为自动化 Lint 规则，集成到 `omnic build` 流程中。L1 不通过则构建失败。L1 模板与 L2 惯用法同等维护——任何新的 L2 惯用法必须同时提供 L1 模板并纳入自动化测试。
- L0 仅作为实现过程中的中间状态，不作为交付目标

### 降级链与失败语义

每个意图类型在 codegen 中必须定义一个**完整的降级链**，确保在所有情况下都能生成合法代码：

```
L2 惯用法匹配
  ├─ 匹配成功 → 使用 L2 惯用法模板，生成地道代码
  └─ 匹配失败 ↓
L1 保底模板
  ├─ 模板存在 → 使用 L1 保底模板，生成可读代码（满足 7 条 L1 标准）
  └─ 模板不存在 → **编译错误，禁止静默降级到 L0**
```

**强制规则：**

1. **每个意图类型必须在 IR 定义的同时，提供至少一个 L1 保底模板。** 没有 L1 模板的意图类型不允许进入 IR 定义文件。
2. **如果某个意图类型在特定目标框架上既无法 L2 匹配，也无 L1 模板，则编译必须报错**，错误信息包含：意图类型名称 + 目标框架名称；`.omni` 源码位置（文件:行号）；建议的开发者行动（如：使用 `@native` 重写该部分）。
3. **L1 模板可以是"啰嗦但正确"的实现，但绝不允许生成不可维护的代码。** 若发现无法为某意图类型写出满足 L1 标准的模板，则该意图类型不应作为独立意图——应拆分为更基础的意图组合。

**示例：Flutter 中 `EventIntent(Submit)` 的降级链：**

```
EventIntent { kind: Submit, handler: fn }
  L2 匹配: ❌ 无（Flutter 无原生 onSubmit 事件）
  L1 保底: ✅ 映射为 onFieldSubmitted（语义最接近的 Flutter 事件）
    → 生成: TextField(onFieldSubmitted: (_) => handler())
    → L1 注释: // .omni:15 — onSubmit 在 Flutter 中映射为 onFieldSubmitted
```

**Phase 0.3 交付物要求：** 15 个意图类型定义文件中，必须各自包含至少一个 L1 模板的伪代码注释。Phase 0.4 codegen 实施时，模板不存在的情况作为编译错误处理。

### L1 可读性验收清单

任何降级到 L1 的代码必须满足以下**全部 7 项**标准，方可视为合格：

| # | 标准 | 验证方法 | 反例（不合格） |
|---|------|---------|--------------|
| 1 | **行号可追溯** | 每个逻辑节点（状态声明、事件处理、条件分支、列表渲染）前有 `/* .omni:行号 */` 注释 | 整个函数只有一个行号注释 |
| 2 | **命名无冲突** | 编译器生成的辅助变量使用统一前缀（`_omni_`），不与用户变量名冲突 | 生成 `_temp1`、`_temp2` 无意义命名 |
| 3 | **结构可导航** | 组件/类的 props、state、render 方法清晰分离，缩进与手写代码一致 | 所有逻辑堆在一个函数中，无空行分隔 |
| 4 | **逻辑可理解** | 一个中级开发者（了解目标框架基础）能在 5 分钟内理解生成的代码在做什么 | 大量嵌套三元表达式、无法追踪的数据流 |
| 5 | **注释可优化** | 降级场景的代码包含"优化提示"注释，说明如何改进到 L2 | 没有任何提示，开发者不知如何改进 |
| 6 | **无运行时依赖** | 生成的代码不 import 任何 OmniLang 运行时库（框架辅助库除外） | `import { _omni_render } from 'omnilang/runtime'` |
| 7 | **可单独提取** | 生成的组件文件可以脱离 .omni 源码独立编译通过 | 缺少隐式生成的类型定义导致编译失败 |

**L1 代码示例（Flutter TextField 降级场景）：**

```dart
// .omni:42 — <Input bind:value={email} placeholder="Enter email" />
// L1 降级：未使用 TextEditingController。优化提示见注释末尾。
TextField(
  decoration: InputDecoration(hintText: 'Enter email'),
  onChanged: (value) {
    setState(() {
      email = value;  // .omni:10 — signal email
    });
  },
  // L2 优化：使用 OmniTextField（@omni/flutter_helpers）可自动管理
  // TextEditingController 生命周期，支持光标控制和输入验证。
  // 操作：将 TextField(onChanged:) 替换为 OmniTextField(value:, onChanged:)
  // 文档：https://omnilang.dev/docs/flutter/l1-to-l2#bind-value
)
```

### 优化提示规范

L1 降级代码中的优化提示注释必须遵循统一格式：

```
// L2 优化：[优化类别] — [优化目标]
// 当前：描述当前 L1 代码做了什么
// 改进：描述 L2 代码应该怎么做
// 操作：具体的开发者行动（1-2 个步骤）
// 文档：[文档链接]
```

**优化类别枚举：**

| 类别 | 含义 | 示例场景 |
|------|------|---------|
| `performance` | L2 代码有更好的性能 | `ListView.builder` 替代 `Column + .map()` |
| `lifecycle` | L2 代码有更完整的生命周期管理 | `TextEditingController` 替代 `onChanged` |
| `accessibility` | L2 代码有更好的无障碍支持 | `Semantics` widget 包裹 |
| `style` | L2 代码有更地道的样式写法 | `Expanded` 替代固定宽度 |
| `pattern` | L2 代码使用框架设计模式 | Provider 模式替代 prop drilling |

### 框架辅助库

对于那些无法通过纯 codegen 模式展开的复杂交互（如 Flutter 的双向绑定生命周期），提供**每个框架的薄辅助库**：

```
@omni/flutter_helpers    // OmniTextField, OmniCheckbox 等
@omni/dioxus_helpers     // OmniInput, OmniCheckbox 等
```

这些辅助库是**纯框架原生代码**——手写的最地道实现，封装了跨框架的语义接口。它们的 API 是 OmniView 语义的（`OmniTextField(value, onChanged)`），内部实现是地道的框架代码。这与"不做运行时 OmniLang 库"的声明不矛盾——辅助库不引入任何 OmniLang 运行时依赖，只是预写的框架 native 组件。

### Phase 0 输出（React L2 地道生成）

Phase 0 的目标是让 React 输出达到 **L2 地道生成**：

| OmniView | React L2 输出 |
|----------|--------------|
| `signal count: i32 = 0` | `const [count, setCount] = useState(0)` |
| `count += 1` | `setCount(count + 1)` |
| `computed double: i32 = count * 2` | `const double = useMemo(() => count * 2, [count])` |
| `@effect(count) { ... }` | `useEffect(() => { ... }, [count])` |
| `@if cond { ... }` | `{cond ? (...) : null}` |
| `@for item in items { ... }` | `{items.map(item => (...))}` |
| `bind:value={email}` | `value={email} onChange={e => setEmail(e.target.value)}` |
| `store` | Zustand `create<T>()(set, get => ({...}))` + 精确选择器 |
| `style { .x { layout: column; } }` | CSS Modules `.x { display: flex; flex-direction: column; }` |
| `test-id="btn"` | `data-testid="btn"` |

### 地道性的诚实边界

即使采用了意图 IR + 惯用法展开的架构，以下边界仍然不可逾越：

**1. 框架设计哲学的差异无法完全弥合。** React 的函数式不可变更新（`setState(newState)`）、Vue 的响应式代理（`state.value = newVal`）、Flutter 的命令式 `setState` 在代码结构上有根本差异。Codegen 可以为每个框架生成地道代码，但 **OmniView 的源码本身会偏向一种哲学**——`signal` 的直接赋值（`count += 1`）更接近 Vue/Dioxus 的响应式模型，对 React 的 immutability 和 Flutter 的 `setState` 需要 codegen "反向展开"。这意味着开发者在写 `.omni` 代码时仍需要理解底层框架的差异——"一次编写，无需理解目标"的理想无法完全实现。

**2. 样式系统的地道性最难实现。** CSS 的级联、特异性、伪元素、动画在 Flutter 中没有直接对应。即使 Flutter codegen 将语义样式展开为 Widget 属性，复杂视觉效果（`::before` 伪元素、`@keyframes`、`backdrop-filter`）仍然无法跨框架。**样式的地道性是设计空间中最诚实的不可达目标**——跨框架样式仅限于设计令牌和基础布局属性。

**3. 框架演进会持续引入新惯用法。** React Server Components、Vue Vapor mode、Flutter Impeller 代表了框架的根本范式转变。当地道代码的"标准答案"发生变化时，codegen 需要更新惯用法规则库。这不是设计问题，而是**维护成本问题**——团队需要评估是否在可承受范围内。

**4. 地道性验证的标准难以定义。** "看起来像资深开发者写的"是主观标准。务实做法：Phase 0 React 输出与手写 TodoMVC 做**逐行对比审查**，定义可量化的差异指标（变量命名、hooks 使用、JSX 结构），将"地道"转化为可度量的目标。

---

## 十、IR 规范与跨框架预埋（Phase 0 子集）

### 核心 IR 类型

```
ComponentIR    — 组件完整意图（props, signals, computeds, events, view, style, composables）
StoreIR        — Store 完整意图（signals, getters, actions）
ViewTree       — @if/@for/@slot/@empty + ElementIR 树
StyleIR        — 语义属性键值对（不存 CSS 字符串）
SignalIR       — 响应式状态（含跨框架语义标注）
EventIR        — 事件（含参数类型标注）
TypeIR         — 类型信息（含 Clone/PartialEq 标注）
BlockIR        — 语句块（fn 方法体、@effect/@mount/@unmount 块、内联事件处理器体）。
                 Phase 0 完整定义：`statements: list[StatementIR]` + `execution_mode: str = "client-only"`
                 StatementIR 联合体：`MutationStmt` | `EventEmitStmt` | `ReturnStmt` | `ExprStmt` | `IfStmt` | `LetStmt`
ComposableIR   — 可组合逻辑单元（P3+），含 signals/computeds/effects/return_expr
                 挂载于 ComponentIR.composables: list[ComposableIR]
```

### 意图 IR：从"语法结构"到"渲染意图"

传统跨框架编译器的 IR 存储的是**语法结构的抽象表示**——例如 `ForLoop { iterable, body }`。这导致 codegen 只能做 1:1 翻译，无法利用各框架的惯用法。

OmniView 的 IR 存储的是**渲染意图**——即开发者想要达成的 UI 语义，而非他们使用的语法。Codegen 从意图出发，将其展开为该框架最地道的实现。

**对比示例：**

| OmniView 源码 | 结构 IR（旧思维） | 意图 IR（OmniView） |
|--------------|-----------------|-------------------|
| `@for item in items { <ListItem key={item.id}> }` | `ForLoop { iterable, item_binding, body, key }` | `ListRenderingIntent { strategy: Dynamic, iterable, item_binding, body, key }` |
| `signal count: i32 = 0` | `SignalDecl { name, type, init }` | `StateIntent { name, type, init, mutability: Mutable }` |
| `<Input bind:value={email} />` | `Element(tag=Input, props=[(bind:value, email)])` | `BindIntent { source: email, element: TextInput, direction: Bidirectional }` |
| `@if cond { <A/> } @else { <B/> }` | `IfElse { cond, then_branch, else_branch }` | `ConditionalIntent { branches: [(cond, A), (default, B)] }` |
| `style { .card { layout: column; } }` | `StyleRule { selector, properties }` | `LayoutIntent { direction: Column, element: card }` |

### 15 个核心意图类型（Phase 0 定义）

以下 15 个意图类型在 Phase 0 完成定义。加粗项为 Phase 0 codegen 激活的 8 个核心意图类型，其余在后续 Phase 激活：

```
StateIntent              — ✅ P0 响应式状态声明与修改（→ useState / ref / use_signal / setState）
ComputedIntent           — ✅ P0 派生值计算（→ useMemo / computed / use_memo / 手动缓存）
EffectIntent             — 后续Phase 依赖驱动的副作用（→ useEffect / watch / use_effect / didUpdateWidget）
LifecycleIntent          — 后续Phase 挂载/卸载（→ useEffect([], cleanup) / onMounted-onUnmounted / initState-dispose）

ListRenderingIntent      — ✅ P0 动态/静态列表渲染（→ .map() / v-for / .iter().map() / ListView.builder）
ConditionalIntent        — ✅ P0 条件渲染（→ 三元 / v-if / if-else / if-else widget）
SlotIntent               — 后续Phase 插槽内容投影（→ children / <slot> / children / child:）

BindIntent               — ✅ P0 双向数据绑定（→ value+onChange / v-model / oninput / TextEditingController）
EventIntent              — ✅ P0 事件绑定（→ onClick / @click / onclick / onPressed）
StyleIntent              — ✅ P0 样式意图（→ CSS / scoped / inline / Widget属性）
LayoutIntent             — ✅ P0 布局意图（→ flex CSS / Row-Column Widget）

StoreIntent              — 后续Phase 全局状态管理（→ create / defineStore / GlobalSignal / Riverpod）
ExternIntent             — 后续Phase 外部依赖（→ import / require / extern crate / import package）

FallbackIntent           — 后续Phase 回退/降级（当目标框架无对应惯用法时，降级到的基础实现）
```

### 10.1a 关键意图类型 Schema 定义

以下 5 个意图类型覆盖了最核心的跨框架差异，在 Phase 0.3 必须完成精确字段定义。其余 10 个类型在 P0 期间逐步细化。

```python
@dataclass
class ListRenderingIntent:
    """列表渲染意图——所有 @for 在 Flutter 默认展开为 ListView.builder"""
    iterable: ExprIR                # 迭代源表达式
    item_binding: BindingIR         # 循环变量绑定（名称 + 类型）
    body: ViewTree                  # 单项模板
    key_expr: ExprIR                # key 表达式（Phase 0 @for 强制要求 key）
    empty_body: Optional[ViewTree]  # @empty 分支（无 @empty 时为 None）
    # 无 strategy 字段——默认行为由各框架 codegen 决定：
    #   React:   .map() + key（天然安全，无虚拟 DOM 列表优化差异）
    #   Vue:     v-for + :key（天然安全）
    #   Dioxus:  .iter().map() + key
    #   Flutter: ListView.builder（默认动态，安全——即使静态数据也无性能损失）
    # 开发者如需 Flutter Column 优化，使用 <StaticList> 元素显式声明

@dataclass
class BindIntent:
    """双向数据绑定意图"""
    source: SignalRefIR             # 绑定的 signal 引用（不能是 prop）
    element_type: BindableElement   # 绑定目标的元素类型
    direction: BindDirection        # 当前仅 Bidirectional

class BindableElement(Enum):
    TEXT_INPUT = "text_input"
    TEXT_AREA = "text_area"
    CHECKBOX = "checkbox"
    # SELECT = "select"       # P3

class BindDirection(Enum):
    BIDIRECTIONAL = "bidirectional"

@dataclass
class ConditionalIntent:
    """条件渲染意图"""
    branches: list[ConditionalBranch]   # 至少一个 if 分支（if/else if）
    fallback: Optional[ViewTree]        # 无 @else 时为 None

@dataclass
class ConditionalBranch:
    condition: ExprIR   # 安全表达式（signal 引用 / 比较 / 三元 / bool 字面量）
    body: ViewTree

@dataclass
class StateIntent:
    """响应式状态声明意图"""
    name: str
    type: TypeIR
    initial_value: ExprIR
    mutability: Mutability   # Mutable(signal) | Derived(computed)

class Mutability(Enum):
    MUTABLE = "mutable"           # signal → useState/ref/use_signal/setState
    DERIVED = "derived"           # computed → useMemo/computed/use_memo

@dataclass
class MutationIntent:
    """状态修改意图（count += 1 或 items = [...items, new] 的 IR 表示）"""
    target: SignalRefIR
    operation: MutationOp

class MutationOp(Enum):
    DIRECT_ASSIGN = "direct_assign"        # x = new_value
    SPREAD_APPEND = "spread_append"        # arr = [...arr, item]
    SPREAD_PREPEND = "spread_prepend"      # arr = [item, ...arr]
    INCREMENT = "increment"                # x += n
    DECREMENT = "decrement"                # x -= n
    MAP_UPDATE = "map_update"              # arr = arr.map(fn)
    FILTER_UPDATE = "filter_update"        # arr = arr.filter(fn)
    FIELD_UPDATE = "field_update"          # obj.field = new_val（struct 展开）
```

**嵌套规则：** `ViewTree` 节点可以是 `ElementIR` 或以下意图类型之一：`ConditionalIntent`、`ListRenderingIntent`、`SlotIntent`。意图类型不可自嵌套（`ListRenderingIntent` 的 `body` 不能直接是另一个 `ListRenderingIntent`——应通过嵌套的 `@for` 组件实现）。

**状态修改的 codegen 展开：**

```
MutationIntent { target: count, op: INCREMENT, amount: 1 } →
  React:   setCount(count + 1)
  Vue:     count.value += 1
  Dioxus:  count.set(count() + 1)
  Flutter: setState(() => _count += 1)

MutationIntent { target: todos, op: SPREAD_APPEND, item: new_todo } →
  React:   setTodos([...todos, new_todo])
  Vue:     todos.value = [...todos.value, new_todo]
  Dioxus:  todos.set({ let mut v = todos(); v.push(new_todo); v })
  Flutter: setState(() => _todos = [..._todos, new_todo])
```

### 10.1b 基础 IR 类型（胶水层）

以下类型是意图 IR 的**基础设施**——它们不直接表达"渲染意图"，而是被意图类型引用，构成 IR 的遍历和序列化基础。Phase 0.3 **第一周**必须完成这些定义。

```python
# ── 安全表达式 ──────────────────────────────────

@dataclass
class ExprIR:
    """安全表达式联合体基类"""
    source_span: Optional[SourceSpan] = None  # P0=None, P1+ 填充

@dataclass
class LiteralExpr(ExprIR):
    value: Any   # i32, f64, bool, String, null

@dataclass
class IdentifierExpr(ExprIR):
    name: str    # signal 名 / prop 名 / store 名

@dataclass
class FieldAccessExpr(ExprIR):
    object: ExprIR      # store.filter → IdentifierExpr("store") 然后 FieldAccess("filter")
    field: str

@dataclass
class BinaryOpExpr(ExprIR):
    op: BinaryOp
    left: ExprIR
    right: ExprIR

class BinaryOp(Enum):
    ADD="+"; SUB="-"; MUL="*"; DIV="/"; MOD="%"
    EQ="=="; NEQ="!="; LT="<"; GT=">"; LTE="<="; GTE=">="
    AND="&&"; OR="||"

@dataclass
class UnaryOpExpr(ExprIR):
    op: UnaryOp
    operand: ExprIR

class UnaryOp(Enum):
    NOT="!"; NEG="-"

@dataclass
class TernaryExpr(ExprIR):
    cond: ExprIR
    then_expr: ExprIR
    else_expr: ExprIR

# ── Signal 引用与绑定 ────────────────────────────

@dataclass
class SignalRefIR:
    """Signal 的引用——用于依赖追踪和 codegen setter/getter 生成"""
    owner: Optional[str]   # None=组件自身 signal；"store"=@store 的 Store
    name: str              # signal 名称

@dataclass
class BindingIR:
    """循环变量绑定（@for item in items）"""
    name: str
    type: TypeIR

# ── 组件实例化 ───────────────────────────────────

@dataclass
class ComponentInstanceIR:
    """子组件实例化——<TodoItem todo={todo} onToggle={fn(){...}}/>"""
    component_name: str               # 目标组件名
    props: dict[str, ExprIR]          # prop 名称 → 值表达式
    events: dict[str, EventHandlerIR] # 事件名 → 处理器
    slot_content: dict[str, ViewTree] # 插槽名 → 内容（默认插槽 key="default"）
    source_span: Optional[SourceSpan] = None

@dataclass
class EventHandlerIR:
    """事件处理器的 IR 表示"""
    kind: EventHandlerKind
    body: Optional[BlockIR]   # kind=INLINE_FN 时的方法体
    ref: Optional[str]        # kind=FN_REF 时的函数名；kind=STORE_ACTION 时的 "store.action"

class EventHandlerKind(Enum):
    INLINE_FN = "inline_fn"          # onPress={fn() { ... }}
    FN_REF = "fn_ref"                # onPress={handleSubmit}（引用组件 fn）
    STORE_ACTION = "store_action"    # onPress={store.add}（引用 Store action）

# ── 样式意图 ─────────────────────────────────────

@dataclass
class StyleIntent:
    """元素上的样式绑定意图（class={...} 或 class_map）"""
    target_element: str            # 元素唯一 ID（IR 内部使用）
    classes: list[ClassNameExpr]

@dataclass
class ClassNameExpr:
    kind: ClassNameKind
    value: Any   # STYLE_REF→str(类名); STRING_LITERAL→str; CLASS_MAP→dict[str, ExprIR]

class ClassNameKind(Enum):
    STYLE_REF = "style_ref"            # style.xxx → 编译时校验
    STRING_LITERAL = "string_literal"   # "flex gap-4" → 直接透传（Web）或 ERROR（非Web）
    CLASS_MAP = "class_map"             # class_map({active: is_active})

@dataclass
class LayoutIntent:
    """语义布局属性意图"""
    target_element: str
    properties: dict[str, ExprIR]  # gap→16px, direction→row, align→center 等

# ── 源码位置（远期 LSP） ─────────────────────────

@dataclass
class SourceSpan:
    """IR 节点的源码位置——P0 设为 None，P1+ 填充用于 LSP 和诊断"""
    file: str
    start_line: int
    start_col: int
    end_line: int
    end_col: int
```

**`StyleRuleIR` 与 `StyleIntent` 的关系：** `StyleRuleIR` 负责 `style { .card { ... } }` 的**定义块**（语义属性→CSS 查找表），`StyleIntent` 负责元素上的 `class={style.card}` 的**使用点**。两者通过 `ClassNameExpr` 中引用的类名字符串连接——codegen 在生成 CSS 文件时消费 `StyleRuleIR`，在生成 JSX/Widget 属性时消费 `StyleIntent.classes`。

### 10.1c ElementIR、StyleRuleIR、TypeIR — IR 数量最多的节点类型

```python
@dataclass
class ElementIR:
    """抽象元素节点——视图树的叶子节点，IR 中数量最多的类型"""
    tag: str                          # "Container" | "Flex" | "Button" | "Input" | ...
    props: dict[str, ExprIR]          # 静态属性（variant="primary", placeholder="..." 等）
    bindings: dict[str, SignalRefIR]  # bind: 绑定（bind:value → SignalRefIR(name="email")）
    events: dict[str, EventHandlerIR] # 事件处理器（onPress→EventHandlerIR, onChange→...）
    style_classes: Optional[StyleIntent]   # class={style.xxx} 或 class_map({...})
    inline_style: Optional[LayoutIntent]   # style={{flex: 1, color: ...}} 内联布局
    children: list[ViewTree]          # 子元素 / 插槽内容
    test_id: Optional[str]            # test-id 属性值
    # ── 预埋字段（Phase 0 默认值） ──
    construction_kind: str = "tag-mapping"       # P4 Flutter 构造器选择
    accessibility_label: Optional[ExprIR] = None  # P1+ 无障碍标签
    accessibility_role: Optional[str] = None      # P1+ 无障碍角色
    key_expr: Optional[ExprIR] = None            # @for 中的 key={...}（从 ListRenderingIntent 下沉）
    keep_alive: bool = False                      # P3 KeepAlive
    portal_target: Optional[str] = None            # P3 Portal/Teleport
    source_span: Optional[SourceSpan] = None

@dataclass
class StyleRuleIR:
    """style { .className { ... } } 定义块中的单条规则"""
    class_name: str                    # "card", "todo-app"
    properties: dict[str, ExprIR]     # layout→column, gap→16px, padding→24px ...
    pseudo_states: Optional[list[str]] = None   # P3: [":hover", ":focus"]
    media_query: Optional[str] = None           # P3: "@media (prefers-color-scheme: dark)"
    animation_def: Optional[AnimationIR] = None  # P3

@dataclass
class TypeIR:
    """OmniView 类型表示——驱动依赖追踪的 Copy/Clone 判断和 codegen 类型生成"""
    kind: TypeKind
    type_args: list['TypeIR'] = field(default_factory=list)  # Array<i32> → [TypeIR(kind=I32)]
    is_nullable: bool = False   # T? 语法糖
    needs_clone: bool = False   # Dioxus Clone derive
    needs_partial_eq: bool = False  # key 比较用

class TypeKind(Enum):
    VOID = "void"
    I32 = "i32"
    F64 = "f64"
    BOOL = "bool"
    STRING = "String"
    ARRAY = "Array"
    MAP = "Map"
    STRUCT = "struct"       # 用户定义的 struct 类型
    COMPONENT = "component" # 组件类型（作为 prop 类型时）
    FUNCTION = "function"   # fn(T) -> U
```

**Copy 类型判断：** `i32`/`f64`/`bool` → Copy，直接读取赋值。`String`/`Array`/`Map`/`struct` → 非 Copy，Dioxus codegen 生成 `.clone()`，React codegen 生成不可变更新模式。`TypeIR` 的 `kind` 字段驱动此判断。

### 10.1d StatementIR 联合体 — 方法体语句的完整定义

`BlockIR.statements: list[StatementIR]` 的 `StatementIR` 联合体需要以下类型才完整：

```python
@dataclass
class IfStmt(StatementIR):
    """方法体内的 if/else 语句——与视图层 ConditionalIntent 语义不同"""
    condition: ExprIR
    then_body: BlockIR
    else_body: Optional[BlockIR]   # 无 else 时为 None

@dataclass
class LetStmt(StatementIR):
    """局部不可变绑定（let x = expr; 在方法体内）"""
    name: str
    type: Optional[TypeIR]     # 类型可推导时省略
    value: ExprIR
```

**与 `ConditionalIntent` 的区分：** `IfStmt` 是方法体中的控制流（→ `if (cond) { ... } else { ... }`），`ConditionalIntent` 是视图层中的条件渲染（→ 三元表达式 / `v-if`）。关键差异：`IfStmt` 中的 signal 读取**不触发重渲染依赖追踪**（它在方法体或 effect 中，由外层 `@effect` 显式声明依赖），`ConditionalIntent` 中的条件表达式**必须纳入依赖追踪**（每次渲染时求值决定分支）。

**StatementIR 完整联合体汇总：**

| StatementIR 子类型 | 源码来源 | Codegen 行为 |
|-------------------|---------|-------------|
| `MutationStmt` | `count += 1;` / `todos = [...];` | 展开为 setter 调用 |
| `EventEmitStmt` | `event onSubmit(...);` | 展开为 props 回调调用 |
| `ReturnStmt` | `return expr;` | 标准 return 语句 |
| `ExprStmt` | `println(...);` / 函数调用 | 表达式语句 |
| `IfStmt` | `if cond { ... } else { ... }` | if/else 控制流 |
| `LetStmt` | `let x = expr;` | const 声明 |

**意图 IR 如何驱动地道代码生成：**

```
ListRenderingIntent { iterable: items, body: <ListItem>, key_expr: item.id }
    │
    ├─ React codegen:    {items.map(item => <ListItem key={item.id} .../>)}
    ├─ Vue codegen:      v-for="item in items" :key="item.id"
    ├─ Dioxus codegen:   {items.iter().map(|item| rsx!(ListItem { key: "{item.id}", ... }))}
    └─ Flutter codegen:  ListView.builder(
                            itemCount: items.length,
                            itemBuilder: (ctx, i) => ListTile(key: ValueKey(items[i].id), ...))
```

Flutter 默认使用 `ListView.builder`——即使对静态数据也无性能损失，且避免了"推断错误导致运行时性能陷阱"的风险。

### 跨框架预埋字段（Phase 0 定义，各 Phase 消费）

| IR 类型 | 字段 | Phase 0 默认值 | 服务目标 |
|--------|------|---------------|---------|
| `ElementIR` | `construction_kind` | `"tag-mapping"` | Flutter 构造器选择 |
| `StylePropertyIR` | 语义属性键值对 | — | Flutter/Dioxus 非CSS输出 |
| `StyleRuleIR` | `output_mode` | `"css-modules"` | Dioxus→inline-style, Flutter→widget |
| `SignalIR` | `update_semantics` | `"setter-call"` | Flutter setState vs 各框架 setter |
| `SignalIR` | `accessor_style` | `"direct"` | Dioxus `()` vs Vue `.value` |
| `SignalIR` | `is_copy` | 类型推导 | Dioxus Copy/Clone |
| `EventIR` | `payload_kind` | `"dom-event"` | Flutter 值事件 vs DOM 事件对象 |
| `TypeIR` | `needs_clone` | `false` | Dioxus Clone derive |
| `TypeIR` | `needs_partial_eq` | `false` | 所有框架 key 比较 |
| `BlockIR` | `execution_mode` | `"client-only"` | 远期 SSR |
| `ComponentIR` | `composables` | `[]` | P3+ 逻辑复用 |
| `ComponentIR` | `i18n_resources` | `{}` | P2 i18n 翻译键值对 |
| `ComponentIR` | `async_loader` | `null` | P3 Suspense/@load 异步数据加载块 |
| `ComponentIR` | `is_lazy` | `false` | P3 代码分割（React.lazy / async defineComponent） |
| `ComponentIR` | `error_handler` | `null` | P2 @error 错误边界处理块 |
| `ComponentIR` | `form_state` | `null` | P3 表单验证状态 signal |
| `ComponentIR` | `generic_params` | `[]` | P2 泛型组件类型参数列表 |
| `ElementIR` | `validation_rules` | `[]` | P3 表单字段验证规则链 |
| `ElementIR` | `keep_alive` | `false` | P3 KeepAlive 组件缓存标记 |
| `ElementIR` | `portal_target` | `null` | P3 Portal/Teleport 挂载目标选择器 |
| `ElementIR` | `virtual_list_config` | `null` | P3 虚拟滚动配置（itemCount/estimateHeight 等） |
| `StylePropertyIR` | `animation_def` | `null` | P3 动画关键帧/时长/缓动定义 |
| `ThemeIR` | `modes` | `["light"]` | P3 主题亮/暗/自定义模式列表 |
| `ThemeIR` | `breakpoints` | `{}` | P3 响应式断点定义（sm/md/lg/xl） |
| `ElementIR` | `accessibility_label` | `null` | P1+ 无障碍标签（→ `aria-label` / `Semantics.label`） |
| `ElementIR` | `accessibility_role` | `null` | P1+ 无障碍角色（→ ARIA role / `SemanticsRole`） |
| `ComponentIR` | `is_pure` | `false` | P2 纯函数标记（→ `React.memo` / `const` Widget 优化） |

---

## 十三、转译约束与逃逸机制

### 可移植性层级

- **Layer 1**：完全可移植 — component/signal/computed/@effect、核心元素、@if/@for/@slot、语义样式
- **Layer 2**：部分可移植 — @native、CSS 字符串、平台特定元素、平台特定 API
- **Layer 3**：不可移植 — 直接 DOM 操作、框架专属 API（RSC/KeepAlive/Hero）

### Analyze 规则（P02xx）

| 规则 | 严重度 | 说明 |
|------|--------|------|
| P0201 | WARNING | @for 缺少 key |
| P0202 | WARNING | signal 嵌套属性修改 |
| P0203 | WARNING | computed/@effect 依赖无法确定 |
| P0214 | ERROR | bind: 绑定到 prop 或 prop.field |
| P0215 | WARNING | Map 原地索引赋值（P1） |
| S0110 | INFO | class 混用 style.xxx + 字符串 |

### 框架一致性层级（Framework Consistency Tiers）

并非所有 OmniView 特性在所有目标框架上都能同等质量地实现。为诚实地管理开发者预期，定义三级一致性：

| 层级 | 名称 | 含义 | 示例 |
|------|------|------|------|
| **T1** | **完全一致 (Native)** | 所有 4 个目标框架均有原生或近乎原生的支持，编译产物地道且无降级 | `@if`/`@for`/`@slot`、`signal`/`computed`、核心 20 元素、语义样式、Composable |
| **T2** | **近似一致 (Emulated)** | 至少 1 个目标框架无原生支持，通过降级策略实现语义近似，发出 P 级可移植性警告 | `<Suspense>`（Dioxus→条件渲染）、`@error`（Dioxus→空 catch）、`breakpoints()`（Flutter→`MediaQuery` 近似）、泛型组件（Flutter→`dynamic`） |
| **T3** | **平台增强 (Platform-Enhanced)** | 仅部分框架支持或需外部依赖。编译器不保证跨框架等价，降级为最简实现或跳过 | `<VirtualList>`（React/Vue 需第三方库）、`animate()`（React/Dioxus 无内置引擎）、`@native` 块内容 |

**使用规则：**
- T1 特性：文档中无需额外标记，默认所有示例基于 T1
- T2 特性：文档和编译输出中标注 `⚠️ 框架近似`，包含具体降级说明
- T3 特性：文档中标注 `🔧 平台增强`，使用前需显式确认目标框架支持（编译 flag 或 `#[compat]` 注解）

**可移植性分数与一致性层级的关系：**
- T1 节点计入"平台无关"分子
- T2 节点计入"平台无关"分子（因为编译器仍生成目标代码），但每个 T2 节点扣减 2% 的可移植性分数以反映降级质量损失
- T3 节点计入"平台特定"分母，降低可移植性分数

**各能力的一致性层级归类（Phase 0 相关）：**

| 特性 | 层级 | 说明 |
|------|------|------|
| component/signal/computed/@effect | T1 | 全部原生 |
| @if/@for/@slot/@empty | T1 | 全部原生 |
| 核心 20 元素 | T1 | 全部原生映射 |
| 语义样式 (output_mode: css-modules) | T1 | React/Vue/Dioxus 原生 CSS |
| 语义样式 (output_mode: widget) | T1 | Flutter Widget 属性 |
| Store (Zustand/Pinia/Riverpod/Signals) | T1 | 全部原生 |
| `bind:value` / `bind:checked` | T1 | 全部可映射 |
| `test-id` | T1 | 全部可映射 |

---

## 十四、Phase 0 精确范围与任务清单

### 目标

> **一个 OmniView 组件 → 一个 L2 地道质量的 React + Zustand 应用**
>
> 生成代码的评审标准：一个资深 React 开发者是否能自然地阅读和维护这些代码。

### 范围

✅ React + Zustand + 20 核心元素 + CSS Modules + 3 生命周期 + 意图 IR + L2 地道生成  
✅ 安全表达式子集（属性值和文本插值中的三元/比较/属性访问）  
❌ 无 Vue / Dioxus / Flutter / 动画 / 复合元素 / SCSS / 泛型 Store / 框架辅助库

### 代码质量量化指标

通过对比 OmniView 生成的 React TodoMVC 与公认的手写 React TodoMVC 实现（如 todomvc.com 的 React 版本），在以下维度达标：

| 维度 | 目标 | 测量方法 |
|------|------|---------|
| **变量命名** | 100% 与 OmniView 源码标识符一致 | 差异对比：无编译器生成的随机命名（除 `set` 前缀和 `_omni_` 辅助变量前缀） |
| **Hooks 使用** | `useState`/`useMemo`/`useEffect` 的调用位置和依赖数组与手写一致 | 逐行对比 hooks 调用：依赖数组完全匹配，无遗漏或冗余依赖 |
| **JSX 结构** | 嵌套层次、条件渲染模式、列表映射模式与手写一致 | AST 节点类型对比：无多余的 Fragment 或包装 div |
| **CSS 类名** | `style.xxx` 引用展开为 CSS Modules 导入，无拼接字符串 | 类名差异：0 个硬编码类名（除字符串 class 透传） |
| **注释覆盖率** | 每个逻辑节点（state/effect/条件/列表）前有 `/* .omni:行号 */` 注释 | 统计：100% 的逻辑节点前有行号注释 |

**审查流程：** P0.8 Fixtures 阶段，由至少一位资深 React 开发者（非编译器实现者）逐行对比生成的 TodoMVC 与手写参考实现，标记所有"不符合习惯"的位置，计算差异率。

**目标：** 差异率 < 5%（即 95% 以上的生成代码无需修改即可通过代码审查）。

### 任务清单（12 周，含 2 周缓冲）

```
□ 0.1 语法解析 — component/store/theme/view/style + @if/@for/@slot/@empty/@native
               + event(?可选)/prop/signal/computed + bind:value/bind:checked
               + 安全表达式子集（属性值/文本插值中的三元、比较、属性访问）

□ 0.2 语义分析 — Props 类型检查 + P0214(bind约束) + P0201(key) + P0203(依赖)
               + 依赖收集(3级) + 安全表达式校验 + @native 严格规则校验

□ 0.3 意图 IR 设计 — ★ 15 个核心意图类型（StateIntent/ComputedIntent/ListRenderingIntent/
                    ConditionalIntent/BindIntent/EventIntent/StyleIntent/LayoutIntent/
                    StoreIntent/LifecycleIntent/EffectIntent/SlotIntent/ExternIntent/
                    FallbackIntent）
                    + 27 个跨框架预埋字段（Optional[...] = None）
                    + IRVersion("0.3") 版本号
    【新增：意图 IR 设计 — 3 周】

□ 0.4 React Codegen（L2 地道生成）— 模式匹配 + 惯用法选择 + 代码展开三阶段
    - component→React.FC<Props> + props interface
    - StateIntent→useState + 精确 setter 调用
    - ComputedIntent→useMemo（自动推断 deps）
    - ListRenderingIntent→.map() + 强制 key
    - ConditionalIntent→三元表达式
    - BindIntent→value={x} onChange={e => setX(e.target.value)}
    - LifecycleIntent→useEffect([], cleanup)
    - EffectIntent→useEffect(fn, [deps])
    - StyleIntent→CSS Modules + 语义属性→CSS 查找表
    - LayoutIntent→flex CSS
    - EventIntent→onClick/onChange 等
    - test-id→data-testid
    【新增：三阶段 codegen 管道 — 3 周】

□ 0.5 Store Codegen（Zustand L2）— StoreIntent→create<T>()(set, get => ({}))
    - signal→state 字段 + set() 包装
    - getter→fn（自动访问器展开）+ 精确选择器
    - action→set() 包裹 + 异步批处理
    - getter 依赖链穿透（递归到底层 signal + 去重）

□ 0.6 CSS 生成 — 语义属性→CSS 查找表(~50条) + CSS Modules + :root 变量
               + class_map + 字符串 class→非 Web 目标 ERROR 规则

□ 0.7 测试（L1 编译器输出）— IR 快照测试 + codegen 输出快照测试
    + 跨 React 版本一致性测试（React 18/19）
    + 可移植性诊断测试（P0214/P0201 触发用例）
    + TodoMVC 集成测试（构建+可运行验证）
    + L1 可读性验收清单 7 条标准自动化检查

□ 0.7a 行为测试 — ★ 5 个行为用例组件（counter/list-rendering/conditional/
                bind-input/store-read）+ React Playwright 测试 + CI 集成
    【新增：1 周 — 跨框架语义等价验证】

□ 0.8 Fixtures — 20+组件 fixture + 5+Store fixture
               + 3 完整应用（Counter/TodoMVC/Form）
               + 代码质量对照：生成代码 vs 手写 React 代码的对比审查

□ 0.9 CLI — omnic build --target react --store-target zustand
            + omnic init --template react + 端到端验证脚本
            + --quality-tier L2（默认）| L1（降级）
```

---

## 十六、TodoMVC 完整示例

```omnilang
// ── 数据模型 ──────────────────────────────────────
struct Todo {
    id: i32;
    text: String;
    done: bool;
}

// ── TodoStore ─────────────────────────────────────
store TodoStore {
    signal todos: Array<Todo> = [];
    signal next_id: i32 = 0;
    signal filter: String = "all";

    getter filtered_todos: Array<Todo> = match filter {
        "all" => todos,
        "active" => todos.filter(fn(t) { !t.done }),
        "done" => todos.filter(fn(t) { t.done }),
    };
    getter active_count: i32 = todos.filter(fn(t) { !t.done }).len();
    getter completed_count: i32 = todos.filter(fn(t) { t.done }).len();

    action set_filter(f: String) { filter = f; }
    action add(text: String) {
        if text.trim().len() > 0 {
            todos = [...todos, Todo { id: next_id, text, done: false }];
            next_id += 1;
        }
    }
    action toggle(id: i32) {
        todos = todos.map(fn(t) {
            if t.id == id { Todo { ...t, done: !t.done } } else { t }
        });
    }
    action remove(id: i32) { todos = todos.filter(fn(t) { t.id != id }); }
    action clear_completed() { todos = todos.filter(fn(t) { !t.done }); }
}

// ── TodoApp ───────────────────────────────────────
component TodoApp {
    @store TodoStore;

    view {
        <Container class="todo-app">
            <Heading level={1}>Todos</Heading>

            <AddTodoInput onSubmit={store.add} />

            <Flex direction="row" gap="8px">
                <Button onPress={fn() { store.set_filter("all"); }}
                        variant={store.filter == "all" ? "primary" : "text"}>
                    <Text>All</Text>
                </Button>
                <Button onPress={fn() { store.set_filter("active"); }}
                        variant={store.filter == "active" ? "primary" : "text"}>
                    <Text>Active</Text>
                </Button>
                <Button onPress={fn() { store.set_filter("done"); }}
                        variant={store.filter == "done" ? "primary" : "text"}>
                    <Text>Completed</Text>
                </Button>
            </Flex>

            <List>
                @for todo in store.filtered_todos {
                    <TodoItem
                        key={todo.id} test-id="todo-item"
                        todo={todo}
                        onToggle={fn() { store.toggle(todo.id); }}
                        onRemove={fn() { store.remove(todo.id); }}
                    />
                }
                @empty {
                    <Container class="empty-state">
                        <Text variant="caption" color={theme.colors.text_secondary}>
                            No todos yet
                        </Text>
                    </Container>
                }
            </List>

            <Flex direction="row" justify="between" align="center">
                <Text variant="caption">{store.active_count} items left</Text>
                @if store.completed_count > 0 {
                    <Button variant="text" onPress={store.clear_completed}>
                        <Text>Clear completed ({store.completed_count})</Text>
                    </Button>
                }
            </Flex>
        </Container>
    }

    style {
        .todo-app { layout: column; gap: theme.spacing.md; max_width: 480px; margin_x: auto; padding: theme.spacing.lg; }
        .empty-state { padding: theme.spacing.xl; align: center; }
    }
}

// ── AddTodoInput ──────────────────────────────────
component AddTodoInput {
    event onSubmit: String;
    event onFocus: void?;

    signal text: String = "";

    fn handle_submit() {
        if text.trim().len() > 0 { event onSubmit(text); text = ""; }
    }

    view {
        <Flex direction="row" gap="8px">
            <Input test-id="todo-input"
                bind:value={text} placeholder="What needs to be done?"
                onKeyDown={fn(e) { if e.key == "Enter" { handle_submit(); } }}
                onFocus={fn() { event onFocus(); }}
                style={{ flex: 1 }}
            />
            <Button test-id="add-btn" onPress={handle_submit} variant="primary">
                <Text>Add</Text>
            </Button>
        </Flex>
    }
}

// ── TodoItem ──────────────────────────────────────
component TodoItem {
    prop todo: Todo;
    event onToggle: void;
    event onRemove: void?;

    view {
        <ListItem key={todo.id}>
            <Flex direction="row" align="center" gap="12px" class={class_map({
                item: true, done: todo.done,
            })}>
                <Checkbox checked={todo.done} onChange={fn() { event onToggle(); }} />
                <Text style={{
                    flex: 1,
                    text_decoration: todo.done ? "line_through" : "none",
                    color: todo.done ? theme.colors.text_disabled : theme.colors.text,
                }}>{todo.text}</Text>
                <Button variant="text" onPress={fn() { event onRemove(); }}>
                    <Text color={theme.colors.danger}>×</Text>
                </Button>
            </Flex>
        </ListItem>
    }

    style {
        .item { layout: row; align: center; gap: 12px; padding: theme.spacing.sm;
                border_radius: theme.radius.md; background: theme.colors.surface;
                transition: [background, opacity] 200ms ease; }
        .done { opacity: 0.6; background: theme.colors.surface_secondary; }
    }
}
```

---

## 十七、测试策略

### 单元测试

**IR 生成测试：** 对每个语法构造（component/store/view/style/@指令）编写输入→IR 快照测试。IR 结构为确定性输出，适合 snapshot testing。

**Codegen 输出测试：** 对每组 (IR, target) 编写期望代码快照比对。覆盖所有 20 个核心元素 × React 输出、Store→Zustand 输出、语义样式→CSS 输出。

### 跨框架语义测试 — 诚实的能力边界

**核心认知：统一测试 API 注定沦为最小公分母。** 由于每个框架的测试工具、渲染机制、事件模型完全不同（JSDOM vs Flutter Test vs Dioxus SSR），试图提供一套跨框架测试断言是不可行的。复杂交互（拖拽、手势、动画、异步数据流）的行为在各框架间差异巨大，无法通过统一测试覆盖。

**务实策略：两层级测试**

| 层级 | 内容 | 工具 | 阶段 |
|------|------|------|------|
| **L1：编译器输出测试** | IR 快照 + codegen 输出快照比对。验证编译器为每个目标生成了**正确语法的代码**（不验证运行时行为） | `cargo test` / `pytest` | P0 |
| **L2：目标原生测试** | 对编译产物使用该框架的原生测试工具编写测试。测试用例由人工编写，编译器仅生成测试文件**骨架**（`import` + `render` + `test-id` 定位） | Playwright (React/Vue) / `flutter_test` / `dioxus::ssr` | P1+ |
| **L3：视觉回归测试** | 截图对比（React DOM 截图 vs Flutter Widget 截图），检查**视觉等价性**而非行为等价性 | Storybook / Percy / `flutter_test` 截图 | P4 |

**L1 覆盖所有目标，L2 由各框架团队维护，L3 是跨框架一致性的最终保障。**

基本原则：开发者不应期望为 React 编写的测试能在 Flutter 上运行。OmniView 保证的是**结构语义一致**（点击按钮触发 onPress，列表数据对应 @for），而非**运行时行为逐帧一致**。

### 跨框架行为测试套件

虽然统一测试 API 不可行，但可以通过**共享行为用例 + 各框架独立实现**的方式验证跨框架语义等价。

**行为用例组件（`tests/behaviors/`）：**

在 `tests/behaviors/` 目录下，定义一套**框架无关的行为用例组件**，每个用例是一个完整 OmniView 组件，使用 `test-id` 标记关键元素：

```
tests/behaviors/
  counter.omni         — signal 声明、修改、computed 计算；点击按钮验证文本变化
  list-rendering.omni  — @for 列表渲染、@empty 空状态；验证列表项数量和数据绑定
  conditional.omni     — @if/@else if/@else 条件分支；验证分支切换
  bind-input.omni      — bind:value 双向绑定；验证输入→状态→输出同步
  bind-checkbox.omni   — bind:checked 双向绑定；验证勾选状态同步
  slot-default.omni    — 默认插槽内容投影；验证 children 渲染
  slot-named.omni      — 具名插槽内容投影；验证命名槽位正确分发
  event-optional.omni  — 可选事件未传递时的 no-op 行为；验证不崩溃
  store-read.omni      — Store signal 读取和精确订阅；验证选择器粒度
  store-action.omni    — Store action 调用和状态更新；验证跨组件状态同步
```

**每个用例包含：**
- 一个完整的 OmniView 组件，展示特定行为
- 使用 `test-id` 标记关键元素
- 包含触发状态变化的机制（按钮点击、输入框修改等）
- 使用共享的基础 Store（`TestStore`）来管理可验证的状态

**验证流程（每个 Phase 的目标框架）：**

1. 将行为用例组件编译到目标框架
2. 使用该框架的原生测试工具编写测试用例（Playwright / flutter_test / dioxus-test）
3. 测试用例验证**语义等价行为**：点击按钮后文本变化、列表项数量正确、条件分支正确切换
4. 每个目标框架维护一套独立的测试实现，但**测试断言的语义内容相同**

**Phase 0 最低要求：**
- 至少 **5 个行为用例**（counter, list-rendering, conditional, bind-input, store-read）
- React 目标下的 Playwright 测试全部通过
- 测试结果纳入 CI 流水线

**跨框架一致性矩阵（各 Phase 完成后填写）：**

| 行为用例 | React | Vue | Dioxus | Flutter |
|---------|-------|-----|--------|---------|
| counter | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| list-rendering | ✅ P0 | ✅ P1 | ⚠️ P2 (key 类型差异) | ✅ P4 |
| conditional | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| bind-input | ✅ P0 | ✅ P1 | ✅ P2 | ⚠️ P4 (TextEditingController 生命周期) |
| bind-checkbox | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| slot-default | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| slot-named | ✅ P0 | ✅ P1 | ✅ P2 | ⚠️ P4 (命名参数映射) |
| event-optional | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| store-read | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| store-action | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |

⚠️ = 已知行为差异，已文档化，测试用例调整为验证"语义等价"而非"行为逐帧一致"。

### 可移植性诊断测试

对所有 P 级规则（P0201/P0202/P0203/P0214）编写触发用例，验证：
- 诊断信息包含正确的源位置
- 严重度正确（ERROR vs WARNING）
- 修复建议（如果有）指向正确的文档

P0 最小集：P0201 和 P0214 的触发用例。

---

## 十八、调试与开发体验

### Source Map（Phase 1）

编译产物为纯框架代码，不依赖 OmniLang 运行时。通过 source map 将编译后代码映射回 `.omni` 源码行。React 目标利用 Webpack/Vite 的 source map 支持；Flutter 目标利用 Dart 的 `--source-map` flag。

### test-id 辅助定位

- **React/Vue/Dioxus**：`test-id="xxx"` → `data-testid="xxx"`，与现有测试工具链无缝配合
- **Flutter**：`test-id="xxx"` → `key: ValueKey("xxx")`。注意：`ValueKey` 仅用于测试定位，**不影响 Widget 重建判断**（与 `GlobalKey` 不同）。若组件同时需要 `key`（如在 `@for` 中），使用复合 key：`ValueKey("${item.id}_${test_id}")`

### 编译输出注释

Phase 0 生成的 React 代码中保留注释形式的原始行号：

```tsx
/* .omni:12 */ const [count, setCount] = useState(0);
/* .omni:13 */ const doubleCount = useMemo(() => count * 2, [count]);
```

便于在浏览器 DevTools 中通过 `<>` 按钮定位到 `.omni` 源文件行。此功能为编译时行为，不影响产物体积（可在生产构建中 strip）。

### 生成代码自动格式化

生成代码的缩进、换行、空格直接影响 L1 可读性验收清单中"结构可导航"和"逻辑可理解"的达成。Phase 0 开始即需自动格式化支持：

| 目标框架 | 格式化工具 | 策略 |
|---------|-----------|------|
| React (.tsx) | `prettier` | 若项目已安装 prettier，编译后自动调用 `prettier --write`；否则使用内置基本缩进引擎（2 空格、属性换行） |
| Vue (.vue) | `prettier` | 同上 |
| Dioxus (.rs) | `rustfmt` | P2 编译后自动调用 `rustfmt`（Cargo 项目通常已配置） |
| Flutter (.dart) | `dart format` | P4 编译后自动调用 `dart format` |

**CLI 选项：** `omnic build --no-format` 跳过格式化（用于 CI 中后续统一格式化的场景）。默认启用。

**CI 验证：** 在行为测试前增加"格式化后代码可解析"验证步骤——确保 `prettier --check` 通过，防止格式化破坏语法。

---

## 二十、已知未决事项（v3.0-final）

以下 5 项 **P0 阻断**事项必须在 Phase 0 代码冻结前彻底解决——它们直接影响 React codegen 的正确性和测试覆盖。

| # | 优先级 | 条目 | 类别 | 目标 Phase | 简述 |
|---|--------|------|------|-----------|------|
| 1 | **P0 阻断** | `@for` 循环变量闭包捕获 | 边界条件 | P0 | 编译规则已制定（块级 const），P0.4 React codegen 必须正确生成闭包捕获 |
| 2 | **P0 阻断** | `@if`+`bind:` 的 key 策略 | 边界条件 | P0 | 编译器给出 S 级提示，P0.2 语义分析实现 |
| 3 | **P0 阻断** | computed/effect 与 Store getter 交叉依赖 | 响应式边界 | P0 | 穿透算法已制定，P0.2 语义分析实现 |
| 4 | **P0 阻断** | 测试策略 — IR 快照 + codegen 快照 | 指导性 | P0 | P0.7 实施。L1 编译器输出测试覆盖所有语法构造 |
| 5 | **P0 阻断** | `BlockIR` + `StatementIR` 完整定义 | 完整性 | P0.3 | `statements: list[StatementIR]`。`MutationIntent` 必须挂载为 BlockIR 中的 `MutationStmt`。P0.3 第一周完成 StatementIR 联合体定义 |

---

> **文档版本：** 从 OmniLang_v0.3_设计方案.md v3.0-final 提取  
> **IR 版本：** 0.3  
> **冻结日期：** 2026-06-05  
> **相位：** Phase 0 — React + Zustand + 20 核心元素 + CSS Modules + 意图 IR + L2 地道生成  
> **迭代纪律：** Phase 0 期间冻结不可修改。任何 IR 调整通过 `IRVersion` 次版本号迭代（`0.3`→`0.4`），不回修改本文档。
