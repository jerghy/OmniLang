use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Motion / Timeline block parsers ─────────────────

    /// Parse @motion NAME { body }
    pub fn parse_motion_def(&mut self) -> Result<MotionDef, ParseError> {
        let span_start = self.peek_span();
        self.expect_token(TokenKind::AtMotion, "expected '@motion'")?;
        let name = self.expect_ident("expected motion name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let body = self.parse_motion_body()?;
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(MotionDef {
            name,
            body,
            span: span_start,
        })
    }

    fn parse_motion_body(&mut self) -> Result<MotionBody, ParseError> {
        // Check percentage tokens FIRST (e.g., "0%") before trying ident-based matching.
        // IntLiteral/FloatLiteral are not idents, so peek_ident_value() returns None for them.
        let is_percentage = matches!(
            self.peek_kind(),
            TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_)
        );
        if is_percentage {
            return Ok(MotionBody::Keyframe(self.parse_keyframe_body()?));
        }

        // First token determines mode.
        let first = self
            .peek_ident_value()
            .ok_or_else(|| self.error("expected motion body"))?;

        match first.as_str() {
            "from" | "to" => Ok(MotionBody::Keyframe(self.parse_keyframe_body()?)),
            _ => Err(self.error(format!(
                "expected 'from' or 'to' to start keyframe body, got '{}'",
                first
            ))),
        }
    }

    /// Parse keyframe body: from { ... } to { ... } with optional over/ease/playback
    fn parse_keyframe_body(&mut self) -> Result<KeyframeBody, ParseError> {
        let mut stops = Vec::new();

        // Parse first stop
        stops.push(self.parse_keyframe_stop()?);

        // Parse remaining stops while they start with "to", "from", or percentage
        loop {
            let is_percentage = matches!(
                self.peek_kind(),
                TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_)
            );
            match self.peek_ident_value() {
                Some(ref s) if s == "to" || s == "from" => {
                    stops.push(self.parse_keyframe_stop()?);
                }
                Some(ref s) if s.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                    // Percentage stop: "50" means 50%
                    stops.push(self.parse_keyframe_stop()?);
                }
                _ if is_percentage => {
                    // IntLiteral/FloatLiteral percentage: e.g., "25", "75.5"
                    stops.push(self.parse_keyframe_stop()?);
                }
                _ => break,
            }
        }

        // Parse shared over/ease
        let mut over = Expr::Literal(
            Literal::Float(0.3),
            Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
        );
        let mut ease = None;

        self.parse_shared_timing(&mut over, &mut ease)?;

        let playback = self.parse_playback_props()?;

        Ok(KeyframeBody {
            stops,
            over,
            ease,
            playback,
        })
    }

    /// Parse a single keyframe stop: "from { ... }", "to { ... }", or "50% { ... }"
    fn parse_keyframe_stop(&mut self) -> Result<KeyframeStop, ParseError> {
        let position = match self.peek_ident_value() {
            Some(ref s) if s == "from" => {
                self.advance();
                StopPosition::From
            }
            Some(ref s) if s == "to" => {
                self.advance();
                StopPosition::To
            }
            Some(ref s) if s.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                let val: f64 = s.parse().unwrap_or(50.0);
                self.advance();
                StopPosition::Percent(val)
            }
            _ => {
                // Also accept IntLiteral/FloatLiteral for percentage (e.g., "0%", "50%")
                match self.peek_kind() {
                    TokenKind::IntLiteral(n) => {
                        let val = *n as f64;
                        self.advance();
                        // Consume optional % token after number (e.g., "0%")
                        while *self.peek_kind() == TokenKind::Percent {
                            self.advance();
                        }
                        StopPosition::Percent(val)
                    }
                    TokenKind::FloatLiteral(f) => {
                        let val = *f;
                        self.advance();
                        // Consume optional % token after number
                        while *self.peek_kind() == TokenKind::Percent {
                            self.advance();
                        }
                        StopPosition::Percent(val)
                    }
                    _ => return Err(self.error("expected 'from', 'to', or percentage")),
                }
            }
        };

        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let mut properties = Vec::new();
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            let key = self.parse_css_property_name("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        // Optional per-stop ease
        let ease = if self.peek_ident_value().is_some_and(|s| s == "ease") {
            self.advance();
            if *self.peek_kind() == TokenKind::Colon {
                self.advance();
            }
            Some(self.parse_easing()?)
        } else {
            None
        };

        Ok(KeyframeStop {
            position,
            properties,
            ease,
        })
    }

    /// Parse shared over/ease after keyframe stops
    fn parse_shared_timing(
        &mut self,
        over: &mut Expr,
        ease: &mut Option<EasingExpr>,
    ) -> Result<(), ParseError> {
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "over" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *over = self.parse_expr()?;
                }
                Some(ref s) if s == "ease" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *ease = Some(self.parse_easing()?);
                }
                _ => break,
            }
            // Optional comma between timing props
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        Ok(())
    }

    /// Parse playback properties (loop, direction, fill, delay, paused)
    fn parse_playback_props(&mut self) -> Result<Vec<PlaybackProp>, ParseError> {
        let mut props = Vec::new();
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "loop" => {
                    self.advance();
                    let value = if matches!(
                        self.peek_kind(),
                        TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_)
                    ) {
                        let t = self.advance();
                        match &t.kind {
                            TokenKind::IntLiteral(n) => LoopValue::Count(*n as u32),
                            _ => LoopValue::Infinite,
                        }
                    } else {
                        LoopValue::Infinite
                    };
                    props.push(PlaybackProp::Loop(value));
                }
                Some(ref s) if s == "direction" => {
                    self.advance();
                    let dir = match self.expect_ident("expected direction")?.as_str() {
                        "normal" => DirectionValue::Normal,
                        "reverse" => DirectionValue::Reverse,
                        "alternate" => DirectionValue::Alternate,
                        "alternateReverse" => DirectionValue::AlternateReverse,
                        other => {
                            return Err(self.error(format!(
                                "expected normal|reverse|alternate|alternateReverse, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Direction(dir));
                }
                Some(ref s) if s == "fill" => {
                    self.advance();
                    let fill = match self.expect_ident("expected fill value")?.as_str() {
                        "none" => FillValue::None,
                        "forwards" => FillValue::Forwards,
                        "backwards" => FillValue::Backwards,
                        "both" => FillValue::Both,
                        other => {
                            return Err(self.error(format!(
                                "expected none|forwards|backwards|both, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Fill(fill));
                }
                Some(ref s) if s == "delay" => {
                    self.advance();
                    props.push(PlaybackProp::Delay(self.parse_expr()?));
                }
                Some(ref s) if s == "paused" => {
                    self.advance();
                    props.push(PlaybackProp::Paused(true));
                }
                _ => break,
            }
        }
        Ok(props)
    }
}
