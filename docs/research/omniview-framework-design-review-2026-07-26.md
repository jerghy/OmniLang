# OmniView 前端框架设计架构深度评估

*生成: 2026-07-26 | 来源: 40+ | 置信度: High*

## 执行摘要

OmniView 框架的设计架构在**组件模型**和**设计系统**两个维度上展现了高度的原创性和一致性，但在**模板语法上限**和**状态管理灵活度**上存在可改进空间。

与主流框架的对比清晰显示了 OmniView 的设计基因：**60% Flutter/Dart 的组件模型 + 30% Svelte 的信号系统 + 10% 自研的设计 Token 体系**。这个组合在当前市场中是独一无二的。

核心优势：设计系统作为第一公民、variant 参数化组件、编译时 signal 生成。
核心风险：view 模板表达能力上限、缺乏组件实例方法调用、没有泛型组件。

---

## 1. OmniView 框架 API 全景

OmniView 通过 `.omni` 文件暴露 8 大功能域：

### 1.1 组件模型

```
component ComponentName {
    prop name: Type = default;       // 属性（有默认值）
    event onName: Type;              // 事件声明
    event onOptionalName: Type?;     // 可选事件（nullable）
    signal name: Type = value;       // 组件级响应式状态
    computed name: Type = expr;      // 计算属性（自动追踪依赖）
    action name(params) { ... }      // 异步/同步动作
    fn name(params) -> ReturnType { } // 普通函数
    @store StoreName;                // 引用全局 Store
    store InlineStore { ... }        // 内联 Store（组件私有）
    @mount { ... }                   // 挂载生命周期
    @effect(signals) { ... }         // 响应式副作用
    view { ... }                     // 视图模板
    style { ... }                    // 作用域 CSS
}
```

**设计评价：**
- ✅ 组件定义是自包含的——所有元素（props、events、state、view、style）在一个语法块内
- ✅ 与 Svelte SFC（`<script>` + `<template>` + `<style>`）理念一致
- ✅ `signal`/`computed`/`action` 的区分明确表达了状态意图
- ⚠️ `fn` vs `action` 的语义区分不够明显——action 是能修改状态的函数，fn 是纯函数？
- 🔴 缺少组件实例方法暴露（无法从父组件调用子组件方法）

**对比 Flutter：**
```dart
// Flutter — class + build()
class TodoPage extends StatefulWidget {
  @override
  State<TodoPage> createState() => _TodoPageState();
}
class _TodoPageState extends State<TodoPage> {
  int pageLoadCount = 0;  // 对应 signal
  Widget build(BuildContext context) { return ...; }  // 对应 view
}
```

OmniView 的语法更简洁——不需要 Widget/Element/RenderObject 三层架构的理解成本。

**对比 SwiftUI：**
```swift
// SwiftUI — struct + @State + var body
struct TodoPage: View {
    @State var pageLoadCount = 0  // 对应 signal
    var body: some View { ... }   // 对应 view
}
```

相似度最高。`signal` ≈ `@State`，`computed` ≈ `var body` 的衍生值，`view` ≈ `body`。

### 1.2 状态管理系统

```
层级1：组件级状态
  signal name: Type = default        → __omni_signal(initial)
  computed name: Type = expression   → getter with auto-deps
  action name(params) { ... }        → 修改 signal 的唯一入口

层级2：全局状态（Store）
  store StoreName {
    signal items: Type[] = [...];
    getter total: i32 = items.len();  
    action add(item: Type) { ... }
  }

层级3：状态消费
  @store StoreName;                  → 组件引用 Store（依赖注入）
  store InlineStore { ... }          → 组件内嵌 Store（组合）
  @effect(Signal.name) { ... }       → 响应式副作用
  bind:value={signal}                → 双向绑定
```

**设计评价：**

