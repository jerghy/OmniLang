# Poiesis Style Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a design-tool-level style editing experience in Poiesis with design tokens, reusable style classes, and multi-dimensional variants, backed by new `design` block syntax in omnic.

**Architecture:** Three-track implementation:
1. **omnic Rust** — `design` block parsing/AST/IR/codegen so omnic can compile design.omni
2. **Poiesis TypeScript UI** — visual editors (Token/Class/Variant), ContextInspector upgrades
3. **Poiesis Save/Load Pipeline** — design.omni parser (TypeScript) + save path in `useSaveProject` + load path in `parseAndPopulate`

**Key principle:** `.omni` is the source of truth. All token/class/variant data comes from parsing `design.omni`. `project.json` stores only editor metadata (positions, viewport, panel layout) — never design semantic data.

**Tech Stack:** Rust (omnilang compiler — parser, AST, IR, codegen), TypeScript/React (Poiesis — Zustand store, visual editors, .omni emitter)

---

## Phase A: omnic Foundation — `design` Block Parsing

### Task A1: AST Types for Design Blocks

**Files:**
- Create: `omnilang/src/parser/ast/design.rs`
- Modify: `omnilang/src/parser/ast/mod.rs`

- [ ] **Step 1: Create AST node types for design blocks**

```rust
// omnilang/src/parser/ast/design.rs
use serde::{Deserialize, Serialize};

/// Top-level design block: `design AppDesign { ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignDef {
    pub name: String,
    pub tokens: Vec<TokenDef>,
    pub classes: Vec<DesignClassDef>,
    pub variants: Vec<VariantDef>,
}

/// Token type annotation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Color,
    Size,
    Font,
    Shadow,
}

/// `token primary: color = "#6366f1";`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenDef {
    pub name: String,
    pub token_type: TokenType,
    pub value: String,
}

/// `class Card { ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignClassDef {
    pub name: String,
    pub properties: Vec<DesignProperty>,
    pub pseudo_states: Vec<PseudoState>,
}

/// A single CSS-like property: `background: token(surface);`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignProperty {
    pub name: String,           // "background", "border-radius"
    pub value: DesignValue,
}

/// Value in a design property
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DesignValue {
    Literal(String),            // "white", "1px solid #ccc"
    TokenRef(String),           // token(surface), token(spacingMd)
    Mixed(Vec<DesignValuePart>), // "1px solid " + token(border)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DesignValuePart {
    Text(String),
    Token(String),
}

/// `&:hover { ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PseudoState {
    pub pseudo: String,         // "hover", "focus", "active"
    pub properties: Vec<DesignProperty>,
}

/// `variant Button { @dim color: primary | danger; ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariantDef {
    pub name: String,
    pub dimensions: Vec<VariantDimension>,
    pub defaults: Vec<(String, String)>,   // [(color, primary), (size, md)]
    pub base: Vec<DesignProperty>,
    pub branches: Vec<VariantBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariantDimension {
    pub name: String,
    pub values: Vec<String>,
}

/// `color.primary { background: token(primary); }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariantBranch {
    pub dimension: String,
    pub value: String,
    pub properties: Vec<DesignProperty>,
}
```

- [ ] **Step 2: Register the module in the AST mod**

```rust
// omnilang/src/parser/ast/mod.rs — add near other pub mod declarations:
pub mod design;
```

- [ ] **Step 3: Build and verify types compile**

Run: `cargo build -p omnilang 2>&1 | head -20`
Expected: Compiles with warnings only (unused types at this stage)

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/parser/ast/design.rs omnilang/src/parser/ast/mod.rs
git commit -m "feat: add AST types for design blocks (TokenDef, DesignClassDef, VariantDef)"
```

---

### Task A2: Parse Token Definitions

**Files:**
- Create: `omnilang/src/parser/design.rs`
- Modify: `omnilang/src/parser/mod.rs`

- [ ] **Step 1: Write failing test for token parsing**

```rust
// omnilang/src/parser/design.rs (test module at bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::design::*;

    #[test]
    fn parse_color_token() {
        let src = r#"token primary: color = "#6366f1";"#;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.tokens.len(), 1);
        assert_eq!(result.tokens[0].name, "primary");
        assert_eq!(result.tokens[0].token_type, TokenType::Color);
        assert_eq!(result.tokens[0].value, "#6366f1");
    }

    #[test]
    fn parse_size_token() {
        let src = r#"token spacingMd: size = "16px";"#;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.tokens[0].token_type, TokenType::Size);
        assert_eq!(result.tokens[0].value, "16px");
    }

    #[test]
    fn parse_multiple_tokens() {
        let src = r#"
            token primary: color = "#6366f1";
            token surface: color = "#ffffff";
            token spacingMd: size = "16px";
        "#;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.tokens.len(), 3);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omnilang -- parser::design::tests 2>&1`
Expected: FAIL — `parse_design_body` not found

- [ ] **Step 3: Implement token parser**

```rust
// omnilang/src/parser/design.rs
use crate::parser::ast::design::*;
use crate::parser::error::{ParseError, ParseResult};

/// Parse the body content inside `design Name { ... }`
pub fn parse_design_body(input: &str) -> ParseResult<DesignDef> {
    let mut tokens = Vec::new();
    let mut classes = Vec::new();
    let mut variants = Vec::new();
    let mut remaining = input.trim();

    while !remaining.is_empty() {
        remaining = skip_ws_and_comments(remaining);

        if remaining.starts_with("token ") {
            let (token, rest) = parse_token(remaining)?;
            tokens.push(token);
            remaining = rest;
        } else if remaining.starts_with("class ") {
            // TODO: Task A3
            break;
        } else if remaining.starts_with("variant ") {
            // TODO: Task A4
            break;
        } else if remaining.starts_with("}") {
            break;
        } else {
            return Err(ParseError::unexpected_token(remaining));
        }
    }

    Ok(DesignDef {
        name: String::new(), // filled by caller
        tokens,
        classes,
        variants,
    })
}

fn parse_token(input: &str) -> ParseResult<(TokenDef, &str)> {
    let rest = input.strip_prefix("token ").ok_or(ParseError::expected("token"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest.strip_prefix(":").ok_or(ParseError::expected(":"))?.trim_start();
    let (token_type, rest) = parse_token_type(rest)?;
    let rest = rest.strip_prefix("=").ok_or(ParseError::expected("="))?.trim_start();
    let (value, rest) = parse_string_or_ident(rest)?;
    let rest = rest.strip_prefix(";").ok_or(ParseError::expected(";"))?.trim_start();

    Ok((TokenDef { name, token_type, value }, rest))
}

fn parse_token_type(input: &str) -> ParseResult<(TokenType, &str)> {
    let input = input.trim_start();
    if input.starts_with("color") {
        Ok((TokenType::Color, &input[5..]))
    } else if input.starts_with("size") {
        Ok((TokenType::Size, &input[4..]))
    } else if input.starts_with("font") {
        Ok((TokenType::Font, &input[4..]))
    } else if input.starts_with("shadow") {
        Ok((TokenType::Shadow, &input[6..]))
    } else {
        Err(ParseError::expected("token type: color|size|font|shadow"))
    }
}

fn parse_ident(input: &str) -> ParseResult<(String, &str)> {
    let input = input.trim_start();
    let end = input.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(input.len());
    if end == 0 {
        return Err(ParseError::expected("identifier"));
    }
    Ok((input[..end].to_string(), &input[end..]))
}

fn parse_string_or_ident(input: &str) -> ParseResult<(String, &str)> {
    let input = input.trim_start();
    if input.starts_with('"') {
        let end = input[1..].find('"').map(|i| i + 2).ok_or(ParseError::expected("closing \""))?;
        Ok((input[1..end-1].to_string(), &input[end..]))
    } else {
        parse_ident(input)
    }
}

fn skip_ws_and_comments(input: &str) -> &str {
    let mut s = input.trim_start();
    while s.starts_with("//") {
        let line_end = s.find('\n').unwrap_or(s.len());
        s = s[line_end..].trim_start();
    }
    s
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omnilang -- parser::design::tests 2>&1`
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/parser/design.rs omnilang/src/parser/mod.rs
git commit -m "feat: add token parser for design blocks"
```

---

### Task A3: Parse Design Class Definitions

**Files:**
- Modify: `omnilang/src/parser/design.rs`

- [ ] **Step 1: Write failing test for class parsing**

```rust
#[test]
fn parse_class_with_properties() {
    let src = r#"
        class Card {
            background: token(surface);
            border-radius: token(radiusMd);
            &:hover {
                box-shadow: "0 4px 12px rgba(0,0,0,0.08)";
            }
        }
    "#;
    let result = parse_design_body(src).unwrap();
    assert_eq!(result.classes.len(), 1);
    let cls = &result.classes[0];
    assert_eq!(cls.name, "Card");
    assert_eq!(cls.properties.len(), 2);
    assert_eq!(cls.pseudo_states.len(), 1);
    assert_eq!(cls.pseudo_states[0].pseudo, "hover");
}

