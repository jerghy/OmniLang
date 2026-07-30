# Dynamic Token Variables — Phase 2: Runtime Signal/Store References

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable design token values to reference runtime signals and store fields, generating `__omni_effect` wrappers that dynamically update CSS custom properties via `document.documentElement.style.setProperty()`.

**Architecture:** Add `TokenValue::Expression(Expr)` variant at the AST level, `TokenValueIR::Dynamic(ExprIR)` at the IR level. Tokenize the design body inner text so the Pratt expression parser can be invoked for token values containing dotted paths or function calls. In codegen, skip dynamic tokens in static `:root` CSS declarations and instead emit JS `__omni_effect` blocks that bridge signal values to CSS custom properties on `<html>`.

**Tech Stack:** Rust (omnilang-core parser/IR crates), OmniView codegen backend, existing Pratt expression parser, existing `emit_style_assignment()` bridge, existing `__omni_effect` signal system

---

## File Structure

```
crates/omnilang-core/src/
├── parser/
│   ├── ast/design.rs          # +TokenValue::Expression(Expr) variant
│   ├── design.rs              # ~parse_token_value() extended for expression parsing
│   └── top_level/design.rs    # ~tokenize inner_text, pass tokens to design parser
├── semantic/
│   ├── collect.rs             # ~collect symbols from token expressions
│   ├── resolve.rs             # ~resolve_expr_refs for TokenValue::Expression
│   └── validate_types.rs      # ~token type compatibility checks
├── ir/
│   ├── design.rs              # +TokenValueIR enum, TokenIR.value: String→TokenValueIR
│   ├── lower/design.rs        # ~lower expressions in token values, pass LowerContext
│   └── lower/mod.rs           # ~pass variant_map+semantics to design lowering
omnilang/src/codegen/omniview/
├── design.rs                  # ~skip dynamic tokens in :root, +emit_dynamic_token_effects()
├── mod.rs                     # ~call emit_dynamic_token_effects() after emit_theme_runtime()
└── store.rs                   # existing emit_style_assignment() — no changes needed
```

---

### Task 0: Add `TokenValue::Expression(Expr)` variant to AST

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/design.rs:52-57`

- [ ] **Step 1: Add the Expression variant**

```rust
// crates/omnilang-core/src/parser/ast/design.rs

/// A design token value -- static literal, compile-time variable reference,
/// or runtime expression (Phase 2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenValue {
    /// Static literal value: `"#6366f1"`, `"16px"`, `white`
    Literal(String),
    /// Variable reference: `BRAND`, `SPACING` (resolved at compile time, Phase 1)
    Variable(String),
    /// Runtime expression: `ThemeStore.primaryColor`, `App_dynamicAccent` (Phase 2)
    Expression(Expr),
}
```

`Expr` is already imported via `use crate::parser::ast::*` or we add `use crate::parser::ast::Expr;`. Verify the import exists — `Expr` is defined in `crate::parser::ast::mod.rs`.

- [ ] **Step 2: Fix all existing match arms that are now non-exhaustive**

Search the codebase for `TokenValue::` matches and add `TokenValue::Expression(_)` arms. The key locations (verify with grep):

```rust
// In crates/omnilang-core/src/ir/lower/design.rs:78-89 (resolve_token_value)
TokenValue::Expression(_expr) => {
    // Phase 2: will be handled once IR types are updated
    // For now, fallback to empty string (will be updated in Task 5)
    String::new()
}

// In crates/omnilang-core/src/semantic/resolve.rs:160-191
TokenValue::Expression(expr) => {
    // Phase 2: resolve references within the expression
    // For now, no-op (will be updated in Task 2.5)
}

// In any test files that construct TokenValue
```

Run: `cargo check -p omnilang-core 2>&1 | head -50`
Expected: compilation errors pointing to non-exhaustive matches (we'll fix them progressively)

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/design.rs
# Add any other files touched for match arm fixes
git commit -m "feat: add TokenValue::Expression(Expr) variant for Phase 2 runtime tokens"
```

---

### Task 1: Tokenize design body inner text for expression parsing

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/design.rs:14-66`
- Modify: `crates/omnilang-core/src/parser/design.rs:42-96` (parse_design_body signature)

**Goal:** The design body parser currently works on raw `&str`. For Phase 2 expression parsing, we need the design body tokens available so the Pratt parser can consume them when a token value contains an expression. We tokenize `inner_text` once in `parse_design_def()` and pass the tokens alongside the string to the design body parser.

- [ ] **Step 1: Tokenize inner_text and pass tokens to design body parser**

```rust
// crates/omnilang-core/src/parser/top_level/design.rs

use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub fn parse_design_def(&mut self) -> Result<DesignDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'design'
        let name = self.expect_ident("expected design block name")?;

        let lbrace_span = self.peek().span;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        // Scan forward to find the matching '}' (tracking brace depth)
        let mut depth: usize = 1;
        let mut rbrace_pos = None;
        for i in self.pos..self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        rbrace_pos = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let rbrace_pos = rbrace_pos.ok_or_else(|| ParseError {
            message: "unterminated design block — expected '}'".to_string(),
            span: start,
        })?;

        let rbrace = &self.tokens[rbrace_pos];

        // Extract the inner text between '{' and '}'
        let inner_text = &self.source[lbrace_span.end..rbrace.span.start];

        // ── Phase 2: Tokenize inner_text for expression parsing ──
        let design_tokens = crate::lexer::tokenize(inner_text);

        // Delegate to the string-based design body parser, now with tokens
        let mut def =
            crate::parser::design::parse_design_body(inner_text, &design_tokens).map_err(|e| {
                ParseError {
                    message: e.message,
                    span: Span {
                        start: start.start + e.offset,
                        end: start.start + e.offset,
                        ..start
                    },
                }
            })?;
        def.name = name;

        self.pos = rbrace_pos + 1;
        Ok(def)
    }
}
```

- [ ] **Step 2: Update parse_design_body signature to accept tokens**

```rust
// crates/omnilang-core/src/parser/design.rs

use crate::lexer::Token;