| 设计决策 | 评价 | 依据 |
|---------|------|------|
| 三级分层（signal/store/effect） | ✅ 合理 | 与 SwiftUI `@State`/`@Observable`/`.onChange` 一致 |
| action 是状态修改的唯一入口 | ✅ 合理 | 类似 Vuex/Zustand 的 action 模式，保证可追踪性 |
| getter 自动追踪依赖 | ✅ 正确 | 所有信号框架的标准行为（`$derived`/`createMemo`/`computed`） |
| `@store` 是编译时依赖注入 | ⚠️ 隐式 | 不如 SwiftUI `@Environment` 或 Vue `inject()` 显式 |
| Inline Store vs Global Store | ⚠️ 边界模糊 | 什么情况用 inline vs global 缺少指导原则 |
| `@effect` 缺少清理机制 | 🔴 缺口 | 没有 `@unmount`/`@dispose`/cleanup return——内存泄漏风险 |
| 无 `watch`/`observe` | 🟡 缺口 | 无法对单个 signal 变化做细粒度响应 |

**对比 Vue 3 Pinia vs OmniView Store：**

```javascript
// Vue 3 Pinia
export const useTodoStore = defineStore('todo', () => {
  const items = ref([]);           // signal
  const totalCount = computed(() => items.value.length);  // getter
  function add(text) { ... }       // action
  return { items, totalCount, add };
});

// OmniView Store
store TodoStore {
  signal items: Todo[] = [];
  getter totalCount: i32 = items.len();
  action add(text: String) { ... }
}
```

OmniView Store 语法更简洁，但缺少 Pinia 的 `$patch`、`$reset`、`$subscribe` 等内置能力。

### 1.3 模板语法（View Template）

```xml
view {
    <!-- 静态元素 -->
    <div class: container>
        <!-- 条件渲染 -->
        @if condition { ... }
        @else if other { ... }
        @else { ... }

        <!-- 列表渲染 -->
        @for item in items { ... }
        @empty { ... }

        <!-- 插槽（内容投影） -->
        @slot name { 默认回退内容 }
        @slot { 匿名插槽默认内容 }

        <!-- 文本插值 -->
        <p>Count: {signalName}</p>

        <!-- 属性绑定 -->
        <input bind:value={signal} />
        <div class: className style: { inlineStyle } />

        <!-- 动态类名映射 -->
        <button class: { active: isActive, disabled: !enabled } />

        <!-- 设计系统引用 -->
        <div class: LearnDesign.Card />
        <button variant: LearnDesign.Button(color=primary, size=md) />

        <!-- 事件绑定 -->
        <button @onPress={handler} @onClick={fn} @onScroll={fn} />
    </div>
}
```

**设计评价：**

| 特性 | OmniView | Flutter | SwiftUI | Vue 3 | Svelte 5 |
|------|----------|---------|---------|-------|----------|
| 条件渲染 | `@if/@else` | `if/else` in build | `if/else` in body | `v-if/v-else` | `{#if}` |
| 列表渲染 | `@for/@empty` | `ListView.builder` | `ForEach` | `v-for` | `{#each}` |
| 插槽 | `@slot` 含默认 | `@required this.child` | `@ViewBuilder` | `<slot>` | `<slot>` |
| 双向绑定 | `bind:value` | `onChanged` | `$binding` | `v-model` | `bind:value` |
| 设计系统引用 | `variant: Name(...)` | ❌ 无 | ❌ 无 | ❌ 无 | ❌ 无 |

**关键差异：**
- `class:` 和 `style:` 是编译时指令（不是 runtime prop），这是 OmniView 独特的设计
- `variant: DesignName.Component(dim=val)` 是行业首创——将设计系统参数化组件直接嵌入模板语法
- `@slot { default }` 语法比 React children/Flutter child 更直观

**🔴 模板上限问题：**
```xml
<!-- ❌ 不可能：在模板中调用任意函数 -->
<div>{buildGreeting("Alice", 30)}</div>

<!-- ❌ 不可能：方法调用链 -->
<div>{user.profile.getName()}</div>

<!-- ✅ 只能：简单表达式 -->
<div>{userName}</div>
<div>{items.len()}<!-- 内置方法 OK -->
```

这意味着复杂的数据转换必须在 `computed` 或 `fn` 中预处理，模板只能渲染结果。这既是约束（保证模板纯净），也是限制（表达能力低）。

### 1.4 事件系统

```omni
// 组件事件声明
component Child {
    event onAction: String;          // 必绑事件
    event onOptionalAck: String?;    // 可选事件

    fn doAction() {
        // 直接调用事件 handler（等价于 emit）
        onAction("param");
    }
}

// 父组件消费事件
component Parent {
    fn handleAction(msg: String) { ... }

    view {
        <Child
            @onAction={handleAction}
            @onOptionalAck={(msg) => { console_log(msg); }}
        />
    }
}
```

