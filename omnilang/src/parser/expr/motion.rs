//! Motion expression and easing parsers for OmniView.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
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

    fn parse_elastic_params(&mut self) -> Result<EasingExpr, ParseError> {
        self.advance(); // '('
        let mut amplitude = 1.0;
        let mut period = 0.5;
        // Parse amplitude if present
        if self.peek_ident_value().map_or(false, |s| s == "amplitude") {
            self.advance(); // ident
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            amplitude = self.parse_number_literal()?;
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        if self.peek_ident_value().map_or(false, |s| s == "period") {
            self.advance(); // ident
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            period = self.parse_number_literal()?;
        }
        self.expect_token(TokenKind::RParen, "expected ')'")?;
        Ok(EasingExpr::Elastic {
            amplitude,
            period,
            direction: None,
        })
    }

    /// Parse easing expression: easeOut, power2In, backOut(intensity:1.7),
    /// cubic(x1:0,y1:0,x2:1,y2:1), spring(tension:200,friction:15),
    /// steps(count:5,direction:end)
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
                let intensity = if self.peek_kind() == TokenKind::LParen {
                    self.advance(); // '('
                    self.expect_ident_eq("intensity")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    let val = self.parse_primary()?;
                    // Extract f64 from literal if possible
                    let num = match &val {
                        Expr::Literal(Literal::Float(f), _) => *f,
                        Expr::Literal(Literal::Int(i), _) => *i as f64,
                        _ => 1.70158,
                    };
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    num
                } else {
                    1.70158
                };
                EasingExpr::Back {
                    intensity,
                    direction: Some(name.clone()),
                }
            }
            "elasticIn" | "elasticOut" | "elasticInOut" => {
                if self.peek_kind() == TokenKind::LParen {
                    self.parse_elastic_params()?
                } else {
                    EasingExpr::Elastic {
                        amplitude: 1.0,
                        period: 0.5,
                        direction: Some(name.clone()),
                    }
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
            "spring" => {
                self.expect_token(TokenKind::LParen, "expected '('")?;
                self.expect_ident_eq("tension")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let tension = self.parse_number_literal()?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                self.expect_ident_eq("friction")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let friction = self.parse_number_literal()?;
                let velocity = if self.peek_kind() == TokenKind::Comma {
                    self.advance();
                    self.expect_ident_eq("velocity")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    Some(self.parse_number_literal()?)
                } else {
                    None
                };
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                EasingExpr::Spring {
                    tension,
                    friction,
                    velocity,
                }
            }
            "steps" => {
                self.expect_token(TokenKind::LParen, "expected '('")?;
                self.expect_ident_eq("count")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let count = self.parse_number_literal()? as u32;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                self.expect_ident_eq("direction")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let dir = self.expect_ident("expected 'end' or 'start'")?;
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                EasingExpr::Steps {
                    count,
                    direction: dir,
                }
            }
            _ => return Err(self.error(format!("unknown easing: '{}'", name))),
        };
        Ok(easing)
    }

    /// Parse stagger(itemIn, each:60ms, from:center)
    pub(crate) fn parse_stagger_call(&mut self) -> Result<StaggerCall, ParseError> {
        self.expect_token(TokenKind::LParen, "expected '('")?;
        let motion = self.parse_motion_expr()?;

        // Support shorthand: stagger(0.05) — single numeric arg means amount
        if self.peek_kind() == TokenKind::RParen {
            let timing = StaggerTiming::Amount(motion);
            self.advance(); // consume ')'
            return Ok(StaggerCall {
                motion: Box::new(Expr::Literal(
                    Literal::Null,
                    Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                )),
                timing,
                from: StaggerDirection::Start,
            });
        }

        self.expect_token(TokenKind::Comma, "expected ','")?;
        let timing = match self.expect_ident("expected 'each' or 'amount'")?.as_str() {
            "each" => {
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                StaggerTiming::Each(self.parse_expr()?)
            }
            "amount" => {
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                StaggerTiming::Amount(self.parse_expr()?)
            }
            n => return Err(self.error(format!("expected 'each' or 'amount', got '{}'", n))),
        };
        let from = if self.peek_kind() == TokenKind::Comma {
            self.advance();
            self.expect_ident_eq("from")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            match self
                .expect_ident("expected start|center|end|edges")?
                .as_str()
            {
                "start" => StaggerDirection::Start,
                "center" => StaggerDirection::Center,
                "end" => StaggerDirection::End,
                "edges" => StaggerDirection::Edges,
                _ => return Err(self.error("expected start|center|end|edges")),
            }
        } else {
            StaggerDirection::Start
        };
        self.expect_token(TokenKind::RParen, "expected ')'")?;
        Ok(StaggerCall {
            motion: Box::new(motion),
            timing,
            from,
        })
    }

    /// Parse a motion expression: either a motion reference (ident), inline tween,
    /// or stagger call.
    pub fn parse_motion_expr(&mut self) -> Result<Expr, ParseError> {
        match self.peek_ident_value() {
            Some(ref s) if s == "to" || s == "from" || s == "set" => {
                // Inline tween literal — parse as a mini-motion body (placeholder)
                let _body = self.parse_timeline_body()?;
                Ok(Expr::NotYetImplemented {
                    feature: "inline_motion".into(),
                    span: self.peek_span(),
                })
            }
            Some(ref s) if s == "stagger" => {
                self.parse_stagger_call()
                    .map(|_sc| Expr::NotYetImplemented {
                        feature: "stagger".into(),
                        span: self.peek_span(),
                    })
            }
            _ => {
                // Named motion reference or complex expression
                self.parse_expr()
            }
        }
    }
}
