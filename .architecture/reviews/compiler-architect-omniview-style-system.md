# OmniView Style System Architecture Review

**Reviewer**: Dr. Lin Wei — Compiler & Language Architect
**Date**: 2026-07-28
**Target**: OmniView Style System (样式系统)
**Type**: Specialist Review

---

## Executive Summary

The OmniView style system is a competent DSL implementation with two complementary subsystems: component-scoped `style { }` blocks and globally organized `design { }` blocks with typed tokens and multi-dimensional variants. The pipeline is cleanly separated across lexer, parser, AST, IR, and codegen stages, and the CSS output is correctly scoped, escaped, and editor-aware. However, there are three CRITICAL issues: hex color parsing bug, the complete absence of compiler-level theme switching support (the `theme` keyword is dead code; design tokens are hardcoded to `:root` with no per-theme value support), and the React/Vue numeric CSS value path. Architectural gaps include the token/component-style split and untyped inline-style IR representation. The design decision to build two parallel style systems (design tokens vs. component styles) with no cross-referencing, and the complete disconnect between design tokens and runtime theme switching, are the dominant architectural concerns. **Maturity rating: B-** (functional for shipped features, latent correctness bugs in less-tested paths, theme switching absent).

---

## Strengths

### 1. Clean Pipeline Separation

The style system maintains distinct types at each pipeline stage with clear transformation boundaries:

- **Parser** (`parser/view/style.rs`, `parser/design.rs`) produces AST: `StyleRule`, `DesignDef`
- **IR** (`ir/types.rs`, `ir/design.rs`) models: `StyleRuleIR`, `DesignIR`, `ClassIR`, `VariantIR`
- **IR Lowering** (`ir/lower/component.rs:176-238`, `ir/lower/design.rs`) transforms AST to IR
- **Codegen** (`omniview/design.rs`, `omniview/view/mod.rs`) emits scoped CSS output

No stage bleeds responsibility. Design blocks and component style blocks are lowered independently and meet only at codegen time.

### 2. Well-Designed Token System

The design system's 13 typed token categories (`Color`, `Size`, `Radius`, `FontFamily`, etc.) enforce discipline without being restrictive. The `token(name)` reference syntax (`crates/omnilang-core/src/ir/lower/design.rs:82-94`) correctly resolves to `var(--name)` across classes and variants. The variant system with `@dim` axes, `@default`, `base`, and branch overrides (`parser/design.rs:377-448`) supports multi-dimensional theming well.

### 3. Robust CSS Scoping and Security

All CSS output is scoped:

- Component `style { }` blocks: `[data-component="Name"] .class { ... }` (`omniview/view/mod.rs:99-101`)
- Design classes: `[data-design~="Scope"] .class { ... }` (`omniview/design.rs:47-51`)
- Editor mode strips scoping for canvas rendering (`emit_design_css_editor` at `omniview/design.rs:34`)

CSS value sanitization (`crates/omnilang-core/src/codegen/sanitize.rs:12-28`) blocks `javascript:`, `expression(`, `-moz-binding`, `data:text/html`, `vbscript:`, `@import`, and DOM access patterns.

### 4. CSS Custom Property Support

The pipeline correctly handles `--` prefixed properties:

- Parsed via `parse_css_property_name()` in `parser/view/style.rs:30-72`
- Static inline `--*` properties emit as HTML style attributes
- Dynamic inline `--*` properties emit as `el.style.setProperty("--name", value)` in `omniview/store.rs:524-533`
- Component style `--name` declarations pass through to CSS unchanged

### 5. Design Variant Resolution at Codegen Time

The variant expansion system (`omniview/design.rs:150-184`) correctly resolves dimension values, applies defaults, and validates against declared dimension values. The output is a list of CSS class names that get applied to the element, with the actual CSS already emitted by `emit_design_css`.

---

## Concerns

### CRITICAL

#### C1: Hex Color Parsing Truncates Leading Zeros

**Location**: `crates/omnilang-core/src/parser/view/style.rs:145-193` (in both `parse_style_rules` and `parse_style_rule`), and lines 267-296 (multi-value variant).