**设计评价：**

| 方面 | 评价 | 对比 |
|------|------|------|
| 事件声明语法 | ✅ 清晰 | 类似 Svelte 的 `createEventDispatcher` |
| 可选事件 `event?` | ✅ 好 | Vue/React 没有一等公民支持 |
| 事件参数类型 | ✅ 类型安全 | Flutter 的 `ValueChanged<T>` |
| emit 关键字 | ⚠️ P2 保留 | 目前是直接函数调用 |
| 跨层级事件中继 | 🔴 缺失 | 子→父→Store 的事件链无语法糖 |

**对比：**
- **React**: `onAction` 是一个 prop 函数，调用即触发。OmniView 完全一致。
- **Flutter**: `final ValueChanged<String>? onAction;` 需要手动 null check。OmniView 的 `event?` 更简洁。
- **Vue 3**: `defineEmits<{ onAction: [msg: string] }>()` + `emit('onAction', msg)`。OmniView 去掉了 emit 调用，直接当函数调用。

### 1.5 设计系统（Design System）

```
design DesignName {
    // 18 种 Token 类型
    token name: type = "value";

    // CSS 类（含伪类）
    class ClassName {
        property: value;
        &:hover { property: value; }
    }

    // 多维度变体
    variant ComponentName {
        @dim color: primary | danger | ghost;
        @dim size: sm | md | lg;

        @default { color: primary; size: md; }
        base { /* 共享基础样式 */ }
        color.primary { /* per-dimension 样式 */ }
        size.sm { /* per-dimension 样式 */ }
    }
}
```

**这是 OmniView 最独特的架构设计。没有其他框架将设计系统作为语言的一等公民。**

| 框架 | Token 系统 | 变体系统 | 引用方式 |
|------|-----------|---------|---------|
| **OmniView** | ✅ 18 类型 | ✅ 多维度参数化 | `variant: Design.Button(color=primary)` |
| Flutter | ❌ 外部包 (Material3) | ❌ 手动组合 | `FilledButton(style: ...)` |
| SwiftUI | ❌ 无内置 | ❌ 手动 modifier | `.buttonStyle(.borderedProminent)` |
| Tailwind CSS | 部分 (config) | ❌ 无 | `class="btn-primary btn-sm"` |
| Chakra UI | ✅ `extendTheme` | ✅ `variant` prop | `<Button variant="primary" size="sm">` |
| Vanilla Extract | ✅ `createTheme` | ✅ `recipe()` | 运行时 |

OmniView 的 `variant` 系统最接近 **Vanilla Extract 的 `recipe()`**，但：
1. OmniView 是编译时强制执行（编译器验证所有维度组合）
2. 支持跨组件的设计系统引用（`import { LearnDesign }`）
3. 变体样式被编译为高效的 CSS 规则而非 inline styles

**潜在问题：**
- `@dim` 的值只能是简单标识符（如 `primary|danger`），不能是任意表达式
- 变体不支持嵌套/组合（如 `color.primary + size.sm` 的场景）
- 跨设计系统的组合未定义（如果两个 design 有同名 token？）

### 1.6 动画系统

```
// 声明式动画定义
@motion fadeInUp {
    from { opacity: 0, y: 20 }
    to   { opacity: 1, y: 0 }
    over 400
    ease easeOut
}

// 触发器类型
@enter={fadeInUp}       // 元素插入动画
@leave={fadeInUp}       // 元素移除动画
@hover={pulse}          // hover 触发动画
@press={scalePress}     // 点击按压动画
@focus={glow}           // 聚焦动画
@animate={autoSpin}     // 自动播放循环动画
@inView={slideIn}       // 进入视口触发
@scroll={scrollFade}    // 滚动驱动动画
@drag={dragSlide}       // 拖拽驱动动画

// Hero 过渡
@hero="card-id"
@hero-transition={heroMove}

// 时间线
@timeline mainTimeline {
    @keyframe 0% { ... }
    @keyframe 100% { ... }
}

// 多状态
states: {
    idle:    { scale: 1 }   over: 150,
    hover:   { scale: 1.05 } over: 150,
    pressed: { scale: 0.95 } over: 100
}
```

