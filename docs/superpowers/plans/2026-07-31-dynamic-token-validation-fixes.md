# Dynamic Design Tokens — Architecture Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close validation gaps and hardening issues identified in the 2026-07-31 architecture review of dynamic design tokens (Phases 1+2). Fix 12 issues: 3 HIGH, 9 MEDIUM — all produce wrong/dropped output with zero diagnostic feedback.

**Architecture:** All fixes are additive validations or targeted replacements within the existing compiler pipeline. No new IR passes, no new trait impls, no structural changes. The majority are in `semantic/resolve.rs` (const-kind + theme-expression validation) and `parser/design.rs` (expression boundary hardening). Codegen changes are single-function consolidation or one-line guards.

**Tech Stack:** Rust (omnilang-core crate), compiler pipeline (Parser → AST → Semantic → IR → Codegen)

---

## File Map

| File | Role | Change Type |
|------|------|-------------|
| `crates/omnilang-core/src/semantic/resolve.rs` | Semantic validation for token values | Modify (3 fixes: const-kind check, theme-expression reject, bare-ident diagnostic) |
| `crates/omnilang-core/src/parser/design.rs` | Design body parser | Modify (2 fixes: `{...}` depth tracking, expression parse error offset) |
| `crates/omnilang-core/src/ir/lower/design.rs` | AST→IR lowering | Modify (2 fixes: remove dead code `resolve_token_value()`, remove `lower_design()` wrapper) |
| `omnilang/src/codegen/omniview/design.rs` | CSS + JS codegen | Modify (3 fixes: consolidate effects, null guard, sanitize static values) |
| `crates/omnilang-core/src/ir/lower/mod.rs` | Lowering orchestrator | Modify (remove `lower_design` call site if any) |

---

## Group A: Semantic Validation Fixes (HIGH Priority)

### Task A1: Reject `TokenValue::Expression` in token theme values

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:354-357`

**Why:** Currently `token primary: color = "#fff" dark ThemeStore.darkColor;` parses, passes semantic validation, but the dark theme override is silently dropped from both CSS and JS output. No diagnostic anywhere. This adds a compile-time restriction symmetric to the existing dynamic-main-value check at line 314.

- [ ] **Step 1: Write the failing test**

Open `crates/omnilang-core/src/semantic/resolve.rs`. Find the `#[cfg(test)] mod tests` block (or the test file). Add this test:

```rust
#[test]
fn rejects_expression_in_theme_value() {
    // token primary: color = "#fff" dark ThemeStore.darkColor;
    // The theme value 'ThemeStore.darkColor' is an expression but there's
    // no codegen path for dynamic theme values — must be rejected at compile time.
    let design = DesignDef {
        name: "App".to_string(),
        themes: vec!["dark".to_string()],
        default_theme: None,
        tokens: vec![TokenDef {
            name: "primary".to_string(),
            token_type: TokenType::Color,
            value: TokenValue::Literal("#ffffff".to_string()),
            theme_values: vec![(
                "dark".to_string(),
                TokenValue::Expression(Expr::FieldAccess {
                    object: Box::new(Expr::VarRef(
                        "ThemeStore".to_string(),
                        Span { start: 0, end: 10, line: 0, col: 0 },
                    )),
                    field: "darkColor".to_string(),
                    span: Span { start: 0, end: 0, line: 0, col: 0 },
                }),
            )],
        }],
        classes: vec![],
        variants: vec![],
    };
    let module = Module {
        designs: vec![design],
        ..Default::default()
    };
    let mut resolver = Resolver::new();
    resolver.resolve_and_validate_module(&module);
    let errors: Vec<_> = resolver
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::E0001)
        .collect();
    assert!(
        !errors.is_empty(),
        "should reject expression in theme value"
    );
    assert!(
        errors.iter().any(|e| e.message.contains("theme") && e.message.contains("dynamic")),
        "diagnostic should mention theme and dynamic expression restriction"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omnilang-core rejects_expression_in_theme_value`
Expected: FAIL — no E0001 diagnostic emitted for expression theme value

- [ ] **Step 3: Add the compile-time restriction**

