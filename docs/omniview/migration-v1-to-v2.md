# v1 → v2 迁移指南

> 语义级 DSL v2 破坏性变更清单——每条删除语法产生硬诊断 + 迁移提示（「诊断即文档」：编译器即迁移工具，每处删除自带替换指引，绝无 parse-and-ignore）。
> 完整删除表见 [00-速查表 §3](00-cheatsheet.md)（verbatim 诊断消息的权威来源）；语法规范见 [14-语法规范-核心语言](14-grammar-lang.md) 与 [15-语法规范-UI框架](15-grammar-view.md)。
> 迁移状态跟踪：`docs/superpowers/specs/semantic-universe.md`（实现状态列）。

## 删除语法总表（17 项）

| 已删除语法 | v1 写法 | v2 写法 | 诊断码 |
|-----------|---------|---------|--------|
| `states: { ... }` 块 | `states: { ... }`（组件内状态动画块） | 顶层 `@motion name { from { ... } to { ... } over: ... }` 定义 + 8 触发器绑定 | `E_STATES_REMOVED` |
| `on: { ... }` 块 | `on: { (e) => { ... } }`（旧 BNF：`'on' ':' '{' params '=>' block '}'`） | 元素属性 `@onXxx={fn}` 或 `@onXxx={() => { ... }}` | `E_ON_REMOVED` |
| `paused:` / `@paused` | `@paused` 属性 / 动画 `paused:` 播放参数 | `@if` 条件渲染包裹动画元素 | `E_PAUSED_REMOVED` |
| `@slot name: Type`（类型约束/数据参数） | `@slot header: String` | `@slot name { 回退内容 }` 纯内容插槽；数据走 prop/信号 | `E_SLOT_REMOVED` |
| `slot="name"` 属性消费 | `<Card slot="header">...</Card>` | 命名属性投影 `<Card header={...}>` | `E_SLOT_REMOVED` |
| `@slot(name)` 指令形式 | `@slot(header)` | body 级声明 `@slot header { ... }` | `E_SLOT_REMOVED` |
| `value={sig}`（带 `@ref`，v1 双向形状） | `<input @ref={el} value={text} />`（双向） | `bind:value={sig}`（双向）/ `value={getter()}` 或 `value={computed}`（单向显示）；无 `@ref` 的 `value={sig}` 即普通单向 prop（合法，v4 5.4） | `E_VALUE_BINDING_REMOVED` |
| `ref.setValue(x)` / `ref.value = x` 命令式写 | `inputEl.setValue("x")` / `inputEl.value = "x"` | `bind:value={sig}` 双向绑定（读 `ref.value` 仍合法） | `E0006`（validate_ref_ops） |
| `capturePointer(id)` / `releasePointer(id)` 参数（D3） | `capturePointer(0)` / `releasePointer(0)` | 无参 `capturePointer()` / `releasePointer()` | `E0006`（validate_ref_ops） |
| `ref.style.*` / DOM API（innerHTML/querySelector 等） | `el.style.color = "red"` / `el.innerHTML = ...` 等 | `style: { ... }` 信号绑定 / `{value}` 插值 / `class: ...` / `ref.measure()` | `E0006`（validate_ref_ops） |
| 通用表达式中的 `x => ...` 箭头 | `items.filter(x => x > 0)` | 管道闭包 `items.filter(\|x\| x > 0)`；`=>` 仅用于事件内联 | `E_CLOSURE_ARROW` |
| store 内 `fn` 成员 | `store S { fn helper() { ... } }` | `getter`（派生值）/ `action`（状态修改） | `E0001`（解析错误） |
| `@hero`（ADR-032/053 移除） | `@hero` 属性 | 8 触发器之一（`@enter`/`@hover`/`@press`/`@focus`/`@in-view`/`@scroll`/`@drag-*`） | `E0001`（解析错误） |
| 原生 `aria-*` 属性（域 10） | `<div aria-label="status">` | `a11y: { label/role/hidden/live/alt }` 语义块 | `E0006`（validate_a11y） |
| v1 `@themes light, dark;` 分号形式 | `@themes light, dark;` | `@themes { light, dark }` | `E0001`（解析错误） |
| 内联完整体动画 `@enter={{ over: ..., from: {...}, to: {...} }}` | `@enter={{ over: 300ms, from: {...}, to: {...} }}` | map 形式 `@enter={{ opacity: 0 }}`；完整体用顶层 `@motion` + 触发器引用 | `E0001`（解析错误） |
| `theme = "dark"` 语句 / `@native(...)` token | `theme="dark"`（元素属性）/ `@native(...)` | `@themes` 声明 + `@theme` 信号 + action / 标准 token 类型 | warning（E0006，validate_props）/ `E0001`（自然解析错误） |