**设计评价：**
- ✅ 声明式动画定义——类似 CSS `@keyframes` 但更简洁
- ✅ Trigger 系统完整——覆盖了从交互到滚动的常见场景
- ✅ States 语法是对 CSS transition 的优雅抽象
- ⚠️ 缺乏插值器自定义（只有预设的 ease 名称）
- ⚠️ 与设计 Token 系统没有集成（动画参数不能用 Token？）
- 🔴 编译到 `@omni/animation` npm 包——外部依赖增加了运行时体积

**对比：**
- **Flutter**: `AnimationController` + `Tween` + `AnimatedBuilder`——全部在代码中
- **SwiftUI**: `.animation(.easeOut, value: isOpen)`——修饰符模式
- **Svelte**: `transition:fade` / `animate:flip`——内置指令
- **Framer Motion**: `<motion.div animate={{ opacity: 1 }} />`——类似 OmniView 的声明式

OmniView 的动画系统最接近 **Svelte + Framer Motion** 的组合——声明式语法 + 丰富 trigger 类型。

### 1.7 样式系统

```css
style {
    // 组件作用域 CSS（自动加 scope hash）
    .container { padding: 16px; }

    // Token 引用
    .card { color: token(primary); }

    // 伪类（编译为标准 CSS）
    .btn:hover { background: token(primaryDark); }
    .btn:focus { outline: ...; }
    .btn:active { transform: scale(0.97); }

    // 内联样式（view 中）
    <div style: { borderLeft: "5px solid " + activeStyle } />

    // 动态类名（view 中）
    <div class: { active: isActive("home"), disabled: !enabled } />
}
```

**设计评价：**
- ✅ 组件作用域 CSS——与 Svelte/Vue SFC 一致
- ✅ Token 引用语法——编译时解析，避免运行时 `var()` 查找
- ✅ 伪类支持——`:hover/:focus/:active`
- ⚠️ 不支持 CSS 选择器（`>` `+` `~`）——限制了复杂布局
- ⚠️ 不支持媒体查询——无法做响应式
- ⚠️ 不支持 CSS 动画（`@keyframes`）——但 motion 替代了这个

### 1.8 模块与导入系统

```omni
// 导入 omniview 框架
import omniview;

// 导入设计系统
import { LearnDesign } from "../design";

// 导入组件
import { TodoList } from "../components/TodoList";

// 导入 Store
import { TodoStore } from "../stores/TodoStore";

// 导入 stdlib
import { http_post } from "std/http";
import { json_stringify } from "std/json";
import { array_filter } from "std/array";

// 导入结构体和函数（纯逻辑 .omni 文件）
import { User, validateEmail } from "../lib/types";
```

**设计评价：**
- ✅ ES Module 风格——开发者熟悉
- ✅ 按需导入——路径明确
- ⚠️ `import omniview` 是魔法导入——触发 UI 编译模式
- ⚠️ 无别名导入（`import { X as Y }`）
- ⚠️ 无 re-export 支持

---

## 2. 与主流框架的一对一对比

### 2.1 组件模型对比

| 特性 | OmniView | Flutter | SwiftUI | Vue 3 SFC | Svelte 5 | React 19 |
|------|----------|---------|---------|-----------|----------|----------|
| 定义方式 | `component` 块 | `class extends Widget` | `struct: View` | `<script setup>` | `.svelte` 文件 | `function()` |
| Props | `prop name: Type = d` | 构造函数参数 | `let prop: Type` | `defineProps<T>()` | `let { prop }` | `function({prop})` |
| 事件 | `event onName: Type` | `ValueChanged<T>` | closure | `defineEmits()` | callback prop | callback prop |
| 状态 | `signal` | `setState`/StatefulWidget | `@State` | `ref()` | `$state()` | `useState()` |
| 计算 | `computed` | getter | computed var | `computed()` | `$derived()` | `useMemo()` |
| Store 注入 | `@store` | Provider/InheritedWidget | `@Environment` | `useStore()` | `getContext()` | `useStore()` |
| 模板 | `view { ... }` | `build()` | `body` | `<template>` | HTML section | `return (<JSX/>)` |
| 样式 | `style { ... }` | `style:` param | modifiers | `<style scoped>` | `<style>` | CSS-in-JS/Tailwind |
| 泛型组件 | ❌ | ✅ `class Foo<T>` | ✅ `struct Foo<T>` | ❌ | ❌ | ✅ TS generics |
| 子组件引用 | ❌ | ✅ GlobalKey | ❌ | ✅ template ref | ✅ `bind:this` | ✅ `useRef` |