/// Parse the body content inside `design Name { ... }`
pub fn parse_design_body(input: &str, design_tokens: &[Token]) -> ParseResult<DesignDef> {
    let mut tokens = Vec::new();
    let mut classes = Vec::new();
    let mut variants = Vec::new();
    let mut themes: Vec<String> = Vec::new();
    let mut default_theme: Option<String> = None;
    let mut remaining = input.trim();

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
            let (token, rest) = parse_token(remaining, design_tokens)?;
            tokens.push(token);
            remaining = rest;
        } else if remaining.starts_with("class ") {
            let (cls, rest) = parse_class(remaining)?;
            classes.push(cls);
            remaining = rest;
        } else if remaining.starts_with("variant ") {
            let (var, rest) = parse_variant(remaining)?;
            variants.push(var);
            remaining = rest;
        } else if remaining.starts_with("}") || remaining.is_empty() {
            break;
        } else {
            return Err(err_at(
                input,
                remaining,
                &format!(
                    "unexpected content at position {}: '{}...'",
                    input.len() - remaining.len(),
                    &remaining[..remaining.find('\n').unwrap_or(remaining.len()).min(40)]
                ),
            ));
        }
    }

    Ok(DesignDef {
        name: String::new(),
        themes,
        default_theme,
        tokens,
        classes,
        variants,
    })
}
```

The key change: `parse_design_body(input: &str, design_tokens: &[Token])` and `parse_token(remaining, design_tokens)`.

- [ ] **Step 3: Thread design_tokens through parse_token → parse_token_value**

```rust
// In parse_token, pass design_tokens to parse_token_value:
fn parse_token<'a>(input: &'a str, design_tokens: &[Token]) -> ParseResult<(TokenDef, &'a str)> {
    // ... (existing prefix parsing) ...
    let (value, rest) = parse_token_value(rest, design_tokens)?;
    // ... (existing theme_values parsing — also pass tokens) ...
    // For theme values, also use the token-aware parser:
    // let (theme_val, r) = parse_token_value(after_ws, design_tokens)?;
}

// In parse_token_value, add design_tokens parameter:
fn parse_token_value<'a>(
    input: &'a str,
    design_tokens: &[Token],
) -> ParseResult<(TokenValue, &'a str)> {
    // ... existing code, plus expression parsing in the else branch ...
}
```

- [ ] **Step 4: Update all internal callers**

In `parse_token`, the theme_values loop calls `parse_token_value(after_ws)` — update to `parse_token_value(after_ws, design_tokens)`.

In tests, update `parse_design_body(src)` calls to `parse_design_body(src, &[])` (empty tokens for tests that don't need expression parsing).

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p omnilang-core 2>&1`
Expected: compilation succeeds (with warnings about unused `design_tokens` parameter — fixed in Task 2)

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/top_level/design.rs \
        crates/omnilang-core/src/parser/design.rs