**Why it matters**: The parser consumes `#` then reads the next token to reconstruct hex colors. When the hex value starts with digits followed by letters (e.g., `#00ff00`), the OmniLang lexer splits it into an `IntLiteral` (for the numeric prefix) and an `Ident` (for the letter suffix). `u64::parse()` strips leading zeros, and `n.to_string()` does not re-add them. The result is an incorrect hex color string.

**Before/After**:

| Source   | Token sequence                   | Output    | Expected   | Visual      |
|----------|----------------------------------|-----------|------------|-------------|
| `#0000ff`| `Hash`, `IntLiteral(0)`, `Ident("ff")` | `#0ff`    | `#0000ff`  | cyan vs blue |
| `#00ff00`| `Hash`, `IntLiteral(0)`, `Ident("ff00")` | `#0ff00` | `#00ff00` | 5-hex, wrong |
| `#0033ff`| `Hash`, `IntLiteral(33)`, `Ident("ff")` | `#33ff`  | `#0033ff`  | short, wrong |
| `#000000`| `Hash`, `IntLiteral(0)`, `Ident("0")` | `#00`    | `#000000`  | invalid |

**Consumer audit**: `StyleValue::HexColor` is consumed by IR lowering (`ir/lower/component.rs:204-206`), which stores it as a JSON string. Both OmniView native codegen and React/Vue codegen emit it as-is. No downstream consumer corrects it.

**Fix**: Replace token reconstruction with source-text extraction. After consuming `#`, read raw hex characters directly from the source byte slice between `#` position and the next non-hex character boundary. Apply fix to all three instances of the pattern (lines 145-193, 267-296, and the legacy `parse_style_rule` at 361-396).

---

### HIGH

#### H1: React/Vue CSS Codegen Appends "px" to All Numeric Values

**Location**: `crates/omnilang-core/src/codegen/helpers.rs:123-131` (function `format_css_value`), called from `generate_css_style_block` at line 147.

**Why it matters**: The function unconditionally converts `Value::Number(n)` to `format!("{}px", n)`. This is incorrect for unitless CSS properties: `opacity`, `flex`, `z-index`, `order`, `line-height`, `font-weight`, `scale`, `rotate`, and animation-related values.

**Before/After**:

| Source `.omni`     | IR                                  | React/Vue CSS output     | Correct CSS |
|--------------------|-------------------------------------|--------------------------|-------------|
| `opacity: 0.5`     | `Number(0.5)` → `JSON 0.5`         | `opacity: 0.5px;`        | `opacity: 0.5;` |
| `flex: 1`          | `Number(1.0)` → `JSON 1`           | `flex: 1px;`             | `flex: 1;` |
| `z-index: 100`     | `Number(100.0)` → `JSON 100`       | `z-index: 100px;`        | `z-index: 100;` |
| `line-height: 1.5` | `Number(1.5)` → `JSON 1.5`         | `line-height: 1.5px;`    | `line-height: 1.5;` |

**Consumer audit**: `format_css_value` is called only from `generate_css_style_block` (line 137), which is used by:
- `omnilang/src/codegen/react/component/mod.rs:539`
- `omnilang/src/codegen/vue/mod.rs:250`

OmniView native codegen is NOT affected — its `expr_to_css` (`omniview/expr.rs:399-417`) correctly emits numbers as-is.

**Fix**: Remove the `px` suffix from `format_css_value`:
```rust
serde_json::Value::Number(n) => n.to_string(),
```

#### H2: camel_to_kebab Corrupts CSS Custom Property Names

**Location**: `crates/omnilang-core/src/codegen/helpers.rs:96-118` (functions `camel_to_separated` and `camel_to_kebab`).

**Callers**: `omniview/design.rs:85,100,117,130`, `omniview/view/mod.rs:105`, `omniview/view/element.rs:373`.

**Why it matters**: `camel_to_separated` processes every uppercase character by inserting a separator. For `--prefixed` names, this transforms `--myCustomColor` into `--my-custom-color`. CSS custom properties are case-sensitive — `--myCustomColor` and `--my-custom-color` are different properties.

**Before/After**:
- Input: `--myCustomColor`
- `camel_to_kebab("--myCustomColor")` → `"--my-custom-color"`
- Correct output: `"--myCustomColor"` (unchanged)