## 迁移路径

1. 编译项目 → 收集全部 `E_*_REMOVED` 等诊断（每诊断自带迁移提示）
2. 按上表逐项替换
3. 重编译 → 0 错误
4. 浏览器验证（三目标：OmniView / React / Flutter）

## 逐项迁移详情

### 1. `states: { ... }` 块 → `E_STATES_REMOVED`

- **诊断（verbatim）**：`states: 已删除（ADR-053），用 @motion + 触发器替代`
- **v1 写法**：`states: { ... }`（组件内状态动画块）
- **v2 写法**：状态动画提升为顶层 `@motion name { from { ... } to { ... } over: ... }` 定义，再在元素上绑定 8 触发器之一（`@enter`/`@hover`/`@press`/`@focus`/`@in-view`/`@scroll`/`@drag-horizontal`/`@drag-vertical`）
- **提示**：动画域完整文档见 [11-运动与动画](11-motion-animation.md)

### 2. `on: { ... }` 块 → `E_ON_REMOVED`

- **诊断（verbatim）**：`on: 已删除，用 @onXxx={...}`
- **v1 写法**：`on: { ... }` 块（旧 BNF：`event_handler ::= 'on' ':' '{' params '=>' block '}'`）
- **v2 写法**：事件处理改元素属性 `@onXxx={fn}`（组件 fn 引用）或 `@onXxx={() => { ... }}`（内联）——`=>` 箭头只允许出现在事件内联位置（见第 11 项）
- **提示**：事件名与别名表见 [00-速查表 §2](00-cheatsheet.md)；完整文档见 [07-事件与绑定](07-events-and-bindings.md)

### 3. `paused:` / `@paused` → `E_PAUSED_REMOVED`

- **诊断（verbatim）**：`paused 已删除，用 @if 条件渲染替代`
- **v1 写法**：`@paused` 属性 / 动画 `paused:` 播放参数
- **v2 写法**：用 `@if` 条件渲染包裹动画元素——`@if show { <div @enter={fadeIn}>...</div> }`
- **提示**：`@if` 语法见 [06-模板语法](06-templates.md)

### 4. `@slot name: Type`（类型约束/数据参数）→ `E_SLOT_REMOVED`

- **诊断（verbatim）**：`类型约束与数据参数已删除；slot = 父传内容`
- **v1 写法**：`@slot header: String`（带类型约束/数据参数）
- **v2 写法**：slot 语义收敛为「父传内容」——`@slot header { <span>Default Header</span> }` 纯内容插槽；需要传数据时改用普通 prop 或共享 store
- **提示**：见 [06-模板语法](06-templates.md) 与 [15-语法规范-UI框架 §1.8](15-grammar-view.md)

### 5. `slot="name"` 属性消费 → `E_SLOT_REMOVED`

- **诊断（verbatim）**：`改用命名 props：name={...}`
- **v1 写法**：`<Card slot="header">...</Card>`
- **v2 写法**：命名属性投影 `<Card header={...}>...</Card>`（默认插槽用子节点 `<Card>custom body</Card>`）
- **提示**：消费端示例见 [00-速查表 §1.4](00-cheatsheet.md)

### 6. `@slot(name)` 指令形式 → `E_SLOT_REMOVED`

- **诊断（verbatim）**：`改用 body 级声明 @slot name=... `
- **v1 写法**：`@slot(header)`（指令形式）
- **v2 写法**：body 级声明 `@slot header { ... }`（或裸 `@slot` 声明默认插槽）
- **提示**：与第 4 项同族——三种遗留 slot 形式全部收敛为 body 级 `@slot name { ... }`

### 7. `value={sig}`（带 `@ref`，v1 双向形状）→ `E_VALUE_BINDING_REMOVED`

- **诊断（verbatim）**：`'value={sig}' 双向绑定形式已删除，双向绑定请用 bind:value；单向显示请用 value={getter()}（计算表达式）或 value={computed}`
- **v1 写法**：`<input @ref={el} value={text} />`（带 `@ref` 时的双向绑定形状）
- **v2 写法**：双向 → `bind:value={sig}`；单向显示 → `value={getter()}` 或 `value={computed}`；元素不带 `@ref` 时 `value={sig}` 即普通单向 prop——合法，无需迁移（v4 5.4 判别器按 `@ref` 区分）
- **提示**：v1 隐式 auto-bind（`<input>` 按声明序自动绑定字符串信号，ADR-046 时代行为）已随 v2 删除——**行为级变更、无诊断码**，v2 起输入值一律显式 `bind:value={sig}`。见 [07-事件与绑定](07-events-and-bindings.md)