In `crates/omnilang-core/src/semantic/resolve.rs`, replace the `TokenValue::Expression(expr)` arm in the theme_values loop (lines 354-357):

Before:
```rust
TokenValue::Expression(expr) => {
    let mut vctx = ValidationContext::new();
    self.resolve_expr_refs(expr, &mut vctx);
}
```

After:
```rust
TokenValue::Expression(expr) => {
    // Phase 2 restriction: dynamic expressions are not supported in theme
    // overrides because neither CSS [data-theme] selectors nor JS codegen
    // has a path for per-theme dynamic values. CSS [data-theme] selectors
    // would be overridden by JS style.setProperty() on <html> anyway.
    self.diagnostics.push(Diagnostic::error(
        DiagnosticKind::E0001,
        &format!(
            "Theme value '{}' for token '{}' cannot be a dynamic expression. \
             Theme overrides must use static values (literals or compile-time constants).",
            theme_name, token.name
        ),
        &Span { start: 0, end: 0, line: 0, col: 0 },
    ));
    // Still resolve refs so other diagnostics (e.g., undefined variables
    // within the expression) are also emitted.
    let mut vctx = ValidationContext::new();
    self.resolve_expr_refs(expr, &mut vctx);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p omnilang-core rejects_expression_in_theme_value`
Expected: PASS

- [ ] **Step 5: Run full test suite to check for regressions**

Run: `cargo test -p omnilang-core`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git commit -m "fix: reject dynamic expressions in design token theme values