The doc comment (line 114) explicitly claims `"--customProp" → "--custom-prop"` which documents this wrong behavior as correct.

**Fix**: Add an early return at the top of `camel_to_kebab`:
```rust
pub fn camel_to_kebab(s: &str) -> String {
    if s.starts_with("--") {
        return s.to_string(); // CSS custom properties are case-sensitive
    }
    camel_to_separated(s, '-')
}
```

---

### MEDIUM

#### M1: Design Token `resolve_token_ref` Ignores Token Map, Silently Produces `var(--undefined)`

**Location**: `crates/omnilang-core/src/ir/lower/design.rs:100-102`.

The function signature is `fn resolve_token_ref(name: &str, _tokens: &FxHashMap<String, TokenIR>) -> String` — the `_tokens` parameter is explicitly unused. Combined with `validate_design` emitting warnings (not errors), undefined token references silently produce CSS `var(--undefined)` which resolves to `initial`/`inherited` at runtime.

The `validate_design` function (line 186) does check for undefined tokens, but:
- It emits `Severity::Warning` (not `Error`) — the compilation succeeds
- The warnings lack source location (`line: 0, col: 0` per `ir/lower/mod.rs:180-183`)

#### M2: No Token Sharing Between Design System and Component Styles

Design tokens (`token(name)`) can only be referenced from within `design { }` blocks. Component `style { }` blocks have no `token()` function — users must manually write `var(--token-name)`. This creates a discoverability gap and couples users to the implementation detail of CSS variable naming.

The two style systems also use different IR types:
- Design: `DesignIR` with `FxHashMap<String, String>` properties (fully resolved at lowering time)
- Component: `StyleRuleIR` with `FxHashMap<String, ExprIR>` properties (evaluated at codegen time)

#### M3: Inline Styles Stored as Untyped JSON in IR

**Location**: `crates/omnilang-core/src/ir/types.rs:622-623`:
```rust
pub style_classes: Option<serde_json::Value>,
pub inline_style: Option<serde_json::Value>,
```

Inline styles bypass the typed IR system. Consequences:
- The DCE optimizer must use `walk_json_for_signals` as a special case (`ir/optimize.rs:320-324`) instead of the standard `walk_expr_for_idents` path
- The type checker cannot reason about inline style values
- Each codegen backend must re-implement `$signal`/`$call`/`$concat` resolution independently

Four codegen backends handle inline styles differently:
- **OmniView native** (`element.rs:272-387`): Full `$signal`/`$call`/`$concat` support with `setProperty()` for `--` props
- **React** (`view.rs:371-383`): JSX `style={ { prop: val } }` with `format_style_value_expr`
- **Vue** (`template.rs:100-112`): Reuses React's `format_style_prop_key` and `format_style_value_expr`
- **Dioxus** (`view.rs`): No inline style handling
- **Flutter** (`view.rs`): No inline style handling

#### M4: CSS Custom Property camelCase Characterization

While CSS custom properties are conventionally kebab-case, the OmniLang compiler has multiple code paths whose interaction with `--` prefixes differs:

| Code path | Behavior | Correct? |
|-----------|----------|----------|
| `parse_css_property_name()` (parser) | Reads `--name` literally | Yes |
| `camel_to_kebab()` (codegen helper) | Transforms `--myProp` → `--my-prop` | **No** |
| `emit_style_assignment()` (OmniView native) | Checks `prop.starts_with("--")` then uses `setProperty` | Yes |
| `format_style_prop_key()` (React/Vue) | Quotes `--` keys in JSX | Yes |
| `camel_to_kebab()` in `emit_style`/`emit_class_css` | Transforms property names through `camel_to_kebab` | **No** (for camelCase custom props) |

#### M5: Duplicate Hex Color Parsing Logic Across Three Code Paths

The hex color parsing pattern appears in three places with slightly different implementations:

1. `parse_style_rules` at line 145-193 (primary, uses source-byte walkback for FloatLiteral)
2. Multi-value loop at line 267-296 (uses `&self.source[next_t.span.start..next_t.span.end]` for FloatLiteral)
3. `parse_style_rule` (legacy) at line 361-396 (uses `n.to_string()` for FloatLiteral)

