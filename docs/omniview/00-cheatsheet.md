# 00 — 速查表（Cheatsheet）

> 面向 LLM few-shot 与人工速查的最小上下文：**10 个语义域 × 1 个最小可编译示例** + 事件名表 + 删除语法表（verbatim 诊断消息）+ 错误码速查。
>
> 每例均为完整 `.omni` 文件（`import omniview;` + 顶层声明 + `component { view { } }`），已通过 docs-gate 完整块 parse-check 与 `omnic build`（OmniView/React 双目标）验证——可直接复制为独立文件编译。
>
> 权威文档：本速查表是入口；逐域细节见 §1 各示例标注的文档章节。

---

## 1. 十域最小可编译示例

### 1.1 域 1 — Ref 系统 → [05-组件](05-components.md)

元素引用：`ref name: Element` 声明 + `@ref={name}` 绑定 + 语义化 ref 操作（命令式 `focus()`/`scrollTo()` 与声明式 `bind:value` 双向绑定）。`ref.value = x` 命令式写已删除。

```omni
import omniview;

component RefDemo {
    ref inputEl: Element;
    signal text: String = "hello";

    view {
        <input @ref={inputEl} bind:value={text} />
        <button @onPress={() => { inputEl.focus(); }}>Focus</button>
    }
}
```

### 1.2 域 2 — 动画系统 → [11-运动与动画](11-motion-animation.md)

`@motion name { from { ... } to { ... } over: ... }` 顶层定义 + 8 触发器之一（`@enter`/`@hover`/`@press`/`@focus`/`@in-view`/`@scroll`/`@drag-horizontal`/`@drag-vertical`）绑定。`paused` 已删除。

```omni
import omniview;

@motion fadeIn {
    from { opacity: 0, translateY: 10 }
    to   { opacity: 1, translateY: 0 }
    over: 300ms
    easing: ease-out
}

component MotionDemo {
    view {
        <div @enter={fadeIn}>fades in</div>
    }
}
```

### 1.3 域 3 — Design Tokens + 主题 → [10-设计系统](10-design-system.md)

`@themes { ... }` 声明主题集合、`@default-theme` 选默认、`token` 定义设计值；`signal ... @theme` 标记当前主题信号（编译器自动接线三目标）。切换主题 = 普通 action 写信号。

```omni
import omniview;

design AppDesign {
    @themes { light, dark }
    @default-theme light;
    token primary: color = "#6366f1" dark "#a5b4fc";
}

store ThemeStore {
    signal mode: String @theme = "light";
    action toggle() { mode = if mode == "light" { "dark" } else { "light" }; }
}

component ThemeDemo {
    @store ThemeStore;
    view {
        <div class: card>mode: {ThemeStore.mode}</div>
    }
}
```

### 1.4 域 4 — Slots → [06-模板语法](06-templates.md)

body 级 `@slot name { 回退 }` 声明 + `{name}`/`{children}` 投影 + 消费者命名属性 `name={...}` 投影。`@slot name: Type` 类型约束、`slot="name"`、`@slot(name)` 均已删除。

```omni
import omniview;

component Card {
    @slot header { <span>Default Header</span> }
    @slot { <p>Default body</p> }

    view {
        <div class: card>
            <div>{header}</div>
            <div>{children}</div>
        </div>
    }
}

component SlotDemo {
    view {
        <Card header={<h3>Custom</h3>}>custom body</Card>
    }
}
```

### 1.5 域 5 — Stores → [08-Store 状态管理](08-stores.md)

`store` 顶层声明：`signal`（状态）+ `getter`（派生）+ `action`（唯一修改途径）。组件 `@store` 引用后按 `Store.field` 访问。`$counter` 存取与 store 内 `fn` 成员已删除。

```omni
import omniview;

store CounterStore {
    signal count: i32 = 0;
    getter doubled: i32 = count * 2;
    action inc() { count = count + 1; }
}

component StoreDemo {
    @store CounterStore;
    view {
        <button @onPress={() => { CounterStore.inc(); }}>
            {CounterStore.count} ({CounterStore.doubled})
        </button>
    }
}
```

### 1.6 域 6 — Control Flow → [06-模板语法](06-templates.md)

`@if` 条件渲染 + `@for item in items key={expr}` 列表渲染（对象项建议提供 `key=` 保持身份）+ `@empty` 空态。`@for` 默认文档流布局；滚动列表加 `[scroll]`。

```omni
import omniview;

component ForDemo {
    signal items: Array<String> = ["a", "b", "c"];
    signal show: bool = true;

    view {
        @if show {
            <ul>
                @for item in items key={item} {
                    <li>{item}</li>
                }
                @empty {
                    <li>none</li>
                }
            </ul>
        }
    }
}
```