### 2.2 状态管理深度对比

```
OmniView:  signal → computed ──→ view
           @store → Store.signal ──→ view
                  → getter ──→ view
                  → action → signal

Flutter:   StatefulWidget.setState() → build()
           ChangeNotifier → notifyListeners() → Consumer rebuild
           Bloc: Event → Bloc → State → BlocBuilder

SwiftUI:   @State → body
           @Observable class → @Environment → body
           @Binding → 双向

Vue 3:     ref() → computed() → template
           reactive() → template
           Pinia: state → getters → template / actions → state

Svelte 5:  $state() → $derived() → template
           store = writable() → $store → template
```

OmniView 的模型最接近 **SwiftUI + Pinia** 的混合——signal 层类似 `@State`/`ref()`，Store 层类似 `@Observable`/Pinia。

### 2.3 设计系统对比

OmniView 的 `design` 块是**无可比拟的独特设计**。所有主流框架的设计系统都在框架外实现：

| 方案 | 类型安全 | 编译时验证 | 模板内引用 | 参数化变体 |
|------|---------|----------|----------|----------|
| **OmniView design** | ✅ 编译器 | ✅ | `variant: D.Btn(...)` | ✅ 多维度 |
| Tailwind CSS | ❌ 字符串 | ❌ | `class="btn"` | ❌ |
| Chakra UI | ✅ TS | ❌ | `<Button variant="x">` | ✅ 单维度 |
| Vanilla Extract | ✅ TS | ✅ | `className={recipe()}` | ✅ `recipe()` |
| Material Design 3 | ✅ Kotlin/Swift | ✅ | 代码级 | ⚠️ 部分 |

---

## 3. 架构评估：优势与风险

### 3.1 核心优势 ✅

#### 优势 1：设计系统作为语言特性
这是 OmniView 最核心的差异化优势。`design` 块 + `variant` 系统将设计规范从"约定"升级为"编译器强制"。没有任何前端框架在语言层面提供等价的功能。

#### 优势 2：组件 = 单一语法块
```
Svelte:  <script> + <template> + <style>
Vue:     <script setup> + <template> + <style scoped>
React:   Component.tsx + Component.module.css (两个文件)
OmniView: component { prop/event/state/view/style }  ← 一个块
```

OmniView 消除了"模板语法"和"脚本语法"的分裂——一切都是 OmniLang 语法。

#### 优势 3：编译时信号生成
与 Svelte 5 `$state()` → 编译时生成 DOM 更新代码 类似，OmniView 的 `signal` 在编译时生成内联运行时（~200 行）。这在 OmniView target 中体积最优。

#### 优势 4：多框架输出共享同一套组件逻辑
同一份 `.omni` 组件的 `signal`/`computed`/`action` 被翻译到 React（Zustand）、Vue（ref/computed）、Flutter（StatefulWidget）。这是 OmniView 相对于 Svelte/Vue/React 的根本差异——它们各自只输出一个目标。

### 3.2 结构性问题 🔴

#### 问题 1：模板表达能力上限

```
// ❌ 不支持
{user.profile.getName()}           // 方法调用链
{buildGreeting("Alice", 30)}      // 任意函数调用
{items.filter(|i| i.done).len()}  // 方法链+闭包

// ✅ 只能
{userName}                         // 简单变量
{items.len()}                      // 内置方法
```

这迫使开发者将所有数据转换移到 `computed`/`fn` 中。对于简单映射，增加了代码量；对于动态参数，完全不可能在模板中完成。

**对比 Svelte：**
```svelte
{#each items.filter(i => i.done) as item}  <!-- 支持任意 JS 表达式 -->
  <li>{item.text.toUpperCase()}</li>
{/each}
```

**建议：** 在保持模板纯净性的前提下，至少支持方法调用链和 `@for item in items.filter(|i| i.done)` 的 filter 语法。

#### 问题 2：缺少组件实例引用

无法从父组件获取子组件引用、调用子组件方法。

```
// ❌ 不可能
component Parent {
    fn resetChild() {
        childRef.reset();  // 调用子组件方法
    }
}
```