The FloatLiteral handling diverges, and any fix to C1 must be applied in all three places.

---

### CRITICAL (补充)

#### C2: 主题切换无编译器支持 — `theme` 关键字是死代码，设计令牌与运行时主题完全断裂

**新增发现** — 用户的追问暴露了上一轮评审遗漏的关键架构缺陷。

**现象一：`theme` 关键字是解析器死胡同**

- `parse_theme_def()` (`parser/top_level/store.rs:69-106`) 正确解析 `theme AppTheme { colors { ... } }` 语法
- 生成 `ThemeDef` AST 节点 (`parser/ast/motion.rs:314-322`)
- 存入 `ModuleDef.themes: Vec<ThemeDef>` (`parser/ast/mod.rs:475`)
- **但没有对应的 IR 类型** — `ir/types.rs` 中无 `ThemeIR`
- **没有 lowering pass** — `ir/lower/` 目录中无 `theme.rs`
- **所有 codegen 后端均不处理** — 零 CSS 输出
- 官方文档承认："代码生成尚未实现" (`docs/omniview/10-design-system.md:248`)

**现象二：设计令牌强制绑定 `:root`，不可主题化**

`emit_design_css()` (`omniview/design.rs:56`) 硬编码：
```rust
css.push_str(":root {\n");
for (name, token) in &sorted {
    css.push_str(&format!("  --{}: {};\n", name, token.value));
}
css.push_str("}\n\n");
```

所有设计令牌值在编译时就被写入 `:root`，没有以下任何机制：
- `[data-theme="dark"] { --token: darkValue; }` 作用域覆盖
- `@media (prefers-color-scheme: dark)` 系统偏好
- `:root.dark` / `:root.light` 类切换
- Token 的 per-theme 值定义（如 `token primary: color = "#6366f1" dark "#a5b4fc"`）

**现象三：运行时主题切换是纯手动应用层方案**

实际项目 (learn-omniview) 中的主题切换工作方式 (`stores/ThemeStore.omni`):
```omni
store ThemeStore {
    signal mode: String = "light";
    getter currentBg: String = if mode == "dark" { "#111827" } else { "#ffffff" };
    getter currentText: String = if mode == "dark" { "#f9fafb" } else { "#111827" };
    // ... 手工为每个颜色值写 if/else ...
    action toggle() { mode = if mode == "dark" { "light" } else { "dark" }; }
}
```

使用方式 (`ThemeCssVarsDemo.omni`):
```omni
<div style: {
    --theme-bg: ThemeStore.currentBg,      // 每个属性单独绑定信号
    --theme-text: ThemeStore.currentText,
    --theme-primary: ThemeStore.currentPrimary,
    // ... 手动列出所有要主题化的属性 ...
}>
```

编译器为每个 `--*` 属性生成独立的 `setProperty()` 调用 (`omniview/store.rs:524-533`)。

**四个层面的断裂**：

| 层面 | 设计令牌系统 | 主题切换 | 关系 |
|------|-------------|---------|------|
| 值定义 | `design { token primary: color = "#6366f1" }` | `store { getter currentPrimary = if mode... }` | **重复定义**，两套独立的值 |
| CSS 输出 | `:root { --primary: #6366f1; }` (编译时静态) | `el.style.setProperty("--theme-primary", value)` (运行时动态) | **不同变量名**，互不可见 |
| 引用方式 | `token(primary)` → `var(--primary)` | 手动 `var(--theme-primary)` | **不同 API**，不可互换 |
| 切换机制 | 无 (永远静态) | store action toggle → 信号级联 → setProperty | **设计令牌不参与切换** |

**后果**：
- 用户必须在 store getter 中重复定义每个主题化颜色值（与 design token 重复）
- 必须在每个使用主题的组件根元素上手动列出 `style: { --theme-*: getter }`
- 设计令牌（`token()`）只在非主题化场景有价值
- 主题切换和应用样式系统完全分离，增加了概念负担
- 每个 `--*` 信号绑定生成独立的 `setProperty` effect，不如 `data-theme` 属性切换高效

