# OmniLang `design` 语法编译管线修复计划

> 基于 9 个 subagent 深度分析 + 4 个设计方案，2026-07-08

---

## 总览

`design { token / class / variant }` 的解析(parse)、降低(lower)、IR 结构均已完成。核心问题是**管线断裂**和**关键语法缺陷**。

## Phase 0 — 致命修复：接通管线 (P0)

### 0.1 修复 parser 静默跳过未知字符

**文件**: `omnilang/src/parser/design.rs:43`

**现状**: `remaining = &remaining[1..]; // skip unknown char` — 任何不识别的字符被逐字丢弃，零错误报告。

**修复**: 改为产生明确的错误信息：
```rust
// 之前
} else {
    remaining = &remaining[1..];
}

// 之后
} else {
    return Err(format!(
        "unexpected content at position {}: '{}...'",
        input.len() - remaining.len(),
        &remaining[..remaining.find('\n').unwrap_or(remaining.len()).min(40)]
    ));
}
```

同时修复 `parse_variant()` 的问题：当 main loop 遇到不匹配的前缀时，应该检查是否遇到 `}` 而不是静默 break。

### 0.2 修复 `DesignValue::Mixed` 解析

**文件**: `omnilang/src/parser/design.rs:234-257` (`parse_design_value`)

**现状**: AST 已定义 `DesignValue::Mixed`，lowering 也完整处理了，但 parser 从不产出 Mixed 变体。写 `"1px solid " token(primary)` 时，parser 只取第一个 `"..."` 段，后面的 `token(primary)` 被丢弃。

**修复**: 重写 `parse_design_value()` — 当遇到 `"` 开头时，扫描到闭合 `"`，然后继续检查紧跟着的文本是否包含 `token(` 模式。如果包含，则产生 `DesignValue::Mixed`：
```rust
fn parse_design_value(input: &str) -> ParseResult<(DesignValue, &str)> {
    let input = input.trim_start();
    if let Some(rest) = input.strip_prefix("token(") {
        let end = rest.find(')').ok_or_else(|| err("expected ')'"))?;
        let name = rest[..end].trim().to_string();
        return Ok((DesignValue::TokenRef(name), &rest[end + 1..]));
    }
    
    let mut parts: Vec<DesignValuePart> = Vec::new();
    let mut remaining = input;
    
    // Scan: alternately parse quoted strings and token() refs
    while !remaining.is_empty() && !remaining.starts_with(';') && !remaining.starts_with('}') {
        if remaining.starts_with('"') {
            let end = remaining[1..].find('"').map(|i| i + 2)
                .ok_or_else(|| err("expected closing '\"'"))?;
            parts.push(DesignValuePart::Text(remaining[1..end - 1].to_string()));
            remaining = remaining[end..].trim_start();
        } else if remaining.starts_with("token(") {
            let end = remaining.find(')').ok_or_else(|| err("expected ')'"))?;
            let name = remaining["token(".len()..end].trim().to_string();
            parts.push(DesignValuePart::Token(name));
            remaining = remaining[end + 1..].trim_start();
        } else {
            // Unquoted literal — read until ; } or token(
            let end = remaining.find([';', '}'])
                .or_else(|| remaining.find("token(").map(|i| i))
                .unwrap_or(remaining.len());
            let text = remaining[..end].trim().to_string();
            if !text.is_empty() {
                parts.push(DesignValuePart::Text(text));
            }
            remaining = &remaining[end..];
        }
    }
    
    if parts.is_empty() {
        return Err(err("expected a value"));
    }
    if parts.len() == 1 {
        match &parts[0] {
            DesignValuePart::Text(s) => return Ok((DesignValue::Literal(s.clone()), remaining)),
            DesignValuePart::Token(n) => return Ok((DesignValue::TokenRef(n.clone()), remaining)),
        }
    }
    Ok((DesignValue::Mixed(parts), remaining))
}
```

### 0.3 将 `emit_design_css()` 接入 `generate()`

**文件**: `omnilang/src/codegen/omniview/mod.rs`