### 1.7 域 7 — Events → [07-事件与绑定](07-events-and-bindings.md)

元素上 `@onEventName={fn}`（或 `@onEventName={() => { ... }}` 内联）；处理器参数 `(e)` 携带类型化事件字段（编译器自动构建最小事件对象）。`on:` 块形式已删除。事件内联 fn 是 `=>` 唯一允许位置。

```omni
import omniview;

component EventDemo {
    signal clicks: i32 = 0;

    fn onClick(e) {
        clicks = clicks + 1;
    }

    view {
        <button @onClick={onClick}>clicked {clicks}</button>
    }
}
```

### 1.8 域 8 — Expressions & Statements → [03-表达式与运算符](03-expressions-and-operators.md) / [04-函数与控制流](04-functions-and-control-flow.md)

管道闭包 `|x| ...` 是唯一通用闭包形式；`filter`/`map`/`count` 是语言方法。`=>` 仅用于事件内联 fn。

```omni
import omniview;

component ExprDemo {
    signal items: Array<i32> = [1, 2, 3, 4, 5];
    computed evens: Array<i32> = items.filter(|x| x % 2 == 0);
    computed evenCount: i32 = items.count(|x| x % 2 == 0);

    view {
        <p>{evens.len()} / {evenCount} even</p>
    }
}
```

### 1.9 域 9 — Styling → [09-样式系统](09-styling.md)

`class: name` 静态类 + `class: { name: cond }` 条件类映射 + `style: { prop: expr }` 内联样式 + `style { ... }` 块。类名仅允许 `[a-zA-Z0-9_-]`；样式值注入安全是硬约束。

```omni
import omniview;

component StyleDemo {
    signal active: bool = false;

    view {
        <div class: card class: { active: active } style: { color: "#333" }>
            styled
        </div>
    }
    style {
        .card { padding: 12px; border-radius: 8px; }
        .card.active { background: #eef2ff; }
    }
}
```

### 1.10 域 10 — a11y → [06-模板语法](06-templates.md) / [15-语法规范-UI框架](15-grammar-view.md)

`a11y: { label/role/hidden/live/alt }` 元素属性块（与 `style: { ... }` 同形态）。原生 `aria-*` 属性已删除（Flutter 静默丢失可访问名称）。

```omni
import omniview;

component A11yDemo {
    signal hidden: bool = false;

    view {
        <div a11y: { label: "status", role: "img", hidden: hidden }>
            <button a11y: { label: "Close" } @onPress={() => { hidden = true; }}>×</button>
        </div>
    }
}
```

---

## 2. 事件名表（canonical + 别名）

**单一事实来源**：`crates/omnilang-core/src/codegen/shared/event_names.rs` 的共享 canonical 表（v2 Task 4.2 起五后端共用，`dblclick` 类漂移 bug 的根治）。共 **54 个 canonical 事件名**。

**解析规则**：`on` 前缀可选、大小写不敏感；别名族（press/click/tap 等）折叠到同一 canonical 名。**未知事件名 → warning（E0006）+ 最近合法建议**（如 `@onTotallyBogus` → 建议 `@onTap`/`@onClick`/`@onPress`），非 error。

| 族 | @on 拼写（建议写法） | canonical（DOM） | 别名 |
|------|----------------------|------------------|------|
| 点击 | `@onPress` | `click` | `onClick`, `onTap`, `press`, `tap` |
| 双击 | `@onDoubleClick` | `dblclick` | `onDblClick`, `doubleclick` |
| 长按 | `@onLongPress` | `longpress` | —（Flutter-only 便利事件，无 DOM 事件） |
| 悬停 | `@onHover` | `hover` | —（无 DOM 事件） |
| 表单 | `@onChange` / `@onInput` / `@onToggle` / `@onSubmit` / `@onFocus` / `@onBlur` / `@onSelect` / `@onResize` | `change` / `input` / `toggle` / `submit` / `focus` / `blur` / `select` / `resize` | — |
| 键盘 | `@onKeyDown` / `@onKeyUp` / `@onKeyPress` | `keydown` / `keyup` / `keypress` | — |
| 鼠标 | `@onMouseEnter` / `@onMouseLeave` / `@onMouseDown` / `@onMouseUp` / `@onMouseMove` / `@onMouseOver` / `@onMouseOut` | 同名小写 | — |
| 指针 | `@onPointerDown` / `@onPointerUp` / `@onPointerMove` / `@onPointerCancel` | 同名小写 | — |
| 滚动/滚轮 | `@onScroll` / `@onWheel` | `scroll` / `wheel` | — |
| 触摸 | `@onTouchStart` / `@onTouchEnd` / `@onTouchMove` / `@onTouchCancel` | 同名小写 | `onTouchcancel` 等大小写变体 |
| 右键 | `@onContextMenu` | `contextmenu` | — |
| 拖放 | `@onDrag` / `@onDragStart` / `@onDragEnd` / `@onDragOver` / `@onDragLeave` / `@onDrop` | 同名小写 | — |
| 剪贴板 | `@onCopy` / `@onCut` / `@onPaste` | 同名小写 | — |
| 媒体 | `@onPlay` / `@onPause` / `@onEnded` / `@onLoad` / `@onError` | 同名小写 | — |
| 动画 | `@onAnimationStart` / `@onAnimationEnd` / `@onAnimationIteration` | 同名小写 | `onAnimationend` 等拼写变体统一为 camelCase |
| 过渡 | `@onTransitionEnd` / `@onTransitionRun` / `@onTransitionStart` / `@onTransitionCancel` | 同名小写 | `onTransitionend` 等拼写变体 |