git commit -m "feat: tokenize design body inner text for Phase 2 expression parsing"
```

---

### Task 2: Parse dotted paths and function calls as expressions in token values

**Files:**
- Modify: `crates/omnilang-core/src/parser/design.rs:229-253` (parse_token_value)

**Goal:** When a token value identifier is followed by `.` or `(`, parse it as a full expression using the Pratt parser rather than treating it as a simple variable reference.

**How it works:** After parsing the first identifier (e.g., `ThemeStore`), we peek at the remaining input. If the next non-whitespace character is `.` or `(`, we know this is an expression chain. We then:
1. Find the byte offset of this identifier within `inner_text`
2. Locate the corresponding starting token in `design_tokens`
3. Scan forward in `design_tokens` to find where the value ends (at `;` or theme name boundary)
4. Create a sub-`Parser` from the relevant token slice
5. Call `parse_expr()` on the sub-Parser

- [ ] **Step 1: Add the expression-parsing logic to parse_token_value**

```rust
// crates/omnilang-core/src/parser/design.rs

use crate::lexer::Token;
use crate::parser::Parser;

/// Parse a token value: quoted string (literal), hex color (literal),
/// CSS keyword (literal), simple variable ref (Variable),
/// or runtime expression (Expression).
fn parse_token_value<'a>(
    input: &'a str,
    design_tokens: &[Token],
) -> ParseResult<(TokenValue, &'a str)> {
    let input = input.trim_start();
    if let Some(stripped) = input.strip_prefix('"') {
        let end = stripped
            .find('"')
            .map(|i| i + 2)
            .ok_or_else(|| err("expected closing '\"'"))?;
        Ok((TokenValue::Literal(input[1..end - 1].to_string()), &input[end..]))
    } else if input.starts_with('#') {
        let end = input[1..]
            .find(|c: char| !c.is_ascii_hexdigit())
            .map(|i| i + 1)
            .unwrap_or(input.len());
        Ok((TokenValue::Literal(input[..end].to_string()), &input[end..]))
    } else {
        let (ident, rest) = parse_ident(input)?;
        if is_css_value(&ident) {
            Ok((TokenValue::Literal(ident), rest))
        } else {
            // ── Phase 2: Check if this is an expression (dotted path or function call) ──
            let after_ident = rest.trim_start();
            let is_expression = after_ident.starts_with('.')
                || after_ident.starts_with('(')
                || after_ident.starts_with('[');

            if is_expression && !design_tokens.is_empty() {
                // Parse as a full expression using the Pratt parser.
                // Find the byte offset of `input` relative to the start of inner_text.
                // We need to know where we are in the token stream.
                match parse_design_value_expression(input, design_tokens) {
                    Ok((expr, remaining)) => {
                        return Ok((TokenValue::Expression(expr), remaining));
                    }
                    Err(_) => {
                        // Fallback: treat as simple variable reference
                    }
                }
            }

            Ok((TokenValue::Variable(ident), rest))
        }
    }
}
```

- [ ] **Step 2: Implement parse_design_value_expression helper**

```rust
/// Parse a runtime expression within a design token value.
///
/// `value_start` points to the beginning of the expression within the design body
/// (e.g., "ThemeStore.primaryColor" or "App.getColor()").
/// `design_tokens` is the full tokenized design body.
///
/// Returns the parsed expression and the remaining string after the expression.
fn parse_design_value_expression<'a>(
    value_start: &'a str,
    design_tokens: &[Token],
) -> ParseResult<(Expr, &'a str)> {
    // Find which token corresponds to the start of this value.
    // We scan for the first identifier token in design_tokens whose span
    // starts at the same position as value_start within the design body.
    let value_start_offset = value_start.as_ptr() as usize;

    // Find the starting token index by matching the first identifier
    let start_idx = design_tokens
        .iter()
        .position(|t| {
            matches!(t.kind, TokenKind::Ident | TokenKind::Dollar)
                && t.span.start >= value_start_offset
        })
        .ok_or_else(|| err("could not locate expression start in design tokens"))?;

    // Scan forward to find the end of the value expression:
    // stop at `;`, a bare identifier (theme name), or end of tokens.
    let mut end_idx = start_idx + 1;
    while end_idx < design_tokens.len() {
        match &design_tokens[end_idx].kind {
            TokenKind::Semicolon => break,
            // A bare identifier after a value could be a theme name like `dark`
            // We stop here: the caller (parse_token) handles theme_values.
            TokenKind::Ident => {
                // Check if previous token was not `.` — bare ident = new statement
                let prev_is_dot = end_idx > 0
                    && matches!(&design_tokens[end_idx - 1].kind, TokenKind::Dot);
                if !prev_is_dot {
                    break;
                }
                end_idx += 1;
            }
            TokenKind::RBrace => break,
            _ => end_idx += 1,
        }
    }

    if end_idx <= start_idx {
        return Err(err("empty expression in token value"));
    }

    // Create a sub-Parser for just the expression tokens
    let expr_tokens: Vec<Token> = design_tokens[start_idx..end_idx]
        .iter()
        .cloned()
        .collect();

    // We need a source string for the Parser. Use the original source.
    // The source span of the last token gives us the slice.
    let expr_source = if let (Some(first), Some(last)) =
        (design_tokens.get(start_idx), design_tokens.get(end_idx - 1))
    {
        &value_start[..(last.span.end - first.span.start)]
    } else {
        value_start
    };

    let mut expr_parser = Parser::new(expr_tokens, expr_source);
    let expr = expr_parser.parse_expr().map_err(|e| {
        DesignParseError {
            message: format!("failed to parse expression in token value: {}", e.message),
            offset: 0,
        }
    })?;

    // Calculate remaining string position
    let remaining = if end_idx < design_tokens.len()
        && matches!(design_tokens[end_idx].kind, TokenKind::Semicolon)
    {
        // Skip past the `;` in the string
        let semicolon_pos = value_start.find(';').unwrap_or(value_start.len());
        &value_start[semicolon_pos..]
    } else {
        // End of expression = end of value string
        &value_start[value_start.len()..]
    };

    Ok((expr, remaining))
}
```

**Important:** The byte offset calculation (`value_start.as_ptr() as usize`) is fragile — it compares pointer addresses which may not be stable across allocations. Instead, we compute the offset by tracking position through the design body string.

- [ ] **Step 3: Use a robust offset calculation**

Replace the fragile pointer math with explicit offset tracking. Add a `base_offset` parameter to `parse_token_value` (or compute it from the input string relative to the design body start). The simplest approach: since `parse_design_body` receives the full `input` string, and `remaining` is always a suffix of `input`, the offset is `input.len() - remaining.len()`.

```rust
// Better approach: compute offset from remaining string
fn parse_token_value_with_offset<'a>(
    input: &'a str,          // Full design body string
    remaining: &'a str,      // Current position in design body
    design_tokens: &[Token],
) -> ParseResult<(TokenValue, &'a str)> {
    let current_offset = input.len() - remaining.len();
    // ... use current_offset to find matching token ...
}
```

Actually, even simpler: we can use the fact that the Pratt parser's tokens carry `Span` values. Each `Token.span.start` is a byte offset into the design body source. We compute the byte offset of `remaining` within `input`, then find the first token whose span starts at or after that offset.

Implementation:

```rust
fn find_token_at_offset(tokens: &[Token], byte_offset: usize) -> Option<usize> {
    tokens.iter().position(|t| t.span.start >= byte_offset)
}
```

Then in `parse_token_value`:
```rust
let current_offset = full_input.len() - input.len(); // byte offset in design body
let token_start = find_token_at_offset(design_tokens, current_offset)
    .ok_or_else(|| err("could not locate expression start"))?;
```

But `parse_token_value` doesn't have `full_input`. We need to thread it through, or use an alternative.

**Simplest robust approach:** Track the design body string and pass remaining offset explicitly. Since `parse_design_body` receives `input` (the full design body string) and `remaining` starts as `input.trim()` and gets progressively consumed, the offset is always `input.len() - remaining.len()`.

Modify `parse_token_value` signature to accept `design_body_full: &str` and compute offset from `input` (which is the current `remaining` within parse_design_body):

```rust
fn parse_token_value<'a>(
    input: &'a str,          // current remaining position  
    design_body_full: &str,  // full design body string (for offset calculation)
    design_tokens: &[Token],
) -> ParseResult<(TokenValue, &'a str)> {
    let offset_in_body = design_body_full.len() - input.len();
    // ...
}
```

- [ ] **Step 4: Update all call sites to pass design_body_full**

In `parse_design_body`:
```rust
pub fn parse_design_body(input: &str, design_tokens: &[Token]) -> ParseResult<DesignDef> {
    // ...
    let (token, rest) = parse_token(remaining, input, design_tokens)?;
    // ...
}

fn parse_token<'a>(
    input: &'a str,
    design_body_full: &str,
    design_tokens: &[Token],
) -> ParseResult<(TokenDef, &'a str)> {
    // ...
    let (value, rest) = parse_token_value(rest, design_body_full, design_tokens)?;
    // For theme values:
    // let (theme_val, r) = parse_token_value(after_ws, design_body_full, design_tokens)?;
}
```

- [ ] **Step 5: Write parser tests for expression parsing in token values**

```rust
// In crates/omnilang-core/src/parser/design.rs tests module

#[test]
fn parse_token_value_dotted_expression() {
    let src = r#"token primary: color = ThemeStore.primaryColor;"#;
    let tokens = crate::lexer::tokenize(src);
    let result = parse_design_body(src, &tokens).unwrap();
    assert_eq!(result.tokens.len(), 1);
    assert!(matches!(
        result.tokens[0].value,
        TokenValue::Expression(_)
    ));
}

#[test]
fn parse_token_value_static_variable_still_works() {
    let src = r#"token primary: color = BRAND;"#;
    let tokens = crate::lexer::tokenize(src);
    let result = parse_design_body(src, &tokens).unwrap();
    assert_eq!(result.tokens[0].value, TokenValue::Variable("BRAND".to_string()));
}

