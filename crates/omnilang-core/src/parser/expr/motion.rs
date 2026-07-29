//! Motion expression and easing parsers for OmniView.

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

    /// Parse easing expression: easeOut, power2In, backOut,
    /// cubic(x1:0,y1:0,x2:1,y2:1)
    pub fn parse_easing(&mut self) -> Result<EasingExpr, ParseError> {
        let name = self.expect_ident("expected easing name")?;
        let easing = match name.as_str() {
            "linear" => EasingExpr::Named("linear".into()),
            "easeIn" | "easeOut" | "easeInOut" => EasingExpr::Named(name.clone()),
            "power1In" | "power1Out" | "power1InOut" | "power2In" | "power2Out" | "power2InOut"
            | "power3In" | "power3Out" | "power3InOut" | "power4In" | "power4Out"
            | "power4InOut" | "bounceIn" | "bounceOut" | "bounceInOut" => {
                EasingExpr::Named(name.clone())
            }
            "backIn" | "backOut" | "backInOut" => {
                let dir = name.strip_prefix("back").unwrap_or("out");
                let dir_lower = dir.to_lowercase();
                EasingExpr::Back {
                    direction: if dir_lower == "out" {
                        None
                    } else {
                        Some(dir_lower)
                    },
                }
            }
            "elasticIn" | "elasticOut" | "elasticInOut" => {
                let dir = name.strip_prefix("elastic").unwrap_or("out");
                let dir_lower = dir.to_lowercase();
                EasingExpr::Elastic {
                    direction: if dir_lower == "out" {
                        None
                    } else {
                        Some(dir_lower)
                    },
                }
            }
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
            _ => return Err(self.error(format!("unknown easing: '{}'", name))),
        };
        Ok(easing)
    }

    /// Parse a motion expression: either a motion reference (ident) or inline tween.
    pub fn parse_motion_expr(&mut self) -> Result<Expr, ParseError> {
        // Named motion reference
        self.parse_expr()
    }
}