**自定义事件**（组件间通信）：`event onXxx: Type;` 声明 + `emit onXxx(value)` 发射，父组件在组件实例上 `@onXxx={fn}` 绑定（不校验 DOM 表）。

**已删除**：事件**名**无删除——被删除的是 `on: { ... }` 块**语法**（见 §3）。

---

## 3. 删除语法表（verbatim 硬诊断消息）

以下语法均已删除（ADR-053、v2 设计 D1-D4）。删除政策：**绝无 parse-and-ignore**——每项产生硬诊断（编译错误）+ 迁移提示。消息为编译器 verbatim 输出（docs-gate 以本表前缀族判定文档是否教授已删除语法）。

| 已删除语法 | verbatim 诊断消息 | 替代 |
|------------|--------------------|------|
| `states: { ... }` 块 | `states: 已删除（ADR-053），用 @motion + 触发器替代` | `@motion` 定义 + `@enter`/`@hover`/… 触发器 |
| `on: { ... }` 块 | `on: 已删除，用 @onXxx={...}` | `@onXxx={...}` 元素属性 |
| `paused:` / `@paused` | `paused 已删除，用 @if 条件渲染替代` | `@if` 条件渲染 |
| `@slot name: Type`（类型约束/数据参数） | `类型约束与数据参数已删除；slot = 父传内容` | `@slot name { ... }` 纯内容插槽 |
| `slot="name"` 属性消费 | `改用命名 props：name={...}` | 命名属性 `name={...}` |
| `@slot(name)` 指令形式 | `改用 body 级声明 @slot name=... ` | body 级 `@slot name { ... }` |
| `value={sig}` 语义绑定（D2） | `'value={sig}' 语义绑定形式已删除，用 bind:value 双向绑定，或普通 value prop 单向` | `bind:value={sig}`（双向）/ `value={expr}`（单向 prop） |
| `ref.setValue(x)` / `ref.value = x` 命令式写 | `命令式写已删除，用 bind:value 双向绑定` | `bind:value` 双向绑定 |
| `capturePointer(id)` / `releasePointer(id)` 参数（D3） | `capturePointer 无参数；多指针锁定不支持` | 无参 `capturePointer()` / `releasePointer()` |
| `ref.style.*` / DOM API（innerHTML/querySelector 等） | `use style:{...} binding with signals instead`（含逐 API 迁移提示，如 `use ref.measure() instead`） | `style: { ... }` 信号绑定 / `ref.measure()` |
| 通用表达式中的 `x => ...` 箭头 | `通用闭包用管道 \|x\|；箭头仅用于事件内联` | 管道闭包 `\|x\| ...` |
| store 内 `fn` 成员 | `store 内 fn 成员不允许；用 getter 或 action` | `getter` / `action` |
| `@hero`（ADR-032/053 移除） | `hero transitions are not supported — '@hero' was removed in the semantic DSL (ADR-032/053); use @enter/@hover/@press/@focus/@in-view/@scroll/@drag-* triggers instead` | 8 触发器之一 |
| 原生 `aria-*` 属性（域 10） | `aria-* 原生属性已删除：'aria-label' 仅 web 透传、Flutter 丢失可访问名称；请迁移到 a11y: { label/role/hidden/live/alt } 语义块` | `a11y: { ... }` 语义块 |
| v1 `@themes light, dark;` 分号形式 | `expected '{' after '@themes'（v2 语法：@themes { light, dark }）` | `@themes { light, dark }` |
| 内联完整体动画 `@enter={{ over: ..., from: {...}, to: {...} }}` | 自然解析错误（E0001；v2 设计记录，未实现） | map 形式 `@enter={{ opacity: 0 }}` |
| `theme = "dark"` 语句 / `@native(...)` token | 自然解析错误（虚构语法，零用户，不设自定义消息——响亮失败即可） | 写 `@theme` 信号 + action / 使用标准 token 类型 |

