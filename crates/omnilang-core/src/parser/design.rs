//! Parser for `design { }` block bodies.
//!
//! This module parses token, class, and variant definitions inside
//! design blocks.  Currently only token definitions are implemented
//! (Task A2); class and variant parsing is deferred to Tasks A3/A4.

use crate::lexer::{Span, Token, TokenKind};
use crate::parser::ast::design::*;
use crate::parser::ast::Expr;
use crate::parser::Parser;

/// Structured error for design block parsing with position tracking.
#[derive(Debug)]
pub struct DesignParseError {
    pub message: String,
    /// Byte offset from the start of the design body string.
    pub offset: usize,
}

impl std::fmt::Display for DesignParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (offset {})", self.message, self.offset)
    }
}

impl std::error::Error for DesignParseError {}

type ParseResult<T> = Result<T, DesignParseError>;

fn err(msg: &str) -> DesignParseError {
    DesignParseError {
        message: msg.to_string(),
        offset: 0,
    }
}

fn err_at(input: &str, remaining: &str, msg: &str) -> DesignParseError {
    DesignParseError {
        message: msg.to_string(),
        offset: input.len() - remaining.len(),
    }
}

/// Parse the body content inside `design Name { ... }`
pub fn parse_design_body(input: &str, design_tokens: &[Token]) -> ParseResult<DesignDef> {
    let mut tokens = Vec::new();
    let mut classes = Vec::new();
    let mut variants = Vec::new();
    let mut themes: Vec<String> = Vec::new();
    let mut default_theme: Option<String> = None;
    // Only trim leading whitespace so that remaining is a suffix of input.
    // This keeps the offset calculation in parse_token_value correct:
    //   offset = design_body_full.len() - raw_input.len()
    // which relies on raw_input being a suffix of design_body_full.
    let mut remaining = input.trim_start();

    while !remaining.is_empty() {
        remaining = skip_ws_and_comments(remaining);

        if remaining.starts_with("@themes") {
            // (F3) a second `@themes { ... }` in the same design block was a
            // silent last-win — hard error (one theme set per design).
            if !themes.is_empty() {
                return Err(err_at(input, remaining, "@themes 重复声明"));
            }
            let (t, rest) = parse_themes_directive(remaining)?;
            themes = t;
            remaining = rest;
        } else if remaining.starts_with("@default-theme ") {
            let (dt, rest) = parse_default_theme_directive(remaining)?;
            default_theme = Some(dt);
            remaining = rest;
        } else if remaining.starts_with("token ") {
            let (token, rest) = parse_token(remaining, input, design_tokens)?;
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
        // Overridden by parse_design_def with the `design` keyword span.
        span: Span {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        },
    })
}