#[test]
fn parse_token_ref_value() {
    let src = r#"class Test { background: token(surface); }"#;
    let result = parse_design_body(src).unwrap();
    let prop = &result.classes[0].properties[0];
    assert!(matches!(prop.value, DesignValue::TokenRef(ref name) if name == "surface"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omnilang -- parser::design::tests 2>&1`
Expected: 2 new tests FAIL (classes not yet parsed)

- [ ] **Step 3: Implement class and property parsing**

Add these functions to `omnilang/src/parser/design.rs`:

```rust
fn parse_class(input: &str) -> ParseResult<(DesignClassDef, &str)> {
    let rest = input.strip_prefix("class ").ok_or(ParseError::expected("class"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest.trim_start().strip_prefix("{").ok_or(ParseError::expected("{"))?.trim_start();

    let mut properties = Vec::new();
    let mut pseudo_states = Vec::new();
    let mut remaining = rest;

    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        if remaining.starts_with("&:") {
            let (pseudo, rest) = parse_pseudo_state(remaining)?;
            pseudo_states.push(pseudo);
            remaining = rest;
        } else if remaining.is_empty() {
            break;
        } else {
            let (prop, rest) = parse_design_property(remaining)?;
            properties.push(prop);
            remaining = rest;
        }
    }

    Ok((DesignClassDef { name, properties, pseudo_states }, remaining))
}

fn parse_pseudo_state(input: &str) -> ParseResult<(PseudoState, &str)> {
    let rest = input.strip_prefix("&:").ok_or(ParseError::expected("&:"))?;
    let (pseudo, rest) = parse_ident(rest)?;
    let rest = rest.trim_start().strip_prefix("{").ok_or(ParseError::expected("{"))?.trim_start();

    let mut properties = Vec::new();
    let mut remaining = rest;
    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        let (prop, rest) = parse_design_property(remaining)?;
        properties.push(prop);
        remaining = rest;
    }

    Ok((PseudoState { pseudo, properties }, remaining))
}

fn parse_design_property(input: &str) -> ParseResult<(DesignProperty, &str)> {
    let (name, rest) = parse_ident(input)?;
    let rest = rest.trim_start().strip_prefix(":").ok_or(ParseError::expected(":"))?.trim_start();
    let (value, rest) = parse_design_value(rest)?;
    let rest = rest.trim_start().strip_prefix(";").ok_or(ParseError::expected(";"))?.trim_start();
    Ok((DesignProperty { name, value }, rest))
}

fn parse_design_value(input: &str) -> ParseResult<(DesignValue, &str)> {
    let input = input.trim_start();
    if input.starts_with("token(") {
        let rest = &input[6..];
        let end = rest.find(')').ok_or(ParseError::expected(")"))?;
        let name = rest[..end].trim().to_string();
        Ok((DesignValue::TokenRef(name), &rest[end+1..]))
    } else if input.starts_with('"') {
        let end = input[1..].find('"').map(|i| i + 2).ok_or(ParseError::expected("closing \""))?;
        Ok((DesignValue::Literal(input[1..end-1].to_string()), &input[end..]))
    } else {
        // Unquoted literal until ; or }
        let end = input.find(|c: char| c == ';' || c == '}').unwrap_or(input.len());
        Ok((DesignValue::Literal(input[..end].trim().to_string()), &input[end..]))
    }
}
```

Update `parse_design_body` to call `parse_class` when encountering `class `:

```rust
} else if remaining.starts_with("class ") {
    let (cls, rest) = parse_class(remaining)?;
    classes.push(cls);
    remaining = rest;
}
```

- [ ] **Step 4: Run all design parser tests**

Run: `cargo test -p omnilang -- parser::design::tests 2>&1`
Expected: All 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/parser/design.rs
git commit -m "feat: add class parser for design blocks"
```

---

### Task A4: Parse Variant Definitions

**Files:**
- Modify: `omnilang/src/parser/design.rs`

- [ ] **Step 1: Write failing test for variant parsing**

```rust
#[test]
fn parse_variant_with_dimensions() {
    let src = r#"
        variant Button {
            @dim color: primary | danger | ghost;
            @dim size: sm | md | lg;
            @default { color: primary; size: md; }
            base {
                display: inline-flex;
                border-radius: token(radiusMd);
            }
            color.primary { background: token(primary); color: white; }
            size.sm { padding: "4px 8px"; font-size: "12px"; }
        }
    "#;
    let result = parse_design_body(src).unwrap();
    assert_eq!(result.variants.len(), 1);
    let v = &result.variants[0];
    assert_eq!(v.name, "Button");
    assert_eq!(v.dimensions.len(), 2);
    assert_eq!(v.dimensions[0].name, "color");
    assert_eq!(v.dimensions[0].values, vec!["primary", "danger", "ghost"]);
    assert_eq!(v.base.len(), 2); // display, border-radius
    assert_eq!(v.branches.len(), 2);
    assert_eq!(v.branches[0].dimension, "color");
    assert_eq!(v.branches[0].value, "primary");
    assert_eq!(v.defaults, vec![
        ("color".into(), "primary".into()),
        ("size".into(), "md".into()),
    ]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omnilang -- parser::design::tests 2>&1`
Expected: 1 new test FAIL

- [ ] **Step 3: Implement variant parser**

Add these functions to `omnilang/src/parser/design.rs`:

```rust
fn parse_variant(input: &str) -> ParseResult<(VariantDef, &str)> {
    let rest = input.strip_prefix("variant ").ok_or(ParseError::expected("variant"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest.trim_start().strip_prefix("{").ok_or(ParseError::expected("{"))?.trim_start();

    let mut dimensions = Vec::new();
    let mut defaults = Vec::new();
    let mut base = Vec::new();
    let mut branches = Vec::new();
    let mut remaining = rest;

    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        if remaining.starts_with("@dim ") {
            let (dim, rest) = parse_variant_dimension(remaining)?;
            dimensions.push(dim);
            remaining = rest;
        } else if remaining.starts_with("@default ") {
            let (defs, rest) = parse_variant_defaults(remaining)?;
            defaults = defs;
            remaining = rest;
        } else if remaining.starts_with("base ") {
            let (props, rest) = parse_variant_base(remaining)?;
            base = props;
            remaining = rest;
        } else if remaining.contains('.') && !remaining.starts_with("token(") {
            // branch: color.primary { ... }
            let (branch, rest) = parse_variant_branch(remaining)?;
            branches.push(branch);
            remaining = rest;
        } else {
            break;
        }
    }

    Ok((VariantDef { name, dimensions, defaults, base, branches }, remaining))
}

fn parse_variant_dimension(input: &str) -> ParseResult<(VariantDimension, &str)> {
    let rest = input.strip_prefix("@dim ").ok_or(ParseError::expected("@dim"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest.trim_start().strip_prefix(":").ok_or(ParseError::expected(":"))?.trim_start();

    let mut values = Vec::new();
    let mut remaining = rest;
    loop {
        remaining = remaining.trim_start();
        if remaining.starts_with(";") || remaining.starts_with("{") || remaining.is_empty() {
            break;
        }
        if remaining.starts_with("|") {
            remaining = &remaining[1..].trim_start();
            continue;
        }
        let (val, rest) = parse_ident(remaining)?;
        values.push(val);
        remaining = rest;
    }
    let remaining = remaining.strip_prefix(";").unwrap_or(remaining);

    Ok((VariantDimension { name, values }, remaining))
}

fn parse_variant_defaults(input: &str) -> ParseResult<(Vec<(String, String)>, &str)> {
    let rest = input.strip_prefix("@default ").ok_or(ParseError::expected("@default"))?;
    let rest = rest.trim_start().strip_prefix("{").ok_or(ParseError::expected("{"))?.trim_start();

    let mut defaults = Vec::new();
    let mut remaining = rest;
    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        let (key, rest) = parse_ident(remaining)?;
        let rest = rest.trim_start().strip_prefix(":").ok_or(ParseError::expected(":"))?.trim_start();
        let (val, rest) = parse_ident(rest)?;
        let rest = rest.trim_start().strip_prefix(";").unwrap_or(rest);
        defaults.push((key, val));
        remaining = rest;
    }

    Ok((defaults, remaining))
}

fn parse_variant_base(input: &str) -> ParseResult<(Vec<DesignProperty>, &str)> {
    let rest = input.strip_prefix("base ").ok_or(ParseError::expected("base"))?;
    let rest = rest.trim_start().strip_prefix("{").ok_or(ParseError::expected("{"))?.trim_start();

    let mut properties = Vec::new();
    let mut remaining = rest;
    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        let (prop, rest) = parse_design_property(remaining)?;
        properties.push(prop);
        remaining = rest;
    }

    Ok((properties, remaining))
}

fn parse_variant_branch(input: &str) -> ParseResult<(VariantBranch, &str)> {
    let dot_pos = input.find('.').ok_or(ParseError::expected("dimension.value"))?;
    let dimension = input[..dot_pos].trim().to_string();
    let rest = &input[dot_pos+1..];
    let (value, rest) = parse_ident(rest)?;
    let rest = rest.trim_start().strip_prefix("{").ok_or(ParseError::expected("{"))?.trim_start();

    let mut properties = Vec::new();
    let mut remaining = rest;
    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        let (prop, rest) = parse_design_property(remaining)?;
        properties.push(prop);
        remaining = rest;
    }

    Ok((VariantBranch { dimension, value, properties }, remaining))
}
```

Update `parse_design_body` to call `parse_variant` when encountering `variant `:

```rust
} else if remaining.starts_with("variant ") {
    let (var, rest) = parse_variant(remaining)?;
    variants.push(var);
    remaining = rest;
}
```

- [ ] **Step 4: Run all design parser tests**

Run: `cargo test -p omnilang -- parser::design::tests 2>&1`
Expected: All 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/parser/design.rs
git commit -m "feat: add variant parser for design blocks"
```

---

### Task A5: Wire Design Parser into Top-Level Module

**Files:**
- Modify: `omnilang/src/parser/mod.rs`

- [ ] **Step 1: Export design parser and wire into `parse_source`**

Find the top-level parse entry point (typically `parse_source` or similar in `mod.rs`) and add a branch for `design` keyword:

```rust
// In omnilang/src/parser/mod.rs, add to imports:
pub mod design;

// In the top-level parser (parse_source or equivalent), add before
// the existing "component" branch or as a new top-level construct:
if input.starts_with("design ") {
    let rest = input.strip_prefix("design ").unwrap();
    let (name, rest) = design::parse_ident(rest)
        .map_err(|e| ParseError::expected("design name"))?;
    let rest = rest.trim_start().strip_prefix("{")
        .ok_or(ParseError::expected("{"))?;
    let mut def = design::parse_design_body(rest)?;
    def.name = name;
    // Store in compilation unit or return as top-level item
    return Ok(TopLevelItem::Design(def));
}
```

(Adapt the exact integration point based on the existing parser architecture — look for where `component`, `store`, `theme` are parsed.)

- [ ] **Step 2: Write integration test — full design.omni**

Create `omnilang/tests/design_parse_test.rs`:

```rust
use omnilang::parser;

#[test]
fn parse_full_design_file() {
    let src = r#"
import omniview;

design AppDesign {
    token primary: color = "#6366f1";
    token spacingMd: size = "16px";

    class Card {
        background: token(surface);
        &:hover {
            box-shadow: "0 4px 12px rgba(0,0,0,0.08)";
        }
    }

    variant Button {
        @dim color: primary | danger;
        @dim size: sm | md;
        @default { color: primary; size: md; }
        base {
            display: inline-flex;
            border-radius: token(radiusMd);
        }
        color.primary { background: token(primary); }
        size.sm { padding: "4px 8px"; }
    }
}
"#;
    let result = parser::parse_source(src, "design.omni");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    // Verify the parsed structure
    // (add assertions based on what TopLevelItem variant is returned)
}
```

- [ ] **Step 3: Run integration test**

Run: `cargo test -p omnilang -- design_parse_test 2>&1`
Expected: PASS (design file parses without errors)

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/parser/mod.rs omnilang/tests/design_parse_test.rs
git commit -m "feat: wire design block parser into top-level parse_source"
```

---

## Phase B: Poiesis Data Model + Store

### Task B1: Design TypeScript Types

**Files:**
- Create: `poiesis/src/core/types/design.ts`

- [ ] **Step 1: Create design type definitions**

```typescript
// poiesis/src/core/types/design.ts
// Design system types for design.omni

import type { StyleProperty } from "./style";

// ── Token ─────────────────────────────────────────────────────

export type TokenType = "color" | "size" | "font" | "shadow";

export interface DesignToken {
  id: string;
  name: string;                       // "primary", "spacingMd"
  category: string;                   // "colors", "spacing", "typography"
  tokenType: TokenType;
  value: string;                      // "#6366f1", "16px"
}

// ── Design Class (extends existing StyleClass) ────────────────

export interface DesignClass {
  id: string;
  name: string;                       // "Card", "BtnPrimary"
  properties: StyleProperty[];        // base properties
  pseudoStates: PseudoState[];        // &:hover, &:focus, etc.
}

export interface PseudoState {
  id: string;
  pseudo: string;                     // "hover", "focus", "active"
  properties: StyleProperty[];
}

// ── Variant ───────────────────────────────────────────────────

export interface VariantDefinition {
  id: string;
  name: string;                       // "Button"
  dimensions: VariantDimension[];
  defaults: Record<string, string>;   // { color: "primary", size: "md" }
  baseProperties: StyleProperty[];
  branches: VariantBranch[];
}

export interface VariantDimension {
  name: string;                       // "color", "size"
  values: string[];                   // ["primary", "danger", "ghost"]
}

export interface VariantBranch {
  id: string;
  dimension: string;                  // "color"
  value: string;                      // "primary"
  properties: StyleProperty[];
}

// ── Design Definition (full design.omni content) ──────────────

export interface DesignDefinition {
  name: string;                       // "AppDesign"
  tokens: DesignToken[];
  classes: DesignClass[];
  variants: VariantDefinition[];
}

// ── Element bindings ──────────────────────────────────────────

export interface ElementVariantBinding {
  variantName: string;
  dimensionValues: Record<string, string>;
}

export const EMPTY_DESIGN: DesignDefinition = {
  name: "",
  tokens: [],
  classes: [],
  variants: [],
};
```

- [ ] **Step 2: Export from core/types/index.ts**

```typescript
// In poiesis/src/core/types/index.ts, add:
export * from "./design";
```

- [ ] **Step 3: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1 | head -20`
Expected: No new errors from design.ts

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/core/types/design.ts poiesis/src/core/types/index.ts
git commit -m "feat: add design system TypeScript types"
```

---

### Task B2: Design Store Slice

**Files:**
- Create: `poiesis/src/store/design-slice.ts`
- Modify: `poiesis/src/store/index.ts`

- [ ] **Step 1: Create design store slice**

```typescript
// poiesis/src/store/design-slice.ts
import type { StateCreator } from "zustand";
import type { PoiesisState } from "./index";
import type {
  DesignDefinition,
  DesignToken,
  DesignClass,
  VariantDefinition,
  ElementVariantBinding,
} from "../core/types/design";
import { EMPTY_DESIGN } from "../core/types/design";
import { uid } from "../core/utils";

export interface DesignSlice {
  // ── State ──
  activeDesign: DesignDefinition;
  elementVariants: Record<string, ElementVariantBinding>; // elementId → binding

  // ── Token actions ──
  loadDesign: (design: DesignDefinition) => void;
  addToken: (name: string, category: string, tokenType: DesignToken["tokenType"], value: string) => void;
  updateToken: (id: string, updates: Partial<DesignToken>) => void;
  removeToken: (id: string) => void;

  // ── Class actions ──
  addDesignClass: (name: string) => void;
  updateDesignClass: (id: string, updates: Partial<DesignClass>) => void;
  removeDesignClass: (id: string) => void;

  // ── Variant actions ──
  addVariant: (name: string) => void;
  updateVariant: (id: string, updates: Partial<VariantDefinition>) => void;
  removeVariant: (id: string) => void;

  // ── Element binding actions ──
  bindVariantToElement: (elementId: string, variantName: string, dimensionValues: Record<string, string>) => void;
  unbindVariantFromElement: (elementId: string) => void;
}

export const createDesignSlice: StateCreator<PoiesisState, [], [], DesignSlice> = (set) => ({
  activeDesign: { ...EMPTY_DESIGN },
  elementVariants: {},

  loadDesign: (design) => set({ activeDesign: design }),

  addToken: (name, category, tokenType, value) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: [
          ...s.activeDesign.tokens,
          { id: uid("tk"), name, category, tokenType, value },
        ],
      },
      isDirty: true,
    })),

  updateToken: (id, updates) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.map((t) =>
          t.id === id ? { ...t, ...updates } : t
        ),
      },
      isDirty: true,
    })),

  removeToken: (id) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        tokens: s.activeDesign.tokens.filter((t) => t.id !== id),
      },
      isDirty: true,
    })),

  addDesignClass: (name) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: [
          ...s.activeDesign.classes,
          { id: uid("cls"), name, properties: [], pseudoStates: [] },
        ],
      },
      isDirty: true,
    })),

  updateDesignClass: (id, updates) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.map((c) =>
          c.id === id ? { ...c, ...updates } : c
        ),
      },
      isDirty: true,
    })),

  removeDesignClass: (id) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        classes: s.activeDesign.classes.filter((c) => c.id !== id),
      },
      isDirty: true,
    })),

  addVariant: (name) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: [
          ...s.activeDesign.variants,
          {
            id: uid("var"),
            name,
            dimensions: [],
            defaults: {},
            baseProperties: [],
            branches: [],
          },
        ],
      },
      isDirty: true,
    })),

  updateVariant: (id, updates) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.map((v) =>
          v.id === id ? { ...v, ...updates } : v
        ),
      },
      isDirty: true,
    })),

  removeVariant: (id) =>
    set((s) => ({
      activeDesign: {
        ...s.activeDesign,
        variants: s.activeDesign.variants.filter((v) => v.id !== id),
      },
      isDirty: true,
    })),

  bindVariantToElement: (elementId, variantName, dimensionValues) =>
    set((s) => ({
      elementVariants: {
        ...s.elementVariants,
        [elementId]: { variantName, dimensionValues },
      },
      isDirty: true,
    })),

  unbindVariantFromElement: (elementId) =>
    set((s) => {
      const { [elementId]: _, ...rest } = s.elementVariants;
      return { elementVariants: rest, isDirty: true };
    }),
});
```

- [ ] **Step 2: Integrate slice into store**

In `poiesis/src/store/index.ts`:

```typescript
// Add import:
import { createDesignSlice, type DesignSlice } from "./design-slice";

// Extend PoiesisState interface to include DesignSlice:
export interface PoiesisState extends DesignSlice {
  // ... existing fields ...
}

// In the store creator, spread the design slice:
export const usePoiesisStore = create<PoiesisState>((set, get) => ({
  // ... existing initial values ...

  // ── Design slice ──
  ...createDesignSlice(set, get),
}));
```

- [ ] **Step 3: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/store/design-slice.ts poiesis/src/store/index.ts
git commit -m "feat: add design store slice with token/class/variant CRUD"
```

---

### Task B3: Extend TabType for design.omni

**Files:**
- Modify: `poiesis/src/core/types/design-page.ts`

- [ ] **Step 1: Add "design" to TabType**

```typescript
// In poiesis/src/core/types/design-page.ts, change:
export type TabType = "component" | "store" | "code" | "design";

// In detectTabType, add design detection:
export function detectTabType(filePath: string): TabType {
  const normalized = filePath.replace(/\\/g, "/");
  if (normalized.endsWith("design.omni") || normalized.includes("/design.omni")) return "design";
  if (normalized.includes("/stores/") || normalized.startsWith("stores/")) return "store";
  if (normalized.includes("/lib/") || normalized.startsWith("lib/")) return "code";
  return "component";
}
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: Compiles without errors (may have unused "design" type warnings — acceptable at this stage)

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/core/types/design-page.ts
git commit -m "feat: add 'design' TabType for design.omni files"
```

---

## Phase C: Poiesis Token Editor UI

### Task C1: DesignTabContent — Tab Router for design.omni

**Files:**
- Create: `poiesis/src/components/design/DesignTabContent.tsx`

- [ ] **Step 1: Create tab content router**

```tsx
// poiesis/src/components/design/DesignTabContent.tsx
import { useState } from "react";
import { TokenEditor } from "./TokenEditor";
import { ClassEditor } from "./ClassEditor";
import { VariantEditor } from "./VariantEditor";

type DesignView = "tokens" | "classes" | "variants";

interface DesignTabContentProps {
  /** Optional: show code preview instead of visual editors */
}

export function DesignTabContent({}: DesignTabContentProps) {
  const [view, setView] = useState<DesignView>("tokens");
  const [showCode, setShowCode] = useState(false);

  if (showCode) {
    return (
      <div style={{ padding: 16 }}>
        <button
          onClick={() => setShowCode(false)}
          style={{
            marginBottom: 8,
            padding: "2px 8px",
            border: "1px solid #d4d4d8",
            borderRadius: 4,
            background: "#fff",
            cursor: "pointer",
            fontSize: 12,
          }}
        >
          ← 返回编辑器
        </button>
        <DesignCodePreview />
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Tab bar */}
      <div
        style={{
          display: "flex",
          gap: 0,
          borderBottom: "1px solid #e2e8f0",
          padding: "0 12px",
          alignItems: "center",
        }}
      >
        {(["tokens", "classes", "variants"] as const).map((v) => (
          <button
            key={v}
            onClick={() => setView(v)}
            style={{
              padding: "8px 16px",
              background: "none",
              border: "none",
              borderBottom: view === v ? "2px solid #6366f1" : "2px solid transparent",
              color: view === v ? "#6366f1" : "#64748b",
              fontSize: 13,
              fontWeight: view === v ? 600 : 400,
              cursor: "pointer",
            }}
          >
            {v === "tokens" ? "Tokens" : v === "classes" ? "Classes" : "Variants"}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        <button
          onClick={() => setShowCode(true)}
          style={{
            padding: "2px 10px",
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            background: "#f8fafc",
            cursor: "pointer",
            fontSize: 11,
            color: "#64748b",
          }}
        >
          Code ▸
        </button>
      </div>

      {/* View content */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {view === "tokens" && <TokenEditor />}
        {view === "classes" && <ClassEditor />}
        {view === "variants" && <VariantEditor />}
      </div>
    </div>
  );
}

function DesignCodePreview() {
  // Readonly code preview — renders .omni source
  return (
    <pre
      style={{
        fontFamily: "monospace",
        fontSize: 12,
        color: "#1e293b",
        background: "#f8fafc",
        padding: 12,
        borderRadius: 6,
        overflow: "auto",
        whiteSpace: "pre-wrap",
      }}
    >
      {"// design.omni source preview\n// (code generation TBD in Phase F)"}
    </pre>
  );
}
```

- [ ] **Step 2: Create stub editor components**

```tsx
// poiesis/src/components/design/TokenEditor.tsx
export function TokenEditor() {
  return <div style={{ padding: 16, color: "#94a3b8" }}>Token Editor — coming next</div>;
}
```

```tsx
// poiesis/src/components/design/ClassEditor.tsx
export function ClassEditor() {
  return <div style={{ padding: 16, color: "#94a3b8" }}>Class Editor — coming next</div>;
}
```

```tsx
// poiesis/src/components/design/VariantEditor.tsx
export function VariantEditor() {
  return <div style={{ padding: 16, color: "#94a3b8" }}>Variant Editor — coming next</div>;
}
```

- [ ] **Step 3: Route design tabs in DesignPage**

In `poiesis/src/pages/DesignPage.tsx`, find where `activeTab` determines the center content, and add:

```tsx
// Import
import { DesignTabContent } from "../components/design/DesignTabContent";

// In the center panel render logic, add:
if (activeTab?.tabType === "design") {
  return <DesignTabContent />;
}
```

- [ ] **Step 4: Verify TypeScript compiles and no runtime crash**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/design/DesignTabContent.tsx poiesis/src/components/design/TokenEditor.tsx poiesis/src/components/design/ClassEditor.tsx poiesis/src/components/design/VariantEditor.tsx poiesis/src/pages/DesignPage.tsx
git commit -m "feat: add design tab content router with stub editors"
```

---

### Task C2: Token Editor — Color Tokens with HUD Picker

**Files:**
- Create: `poiesis/src/components/panels/inspector/ColorPicker.tsx`
- Modify: `poiesis/src/components/design/TokenEditor.tsx`

- [ ] **Step 1: Create HUD color picker component**

```tsx
// poiesis/src/components/panels/inspector/ColorPicker.tsx
import { useState, useCallback, useRef, useEffect } from "react";

interface ColorPickerProps {
  color: string;
  onChange: (color: string) => void;
  onClose: () => void;
  anchorRect: DOMRect;
}

/** HSL→Hex conversion — returns hex string */
function hslToHex(h: number, s: number, l: number): string {
  s /= 100;
  l /= 100;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => {
    const k = (n + (h / 30)) % 12;
    return Math.round((l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1)) * 255);
  };
  return `#${f(0).toString(16).padStart(2, "0")}${f(8).toString(16).padStart(2, "0")}${f(4).toString(16).padStart(2, "0")}`;
}

/** Parse hex to {h,s,l} */
function hexToHsl(hex: string): { h: number; s: number; l: number } {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return { h: 0, s: 0, l: Math.round(l * 100) };
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === r
    ? ((g - b) / d + (g < b ? 6 : 0)) * 60
    : max === g
    ? ((b - r) / d + 2) * 60
    : ((r - g) / d + 4) * 60;
  return { h: Math.round(h), s: Math.round(s * 100), l: Math.round(l * 100) };
}

export function ColorPicker({ color, onChange, onClose, anchorRect }: ColorPickerProps) {
  const [hsl, setHsl] = useState(() => hexToHsl(color));
  const [hexInput, setHexInput] = useState(color);
  const svRef = useRef<HTMLDivElement>(null);
  const hueRef = useRef<HTMLDivElement>(null);

  // Position HUD near anchor, clamped to viewport
  const popupStyle: React.CSSProperties = {
    position: "fixed",
    top: Math.min(anchorRect.bottom + 8, window.innerHeight - 300),
    left: Math.min(anchorRect.left, window.innerWidth - 260),
    zIndex: 1000,
    background: "#fff",
    borderRadius: 10,
    padding: 12,
    boxShadow: "0 8px 30px rgba(0,0,0,0.18), 0 2px 8px rgba(0,0,0,0.08)",
    width: 232,
  };

  const updateFromHsl = useCallback((h: number, s: number, l: number) => {
    const hex = hslToHex(h, s, l);
    setHsl({ h, s, l });
    setHexInput(hex);
    onChange(hex);
  }, [onChange]);

  const handleSvMouse = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!svRef.current) return;
    const rect = svRef.current.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const y = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
    updateFromHsl(hsl.h, Math.round(x * 100), Math.round((1 - y) * 100));
  }, [hsl.h, updateFromHsl]);

  const handleHueMouse = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!hueRef.current) return;
    const rect = hueRef.current.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    updateFromHsl(Math.round(x * 360), hsl.s, hsl.l);
  }, [hsl.s, hsl.l, updateFromHsl]);

  // Close on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      // Delay to avoid immediate close from the click that opened it
      setTimeout(() => {
        document.removeEventListener("click", handler);
        onClose();
      }, 0);
    };
    setTimeout(() => document.addEventListener("click", handler), 50);
    return () => document.removeEventListener("click", handler);
  }, [onClose]);

  return (
    <div style={popupStyle} onClick={(e) => e.stopPropagation()}>
      {/* SV plane */}
      <div
        ref={svRef}
        onMouseDown={handleSvMouse}
        onMouseMove={(e) => e.buttons === 1 && handleSvMouse(e)}
        style={{
          width: "100%",
          height: 150,
          borderRadius: 6,
          cursor: "crosshair",
          position: "relative",
          background: `linear-gradient(to right, white, hsl(${hsl.h}, 100%, 50%))`,
        }}
      >
        <div
          style={{
            position: "absolute",
            inset: 0,
            borderRadius: 6,
            background: "linear-gradient(to top, black, transparent)",
          }}
        />
        <div
          style={{
            position: "absolute",
            left: `${hsl.s}%`,
            top: `${100 - hsl.l}%`,
            width: 14,
            height: 14,
            borderRadius: "50%",
            border: "2px solid white",
            boxShadow: "0 1px 4px rgba(0,0,0,0.3)",
            transform: "translate(-50%, -50%)",
            pointerEvents: "none",
          }}
        />
      </div>

      {/* Hue strip */}
      <div
        ref={hueRef}
        onMouseDown={handleHueMouse}
        onMouseMove={(e) => e.buttons === 1 && handleHueMouse(e)}
        style={{
          width: "100%",
          height: 14,
          borderRadius: 7,
          marginTop: 10,
          cursor: "pointer",
          position: "relative",
          background: "linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)",
        }}
      >
        <div
          style={{
            position: "absolute",
            left: `${(hsl.h / 360) * 100}%`,
            top: -2,
            width: 18,
            height: 18,
            borderRadius: "50%",
            border: "2px solid white",
            boxShadow: "0 1px 4px rgba(0,0,0,0.3)",
            transform: "translateX(-50%)",
            pointerEvents: "none",
          }}
        />
      </div>

      {/* Hex input + preview */}
      <div style={{ display: "flex", gap: 8, marginTop: 10, alignItems: "center" }}>
        <div
          style={{
            width: 28, height: 28, borderRadius: 6,
            background: hexInput, border: "1px solid #d4d4d8",
            flexShrink: 0,
          }}
        />
        <input
          value={hexInput}
          onChange={(e) => {
            const val = e.target.value;
            setHexInput(val);
            if (/^#[0-9a-fA-F]{6}$/.test(val)) {
              const newHsl = hexToHsl(val);
              setHsl(newHsl);
              onChange(val);
            }
          }}
          style={{
            flex: 1, padding: "4px 8px", fontSize: 13, fontFamily: "monospace",
            border: "1px solid #e2e8f0", borderRadius: 4, outline: "none",
          }}
          onFocus={(e) => (e.target.style.borderColor = "#6366f1")}
          onBlur={(e) => {
            e.target.style.borderColor = "#e2e8f0";
            if (!/^#[0-9a-fA-F]{6}$/.test(hexInput)) {
              setHexInput(color);
            }
          }}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors from ColorPicker.tsx

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/panels/inspector/ColorPicker.tsx
git commit -m "feat: add HUD color picker component"
```

---

### Task C3: Token Editor — Full Implementation

**Files:**
- Modify: `poiesis/src/components/design/TokenEditor.tsx`

- [ ] **Step 1: Implement Token Editor with color/spacing/font support**

```tsx
// poiesis/src/components/design/TokenEditor.tsx
import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import type { DesignToken, TokenType } from "../../core/types/design";
import { ColorPicker } from "../panels/inspector/ColorPicker";

const CATEGORY_LABELS: Record<string, string> = {
  colors: "Colors",
  spacing: "Spacing",
  typography: "Typography",
  radii: "Radii",
};

interface DragScrubInputProps {
  value: string;
  onChange: (value: string) => void;
  style?: React.CSSProperties;
}

/** Numeric input that supports click+drag to adjust */
function DragScrubInput({ value, onChange, style }: DragScrubInputProps) {
  const [editing, setEditing] = useState(false);
  const [local, setLocal] = useState(value);
  const numVal = parseFloat(value);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (isNaN(numVal)) return;
    const startX = e.clientX;
    const startVal = numVal;
    const onMove = (ev: MouseEvent) => {
      const delta = ev.clientX - startX;
      const step = ev.shiftKey ? 10 : 1;
      const newVal = startVal + delta * step;
      const rounded = Math.round(newVal);
      const unit = value.match(/[a-z%]+$/)?.[0] ?? "px";
      const newStr = `${rounded}${unit}`;
      setLocal(newStr);
      onChange(newStr);
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }, [numVal, value, onChange]);

  return (
    <input
      value={editing ? local : value}
      onChange={(e) => {
        const v = e.target.value;
        setLocal(v);
        onChange(v);
      }}
      onFocus={() => { setEditing(true); setLocal(value); }}
      onBlur={() => setEditing(false)}
      onMouseDown={handleMouseDown}
      style={{
        width: 60,
        padding: "3px 6px",
        fontSize: 12,
        fontFamily: "monospace",
        border: "1px solid #e2e8f0",
        borderRadius: 4,
        outline: "none",
        cursor: isNaN(numVal) ? "text" : "ew-resize",
        ...style,
      }}
    />
  );
}

