//! Timeline definition parser for OmniLang.
//!
//! Extracted from `motion.rs` to keep that file below 800 lines.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    /// Parse a single timeline item
    pub(super) fn parse_timeline_item(&mut self) -> Result<TimelineItemKind, ParseError> {
        // Check for stagger(...), label(...), set {...}, or IDENT motion_ref
        match self.peek_ident_value() {
            Some(ref s) if s == "stagger" => {
                self.advance();
                Ok(TimelineItemKind::Stagger(self.parse_stagger_call()?))
            }
            Some(ref s) if s == "label" => {
                self.advance();
                // Support label(name), label name, and label "name"
                if *self.peek_kind() == TokenKind::LParen {
                    // label(name) — parens form
                    self.advance();
                    let label_name = if matches!(self.peek_kind(), TokenKind::StringLiteral(_)) {
                        let t = self.advance();
                        if let TokenKind::StringLiteral(s) = &t.kind {
                            s.clone()
                        } else {
                            Err(self.error("expected string literal"))?
                        }
                    } else {
                        self.expect_ident("expected label name")?
                    };
                    let position = if *self.peek_kind() == TokenKind::Comma {
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
                        Err(self.error("expected string literal"))
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
                while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof
                {
                    let key = self.parse_css_property_name("expected property name")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    let value = self.parse_expr()?;
                    properties.push((key, value));
                    if *self.peek_kind() == TokenKind::Comma {
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
                        .is_some_and(|t| t.kind == TokenKind::Colon) =>
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
    pub(super) fn parse_direction_item(&mut self) -> Result<DirectionItem, ParseError> {
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
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            let key = self.expect_ident("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if *self.peek_kind() == TokenKind::Comma {
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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn parse_timeline_item_timing(
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
                Some(ref s) if s == "delay" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *delay = Some(self.parse_expr()?);
                }
                Some(ref s) if s == "color_space" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *color_space = Some(self.expect_ident("expected color space")?);
                }
                Some(ref s) if s == "start" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *start = Some(self.parse_position_value()?);
                }
                Some(ref s) if s == "path" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
                        self.advance();
                    }
                    *path = Some(self.expect_string("expected path string")?);
                }
                Some(ref s) if s == "pathRotate" => {
                    self.advance();
                    if *self.peek_kind() == TokenKind::Colon {
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
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        Ok(())
    }

    /// Parse position value for start:
    /// Supports both string literal and bare ident syntax:
    ///   start: "withPrev" — string literal (preferred in source)
    ///   start: with(id)   — bare ident with parens (legacy)
    pub(super) fn parse_position_value(&mut self) -> Result<PositionValue, ParseError> {
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
                if *self.peek_kind() == TokenKind::LParen {
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
                if *self.peek_kind() == TokenKind::LParen {
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
    pub(super) fn parse_timeline_items_with_expansion(
        &mut self,
    ) -> Result<Vec<TimelineItem>, ParseError> {
        let mut items = Vec::new();
        let mut connector = Connector::None;
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            let kind = self.parse_timeline_item()?;

            // Handle: ident { from { ... } to { ... } over ... }
            // Expand into multiple timeline items; parse_direction_item handles its own timing.
            if matches!(kind, TimelineItemKind::MotionRef(_))
                && *self.peek_kind() == TokenKind::LBrace
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
                    .is_some_and(|s| s == "to" || s == "from");
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
            if *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
                connector = self.parse_connector()?;
            }
        }

        Ok(items)
    }

    /// Parse @timeline NAME { body }
    pub(super) fn parse_timeline_def(&mut self) -> Result<TimelineDef, ParseError> {
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
}
