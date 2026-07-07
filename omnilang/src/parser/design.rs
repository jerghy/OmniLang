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
    let classes = Vec::new();
    let variants = Vec::new();
    let mut remaining = input.trim();

    while !remaining.is_empty() {
        remaining = skip_ws_and_comments(remaining);

        if remaining.starts_with("token ") {
            let (token, rest) = parse_token(remaining)?;
            tokens.push(token);
            remaining = rest;
        } else if remaining.starts_with("class ") {
            break; // Task A3
        } else if remaining.starts_with("variant ") {
            break; // Task A4
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
}
