//! Template string parsing — handling `${expr}` interpolations.
//!
//! This module provides the free functions for parsing template string
//! content into a vector of `TemplatePart` segments (literals and expressions).

use crate::error::ParseError;
use crate::lexer::Span;
use crate::parser::ast::*;

/// Parse a template string containing `${expr}` interpolations into parts.
/// Splits the raw string into literal text and parsed expression segments.
pub(super) fn parse_template_parts(
    s: &str,
    base_span: Span,
) -> Result<Vec<TemplatePart>, ParseError> {
    let mut parts: Vec<TemplatePart> = Vec::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut literal_start = 0;

    while i < len {
        if bytes[i] == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
            // Push literal text before this interpolation
            if i > literal_start {
                parts.push(TemplatePart::Literal(s[literal_start..i].to_string()));
            }

            // Skip past ${
            i += 2;

            // Find matching closing brace, tracking nesting
            let mut depth = 1u32;
            let expr_start = i;
            while i < len && depth > 0 {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    i += 1;
                }
            }
            if depth != 0 {
                return Err(ParseError {
                    kind: None,
                    message: "Unterminated template expression".into(),
                    span: Span {
                        start: base_span.start + expr_start,
                        end: base_span.start + i,
                        line: base_span.line,
                        col: base_span.col + expr_start as u32,
                    },
                });
            }
            let expr_str = s[expr_start..i].trim();
            if expr_str.is_empty() {
                return Err(ParseError {
                    kind: None,
                    message: "Empty template expression".into(),
                    span: Span {
                        start: base_span.start + expr_start,
                        end: base_span.start + i,
                        line: base_span.line,
                        col: base_span.col + expr_start as u32,
                    },
                });
            }

            // Parse the expression string
            let expr = parse_simple_template_expr(expr_str).map_err(|e| ParseError {
                kind: None,
                message: format!("In template expression '{}': {}", expr_str, e),
                span: Span {
                    start: base_span.start + expr_start,
                    end: base_span.start + i,
                    line: base_span.line,
                    col: base_span.col + expr_start as u32,
                },
            })?;
            parts.push(TemplatePart::Expr(expr));

            // Move past the closing }
            i += 1;
            literal_start = i;
        } else {
            i += 1;
        }
    }

    // Push trailing literal text
    if literal_start < len {
        parts.push(TemplatePart::Literal(s[literal_start..].to_string()));
    }

    Ok(parts)
}

/// Parse a simple expression inside `${}` in template strings.
/// Handles identifiers, field access, method calls, and function calls.
fn parse_simple_template_expr(s: &str) -> Result<Expr, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty expression".to_string());
    }

    let span = Span {
        start: 0,
        end: 0,
        line: 0,
        col: 0,
    };

    // Helper: parse an identifier at the start of a string
    fn parse_ident(input: &str) -> Option<(String, usize)> {
        let mut i = 0;
        let bytes = input.as_bytes();
        if i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            Some((input[..i].to_string(), i))
        } else {
            None
        }
    }

    // Skip whitespace helper
    fn skip_ws(input: &str, pos: &mut usize) {
        while *pos < input.len() && input.as_bytes()[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
    }

    let mut pos = 0;
    skip_ws(s, &mut pos);

    // Parse the primary expression
    let (mut expr, mut pos) = {
        if let Some((name, end)) = parse_ident(&s[pos..]) {
            (Expr::VarRef(name, span), pos + end)
        } else {
            return Err(format!("unexpected token at '{}'", &s[pos..]));
        }
    };

    // Chain field access / method calls
    skip_ws(s, &mut pos);
    while pos < s.len() {
        match s.as_bytes()[pos] {
            b'.' => {
                pos += 1;
                skip_ws(s, &mut pos);
                if let Some((field, end)) = parse_ident(&s[pos..]) {
                    // Check if it's a method call
                    let mut new_pos = pos + end;
                    skip_ws(s, &mut new_pos);
                    if new_pos < s.len() && s.as_bytes()[new_pos] == b'(' {
                        // Method call
                        pos = new_pos;
                        let mut args = Vec::new();
                        pos += 1; // skip (
                        let arg_str = &s[pos..];
                        let mut depth = 0u32;
                        let mut arg_end = 0;
                        let mut found_close = false;
                        for (j, ch) in arg_str.char_indices() {
                            match ch {
                                '(' => depth += 1,
                                ')' if depth == 0 => {
                                    arg_end = j;
                                    found_close = true;
                                    break;
                                }
                                ')' => depth -= 1,
                                _ => {}
                            }
                        }
                        if !found_close {
                            return Err("unterminated method call".to_string());
                        }
                        let args_section = arg_str[..arg_end].trim();
                        if !args_section.is_empty() {
                            for arg in args_section.split(',') {
                                let arg = arg.trim();
                                if !arg.is_empty() {
                                    let parsed = parse_simple_template_expr(arg)?;
                                    args.push(parsed);
                                }
                            }
                        }
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span,
                        };
                        pos += arg_end + 1; // skip past )
                    } else {
                        // Simple field access
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span,
                        };
                        pos = new_pos;
                    }
                } else {
                    return Err(format!("expected identifier after '.' at '{}'", &s[pos..]));
                }
            }
            b'(' => {
                // Function call on the current expression
                pos += 1;
                let arg_str = &s[pos..];
                let mut args = Vec::new();
                let mut depth = 0u32;
                let mut arg_end = 0;
                let mut found_close = false;
                for (j, ch) in arg_str.char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' if depth == 0 => {
                            arg_end = j;
                            found_close = true;
                            break;
                        }
                        ')' => depth -= 1,
                        _ => {}
                    }
                }
                if !found_close {
                    return Err("unterminated function call".to_string());
                }
                let args_section = arg_str[..arg_end].trim();
                if !args_section.is_empty() {
                    for arg in args_section.split(',') {
                        let arg = arg.trim();
                        if !arg.is_empty() {
                            args.push(parse_simple_template_expr(arg)?);
                        }
                    }
                }
                expr = Expr::FuncCall {
                    func: Box::new(expr),
                    args,
                    span,
                };
                pos += arg_end + 1;
            }
            _ => break,
        }
        skip_ws(s, &mut pos);
    }

    Ok(expr)
}