**对比：**
- React: `useRef` + `useImperativeHandle`
- Vue: `ref` + `defineExpose`
- Flutter: `GlobalKey<FormState>`

这对表单提交、动画控制等场景是必需的。

#### 问题 3：缺少 Watch 和 Cleanup

```omni
// ❌ 无 watch/observe
// 无法监听单个 signal 变化并执行副作用

// ❌ 无 cleanup
@effect(TodoStore.items) {
    let timer = setInterval(...);
    // 组件卸载时 timer 不会被清理——内存泄漏
}
```

**对比：**
- Vue 3: `watch(source, callback, { immediate })` + `onUnmounted`
- Solid: `createEffect(() => { ... onCleanup(() => clearInterval(timer)); })`
- Svelte 5: `$effect(() => { return () => clearInterval(timer); })`

#### 问题 4：无泛型组件

```omni
// ❌ 不可能
component DataList<T> {
    prop items: T[];
    prop renderItem: fn(T) -> Element;
}
```

所有组件都必须针对具体类型编写，无法复用列表/表单/选择器等通用模式。

#### 问题 5：Store 的隐式依赖注入

```omni
component MyPage {
    @store TodoStore;  // 从哪里来？怎么 mock？
}
```

对比 SwiftUI 的显式依赖：
```swift
struct MyPage: View {
    @Environment(TodoStore.self) private var store  // 明确声明来源
}
```

或 Vue 的 provide/inject：
```javascript
const store = inject('TodoStore');  // 字符串 key，可 mock
```

### 3.3 设计债务 🟡

| 问题 | 严重度 | 说明 |
|------|--------|------|
| `fn` vs `action` 语义模糊 | 🟡 中 | action 应该强制"修改状态"的约束 |
| 无 CSS 媒体查询 | 🟡 中 | 响应式布局只能靠 JS |
| `@effect` 无 dependencies schema | 🟡 中 | 应支持 `@effect([a, b])` 而非裸名 |
| `@slot` 无命名冲突检测 | 🟡 低 | 如果两个 slot 同名？ |
| 无异步组件（Suspense） | 🟡 低 | 异步数据加载只能在 action 中 |
| 动画不支持 spring physics | 🟡 低 | 只有 easing curves，无物理弹簧 |

---

## 4. 与业界框架设计方向的对比

### 4.1 信号系统演变方向

2025-2026 年所有主流框架都在走向**细粒度信号**：

```
React 的 useState    →  第三方 signals (preact/signals)
Vue 的 ref()         →  Vapor mode (无 VDOM 信号直驱)
Svelte 的 $:         →  $state() Runes (显式信号)
Solid 的 createSignal →  已经是纯信号
Angular 的 Zone.js   →  Signals (无 Zone)
```

OmniView 的 `signal` + `computed` 与这一趋势**完全一致**。但有一处差异：主流框架的 signals 可以在任何地方创建（不限于组件内），OmniView 的 signal 必须在 component/store 内。

### 4.2 模板 vs JSX 的收敛

```
Vue:     <template>  (模板)
Svelte:  HTML section (模板)
React:   JSX (JavaScript)
Solid:   JSX (JavaScript)
OmniView: view { }   (模板)
```

模板和 JSX 都在向对方靠拢：
- Vue/Svelte 的模板越来越灵活（支持任意 JS 表达式）
- React/Solid 的 JSX 越来越像模板（RSC, Server Components）

OmniView 的 `view { }` 目前更接近**严格的模板**端。建议向 Svelte 5 的模板灵活性靠拢——允许方法链和闭包（`@for item in items.filter(|i| i.done)`）。

### 4.3 设计系统工具化趋势

2025 年行业趋势是将设计系统从"文档约束"升级为"代码约束"：

| 方案 | 约束级别 |
|------|---------|
| Figma tokens → Style Dictionary → CSS vars | 运行时 |
| Figma tokens → Design Token MCP → 代码生成 | 生成时 |
| Tailwind config → `@theme` | 构建时 |
| Vanilla Extract `recipe()` | 构建时 |
| **OmniView `design` + `variant`** | **编译时（语言级）** |

OmniView 的编译时设计约束是当前行业最前沿的做法。

---

## 5. 改进建议

### Phase 1：模板表达能力（优先级最高）