fn parse_token<'a>(
    input: &'a str,
    design_body_full: &'a str,
    design_tokens: &[Token],
) -> ParseResult<(TokenDef, &'a str)> {
    let rest = input
        .strip_prefix("token ")
        .ok_or_else(|| err("expected 'token'"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix(":")
        .ok_or_else(|| err("expected ':'"))?
        .trim_start();
    let (token_type, rest) = parse_token_type(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix("=")
        .ok_or_else(|| err("expected '='"))?
        .trim_start();
    let (value, rest) = parse_token_value(rest, design_body_full, design_tokens)?;

    // Parse optional per-theme value overrides: `dark "#a5b4fc" light "#ffffff"`
    let mut theme_values: Vec<(String, TokenValue)> = Vec::new();
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
        match parse_token_value(after_ws, design_body_full, design_tokens) {
            Ok((theme_val, r)) => {
                theme_values.push((maybe_theme, theme_val));
                remaining = r.trim_start();
            }
            Err(_) => break,
        }
    }

    Ok((
        TokenDef {
            name,
            token_type,
            value,
            theme_values,
        },
        remaining,
    ))
}

/// Parse `@themes { dark, light }` — comma-separated theme name list.
///
/// v2 BNF (设计文档 域 3): `themes_decl ::= '@themes' '{' ident (',' ident)* '}'`.
/// The v1 `@themes light, dark;` (semicolon-terminated) form is replaced and
/// hard-errors with a migration hint. An empty `@themes { }` hard-errors
/// here (「@themes 声明不能为空」) because the IR cannot distinguish an
/// explicit empty declaration from no declaration.
fn parse_themes_directive(input: &str) -> ParseResult<(Vec<String>, &str)> {
    let rest = input
        .strip_prefix("@themes")
        .ok_or_else(|| err("expected '@themes'"))?;
    let rest = rest
        .trim_start()
        .strip_prefix('{')
        .ok_or_else(|| err("expected '{' after '@themes'（v2 语法：@themes { light, dark }）"))?
        .trim_start();
    let mut themes = Vec::new();
    let mut remaining = rest;

    // (f) empty `@themes { }` — hard error. The IR cannot distinguish an
    // explicit empty declaration from no declaration (`themes: []` in both
    // cases), so the check lives here where the source shape is visible.
    if remaining.starts_with('}') {
        return Err(err_at(input, remaining, "@themes 声明不能为空"));
    }

    loop {
        if let Some(rest) = remaining.strip_prefix('}') {
            remaining = rest;
            break;
        }
        let (name, after) = parse_ident(remaining)?;
        themes.push(name);
        let after = after.trim_start();
        if let Some(rest) = after.strip_prefix(',') {
            remaining = rest.trim_start();
            if remaining.starts_with('}') {
                // Trailing comma — not part of `ident (',' ident)*`.
                return Err(err_at(input, remaining, "trailing ',' in @themes"));
            }
            continue;
        }
        if let Some(rest) = after.strip_prefix('}') {
            remaining = rest;
            break;
        }
        return Err(err_at(
            input,
            remaining,
            "expected ',' or '}' in @themes declaration",
        ));
    }

    Ok((themes, remaining))
}

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

fn parse_token_type(input: &str) -> ParseResult<(TokenType, &str)> {
    let (ident, rest) = parse_ident(input)?;
    match ident.as_str() {
        "color" => Ok((TokenType::Color, rest)),
        "size" => Ok((TokenType::Size, rest)),
        "radius" => Ok((TokenType::Radius, rest)),
        "fontFamily" => Ok((TokenType::FontFamily, rest)),
        "fontSize" => Ok((TokenType::FontSize, rest)),
        "fontWeight" => Ok((TokenType::FontWeight, rest)),
        "lineHeight" => Ok((TokenType::LineHeight, rest)),
        "letterSpacing" => Ok((TokenType::LetterSpacing, rest)),
        "shadow" => Ok((TokenType::Shadow, rest)),
        "duration" => Ok((TokenType::Duration, rest)),
        "easing" => Ok((TokenType::Easing, rest)),
        "opacity" => Ok((TokenType::Opacity, rest)),
        "zIndex" => Ok((TokenType::ZIndex, rest)),
        _ => Err(err("expected token type: color|size|radius|fontFamily|fontSize|fontWeight|lineHeight|letterSpacing|shadow|duration|easing|opacity|zIndex")),
    }
}

fn parse_ident(input: &str) -> ParseResult<(String, &str)> {
    let input = input.trim_start();
    let end = input
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .unwrap_or(input.len());
    if end == 0 {
        return Err(err("expected identifier"));
    }
    Ok((input[..end].to_string(), &input[end..]))
}

/// Parse a token value: either a quoted string (literal), hex color,
/// CSS keyword (literal), simple variable ref (Variable),
/// or runtime expression (Expression).
fn parse_token_value<'a>(
    raw_input: &'a str,
    design_body_full: &'a str,
    design_tokens: &[Token],
) -> ParseResult<(TokenValue, &'a str)> {
    let input = raw_input.trim_start();
    if let Some(stripped) = input.strip_prefix('"') {
        let end = stripped
            .find('"')
            .map(|i| i + 2)
            .ok_or_else(|| err("expected closing '\"'"))?;
        Ok((
            TokenValue::Literal(input[1..end - 1].to_string()),
            &input[end..],
        ))
    } else if input.starts_with('#') {
        // Hex color: `#6366f1` -- always literal
        let end = input[1..]
            .find(|c: char| !c.is_ascii_hexdigit())
            .map(|i| i + 1)
            .unwrap_or(input.len());
        Ok((TokenValue::Literal(input[..end].to_string()), &input[end..]))
    } else {
        let (ident, rest) = parse_ident(input)?;

        // ── Phase 2: Check if this is an expression (dotted path or function call) ──
        // Must check BEFORE the CSS-value heuristic so that identifiers like
        // `store.getColor(primary)` are not miscategorized as literals.
        let after_ident = rest.trim_start();
        let is_expression = after_ident.starts_with('.')
            || after_ident.starts_with('(')
            || after_ident.starts_with('[');

        if is_expression && !design_tokens.is_empty() {
            let offset = design_body_full.len().saturating_sub(input.len());
            if let Ok((expr, remaining)) =
                parse_design_value_expression(design_body_full, design_tokens, offset)
            {
                return Ok((TokenValue::Expression(expr), remaining));
            }
            // Fallback: treat as simple variable reference
            return Ok((TokenValue::Variable(ident), rest));
        }

        // Heuristic: if it looks like a CSS value, treat as literal
        if is_css_value(&ident) {
            Ok((TokenValue::Literal(ident), rest))
        } else {
            Ok((TokenValue::Variable(ident), rest))
        }
    }
}

/// Parse a runtime expression within a design token value.
///
/// `design_body_full` is the full design body source; token spans are offsets into it.
/// `offset` is the byte offset of the value start within the design body.
///
/// Returns the parsed expression and the remaining string after the expression.
fn parse_design_value_expression<'a>(
    design_body_full: &'a str,
    design_tokens: &[Token],
    offset: usize,
) -> ParseResult<(Expr, &'a str)> {
    // Find first identifier token at or after the offset
    let start_idx = design_tokens
        .iter()
        .position(|t| matches!(t.kind, TokenKind::Ident(_)) && t.span.start >= offset)
        .ok_or_else(|| err("could not locate expression start in design tokens"))?;

    // Scan forward to find expression end: stop at `;`, `}`, or bare identifier.
    // Track paren/bracket depth so identifiers inside `(...)` or `[...]` are not
    // mistaken for theme name boundaries.
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
                    // Bare identifier (not preceded by `.`) = theme name boundary
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

    if end_idx <= start_idx {
        return Err(err("empty expression in token value"));
    }

    // Extract tokens for the expression and create a sub-Parser
    let expr_tokens: Vec<Token> = design_tokens[start_idx..end_idx].to_vec();

    // Capture offset before moving expr_tokens into Parser.
    // Uses the full design body as source (token spans are offsets into it).
    let error_offset = expr_tokens.first().map(|t| t.span.start).unwrap_or(offset);
    let mut expr_parser = Parser::new(expr_tokens, design_body_full);
    let expr = expr_parser.parse_expr().map_err(|e| DesignParseError {
        message: format!("failed to parse expression in token value: {}", e.message),
        offset: error_offset,
    })?;

    // Remaining string starts after the last expression token
    let last_token = &design_tokens[end_idx - 1];
    let remaining = &design_body_full[last_token.span.end..];

    Ok((expr, remaining))
}

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
    if s.is_empty() {
        return true;
    }
    let first = s.chars().next().unwrap();
    // Numbers -- CSS values: "16px", "1.5", "0.5s"
    if first.is_ascii_digit() {
        return true;
    }
    // Hex colors -- CSS values: "#6366f1"
    if first == '#' {
        return true;
    }
    // Known CSS keywords
    if CSS_KEYWORDS.contains(&s) {
        return true;
    }
    // All lowercase with hyphens (CSS properties/values):
    //   "ease-in-out", "inline-flex", "nowrap", "white"
    // But NOT PascalCase or camelCase identifiers (variables):
    //   "BRAND", "spacingUnit"
    s.chars()
        .all(|c| c.is_lowercase() || c == '-' || c == '.' || c == '_')
}

