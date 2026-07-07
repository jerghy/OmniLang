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
            let mut body = self.parse_keyframe_body()?;
            body.callbacks = self.parse_optional_callbacks()?;
            return Ok(MotionBody::Keyframe(body));
        }

        // First token determines mode.
        let first = self
            .peek_ident_value()
            .ok_or_else(|| self.error("expected motion body"))?;

        match first.as_str() {
            "from" | "to" => {
                let mut body = self.parse_keyframe_body()?;
                // Check for optional callback blocks after keyframe stops
                body.callbacks = self.parse_optional_callbacks()?;
                Ok(MotionBody::Keyframe(body))
            }
            "set" | "stagger" | "label" => self.parse_timeline_body(),
            "onStart" | "onUpdate" | "onComplete" | "onRepeat" | "onInterrupt" => {
                self.parse_callback_body()
            }
            _ => self.parse_timeline_body(), // IDENT => motion reference
        }
    }

    /// Parse optional callback blocks (onStart { } onComplete { } etc.) after keyframe body.
    /// Returns empty vec if no callbacks present.
    fn parse_optional_callbacks(&mut self) -> Result<Vec<CallbackSpec>, ParseError> {
        let mut callbacks = Vec::new();
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "onStart" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnStart(*expr));
                }
                Some(ref s) if s == "onUpdate" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnUpdate(*expr));
                }
                Some(ref s) if s == "onComplete" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnComplete(*expr));
                }
                Some(ref s) if s == "onRepeat" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnRepeat(*expr));
                }
                Some(ref s) if s == "onInterrupt" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnInterrupt(*expr));
                }
                _ => break,
            }
        }
        Ok(callbacks)
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

        // Parse shared over/ease/color_space
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
        let mut color_space = None;

        self.parse_shared_timing(&mut over, &mut ease, &mut color_space)?;

        // Parse optional path/pathRotate
        let mut path: Option<String> = None;
        let mut path_rotate: Option<bool> = None;
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "path" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    path = Some(self.expect_string("expected path string")?);
                }
                Some(ref s) if s == "pathRotate" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    path_rotate = Some(match self.peek_kind() {
                        TokenKind::True => {
                            self.advance();
                            true
                        }
                        TokenKind::False => {
                            self.advance();
                            false
                        }
                        _ => return Err(self.error("expected bool for pathRotate")),
                    });
                }
                _ => break,
            }
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }

        let playback = self.parse_playback_props()?;

        // Second pass for path/pathRotate — they may appear after playback props
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "path" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    path = Some(self.expect_string("expected path string")?);
                }
                Some(ref s) if s == "pathRotate" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    path_rotate = Some(match self.peek_kind() {
                        TokenKind::True => {
                            self.advance();
                            true
                        }
                        TokenKind::False => {
                            self.advance();
                            false
                        }
                        _ => return Err(self.error("expected bool for pathRotate")),
                    });
                }
                _ => break,
            }
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }

        Ok(KeyframeBody {
            stops,
            over,
            ease,
            color_space,
            playback,
            path,
            path_rotate,
            callbacks: Vec::new(),
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

    /// Parse shared over/ease/color_space after keyframe stops
    fn parse_shared_timing(
        &mut self,
        over: &mut Expr,
        ease: &mut Option<EasingExpr>,
        color_space: &mut Option<String>,
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
                Some(ref s) if s == "color_space" || s == "colorSpace" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *color_space = Some(self.expect_ident("expected color space")?);
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

    /// Parse timeline body: sequence of items with connectors
    pub(crate) fn parse_timeline_body(&mut self) -> Result<MotionBody, ParseError> {
        let items = self.parse_timeline_items_with_expansion()?;
        let playback = self.parse_playback_props()?;
        Ok(MotionBody::Timeline(TimelineBody { items, playback }))
    }
    /// Parse callback-only motion body
    fn parse_callback_body(&mut self) -> Result<MotionBody, ParseError> {
        let mut callbacks = Vec::new();

        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "onStart" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnStart(*expr));
                }
                Some(ref s) if s == "onUpdate" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnUpdate(*expr));
                }
                Some(ref s) if s == "onComplete" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnComplete(*expr));
                }
                Some(ref s) if s == "onRepeat" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnRepeat(*expr));
                }
                Some(ref s) if s == "onInterrupt" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnInterrupt(*expr));
                }
                _ => break,
            }
        }

        let mut over = Expr::Literal(
            Literal::Float(0.3),
            Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
        );
        if self.peek_ident_value().is_some_and(|s| s == "over") {
            self.advance();
            over = self.parse_expr()?;
        }

        let playback = self.parse_playback_props()?;

        Ok(MotionBody::CallbackOnly(CallbackBody {
            callbacks,
            over,
            playback,
        }))
    }

    /// Parse playback properties (loop, direction, fill, delay, paused, etc.)
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
                Some(ref s) if s == "repeatDelay" => {
                    self.advance();
                    props.push(PlaybackProp::RepeatDelay(self.parse_expr()?));
                }
                Some(ref s) if s == "repeatRefresh" => {
                    self.advance();
                    props.push(PlaybackProp::RepeatRefresh(true));
                }
                Some(ref s) if s == "timeScale" => {
                    self.advance();
                    props.push(PlaybackProp::TimeScale(self.parse_expr()?));
                }
                Some(ref s) if s == "interrupt" => {
                    self.advance();
                    let val = match self.expect_ident("expected interrupt value")?.as_str() {
                        "stop" => InterruptValue::Stop,
                        "complete" => InterruptValue::Complete,
                        "queue" => InterruptValue::Queue,
                        "replace" => InterruptValue::Replace,
                        other => {
                            return Err(self.error(format!(
                                "expected stop|complete|queue|replace, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Interrupt(val));
                }
                Some(ref s) if s == "respectReducedMotion" => {
                    self.advance();
                    props.push(PlaybackProp::RespectReducedMotion(true));
                }
                Some(ref s) if s == "delayChildren" => {
                    self.advance();
                    props.push(PlaybackProp::DelayChildren(self.parse_expr()?));
                }
                Some(ref s) if s == "staggerChildren" => {
                    self.advance();
                    props.push(PlaybackProp::StaggerChildren(self.parse_expr()?));
                }
                Some(ref s) if s == "staggerFrom" => {
                    self.advance();
                    let from = match self.expect_ident("expected stagger direction")?.as_str() {
                        "start" => StaggerDirection::Start,
                        "center" => StaggerDirection::Center,
                        "end" => StaggerDirection::End,
                        "edges" => StaggerDirection::Edges,
                        other => {
                            return Err(self.error(format!(
                                "expected start|center|end|edges, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::StaggerFrom(from));
                }
                Some(ref s) if s == "when" => {
                    self.advance();
                    let value = match self.expect_ident("expected when value")?.as_str() {
                        "afterChildren" => WhenValue::AfterChildren,
                        "beforeChildren" => WhenValue::BeforeChildren,
                        "withChildren" => WhenValue::WithChildren,
                        other => {
                            return Err(self.error(format!(
                                "expected afterChildren|beforeChildren|withChildren, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::When(value));
                }
                _ => break,
            }
        }
        Ok(props)
    }
}
