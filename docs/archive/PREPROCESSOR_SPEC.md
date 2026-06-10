# OmniView 大括号平衡预处理器规范

> **版本:** 1.0 (2026-06-05)  
> **状态:** Phase 0.0 编码依据  
> **依赖:** OmniLang `parse_inline_expr` / `parse_inline_block`

---

## 1. 目的

OmniView 语法文件只包含 UI 结构规则。`.omni` 源文件中的 `{...}` 表达式和语句块由预处理器提取、替换为占位符，再交给 OmniLang 解析。

---

## 2. 输入输出

**输入：** `.omni` 源文本（原始 bytes/str）

**输出：** `PreprocessorOutput`

```python
@dataclass
class PreprocessorOutput:
    processed_source: str        # 占位符替换后的源文本
    expressions: dict[str, str]  # placeholder_id (如 "__OMNI_EXPR_0__") → 原始表达式文本
    blocks: dict[str, str]       # placeholder_id (如 "__OMNI_BLOCK_0__") → 原始语句块文本
    position_map: dict[str, int] # placeholder_id → .omni 源中的起始字节偏移
```

---

## 3. 识别阶段：上下文分类

预处理器按以下优先级识别 `{...}` 区域的**上下文**：

| 上下文 | 判定规则 | 替换为 | OmniLang 解析方式 |
|--------|---------|--------|-----------------|
| **语句块** | `{` 前有 `)` 且属于 `fn`/`action`/`@mount`/`@unmount`/`@effect` 头 | `__OMNI_BLOCK_N__` | `parse_inline_block()` |
| **style 对象** | `style = { { ... } }` — 双花括号 | `__OMNI_STYLE_N__` | OmniView 直接解析（非 OmniLang 表达式） |
| **class_map** | `class = { class_map({ ... }) }` | `__OMNI_CLASSMAP_N__` | OmniView 直接解析（非 OmniLang 表达式） |
| **事件处理器** | `{ fn() { ... } }` 或 `{ fn_ref }` 或 `{ store.action }` | `__OMNI_EVENT_N__` | OmniView 解析结构 + OmniLang 解析 fn body |
| **view 块** | `view { ... }` | 保持原样 | OmniView Lark 解析 |
| **style 块** | `style { .class { ... } }` | 保持原样 | OmniView Lark 解析 |
| **表达式** | 所有其他 `{ ... }` | `__OMNI_EXPR_N__` | `parse_inline_expr()` |

---

## 4. 括号平衡算法

```
algorithm balance_braces(source, start_pos):
  // start_pos 指向 { 之后的第一个字符
  depth = 1
  i = start_pos
  while i < len(source) and depth > 0:
    if source[i] == '{' and not in_string and not in_comment:
      depth += 1
    elif source[i] == '}' and not in_string and not in_comment:
      depth -= 1
      if depth == 0:
        return source[start_pos:i]  // 大括号之间的内容
    elif source[i] == '"' and not in_comment:
      in_string = !in_string
    elif source[i] == '/' and source[i+1] == '/' and not in_string:
      in_line_comment = true  // 跳过至行尾
    elif source[i] == '/' and source[i+1] == '*' and not in_string:
      in_block_comment = true
    ...
    i += 1
  if depth > 0:
    raise PreprocessorError("Unmatched { at position ...")
```

---

## 5. 占位符格式

```
__OMNI_EXPR_{N}__    — 内联表达式（文本插值、属性值、@if 条件等）
__OMNI_BLOCK_{N}__   — 语句块（fn body、action body、lifecycle body）
__OMNI_EVENT_{N}__   — 事件处理器块
__OMNI_STYLE_{N}__   — style 对象（双花括号）
__OMNI_CLASSMAP_{N}__ — class_map 表达式
```

占位符 ID 从 0 开始递增，每种类型独立计数。

占位符必须在 Lark 语法中作为终端可用——它们被定义为匹配 `__OMNI_{TYPE}_\d+__` 的正则表达式。