export function TokenEditor() {
  const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
  const addToken = usePoiesisStore((s) => s.addToken);
  const updateToken = usePoiesisStore((s) => s.updateToken);
  const removeToken = usePoiesisStore((s) => s.removeToken);

  const [search, setSearch] = useState("");
  const [pickerTokenId, setPickerTokenId] = useState<string | null>(null);
  const [pickerAnchor, setPickerAnchor] = useState<DOMRect | null>(null);
  const [paletteView, setPaletteView] = useState(false);

  const filtered = tokens.filter((t) =>
    t.name.toLowerCase().includes(search.toLowerCase())
  );

  const grouped = new Map<string, DesignToken[]>();
  for (const t of filtered) {
    const list = grouped.get(t.category) ?? [];
    list.push(t);
    grouped.set(t.category, list);
  }

  const handleAddToken = (category: string, tokenType: TokenType) => {
    const name = `${category}-${tokens.filter((t) => t.category === category).length + 1}`;
    const defaultVal = tokenType === "color" ? "#6366f1"
      : tokenType === "size" ? "0px"
      : "14px/1.5 system-ui";
    addToken(name, category, tokenType, defaultVal);
  };

  const renderColorTokens = (tokenList: DesignToken[]) => (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {paletteView ? (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
          {tokenList.map((t) => (
            <div
              key={t.id}
              onClick={(e) => {
                setPickerAnchor((e.target as HTMLElement).getBoundingClientRect());
                setPickerTokenId(t.id);
              }}
              style={{
                width: 60, height: 60, borderRadius: 8,
                background: t.value, border: "1px solid #d4d4d8",
                cursor: "pointer", position: "relative",
              }}
              title={`${t.name} — ${t.value}`}
            >
              <span style={{
                position: "absolute", bottom: 2, left: 4,
                fontSize: 9, color: "#fff", textShadow: "0 1px 2px #000",
              }}>{t.name}</span>
            </div>
          ))}
        </div>
      ) : (
        tokenList.map((t) => (
          <div key={t.id} style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div
              onClick={(e) => {
                if (t.tokenType === "color") {
                  setPickerAnchor((e.target as HTMLElement).getBoundingClientRect());
                  setPickerTokenId(t.id);
                }
              }}
              style={{
                width: 24, height: 24, borderRadius: 6,
                background: t.tokenType === "color" ? t.value : "transparent",
                border: "1px solid #d4d4d8",
                cursor: t.tokenType === "color" ? "pointer" : "default",
                flexShrink: 0,
                display: "flex", alignItems: "center", justifyContent: "center",
                fontSize: 10, color: "#94a3b8",
              }}
            >
              {t.tokenType !== "color" && t.tokenType[0].toUpperCase()}
            </div>
            <span style={{ fontSize: 12, color: "#374151", minWidth: 80 }}>{t.name}</span>
            <DragScrubInput
              value={t.value}
              onChange={(val) => updateToken(t.id, { value: val })}
            />
            <select
              value={t.tokenType}
              onChange={(e) => updateToken(t.id, { tokenType: e.target.value as TokenType })}
              style={{ fontSize: 10, padding: "1px 4px", border: "1px solid #e2e8f0", borderRadius: 3 }}
            >
              <option value="color">color</option>
              <option value="size">size</option>
              <option value="font">font</option>
              <option value="shadow">shadow</option>
            </select>
            <button
              onClick={() => removeToken(t.id)}
              style={{
                border: "none", background: "none", cursor: "pointer",
                color: "#ef4444", fontSize: 14, padding: "0 4px",
              }}
            >
              ×
            </button>
          </div>
        ))
      )}
      <button
        onClick={() => handleAddToken(tokenList[0]?.category ?? "colors", "color")}
        style={{
          padding: "3px 10px", fontSize: 11, border: "1px dashed #d4d4d8",
          borderRadius: 4, background: "#fff", color: "#94a3b8", cursor: "pointer",
        }}
      >
        + 添加{tokenList[0]?.category === "colors" ? "颜色" : ""}
      </button>
    </div>
  );

  const renderSizeTokens = (tokenList: DesignToken[]) => (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {tokenList.map((t) => {
        const maxVal = Math.max(...tokenList.map((x) => parseFloat(x.value) || 0), 1);
        const pct = ((parseFloat(t.value) || 0) / maxVal) * 100;
        return (
          <div key={t.id} style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontSize: 12, color: "#374151", minWidth: 60 }}>{t.name}</span>
            <DragScrubInput
              value={t.value}
              onChange={(val) => updateToken(t.id, { value: val })}
            />
            <div
              style={{
                flex: 1, height: 6, borderRadius: 3,
                background: `linear-gradient(to right, #6366f1 ${pct}%, #e2e8f0 ${pct}%)`,
              }}
              onClick={(e) => {
                const rect = (e.target as HTMLElement).getBoundingClientRect();
                const x = e.clientX - rect.left;
                const pct = x / rect.width;
                const newVal = Math.round(pct * maxVal);
                const unit = t.value.match(/[a-z%]+$/)?.[0] ?? "px";
                updateToken(t.id, { value: `${newVal}${unit}` });
              }}
              title={`${t.value} — click to adjust`}
              style={{ cursor: "pointer" }}
            />
            <button
              onClick={() => removeToken(t.id)}
              style={{
                border: "none", background: "none", cursor: "pointer",
                color: "#ef4444", fontSize: 14, padding: "0 4px",
              }}
            >
              ×
            </button>
          </div>
        );
      })}
      <button
        onClick={() => handleAddToken(tokenList[0]?.category ?? "spacing", "size")}
        style={{
          padding: "3px 10px", fontSize: 11, border: "1px dashed #d4d4d8",
          borderRadius: 4, background: "#fff", color: "#94a3b8", cursor: "pointer",
        }}
      >
        + 添加间距
      </button>
    </div>
  );

  const renderFontTokens = (tokenList: DesignToken[]) => (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {tokenList.map((t) => (
        <div key={t.id} style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 12, color: "#374151", minWidth: 60 }}>{t.name}</span>
          <input
            value={t.value}
            onChange={(e) => updateToken(t.id, { value: e.target.value })}
            style={{
              flex: 1, padding: "3px 6px", fontSize: 12, fontFamily: "monospace",
              border: "1px solid #e2e8f0", borderRadius: 4, outline: "none",
            }}
          />
          <button
            onClick={() => removeToken(t.id)}
            style={{
              border: "none", background: "none", cursor: "pointer",
              color: "#ef4444", fontSize: 14, padding: "0 4px",
            }}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );

  return (
    <div style={{ padding: 16 }}>
      {/* Header */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <input
          placeholder="搜索 token..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          style={{
            flex: 1, padding: "6px 10px", fontSize: 12,
            border: "1px solid #e2e8f0", borderRadius: 6, outline: "none",
          }}
        />
        <button
          onClick={() => handleAddToken("colors", "color")}
          style={{
            padding: "6px 14px", fontSize: 12, border: "none",
            borderRadius: 6, background: "#6366f1", color: "#fff",
            fontWeight: 600, cursor: "pointer",
          }}
        >
          + 新建 token
        </button>
        <label style={{ fontSize: 11, color: "#64748b", display: "flex", alignItems: "center", gap: 4 }}>
          <input
            type="checkbox"
            checked={paletteView}
            onChange={(e) => setPaletteView(e.target.checked)}
          />
          调色板视图
        </label>
      </div>

      {/* Token categories */}
      {[...grouped.entries()].map(([category, tokenList]) => (
        <div key={category} style={{ marginBottom: 20 }}>
          <h4
            style={{
              fontSize: 12, fontWeight: 600, color: "#64748b",
              textTransform: "uppercase", marginBottom: 8,
            }}
          >
            {CATEGORY_LABELS[category] ?? category} ({tokenList.length})
          </h4>
          {category === "colors" ? renderColorTokens(tokenList)
            : category === "spacing" || category === "radii" ? renderSizeTokens(tokenList)
            : renderFontTokens(tokenList)}
        </div>
      ))}

      {grouped.size === 0 && (
        <p style={{ fontSize: 13, color: "#94a3b8", textAlign: "center", marginTop: 40 }}>
          暂无 token · 点击"+ 新建 token"开始
        </p>
      )}

      {/* HUD Color Picker */}
      {pickerTokenId && pickerAnchor && (
        <ColorPicker
          color={tokens.find((t) => t.id === pickerTokenId)?.value ?? "#000000"}
          onChange={(color) => updateToken(pickerTokenId, { value: color })}
          onClose={() => { setPickerTokenId(null); setPickerAnchor(null); }}
          anchorRect={pickerAnchor}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/TokenEditor.tsx
git commit -m "feat: implement token editor with color HUD picker, drag-scrub, palette view"
```

---

## Phase D: Poiesis Class Editor UI

### Task D1: Class Editor Implementation

**Files:**
- Modify: `poiesis/src/components/design/ClassEditor.tsx`

- [ ] **Step 1: Implement class editor with grouping, pseudo-states, token binding**

```tsx
// poiesis/src/components/design/ClassEditor.tsx
import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import type { DesignClass, PseudoState } from "../../core/types/design";
import type { StyleProperty } from "../../core/types/style";
import { uid } from "../../core/utils";

// ── Property labels: designer-friendly ──
const PROPERTY_LABELS: Record<string, string> = {
  display: "Display",
  flexDirection: "方向",
  position: "Position",
  width: "宽度",
  height: "高度",
  padding: "内边距",
  paddingTop: "上内边距", paddingRight: "右内边距",
  paddingBottom: "下内边距", paddingLeft: "左内边距",
  margin: "外边距", marginTop: "上外边距",
  marginRight: "右外边距", marginBottom: "下外边距", marginLeft: "左外边距",
  gap: "间距",
  background: "背景色",
  color: "文字色",
  border: "边框",
  borderColor: "边框色",
  borderRadius: "圆角",
  boxShadow: "阴影",
  opacity: "透明度",
  fontFamily: "字体",
  fontSize: "字号",
  fontWeight: "字重",
  lineHeight: "行高",
  textAlign: "对齐",
  cursor: "光标",
  transition: "过渡",
  transform: "变换",
};

// ── Property categories ──
const LAYOUT_PROPS = new Set([
  "display", "position", "width", "height", "flexDirection",
  "justifyContent", "alignItems", "alignSelf",
  "padding", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
  "margin", "marginTop", "marginRight", "marginBottom", "marginLeft",
  "gap", "overflow", "overflowX", "overflowY", "flexWrap",
]);
const VISUAL_PROPS = new Set([
  "background", "border", "borderColor", "borderRadius", "boxShadow",
  "opacity", "cursor", "transition", "transform",
]);
const TEXT_PROPS = new Set([
  "color", "fontFamily", "fontSize", "fontWeight", "lineHeight",
  "textAlign", "textDecoration", "textTransform", "letterSpacing",
]);

function getCategory(propName: string): string {
  if (LAYOUT_PROPS.has(propName)) return "布局";
  if (VISUAL_PROPS.has(propName)) return "视觉";
  if (TEXT_PROPS.has(propName)) return "文字";
  return "其他";
}

const AVAILABLE_PSEUDOS = ["hover", "focus", "active", "disabled", "focus-visible"];
const DROPDOWN_PROPS: Record<string, string[]> = {
  display: ["block", "flex", "grid", "inline", "inline-block", "inline-flex", "none"],
  position: ["static", "relative", "absolute", "fixed", "sticky"],
  flexDirection: ["row", "row-reverse", "column", "column-reverse"],
  justifyContent: ["flex-start", "center", "flex-end", "space-between", "space-around"],
  alignItems: ["stretch", "flex-start", "center", "flex-end", "baseline"],
  fontWeight: ["400", "500", "600", "700", "800"],
  textAlign: ["left", "center", "right", "justify"],
  cursor: ["auto", "pointer", "text", "move", "not-allowed", "grab"],
};

interface PropertyRowProps {
  prop: StyleProperty;
  onChange: (id: string, value: string) => void;
  onRemove: (id: string) => void;
  onTokenBind: (id: string) => void;
}

function PropertyRow({ prop, onChange, onRemove, onTokenBind }: PropertyRowProps) {
  const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
  const [showTokenPicker, setShowTokenPicker] = useState(false);
  const opts = DROPDOWN_PROPS[prop.name];
  const label = PROPERTY_LABELS[prop.name] ?? prop.name;
  const isTokenBound = prop.segments.some((s) => s.type === "var-ref");
  const displayValue = prop.segments.map((s) =>
    s.type === "text" ? s.value : s.type === "var-ref" ? `var(${s.name})` : s.name
  ).join(" ");

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
      <span style={{ fontSize: 11, color: "#64748b", minWidth: 70, flexShrink: 0 }}>{label}</span>
      {opts ? (
        <select
          value={displayValue}
          onChange={(e) => onChange(prop.id, e.target.value)}
          style={{ flex: 1, padding: "2px 6px", fontSize: 11, border: "1px solid #e2e8f0", borderRadius: 4 }}
        >
          {opts.map((o) => <option key={o} value={o}>{o}</option>)}
        </select>
      ) : (
        <input
          value={displayValue}
          onChange={(e) => onChange(prop.id, e.target.value)}
          style={{ flex: 1, padding: "3px 6px", fontSize: 11, fontFamily: "monospace", border: "1px solid #e2e8f0", borderRadius: 4, outline: "none" }}
        />
      )}
      <button
        onClick={() => setShowTokenPicker(!showTokenPicker)}
        title="绑定 token"
        style={{
          border: "none", background: "none", cursor: "pointer",
          color: isTokenBound ? "#6366f1" : "#cbd5e1", fontSize: 14, padding: "0 2px",
        }}
      >
        🔗
      </button>
      <button
        onClick={() => onRemove(prop.id)}
        style={{ border: "none", background: "none", cursor: "pointer", color: "#ef4444", fontSize: 14, padding: "0 2px" }}
      >
        ×
      </button>
      {showTokenPicker && (
        <select
          size={Math.min(tokens.length + 1, 8)}
          onChange={(e) => {
            if (e.target.value) {
              onChange(prop.id, `var(${e.target.value})`);
            }
            setShowTokenPicker(false);
          }}
          style={{ position: "absolute", zIndex: 100, fontSize: 11 }}
        >
          <option value="">-- 选择 token --</option>
          {tokens.map((t) => (
            <option key={t.id} value={`--${t.name}`}>
              {t.name} ({t.value})
            </option>
          ))}
        </select>
      )}
    </div>
  );
}

export function ClassEditor() {
  const classes = usePoiesisStore((s) => s.activeDesign.classes);
  const addDesignClass = usePoiesisStore((s) => s.addDesignClass);
  const updateDesignClass = usePoiesisStore((s) => s.updateDesignClass);
  const removeDesignClass = usePoiesisStore((s) => s.removeDesignClass);

  const [selectedClassId, setSelectedClassId] = useState<string | null>(
    classes[0]?.id ?? null
  );
  const [activePseudo, setActivePseudo] = useState<string>("base");
  const [newClassName, setNewClassName] = useState("");

  const selectedClass = classes.find((c) => c.id === selectedClassId) ?? null;
  const isBase = activePseudo === "base";
  const currentProperties: StyleProperty[] = isBase
    ? selectedClass?.properties ?? []
    : selectedClass?.pseudoStates.find((p) => p.pseudo === activePseudo)?.properties ?? [];

  const getBasePropertyValue = (name: string): string | null => {
    if (!selectedClass) return null;
    const bp = selectedClass.properties.find((p) => p.name === name);
    if (!bp) return null;
    return bp.segments.map((s) => s.type === "text" ? s.value : `var(${(s as any).name})`).join(" ");
  };

  const handlePropChange = useCallback((propId: string, value: string) => {
    if (!selectedClass) return;
    if (isBase) {
      updateDesignClass(selectedClass.id, {
        properties: selectedClass.properties.map((p) =>
          p.id === propId ? { ...p, segments: [{ type: "text" as const, value }] } : p
        ),
      });
    } else {
      updateDesignClass(selectedClass.id, {
        pseudoStates: selectedClass.pseudoStates.map((ps) =>
          ps.pseudo === activePseudo
            ? {
                ...ps,
                properties: ps.properties.map((p) =>
                  p.id === propId ? { ...p, segments: [{ type: "text" as const, value }] } : p
                ),
              }
            : ps
        ),
      });
    }
  }, [selectedClass, isBase, activePseudo, updateDesignClass]);

  const handlePropRemove = useCallback((propId: string) => {
    if (!selectedClass) return;
    if (isBase) {
      updateDesignClass(selectedClass.id, {
        properties: selectedClass.properties.filter((p) => p.id !== propId),
      });
    } else {
      updateDesignClass(selectedClass.id, {
        pseudoStates: selectedClass.pseudoStates.map((ps) =>
          ps.pseudo === activePseudo
            ? { ...ps, properties: ps.properties.filter((p) => p.id !== propId) }
            : ps
        ),
      });
    }
  }, [selectedClass, isBase, activePseudo, updateDesignClass]);

  const handleAddProp = useCallback((propName: string) => {
    if (!selectedClass) return;
    const newProp: StyleProperty = {
      id: uid("sp"),
      name: propName,
      segments: [{ type: "text", value: "" }],
    };
    if (isBase) {
      updateDesignClass(selectedClass.id, { properties: [...selectedClass.properties, newProp] });
    } else {
      updateDesignClass(selectedClass.id, {
        pseudoStates: selectedClass.pseudoStates.map((ps) =>
          ps.pseudo === activePseudo
            ? { ...ps, properties: [...ps.properties, newProp] }
            : ps
        ),
      });
    }
  }, [selectedClass, isBase, activePseudo, updateDesignClass]);

  const handleAddPseudo = useCallback((pseudo: string) => {
    if (!selectedClass) return;
    const exists = selectedClass.pseudoStates.some((ps) => ps.pseudo === pseudo);
    if (exists) return;
    const newPseudo: PseudoState = { id: uid("ps"), pseudo, properties: [] };
    updateDesignClass(selectedClass.id, {
      pseudoStates: [...selectedClass.pseudoStates, newPseudo],
    });
    setActivePseudo(pseudo);
  }, [selectedClass, updateDesignClass]);

  // Group properties
  const grouped = new Map<string, StyleProperty[]>();
  for (const p of currentProperties) {
    const cat = getCategory(p.name);
    const list = grouped.get(cat) ?? [];
    list.push(p);
    grouped.set(cat, list);
  }

  return (
    <div style={{ display: "flex", height: "100%" }}>
      {/* Class list sidebar */}
      <div style={{ width: 160, borderRight: "1px solid #e2e8f0", padding: 12, overflow: "auto" }}>
        {classes.map((c) => (
          <div
            key={c.id}
            onClick={() => setSelectedClassId(c.id)}
            style={{
              padding: "6px 10px", borderRadius: 4, cursor: "pointer",
              background: c.id === selectedClassId ? "#eef2ff" : "transparent",
              color: c.id === selectedClassId ? "#6366f1" : "#374151",
              fontSize: 12, fontWeight: 500, marginBottom: 2,
            }}
          >
            .{c.name}
          </div>
        ))}
        <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
          <input
            value={newClassName}
            onChange={(e) => setNewClassName(e.target.value)}
            placeholder="新 class 名..."
            style={{ flex: 1, padding: "4px 8px", fontSize: 11, border: "1px solid #e2e8f0", borderRadius: 4 }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && newClassName.trim()) {
                addDesignClass(newClassName.trim());
                setNewClassName("");
              }
            }}
          />
          <button
            onClick={() => { if (newClassName.trim()) { addDesignClass(newClassName.trim()); setNewClassName(""); } }}
            style={{ padding: "4px 10px", fontSize: 11, border: "none", borderRadius: 4, background: "#6366f1", color: "#fff", cursor: "pointer" }}
          >
            +
          </button>
        </div>
      </div>

      {/* Property editor */}
      <div style={{ flex: 1, padding: 16, overflow: "auto" }}>
        {!selectedClass ? (
          <p style={{ color: "#94a3b8", fontSize: 13, textAlign: "center", marginTop: 40 }}>
            选择一个 class 或新建
          </p>
        ) : (
          <>
            {/* Header */}
            <div style={{ marginBottom: 12 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                <h3 style={{ fontSize: 15, fontWeight: 600, color: "#1e293b", margin: 0 }}>
                  .{selectedClass.name}
                </h3>
                <span style={{ fontSize: 11, color: "#94a3b8" }}>{classes.filter((c) => c.id === selectedClass.id).length} elements</span>
              </div>

              {/* Pseudo tabs */}
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                <button
                  onClick={() => setActivePseudo("base")}
                  style={{
                    padding: "2px 10px", fontSize: 11, borderRadius: 4, border: "none",
                    background: isBase ? "#6366f1" : "#f1f5f9",
                    color: isBase ? "#fff" : "#64748b", cursor: "pointer",
                  }}
                >
                  Base
                </button>
                {selectedClass.pseudoStates.map((ps) => (
                  <button
                    key={ps.id}
                    onClick={() => setActivePseudo(ps.pseudo)}
                    style={{
                      padding: "2px 10px", fontSize: 11, borderRadius: 4, border: "none",
                      background: activePseudo === ps.pseudo ? "#6366f1" : "#f1f5f9",
                      color: activePseudo === ps.pseudo ? "#fff" : "#64748b", cursor: "pointer",
                    }}
                  >
                    :{ps.pseudo}
                  </button>
                ))}
                {/* Add pseudo dropdown */}
                <select
                  value=""
                  onChange={(e) => { if (e.target.value) handleAddPseudo(e.target.value); }}
                  style={{
                    padding: "2px 8px", fontSize: 11, borderRadius: 4,
                    border: "1px dashed #d4d4d8", background: "#fff", color: "#94a3b8", cursor: "pointer",
                  }}
                >
                  <option value="">+ 伪类</option>
                  {AVAILABLE_PSEUDOS.filter((p) => !selectedClass.pseudoStates.some((ps) => ps.pseudo === p)).map((p) => (
                    <option key={p} value={p}>:{p}</option>
                  ))}
                </select>
              </div>
            </div>

            {/* Property groups */}
            {[...grouped.entries()].map(([category, props]) => (
              <div key={category} style={{ marginBottom: 16 }}>
                <h5 style={{
                  fontSize: 10, fontWeight: 600, color: "#94a3b8",
                  textTransform: "uppercase", marginBottom: 6,
                  paddingBottom: 4, borderBottom: "1px solid #f1f5f9",
                }}>
                  {category}
                </h5>
                {props.map((p) => {
                  const baseVal = !isBase ? getBasePropertyValue(p.name) : null;
                  const currentVal = p.segments.map((s) => s.type === "text" ? s.value : `var(${(s as any).name})`).join(" ");
                  const isOverride = !isBase && baseVal !== null && currentVal !== baseVal;
                  return (
                    <div key={p.id} style={{ position: "relative" }}>
                      {isOverride && (
                        <div style={{ position: "absolute", left: -8, top: 4, width: 6, height: 6, borderRadius: "50%", background: "#6366f1" }} />
                      )}
                      <div style={{ opacity: isOverride ? 1 : (!isBase && baseVal !== null ? 0.5 : 1) }}>
                        <PropertyRow
                          prop={p}
                          onChange={handlePropChange}
                          onRemove={handlePropRemove}
                          onTokenBind={() => {}}
                        />
                      </div>
                      {!isBase && baseVal !== null && !isOverride && (
                        <div style={{ fontSize: 9, color: "#94a3b8", marginLeft: 78, marginTop: -2 }} title={`继承自 Base: ${baseVal}`}>
                          继承自 Base ({baseVal})
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            ))}

            {/* Add property */}
            <AddPropertyDropdown onSelect={handleAddProp} />
          </>
        )}
      </div>
    </div>
  );
}

// ── Add Property Dropdown ──
const ALL_PROP_NAMES = Object.keys(PROPERTY_LABELS);

function AddPropertyDropdown({ onSelect }: { onSelect: (name: string) => void }) {
  const [search, setSearch] = useState("");
  const [open, setOpen] = useState(false);

  const filtered = search.trim()
    ? ALL_PROP_NAMES.filter((n) =>
        (PROPERTY_LABELS[n] ?? n).toLowerCase().includes(search.toLowerCase()) ||
        n.toLowerCase().includes(search.toLowerCase())
      ).slice(0, 10)
    : [];

  return (
    <div style={{ position: "relative" }}>
      <button
        onClick={() => setOpen(!open)}
        style={{
          padding: "4px 12px", fontSize: 11, border: "1px dashed #d4d4d8",
          borderRadius: 4, background: "#fff", color: "#94a3b8", cursor: "pointer", width: "100%",
        }}
      >
        + 添加属性
      </button>
      {open && (
        <div style={{
          position: "absolute", top: "100%", left: 0, width: "100%",
          background: "#fff", border: "1px solid #e2e8f0", borderRadius: 6,
          boxShadow: "0 4px 12px rgba(0,0,0,0.1)", zIndex: 100, padding: 4,
        }}>
          <input
            autoFocus
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索属性..."
            style={{
              width: "100%", padding: "4px 8px", fontSize: 11,
              border: "1px solid #e2e8f0", borderRadius: 4, outline: "none", boxSizing: "border-box",
            }}
          />
          <div style={{ maxHeight: 200, overflow: "auto", marginTop: 4 }}>
            {filtered.map((name) => (
              <div
                key={name}
                onClick={() => { onSelect(name); setOpen(false); setSearch(""); }}
                style={{
                  padding: "4px 8px", fontSize: 11, cursor: "pointer",
                  borderRadius: 3, color: "#374151",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "#f1f5f9")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              >
                {PROPERTY_LABELS[name] ?? name} <span style={{ color: "#94a3b8" }}>({name})</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/ClassEditor.tsx
git commit -m "feat: implement class editor with grouped properties, pseudo-states, token binding"
```

---

## Phase E: Poiesis Variant Editor UI

### Task E1: Variant Editor — Dimensions Tree

**Files:**
- Modify: `poiesis/src/components/design/VariantEditor.tsx`

- [ ] **Step 1: Implement variant editor with dimension tree, base editor, branch editor**

```tsx
// poiesis/src/components/design/VariantEditor.tsx
import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import type { VariantDefinition, VariantDimension, VariantBranch } from "../../core/types/design";
import type { StyleProperty } from "../../core/types/style";
import { uid } from "../../core/utils";

export function VariantEditor() {
  const variants = usePoiesisStore((s) => s.activeDesign.variants);
  const addVariant = usePoiesisStore((s) => s.addVariant);
  const updateVariant = usePoiesisStore((s) => s.updateVariant);
  const removeVariant = usePoiesisStore((s) => s.removeVariant);

  const [selectedId, setSelectedId] = useState<string | null>(variants[0]?.id ?? null);
  const [editingBranchKey, setEditingBranchKey] = useState<string | null>(null); // "dim.value" or "base"
  const [newDimName, setNewDimName] = useState("");
  const [newDimValue, setNewDimValue] = useState("");
  const [newVariantName, setNewVariantName] = useState("");

  const variant = variants.find((v) => v.id === selectedId) ?? null;

  const handleAddDimension = useCallback(() => {
    if (!variant || !newDimName.trim()) return;
    const dim: VariantDimension = { name: newDimName.trim(), values: [] };
    updateVariant(variant.id, { dimensions: [...variant.dimensions, dim] });
    setNewDimName("");
  }, [variant, newDimName, updateVariant]);

  const handleAddDimValue = useCallback((dimName: string) => {
    if (!variant || !newDimValue.trim()) return;
    updateVariant(variant.id, {
      dimensions: variant.dimensions.map((d) =>
        d.name === dimName ? { ...d, values: [...d.values, newDimValue.trim()] } : d
      ),
    });
    setNewDimValue("");
  }, [variant, newDimValue, updateVariant]);

  const handleRemoveDimValue = useCallback((dimName: string, value: string) => {
    if (!variant) return;
    updateVariant(variant.id, {
      dimensions: variant.dimensions.map((d) =>
        d.name === dimName ? { ...d, values: d.values.filter((v) => v !== value) } : d
      ),
      branches: variant.branches.filter(
        (b) => !(b.dimension === dimName && b.value === value)
      ),
    });
  }, [variant, updateVariant]);

  const handleSetDefault = useCallback((dimName: string, value: string) => {
    if (!variant) return;
    updateVariant(variant.id, {
      defaults: { ...variant.defaults, [dimName]: value },
    });
  }, [variant, updateVariant]);

  // Get the properties being edited (base or a branch)
  const editingProperties: StyleProperty[] = (() => {
    if (!variant) return [];
    if (editingBranchKey === "base") return variant.baseProperties;
    if (editingBranchKey) {
      const [dim, val] = editingBranchKey.split(".");
      const branch = variant.branches.find((b) => b.dimension === dim && b.value === val);
      return branch?.properties ?? [];
    }
    return [];
  })();

  const handleEditProp = useCallback((propId: string, value: string) => {
    if (!variant) return;
    const updateProps = (props: StyleProperty[]) =>
      props.map((p) => p.id === propId ? { ...p, segments: [{ type: "text" as const, value }] } : p);

    if (editingBranchKey === "base") {
      updateVariant(variant.id, { baseProperties: updateProps(variant.baseProperties) });
    } else if (editingBranchKey) {
      const [dim, val] = editingBranchKey.split(".");
      updateVariant(variant.id, {
        branches: variant.branches.map((b) =>
          b.dimension === dim && b.value === val
            ? { ...b, properties: updateProps(b.properties) }
            : b
        ),
      });
    }
  }, [variant, editingBranchKey, updateVariant]);

  const handleAddBranchProp = useCallback((propName: string) => {
    if (!variant) return;
    const newProp: StyleProperty = { id: uid("sp"), name: propName, segments: [{ type: "text", value: "" }] };
    const addProp = (props: StyleProperty[]) => [...props, newProp];

    if (editingBranchKey === "base") {
      updateVariant(variant.id, { baseProperties: addProp(variant.baseProperties) });
    } else if (editingBranchKey) {
      const [dim, val] = editingBranchKey.split(".");
      const exists = variant.branches.some((b) => b.dimension === dim && b.value === val);
      if (exists) {
        updateVariant(variant.id, {
          branches: variant.branches.map((b) =>
            b.dimension === dim && b.value === val
              ? { ...b, properties: addProp(b.properties) }
              : b
          ),
        });
      } else {
        const newBranch: VariantBranch = {
          id: uid("vb"), dimension: dim, value: val, properties: [newProp],
        };
        updateVariant(variant.id, { branches: [...variant.branches, newBranch] });
      }
    }
  }, [variant, editingBranchKey, updateVariant]);

  return (
    <div style={{ display: "flex", height: "100%" }}>
      {/* Variant list sidebar */}
      <div style={{ width: 160, borderRight: "1px solid #e2e8f0", padding: 12, overflow: "auto" }}>
        {variants.map((v) => (
          <div
            key={v.id}
            onClick={() => setSelectedId(v.id)}
            style={{
              padding: "6px 10px", borderRadius: 4, cursor: "pointer",
              background: v.id === selectedId ? "#eef2ff" : "transparent",
              color: v.id === selectedId ? "#6366f1" : "#374151",
              fontSize: 12, fontWeight: 500, marginBottom: 2,
            }}
          >
            {v.name}
          </div>
        ))}
        <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
          <input
            value={newVariantName}
            onChange={(e) => setNewVariantName(e.target.value)}
            placeholder="Variant 名..."
            style={{ flex: 1, padding: "4px 8px", fontSize: 11, border: "1px solid #e2e8f0", borderRadius: 4 }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && newVariantName.trim()) {
                addVariant(newVariantName.trim());
                setNewVariantName("");
              }
            }}
          />
          <button
            onClick={() => { if (newVariantName.trim()) { addVariant(newVariantName.trim()); setNewVariantName(""); } }}
            style={{ padding: "4px 10px", fontSize: 11, border: "none", borderRadius: 4, background: "#6366f1", color: "#fff", cursor: "pointer" }}
          >
            +
          </button>
        </div>
      </div>

      {/* Editor */}
      <div style={{ flex: 1, padding: 16, overflow: "auto" }}>
        {!variant ? (
          <p style={{ color: "#94a3b8", fontSize: 13, textAlign: "center", marginTop: 40 }}>
            选择一个 variant 或新建
          </p>
        ) : (
          <>
            <h3 style={{ fontSize: 15, fontWeight: 600, color: "#1e293b", margin: 0 }}>
              {variant.name}
            </h3>

            {/* Dimensions */}
            <div style={{ marginTop: 12 }}>
              <h5 style={{ fontSize: 10, fontWeight: 600, color: "#94a3b8", textTransform: "uppercase", marginBottom: 6 }}>
                Dimensions
              </h5>
              {variant.dimensions.map((dim) => (
                <div key={dim.name} style={{ marginBottom: 10 }}>
                  <div style={{ fontSize: 12, fontWeight: 600, color: "#374151", marginBottom: 4 }}>
                    {dim.name}
                    {variant.defaults[dim.name] && (
                      <span style={{ fontSize: 10, color: "#94a3b8", marginLeft: 6 }}>
                        (默认: {variant.defaults[dim.name]})
                      </span>
                    )}
                  </div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center" }}>
                    {dim.values.map((val) => (
                      <span
                        key={val}
                        style={{
                          padding: "2px 10px", fontSize: 11, borderRadius: 4,
                          background: variant.defaults[dim.name] === val ? "#6366f1" : "#f1f5f9",
                          color: variant.defaults[dim.name] === val ? "#fff" : "#374151",
                          cursor: "pointer",
                          position: "relative",
                        }}
                        onClick={() => handleSetDefault(dim.name, val)}
                      >
                        {val}
                        <span
                          onClick={(e) => { e.stopPropagation(); handleRemoveDimValue(dim.name, val); }}
                          style={{ marginLeft: 4, color: "inherit", opacity: 0.5, cursor: "pointer" }}
                        >
                          ×
                        </span>
                      </span>
                    ))}
                    <div style={{ display: "flex", gap: 2 }}>
                      <input
                        value={newDimValue}
                        onChange={(e) => setNewDimValue(e.target.value)}
                        placeholder={`+ ${dim.name} 值`}
                        style={{ width: 80, padding: "2px 6px", fontSize: 10, border: "1px dashed #d4d4d8", borderRadius: 4 }}
                        onKeyDown={(e) => { if (e.key === "Enter") handleAddDimValue(dim.name); }}
                      />
                    </div>
                  </div>
                </div>
              ))}
              <div style={{ display: "flex", gap: 4 }}>
                <input
                  value={newDimName}
                  onChange={(e) => setNewDimName(e.target.value)}
                  placeholder="新维度名..."
                  style={{ padding: "3px 8px", fontSize: 11, border: "1px dashed #d4d4d8", borderRadius: 4, width: 100 }}
                  onKeyDown={(e) => { if (e.key === "Enter") handleAddDimension(); }}
                />
                <button
                  onClick={handleAddDimension}
                  disabled={!newDimName.trim()}
                  style={{
                    padding: "3px 10px", fontSize: 11, border: "none", borderRadius: 4,
                    background: newDimName.trim() ? "#6366f1" : "#e2e8f0",
                    color: newDimName.trim() ? "#fff" : "#94a3b8", cursor: newDimName.trim() ? "pointer" : "default",
                  }}
                >
                  + 添加维度
                </button>
              </div>
            </div>

            {/* Default values display */}
            {Object.keys(variant.defaults).length > 0 && (
              <div style={{ marginTop: 12, fontSize: 11, color: "#64748b" }}>
                默认组合: {Object.entries(variant.defaults).map(([k, v]) => `${k}=${v}`).join(", ")}
              </div>
            )}

            {/* Branch selector */}
            <div style={{ marginTop: 16 }}>
              <h5 style={{ fontSize: 10, fontWeight: 600, color: "#94a3b8", textTransform: "uppercase", marginBottom: 6 }}>
                编辑分支
              </h5>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                <button
                  onClick={() => setEditingBranchKey("base")}
                  style={{
                    padding: "3px 12px", fontSize: 11, borderRadius: 4, border: "none",
                    background: editingBranchKey === "base" ? "#6366f1" : "#f1f5f9",
                    color: editingBranchKey === "base" ? "#fff" : "#374151", cursor: "pointer",
                  }}
                >
                  base
                </button>
                {variant.dimensions.flatMap((dim) =>
                  dim.values.map((val) => (
                    <button
                      key={`${dim.name}.${val}`}
                      onClick={() => setEditingBranchKey(`${dim.name}.${val}`)}
                      style={{
                        padding: "3px 10px", fontSize: 11, borderRadius: 4, border: "none",
                        background: editingBranchKey === `${dim.name}.${val}` ? "#6366f1" : "#f1f5f9",
                        color: editingBranchKey === `${dim.name}.${val}` ? "#fff" : "#374151", cursor: "pointer",
                      }}
                    >
                      {dim.name}.{val}
                    </button>
                  ))
                )}
              </div>
            </div>

            {/* Editing properties */}
            {editingBranchKey && (
              <div style={{ marginTop: 12 }}>
                <div style={{ fontSize: 11, color: "#94a3b8", marginBottom: 8 }}>
                  编辑: {editingBranchKey}（仅显示与 base 不同的属性 · 灰色=继承自 base）
                </div>
                {editingProperties.map((p) => {
                  const baseVal = editingBranchKey !== "base"
                    ? variant.baseProperties.find((bp) => bp.name === p.name)?.segments.map((s) => s.type === "text" ? s.value : "").join(" ") ?? null
                    : null;
                  return (
                    <div key={p.id} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                      <span style={{ fontSize: 11, color: "#64748b", minWidth: 80 }}>{p.name}</span>
                      <input
                        value={p.segments.map((s) => s.type === "text" ? s.value : "").join(" ")}
                        onChange={(e) => handleEditProp(p.id, e.target.value)}
                        style={{ flex: 1, padding: "2px 6px", fontSize: 11, fontFamily: "monospace", border: "1px solid #e2e8f0", borderRadius: 4 }}
                      />
                      {baseVal && <span style={{ fontSize: 9, color: "#94a3b8" }}>← base: {baseVal}</span>}
                    </div>
                  );
                })}
                <button
                  onClick={() => {
                    const name = prompt("属性名 (如 padding, background):");
                    if (name) handleAddBranchProp(name.trim());
                  }}
                  style={{
                    padding: "3px 10px", fontSize: 11, border: "1px dashed #d4d4d8",
                    borderRadius: 4, background: "#fff", color: "#94a3b8", cursor: "pointer", marginTop: 4,
                  }}
                >
                  + 添加属性
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/VariantEditor.tsx
git commit -m "feat: implement variant editor with dimensions, branches, base inheritance"
```

---

## Phase F: ContextInspector Upgrade + Code Generation

### Task F1: Upgrade ContextInspector with Class/Variant Binding

**Files:**
- Modify: `poiesis/src/components/panels/ContextInspector.tsx`

- [ ] **Step 1: Add Class and Variant sections to ContextInspector**

Add these sections below the existing `<PropsSection>` and `<StylesSection>`:

```tsx
// Add imports at top:
import { usePoiesisStore } from "../../store";
import type { DesignClass } from "../../core/types/design";

// Inside ContextInspector component, after existing content:

function ClassesSection({ element }: { element: ElementData }) {
  const classes = usePoiesisStore((s) => s.activeDesign.classes);
  const setElementClassRefs = usePoiesisStore((s) => s.setElementClassRefs);
  const elementClassRefs: string[] = "style" in element ? (element.style?.classRefs ?? []) : [];

  const [search, setSearch] = useState("");

  const handleBind = (className: string) => {
    const newRefs = [...new Set([...elementClassRefs, className])];
    setElementClassRefs(element.id, newRefs);
  };

  const handleUnbind = (className: string) => {
    setElementClassRefs(element.id, elementClassRefs.filter((c) => c !== className));
  };

  const filtered = classes.filter((c) =>
    c.name.toLowerCase().includes(search.toLowerCase()) &&
    !elementClassRefs.includes(c.name)
  );

  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: "#6b7280", textTransform: "uppercase", marginBottom: 6 }}>
        Classes
      </div>
      {/* Bound class pills */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }}>
        {elementClassRefs.map((cn) => (
          <span
            key={cn}
            style={{
              padding: "2px 8px", fontSize: 11, borderRadius: 4,
              background: "#eef2ff", color: "#6366f1", cursor: "default",
              display: "flex", alignItems: "center", gap: 4,
            }}
          >
            .{cn}
            <span onClick={() => handleUnbind(cn)} style={{ cursor: "pointer", opacity: 0.5 }}>
              ×
            </span>
          </span>
        ))}
      </div>
      {/* Search + bind */}
      <div style={{ position: "relative" }}>
        <input
          placeholder="🔍 搜索 class..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          style={{
            width: "100%", padding: "4px 8px", fontSize: 11,
            border: "1px solid #e2e8f0", borderRadius: 4, outline: "none",
            boxSizing: "border-box",
          }}
        />
        {search.trim() && filtered.length > 0 && (
          <div style={{
            position: "absolute", top: "100%", left: 0, width: "100%",
            background: "#fff", border: "1px solid #e2e8f0", borderRadius: 4,
            boxShadow: "0 2px 8px rgba(0,0,0,0.1)", zIndex: 100, maxHeight: 160, overflow: "auto",
          }}>
            {filtered.map((c) => (
              <div
                key={c.id}
                onClick={() => { handleBind(c.name); setSearch(""); }}
                style={{
                  padding: "4px 8px", fontSize: 11, cursor: "pointer",
                  color: "#374151", borderBottom: "1px solid #f1f5f9",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "#f1f5f9")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              >
                .{c.name}
                <span style={{ color: "#94a3b8", marginLeft: 8, fontSize: 10 }}>
                  {c.properties.length} 属性
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function VariantBindingSection({ element }: { element: ElementData }) {
  const variants = usePoiesisStore((s) => s.activeDesign.variants);
  const elementVariants = usePoiesisStore((s) => s.elementVariants);
  const bindVariantToElement = usePoiesisStore((s) => s.bindVariantToElement);
  const unbindVariantFromElement = usePoiesisStore((s) => s.unbindVariantFromElement);

  const binding = elementVariants[element.id];

  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: "#6b7280", textTransform: "uppercase", marginBottom: 6 }}>
        Variant
      </div>
      {binding ? (
        <div>
          <div style={{ fontSize: 12, fontWeight: 600, color: "#374151", marginBottom: 4 }}>
            {binding.variantName}
          </div>
          {Object.entries(binding.dimensionValues).map(([dim, val]) => {
            const variantDef = variants.find((v) => v.name === binding.variantName);
            const dimDef = variantDef?.dimensions.find((d) => d.name === dim);
            const values = dimDef?.values ?? [];
            return (
              <div key={dim} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
                <span style={{ fontSize: 11, color: "#64748b", minWidth: 50 }}>{dim}:</span>
                <select
                  value={val}
                  onChange={(e) =>
                    bindVariantToElement(element.id, binding.variantName, {
                      ...binding.dimensionValues,
                      [dim]: e.target.value,
                    })
                  }
                  style={{ flex: 1, padding: "2px 6px", fontSize: 11, border: "1px solid #e2e8f0", borderRadius: 4 }}
                >
                  {values.map((v) => (
                    <option key={v} value={v}>{v}</option>
                  ))}
                </select>
              </div>
            );
          })}
          <button
            onClick={() => unbindVariantFromElement(element.id)}
            style={{
              marginTop: 6, padding: "2px 8px", fontSize: 10, border: "1px solid #e2e8f0",
              borderRadius: 4, background: "#fff", color: "#ef4444", cursor: "pointer",
            }}
          >
            解除 variant
          </button>
        </div>
      ) : (
        <select
          value=""
          onChange={(e) => {
            if (!e.target.value) return;
            const variant = variants.find((v) => v.name === e.target.value);
            if (!variant) return;
            bindVariantToElement(element.id, variant.name, variant.defaults);
          }}
          style={{ width: "100%", padding: "3px 6px", fontSize: 11, border: "1px solid #e2e8f0", borderRadius: 4 }}
        >
          <option value="">选择 variant...</option>
          {variants.map((v) => (
            <option key={v.id} value={v.name}>{v.name}</option>
          ))}
        </select>
      )}
    </div>
  );
}
```

Then, in the main `ContextInspector` render, add these sections between `<PropsSection>` and `<StylesSection>`:

```tsx
export function ContextInspector() {
  // ...existing code...

  return (
    <div className="context-inspector" ...>
      {/* existing header */}
      <PropsSection element={el} />
      {/* NEW: Classes and Variant for design-tab-aware elements */}
      {isBasic && (
        <>
          <ClassesSection element={el} />
          <VariantBindingSection element={el} />
        </>
      )}
      {isBasic && <StylesSection element={el} />}
      {!isBasic && <EventsSection element={el} />}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/panels/ContextInspector.tsx
git commit -m "feat: add class binding and variant binding sections to ContextInspector"
```

---

### Task F2: Design → .omni Code Emitter

**Files:**
- Create: `poiesis/src/services/codegen/design-emitter.ts`

- [ ] **Step 1: Create design.omni source emitter**

```typescript
// poiesis/src/services/codegen/design-emitter.ts
// Emits `design AppDesign { ... }` from DesignDefinition state.
import type { DesignDefinition, DesignToken, DesignClass, VariantDefinition } from "../../core/types/design";
import type { StyleProperty } from "../../core/types/style";
import { compileSegments } from "./collect-styles";

export function emitDesignSource(design: DesignDefinition): string {
  const lines: string[] = [];
  lines.push("import omniview;");
  lines.push("");
  lines.push(`design ${design.name} {`);

  // ── Tokens ──
  if (design.tokens.length > 0) {
    lines.push("    // ── Tokens ──");
    const grouped = groupBy(design.tokens, (t) => t.category);
    for (const [category, tokens] of Object.entries(grouped)) {
      const typeLabel = tokens[0]?.tokenType ?? "color";
      lines.push(`    // ${category}`);
      for (const t of tokens) {
        const q = typeLabel === "size" || typeLabel === "color" ? '"' : "";
        lines.push(`    token ${t.name}: ${typeLabel} = ${q}${t.value}${q};`);
      }
    }
    lines.push("");
  }

  // ── Classes ──
  for (const cls of design.classes) {
    emitClass(lines, cls, "    ");
    lines.push("");
  }

  // ── Variants ──
  for (const variant of design.variants) {
    emitVariant(lines, variant, "    ");
    lines.push("");
  }

  lines.push("}");
  return lines.join("\n") + "\n";
}

function emitClass(lines: string[], cls: DesignClass, indent: string) {
  lines.push(`${indent}class ${cls.name} {`);
  for (const prop of cls.properties) {
    const val = compileSegments(prop.segments);
    lines.push(`${indent}    ${prop.name}: ${val};`);
  }
  for (const pseudo of cls.pseudoStates) {
    lines.push(`${indent}    &:${pseudo.pseudo} {`);
    for (const prop of pseudo.properties) {
      const val = compileSegments(prop.segments);
      lines.push(`${indent}        ${prop.name}: ${val};`);
    }
    lines.push(`${indent}    }`);
  }
  lines.push(`${indent}}`);
}

function emitVariant(lines: string[], v: VariantDefinition, indent: string) {
  lines.push(`${indent}variant ${v.name} {`);
  // @dim declarations
  for (const dim of v.dimensions) {
    const vals = dim.values.join(" | ");
    lines.push(`${indent}    @dim ${dim.name}: ${vals};`);
  }
  // @default
  if (Object.keys(v.defaults).length > 0) {
    const defs = Object.entries(v.defaults).map(([k, val]) => `${k}: ${val}`).join("; ");
    lines.push(`${indent}    @default { ${defs}; }`);
  }
  // base
  if (v.baseProperties.length > 0) {
    lines.push(`${indent}    base {`);
    for (const prop of v.baseProperties) {
      const val = compileSegments(prop.segments);
      lines.push(`${indent}        ${prop.name}: ${val};`);
    }
    lines.push(`${indent}    }`);
  }
  // branches
  for (const branch of v.branches) {
    lines.push(`${indent}    ${branch.dimension}.${branch.value} {`);
    for (const prop of branch.properties) {
      const val = compileSegments(prop.segments);
      lines.push(`${indent}        ${prop.name}: ${val};`);
    }
    lines.push(`${indent}    }`);
  }
  lines.push(`${indent}}`);
}

function groupBy<T>(arr: T[], fn: (item: T) => string): Record<string, T[]> {
  const result: Record<string, T[]> = {};
  for (const item of arr) {
    const key = fn(item);
    (result[key] ??= []).push(item);
  }
  return result;
}
```

- [ ] **Step 2: Wire into save pipeline**

In `poiesis/src/services/codegen/pipeline.ts`, add a branch: when the active tab is a `design` tab, use `emitDesignSource` instead of the component pipeline:

```typescript
// Import
import { emitDesignSource } from "./design-emitter";

// In generateOmniSource or equivalent:
export function generateOmniSource(): string {
  const state = usePoiesisStore.getState();
  const tab = state.designPage.openTabs.find((t) => t.id === state.designPage.activeTabId);

  if (tab?.tabType === "design") {
    return emitDesignSource(state.activeDesign);
  }
  // ... existing component codegen ...
}
```

- [ ] **Step 3: Update DesignCodePreview to use emitter**

In `DesignTabContent.tsx`, update the `DesignCodePreview` component:

```tsx
function DesignCodePreview() {
  const design = usePoiesisStore((s) => s.activeDesign);
  const source = emitDesignSource(design);

  return (
    <pre
      style={{
        fontFamily: "monospace", fontSize: 12, color: "#1e293b",
        background: "#f8fafc", padding: 12, borderRadius: 6,
        overflow: "auto", whiteSpace: "pre-wrap", margin: 0,
      }}
    >
      {source}
    </pre>
  );
}
```

- [ ] **Step 4: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/design-emitter.ts poiesis/src/services/codegen/pipeline.ts poiesis/src/components/design/DesignTabContent.tsx
git commit -m "feat: add design→.omni source emitter and wire into save pipeline"
```

---

## Phase G: omnic Codegen — Token Resolution & Variant Expansion

### Task G1: Design IR Types

**Files:**
- Create: `omnilang/src/ir/design.rs`
- Modify: `omnilang/src/ir/mod.rs`

- [ ] **Step 1: Create IR types mirroring AST**

```rust
// omnilang/src/ir/design.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resolved design system in IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignIR {
    pub name: String,
    pub tokens: HashMap<String, TokenIR>,
    pub classes: Vec<ClassIR>,
    pub variants: Vec<VariantIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIR {
    pub value: String,
    pub token_type: TokenTypeIR,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenTypeIR {
    Color,
    Size,
    Font,
    Shadow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassIR {
    pub name: String,
    pub properties: HashMap<String, String>,      // property → resolved value
    pub pseudo_states: Vec<PseudoStateIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PseudoStateIR {
    pub pseudo: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantIR {
    pub name: String,
    pub dimensions: Vec<VariantDimIR>,
    pub defaults: HashMap<String, String>,
    pub base: HashMap<String, String>,
    pub branches: Vec<VariantBranchIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDimIR {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantBranchIR {
    pub dimension: String,
    pub value: String,
    pub properties: HashMap<String, String>,
}
```

- [ ] **Step 2: Register in IR mod**

```rust
// omnilang/src/ir/mod.rs
pub mod design;
```

- [ ] **Step 3: Build and verify**

Run: `cargo build -p omnilang 2>&1 | head -10`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/ir/design.rs omnilang/src/ir/mod.rs
git commit -m "feat: add DesignIR types for resolved design system"
```

---

### Task G2: AST → IR Lowering

**Files:**
- Create: `omnilang/src/ir/lowering/design.rs`
- Modify: `omnilang/src/ir/lowering/mod.rs`

- [ ] **Step 1: Implement lower_design**

```rust
// omnilang/src/ir/lowering/design.rs
use crate::ir::design::*;
use crate::parser::ast::design::*;
use std::collections::HashMap;

pub fn lower_design(ast: &DesignDef) -> DesignIR {
    let mut tokens = HashMap::new();

    // Step 1: Collect tokens
    for t in &ast.tokens {
        tokens.insert(t.name.clone(), TokenIR {
            value: t.value.clone(),
            token_type: match t.token_type {
                TokenType::Color => TokenTypeIR::Color,
                TokenType::Size => TokenTypeIR::Size,
                TokenType::Font => TokenTypeIR::Font,
                TokenType::Shadow => TokenTypeIR::Shadow,
            },
        });
    }

    // Step 2: Resolve token() references in class properties
    let resolve_value = |val: &DesignValue| -> String {
        match val {
            DesignValue::Literal(s) => s.clone(),
            DesignValue::TokenRef(name) => {
                tokens.get(name)
                    .map(|t| t.value.clone())
                    .unwrap_or_else(|| format!("var(--{})", name))
            }
            DesignValue::Mixed(parts) => {
                parts.iter().map(|p| match p {
                    DesignValuePart::Text(s) => s.clone(),
                    DesignValuePart::Token(name) => {
                        tokens.get(name)
                            .map(|t| t.value.clone())
                            .unwrap_or_else(|| format!("var(--{})", name))
                    }
                }).collect::<Vec<_>>().join("")
            }
        }
    };

    let resolve_props = |props: &[DesignProperty]| -> HashMap<String, String> {
        props.iter().map(|p| (p.name.clone(), resolve_value(&p.value))).collect()
    };

    // Step 3: Lower classes
    let classes: Vec<ClassIR> = ast.classes.iter().map(|c| ClassIR {
        name: c.name.clone(),
        properties: resolve_props(&c.properties),
        pseudo_states: c.pseudo_states.iter().map(|ps| PseudoStateIR {
            pseudo: ps.pseudo.clone(),
            properties: resolve_props(&ps.properties),
        }).collect(),
    }).collect();

    // Step 4: Lower variants
    let variants: Vec<VariantIR> = ast.variants.iter().map(|v| VariantIR {
        name: v.name.clone(),
        dimensions: v.dimensions.iter().map(|d| VariantDimIR {
            name: d.name.clone(),
            values: d.values.clone(),
        }).collect(),
        defaults: v.defaults.iter().cloned().collect(),
        base: resolve_props(&v.base),
        branches: v.branches.iter().map(|b| VariantBranchIR {
            dimension: b.dimension.clone(),
            value: b.value.clone(),
            properties: resolve_props(&b.properties),
        }).collect(),
    }).collect();

    DesignIR { name: ast.name.clone(), tokens, classes, variants }
}
```

- [ ] **Step 2: Register in lowering mod**

```rust
// omnilang/src/ir/lowering/mod.rs
pub mod design;
```

- [ ] **Step 3: Write integration test**

```rust
// omnilang/tests/design_lowering_test.rs
use omnilang::parser;
use omnilang::ir::lowering::design::lower_design;

#[test]
fn lower_token_resolves_value() {
    let src = r#"token primary: color = "#6366f1";"#;
    let ast = parser::design::parse_design_body(src).unwrap();
    let ir = lower_design(&ast);
    assert_eq!(ir.tokens.get("primary").unwrap().value, "#6366f1");
}

#[test]
fn lower_class_resolves_token_refs() {
    let src = r#"
        token surface: color = "#ffffff";
        class Card {
            background: token(surface);
            border-radius: "12px";
        }
    "#;
    let ast = parser::design::parse_design_body(src).unwrap();
    let ir = lower_design(&ast);
    let cls = &ir.classes[0];
    assert_eq!(cls.properties.get("background").unwrap(), "#ffffff");
    assert_eq!(cls.properties.get("border-radius").unwrap(), "12px");
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omnilang -- design_lowering 2>&1`
Expected: Tests PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/ir/lowering/design.rs omnilang/src/ir/lowering/mod.rs omnilang/tests/design_lowering_test.rs
git commit -m "feat: add design AST→IR lowering with token resolution"
```

---

### Task G3: Style Codegen — OmniView CSS Output

**Files:**
- Modify: `omnilang/src/codegen/omniview/view.rs` (or create `design.rs`)

- [ ] **Step 1: Implement emit_design_styles**

In the existing OmniView codegen, add a function that takes a `DesignIR` and emits CSS for all classes and resolved variants:

```rust
// omnilang/src/codegen/omniview/design.rs
use crate::ir::design::*;
use std::collections::HashMap;

/// Generate CSS for all design classes as scoped style blocks.
/// Each class becomes a CSS class scoped by `[data-component]`.
pub fn emit_design_css(design: &DesignIR) -> String {
    let mut css = String::new();

    for cls in &design.classes {
        css.push_str(&emit_class_css(cls));
        css.push('\n');
    }

    Ok(css)
}

fn emit_class_css(cls: &ClassIR) -> String {
    let mut out = String::new();

    // Base class
    out.push_str(&format!(".{} {{\n", cls.name));
    for (prop, val) in &cls.properties {
        out.push_str(&format!("  {}: {};\n", prop, val));
    }
    out.push_str("}\n");

    // Pseudo states
    for pseudo in &cls.pseudo_states {
        out.push_str(&format!(".{}:{} {{\n", cls.name, pseudo.pseudo));
        for (prop, val) in &pseudo.properties {
            out.push_str(&format!("  {}: {};\n", prop, val));
        }
        out.push_str("}\n");
    }

    out
}

/// Expand a variant binding into the set of CSS classes needed.
/// Returns (class_names, additional_css).
pub fn expand_variant(
    design: &DesignIR,
    variant_name: &str,
    dimension_values: &HashMap<String, String>,
) -> Result<(Vec<String>, String), String> {
    let variant = design.variants.iter()
        .find(|v| v.name == variant_name)
        .ok_or_else(|| format!("Variant '{}' not found", variant_name))?;

    let mut class_names = vec![format!("{}-base", variant_name)];
    let mut css = String::new();

    // Base styles
    css.push_str(&format!(".{}-base {{\n", variant_name));
    for (prop, val) in &variant.base {
        css.push_str(&format!("  {}: {};\n", prop, val));
    }
    css.push_str("}\n");

    // Apply selected branches
    for (dim, val) in dimension_values {
        class_names.push(format!("{}-{}-{}", variant_name, dim, val));

        // Branch properties
        if let Some(branch) = variant.branches.iter()
            .find(|b| b.dimension == *dim && b.value == *val)
        {
            css.push_str(&format!(".{}-{}-{} {{\n", variant_name, dim, val));
            for (prop, prop_val) in &branch.properties {
                css.push_str(&format!("  {}: {};\n", prop, prop_val));
            }
            css.push_str("}\n");
        }
    }

    Ok((class_names, css))
}
```

- [ ] **Step 2: Wire into existing OmniView codegen**

Find the main CSS emission function in `view.rs` and add a call to include design CSS when a design file is imported:

```rust
// In the component style emission path:
if let Some(design) = compilation_unit.design {
    let design_css = emit_design_css(design)?;
    css_output.push_str(&design_css);
}
```

- [ ] **Step 3: Integration test — full roundtrip**

Create `omnilang/tests/design_codegen_test.rs`:

```rust
use omnilang::parser;
use omnilang::ir::lowering::design::lower_design;
use omnilang::codegen::omniview::design::emit_design_css;

#[test]
fn emit_class_css_output() {
    let src = r#"
        token surface: color = "#ffffff";
        class Card {
            background: token(surface);
            border-radius: "12px";
            &:hover {
                box-shadow: "0 2px 8px rgba(0,0,0,0.1)";
            }
        }
    "#;
    let ast = parser::design::parse_design_body(src).unwrap();
    let ir = lower_design(&ast);
    let css = emit_design_css(&ir).unwrap();
    assert!(css.contains(".Card {"));
    assert!(css.contains("background: #ffffff;"));
    assert!(css.contains(".Card:hover {"));
}
```

- [ ] **Step 4: Run integration test**

Run: `cargo test -p omnilang -- design_codegen 2>&1`
Expected: Test PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/design.rs omnilang/src/codegen/omniview/view.rs omnilang/tests/design_codegen_test.rs
git commit -m "feat: add design→CSS codegen with token resolution and variant expansion"
```

---

## Phase H: Poiesis Design.omni Save/Load Pipeline

> **Critical constraint:** `.omni` is the source of truth. `project.json` stores only editor metadata (positions, viewport, panel layout). All token/class/variant data comes from parsing `design.omni` on load, and is written back to `design.omni` on save. Nothing design-related goes into `project.json`.

### Task H0: Design Annotation System — Stable IDs for Round-Tripping

**Files:**
- Modify: `poiesis/src/services/codegen/design-emitter.ts`
- Modify: `poiesis/src/services/codegen/parser.ts`
- Modify: `poiesis/src/core/types/design.ts`

- [ ] **Step 1: Add annotation constants to design types**

```typescript
// In poiesis/src/core/types/design.ts, add:
export const DESIGN_ANNOTATION_PREFIX = {
  token: "// @token",
  class: "// @class",
  variant: "// @variant",
} as const;

/** Parse a design annotation line, e.g. "// @token id=tk_1" → { type: "token", id: "tk_1" } */
export function parseDesignAnnotation(line: string): { type: "token" | "class" | "variant"; id: string } | null {
  const trimmed = line.trim();
  for (const [type, prefix] of Object.entries(DESIGN_ANNOTATION_PREFIX)) {
    if (trimmed.startsWith(prefix)) {
      const match = trimmed.match(/id=(\S+)/);
      if (match) {
        return { type: type as "token" | "class" | "variant", id: match[1] };
      }
    }
  }
  return null;
}
```

- [ ] **Step 2: Update design-emitter to emit annotations before each block**

In `emitDesignSource()`, add annotations:

```typescript
// In the token loop:
for (const t of tokens) {
  lines.push(`    // @token id=${t.id}`);
  const q = typeLabel === "size" || typeLabel === "color" ? '"' : "";
  lines.push(`    token ${t.name}: ${typeLabel} = ${q}${t.value}${q};`);
}

// Before each class:
lines.push(`    // @class id=${cls.id}`);
lines.push(`${indent}class ${cls.name} {`);

// Before each variant:
lines.push(`    // @variant id=${v.id}`);
lines.push(`${indent}variant ${v.name} {`);
```

- [ ] **Step 3: Update parser to read design annotations and use stable IDs**

In `parseDesignBody()`, detect annotation comments BEFORE each block:

```typescript
private parseDesignBody(): void {
  let pendingAnnotation: { type: string; id: string } | null = null;

  while (!this.stream.isEof()) {
    this.stream.skipNewlines();
    if (this.stream.isEof()) break;

    const tok = this.stream.peek();
    if (tok.kind === "BRACE_CLOSE") {
      this.stream.consume();
      this.popState();
      return;
    }

    // Detect design annotations: // @token id=..., // @class id=..., // @variant id=...
    if (tok.kind === "COMMENT_LINE") {
      const text = tok.text.trim();
      const ann = parseDesignAnnotation(text);
      if (ann) {
        pendingAnnotation = { type: ann.type, id: ann.id };
        this.stream.consume();
        continue;
      }
      // For non-annotation comments, skip
      this.stream.consume();
      continue;
    }

    if (tok.kind === "IDENTIFIER") {
      const kw = tok.text;
      if (kw === "token") {
        this.parseDesignToken(pendingAnnotation?.id ?? uid("tk"));
      } else if (kw === "class") {
        this.parseDesignClass(pendingAnnotation?.id ?? uid("dc"));
      } else if (kw === "variant") {
        this.parseDesignVariant(pendingAnnotation?.id ?? uid("dv"));
      } else {
        this.stream.consume();
      }
      pendingAnnotation = null; // consumed
    } else {
      this.stream.consume();
    }
  }
}
```

Update `parseDesignToken/Class/Variant` to accept the stable ID:

```typescript
private parseDesignToken(stableId: string): void {
  // ... parse name, type, value ...
  this.ctx.designTokens.push({
    id: stableId,  // Use annotation ID, not generated
    name: nameTok.text,
    tokenType: typeTok.text,
    value,
  });
}
```

- [ ] **Step 4: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/core/types/design.ts poiesis/src/services/codegen/design-emitter.ts poiesis/src/services/codegen/parser.ts
git commit -m "feat: add design annotation system (// @token/@class/@variant) for stable round-trip IDs"
```

---

### Task H1: Poiesis Parser — Parse `design` Blocks from .omni

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`

- [ ] **Step 1: Add `design` keyword to root-level parser**

In `parseRootIdentifier()` (line 194 of parser.ts), add a branch for `design` before the `component` branch:

```typescript
if (kw === "design") {
  this.stream.consume(); // "design"
  const nameTok = this.stream.expect("IDENTIFIER");
  if (nameTok.kind === "EOF") {
    this.pushError("error", "P_DESIGN", "Invalid design declaration: missing name", nameTok);
    return;
  }
  this.ctx.currentComponent = nameTok.text; // reuse for design name
  this.stream.expect("BRACE_OPEN");
  this.pushState("IN_DESIGN");
  return;
}
```

- [ ] **Step 2: Add `IN_DESIGN` state handler**

Add a new state `"IN_DESIGN"` to the `ParserState` type (in `parser-types.ts`):

```typescript
export type ParserState =
  | "ROOT"
  | "IN_COMPONENT"
  | "IN_VIEW"
  | "IN_FN"
  | "IN_STORE"
  | "IN_STYLE"
  | "IN_CHAIN"
  | "IN_DESIGN";  // NEW
```

Then add the handler in `pushState()` (line 614):

```typescript
case "IN_DESIGN":
  this.parseDesignBody();
  break;
```

- [ ] **Step 3: Implement `parseDesignBody()` — token/class/variant parsing**

Add these methods to the `Parser` class:

```typescript
private parseDesignBody(): void {
  while (!this.stream.isEof()) {
    this.stream.skipNewlines();
    if (this.stream.isEof()) break;

    const tok = this.stream.peek();
    if (tok.kind === "BRACE_CLOSE") {
      this.stream.consume();
      this.popState();
      return;
    }

    // Parse design items: token, class, variant
    if (tok.kind === "COMMENT_LINE") {
      this.stream.consume(); // skip comments inside design
      continue;
    }

    if (tok.kind === "IDENTIFIER") {
      const kw = tok.text;
      if (kw === "token") {
        this.parseDesignToken();
      } else if (kw === "class") {
        this.parseDesignClass();
      } else if (kw === "variant") {
        this.parseDesignVariant();
      } else {
        this.stream.consume(); // skip unknown
      }
    } else {
      this.stream.consume();
    }
  }
}

private parseDesignToken(): void {
  this.stream.consume(); // "token"
  const nameTok = this.stream.expect("IDENTIFIER");
  this.stream.expect("COLON");
  const typeTok = this.stream.expect("IDENTIFIER"); // color | size | font | shadow
  this.stream.expect("EQUALS");

  // Read value (quoted string or unquoted)
  let value = "";
  const valTok = this.stream.peek();
  if (valTok.kind === "STRING") {
    value = this.stream.consume().text.slice(1, -1);
  } else {
    // Read tokens until semicolon
    while (!this.stream.isEof() && this.stream.peek().kind !== "SEMICOLON") {
      value += this.stream.consume().text;
    }
    value = value.trim();
  }
  this.stream.expect("SEMICOLON");

  // Store token as a codeblock element in the parse context
  this.ctx.elements.push(this.codeblockEl(
    `design.${this.ctx.currentComponent}.token.${nameTok.text}`,
    "design-token",
    {
      name: nameTok.text,
      tokenType: typeTok.text,
      value,
      designName: this.ctx.currentComponent,
    },
  ));
}

private parseDesignClass(): void {
  this.stream.consume(); // "class"
  const nameTok = this.stream.expect("IDENTIFIER");
  this.stream.expect("BRACE_OPEN");

  // Parse class body — reuse style block parsing
  // Read until matching BRACE_CLOSE
  let depth = 1;
  let body = "";
  while (depth > 0 && !this.stream.isEof()) {
    const t = this.stream.peek();
    if (t.kind === "BRACE_OPEN") depth++;
    else if (t.kind === "BRACE_CLOSE") depth--;
    if (depth === 0) {
      this.stream.consume(); // consume closing }
      break;
    }
    body += this.stream.consume().text;
  }

  this.ctx.elements.push(this.codeblockEl(
    `design.${this.ctx.currentComponent}.class.${nameTok.text}`,
    "design-class",
    { name: nameTok.text, body, designName: this.ctx.currentComponent },
  ));
}

private parseDesignVariant(): void {
  this.stream.consume(); // "variant"
  const nameTok = this.stream.expect("IDENTIFIER");
  this.stream.expect("BRACE_OPEN");

  // Parse variant body by brace counting
  let depth = 1;
  let body = "";
  while (depth > 0 && !this.stream.isEof()) {
    const t = this.stream.peek();
    if (t.kind === "BRACE_OPEN") depth++;
    else if (t.kind === "BRACE_CLOSE") depth--;
    if (depth === 0) {
      this.stream.consume();
      break;
    }
    body += this.stream.consume().text;
  }

  this.ctx.elements.push(this.codeblockEl(
    `design.${this.ctx.currentComponent}.variant.${nameTok.text}`,
    "design-variant",
    { name: nameTok.text, body, designName: this.ctx.currentComponent },
  ));
}
```

- [ ] **Step 4: Update ParseContext to track design data**

In `parser-types.ts`, add to `ParseContext`:

```typescript
export interface ParseContext {
  // ...existing fields...
  designTokens: Array<{ name: string; tokenType: string; value: string }>;
  designClasses: Array<{ name: string; body: string }>;
  designVariants: Array<{ name: string; body: string }>;
}
```

Initialize these arrays in `createContext()`:
```typescript
designTokens: [],
designClasses: [],
designVariants: [],
```

Update `parseDesignToken/Class/Variant` to push to these arrays instead of (or in addition to) `ctx.elements`.

- [ ] **Step 5: Extend `parser-finalize.ts` to build `DesignDefinition`**

In `finalizeParse()`, add after the existing style parsing:

```typescript
// If this is a design file, build DesignDefinition from parsed data
if (internals.ctx.designTokens.length > 0 || internals.ctx.designClasses.length > 0) {
  const designDef = buildDesignDefinition(internals.ctx);
  internals.ctx.designDefinition = designDef;
}
```

Add `buildDesignDefinition` function:
```typescript
function buildDesignDefinition(ctx: ParseContext): import("../../core/types/design").DesignDefinition {
  const tokens = ctx.designTokens.map((t, i) => ({
    id: `dt_${i}`,
    name: t.name,
    category: inferCategory(t.tokenType),
    tokenType: t.tokenType as "color" | "size" | "font" | "shadow",
    value: t.value,
  }));

  const classes = ctx.designClasses.map((c, i) => ({
    id: `dc_${i}`,
    name: c.name,
    properties: [],    // Parsed from body in follow-up task
    pseudoStates: [],
  }));

  const variants = ctx.designVariants.map((v, i) => ({
    id: `dv_${i}`,
    name: v.name,
    dimensions: [],
    defaults: {},
    baseProperties: [],
    branches: [],
  }));

  return {
    name: ctx.currentComponent || "Design",
    tokens,
    classes,
    variants,
  };
}

function inferCategory(tokenType: string): string {
  switch (tokenType) {
    case "color": return "colors";
    case "size": return "spacing";
    case "font": return "typography";
    case "shadow": return "shadows";
    default: return "other";
  }
}
```

- [ ] **Step 6: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/parser-types.ts poiesis/src/services/codegen/parser-finalize.ts
git commit -m "feat: add design block parsing to Poiesis parser (tokens, classes, variants)"
```

---

### Task H2: Design Save Path — `useSaveProject` Extension

**Files:**
- Modify: `poiesis/src/hooks/useSaveProject.ts`

- [ ] **Step 1: Add design tab save branch**

In `handleSave()`, add a branch for `design` tab type BEFORE the component pipeline:

```typescript
const handleSave = useCallback(async () => {
  setSaving(true);
  try {
    const store = usePoiesisStore.getState();
    const activeTabId = store.designPage.activeTabId;
    const activeTab = activeTabId
      ? store.designPage.openTabs.find((t) => t.id === activeTabId)
      : null;

    // ── DESIGN TAB: emit design source, write to disk ──
    if (activeTab?.tabType === "design") {
      const { emitDesignSource } = await import("../services/codegen/design-emitter");
      const source = emitDesignSource(store.activeDesign);
      const filePath = activeTab.filePath || "src/design.omni";
      await api.project.writeFile(filePath, source);

      // Still save project.json for editor metadata
      const result = await api.annotation.saveProject();
      if (result.ok) {
        markClean();
      }
      return;
    }

    // ── COMPONENT TAB: existing pipeline ──
    // ... existing code ...
  } catch (e) {
    log.error("failed:", e);
  } finally {
    setSaving(false);
  }
}, [markClean]);
```

- [ ] **Step 2: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/hooks/useSaveProject.ts
git commit -m "feat: add design tab save path to useSaveProject"
```

---

### Task H3: Design Load Path — Parse design.omni into `activeDesign`

**Files:**
- Modify: `poiesis/src/services/parser/source-parser.ts`

- [ ] **Step 1: Add design file detection and population**

In `parseAndPopulate()`, after parsing and setting elements/wires, add logic to detect and populate design data:

```typescript
// After the existing parse + setState code:

// If the parsed source contains design data, populate activeDesign
if (parseResult.designElements) {
  const designElements = parseResult.designElements.filter(
    (el) => el.nodeType === "codeblock" &&
      (el.tag === "design-token" || el.tag === "design-class" || el.tag === "design-variant")
  );

  if (designElements.length > 0) {
    const design = buildDesignFromParseResult(
      parseResult,
      activeFilePath || "src/design.omni"
    );
    usePoiesisStore.getState().loadDesign(design);
  }
}
```

Add helper function:
```typescript
function buildDesignFromParseResult(
  result: ParseResult,
  filePath: string,
): import("../../core/types/design").DesignDefinition {
  const tokens: import("../../core/types/design").DesignToken[] = [];
  const classes: import("../../core/types/design").DesignClass[] = [];
  const variants: import("../../core/types/design").VariantDefinition[] = [];

  for (const el of result.designElements ?? []) {
    if (el.nodeType !== "codeblock") continue;

    const props = el.props as Record<string, unknown>;

    if (el.tag === "design-token") {
      tokens.push({
        id: el.id,
        name: props.name as string,
        category: inferCategoryFromType(props.tokenType as string),
        tokenType: (props.tokenType as "color" | "size" | "font" | "shadow") || "color",
        value: props.value as string,
      });
    } else if (el.tag === "design-class") {
      classes.push({
        id: el.id,
        name: props.name as string,
        properties: [],   // Parsed in a follow-up: implement class body parser
        pseudoStates: [],
      });
    } else if (el.tag === "design-variant") {
      variants.push({
        id: el.id,
        name: props.name as string,
        dimensions: [],
        defaults: {},
        baseProperties: [],
        branches: [],
      });
    }
  }

  // Derive design name from file path: "src/design.omni" → "Design"
  const fileName = filePath.split("/").pop()?.replace(".omni", "") ?? "Design";
  const designName = fileName.charAt(0).toUpperCase() + fileName.slice(1);

  return { name: designName, tokens, classes, variants };
}

function inferCategoryFromType(tokenType: string): string {
  switch (tokenType) {
    case "color": return "colors";
    case "size": return "spacing";
    case "font": return "typography";
    case "shadow": return "shadows";
    default: return "other";
  }
}
```

- [ ] **Step 2: Also update `loadManager.loadProject()`**

In `load-manager.ts`, after loading project.json, add design file scanning:

```typescript
// After store.loadFromProjectJson(projectJson):

// Scan for design.omni and parse into activeDesign
try {
  // The project service should list files; for now check the well-known path
  const designPath = `${projectDir}/src/design.omni`;
  const designSource = await api.project.readFile(designPath);
  if (designSource) {
    const parseResult = parseOmniSource(designSource, designPath);
    if (parseResult.designElements) {
      const design = buildDesignFromParseResult(parseResult, designPath);
      store.loadDesign(design);
    }
  }
} catch {
  // No design.omni — start with empty design
  log.info("No design.omni found, starting with empty design system");
}
```

- [ ] **Step 3: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/parser/source-parser.ts poiesis/src/services/persistence/load-manager.ts
git commit -m "feat: add design.omni load path — parse into activeDesign store"
```

---

### Task H4: Wire StyleRegistry to Active Design Data

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline.ts`

- [ ] **Step 1: Replace `EMPTY_REGISTRY` with live design data**

In `runPhaseC()`, replace line 212:

```typescript
// BEFORE:
const registry = EMPTY_REGISTRY;

// AFTER:
import { usePoiesisStore } from "../../store";

const state = usePoiesisStore.getState();
const designClasses = state.activeDesign.classes;
const registry = {
  variables: state.activeDesign.tokens.map((t) => ({
    id: t.id,
    name: `--${t.name}`,
    segments: [{ type: "text" as const, value: t.value }],
  })),
  classes: designClasses.map((c) => ({
    id: c.id,
    name: c.name,
    properties: c.properties,
  })),
};
```

- [ ] **Step 2: Verify component styles reference design tokens/classes in .omni output**

Test by creating a component element with `classRefs` matching a design class name — the pipeline should include those classes' CSS rules in the emitted style block.

- [ ] **Step 3: Verify TypeScript compilation**

Run: `cd poiesis && npx tsc --noEmit 2>&1`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/pipeline.ts
git commit -m "feat: wire StyleRegistry to activeDesign data (replace EMPTY_REGISTRY)"
```

---

## Plan Summary

| Phase | Tasks | Deliverable |
|-------|-------|-------------|
| A | A1–A5 | omnic parses `design AppDesign { }` without errors |
| B | B1–B3 | TypeScript types, Zustand store slice, `"design"` TabType |
| C | C1–C3 | Token editor with HUD color picker, drag-scrub, palette view |
| D | D1 | Class editor with grouped properties, pseudo-states, token binding |
| E | E1 | Variant editor with dimensions, branches, base inheritance |
| F | F1–F2 | ContextInspector class/variant binding, design→.omni emitter |
| G | G1–G3 | omnic IR lowering, token resolution, CSS codegen |
| **H** | **H0–H4** | **Design annotations + Poiesis parser + save/load pipeline + StyleRegistry wiring** |

**Independent test points:**
- After Phase A: `omnic` can compile `design.omni` files (no codegen, just no errors)
- After Phase B+C: Poiesis can open a `design.omni` tab and edit tokens visually
- After Phase D+E: Full class/variant editing works in Poiesis
- After Phase F+H: Full save/load round-trip — create tokens in Poiesis → save → .omni has `design` block → reload → tokens restored
- After Phase G: omnic resolves `token()` references and generates CSS from design blocks
- After Phase H4: component style blocks include design class CSS rules

**Key principle verified:** `.omni` is the source of truth. All token/class/variant data lives in `design.omni`. `project.json` stores only editor metadata (positions, viewport, panel layout) — never design semantic data.