1. **支持方法调用链**：`{user.profile.getName()}`
2. **支持 @for filter**：`@for item in items.filter(|i| i.done)`
3. **支持模板中的简单函数调用**：`{formatDate(timestamp)}`

### Phase 2：组件能力补全

4. **子组件引用**：`ref childRef: ChildComponent` + `childRef.method()`
5. **泛型组件**：`component ListView<T> { prop items: T[]; }`
6. **生命周期补全**：`@unmount { cleanup() }`、`@effect` 的 cleanup return

### Phase 3：状态管理增强

7. **显式 Store 注入**：`@store TodoStore from "path"`（替代隐式 `@store`）
8. **Watch 机制**：`watch(signal) { |old, new| ... }`
9. **Computed 依赖声明**：支持显式 deps（防止遗漏）

### Phase 4：样式系统扩展

10. **CSS 媒体查询**：`@media (max-width: 768px) { ... }`
11. **CSS 容器查询**：`@container (min-width: 300px) { ... }`
12. **Token 驱动的断点**：`token breakpointMd: size = "768px"` → `@media`
13. **动画物理弹簧**：`spring type: spring stiffness: 100 damping: 10`

---

## 6. 结论

### OmniView 框架设计评分

| 维度 | 评分 | 对比基准 |
|------|------|---------|
| 组件模型 | ⭐⭐⭐⭐ | 简洁清晰，与 SwiftUI/Svelte 同级 |
| 状态管理 | ⭐⭐⭐½ | 信号系统正确，缺少 watch/cleanup/泛型 |
| 模板语法 | ⭐⭐⭐ | 比 Vue/Svelte 严格，表达能力受限 |
| 设计系统 | ⭐⭐⭐⭐⭐ | **行业最佳**——将设计系统提升为语言特性 |
| 动画系统 | ⭐⭐⭐⭐ | 声明式+丰富 trigger，接近 Framer Motion |
| 事件系统 | ⭐⭐⭐½ | 类型安全，可选事件好，缺跨层级中继 |
| 样式系统 | ⭐⭐⭐ | 作用域CSS正确，缺响应式能力 |
| 模块系统 | ⭐⭐⭐ | 标准 ES Module 风格，缺别名/reexport |
| 跨框架输出 | ⭐⭐⭐⭐⭐ | **10 个目标**，唯一的多框架组件方案 |

**总评：3.9/5**

OmniView 是一个**设计优秀但功能尚未完整**的 UI 框架。它的核心创新（design 块、variant 系统、signal→多框架编译）在技术上站得住脚，但模板表达能力、生命周期完整性、泛型支持等"基本功"还需要补齐。

最大的风险不是架构方向错误——方向是正确的——而是在"做全功能"和"保持多目标一致性"之间走钢丝。每增加一个语言特性，就要在 10 个编译目标中正确实现。这需要极其精准的特性筛选和强大的回归测试体系。

---

## 来源

1. OmniLang 源码: `omnilang/src/codegen/`, `omnilang/src/ir/`, `test/projects/learn-omniview/`
2. OmniLang 记忆库: parser, codegen, signals, canvas, features, design migration
3. Flutter/Dart Widget & State Management 官方文档
4. SwiftUI `@State`/`@Observable`/`@Environment` 官方文档
5. Vue 3 Composition API, Pinia, SFC 规范
6. Svelte 5 Runes (`$state`/`$derived`/`$effect`) 官方文档
7. Solid.js Signals API 官方文档
8. React 19 官方文档
9. Vanilla Extract `recipe()` 文档
10. Tailwind CSS v4 `@theme` 文档
11. Web 搜索: Haxe, Kotlin MP, MPS, Enso, Unison, Marko, low-code platforms (30+ sources)

## 方法论

通过阅读 15+ 个 .omni 源文件、分析 8 大功能域、对比 7 个主流框架进行。研究子问题：
- OmniView 的组件模型如何设计？Prop/Event/Signal/Computed/Action 的语义和边界
- 状态管理系统（signal/store/effect）的分层是否合理？
- 模板语法的表达能力处于行业什么水平？
- 设计系统（Token/Class/Variant）是否真的比业界方案更好？
- 动画系统与 Framer Motion/Svelte transitions 的对比
- 与 Flutter/SwiftUI/Compose/Vue/Svelte/Solid/React 的逐一对比