**Severity: CRITICAL** — 这不仅是功能缺失，而是架构设计缺陷：两个并行系统（design tokens vs theme switching）各自工作但互不感知，导致用户必须在应用层手动桥接，产生大量重复代码。

---

### LOW

#### L1: Component Style Class Validation is Deferred to Codegen

**Location**: `omnilang/src/codegen/omniview/view/element.rs:241-263`

Class name validation (checking if a class referenced in `class:` exists in `design_map` or `component_style_classes`) happens at codegen time, not during semantic analysis. Missing classes produce a string warning that is pushed to `self.warnings` but not surfaced through the compiler's diagnostic pipeline.

#### L2: Pseudo-element Parsing Correctly Stored, But Compound Selectors Not Supported

The parser correctly handles `&::before` syntax in design blocks (`parser/design.rs:258-285`) by prefixing the pseudo with `::` so codegen emits `.Card::before` not `.Card:before`. However, compound selectors like `.a.b` in component `style { }` blocks are handled only through inline class-name string manipulation in the selector name, not as structured AST nodes — this is fragile and could intersect poorly with scoping.

---

## Feature Coverage Matrix

| CSS Feature | Component `style { }` | Design `design { }` | Inline `style:` |
|---|---|---|---|
| Property-value pairs | Yes | Yes | Yes |
| Class selectors `.class` | Yes | Yes | N/A |
| Tag selectors `div` | Yes | No | N/A |
| Pseudo-classes `:hover` | Yes (in selector name) | Yes (`&:hover`) | N/A |
| Pseudo-elements `::before` | Yes (via `parse_style_rules` colon handling) | Yes (`&::before`) | N/A |
| Compound selectors `.a.b` | Partial (fragile) | No | N/A |
| Descendant combinators | No | No | N/A |
| CSS custom properties `--name` | Yes | Yes | Yes (static + dynamic) |
| `var()` references | Yes (manual) | Yes (`token()` → `var()`) | N/A |
| Design token references | No | Yes | No |
| Media queries `@media` | No | No | N/A |
| `@keyframes` | No | No | N/A |
| CSS Grid / Flexbox values | Passthrough | Passthrough | Passthrough |
| CSS functions `calc()`, `rgb()` | Passthrough | Passthrough | Passthrough |
| Dynamic (signal-driven) values | No | No | Yes |
| Variant system (multi-dim theming) | No | Yes | No |
| Container queries | No | No | No |
| `@layer` / `@scope` | No | No | No |

---

## Architecture Assessment

### Pipeline Cleanliness: B+

The pipeline separation is clean at the macro level:
- **Parser + AST** → **IR lowering** → **Codegen** with distinct types at each stage
- No back-propagation of codegen concerns into IR or parsing stages
- Design blocks and component styles are independently lowered and meet only at codegen

### Architectural Coherence: B-

The dominant architectural weakness is the coexistence of two parallel style systems:

```
Component style { } blocks          Design { } blocks
        │                                    │
  StyleRule (AST)                      DesignDef (AST)
        │                                    │
  StyleRuleIR (IR)                     DesignIR (IR)
        │                                    │
  emit_style()                        emit_design_css()
  (per-component CSS,                 (global CSS,
   [data-component] scoped)            [data-design] scoped)
```

These share no common IR. A future unified `StyleIR` could bridge the gap, but the current split is a pragmatic shortcut, not a principled abstraction.

### Extensibility: B-

Adding new CSS features requires changes at multiple layers. The 13 hardcoded token types in `parser/ast/design.rs:20-34` and `ir/design.rs:39-53` would need extension for new categories. The inline-style JSON representation is flexible but untyped.

### Cross-Target Consistency: B-

Key gaps:
- React/Vue codegen has the "px" suffix bug (H1) — OmniView native does not
- Dioxus and Flutter codegen omit inline styles entirely
- The React codegen uses CSS modules (`styles['className']`), while OmniView native uses `[data-component]` scoping — these are fundamentally different models
- Design variant resolution only happens in OmniView native codegen; other backends must handle `_design_variant` JSON prop independently

### Error Reporting: C

Design validation errors are demoted to `Warning` severity and lack source locations. The `validate_design` function (called at `ir/lower/mod.rs:174`) correctly checks for undefined tokens and invalid dimension branches, but these checks happen at lowering time and the results aren't surfaced clearly.

