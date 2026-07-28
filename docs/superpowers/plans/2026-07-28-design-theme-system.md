# Design Theme System Refactoring — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the dead `theme` keyword and extend `design` blocks with `@themes` declarations + per-theme token values, generating `[data-theme]` CSS scoping + `@media (prefers-color-scheme)` + `__omni_theme` JS runtime for pure CSS-cascade theme switching.

**Architecture:** Theme switching moves from JS signal-cascade (`store getter → setProperty() per variable`) to CSS cascade (`data-theme` attribute → CSS variable override). The `design` block becomes the single source of truth for all design tokens including their theme variants. The old `theme` keyword is removed entirely — it was never lowered to IR or codegen.

**Tech Stack:** Rust (compiler pipeline), OmniLang parser (string-based for design blocks), CSS custom properties, JS runtime injection

**Design Reference:** See `.architecture/reviews/compiler-architect-omniview-style-system.md` Section C2 for full problem analysis.

---

## File Map

| File | Role | Action |
|------|------|--------|
| `crates/omnilang-core/src/lexer/keywords.rs:63` | `"theme"` keyword mapping | Remove |
| `crates/omnilang-core/src/lexer/mod.rs:1149-1150` | Theme keyword test | Remove |
| `crates/omnilang-core/src/parser/ast/motion.rs:314-322` | `ThemeDef`, `ThemeCategory` structs | Remove |
| `crates/omnilang-core/src/parser/ast/motion.rs:3` | Doc comment mentioning ThemeDef | Update |
| `crates/omnilang-core/src/parser/ast/mod.rs:475` | `pub themes: Vec<ThemeDef>` | Remove |
| `crates/omnilang-core/src/parser/top_level/store.rs:69-106` | `parse_theme_def()` | Remove |
| `crates/omnilang-core/src/parser/top_level/mod.rs:40-41` | `TokenKind::Theme` dispatch | Remove |
| `crates/omnilang-core/src/parser/mod.rs:272` | `TokenKind::Theme` in TOP_LEVEL | Remove |
| `crates/omnilang-core/src/parser/stmt.rs:558` | `TokenKind::Theme => Some("theme")` | Remove |
| `crates/omnilang-core/src/validate.rs:136` | `!module.themes.is_empty()` | Remove |
| `crates/omnilang-core/src/parser/ast/design.rs` | `DesignDef`, `TokenDef` | Extend |
| `crates/omnilang-core/src/parser/design.rs` | Design block parser | Extend |
| `crates/omnilang-core/src/ir/design.rs` | `DesignIR`, `TokenIR` | Extend |
| `crates/omnilang-core/src/ir/lower/design.rs` | Design lowering | Extend |
| `omnilang/src/codegen/omniview/design.rs` | CSS + JS codegen | Extend |
| `docs/omniview/10-design-system.md` | Design system docs | Update |
| Test files (5 lint tests) | `themes: vec![]` fields | Remove |

---

### Task 1: Remove `theme` keyword — lexer

**Files:**
- Modify: `crates/omnilang-core/src/lexer/keywords.rs:63`
- Modify: `crates/omnilang-core/src/lexer/mod.rs:1149-1150`

- [ ] **Step 1: Remove keyword mapping**

In `crates/omnilang-core/src/lexer/keywords.rs`, remove line 63:
```rust
        "theme" => Some(TokenKind::Theme),
```

Any `.omni` file using the `theme` keyword will now produce a standard parse error like "unexpected token" instead of being recognized as a keyword.

- [ ] **Step 2: Remove keyword test**

In `crates/omnilang-core/src/lexer/mod.rs`, remove lines 1149-1150:
```rust
    fn test_keyword_theme() {
        assert_token("theme", TokenKind::Theme);
    }
```

- [ ] **Step 3: Build and verify lexer compiles**

Run: `cargo build -p omnilang-core 2>&1`
Expected: Build succeeds (will have errors from other files still referencing `Theme`)

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/lexer/keywords.rs crates/omnilang-core/src/lexer/mod.rs
git commit -m "refactor: remove 'theme' keyword from lexer"
```

---

### Task 2: Remove `theme` keyword — AST types

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/motion.rs:3, 314-322`
- Modify: `crates/omnilang-core/src/parser/ast/mod.rs:475`

- [ ] **Step 1: Remove ThemeDef and ThemeCategory structs**