在 `generate()` 中，在 component loop 之后、`emit_dom_and_bindings()` 之前，遍历所有 module 的 designs 并将 CSS 注入 `gen.css`：

```rust
// ── Emit design block CSS (after components, before DOM) ──
for module in ir.modules.values() {
    for design in &module.designs {
        if let Ok(css) = design::emit_design_css(design) {
            gen.css.push_str(&format!("\n/* ── Design: {} ── */\n", design.name));
            gen.css.push_str(&css);
        }
    }
}
```

同时在 `generate()` 开头收集所有 design 名称到 `gen.design_names`，用于 view.rs 的 HTML 属性注入。

---

## Phase 1 — 核心语义修复 (P1)

### 1.1 Token → CSS 变量策略

**设计决定**: 纯 CSS 变量方案。`resolve_token_ref()` 始终产出 `var(--name)`，`emit_design_css()` 产出 `:root { --name: value; }` 声明。

**文件变更**:

| 文件 | 变更 |
|------|------|
| `omnilang/src/ir/lower/design.rs:86-91` | `resolve_token_ref()` — 删除 token map lookup，始终返回 `format!("var(--{})", name)` |
| `omnilang/src/codegen/omniview/design.rs:21-38` | `emit_design_css()` — 在 class/variant 循环前新增 `:root { }` 块，遍历 `design.tokens` 输出 `--name: value;` |
| `omnilang/src/ir/lower/design.rs` tests | 更新 `lower_class_resolves_token_refs` — 期望 `var(--surface)` 而非 `#ffffff`；更新 fallback test |
| `omnilang/src/codegen/omniview/design.rs` tests | 更新 `emit_class_css_output` — 期望 CSS 输出中包含 `:root` 块和 `var(--primary)` |

### 1.2 Design CSS 作用域

**设计决定**: `[data-design~="DesignName"]` 属性选择器，与现有 `[data-component]` 模式一致。

**文件变更**:

| 文件 | 变更 |
|------|------|
| `omnilang/src/codegen/omniview/design.rs:21` | `emit_design_css(design, scope)` — 新增 `scope: &str` 参数，构建 `scope_prefix = format!("[data-design~=\"{}\"] ", scope)`，传给 `emit_class_css` 和 `emit_variant_css` |
| `omnilang/src/codegen/omniview/design.rs:42-64` | `emit_class_css(class, scope_prefix)` — `.Card {` → `[data-design~="X"] .Card {` |
| `omnilang/src/codegen/omniview/design.rs:67-91` | `emit_variant_css(variant, scope_prefix)` — 同上 |
| `omnilang/src/codegen/omniview/mod.rs` | `OmniViewGenerator` 新增 `design_names: Vec<String>` 字段；`test_new()` 初始化 |
| `omnilang/src/codegen/omniview/view.rs` | `emit_view()` — 根 div 添加 `data-design="Name1 Name2"` 属性 |
| tests | 更新所有 design CSS 测试的期望输出 |

### 1.3 `expand_variant()` 应用默认值

**文件**: `omnilang/src/codegen/omniview/design.rs:104-134`

当前 `expand_variant()` 不读取 `variant.defaults`。修复：遍历 `variant.dimensions`，对于 `dimension_values` 中缺失的维度，从 `variant.defaults` 补入。

```rust
// 在 classes.push(variant_name) 之后
for dim in &variant.dimensions {
    let val = dimension_values
        .get(&dim.name)
        .or_else(|| variant.defaults.get(&dim.name));
    if let Some(val) = val {
        if dim.values.contains(val) {
            classes.push(val.clone());
        }
    }
}
```

### 1.4 扩展 TokenType 枚举

**设计决定**: 4 种 → 13 种，闭合集合，keyword 全词匹配。

**Rust 侧变更**:

| 文件 | 变更 |
|------|------|
| `omnilang/src/parser/ast/design.rs:19-25` | `TokenType` enum 从 `Color \| Size \| Font \| Shadow` 扩展为 13 种 |
| `omnilang/src/ir/design.rs:32-37` | `TokenTypeIR` 同步扩展 |
| `omnilang/src/parser/design.rs:88-101` | `parse_token_type()` — 从 prefix-matching 改为 `parse_ident()` + 全词 `match`，消除 `font` 歧义 |
| `omnilang/src/ir/lower/design.rs:24-29` | `lower_design()` — 匹配新增的 token type 变体 |