### 8. `ref.setValue(x)` / `ref.value = x` 命令式写 → `E0006`（validate_ref_ops）

- **诊断（verbatim）**：`命令式写已删除，用 bind:value 双向绑定`
- **v1 写法**：`inputEl.setValue("x")` / `inputEl.value = "x"`
- **v2 写法**：`bind:value={sig}` 双向绑定（声明式写路径）；**读** `ref.value` 仍合法（`RefOpIR::Value`）
- **提示**：Ref 系统域见 [00-速查表 §1.1](00-cheatsheet.md) 与 [05-组件](05-components.md)

### 9. `capturePointer(id)` / `releasePointer(id)` 参数（D3）→ `E0006`（validate_ref_ops）

- **诊断（verbatim）**：`capturePointer 无参数；多指针锁定不支持` / `releasePointer 无参数；多指针锁定不支持`
- **v1 写法**：`capturePointer(0)` / `releasePointer(0)`（id 参数是 DOM pointerId 平台形态泄漏）
- **v2 写法**：无参 `capturePointer()` / `releasePointer()`（语义：锁定后续指针事件到本元素）；多指针显式锁定不支持

### 10. `ref.style.*` / DOM API（innerHTML/querySelector 等）→ `E0006`（validate_ref_ops）

- **诊断（verbatim，逐 API 迁移提示）**：

| 被调 API（v1） | verbatim 迁移提示 |
|----------------|-------------------|
| `ref.style` | `use style:{...} binding with signals instead` |
| `ref.innerHTML` / `ref.textContent` | `use text interpolation {value} instead` |
| `ref.addEventListener` | `use @on* event binding syntax instead` |
| `ref.querySelector` / `ref.parentElement` | `violates component encapsulation` |
| `ref.classList` | `use class: binding syntax instead` |
| `ref.getBoundingClientRect` | `use ref.measure() instead` |
| `ref.offsetWidth` / `ref.offsetHeight` | `use ref.measure().width instead` / `use ref.measure().height instead` |

- **v2 写法**：`style: { ... }` 信号绑定 / `{value}` 文本插值 / `class: ...` 绑定 / `@on*` 事件 / `ref.measure()` 测量
- **提示**：DOM 专属 API 会破坏跨目标编译（Flutter 无 DOM）——跨目标安全是删除原因。样式与类绑定见 [09-样式系统](09-styling.md)

### 11. 通用表达式中的 `x => ...` 箭头 → `E_CLOSURE_ARROW`

- **诊断（verbatim）**：`通用闭包用管道 |x|；箭头仅用于事件内联`
- **v1 写法**：`items.filter(x => x > 0)`
- **v2 写法**：管道闭包 `items.filter(|x| x > 0)`；`=>` 仅用于事件内联 `@onPress={() => { ... }}`
- **提示**：管道闭包是唯一通用闭包形式——见 [03-表达式与运算符](03-expressions-and-operators.md) 与 [00-速查表 §1.8](00-cheatsheet.md)

### 12. store 内 `fn` 成员 → `E0001`（解析错误）

- **诊断（verbatim）**：`store 内 fn 成员不允许；用 getter 或 action`
- **v1 写法**：`store S { fn helper() { ... } }`
- **v2 写法**：派生值 → `getter`；状态修改 → `action`（store 成员集统一为 `signal | getter | action`）
- **提示**：见 [08-Store 状态管理](08-stores.md)

### 13. `@hero`（ADR-032/053 移除）→ `E0001`（解析错误）

- **诊断（verbatim）**：`hero transitions are not supported — '@hero' was removed in the semantic DSL (ADR-032/053); use @enter/@hover/@press/@focus/@in-view/@scroll/@drag-* triggers instead`
- **v1 写法**：`@hero` 属性
- **v2 写法**：8 触发器之一（`@enter`/`@hover`/`@press`/`@focus`/`@in-view`/`@scroll`/`@drag-*`）绑定 `@motion`
- **提示**：见 [11-运动与动画](11-motion-animation.md)

### 14. 原生 `aria-*` 属性（域 10）→ `E0006`（validate_a11y）

- **诊断（verbatim）**：`aria-* 原生属性已删除：'{name}' 仅 web 透传、Flutter 丢失可访问名称；请迁移到 a11y: { label/role/hidden/live/alt } 语义块`（每属性一条，`{name}` 为属性名）
- **v1 写法**：`<div aria-label="status">`
- **v2 写法**：`<div a11y: { label: "status" }>`（label/role/hidden/live/alt 五字段语义块，与 `style: { ... }` 同形态）
- **提示**：见 [00-速查表 §1.10](00-cheatsheet.md) 与 [06-模板语法](06-templates.md)

### 15. v1 `@themes light, dark;` 分号形式 → `E0001`（解析错误）

