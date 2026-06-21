# OmniView Compiler 语法特性 vs learn-omniview 项目使用情况对照

> 基于 57 个 .omni 文件的全面检查 (2026-06-20)

---

## ✅ 已经使用的语法特性

### 模块系统
- `import omniview;` ✓ (所有组件文件)
- `import { Name } from "path"` ✓ (跨文件组件导入)
- `import type { Type } from "path"` ✓ (main.omni)
- `import "path" as Alias` ✓ (main.omni)
- `export { Name } from "path"` ✓ (lib.omni, reexports.omni)
- `pub fn` / `pub struct` / `pub type` ✓ (lib/ 下所有文件)
- **Stdlib 模块所有函数全部已使用** ✓

### 组件声明
- `component Name { ... }` ✓
- `prop name: Type [= default]` ✓
- `signal name: Type = expr` ✓
- `const name = value` ✓ (StatsDashboard, constants.omni, SettingsPage)
- `computed name: Type = expr` ✓
- `event name: Type` ✓
- `event name: Type?` (optional event) ✓ (Timer.omni)
- `ref name: Type` ✓ (NotificationsPanel, StatsDashboard)
- `@store StoreName` ✓
- `action name(params) [async] [-> Type] { body }` ✓
- `@mount { body }` ✓
- `@unmount { body }` ✓
- `@effect(dep1, dep2) { body }` ✓
- `view { ... }` ✓
- `style { .class { ... } }` ✓

### 语句
- `let name = expr` ✓ / `let mut name = expr` ✓ / `let name: Type = expr` ✓
- `if/else if/else` ✓ / `while` ✓ / `for-in` ✓
- `return` ✓ / `break` ✓ / `continue` ✓
- `print` ✓ / `println` ✓
- `try/catch/finally` ✓ (error_handling.omni)
- `throw` ✓ (error_handling.omni)

### 表达式
- 字面量 (int, float, string, bool, null) ✓
- 数组字面量 ✓ / 结构体字面量 ✓ / 映射字面量 ✓
- 十六进制 0xFF ✓ / 八进制 0o77 ✓ / 二进制 0b1010 ✓
- 科学计数法 6.022e23 ✓
- 反引号字符串 ✓ / 单引号字符串 ✓
- 一元运算符 (!, -) ✓
- 二元运算符 (+, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||) ✓
- 复合赋值 (+=, -=, *=, /=, %=) ✓ (StatsDashboard)
- 空值合并 `??` ✓ (NullishDemo)
- 函数调用 / 方法调用 / 字段访问 ✓
- 索引访问 `arr[0]` ✓ (StatsDashboard)
- If 表达式 ✓ / Match 表达式 ✓ (api.omni, validation.omni)
- 闭包 `|params| expr` ✓
- async/await ✓
- tagged union 变体构造 `Ok { data: data }` ✓ (api.omni)
- 类型别名 `type Name = Type` ✓ / `pub type` ✓
- `struct nominal` ✓ (types.omni)

### View 模板
- `<div>` 等 HTML 标签 ✓ (30+ 标签)
- `<a>` / `<select>` / `<option>` ✓ (FormControlsDemo)
- `<form>` / `<input>` / `<textarea>` / `<button>` ✓
- `<details>` / `<summary>` ✓ (EventsDemo)
- `<progress>` / `<meter>` / `<video>` / `<audio>` ✓ (EventsDemo)
- `<b>` / `<strong>` / `<i>` / `<mark>` / `<code>` / `<blockquote>` 等语义标签 ✓ (EventsDemo)
- `@if/@else if/@else` ✓ / `@for` ✓ / `@empty` ✓ / `@slot` ✓
- `class:` 绑定 ✓ / `class: { key: expr }` 绑定 ✓
- `style: { key: expr }` 内联样式 ✓
- `bind:value` ✓ / `bind:checked` ✓
- `ref:` 元素引用 ✓
- `testId` ✓ / `disabled` ✓ / `readonly` ✓ (FormPage)
- `data-*` / `aria-*` 属性 ✓ (FormPage)
- 事件处理器: onPress, onClick, onChange, onScroll, onFocus, onBlur, onMouseEnter, onMouseLeave, onKeyDown, onDoubleClick, onContextMenu, onDragStart, onDragOver, onDragEnd, onDrop, onTouchStart, onTouchEnd, onTouchMove, onWheel, onLoad, onError, onPlay, onPause, onEnded, onAnimationStart, onAnimationEnd, onAnimationIteration, onTransitionEnd, onToggle, onInput, onKeyUp, onCopy, onCut, onPaste ✓ (EventsDemo 全覆盖)
- `{fnName(args)}` 事件简写 ✓ (EventShorthand)
- `{store.action}` 事件引用 ✓ (EventShorthand)

### CSS 样式
- 标准 CSS 属性 ✓
- 裸 #hex 颜色 ✓ (StyleAdvanced)
- CSS 单位: px, rem, %, em, vw, vh, fr, pt, cm, mm, ms, deg, rad, grad, turn, vmin, svh, lvh, dvh, ch, ex, ic, lh, cqw, cqi ✓ (CssUnitsDemo)

### 标准库（全部已覆盖）
- std/array ✓ / std/string ✓ / std/math ✓ / std/console ✓
- std/http ✓ / std/json ✓ / std/storage ✓
- std/async ✓ / std/datetime ✓ / std/regex ✓ / std/encode ✓

---

## ❌ 语法特性未在 learn-omniview 中使用

### 一、语言表达式特性

#### 1. 三元表达式 `cond ? a : b`
**编译器支持**: AST 有 `TernaryExpr` 节点
**项目中**: 所有条件选择均使用 `if expr { a } else { b }` 形式
**位置**: 无任何文件使用

