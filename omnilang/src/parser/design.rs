//! Parser for `design { }` block bodies.
//!
//! This module parses token, class, and variant definitions inside
//! design blocks.  Currently only token definitions are implemented
//! (Task A2); class and variant parsing is deferred to Tasks A3/A4.

use crate::parser::ast::design::*;

// Placeholder error types until we properly integrate:
type ParseResult<T> = Result<T, String>;

fn err(msg: &str) -> String {
    msg.to_string()
}

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
            let (cls, rest) = parse_class(remaining)?;
            classes.push(cls);
            remaining = rest;
        } else if remaining.starts_with("variant ") {
            let (var, rest) = parse_variant(remaining)?;
            variants.push(var);
            remaining = rest;
        } else if remaining.starts_with("}") {
            break;
        } else if remaining.is_empty() {
            break;
        } else {
            remaining = &remaining[1..]; // skip unknown char
        }
    }

    Ok(DesignDef {
        name: String::new(),
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
    let rest = rest
        .trim_start()
        .strip_prefix(";")
        .ok_or_else(|| err("expected ';'"))?
        .trim_start();

    Ok((
        TokenDef {
            name,
            token_type,
            value,
        },
        rest,
    ))
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
        Err(err("expected token type: color|size|font|shadow"))
    }
}

fn parse_ident(input: &str) -> ParseResult<(String, &str)> {
    let input = input.trim_start();
    let end = input
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(input.len());
    if end == 0 {
        return Err(err("expected identifier"));
    }
    Ok((input[..end].to_string(), &input[end..]))
}

fn parse_string_or_ident(input: &str) -> ParseResult<(String, &str)> {
    let input = input.trim_start();
    if input.starts_with('"') {
        let end = input[1..]
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
    while s.starts_with("//") {
        let line_end = s.find('\n').unwrap_or(s.len());
        s = s[line_end..].trim_start();
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
    if let Some(rest) = input.strip_prefix("token(") {
        let end = rest
            .find(')')
            .ok_or_else(|| err("expected ')'"))?;
        let name = rest[..end].trim().to_string();
        Ok((DesignValue::TokenRef(name), &rest[end + 1..]))
    } else if input.starts_with('"') {
        let end = input[1..]
            .find('"')
            .map(|i| i + 2)
            .ok_or_else(|| err("expected closing '\"'"))?;
        Ok((
            DesignValue::Literal(input[1..end - 1].to_string()),
            &input[end..],
        ))
    } else {
        // Unquoted literal until ; or }
        let end = input
            .find([';', '}'])
            .unwrap_or(input.len());
        Ok((
            DesignValue::Literal(input[..end].trim().to_string()),
            &input[end..],
        ))
    }
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
        } else {
            // Try parsing as dimension.value { ... } branch
            let dot_pos = remaining.find('.').unwrap_or(0);
            let space_pos = remaining
                .find([' ', '{'])
                .unwrap_or(remaining.len());
            if dot_pos > 0 && dot_pos < space_pos {
                let (branch, rest) = parse_variant_branch(remaining)?;
                branches.push(branch);
                remaining = rest;
            } else {
                break;
            }
        }
    }

    Ok((VariantDef { name, dimensions, defaults, base, branches }, remaining))
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

    Ok((VariantBranch { dimension, value, properties }, rest))
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
        assert_eq!(v.defaults, vec![
            ("color".into(), "primary".into()),
            ("size".into(), "md".into()),
        ]);
    }
}