Theme values containing TokenValue::Expression were accepted by the parser
and semantic analysis, but silently dropped by both CSS codegen (no
[data-theme] output for Dynamic variants) and JS codegen (only collects
from token.value, not token.theme_values). Add a compile-time E0001
diagnostic so users get clear feedback instead of silent data loss.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task A2: Add const-kind validation for `TokenValue::Variable` references

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:282-297`

**Why:** Currently `TokenValue::Variable` only checks symbol existence via `self.symbols.contains(name)`. A `SymbolKind::Store`, `SymbolKind::Component`, or `SymbolKind::Signal` passes this check, but at IR lowering time the name isn't in `const_values`, so it silently degrades to `var(--name)` in CSS. Similarly, a `const` with a non-string value (e.g., `const RADIUS = 8`) passes validation but can't be resolved.

- [ ] **Step 1: Write the failing test**

Add to the test module in `crates/omnilang-core/src/semantic/resolve.rs`:

```rust
#[test]
fn rejects_non_const_symbol_in_token_variable_reference() {
    // A Store symbol exists but is NOT a compile-time constant.
    // Using it as TokenValue::Variable should produce a diagnostic.
    let mut resolver = Resolver::new();
    // Register a Store symbol in the symbol table
    resolver.symbols.enter_scope(); // module scope
    resolver.symbols.declare(
        "ThemeStore".to_string(),
        SymbolKind::Store,
        None,
        false,
    );
    let design = DesignDef {
        name: "App".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: vec![TokenDef {
            name: "primary".to_string(),
            token_type: TokenType::Color,
            value: TokenValue::Variable("ThemeStore".to_string()),
            theme_values: vec![],
        }],
        classes: vec![],
        variants: vec![],
    };
    let module = Module {
        designs: vec![design],
        ..Default::default()
    };
    resolver.resolve_and_validate_module(&module);
    let warnings: Vec<_> = resolver
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::E0004)
        .collect();
    assert!(
        !warnings.is_empty(),
        "should warn when TokenValue::Variable references a non-const symbol"
    );
    assert!(
        warnings.iter().any(|w| w.message.contains("compile-time constant")),
        "diagnostic should explain that only compile-time constants are valid"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omnilang-core rejects_non_const_symbol_in_token_variable_reference`
Expected: FAIL — no warning emitted for non-const symbol

- [ ] **Step 3: Add const-kind validation**

In `crates/omnilang-core/src/semantic/resolve.rs`, replace the `TokenValue::Variable(name)` match arm (lines 282-297):

Before:
```rust
TokenValue::Variable(name) => {
    if !self.symbols.contains(name) {
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::E0001,
            &format!(
                "Undefined variable '{}' in design token '{}'",
                name, token.name
            ),
            &Span { start: 0, end: 0, line: 0, col: 0 },
        ));
    }
}
```

After:
```rust
TokenValue::Variable(name) => {
    if !self.symbols.contains(name) {
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::E0001,
            &format!(
                "Undefined variable '{}' in design token '{}'",
                name, token.name
            ),
            &Span { start: 0, end: 0, line: 0, col: 0 },
        ));
    } else if let Some(sym) = self.symbols.resolve(name) {
        // Phase 1 restriction: only compile-time constants (const declarations
        // with string-literal values) can be used as TokenValue::Variable refs.
        // Stores, signals, components, and other non-const symbols must use
        // Phase 2 expression syntax (e.g., `StoreName.fieldName`).
        if sym.kind != SymbolKind::Const {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0001,
                &format!(
                    "'{}' is a {} in design token '{}', not a compile-time constant. \
                     Only const declarations can be used as direct variable references. \
                     For runtime values, use dotted expression syntax: \
                     `token {}: {} = StoreName.{};`",
                    name,
                    format!("{:?}", sym.kind).to_lowercase(),
                    token.name,
                    token.name,
                    format!("{:?}", token.token_type).to_lowercase(),
                    name,
                ),
                &Span { start: 0, end: 0, line: 0, col: 0 },
            ));
        }
        // Note: string-compatibility of const values is validated at IR lowering
        // time — non-string consts that slip through will produce var(--name) CSS
        // fallback, which is a visible degradation the user can debug in DevTools.
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p omnilang-core rejects_non_const_symbol_in_token_variable_reference`
Expected: PASS

- [ ] **Step 5: Add test for valid const reference (regression guard)**

```rust
#[test]
fn accepts_const_symbol_in_token_variable_reference() {
    let mut resolver = Resolver::new();
    resolver.symbols.enter_scope();
    resolver.symbols.declare(
        "BRAND".to_string(),
        SymbolKind::Const,
        Some("String".to_string()),
        false,
    );
    let design = DesignDef {
        name: "App".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: vec![TokenDef {
            name: "primary".to_string(),
            token_type: TokenType::Color,
            value: TokenValue::Variable("BRAND".to_string()),
            theme_values: vec![],
        }],
        classes: vec![],
        variants: vec![],
    };
    let module = Module {
        designs: vec![design],
        ..Default::default()
    };
    resolver.resolve_and_validate_module(&module);
    let errors: Vec<_> = resolver
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::E0001)
        .collect();
    assert!(
        errors.is_empty(),
        "should NOT error for valid const reference, but got: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 6: Run full test suite**

Run: `cargo test -p omnilang-core`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git commit -m "fix: validate TokenValue::Variable references are compile-time constants

Previously only symbol existence was checked, so Store/Component/Signal
symbols passed validation but silently degraded to var(--name) CSS fallback
at IR lowering time. Now emit E0001 diagnostic when a non-Const symbol is
referenced as a TokenValue::Variable.

Phase 1 restriction per ADR-047 D1: only const declarations with
string-literal values can be used in token values.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task A3: Verify bare-ident signal refs are caught by const-kind check

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs` (test only — the fix from A2 covers this)

**Why:** The architecture review identified that `App_dynamicAccent` (a bare signal name, documented as a Phase 2 use case) produces silent `var(--name)` fallback. The const-kind check from A2 already catches this case — `App_dynamicAccent` is a `SymbolKind::Signal`, not `SymbolKind::Const`, so A2's new check emits the E0001 diagnostic. This task adds an explicit regression test to prove the gap is closed.

- [ ] **Step 1: Write the regression test**

Add to the test module:

```rust
#[test]
fn bare_signal_name_in_token_value_gets_const_kind_diagnostic() {
    // Regression test for H1 from architecture review:
    //   token accent: color = App_dynamicAccent;
    //   App_dynamicAccent is a signal, not a const — must get a diagnostic.
    let mut resolver = Resolver::new();
    resolver.symbols.enter_scope();
    resolver.symbols.declare(
        "App_dynamicAccent".to_string(),
        SymbolKind::Signal,
        Some("String".to_string()),
        false,
    );
    let design = DesignDef {
        name: "App".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: vec![TokenDef {
            name: "accent".to_string(),
            token_type: TokenType::Color,
            value: TokenValue::Variable("App_dynamicAccent".to_string()),
            theme_values: vec![],
        }],
        classes: vec![],
        variants: vec![],
    };
    let module = Module {
        designs: vec![design],
        ..Default::default()
    };
    resolver.resolve_and_validate_module(&module);
    let errors: Vec<_> = resolver
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::E0001)
        .collect();
    assert!(
        errors.iter().any(|e| e.message.contains("signal")
            && e.message.contains("App_dynamicAccent")
            && e.message.contains("compile-time constant")),
        "bare signal name in token value should produce const-kind diagnostic. \
         Got diagnostics: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 2: Run test to verify it passes (already fixed by A2)**

Run: `cargo test -p omnilang-core bare_signal_name_in_token_value_gets_const_kind_diagnostic`
Expected: PASS

- [ ] **Step 3: Run full suite and commit**

```bash
cargo test -p omnilang-core
git add crates/omnilang-core/src/semantic/resolve.rs
git commit -m "test: add regression test for bare signal names in token values

Verifies that the architecture review H1 gap (App_dynamicAccent silently
producing var(--name) CSS fallback) is closed by the const-kind validation
added in the previous commit.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group B: Parser Hardening (MEDIUM Priority)

### Task B1: Add `{...}` depth tracking to expression boundary scanner

**Files:**
- Modify: `crates/omnilang-core/src/parser/design.rs:312-338`

**Why:** The expression boundary scanner in `parse_design_value_expression()` tracks `(...)` and `[...]` nesting but not `{...}`. An expression like `store.getConfig({theme: "dark"})` would be truncated at the `}` (treated as design-block terminator).

- [ ] **Step 1: Write the failing test**

Add to `crates/omnilang-core/src/parser/design.rs` in the `#[cfg(test)] mod tests`:

```rust
#[test]
fn parse_expression_with_brace_object_argument() {
    let src = r#"token config: color = store.getConfig({theme: "dark"});"#;
    let tokens = crate::lexer::tokenize(src);
    let result = parse_design_body(src, &tokens).unwrap();
    assert_eq!(result.tokens.len(), 1);
    assert!(
        matches!(result.tokens[0].value, TokenValue::Expression(_)),
        "expression with object literal arg should parse as Expression, got {:?}",
        result.tokens[0].value
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omnilang-core parse_expression_with_brace_object_argument`
Expected: FAIL — expression truncated at `{`, falls back to Variable("store")

- [ ] **Step 3: Add `{...}` depth tracking**

In `crates/omnilang-core/src/parser/design.rs`, in `parse_design_value_expression()`, modify the scanner loop (lines 312-339):

Before:
```rust
let mut end_idx = start_idx + 1;
let mut depth: i32 = 0;
while end_idx < design_tokens.len() {
    match &design_tokens[end_idx].kind {
        TokenKind::Semicolon if depth == 0 => break,
        TokenKind::RBrace => break,
        TokenKind::LParen | TokenKind::LBracket => {
            depth += 1;
            end_idx += 1;
        }
        TokenKind::RParen | TokenKind::RBracket => {
            depth -= 1;
            end_idx += 1;
        }
        TokenKind::Ident(_) => {
            if depth == 0 {
                let prev_is_dot =
                    end_idx > 0 && matches!(&design_tokens[end_idx - 1].kind, TokenKind::Dot);
                if !prev_is_dot {
                    break;
                }
            }
            end_idx += 1;
        }
        _ => end_idx += 1,
    }
}
```

After:
```rust
let mut end_idx = start_idx + 1;
let mut depth: i32 = 0;
while end_idx < design_tokens.len() {
    match &design_tokens[end_idx].kind {
        TokenKind::Semicolon if depth == 0 => break,
        TokenKind::LBrace => {
            depth += 1;
            end_idx += 1;
        }
        TokenKind::RBrace if depth > 0 => {
            depth -= 1;
            end_idx += 1;
        }
        TokenKind::RBrace => break,
        TokenKind::LParen | TokenKind::LBracket => {
            depth += 1;
            end_idx += 1;
        }
        TokenKind::RParen | TokenKind::RBracket => {
            depth -= 1;
            end_idx += 1;
        }
        TokenKind::Ident(_) => {
            if depth == 0 {
                let prev_is_dot =
                    end_idx > 0 && matches!(&design_tokens[end_idx - 1].kind, TokenKind::Dot);
                if !prev_is_dot {
                    break;
                }
            }
            end_idx += 1;
        }
        _ => end_idx += 1,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p omnilang-core parse_expression_with_brace_object_argument`
Expected: PASS — expression with object literal parses correctly

- [ ] **Step 5: Run full parser test suite**

Run: `cargo test -p omnilang-core parse_`
Expected: All parse tests pass (no regressions in existing expression parsing)

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/design.rs
git commit -m "fix: add brace depth tracking to design token expression scanner

The expression boundary scanner only tracked (...) and [...] nesting but
treated any RBrace as a hard terminator. Expressions containing object
literal arguments (e.g., store.getConfig({theme: \"dark\"})) were truncated.

Now LBrace/RBrace are tracked with the same depth logic as LParen/RParen.
RBrace at depth=0 still terminates (design block end). RBrace at depth>0
decrements (object literal close within expression).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task B2: Fix error offset in sub-parser expression failures

**Files:**
- Modify: `crates/omnilang-core/src/parser/design.rs:349-353`

**Why:** When `parse_expr()` fails inside `parse_design_value_expression()`, the error reports `offset: 0`, pointing to the design block's `{` instead of the failed expression. The correct offset is available from the expression's first token span.

- [ ] **Step 1: Make the fix**

In `crates/omnilang-core/src/parser/design.rs`, in `parse_design_value_expression()`, replace lines 349-353:

Before:
```rust
let mut expr_parser = Parser::new(expr_tokens, design_body_full);
let expr = expr_parser.parse_expr().map_err(|e| DesignParseError {
    message: format!("failed to parse expression in token value: {}", e.message),
    offset: 0,
})?;
```

After:
```rust
let mut expr_parser = Parser::new(expr_tokens, design_body_full);
let error_offset = expr_tokens.first().map(|t| t.span.start).unwrap_or(offset);
let expr = expr_parser.parse_expr().map_err(|e| DesignParseError {
    message: format!("failed to parse expression in token value: {}", e.message),
    offset: error_offset,
})?;
```

- [ ] **Step 2: Run parser tests to verify no regression**

Run: `cargo test -p omnilang-core parse_token_value`
Expected: All pass

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/parser/design.rs
git commit -m "fix: use expression start token offset for parse error location

Previously, expression parse failures in design token values always
reported offset 0, which mapped to the design block's opening brace
rather than the actual expression location. Use the first expression
token's span start as the error offset so the diagnostic points to
the correct source location.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group C: Codegen Hardening (MEDIUM Priority)

### Task C1: Consolidate dynamic tokens into single `__omni_effect` per design block

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs:278-298`

**Why:** Each dynamic token currently creates its own `__omni_effect` subscription. At realistic scale (1-5 tokens) this is negligible, but the signal runtime calls each effect synchronously, and each `setProperty` crosses the JS/DOM boundary. Grouping into one `__omni_effect` per design block reduces O(N) subscriptions to O(1).

- [ ] **Step 1: Rewrite `emit_dynamic_token_effects`**

In `omnilang/src/codegen/omniview/design.rs`, replace the function body (lines 285-297):

Before:
```rust
    let mut js = String::new();
    for (name, expr) in tokens {
        let js_expr = expr_to_js(expr);
        let _ = writeln!(
            js,
            "__omni_effect(() => {{\n\
             \x20 const root = document.documentElement;\n\
             \x20 root.style.setProperty(\"--{}\", __omni_sanitizeCss({}));\n\
             }});\n",
            name, js_expr
        );
    }
    js
```

After:
```rust
    if tokens.is_empty() {
        return String::new();
    }
    let mut js = String::new();
    // Emit all dynamic token setProperty calls inside a single __omni_effect
    // block to reduce effect subscription count from O(N) to O(1).
    // setProperty with the same value is a browser no-op, so re-setting
    // unaffected tokens when any signal changes has zero visual cost.
    js.push_str("__omni_effect(() => {\n");
    js.push_str("  const root = document.documentElement;\n");
    for (name, expr) in tokens {
        let js_expr = expr_to_js(expr);
        let _ = writeln!(
            js,
            "  root.style.setProperty(\"--{}\", __omni_sanitizeCss({}));",
            name, js_expr
        );
    }
    js.push_str("\n});\n");
    js
```

- [ ] **Step 2: Update the doc comment**

Replace lines 263-277 (the doc comment) with:

```rust
/// Emit a single JS `__omni_effect` block for all dynamic design tokens.
///
/// Takes a pre-collected slice of `(token_name, ExprIR)` pairs and a
/// function that converts each `ExprIR` into its JavaScript expression string.
///
/// Generates a single effect containing all `setProperty` calls:
/// ```javascript
/// __omni_effect(() => {
///   const root = document.documentElement;
///   root.style.setProperty("--primary", __omni_sanitizeCss(expr1));
///   root.style.setProperty("--accent", __omni_sanitizeCss(expr2));
/// });
/// ```
///
/// This sets CSS custom properties on `<html>` at runtime, giving them
/// the highest CSS specificity (inline style on root element).
```

- [ ] **Step 3: Run regression tests**

Run: `cargo test -p omnilang design`
Expected: All design codegen tests pass (test snapshots may need updating with `cargo insta review`)

- [ ] **Step 4: Update insta snapshots if needed**

If snapshot tests fail, run:
```bash
cargo test -p omnilang design
# If snapshot mismatches, review and accept:
cargo insta review
```

- [ ] **Step 5: Commit**

```bash
git add om nilang/src/codegen/omniview/design.rs
git commit -m "perf: consolidate dynamic token effects into single __omni_effect block

Previously each dynamic token created its own __omni_effect subscription,
producing O(N) effect runs and setProperty calls per signal change.
Now all dynamic tokens in a design block share one __omni_effect,
reducing subscriptions to O(1).

setProperty with the same value is a browser no-op, so re-setting
unaffected tokens when a different signal changes has zero visual cost.
At typical scale (1-5 dynamic tokens) this is not measurable, but it
structurally prevents O(N) scaling if dynamic token adoption grows.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task C2: Add `document.documentElement` null guard in dynamic token effects

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs:290-292` (the new location after C1)

**Why:** Dynamic token effects access `document.documentElement` without a null check. While this is always non-null in a browser, it can be null in SSR, test runners (jsdom), or Web Worker contexts. Inline style effects in `style.rs` already guard with `if (el)` — this adds the same defense-in-depth pattern.

- [ ] **Step 1: Add the null guard**

In `omnilang/src/codegen/omniview/design.rs`, in `emit_dynamic_token_effects()`, modify the generated JS template (after applying C1's consolidation, the line will be):

Before:
```rust
    js.push_str("__omni_effect(() => {\n");
    js.push_str("  const root = document.documentElement;\n");
```

After:
```rust
    js.push_str("__omni_effect(() => {\n");
    js.push_str("  const root = document.documentElement;\n");
    js.push_str("  if (!root) return;\n");
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang design`
Expected: All pass (snapshot may need update)

- [ ] **Step 3: Commit**

```bash
git add om nilang/src/codegen/omniview/design.rs
git commit -m "fix: add document.documentElement null guard to dynamic token effects

Consistent with the existing inline style effect guard in style.rs.
Prevents TypeError in SSR, test runner, or Worker contexts where
document.documentElement may be null.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task C3: Apply `sanitize_css_value()` to static token `:root` values

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs:79`

**Why:** Class properties (`emit_class_css`) and variant properties (`emit_variant_css`) both apply `sanitize_css_value()` to their values, but static token values in `:root` declarations do not. While not exploitable in modern browsers (CSS `url()` with `javascript:` protocol is treated as a broken resource), this is a defense-in-depth inconsistency.

- [ ] **Step 1: Add sanitization**

In `omnilang/src/codegen/omniview/design.rs`, in `emit_design_css_impl()`, line 79:

Before:
```rust
            if let TokenValueIR::Static(s) = &token.value {
                let _ = writeln!(css, "  --{}: {};", name, s);
            }
```

After:
```rust
            if let TokenValueIR::Static(s) = &token.value {
                let sanitized = sanitize_css_value(s);
                let _ = writeln!(css, "  --{}: {};", name, sanitized);
            }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang design`
Expected: All pass (snapshot may need update since sanitization may normalize values)

- [ ] **Step 3: Commit**

```bash
git add om nilang/src/codegen/omniview/design.rs
git commit -m "fix: apply sanitize_css_value to static token :root values

Aligns the static token CSS path with class/variant property emission,
which already sanitize values. Defense-in-depth consistency — no known
exploit in modern browsers but prevents value-level injection vectors
from reaching CSS output.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group D: Dead Code & API Cleanup (MEDIUM Priority)

### Task D1: Remove `resolve_token_value()` dead code

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/design.rs:117-131`

**Why:** `resolve_token_value()` has `#[allow(dead_code)]` and zero callers across the entire project. It was retained as a "convenience wrapper" for a speculative future where class/variant properties might contain dynamic expressions. Dead code with suppressed lint warnings is a maintenance trap — if `TokenValueIR` changes, this function silently becomes wrong with no compiler or clippy feedback.

- [ ] **Step 1: Verify zero callers**

Run: `cd crates/omnilang-core && cargo check 2>&1`
Expected: No compilation errors. Then:
Run: `rg "resolve_token_value\(" crates/omnilang-core/src/ --include '*.rs'`
Expected: Only the definition at `ir/lower/design.rs:121`, no call sites

- [ ] **Step 2: Remove the function**

In `crates/omnilang-core/src/ir/lower/design.rs`, delete lines 117-131 entirely:

Remove this block:
```rust
/// Resolve a `TokenValue` to a CSS string using the compile-time constant map.
/// Convenience wrapper for places that still need a plain string (e.g. class
/// / variant property resolution where dynamic expressions are Phase 2+).
#[allow(dead_code)]
fn resolve_token_value(
    value: &TokenValue,
    const_values: &FxHashMap<String, String>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> String {
    match resolve_token_value_ir(value, const_values, variant_map, semantics) {
        TokenValueIR::Static(s) => s,
        TokenValueIR::Dynamic(_) => String::new(),
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p omnilang-core`
Expected: Compiles with zero warnings

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/design.rs
git commit -m "refactor: remove dead resolve_token_value() function

Zero callers, suppressed by #[allow(dead_code)]. This speculative wrapper
was retained for a future where class/variant properties might contain
dynamic expressions, but has no consumer. Removing eliminates a
maintenance trap: if TokenValueIR changes, this would silently become
wrong with no compiler or clippy feedback.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task D2: Remove `lower_design()` convenience wrapper

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/design.rs:14-21`
- Check: `crates/omnilang-core/src/ir/lower/mod.rs` (if re-exported or used)
- Check: Any integration test files

**Why:** `lower_design()` creates a default `SemanticResult`, passes it to `lower_design_with_consts()`, then discards it — silently swallowing any diagnostics from expression lowering. Production code always uses `lower_design_with_consts()` directly. The wrapper is a public API trap.

- [ ] **Step 1: Locate all callers**

Run: `rg "lower_design\b" crates/ --include '*.rs' -n`
Expected: Find definition at `ir/lower/design.rs:16`, possible re-export at `ir/lower/mod.rs`, possible test callers.

- [ ] **Step 2: Handle each caller**

For each caller found in Step 1:

**If caller is in `#[cfg(test)]`:** Replace `lower_design(&ast)` with:
```rust
let const_values = FxHashMap::default();
let variant_map = FxHashMap::default();
let mut semantics = SemanticResult::default();
lower_design_with_consts(&ast, &const_values, &variant_map, &mut semantics)
```

**If re-exported from `ir/lower/mod.rs`:** Remove the `pub use` or `pub mod` line.

- [ ] **Step 3: Remove the function**

In `crates/omnilang-core/src/ir/lower/design.rs`, delete lines 14-21:

Remove:
```rust
/// Lower a parsed `design` block AST into a `DesignIR`,
/// using an empty const_values map (backward-compatible wrapper).
pub fn lower_design(ast: &DesignDef) -> DesignIR {
    let const_values = FxHashMap::default();
    let variant_map = FxHashMap::default();
    let mut semantics = SemanticResult::default();
    lower_design_with_consts(ast, &const_values, &variant_map, &mut semantics)
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --workspace`
Expected: Compiles with zero errors

- [ ] **Step 5: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/design.rs
# Add any other modified files (call sites)
git commit -m "refactor: remove lower_design() convenience wrapper

This wrapper silently discarded SemanticResult diagnostics from
expression lowering, creating a trap for anyone using the public API
with TokenValue::Expression tokens. All production callers already
use lower_design_with_consts() directly. Test callers updated to
pass explicit empty contexts.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group E: Documentation (MEDIUM Priority)

### Task E1: Document `is_css_value()` heuristic and naming convention

**Files:**
- Modify: `crates/omnilang-core/src/parser/design.rs:362-387`

**Why:** The `is_css_value()` heuristic treats all-lowercase identifiers as CSS literal values. A `const brand = "#fff"` with `token x = brand;` silently produces `--x: brand` (literal string) instead of `--x: #fff`. While the workaround (PascalCase/UPPER_SNAKE_CASE) is documented in ADR-047, the code itself should carry a clear warning.

- [ ] **Step 1: Add doc comment above `is_css_value()`**

In `crates/omnilang-core/src/parser/design.rs`, replace lines 362-363:

Before:
```rust
/// Returns true if the identifier looks like a static CSS value
/// rather than a variable/constant reference.
fn is_css_value(s: &str) -> bool {
```

After:
```rust
/// Returns true if the identifier looks like a static CSS value
/// rather than a variable/constant reference.
///
/// **Naming convention**: This heuristic classifies identifiers containing
/// uppercase characters as variable references and all-lowercase identifiers
/// as CSS literals. To reference a compile-time constant in a token value,
/// use PascalCase or UPPER_SNAKE_CASE (e.g., `BRAND`, `SpacingUnit`).
/// Lowercase names like `brand` or `primary` are treated as CSS keywords.
///
/// See ADR-047 (Phase 1 Dynamic Design Tokens) for the design rationale.
/// If this convention proves confusing in practice, a future language version
/// may add explicit reference syntax (e.g., `$BRAND` or `@brand`).
fn is_css_value(s: &str) -> bool {
```

- [ ] **Step 2: Verify no compilation warnings**

Run: `cargo check -p omnilang-core`
Expected: Clean

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/parser/design.rs
git commit -m "docs: document is_css_value() heuristic and naming convention

The all-lowercase = CSS literal heuristic is a known tradeoff from
ADR-047. Adding an explicit doc comment with the PascalCase/UPPER_SNAKE_CASE
convention so future maintainers (and AI code generators) immediately
understand the naming requirement for const references in token values.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Execution Order

```
A1 (reject dynamic theme values) → A2 (const-kind validation) → A3 (verify H1 closed)
    ↓
B1 ({...} depth tracking) → B2 (error offset fix)
    ↓
C1 (consolidate effects) → C2 (null guard) → C3 (sanitize static values)
    ↓
D1 (remove dead resolve_token_value) → D2 (remove lower_design wrapper)
    ↓
E1 (document is_css_value heuristic)
```

Each group is independent; tasks within groups are sequential (later tasks may depend on earlier refactors).

**Total estimated effort**: ~3-4 hours for all 11 tasks (most are XS-S).

## Verification Checklist

After all tasks are complete, run:

```bash
# Full workspace test suite
cargo test --workspace

# Clippy lint (zero warnings expected)
cargo clippy --workspace -- -D warnings

# Check design token tests specifically
cargo test -p omnilang-core design
cargo test -p omnilang design

# Verify no dead_code warnings remain in changed files
cargo check -p omnilang-core 2>&1 | grep -i "dead_code" || echo "No dead code warnings"
```