In `crates/omnilang-core/src/parser/ast/motion.rs`, remove lines 314-322:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct ThemeDef {
    pub categories: Vec<ThemeCategory>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeCategory {
    pub name: String,
    pub values: Vec<(String, String)>,
}
```

- [ ] **Step 2: Update doc comment on motion.rs**

In `crates/omnilang-core/src/parser/ast/motion.rs`, change line 3 from:
```rust
//! Contains: MotionDef, TimelineDef, EasingExpr, StaggerCall, PlaybackProp, StoreDef, ThemeDef, etc.
```
to:
```rust
//! Contains: MotionDef, TimelineDef, EasingExpr, StaggerCall, PlaybackProp, StoreDef.
```

- [ ] **Step 3: Remove themes field from Module**

In `crates/omnilang-core/src/parser/ast/mod.rs`, remove line 475:
```rust
    pub themes: Vec<ThemeDef>,
```

- [ ] **Step 4: Verify AST compiles**

Run: `cargo build -p omnilang-core 2>&1 | head -40`
Expected: Errors only from parser/top_level and other consumers of removed items

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/motion.rs crates/omnilang-core/src/parser/ast/mod.rs
git commit -m "refactor: remove ThemeDef and ThemeCategory AST types"
```

---

### Task 3: Remove `theme` keyword — parser consumer sites

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/store.rs:69-106`
- Modify: `crates/omnilang-core/src/parser/top_level/mod.rs:40-41`
- Modify: `crates/omnilang-core/src/parser/mod.rs:272`
- Modify: `crates/omnilang-core/src/parser/stmt.rs:558`
- Modify: `crates/omnilang-core/src/validate.rs:136`
- Modify: `crates/omnilang-core/src/semantic/lints/computed_purity.rs:109, 198`
- Modify: `crates/omnilang-core/src/semantic/lints/dead_code.rs:267, 339`
- Modify: `crates/omnilang-core/src/semantic/lints/prop_immutability.rs:117`
- Modify: `crates/omnilang-core/src/semantic/lints/unused_import.rs:146, 225`

- [ ] **Step 1: Remove parse_theme_def() function**

In `crates/omnilang-core/src/parser/top_level/store.rs`, remove the entire `parse_theme_def` method (lines 67-106):
```rust
    // ── Theme ──────────────────────────────────────────

    pub fn parse_theme_def(&mut self) -> Result<ThemeDef, ParseError> {
        self.advance(); // 'theme'
        self.expect_ident("expected theme name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut categories = Vec::new();
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            let cat_name = self.expect_ident("expected category name")?;
            self.expect_token(TokenKind::LBrace, "expected '{'")?;
            let mut values = Vec::new();
            while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
                let key = self.expect_ident("expected key")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let val_t = self.advance();
                let val = match &val_t.kind {
                    TokenKind::StringLiteral(s) => s.clone(),
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        return Err(ParseError {
                            message: "Expected string or identifier value".into(),
                            span: val_t.span,
                        })
                    }
                };
                values.push((key, val));
                if *self.peek_kind() == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            categories.push(ThemeCategory {
                name: cat_name,
                values,
            });
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(ThemeDef { categories })
    }
```

Replace with nothing (the entire `// ── Theme ──` block through the closing `}` is removed).

- [ ] **Step 2: Remove TokenKind::Theme dispatch in top-level parser**

In `crates/omnilang-core/src/parser/top_level/mod.rs`, remove lines 40-41:
```rust
                    TokenKind::Theme => {
                        module.themes.push(self.parse_theme_def()?);
                    }
```

- [ ] **Step 3: Remove TokenKind::Theme from TOP_LEVEL skip list**

In `crates/omnilang-core/src/parser/mod.rs`, remove line 272:
```rust
            TokenKind::Theme,
```

- [ ] **Step 4: Remove TokenKind::Theme from statement context**

In `crates/omnilang-core/src/parser/stmt.rs`, remove line 558:
```rust
        TokenKind::Theme => Some("theme"),
```

- [ ] **Step 5: Remove themes check from validate.rs**

In `crates/omnilang-core/src/validate.rs`, change line 136 from:
```rust
        !module.components.is_empty() || !module.stores.is_empty() || !module.themes.is_empty()
```
to:
```rust
        !module.components.is_empty() || !module.stores.is_empty()
```

- [ ] **Step 6: Remove themes fields from all lint tests**

In `crates/omnilang-core/src/semantic/lints/computed_purity.rs`:
- Line 109: Remove `themes: vec![],`
- Line 198: Remove `themes: vec![],`

In `crates/omnilang-core/src/semantic/lints/dead_code.rs`:
- Line 267: Remove `themes: vec![],`
- Line 339: Remove `themes: vec![],`

In `crates/omnilang-core/src/semantic/lints/prop_immutability.rs`:
- Line 117: Remove `themes: vec![],`

In `crates/omnilang-core/src/semantic/lints/unused_import.rs`:
- Line 146: Remove `themes: vec![],`
- Line 225: Remove `themes: vec![],`

- [ ] **Step 7: Build and fix remaining compilation errors**

Run: `cargo build -p omnilang-core 2>&1`
Expected: Build succeeds, 0 errors
Run: `cargo clippy -p omnilang-core -- -D warnings 2>&1`
Expected: Clippy passes

- [ ] **Step 8: Run tests**

Run: `cargo test -p omnilang-core 2>&1`
Expected: All existing tests pass (baseline: 804+ tests)

- [ ] **Step 9: Commit**

```bash
git add crates/omnilang-core/src/parser/top_level/store.rs crates/omnilang-core/src/parser/top_level/mod.rs crates/omnilang-core/src/parser/mod.rs crates/omnilang-core/src/parser/stmt.rs crates/omnilang-core/src/validate.rs crates/omnilang-core/src/semantic/lints/
git commit -m "refactor: remove all theme keyword consumer sites"
```

---

### Task 4: Remove TokenKind::Theme variant (deferred cleanup)

**Files:**
- Modify: `crates/omnilang-core/src/lexer/mod.rs` (TokenKind enum)

Note: This task depends on confirming that `TokenKind::Theme` has zero remaining references after Tasks 1-3. Only proceed if `cargo build` succeeds with the variant removed.

- [ ] **Step 1: Verify zero remaining references**

Run: `grep -r "TokenKind::Theme" crates/omnilang-core/src/`
Expected: No output (zero references)

- [ ] **Step 2: Remove the enum variant**

Find `Theme` variant in the `TokenKind` enum in `crates/omnilang-core/src/lexer/mod.rs` and remove it. The variant declaration looks like:
```rust
    Theme,
```

- [ ] **Step 3: Build and test**

Run: `cargo build -p omnilang-core 2>&1 && cargo test -p omnilang-core 2>&1`
Expected: Build succeeds, all tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/lexer/mod.rs
git commit -m "refactor: remove TokenKind::Theme enum variant"
```

---

### Task 5: Extend DesignDef AST — themes config and per-theme tokens

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/design.rs`

- [ ] **Step 1: Extend DesignDef with themes fields**

Change `DesignDef` from:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignDef {
    pub name: String,
    pub tokens: Vec<TokenDef>,
    pub classes: Vec<DesignClassDef>,
    pub variants: Vec<VariantDef>,
}
```
to:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignDef {
    pub name: String,
    /// Theme names declared via `@themes light, dark;`
    #[serde(default)]
    pub themes: Vec<String>,
    /// Default theme from `@default-theme light;`
    #[serde(default)]
    pub default_theme: Option<String>,
    pub tokens: Vec<TokenDef>,
    pub classes: Vec<DesignClassDef>,
    pub variants: Vec<VariantDef>,
}
```

- [ ] **Step 2: Extend TokenDef with per-theme values**

Change `TokenDef` from:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenDef {
    pub name: String,
    pub token_type: TokenType,
    pub value: String,
}
```
to:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenDef {
    pub name: String,
    pub token_type: TokenType,
    /// Default value (used when no theme is active, or for the default theme)
    pub value: String,
    /// Per-theme value overrides: theme name → CSS value
    /// e.g. `token primary: color = "#6366f1" dark "#a5b4fc";`
    ///   → value = "#6366f1", theme_values = {"dark": "#a5b4fc"}
    #[serde(default)]
    pub theme_values: Vec<(String, String)>,
}
```

- [ ] **Step 3: Build and verify**

Run: `cargo build -p omnilang-core 2>&1`
Expected: Compilation errors (consumers not yet updated) — confirms the new fields are wired in

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/design.rs
git commit -m "feat: extend DesignDef and TokenDef AST with theme support"
```

---

### Task 6: Extend design parser — `@themes`, `@default-theme`, per-theme token values

**Files:**
- Modify: `crates/omnilang-core/src/parser/design.rs`

- [ ] **Step 1: Add @themes and @default-theme parsing to parse_design_body()**

In `crates/omnilang-core/src/parser/design.rs`, update `parse_design_body()` to parse `@themes` and `@default-theme` directives before tokens/classes/variants.

Add two new `else if` branches to the main loop in `parse_design_body()`, positioned before the `token`/`class`/`variant` branches:

```rust
    let mut themes: Vec<String> = Vec::new();
    let mut default_theme: Option<String> = None;

    while !remaining.is_empty() {
        remaining = skip_ws_and_comments(remaining);

        if remaining.starts_with("@themes ") {
            let (t, rest) = parse_themes_directive(remaining)?;
            themes = t;
            remaining = rest;
        } else if remaining.starts_with("@default-theme ") {
            let (dt, rest) = parse_default_theme_directive(remaining)?;
            default_theme = Some(dt);
            remaining = rest;
        } else if remaining.starts_with("token ") {
            // ... existing token parsing
```

And update the `Ok(DesignDef { ... })` return to include the new fields:
```rust
    Ok(DesignDef {
        name: String::new(),
        themes,
        default_theme,
        tokens,
        classes,
        variants,
    })
```

- [ ] **Step 2: Add parse_themes_directive() function**

Add a new function after `parse_token`:

```rust
/// Parse `@themes light, dark;` — comma-separated theme name list.
fn parse_themes_directive(input: &str) -> ParseResult<(Vec<String>, &str)> {
    let rest = input
        .strip_prefix("@themes ")
        .ok_or_else(|| err("expected '@themes'"))?;
    let mut themes = Vec::new();
    let mut remaining = rest.trim_start();

    loop {
        if remaining.starts_with(';') {
            remaining = &remaining[1..];
            break;
        }
        if remaining.starts_with(',') {
            remaining = remaining[1..].trim_start();
            continue;
        }
        let (name, rest) = parse_ident(remaining)?;
        themes.push(name);
        remaining = rest.trim_start();
    }

    Ok((themes, remaining))
}
```

- [ ] **Step 3: Add parse_default_theme_directive() function**

```rust
/// Parse `@default-theme light;` — single theme name.
fn parse_default_theme_directive(input: &str) -> ParseResult<(String, &str)> {
    let rest = input
        .strip_prefix("@default-theme ")
        .ok_or_else(|| err("expected '@default-theme'"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix(';')
        .ok_or_else(|| err("expected ';'"))?;
    Ok((name, rest))
}
```

- [ ] **Step 4: Extend parse_token() to support per-theme values**

Modify `parse_token()` to accept `dark "#a5b4fc"` after the default value. After parsing the default value and before consuming `;`:

```rust
fn parse_token(input: &str) -> ParseResult<(TokenDef, &str)> {
    let rest = input.strip_prefix("token ").ok_or_else(|| err("expected 'token'"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest.trim_start().strip_prefix(":").ok_or_else(|| err("expected ':'"))?.trim_start();
    let (token_type, rest) = parse_token_type(rest)?;
    let rest = rest.trim_start().strip_prefix("=").ok_or_else(|| err("expected '='"))?.trim_start();
    let (value, rest) = parse_string_or_ident(rest)?;

    // Parse optional per-theme value overrides: `dark "#a5b4fc" light "#ffffff"`
    let mut theme_values: Vec<(String, String)> = Vec::new();
    let mut remaining = rest.trim_start();
    loop {
        if remaining.starts_with(';') {
            remaining = &remaining[1..];
            break;
        }
        // Try parsing `<theme_name> <value>` pair
        let (maybe_theme, after_theme) = match parse_ident(remaining) {
            Ok((ident, r)) => (ident, r),
            Err(_) => break, // Not a theme name, stop
        };
        // Must be followed by a value
        let after_ws = after_theme.trim_start();
        if after_ws.starts_with(';') {
            // This is the ';' — the "theme name" was actually part of the value
            // Put it back: the value consumed too greedily
            break;
        }
        match parse_string_or_ident(after_ws) {
            Ok((theme_val, r)) => {
                theme_values.push((maybe_theme, theme_val));
                remaining = r.trim_start();
            }
            Err(_) => break,
        }
    }

    Ok((TokenDef { name, token_type, value, theme_values }, remaining))
}
```

- [ ] **Step 5: Add parser tests for new syntax**

Add at the end of the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn parse_themes_directive() {
        let src = r#"
            @themes light, dark;
            token primary: color = "#6366f1";
        "#;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.themes, vec!["light", "dark"]);
        assert_eq!(result.tokens.len(), 1);
    }

    #[test]
    fn parse_default_theme_directive() {
        let src = r#"
            @themes light, dark;
            @default-theme light;
            token primary: color = "#6366f1";
        "#;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.default_theme, Some("light".to_string()));
    }

    #[test]
    fn parse_token_with_theme_values() {
        let src = r##"token primary: color = light "#6366f1" dark "#a5b4fc";"##;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.tokens.len(), 1);
        let token = &result.tokens[0];
        assert_eq!(token.name, "primary");
        assert_eq!(token.value, "light");  // first value after '='
        assert_eq!(token.theme_values.len(), 1);
        assert_eq!(token.theme_values[0], ("dark".to_string(), "#a5b4fc".to_string()));
    }

    #[test]
    fn parse_token_with_multiple_theme_values() {
        let src = r##"token surface: color = light "#ffffff" dark "#111827" highContrast "#000000";"##;
        let result = parse_design_body(src).unwrap();
        let token = &result.tokens[0];
        assert_eq!(token.value, "light");
        assert_eq!(token.theme_values.len(), 2);
        assert_eq!(token.theme_values[0], ("dark".to_string(), "#111827".to_string()));
        assert_eq!(token.theme_values[1], ("highContrast".to_string(), "#000000".to_string()));
    }

    #[test]
    fn parse_token_without_theme_values_still_works() {
        let src = r##"token primary: color = "#6366f1";"##;
        let result = parse_design_body(src).unwrap();
        let token = &result.tokens[0];
        assert_eq!(token.value, "#6366f1");
        assert!(token.theme_values.is_empty());
    }
```

- [ ] **Step 6: Fix all existing parser tests**

The existing `parse_color_token()` and similar tests construct `DesignDef` directly. These now need the new fields. Update each test that compares against `DesignDef` to include `themes: vec![], default_theme: None`. But since we only use `parse_design_body().unwrap()` and check individual fields, most existing tests should still pass as-is (they don't construct `DesignDef { ... }` manually).

- [ ] **Step 7: Build and run tests**

Run: `cargo build -p omnilang-core 2>&1`
Expected: Build succeeds (may have errors in lower/codegen — those are Task 7-9)
Run: `cargo test -p omnilang-core --lib parser::design::tests 2>&1`
Expected: All design parser tests pass including new ones

- [ ] **Step 8: Commit**

```bash
git add crates/omnilang-core/src/parser/design.rs
git commit -m "feat: parse @themes, @default-theme, and per-theme token values in design blocks"
```

---

### Task 7: Fix AST consumers — add missing fields everywhere DesignDef is constructed

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/design.rs` (if `DesignDef` is constructed there)
- Modify: Any test files that construct `DesignDef { ... }` manually

- [ ] **Step 1: Find all DesignDef construction sites**

Run: `grep -rn "DesignDef\s*\{" crates/omnilang-core/src/`
Expected: List of files constructing `DesignDef { name: ..., tokens: ..., ... }`

- [ ] **Step 2: Add themes and default_theme to each construction**

For each site, add:
```rust
    themes: vec![],
    default_theme: None,
```

between `name` and `tokens` fields.

- [ ] **Step 3: Build and verify**

Run: `cargo build -p omnilang-core 2>&1`
Expected: Build succeeds, 0 errors

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/
git commit -m "fix: add themes/default_theme fields to all DesignDef construction sites"
```

---

### Task 8: Extend IR types — DesignIR and TokenIR theme support

**Files:**
- Modify: `crates/omnilang-core/src/ir/design.rs`

- [ ] **Step 1: Extend DesignIR**

Add `themes` and `default_theme` fields after `name`:

```rust
pub struct DesignIR {
    pub name: String,
    /// Theme names declared via `@themes` (e.g. `["light", "dark"]`)
    #[serde(default)]
    pub themes: Vec<String>,
    /// Default theme from `@default-theme` (e.g. `Some("light")`)
    #[serde(default)]
    pub default_theme: Option<String>,
    pub tokens: FxHashMap<String, TokenIR>,
    pub classes: Vec<ClassIR>,
    pub variants: Vec<VariantIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}
```

- [ ] **Step 2: Extend TokenIR**

Add `theme_values` field after `token_type`:

```rust
pub struct TokenIR {
    pub value: String,
    pub token_type: TokenTypeIR,
    /// Per-theme value overrides: theme name → resolved CSS value
    #[serde(default)]
    pub theme_values: FxHashMap<String, String>,
    #[serde(default)]
    pub source_span: Option<Span>,
}
```

- [ ] **Step 3: Build and verify**

Run: `cargo build -p omnilang-core 2>&1`
Expected: Compilation errors from lower/codegen (will fix in Tasks 9-10)

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ir/design.rs
git commit -m "feat: extend DesignIR and TokenIR with theme fields"
```

---

### Task 9: Extend IR lowering — lower_design() handles theme fields

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/design.rs`

- [ ] **Step 1: Extend lower_design() to pass through theme fields**

In `lower_design()`, update the `DesignIR` construction to include the new fields:

```rust
pub fn lower_design(ast: &DesignDef) -> DesignIR {
    let tokens: FxHashMap<String, TokenIR> = ast
        .tokens
        .iter()
        .map(|t| {
            let token_type = match t.token_type {
                // ... existing type mapping ...
            };
            // Convert theme_values from Vec<(String, String)> to FxHashMap
            let theme_values: FxHashMap<String, String> =
                t.theme_values.iter().cloned().collect();
            (
                t.name.clone(),
                TokenIR {
                    value: t.value.clone(),
                    token_type,
                    theme_values,
                    source_span: None,
                },
            )
        })
        .collect();

    // ... existing class/variant lowering ...

    DesignIR {
        name: ast.name.clone(),
        themes: ast.themes.clone(),
        default_theme: ast.default_theme.clone(),
        tokens,
        classes,
        variants,
        source_span: None,
    }
}
```

- [ ] **Step 2: Update existing lowering tests**

Update tests in `#[cfg(test)] mod tests` that construct `DesignIR { ... }` or `TokenIR { ... }` to include the new fields. Example for `make_test_design()`:

```rust
TokenIR {
    value: "#ffffff".to_string(),
    token_type: TokenTypeIR::Color,
    theme_values: FxHashMap::default(),
    source_span: None,
},
```

And for `DesignIR { ... }`:
```rust
DesignIR {
    name: "AppDesign".to_string(),
    themes: vec![],
    default_theme: None,
    tokens,
    classes,
    variants,
    source_span: None,
}
```

- [ ] **Step 3: Add lowering test for theme fields**

Add a new test:

```rust
#[test]
fn lower_design_with_themes() {
    let design = lower_design_source(
        r#"
        design App {
            @themes light, dark;
            @default-theme light;
            token primary: color = light "#6366f1" dark "#a5b4fc";
        }
        "#,
    );
    assert_eq!(design.themes, vec!["light", "dark"]);
    assert_eq!(design.default_theme, Some("light".to_string()));
    let primary = design.tokens.get("primary").expect("should have primary token");
    assert_eq!(primary.value, "light");
    assert_eq!(primary.theme_values.get("dark"), Some(&"#a5b4fc".to_string()));
}
```

NOTE: The `lower_design_source` helper parses with the token-based parser, but design blocks use string-based `parse_design_body()`. The integration test path (`tokenize → parse → lower`) already goes through `lower_design(ast)` where `ast` is a `DesignDef`. The new test needs the full pipeline, not just `parse_design_body`. Adjust the test helper if needed to use the full `lower_source` pattern.

- [ ] **Step 4: Build and run tests**

Run: `cargo build -p omnilang-core 2>&1 && cargo test -p omnilang-core --lib ir::lower::design::tests 2>&1`
Expected: Build succeeds, all design lowering tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/design.rs
git commit -m "feat: lower theme fields from DesignDef to DesignIR"
```

---

### Task 10: Fix all codegen consumers — add theme fields to DesignIR/TokenIR construction

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs` (tests)
- Modify: Any other files constructing `DesignIR` or `TokenIR`

- [ ] **Step 1: Find all TokenIR and DesignIR construction sites**

Run: `grep -rn "TokenIR\s*\{" omnilang/src/ crates/omnilang-core/src/`
Run: `grep -rn "DesignIR\s*\{" omnilang/src/ crates/omnilang-core/src/`
Expected: List of construction sites needing new fields

- [ ] **Step 2: Add theme_values: FxHashMap::default() to all TokenIR constructions**

- [ ] **Step 3: Add themes: vec![], default_theme: None to all DesignIR constructions**

- [ ] **Step 4: Build and verify**

Run: `cargo build --workspace 2>&1`
Expected: Workspace builds, 0 errors

- [ ] **Step 5: Run full test suite**

Run: `cargo test --workspace 2>&1`
Expected: All tests pass (baseline: 804+ tests, may have new ones from Task 9)

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/ crates/omnilang-core/src/
git commit -m "fix: add theme fields to all DesignIR/TokenIR construction sites"
```

---

### Task 11: Extend CSS codegen — emit [data-theme] scoped blocks and @media query

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs`

- [ ] **Step 1: Extend emit_design_css_impl() to emit theme-scoped token blocks**

Replace the existing `:root` token emission block (lines 54-63) with theme-aware emission:

```rust
    // ── Emit token CSS custom property declarations ──
    if !design.tokens.is_empty() {
        let mut sorted: Vec<_> = design.tokens.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        // Build a map: theme_name → Vec<(token_name, value)>
        let mut theme_token_map: FxHashMap<&str, Vec<(&String, &String)>> = FxHashMap::default();
        for (name, token) in &sorted {
            for (theme_name, theme_val) in &token.theme_values {
                theme_token_map
                    .entry(theme_name.as_str())
                    .or_default()
                    .push((name, theme_val));
            }
        }

        // 1. Default theme (or first listed value) on :root — always emitted
        css.push_str(":root {\n");
        for (name, token) in &sorted {
            css.push_str(&format!("  --{}: {};\n", name, token.value));
        }
        css.push_str("}\n\n");

        // 2. Per-theme overrides: [data-theme="themeName"] { --token: value; }
        if !theme_token_map.is_empty() {
            let mut theme_names: Vec<&str> = theme_token_map.keys().copied().collect();
            theme_names.sort();
            for theme_name in &theme_names {
                if let Some(pairs) = theme_token_map.get(theme_name) {
                    css.push_str(&format!("[data-theme=\"{}\"] {{\n", theme_name));
                    for (name, val) in pairs {
                        css.push_str(&format!("  --{}: {};\n", name, val));
                    }
                    css.push_str("}\n\n");
                }
            }
        }

        // 3. @media (prefers-color-scheme: dark) auto-detection
        // Use the dark theme values (if declared), scoped to :root:not([data-theme])
        // so it doesn't override explicit user choice
        if let Some(dark_pairs) = theme_token_map.get("dark") {
            css.push_str("@media (prefers-color-scheme: dark) {\n");
            css.push_str("  :root:not([data-theme]) {\n");
            for (name, val) in dark_pairs {
                css.push_str(&format!("    --{}: {};\n", name, val));
            }
            css.push_str("  }\n");
            css.push_str("}\n\n");
        }
    }
```

The `theme_token_map` variable needs a `use rustc_hash::FxHashMap;` import at the top (already imported in this file).

- [ ] **Step 2: Update existing tests to handle new CSS output**

The existing tests (`emit_class_css_output`, `emit_variant_css_output`) build a `DesignIR` with no themes. Their token blocks should remain `:root { ... }` only, unchanged. But we should verify:

Run: `cargo test -p omnilang -- omniview::design::tests 2>&1`
Expected: All existing tests pass

- [ ] **Step 3: Add test for theme CSS output**

Add a new test in the `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn emit_design_css_with_themes() {
        let mut tokens = FxHashMap::default();
        let mut theme_values_dark = FxHashMap::default();
        theme_values_dark.insert("dark".to_string(), "#a5b4fc".to_string());
        tokens.insert(
            "primary".to_string(),
            TokenIR {
                value: "#6366f1".to_string(),
                token_type: TokenTypeIR::Color,
                theme_values: theme_values_dark,
                source_span: None,
            },
        );

        let design = DesignIR {
            name: "AppDesign".to_string(),
            themes: vec!["light".to_string(), "dark".to_string()],
            default_theme: Some("light".to_string()),
            tokens,
            classes: vec![],
            variants: vec![],
            source_span: None,
        };

        let css = emit_design_css(&design, "AppDesign").expect("should emit CSS");

        // Should contain :root default
        assert!(css.contains(":root {\n  --primary: #6366f1;\n}"));
        // Should contain [data-theme="dark"] override
        assert!(css.contains("[data-theme=\"dark\"] {\n  --primary: #a5b4fc;\n}"));
        // Should contain @media (prefers-color-scheme: dark)
        assert!(css.contains("@media (prefers-color-scheme: dark) {"));
        assert!(css.contains(":root:not([data-theme]) {"));
    }

    #[test]
    fn emit_design_css_without_themes_unchanged() {
        // Token with no theme_values — should only emit :root block
        let mut tokens = FxHashMap::default();
        tokens.insert(
            "radiusMd".to_string(),
            TokenIR {
                value: "8px".to_string(),
                token_type: TokenTypeIR::Size,
                theme_values: FxHashMap::default(),
                source_span: None,
            },
        );

        let design = DesignIR {
            name: "Simple".to_string(),
            themes: vec![],
            default_theme: None,
            tokens,
            classes: vec![],
            variants: vec![],
            source_span: None,
        };

        let css = emit_design_css(&design, "Simple").expect("should emit CSS");
        assert!(css.contains(":root {\n  --radiusMd: 8px;\n}"));
        assert!(!css.contains("[data-theme="), "no theme blocks when no themes declared");
        assert!(!css.contains("@media"), "no media query when no dark theme");
    }
```

- [ ] **Step 4: Build and run tests**

Run: `cargo test -p omnilang -- omniview::design::tests 2>&1`
Expected: All tests pass including new ones

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/design.rs
git commit -m "feat: emit [data-theme] and @media CSS blocks for themed design tokens"
```

---

### Task 12: Generate __omni_theme JS runtime

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs` (or a new file like `theme_runtime.rs`)

- [ ] **Step 1: Add theme runtime injection to Omniview codegen**

In `omnilang/src/codegen/omniview/mod.rs`, find where the main `<script>` block is emitted. Add a new method `emit_theme_runtime()` that generates the JS runtime, and call it from the main codegen entry point when any `DesignIR` has `!design.themes.is_empty()`.

Add the method to `impl OmniViewGenerator` (or equivalent struct):

```rust
    /// Emit the __omni_theme JS runtime for design blocks that declare @themes.
    pub fn emit_theme_runtime(&mut self, designs: &[DesignIR]) {
        // Find all unique theme configurations across all design blocks
        let mut all_themes: Vec<&str> = Vec::new();
        let mut has_dark = false;
        for design in designs {
            for theme in &design.themes {
                if !all_themes.contains(&theme.as_str()) {
                    all_themes.push(theme.as_str());
                }
                if theme == "dark" {
                    has_dark = true;
                }
            }
        }

        if all_themes.is_empty() {
            return;
        }

        let themes_json: Vec<String> = all_themes.iter().map(|t| format!("\"{}\"", t)).collect();
        let themes_array = themes_json.join(", ");

        self.js.push_str(&format!(
            r#"// ═══ Theme Runtime (generated from design @themes) ═══
const __omni_theme = {{
  _themes: [{themes}],
  _current: null,
  _storageKey: "omni-theme",

  get current() {{ return this._current; }},

  set(name) {{
    if (!this._themes.includes(name)) return;
    document.documentElement.dataset.theme = name;
    this._current = name;
    try {{ localStorage.setItem(this._storageKey, name); }} catch (_) {{}}
  }},

  toggle() {{
    const i = this._themes.indexOf(this._current);
    const next = this._themes[(i + 1) % this._themes.length];
    this.set(next);
  }},

  init() {{
    const saved = (() => {{ try {{ return localStorage.getItem(this._storageKey); }} catch (_) {{ return null; }} }})();
    if (saved && this._themes.includes(saved)) {{
      this.set(saved);
      return;
    }}
{media_init}
    // Default: first theme
    if (!this._current) {{
      this.set(this._themes[0]);
    }}
  }}
}};
__omni_theme.init();
"#,
            themes = themes_array,
            media_init = if has_dark {
                r#"    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    if (prefersDark && this._themes.includes("dark")) {
      this.set("dark");
      return;
    }
"#
            } else {
                ""
            }
        ));
    }
```

- [ ] **Step 2: Call emit_theme_runtime from the main codegen entry point**

Find where designs are iterated for CSS emission (likely in `mod.rs` around `emit_design_css` calls). After the CSS emission loop, add:

```rust
        // Emit theme runtime JS if any design block declares @themes
        self.emit_theme_runtime(&all_designs);
```

Where `all_designs` is the collection of `DesignIR` from all modules.

- [ ] **Step 3: Add a test for theme runtime JS generation**

Add a test in the appropriate `#[cfg(test)]` module that verifies:
- `__omni_theme` object is generated when designs have themes
- `__omni_theme` is NOT generated when no designs have themes
- The `_themes` array contains the correct theme names
- `_storageKey` is `"omni-theme"`

- [ ] **Step 4: Build and run tests**

Run: `cargo build --workspace 2>&1 && cargo test --workspace 2>&1`
Expected: Build succeeds, all tests pass

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/
git commit -m "feat: generate __omni_theme JS runtime for design block theme switching"
```

---

### Task 13: Integration test — full pipeline with themed design block

**Files:**
- Create: `crates/omnilang-core/tests/design_theme_test.rs` (or add to existing integration test)

- [ ] **Step 1: Write full-pipeline integration test**

```rust
use omnilang_core::lexer::tokenize;
use omnilang_core::parser::parse;
use omnilang_core::ir::lower::{lower, lower_design};
use omnilang_core::semantic::analyze;

#[test]
fn full_pipeline_design_with_themes() {
    let source = r#"
import omniview;

design AppDesign {
    @themes light, dark;
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token surface: color = "#ffffff" dark "#111827";

    class Card {
        background: token(surface);
    }
}
"#;
    let tokens = tokenize(source);
    let program = parse(tokens, source).expect("parse should succeed");
    let mut semantics = analyze(&program);
    let ir = lower(&program, &mut semantics);

    // Find the design in the IR
    let module = ir.modules.get("input.omni").expect("should have input.omni module");
    let design = module.designs.iter()
        .find(|d| d.name == "AppDesign")
        .expect("should have AppDesign design");

    // Verify theme fields survived the pipeline
    assert_eq!(design.themes, vec!["light", "dark"]);
    assert_eq!(design.default_theme, Some("light".to_string()));

    // Verify token theme values
    let primary = design.tokens.get("primary").expect("should have primary token");
    assert_eq!(primary.value, "#6366f1");
    assert_eq!(primary.theme_values.get("dark"), Some(&"#a5b4fc".to_string()));

    let surface = design.tokens.get("surface").expect("should have surface token");
    assert_eq!(surface.value, "#ffffff");
    assert_eq!(surface.theme_values.get("dark"), Some(&"#111827".to_string()));

    // Verify class still resolves token references
    let card = design.classes.iter()
        .find(|c| c.name == "Card")
        .expect("should have Card class");
    assert!(card.properties.contains_key("background"));

    // No diagnostics should be emitted
    assert!(
        semantics.diagnostics.is_empty(),
        "should have no diagnostics, got: {:?}",
        semantics.diagnostics
    );
}

#[test]
fn design_without_themes_still_works() {
    let source = r#"
import omniview;

design SimpleDesign {
    token primary: color = "#6366f1";
    class Card {
        background: token(primary);
    }
}
"#;
    let tokens = tokenize(source);
    let program = parse(tokens, source).expect("parse should succeed");
    let mut semantics = analyze(&program);
    let ir = lower(&program, &mut semantics);

    let module = ir.modules.get("input.omni").expect("should have input.omni module");
    let design = module.designs.iter()
        .find(|d| d.name == "SimpleDesign")
        .expect("should have SimpleDesign design");

    assert!(design.themes.is_empty());
    assert_eq!(design.default_theme, None);

    let primary = design.tokens.get("primary").expect("should have primary token");
    assert_eq!(primary.value, "#6366f1");
    assert!(primary.theme_values.is_empty());
}
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test --test design_theme_test 2>&1`
Expected: Both tests pass

- [ ] **Step 3: Run full workspace test suite**

Run: `cargo test --workspace 2>&1`
Expected: All 804+ tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/tests/design_theme_test.rs
git commit -m "test: add full-pipeline integration tests for themed design blocks"
```

---

### Task 14: Update design system documentation

**Files:**
- Modify: `docs/omniview/10-design-system.md`

- [ ] **Step 1: Replace the "4. Theme Keyword" section (lines 203-251)**

Remove the old `theme` keyword documentation and replace with new `@themes` documentation:

```markdown
## 4. 主题系统（Theme System）

`design` 块通过 `@themes` 声明和 token 的 per-theme 值原生支持多主题切换。

### 语法

```omni
design AppDesign {
    @themes light, dark;
    @default-theme light;

    // Token: 第一个值是默认，后续是 per-theme 覆盖
    token primary: color = light "#6366f1" dark "#a5b4fc";
    token surface: color = light "#ffffff" dark "#111827";
    token text:    color = light "#111827" dark "#f9fafb";

    // 没有 per-theme 值的 token 在所有主题下相同
    token radiusMd: radius = "8px";
}
```

### 编译输出

```css
/* 默认主题 */
:root {
  --primary: #6366f1;
  --surface: #ffffff;
  --text: #111827;
  --radiusMd: 8px;
}

/* dark 主题覆盖 — 设置 data-theme="dark" 即生效 */
[data-theme="dark"] {
  --primary: #a5b4fc;
  --surface: #111827;
  --text: #f9fafb;
}

/* 系统偏好自动跟随 */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    --primary: #a5b4fc;
    --surface: #111827;
    --text: #f9fafb;
  }
}
```

### 运行时 API

编译器自动生成 `__omni_theme` 全局对象：

```js
__omni_theme.current          // "light" | "dark" — 当前主题名
__omni_theme.set("dark")      // 切换到指定主题
__omni_theme.toggle()         // 在声明的主题之间切换
```

### 在 Omni 代码中使用

```omni
component AppShell {
    fn toggleTheme() {
        __omni_theme.toggle();
    }

    view {
        <button @onPress={toggleTheme}>
            Switch to {if __omni_theme.current == "dark" { "Light" } else { "Dark" }}
        </button>

        <!-- 所有 class/variant 自动跟随主题，无需任何额外代码 -->
        <div class: AppDesign.Card>
            This card automatically follows the theme.
        </div>
    }
}
```

### 设计理念

主题切换是 CSS 级联的职责，不是 JS signal 的职责。设置 `data-theme` 属性即可一次性覆盖所有 CSS 变量，无需逐个 `setProperty()`。
```

- [ ] **Step 2: Commit**

```bash
git add docs/omniview/10-design-system.md
git commit -m "docs: replace theme keyword docs with @themes design-block theme system"
```

---

### Task 15: Migrate learn-omniview — remove manual ThemeStore color mapping

**Files:**
- Modify: `test/projects/learn-omniview/src/design.omni` (add @themes + per-theme values)
- Modify: `test/projects/learn-omniview/src/stores/ThemeStore.omni` (remove color getters)
- Modify: `test/projects/learn-omniview/src/components/ThemeCssVarsDemo.omni` (remove manual style bindings)
- Modify: `test/projects/learn-omniview/src/components/SettingsPage.omni` (if applicable)

- [ ] **Step 1: Add @themes and per-theme values to design.omni**

Read `test/projects/learn-omniview/src/design.omni` to find the existing design block. At the top of the design block, add:

```omni
    @themes light, dark;
    @default-theme light;
```

Then extend relevant color tokens with dark mode values. For tokens like `primary`, `surface`, `textPrimary`, `border`, add `dark "<value>"`:

```omni
    token primary:      color = light "#6366f1" dark "#a5b4fc";
    token surface:      color = light "#ffffff" dark "#111827";
    token textPrimary:  color = light "#111827" dark "#f9fafb";
    token border:       color = light "#d4d4d8" dark "#374151";
    // ... any other color tokens
```

- [ ] **Step 2: Simplify ThemeStore.omni**

Remove all the `getter currentBg`, `getter currentText`, etc. color getters. ThemeStore should only manage the mode label for display and persistence:

```omni
store ThemeStore {
    signal toggleCount: i32 = 0;

    getter modeLabel: String = if __omni_theme.current == "dark" { "Dark" } else { "Light" };
    getter nextLabel: String = if __omni_theme.current == "dark" { "Light" } else { "Dark" };

    action toggle() {
        __omni_theme.toggle();
        toggleCount = toggleCount + 1;
    }

    action setMode(newMode: String) {
        __omni_theme.set(newMode);
    }

    action load() {
        // __omni_theme.init() already handles localStorage
        // Nothing needed here — theme is already restored
    }
}
```

- [ ] **Step 3: Simplify ThemeCssVarsDemo.omni**

Remove the manual `style: { --theme-bg: ThemeStore.currentBg, --theme-text: ..., ... }` block from the `theme-root` div. The CSS variables now come from the design block's theme system automatically. The component should reference design tokens via class/variant:

```omni
    view {
        <div class: wrapper>
            <!-- No more manual --theme-* style binding needed -->
            <div class: theme-root>
                <div class: themed-card>
                    <!-- Uses var(--surface), var(--text) etc from design tokens -->
                </div>
            </div>
        </div>
    }
```

- [ ] **Step 4: Rebuild and verify**

Run: `cargo run -p omnic -- build test/projects/learn-omniview -o test/projects/learn-omniview/build2`
Expected: Build succeeds

Check the generated CSS for `[data-theme="dark"]` blocks and `__omni_theme` JS runtime.

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/src/
git commit -m "refactor: migrate learn-omniview to design-block theme system, remove manual color getters"
```

---

## Self-Review

### 1. Spec Coverage

| Requirement | Task(s) |
|-------------|---------|
| Remove old `theme` keyword | Tasks 1-4 |
| `theme` becomes ordinary parse error | Task 1 (remove keyword mapping) |
| `@themes` declaration in design blocks | Tasks 5-6 |
| `@default-theme` declaration | Tasks 5-6 |
| Per-theme token values | Tasks 5-6, 8-9 |
| `[data-theme]` CSS scoping | Task 11 |
| `@media (prefers-color-scheme)` | Task 11 |
| `__omni_theme` JS runtime | Task 12 |
| Full pipeline integration | Task 13 |
| Documentation update | Task 14 |
| learn-omniview migration | Task 15 |

### 2. Placeholder Scan

No TBDs, TODOs, or "implement later" patterns. All code is concrete and complete.

### 3. Type Consistency

- `DesignDef.themes: Vec<String>` → `DesignIR.themes: Vec<String>` → codegen reads `Vec<String>` ✓
- `DesignDef.default_theme: Option<String>` → `DesignIR.default_theme: Option<String>` → codegen reads `Option<String>` ✓
- `TokenDef.theme_values: Vec<(String, String)>` → `TokenIR.theme_values: FxHashMap<String, String>` (converted during lowering) ✓
- `TokenDef.value: String` → `TokenIR.value: String` → codegen emits as-is ✓

### 4. Gap Check

- **What if a token has `theme_values` but `@themes` is not declared?** — The theme_values would still parse, but no `[data-theme]` blocks would be emitted (since `emit_design_css_impl` only iterates declared themes). This is acceptable: theme_values without @themes declaration are silently ignored.
- **What if `@default-theme` references a theme not in `@themes`?** — This won't cause a CSS error, but validation could be added in a follow-up. Not blocking.
- **Cross-module design imports** — The existing import system clones `DesignIR` including `themes` and `theme_values`, so imported designs carry their theme config. ✓

---

**Plan complete and saved to `docs/superpowers/plans/2026-07-28-design-theme-system.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