- **诊断（verbatim）**：`expected '{' after '@themes'（v2 语法：@themes { light, dark }）`
- **v1 写法**：`@themes light, dark;`
- **v2 写法**：`@themes { light, dark }`（块形式）
- **提示**：设计系统域见 [00-速查表 §1.3](00-cheatsheet.md) 与 [10-设计系统](10-design-system.md)

### 16. 内联完整体动画 `@enter={{ over: ..., from: {...}, to: {...} }}` → `E0001`（解析错误）

- **诊断（verbatim）**：自然解析错误（E0001；v2 设计记录，未实现）
- **v1 写法**：`@enter={{ over: 300ms, from: {...}, to: {...} }}`（内联完整体）
- **v2 写法**：map 形式 `@enter={{ opacity: 0, translateY: 10 }}`（简单属性内联）；完整体用顶层 `@motion` 定义 + 触发器引用 `@enter={fadeIn}`
- **提示**：见 [11-运动与动画](11-motion-animation.md)

### 17. `theme = "dark"` 语句 / `@native(...)` token → warning / `E0001`

- **`theme="..."`（已知 DOM 标签，v4 warning）**：`theme="..."` 是已删除的虚构语法（主题切换请用 @themes 声明 + @theme 信号绑定）——此属性在已知 DOM 标签上无任何效果（verbatim，validate_props.rs）。注：v4 修订前按「自然解析错误」裁决，实测为静默惰性属性——已知 DOM 标签发 warning，自定义组件上为合法 prop（豁免不误伤）
- **v1 写法**：`theme="dark"`（元素属性）/ `@native(...)` token
- **v2 写法**：`@themes { ... }` 声明主题集合 + `signal mode: String @theme = "light"` 标记当前主题信号，切换主题 = 普通 action 写信号；`@native(...)` 用标准 token 类型（`token primary: color = ...`）
- **提示**：主题接线见 [00-速查表 §1.3](00-cheatsheet.md)；`@native` 为虚构语法零用户，自然解析错误（E0001）响亮失败即可

## 诊断码索引

| 诊断码 | 类型 | 对应删除项 |
|--------|------|------------|
| `E_STATES_REMOVED` | error | 1. `states: { ... }` 块 |
| `E_ON_REMOVED` | error | 2. `on: { ... }` 块 |
| `E_PAUSED_REMOVED` | error | 3. `paused:` / `@paused` |
| `E_SLOT_REMOVED` | error | 4-6. slot 三形式（类型约束 / `slot=` 属性 / `@slot()` 指令） |
| `E_VALUE_BINDING_REMOVED` | error | 7. `value={sig}` 双向形状（带 `@ref`） |
| `E_CLOSURE_ARROW` | error | 11. 通用表达式 `=>` |
| `E0006`（验证错误） | error / warning | 8-10. ref 命令式写与 DOM API（validate_ref_ops）；14. `aria-*`（validate_a11y）；17. `theme="..."`（validate_props，warning） |
| `E0001`（解析错误） | error | 12. store `fn`；13. `@hero`；15. `@themes` 分号形式；16. 内联完整体动画；17. `@native` |

> 完整错误码速查（编号码 E0000-E0011 / 命名码 / 警告码）见 [00-速查表 §4](00-cheatsheet.md)。

## v2 目标形态完整示例

以下文件一次性覆盖上述主要替换（`bind:value` / `@slot` 纯内容插槽 / `@motion` + 触发器 / `@themes` + `@theme` 信号 / `@if` 条件渲染 / `a11y:` 块 / store `signal`+`action`）——可直接复制为独立文件编译：

```omni
import omniview;

@motion fadeIn {
    from { opacity: 0, translateY: 10 }
    to   { opacity: 1, translateY: 0 }
    over: 300ms
    easing: ease-out
}

design AppDesign {
    @themes { light, dark }
    @default-theme light;
    token primary: color = "#6366f1" dark "#a5b4fc";
}

store ThemeStore {
    signal mode: String @theme = "light";
    action toggle() { mode = if mode == "light" { "dark" } else { "light" }; }
}

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

component App {
    @store ThemeStore;
    ref inputEl: Element;
    signal text: String = "hello";
    signal show: bool = true;
    signal clicks: i32 = 0;

    view {
        <div a11y: { label: "demo" } @enter={fadeIn}>
            <input @ref={inputEl} bind:value={text} />
            <Card header={<h3>Custom</h3>}>custom body</Card>
            <button @onPress={() => { clicks = clicks + 1; }}>clicked {clicks}</button>
            @if show { <p>conditional</p> }
            <p>theme: {ThemeStore.mode}</p>
        </div>
    }
}
```