---

## 6. 边界 Case（15+）

### 6.1 嵌套花括号
```omni
@if (a > 0 && (b < 10 || c == 3)) { ... }
```
`{` 在 `)` 之后 → 判定为 view block body，不是表达式区域。
表达式 `a > 0 && (b < 10 || c == 3)` 不包含 `{` → 无需括号平衡，直接提取。

### 6.2 字符串字面量中的花括号
```omni
<Input placeholder={"type {name} here"} />
```
字符串 `"type {name} here"` 内部的 `{name}` 在双引号内 → 不计入括号深度。
OmniView 属性值 `"type {name} here"` 本身不是 `{...}` 表达式区域。
⚠️ 修正：`{"type {name} here"}` 外层是表达式区域，内层 `{name}` 在字符串中不增加深度。

### 6.3 模板字符串
```omni
let msg = `Hello ${name}!`;
```
模板字符串中的 `${...}` 有自己的括号平衡。预处理器追踪 `${` 的深度，独立于 `{` 深度。

### 6.4 CSS 块中的花括号
```omni
style {
  .card {
    padding: 16px;
  }
}
```
最外层 `style { }` 和内部 `.card { }` 都是 OmniView 的 UI 结构 → 由 Lark 解析，不由预处理器提取。预处理器区分：`style` 关键字后的 `{` 属于 CSS 上下文。

### 6.5 View 块中的嵌套花括号
```omni
view {
  @if show {
    <Text>{name}</Text>
  }
}
```
`view {` 后的 `{` → view body 开始（Lark 解析）。  
`@if show {` → view block body（Lark 解析）。  
`{name}` → 内联表达式（预处理器提取为 `__OMNI_EXPR_N__`）。

### 6.6 事件处理器内联函数
```omni
<Button onPress={fn(e) { handle_click(e); }} />
```
`{fn(e) { handle_click(e); }}` 外层是事件处理器。  
内部 `{ handle_click(e); }` 是内联函数体。  
预处理器：识别事件处理器模式 → 提取内联函数体为 `__OMNI_BLOCK_N__`，保留 `fn(e)` 签名在事件处理器结构中。

### 6.7 Store Action 调用（空括号被 Lark 丢弃）
```omni
fn do_stuff() {
  store.increment();
}
```
OmniLang 解析 `store.increment();` → `ExprStmt(MethodCall(object=VarRef("store"), method="increment", args=[]))`。  
Lark 丢弃空括号的问题只存在于 OmniView 的旧语法——OmniLang 正确处理空括号。

### 6.8 单行注释中的花括号
```omni
// This is a {comment}
signal x: i32 = 0;
```
注释中的 `{comment}` 不计入括号深度。预处理器跳过 `// ... \n` 和 `/* ... */`。

### 6.9 嵌套 View 块中的 @if/@for
```omni
view {
  @if items.length > 0 {
    @for item in items {
      <Row>{item.name}</Row>
    }
  }
}
```
多层嵌套 `{...}` 由 Lark 解析（它们属于 UI 结构）。只有最内层 `{item.name}` 是表达式。

### 6.10 空块
```omni
@mount {}
action reset() {}
fn noop() {}
```
空的 `{}` 产生空语句块。`parse_inline_block("")` → `Block(stmts=[])`。

### 6.11 多语句块
```omni
action login(name: String) {
  let u = name;
  current_user = u;
  logged_in = true;
}
```
`{ let u = name; current_user = u; logged_in = true; }` → `parse_inline_block("let u = name; current_user = u; logged_in = true;")` → 包含 3 条语句的 `Block`。

### 6.12 表达式中的 Map 字面量
```omni
signal config: Map<String, i32> = {"a": 1, "b": 2};
```
`{"a": 1, "b": 2}` 既是 `{...}` 表达式区域，内部又包含 `{` ... `}`。OmniLang 将 `"a": 1, "b": 2` 解析为 Map 条目，花括号由 Map 字面量语法处理。