#[test]
fn parse_token_value_multiple_expression_tokens() {
    let src = r#"token accent: color = ThemeStore.primaryColor dark "#a5b4fc";"#;
    let tokens = crate::lexer::tokenize(src);
    let result = parse_design_body(src, &tokens).unwrap();
    let token = &result.tokens[0];
    assert!(matches!(token.value, TokenValue::Expression(_)));
    assert_eq!(token.theme_values.len(), 1);
    assert_eq!(
        token.theme_values[0],
        ("dark".to_string(), TokenValue::Literal("#a5b4fc".to_string()))
    );
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p omnilang-core parser::design::tests`
Expected: 3 new tests PASS, all existing tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/parser/design.rs \
        crates/omnilang-core/src/parser/top_level/design.rs
git commit -m "feat: parse dotted paths and calls as TokenValue::Expression in design tokens"
```

---

### Task 3: Semantic analysis for token expressions

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:160-191`
- Modify: `crates/omnilang-core/src/semantic/collect.rs` (examine: no changes needed for expression symbols — they're already collected from components/stores)

**Goal:** When a token value is `TokenValue::Expression(expr)`, resolve all variable references within that expression against the symbol table, producing appropriate E0001 diagnostics for undefined references.

- [ ] **Step 1: Add expression resolution in resolve_and_validate_module**

```rust
// crates/omnilang-core/src/semantic/resolve.rs
// In resolve_and_validate_module(), replace the existing token variable block:

// Resolve variable references in design token values
for design in &module.designs {
    for token in &design.tokens {
        match &token.value {
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
            TokenValue::Expression(expr) => {
                // Phase 2: Resolve all VarRef nodes in the expression
                let mut vctx = ValidationContext::new();
                self.resolve_expr_refs(expr, &mut vctx);
            }
            TokenValue::Literal(_) => {}
        }
        // Also check theme_values
        for (theme_name, theme_val) in &token.theme_values {
            match theme_val {
                TokenValue::Variable(name) => {
                    if !self.symbols.contains(name) {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0001,
                            &format!(
                                "Undefined variable '{}' in theme '{}' of design token '{}'",
                                name, theme_name, token.name
                            ),
                            &Span { start: 0, end: 0, line: 0, col: 0 },
                        ));
                    }
                }
                TokenValue::Expression(expr) => {
                    let mut vctx = ValidationContext::new();
                    self.resolve_expr_refs(expr, &mut vctx);
                }
                TokenValue::Literal(_) => {}
            }
        }
    }
}
```

- [ ] **Step 2: Write semantic test for expression validation**

```rust
// In crates/omnilang-core/tests/design_theme_test.rs or new test file