**TypeScript 侧变更**:

| 文件 | 变更 |
|------|------|
| `poiesis/src/core/types/design.ts` | `TokenType` union 扩展到 13 种；`tokenTypeToCategory()` 更新映射 |
| `poiesis/src/lib/constants/style-properties.ts` | 可选：新增 token type 对应的 validation config |

### 1.5 修复文档变体默认语法

**文件**: `docs/omniview-syntax-reference.md:~1723`

文档显示内联语法 `color: primary; size: md;`，但 parser 要求 `@default { color: primary; size: md; }`。Parser/emitter/editor 三者均已正确实现 `@default` 包装器，只需修正文档。**零代码变更。**

---

## Phase 2 — 质量提升 (P2)

### 2.1 修复缩进一致性

**文件**: `omnilang/src/codegen/omniview/design.rs:48,74`

`\t` (硬 tab) → `  ` (2 空格)，与 component style 输出一致。

### 2.2 伪元素支持 (`&::before` / `&::after`)

**文件**: `omnilang/src/parser/design.rs:157`

在 `parse_class()` 中检测 `&:` 前缀时，同时检测 `&::`：
```rust
if remaining.starts_with("&::") {
    // 伪元素
    let (pseudo, rest) = parse_pseudo_element(remaining)?;
    pseudo_states.push(pseudo);
    remaining = rest;
} else if remaining.starts_with("&:") {
    // 伪类（现有逻辑）
    ...
}
```

`parse_pseudo_element()` 与 `parse_pseudo_state()` 类似，但 `ps.pseudo` 存储为 `"::before"`（带双冒号）。

**codegen 侧**: `emit_class_css()` 已经用 `format!(".{}:{}", name, ps.pseudo)` 拼接。若 pseudo 已是 `"::before"`，产出却是 `.Card:::before`（三冒号）。需修正为：检测 pseudo 是否 `::` 前缀，若是则直接拼接，否则加 `:`。

### 2.3 Parser 错误类型升级

**文件**: `omnilang/src/parser/design.rs`

将 `type ParseResult<T> = Result<T, String>` 升级为结构化错误类型，携带 position offset：

```rust
struct DesignParseError {
    message: String,
    offset: usize,    // byte offset into the design body string
}

type ParseResult<T> = Result<T, DesignParseError>;
```

所有 `err("msg")` 调用改为携带当前 position。顶层 `parse_design_def()` 将 position 映射回源文件行列号。

### 2.4 废弃 `theme` 关键字

**文件**: `omnilang/src/parser/top_level/mod.rs:40-42`, `omnilang/src/ir/lower/mod.rs:121`

- 保留 parser 中的 theme 关键字识别（避免语法错误），但 parse 后丢弃并发出 warning
- `ir/lower/mod.rs` 已有 `themes: Vec::new()` — 不变
- `validate.rs:136` 保留 theme 需要 `import omniview` 的提示

---

## Phase 3 — 测试补全 (P3)

### 3.1 Parser 层测试

在 `omnilang/tests/design_parse_test.rs` 新增：

| 测试 | 内容 |
|------|------|
| `parse_design_mixed_value` | `"1px solid " token(primary)` → 验证 `DesignValue::Mixed` |
| `parse_design_pseudo_element` | `&::before { content: "x"; }` → 验证 pseudo `"::before"` |
| `parse_all_token_types` | 13 种 token type 各一个 → 验证全解析 |
| `parse_design_error_recovery` | 畸形输入 → 验证不 panic，产有意义错误 |
| `parse_variant_defaults_wrapper` | `@default { color: primary; }` → 验证正确解析 |
| `parse_design_block_comment` | `/* comment */` → 验证不崩溃 |

### 3.2 Codegen 层测试