### 6.13 @if 条件中的表达式
```omni
@if filter == "all" || filter == "active" {
  <Content/>
}
```
`filter == "all" || filter == "active"` 是 @if 的条件表达式。预处理器识别 `@if` 关键字 → 条件部分提取为 `__OMNI_EXPR_N__`，view block body `{ <Content/> }` 由 Lark 解析。

### 6.14 属性中的函数调用表达式
```omni
<Input value={format_name(user.name)} />
```
`format_name(user.name)` 是嵌套表达式（函数调用内嵌套字段访问）。OmniLang 完整解析。

### 6.15 展开语法
```omni
action append(item: Item) {
  items = [...items, item];
}
```
`[...items, item]` 中的 `...items` 是 SpreadExpr。OmniLang 的 ArrayLiteral 正确处理。

### 6.16 三元表达式
```omni
<Text>{is_active ? "On" : "Off"}</Text>
```
`is_active ? "On" : "Off"` 是三元条件表达式。OmniLang 解析为 `IfExpr`。

### 6.17 带闭包的事件处理器
```omni
<Button onPress={fn(e) {
  if (e.target.value != "") {
    submit(e.target.value);
  }
}} />
```
嵌套 if 语句在闭包体内。`parse_inline_block("if (e.target.value != \"\") { submit(e.target.value); }")`。

---

## 7. 词法状态机

预处理器维护以下状态标志：

```
in_string: bool = false        // 双引号字符串内
in_template: bool = false      // 模板字符串内
in_template_expr: bool = false // 模板字符串 ${...} 内
in_line_comment: bool = false  // // 注释内
in_block_comment: bool = false // /* */ 注释内
```

括号深度仅在 `not in_string and not in_template and not in_line_comment and not in_block_comment` 时计算。

---

## 8. 错误位置映射

OmniLang 对嵌入表达式产生的诊断位置（行号、列号）是相对于嵌入表达式的。预处理器生成的位置映射表将诊断位置映射回 `.omni` 源位置。

### 映射算法

```python
def map_diagnostic(
    diag: Diagnostic,
    placeholder_id: str,
    position_map: dict[str, int],
    original_source: str,
) -> Diagnostic:
    """将 OmniLang 诊断中的相对位置映射到 .omni 源中的绝对位置。"""
    base_offset = position_map[placeholder_id]
    # diag.span.start_line, diag.span.start_col 是相对于嵌入表达式的
    # 需要加上 base_offset 对应的行/列
    abs_line = base_line + diag.span.start_line - 1
    abs_col = base_col + diag.span.start_col - 1  # 如果嵌入在第 1 列
    return Diagnostic(
        ...,
        span=Span(start_line=abs_line, start_col=abs_col, ...),
    )
```

---

## 9. 性能约束

- 单文件预处理 < 10ms（典型 ~500 行 .omni 文件）
- 解析器实例缓存：`create_parser()` 只调用一次，缓存在 `omnilang_bridge.py` 中
- 占位符 → 解析结果的 memoization：相同源文本的表达式只解析一次

---

## 10. 测试策略

| 测试类别 | 最小用例数 | 说明 |
|---------|-----------|------|
| 基本表达式提取 | 5 | 字面量、变量引用、二元运算、方法调用、三元 |
| 语句块提取 | 3 | 空块、单语句、多语句 |
| 嵌套花括号 | 3 | 2 层、3 层、深度嵌套 |
| 字符串中的花括号 | 2 | 双引号内、模板字符串内 |
| 注释中的花括号 | 2 | 单行注释、块注释 |
| 事件处理器 | 2 | 简单 fn、嵌套 if |
| 错误处理 | 3 | 不匹配的 {、不匹配的 }、空表达式 |
| 完整文件 | 2 | TodoMVC、Shop |
| **总计** | **22+** | |
