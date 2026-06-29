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
            if self.peek_kind() == TokenKind::Comma {
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
            if self.peek_kind() == TokenKind::Comma {
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
                        let val = n as f64;
                        self.advance();
                        // Consume optional % token after number (e.g., "0%")
                        while self.peek_kind() == TokenKind::Percent {
                            self.advance();
                        }
                        StopPosition::Percent(val)
                    }
                    TokenKind::FloatLiteral(f) => {
                        let val = f;
                        self.advance();
                        // Consume optional % token after number
                        while self.peek_kind() == TokenKind::Percent {
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
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let key = self.parse_css_property_name("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        // Optional per-stop ease
        let ease = if self.peek_ident_value().map_or(false, |s| s == "ease") {
            self.advance();
            if self.peek_kind() == TokenKind::Colon {
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
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *over = self.parse_expr()?;
                }
                Some(ref s) if s == "ease" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *ease = Some(self.parse_easing()?);
                }
                Some(ref s) if s == "color_space" || s == "colorSpace" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *color_space = Some(self.expect_ident("expected color space")?);
                }
                _ => break,
            }
            // Optional comma between timing props
            if self.peek_kind() == TokenKind::Comma {
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

    /// Parse a single timeline item
    fn parse_timeline_item(&mut self) -> Result<TimelineItemKind, ParseError> {
        // Check for stagger(...), label(...), set {...}, or IDENT motion_ref
        match self.peek_ident_value() {
            Some(ref s) if s == "stagger" => {
                self.advance();
                Ok(TimelineItemKind::Stagger(self.parse_stagger_call()?))
            }
            Some(ref s) if s == "label" => {
                self.advance();
                // Support label(name), label name, and label "name"
                if self.peek_kind() == TokenKind::LParen {
                    // label(name) — parens form
                    self.advance();
                    let label_name = if matches!(self.peek_kind(), TokenKind::StringLiteral(_)) {
                        let t = self.advance();
                        if let TokenKind::StringLiteral(s) = &t.kind {
                            s.clone()
                        } else {
                            unreachable!()
                        }
                    } else {
                        self.expect_ident("expected label name")?
                    };
                    let position = if self.peek_kind() == TokenKind::Comma {
                        self.advance();
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    Ok(TimelineItemKind::Label(label_name, position))
                } else if matches!(self.peek_kind(), TokenKind::StringLiteral(_)) {
                    // label "name" — string literal form
                    let t = self.advance();
                    if let TokenKind::StringLiteral(s) = &t.kind {
                        Ok(TimelineItemKind::Label(s.clone(), None))
                    } else {
                        unreachable!()
                    }
                } else {
                    // label name — bare ident form
                    let label_name = self.expect_ident("expected label name")?;
                    Ok(TimelineItemKind::Label(label_name, None))
                }
            }
            Some(ref s) if s == "set" => {
                self.advance();
                self.expect_token(TokenKind::LBrace, "expected '{'")?;
                let mut properties = Vec::new();
                while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                    let key = self.parse_css_property_name("expected property name")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    let value = self.parse_expr()?;
                    properties.push((key, value));
                    if self.peek_kind() == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                Ok(TimelineItemKind::SetBlock(properties))
            }
            Some(ref s) if s == "from" || s == "to" => {
                // Direction item starting with "from" or "to"
                let dir = self.parse_direction_item()?;
                Ok(TimelineItemKind::Direction(dir))
            }
            // start: "withPrev" | "overlap(N)" | etc. — position modifier for previous item
            Some(ref s)
                if s == "start"
                    && self
                        .tokens
                        .get(self.pos + 1)
                        .map_or(false, |t| t.kind == TokenKind::Colon) =>
            {
                self.advance(); // consume 'start'
                self.advance(); // consume ':'
                let pos = self.parse_position_value()?;
                Ok(TimelineItemKind::StartPos(pos))
            }
            _ => {
                // Motion reference or direction item
                let elem_or_motion = self.expect_ident("expected motion reference or 'from'")?;

                // Check if next token is a motion name (ident) or direction keyword
                match self.peek_ident_value() {
                    Some(ref s) if s == "from" || s == "to" => {
                        // This was an element id, now parse direction
                        let _elem_id = elem_or_motion;
                        let dir = self.parse_direction_item()?;
                        Ok(TimelineItemKind::Direction(dir))
                    }
                    Some(ref s) if s == "over" || s == "ease" || s == "delay" => {
                        // Motion reference with inline over
                        Ok(TimelineItemKind::MotionRef(elem_or_motion))
                    }
                    Some(ref s) if s == "then" || s == "with" => {
                        // Motion reference, connector follows
                        Ok(TimelineItemKind::MotionRef(elem_or_motion))
                    }
                    _ => {
                        // Plain motion reference
                        Ok(TimelineItemKind::MotionRef(elem_or_motion))
                    }
                }
            }
        }
    }

    /// Parse a direction item (from/to with over/ease/delay)
    fn parse_direction_item(&mut self) -> Result<DirectionItem, ParseError> {
        let direction = match self.peek_ident_value() {
            Some(ref s) if s == "from" => {
                self.advance();
                Direction::FromTo
            }
            Some(ref s) if s == "to" => {
                self.advance();
                Direction::To
            }
            _ => return Err(self.error("expected 'from' or 'to'")),
        };

        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let mut properties = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let key = self.expect_ident("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        // Parse over/ease/delay/color_space/start
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
        let mut delay = None;
        let mut color_space = None;
        let mut start = None;
        let mut path: Option<String> = None;
        let mut path_rotate: Option<bool> = None;

        self.parse_timeline_item_timing(
            &mut over,
            &mut ease,
            &mut delay,
            &mut color_space,
            &mut start,
            &mut path,
            &mut path_rotate,
        )?;

        Ok(DirectionItem {
            direction,
            properties,
            over,
            ease,
            delay,
            color_space,
            start,
            path,
            path_rotate,
        })
    }

    /// Parse timeline item timing: over:, ease:, delay:, start:, path:, pathRotate:
    fn parse_timeline_item_timing(
        &mut self,
        over: &mut Expr,
        ease: &mut Option<EasingExpr>,
        delay: &mut Option<Expr>,
        color_space: &mut Option<String>,
        start: &mut Option<PositionValue>,
        path: &mut Option<String>,
        path_rotate: &mut Option<bool>,
    ) -> Result<(), ParseError> {
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "over" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *over = self.parse_expr()?;
                }
                Some(ref s) if s == "ease" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *ease = Some(self.parse_easing()?);
                }
                Some(ref s) if s == "delay" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *delay = Some(self.parse_expr()?);
                }
                Some(ref s) if s == "color_space" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *color_space = Some(self.expect_ident("expected color space")?);
                }
                Some(ref s) if s == "start" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *start = Some(self.parse_position_value()?);
                }
                Some(ref s) if s == "path" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *path = Some(self.expect_string("expected path string")?);
                }
                Some(ref s) if s == "pathRotate" => {
                    self.advance();
                    if self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *path_rotate = Some(match self.peek_kind() {
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
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        Ok(())
    }

    /// Parse position value for start:
    /// Supports both string literal and bare ident syntax:
    ///   start: "withPrev" — string literal (preferred in source)
    ///   start: with(id)   — bare ident with parens (legacy)
    fn parse_position_value(&mut self) -> Result<PositionValue, ParseError> {
        // Check for string literal first — this is the primary syntax used in source:
        //   start: "withPrev", start: "overlap(150)", etc.
        if matches!(self.peek_kind(), TokenKind::StringLiteral(_)) {
            let raw = self.expect_string("expected position string")?;
            return Ok(super::parse_position_string(&raw));
        }
        // Fall back to bare identifier syntax (legacy: with(id), after, etc.)
        let id = self.peek_ident_value();
        match id.as_deref() {
            Some("after") => {
                self.advance();
                // Check for afterPrev syntax: after(id)
                if self.peek_kind() == TokenKind::LParen {
                    self.advance();
                    let id_str = self.expect_ident("expected element id")?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    Ok(PositionValue::AfterPrev(id_str))
                } else {
                    Ok(PositionValue::After)
                }
            }
            Some("with") => {
                self.advance();
                // Check for with(prev) syntax
                if self.peek_kind() == TokenKind::LParen {
                    self.advance();
                    let _id = self.expect_ident("expected element id")?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                }
                Ok(PositionValue::WithPrev)
            }
            _ => {
                // Fallback: try to parse raw expression
                let _expr = self.parse_expr()?;
                Ok(PositionValue::Absolute("0ms".into()))
            }
        }
    }

    /// Collect timeline items from current position to RBrace/Eof, handling ident { ... } expansion.
    /// NOTE: callers must call parse_playback_props() after this if needed.
    fn parse_timeline_items_with_expansion(&mut self) -> Result<Vec<TimelineItem>, ParseError> {
        let mut items = Vec::new();
        let mut connector = Connector::None;
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let kind = self.parse_timeline_item()?;

            // Handle: ident { from { ... } to { ... } over ... }
            // Expand into multiple timeline items; parse_direction_item handles its own timing.
            if matches!(kind, TimelineItemKind::MotionRef(_))
                && self.peek_kind() == TokenKind::LBrace
            {
                self.advance(); // consume '{'
                                // Parse from { ... } direction item (handles its own timing internally)
                let dir1 = self.parse_direction_item()?;
                items.push(TimelineItem {
                    kind: TimelineItemKind::Direction(dir1),
                    connector: connector.clone(),
                });

                // Check for to { ... } — second direction item (handles its own timing)
                let has_to = self
                    .peek_ident_value()
                    .map_or(false, |s| s == "to" || s == "from");
                if has_to {
                    let dir2 = self.parse_direction_item()?;
                    items.push(TimelineItem {
                        kind: TimelineItemKind::Direction(dir2),
                        connector: Connector::Then,
                    });
                }

                self.expect_token(TokenKind::RBrace, "expected '}' after timeline item block")?;
            } else {
                items.push(TimelineItem {
                    kind,
                    connector: connector.clone(),
                });
            }

            // Parse connector between items (only if we're not at end of block)
            if self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                connector = self.parse_connector()?;
            }
        }

        Ok(items)
    }

    /// Parse @timeline NAME { body }
    pub fn parse_timeline_def(&mut self) -> Result<TimelineDef, ParseError> {
        let span_start = self.peek_span();
        self.expect_token(TokenKind::AtTimeline, "expected '@timeline'")?;
        let name = self.expect_ident("expected timeline name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let items = self.parse_timeline_items_with_expansion()?;
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        Ok(TimelineDef {
            name,
            items,
            span: span_start,
        })
    }

    /// Parse connector: "then", "with", or defaults to Then
    fn parse_connector(&mut self) -> Result<Connector, ParseError> {
        match self.peek_ident_value() {
            Some(ref s) if s == "then" => {
                self.advance();
                Ok(Connector::Then)
            }
            Some(ref s) if s == "with" => {
                self.advance();
                Ok(Connector::With)
            }
            _ => Ok(Connector::Then), // default to sequential
        }
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
        if self.peek_ident_value().map_or(false, |s| s == "over") {
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
                _ => break,
            }
        }
        Ok(props)
    }
}