在 `omnilang/tests/design_codegen_test.rs` 新增：

| 测试 | 内容 |
|------|------|
| `emit_css_with_variables` | 验证 `:root { --primary: ...; }` + `.Card { background: var(--primary); }` |
| `emit_css_with_scope` | 验证 `[data-design~="AppDesign"] .Card { }` 前缀 |
| `expand_variant_with_defaults` | 部分维度值 → 验证默认值填充 |

### 3.3 `.omni` fixture 文件

在 `test/projects/learn-omniview/src/` 下新增 `design.omni`:
```omni
import omniview;

design LearnDesign {
    token primary: color = "#6366f1";
    token spacingMd: size = "16px";
    token radiusMd: radius = "8px";
    token bodyFont: fontFamily = "system-ui, sans-serif";
    token bodySize: fontSize = "16px";
    token fastMs: duration = "200ms";
    token disabledOpacity: opacity = "0.5";
    token modalZ: zIndex = "100";

    class Card {
        background: token(primary);
        border-radius: token(radiusMd);
        padding: token(spacingMd);
        &:hover {
            opacity: token(disabledOpacity);
        }
    }

    variant Button {
        @dim color: primary | danger;
        @dim size: sm | md | lg;
        @default { color: primary; size: md; }
        base {
            display: inline-flex;
            border-radius: token(radiusMd);
            font-family: token(bodyFont);
            font-size: token(bodySize);
        }
        color.primary { background: token(primary); }
        color.danger { background: "#ef4444"; }
        size.sm { padding: "4px 8px"; font-size: "12px"; }
        size.lg { padding: "12px 24px"; font-size: "20px"; }
    }
}
```

---

## 变更文件汇总

| Phase | 文件 | 变更类型 |
|-------|------|---------|
| 0.1 | `omnilang/src/parser/design.rs` | 修复静默跳过 → 报错 |
| 0.2 | `omnilang/src/parser/design.rs` | 重写 `parse_design_value()` |
| 0.3 | `omnilang/src/codegen/omniview/mod.rs` | 接入 `emit_design_css()` + 收集 design_names |
| 1.1 | `omnilang/src/ir/lower/design.rs` | `resolve_token_ref()` → 始终 `var(--name)` |
| 1.1 | `omnilang/src/codegen/omniview/design.rs` | 新增 `:root` token 声明 |
| 1.2 | `omnilang/src/codegen/omniview/design.rs` | 新增 `scope` 参数 + `[data-design]` 前缀 |
| 1.2 | `omnilang/src/codegen/omniview/view.rs` | 根 div 添加 `data-design` 属性 |
| 1.2 | `omnilang/src/codegen/omniview/mod.rs` | 新增 `design_names` 字段 |
| 1.3 | `omnilang/src/codegen/omniview/design.rs` | `expand_variant()` 读取 defaults |
| 1.4 | `omnilang/src/parser/ast/design.rs` | TokenType enum 4→13 |
| 1.4 | `omnilang/src/ir/design.rs` | TokenTypeIR 同步 |
| 1.4 | `omnilang/src/parser/design.rs` | `parse_token_type()` 全词匹配 |
| 1.4 | `poiesis/src/core/types/design.ts` | TokenType union 更新 |
| 1.5 | `docs/omniview-syntax-reference.md` | 修正 variant 默认语法 |
| 2.1 | `omnilang/src/codegen/omniview/design.rs` | `\t` → 2 空格 |
| 2.2 | `omnilang/src/parser/design.rs` | 新增 `&::` 检测 |
| 2.2 | `omnilang/src/codegen/omniview/design.rs` | 伪元素 selector 拼接修正 |
| 2.3 | `omnilang/src/parser/design.rs` | 错误类型 String → 结构化 |
| 2.4 | `omnilang/src/ir/lower/mod.rs` | theme 警告 |
| 3.1 | `omnilang/tests/design_parse_test.rs` | 新测试 |
| 3.2 | `omnilang/tests/design_codegen_test.rs` | 新测试 |
| 3.3 | `test/projects/learn-omniview/src/design.omni` | 新 fixture |