---

## Recommendations

### Immediate (Must Fix)

1. **Fix hex color parsing (C1)** — Replace token-reconstruction with source-text extraction in all three hex color parsing sites (`style.rs:145-193`, `267-296`, `361-396`). After consuming `#`, read hex characters directly from `self.source` between the `#` byte position and the next non-hex character boundary.

2. **Fix React/Vue "px" suffix bug (H1)** — Remove the `format!("{}px", n)` from `format_css_value` in `crates/omnilang-core/src/codegen/helpers.rs:127`. Emit numbers as-is.

3. **Fix camel_to_kebab for CSS custom properties (H2)** — Add `--prefixed` early return to `camel_to_kebab` in `crates/omnilang-core/src/codegen/helpers.rs:116`.

### Short-Term (Next Sprint)

4. **设计令牌支持 per-theme 值和 `[data-theme]` 作用域 (C2 第一阶段)** — 这是 C2 的 minimum viable fix：
   - 扩展 `token` 语法支持 per-theme 值：`token primary: color = "#6366f1" dark "#a5b4fc"`
   - 生成 `[data-theme="dark"] { --primary: #a5b4fc; }` 替代仅 `:root`
   - 主题切换变为 `document.documentElement.dataset.theme = "dark"` (一个属性操作)
   - 消除 store getter 中手工颜色映射的需求
   - 这是最小改动、最大收益的方案

5. **Add source location to design validation diagnostics** — Thread the design block's source span through `validate_design()` so errors include file/line/col.

6. **Make undefined token references a compile error** — Change the diagnostic severity from `Warning` to `Error` for undefined tokens, or add a `--deny-undefined-tokens` flag.

7. **Add regression tests for hex colors with leading zeros** — Add test cases covering `#00ff00`, `#0033ff`, `#0000ff`, `#000000`, `#0f00f0`, and `#ff0000` to the style parser test suite.

### Short-Term (Next Sprint)

4. **Add source location to design validation diagnostics** — Thread the design block's source span through `validate_design()` so errors include file/line/col.

5. **Make undefined token references a compile error** — Change the diagnostic severity from `Warning` to `Error` for undefined tokens, or add a `--deny-undefined-tokens` flag.

6. **Add regression tests for hex colors with leading zeros** — Add test cases covering `#00ff00`, `#0033ff`, `#0000ff`, `#000000`, `#0f00f0`, and `#ff0000` to the style parser test suite.

### Long-Term (Architectural)

7. **Bridge design tokens and component styles** — Allow `token(name)` references in component `style { }` blocks by resolving them during component IR lowering, using the same resolver as `lower_design`.

8. **Type inline styles in IR** — Replace `Option<serde_json::Value>` with a typed inline-style IR node (e.g., `StructLitExprIR` with CSS property validation), eliminating the special-case JSON walking in DCE.

9. **Consolidate design block parsing** — Migrate `parser/design.rs` from string-based parsing to token-based recursive descent, matching the component style parser's approach.

---

## Risks if Unaddressed

- Hex color bug silently produces wrong CSS — users must visually verify every styled component
- React/Vue "px" bug corrupts unitless CSS properties — framework-specific correctness gap
- camel_to_kebab on custom properties produces subtly wrong CSS variable names — hard to debug
- Design token validation warnings may scroll off in large builds, masking undefined references
- Two parallel style systems without bridging limit design system adoption
- **Theme switching is entirely manual** — users must duplicate design token values in store getters, manually wire `--theme-*` variables per component, and each themed property generates a separate `setProperty()` effect. This doesn't scale beyond trivial cases

---

## Success Metrics

| Metric | Verification | Target |
|--------|-------------|--------|
| Hex color correctness | Add 7 regression tests | All pass |
| React/Vue unitless CSS | Compare built CSS against OmniView native for `opacity: 0.5` | No "px" suffix |
| Custom property name preservation | Assert `camel_to_kebab("--myProp") == "--myProp"` | Test passes |
| Design validation source info | Examine diagnostic output for undefined token | Has file:line:col |
| Token/component style bridging | Write `.omni` file using `token()` in component `style { }` block | Compiles and resolves |
