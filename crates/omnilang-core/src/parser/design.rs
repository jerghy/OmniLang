//! Parser for `design { }` block bodies.
//!
//! This module parses token, class, and variant definitions inside
//! design blocks.  Currently only token definitions are implemented
//! (Task A2); class and variant parsing is deferred to Tasks A3/A4.

use crate::parser::ast::design::*;

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
pub fn parse_design_body(input: &str) -> ParseResult<DesignDef> {
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
            let (token, rest) = parse_token(remaining)?;
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

fn parse_token(input: &str) -> ParseResult<(TokenDef, &str)> {
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

fn parse_string_or_ident(input: &str) -> ParseResult<(String, &str)> {
    let input = input.trim_start();
    if let Some(stripped) = input.strip_prefix('"') {
        let end = stripped
            .find('"')
            .map(|i| i + 2)
            .ok_or_else(|| err("expected closing '\"'"))?;
        Ok((input[1..end - 1].to_string(), &input[end..]))
    } else {
        parse_ident(input)
    }
}

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
        let src = r##"
            token primary: color = "#6366f1";
            token surface: color = "#ffffff";
            token spacingMd: size = "16px";
        "##;
        let result = parse_design_body(src).unwrap();
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

    #[test]
    fn parse_literal_value() {
        let src = r#"class Test { color: white; font-size: "14px"; }"#;
        let result = parse_design_body(src).unwrap();
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
        assert_eq!(
            v.defaults,
            vec![
                ("color".into(), "primary".into()),
                ("size".into(), "md".into()),
            ]
        );
    }

    #[test]
    fn parse_themes_directive() {
        let src = r##"
            @themes light, dark;
            token primary: color = "#6366f1";
        "##;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.themes, vec!["light", "dark"]);
        assert_eq!(result.tokens.len(), 1);
    }

    #[test]
    fn parse_default_theme_directive() {
        let src = r##"
            @themes light, dark;
            @default-theme light;
            token primary: color = "#6366f1";
        "##;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.default_theme, Some("light".to_string()));
    }

    #[test]
    fn parse_token_with_theme_values() {
        let src = r##"token primary: color = "#6366f1" dark "#a5b4fc";"##;
        let result = parse_design_body(src).unwrap();
        assert_eq!(result.tokens.len(), 1);
        let token = &result.tokens[0];
        assert_eq!(token.name, "primary");
        assert_eq!(token.value, "#6366f1");
        assert_eq!(token.theme_values.len(), 1);
        assert_eq!(
            token.theme_values[0],
            ("dark".to_string(), "#a5b4fc".to_string())
        );
    }

    #[test]
    fn parse_token_with_multiple_theme_values() {
        let src = r##"token surface: color = "#ffffff" dark "#111827" highContrast "#000000";"##;
        let result = parse_design_body(src).unwrap();
        let token = &result.tokens[0];
        assert_eq!(token.value, "#ffffff");
        assert_eq!(token.theme_values.len(), 2);
        assert_eq!(
            token.theme_values[0],
            ("dark".to_string(), "#111827".to_string())
        );
        assert_eq!(
            token.theme_values[1],
            ("highContrast".to_string(), "#000000".to_string())
        );
    }

    #[test]
    fn parse_token_without_theme_values_still_works() {
        let src = r##"token primary: color = "#6366f1";"##;
        let result = parse_design_body(src).unwrap();
        let token = &result.tokens[0];
        assert_eq!(token.value, "#6366f1");
        assert!(token.theme_values.is_empty());
    }
}