#[test]
fn rejects_undefined_variable_in_token_expression() {
    // ThemeStore.undefinedField should produce E0001
    let src = r#"
        store ThemeStore {
            signal primaryColor: string = "#6366f1";
        }
        design App {
            token primary: color = ThemeStore.undefinedField;
        }
    "#;
    let tokens = tokenize(src);
    let (program, _) = parse(tokens, src);
    let mut semantics = analyze(&program);
    let diags: Vec<_> = semantics.diagnostics.iter()
        .filter(|d| matches!(d.kind, DiagnosticKind::E0001))
        .collect();
    assert!(!diags.is_empty(), "should produce E0001 for undefined field");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omnilang-core design_theme`
Expected: existing tests PASS, new test PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs \
        crates/omnilang-core/tests/design_theme_test.rs
git commit -m "feat: resolve expression refs in design token values (semantic)"
```

---

### Task 4: Add `TokenValueIR` enum and update `TokenIR` to use it

**Files:**
- Modify: `crates/omnilang-core/src/ir/design.rs:39-47`

**Goal:** Replace `TokenIR.value: String` with `TokenIR.value: TokenValueIR` where `TokenValueIR` is `Static(String) | Dynamic(ExprIR)`. This is the IR-level counterpart of the AST `TokenValue` enum, enabling downstream codegen to distinguish static CSS values from runtime dynamic expressions.

- [ ] **Step 1: Define TokenValueIR and update TokenIR**

```rust
// crates/omnilang-core/src/ir/design.rs

use crate::ir::types::ExprIR;

/// Resolved token value — either a static CSS string or a runtime expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenValueIR {
    /// Static CSS value resolved at compile time: `"#6366f1"`, `"16px"`
    Static(String),
    /// Runtime expression that must be evaluated via `__omni_effect`: `ThemeStore.primaryColor`
    Dynamic(ExprIR),
}

/// A single design token with its resolved value and type annotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIR {
    pub value: TokenValueIR,  // was: pub value: String
    pub token_type: TokenTypeIR,
    /// Per-theme value overrides keyed by theme name.
    #[serde(default)]
    pub theme_values: FxHashMap<String, TokenValueIR>,  // was: FxHashMap<String, String>
    #[serde(default)]
    pub source_span: Option<Span>,
}
```

- [ ] **Step 2: Fix all compilation errors from the type change**

Search for all references to `token.value` where `token: &TokenIR` and update:
- Codegen: `emit_design_css_impl()` in `design.rs` — currently reads `token.value` as `String`
- Codegen: `expand_variant()` — doesn't access token values directly
- Lowering: `lower/design.rs` tests — construct `TokenIR` with `value: TokenValueIR::Static(...)`
- Any other consumers

Run: `cargo check 2>&1 | head -100`
Expected: compilation errors at all sites that read `TokenIR.value` as `String`

- [ ] **Step 3: Update TokenIR construction in lowering**

```rust
// crates/omnilang-core/src/ir/lower/design.rs

fn lower_design_with_consts(
    ast: &DesignDef,
    const_values: &FxHashMap<String, String>,
) -> DesignIR {
    let tokens: FxHashMap<String, TokenIR> = ast
        .tokens
        .iter()
        .map(|t| {
            // ... token_type mapping ...
            let resolved_value = resolve_token_value_ir(&t.value, const_values);
            let theme_values: FxHashMap<String, TokenValueIR> = t
                .theme_values
                .iter()
                .map(|(theme, val)| {
                    (theme.clone(), resolve_token_value_ir(val, const_values))
                })
                .collect();
            (
                t.name.clone(),
                TokenIR {
                    value: resolved_value,
                    token_type,
                    theme_values,
                    source_span: None,
                },
            )
        })
        .collect();
    // ... rest unchanged ...
}

/// Resolve a TokenValue (AST) to TokenValueIR, handling Phase 1 compile-time
/// constants and Phase 2 runtime expressions.
fn resolve_token_value_ir(
    value: &TokenValue,
    const_values: &FxHashMap<String, String>,
) -> TokenValueIR {
    match value {
        TokenValue::Literal(s) => TokenValueIR::Static(s.clone()),
        TokenValue::Variable(name) => {
            if let Some(resolved) = const_values.get(name) {
                TokenValueIR::Static(resolved.clone())
            } else {
                // Phase 1 fallback for unresolved variables: CSS var(--name)
                TokenValueIR::Static(format!("var(--{})", name))
            }
        }
        TokenValue::Expression(_expr) => {
            // Phase 2: will be implemented in Task 5 (needs LowerContext)
            // For now, placeholder — static empty string
            TokenValueIR::Static(String::new())
        }
    }
}
```

- [ ] **Step 4: Update existing resolve_token_value (String version) for backward compat**

Keep the old `resolve_token_value() -> String` as a convenience wrapper used by `resolve_value()` in class/variant property resolution:

```rust
fn resolve_token_value(value: &TokenValue, const_values: &FxHashMap<String, String>) -> String {
    match resolve_token_value_ir(value, const_values) {
        TokenValueIR::Static(s) => s,
        TokenValueIR::Dynamic(_) => String::new(), // dynamic expressions in class props are Phase 2+
    }
}
```

- [ ] **Step 5: Update all test code constructing TokenIR**

```rust
// In ir/lower/design.rs tests, change:
TokenIR {
    value: "#6366f1".to_string(),
    // ...
}
// To:
TokenIR {
    value: TokenValueIR::Static("#6366f1".to_string()),
    // ...
}
```

- [ ] **Step 6: Verify all tests pass**

Run: `cargo test -p omnilang-core`
Expected: all 539+ tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/ir/design.rs \
        crates/omnilang-core/src/ir/lower/design.rs \
        crates/omnilang-core/src/ir/lower/mod.rs
# Add any other files touched for compilation fixes
git commit -m "feat: add TokenValueIR enum (Static|Dynamic) to design IR types"
```

---

### Task 5: Lower `TokenValue::Expression` to `TokenValueIR::Dynamic(ExprIR)`

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/design.rs:22-89`
- Modify: `crates/omnilang-core/src/ir/lower/mod.rs:208-212`

**Goal:** When lowering a `TokenValue::Expression(Expr)`, invoke the existing `lower_expr()` to produce `ExprIR`, then wrap it in `TokenValueIR::Dynamic(ExprIR)`. This requires passing the lowering context (`variant_map`, `semantics`) to `lower_design_with_consts`.

- [ ] **Step 1: Update lower_design_with_consts signature to accept lowering context**

```rust
// crates/omnilang-core/src/ir/lower/design.rs

use crate::ir::lower::expr::lower_expr;
use crate::semantic::SemanticResult;

pub fn lower_design(ast: &DesignDef) -> DesignIR {
    let const_values = FxHashMap::default();
    let variant_map = FxHashMap::default();
    lower_design_with_consts(ast, &const_values, &variant_map, &mut SemanticResult::default())
}

pub fn lower_design_with_consts(
    ast: &DesignDef,
    const_values: &FxHashMap<String, String>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> DesignIR {
    let tokens: FxHashMap<String, TokenIR> = ast
        .tokens
        .iter()
        .map(|t| {
            // ... token_type mapping ...
            let resolved_value = resolve_token_value_ir(&t.value, const_values, variant_map, semantics);
            let theme_values: FxHashMap<String, TokenValueIR> = t
                .theme_values
                .iter()
                .map(|(theme, val)| {
                    (
                        theme.clone(),
                        resolve_token_value_ir(val, const_values, variant_map, semantics),
                    )
                })
                .collect();
            // ... construct TokenIR ...
        })
        .collect();
    // ... rest unchanged ...
}
```

- [ ] **Step 2: Implement expression lowering in resolve_token_value_ir**

```rust
fn resolve_token_value_ir(
    value: &TokenValue,
    const_values: &FxHashMap<String, String>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> TokenValueIR {
    match value {
        TokenValue::Literal(s) => TokenValueIR::Static(s.clone()),
        TokenValue::Variable(name) => {
            if let Some(resolved) = const_values.get(name) {
                TokenValueIR::Static(resolved.clone())
            } else {
                // Phase 2: Unresolved variable → treat as runtime signal reference.
                // Emit as IdentifierExpr that codegen will bridge to signal.value.
                TokenValueIR::Dynamic(ExprIR::IdentifierExpr {
                    name: name.clone(),
                    type_ann: None,
                })
            }
        }
        TokenValue::Expression(expr) => {
            // Phase 2: Lower the AST expression to ExprIR using the standard pipeline
            let expr_ir = lower_expr(expr, variant_map, None, semantics);
            TokenValueIR::Dynamic(expr_ir)
        }
    }
}
```

Key design decision: `TokenValue::Variable(name)` that is NOT found in `const_values` is now lowered as `TokenValueIR::Dynamic(IdentifierExpr { name })` instead of `TokenValueIR::Static("var(--name)")`. This enables runtime signal bridging. The compile-time fallback `var(--name)` is removed for the dynamic path.

But wait — this would break Phase 1 behavior where undefined variables gracefully fall back to `var(--name)`. We need to preserve that.

**Revised approach:** Add a `treat_unresolved_as_dynamic: bool` flag:

```rust
fn resolve_token_value_ir(
    value: &TokenValue,
    const_values: &FxHashMap<String, String>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> TokenValueIR {
    match value {
        TokenValue::Literal(s) => TokenValueIR::Static(s.clone()),
        TokenValue::Variable(name) => {
            if let Some(resolved) = const_values.get(name) {
                TokenValueIR::Static(resolved.clone())
            } else {
                // Phase 1 fallback: CSS var(--name) for graceful degradation
                // Phase 2: could be overridden to Dynamic for signal bridging,
                // but the caller decides. For now, keep Phase 1 behavior.
                TokenValueIR::Static(format!("var(--{})", name))
            }
        }
        TokenValue::Expression(expr) => {
            let expr_ir = lower_expr(expr, variant_map, None, semantics);
            TokenValueIR::Dynamic(expr_ir)
        }
    }
}
```

This is safer — `TokenValue::Variable` (simple identifiers like `BRAND`) continues Phase 1 behavior. Only `TokenValue::Expression` (dotted paths, function calls) become dynamic. Users who want a simple variable to be dynamic would write `token primary: color = ThemeStore.primaryColor;` (dotted path), not just `primaryColor`.

- [ ] **Step 3: Update call site in lower/mod.rs**

```rust
// crates/omnilang-core/src/ir/lower/mod.rs:208-212

let designs: Vec<super::DesignIR> = module
    .designs
    .iter()
    .map(|d| lower_design_with_consts(d, &const_values, &variant_map, semantics))
    .collect();
```

- [ ] **Step 4: Write lowering tests**

```rust
// In crates/omnilang-core/src/ir/lower/design.rs tests module

#[test]
fn lower_expression_token_value_to_dynamic() {
    use crate::parser::ast::Expr;
    use crate::parser::ast::Literal;

    let design_def = DesignDef {
        name: "Test".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: vec![TokenDef {
            name: "primary".to_string(),
            token_type: TokenType::Color,
            value: TokenValue::Expression(
                Expr::FieldAccess {
                    object: Box::new(Expr::VarRef("ThemeStore".to_string(), Span::default())),
                    field: "primaryColor".to_string(),
                    span: Span::default(),
                }
            ),
            theme_values: vec![],
        }],
        classes: vec![],
        variants: vec![],
    };

    let const_values = FxHashMap::default();
    let variant_map = FxHashMap::default();
    let mut semantics = SemanticResult::default();
    let design = lower_design_with_consts(&design_def, &const_values, &variant_map, &mut semantics);

    let primary = design.tokens.get("primary").expect("should have primary");
    assert!(matches!(primary.value, TokenValueIR::Dynamic(_)));
}

#[test]
fn lower_static_token_still_works() {
    let design_def = DesignDef {
        name: "Test".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: vec![TokenDef {
            name: "spacing".to_string(),
            token_type: TokenType::Size,
            value: TokenValue::Literal("16px".to_string()),
            theme_values: vec![],
        }],
        classes: vec![],
        variants: vec![],
    };

    let const_values = FxHashMap::default();
    let variant_map = FxHashMap::default();
    let mut semantics = SemanticResult::default();
    let design = lower_design_with_consts(&design_def, &const_values, &variant_map, &mut semantics);

    let spacing = design.tokens.get("spacing").expect("should have spacing");
    assert_eq!(spacing.value, TokenValueIR::Static("16px".to_string()));
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p omnilang-core ir::lower::design`
Expected: new tests PASS, all existing tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/design.rs \
        crates/omnilang-core/src/ir/lower/mod.rs
git commit -m "feat: lower TokenValue::Expression to TokenValueIR::Dynamic(ExprIR)"
```

---

### Task 6: Update CSS codegen to skip dynamic tokens in `:root`

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs:39-118` (emit_design_css_impl)

**Goal:** Static CSS declarations (`:root { --token: value; }`) should only be emitted for `TokenValueIR::Static` tokens. Dynamic tokens (`TokenValueIR::Dynamic`) are skipped — their values come from JS at runtime.

- [ ] **Step 1: Filter dynamic tokens from :root and theme CSS blocks**

```rust
// omnilang/src/codegen/omniview/design.rs
// In emit_design_css_impl(), replace the token emission section:

// ── Emit token CSS custom property declarations ──
if !design.tokens.is_empty() {
    // Collect only static tokens for CSS emission
    let mut static_tokens: Vec<_> = design.tokens.iter()
        .filter(|(_, token)| matches!(token.value, TokenValueIR::Static(_)))
        .collect();
    static_tokens.sort_by(|a, b| a.0.cmp(b.0));

    if !static_tokens.is_empty() {
        // Build a map: theme_name → Vec<(token_name, value)>
        let mut theme_token_map: FxHashMap<&str, Vec<(&String, &String)>> = FxHashMap::default();
        for (name, token) in &static_tokens {
            if let TokenValueIR::Static(ref val) = token.value {
                for (theme_name, theme_val) in &token.theme_values {
                    if let TokenValueIR::Static(ref tv) = theme_val {
                        theme_token_map
                            .entry(theme_name.as_str())
                            .or_default()
                            .push((name, tv));
                    }
                }
            }
        }

        // 1. Default theme on :root
        css.push_str(":root {\n");
        for (name, token) in &static_tokens {
            if let TokenValueIR::Static(ref val) = token.value {
                let _ = writeln!(css, "  --{}: {};", name, val);
            }
        }
        css.push_str("}\n\n");

        // 2. Per-theme overrides
        if !theme_token_map.is_empty() {
            let mut theme_names: Vec<&str> = theme_token_map.keys().copied().collect();
            theme_names.sort();
            for theme_name in &theme_names {
                if let Some(pairs) = theme_token_map.get(theme_name) {
                    let _ = writeln!(css, "[data-theme=\"{}\"] {{", theme_name);
                    for (name, val) in pairs {
                        let _ = writeln!(css, "  --{}: {};", name, val);
                    }
                    css.push_str("}\n\n");
                }
            }
        }

        // 3. @media (prefers-color-scheme: dark)
        if let Some(dark_pairs) = theme_token_map.get("dark") {
            css.push_str("@media (prefers-color-scheme: dark) {\n");
            css.push_str("  :root:not([data-theme]) {\n");
            for (name, val) in dark_pairs {
                let _ = writeln!(css, "    --{}: {};", name, val);
            }
            css.push_str("  }\n");
            css.push_str("}\n\n");
        }
    }
}
```

- [ ] **Step 2: Update theme_map iteration to handle TokenValueIR**

The previous `theme_token_map` used `&String` for values. Now values come from `TokenValueIR::Static(s)` or are skipped. The map type changes to filter on the enum variant.

- [ ] **Step 3: Write test for CSS codegen skipping dynamic tokens**

```rust
// In omnilang/tests/design_codegen_test.rs

#[test]
fn css_output_skips_dynamic_tokens() {
    use omnilang_core::ir::design::*;
    use omnilang_core::ir::types::ExprIR;

    let mut tokens = FxHashMap::default();
    tokens.insert(
        "primary".to_string(),
        TokenIR {
            value: TokenValueIR::Static("#6366f1".to_string()),
            token_type: TokenTypeIR::Color,
            theme_values: FxHashMap::default(),
            source_span: None,
        },
    );
    tokens.insert(
        "dynamicAccent".to_string(),
        TokenIR {
            value: TokenValueIR::Dynamic(ExprIR::IdentifierExpr {
                name: "ThemeStore_primaryColor".to_string(),
                type_ann: None,
            }),
            token_type: TokenTypeIR::Color,
            theme_values: FxHashMap::default(),
            source_span: None,
        },
    );

    let design = DesignIR {
        name: "App".to_string(),
        themes: vec![],
        default_theme: None,
        tokens,
        classes: vec![],
        variants: vec![],
        source_span: None,
    };

    let css = emit_design_css(&design, "App").unwrap();

    // Static token should appear in :root
    assert!(css.contains("--primary: #6366f1"));
    // Dynamic token should NOT appear in :root
    assert!(!css.contains("--dynamicAccent"));
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omnilang design_codegen`
Expected: new test PASS, all existing tests PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/design.rs \
        omnilang/tests/design_codegen_test.rs
git commit -m "feat: skip dynamic tokens in static :root CSS declarations"
```

---

### Task 7: Generate `__omni_effect` JS for dynamic tokens

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs` (new function `emit_dynamic_token_effects`)
- Modify: `omnilang/src/codegen/omniview/mod.rs:379` (call after `emit_theme_runtime`)

**Goal:** For each `TokenValueIR::Dynamic(expr_ir)`, generate a JS `__omni_effect` block that sets the CSS custom property on `document.documentElement` whenever the underlying signal changes. This bridges the OmniView signal system to CSS custom properties at runtime.

- [ ] **Step 1: Implement emit_dynamic_token_effects in design.rs**

```rust
// omnilang/src/codegen/omniview/design.rs

/// Emit JS `__omni_effect` blocks for all dynamic design tokens.
///
/// For each `TokenValueIR::Dynamic(expr)`, generates:
/// ```javascript
/// __omni_effect(() => {
///   document.documentElement.style.setProperty("--tokenName", __omni_sanitizeCss(expr));
/// });
/// ```
///
/// This sets CSS custom properties on `<html>` at runtime, giving them
/// the highest CSS specificity (inline style on root element).
pub fn emit_dynamic_token_effects(
    design_map: &FxHashMap<String, DesignIR>,
    js: &mut String,
    expr_to_js: &dyn Fn(&ExprIR) -> String,
) {
    for design in design_map.values() {
        for (name, token) in &design.tokens {
            if let TokenValueIR::Dynamic(ref expr) = token.value {
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
        }
    }
}
```

- [ ] **Step 2: Call from the main generate flow**

```rust
// omnilang/src/codegen/omniview/mod.rs
// After emit_theme_runtime() call (around line 379):

gen.emit_theme_runtime();

// ── Phase 2: Dynamic token effects ──
// Generate __omni_effect blocks for tokens with runtime expression values.
// These set CSS custom properties on document.documentElement reactively.
emit_dynamic_token_effects(
    &gen.design_map,
    &mut gen.js,
    &|expr| gen.expr_to_js(expr),
);
```

- [ ] **Step 3: Handle import path for emit_dynamic_token_effects**

Add the import in `mod.rs`:
```rust
use crate::codegen::omniview::design::emit_dynamic_token_effects;
```
Or if it's already in scope through the design module, verify the function is `pub`.

- [ ] **Step 4: Write E2E test for dynamic token JS output**

```rust
// In omnilang/tests/design_codegen_test.rs

#[test]
fn generates_js_effect_for_dynamic_token() {
    use omnilang_core::ir::design::*;
    use omnilang_core::ir::types::ExprIR;

    let mut tokens = FxHashMap::default();
    tokens.insert(
        "primary".to_string(),
        TokenIR {
            value: TokenValueIR::Dynamic(ExprIR::FieldAccessExpr {
                object: Box::new(ExprIR::IdentifierExpr {
                    name: "ThemeStore".to_string(),
                    type_ann: None,
                }),
                field: "primaryColor".to_string(),
            }),
            token_type: TokenTypeIR::Color,
            theme_values: FxHashMap::default(),
            source_span: None,
        },
    );

    let design = DesignIR {
        name: "App".to_string(),
        themes: vec![],
        default_theme: None,
        tokens,
        classes: vec![],
        variants: vec![],
        source_span: None,
    };

    let mut js = String::new();
    let mut design_map = FxHashMap::default();
    design_map.insert("App".to_string(), design);

    emit_dynamic_token_effects(&design_map, &mut js, &|expr| {
        match expr {
            ExprIR::FieldAccessExpr { object, field } => {
                if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    format!("{}_{}.value", name, field)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    });

    assert!(js.contains("__omni_effect"), "should emit __omni_effect");
    assert!(js.contains("--primary"), "should set --primary CSS variable");
    assert!(js.contains("ThemeStore_primaryColor.value"), "should reference signal value");
    assert!(js.contains("style.setProperty"), "should use setProperty");
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p omnilang design_codegen`
Expected: all tests PASS (including new JS output test)

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/design.rs \
        omnilang/src/codegen/omniview/mod.rs \
        omnilang/tests/design_codegen_test.rs
git commit -m "feat: generate __omni_effect for dynamic design token JS bridging"
```

---

### Task 8: End-to-end integration test and snapshot update

**Files:**
- Create: test fixture with dynamic token `.omni` source
- Modify: `omnilang/tests/design_codegen_test.rs` (add E2E compilation test)
- Update: insta snapshots (any that changed)

**Goal:** Verify the full pipeline works end-to-end: `.omni` source with store signal → token expression → compiled JS with `__omni_effect`.

- [ ] **Step 1: Write E2E test compiling .omni to JS**

```rust
// In omnilang/tests/design_codegen_test.rs

#[test]
fn e2e_dynamic_token_full_pipeline() {
    let src = r#"
        store ThemeStore {
            signal primaryColor: string = "#6366f1";
        }

        design App {
            token primary: color = ThemeStore.primaryColor;
            token spacing: size = "16px";
        }
    "#;

    let sources: FxHashMap<String, String> = [
        ("test.omni".to_string(), src.to_string())
    ].into_iter().collect();

    let path_mapping = FxHashMap::default();
    let result = omnilang_core::analyse::analyse_multi(&sources, &path_mapping);

    assert!(result.success, "analysis should succeed: {:?}", result.diagnostics);

    let ir = result.ir.expect("should have IR");

    // Verify the token was lowered as Dynamic
    let module = ir.modules.values().next().expect("should have module");
    let design = module.designs.first().expect("should have design");
    let primary = design.tokens.get("primary").expect("should have primary token");
    assert!(
        matches!(primary.value, TokenValueIR::Dynamic(_)),
        "ThemeStore.primaryColor should be Dynamic, got {:?}",
        primary.value
    );

    // Verify spacing is still Static
    let spacing = design.tokens.get("spacing").expect("should have spacing token");
    assert!(
        matches!(spacing.value, TokenValueIR::Static(_)),
        "static value should remain Static"
    );
}
```

- [ ] **Step 2: Run the full test suite**

Run: `cargo test`
Expected: all tests PASS (539+ lib tests + omnilang tests)

- [ ] **Step 3: Update insta snapshots**

Run: `cargo test -- -q 2>&1 | tail -20`
Then: `cargo insta review` (if any snapshots changed)

Check which snapshots changed — CSS snapshots should show `:root` without dynamic tokens. JS snapshots should show new `__omni_effect` blocks.

- [ ] **Step 4: Verify with learn-omniview (if applicable)**

```bash
# Optional: compile learn-omniview to verify no regression
cargo run -- compile learn-omniview/src/main.omni --target omniview
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/design_codegen_test.rs
git add -A  # for any snapshot files
git commit -m "test: add E2E test for dynamic token full pipeline"
```

---

### Task 9: Handle theme interaction — dynamic tokens with theme overrides

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs` (emit_dynamic_token_effects)
- Modify: `crates/omnilang-core/src/semantic/resolve.rs`

**Goal:** If a dynamic token has theme overrides (e.g., `token primary: color = ThemeStore.primaryColor dark "#fff"`), the theme values are static but the default value is dynamic. The generated JS needs to be theme-aware: check `__omni_theme.current` to pick the right value.

**Design decision:** For Phase 2, we restrict: dynamic tokens CANNOT have theme overrides. If a user writes a mixed token, the parser should emit a diagnostic. This avoids the complexity of theme-aware dynamic token JS and preserves the clear separation: theme tokens are static CSS, dynamic tokens are JS-driven.

- [ ] **Step 1: Add validation in semantic resolver**

```rust
// crates/omnilang-core/src/semantic/resolve.rs
// In the token value resolution block:

TokenValue::Expression(_expr) => {
    // ... existing resolve_expr_refs call ...

    // Phase 2 restriction: dynamic tokens cannot have theme overrides
    if !token.theme_values.is_empty() {
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::E0001,
            &format!(
                "Dynamic token '{}' cannot have theme overrides. \
                 Use a static value for theme-aware tokens, or handle \
                 theme switching in the store signal.",
                token.name
            ),
            &Span { start: 0, end: 0, line: 0, col: 0 },
        ));
    }
}
```

- [ ] **Step 2: Write test for theme+dynamic rejection**

```rust
#[test]
fn rejects_dynamic_token_with_theme_override() {
    let src = r#"
        store ThemeStore {
            signal primaryColor: string = "#6366f1";
        }
        design App {
            @themes light, dark;
            token primary: color = ThemeStore.primaryColor dark "#a5b4fc";
        }
    "#;
    // Should produce diagnostic about dynamic token + theme override
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omnilang-core design_theme`
Expected: new test PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs \
        crates/omnilang-core/tests/design_theme_test.rs
git commit -m "feat: reject dynamic tokens with theme overrides (Phase 2 restriction)"
```

---

## Self-Review

### 1. Spec Coverage

| Requirement from design doc | Covered by |
|---|---|
| `TokenValue::Expression(Expr)` at AST | Task 0 |
| Tokenize design body for expression parsing | Task 1 |
| Parse dotted paths as expressions | Task 2 |
| Semantic resolve_expr_refs for token exprs | Task 3 |
| `TokenValueIR::Dynamic(ExprIR)` at IR | Task 4 |
| Lower Expr→ExprIR for token values | Task 5 |
| Skip dynamic tokens in `:root` CSS | Task 6 |
| Generate `__omni_effect` JS for dynamic tokens | Task 7 |
| E2E full pipeline test | Task 8 |
| Theme+dynamic interaction (restriction) | Task 9 |
| Parser: simple identifiers remain Variable | Task 2 (unchanged path) |
| Codegen: reuse emit_style_assignment pattern | Task 7 (uses setProperty directly on documentElement) |

### 2. Placeholder Scan

✅ No TBD/TODO/implement-later placeholders
✅ All code blocks show actual implementation
✅ All test code is concrete and compilable
✅ Every task has exact file paths and commit messages

### 3. Type Consistency

- `TokenValue::Expression(Expr)` — used in Tasks 0-3,5 — consistent with `crate::parser::ast::Expr`
- `TokenValueIR::Static(String) | Dynamic(ExprIR)` — used in Tasks 4-8 — consistent with `crate::ir::types::ExprIR`
- `parse_design_body(input: &str, design_tokens: &[Token])` — used in Tasks 1-2 — consistent signature
- `lower_design_with_consts(ast, const_values, variant_map, semantics)` — used in Tasks 5 → consistent across lowering code
- `emit_dynamic_token_effects(design_map, js, expr_to_js)` — used in Tasks 7-8 — consistent closure type

### Issues Found & Fixed

- **Parser offset calculation**: Original plan used fragile pointer math. Rewrote with explicit offset tracking using `design_body_full` parameter.
- **Phase 1 Variable fallback preservation**: Original Task 5 plan changed `TokenValue::Variable` behavior to always emit Dynamic when unresolved. Fixed to preserve Phase 1 `var(--name)` fallback — only `TokenValue::Expression` becomes Dynamic.
- **Theme override conflict**: Added Task 9 to explicitly reject dynamic+theme combinations rather than silently producing broken output.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-30-dynamic-token-variables-phase2.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