---

## 4. 错误码速查（E_* 前缀 + 含义 + 修复方向）

诊断码注册于 `crates/omnilang-core/src/error.rs::DiagnosticKind`。

### 编号码（E0000–E0011）

| 码 | 含义 | 典型场景 / 修复方向 |
|----|------|----------------------|
| `E0000` | FFI 错误 | WASM FFI 边界序列化失败——检查跨边界类型契约 |
| `E0001` | 解析错误 | 语法错误（`expected X — found Y`）——对照 §1 示例或 BNF 规范（[14](14-grammar-lang.md)/[15](15-grammar-view.md)）修正写法 |
| `E0002` | 重复符号 | 同名声明冲突——重命名或删除重复声明 |
| `E0003` | 类型不匹配 | `expected X, got Y`——检查表达式类型与目标类型 |
| `E0004` | 非法类型 | 类型不存在或拼写错误 |
| `E0005` | 未定义引用 | 名称未声明——检查作用域、import、store 引用 |
| `E0006` | 验证错误 | 规则验证：类名非法字符（`类名仅允许字母数字下划线连字符`）、a11y 校验（如 hidden 使可聚焦元素不可见）、未知事件名 warning（带最近合法建议）——按消息指示修正 |
| `E0007` | 动画验证错误 | motion 定义/触发器问题——检查 `@motion` 语法与白名单属性 |
| `E0008` | store 字段错误 | store 字段访问/声明问题 |
| `E0009` | store action 错误 | store action 调用/声明问题 |
| `E0010` | 语义类型错误 | 类型系统语义检查 |
| `E0011` | 缺少必需 prop | 组件实例缺必需 prop——按组件声明补齐 |

### 命名码（E_*）

| 码 | 含义 | 修复方向 |
|----|------|----------|
| `E_VISIBILITY` | 可见性错误 | `pub` 访问违规——检查模块可见性 |
| `E_IMPORT` | 导入错误 | import 目标不存在/无法解析——检查路径与导出名 |
| `E_BOUNDARY` | 边界错误 | 跨模块边界约束违反 |
| `E_CYCLE` | 循环检测 | import/依赖环——打破循环依赖 |
| `E_CODEGEN` | 代码生成错误 | 目标后端不支持该构造（如含语句 BlockExpr）——改用目标支持的形式 |
| `E_CLOSURE_ARROW` | 闭包语法错误 | `通用闭包用管道 \|x\|；箭头仅用于事件内联`——通用位置改用管道闭包 |
| `E_ARRAY_METHOD` | 数组方法错误 | `count 需要一个谓词闭包`——`count()`/`filter()`/`map()` 需管道闭包参数 |
| `E_EFFECT_DEPS_MISSING` | effect 依赖缺失 | `effect 依赖漏掉：Store.field`——声明 deps 必须覆盖 body 读取的 store 字段 |
| `E_PAUSED_REMOVED` | paused 已删除 | `paused 已删除，用 @if 条件渲染替代`（见 §3） |
| `E_SLOT_REMOVED` | slot 形式已删除 | `类型约束与数据参数已删除；slot = 父传内容`（见 §3） |
| `E_STATES_REMOVED` | `states:` 块已删除 | `states: 已删除（ADR-053），用 @motion + 触发器替代`（见 §3） |
| `E_ON_REMOVED` | `on:` 块已删除 | `on: 已删除，用 @onXxx={...}`（见 §3） |
| `E_VALUE_BINDING_REMOVED` | 值绑定已删除 | `'value={sig}' 语义绑定形式已删除，用 bind:value 双向绑定，或普通 value prop 单向`（见 §3） |
| `E_LINT` | lint 规则错误 | 规则 ID 见 `LintRuleId`：`undefined-ref`、`type-mismatch`、`store-field-direct-mutation`、`prop-immutability`、`computed-purity`、`missing-return`、`dead-code`、`unused-import`、`invalid-event-field`、`store-action-no-event-data`、`for-key`（对象项 @for 无 key → warning） |

### 警告码（W*）

| 码 | 含义 | 说明 |
|----|------|------|
| `W_OMNIVIEW_MARKER` | OmniView 降级警告 | 生成代码带降级注释——如不可映射 design 值 |
| `W0001` | motion 警告 | 如 Flutter `steps()` 回退 `Curves.linear` |