#### 2. 可选链 `obj?.field` 或 `obj?.method()`
**编译器支持**: 词法分析器有 `?.` token，AST 通过 FieldAccess/MethodCall 处理
**项目中**: NullishDemo 注释提到了 "?. + ?? 演示" 但代码中仅使用了 `??` 和 `String?` 类型标注，未见 `?.` 运算符的实际使用
**位置**: 无任何文件使用

#### 3. 模板字符串插值 `` `text ${expr} text` ``
**编译器支持**: AST 有 `TemplateString { parts: Vec<TemplatePart> }`
**项目中**: StringLiteralsDemo 使用了反引号字符串，但**不含** `${expr}` 插值语法，所有字符串插值均通过 `{expr}` 在 view 中实现
```omniview
// 使用的（无插值）：
signal backtickResult: String = `Hello "OmniView" world`;
// 未使用的（带插值）：
// signal x: String = `Hello ${name}`;
```

### 二、View 模板特性

#### 4. `@for item in list key={expr}` key 表达式
**编译器支持**: `ForDirective` 有 `key` 字段
**项目中**: `@for` 在多个文件中使用，但均**未指定 key** 表达式
```omniview
// 使用的：
@for row in rows { ... }
// 未使用的：
// @for row in rows key={row.id} { ... }
```

#### 5. `class: "string literal"` 字符串字面量 class 绑定
**编译器支持**: `ClassValue::StringLiteral(String)`
**项目中**: 所有 class 绑定都是 `class: name` (StyleRef) 或 `class: { key: expr }` (ClassMap)
```omniview
// 都用了，但缺少：
// <div class: "bg-red text-bold">...</div>
```

#### 6. Slot 内容投影——父传子（projection into @slot）
**编译器支持**: view 中 `<Component>children</Component>` 的 children 填充到 `@slot`
**项目中**: 
- `@slot` 定义在子组件中 ✓ (Modal, NotificationsPanel)
- 但所有 `<Component> ... </Component>` 调用中要么无子节点，要么只是 HTML 注释
- 没有**实际将 UI 内容投影到子组件的 @slot 中**
```omniview
// 使用的（仅默认内容）：
<Modal><!-- 空注释 --></Modal>
// 未使用的：
// <Modal>
//   <p slot="header">自定义标题</p>
//   <p>自定义 body</p>
// </Modal>
```

### 三、组件内部特性

#### 7. `computed` 块体形式 `computed name() -> Type { body }`
**编译器支持**: `ComputedDecl` 可以包含 `Block` (块体) 而非 `Expr`
**项目中**: 所有 computed 均使用单表达式形式 `computed name: Type = expr`
```omniview
// 使用的：
computed doubled: i32 = count * 2;
// 未使用的：
// computed summary() -> String {
//     let prefix = "Total: ";
//     return prefix + count.to_string();
// }
```

#### 8. 内联 store 声明 `store Name { ... }` 在 component 内部
**编译器支持**: 组件可以内嵌 `store Name { ... }` 定义
**项目中**: 所有 store 都是顶级定义（`stores/*.omni`），无 inline store
```omniview
// 未使用的：
component MyComp {
    store LocalStore {
        signal x: i32 = 0;
        action inc() { x = x + 1; }
    }
    @store LocalStore;
    // ...
}
```

### 四、CSS 单位——15 个已覆盖，8 个未覆盖

| CSS 单位 | 状态 | 位置 |
|----------|------|------|
| `s` (秒) | ❌ 未使用 | CssUnitsDemo 有 ms 但无 s |
| `vmax` | ❌ 未使用 | CssUnitsDemo 有 vmin 但无 vmax |
| `svw` | ❌ 未使用 | 有 svh 但无 svw |
| `lvw` | ❌ 未使用 | 有 lvh 但无 lvw |
| `dvw` | ❌ 未使用 | 有 dvh 但无 dvw |
| `cqh` | ❌ 未使用 | 有 cqw/cqi 但无 cqh |
| `cqb` | ❌ 未使用 | - |
| `cqmin` | ❌ 未使用 | - |
| `cqmax` | ❌ 未使用 | - |
| `rlh` | ❌ 未使用 | 有 lh 但无 rlh |

### 五、CSS 伪类选择器

#### 9. 伪类选择器 `:hover`, `:focus`, `:active` 等
**编译器支持**: `StyleRule` 有 `pseudo_class: Option<String>` 字段
**项目中**: 所有 style 规则都是 `.className { ... }` 裸选择器，**无任何伪类**
```css
// 使用的：
.btn { background: white; }
// 未使用的：
// .btn:hover { background: #eee; }
// .btn:focus { outline: 2px solid blue; }
```

### 六、标准库边缘

#### 10. `std/http::http_send()` —— 全参数 HTTP 发送
**编译器支持**: `http_send(request)` 函数
**项目中**: 仅使用了 `http_get` 和 `http_post`，均为简化版。`http_send`（支持自定义请求头、方法、body）未使用

---

## 汇总

| 类别 | 总数 | 已使用 | 未使用 |
|------|------|--------|--------|
| 语言表达式特性 | ~25 | 22 | 3 (ternary `?:`, optional chain `?.`, template interpolation `${}`) |
| View 模板特性 | ~15 | 12 | 3 (@for key, class:"string", slot projection) |
| 组件声明特性 | ~18 | 16 | 2 (computed block-body, inline store) |
| CSS 单位 | ~28 | 20 | 8 (s, vmax, svw, lvw, dvw, cqh, cqb, cqmin, cqmax, rlh - 实为10个) |
| CSS 伪类 | ~5 | 0 | 所有伪类 (`:hover`, `:focus`, `:active`, `:disabled`, `:checked`) |
| 标准库函数 | ~45 | 44 | 1 (http_send) |
| **合计** | **~136** | **~114** | **~22** |
