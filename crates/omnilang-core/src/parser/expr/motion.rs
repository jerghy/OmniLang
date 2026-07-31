//! Motion expression and easing parsers for OmniView.

use crate::codegen::shared::easing::resolve_easing;
use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    /// Helper: parse a numeric literal and extract its f64 value
    fn parse_number_literal(&mut self) -> Result<f64, ParseError> {
        let expr = self.parse_primary()?;
        match expr {
            Expr::Literal(Literal::Float(f), _) => Ok(f),
            Expr::Literal(Literal::Int(i), _) => Ok(i as f64),
            Expr::UnaryOp {
                op: UnaryOpKind::Neg,
                operand,
                ..
            } => match *operand {
                Expr::Literal(Literal::Float(f), _) => Ok(-f),
                Expr::Literal(Literal::Int(i), _) => Ok(-(i as f64)),
                _ => Err(self.error("expected number")),
            },
            _ => Err(self.error("expected number")),
        }
    }

    /// Helper: expect identifier equal to a specific string
    fn expect_ident_eq(&mut self, expected: &str) -> Result<(), ParseError> {
        let ident = self.expect_ident("expected identifier")?;
        if ident != expected {
            return Err(self.error(format!("expected '{}', got '{}'", expected, ident)));
        }
        Ok(())
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        // Delegate to the prefix parser. This is used by easing helpers
        // to parse literal numbers without going through the full Pratt pipeline.
        self.parse_prefix()
    }

    /// Read an easing name, reassembling hyphenated idents (the lexer does not
    /// include '-' in identifiers, so "ease-out" arrives as Ident, Minus, Ident).
    fn read_easing_name(&mut self) -> Result<String, ParseError> {
        let mut name = self.expect_ident("expected easing name")?;
        while *self.peek_kind() == TokenKind::Minus {
            self.advance(); // consume '-'
            let part = self.expect_ident("expected identifier after '-' in easing name")?;
            name = format!("{}-{}", name, part);
        }
        Ok(name)
    }

    /// Map an easing name to its canonical kebab-case form.
    ///
    /// Accepts both camelCase (existing code — transition period) and
    /// kebab-case (semantic DSL spec). Old GSAP-style names map to their
    /// closest semantic equivalents.
    fn normalize_easing_name(name: &str) -> String {
        match name {
            "easeIn" | "ease-in" => "ease-in".into(),
            "easeOut" | "ease-out" => "ease-out".into(),
            "easeInOut" | "ease-in-out" => "ease-in-out".into(),
            "power2Out" => "ease-out".into(), // closest semantic equivalent
            "backOut" => "back-out".into(),
            "elasticOut" => "elastic-out".into(),
            other => camel_to_kebab(other),
        }
    }

    /// Extract the direction suffix ("in" | "out" | "inout") from a kebab-case
    /// back/elastic name. `family` is "back" or "elastic".
    fn easing_direction(name: &str, family: &str) -> String {
        let dir = name
            .strip_prefix(family)
            .and_then(|s| s.strip_prefix('-'))
            .unwrap_or("out")
            .to_lowercase();
        if dir == "in-out" {
            "inout".into()
        } else {
            dir
        }
    }

    /// Parse easing expression: ease-out, ease-in, linear, back-in,
    /// cubic(x1:0,y1:0,x2:1,y2:1), steps(6, end)
    pub fn parse_easing(&mut self) -> Result<EasingExpr, ParseError> {
        let name = self.read_easing_name()?;
        let easing = match name.as_str() {
            "cubic" => {
                self.expect_token(TokenKind::LParen, "expected '('")?;
                self.expect_ident_eq("x1")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let x1 = self.parse_number_literal()?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                self.expect_ident_eq("y1")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let y1 = self.parse_number_literal()?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                self.expect_ident_eq("x2")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let x2 = self.parse_number_literal()?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                self.expect_ident_eq("y2")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let y2 = self.parse_number_literal()?;
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                EasingExpr::CubicBezier { x1, y1, x2, y2 }
            }
            "steps" => {
                self.expect_token(TokenKind::LParen, "expected '('")?;
                let count: u32 = match self.peek_kind() {
                    TokenKind::IntLiteral(n) => {
                        let count = *n as u32;
                        self.advance();
                        count
                    }
                    _ => return Err(self.error("expected step count")),
                };
                self.expect_token(TokenKind::Comma, "expected ','")?;
                let direction = self.expect_ident("expected 'start' or 'end'")?;
                if direction != "start" && direction != "end" {
                    return Err(self.error(format!(
                        "expected 'start' or 'end' as steps direction, got '{}'",
                        direction
                    )));
                }
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                EasingExpr::Steps { count, direction }
            }
            _ => {
                let canonical = Self::normalize_easing_name(&name);
                match canonical.as_str() {
                    "back-in" | "back-out" | "back-in-out" => EasingExpr::Back {
                        direction: Self::easing_direction(&canonical, "back"),
                    },
                    "elastic-in" | "elastic-out" | "elastic-in-out" => EasingExpr::Elastic {
                        direction: Self::easing_direction(&canonical, "elastic"),
                    },
                    n if resolve_easing(n).is_some() => EasingExpr::Named(n.to_string()),
                    _ => {
                        return Err(self.error(format!("unknown easing: '{}'", name)));
                    }
                }
            }
        };
        Ok(easing)
    }

    /// Parse a motion expression: either a motion reference (ident) or inline tween.
    pub fn parse_motion_expr(&mut self) -> Result<Expr, ParseError> {
        // Named motion reference
        self.parse_expr()
    }
}

/// Convert a camelCase identifier to kebab-case ("easeInOut" -> "ease-in-out").
fn camel_to_kebab(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