// Common CSS value keywords (case-sensitive lowercase)
const CSS_KEYWORDS: &[&str] = &[
    "none",
    "auto",
    "inherit",
    "initial",
    "unset",
    "revert",
    "white",
    "black",
    "red",
    "blue",
    "green",
    "yellow",
    "orange",
    "transparent",
    "currentColor",
    "solid",
    "dashed",
    "dotted",
    "double",
    "left",
    "right",
    "center",
    "top",
    "bottom",
    "bold",
    "normal",
    "italic",
    "block",
    "inline",
    "inline-block",
    "flex",
    "grid",
    "visible",
    "hidden",
    "scroll",
    "clip",
    "nowrap",
    "wrap",
    "pre",
    "pointer",
    "default",
    "text",
    "ease",
    "linear",
    "ease-in",
    "ease-out",
    "ease-in-out",
    "cover",
    "contain",
    "fill",
    "uppercase",
    "lowercase",
    "capitalize",
    "underline",
    "overline",
    "line-through",
    "repeat",
    "no-repeat",
    "repeat-x",
    "repeat-y",
    "border-box",
    "content-box",
    "padding-box",
    "start",
    "end",
    "stretch",
    "baseline",
];

fn skip_ws_and_comments(input: &str) -> &str {
    let mut s = input.trim_start();
    loop {
        if s.starts_with("//") {
            let line_end = s.find('\n').unwrap_or(s.len());
            s = s[line_end..].trim_start();
        } else if s.starts_with("/*") {
            let end = s[2..].find("*/").map(|i| i + 4).unwrap_or(s.len());
            s = s[end..].trim_start();
        } else {
            break;
        }
    }
    s
}

fn parse_class(input: &str) -> ParseResult<(DesignClassDef, &str)> {
    let rest = input
        .strip_prefix("class ")
        .ok_or_else(|| err("expected 'class'"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix("{")
        .ok_or_else(|| err("expected '{'"))?
        .trim_start();

    let mut properties = Vec::new();
    let mut pseudo_states = Vec::new();
    let mut remaining = rest;

    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        if remaining.starts_with("&::") {
            let (pseudo, rest) = parse_pseudo_element(remaining)?;
            pseudo_states.push(pseudo);
            remaining = rest;
        } else if remaining.starts_with("&:") {
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

    Ok((
        DesignClassDef {
            name,
            properties,
            pseudo_states,
        },
        remaining,
    ))
}

fn parse_pseudo_state(input: &str) -> ParseResult<(PseudoState, &str)> {
    let rest = input
        .strip_prefix("&:")
        .ok_or_else(|| err("expected '&:'"))?;
    let (pseudo, rest) = parse_ident(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix("{")
        .ok_or_else(|| err("expected '{'"))?
        .trim_start();

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

/// Parse `&::before { ... }` — pseudo-element with double-colon prefix.
///
/// Unlike pseudo-classes (which store `"hover"`), pseudo-elements store the
/// full `"::before"` string so the codegen can distinguish `:hover` from `::before`.
fn parse_pseudo_element(input: &str) -> ParseResult<(PseudoState, &str)> {
    let rest = input
        .strip_prefix("&::")
        .ok_or_else(|| err("expected '&::'"))?;
    let (name, rest) = parse_ident(rest)?;
    // prefix with :: so codegen emits `.Card::before` not `.Card:before`
    let pseudo = format!("::{}", name);
    let rest = rest
        .trim_start()
        .strip_prefix("{")
        .ok_or_else(|| err("expected '{'"))?
        .trim_start();

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

fn parse_css_property_name(input: &str) -> ParseResult<(String, &str)> {
    let input = input.trim_start();
    let end = input
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .unwrap_or(input.len());
    if end == 0 {
        return Err(err("expected property name"));
    }
    Ok((input[..end].to_string(), &input[end..]))
}

fn parse_design_property(input: &str) -> ParseResult<(DesignProperty, &str)> {
    let (name, rest) = parse_css_property_name(input)?;
    let rest = rest
        .trim_start()
        .strip_prefix(":")
        .ok_or_else(|| err("expected ':'"))?
        .trim_start();
    let (value, rest) = parse_design_value(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix(";")
        .ok_or_else(|| err("expected ';'"))?
        .trim_start();
    Ok((DesignProperty { name, value }, rest))
}

fn parse_design_value(input: &str) -> ParseResult<(DesignValue, &str)> {
    let input = input.trim_start();

    // Fast path: bare token(name)
    if let Some(rest) = input.strip_prefix("token(") {
        let end = rest.find(')').ok_or_else(|| err("expected ')'"))?;
        let name = rest[..end].trim().to_string();
        return Ok((DesignValue::TokenRef(name), &rest[end + 1..]));
    }

    // Scan for mixed content: quoted strings and token() refs
    let mut parts: Vec<DesignValuePart> = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() && !remaining.starts_with(';') && !remaining.starts_with('}') {
        if remaining.starts_with('"') {
            // quoted string
            let end = remaining[1..]
                .find('"')
                .map(|i| i + 2)
                .ok_or_else(|| err("expected closing '\"'"))?;
            parts.push(DesignValuePart::Text(remaining[1..end - 1].to_string()));
            remaining = remaining[end..].trim_start();
        } else if remaining.starts_with("token(") {
            // inline token reference
            let end = remaining.find(')').ok_or_else(|| err("expected ')'"))?;
            let name = remaining["token(".len()..end].trim().to_string();
            parts.push(DesignValuePart::Token(name));
            remaining = remaining[end + 1..].trim_start();
        } else {
            // Unquoted literal — read until the next delimiter (;, }, ", or token()
            let delim = remaining.find([';', '}', '"']);
            let token_paren = remaining.find("token(");
            let end = match (delim, token_paren) {
                (Some(a), Some(b)) => a.min(b),
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => remaining.len(),
            };
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
            DesignValuePart::Text(s) => {
                return Ok((DesignValue::Literal(s.clone()), remaining));
            }
            DesignValuePart::Token(n) => {
                return Ok((DesignValue::TokenRef(n.clone()), remaining));
            }
        }
    }
    Ok((DesignValue::Mixed(parts), remaining))
}

fn parse_variant(input: &str) -> ParseResult<(VariantDef, &str)> {
    let rest = input
        .strip_prefix("variant ")
        .ok_or_else(|| err("expected 'variant'"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix("{")
        .ok_or_else(|| err("expected '{'"))?
        .trim_start();

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
            let body = remaining
                .strip_prefix("base ")
                .ok_or_else(|| err("expected 'base'"))?;
            let (props, rest) = parse_variant_block_body(body)?;
            base = props;
            remaining = rest;
        } else if remaining.starts_with('}') {
            break;
        } else {
            // Try parsing as dimension.value { ... } branch
            let dot_pos = remaining.find('.').unwrap_or(0);
            let space_pos = remaining.find([' ', '{']).unwrap_or(remaining.len());
            if dot_pos > 0 && dot_pos < space_pos {
                let (branch, rest) = parse_variant_branch(remaining)?;
                branches.push(branch);
                remaining = rest;
            } else {
                return Err(err_at(
                    input,
                    remaining,
                    &format!(
                        "unexpected content in variant: '{}...'",
                        &remaining[..remaining.find('\n').unwrap_or(remaining.len()).min(40)]
                    ),
                ));
            }
        }
    }

    Ok((
        VariantDef {
            name,
            dimensions,
            defaults,
            base,
            branches,
        },
        remaining,
    ))
}

fn parse_variant_dimension(input: &str) -> ParseResult<(VariantDimension, &str)> {
    let rest = input
        .strip_prefix("@dim ")
        .ok_or_else(|| err("expected '@dim'"))?;
    let (name, rest) = parse_ident(rest)?;
    let rest = rest
        .trim_start()
        .strip_prefix(":")
        .ok_or_else(|| err("expected ':'"))?
        .trim_start();

    let mut values = Vec::new();
    let mut remaining = rest;
    loop {
        remaining = remaining.trim_start();
        if remaining.starts_with(";") {
            remaining = &remaining[1..];
            break;
        }
        if remaining.starts_with("|") {
            remaining = remaining[1..].trim_start();
            continue;
        }
        let (val, rest) = parse_ident(remaining)?;
        values.push(val);
        remaining = rest;
    }

    Ok((VariantDimension { name, values }, remaining))
}

fn parse_variant_defaults(input: &str) -> ParseResult<(Vec<(String, String)>, &str)> {
    let rest = input
        .strip_prefix("@default ")
        .ok_or_else(|| err("expected '@default'"))?;
    let rest = rest
        .trim_start()
        .strip_prefix("{")
        .ok_or_else(|| err("expected '{'"))?
        .trim_start();

    let mut defaults = Vec::new();
    let mut remaining = rest;
    loop {
        remaining = skip_ws_and_comments(remaining);
        if remaining.starts_with("}") {
            remaining = &remaining[1..];
            break;
        }
        let (key, rest) = parse_ident(remaining)?;
        let rest = rest
            .trim_start()
            .strip_prefix(":")
            .ok_or_else(|| err("expected ':'"))?
            .trim_start();
        let (val, rest) = parse_ident(rest)?;
        let rest = rest.trim_start().strip_prefix(";").unwrap_or(rest);
        defaults.push((key, val));
        remaining = rest;
    }

    Ok((defaults, remaining))
}

fn parse_variant_block_body(input: &str) -> ParseResult<(Vec<DesignProperty>, &str)> {
    let rest = input
        .trim_start()
        .strip_prefix("{")
        .ok_or_else(|| err("expected '{'"))?
        .trim_start();
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
    let dot_pos = input
        .find('.')
        .ok_or_else(|| err("expected 'dimension.value'"))?;
    let dimension = input[..dot_pos].trim().to_string();
    let rest = &input[dot_pos + 1..];
    let (value, rest) = parse_ident(rest)?;
    let (properties, rest) = parse_variant_block_body(rest)?;

    Ok((
        VariantBranch {
            dimension,
            value,
            properties,
        },
        rest,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_token() {
        let src = r##"token primary: color = "#6366f1";"##;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.tokens.len(), 1);
        assert_eq!(result.tokens[0].name, "primary");
        assert_eq!(result.tokens[0].token_type, TokenType::Color);
        assert_eq!(
            result.tokens[0].value,
            TokenValue::Literal("#6366f1".to_string())
        );
    }

    #[test]
    fn parse_size_token() {
        let src = r#"token spacingMd: size = "16px";"#;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.tokens[0].token_type, TokenType::Size);
        assert_eq!(
            result.tokens[0].value,
            TokenValue::Literal("16px".to_string())
        );
    }

    #[test]
    fn parse_multiple_tokens() {
        let src = r##"
            token primary: color = "#6366f1";
            token surface: color = "#ffffff";
            token spacingMd: size = "16px";
        "##;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.tokens.len(), 3);
    }

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
        let result = parse_design_body(src, &[]).unwrap();
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
        let result = parse_design_body(src, &[]).unwrap();
        let prop = &result.classes[0].properties[0];
        assert!(matches!(prop.value, DesignValue::TokenRef(ref name) if name == "surface"));
    }

    #[test]
    fn parse_literal_value() {
        let src = r#"class Test { color: white; font-size: "14px"; }"#;
        let result = parse_design_body(src, &[]).unwrap();
        let cls = &result.classes[0];
        assert_eq!(cls.properties.len(), 2);
    }

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
        let result = parse_design_body(src, &[]).unwrap();
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
        assert_eq!(
            v.defaults,
            vec![
                ("color".into(), "primary".into()),
                ("size".into(), "md".into()),
            ]
        );
    }

    #[test]
    fn parse_themes_brace_form() {
        // v2 BNF (设计文档 域 3): themes_decl ::= '@themes' '{' ident (',' ident)* '}'
        let src = r##"
            @themes { light, dark }
            token primary: color = "#6366f1";
        "##;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.themes, vec!["light", "dark"]);
        assert_eq!(result.tokens.len(), 1);
    }

    #[test]
    fn parse_themes_single_theme() {
        let src = r#"@themes { dark }"#;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.themes, vec!["dark"]);
    }

    #[test]
    fn parse_themes_rejects_old_semicolon_form() {
        // v1 `@themes light, dark;` is replaced by the brace form; the old
        // form must hard-error (迁移提示 in the message).
        let src = r#"@themes light, dark;"#;
        let err = parse_design_body(src, &[]).unwrap_err();
        assert!(
            err.message.contains("@themes { light, dark }"),
            "should carry the v2 migration hint, got: {}",
            err.message
        );
    }

    #[test]
    fn parse_themes_empty_braces_rejected() {
        // (f) empty @themes — hard parse error (the IR cannot distinguish an
        // explicit empty declaration from no declaration).
        let src = r#"@themes { }"#;
        let err = parse_design_body(src, &[]).unwrap_err();
        assert!(
            err.message.contains("不能为空"),
            "empty @themes must error, got: {}",
            err.message
        );
    }

    #[test]
    fn parse_themes_trailing_comma_rejected() {
        let src = r#"@themes { dark, }"#;
        assert!(
            parse_design_body(src, &[]).is_err(),
            "trailing comma is not part of the BNF"
        );
    }

    #[test]
    fn parse_themes_missing_comma_rejected() {
        let src = r#"@themes { dark light }"#;
        assert!(
            parse_design_body(src, &[]).is_err(),
            "theme names must be comma-separated"
        );
    }

    #[test]
    fn parse_themes_duplicate_declaration_rejected() {
        // (F3) repeated `@themes { ... }` — last-win is a silent footgun.
        let src = r#"@themes { dark } @themes { light }"#;
        let err = parse_design_body(src, &[]).unwrap_err();
        assert!(
            err.message.contains("@themes 重复声明"),
            "duplicate @themes must error, got: {}",
            err.message
        );
    }

    #[test]
    fn parse_themes_missing_brace_rejected() {
        let src = r#"@themes dark"#;
        assert!(
            parse_design_body(src, &[]).is_err(),
            "@themes requires the {{ ... }} block"
        );
    }

    #[test]
    fn parse_default_theme_directive() {
        let src = r##"
            @themes { light, dark }
            @default-theme light;
            token primary: color = "#6366f1";
        "##;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.default_theme, Some("light".to_string()));
    }

    #[test]
    fn parse_token_with_theme_values() {
        let src = r##"token primary: color = "#6366f1" dark "#a5b4fc";"##;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.tokens.len(), 1);
        let token = &result.tokens[0];
        assert_eq!(token.name, "primary");
        assert_eq!(token.value, TokenValue::Literal("#6366f1".to_string()));
        assert_eq!(token.theme_values.len(), 1);
        assert_eq!(
            token.theme_values[0],
            (
                "dark".to_string(),
                TokenValue::Literal("#a5b4fc".to_string())
            )
        );
    }

    #[test]
    fn parse_token_with_multiple_theme_values() {
        let src = r##"token surface: color = "#ffffff" dark "#111827" highContrast "#000000";"##;
        let result = parse_design_body(src, &[]).unwrap();
        let token = &result.tokens[0];
        assert_eq!(token.value, TokenValue::Literal("#ffffff".to_string()));
        assert_eq!(token.theme_values.len(), 2);
        assert_eq!(
            token.theme_values[0],
            (
                "dark".to_string(),
                TokenValue::Literal("#111827".to_string())
            )
        );
        assert_eq!(
            token.theme_values[1],
            (
                "highContrast".to_string(),
                TokenValue::Literal("#000000".to_string())
            )
        );
    }

    #[test]
    fn parse_token_without_theme_values_still_works() {
        let src = r##"token primary: color = "#6366f1";"##;
        let result = parse_design_body(src, &[]).unwrap();
        let token = &result.tokens[0];
        assert_eq!(token.value, TokenValue::Literal("#6366f1".to_string()));
        assert!(token.theme_values.is_empty());
    }

    #[test]
    fn parse_token_with_variable_value() {
        let src = r#"token primary: color = BRAND;"#;
        let result = parse_design_body(src, &[]).unwrap();
        assert_eq!(result.tokens.len(), 1);
        assert_eq!(result.tokens[0].name, "primary");
        assert_eq!(
            result.tokens[0].value,
            TokenValue::Variable("BRAND".to_string())
        );
    }

    #[test]
    fn parse_token_heuristic_css_value_as_literal() {
        let src = r#"token easing: easing = ease-in-out;"#;
        let result = parse_design_body(src, &[]).unwrap();
        assert!(matches!(result.tokens[0].value, TokenValue::Literal(_)));
    }

    #[test]
    fn parse_token_hex_color_as_literal() {
        let src = r##"token primary: color = #6366f1;"##;
        let result = parse_design_body(src, &[]).unwrap();
        assert!(matches!(result.tokens[0].value, TokenValue::Literal(_)));
    }

    // ── Phase 2: Expression parsing in token values ──

    #[test]
    fn parse_token_value_dotted_expression() {
        let src = r#"token primary: color = ThemeStore.primaryColor;"#;
        let tokens = crate::lexer::tokenize(src);
        let result = parse_design_body(src, &tokens).unwrap();
        assert_eq!(result.tokens.len(), 1);
        assert!(
            matches!(result.tokens[0].value, TokenValue::Expression(_)),
            "expected Expression variant, got {:?}",
            result.tokens[0].value
        );
    }

    #[test]
    fn parse_token_value_static_variable_still_works() {
        let src = r#"token primary: color = BRAND;"#;
        let tokens = crate::lexer::tokenize(src);
        let result = parse_design_body(src, &tokens).unwrap();
        assert_eq!(
            result.tokens[0].value,
            TokenValue::Variable("BRAND".to_string())
        );
    }

    #[test]
    fn parse_token_value_expression_with_theme_value() {
        let src = r##"token accent: color = ThemeStore.primaryColor dark "#a5b4fc";"##;
        let tokens = crate::lexer::tokenize(src);
        let result = parse_design_body(src, &tokens).unwrap();
        let token = &result.tokens[0];
        assert!(
            matches!(token.value, TokenValue::Expression(_)),
            "expected Expression variant, got {:?}",
            token.value
        );
        assert_eq!(token.theme_values.len(), 1);
        assert_eq!(
            token.theme_values[0],
            (
                "dark".to_string(),
                TokenValue::Literal("#a5b4fc".to_string())
            )
        );
    }

    #[test]
    fn parse_token_value_function_call_expression() {
        let src = r#"token primary: color = getAccentColor();"#;
        let tokens = crate::lexer::tokenize(src);
        let result = parse_design_body(src, &tokens).unwrap();
        assert!(
            matches!(result.tokens[0].value, TokenValue::Expression(_)),
            "expected Expression variant for function call, got {:?}",
            result.tokens[0].value
        );
    }

    #[test]
    fn parse_token_value_function_call_with_arg() {
        let src = r#"token primary: color = getColor(primary);"#;
        let tokens = crate::lexer::tokenize(src);
        let result = parse_design_body(src, &tokens).unwrap();
        assert!(
            matches!(result.tokens[0].value, TokenValue::Expression(_)),
            "expected Expression variant for function call with arg, got {:?}",
            result.tokens[0].value
        );
    }

    #[test]
    fn parse_token_value_method_call_with_arg() {
        let src = r#"token primary: color = store.getColor(primary);"#;
        let tokens = crate::lexer::tokenize(src);
        let result = parse_design_body(src, &tokens).unwrap();
        assert!(
            matches!(result.tokens[0].value, TokenValue::Expression(_)),
            "expected Expression variant for method call with arg, got {:?}",
            result.tokens[0].value
        );
    }

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
}
